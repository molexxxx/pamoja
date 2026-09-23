//! The C ABI for one serial port that a program and every driver on it share.
//!
//! A port is the kernel's serial device on a Linux board, a line looped back on itself, one
//! end of a null-modem pair, or a script of what a driver is expected to write, and the same
//! calls work on each: a write, and a read that waits up to a timeout for the first byte. On
//! anything but the kernel's device a read never waits; the time it would have waited is
//! counted instead, so a test of a device that never answers runs at once.
//!
//! A port holds its line in shared hands. [`pamoja_serial_port_clone`] gives another holder,
//! such as a thread that reads, the same port, and the line closes when the last holder
//! lets go. A script is built here and copied onto a port, so its handle stays the caller's
//! to free. A failed read or write returns [`PamojaStatus::Io`] with the reason in the last
//! error message.

use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Duration;

use pamoja_hal::port::{
    OpenError, Parity, PortError, PortKind, PortStep, SerialPort, Settings, StopBits,
};

use crate::{read_bytes, read_str, set_last_error, PamojaStatus};

/// No parity bit.
pub const PAMOJA_PARITY_NONE: u8 = 0;

/// A parity bit that makes the count of ones even, what Modbus RTU asks for by default.
pub const PAMOJA_PARITY_EVEN: u8 = 1;

/// A parity bit that makes the count of ones odd.
pub const PAMOJA_PARITY_ODD: u8 = 2;

/// A port kind: the kernel's serial device, with a real line on the other end.
pub const PAMOJA_SERIAL_PORT_DEVICE: u8 = 0;

/// A port kind: the port's own output, looped back to its input.
pub const PAMOJA_SERIAL_PORT_LOOPED: u8 = 1;

/// A port kind: the other end of a null-modem pair.
pub const PAMOJA_SERIAL_PORT_PAIRED: u8 = 2;

/// A port kind: a simulated device that answers each write.
pub const PAMOJA_SERIAL_PORT_SIMULATED: u8 = 3;

/// A port kind: a script of the writes a driver is expected to make.
pub const PAMOJA_SERIAL_PORT_SCRIPTED: u8 = 4;

/// A port's speed and character format: eight data bits, with the parity and stop bits given.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaSerialSettings {
    /// The speed, in bits a second.
    pub baud: u32,
    /// [`PAMOJA_PARITY_NONE`], [`PAMOJA_PARITY_EVEN`], or [`PAMOJA_PARITY_ODD`].
    pub parity: u8,
    /// 1 or 2.
    pub stop_bits: u8,
}

/// One serial port, shared with every holder of it. Opaque; release it with
/// [`pamoja_serial_port_free`].
pub struct PamojaSerialPort {
    pub(crate) port: SerialPort,
}

/// The writes a driver is expected to make and the bytes the far end sends, in order. Opaque;
/// release it with [`pamoja_serial_script_free`].
pub struct PamojaSerialScript {
    steps: Vec<PortStep>,
}

impl From<Settings> for PamojaSerialSettings {
    fn from(settings: Settings) -> Self {
        PamojaSerialSettings {
            baud: settings.baud,
            parity: match settings.parity {
                Parity::None => PAMOJA_PARITY_NONE,
                Parity::Even => PAMOJA_PARITY_EVEN,
                Parity::Odd => PAMOJA_PARITY_ODD,
            },
            stop_bits: match settings.stop_bits {
                StopBits::One => 1,
                StopBits::Two => 2,
            },
        }
    }
}

impl PamojaSerialSettings {
    /// Reads the settings back, refusing a parity or a stop bit count the port does not have.
    ///
    /// # Returns
    ///
    /// The settings, or [`PamojaStatus::InvalidArgument`] with the reason in the last error
    /// message.
    pub(crate) fn settings(self) -> Result<Settings, PamojaStatus> {
        let parity = match self.parity {
            PAMOJA_PARITY_NONE => Parity::None,
            PAMOJA_PARITY_EVEN => Parity::Even,
            PAMOJA_PARITY_ODD => Parity::Odd,
            other => {
                set_last_error(format!("parity {other} is not none, even, or odd"));
                return Err(PamojaStatus::InvalidArgument);
            }
        };
        let stop_bits = match self.stop_bits {
            1 => StopBits::One,
            2 => StopBits::Two,
            other => {
                set_last_error(format!("{other} stop bits is not 1 or 2"));
                return Err(PamojaStatus::InvalidArgument);
            }
        };
        if self.baud == 0 {
            set_last_error("the speed must be above zero".to_owned());
            return Err(PamojaStatus::InvalidArgument);
        }
        Ok(Settings::new(self.baud)
            .with_parity(parity)
            .with_stop_bits(stop_bits))
    }
}

