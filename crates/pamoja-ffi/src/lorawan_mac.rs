//! The C ABI for the commands a LoRaWAN network and device configure each other with.
//!
//! A frame carries these either in its options field, at most
//! [`PAMOJA_LORAWAN_FOPTS_MAX`] bytes of them, or as a whole payload on port zero. A caller
//! that has either in hand walks it here.
//!
//! Every command crosses as one [`PamojaLorawanMacCommand`], a flat record whose `cid` says
//! which command it is and whose remaining fields carry whatever that command holds. The
//! rest read as zero. A command means a different thing in each direction, so the direction
//! goes in and there is no default.
//!
//! A command does not carry its own length, so a reader cannot step over one it does not
//! know. [`pamoja_lorawan_mac_count`] reports how many were read before that happened, and
//! the bytes after it are simply not readable.

use pamoja_lorawan::mac::{MacCommand, MacCommands};
use pamoja_lorawan::Direction;

use crate::lorawan::PamojaLorawanDirection;
use crate::{read_bytes, set_last_error, PamojaStatus};

/// How many bytes of commands a frame can carry beside a payload.
pub const PAMOJA_LORAWAN_FOPTS_MAX: usize = pamoja_lorawan::mac::FOPTS_MAX;

/// The longest single command, in bytes.
pub const PAMOJA_LORAWAN_MAC_MAX: usize = pamoja_lorawan::mac::MAX_COMMAND;

/// One command, with the fields of whichever command it is.
///
/// `cid` names the command and `direction` says which way it travels; together they decide
/// which of the other fields carry anything. The rest are zero.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanMacCommand {
    /// Which command this is.
    pub cid: u8,
    /// Which way it travels.
    pub direction: PamojaLorawanDirection,
    /// How far above the floor a link check arrived, in dB.
    pub margin: u8,
    /// How many gateways heard it.
    pub gateways: u8,
    /// The data rate a network asks a device to use.
    pub data_rate: u8,
    /// The transmit power it may use, as a ceiling.
    pub tx_power: u8,
    /// Which channels may carry an uplink.
    pub channel_mask: u16,
    /// Which block of sixteen channels that mask applies to.
    pub mask_control: u8,
    /// How many times to send an unconfirmed uplink.
    pub transmissions: u8,
    /// Whether the power was set, `1` for yes.
    pub power_ack: u8,
    /// Whether the data rate was set.
    pub data_rate_ack: u8,
    /// Whether the channel mask was usable.
    pub channel_mask_ack: u8,
    /// The share of the air a device is held to, as one over two to this.
    pub max_duty_cycle: u8,
    /// How far the first receive window sits below the uplink rate.
    pub rx1_offset: u8,
    /// The rate of the second receive window.
    pub rx2_data_rate: u8,
    /// A frequency in hertz, for the windows and the channel commands.
    pub frequency_hz: u32,
    /// Whether the window offset was in range.
    pub rx1_offset_ack: u8,
    /// Whether the window rate was known.
    pub rx2_data_rate_ack: u8,
    /// Whether the frequency was usable.
    pub channel_ack: u8,
    /// A device battery level: `0` on external power, `255` when it cannot tell.
    pub battery: u8,
    /// The signal-to-noise ratio of the last request, in dB.
    pub snr_margin: i8,
    /// Which channel a channel command names.
    pub index: u8,
    /// The fastest rate allowed on it.
    pub max_data_rate: u8,
    /// The slowest rate allowed on it.
    pub min_data_rate: u8,
    /// Whether the device can run that range of rates.
    pub data_rate_range_ok: u8,
    /// Whether its radio can reach that frequency.
    pub frequency_ok: u8,
    /// How long a device waits before its first receive window, as the command codes it.
    pub delay: u8,
    /// The coded transmit power ceiling a region imposes.
    pub max_eirp: u8,
    /// Whether an uplink is held to 400 ms of air time.
    pub uplink_dwell: u8,
    /// Whether a downlink is.
    pub downlink_dwell: u8,
    /// Whether the channel already had an uplink frequency to pair a downlink with.
    pub uplink_frequency_exists: u8,
    /// Seconds since the GPS epoch.
    pub seconds: u32,
    /// The fraction of that second, in steps of one part in 256.
    pub fraction: u8,
}

