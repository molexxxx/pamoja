//! The commands a network and a device configure each other with.
//!
//! A frame carries a payload for the application, and it can carry commands for the device
//! itself: change to this data rate, listen on this frequency, tell me how your battery is.
//! They ride either in the frame options, unencrypted and at most [`FOPTS_MAX`] bytes of
//! them, or as a whole payload on port zero, which a session encrypts with the network key.
//! A frame that carries them in both places is one a device is told to ignore.
//!
//! Two things here are easy to get wrong, and both are quiet when you do:
//!
//! - A command identifier means different things in each direction. `0x03` going down is a
//!   request to change rate, and the same byte coming up is the answer to one. Nothing in
//!   the bytes says which, so [`MacCommand::parse`] is told the direction and there is no
//!   default.
//! - A command does not carry its own length. A reader is expected to know it, which means
//!   an identifier it does not know cannot be stepped over: everything after it is
//!   unreadable. [`MacCommands`] stops there rather than guessing, and leaves the rest in
//!   [`remaining`](MacCommands::remaining) so a caller can see what was not read.
//!
//! Every layout here is from the LoRaWAN 1.0.3 specification, section 5, and multi-byte
//! fields go out low byte first as the rest of the protocol does.

use crate::{Direction, LorawanError};

/// How many bytes of commands a frame can carry beside a payload.
///
/// Past this they have to go in a payload of their own, on port zero.
pub const FOPTS_MAX: usize = 15;

/// The longest single command, which is the one that creates a channel.
pub const MAX_COMMAND: usize = 6;

/// Checks the link, and asks how well it was heard.
pub const CID_LINK_CHECK: u8 = 0x02;

/// Sets the data rate, the power, and which channels may be used.
pub const CID_LINK_ADR: u8 = 0x03;

/// Holds a device to a share of the air.
pub const CID_DUTY_CYCLE: u8 = 0x04;

/// Sets where and how fast the second receive window listens.
pub const CID_RX_PARAM_SETUP: u8 = 0x05;

/// Asks a device for its battery and how well it is hearing.
pub const CID_DEV_STATUS: u8 = 0x06;

/// Creates or changes a channel.
pub const CID_NEW_CHANNEL: u8 = 0x07;

/// Sets how long after sending a device starts listening.
pub const CID_RX_TIMING_SETUP: u8 = 0x08;

/// Sets the power and the dwell time a region asks for.
pub const CID_TX_PARAM_SETUP: u8 = 0x09;

/// Moves the downlink frequency of a channel away from its uplink one.
pub const CID_DL_CHANNEL: u8 = 0x0a;

/// Asks the network what time it is.
pub const CID_DEVICE_TIME: u8 = 0x0d;

/// The transmit powers the dwell time command can name, in dBm, by their coded value.
///
/// The steps are uneven because they are chosen to land on the ceilings different regions
/// impose rather than to divide a range evenly.
pub const EIRP_DBM: [u8; 16] = [
    8, 10, 12, 13, 14, 16, 18, 20, 21, 24, 26, 27, 29, 30, 33, 36,
];

/// What a coded transmit power means in dBm.
///
/// # Arguments
///
/// * `code` - the coded value, 0 through 15.
///
/// # Returns
///
/// The power, or `None` for a value outside the table.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::mac::eirp_dbm;
///
/// assert_eq!(eirp_dbm(5), Some(16));
/// assert_eq!(eirp_dbm(15), Some(36));
/// assert_eq!(eirp_dbm(16), None);
/// ```
#[must_use]
pub const fn eirp_dbm(code: u8) -> Option<u8> {
    if (code as usize) < EIRP_DBM.len() {
        Some(EIRP_DBM[code as usize])
    } else {
        None
    }
}

/// The code for a transmit power, when the table has one for it exactly.
///
/// # Arguments
///
/// * `dbm` - the power.
///
/// # Returns
///
/// The coded value, or `None` when the table holds no such power. It is not rounded,
/// because rounding up would put a device over a limit a region set.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::mac::eirp_code;
///
/// assert_eq!(eirp_code(16), Some(5));
/// assert_eq!(eirp_code(17), None, "not a power the command can name");
/// ```
#[must_use]
pub const fn eirp_code(dbm: u8) -> Option<u8> {
    let mut at = 0;
    while at < EIRP_DBM.len() {
        if EIRP_DBM[at] == dbm {
            return Some(at as u8);
        }
        at += 1;
    }
    None
}

/// How long a device waits before its first receive window, in seconds.
///
/// # Arguments
///
/// * `coded` - the value the command carries, 0 through 15.
///
/// # Returns
///
/// The delay. Zero and one both mean one second, which is the one place the coding is not
/// the number itself.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::mac::receive_delay_s;
///
/// assert_eq!(receive_delay_s(0), 1, "zero is one second, not none");
/// assert_eq!(receive_delay_s(1), 1);
/// assert_eq!(receive_delay_s(5), 5);
/// ```
#[must_use]
pub const fn receive_delay_s(coded: u8) -> u8 {
    if coded == 0 {
        1
    } else {
        coded
    }
}

