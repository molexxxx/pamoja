//! The C ABI for the network side of one site.
//!
//! These functions wrap [`pamoja_gateway::network`] for callers that reach the SDK through
//! the flat C boundary. A network holds the devices it admits, the sessions it has granted,
//! and the counters it has seen, so it crosses as an opaque handle: open one on a channel
//! plan, register the devices it should admit, hand it each packet a gateway forwarded, and
//! release it with [`pamoja_gateway_network_free`].
//!
//! A forwarded packet turns out to be one of three things, so the answer crosses as a flat
//! event with an outcome byte that says which: a device joined and its accept is ready to
//! transmit, a session frame arrived and was decrypted into the caller buffer, or the frame
//! belongs to a network this site never granted, which a gateway hears all the time and is
//! not an error.
//!
//! Where and when to answer comes back with the event, so a downlink is built for the same
//! window the uplink opened without the caller working out any of it.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

use pamoja_gateway::network::{Event, Network, NetworkError, Registration, Rx1Channels, Slot};
use pamoja_gateway::udp::Rxpk;
use pamoja_lora::region::ChannelBlock;
use pamoja_lorawan::relay::WorChannel;

use crate::gateway::{
    link_to_c, missing, rxpk_of, txpk_to_c, PamojaGatewayRxpk, PamojaGatewayTxpk,
};
use crate::lora::PamojaLoraLink;
use crate::lora_region::PamojaLoraPlan;
use crate::lorawan_mac::{read_command, write_command, PamojaLorawanMacCommand};
use crate::{read_bytes, set_last_error, PamojaStatus};

/// A device joined, and its accept is in the event.
pub const PAMOJA_GATEWAY_NETWORK_JOINED: u8 = 0;

/// A session frame arrived, decrypted into the caller buffer.
pub const PAMOJA_GATEWAY_NETWORK_DATA: u8 = 1;

/// The frame belongs to a device this site never granted.
pub const PAMOJA_GATEWAY_NETWORK_FOREIGN: u8 = 2;

/// The first receive window answers on the frequency the uplink arrived on.
pub const PAMOJA_GATEWAY_NETWORK_RX1_SAME: u8 = 0;

/// The first receive window answers on a run of downlink channels, chosen by the uplink
/// channel number modulo how many the run holds.
pub const PAMOJA_GATEWAY_NETWORK_RX1_DOWNSTREAM: u8 = 1;

/// The first receive window answers where the network's channel plan says: on the uplink
/// frequency in a dynamic plan, and on the downlink channel a fixed plan numbers for the
/// uplink channel.
pub const PAMOJA_GATEWAY_NETWORK_RX1_PLAN: u8 = 2;

/// The delay before the first receive window, in microseconds.
pub const PAMOJA_GATEWAY_NETWORK_RECEIVE_DELAY_US: u32 = pamoja_gateway::network::RECEIVE_DELAY1_US;

/// The delay before the window a join accept is sent in, in microseconds.
pub const PAMOJA_GATEWAY_NETWORK_JOIN_DELAY_US: u32 =
    pamoja_gateway::network::JOIN_ACCEPT_DELAY1_US;

/// The network side of one site, released with [`pamoja_gateway_network_free`].
pub struct PamojaGatewayNetwork {
    network: Network,
}

/// When and where a network answers, and at what rate.
///
/// The recommended values are the delays above, no offset between the uplink data rate and
/// the downlink one, and a first window where the channel plan says.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PamojaGatewayNetworkWindows {
    /// The delay before the first receive window, in microseconds.
    pub receive_delay_us: u32,
    /// The delay before the window a join accept is sent in, in microseconds.
    pub join_delay_us: u32,
    /// The offset between the uplink data rate and the rate the first window answers at.
    pub rx1_data_rate_offset: u8,
    /// [`PAMOJA_GATEWAY_NETWORK_RX1_PLAN`], [`PAMOJA_GATEWAY_NETWORK_RX1_SAME`] or
    /// [`PAMOJA_GATEWAY_NETWORK_RX1_DOWNSTREAM`].
    pub rx1_channels: u8,
    /// The first downlink channel, in hertz, when the channels are downstream.
    pub downstream_start_hz: u32,
    /// The spacing between those channels, in hertz.
    pub downstream_step_hz: u32,
    /// How many there are.
    pub downstream_count: u16,
}

