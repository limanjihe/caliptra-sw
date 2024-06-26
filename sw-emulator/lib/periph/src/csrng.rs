// Licensed under the Apache-2.0 license

use caliptra_emu_bus::{BusError, ReadOnlyRegister, ReadWriteRegister, WriteOnlyRegister};

use caliptra_emu_derive::Bus;
use caliptra_emu_types::{RvData, RvSize};

use rand_impl::{Block, CtrDrbg};

use std::mem;

#[derive(Bus)]
pub struct Csrng {
    // CSRNG registers
    #[register(offset = 0x14)]
    ctrl: ReadWriteRegister<u32>,

    #[register(offset = 0x18, write_fn = cmd_req_write)]
    cmd_req: WriteOnlyRegister<u32>,

    #[register(offset = 0x1c)]
    sw_cmd_sts: ReadOnlyRegister<u32>,

    #[register(offset = 0x20, read_fn = genbits_vld_read)]
    genbits_vld: ReadOnlyRegister<u32>,

    #[register(offset = 0x24, read_fn = genbits_read)]
    genbits: ReadOnlyRegister<u32>,

    // Entropy Source registers
    #[register(offset = 0x1020)]
    module_enable: ReadWriteRegister<u32>,

    #[register(offset = 0x1024)]
    conf: ReadWriteRegister<u32>,

    #[register(offset = 0x10a4)]
    alert_summary_fail_counts: ReadOnlyRegister<u32>,

    #[register(offset = 0x10a8)]
    alert_fail_counts: ReadOnlyRegister<u32>,

    #[register(offset = 0x10d0)]
    debug_status: ReadOnlyRegister<u32>,

    cmd_req_state: CmdReqState,

    seed: Vec<u32>,

    ctr_drbg: CtrDrbg,

    words: Words,
}

impl Csrng {
    pub fn new() -> Self {
        Self {
            // TODO(rkr35): implement CTRL, CONF, and MODULE_ENABLE register logic.
            ctrl: ReadWriteRegister::new(0x999),
            cmd_req: WriteOnlyRegister::new(0),
            sw_cmd_sts: ReadOnlyRegister::new(0b01),
            genbits_vld: ReadOnlyRegister::new(0b01),
            genbits: ReadOnlyRegister::new(0),

            module_enable: ReadWriteRegister::new(0x9),
            conf: ReadWriteRegister::new(0x909099),
            alert_summary_fail_counts: ReadOnlyRegister::new(0),
            alert_fail_counts: ReadOnlyRegister::new(0),
            debug_status: ReadOnlyRegister::new(1 << 17),

            cmd_req_state: CmdReqState::ExpectNewCommand,
            seed: vec![],
            ctr_drbg: CtrDrbg::new(),
            words: Words::default(),
        }
    }

    fn cmd_req_write(&mut self, _: RvSize, data: RvData) -> Result<(), BusError> {
        // Since the CMD_REQ register can be use to initiate new commands or
        // supply words to an existing command, we need to track which "state"
        // we're in for this register and branch accordingly.
        match self.cmd_req_state {
            CmdReqState::ExpectNewCommand => self.process_new_cmd(data),

            CmdReqState::ExpectSeedWords { num_words } => {
                self.seed.push(data);
                if self.seed.len() == num_words {
                    self.ctr_drbg.instantiate(&self.seed);
                    self.seed.clear();
                    self.cmd_req_state = CmdReqState::ExpectNewCommand;
                }
            }
        }
        Ok(())
    }