/// One command, in either direction.
///
/// The name says which way it travels: a request goes to a device, an answer comes back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MacCommand {
    /// A device asking whether the network can hear it.
    LinkCheckReq,
    /// How well it was heard, and by how many gateways.
    LinkCheckAns {
        /// How far above the floor the last check arrived, in dB. 255 is reserved.
        margin: u8,
        /// How many gateways heard it.
        gateways: u8,
    },
    /// Change rate, power, and which channels may be used.
    LinkAdrReq {
        /// The rate, which each region codes for itself.
        data_rate: u8,
        /// The power, as a ceiling rather than an instruction.
        tx_power: u8,
        /// Which channels may carry an uplink, lowest channel in the lowest bit.
        channel_mask: u16,
        /// Which block of sixteen channels the mask applies to.
        mask_control: u8,
        /// How many times to send an unconfirmed uplink. Zero means the device keeps one.
        transmissions: u8,
    },
    /// Which parts of the request a device took.
    LinkAdrAns {
        /// Whether the power was set.
        power_ack: bool,
        /// Whether the rate was set.
        data_rate_ack: bool,
        /// Whether the channel mask was usable.
        channel_mask_ack: bool,
    },
    /// Hold the device to a share of the air.
    DutyCycleReq {
        /// The share is one over two to this, so 0 is no limit beyond the regional one.
        max_duty_cycle: u8,
    },
    /// The device took it.
    DutyCycleAns,
    /// Set where and how fast the second receive window listens.
    RxParamSetupReq {
        /// How far the first window's rate sits below the uplink rate.
        rx1_offset: u8,
        /// The rate of the second window.
        rx2_data_rate: u8,
        /// The frequency of the second window, in hertz.
        frequency_hz: u32,
    },
    /// Which parts of that the device took.
    RxParamSetupAns {
        /// Whether the offset was in range.
        rx1_offset_ack: bool,
        /// Whether the rate was known.
        rx2_data_rate_ack: bool,
        /// Whether the frequency was usable.
        channel_ack: bool,
    },
    /// Ask a device how it is.
    DevStatusReq,
    /// How it is.
    DevStatusAns {
        /// 0 on external power, 1 through 254 from empty to full, 255 when it cannot tell.
        battery: u8,
        /// The signal-to-noise ratio of the last request, in dB, from -32 to 31.
        margin: i8,
    },
    /// Create a channel, or change one.
    NewChannelReq {
        /// Which channel. The ones a region fixes cannot be changed.
        index: u8,
        /// The center frequency in hertz. Zero switches the channel off.
        frequency_hz: u32,
        /// The fastest rate allowed on it.
        max_data_rate: u8,
        /// The slowest rate allowed on it.
        min_data_rate: u8,
    },
    /// Whether the channel could be made.
    NewChannelAns {
        /// Whether the device can run that range of rates.
        data_rate_range_ok: bool,
        /// Whether its radio can reach that frequency.
        frequency_ok: bool,
    },
    /// Set how long after sending the device starts listening.
    RxTimingSetupReq {
        /// The delay in seconds as the command codes it, which
        /// [`receive_delay_s`] reads.
        delay: u8,
    },
    /// The device took it.
    RxTimingSetupAns,
    /// Set the power and dwell time a region asks for.
    TxParamSetupReq {
        /// The coded power, which [`eirp_dbm`] reads.
        max_eirp: u8,
        /// Whether an uplink is held to 400 ms of air time.
        uplink_dwell: bool,
        /// Whether a downlink is.
        downlink_dwell: bool,
    },
    /// The device took it.
    TxParamSetupAns,
    /// Move a channel's downlink frequency away from its uplink one.
    DlChannelReq {
        /// Which channel.
        index: u8,
        /// The downlink frequency in hertz.
        frequency_hz: u32,
    },
    /// Whether that could be done.
    DlChannelAns {
        /// Whether the channel already had an uplink frequency to pair with.
        uplink_frequency_exists: bool,
        /// Whether the radio can reach the frequency.
        frequency_ok: bool,
    },
    /// A device asking the network what time it is.
    DeviceTimeReq,
    /// The time, taken as the uplink that asked finished arriving.
    DeviceTimeAns {
        /// Seconds since the GPS epoch, which leap seconds separate from UTC.
        seconds: u32,
        /// The fraction of that second, in steps of one part in 256.
        fraction: u8,
    },
}

