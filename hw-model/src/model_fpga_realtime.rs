// Licensed under the Apache-2.0 license

use std::{env, str::FromStr};

use bitfield::bitfield;
use caliptra_emu_bus::{Bus, BusError};
use caliptra_emu_types::{RvAddr, RvData, RvSize};
use std::io::Write;
use std::time;
use uio::{UioDevice, UioError};

use crate::HwModel;
use crate::Output;

// TODO: Make PAUSER configurable
const SOC_PAUSER: u32 = 0xffff_ffff;

fn fmt_uio_error(err: UioError) -> String {
    format!("{err:?}")
}

// FPGA SOC wire register offsets
const GPIO_OUTPUT_OFFSET: isize = 0x0000 / 4;
const GPIO_INPUT_OFFSET: isize = 0x0008 / 4;
const GPIO_PAUSER_OFFSET: isize = 0x000C / 4;
const GPIO_DEOBF_KEY_OFFSET: isize = 0x0020 / 4;
const GPIO_LOG_FIFO_DATA_OFFSET: isize = 0x1000 / 4;
const GPIO_LOG_FIFO_STATUS_OFFSET: isize = 0x1004 / 4;

bitfield! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    /// GPIO SOC wires -> Caliptra
    pub struct GpioOutput(u32);
    cptra_rst_b, set_cptra_rst_b: 0, 0;
    cptra_pwrgood, set_cptra_pwrgood: 1, 1;
    security_state, set_security_state: 6, 4;
}

bitfield! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    /// GPIO SOC wires <- Caliptra
    pub struct GpioInput(u32);
    cptra_error_fatal, _: 26, 26;
    cptra_error_non_fatal, _: 27, 27;
    ready_for_fw, _: 28, 28;
    ready_for_runtime, _: 29, 29;
    ready_for_fuses, _: 30, 30;
}

bitfield! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    /// Log FIFO data
    pub struct FifoData(u32);
    log_fifo_char, _: 7, 0;
    log_fifo_valid, _: 8, 8;
}

bitfield! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    /// Log FIFO status
    pub struct FifoStatus(u32);
    log_fifo_empty, _: 0, 0;
    log_fifo_full, _: 1, 1;
}

pub struct ModelFpgaRealtime {
    gpio: *mut u32,
    mbox: *mut u32,
    soc_ifc: *mut u32,
    output: Output,
    start_time: time::Instant,
}

impl ModelFpgaRealtime {
    fn is_ready_for_fuses(&self) -> bool {
        unsafe {
            GpioInput(self.gpio.offset(GPIO_INPUT_OFFSET).read_volatile()).ready_for_fuses() != 0
        }
    }
    fn set_cptra_pwrgood(&mut self, value: bool) {
        unsafe {
            let mut val = GpioOutput(self.gpio.offset(GPIO_OUTPUT_OFFSET).read_volatile());
            val.set_cptra_pwrgood(value as u32);
            self.gpio.offset(GPIO_OUTPUT_OFFSET).write_volatile(val.0);
        }
    }
    fn set_cptra_rst_b(&mut self, value: bool) {
        unsafe {
            let mut val = GpioOutput(self.gpio.offset(GPIO_OUTPUT_OFFSET).read_volatile());
            val.set_cptra_rst_b(value as u32);
            self.gpio.offset(GPIO_OUTPUT_OFFSET).write_volatile(val.0);
        }
    }
    fn set_security_state(&mut self, value: u32) {
        unsafe {
            let mut val = GpioOutput(self.gpio.offset(GPIO_OUTPUT_OFFSET).read_volatile());
            val.set_security_state(value);
            self.gpio.offset(GPIO_OUTPUT_OFFSET).write_volatile(val.0);
        }
    }
    fn set_pauser(&mut self, pauser: u32) {
        unsafe {
            self.gpio.offset(GPIO_PAUSER_OFFSET).write_volatile(pauser);
        }
    }
}

impl HwModel for ModelFpgaRealtime {
    type TBus<'a> = FpgaRealtimeBus<'a>;