    fn process_new_cmd(&mut self, data: RvData) {
        const INSTANTIATE: u32 = 1;
        const GENERATE: u32 = 3;
        const UNINSTANTIATE: u32 = 5;

        let acmd = data & 0xf;
        let clen = (data >> 4) & 0xf;
        let flag0 = (data >> 8) & 0xf;
        let glen = (data >> 12) & 0x1fff;

        match acmd {
            INSTANTIATE => {
                const FALSE: u32 = MultiBitBool::False as u32;
                const TRUE: u32 = MultiBitBool::True as u32;

                // https://opentitan.org/book/hw/ip/csrng/doc/theory_of_operation.html#command-description
                match [flag0, clen] {
                    [FALSE, 0] => {
                        // Seed from entropy_src.

                        // TODO(rkr35): Figure out a better way to pass the entropy bits the tests are using.
                        self.ctr_drbg.instantiate(&[
                            0x4B7DE947, 0x27E4ED3E, 0xF763FC5D, 0x11731D9D, 0xA08B3943, 0x71DC56AA,
                            0xF4ECBEBA, 0x10518E4B, 0xE743CC50, 0x65693560, 0xF57AD687, 0x33F63B65,
                        ]);
                    }

                    [FALSE, _] => unimplemented!("seed: entropy_src XOR constant"),

                    [TRUE, 0] => {
                        // Zero seed.
                        self.ctr_drbg.instantiate(&[])
                    }

                    [TRUE, _] => {
                        self.cmd_req_state = CmdReqState::ExpectSeedWords {
                            num_words: clen as usize,
                        };
                    }

                    _ => unreachable!("invalid INSTANTIATE state: flag0={flag0}, clen={clen}"),
                }
            }

            GENERATE => {
                self.ctr_drbg.generate(glen as usize);
            }

            UNINSTANTIATE => {
                self.ctr_drbg.uninstantiate();
            }

            _ => {
                unimplemented!("CSRNG cmd: {acmd}");
            }
        }
    }

    fn genbits_vld_read(&mut self, _: RvSize) -> Result<RvData, BusError> {
        if self.words.is_empty() {
            // Check if the CTR_DRBG has any bits for us.
            if let Some(block) = self.ctr_drbg.pop_block() {
                self.words = Words::new(block);
                Ok(0b01)
            } else {
                Ok(0b00)
            }
        } else {
            Ok(0b01)
        }
    }

    fn genbits_read(&mut self, _: RvSize) -> Result<RvData, BusError> {
        Ok(self.words.next().unwrap_or(0xCAFE_F00D))
    }
}

impl Default for Csrng {
    fn default() -> Self {
        Self::new()
    }
}

type Word = u32;
const WORD_SIZE_BYTES: usize = mem::size_of::<Word>();

#[derive(Default)]
struct Words {
    block: Block,
    cursor: usize,
}