/// Returns the format most devices start in: eight data bits, no parity, one stop bit.
///
/// # Arguments
///
/// * `baud` - the speed, in bits a second.
///
/// # Returns
///
/// The settings.
#[no_mangle]
pub extern "C" fn pamoja_serial_settings(baud: u32) -> PamojaSerialSettings {
    Settings::new(baud).into()
}

/// Returns the bits one character takes on the wire: a start bit, eight data bits, the parity
/// bit if there is one, and the stop bits.
///
/// # Arguments
///
/// * `settings` - the format.
///
/// # Returns
///
/// The bit count, or 0 for settings the port does not accept.
#[no_mangle]
pub extern "C" fn pamoja_serial_settings_bits_per_character(settings: PamojaSerialSettings) -> u32 {
    settings
        .settings()
        .map_or(0, |settings| settings.bits_per_character())
}

/// Returns how long one character takes on the wire.
///
/// # Arguments
///
/// * `settings` - the speed and format.
///
/// # Returns
///
/// The time in nanoseconds, rounded up, or 0 for settings the port does not accept.
#[no_mangle]
pub extern "C" fn pamoja_serial_settings_character_nanos(settings: PamojaSerialSettings) -> u64 {
    settings
        .settings()
        .map_or(0, |settings| settings.character_nanos())
}