/// Where and when a downlink answers an uplink, in the concentrator's own terms.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PamojaGatewayNetworkSlot {
    /// The concentrator timestamp to transmit at, in microseconds.
    pub timestamp_us: u32,
    /// The frequency to transmit on, in hertz.
    pub frequency_hz: u32,
    /// The settings to transmit with.
    pub link: PamojaLoraLink,
}

/// What a forwarded packet turned out to be.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PamojaGatewayNetworkEvent {
    /// [`PAMOJA_GATEWAY_NETWORK_JOINED`], [`PAMOJA_GATEWAY_NETWORK_DATA`], or
    /// [`PAMOJA_GATEWAY_NETWORK_FOREIGN`].
    pub outcome: u8,
    /// The device that joined, for a join.
    pub dev_eui: [u8; 8],
    /// The address granted, or the address a frame claimed.
    pub dev_addr: u32,
    /// The counter the frame carried, reconstructed to its full width, for data.
    pub fcnt: u32,
    /// The port the frame was sent on, for data on a port.
    pub fport: u8,
    /// Whether the frame carried a port at all.
    pub has_fport: bool,
    /// Whether the device asked to be acknowledged.
    pub confirmed: bool,
    /// How many bytes were written into the caller buffer: the decrypted payload for data,
    /// and the accept to transmit for a join.
    pub len: usize,
    /// Whether the payload was longer than the buffer, in which case nothing was written.
    pub truncated: bool,
    /// Where an answer goes, for a join or for data.
    pub slot: PamojaGatewayNetworkSlot,
    /// The packet that carries the accept, for a join.
    pub accept: PamojaGatewayTxpk,
    /// Whether a relay forwarded this uplink, TS011-1.0.1 section 9.1. An answer to it goes
    /// back through the same relay, which [`pamoja_gateway_network_answer`] does by itself.
    pub relayed: bool,
    /// The relay that forwarded it.
    pub relay_dev_addr: u32,
    /// What the relay heard of the uplink, in dBm.
    pub relay_rssi_dbm: i16,
    /// Its signal-to-noise ratio, in dB.
    pub relay_snr_db: i8,
    /// The data rate it arrived at.
    pub relay_data_rate: u8,
    /// The WOR channel the device woke the relay on: 0 for the default, 1 for the second.
    pub relay_wor_channel: u8,
    /// The frequency the uplink arrived on, in hertz.
    pub relay_frequency_hz: u32,
}

/// Returns the windows a network answers in by default.
///
/// # Returns
///
/// The recommended delays, no data-rate offset, and a first window where the plan says.
#[no_mangle]
pub extern "C" fn pamoja_gateway_network_windows_default() -> PamojaGatewayNetworkWindows {
    PamojaGatewayNetworkWindows {
        receive_delay_us: PAMOJA_GATEWAY_NETWORK_RECEIVE_DELAY_US,
        join_delay_us: PAMOJA_GATEWAY_NETWORK_JOIN_DELAY_US,
        rx1_data_rate_offset: 0,
        rx1_channels: PAMOJA_GATEWAY_NETWORK_RX1_PLAN,
        downstream_start_hz: 0,
        downstream_step_hz: 0,
        downstream_count: 0,
    }
}

/// Opens the network side of a site on a channel plan.
///
/// # Arguments
///
/// * `plan` - the band this site operates in, which is copied into the network.
/// * `net_id` - the network identifier granted addresses carry; only its low 24 bits travel.
/// * `windows` - when and where to answer, from
///   [`pamoja_gateway_network_windows_default`].
/// * `first_dev_addr` - the first address to grant; later joins take the ones after it.
/// * `out_network` - receives the handle.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null plan or output.
///
/// # Safety
///
/// `plan` must be a live handle from the region calls, and `out_network` must point at a
/// writable pointer.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_network_open(
    plan: *const PamojaLoraPlan,
    net_id: u32,
    windows: PamojaGatewayNetworkWindows,
    first_dev_addr: u32,
    out_network: *mut *mut PamojaGatewayNetwork,
) -> PamojaStatus {
    if out_network.is_null() {
        return missing("out_network");
    }
    *out_network = ptr::null_mut();
    let Some(plan) = plan.as_ref() else {
        return missing("plan");
    };

    let network = plan.with(|plan| {
        Network::new(plan, net_id)
            .with_windows(windows_of(windows))
            .with_first_dev_addr(first_dev_addr)
    });
    *out_network = Box::into_raw(Box::new(PamojaGatewayNetwork { network }));
    PamojaStatus::Ok
}