impl Words {
    pub fn new(block: Block) -> Self {
        Self {
            block,
            cursor: block.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Iterator for Words {
    type Item = Word;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor == 0 {
            None
        } else {
            // We have to return, in reverse order, the words within this block.
            // Reverse order because of https://opentitan.org/book/hw/ip/csrng/doc/programmers_guide.html#endianness-and-known-answer-tests
            let start = self.cursor - WORD_SIZE_BYTES;
            let end = self.cursor;

            let word = &self.block[start..end];
            let word = word.try_into().expect("byte slice to 4-byte array");
            let word = u32::from_be_bytes(word);

            self.cursor = start;
            Some(word)
        }
    }
}

impl ExactSizeIterator for Words {
    fn len(&self) -> usize {
        self.cursor / WORD_SIZE_BYTES
    }
}

enum CmdReqState {
    ExpectNewCommand,
    ExpectSeedWords { num_words: usize },
}

#[repr(u32)]
enum MultiBitBool {
    False = 9,
    True = 6,
}

mod rand_impl {
    // Informal, unverified implementation of CTR_DRGB AES-256
    // Section 10.2 (page 48) of https://doi.org/10.6028/NIST.SP.800-90Ar1

    use std::iter;

    use super::WORD_SIZE_BYTES;

    // Table 3 of Section 10.2.1 (page 49).
    const BLOCK_LEN_BYTES: usize = 128 / 8;
    const KEY_LEN_BYTES: usize = 256 / 8;
    const SEED_LEN_BYTES: usize = BLOCK_LEN_BYTES + KEY_LEN_BYTES;

    pub type Block = [u8; BLOCK_LEN_BYTES];
    type Key = [u8; KEY_LEN_BYTES];
    type Seed = [u8; SEED_LEN_BYTES];

    pub struct CtrDrbg {
        v: Block,
        key: Key,
        generated_bytes: Vec<Block>,
    }

    impl CtrDrbg {
        pub fn new() -> Self {
            Self {
                v: [0; BLOCK_LEN_BYTES],
                key: [0; KEY_LEN_BYTES],
                generated_bytes: vec![],
            }
        }

        fn update(&mut self, provided_data: Seed) {
            // Section 10.2.1.2 (page 51).
            let mut temp = [0_u8; SEED_LEN_BYTES];

            for chunk in temp.chunks_exact_mut(BLOCK_LEN_BYTES) {
                block_increment(&mut self.v);
                let output_block = block_encrypt(&self.key, &self.v);
                chunk.copy_from_slice(output_block.as_slice());
            }

            for (t, d) in iter::zip(&mut temp, &provided_data) {
                *t ^= d;
            }

            self.key.copy_from_slice(&temp[..KEY_LEN_BYTES]);
            self.v.copy_from_slice(&temp[KEY_LEN_BYTES..]);
        }

        pub fn instantiate(&mut self, seed: &[u32]) {
            // Section 10.2.1.3 (page 52).
            let seed_material = massage_seed(seed);
            self.key = [0; KEY_LEN_BYTES];
            self.v = [0; BLOCK_LEN_BYTES];
            self.update(seed_material);
        }

        pub fn generate(&mut self, num_128_bit_blocks: usize) {
            // Section 10.2.1.5 (page 55).

            let additional_input = [0; SEED_LEN_BYTES];
            self.generated_bytes.clear();

            for _ in 0..num_128_bit_blocks {
                block_increment(&mut self.v);
                let output_block = block_encrypt(&self.key, &self.v);
                self.generated_bytes.push(output_block);
            }

            // https://opentitan.org/book/hw/ip/csrng/doc/programmers_guide.html#endianness-and-known-answer-tests
            self.generated_bytes.reverse();

            self.update(additional_input);
        }

        pub fn uninstantiate(&mut self) {
            self.v = [0; BLOCK_LEN_BYTES];
            self.key = [0; KEY_LEN_BYTES];
            self.generated_bytes.clear();
        }

        pub fn pop_block(&mut self) -> Option<Block> {
            self.generated_bytes.pop()
        }

        #[allow(dead_code)]
        fn print_state(&self) {
            println!("key={:x?}\nv={:x?}", self.key, self.v);
        }
    }

    fn block_increment(block: &mut Block) {
        for byte in block.iter_mut().rev() {
            if *byte == u8::MAX {
                *byte = 0;
            } else {
                *byte += 1;
                break;
            }
        }
    }

    fn block_encrypt(key: &Key, block: &Block) -> Block {
        use aes::cipher::generic_array::GenericArray;
        use aes::cipher::{BlockEncrypt, KeyInit};
        use aes::Aes256Enc;

        let cipher = Aes256Enc::new_from_slice(key).expect("construct AES-256");
        let mut output_block = GenericArray::clone_from_slice(block);
        cipher.encrypt_block(&mut output_block);
        output_block
            .as_slice()
            .try_into()
            .expect("block slice to block array")
    }

    fn massage_seed(input: &[u32]) -> Seed {
        // Starting from the end of `input` words, paste the bytes (big-endian)
        // of each word into `out`. This final form is what the NIST algorithm
        // expects for seeds.
        let mut out = [0; SEED_LEN_BYTES];

        for (dst, word) in iter::zip(out.chunks_exact_mut(WORD_SIZE_BYTES), input.iter().rev()) {
            dst.copy_from_slice(&word.to_be_bytes());
        }

        out
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn block_increment_zero() {
            let mut actual = [0; BLOCK_LEN_BYTES];
            block_increment(&mut actual);

            let mut expected = [0; BLOCK_LEN_BYTES];
            *expected.last_mut().unwrap() = 1;

            assert_eq!(actual, expected);
        }

        #[test]
        fn block_increment_max_first_byte() {
            let mut actual = [0; BLOCK_LEN_BYTES];
            *actual.last_mut().unwrap() = u8::MAX;

            block_increment(&mut actual);

            let mut expected = [0; BLOCK_LEN_BYTES];
            expected[expected.len() - 2] = 1;

            assert_eq!(actual, expected);
        }

        #[test]
        fn block_increment_non_zero_first_byte() {
            let mut actual = [0; BLOCK_LEN_BYTES];
            *actual.last_mut().unwrap() = 0xa0;

            block_increment(&mut actual);

            let mut expected = [0; BLOCK_LEN_BYTES];
            *expected.last_mut().unwrap() = 0xa1;

            assert_eq!(actual, expected);
        }

        #[test]
        fn block_increment_max() {
            let mut actual = [u8::MAX; BLOCK_LEN_BYTES];
            block_increment(&mut actual);

            let expected = [0; BLOCK_LEN_BYTES];

            assert_eq!(actual, expected);
        }

        #[test]
        fn massage_seed_zero_words() {
            let expected = [0; SEED_LEN_BYTES];
            assert_eq!(massage_seed(&[]), expected);
        }

        #[test]
        fn massage_seed_single_word() {
            let input = 0xA1B2C3D4_u32;
            let mut expected = [0; SEED_LEN_BYTES];
            expected[0..4].copy_from_slice(&input.to_be_bytes());
            assert_eq!(massage_seed(&[input]), expected);
        }

        #[test]
        fn massage_seed_two_words() {
            let word0 = 0xA1B2C3D4_u32;
            let word1 = 0xC1D2E3F4_u32;

            let mut expected = [0; SEED_LEN_BYTES];
            expected[0..4].copy_from_slice(&word1.to_be_bytes());
            expected[4..8].copy_from_slice(&word0.to_be_bytes());

            assert_eq!(massage_seed(&[word0, word1]), expected);
        }

        #[test]
        fn massage_seed_nist_test_vector() {
            const SEED: [u32; 12] = [
                0x73bec010, 0x9262474c, 0x16a30f76, 0x531b51de, 0x2ee494e5, 0xdfec9db3, 0xcb7a879d,
                0x5600419c, 0xca79b0b0, 0xdda33b5c, 0xa468649e, 0xdf5d73fa,
            ];

            assert_eq!(
                massage_seed(&SEED),
                *b"\xdf\x5d\x73\xfa\xa4\x68\x64\x9e\xdd\xa3\x3b\x5c\xca\x79\xb0\
                \xb0\x56\x00\x41\x9c\xcb\x7a\x87\x9d\xdf\xec\x9d\xb3\x2e\xe4\
                \x94\xe5\x53\x1b\x51\xde\x16\xa3\x0f\x76\x92\x62\x47\x4c\x73\
                \xbe\xc0\x10",
            );
        }

        #[test]
        fn ctr_drbg_nist_test_vector() {
            // https://csrc.nist.gov/CSRC/media/Projects/Cryptographic-Algorithm-Validation-Program/documents/drbg/drbgtestvectors.zip
            // Count 2 of CTR_DRBG.txt (no reseed) with the section heading:
            // [AES-256 no df]
            // [PredictionResistance = False]
            // [EntropyInputLen = 384]
            // [NonceLen = 0]
            // [PersonalizationStringLen = 0]
            // [AdditionalInputLen = 0]
            // [ReturnedBitsLen = 512]

            const ENTROPY_INPUT: [u32; 12] = [
                0x4835c677, 0xff87f32f, 0x98662f2d, 0x5592efed, 0xb4c78ead, 0x160d1ce0, 0x869dcbe2,
                0x8d038018, 0xa694bca2, 0xab7bdcd5, 0xf2f8e2c4, 0x0217a8ac,
            ];

            let mut ctr_drbg = CtrDrbg::new();

            ctr_drbg.instantiate(&ENTROPY_INPUT);
            assert_eq!(
                ctr_drbg.key,
                [
                    0x51, 0x18, 0x22, 0x57, 0x35, 0xbd, 0xd4, 0x7d, 0x02, 0x18, 0x68, 0x24, 0x62,
                    0x5f, 0xcf, 0x29, 0x43, 0xa4, 0xc0, 0x25, 0xcb, 0xfd, 0xa0, 0x8c, 0x11, 0x43,
                    0xd9, 0x33, 0x0e, 0x34, 0x13, 0xb5
                ]
            );
            assert_eq!(
                ctr_drbg.v,
                [
                    0x27, 0xf2, 0xec, 0x27, 0xaf, 0xc0, 0x05, 0x59, 0x2e, 0x25, 0x06, 0xa1, 0x3d,
                    0x33, 0xf3, 0xf9
                ]
            );

            ctr_drbg.generate(4);
            assert_eq!(
                ctr_drbg.key,
                [
                    0x89, 0x21, 0xa5, 0x8f, 0xe7, 0x4e, 0xbb, 0xaf, 0x81, 0xc0, 0xe2, 0x44, 0x1b,
                    0xf5, 0x6a, 0x11, 0x0e, 0x74, 0xbf, 0x47, 0x33, 0x9b, 0xad, 0xbf, 0x68, 0x79,
                    0x14, 0x67, 0xbf, 0x24, 0xa2, 0xc9
                ]
            );
            assert_eq!(
                ctr_drbg.v,
                [
                    0xec, 0x25, 0x56, 0x95, 0x17, 0x48, 0x09, 0xd8, 0x2b, 0xc3, 0x33, 0x99, 0x3f,
                    0xe3, 0x88, 0x56
                ]
            );

            ctr_drbg.generate(4);
            assert_eq!(
                ctr_drbg.key,
                [
                    0x29, 0xa7, 0xba, 0xbe, 0xda, 0x56, 0x1b, 0xc3, 0x0e, 0x8e, 0xaa, 0xd7, 0x07,
                    0x1e, 0xfd, 0xe5, 0x1a, 0xa6, 0x11, 0xab, 0x42, 0xe9, 0x67, 0x6a, 0xfe, 0xf6,
                    0xad, 0x25, 0x85, 0x1c, 0x4b, 0x82
                ]
            );
            assert_eq!(
                ctr_drbg.v,
                [
                    0x98, 0x1f, 0x26, 0x0a, 0x2e, 0x69, 0xd2, 0x60, 0xd0, 0xdd, 0xcd, 0x94, 0x1a,
                    0xf0, 0x35, 0xfa
                ]
            );

            assert_eq!(
                &ctr_drbg.generated_bytes,
                &[
                    [
                        0x58, 0x31, 0xc9, 0xe6, 0x4f, 0x5b, 0x64, 0x10, 0xae, 0x90, 0x8d, 0x30,
                        0x61, 0xf7, 0x6c, 0x84
                    ],
                    [
                        0x2e, 0x29, 0xf5, 0xa0, 0x93, 0x8a, 0x3b, 0xcd, 0x72, 0x2b, 0xb7, 0x18,
                        0xd0, 0x1b, 0xbf, 0xc3
                    ],
                    [
                        0xd7, 0xf3, 0xf9, 0x46, 0x8a, 0x5b, 0x24, 0x6c, 0xcd, 0xe3, 0x16, 0xd2,
                        0xab, 0x91, 0x87, 0x9c
                    ],
                    [
                        0xaa, 0x36, 0x77, 0x97, 0x26, 0xf5, 0x28, 0x75, 0x31, 0x25, 0x07, 0xfb,
                        0x08, 0x47, 0x44, 0xd4
                    ],
                ]
            );
        }
    }
}
