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
//! Every layout here is from the LoRaWAN 1.0.3 specification, section 5, and the relay
//! commands from TS011-1.0.1 section 10. Multi-byte fields go out low byte first as the rest
//! of the protocol does.

use pamoja_lora::region::RelayChannel;

use crate::{Direction, LorawanError};

/// How many bytes of commands a frame can carry beside a payload.
///
/// Past this they have to go in a payload of their own, on port zero.
pub const FOPTS_MAX: usize = 15;

/// The longest single command, which is the one that adds an end device to a relay's
/// trusted list.
pub const MAX_COMMAND: usize = 27;

/// The longest join filter a relay rule carries: a JoinEUI and a DevEUI.
pub const FILTER_EUI_MAX: usize = 16;

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

/// Configures a relay's wake-on-radio channels, TS011-1.0.1 section 10.1.
pub const CID_RELAY_CONF: u8 = 0x40;

/// Configures how an end device uses a relay, TS011-1.0.1 section 10.2.
pub const CID_END_DEVICE_CONF: u8 = 0x41;

/// Sets a rule of a relay's join request filter, TS011-1.0.1 section 10.3.
pub const CID_FILTER_LIST: u8 = 0x42;

/// Adds an end device to a relay's trusted list, TS011-1.0.1 section 10.4.
pub const CID_UPDATE_UPLINK_LIST: u8 = 0x43;

/// Reads or removes an end device of a relay's trusted list, TS011-1.0.1 section 10.5.
pub const CID_CTRL_UPLINK_LIST: u8 = 0x44;

/// Sets a relay's forwarding limits, TS011-1.0.1 section 10.6.
pub const CID_CONFIGURE_FWD_LIMIT: u8 = 0x45;

/// A relay telling the network of an end device it could not verify, TS011-1.0.1
/// section 10.7.
pub const CID_NOTIFY_NEW_END_DEVICE: u8 = 0x46;

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
    /// Start or stop a relay, and set the channels it scans.
    RelayConfReq {
        /// Whether the relay runs. With it stopped, every other field is ignored.
        enabled: bool,
        /// How often the relay scans its default channel, as TS011-1.0.1 table 18 codes it.
        cad_periodicity: u8,
        /// Which of the region's relay channels is the default one, 0 or 1.
        default_channel_index: u8,
        /// Whether a second channel is set, as table 34 codes it: 0 for none, 1 for the one
        /// the next fields describe.
        second_channel_index: u8,
        /// The second channel's data rate.
        second_channel_data_rate: u8,
        /// How far above its frequency the second channel is acknowledged, as table 35
        /// codes it.
        second_channel_ack_offset: u8,
        /// The second channel's frequency in hertz.
        second_channel_frequency_hz: u32,
    },
    /// Which parts of the relay configuration were valid.
    RelayConfAns {
        /// Whether the scan period was.
        cad_periodicity_ack: bool,
        /// Whether the default channel was.
        default_channel_index_ack: bool,
        /// Whether the second channel index was.
        second_channel_index_ack: bool,
        /// Whether the second channel's data rate was.
        second_channel_data_rate_ack: bool,
        /// Whether its acknowledgment offset was.
        second_channel_ack_offset_ack: bool,
        /// Whether its frequency was.
        second_channel_frequency_ack: bool,
    },
    /// Set how an end device uses a relay.
    EndDeviceConfReq {
        /// Whether relaying is off (0), on (1), turned on after uplinks go unanswered (2), or
        /// left to the device (3), TS011-1.0.1 table 40.
        relay_mode: u8,
        /// How many unanswered uplinks turn relaying on in mode 2, as table 41 codes it.
        smart_enable_level: u8,
        /// How many wake-on-radio frames without an acknowledgment before the uplink goes
        /// anyway, 0 meaning every time, table 43.
        back_off: u8,
        /// Whether a second channel is set, 0 for none and 1 for the one described.
        second_channel_index: u8,
        /// The second channel's data rate.
        second_channel_data_rate: u8,
        /// How far above its frequency the second channel is acknowledged, table 35.
        second_channel_ack_offset: u8,
        /// The second channel's frequency in hertz.
        second_channel_frequency_hz: u32,
    },
    /// Which parts of the end device configuration were valid, TS011-1.0.1 table 45.
    EndDeviceConfAns {
        /// Whether the second channel's acknowledgment offset was.
        second_channel_ack_offset_ack: bool,
        /// Whether the second channel index was.
        second_channel_index_ack: bool,
        /// Whether its data rate was.
        second_channel_data_rate_ack: bool,
        /// Whether its frequency was.
        second_channel_frequency_ack: bool,
    },
    /// Set one rule of a relay's join request filter.
    FilterListReq {
        /// The rule, 0 being the action when no other rule matches.
        index: u8,
        /// No rule (0), forward (1) or filter (2), TS011-1.0.1 table 48.
        action: u8,
        /// How many leading bytes of JoinEUI and DevEUI the rule matches. A length past
        /// [`FILTER_EUI_MAX`] is kept, so a relay can refuse it, but not its bytes past the
        /// sixteenth.
        eui_len: u8,
        /// The leading bytes of JoinEUI then DevEUI, most significant first, as an EUI is
        /// written.
        eui: [u8; FILTER_EUI_MAX],
    },
    /// Which parts of the filter rule were valid.
    FilterListAns {
        /// Whether the fields together made a rule to create, change or remove.
        combined_rules_ack: bool,
        /// Whether the length was.
        eui_len_ack: bool,
        /// Whether the action was.
        action_ack: bool,
    },
    /// Trust an end device, so a relay verifies its wake-on-radio frames and forwards it.
    UpdateUplinkListReq {
        /// The entry of the trusted list, 0 to 15.
        index: u8,
        /// Tokens earned an hour, 63 meaning no limit, TS011-1.0.1 table 54.
        reload_rate: u8,
        /// The bucket size multiplier, as table 55 codes it.
        bucket_size: u8,
        /// The end device's address.
        dev_addr: u32,
        /// The next wake-on-radio frame counter the network expects from it.
        wfcnt: u32,
        /// The root relay session key the device's wake-on-radio keys come from.
        root_wor_s_key: [u8; 16],
    },
    /// The relay took it.
    UpdateUplinkListAns,
    /// Read the counter of a trusted end device, or remove it.
    CtrlUplinkListReq {
        /// The entry of the trusted list.
        index: u8,
        /// Read (0) or remove (1), TS011-1.0.1 table 58.
        action: u8,
    },
    /// The entry's last valid counter.
    CtrlUplinkListAns {
        /// Whether the entry was in use.
        index_ack: bool,
        /// The last wake-on-radio frame counter the relay accepted from it.
        wfcnt: u32,
    },
    /// Set a relay's forwarding limits, TS011-1.0.1 section 10.6.
    ConfigureFwdLimitReq {
        /// What happens to the token counters, as table 63 codes it.
        reset_limit_counters: u8,
        /// Join requests forwarded an hour, 127 meaning no limit.
        join_request_reload_rate: u8,
        /// New end device notifications an hour.
        notify_reload_rate: u8,
        /// Uplinks forwarded an hour across every trusted end device.
        global_uplink_reload_rate: u8,
        /// Every message the relay sends an hour.
        overall_reload_rate: u8,
        /// The join request bucket size multiplier, table 55.
        join_request_bucket_size: u8,
        /// The notification bucket size multiplier.
        notify_bucket_size: u8,
        /// The global uplink bucket size multiplier.
        global_uplink_bucket_size: u8,
        /// The overall bucket size multiplier.
        overall_bucket_size: u8,
    },
    /// The relay took it.
    ConfigureFwdLimitAns,
    /// A relay heard a wake-on-radio frame it could not verify.
    NotifyNewEndDeviceReq {
        /// The address the frame named.
        dev_addr: u32,
        /// The frame's signal strength in dBm, carried from -142 to -15.
        rssi_dbm: i16,
        /// Its signal-to-noise ratio in dB, carried from -20 to 11.
        snr_db: i8,
    },
}

