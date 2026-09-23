//! The C ABI for a node on a CAN bus.
//!
//! A node is a SocketCAN socket on a kernel interface such as `can0`, on a Linux board, or a
//! node on a bus simulated inside the program. [`pamoja_can_bus_join`] puts another node on the
//! same bus, and a node hears every frame the others send and none of its own. Frames cross as
//! the [`PamojaCanFrame`] handles of the framing calls; a filter is only scalars, so it crosses
//! by value as [`PamojaCanFilter`].
//!
//! A receive on a simulated bus never waits: with nothing there it returns at once and counts
//! its timeout in [`pamoja_can_bus_waited_micros`].

use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::time::Duration;

use pamoja_can::bus::{BusError, CanBus, CanBusKind, Filter, OpenError};
use pamoja_can::CanId;

use crate::can::PamojaCanFrame;
use crate::{read_str, set_last_error, PamojaStatus};

/// A bus kind: a kernel CAN interface reached through SocketCAN.
pub const PAMOJA_CAN_BUS_DEVICE: u8 = 0;

/// A bus kind: a bus inside the program.
pub const PAMOJA_CAN_BUS_SIMULATED: u8 = 1;

/// A node on a CAN bus. Opaque; release it with [`pamoja_can_bus_free`].
pub struct PamojaCanBus {
    bus: CanBus,
}

/// A frame a node keeps: one whose identifier, masked, equals `id`, masked, and whose format
/// is the filter's.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaCanFilter {
    /// The identifier to match.
    pub id: u32,
    /// The identifier bits that have to match.
    pub mask: u32,
    /// `1` for an extended 29-bit identifier, `0` for a standard 11-bit one.
    pub extended: u8,
}

impl From<Filter> for PamojaCanFilter {
    fn from(filter: Filter) -> Self {
        PamojaCanFilter {
            id: filter.id().raw(),
            mask: filter.mask(),
            extended: u8::from(filter.id().is_extended()),
        }
    }
}

impl PamojaCanFilter {
    fn filter(self) -> Filter {
        Filter::new(identifier(self.id, self.extended != 0), self.mask)
    }
}

fn identifier(id: u32, extended: bool) -> CanId {
    if extended {
        CanId::extended(id)
    } else {
        CanId::standard(id as u16)
    }
}

fn into_raw(bus: CanBus) -> *mut PamojaCanBus {
    Box::into_raw(Box::new(PamojaCanBus { bus }))
}

fn opened(result: Result<CanBus, OpenError>, out_bus: &mut *mut PamojaCanBus) -> PamojaStatus {
    match result {
        Ok(bus) => {
            *out_bus = into_raw(bus);
            PamojaStatus::Ok
        }
        Err(error) => {
            let status = match error {
                OpenError::Unsupported => PamojaStatus::Unsupported,
                OpenError::Device { .. } => PamojaStatus::Io,
            };
            set_last_error(error.to_string());
            status
        }
    }
}