impl MacCommand {
    /// The identifier this command goes out under.
    ///
    /// # Returns
    ///
    /// The byte, which is shared with the command answering it in the other direction.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::mac::{MacCommand, CID_LINK_ADR};
    ///
    /// let answer = MacCommand::LinkAdrAns {
    ///     power_ack: true,
    ///     data_rate_ack: true,
    ///     channel_mask_ack: true,
    /// };
    /// assert_eq!(answer.cid(), CID_LINK_ADR, "the same byte as the request");
    /// ```
    #[must_use]
    pub const fn cid(&self) -> u8 {
        match self {
            MacCommand::LinkCheckReq | MacCommand::LinkCheckAns { .. } => CID_LINK_CHECK,
            MacCommand::LinkAdrReq { .. } | MacCommand::LinkAdrAns { .. } => CID_LINK_ADR,
            MacCommand::DutyCycleReq { .. } | MacCommand::DutyCycleAns => CID_DUTY_CYCLE,
            MacCommand::RxParamSetupReq { .. } | MacCommand::RxParamSetupAns { .. } => {
                CID_RX_PARAM_SETUP
            }
            MacCommand::DevStatusReq | MacCommand::DevStatusAns { .. } => CID_DEV_STATUS,
            MacCommand::NewChannelReq { .. } | MacCommand::NewChannelAns { .. } => CID_NEW_CHANNEL,
            MacCommand::RxTimingSetupReq { .. } | MacCommand::RxTimingSetupAns => {
                CID_RX_TIMING_SETUP
            }
            MacCommand::TxParamSetupReq { .. } | MacCommand::TxParamSetupAns => CID_TX_PARAM_SETUP,
            MacCommand::DlChannelReq { .. } | MacCommand::DlChannelAns { .. } => CID_DL_CHANNEL,
            MacCommand::DeviceTimeReq | MacCommand::DeviceTimeAns { .. } => CID_DEVICE_TIME,
        }
    }

    /// Which way this command travels.
    ///
    /// # Returns
    ///
    /// [`Direction::Downlink`] for what a network sends, [`Direction::Uplink`] for what a
    /// device answers with.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::{Direction, mac::MacCommand};
    ///
    /// assert_eq!(MacCommand::DevStatusReq.direction(), Direction::Downlink);
    /// assert_eq!(MacCommand::LinkCheckReq.direction(), Direction::Uplink);
    /// ```
    #[must_use]
    pub const fn direction(&self) -> Direction {
        match self {
            MacCommand::LinkCheckReq
            | MacCommand::LinkAdrAns { .. }
            | MacCommand::DutyCycleAns
            | MacCommand::RxParamSetupAns { .. }
            | MacCommand::DevStatusAns { .. }
            | MacCommand::NewChannelAns { .. }
            | MacCommand::RxTimingSetupAns
            | MacCommand::TxParamSetupAns
            | MacCommand::DlChannelAns { .. }
            | MacCommand::DeviceTimeReq => Direction::Uplink,
            _ => Direction::Downlink,
        }
    }

    /// How many bytes this command takes, its identifier included.
    ///
    /// # Returns
    ///
    /// The length, which a reader has to know because the bytes do not carry it.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::mac::MacCommand;
    ///
    /// assert_eq!(MacCommand::LinkCheckReq.len(), 1, "the identifier and nothing else");
    /// assert_eq!(MacCommand::DevStatusAns { battery: 200, margin: -3 }.len(), 3);
    /// ```
    #[must_use]
    pub const fn len(&self) -> usize {
        1 + match self {
            MacCommand::LinkCheckReq
            | MacCommand::DutyCycleAns
            | MacCommand::DevStatusReq
            | MacCommand::RxTimingSetupAns
            | MacCommand::TxParamSetupAns
            | MacCommand::DeviceTimeReq => 0,
            MacCommand::LinkAdrAns { .. }
            | MacCommand::DutyCycleReq { .. }
            | MacCommand::RxParamSetupAns { .. }
            | MacCommand::NewChannelAns { .. }
            | MacCommand::RxTimingSetupReq { .. }
            | MacCommand::TxParamSetupReq { .. }
            | MacCommand::DlChannelAns { .. } => 1,
            MacCommand::LinkCheckAns { .. } | MacCommand::DevStatusAns { .. } => 2,
            MacCommand::LinkAdrReq { .. }
            | MacCommand::RxParamSetupReq { .. }
            | MacCommand::DlChannelReq { .. } => 4,
            MacCommand::NewChannelReq { .. } | MacCommand::DeviceTimeAns { .. } => 5,
        }
    }

