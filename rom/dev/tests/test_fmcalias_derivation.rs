// Licensed under the Apache-2.0 license

use caliptra_builder::{FwId, ImageOptions, APP_WITH_UART, ROM_WITH_UART};
use caliptra_common::RomBootStatus::ColdResetComplete;
use caliptra_common::{FirmwareHandoffTable, FuseLogEntry, FuseLogEntryId};
use caliptra_common::{PcrLogEntry, PcrLogEntryId};
use caliptra_drivers::{ColdResetEntry4, RomVerifyConfig};
use caliptra_error::CaliptraError;
use caliptra_hw_model::{BootParams, Fuses, HwModel, InitParams, ModelError, SecurityState};
use caliptra_image_fake_keys::{OWNER_CONFIG, VENDOR_CONFIG_KEY_1};
use caliptra_image_gen::ImageGenerator;
use caliptra_image_openssl::OsslCrypto;
use caliptra_image_types::IMAGE_BYTE_SIZE;
use zerocopy::{AsBytes, FromBytes};

pub mod helpers;

#[test]
fn test_zero_firmware_size() {
    let (mut hw, _image_bundle) =
        helpers::build_hw_model_and_image_bundle(Fuses::default(), ImageOptions::default());

    // Zero-sized firmware.
    assert_eq!(
        hw.upload_firmware(&[]).unwrap_err(),
        ModelError::MailboxCmdFailed(CaliptraError::FW_PROC_INVALID_IMAGE_SIZE.into())
    );
    assert_eq!(
        hw.soc_ifc().cptra_fw_error_fatal().read(),
        CaliptraError::FW_PROC_INVALID_IMAGE_SIZE.into()
    );
}

#[test]
fn test_firmware_gt_max_size() {
    const FW_LOAD_CMD_OPCODE: u32 = 0x4657_4C44;

    // Firmware size > 128 KB.

    let (mut hw, _image_bundle) =
        helpers::build_hw_model_and_image_bundle(Fuses::default(), ImageOptions::default());

    // Manually put the oversize data in the mailbox because
    // HwModel::upload_firmware won't let us.
    assert!(!hw.soc_mbox().lock().read().lock());
    hw.soc_mbox().cmd().write(|_| FW_LOAD_CMD_OPCODE);
    hw.soc_mbox().dlen().write(|_| (IMAGE_BYTE_SIZE + 1) as u32);
    for i in 0..((IMAGE_BYTE_SIZE + 1 + 3) / 4) {
        hw.soc_mbox().datain().write(|_| i as u32);
    }
    hw.soc_mbox().execute().write(|w| w.execute(true));
    while hw.soc_mbox().status().read().status().cmd_busy() {
        hw.step();
    }
    hw.soc_mbox().execute().write(|w| w.execute(false));

    assert_eq!(
        hw.soc_ifc().cptra_fw_error_fatal().read(),
        CaliptraError::FW_PROC_INVALID_IMAGE_SIZE.into()
    );
}