/// Runs a call on a node, turning a null handle, a failure, or a panic into a status.
///
/// # Safety
///
/// `bus` must be a live handle or null.
unsafe fn on_bus(
    bus: *const PamojaCanBus,
    call: impl FnOnce(&CanBus) -> Result<(), BusError>,
) -> PamojaStatus {
    let Some(bus) = bus.as_ref() else {
        set_last_error("bus must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    match catch_unwind(AssertUnwindSafe(|| call(&bus.bus))) {
        Ok(Ok(())) => PamojaStatus::Ok,
        Ok(Err(error)) => {
            set_last_error(error.to_string());
            PamojaStatus::Io
        }
        Err(_) => {
            set_last_error("panic at the FFI boundary".to_owned());
            PamojaStatus::Panic
        }
    }
}

/// Makes a new bus inside the program, with the returned node the first on it.
///
/// # Returns
///
/// The node, which the caller releases with [`pamoja_can_bus_free`].
#[no_mangle]
pub extern "C" fn pamoja_can_bus_simulated() -> *mut PamojaCanBus {
    into_raw(CanBus::simulated())
}

/// Opens a kernel CAN interface through SocketCAN, as one node on its bus.
///
/// # Arguments
///
/// * `interface` - the interface, such as `can0` or `vcan0`, as UTF-8.
/// * `out_bus` - receives the node.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::Unsupported`] anywhere but Linux;
/// [`PamojaStatus::Io`] when the interface does not exist or cannot be bound; or
/// [`PamojaStatus::InvalidArgument`] for a null argument. The reason is in the last error
/// message.
///
/// # Safety
///
/// `interface` must be a NUL-terminated string or null, and `out_bus` a writable pointer or
/// null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_bus_open(
    interface: *const c_char,
    out_bus: *mut *mut PamojaCanBus,
) -> PamojaStatus {
    let Some(out_bus) = out_bus.as_mut() else {
        set_last_error("out_bus must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out_bus = ptr::null_mut();
    let Some(interface) = read_str(interface, "interface") else {
        return PamojaStatus::InvalidArgument;
    };
    opened(CanBus::open(interface), out_bus)
}

/// Puts another node on the same bus: another socket on the same interface, or another node on
/// the same simulated bus.
///
/// # Arguments
///
/// * `bus` - a node on the bus.
/// * `out_bus` - receives the new node.
///
/// # Returns
///
/// As [`pamoja_can_bus_open`].
///
/// # Safety
///
/// `bus` must be a live handle or null, and `out_bus` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_bus_join(
    bus: *const PamojaCanBus,
    out_bus: *mut *mut PamojaCanBus,
) -> PamojaStatus {
    let Some(out_bus) = out_bus.as_mut() else {
        set_last_error("out_bus must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out_bus = ptr::null_mut();
    let Some(bus) = bus.as_ref() else {
        set_last_error("bus must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    opened(bus.bus.join(), out_bus)
}

/// Returns what a node's bus is: [`PAMOJA_CAN_BUS_DEVICE`] or [`PAMOJA_CAN_BUS_SIMULATED`],
/// which a null handle also reports.
///
/// # Safety
///
/// `bus` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_bus_kind(bus: *const PamojaCanBus) -> u8 {
    match bus.as_ref().map(|bus| bus.bus.kind()) {
        Some(CanBusKind::Device) => PAMOJA_CAN_BUS_DEVICE,
        _ => PAMOJA_CAN_BUS_SIMULATED,
    }
}

/// Sends a frame to every other node on the bus.
///
/// # Arguments
///
/// * `bus` - the node.
/// * `frame` - the frame, which stays the caller's.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null argument; or
/// [`PamojaStatus::Io`] when the kernel refuses the frame, with the reason in the last error
/// message.
///
/// # Safety
///
/// `bus` and `frame` must be live handles or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_bus_send(
    bus: *const PamojaCanBus,
    frame: *const PamojaCanFrame,
) -> PamojaStatus {
    let Some(frame) = frame.as_ref() else {
        set_last_error("frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let frame = frame.frame;
    on_bus(bus, |bus| bus.send(&frame))
}

/// Takes the next frame the node keeps, waiting up to a timeout for one.
///
/// # Arguments
///
/// * `bus` - the node.
/// * `timeout_micros` - how long to wait.
/// * `out_frame` - receives a new frame handle, which the caller releases with
///   [`pamoja_can_frame_free`](crate::can::pamoja_can_frame_free), or null when the timeout
///   passed with nothing.
///
/// # Returns
///
/// As [`pamoja_can_bus_send`].
///
/// # Safety
///
/// `bus` must be a live handle or null, and `out_frame` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_bus_receive(
    bus: *const PamojaCanBus,
    timeout_micros: u64,
    out_frame: *mut *mut PamojaCanFrame,
) -> PamojaStatus {
    let Some(out_frame) = out_frame.as_mut() else {
        set_last_error("out_frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out_frame = ptr::null_mut();
    on_bus(bus, |bus| {
        if let Some(frame) = bus.receive(Duration::from_micros(timeout_micros))? {
            *out_frame = Box::into_raw(Box::new(PamojaCanFrame { frame }));
        }
        Ok(())
    })
}

/// Keeps only the frames that pass at least one of the filters, from now on. An empty list
/// keeps nothing.
///
/// # Arguments
///
/// * `bus` - the node.
/// * `filters` - the filters.
/// * `len` - how many, at most 512.
///
/// # Returns
///
/// As [`pamoja_can_bus_send`].
///
/// # Safety
///
/// `bus` must be a live handle or null, and `filters` must point to `len` readable filters.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_bus_set_filters(
    bus: *const PamojaCanBus,
    filters: *const PamojaCanFilter,
    len: usize,
) -> PamojaStatus {
    let filters: Vec<Filter> = if len == 0 {
        Vec::new()
    } else if filters.is_null() {
        set_last_error("filters must not be null when len is non-zero".to_owned());
        return PamojaStatus::InvalidArgument;
    } else {
        std::slice::from_raw_parts(filters, len)
            .iter()
            .map(|filter| filter.filter())
            .collect()
    };
    on_bus(bus, |bus| bus.set_filters(&filters))
}

/// Keeps every frame again, as a node does when it joins.
///
/// # Returns
///
/// As [`pamoja_can_bus_send`].
///
/// # Safety
///
/// `bus` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_bus_clear_filters(bus: *const PamojaCanBus) -> PamojaStatus {
    on_bus(bus, CanBus::clear_filters)
}

/// Returns how many frames a node has sent, or 0 for a null handle.
///
/// # Safety
///
/// `bus` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_bus_sent(bus: *const PamojaCanBus) -> usize {
    bus.as_ref().map_or(0, |bus| bus.bus.sent())
}

/// Returns how many frames a node has received, or 0 for a null handle.
///
/// # Safety
///
/// `bus` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_bus_received(bus: *const PamojaCanBus) -> usize {
    bus.as_ref().map_or(0, |bus| bus.bus.received())
}

/// Returns how long receives on a node have waited without a frame, in microseconds, whether
/// or not the process slept through it, or 0 for a null handle.
///
/// # Safety
///
/// `bus` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_bus_waited_micros(bus: *const PamojaCanBus) -> u64 {
    bus.as_ref().map_or(0, |bus| bus.bus.waited_micros())
}

/// Releases a node; it leaves the bus. A null pointer is ignored.
///
/// # Safety
///
/// `bus` must be a handle that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_can_bus_free(bus: *mut PamojaCanBus) {
    if !bus.is_null() {
        drop(Box::from_raw(bus));
    }
}