/// The frequency offsets a relay's second channel can be acknowledged at, TS011-1.0.1
/// table 35, by their coded value.
pub const RELAY_ACK_OFFSET_HZ: [u32; 6] = [0, 200_000, 400_000, 800_000, 1_600_000, 3_200_000];

/// What a coded bucket size multiplies a reload rate by, TS011-1.0.1 table 55.
pub const RELAY_BUCKET_MULTIPLIER: [u16; 4] = [1, 2, 4, 12];

/// The second channel a relay configuration describes, where it names one.
///
/// # Arguments
///
/// * `second_channel_index` - the coded index, 1 for a second channel.
/// * `data_rate` - its data rate.
/// * `ack_offset` - the coded acknowledgment offset.
/// * `frequency_hz` - its frequency.
///
/// # Returns
///
/// The channel, with the acknowledgment frequency worked out, or `None` when the index or
/// offset is not one TS011-1.0.1 defines.
///
/// # Examples
///
/// ```
/// use pamoja_lora::region::RelayChannel;
/// use pamoja_lorawan::mac::relay_second_channel;
///
/// assert_eq!(
///     relay_second_channel(1, 3, 1, 868_100_000),
///     Some(RelayChannel::new(868_100_000, 868_300_000, 3)),
///     "offset 1 is 200 kHz"
/// );
/// assert_eq!(relay_second_channel(0, 3, 1, 868_100_000), None, "no second channel");
/// assert_eq!(relay_second_channel(1, 3, 6, 868_100_000), None, "offset 6 is reserved");
/// ```
#[must_use]
pub fn relay_second_channel(
    second_channel_index: u8,
    data_rate: u8,
    ack_offset: u8,
    frequency_hz: u32,
) -> Option<RelayChannel> {
    if second_channel_index != 1 {
        return None;
    }
    let offset = *RELAY_ACK_OFFSET_HZ.get(usize::from(ack_offset))?;
    Some(RelayChannel::new(
        frequency_hz,
        frequency_hz.checked_add(offset)?,
        data_rate,
    ))
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
            MacCommand::RelayConfReq { .. } | MacCommand::RelayConfAns { .. } => CID_RELAY_CONF,
            MacCommand::EndDeviceConfReq { .. } | MacCommand::EndDeviceConfAns { .. } => {
                CID_END_DEVICE_CONF
            }
            MacCommand::FilterListReq { .. } | MacCommand::FilterListAns { .. } => CID_FILTER_LIST,
            MacCommand::UpdateUplinkListReq { .. } | MacCommand::UpdateUplinkListAns => {
                CID_UPDATE_UPLINK_LIST
            }
            MacCommand::CtrlUplinkListReq { .. } | MacCommand::CtrlUplinkListAns { .. } => {
                CID_CTRL_UPLINK_LIST
            }
            MacCommand::ConfigureFwdLimitReq { .. } | MacCommand::ConfigureFwdLimitAns => {
                CID_CONFIGURE_FWD_LIMIT
            }
            MacCommand::NotifyNewEndDeviceReq { .. } => CID_NOTIFY_NEW_END_DEVICE,
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
            | MacCommand::DeviceTimeReq
            | MacCommand::RelayConfAns { .. }
            | MacCommand::EndDeviceConfAns { .. }
            | MacCommand::FilterListAns { .. }
            | MacCommand::UpdateUplinkListAns
            | MacCommand::CtrlUplinkListAns { .. }
            | MacCommand::ConfigureFwdLimitAns
            | MacCommand::NotifyNewEndDeviceReq { .. } => Direction::Uplink,
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
            | MacCommand::DeviceTimeReq
            | MacCommand::UpdateUplinkListAns
            | MacCommand::ConfigureFwdLimitAns => 0,
            MacCommand::LinkAdrAns { .. }
            | MacCommand::RelayConfAns { .. }
            | MacCommand::EndDeviceConfAns { .. }
            | MacCommand::FilterListAns { .. }
            | MacCommand::CtrlUplinkListReq { .. }
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
            MacCommand::NewChannelReq { .. }
            | MacCommand::DeviceTimeAns { .. }
            | MacCommand::RelayConfReq { .. }
            | MacCommand::CtrlUplinkListAns { .. }
            | MacCommand::ConfigureFwdLimitReq { .. } => 5,
            MacCommand::EndDeviceConfReq { .. } | MacCommand::NotifyNewEndDeviceReq { .. } => 6,
            MacCommand::FilterListReq { eui_len, .. } => 2 + *eui_len as usize,
            MacCommand::UpdateUplinkListReq { .. } => 26,
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
    /// above 1.67 GHz or one that is not a whole number of hundreds of hertz, and for a join
    /// filter longer than [`FILTER_EUI_MAX`] bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::mac::MacCommand;
    /// use pamoja_lorawan::Direction;
    ///
    /// // The network's answer to a link check: heard 20 dB above the floor by three gateways.
    /// let mut out = [0u8; 8];
    /// let command = MacCommand::LinkCheckAns { margin: 20, gateways: 3 };
    /// let written = command.encode(&mut out).unwrap();
    ///
    /// // The device reads it back from the downlink's frame options.
    /// let (heard, taken) = MacCommand::parse(Direction::Downlink, &out[..written]).unwrap();
    /// assert_eq!((heard, taken), (command, written));
    /// ```
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, LorawanError> {
        if let MacCommand::FilterListReq { eui_len, .. } = *self {
            if usize::from(eui_len) > FILTER_EUI_MAX {
                return Err(LorawanError::MalformedFrame);
            }
        }
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
            | MacCommand::DeviceTimeReq
            | MacCommand::UpdateUplinkListAns
            | MacCommand::ConfigureFwdLimitAns => {}
            MacCommand::LinkCheckAns { margin, gateways } => {
                out[1] = margin;
                out[2] = gateways;
            }
            MacCommand::RelayConfReq {
                enabled,
                cad_periodicity,
                default_channel_index,
                second_channel_index,
                second_channel_data_rate,
                second_channel_ack_offset,
                second_channel_frequency_hz,
            } => {
                let settings = (u16::from(enabled) << 13)
                    | (u16::from(cad_periodicity & 0x07) << 10)
                    | (u16::from(default_channel_index & 0x01) << 9)
                    | second_channel_bits(
                        second_channel_index,
                        second_channel_data_rate,
                        second_channel_ack_offset,
                    );
                out[1..3].copy_from_slice(&settings.to_le_bytes());
                write_frequency(second_channel_frequency_hz, &mut out[3..6])?;
            }
            MacCommand::RelayConfAns {
                cad_periodicity_ack,
                default_channel_index_ack,
                second_channel_index_ack,
                second_channel_data_rate_ack,
                second_channel_ack_offset_ack,
                second_channel_frequency_ack,
            } => {
                out[1] = (u8::from(cad_periodicity_ack) << 5)
                    | (u8::from(default_channel_index_ack) << 4)
                    | (u8::from(second_channel_index_ack) << 3)
                    | (u8::from(second_channel_data_rate_ack) << 2)
                    | (u8::from(second_channel_ack_offset_ack) << 1)
                    | u8::from(second_channel_frequency_ack);
            }
            MacCommand::EndDeviceConfReq {
                relay_mode,
                smart_enable_level,
                back_off,
                second_channel_index,
                second_channel_data_rate,
                second_channel_ack_offset,
                second_channel_frequency_hz,
            } => {
                out[1] = ((relay_mode & 0x03) << 2) | (smart_enable_level & 0x03);
                let settings = (u16::from(back_off & 0x3f) << 9)
                    | second_channel_bits(
                        second_channel_index,
                        second_channel_data_rate,
                        second_channel_ack_offset,
                    );
                out[2..4].copy_from_slice(&settings.to_le_bytes());
                write_frequency(second_channel_frequency_hz, &mut out[4..7])?;
            }
            MacCommand::EndDeviceConfAns {
                second_channel_ack_offset_ack,
                second_channel_index_ack,
                second_channel_data_rate_ack,
                second_channel_frequency_ack,
            } => {
                out[1] = (u8::from(second_channel_ack_offset_ack) << 3)
                    | (u8::from(second_channel_index_ack) << 2)
                    | (u8::from(second_channel_data_rate_ack) << 1)
                    | u8::from(second_channel_frequency_ack);
            }
            MacCommand::FilterListReq {
                index,
                action,
                eui_len,
                eui,
            } => {
                let param = (u16::from(index & 0x0f) << 7)
                    | (u16::from(action & 0x03) << 5)
                    | u16::from(eui_len & 0x1f);
                out[1..3].copy_from_slice(&param.to_le_bytes());
                let len = usize::from(eui_len);
                for (at, byte) in eui[..len].iter().rev().enumerate() {
                    out[3 + at] = *byte;
                }
            }
            MacCommand::FilterListAns {
                combined_rules_ack,
                eui_len_ack,
                action_ack,
            } => {
                out[1] = (u8::from(combined_rules_ack) << 2)
                    | (u8::from(eui_len_ack) << 1)
                    | u8::from(action_ack);
            }
            MacCommand::UpdateUplinkListReq {
                index,
                reload_rate,
                bucket_size,
                dev_addr,
                wfcnt,
                root_wor_s_key,
            } => {
                out[1] = index & 0x0f;
                out[2] = ((bucket_size & 0x03) << 6) | (reload_rate & 0x3f);
                out[3..7].copy_from_slice(&dev_addr.to_le_bytes());
                out[7..11].copy_from_slice(&wfcnt.to_le_bytes());
                out[11..27].copy_from_slice(&root_wor_s_key);
            }
            MacCommand::CtrlUplinkListReq { index, action } => {
                out[1] = ((action & 0x01) << 4) | (index & 0x0f);
            }
            MacCommand::CtrlUplinkListAns { index_ack, wfcnt } => {
                out[1] = u8::from(index_ack);
                out[2..6].copy_from_slice(&wfcnt.to_le_bytes());
            }
            MacCommand::ConfigureFwdLimitReq {
                reset_limit_counters,
                join_request_reload_rate,
                notify_reload_rate,
                global_uplink_reload_rate,
                overall_reload_rate,
                join_request_bucket_size,
                notify_bucket_size,
                global_uplink_bucket_size,
                overall_bucket_size,
            } => {
                let rates = (u32::from(reset_limit_counters & 0x03) << 28)
                    | (u32::from(join_request_reload_rate & 0x7f) << 21)
                    | (u32::from(notify_reload_rate & 0x7f) << 14)
                    | (u32::from(global_uplink_reload_rate & 0x7f) << 7)
                    | u32::from(overall_reload_rate & 0x7f);
                out[1..5].copy_from_slice(&rates.to_le_bytes());
                out[5] = ((join_request_bucket_size & 0x03) << 6)
                    | ((notify_bucket_size & 0x03) << 4)
                    | ((global_uplink_bucket_size & 0x03) << 2)
                    | (overall_bucket_size & 0x03);
            }
            MacCommand::NotifyNewEndDeviceReq {
                dev_addr,
                rssi_dbm,
                snr_db,
            } => {
                out[1..5].copy_from_slice(&dev_addr.to_le_bytes());
                let power = (relay_rssi_code(rssi_dbm) << 5) | relay_snr_code(snr_db);
                out[5..7].copy_from_slice(&power.to_le_bytes());
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
    /// // A request and its answer share an identifier, so the direction decides which one
    /// // the same bytes are.
    /// let mut bytes = [0u8; 2];
    /// let request = MacCommand::DutyCycleReq { max_duty_cycle: 10 };
    /// request.encode(&mut bytes).unwrap();
    ///
    /// let (down, _) = MacCommand::parse(Direction::Downlink, &bytes).unwrap();
    /// assert_eq!(down, request);
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
            (CID_RELAY_CONF, true) => {
                let settings = u16::from_le_bytes([byte(rest, 0)?, byte(rest, 1)?]);
                MacCommand::RelayConfReq {
                    enabled: settings & (1 << 13) != 0,
                    cad_periodicity: ((settings >> 10) & 0x07) as u8,
                    default_channel_index: ((settings >> 9) & 0x01) as u8,
                    second_channel_index: ((settings >> 7) & 0x03) as u8,
                    second_channel_data_rate: ((settings >> 3) & 0x0f) as u8,
                    second_channel_ack_offset: (settings & 0x07) as u8,
                    second_channel_frequency_hz: read_frequency(rest, 2)?,
                }
            }
            (CID_RELAY_CONF, false) => {
                let status = byte(rest, 0)?;
                MacCommand::RelayConfAns {
                    cad_periodicity_ack: status & 0x20 != 0,
                    default_channel_index_ack: status & 0x10 != 0,
                    second_channel_index_ack: status & 0x08 != 0,
                    second_channel_data_rate_ack: status & 0x04 != 0,
                    second_channel_ack_offset_ack: status & 0x02 != 0,
                    second_channel_frequency_ack: status & 0x01 != 0,
                }
            }
            (CID_END_DEVICE_CONF, true) => {
                let mode = byte(rest, 0)?;
                let settings = u16::from_le_bytes([byte(rest, 1)?, byte(rest, 2)?]);
                MacCommand::EndDeviceConfReq {
                    relay_mode: (mode >> 2) & 0x03,
                    smart_enable_level: mode & 0x03,
                    back_off: ((settings >> 9) & 0x3f) as u8,
                    second_channel_index: ((settings >> 7) & 0x03) as u8,
                    second_channel_data_rate: ((settings >> 3) & 0x0f) as u8,
                    second_channel_ack_offset: (settings & 0x07) as u8,
                    second_channel_frequency_hz: read_frequency(rest, 3)?,
                }
            }
            (CID_END_DEVICE_CONF, false) => {
                let status = byte(rest, 0)?;
                MacCommand::EndDeviceConfAns {
                    second_channel_ack_offset_ack: status & 0x08 != 0,
                    second_channel_index_ack: status & 0x04 != 0,
                    second_channel_data_rate_ack: status & 0x02 != 0,
                    second_channel_frequency_ack: status & 0x01 != 0,
                }
            }
            (CID_FILTER_LIST, true) => {
                let param = u16::from_le_bytes([byte(rest, 0)?, byte(rest, 1)?]);
                let eui_len = (param & 0x1f) as u8;
                let len = usize::from(eui_len);
                let wire = rest.get(2..2 + len).ok_or(LorawanError::FrameTooShort)?;
                let mut eui = [0u8; FILTER_EUI_MAX];
                for (at, byte) in wire.iter().rev().take(FILTER_EUI_MAX).enumerate() {
                    eui[at] = *byte;
                }
                MacCommand::FilterListReq {
                    index: ((param >> 7) & 0x0f) as u8,
                    action: ((param >> 5) & 0x03) as u8,
                    eui_len,
                    eui,
                }
            }
            (CID_FILTER_LIST, false) => {
                let status = byte(rest, 0)?;
                MacCommand::FilterListAns {
                    combined_rules_ack: status & 0x04 != 0,
                    eui_len_ack: status & 0x02 != 0,
                    action_ack: status & 0x01 != 0,
                }
            }
            (CID_UPDATE_UPLINK_LIST, true) => {
                let key = rest.get(10..26).ok_or(LorawanError::FrameTooShort)?;
                let limit = byte(rest, 1)?;
                let mut root_wor_s_key = [0u8; 16];
                root_wor_s_key.copy_from_slice(key);
                MacCommand::UpdateUplinkListReq {
                    index: byte(rest, 0)? & 0x0f,
                    reload_rate: limit & 0x3f,
                    bucket_size: limit >> 6,
                    dev_addr: u32::from_le_bytes([
                        byte(rest, 2)?,
                        byte(rest, 3)?,
                        byte(rest, 4)?,
                        byte(rest, 5)?,
                    ]),
                    wfcnt: u32::from_le_bytes([
                        byte(rest, 6)?,
                        byte(rest, 7)?,
                        byte(rest, 8)?,
                        byte(rest, 9)?,
                    ]),
                    root_wor_s_key,
                }
            }
            (CID_UPDATE_UPLINK_LIST, false) => MacCommand::UpdateUplinkListAns,
            (CID_CTRL_UPLINK_LIST, true) => {
                let field = byte(rest, 0)?;
                MacCommand::CtrlUplinkListReq {
                    index: field & 0x0f,
                    action: (field >> 4) & 0x01,
                }
            }
            (CID_CTRL_UPLINK_LIST, false) => MacCommand::CtrlUplinkListAns {
                index_ack: byte(rest, 0)? & 0x01 != 0,
                wfcnt: u32::from_le_bytes([
                    byte(rest, 1)?,
                    byte(rest, 2)?,
                    byte(rest, 3)?,
                    byte(rest, 4)?,
                ]),
            },
            (CID_CONFIGURE_FWD_LIMIT, true) => {
                let rates = u32::from_le_bytes([
                    byte(rest, 0)?,
                    byte(rest, 1)?,
                    byte(rest, 2)?,
                    byte(rest, 3)?,
                ]);
                let sizes = byte(rest, 4)?;
                MacCommand::ConfigureFwdLimitReq {
                    reset_limit_counters: ((rates >> 28) & 0x03) as u8,
                    join_request_reload_rate: ((rates >> 21) & 0x7f) as u8,
                    notify_reload_rate: ((rates >> 14) & 0x7f) as u8,
                    global_uplink_reload_rate: ((rates >> 7) & 0x7f) as u8,
                    overall_reload_rate: (rates & 0x7f) as u8,
                    join_request_bucket_size: sizes >> 6,
                    notify_bucket_size: (sizes >> 4) & 0x03,
                    global_uplink_bucket_size: (sizes >> 2) & 0x03,
                    overall_bucket_size: sizes & 0x03,
                }
            }
            (CID_CONFIGURE_FWD_LIMIT, false) => MacCommand::ConfigureFwdLimitAns,
            (CID_NOTIFY_NEW_END_DEVICE, false) => {
                let power = u16::from_le_bytes([byte(rest, 4)?, byte(rest, 5)?]);
                MacCommand::NotifyNewEndDeviceReq {
                    dev_addr: u32::from_le_bytes([
                        byte(rest, 0)?,
                        byte(rest, 1)?,
                        byte(rest, 2)?,
                        byte(rest, 3)?,
                    ]),
                    rssi_dbm: -i16::from(((power >> 5) & 0x7f) as u8) - 15,
                    snr_db: ((power & 0x1f) as i8) - 20,
                }
            }
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
/// use pamoja_lorawan::mac::{encode_all, MacCommand, MacCommands, FOPTS_MAX};
/// use pamoja_lorawan::Direction;
///
/// // A device asks how well it is heard and what time it is, in one frame's options.
/// let asked = [MacCommand::LinkCheckReq, MacCommand::DeviceTimeReq];
/// let mut out = [0u8; FOPTS_MAX];
/// let written = encode_all(&asked, &mut out).unwrap();
///
/// let read: Vec<MacCommand> = MacCommands::new(Direction::Uplink, &out[..written])
///     .map(|command| command.unwrap())
///     .collect();
/// assert_eq!(read, asked);
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
    /// use pamoja_lorawan::{Direction, mac::{encode_all, MacCommand, MacCommands}};
    ///
    /// // The frame options of an uplink that asks for a link check and the time.
    /// let mut field = [0u8; 2];
    /// encode_all(&[MacCommand::LinkCheckReq, MacCommand::DeviceTimeReq], &mut field).unwrap();
    ///
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
    /// use pamoja_lorawan::{Direction, mac::{MacCommand, MacCommands}};
    ///
    /// // A link check request, then a command from a later version of the specification,
    /// // identifier 0x7F with two bytes of its own, which this one cannot step over.
    /// let later = [0x7F, 0x11, 0x22];
    /// let mut field = [0u8; 4];
    /// let written = MacCommand::LinkCheckReq.encode(&mut field).unwrap();
    /// field[written..].copy_from_slice(&later);
    ///
    /// let mut walk = MacCommands::new(Direction::Uplink, &field);
    /// assert!(walk.next().is_some(), "the first one is known");
    /// assert!(walk.next().is_none(), "the later one is not, so the walk stops");
    /// assert_eq!(walk.remaining(), later);
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

// The second channel bits a relay and an end device configuration share: index, data rate
// and acknowledgment offset.
fn second_channel_bits(index: u8, data_rate: u8, ack_offset: u8) -> u16 {
    (u16::from(index & 0x03) << 7)
        | (u16::from(data_rate & 0x0f) << 3)
        | u16::from(ack_offset & 0x07)
}

/// The coded signal strength a relay reports, TS011-1.0.1 sections 9.1 and 10.7: `-15 - code`
/// dBm, clamped to what seven bits carry.
///
/// # Arguments
///
/// * `rssi_dbm` - the strength in dBm.
///
/// # Returns
///
/// The code, 0 for -15 dBm or stronger and 127 for -142 dBm or weaker.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::mac::relay_rssi_code;
///
/// assert_eq!(relay_rssi_code(-15), 0);
/// assert_eq!(relay_rssi_code(-100), 85);
/// assert_eq!(relay_rssi_code(-160), 127, "clamped to the weakest");
/// assert_eq!(relay_rssi_code(0), 0, "clamped to the strongest");
/// ```
#[must_use]
pub fn relay_rssi_code(rssi_dbm: i16) -> u16 {
    (-15 - rssi_dbm.clamp(-142, -15)) as u16
}

/// The coded signal-to-noise ratio a relay reports, TS011-1.0.1 sections 9.1 and 10.7:
/// `code - 20` dB, clamped to what five bits carry.
///
/// # Arguments
///
/// * `snr_db` - the ratio in dB.
///
/// # Returns
///
/// The code, 0 for -20 dB or worse and 31 for 11 dB or better.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::mac::relay_snr_code;
///
/// assert_eq!(relay_snr_code(-20), 0);
/// assert_eq!(relay_snr_code(0), 20);
/// assert_eq!(relay_snr_code(15), 31, "clamped to the best");
/// ```
#[must_use]
pub fn relay_snr_code(snr_db: i8) -> u16 {
    (snr_db.clamp(-20, 11) + 20) as u16
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

    /// Encodes a command, checks it reads back the same in its direction, and hands back
    /// the bytes.
    fn round_trip(command: MacCommand) -> Vec<u8> {
        let mut out = [0u8; MAX_COMMAND];
        let len = command.encode(&mut out).expect("it encodes");
        assert_eq!(len, command.len());
        let (read, taken) = MacCommand::parse(command.direction(), &out[..len]).expect("it parses");
        assert_eq!(read, command, "what goes out reads back the same");
        assert_eq!(taken, len);
        out[..len].to_vec()
    }

    #[test]
    fn a_relay_configuration_packs_its_channel_settings_as_table_32_lays_them_out() {
        // StartStop bit 13, CADPeriodicity 12:10, DefaultChIdx 9, SecondChIdx 8:7,
        // SecondChDr 6:3, SecondChAckOffset 2:0, then the second frequency.
        let started = MacCommand::RelayConfReq {
            enabled: true,
            cad_periodicity: 2,
            default_channel_index: 1,
            second_channel_index: 1,
            second_channel_data_rate: 5,
            second_channel_ack_offset: 1,
            second_channel_frequency_hz: 868_100_000,
        };
        assert_eq!(round_trip(started), [0x40, 0xa9, 0x2a, 0x28, 0x76, 0x84]);

        let stopped = MacCommand::RelayConfReq {
            enabled: false,
            cad_periodicity: 0,
            default_channel_index: 0,
            second_channel_index: 0,
            second_channel_data_rate: 0,
            second_channel_ack_offset: 0,
            second_channel_frequency_hz: 0,
        };
        assert_eq!(round_trip(stopped), [0x40, 0, 0, 0, 0, 0]);

        let answer = MacCommand::RelayConfAns {
            cad_periodicity_ack: true,
            default_channel_index_ack: false,
            second_channel_index_ack: true,
            second_channel_data_rate_ack: true,
            second_channel_ack_offset_ack: false,
            second_channel_frequency_ack: true,
        };
        assert_eq!(round_trip(answer), [0x40, 0b0010_1101]);
    }

    #[test]
    fn an_end_device_configuration_and_its_answer_follow_tables_39_42_and_45() {
        let request = MacCommand::EndDeviceConfReq {
            relay_mode: 2,
            smart_enable_level: 1,
            back_off: 8,
            second_channel_index: 1,
            second_channel_data_rate: 3,
            second_channel_ack_offset: 2,
            second_channel_frequency_hz: 869_525_000,
        };
        assert_eq!(
            round_trip(request),
            [0x41, 0x09, 0x9a, 0x10, 0xd2, 0xad, 0x84]
        );

        // TS011-1.0.1 moved bit 3 from BackOffACK to SecondChAckOffsetACK.
        let answer = MacCommand::EndDeviceConfAns {
            second_channel_ack_offset_ack: true,
            second_channel_index_ack: false,
            second_channel_data_rate_ack: false,
            second_channel_frequency_ack: true,
        };
        assert_eq!(round_trip(answer), [0x41, 0b0000_1001]);
    }

    #[test]
    fn a_join_filter_carries_its_eui_prefix_low_byte_first() {
        // The rules of TS011-1.0.1 appendix 3.
        let oui = MacCommand::FilterListReq {
            index: 1,
            action: 1,
            eui_len: 3,
            eui: [0xab, 0xcd, 0xef, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        assert_eq!(round_trip(oui), [0x42, 0xa3, 0x00, 0xef, 0xcd, 0xab]);

        let range = MacCommand::FilterListReq {
            index: 3,
            action: 1,
            eui_len: 15,
            eui: [
                0xab, 0xcd, 0xef, 0xab, 0xcd, 0xef, 0xab, 0xcd, 0x12, 0x34, 0x56, 0x78, 0x28, 0x37,
                0x46, 0,
            ],
        };
        assert_eq!(
            round_trip(range),
            [
                0x42, 0xaf, 0x01, 0x46, 0x37, 0x28, 0x78, 0x56, 0x34, 0x12, 0xcd, 0xab, 0xef, 0xcd,
                0xab, 0xef, 0xcd, 0xab
            ]
        );

        let default_rule = MacCommand::FilterListReq {
            index: 0,
            action: 2,
            eui_len: 0,
            eui: [0; FILTER_EUI_MAX],
        };
        assert_eq!(round_trip(default_rule), [0x42, 0x40, 0x00]);

        let answer = MacCommand::FilterListAns {
            combined_rules_ack: true,
            eui_len_ack: true,
            action_ack: false,
        };
        assert_eq!(round_trip(answer), [0x42, 0b0000_0110]);
    }

    #[test]
    fn a_join_filter_longer_than_two_euis_is_read_so_it_can_be_refused_but_not_written() {
        let mut bytes = vec![0x42, 0x94, 0x01];
        bytes.extend(1..=20u8);
        let (read, taken) = MacCommand::parse(Direction::Downlink, &bytes).expect("it parses");
        assert_eq!(taken, 23, "every byte of the rule is stepped over");
        let MacCommand::FilterListReq { eui_len, eui, .. } = read else {
            panic!("a filter rule");
        };
        assert_eq!(eui_len, 20);
        assert_eq!(
            eui[0], 20,
            "the last byte on the air is the most significant"
        );

        let mut out = [0u8; MAX_COMMAND];
        assert_eq!(read.encode(&mut out), Err(LorawanError::MalformedFrame));

        assert_eq!(
            MacCommand::parse(Direction::Downlink, &[0x42, 0xa3, 0x00, 0xef]),
            Err(LorawanError::FrameTooShort),
            "three bytes promised, one given"
        );
    }

    #[test]
    fn a_trusted_end_device_carries_its_limit_address_counter_and_root_key() {
        let key: [u8; 16] = core::array::from_fn(|at| at as u8);
        let request = MacCommand::UpdateUplinkListReq {
            index: 3,
            reload_rate: 10,
            bucket_size: 2,
            dev_addr: 0x2601_1bda,
            wfcnt: 7,
            root_wor_s_key: key,
        };
        let mut want = vec![
            0x43, 0x03, 0x8a, 0xda, 0x1b, 0x01, 0x26, 0x07, 0x00, 0x00, 0x00,
        ];
        want.extend_from_slice(&key);
        assert_eq!(round_trip(request), want);
        assert_eq!(request.len(), MAX_COMMAND, "the longest command there is");
        assert_eq!(round_trip(MacCommand::UpdateUplinkListAns), [0x43]);

        assert_eq!(
            round_trip(MacCommand::CtrlUplinkListReq {
                index: 3,
                action: 1
            }),
            [0x44, 0x13]
        );
        assert_eq!(
            round_trip(MacCommand::CtrlUplinkListAns {
                index_ack: true,
                wfcnt: 0x0102_0304
            }),
            [0x44, 0x01, 0x04, 0x03, 0x02, 0x01]
        );
    }

    #[test]
    fn forwarding_limits_pack_four_reload_rates_and_four_bucket_sizes() {
        // Tables 62 and 65: ResetLimitCounters 29:28, JoinReq 27:21, Notify 20:14,
        // GlobalUplink 13:7, Overall 6:0; then JoinReq 7:6, Notify 5:4, GlobalUplink 3:2,
        // Overall 1:0.
        let request = MacCommand::ConfigureFwdLimitReq {
            reset_limit_counters: 3,
            join_request_reload_rate: 4,
            notify_reload_rate: 4,
            global_uplink_reload_rate: 8,
            overall_reload_rate: 127,
            join_request_bucket_size: 1,
            notify_bucket_size: 1,
            global_uplink_bucket_size: 2,
            overall_bucket_size: 0,
        };
        assert_eq!(round_trip(request), [0x45, 0x7f, 0x04, 0x81, 0x30, 0x58]);
        assert_eq!(round_trip(MacCommand::ConfigureFwdLimitAns), [0x45]);
    }

    #[test]
    fn a_new_end_device_notice_codes_strength_and_ratio_as_table_67_says() {
        let notice = MacCommand::NotifyNewEndDeviceReq {
            dev_addr: 0x2601_1bda,
            rssi_dbm: -100,
            snr_db: 5,
        };
        assert_eq!(
            round_trip(notice),
            [0x46, 0xda, 0x1b, 0x01, 0x26, 0xb9, 0x0a]
        );

        // Past what the fields carry, the closest value goes out.
        let mut out = [0u8; MAX_COMMAND];
        let len = MacCommand::NotifyNewEndDeviceReq {
            dev_addr: 1,
            rssi_dbm: -160,
            snr_db: 30,
        }
        .encode(&mut out)
        .expect("it encodes");
        let (read, _) = MacCommand::parse(Direction::Uplink, &out[..len]).expect("it parses");
        assert_eq!(
            read,
            MacCommand::NotifyNewEndDeviceReq {
                dev_addr: 1,
                rssi_dbm: -142,
                snr_db: 11
            }
        );

        assert_eq!(
            MacCommand::parse(Direction::Downlink, &[0x46, 0, 0, 0, 0, 0, 0]),
            Err(LorawanError::UnknownCommand(0x46)),
            "only a relay sends it"
        );
    }

    #[test]
    fn relay_commands_no_longer_stop_a_walk() {
        // A device that does not relay still reads past them to the commands after.
        let mut field = [0u8; 32];
        let len = encode_all(
            &[
                MacCommand::CtrlUplinkListReq {
                    index: 0,
                    action: 0,
                },
                MacCommand::DevStatusReq,
            ],
            &mut field,
        )
        .expect("they fit");
        let read: Vec<MacCommand> = MacCommands::new(Direction::Downlink, &field[..len])
            .map(|command| command.expect("each one parses"))
            .collect();
        assert_eq!(read.len(), 2);
        assert_eq!(read[1], MacCommand::DevStatusReq);
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