/// Opens the kernel's serial device raw, at a speed and a character format. Whatever the device
/// received before it was opened is dropped.
///
/// # Arguments
///
/// * `path` - the device file: `/dev/serial0` for a Raspberry Pi's own UART, `/dev/ttyUSB0` or
///   `/dev/ttyACM0` for a USB adapter.
/// * `settings` - the speed, one of the standard rates from 1200 to 921600, and the format.
/// * `out_port` - receives the port, or null when opening fails.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with the port in `out_port`; [`PamojaStatus::Unsupported`] on any
/// platform but Linux; [`PamojaStatus::InvalidArgument`] for a null or non-UTF-8 argument or
/// settings the port does not have; or [`PamojaStatus::Io`] when the device cannot be opened
/// or set up, with a last error message that names it.
///
/// # Safety
///
/// `path` must be a null-terminated string or null, and `out_port` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_port_open(
    path: *const c_char,
    settings: PamojaSerialSettings,
    out_port: *mut *mut PamojaSerialPort,
) -> PamojaStatus {
    if out_port.is_null() {
        set_last_error("out_port must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_port = std::ptr::null_mut();
    let Some(path) = read_str(path, "path") else {
        return PamojaStatus::InvalidArgument;
    };
    let settings = match settings.settings() {
        Ok(settings) => settings,
        Err(status) => return status,
    };
    match catch_unwind(AssertUnwindSafe(|| SerialPort::open(path, settings))) {
        Ok(Ok(port)) => {
            *out_port = into_raw(port);
            PamojaStatus::Ok
        }
        Ok(Err(error)) => {
            set_last_error(error.to_string());
            match error {
                OpenError::Unsupported => PamojaStatus::Unsupported,
                OpenError::Device { .. } => PamojaStatus::Io,
            }
        }
        Err(_) => panicked(),
    }
}

/// Creates a line looped back on itself: every byte written is waiting to be read, as with TX
/// wired to RX.
///
/// # Arguments
///
/// * `settings` - the speed and format the line runs at.
///
/// # Returns
///
/// The port, which the caller releases with [`pamoja_serial_port_free`], or null for settings
/// the port does not have.
#[no_mangle]
pub extern "C" fn pamoja_serial_port_looped(
    settings: PamojaSerialSettings,
) -> *mut PamojaSerialPort {
    match settings.settings() {
        Ok(settings) => into_raw(SerialPort::looped(settings)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Creates the two ends of a null-modem pair: what one end writes, the other reads.
///
/// # Arguments
///
/// * `settings` - the speed and format both ends run at.
/// * `out_one` - receives one end.
/// * `out_other` - receives the other end.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] with both ends set, each released with [`pamoja_serial_port_free`];
/// or [`PamojaStatus::InvalidArgument`] for a null out pointer or settings the port does not
/// have.
///
/// # Safety
///
/// `out_one` and `out_other` must be writable pointers or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_port_pair(
    settings: PamojaSerialSettings,
    out_one: *mut *mut PamojaSerialPort,
    out_other: *mut *mut PamojaSerialPort,
) -> PamojaStatus {
    if out_one.is_null() || out_other.is_null() {
        set_last_error("out_one and out_other must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    *out_one = std::ptr::null_mut();
    *out_other = std::ptr::null_mut();
    let settings = match settings.settings() {
        Ok(settings) => settings,
        Err(status) => return status,
    };
    let (one, other) = SerialPort::pair(settings);
    *out_one = into_raw(one);
    *out_other = into_raw(other);
    PamojaStatus::Ok
}

/// Creates an empty script.
///
/// # Returns
///
/// The script, which the caller releases with [`pamoja_serial_script_free`].
#[no_mangle]
pub extern "C" fn pamoja_serial_script_new() -> *mut PamojaSerialScript {
    Box::into_raw(Box::new(PamojaSerialScript { steps: Vec::new() }))
}

/// Adds a write the program is expected to make, in one call.
///
/// # Arguments
///
/// * `script` - the script.
/// * `bytes` - the bytes of the write.
/// * `len` - how many bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] for a null argument.
///
/// # Safety
///
/// `script` must be a live handle or null, and `bytes` must point to `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_script_write(
    script: *mut PamojaSerialScript,
    bytes: *const u8,
    len: usize,
) -> PamojaStatus {
    add_step(script, bytes, len, PortStep::Write)
}

/// Adds bytes the far end sends, readable once every step before them has happened.
///
/// # Arguments
///
/// * `script` - the script.
/// * `bytes` - what arrives.
/// * `len` - how many bytes.
///
/// # Returns
///
/// As [`pamoja_serial_script_write`].
///
/// # Safety
///
/// `script` must be a live handle or null, and `bytes` must point to `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_script_read(
    script: *mut PamojaSerialScript,
    bytes: *const u8,
    len: usize,
) -> PamojaStatus {
    add_step(script, bytes, len, PortStep::Read)
}

/// Returns how many steps a script holds.
///
/// # Arguments
///
/// * `script` - the script.
///
/// # Returns
///
/// The count, or 0 for a null script.
///
/// # Safety
///
/// `script` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_script_len(script: *const PamojaSerialScript) -> usize {
    script.as_ref().map_or(0, |script| script.steps.len())
}

/// Releases a script. A null pointer is ignored.
///
/// # Safety
///
/// `script` must be a handle that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_script_free(script: *mut PamojaSerialScript) {
    if !script.is_null() {
        drop(Box::from_raw(script));
    }
}

/// Creates a port that plays a script: each write has to be the one the script expects next,
/// and the bytes the far end sends become readable as the script reaches them.
///
/// # Arguments
///
/// * `settings` - the speed and format the line runs at.
/// * `script` - the steps to copy; the caller still owns the handle.
///
/// # Returns
///
/// The port, which the caller releases with [`pamoja_serial_port_free`], or null for a null
/// script or settings the port does not have.
///
/// # Safety
///
/// `script` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_port_scripted(
    settings: PamojaSerialSettings,
    script: *const PamojaSerialScript,
) -> *mut PamojaSerialPort {
    let Some(script) = script.as_ref() else {
        set_last_error("script must not be null".to_owned());
        return std::ptr::null_mut();
    };
    match settings.settings() {
        Ok(settings) => into_raw(SerialPort::scripted(settings, script.steps.clone())),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Gives another holder the same port, such as a thread that reads while the caller writes.
///
/// # Arguments
///
/// * `port` - the port.
///
/// # Returns
///
/// A new handle to the same port, released with [`pamoja_serial_port_free`], or null for a null
/// port.
///
/// # Safety
///
/// `port` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_port_clone(
    port: *const PamojaSerialPort,
) -> *mut PamojaSerialPort {
    match port.as_ref() {
        Some(port) => into_raw(port.port.clone()),
        None => std::ptr::null_mut(),
    }
}

/// Returns what is on the other end of a port.
///
/// # Arguments
///
/// * `port` - the port.
///
/// # Returns
///
/// One of the `PAMOJA_SERIAL_PORT_` kinds; the looped code for a null port.
///
/// # Safety
///
/// `port` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_port_kind(port: *const PamojaSerialPort) -> u8 {
    match port.as_ref().map(|port| port.port.kind()) {
        Some(PortKind::Device) => PAMOJA_SERIAL_PORT_DEVICE,
        Some(PortKind::Looped) | None => PAMOJA_SERIAL_PORT_LOOPED,
        Some(PortKind::Paired) => PAMOJA_SERIAL_PORT_PAIRED,
        Some(PortKind::Simulated) => PAMOJA_SERIAL_PORT_SIMULATED,
        Some(PortKind::Scripted) => PAMOJA_SERIAL_PORT_SCRIPTED,
    }
}

/// Returns the speed and format a port runs at.
///
/// # Arguments
///
/// * `port` - the port.
///
/// # Returns
///
/// The settings, or 9600 8N1 for a null port.
///
/// # Safety
///
/// `port` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_port_settings(
    port: *const PamojaSerialPort,
) -> PamojaSerialSettings {
    port.as_ref()
        .map_or(Settings::new(9_600), |port| port.port.settings())
        .into()
}

