//! The acknowledgment a relay answers a wake-on-radio frame with, TS011-1.0.1 section 6.

use super::keys::WorKeys;
use super::wor::Carrier;
use super::WOR_ACK_LEN;
use crate::crypto::Cipher;
use crate::LorawanError;

/// How often a relay scans a channel for a wake-on-radio preamble, TS011-1.0.1 table 18.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CadPeriodicity {
    /// Once a second, the default.
    Ms1000,
    /// Every 500 milliseconds.
    Ms500,
    /// Every 250 milliseconds.
    Ms250,
    /// Every 100 milliseconds.
    Ms100,
    /// Every 50 milliseconds.
    Ms50,
    /// Every 20 milliseconds.
    Ms20,
}

impl CadPeriodicity {
    /// Reads a coded periodicity.
    ///
    /// # Arguments
    ///
    /// * `code` - the value a frame or command carries, 0 through 5.
    ///
    /// # Returns
    ///
    /// The periodicity, or `None` for 6 and 7, which are reserved.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<CadPeriodicity> {
        Some(match code {
            0 => CadPeriodicity::Ms1000,
            1 => CadPeriodicity::Ms500,
            2 => CadPeriodicity::Ms250,
            3 => CadPeriodicity::Ms100,
            4 => CadPeriodicity::Ms50,
            5 => CadPeriodicity::Ms20,
            _ => return None,
        })
    }

    /// Returns the value a frame or command carries.
    ///
    /// # Returns
    ///
    /// The code, 0 through 5.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Returns the time between two scans of one channel.
    ///
    /// # Returns
    ///
    /// The period in microseconds.
    #[must_use]
    pub const fn period_us(self) -> u64 {
        match self {
            CadPeriodicity::Ms1000 => 1_000_000,
            CadPeriodicity::Ms500 => 500_000,
            CadPeriodicity::Ms250 => 250_000,
            CadPeriodicity::Ms100 => 100_000,
            CadPeriodicity::Ms50 => 50_000,
            CadPeriodicity::Ms20 => 20_000,
        }
    }
}

/// How many symbols a relay takes from detecting activity to receiving, TS011-1.0.1
/// table 15.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CadToRx {
    /// Two symbols.
    Symbols2,
    /// Four symbols.
    Symbols4,
    /// Six symbols.
    Symbols6,
    /// Eight symbols, which an end device assumes before it has heard from a relay.
    Symbols8,
}

impl CadToRx {
    /// Reads a coded value, 0 through 3.
    ///
    /// # Arguments
    ///
    /// * `code` - the two bits a WOR ACK carries.
    ///
    /// # Returns
    ///
    /// The value; only the low two bits are read.
    #[must_use]
    pub const fn from_code(code: u8) -> CadToRx {
        match code & 0x03 {
            0 => CadToRx::Symbols2,
            1 => CadToRx::Symbols4,
            2 => CadToRx::Symbols6,
            _ => CadToRx::Symbols8,
        }
    }

    /// Returns the value a WOR ACK carries.
    ///
    /// # Returns
    ///
    /// The code, 0 through 3.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Returns the number of symbols.
    ///
    /// # Returns
    ///
    /// 2, 4, 6 or 8.
    #[must_use]
    pub const fn symbols(self) -> u16 {
        (self as u16 + 1) * 2
    }
}

/// How accurate a relay's crystal is, TS011-1.0.1 table 17.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum XtalAccuracy {
    /// Better than 10 parts per million.
    Ppm10,
    /// Better than 20.
    Ppm20,
    /// Better than 30.
    Ppm30,
    /// Better than 40, which an end device assumes before it has heard from a relay.
    Ppm40,
}

impl XtalAccuracy {
    /// Reads a coded value, 0 through 3.
    ///
    /// # Arguments
    ///
    /// * `code` - the two bits a WOR ACK carries.
    ///
    /// # Returns
    ///
    /// The value; only the low two bits are read.
    #[must_use]
    pub const fn from_code(code: u8) -> XtalAccuracy {
        match code & 0x03 {
            0 => XtalAccuracy::Ppm10,
            1 => XtalAccuracy::Ppm20,
            2 => XtalAccuracy::Ppm30,
            _ => XtalAccuracy::Ppm40,
        }
    }

    /// Returns the value a WOR ACK carries.
    ///
    /// # Returns
    ///
    /// The code, 0 through 3.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Returns the accuracy.
    ///
    /// # Returns
    ///
    /// Parts per million: 10, 20, 30 or 40.
    #[must_use]
    pub const fn ppm(self) -> u32 {
        (self as u32 + 1) * 10
    }
}