/// Admits a device, so a join request signed with its key is accepted.
///
/// # Arguments
///
/// * `network` - the network.
/// * `dev_eui` - the device identifier, eight bytes.
/// * `app_eui` - the application identifier, eight bytes.
/// * `app_key` - the root key, sixteen bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument.
///
/// # Safety
///
/// `network` must be a live handle, and each identifier must point at its own length in
/// readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_network_register(
    network: *mut PamojaGatewayNetwork,
    dev_eui: *const u8,
    app_eui: *const u8,
    app_key: *const u8,
) -> PamojaStatus {
    let Some(network) = network.as_mut() else {
        return missing("network");
    };
    let (Some(dev_eui), Some(app_eui)) = (eight(dev_eui), eight(app_eui)) else {
        return missing("dev_eui and app_eui");
    };
    let Some(app_key) = sixteen(app_key) else {
        return missing("app_key");
    };

    network
        .network
        .register(Registration::new(dev_eui, app_eui, app_key));
    PamojaStatus::Ok
}

/// Reads a packet the gateway forwarded.
///
/// A join request is verified against every registered key, granted an address and a session,
/// and answered with an accept written into the buffer and described by `event.accept`. A
/// data frame is routed by its address, checked against the counter last seen, and decrypted
/// into the buffer. A frame for a device this site never granted is reported rather than
/// refused, because a gateway hears every network in range.
///
/// # Arguments
///
/// * `network` - the network.
/// * `packet` - the metadata of what the gateway heard.
/// * `payload` - the frame as it came off the air.
/// * `payload_len` - its length.
/// * `buffer` - where to write the decrypted payload, or the accept to transmit.
/// * `capacity` - how many bytes the buffer holds.
/// * `out_event` - receives what the packet turned out to be.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], [`PamojaStatus::InvalidArgument`] for a null argument, or
/// [`PamojaStatus::Codec`] when the frame is refused, whose reason is available from
/// `pamoja_last_error_message`.
///
/// # Safety
///
/// `network` must be a live handle, `payload` must point at `payload_len` readable bytes or
/// be null, `buffer` must point at `capacity` writable bytes or be null, and `out_event` must
/// point at a writable event.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_network_uplink(
    network: *mut PamojaGatewayNetwork,
    packet: PamojaGatewayRxpk,
    payload: *const u8,
    payload_len: usize,
    buffer: *mut u8,
    capacity: usize,
    out_event: *mut PamojaGatewayNetworkEvent,
) -> PamojaStatus {
    if out_event.is_null() {
        return missing("out_event");
    }
    *out_event = PamojaGatewayNetworkEvent::default();
    let Some(network) = network.as_mut() else {
        return missing("network");
    };
    let payload = match read_bytes(payload, payload_len) {
        Ok(payload) => payload,
        Err(status) => return status,
    };
    let heard: Rxpk = rxpk_of(packet, payload);

    let read = catch_unwind(AssertUnwindSafe(|| network.network.uplink(&heard)));
    let event = match read {
        Ok(Ok(event)) => event,
        Ok(Err(error)) => return refused(&error),
        Err(_) => {
            set_last_error("a panic was caught while reading the packet".to_owned());
            return PamojaStatus::Panic;
        }
    };

    *out_event = event_to_c(event, buffer, capacity);
    PamojaStatus::Ok
}

