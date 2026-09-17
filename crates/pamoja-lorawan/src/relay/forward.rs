//! An uplink a relay forwards to its network, TS011-1.0.1 section 9.1.

use crate::mac::{relay_rssi_code, relay_snr_code};
use crate::LorawanError;

/// The bytes `ForwardUplinkReq` adds in front of the end device's frame: three of metadata
/// and three of frequency.
pub const FORWARD_OVERHEAD: usize = 6;

/// Which of a relay's channels a WOR frame arrived on, TS011-1.0.1 table 28.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorChannel {
    /// The default channel.
    Default,
    /// The second channel a network configured.
    Second,
}

/// What a relay heard of an uplink, TS011-1.0.1 table 27.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UplinkMetadata {
    /// The channel the WOR frame came in on.
    pub wor_channel: WorChannel,
    /// The uplink's signal strength in dBm, carried from -142 to -15.
    pub rssi_dbm: i16,
    /// Its signal-to-noise ratio in dB, carried from -20 to 11.
    pub snr_db: i8,
    /// The data rate the uplink arrived at.
    pub data_rate: u8,
}

/// An end device's uplink as a relay forwards it: the payload of a relay uplink on
/// [`LA_FPORT_RELAY`](super::LA_FPORT_RELAY).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForwardedUplink<'a> {
    /// What the relay heard of it.
    pub metadata: UplinkMetadata,
    /// The frequency it arrived on, in hertz.
    pub frequency_hz: u32,
    /// The end device's frame, as the relay received it.
    pub phy_payload: &'a [u8],
}

impl<'a> ForwardedUplink<'a> {
    /// Writes the forwarded uplink out.
    ///
    /// A strength or ratio past what its field carries goes out as the closest value, as
    /// section 9.1 asks.
    ///
    /// # Arguments
    ///
    /// * `out` - where to write it.
    ///
    /// # Returns
    ///
    /// How many bytes were written: [`FORWARD_OVERHEAD`] and the frame.
    ///
    /// # Errors
    ///
    /// Returns [`LorawanError::PayloadTooLong`] when `out` is too short, and
    /// [`LorawanError::MalformedFrame`] for a data rate past 15 or a frequency the three-byte
    /// field cannot carry.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::relay::{ForwardedUplink, UplinkMetadata, WorChannel};
    ///
    /// let forwarded = ForwardedUplink {
    ///     metadata: UplinkMetadata {
    ///         wor_channel: WorChannel::Second,
    ///         rssi_dbm: -100,
    ///         snr_db: 5,
    ///         data_rate: 5,
    ///     },
    ///     frequency_hz: 868_100_000,
    ///     phy_payload: &[0x40, 0x01, 0x02],
    /// };
    /// let mut out = [0u8; 16];
    /// let len = forwarded.encode(&mut out)?;
    /// assert_eq!(&out[..len], &[0x95, 0xAB, 0x01, 0x28, 0x76, 0x84, 0x40, 0x01, 0x02]);
    /// assert_eq!(ForwardedUplink::parse(&out[..len])?, forwarded);
    /// # Ok::<(), pamoja_lorawan::LorawanError>(())
    /// ```
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, LorawanError> {
        let len = FORWARD_OVERHEAD + self.phy_payload.len();
        if out.len() < len {
            return Err(LorawanError::PayloadTooLong);
        }
        if self.metadata.data_rate > 0x0f || !self.frequency_hz.is_multiple_of(100) {
            return Err(LorawanError::MalformedFrame);
        }
        let steps = self.frequency_hz / 100;
        if steps > 0x00ff_ffff {
            return Err(LorawanError::MalformedFrame);
        }
        let channel = match self.metadata.wor_channel {
            WorChannel::Default => 0u32,
            WorChannel::Second => 1,
        };
        let metadata = (channel << 16)
            | (u32::from(relay_rssi_code(self.metadata.rssi_dbm)) << 9)
            | (u32::from(relay_snr_code(self.metadata.snr_db)) << 4)
            | u32::from(self.metadata.data_rate);
        out[..3].copy_from_slice(&metadata.to_le_bytes()[..3]);
        out[3..6].copy_from_slice(&steps.to_le_bytes()[..3]);
        out[FORWARD_OVERHEAD..len].copy_from_slice(self.phy_payload);
        Ok(len)
    }

    /// Reads a forwarded uplink.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the payload of a relay uplink on port 226.
    ///
    /// # Returns
    ///
    /// The forwarded uplink, borrowing the end device's frame from `bytes`.
    ///
    /// # Errors
    ///
    /// Returns [`LorawanError::FrameTooShort`] for fewer than [`FORWARD_OVERHEAD`] bytes, and
    /// [`LorawanError::MalformedFrame`] for a reserved WOR channel.
    pub fn parse(bytes: &'a [u8]) -> Result<ForwardedUplink<'a>, LorawanError> {
        if bytes.len() < FORWARD_OVERHEAD {
            return Err(LorawanError::FrameTooShort);
        }
        let metadata = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0]);
        let wor_channel = match (metadata >> 16) & 0x03 {
            0 => WorChannel::Default,
            1 => WorChannel::Second,
            _ => return Err(LorawanError::MalformedFrame),
        };
        Ok(ForwardedUplink {
            metadata: UplinkMetadata {
                wor_channel,
                rssi_dbm: -i16::from(((metadata >> 9) & 0x7f) as u8) - 15,
                snr_db: (((metadata >> 4) & 0x1f) as i8) - 20,
                data_rate: (metadata & 0x0f) as u8,
            },
            frequency_hz: u32::from_le_bytes([bytes[3], bytes[4], bytes[5], 0]) * 100,
            phy_payload: &bytes[FORWARD_OVERHEAD..],
        })
    }
}