impl PamojaLorawanMacCommand {
    const fn blank(cid: u8, direction: PamojaLorawanDirection) -> PamojaLorawanMacCommand {
        PamojaLorawanMacCommand {
            cid,
            direction,
            margin: 0,
            gateways: 0,
            data_rate: 0,
            tx_power: 0,
            channel_mask: 0,
            mask_control: 0,
            transmissions: 0,
            power_ack: 0,
            data_rate_ack: 0,
            channel_mask_ack: 0,
            max_duty_cycle: 0,
            rx1_offset: 0,
            rx2_data_rate: 0,
            frequency_hz: 0,
            rx1_offset_ack: 0,
            rx2_data_rate_ack: 0,
            channel_ack: 0,
            battery: 0,
            snr_margin: 0,
            index: 0,
            max_data_rate: 0,
            min_data_rate: 0,
            data_rate_range_ok: 0,
            frequency_ok: 0,
            delay: 0,
            max_eirp: 0,
            uplink_dwell: 0,
            downlink_dwell: 0,
            uplink_frequency_exists: 0,
            seconds: 0,
            fraction: 0,
        }
    }
}

fn crossing(direction: PamojaLorawanDirection) -> Direction {
    match direction {
        PamojaLorawanDirection::Uplink => Direction::Uplink,
        PamojaLorawanDirection::Downlink => Direction::Downlink,
    }
}

fn travelling(direction: Direction) -> PamojaLorawanDirection {
    match direction {
        Direction::Uplink => PamojaLorawanDirection::Uplink,
        Direction::Downlink => PamojaLorawanDirection::Downlink,
    }
}

// One command, flattened into the record that crosses the boundary.
fn flatten(command: MacCommand) -> PamojaLorawanMacCommand {
    let mut flat = PamojaLorawanMacCommand::blank(command.cid(), travelling(command.direction()));
    match command {
        MacCommand::LinkCheckReq
        | MacCommand::DutyCycleAns
        | MacCommand::DevStatusReq
        | MacCommand::RxTimingSetupAns
        | MacCommand::TxParamSetupAns
        | MacCommand::DeviceTimeReq => {}
        MacCommand::LinkCheckAns { margin, gateways } => {
            flat.margin = margin;
            flat.gateways = gateways;
        }
        MacCommand::LinkAdrReq {
            data_rate,
            tx_power,
            channel_mask,
            mask_control,
            transmissions,
        } => {
            flat.data_rate = data_rate;
            flat.tx_power = tx_power;
            flat.channel_mask = channel_mask;
            flat.mask_control = mask_control;
            flat.transmissions = transmissions;
        }
        MacCommand::LinkAdrAns {
            power_ack,
            data_rate_ack,
            channel_mask_ack,
        } => {
            flat.power_ack = u8::from(power_ack);
            flat.data_rate_ack = u8::from(data_rate_ack);
            flat.channel_mask_ack = u8::from(channel_mask_ack);
        }
        MacCommand::DutyCycleReq { max_duty_cycle } => flat.max_duty_cycle = max_duty_cycle,
        MacCommand::RxParamSetupReq {
            rx1_offset,
            rx2_data_rate,
            frequency_hz,
        } => {
            flat.rx1_offset = rx1_offset;
            flat.rx2_data_rate = rx2_data_rate;
            flat.frequency_hz = frequency_hz;
        }
        MacCommand::RxParamSetupAns {
            rx1_offset_ack,
            rx2_data_rate_ack,
            channel_ack,
        } => {
            flat.rx1_offset_ack = u8::from(rx1_offset_ack);
            flat.rx2_data_rate_ack = u8::from(rx2_data_rate_ack);
            flat.channel_ack = u8::from(channel_ack);
        }
        MacCommand::DevStatusAns { battery, margin } => {
            flat.battery = battery;
            flat.snr_margin = margin;
        }
        MacCommand::NewChannelReq {
            index,
            frequency_hz,
            max_data_rate,
            min_data_rate,
        } => {
            flat.index = index;
            flat.frequency_hz = frequency_hz;
            flat.max_data_rate = max_data_rate;
            flat.min_data_rate = min_data_rate;
        }
        MacCommand::NewChannelAns {
            data_rate_range_ok,
            frequency_ok,
        } => {
            flat.data_rate_range_ok = u8::from(data_rate_range_ok);
            flat.frequency_ok = u8::from(frequency_ok);
        }
        MacCommand::RxTimingSetupReq { delay } => flat.delay = delay,
        MacCommand::TxParamSetupReq {
            max_eirp,
            uplink_dwell,
            downlink_dwell,
        } => {
            flat.max_eirp = max_eirp;
            flat.uplink_dwell = u8::from(uplink_dwell);
            flat.downlink_dwell = u8::from(downlink_dwell);
        }
        MacCommand::DlChannelReq {
            index,
            frequency_hz,
        } => {
            flat.index = index;
            flat.frequency_hz = frequency_hz;
        }
        MacCommand::DlChannelAns {
            uplink_frequency_exists,
            frequency_ok,
        } => {
            flat.uplink_frequency_exists = u8::from(uplink_frequency_exists);
            flat.frequency_ok = u8::from(frequency_ok);
        }
        MacCommand::DeviceTimeAns { seconds, fraction } => {
            flat.seconds = seconds;
            flat.fraction = fraction;
        }
    }
    flat
}