/// Whether a relay will forward the uplink after a WOR frame, TS011-1.0.1 table 16.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Forward {
    /// The relay has room to forward it.
    Available,
    /// A forwarding limit is reached; try again in 30 minutes.
    RetryIn30Minutes,
    /// A forwarding limit is reached; try again in 60 minutes.
    RetryIn60Minutes,
    /// Forwarding is off.
    Disabled,
}

impl Forward {
    /// Reads a coded value, 0 through 3.
    ///
    /// # Arguments
    ///
    /// * `code` - the two bits a WOR ACK carries.
    ///
    /// # Returns
    ///
    /// The value; only the low two bits are read.
    #[must_use]
    pub const fn from_code(code: u8) -> Forward {
        match code & 0x03 {
            0 => Forward::Available,
            1 => Forward::RetryIn30Minutes,
            2 => Forward::RetryIn60Minutes,
            _ => Forward::Disabled,
        }
    }

    /// Returns the value a WOR ACK carries.
    ///
    /// # Returns
    ///
    /// The code, 0 through 3.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }
}

/// What a relay tells an end device about itself in a WOR ACK, TS011-1.0.1 table 14.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StateSync {
    /// How long the relay takes from detecting activity to receiving.
    pub cad_to_rx: CadToRx,
    /// Whether it will forward the uplink.
    pub forward: Forward,
    /// The data rate the relay forwards at, which bounds the device's payload.
    pub relay_data_rate: u8,
    /// How accurate its crystal is.
    pub xtal_accuracy: XtalAccuracy,
    /// How often it scans.
    pub cad_periodicity: CadPeriodicity,
    /// The milliseconds from the start of the scan that heard the frame to the end of the
    /// frame's preamble, eleven bits wide.
    pub t_offset_ms: u16,
}

impl StateSync {
    /// The largest offset the eleven-bit field holds, in milliseconds.
    pub const MAX_T_OFFSET_MS: u16 = 0x07ff;

    /// Packs the state into the 24 bits a WOR ACK carries.
    ///
    /// # Returns
    ///
    /// The bits: CadToRx 23:22, Forward 21:20, RelayDataRate 19:16, XTALAccuracy 15:14,
    /// CADPeriodicity 13:11 and TOffset 10:0.
    ///
    /// # Errors
    ///
    /// Returns [`LorawanError::MalformedFrame`] for a data rate past 15 or an offset past
    /// [`MAX_T_OFFSET_MS`](Self::MAX_T_OFFSET_MS).
    pub fn to_bits(&self) -> Result<u32, LorawanError> {
        if self.relay_data_rate > 0x0f || self.t_offset_ms > Self::MAX_T_OFFSET_MS {
            return Err(LorawanError::MalformedFrame);
        }
        Ok((u32::from(self.cad_to_rx.code()) << 22)
            | (u32::from(self.forward.code()) << 20)
            | (u32::from(self.relay_data_rate) << 16)
            | (u32::from(self.xtal_accuracy.code()) << 14)
            | (u32::from(self.cad_periodicity.code()) << 11)
            | u32::from(self.t_offset_ms))
    }

    /// Unpacks the 24 bits a WOR ACK carries.
    ///
    /// # Arguments
    ///
    /// * `bits` - the state; bits past the 24th are ignored.
    ///
    /// # Returns
    ///
    /// The state.
    ///
    /// # Errors
    ///
    /// Returns [`LorawanError::MalformedFrame`] for a reserved scan periodicity.
    pub fn from_bits(bits: u32) -> Result<StateSync, LorawanError> {
        Ok(StateSync {
            cad_to_rx: CadToRx::from_code((bits >> 22) as u8),
            forward: Forward::from_code((bits >> 20) as u8),
            relay_data_rate: ((bits >> 16) & 0x0f) as u8,
            xtal_accuracy: XtalAccuracy::from_code((bits >> 14) as u8),
            cad_periodicity: CadPeriodicity::from_code(((bits >> 11) & 0x07) as u8)
                .ok_or(LorawanError::MalformedFrame)?,
            t_offset_ms: (bits & 0x07ff) as u16,
        })
    }
}

