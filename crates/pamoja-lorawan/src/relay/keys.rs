//! The keys a relay and an end device protect wake-on-radio frames with, TS011-1.0.1
//! section 4.

use crate::crypto::Cipher;

/// Derives the root relay session key of an end device, TS011-1.0.1 section 4.4.
///
/// `RootWorSKey = aes128_encrypt(Key, 0x01 | pad16)`, where the key is the network session
/// key: `NwkSKey` under LoRaWAN 1.0.x and `NwkSEncKey` from 1.1. An over-the-air device
/// derives it after each join, and a network sends it to a relay in `UpdateUplinkListReq`.
///
/// # Arguments
///
/// * `network_key` - the end device's network session key.
///
/// # Returns
///
/// The root relay session key.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::relay::root_wor_s_key;
///
/// // The vector The Things Stack tests its derivation with.
/// let nwk_s_enc_key = [
///     0xCE, 0x07, 0xA0, 0x09, 0xA3, 0x97, 0x0A, 0xC0, 0x51, 0x9A, 0x09, 0x9E, 0xD5, 0x3E, 0x55,
///     0x0B,
/// ];
/// assert_eq!(
///     root_wor_s_key(&nwk_s_enc_key),
///     [
///         0xEE, 0x91, 0xDC, 0x1A, 0x66, 0x66, 0xC0, 0x6E, 0x82, 0x77, 0xDE, 0x6D, 0xB4, 0xDB,
///         0x94, 0x5F
///     ]
/// );
/// ```
#[must_use]
pub fn root_wor_s_key(network_key: &[u8; 16]) -> [u8; 16] {
    let mut block = [0u8; 16];
    block[0] = 0x01;
    Cipher::new(network_key).encrypt_block(&block)
}

/// The integrity and encryption keys of one end device's wake-on-radio frames.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct WorKeys {
    integrity: [u8; 16],
    encryption: [u8; 16],
}

impl WorKeys {
    /// Derives an end device's keys from its root relay session key, TS011-1.0.1
    /// section 4.5.
    ///
    /// `WorSIntKey = aes128_encrypt(RootWorSKey, 0x01 | DevAddr | pad16)` and
    /// `WorSEncKey = aes128_encrypt(RootWorSKey, 0x02 | DevAddr | pad16)`.
    ///
    /// # Arguments
    ///
    /// * `root_wor_s_key` - the device's root relay session key.
    /// * `dev_addr` - the device's address.
    ///
    /// # Returns
    ///
    /// The two keys.
    #[must_use]
    pub fn derive(root_wor_s_key: &[u8; 16], dev_addr: u32) -> WorKeys {
        let cipher = Cipher::new(root_wor_s_key);
        let mut block = [0u8; 16];
        block[1..5].copy_from_slice(&dev_addr.to_le_bytes());
        block[0] = 0x01;
        let integrity = cipher.encrypt_block(&block);
        block[0] = 0x02;
        let encryption = cipher.encrypt_block(&block);
        WorKeys {
            integrity,
            encryption,
        }
    }

    /// Holds keys derived earlier, such as ones a relay keeps for an end device it trusts.
    ///
    /// # Arguments
    ///
    /// * `integrity` - `WorSIntKey`.
    /// * `encryption` - `WorSEncKey`.
    ///
    /// # Returns
    ///
    /// The keys.
    #[must_use]
    pub const fn new(integrity: [u8; 16], encryption: [u8; 16]) -> WorKeys {
        WorKeys {
            integrity,
            encryption,
        }
    }

    /// Returns the key a WOR frame's and a WOR ACK's integrity code is computed with,
    /// `WorSIntKey`.
    ///
    /// # Returns
    ///
    /// The key.
    #[must_use]
    pub const fn integrity(&self) -> &[u8; 16] {
        &self.integrity
    }

    /// Returns the key their payloads are encrypted with, `WorSEncKey`.
    ///
    /// # Returns
    ///
    /// The key.
    #[must_use]
    pub const fn encryption(&self) -> &[u8; 16] {
        &self.encryption
    }
}

impl core::fmt::Debug for WorKeys {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("WorKeys { .. }")
    }
}