// The command a record stands for, or nothing when the identifier and the direction do not
// name one.
fn sharpen(flat: &PamojaLorawanMacCommand) -> Option<MacCommand> {
    use pamoja_lorawan::mac;

    let down = matches!(flat.direction, PamojaLorawanDirection::Downlink);
    let on = |value: u8| value != 0;

    let command = match (flat.cid, down) {
        (mac::CID_LINK_CHECK, false) => MacCommand::LinkCheckReq,
        (mac::CID_LINK_CHECK, true) => MacCommand::LinkCheckAns {
            margin: flat.margin,
            gateways: flat.gateways,
        },
        (mac::CID_LINK_ADR, true) => MacCommand::LinkAdrReq {
            data_rate: flat.data_rate,
            tx_power: flat.tx_power,
            channel_mask: flat.channel_mask,
            mask_control: flat.mask_control,
            transmissions: flat.transmissions,
        },
        (mac::CID_LINK_ADR, false) => MacCommand::LinkAdrAns {
            power_ack: on(flat.power_ack),
            data_rate_ack: on(flat.data_rate_ack),
            channel_mask_ack: on(flat.channel_mask_ack),
        },
        (mac::CID_DUTY_CYCLE, true) => MacCommand::DutyCycleReq {
            max_duty_cycle: flat.max_duty_cycle,
        },
        (mac::CID_DUTY_CYCLE, false) => MacCommand::DutyCycleAns,
        (mac::CID_RX_PARAM_SETUP, true) => MacCommand::RxParamSetupReq {
            rx1_offset: flat.rx1_offset,
            rx2_data_rate: flat.rx2_data_rate,
            frequency_hz: flat.frequency_hz,
        },
        (mac::CID_RX_PARAM_SETUP, false) => MacCommand::RxParamSetupAns {
            rx1_offset_ack: on(flat.rx1_offset_ack),
            rx2_data_rate_ack: on(flat.rx2_data_rate_ack),
            channel_ack: on(flat.channel_ack),
        },
        (mac::CID_DEV_STATUS, true) => MacCommand::DevStatusReq,
        (mac::CID_DEV_STATUS, false) => MacCommand::DevStatusAns {
            battery: flat.battery,
            margin: flat.snr_margin,
        },
        (mac::CID_NEW_CHANNEL, true) => MacCommand::NewChannelReq {
            index: flat.index,
            frequency_hz: flat.frequency_hz,
            max_data_rate: flat.max_data_rate,
            min_data_rate: flat.min_data_rate,
        },
        (mac::CID_NEW_CHANNEL, false) => MacCommand::NewChannelAns {
            data_rate_range_ok: on(flat.data_rate_range_ok),
            frequency_ok: on(flat.frequency_ok),
        },
        (mac::CID_RX_TIMING_SETUP, true) => MacCommand::RxTimingSetupReq { delay: flat.delay },
        (mac::CID_RX_TIMING_SETUP, false) => MacCommand::RxTimingSetupAns,
        (mac::CID_TX_PARAM_SETUP, true) => MacCommand::TxParamSetupReq {
            max_eirp: flat.max_eirp,
            uplink_dwell: on(flat.uplink_dwell),
            downlink_dwell: on(flat.downlink_dwell),
        },
        (mac::CID_TX_PARAM_SETUP, false) => MacCommand::TxParamSetupAns,
        (mac::CID_DL_CHANNEL, true) => MacCommand::DlChannelReq {
            index: flat.index,
            frequency_hz: flat.frequency_hz,
        },
        (mac::CID_DL_CHANNEL, false) => MacCommand::DlChannelAns {
            uplink_frequency_exists: on(flat.uplink_frequency_exists),
            frequency_ok: on(flat.frequency_ok),
        },
        (mac::CID_DEVICE_TIME, false) => MacCommand::DeviceTimeReq,
        (mac::CID_DEVICE_TIME, true) => MacCommand::DeviceTimeAns {
            seconds: flat.seconds,
            fraction: flat.fraction,
        },
        _ => return None,
    };
    Some(command)
}

