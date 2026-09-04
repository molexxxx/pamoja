//! The AES-128 primitive and the AES-CMAC built on it, the basis of every LoRaWAN
//! integrity check and key derivation.

use aes::cipher::{Array, BlockCipherEncrypt, KeyInit};
use aes::Aes128;

// An AES-128 key in use. Every LoRaWAN security operation reduces to encrypting 16-byte
// blocks with one of the session or root keys, which this wraps.
pub(crate) struct Cipher {
    inner: Aes128,
}

impl Cipher {
    // Prepares the cipher with a key.
    pub(crate) fn new(key: &[u8; 16]) -> Self {
        Cipher {
            inner: Aes128::new(&Array::from(*key)),
        }
    }

    // Encrypts one 16-byte block, returning the result rather than mutating in place.
    pub(crate) fn encrypt_block(&self, block: &[u8; 16]) -> [u8; 16] {
        let mut buf = Array::from(*block);
        self.inner.encrypt_block(&mut buf);
        let mut out = [0u8; 16];
        out.copy_from_slice(&buf);
        out
    }

    // Computes the full 16-byte AES-CMAC (RFC 4493) over the data. LoRaWAN uses the first
    // four bytes of this as a frame's MIC.
    pub(crate) fn cmac(&self, data: &[u8]) -> [u8; 16] {
        let (k1, k2) = self.subkeys();
        let n = data.len();
        let complete = n > 0 && n.is_multiple_of(16);
        // The blocks processed before the final one, which is handled specially.
        let leading = if n == 0 {
            0
        } else if complete {
            n / 16 - 1
        } else {
            n / 16
        };

        let mut x = [0u8; 16];
        for i in 0..leading {
            xor_into(&mut x, &data[i * 16..i * 16 + 16]);
            x = self.encrypt_block(&x);
        }

        let mut last = [0u8; 16];
        if complete {
            last.copy_from_slice(&data[leading * 16..leading * 16 + 16]);
            xor_into(&mut last, &k1);
        } else {
            let rest = &data[leading * 16..];
            last[..rest.len()].copy_from_slice(rest);
            last[rest.len()] = 0x80;
            xor_into(&mut last, &k2);
        }
        xor_into(&mut x, &last);
        self.encrypt_block(&x)
    }

    // Decrypts one 16-byte block. The spec has a network encrypt a join-accept with AES
    // decryption so a device recovers it with AES encryption, which is why the device half
    // never needs this and the network half does.
    pub(crate) fn decrypt_block(&self, block: &[u8; 16]) -> [u8; 16] {
        use aes::cipher::BlockCipherDecrypt;
        let mut buf = Array::from(*block);
        self.inner.decrypt_block(&mut buf);
        let mut out = [0u8; 16];
        out.copy_from_slice(&buf);
        out
    }

    // The two CMAC subkeys, each a GF(2^128) doubling of the cipher applied to zero.
    fn subkeys(&self) -> ([u8; 16], [u8; 16]) {
        let l = self.encrypt_block(&[0u8; 16]);
        let k1 = double(&l);
        let k2 = double(&k1);
        (k1, k2)
    }
}

// XORs `src` into `dst`, byte for byte.
fn xor_into(dst: &mut [u8; 16], src: &[u8]) {
    for (d, s) in dst.iter_mut().zip(src) {
        *d ^= *s;
    }
}

// Doubles a 128-bit value over GF(2^128): a left shift by one bit, with a conditional
// XOR of the field polynomial's constant when the top bit was set.
fn double(input: &[u8; 16]) -> [u8; 16] {
    let mut out = [0u8; 16];
    let mut carry = 0u8;
    for i in (0..16).rev() {
        out[i] = (input[i] << 1) | carry;
        carry = input[i] >> 7;
    }
    if input[0] & 0x80 != 0 {
        out[15] ^= 0x87;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes128_matches_the_fips_197_vector() {
        let key = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];
        let plaintext = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        let expected = [
            0x69, 0xc4, 0xe0, 0xd8, 0x6a, 0x7b, 0x04, 0x30, 0xd8, 0xcd, 0xb7, 0x80, 0x70, 0xb4,
            0xc5, 0x5a,
        ];
        assert_eq!(Cipher::new(&key).encrypt_block(&plaintext), expected);
    }

    // The key shared by the RFC 4493 CMAC examples.
    const RFC4493_KEY: [u8; 16] = [
        0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f,
        0x3c,
    ];

    #[test]
    fn cmac_of_the_empty_message_matches_rfc_4493() {
        let expected = [
            0xbb, 0x1d, 0x69, 0x29, 0xe9, 0x59, 0x37, 0x28, 0x7f, 0xa3, 0x7d, 0x12, 0x9b, 0x75,
            0x67, 0x46,
        ];
        assert_eq!(Cipher::new(&RFC4493_KEY).cmac(&[]), expected);
    }

    #[test]
    fn cmac_of_one_block_matches_rfc_4493() {
        let message = [
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93,
            0x17, 0x2a,
        ];
        let expected = [
            0x07, 0x0a, 0x16, 0xb4, 0x6b, 0x4d, 0x41, 0x44, 0xf7, 0x9b, 0xdd, 0x9d, 0xd0, 0x4a,
            0x28, 0x7c,
        ];
        assert_eq!(Cipher::new(&RFC4493_KEY).cmac(&message), expected);
    }

    #[test]
    fn cmac_of_a_partial_final_block_matches_rfc_4493() {
        // The RFC's 40-byte example, whose last block is incomplete and so is padded.
        let message = [
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93,
            0x17, 0x2a, 0xae, 0x2d, 0x8a, 0x57, 0x1e, 0x03, 0xac, 0x9c, 0x9e, 0xb7, 0x6f, 0xac,
            0x45, 0xaf, 0x8e, 0x51, 0x30, 0xc8, 0x1c, 0x46, 0xa3, 0x5c, 0xe4, 0x11,
        ];
        let expected = [
            0xdf, 0xa6, 0x67, 0x47, 0xde, 0x9a, 0xe6, 0x30, 0x30, 0xca, 0x32, 0x61, 0x14, 0x97,
            0xc8, 0x27,
        ];
        assert_eq!(Cipher::new(&RFC4493_KEY).cmac(&message), expected);
    }

    #[test]
    fn cmac_of_a_full_final_block_matches_rfc_4493() {
        // The RFC's 64-byte example, an exact multiple of the block size, which exercises
        // the complete-final-block path of the algorithm.
        let message = [
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93,
            0x17, 0x2a, 0xae, 0x2d, 0x8a, 0x57, 0x1e, 0x03, 0xac, 0x9c, 0x9e, 0xb7, 0x6f, 0xac,
            0x45, 0xaf, 0x8e, 0x51, 0x30, 0xc8, 0x1c, 0x46, 0xa3, 0x5c, 0xe4, 0x11, 0xe5, 0xfb,
            0xc1, 0x19, 0x1a, 0x0a, 0x52, 0xef, 0xf6, 0x9f, 0x24, 0x45, 0xdf, 0x4f, 0x9b, 0x17,
            0xad, 0x2b, 0x41, 0x7b, 0xe6, 0x6c, 0x37, 0x10,
        ];
        let expected = [
            0x51, 0xf0, 0xbe, 0xbf, 0x7e, 0x3b, 0x9d, 0x92, 0xfc, 0x49, 0x74, 0x17, 0x79, 0x36,
            0x3c, 0xfe,
        ];
        assert_eq!(Cipher::new(&RFC4493_KEY).cmac(&message), expected);
    }
}
