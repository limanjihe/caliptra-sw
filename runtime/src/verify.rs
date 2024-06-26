// Licensed under the Apache-2.0 license

use crate::{Drivers, EcdsaVerifyReq, MailboxResp};
use caliptra_drivers::{
    Array4x12, CaliptraError, CaliptraResult, Ecc384PubKey, Ecc384Result, Ecc384Scalar,
    Ecc384Signature,
};
use zerocopy::FromBytes;

pub struct EcdsaVerifyCmd;
impl EcdsaVerifyCmd {
    pub(crate) fn execute(drivers: &mut Drivers, cmd_args: &[u8]) -> CaliptraResult<MailboxResp> {
        if let Some(cmd) = EcdsaVerifyReq::read_from(cmd_args) {
            // Won't panic, full_digest is always larger than digest
            let full_digest = drivers.sha_acc.regs().digest().read();
            let mut digest = Array4x12::default();
            for (i, target_word) in digest.0.iter_mut().enumerate() {
                *target_word = full_digest[i];
            }

            let pubkey = Ecc384PubKey {
                x: Ecc384Scalar::from(cmd.pub_key_x),
                y: Ecc384Scalar::from(cmd.pub_key_y),
            };

            let sig = Ecc384Signature {
                r: Ecc384Scalar::from(cmd.signature_r),
                s: Ecc384Scalar::from(cmd.signature_s),
            };

            let success = drivers.ecdsa.verify(&pubkey, &digest, &sig)?;
            if success != Ecc384Result::Success {
                return Err(CaliptraError::RUNTIME_ECDSA_VERIFY_FAILED);
            }
        } else {
            return Err(CaliptraError::RUNTIME_INSUFFICIENT_MEMORY);
        };

        Ok(MailboxResp::default())
    }
}