#[test]
fn test_pcr_log() {
    let gen = ImageGenerator::new(OsslCrypto::default());
    let (_hw, image_bundle) =
        helpers::build_hw_model_and_image_bundle(Fuses::default(), ImageOptions::default());
    let mut vendor_pubkey_digest = gen
        .vendor_pubkey_digest(&image_bundle.manifest.preamble)
        .unwrap();

    let mut owner_pubkey_digest = gen
        .owner_pubkey_digest(&image_bundle.manifest.preamble)
        .unwrap();

    pub const TEST_FMC_WITH_UART: FwId = FwId {
        crate_name: "caliptra-rom-test-fmc",
        bin_name: "caliptra-rom-test-fmc",
        features: &["emu"],
        workspace_dir: None,
    };

    let fuses = Fuses {
        anti_rollback_disable: true,
        lms_verify: true,
        key_manifest_pk_hash: vendor_pubkey_digest,
        owner_pk_hash: owner_pubkey_digest,
        ..Default::default()
    };
    let rom = caliptra_builder::build_firmware_rom(&ROM_WITH_UART).unwrap();
    let mut hw = caliptra_hw_model::new(BootParams {
        init_params: InitParams {
            rom: &rom,
            security_state: SecurityState::from(fuses.life_cycle as u32),
            ..Default::default()
        },
        fuses,
        ..Default::default()
    })
    .unwrap();

    const FMC_SVN: u32 = 1;
    let image_options = ImageOptions {
        vendor_config: VENDOR_CONFIG_KEY_1,
        fmc_svn: FMC_SVN,
        ..Default::default()
    };
    let image_bundle =
        caliptra_builder::build_and_sign_image(&TEST_FMC_WITH_UART, &APP_WITH_UART, image_options)
            .unwrap();

    assert!(hw
        .upload_firmware(&image_bundle.to_bytes().unwrap())
        .is_ok());

    hw.step_until_boot_status(ColdResetComplete.into(), true);

    let result = hw.mailbox_execute(0x1000_0000, &[]);
    assert!(result.is_ok());

    let pcr_entry_arr = result.unwrap().unwrap();
    let mut pcr_log_entry_offset = 0;
    // Check PCR entry for DeviceLifecycle.
    let pcr_log_entry =
        PcrLogEntry::read_from_prefix(pcr_entry_arr[pcr_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(pcr_log_entry.id, PcrLogEntryId::DeviceLifecycle as u16);
    assert_eq!(pcr_log_entry.pcr_id, 0);
    let device_lifecycle = hw
        .soc_ifc()
        .cptra_security_state()
        .read()
        .device_lifecycle();
    assert_eq!(pcr_log_entry.pcr_data[0] as u8, device_lifecycle as u8);

    // Check PCR entry for DebugLocked
    pcr_log_entry_offset += core::mem::size_of::<PcrLogEntry>();
    let pcr_log_entry =
        PcrLogEntry::read_from_prefix(pcr_entry_arr[pcr_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(pcr_log_entry.id, PcrLogEntryId::DebugLocked as u16);
    assert_eq!(pcr_log_entry.pcr_id, 0);
    let debug_locked = hw.soc_ifc().cptra_security_state().read().debug_locked();
    assert_eq!((pcr_log_entry.pcr_data[0] as u8) != 0, debug_locked);

    // Check PCR entry for AntiRollbackDisabled.
    pcr_log_entry_offset += core::mem::size_of::<PcrLogEntry>();
    let pcr_log_entry =
        PcrLogEntry::read_from_prefix(pcr_entry_arr[pcr_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(pcr_log_entry.id, PcrLogEntryId::AntiRollbackDisabled as u16);
    assert_eq!(pcr_log_entry.pcr_id, 0);
    let anti_rollback_disable = hw.soc_ifc().fuse_anti_rollback_disable().read().dis();
    assert_eq!(
        (pcr_log_entry.pcr_data[0] as u8) != 0,
        anti_rollback_disable
    );

    // Check PCR entry for VendorPubKeyHash.
    pcr_log_entry_offset += core::mem::size_of::<PcrLogEntry>();
    let pcr_log_entry =
        PcrLogEntry::read_from_prefix(pcr_entry_arr[pcr_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(pcr_log_entry.id, PcrLogEntryId::VendorPubKeyHash as u16);
    assert_eq!(pcr_log_entry.pcr_id, 0);
    helpers::change_dword_endianess(vendor_pubkey_digest.as_bytes_mut());
    assert_eq!(pcr_log_entry.pcr_data, vendor_pubkey_digest);

    // Check PCR entry for OwnerPubKeyHash.
    pcr_log_entry_offset += core::mem::size_of::<PcrLogEntry>();
    let pcr_log_entry =
        PcrLogEntry::read_from_prefix(pcr_entry_arr[pcr_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(pcr_log_entry.id, PcrLogEntryId::OwnerPubKeyHash as u16);
    assert_eq!(pcr_log_entry.pcr_id, 0);
    helpers::change_dword_endianess(owner_pubkey_digest.as_bytes_mut());
    assert_eq!(pcr_log_entry.pcr_data, owner_pubkey_digest);

    // Check PCR entry for VendorPubKeyIndex.
    pcr_log_entry_offset += core::mem::size_of::<PcrLogEntry>();
    let pcr_log_entry =
        PcrLogEntry::read_from_prefix(pcr_entry_arr[pcr_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(pcr_log_entry.id, PcrLogEntryId::EccVendorPubKeyIndex as u16);
    assert_eq!(pcr_log_entry.pcr_id, 0);
    assert_eq!(
        pcr_log_entry.pcr_data[0] as u8,
        VENDOR_CONFIG_KEY_1.ecc_key_idx as u8
    );

    // Check PCR entry for FmcTci.
    pcr_log_entry_offset += core::mem::size_of::<PcrLogEntry>();
    let pcr_log_entry =
        PcrLogEntry::read_from_prefix(pcr_entry_arr[pcr_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(pcr_log_entry.id, PcrLogEntryId::FmcTci as u16);
    assert_eq!(pcr_log_entry.pcr_id, 0);

    // Check PCR entry for FmcSvn.
    pcr_log_entry_offset += core::mem::size_of::<PcrLogEntry>();
    let pcr_log_entry =
        PcrLogEntry::read_from_prefix(pcr_entry_arr[pcr_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(pcr_log_entry.id, PcrLogEntryId::FmcSvn as u16);
    assert_eq!(pcr_log_entry.pcr_id, 0);
    assert_eq!(pcr_log_entry.pcr_data[0] as u8, FMC_SVN as u8);

    // Check PCR entry for FmcFuseSvn.
    pcr_log_entry_offset += core::mem::size_of::<PcrLogEntry>();
    let pcr_log_entry =
        PcrLogEntry::read_from_prefix(pcr_entry_arr[pcr_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(pcr_log_entry.id, PcrLogEntryId::FmcFuseSvn as u16);
    assert_eq!(pcr_log_entry.pcr_id, 0);
    assert_eq!(pcr_log_entry.pcr_data[0] as u8, 0); // anti_rollback_disable is true

    // Check PCR entry for LmsVendorPubKeyIndex.
    pcr_log_entry_offset += core::mem::size_of::<PcrLogEntry>();
    let pcr_log_entry =
        PcrLogEntry::read_from_prefix(pcr_entry_arr[pcr_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(pcr_log_entry.id, PcrLogEntryId::LmsVendorPubKeyIndex as u16);
    assert_eq!(pcr_log_entry.pcr_id, 0);
    assert_eq!(
        pcr_log_entry.pcr_data[0] as u8,
        VENDOR_CONFIG_KEY_1.lms_key_idx as u8
    );

    // Check Rom verify config
    pcr_log_entry_offset += core::mem::size_of::<PcrLogEntry>();
    let pcr_log_entry =
        PcrLogEntry::read_from_prefix(pcr_entry_arr[pcr_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(pcr_log_entry.id, PcrLogEntryId::RomVerifyConfig as u16);
    assert_eq!(pcr_log_entry.pcr_id, 0);
    assert_eq!(
        pcr_log_entry.pcr_data[0] as u8,
        RomVerifyConfig::EcdsaAndLms as u8
    );
}

#[test]
fn test_pcr_log_fmc_fuse_svn() {
    let gen = ImageGenerator::new(OsslCrypto::default());
    let (_hw, image_bundle) =
        helpers::build_hw_model_and_image_bundle(Fuses::default(), ImageOptions::default());
    let vendor_pubkey_digest = gen
        .vendor_pubkey_digest(&image_bundle.manifest.preamble)
        .unwrap();

    let owner_pubkey_digest = gen
        .owner_pubkey_digest(&image_bundle.manifest.preamble)
        .unwrap();

    pub const TEST_FMC_WITH_UART: FwId = FwId {
        crate_name: "caliptra-rom-test-fmc",
        bin_name: "caliptra-rom-test-fmc",
        features: &["emu"],
        workspace_dir: None,
    };

    let fuses = Fuses {
        anti_rollback_disable: false,
        key_manifest_pk_hash: vendor_pubkey_digest,
        owner_pk_hash: owner_pubkey_digest,
        fmc_key_manifest_svn: 0b11, // FMC Fuse SVN = 2
        ..Default::default()
    };
    let rom = caliptra_builder::build_firmware_rom(&ROM_WITH_UART).unwrap();
    let mut hw = caliptra_hw_model::new(BootParams {
        init_params: InitParams {
            rom: &rom,
            security_state: SecurityState::from(fuses.life_cycle as u32),
            ..Default::default()
        },
        fuses,
        ..Default::default()
    })
    .unwrap();

    const FMC_SVN: u32 = 2;
    let image_options = ImageOptions {
        vendor_config: VENDOR_CONFIG_KEY_1,
        fmc_svn: FMC_SVN,
        ..Default::default()
    };
    let image_bundle =
        caliptra_builder::build_and_sign_image(&TEST_FMC_WITH_UART, &APP_WITH_UART, image_options)
            .unwrap();

    assert!(hw
        .upload_firmware(&image_bundle.to_bytes().unwrap())
        .is_ok());

    hw.step_until_boot_status(ColdResetComplete.into(), true);

    let result = hw.mailbox_execute(0x1000_0000, &[]);
    assert!(result.is_ok());

    let pcr_entry_arr = result.unwrap().unwrap();

    // Check PCR entry for FmcFuseSvn.
    let pcr_log_entry = PcrLogEntry::read_from_prefix(
        pcr_entry_arr
            [((PcrLogEntryId::FmcFuseSvn as usize - 1) * core::mem::size_of::<PcrLogEntry>())..]
            .as_bytes(),
    )
    .unwrap();
    assert_eq!(pcr_log_entry.id, PcrLogEntryId::FmcFuseSvn as u16);
    assert_eq!(pcr_log_entry.pcr_id, 0);
    assert_eq!(pcr_log_entry.pcr_data[0] as u8, FMC_SVN as u8); // anti_rollback_disable is false
}

#[test]
fn test_fuse_log() {
    const FMC_SVN: u32 = 4;
    const FMC_MIN_SVN: u32 = 2;

    let fuses = Fuses {
        anti_rollback_disable: true,
        fmc_key_manifest_svn: 0x0F,  // Value of FMC_SVN
        runtime_svn: [0xF, 0, 0, 0], // Value of RT_SVN
        lms_verify: true,
        ..Default::default()
    };

    pub const TEST_FMC_WITH_UART: FwId = FwId {
        crate_name: "caliptra-rom-test-fmc",
        bin_name: "caliptra-rom-test-fmc",
        features: &["emu"],
        workspace_dir: None,
    };

    let rom = caliptra_builder::build_firmware_rom(&ROM_WITH_UART).unwrap();
    let mut hw = caliptra_hw_model::new(BootParams {
        init_params: InitParams {
            rom: &rom,
            security_state: SecurityState::from(fuses.life_cycle as u32),
            ..Default::default()
        },
        fuses,
        ..Default::default()
    })
    .unwrap();

    let image_options = ImageOptions {
        vendor_config: VENDOR_CONFIG_KEY_1,
        owner_config: Some(OWNER_CONFIG),
        fmc_svn: FMC_SVN,
        fmc_min_svn: FMC_MIN_SVN,
        app_svn: FMC_SVN,
        app_min_svn: FMC_MIN_SVN,
    };
    let image_bundle =
        caliptra_builder::build_and_sign_image(&TEST_FMC_WITH_UART, &APP_WITH_UART, image_options)
            .unwrap();

    assert!(hw
        .upload_firmware(&image_bundle.to_bytes().unwrap())
        .is_ok());

    hw.step_until_boot_status(ColdResetComplete.into(), true);

    let result = hw.mailbox_execute(0x1000_0002, &[]);
    assert!(result.is_ok());

    let fuse_entry_arr = result.unwrap().unwrap();

    let mut fuse_log_entry_offset = 0;

    // Check entry for VendorPubKeyIndex.
    let fuse_log_entry =
        FuseLogEntry::read_from_prefix(fuse_entry_arr[fuse_log_entry_offset..].as_bytes()).unwrap();

    assert_eq!(
        fuse_log_entry.entry_id,
        FuseLogEntryId::VendorEccPubKeyIndex as u32
    );

    assert_eq!(fuse_log_entry.log_data[0], VENDOR_CONFIG_KEY_1.ecc_key_idx);

    // Validate that the ID is VendorPubKeyRevocation
    fuse_log_entry_offset += core::mem::size_of::<FuseLogEntry>();
    let fuse_log_entry =
        FuseLogEntry::read_from_prefix(fuse_entry_arr[fuse_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(
        fuse_log_entry.entry_id,
        FuseLogEntryId::VendorEccPubKeyRevocation as u32
    );
    assert_eq!(fuse_log_entry.log_data[0], 0,);

    // Validate the ManifestFmcSvn
    fuse_log_entry_offset += core::mem::size_of::<FuseLogEntry>();
    let fuse_log_entry =
        FuseLogEntry::read_from_prefix(fuse_entry_arr[fuse_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(
        fuse_log_entry.entry_id,
        FuseLogEntryId::ManifestFmcSvn as u32
    );
    assert_eq!(fuse_log_entry.log_data[0], FMC_SVN);

    // Validate the ManifestFmcMinSvn
    fuse_log_entry_offset += core::mem::size_of::<FuseLogEntry>();
    let fuse_log_entry =
        FuseLogEntry::read_from_prefix(fuse_entry_arr[fuse_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(
        fuse_log_entry.entry_id,
        FuseLogEntryId::ManifestFmcMinSvn as u32
    );
    assert_eq!(fuse_log_entry.log_data[0], FMC_MIN_SVN);

    // Validate the FuseFmcSvn
    fuse_log_entry_offset += core::mem::size_of::<FuseLogEntry>();
    let fuse_log_entry =
        FuseLogEntry::read_from_prefix(fuse_entry_arr[fuse_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(fuse_log_entry.entry_id, FuseLogEntryId::FuseFmcSvn as u32);
    assert_eq!(fuse_log_entry.log_data[0], FMC_SVN);

    // Validate the ManifestRtSvn
    fuse_log_entry_offset += core::mem::size_of::<FuseLogEntry>();
    let fuse_log_entry =
        FuseLogEntry::read_from_prefix(fuse_entry_arr[fuse_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(
        fuse_log_entry.entry_id,
        FuseLogEntryId::ManifestRtSvn as u32
    );
    assert_eq!(fuse_log_entry.log_data[0], FMC_SVN);

    // Validate the ManifestRtMinSvn
    fuse_log_entry_offset += core::mem::size_of::<FuseLogEntry>();
    let fuse_log_entry =
        FuseLogEntry::read_from_prefix(fuse_entry_arr[fuse_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(
        fuse_log_entry.entry_id,
        FuseLogEntryId::ManifestRtMinSvn as u32
    );
    assert_eq!(fuse_log_entry.log_data[0], FMC_MIN_SVN);

    // Validate the FuseRtSvn
    fuse_log_entry_offset += core::mem::size_of::<FuseLogEntry>();
    let fuse_log_entry =
        FuseLogEntry::read_from_prefix(fuse_entry_arr[fuse_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(fuse_log_entry.entry_id, FuseLogEntryId::FuseRtSvn as u32);
    assert_eq!(fuse_log_entry.log_data[0], FMC_SVN);

    // Validate the VendorLmsPubKeyIndex
    fuse_log_entry_offset += core::mem::size_of::<FuseLogEntry>();
    let fuse_log_entry =
        FuseLogEntry::read_from_prefix(fuse_entry_arr[fuse_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(
        fuse_log_entry.entry_id,
        FuseLogEntryId::VendorLmsPubKeyIndex as u32
    );
    assert_eq!(fuse_log_entry.log_data[0], VENDOR_CONFIG_KEY_1.lms_key_idx);

    // Validate that the ID is VendorPubKeyRevocation
    fuse_log_entry_offset += core::mem::size_of::<FuseLogEntry>();
    let fuse_log_entry =
        FuseLogEntry::read_from_prefix(fuse_entry_arr[fuse_log_entry_offset..].as_bytes()).unwrap();
    assert_eq!(
        fuse_log_entry.entry_id,
        FuseLogEntryId::VendorLmsPubKeyRevocation as u32
    );
    assert_eq!(fuse_log_entry.log_data[0], 0,);
}

#[test]
fn test_fht_info() {
    pub const TEST_FMC_WITH_UART: FwId = FwId {
        crate_name: "caliptra-rom-test-fmc",
        bin_name: "caliptra-rom-test-fmc",
        features: &["emu"],
        workspace_dir: None,
    };
    let rom = caliptra_builder::build_firmware_rom(&ROM_WITH_UART).unwrap();
    let mut hw = caliptra_hw_model::new(BootParams {
        init_params: InitParams {
            rom: &rom,
            ..Default::default()
        },
        fuses: Fuses::default(),
        ..Default::default()
    })
    .unwrap();

    let image_bundle = caliptra_builder::build_and_sign_image(
        &TEST_FMC_WITH_UART,
        &APP_WITH_UART,
        ImageOptions::default(),
    )
    .unwrap();
    assert!(hw
        .upload_firmware(&image_bundle.to_bytes().unwrap())
        .is_ok());

    hw.step_until_boot_status(ColdResetComplete.into(), true);

    let result = hw.mailbox_execute(0x1000_0003, &[]);
    assert!(result.is_ok());

    let data = result.unwrap().unwrap();
    let fht = FirmwareHandoffTable::read_from_prefix(data.as_bytes()).unwrap();
    assert_eq!(fht.ldevid_tbs_size, 530);
    assert_eq!(fht.fmcalias_tbs_size, 742);
    assert_eq!(fht.ldevid_tbs_addr, 0x50003800);
    assert_eq!(fht.fmcalias_tbs_addr, 0x50003C00);
    assert_eq!(fht.pcr_log_addr, 0x50004000);
    assert_eq!(fht.fuse_log_addr, 0x50004400);

    // [TODO] Expand test to validate additional FHT fields.
}

#[test]
fn test_check_no_lms_info_in_datavault_on_lms_unavailable() {
    let (_hw, _image_bundle) =
        helpers::build_hw_model_and_image_bundle(Fuses::default(), ImageOptions::default());

    pub const TEST_FMC_WITH_UART: FwId = FwId {
        crate_name: "caliptra-rom-test-fmc",
        bin_name: "caliptra-rom-test-fmc",
        features: &["emu"],
        workspace_dir: None,
    };

    let fuses = Fuses {
        lms_verify: false,
        ..Default::default()
    };
    let rom = caliptra_builder::build_firmware_rom(&ROM_WITH_UART).unwrap();
    let mut hw = caliptra_hw_model::new(BootParams {
        init_params: InitParams {
            rom: &rom,
            security_state: SecurityState::from(fuses.life_cycle as u32),
            ..Default::default()
        },
        fuses,
        ..Default::default()
    })
    .unwrap();

    let image_bundle = caliptra_builder::build_and_sign_image(
        &TEST_FMC_WITH_UART,
        &APP_WITH_UART,
        ImageOptions::default(),
    )
    .unwrap();

    assert!(hw
        .upload_firmware(&image_bundle.to_bytes().unwrap())
        .is_ok());

    hw.step_until_boot_status(ColdResetComplete.into(), true);

    let result = hw.mailbox_execute(0x1000_0005, &[]);
    assert!(result.is_ok());

    let coldresetentry4_array = result.unwrap().unwrap();
    let mut coldresetentry4_offset = core::mem::size_of::<u32>() * 6; // Skip first 3 entries

    // Check LmsVendorPubKeyIndex datavault value.
    let coldresetentry4_id =
        u32::read_from_prefix(coldresetentry4_array[coldresetentry4_offset..].as_bytes()).unwrap();
    assert_eq!(
        coldresetentry4_id,
        ColdResetEntry4::LmsVendorPubKeyIndex as u32
    );
    coldresetentry4_offset += core::mem::size_of::<u32>();
    let coldresetentry4_value =
        u32::read_from_prefix(coldresetentry4_array[coldresetentry4_offset..].as_bytes()).unwrap();
    assert_eq!(coldresetentry4_value, u32::MAX);
}