    /// Whether this command is only its identifier.
    ///
    /// # Returns
    ///
    /// Whether it carries no fields.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 1
    }

    /// Writes this command out.
    ///
    /// # Arguments
    ///
    /// * `out` - where to write it.
    ///
    /// # Returns
    ///
    /// How many bytes were written.
    ///
    /// # Errors
    ///
    /// [`LorawanError::PayloadTooLong`] when `out` is shorter than the command, and
    /// [`LorawanError::MalformedFrame`] for a frequency the field cannot hold, which is one
    /// above 1.67 GHz or one that is not a whole number of hundreds of hertz.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::mac::MacCommand;
    ///
    /// let mut out = [0u8; 8];
    /// let command = MacCommand::LinkCheckAns { margin: 20, gateways: 3 };
    /// assert_eq!(command.encode(&mut out).unwrap(), 3);
    /// assert_eq!(&out[..3], &[0x02, 20, 3]);
    /// ```
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, LorawanError> {
        let len = self.len();
        if out.len() < len {
            return Err(LorawanError::PayloadTooLong);
        }
        out[0] = self.cid();

        match *self {
            MacCommand::LinkCheckReq
            | MacCommand::DutyCycleAns
            | MacCommand::DevStatusReq
            | MacCommand::RxTimingSetupAns
            | MacCommand::TxParamSetupAns
            | MacCommand::DeviceTimeReq => {}
            MacCommand::LinkCheckAns { margin, gateways } => {
                out[1] = margin;
                out[2] = gateways;
            }
            MacCommand::LinkAdrReq {
                data_rate,
                tx_power,
                channel_mask,
                mask_control,
                transmissions,
            } => {
                out[1] = (data_rate << 4) | (tx_power & 0x0f);
                out[2..4].copy_from_slice(&channel_mask.to_le_bytes());
                out[4] = ((mask_control & 0x07) << 4) | (transmissions & 0x0f);
            }
            MacCommand::LinkAdrAns {
                power_ack,
                data_rate_ack,
                channel_mask_ack,
            } => {
                out[1] = (u8::from(power_ack) << 2)
                    | (u8::from(data_rate_ack) << 1)
                    | u8::from(channel_mask_ack);
            }
            MacCommand::DutyCycleReq { max_duty_cycle } => out[1] = max_duty_cycle & 0x0f,
            MacCommand::RxParamSetupReq {
                rx1_offset,
                rx2_data_rate,
                frequency_hz,
            } => {
                out[1] = ((rx1_offset & 0x07) << 4) | (rx2_data_rate & 0x0f);
                write_frequency(frequency_hz, &mut out[2..5])?;
            }
            MacCommand::RxParamSetupAns {
                rx1_offset_ack,
                rx2_data_rate_ack,
                channel_ack,
            } => {
                out[1] = (u8::from(rx1_offset_ack) << 2)
                    | (u8::from(rx2_data_rate_ack) << 1)
                    | u8::from(channel_ack);
            }
            MacCommand::DevStatusAns { battery, margin } => {
                out[1] = battery;
                out[2] = (margin as u8) & 0x3f;
            }
            MacCommand::NewChannelReq {
                index,
                frequency_hz,
                max_data_rate,
                min_data_rate,
            } => {
                out[1] = index;
                write_frequency(frequency_hz, &mut out[2..5])?;
                out[5] = (max_data_rate << 4) | (min_data_rate & 0x0f);
            }
            MacCommand::NewChannelAns {
                data_rate_range_ok,
                frequency_ok,
            } => {
                out[1] = (u8::from(data_rate_range_ok) << 1) | u8::from(frequency_ok);
            }
            MacCommand::RxTimingSetupReq { delay } => out[1] = delay & 0x0f,
            MacCommand::TxParamSetupReq {
                max_eirp,
                uplink_dwell,
                downlink_dwell,
            } => {
                out[1] = (u8::from(downlink_dwell) << 5)
                    | (u8::from(uplink_dwell) << 4)
                    | (max_eirp & 0x0f);
            }
            MacCommand::DlChannelReq {
                index,
                frequency_hz,
            } => {
                out[1] = index;
                write_frequency(frequency_hz, &mut out[2..5])?;
            }
            MacCommand::DlChannelAns {
                uplink_frequency_exists,
                frequency_ok,
            } => {
                out[1] = (u8::from(uplink_frequency_exists) << 1) | u8::from(frequency_ok);
            }
            MacCommand::DeviceTimeAns { seconds, fraction } => {
                out[1..5].copy_from_slice(&seconds.to_le_bytes());
                out[5] = fraction;
            }
        }

        Ok(len)
    }

    /// Reads one command.
    ///
    /// # Arguments
    ///
    /// * `direction` - which way the frame carrying it travels. The same identifier is a
    ///   different command each way, and nothing in the bytes says which, so this is not
    ///   guessed.
    /// * `bytes` - the command and whatever follows it.
    ///
    /// # Returns
    ///
    /// The command and how many bytes it took.
    ///
    /// # Errors
    ///
    /// [`LorawanError::FrameTooShort`] when the bytes stop inside the command, and
    /// [`LorawanError::UnknownCommand`] carrying the identifier when it is one this
    /// version does not know, which a caller cannot step over.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::{Direction, mac::MacCommand};
    ///
    /// // The same byte, read both ways.
    /// let bytes = [0x04, 0x0a];
    /// let (down, _) = MacCommand::parse(Direction::Downlink, &bytes).unwrap();
    /// assert_eq!(down, MacCommand::DutyCycleReq { max_duty_cycle: 10 });
    ///
    /// let (up, taken) = MacCommand::parse(Direction::Uplink, &bytes).unwrap();
    /// assert_eq!(up, MacCommand::DutyCycleAns);
    /// assert_eq!(taken, 1, "the answer has no payload, so the second byte is not its own");
    /// ```
    pub fn parse(direction: Direction, bytes: &[u8]) -> Result<(MacCommand, usize), LorawanError> {
        let Some(&cid) = bytes.first() else {
            return Err(LorawanError::FrameTooShort);
        };
        let rest = &bytes[1..];
        let down = matches!(direction, Direction::Downlink);

        let command = match (cid, down) {
            (CID_LINK_CHECK, false) => MacCommand::LinkCheckReq,
            (CID_LINK_CHECK, true) => MacCommand::LinkCheckAns {
                margin: byte(rest, 0)?,
                gateways: byte(rest, 1)?,
            },
            (CID_LINK_ADR, true) => {
                let rate_power = byte(rest, 0)?;
                let redundancy = byte(rest, 3)?;
                MacCommand::LinkAdrReq {
                    data_rate: rate_power >> 4,
                    tx_power: rate_power & 0x0f,
                    channel_mask: u16::from_le_bytes([byte(rest, 1)?, byte(rest, 2)?]),
                    mask_control: (redundancy >> 4) & 0x07,
                    transmissions: redundancy & 0x0f,
                }
            }
            (CID_LINK_ADR, false) => {
                let status = byte(rest, 0)?;
                MacCommand::LinkAdrAns {
                    power_ack: status & 0x04 != 0,
                    data_rate_ack: status & 0x02 != 0,
                    channel_mask_ack: status & 0x01 != 0,
                }
            }
            (CID_DUTY_CYCLE, true) => MacCommand::DutyCycleReq {
                max_duty_cycle: byte(rest, 0)? & 0x0f,
            },
            (CID_DUTY_CYCLE, false) => MacCommand::DutyCycleAns,
            (CID_RX_PARAM_SETUP, true) => {
                let settings = byte(rest, 0)?;
                MacCommand::RxParamSetupReq {
                    rx1_offset: (settings >> 4) & 0x07,
                    rx2_data_rate: settings & 0x0f,
                    frequency_hz: read_frequency(rest, 1)?,
                }
            }
            (CID_RX_PARAM_SETUP, false) => {
                let status = byte(rest, 0)?;
                MacCommand::RxParamSetupAns {
                    rx1_offset_ack: status & 0x04 != 0,
                    rx2_data_rate_ack: status & 0x02 != 0,
                    channel_ack: status & 0x01 != 0,
                }
            }
            (CID_DEV_STATUS, true) => MacCommand::DevStatusReq,
            (CID_DEV_STATUS, false) => MacCommand::DevStatusAns {
                battery: byte(rest, 0)?,
                margin: sign_extend_6(byte(rest, 1)?),
            },
            (CID_NEW_CHANNEL, true) => {
                let range = byte(rest, 4)?;
                MacCommand::NewChannelReq {
                    index: byte(rest, 0)?,
                    frequency_hz: read_frequency(rest, 1)?,
                    max_data_rate: range >> 4,
                    min_data_rate: range & 0x0f,
                }
            }
            (CID_NEW_CHANNEL, false) => {
                let status = byte(rest, 0)?;
                MacCommand::NewChannelAns {
                    data_rate_range_ok: status & 0x02 != 0,
                    frequency_ok: status & 0x01 != 0,
                }
            }
            (CID_RX_TIMING_SETUP, true) => MacCommand::RxTimingSetupReq {
                delay: byte(rest, 0)? & 0x0f,
            },
            (CID_RX_TIMING_SETUP, false) => MacCommand::RxTimingSetupAns,
            (CID_TX_PARAM_SETUP, true) => {
                let field = byte(rest, 0)?;
                MacCommand::TxParamSetupReq {
                    max_eirp: field & 0x0f,
                    uplink_dwell: field & 0x10 != 0,
                    downlink_dwell: field & 0x20 != 0,
                }
            }
            (CID_TX_PARAM_SETUP, false) => MacCommand::TxParamSetupAns,
            (CID_DL_CHANNEL, true) => MacCommand::DlChannelReq {
                index: byte(rest, 0)?,
                frequency_hz: read_frequency(rest, 1)?,
            },
            (CID_DL_CHANNEL, false) => {
                let status = byte(rest, 0)?;
                MacCommand::DlChannelAns {
                    uplink_frequency_exists: status & 0x02 != 0,
                    frequency_ok: status & 0x01 != 0,
                }
            }
            (CID_DEVICE_TIME, false) => MacCommand::DeviceTimeReq,
            (CID_DEVICE_TIME, true) => MacCommand::DeviceTimeAns {
                seconds: u32::from_le_bytes([
                    byte(rest, 0)?,
                    byte(rest, 1)?,
                    byte(rest, 2)?,
                    byte(rest, 3)?,
                ]),
                fraction: byte(rest, 4)?,
            },
            _ => return Err(LorawanError::UnknownCommand(cid)),
        };

        Ok((command, command.len()))
    }
}