/// A filter that passes one identifier and nothing else.
///
/// # Arguments
///
/// * `id` - the identifier.
/// * `extended` - whether it is a 29-bit extended identifier.
///
/// # Returns
///
/// The filter.
#[no_mangle]
pub extern "C" fn pamoja_can_filter_exact(id: u32, extended: bool) -> PamojaCanFilter {
    Filter::exact(identifier(id, extended)).into()
}

/// A filter that passes one J1939 parameter group at any priority, from any source, and for
/// an addressed group, to any destination.
///
/// # Arguments
///
/// * `pgn` - the parameter group number.
///
/// # Returns
///
/// The filter.
#[no_mangle]
pub extern "C" fn pamoja_can_filter_pgn(pgn: u32) -> PamojaCanFilter {
    Filter::pgn(pgn).into()
}

/// Reports whether a frame with an identifier passes a filter.
///
/// # Arguments
///
/// * `filter` - the filter.
/// * `id` - the frame's identifier.
/// * `extended` - whether it is a 29-bit extended identifier.
///
/// # Returns
///
/// `true` when the frame formats agree and the masked bits are equal.
#[no_mangle]
pub extern "C" fn pamoja_can_filter_matches(
    filter: PamojaCanFilter,
    id: u32,
    extended: bool,
) -> bool {
    filter.filter().matches(identifier(id, extended))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::can::{pamoja_can_frame_data_len, pamoja_can_frame_free, pamoja_can_frame_new};

    #[test]
    fn two_nodes_trade_a_frame_through_the_c_abi() {
        unsafe {
            let engine = pamoja_can_bus_simulated();
            let mut gateway = ptr::null_mut();
            assert_eq!(pamoja_can_bus_join(engine, &mut gateway), PamojaStatus::Ok);
            assert_eq!(pamoja_can_bus_kind(gateway), PAMOJA_CAN_BUS_SIMULATED);
            let filter = pamoja_can_filter_pgn(61_444);
            assert_eq!(
                pamoja_can_bus_set_filters(gateway, &filter, 1),
                PamojaStatus::Ok
            );

            let speed = pamoja_can::J1939Id::broadcast(3, 61_444, 0).to_id().raw();
            let data = [0xFFu8; 8];
            let mut frame = ptr::null_mut();
            assert_eq!(
                pamoja_can_frame_new(speed, true, data.as_ptr(), 8, &mut frame),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_can_bus_send(engine, frame), PamojaStatus::Ok);
            pamoja_can_frame_free(frame);

            let mut heard = ptr::null_mut();
            assert_eq!(
                pamoja_can_bus_receive(gateway, 1_000, &mut heard),
                PamojaStatus::Ok
            );
            assert!(!heard.is_null());
            assert_eq!(pamoja_can_frame_data_len(heard), 8);
            pamoja_can_frame_free(heard);

            assert_eq!(
                pamoja_can_bus_receive(gateway, 250_000, &mut heard),
                PamojaStatus::Ok
            );
            assert!(heard.is_null());
            assert_eq!(pamoja_can_bus_waited_micros(gateway), 250_000);
            assert_eq!(
                (
                    pamoja_can_bus_sent(engine),
                    pamoja_can_bus_received(gateway)
                ),
                (1, 1)
            );
            assert!(pamoja_can_filter_matches(filter, speed, true));
            assert!(!pamoja_can_filter_matches(filter, speed, false));

            pamoja_can_bus_free(gateway);
            pamoja_can_bus_free(engine);
        }
    }

    #[test]
    fn opening_an_interface_off_linux_is_unsupported() {
        if cfg!(not(target_os = "linux")) {
            let mut bus = ptr::null_mut();
            let name = c"can0";
            assert_eq!(
                unsafe { pamoja_can_bus_open(name.as_ptr(), &mut bus) },
                PamojaStatus::Unsupported
            );
            assert!(bus.is_null());
        }
    }
}