/// Builds a downlink for a device, encrypted with its session.
///
/// # Arguments
///
/// * `network` - the network.
/// * `dev_addr` - the device to answer.
/// * `slot` - where and when to transmit, from the event that reported the uplink.
/// * `fport` - the port to answer on.
/// * `payload` - what to send.
/// * `payload_len` - its length.
/// * `buffer` - where to write the frame to transmit.
/// * `capacity` - how many bytes the buffer holds.
/// * `out_txpk` - receives the packet that carries it.
/// * `out_len` - receives how many bytes were written.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], [`PamojaStatus::InvalidArgument`] for a null argument or a buffer
/// too small, or [`PamojaStatus::Codec`] when no session is held for the address.
///
/// # Safety
///
/// `network` must be a live handle, `payload` must point at `payload_len` readable bytes or
/// be null, `buffer` must point at `capacity` writable bytes or be null, and both outputs
/// must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_network_answer(
    network: *mut PamojaGatewayNetwork,
    dev_addr: u32,
    slot: PamojaGatewayNetworkSlot,
    fport: u8,
    payload: *const u8,
    payload_len: usize,
    buffer: *mut u8,
    capacity: usize,
    out_txpk: *mut PamojaGatewayTxpk,
    out_len: *mut usize,
) -> PamojaStatus {
    if out_txpk.is_null() || out_len.is_null() {
        return missing("out_txpk and out_len");
    }
    *out_txpk = PamojaGatewayTxpk::default();
    *out_len = 0;
    let Some(network) = network.as_mut() else {
        return missing("network");
    };
    let payload = match read_bytes(payload, payload_len) {
        Ok(payload) => payload,
        Err(status) => return status,
    };

    let answered = network
        .network
        .answer(dev_addr, slot_of(slot), fport, &payload);
    let downlink = match answered {
        Ok(downlink) => downlink,
        Err(error) => return refused(&error),
    };

    if !write_payload(&downlink.payload, buffer, capacity) {
        set_last_error("the frame does not fit the buffer".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_len = downlink.payload.len();
    *out_txpk = txpk_to_c(&downlink);
    PamojaStatus::Ok
}

/// A device a relay heard and could not verify, TS011-1.0.1 section 10.7.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PamojaGatewayNetworkNotice {
    /// The relay that heard it.
    pub relay: u32,
    /// The address the wake-on-radio frame named.
    pub dev_addr: u32,
    /// The frame's signal strength in dBm.
    pub rssi_dbm: i16,
    /// Its signal-to-noise ratio in dB.
    pub snr_db: i8,
}

/// Takes the devices relays have reported hearing and could not verify.
///
/// A relay carries these in the MAC commands of its own uplinks, alongside whatever else that
/// uplink was for, so they wait in the network until they are read. Every notice is taken:
/// a second call returns only what arrived since the first.
///
/// # Arguments
///
/// * `network` - the network.
/// * `out_notices` - where to write them, or null to count what is waiting.
/// * `capacity` - how many notices that buffer holds.
/// * `out_len` - receives how many are waiting, which may be more than were written.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] when every notice was written, and
/// [`PamojaStatus::InvalidArgument`] when the buffer was too small, in which case none were
/// taken and `*out_len` says how many there are.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null handle or out pointer.
///
/// # Safety
///
/// `network` must be a live handle, `out_notices` must point at `capacity` writable notices
/// or be null, and `out_len` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_network_notices(
    network: *mut PamojaGatewayNetwork,
    out_notices: *mut PamojaGatewayNetworkNotice,
    capacity: usize,
    out_len: *mut usize,
) -> PamojaStatus {
    if out_len.is_null() {
        return missing("out_len");
    }
    *out_len = 0;
    let Some(network) = network.as_mut() else {
        return missing("network");
    };
    let waiting = network.network.notice_count();
    *out_len = waiting;
    if out_notices.is_null() || capacity < waiting {
        if waiting == 0 {
            return PamojaStatus::Ok;
        }
        set_last_error(format!("the buffer holds {capacity} of {waiting} notices"));
        return PamojaStatus::InvalidArgument;
    }
    for (index, notice) in network.network.notices().into_iter().enumerate() {
        *out_notices.add(index) = PamojaGatewayNetworkNotice {
            relay: notice.relay,
            dev_addr: notice.dev_addr,
            rssi_dbm: notice.rssi_dbm,
            snr_db: notice.snr_db,
        };
    }
    PamojaStatus::Ok
}