/// Builds a relay's WOR ACK, TS011-1.0.1 section 6.2.
///
/// # Arguments
///
/// * `keys` - the end device's keys.
/// * `dev_addr` - its address.
/// * `wfcnt` - the 32-bit counter of the WOR frame being acknowledged.
/// * `ack` - the carrier the acknowledgment goes out on, which its encryption folds in.
/// * `uplink` - the carrier the WOR frame said the uplink follows on, which its integrity
///   code folds in.
/// * `state` - what the relay tells the device.
///
/// # Returns
///
/// The acknowledgment.
///
/// # Errors
///
/// Returns [`LorawanError::MalformedFrame`] for a state the bits cannot carry or a carrier
/// the fields cannot.
pub fn wor_ack(
    keys: &WorKeys,
    dev_addr: u32,
    wfcnt: u32,
    ack: Carrier,
    uplink: Carrier,
    state: StateSync,
) -> Result<[u8; WOR_ACK_LEN], LorawanError> {
    let plain = state.to_bits()?.to_le_bytes();
    let stream = ack_stream(keys, dev_addr, wfcnt, ack)?;
    let mut frame = [0u8; WOR_ACK_LEN];
    for at in 0..3 {
        frame[at] = plain[at] ^ stream[at];
    }
    let mic = ack_mic(
        keys,
        dev_addr,
        wfcnt,
        uplink,
        [frame[0], frame[1], frame[2]],
    )?;
    frame[3..7].copy_from_slice(&mic);
    Ok(frame)
}

/// Checks and reads a WOR ACK, TS011-1.0.1 section 6.2.
///
/// # Arguments
///
/// * `bytes` - the acknowledgment the end device received.
/// * `keys` - the device's keys.
/// * `dev_addr` - its address.
/// * `wfcnt` - the 32-bit counter of the WOR frame it sent.
/// * `ack` - the carrier the acknowledgment arrived on.
/// * `uplink` - the carrier the WOR frame named for the uplink.
///
/// # Returns
///
/// What the relay said about itself.
///
/// # Errors
///
/// Returns [`LorawanError::MalformedFrame`] for a frame that is not seven bytes or a reserved
/// periodicity, and [`LorawanError::MicMismatch`] when its integrity code does not verify.
pub fn open_wor_ack(
    bytes: &[u8],
    keys: &WorKeys,
    dev_addr: u32,
    wfcnt: u32,
    ack: Carrier,
    uplink: Carrier,
) -> Result<StateSync, LorawanError> {
    if bytes.len() != WOR_ACK_LEN {
        return Err(LorawanError::MalformedFrame);
    }
    let encrypted = [bytes[0], bytes[1], bytes[2]];
    if ack_mic(keys, dev_addr, wfcnt, uplink, encrypted)? != bytes[3..7] {
        return Err(LorawanError::MicMismatch);
    }
    let stream = ack_stream(keys, dev_addr, wfcnt, ack)?;
    let bits = u32::from_le_bytes([
        encrypted[0] ^ stream[0],
        encrypted[1] ^ stream[1],
        encrypted[2] ^ stream[2],
        0,
    ]);
    StateSync::from_bits(bits)
}

/// The block `SACK` an acknowledgment is XORed with: `aes128_encrypt(WorSEncKey, AACK)`,
/// table 19.
fn ack_stream(
    keys: &WorKeys,
    dev_addr: u32,
    wfcnt: u32,
    ack: Carrier,
) -> Result<[u8; 16], LorawanError> {
    let carrier = ack.encode()?;
    let mut block = [0u8; 16];
    block[0] = 0x01;
    block[3] = 0x01;
    block[4..8].copy_from_slice(&dev_addr.to_le_bytes());
    block[8..12].copy_from_slice(&wfcnt.to_le_bytes());
    block[12..15].copy_from_slice(&carrier[1..4]);
    block[15] = carrier[0];
    Ok(Cipher::new(keys.encryption()).encrypt_block(&block))
}

/// The integrity code of an acknowledgment: CMAC over
/// `B0 | AckUplinkEnc | WOR | pad16`, tables 20 and 21.
fn ack_mic(
    keys: &WorKeys,
    dev_addr: u32,
    wfcnt: u32,
    uplink: Carrier,
    encrypted: [u8; 3],
) -> Result<[u8; 4], LorawanError> {
    let carrier = uplink.encode()?;
    let mut message = [0u8; 32];
    message[0] = 0x49;
    message[5] = 0x01;
    message[6..10].copy_from_slice(&dev_addr.to_le_bytes());
    message[10..14].copy_from_slice(&wfcnt.to_le_bytes());
    message[15] = 0x07;
    message[16..19].copy_from_slice(&encrypted);
    message[19..23].copy_from_slice(&carrier);
    message[23..25].copy_from_slice(&(wfcnt as u16).to_le_bytes());
    message[25..29].copy_from_slice(&dev_addr.to_le_bytes());
    let cmac = Cipher::new(keys.integrity()).cmac(&message);
    Ok([cmac[0], cmac[1], cmac[2], cmac[3]])
}