/// Writes a run of commands into one buffer.
///
/// # Arguments
///
/// * `commands` - what to write, in the order they go out.
/// * `out` - where to write them.
///
/// # Returns
///
/// How many bytes were written.
///
/// # Errors
///
/// [`LorawanError::PayloadTooLong`] when they do not fit, and whatever
/// [`MacCommand::encode`] refuses.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::mac::{encode_all, MacCommand, FOPTS_MAX};
///
/// let mut out = [0u8; FOPTS_MAX];
/// let written = encode_all(
///     &[MacCommand::LinkCheckReq, MacCommand::DeviceTimeReq],
///     &mut out,
/// )
/// .unwrap();
/// assert_eq!(&out[..written], &[0x02, 0x0d]);
/// ```
pub fn encode_all(commands: &[MacCommand], out: &mut [u8]) -> Result<usize, LorawanError> {
    let mut at = 0;
    for command in commands {
        at += command.encode(out.get_mut(at..).ok_or(LorawanError::PayloadTooLong)?)?;
    }
    Ok(at)
}

/// Walks the commands packed into one field.
///
/// Iteration ends at an identifier this version does not know, because a command does not
/// carry its own length and so cannot be stepped over. What was not read is left in
/// [`remaining`](Self::remaining).
pub struct MacCommands<'a> {
    direction: Direction,
    bytes: &'a [u8],
    at: usize,
    stopped: bool,
}