/// Builds a downlink carrying MAC commands, which is how a network configures a relay.
///
/// The commands ride in the frame options where they fit, and in a frame of their own on
/// port 0 where they do not.
///
/// # Arguments
///
/// * `network` - the network.
/// * `dev_addr` - the device to configure.
/// * `slot` - where and when to transmit, from the event that reported its uplink.
/// * `commands` - the commands to send.
/// * `commands_len` - how many there are.
/// * `buffer` - where to write the frame to transmit.
/// * `capacity` - how many bytes the buffer holds.
/// * `out_txpk` - receives the packet that carries it.
/// * `out_len` - receives how many bytes were written.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], [`PamojaStatus::InvalidArgument`] for a null argument, a command
/// the boundary cannot read, or a buffer too small, or [`PamojaStatus::Codec`] when no
/// session is held for the address.
///
/// # Safety
///
/// `network` must be a live handle, `commands` must point at `commands_len` readable
/// commands, `buffer` must point at `capacity` writable bytes or be null, and both outputs
/// must be writable.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn pamoja_gateway_network_command(
    network: *mut PamojaGatewayNetwork,
    dev_addr: u32,
    slot: PamojaGatewayNetworkSlot,
    commands: *const PamojaLorawanMacCommand,
    commands_len: usize,
    buffer: *mut u8,
    capacity: usize,
    out_txpk: *mut PamojaGatewayTxpk,
    out_len: *mut usize,
) -> PamojaStatus {
    if out_txpk.is_null() || out_len.is_null() {
        return missing("out_txpk and out_len");
    }
    *out_txpk = PamojaGatewayTxpk::default();
    *out_len = 0;
    let (Some(network), false) = (network.as_mut(), commands.is_null() && commands_len > 0) else {
        return missing("network and commands");
    };
    let mut built = Vec::with_capacity(commands_len);
    for index in 0..commands_len {
        let Some(command) = read_command(&*commands.add(index)) else {
            set_last_error(format!("command {index} is not one this build writes"));
            return PamojaStatus::InvalidArgument;
        };
        built.push(command);
    }

    let downlink = match network.network.command(dev_addr, slot_of(slot), &built) {
        Ok(downlink) => downlink,
        Err(error) => return refused(&error),
    };
    if !write_payload(&downlink.payload, buffer, capacity) {
        set_last_error("the frame does not fit the buffer".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_len = downlink.payload.len();
    *out_txpk = txpk_to_c(&downlink);
    PamojaStatus::Ok
}

/// Builds the command that tells a relay to trust a device, with the key that lets it verify
/// the device's wake-on-radio frames, TS011-1.0.1 section 10.4.
///
/// # Arguments
///
/// * `network` - the network.
/// * `dev_addr` - the device the relay should forward for.
/// * `index` - the entry in the relay's list, 0 to 15.
/// * `reload_rate` - how many of the device's uplinks the relay forwards an hour, 63 for no
///   limit.
/// * `bucket_size` - the coded bucket size multiplier, table 55.
/// * `out_command` - receives the command, to send with
///   [`pamoja_gateway_network_command`].
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::Codec`] when no session is held for the address.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null handle or out pointer.
///
/// # Safety
///
/// `network` must be a live handle and `out_command` writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_network_trust_command(
    network: *mut PamojaGatewayNetwork,
    dev_addr: u32,
    index: u8,
    reload_rate: u8,
    bucket_size: u8,
    out_command: *mut PamojaLorawanMacCommand,
) -> PamojaStatus {
    if out_command.is_null() {
        return missing("out_command");
    }
    let Some(network) = network.as_mut() else {
        return missing("network");
    };
    match network
        .network
        .trust_command(dev_addr, index, reload_rate, bucket_size)
    {
        Ok(command) => {
            *out_command = write_command(command);
            PamojaStatus::Ok
        }
        Err(error) => refused(&error),
    }
}

/// Releases a network.
///
/// # Arguments
///
/// * `network` - the network, which must not be used again.
///
/// # Safety
///
/// `network` must be a live handle from [`pamoja_gateway_network_open`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_gateway_network_free(network: *mut PamojaGatewayNetwork) {
    if !network.is_null() {
        drop(Box::from_raw(network));
    }
}

/// Reads the windows from the boundary.
fn windows_of(windows: PamojaGatewayNetworkWindows) -> pamoja_gateway::network::Windows {
    let channels = match windows.rx1_channels {
        PAMOJA_GATEWAY_NETWORK_RX1_DOWNSTREAM => Rx1Channels::Downstream(ChannelBlock::new(
            windows.downstream_start_hz,
            windows.downstream_step_hz,
            windows.downstream_count,
            0,
            0,
        )),
        PAMOJA_GATEWAY_NETWORK_RX1_SAME => Rx1Channels::SameAsUplink,
        _ => Rx1Channels::Plan,
    };

    pamoja_gateway::network::Windows::new()
        .with_receive_delay_us(windows.receive_delay_us)
        .with_join_delay_us(windows.join_delay_us)
        .with_rx1_data_rate_offset(windows.rx1_data_rate_offset)
        .with_rx1_channels(channels)
}

/// Reads a window from the boundary.
fn slot_of(slot: PamojaGatewayNetworkSlot) -> Slot {
    Slot {
        timestamp_us: slot.timestamp_us,
        frequency_hz: slot.frequency_hz,
        link: crate::lora::settings(slot.link),
    }
}

/// Writes a window to the boundary.
fn slot_to_c(slot: Slot) -> PamojaGatewayNetworkSlot {
    PamojaGatewayNetworkSlot {
        timestamp_us: slot.timestamp_us,
        frequency_hz: slot.frequency_hz,
        link: link_to_c(slot.link),
    }
}

/// Writes an event to the boundary, filling the caller buffer with what it carried.
unsafe fn event_to_c(event: Event, buffer: *mut u8, capacity: usize) -> PamojaGatewayNetworkEvent {
    match event {
        Event::Joined {
            dev_eui,
            dev_addr,
            accept,
        } => {
            let written = write_payload(&accept.payload, buffer, capacity);
            PamojaGatewayNetworkEvent {
                outcome: PAMOJA_GATEWAY_NETWORK_JOINED,
                dev_eui,
                dev_addr,
                len: if written { accept.payload.len() } else { 0 },
                truncated: !written,
                slot: PamojaGatewayNetworkSlot {
                    timestamp_us: accept.timestamp_us.unwrap_or(0),
                    frequency_hz: accept.frequency_hz,
                    link: link_to_c(
                        accept
                            .modulation
                            .link()
                            .unwrap_or(pamoja_lora::LinkSettings::new(7, 125_000)),
                    ),
                },
                accept: txpk_to_c(&accept),
                ..PamojaGatewayNetworkEvent::default()
            }
        }
        Event::Data {
            dev_addr,
            fcnt,
            fport,
            payload,
            confirmed,
            slot,
            relay,
        } => {
            let written = write_payload(&payload, buffer, capacity);
            PamojaGatewayNetworkEvent {
                outcome: PAMOJA_GATEWAY_NETWORK_DATA,
                dev_addr,
                fcnt,
                fport: fport.unwrap_or(0),
                has_fport: fport.is_some(),
                confirmed,
                len: if written { payload.len() } else { 0 },
                truncated: !written,
                slot: slot_to_c(slot),
                relayed: relay.is_some(),
                relay_dev_addr: relay.map_or(0, |relay| relay.relay),
                relay_rssi_dbm: relay.map_or(0, |relay| relay.metadata.rssi_dbm),
                relay_snr_db: relay.map_or(0, |relay| relay.metadata.snr_db),
                relay_data_rate: relay.map_or(0, |relay| relay.metadata.data_rate),
                relay_wor_channel: relay.map_or(0, |relay| match relay.metadata.wor_channel {
                    WorChannel::Default => 0,
                    WorChannel::Second => 1,
                }),
                relay_frequency_hz: relay.map_or(0, |relay| relay.frequency_hz),
                ..PamojaGatewayNetworkEvent::default()
            }
        }
        Event::Foreign { dev_addr } => PamojaGatewayNetworkEvent {
            outcome: PAMOJA_GATEWAY_NETWORK_FOREIGN,
            dev_addr,
            ..PamojaGatewayNetworkEvent::default()
        },
    }
}

/// Copies bytes into the caller buffer, reporting whether they fit.
unsafe fn write_payload(payload: &[u8], buffer: *mut u8, capacity: usize) -> bool {
    if payload.is_empty() {
        return true;
    }
    if buffer.is_null() || capacity < payload.len() {
        return false;
    }
    ptr::copy_nonoverlapping(payload.as_ptr(), buffer, payload.len());
    true
}

/// Reports a refused frame by name.
fn refused(error: &NetworkError) -> PamojaStatus {
    set_last_error(error.to_string());
    PamojaStatus::Codec
}

/// Reads an eight-byte identifier from the boundary.
unsafe fn eight(bytes: *const u8) -> Option<[u8; 8]> {
    if bytes.is_null() {
        return None;
    }
    let mut read = [0u8; 8];
    ptr::copy_nonoverlapping(bytes, read.as_mut_ptr(), read.len());
    Some(read)
}

/// Reads a sixteen-byte key from the boundary.
unsafe fn sixteen(bytes: *const u8) -> Option<[u8; 16]> {
    if bytes.is_null() {
        return None;
    }
    let mut read = [0u8; 16];
    ptr::copy_nonoverlapping(bytes, read.as_mut_ptr(), read.len());
    Some(read)
}