/// Counts the commands packed into a field.
///
/// # Arguments
///
/// * `direction` - which way the frame carrying them travels.
/// * `bytes` - the options field, or a payload sent on port zero.
/// * `len` - how many bytes that is.
/// * `out_count` - where to put the count.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], with the count written. Counting stops at an identifier this build
/// does not know, because nothing says how long it is, so the count is what was readable.
///
/// # Safety
///
/// `bytes` must point to `len` readable bytes or be null when `len` is zero, and `out_count`
/// must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_mac_count(
    direction: PamojaLorawanDirection,
    bytes: *const u8,
    len: usize,
    out_count: *mut usize,
) -> PamojaStatus {
    if out_count.is_null() {
        set_last_error("the count must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let field = match read_bytes(bytes, len) {
        Ok(field) => field,
        Err(status) => return status,
    };
    let read = MacCommands::new(crossing(direction), &field)
        .take_while(Result::is_ok)
        .count();
    *out_count = read;
    PamojaStatus::Ok
}

/// Reads one of the commands packed into a field.
///
/// # Arguments
///
/// * `direction` - which way the frame carrying them travels.
/// * `bytes` - the options field, or a payload sent on port zero.
/// * `len` - how many bytes that is.
/// * `index` - which command, counting from zero.
/// * `out_command` - where to put it.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the command written, or
/// [`PamojaStatus::InvalidArgument`] when there is no command at that position, which
/// includes one cut short and one this build does not know.
///
/// # Safety
///
/// `bytes` must point to `len` readable bytes or be null when `len` is zero, and
/// `out_command` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_mac_at(
    direction: PamojaLorawanDirection,
    bytes: *const u8,
    len: usize,
    index: usize,
    out_command: *mut PamojaLorawanMacCommand,
) -> PamojaStatus {
    if out_command.is_null() {
        set_last_error("the command must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let field = match read_bytes(bytes, len) {
        Ok(field) => field,
        Err(status) => return status,
    };
    let found = MacCommands::new(crossing(direction), &field)
        .take_while(Result::is_ok)
        .nth(index)
        .and_then(Result::ok);
    match found {
        Some(command) => {
            *out_command = flatten(command);
            PamojaStatus::Ok
        }
        None => {
            set_last_error(format!("there is no readable command at position {index}"));
            PamojaStatus::InvalidArgument
        }
    }
}

/// Writes one command out.
///
/// # Arguments
///
/// * `command` - the command to write.
/// * `out` - where to write it.
/// * `capacity` - how much room that is, at least [`PAMOJA_LORAWAN_MAC_MAX`] for any
///   command.
/// * `out_written` - where to put how many bytes were written.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the bytes written, or [`PamojaStatus::InvalidArgument`] when
/// the identifier and direction name no command, a field will not fit what it is carried in,
/// or there is not enough room.
///
/// # Safety
///
/// `command` must be readable, `out` must point to `capacity` writable bytes, and
/// `out_written` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_mac_encode(
    command: *const PamojaLorawanMacCommand,
    out: *mut u8,
    capacity: usize,
    out_written: *mut usize,
) -> PamojaStatus {
    if command.is_null() || out.is_null() || out_written.is_null() {
        set_last_error("the command, the buffer, and the count must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(command) = sharpen(&*command) else {
        set_last_error(format!(
            "identifier {:#04x} names no command in that direction",
            (*command).cid
        ));
        return PamojaStatus::InvalidArgument;
    };

    let mut buffer = [0u8; PAMOJA_LORAWAN_MAC_MAX];
    let written = match command.encode(&mut buffer) {
        Ok(written) => written,
        Err(error) => {
            set_last_error(error.to_string());
            return PamojaStatus::InvalidArgument;
        }
    };
    if written > capacity {
        set_last_error(format!(
            "the command takes {written} bytes and there is room for {capacity}"
        ));
        return PamojaStatus::InvalidArgument;
    }

    std::ptr::copy_nonoverlapping(buffer.as_ptr(), out, written);
    *out_written = written;
    PamojaStatus::Ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(
        direction: PamojaLorawanDirection,
        bytes: &[u8],
        index: usize,
    ) -> PamojaLorawanMacCommand {
        let mut command = PamojaLorawanMacCommand::blank(0, PamojaLorawanDirection::Uplink);
        let status = unsafe {
            pamoja_lorawan_mac_at(direction, bytes.as_ptr(), bytes.len(), index, &mut command)
        };
        assert_eq!(status, PamojaStatus::Ok);
        command
    }

    #[test]
    fn the_same_bytes_read_as_different_commands_in_each_direction() {
        let bytes = [0x03, 0x52, 0xff, 0x00, 0x01];

        let down = at(PamojaLorawanDirection::Downlink, &bytes, 0);
        assert_eq!(down.data_rate, 5);
        assert_eq!(down.tx_power, 2);
        assert_eq!(down.channel_mask, 0x00ff);

        let up = at(PamojaLorawanDirection::Uplink, &bytes, 0);
        assert_eq!(up.cid, down.cid, "the same identifier");
        assert_eq!(up.data_rate_ack, 1, "read as a status byte instead");
        assert_eq!(up.data_rate, 0, "and the request fields carry nothing");
    }

    #[test]
    fn a_field_of_commands_is_counted_and_read_one_by_one() {
        let bytes = [0x02, 0x0d];
        let mut count = 0usize;
        let status = unsafe {
            pamoja_lorawan_mac_count(
                PamojaLorawanDirection::Uplink,
                bytes.as_ptr(),
                bytes.len(),
                &mut count,
            )
        };

        assert_eq!(status, PamojaStatus::Ok);
        assert_eq!(count, 2);
        assert_eq!(at(PamojaLorawanDirection::Uplink, &bytes, 0).cid, 0x02);
        assert_eq!(at(PamojaLorawanDirection::Uplink, &bytes, 1).cid, 0x0d);
    }

    #[test]
    fn counting_stops_where_reading_does() {
        // Nothing says how long an unknown command is, so neither the count nor a read goes
        // past it.
        let bytes = [0x02, 0x7f, 0x11];
        let mut count = 0usize;
        unsafe {
            pamoja_lorawan_mac_count(
                PamojaLorawanDirection::Uplink,
                bytes.as_ptr(),
                bytes.len(),
                &mut count,
            );
        }
        assert_eq!(count, 1);

        let mut command = PamojaLorawanMacCommand::blank(0, PamojaLorawanDirection::Uplink);
        let status = unsafe {
            pamoja_lorawan_mac_at(
                PamojaLorawanDirection::Uplink,
                bytes.as_ptr(),
                bytes.len(),
                1,
                &mut command,
            )
        };
        assert_eq!(status, PamojaStatus::InvalidArgument);
    }

    #[test]
    fn a_command_written_out_reads_back_as_itself() {
        let mut command = PamojaLorawanMacCommand::blank(0x07, PamojaLorawanDirection::Downlink);
        command.index = 3;
        command.frequency_hz = 867_100_000;
        command.max_data_rate = 5;
        command.min_data_rate = 0;

        let mut out = [0u8; PAMOJA_LORAWAN_MAC_MAX];
        let mut written = 0usize;
        let status = unsafe {
            pamoja_lorawan_mac_encode(&command, out.as_mut_ptr(), out.len(), &mut written)
        };

        assert_eq!(status, PamojaStatus::Ok);
        assert_eq!(&out[..written], &[0x07, 0x03, 0x18, 0x4f, 0x84, 0x50]);
        assert_eq!(
            at(PamojaLorawanDirection::Downlink, &out[..written], 0),
            command
        );
    }

    #[test]
    fn an_identifier_that_names_no_command_is_refused() {
        let command = PamojaLorawanMacCommand::blank(0x7f, PamojaLorawanDirection::Downlink);
        let mut out = [0u8; PAMOJA_LORAWAN_MAC_MAX];
        let mut written = 0usize;
        let status = unsafe {
            pamoja_lorawan_mac_encode(&command, out.as_mut_ptr(), out.len(), &mut written)
        };

        assert_eq!(status, PamojaStatus::InvalidArgument);
    }

    #[test]
    fn a_buffer_too_small_for_the_command_is_refused() {
        let mut command = PamojaLorawanMacCommand::blank(0x02, PamojaLorawanDirection::Downlink);
        command.margin = 20;
        command.gateways = 3;

        let mut out = [0u8; 2];
        let mut written = 0usize;
        let status = unsafe {
            pamoja_lorawan_mac_encode(&command, out.as_mut_ptr(), out.len(), &mut written)
        };

        assert_eq!(status, PamojaStatus::InvalidArgument);
        assert_eq!(written, 0, "and nothing was written");
    }
}