/// Writes bytes, and on the kernel's device waits until they have left the UART.
///
/// # Arguments
///
/// * `port` - the port.
/// * `bytes` - the bytes, in order.
/// * `len` - how many bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`]; [`PamojaStatus::InvalidArgument`] for a null argument; or
/// [`PamojaStatus::Io`] when a script expected another write or the device failed, with the
/// reason in the last error message.
///
/// # Safety
///
/// `port` must be a live handle or null, and `bytes` must point to `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_port_write(
    port: *const PamojaSerialPort,
    bytes: *const u8,
    len: usize,
) -> PamojaStatus {
    let bytes = match read_bytes(bytes, len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    on_port(port, |port| port.write(&bytes))
}

/// Reads what has arrived, waiting up to a timeout for the first byte when nothing has. On any
/// port but the kernel's device the read does not wait, and the timeout is counted instead.
///
/// # Arguments
///
/// * `port` - the port.
/// * `out` - receives the bytes.
/// * `capacity` - the most bytes to read.
/// * `timeout_micros` - how long to wait for the first byte.
/// * `out_len` - receives how many bytes were read, zero when the timeout passed with nothing.
///
/// # Returns
///
/// As [`pamoja_serial_port_write`], and [`PamojaStatus::InvalidArgument`] for a null `out_len`.
///
/// # Safety
///
/// `port` must be a live handle or null, `out` must point to `capacity` writable bytes, and
/// `out_len` must be a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_port_read(
    port: *const PamojaSerialPort,
    out: *mut u8,
    capacity: usize,
    timeout_micros: u64,
    out_len: *mut usize,
) -> PamojaStatus {
    let Some(out_len) = out_len.as_mut() else {
        set_last_error("out_len must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    *out_len = 0;
    let buffer: &mut [u8] = if capacity == 0 {
        &mut []
    } else if out.is_null() {
        set_last_error("out must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    } else {
        std::slice::from_raw_parts_mut(out, capacity)
    };
    let timeout = Duration::from_micros(timeout_micros);
    on_port(port, |port| {
        *out_len = port.read(buffer, timeout)?;
        Ok(())
    })
}

/// Drops whatever has arrived and not been read, as a client does before a request so a stale
/// reply cannot be taken for the new one.
///
/// # Arguments
///
/// * `port` - the port.
///
/// # Returns
///
/// As [`pamoja_serial_port_write`].
///
/// # Safety
///
/// `port` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_port_discard_input(
    port: *const PamojaSerialPort,
) -> PamojaStatus {
    on_port(port, SerialPort::discard_input)
}

/// Waits, as a protocol does to leave the line silent between frames: the process sleeps on
/// the kernel's device, and anywhere else the wait is only counted.
///
/// # Arguments
///
/// * `port` - the port.
/// * `micros` - how long.
///
/// # Safety
///
/// `port` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_port_wait(port: *const PamojaSerialPort, micros: u64) {
    if let Some(port) = port.as_ref() {
        port.port.wait(Duration::from_micros(micros));
    }
}

/// Returns how many bytes have been written through a port and every holder of it.
///
/// # Safety
///
/// `port` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_port_written(port: *const PamojaSerialPort) -> usize {
    port.as_ref().map_or(0, |port| port.port.written())
}

/// Returns how many bytes have been read through a port and every holder of it.
///
/// # Safety
///
/// `port` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_port_received(port: *const PamojaSerialPort) -> usize {
    port.as_ref().map_or(0, |port| port.port.received())
}