    fn new_unbooted(params: crate::InitParams) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized,
    {
        let output = Output::new(params.log_writer);
        let uio_num = usize::from_str(&env::var("CPTRA_UIO_NUM")?)?;
        let dev = UioDevice::new(uio_num)?;

        let gpio = dev.map_mapping(0).map_err(fmt_uio_error)? as *mut u32;
        let mbox = dev.map_mapping(1).map_err(fmt_uio_error)? as *mut u32;
        let soc_ifc = dev.map_mapping(2).map_err(fmt_uio_error)? as *mut u32;
        let start_time = time::Instant::now();

        let mut m = Self {
            gpio,
            mbox,
            soc_ifc,
            output,
            start_time,
        };

        writeln!(m.output().logger(), "new_unbooted")?;
        // Set pwrgood and rst_b to 0 to boot from scratch
        m.set_cptra_pwrgood(false);
        m.set_cptra_rst_b(false);

        // Set Security State signal wires
        m.set_security_state(u32::from(params.security_state));

        // Set initial PAUSER
        m.set_pauser(SOC_PAUSER);

        // Set deobfuscation key
        for i in 0..8 {
            unsafe {
                m.gpio
                    .offset(GPIO_DEOBF_KEY_OFFSET + i)
                    .write_volatile(params.cptra_obf_key[i as usize])
            };
        }

        // Write ROM image over backdoor
        let mut rom_driver = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/caliptra-rom-backdoor")
            .unwrap();
        rom_driver.write_all(params.rom)?;
        rom_driver.sync_all()?;

        // Bring Caliptra out of reset and wait for ready_for_fuses
        m.set_cptra_pwrgood(true);
        m.set_cptra_rst_b(true);
        while !m.is_ready_for_fuses() {}
        writeln!(m.output().logger(), "ready_for_fuses is high")?;

        Ok(m)
    }

    fn apb_bus(&mut self) -> Self::TBus<'_> {
        FpgaRealtimeBus { m: self }
    }

    fn step(&mut self) {
        // Check if the FIFO is full (which probably means there was an overrun)
        let fifosts = unsafe {
            FifoStatus(
                self.gpio
                    .offset(GPIO_LOG_FIFO_STATUS_OFFSET)
                    .read_volatile(),
            )
        };
        if fifosts.log_fifo_full() != 0 {
            panic!("FPGA log FIFO overran");
        }
        // Check and empty log FIFO
        loop {
            let fifodata =
                unsafe { FifoData(self.gpio.offset(GPIO_LOG_FIFO_DATA_OFFSET).read_volatile()) };
            // Add byte to log if it is valid
            if fifodata.log_fifo_valid() != 0 {
                self.output()
                    .sink()
                    .push_uart_char(fifodata.log_fifo_char().try_into().unwrap());
            } else {
                break;
            }
        }

        // Handle etrng request
        if self.soc_ifc_trng().cptra_trng_status().read().data_req() {
            // Write CPTRA_TRNG_STATUS.DATA_WR_DONE
            self.soc_ifc_trng()
                .cptra_trng_status()
                .write(|w| w.data_wr_done(true));
        }
    }

    fn output(&mut self) -> &mut crate::Output {
        self.output
            .sink()
            .set_now(self.start_time.elapsed().as_millis().try_into().unwrap());
        &mut self.output
    }

    fn warm_reset(&mut self) {
        // Toggle reset pin
        self.set_cptra_rst_b(false);
        self.set_cptra_rst_b(true);
        // Wait for ready_for_fuses
        while !self.is_ready_for_fuses() {}
    }

    fn ready_for_fw(&self) -> bool {
        unsafe {
            GpioInput(self.gpio.offset(GPIO_INPUT_OFFSET).read_volatile()).ready_for_fw() != 0
        }
    }

    fn tracing_hint(&mut self, _enable: bool) {
        // Do nothing; we don't support tracing yet
    }
}

pub struct FpgaRealtimeBus<'a> {
    m: &'a mut ModelFpgaRealtime,
}
impl<'a> FpgaRealtimeBus<'a> {
    fn ptr_for_addr(&mut self, addr: RvAddr) -> Option<*mut u32> {
        let addr = addr as usize;
        unsafe {
            match addr {
                0x3002_0000..=0x3002_ffff => Some(self.m.mbox.add((addr & 0xffff) / 4)),
                0x3003_0000..=0x3003_ffff => Some(self.m.soc_ifc.add((addr & 0xffff) / 4)),
                _ => None,
            }
        }
    }
}
impl<'a> Bus for FpgaRealtimeBus<'a> {
    fn read(&mut self, _size: RvSize, addr: RvAddr) -> Result<RvData, BusError> {
        if let Some(ptr) = self.ptr_for_addr(addr) {
            Ok(unsafe { ptr.read_volatile() })
        } else {
            println!("Error LoadAccessFault");
            Err(BusError::LoadAccessFault)
        }
    }

    fn write(
        &mut self,
        _size: RvSize,
        addr: RvAddr,
        val: RvData,
    ) -> Result<(), caliptra_emu_bus::BusError> {
        if let Some(ptr) = self.ptr_for_addr(addr) {
            // TODO: support 16-bit and 8-bit writes
            unsafe { ptr.write_volatile(val) };
            Ok(())
        } else {
            Err(BusError::StoreAccessFault)
        }
    }
}
