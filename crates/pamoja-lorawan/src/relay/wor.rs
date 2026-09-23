//! The wake-on-radio frames an end device sends ahead of an uplink, TS011-1.0.1 section 5.3.

use super::keys::WorKeys;
use super::{WOR_JOIN_REQUEST_LEN, WOR_UPLINK_LEN};
use crate::crypto::Cipher;
use crate::LorawanError;

const TYPE_JOIN_REQUEST: u8 = 0;
const TYPE_UPLINK: u8 = 1;

/// Where a frame goes and how fast: a frequency and a data rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Carrier {
    /// The frequency in hertz, which a frame carries in hundreds of hertz.
    pub frequency_hz: u32,
    /// The data rate, which a frame carries in four bits.
    pub data_rate: u8,
}

impl Carrier {
    /// Names a carrier.
    ///
    /// # Arguments
    ///
    /// * `frequency_hz` - the frequency in hertz.
    /// * `data_rate` - the data rate.
    ///
    /// # Returns
    ///
    /// The carrier.
    #[must_use]
    pub const fn new(frequency_hz: u32, data_rate: u8) -> Carrier {
        Carrier {
            frequency_hz,
            data_rate,
        }
    }

    /// The data rate byte and the three frequency bytes, as every relay frame carries them.
    pub(crate) fn encode(self) -> Result<[u8; 4], LorawanError> {
        if self.data_rate > 0x0f || !self.frequency_hz.is_multiple_of(100) {
            return Err(LorawanError::MalformedFrame);
        }
        let steps = self.frequency_hz / 100;
        if steps > 0x00ff_ffff {
            return Err(LorawanError::MalformedFrame);
        }
        let low = steps.to_le_bytes();
        Ok([self.data_rate, low[0], low[1], low[2]])
    }

    /// Reads the data rate byte, whose top four bits are reserved, and a frequency.
    pub(crate) fn decode(bytes: [u8; 4]) -> Carrier {
        Carrier {
            data_rate: bytes[0] & 0x0f,
            frequency_hz: u32::from_le_bytes([bytes[1], bytes[2], bytes[3], 0]) * 100,
        }
    }
}

/// A wake-on-radio frame, as a relay reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wor {
    /// Ahead of a join request. Nothing protects it, since the device has no session yet.
    JoinRequest {
        /// Where and how fast the join request follows.
        uplink: Carrier,
    },
    /// Ahead of a Class A uplink, sealed with the device's [`WorKeys`].
    Uplink(SealedWor),
}

impl Wor {
    /// Reads a wake-on-radio frame.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the frame the relay received.
    ///
    /// # Returns
    ///
    /// The frame. An uplink's is still sealed; [`SealedWor::open`] checks and decrypts it.
    ///
    /// # Errors
    ///
    /// Returns [`LorawanError::MalformedFrame`] for a reserved or proprietary WOR type, which
    /// TS011-1.0.1 section 5.3 has a relay discard, and for a frame whose length is not its
    /// type's.
    pub fn parse(bytes: &[u8]) -> Result<Wor, LorawanError> {
        let header = *bytes.first().ok_or(LorawanError::FrameTooShort)?;
        match header & 0x0f {
            TYPE_JOIN_REQUEST if bytes.len() == WOR_JOIN_REQUEST_LEN => Ok(Wor::JoinRequest {
                uplink: Carrier::decode([bytes[1], bytes[2], bytes[3], bytes[4]]),
            }),
            TYPE_UPLINK if bytes.len() == WOR_UPLINK_LEN => Ok(Wor::Uplink(SealedWor {
                dev_addr: u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]),
                encrypted: [bytes[5], bytes[6], bytes[7], bytes[8]],
                wfcnt: u16::from_le_bytes([bytes[9], bytes[10]]),
                mic: [bytes[11], bytes[12], bytes[13], bytes[14]],
            })),
            _ => Err(LorawanError::MalformedFrame),
        }
    }
}

/// A wake-on-radio frame ahead of a Class A uplink, before its integrity code is checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SealedWor {
    dev_addr: u32,
    encrypted: [u8; 4],
    wfcnt: u16,
    mic: [u8; 4],
}

impl SealedWor {
    /// Returns the address the frame names, which picks the keys to open it with.
    ///
    /// # Returns
    ///
    /// The address.
    #[must_use]
    pub const fn dev_addr(&self) -> u32 {
        self.dev_addr
    }

    /// Returns the low sixteen bits of the frame counter the frame carries.
    ///
    /// # Returns
    ///
    /// The counter's low bits, which a relay extends to the 32-bit counter it expects.
    #[must_use]
    pub const fn wfcnt(&self) -> u16 {
        self.wfcnt
    }

    /// Checks the frame's integrity code and decrypts where the uplink will follow.
    ///
    /// # Arguments
    ///
    /// * `keys` - the device's keys.
    /// * `wfcnt` - the full 32-bit frame counter the relay takes the frame to carry.
    /// * `wor` - the carrier the frame arrived on, which the encryption folds in.
    ///
    /// # Returns
    ///
    /// Where and how fast the uplink follows.
    ///
    /// # Errors
    ///
    /// Returns [`LorawanError::FcntMismatch`] for a counter whose low bits are not the
    /// frame's, and [`LorawanError::MicMismatch`] when the integrity code does not verify.
    pub fn open(&self, keys: &WorKeys, wfcnt: u32, wor: Carrier) -> Result<Carrier, LorawanError> {
        if wfcnt as u16 != self.wfcnt {
            return Err(LorawanError::FcntMismatch);
        }
        if uplink_mic(keys, self.dev_addr, wfcnt, &self.encrypted) != self.mic {
            return Err(LorawanError::MicMismatch);
        }
        let stream = uplink_stream(keys, self.dev_addr, wfcnt, wor)?;
        let mut plain = [0u8; 4];
        for (at, byte) in plain.iter_mut().enumerate() {
            *byte = self.encrypted[at] ^ stream[at];
        }
        Ok(Carrier::decode(plain))
    }
}