/// Returns how long reads have waited without an answer, and waits have waited, in
/// microseconds, whether or not the process slept through it.
///
/// # Safety
///
/// `port` must be a live handle or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_port_waited_micros(port: *const PamojaSerialPort) -> u64 {
    port.as_ref().map_or(0, |port| port.port.waited_micros())
}

/// Reports how many steps a scripted port has left.
///
/// # Arguments
///
/// * `port` - the port.
/// * `out_remaining` - receives the steps not yet reached.
///
/// # Returns
///
/// `true` with the count in `out_remaining` for a scripted port; `false` for any other port, a
/// null port, or a null `out_remaining`.
///
/// # Safety
///
/// `port` must be a live handle or null, and `out_remaining` a writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_port_remaining(
    port: *const PamojaSerialPort,
    out_remaining: *mut usize,
) -> bool {
    let (Some(port), Some(out)) = (port.as_ref(), out_remaining.as_mut()) else {
        return false;
    };
    match port.port.remaining() {
        Some(remaining) => {
            *out = remaining;
            true
        }
        None => false,
    }
}

/// Releases a holder's share of a port. The line closes when no holder has it. A null pointer is
/// ignored.
///
/// # Safety
///
/// `port` must be a handle that has not been freed, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_serial_port_free(port: *mut PamojaSerialPort) {
    if !port.is_null() {
        drop(Box::from_raw(port));
    }
}

fn into_raw(port: SerialPort) -> *mut PamojaSerialPort {
    Box::into_raw(Box::new(PamojaSerialPort { port }))
}