impl<'a> MacCommands<'a> {
    /// Reads the commands in a field.
    ///
    /// # Arguments
    ///
    /// * `direction` - which way the frame carrying them travels.
    /// * `bytes` - the frame options, or a payload sent on port zero.
    ///
    /// # Returns
    ///
    /// The walk.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::{Direction, mac::{MacCommand, MacCommands}};
    ///
    /// let field = [0x02, 0x0d];
    /// let read: Vec<MacCommand> = MacCommands::new(Direction::Uplink, &field)
    ///     .map(|command| command.unwrap())
    ///     .collect();
    /// assert_eq!(read, [MacCommand::LinkCheckReq, MacCommand::DeviceTimeReq]);
    /// ```
    #[must_use]
    pub const fn new(direction: Direction, bytes: &'a [u8]) -> MacCommands<'a> {
        MacCommands {
            direction,
            bytes,
            at: 0,
            stopped: false,
        }
    }

    /// What was not read.
    ///
    /// # Returns
    ///
    /// The bytes left, which is empty once every command has been read and is whatever
    /// followed an identifier this version does not know.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::{Direction, mac::MacCommands};
    ///
    /// let field = [0x02, 0x7f, 0x11, 0x22];
    /// let mut walk = MacCommands::new(Direction::Uplink, &field);
    /// assert!(walk.next().is_some(), "the first one is known");
    /// assert!(walk.next().is_none(), "0x7f is not, so the walk stops");
    /// assert_eq!(walk.remaining(), &[0x7f, 0x11, 0x22]);
    /// ```
    #[must_use]
    pub fn remaining(&self) -> &'a [u8] {
        &self.bytes[self.at.min(self.bytes.len())..]
    }
}

impl Iterator for MacCommands<'_> {
    type Item = Result<MacCommand, LorawanError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stopped || self.at >= self.bytes.len() {
            return None;
        }
        match MacCommand::parse(self.direction, &self.bytes[self.at..]) {
            Ok((command, taken)) => {
                self.at += taken;
                Some(Ok(command))
            }
            Err(LorawanError::UnknownCommand(_)) => {
                self.stopped = true;
                None
            }
            Err(error) => {
                self.stopped = true;
                Some(Err(error))
            }
        }
    }
}

// One byte of a command's fields, or a refusal when the command is cut short.
fn byte(rest: &[u8], at: usize) -> Result<u8, LorawanError> {
    rest.get(at).copied().ok_or(LorawanError::FrameTooShort)
}

// A frequency field: three bytes, low byte first, counting hundreds of hertz.
fn read_frequency(rest: &[u8], at: usize) -> Result<u32, LorawanError> {
    let raw = u32::from_le_bytes([byte(rest, at)?, byte(rest, at + 1)?, byte(rest, at + 2)?, 0]);
    Ok(raw * 100)
}

fn write_frequency(hertz: u32, out: &mut [u8]) -> Result<(), LorawanError> {
    if !hertz.is_multiple_of(100) {
        return Err(LorawanError::MalformedFrame);
    }
    let raw = hertz / 100;
    if raw > 0x00ff_ffff {
        return Err(LorawanError::MalformedFrame);
    }
    let bytes = raw.to_le_bytes();
    out[..3].copy_from_slice(&bytes[..3]);
    Ok(())
}