/// Builds the wake-on-radio frame ahead of a join request, TS011-1.0.1 section 5.3.1.
///
/// # Arguments
///
/// * `uplink` - where and how fast the join request follows.
///
/// # Returns
///
/// The frame.
///
/// # Errors
///
/// Returns [`LorawanError::MalformedFrame`] for a data rate past 15 or a frequency the
/// three-byte field cannot carry.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::relay::{wor_join_request, Carrier, Wor};
///
/// // A device wakes its relay to say a join request follows on 868.1 MHz at DR5.
/// let uplink = Carrier::new(868_100_000, 5);
/// let frame = wor_join_request(uplink)?;
/// assert_eq!(frame.len(), 5, "a kind, a data rate, and three bytes of frequency");
///
/// // The relay reads where to listen for it.
/// assert_eq!(Wor::parse(&frame)?, Wor::JoinRequest { uplink });
/// # Ok::<(), pamoja_lorawan::LorawanError>(())
/// ```
pub fn wor_join_request(uplink: Carrier) -> Result<[u8; WOR_JOIN_REQUEST_LEN], LorawanError> {
    let carrier = uplink.encode()?;
    Ok([
        TYPE_JOIN_REQUEST,
        carrier[0],
        carrier[1],
        carrier[2],
        carrier[3],
    ])
}

/// Builds the wake-on-radio frame ahead of a Class A uplink, TS011-1.0.1 section 5.3.2.
///
/// The uplink's carrier is encrypted under `WorSEncKey` with a block that folds in the
/// frame's own carrier and counter, and the frame is signed with `WorSIntKey`.
///
/// # Arguments
///
/// * `keys` - the device's keys.
/// * `dev_addr` - the device's address.
/// * `wfcnt` - the WOR frame counter, which the device raises for every WOR frame it sends.
/// * `uplink` - where and how fast the uplink follows.
/// * `wor` - the carrier this WOR frame goes out on.
///
/// # Returns
///
/// The frame.
///
/// # Errors
///
/// Returns [`LorawanError::MalformedFrame`] for a data rate past 15 or a frequency the
/// three-byte field cannot carry, in either carrier.
pub fn wor_uplink(
    keys: &WorKeys,
    dev_addr: u32,
    wfcnt: u32,
    uplink: Carrier,
    wor: Carrier,
) -> Result<[u8; WOR_UPLINK_LEN], LorawanError> {
    let plain = uplink.encode()?;
    let stream = uplink_stream(keys, dev_addr, wfcnt, wor)?;
    let mut encrypted = [0u8; 4];
    for (at, byte) in encrypted.iter_mut().enumerate() {
        *byte = plain[at] ^ stream[at];
    }
    let mic = uplink_mic(keys, dev_addr, wfcnt, &encrypted);

    let mut frame = [0u8; WOR_UPLINK_LEN];
    frame[0] = TYPE_UPLINK;
    frame[1..5].copy_from_slice(&dev_addr.to_le_bytes());
    frame[5..9].copy_from_slice(&encrypted);
    frame[9..11].copy_from_slice(&(wfcnt as u16).to_le_bytes());
    frame[11..15].copy_from_slice(&mic);
    Ok(frame)
}

/// The block `SWOR` a WOR uplink's payload is XORed with: `aes128_encrypt(WorSEncKey, AWOR)`,
/// table 10.
fn uplink_stream(
    keys: &WorKeys,
    dev_addr: u32,
    wfcnt: u32,
    wor: Carrier,
) -> Result<[u8; 16], LorawanError> {
    let carrier = wor.encode()?;
    let mut block = [0u8; 16];
    block[0] = 0x01;
    block[4..8].copy_from_slice(&dev_addr.to_le_bytes());
    block[8..12].copy_from_slice(&wfcnt.to_le_bytes());
    block[12..15].copy_from_slice(&carrier[1..4]);
    block[15] = carrier[0];
    Ok(Cipher::new(keys.encryption()).encrypt_block(&block))
}

/// The integrity code of a WOR uplink: CMAC over `B0 | DevAddr | WorUplinkEnc | WFCnt`,
/// table 11.
fn uplink_mic(keys: &WorKeys, dev_addr: u32, wfcnt: u32, encrypted: &[u8; 4]) -> [u8; 4] {
    let mut message = [0u8; 26];
    message[0] = 0x49;
    message[6..10].copy_from_slice(&dev_addr.to_le_bytes());
    message[10..14].copy_from_slice(&wfcnt.to_le_bytes());
    message[15] = 0x0e;
    message[16..20].copy_from_slice(&dev_addr.to_le_bytes());
    message[20..24].copy_from_slice(encrypted);
    message[24..26].copy_from_slice(&(wfcnt as u16).to_le_bytes());
    let cmac = Cipher::new(keys.integrity()).cmac(&message);
    [cmac[0], cmac[1], cmac[2], cmac[3]]
}