/// Adds a step to a script, turning a null script or null bytes into a status.
///
/// # Safety
///
/// `script` must be a live handle or null, and `bytes` must point to `len` readable bytes.
unsafe fn add_step(
    script: *mut PamojaSerialScript,
    bytes: *const u8,
    len: usize,
    step: fn(Vec<u8>) -> PortStep,
) -> PamojaStatus {
    let Some(script) = script.as_mut() else {
        set_last_error("script must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    match read_bytes(bytes, len) {
        Ok(bytes) => {
            script.steps.push(step(bytes));
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Runs a call on a port, turning a failure or a panic into a status.
///
/// # Safety
///
/// `port` must be a live handle or null.
unsafe fn on_port(
    port: *const PamojaSerialPort,
    call: impl FnOnce(&SerialPort) -> Result<(), PortError>,
) -> PamojaStatus {
    let Some(port) = port.as_ref() else {
        set_last_error("port must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    match catch_unwind(AssertUnwindSafe(|| call(&port.port))) {
        Ok(Ok(())) => PamojaStatus::Ok,
        Ok(Err(error)) => {
            set_last_error(error.to_string());
            PamojaStatus::Io
        }
        Err(_) => panicked(),
    }
}

/// Records a caught panic and reports it as [`PamojaStatus::Panic`].
fn panicked() -> PamojaStatus {
    set_last_error("panic at the FFI boundary".to_owned());
    PamojaStatus::Panic
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};
    use std::ptr;

    fn last_error() -> String {
        let message = crate::pamoja_last_error_message();
        if message.is_null() {
            return String::new();
        }
        unsafe { CStr::from_ptr(message) }
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn settings_cross_the_boundary_and_are_checked() {
        let settings = pamoja_serial_settings(9_600);
        assert_eq!(
            settings,
            PamojaSerialSettings {
                baud: 9_600,
                parity: PAMOJA_PARITY_NONE,
                stop_bits: 1
            }
        );
        let modbus = PamojaSerialSettings {
            parity: PAMOJA_PARITY_EVEN,
            ..settings
        };
        assert_eq!(pamoja_serial_settings_bits_per_character(modbus), 11);
        assert_eq!(pamoja_serial_settings_character_nanos(modbus), 1_145_834);
        let odd = PamojaSerialSettings {
            parity: 7,
            ..settings
        };
        assert_eq!(pamoja_serial_settings_bits_per_character(odd), 0);
        assert!(pamoja_serial_port_looped(odd).is_null());
        assert!(last_error().contains("parity 7"), "{}", last_error());
    }

    #[test]
    fn a_pair_carries_bytes_both_ways_and_counts_an_empty_read() {
        unsafe {
            let (mut one, mut other) = (ptr::null_mut(), ptr::null_mut());
            assert_eq!(
                pamoja_serial_port_pair(pamoja_serial_settings(115_200), &mut one, &mut other),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_serial_port_kind(one), PAMOJA_SERIAL_PORT_PAIRED);
            let message = b"t=21.5";
            assert_eq!(
                pamoja_serial_port_write(one, message.as_ptr(), message.len()),
                PamojaStatus::Ok
            );
            let reader = pamoja_serial_port_clone(other);
            let mut buffer = [0u8; 16];
            let mut got = 0;
            assert_eq!(
                pamoja_serial_port_read(
                    reader,
                    buffer.as_mut_ptr(),
                    buffer.len(),
                    50_000,
                    &mut got
                ),
                PamojaStatus::Ok
            );
            assert_eq!(&buffer[..got], message);
            assert_eq!(
                pamoja_serial_port_read(other, buffer.as_mut_ptr(), buffer.len(), 50_000, &mut got),
                PamojaStatus::Ok
            );
            assert_eq!(got, 0);
            assert_eq!(pamoja_serial_port_waited_micros(other), 50_000);
            assert_eq!(pamoja_serial_port_received(other), message.len());
            pamoja_serial_port_wait(one, 1_750);
            assert_eq!(pamoja_serial_port_waited_micros(one), 1_750);
            assert_eq!(pamoja_serial_port_settings(one).baud, 115_200);
            pamoja_serial_port_free(reader);
            pamoja_serial_port_free(one);
            pamoja_serial_port_free(other);
        }
    }

    #[test]
    fn a_script_refuses_an_unexpected_write() {
        unsafe {
            let script = pamoja_serial_script_new();
            assert_eq!(
                pamoja_serial_script_write(script, b"?".as_ptr(), 1),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_serial_script_read(script, b"42".as_ptr(), 2),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_serial_script_len(script), 2);
            let port = pamoja_serial_port_scripted(pamoja_serial_settings(9_600), script);
            pamoja_serial_script_free(script);
            assert_eq!(pamoja_serial_port_kind(port), PAMOJA_SERIAL_PORT_SCRIPTED);
            assert_eq!(
                pamoja_serial_port_write(port, b"!".as_ptr(), 1),
                PamojaStatus::Io
            );
            assert!(last_error().contains("expected 3f"), "{}", last_error());
            assert_eq!(
                pamoja_serial_port_write(port, b"?".as_ptr(), 1),
                PamojaStatus::Ok
            );
            let mut remaining = 9;
            assert!(pamoja_serial_port_remaining(port, &mut remaining));
            assert_eq!(remaining, 0);
            let mut buffer = [0u8; 4];
            let mut got = 0;
            pamoja_serial_port_read(port, buffer.as_mut_ptr(), buffer.len(), 0, &mut got);
            assert_eq!(&buffer[..got], b"42");
            assert_eq!(pamoja_serial_port_discard_input(port), PamojaStatus::Ok);
            pamoja_serial_port_free(port);
        }
    }

    #[test]
    fn a_looped_line_and_the_null_checks() {
        unsafe {
            let port = pamoja_serial_port_looped(pamoja_serial_settings(9_600));
            assert_eq!(
                pamoja_serial_port_write(port, b"abc".as_ptr(), 3),
                PamojaStatus::Ok
            );
            let mut buffer = [0u8; 3];
            let mut got = 0;
            assert_eq!(
                pamoja_serial_port_read(port, buffer.as_mut_ptr(), 3, 0, ptr::null_mut()),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(
                pamoja_serial_port_read(port, buffer.as_mut_ptr(), 3, 0, &mut got),
                PamojaStatus::Ok
            );
            assert_eq!(&buffer, b"abc");
            assert_eq!(pamoja_serial_port_written(port), 3);
            assert_eq!(
                pamoja_serial_port_write(ptr::null(), b"x".as_ptr(), 1),
                PamojaStatus::InvalidArgument
            );
            pamoja_serial_port_free(port);

            let path = CString::new("/dev/serial0").unwrap();
            let mut opened = ptr::null_mut();
            let status = pamoja_serial_port_open(
                path.as_ptr(),
                pamoja_serial_settings(115_200),
                &mut opened,
            );
            if cfg!(target_os = "linux") {
                assert_ne!(status, PamojaStatus::Unsupported);
            } else {
                assert_eq!(status, PamojaStatus::Unsupported);
            }
            pamoja_serial_port_free(opened);
        }
    }
}