// A six-bit signed field, widened to the number it stands for.
const fn sign_extend_6(byte: u8) -> i8 {
    let value = byte & 0x3f;
    if value & 0x20 != 0 {
        (value as i8) - 64
    } else {
        value as i8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encodes a command and hands back the bytes.
    fn encoded(command: MacCommand) -> [u8; MAX_COMMAND] {
        let mut out = [0u8; MAX_COMMAND];
        let len = command.encode(&mut out).expect("it encodes");
        assert_eq!(
            len,
            command.len(),
            "the length it reports is the length it writes"
        );
        out
    }

    #[test]
    fn the_same_identifier_is_a_different_command_in_each_direction() {
        let bytes = [0x03, 0x52, 0xff, 0x00, 0x01];

        let (down, taken) = MacCommand::parse(Direction::Downlink, &bytes).expect("it parses");
        assert_eq!(
            down,
            MacCommand::LinkAdrReq {
                data_rate: 5,
                tx_power: 2,
                channel_mask: 0x00ff,
                mask_control: 0,
                transmissions: 1,
            }
        );
        assert_eq!(taken, 5);

        let (up, taken) = MacCommand::parse(Direction::Uplink, &bytes).expect("it parses");
        assert_eq!(
            up,
            MacCommand::LinkAdrAns {
                power_ack: false,
                data_rate_ack: true,
                channel_mask_ack: false,
            },
            "0x52 read as a status byte, not as a rate"
        );
        assert_eq!(taken, 2, "the answer is shorter than the request");
    }

    #[test]
    fn every_layout_matches_the_specification() {
        assert_eq!(
            &encoded(MacCommand::LinkCheckAns {
                margin: 20,
                gateways: 3
            })[..3],
            &[0x02, 20, 3]
        );
        assert_eq!(
            &encoded(MacCommand::LinkAdrReq {
                data_rate: 5,
                tx_power: 2,
                channel_mask: 0x00ff,
                mask_control: 0,
                transmissions: 1,
            })[..5],
            &[0x03, 0x52, 0xff, 0x00, 0x01]
        );
        assert_eq!(
            &encoded(MacCommand::DutyCycleReq { max_duty_cycle: 4 })[..2],
            &[0x04, 0x04]
        );
        assert_eq!(
            &encoded(MacCommand::RxParamSetupReq {
                rx1_offset: 2,
                rx2_data_rate: 3,
                frequency_hz: 869_525_000,
            })[..5],
            &[0x05, 0x23, 0xd2, 0xad, 0x84]
        );
        assert_eq!(
            &encoded(MacCommand::NewChannelReq {
                index: 3,
                frequency_hz: 867_100_000,
                max_data_rate: 5,
                min_data_rate: 0,
            })[..6],
            &[0x07, 0x03, 0x18, 0x4f, 0x84, 0x50]
        );
        assert_eq!(
            &encoded(MacCommand::RxTimingSetupReq { delay: 5 })[..2],
            &[0x08, 0x05]
        );
        assert_eq!(
            &encoded(MacCommand::DlChannelReq {
                index: 1,
                frequency_hz: 869_525_000,
            })[..5],
            &[0x0a, 0x01, 0xd2, 0xad, 0x84]
        );
    }

    #[test]
    fn a_frequency_is_carried_in_hundreds_of_hertz_low_byte_first() {
        let command = MacCommand::DlChannelReq {
            index: 0,
            frequency_hz: 868_100_000,
        };
        let bytes = encoded(command);
        assert_eq!(
            u32::from_le_bytes([bytes[2], bytes[3], bytes[4], 0]) * 100,
            868_100_000
        );

        let (read, _) = MacCommand::parse(Direction::Downlink, &bytes).expect("it parses");
        assert_eq!(read, command, "what goes out reads back the same");

        // A frequency the field cannot hold is refused rather than truncated.
        let mut out = [0u8; MAX_COMMAND];
        assert_eq!(
            MacCommand::DlChannelReq {
                index: 0,
                frequency_hz: 2_000_000_000,
            }
            .encode(&mut out),
            Err(LorawanError::MalformedFrame)
        );
        assert_eq!(
            MacCommand::DlChannelReq {
                index: 0,
                frequency_hz: 868_100_050,
            }
            .encode(&mut out),
            Err(LorawanError::MalformedFrame),
            "the field counts hundreds, so this is not representable"
        );
    }

    #[test]
    fn a_margin_is_a_six_bit_signed_number() {
        for margin in [-32i8, -10, -1, 0, 1, 31] {
            let command = MacCommand::DevStatusAns {
                battery: 200,
                margin,
            };
            let bytes = encoded(command);
            let (read, _) = MacCommand::parse(Direction::Uplink, &bytes).expect("it parses");
            assert_eq!(read, command, "at {margin}");
        }

        assert_eq!(
            &encoded(MacCommand::DevStatusAns {
                battery: 0x80,
                margin: -10
            })[..3],
            &[0x06, 0x80, 0x36]
        );
    }

    #[test]
    fn a_command_this_version_does_not_know_stops_the_walk() {
        // Nothing carries its own length, so the reader cannot step over what it does not
        // know. It stops, and says what it did not read.
        let field = [0x02, 0x7f, 0x11, 0x22];
        let mut walk = MacCommands::new(Direction::Uplink, &field);

        assert_eq!(walk.next(), Some(Ok(MacCommand::LinkCheckReq)));
        assert_eq!(walk.next(), None);
        assert_eq!(walk.remaining(), &[0x7f, 0x11, 0x22]);
    }

    #[test]
    fn a_command_cut_short_is_refused_rather_than_guessed() {
        assert_eq!(
            MacCommand::parse(Direction::Downlink, &[0x03, 0x52]),
            Err(LorawanError::FrameTooShort)
        );
        assert_eq!(
            MacCommand::parse(Direction::Uplink, &[]),
            Err(LorawanError::FrameTooShort)
        );

        let field = [0x02, 0x06, 0xc8];
        let mut walk = MacCommands::new(Direction::Uplink, &field);
        assert_eq!(walk.next(), Some(Ok(MacCommand::LinkCheckReq)));
        assert_eq!(
            walk.next(),
            Some(Err(LorawanError::FrameTooShort)),
            "the status answer wants two bytes and only one is left"
        );
        assert_eq!(walk.next(), None, "and it does not carry on afterwards");
    }

    #[test]
    fn a_run_of_commands_reads_back_as_it_was_written() {
        let commands = [
            MacCommand::DevStatusAns {
                battery: 255,
                margin: -3,
            },
            MacCommand::LinkCheckReq,
            MacCommand::RxTimingSetupAns,
        ];

        let mut out = [0u8; FOPTS_MAX];
        let written = encode_all(&commands, &mut out).expect("they fit");

        let read: Vec<MacCommand> = MacCommands::new(Direction::Uplink, &out[..written])
            .map(|command| command.expect("each one parses"))
            .collect();
        assert_eq!(read.as_slice(), &commands[..]);
    }

    #[test]
    fn commands_that_do_not_fit_the_options_field_are_refused() {
        // Sixteen bytes of commands do not fit beside a payload; they need a frame of their
        // own, on port zero.
        let long = [MacCommand::NewChannelReq {
            index: 0,
            frequency_hz: 868_100_000,
            max_data_rate: 5,
            min_data_rate: 0,
        }; 3];

        let mut out = [0u8; FOPTS_MAX];
        assert_eq!(
            encode_all(&long, &mut out),
            Err(LorawanError::PayloadTooLong)
        );

        let mut roomy = [0u8; 32];
        assert_eq!(encode_all(&long, &mut roomy), Ok(18));
    }

    #[test]
    fn the_transmit_power_table_is_the_one_the_specification_prints() {
        assert_eq!(eirp_dbm(0), Some(8));
        assert_eq!(eirp_dbm(5), Some(16));
        assert_eq!(eirp_dbm(15), Some(36));
        assert_eq!(eirp_dbm(16), None);

        assert_eq!(eirp_code(16), Some(5));
        assert_eq!(eirp_code(36), Some(15));
        assert_eq!(
            eirp_code(17),
            None,
            "a power between two steps is not rounded"
        );

        let command = MacCommand::TxParamSetupReq {
            max_eirp: 5,
            uplink_dwell: true,
            downlink_dwell: false,
        };
        assert_eq!(&encoded(command)[..2], &[0x09, 0x15]);

        let (read, _) =
            MacCommand::parse(Direction::Downlink, &encoded(command)).expect("it parses");
        assert_eq!(read, command);
    }

    #[test]
    fn a_receive_delay_of_zero_is_one_second() {
        assert_eq!(receive_delay_s(0), 1);
        assert_eq!(receive_delay_s(1), 1);
        assert_eq!(receive_delay_s(15), 15);
    }

    #[test]
    fn every_command_reports_the_direction_it_travels() {
        assert_eq!(MacCommand::LinkCheckReq.direction(), Direction::Uplink);
        assert_eq!(
            MacCommand::LinkCheckAns {
                margin: 0,
                gateways: 0
            }
            .direction(),
            Direction::Downlink
        );
        assert_eq!(MacCommand::DeviceTimeReq.direction(), Direction::Uplink);
        assert_eq!(
            MacCommand::DeviceTimeAns {
                seconds: 0,
                fraction: 0
            }
            .direction(),
            Direction::Downlink
        );
    }

    #[test]
    fn the_network_time_is_seconds_since_the_gps_epoch() {
        // The worked example the specification gives.
        let command = MacCommand::DeviceTimeAns {
            seconds: 1_139_322_288,
            fraction: 0,
        };
        let bytes = encoded(command);
        assert_eq!(bytes[0], CID_DEVICE_TIME);
        assert_eq!(
            u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]),
            1_139_322_288
        );

        let (read, taken) = MacCommand::parse(Direction::Downlink, &bytes).expect("it parses");
        assert_eq!(read, command);
        assert_eq!(taken, 6);
    }
}
