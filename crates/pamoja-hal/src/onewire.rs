//! The 1-Wire bus: one data line, a pull-up resistor, and strict timing.
//!
//! A 1-Wire device such as the DS18B20 shares a single open-drain line with the
//! controller and any number of other devices. The controller drives the line low for
//! fixed intervals and releases it for the pull-up to raise; the device answers by
//! holding the line low inside windows the controller samples. Everything on the bus
//! is built from three primitives, and [`OneWireBus`] names them: a reset pulse that
//! every device answers with a presence pulse, a write slot that carries one bit, and
//! a read slot that samples one bit. [`BitBang`] implements them over any pin that can
//! be driven low and read back, plus a delay, so a microcontroller runs the bus from
//! one GPIO with no peripheral at all.
//!
//! Above the primitives sit the ROM commands every 1-Wire device understands, the
//! 64-bit [`RomCode`] that identifies each one (an 8-bit family code, a 48-bit serial,
//! and a CRC-8), and the [`Search`] that enumerates every device on a bus one ROM
//! code at a time. The slot and reset timings are the ones the DS18B20 datasheet
//! specifies, and they match the timings the Linux kernel's `w1` bus driver uses for a
//! bit-banged controller.
//!
//! Bit-banging needs microsecond-accurate delays, which a microcontroller has and a
//! Linux process does not. On a Raspberry Pi use the kernel's own `w1-gpio` driver
//! and read a thermometer through the sysfs file it exposes; the DS18B20 driver in
//! `pamoja-sensors` covers both paths.

use core::fmt;

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};

/// The ROM commands every 1-Wire device answers, issued right after a reset.
pub mod command {
    /// Enumerate the devices on the bus one ROM code at a time; see [`super::Search`].
    pub const SEARCH_ROM: u8 = 0xF0;
    /// Like `SEARCH_ROM`, but only devices with an alarm condition take part.
    pub const ALARM_SEARCH: u8 = 0xEC;
    /// Read the ROM code of the only device on the bus.
    pub const READ_ROM: u8 = 0x33;
    /// Address one device by its ROM code; the next function command goes to it alone.
    pub const MATCH_ROM: u8 = 0x55;
    /// Address every device at once, for a bus with one device or a broadcast command.
    pub const SKIP_ROM: u8 = 0xCC;
    /// Resume addressing the device most recently selected with `MATCH_ROM`.
    pub const RESUME: u8 = 0xA5;
}

/// The bus timings in microseconds, from the DS18B20 datasheet's slot definitions.
pub mod timing {
    /// The reset pulse: the controller holds the line low at least 480 us.
    pub const RESET_LOW_US: u32 = 500;
    /// How long after releasing the reset pulse the presence pulse is sampled. A device
    /// pulls the line low 15 to 60 us after the release and holds it 60 to 240 us, so
    /// 70 us falls inside the presence pulse of every device.
    pub const PRESENCE_SAMPLE_US: u32 = 70;
    /// The rest of the 480 us receive window after the presence sample.
    pub const PRESENCE_TAIL_US: u32 = 410;
    /// Write 1: the line is pulled low briefly, within the 1 to 15 us the slot allows.
    pub const WRITE_ONE_LOW_US: u32 = 6;
    /// Write 1: the line stays released for the rest of the slot plus recovery.
    pub const WRITE_ONE_HIGH_US: u32 = 64;
    /// Write 0: the line is held low for the whole 60 to 120 us slot.
    pub const WRITE_ZERO_LOW_US: u32 = 60;
    /// Write 0: the recovery time before the next slot.
    pub const WRITE_ZERO_HIGH_US: u32 = 10;
    /// Read: the controller starts the slot with a short low pulse of at least 1 us.
    pub const READ_LOW_US: u32 = 2;
    /// Read: the line is sampled this long after release, inside the 15 us window
    /// from the start of the slot in which the device's bit is valid.
    pub const READ_SAMPLE_US: u32 = 10;
    /// Read: the rest of the 60 us slot plus recovery.
    pub const READ_TAIL_US: u32 = 58;
}

/// Computes the Maxim 1-Wire CRC-8 over `data`.
///
/// This is the CRC every 1-Wire device appends to its ROM code, and the DS18B20 to its
/// scratchpad. The polynomial is X^8 + X^5 + X^4 + 1, processed least-significant-bit
/// first from a zero shift register, which is the reflected form `0x8C`.
///
/// # Arguments
///
/// * `data` - the bytes the CRC covers, in transmission order.
///
/// # Returns
///
/// The 8-bit CRC. Over a message followed by its own CRC byte the result is zero.
///
/// # Examples
///
/// ```
/// use pamoja_hal::onewire::crc8;
///
/// // The published check value of CRC-8/MAXIM over the ASCII digits 1 to 9.
/// assert_eq!(crc8(b"123456789"), 0xA1);
/// ```
pub fn crc8(data: &[u8]) -> u8 {
    let mut crc = 0u8;
    for &byte in data {
        let mut bits = byte;
        for _ in 0..8 {
            let mix = (crc ^ bits) & 0x01;
            crc >>= 1;
            if mix != 0 {
                crc ^= 0x8C;
            }
            bits >>= 1;
        }
    }
    crc
}

/// What can go wrong above the pin: the bus protocol itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OneWireError<E> {
    /// The pin or delay underneath the bus failed.
    Pin(E),
    /// No device answered the reset pulse with a presence pulse.
    NoDevice,
    /// A ROM code or a scratchpad arrived with a CRC that does not match its bytes.
    Crc,
}

impl<E> From<E> for OneWireError<E> {
    fn from(error: E) -> Self {
        OneWireError::Pin(error)
    }
}

impl<E: fmt::Debug> fmt::Display for OneWireError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OneWireError::Pin(error) => write!(f, "1-wire pin error: {error:?}"),
            OneWireError::NoDevice => f.write_str("no 1-wire device answered the reset"),
            OneWireError::Crc => f.write_str("1-wire CRC mismatch"),
        }
    }
}

impl<E: fmt::Debug> core::error::Error for OneWireError<E> {}

/// The 64-bit identity every 1-Wire device carries: family, serial, and CRC.
///
/// The bytes are in bus order: the family code first, six serial bytes, then the CRC-8
/// over the first seven.
///
/// # Examples
///
/// ```
/// use pamoja_hal::onewire::RomCode;
///
/// let rom = RomCode::new(0x28, 0x0000_05E2_FDC3).expect("a 48-bit serial");
/// assert_eq!(rom.family(), 0x28);
/// assert_eq!(rom.serial(), 0x0000_05E2_FDC3);
/// assert_eq!(RomCode::from_bytes(rom.bytes())?, rom);
/// # Ok::<(), pamoja_hal::onewire::OneWireError<core::convert::Infallible>>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RomCode([u8; 8]);

impl RomCode {
    /// Builds a ROM code from a family code and a 48-bit serial, computing the CRC.
    ///
    /// # Arguments
    ///
    /// * `family` - the device family code, `0x28` for a DS18B20.
    /// * `serial` - the 48-bit serial number.
    ///
    /// # Returns
    ///
    /// The ROM code, or `None` if `serial` does not fit in 48 bits.
    pub fn new(family: u8, serial: u64) -> Option<RomCode> {
        if serial >> 48 != 0 {
            return None;
        }
        let mut bytes = [0u8; 8];
        bytes[0] = family;
        bytes[1..7].copy_from_slice(&serial.to_le_bytes()[..6]);
        bytes[7] = crc8(&bytes[..7]);
        Some(RomCode(bytes))
    }

    /// Checks the CRC of eight bytes read off the bus and keeps them as a ROM code.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the eight bytes in bus order.
    ///
    /// # Returns
    ///
    /// The ROM code.
    ///
    /// # Errors
    ///
    /// Returns [`OneWireError::Crc`] if the last byte is not the CRC-8 of the first
    /// seven.
    pub fn from_bytes<E>(bytes: [u8; 8]) -> Result<RomCode, OneWireError<E>> {
        if crc8(&bytes[..7]) != bytes[7] {
            return Err(OneWireError::Crc);
        }
        Ok(RomCode(bytes))
    }

    /// Returns the eight bytes in bus order.
    pub fn bytes(&self) -> [u8; 8] {
        self.0
    }

    /// Returns the family code, the first byte.
    pub fn family(&self) -> u8 {
        self.0[0]
    }

    /// Returns the 48-bit serial number.
    pub fn serial(&self) -> u64 {
        let mut serial = [0u8; 8];
        serial[..6].copy_from_slice(&self.0[1..7]);
        u64::from_le_bytes(serial)
    }

    /// Returns the CRC byte.
    pub fn crc(&self) -> u8 {
        self.0[7]
    }

    fn bit(&self, index: u8) -> bool {
        (self.0[usize::from(index / 8)] >> (index % 8)) & 1 == 1
    }

    fn set_bit(&mut self, index: u8, value: bool) {
        let mask = 1 << (index % 8);
        if value {
            self.0[usize::from(index / 8)] |= mask;
        } else {
            self.0[usize::from(index / 8)] &= !mask;
        }
    }
}

/// A 1-Wire bus: the three signaling primitives, and the byte and ROM operations
/// built on them.
///
/// Implement `reset`, `write_bit`, and `read_bit` for a controller; everything else has
/// a default built on those. [`BitBang`] is the implementation over a bare pin.
pub trait OneWireBus {
    /// The error the controller underneath reports.
    type Error;

    /// Sends a reset pulse and reports whether any device answered with presence.
    ///
    /// # Returns
    ///
    /// `true` if at least one device is on the bus.
    ///
    /// # Errors
    ///
    /// Returns the controller's error if the line cannot be driven or read.
    fn reset(&mut self) -> Result<bool, Self::Error>;

    /// Sends one bit in a write slot.
    ///
    /// # Arguments
    ///
    /// * `bit` - the bit to send.
    ///
    /// # Errors
    ///
    /// Returns the controller's error if the line cannot be driven.
    fn write_bit(&mut self, bit: bool) -> Result<(), Self::Error>;

    /// Samples one bit in a read slot.
    ///
    /// # Returns
    ///
    /// The bit the addressed device drove.
    ///
    /// # Errors
    ///
    /// Returns the controller's error if the line cannot be driven or read.
    fn read_bit(&mut self) -> Result<bool, Self::Error>;

    /// Sends one byte, least significant bit first, as the bus requires.
    ///
    /// # Arguments
    ///
    /// * `byte` - the byte to send.
    ///
    /// # Errors
    ///
    /// Returns the controller's error if the line cannot be driven.
    fn write_byte(&mut self, byte: u8) -> Result<(), Self::Error> {
        for shift in 0..8 {
            self.write_bit((byte >> shift) & 1 == 1)?;
        }
        Ok(())
    }

    /// Reads one byte, least significant bit first.
    ///
    /// # Returns
    ///
    /// The byte the device sent.
    ///
    /// # Errors
    ///
    /// Returns the controller's error if the line cannot be driven or read.
    fn read_byte(&mut self) -> Result<u8, Self::Error> {
        let mut byte = 0u8;
        for shift in 0..8 {
            if self.read_bit()? {
                byte |= 1 << shift;
            }
        }
        Ok(byte)
    }

    /// Sends every byte of `bytes` in order.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the bytes to send.
    ///
    /// # Errors
    ///
    /// Returns the controller's error if the line cannot be driven.
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        for &byte in bytes {
            self.write_byte(byte)?;
        }
        Ok(())
    }

    /// Fills `buffer` with bytes read in order.
    ///
    /// # Arguments
    ///
    /// * `buffer` - where the bytes go; its length is how many are read.
    ///
    /// # Errors
    ///
    /// Returns the controller's error if the line cannot be driven or read.
    fn read_bytes(&mut self, buffer: &mut [u8]) -> Result<(), Self::Error> {
        for slot in buffer.iter_mut() {
            *slot = self.read_byte()?;
        }
        Ok(())
    }

    /// Resets the bus and addresses every device at once with `SKIP_ROM`.
    ///
    /// The next function command reaches every device, which is what a bus with a
    /// single device, or a broadcast such as a temperature conversion, wants.
    ///
    /// # Errors
    ///
    /// Returns [`OneWireError::NoDevice`] if nothing answered the reset, or the
    /// controller's error.
    fn skip_rom(&mut self) -> Result<(), OneWireError<Self::Error>> {
        if !self.reset()? {
            return Err(OneWireError::NoDevice);
        }
        self.write_byte(command::SKIP_ROM)?;
        Ok(())
    }

    /// Resets the bus and addresses one device by its ROM code with `MATCH_ROM`.
    ///
    /// # Arguments
    ///
    /// * `rom` - the device to address; the next function command goes to it alone.
    ///
    /// # Errors
    ///
    /// Returns [`OneWireError::NoDevice`] if nothing answered the reset, or the
    /// controller's error.
    fn match_rom(&mut self, rom: &RomCode) -> Result<(), OneWireError<Self::Error>> {
        if !self.reset()? {
            return Err(OneWireError::NoDevice);
        }
        self.write_byte(command::MATCH_ROM)?;
        self.write_bytes(&rom.bytes())?;
        Ok(())
    }

    /// Resets the bus and reads the ROM code of the only device on it.
    ///
    /// With more than one device the answers collide and the CRC fails; use
    /// [`Search`] instead.
    ///
    /// # Returns
    ///
    /// The device's ROM code.
    ///
    /// # Errors
    ///
    /// Returns [`OneWireError::NoDevice`] if nothing answered the reset,
    /// [`OneWireError::Crc`] if the code read does not check, or the controller's
    /// error.
    fn read_rom(&mut self) -> Result<RomCode, OneWireError<Self::Error>> {
        if !self.reset()? {
            return Err(OneWireError::NoDevice);
        }
        self.write_byte(command::READ_ROM)?;
        let mut bytes = [0u8; 8];
        self.read_bytes(&mut bytes)?;
        RomCode::from_bytes(bytes)
    }
}

impl<T: OneWireBus + ?Sized> OneWireBus for &mut T {
    type Error = T::Error;

    fn reset(&mut self) -> Result<bool, Self::Error> {
        (**self).reset()
    }

    fn write_bit(&mut self, bit: bool) -> Result<(), Self::Error> {
        (**self).write_bit(bit)
    }

    fn read_bit(&mut self) -> Result<bool, Self::Error> {
        (**self).read_bit()
    }
}

/// A 1-Wire controller bit-banged over one open-drain pin and a delay.
///
/// The pin must drive the line low when set low and release it when set high, with a
/// pull-up resistor (4.7 kΩ is the datasheet value) raising the released line, and
/// it must read back the line's level. On a microcontroller that is an open-drain
/// output that can be read, or a pin switched between output and input.
///
/// # Examples
///
/// ```
/// use pamoja_hal::digital::PinState::{High, Low};
/// use pamoja_hal::onewire::{BitBang, OneWireBus};
/// use pamoja_hal::script::{DelayLog, PinScript};
///
/// // The scripted pin answers the presence sample low: a device is there.
/// let pin = PinScript::new([Low]);
/// let mut bus = BitBang::new(pin, DelayLog::new());
/// assert!(bus.reset()?);
///
/// let (pin, delay) = bus.into_parts();
/// assert_eq!(pin.driven(), [Low, High]);
/// assert_eq!(delay.total_micros(), 980);
/// # Ok::<(), core::convert::Infallible>(())
/// ```
#[derive(Debug)]
pub struct BitBang<P, D> {
    pin: P,
    delay: D,
}

impl<P, D> BitBang<P, D> {
    /// Creates a controller over `pin`, timed by `delay`.
    ///
    /// # Arguments
    ///
    /// * `pin` - the open-drain line, driven low to signal and released to listen.
    /// * `delay` - the microsecond timer that paces the slots.
    ///
    /// # Returns
    ///
    /// The controller, with the line released.
    pub fn new(pin: P, delay: D) -> BitBang<P, D> {
        BitBang { pin, delay }
    }

    /// Gives back the pin and the delay.
    ///
    /// # Returns
    ///
    /// The pin and delay the controller was built from.
    pub fn into_parts(self) -> (P, D) {
        (self.pin, self.delay)
    }
}

impl<P: OutputPin + InputPin, D: DelayNs> OneWireBus for BitBang<P, D> {
    type Error = P::Error;

    fn reset(&mut self) -> Result<bool, Self::Error> {
        self.pin.set_low()?;
        self.delay.delay_us(timing::RESET_LOW_US);
        self.pin.set_high()?;
        self.delay.delay_us(timing::PRESENCE_SAMPLE_US);
        let present = self.pin.is_low()?;
        self.delay.delay_us(timing::PRESENCE_TAIL_US);
        Ok(present)
    }

    fn write_bit(&mut self, bit: bool) -> Result<(), Self::Error> {
        let (low, high) = if bit {
            (timing::WRITE_ONE_LOW_US, timing::WRITE_ONE_HIGH_US)
        } else {
            (timing::WRITE_ZERO_LOW_US, timing::WRITE_ZERO_HIGH_US)
        };
        self.pin.set_low()?;
        self.delay.delay_us(low);
        self.pin.set_high()?;
        self.delay.delay_us(high);
        Ok(())
    }

    fn read_bit(&mut self) -> Result<bool, Self::Error> {
        self.pin.set_low()?;
        self.delay.delay_us(timing::READ_LOW_US);
        self.pin.set_high()?;
        self.delay.delay_us(timing::READ_SAMPLE_US);
        let bit = self.pin.is_high()?;
        self.delay.delay_us(timing::READ_TAIL_US);
        Ok(bit)
    }
}

/// Enumerates the devices on a bus, one ROM code per call.
///
/// This is the binary tree walk the 1-Wire specification defines: after a
/// `SEARCH_ROM` every device sends each bit of its ROM code and then the complement,
/// the controller writes back the branch it takes, and the devices whose bit differs
/// drop out until one remains. Each call to [`next`](Search::next) returns the next
/// device; the walk remembers where it branched and returns `None` once every device
/// has been named.
///
/// # Examples
///
/// ```
/// use pamoja_hal::onewire::{OneWireBus, RomCode, Search};
///
/// fn thermometers<B: OneWireBus>(bus: &mut B) -> Result<Vec<RomCode>, B::Error> {
///     let mut found = Vec::new();
///     let mut search = Search::new();
///     while let Some(rom) = search.next(bus).map_err(|error| match error {
///         pamoja_hal::onewire::OneWireError::Pin(error) => error,
///         _ => unreachable!("a bus with no devices or a bad CRC ends the search"),
///     })? {
///         if rom.family() == 0x28 {
///             found.push(rom);
///         }
///     }
///     Ok(found)
/// }
/// # let _ = thermometers::<pamoja_hal::onewire::BitBang<pamoja_hal::script::PinScript, pamoja_hal::script::DelayLog>>;
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Search {
    command: u8,
    rom: RomCode,
    last_discrepancy: u8,
    last_device: bool,
}

impl Default for Search {
    fn default() -> Self {
        Search::new()
    }
}

impl Search {
    /// Starts a search that visits every device.
    ///
    /// # Returns
    ///
    /// The search, positioned before the first device.
    pub fn new() -> Search {
        Search::with_command(command::SEARCH_ROM)
    }

    /// Starts a search that visits only the devices with an alarm condition.
    ///
    /// # Returns
    ///
    /// The search, positioned before the first alarming device.
    pub fn alarms() -> Search {
        Search::with_command(command::ALARM_SEARCH)
    }

    fn with_command(command: u8) -> Search {
        Search {
            command,
            rom: RomCode([0; 8]),
            last_discrepancy: 0,
            last_device: false,
        }
    }

    /// Finds the next device on `bus`.
    ///
    /// # Arguments
    ///
    /// * `bus` - the bus to search.
    ///
    /// # Returns
    ///
    /// The next device's ROM code, or `None` when every device has been returned or no
    /// device answered the reset.
    ///
    /// # Errors
    ///
    /// Returns [`OneWireError::Crc`] if a ROM code arrived corrupted, which happens
    /// when a device joins or drops mid-search, or the controller's error.
    pub fn next<B: OneWireBus>(
        &mut self,
        bus: &mut B,
    ) -> Result<Option<RomCode>, OneWireError<B::Error>> {
        if self.last_device {
            return Ok(None);
        }
        if !bus.reset()? {
            self.last_discrepancy = 0;
            return Ok(None);
        }
        bus.write_byte(self.command)?;

        let mut last_zero = 0u8;
        for index in 0..64u8 {
            let bit = bus.read_bit()?;
            let complement = bus.read_bit()?;
            if bit && complement {
                self.last_discrepancy = 0;
                self.last_device = false;
                return Ok(None);
            }
            let position = index + 1;
            let direction = if bit != complement {
                bit
            } else {
                let taken = if position < self.last_discrepancy {
                    self.rom.bit(index)
                } else {
                    position == self.last_discrepancy
                };
                if !taken {
                    last_zero = position;
                }
                taken
            };
            self.rom.set_bit(index, direction);
            bus.write_bit(direction)?;
        }

        self.last_discrepancy = last_zero;
        if last_zero == 0 {
            self.last_device = true;
        }
        RomCode::from_bytes(self.rom.bytes()).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::{DelayLog, PinScript};
    use core::convert::Infallible;
    use embedded_hal::digital::PinState::{High, Low};

    #[test]
    fn crc8_matches_the_published_check_value_and_zeros_over_its_own_crc() {
        assert_eq!(crc8(b"123456789"), 0xA1);
        let rom = RomCode::new(0x28, 0x0000_05E2_FDC3).unwrap();
        assert_eq!(crc8(&rom.bytes()), 0);
    }

    #[test]
    fn a_rom_code_keeps_its_family_serial_and_crc_in_bus_order() {
        let rom = RomCode::new(0x28, 0x1234_5678_9ABC).unwrap();
        let bytes = rom.bytes();
        assert_eq!(bytes[0], 0x28);
        assert_eq!(&bytes[1..7], &[0xBC, 0x9A, 0x78, 0x56, 0x34, 0x12]);
        assert_eq!(bytes[7], crc8(&bytes[..7]));
        assert_eq!(rom.serial(), 0x1234_5678_9ABC);
        assert_eq!(rom.crc(), bytes[7]);
        let mut corrupted = bytes;
        corrupted[3] ^= 0x01;
        assert_eq!(
            RomCode::from_bytes::<Infallible>(corrupted),
            Err(OneWireError::Crc)
        );
        assert!(RomCode::new(0x28, 1 << 48).is_none());
    }

    #[test]
    fn reset_drives_the_datasheet_pulse_and_samples_presence() {
        let mut bus = BitBang::new(PinScript::new([Low]), DelayLog::new());
        assert!(bus.reset().unwrap());
        let (pin, delay) = bus.into_parts();
        assert_eq!(pin.driven(), [Low, High]);
        assert_eq!(delay.waits_ns(), [500_000, 70_000, 410_000]);

        let mut empty = BitBang::new(PinScript::new([High]), DelayLog::new());
        assert!(!empty.reset().unwrap());
    }

    #[test]
    fn write_slots_hold_the_line_low_for_the_bit_they_carry() {
        let mut bus = BitBang::new(PinScript::new([]), DelayLog::new());
        bus.write_bit(true).unwrap();
        bus.write_bit(false).unwrap();
        let (pin, delay) = bus.into_parts();
        assert_eq!(pin.driven(), [Low, High, Low, High]);
        assert_eq!(delay.waits_ns(), [6_000, 64_000, 60_000, 10_000]);
    }

    #[test]
    fn a_byte_goes_out_least_significant_bit_first() {
        let mut bus = BitBang::new(PinScript::new([]), DelayLog::new());
        bus.write_byte(0xCC).unwrap();
        let (_, delay) = bus.into_parts();
        let bits: alloc::vec::Vec<bool> = delay
            .waits_ns()
            .chunks(2)
            .map(|slot| slot[0] == 6_000)
            .collect();
        assert_eq!(
            bits,
            [false, false, true, true, false, false, true, true],
            "0xCC is 1100 1100, sent from bit 0"
        );
    }

    #[test]
    fn read_slots_sample_inside_the_window_and_assemble_a_byte() {
        let levels = [High, Low, High, High, Low, Low, High, Low];
        let mut bus = BitBang::new(PinScript::new(levels), DelayLog::new());
        assert_eq!(bus.read_byte().unwrap(), 0b0100_1101);
        let (pin, delay) = bus.into_parts();
        assert_eq!(pin.driven().len(), 16);
        assert_eq!(&delay.waits_ns()[..3], &[2_000, 10_000, 58_000]);
    }

    #[test]
    fn skip_and_match_rom_fail_when_nothing_answers() {
        let mut bus = BitBang::new(PinScript::new([High]), DelayLog::new());
        assert_eq!(bus.skip_rom(), Err(OneWireError::NoDevice));
        let rom = RomCode::new(0x28, 1).unwrap();
        let mut bus = BitBang::new(PinScript::new([High]), DelayLog::new());
        assert_eq!(bus.match_rom(&rom), Err(OneWireError::NoDevice));
    }

    #[test]
    fn read_rom_returns_the_single_device_and_checks_its_crc() {
        let rom = RomCode::new(0x28, 0x0000_05E2_FDC3).unwrap();
        let mut levels = alloc::vec![Low];
        for byte in rom.bytes() {
            for shift in 0..8 {
                levels.push(if (byte >> shift) & 1 == 1 { High } else { Low });
            }
        }
        let mut bus = BitBang::new(PinScript::new(levels.clone()), DelayLog::new());
        assert_eq!(bus.read_rom().unwrap(), rom);

        levels[9] = if levels[9] == High { Low } else { High };
        let mut bus = BitBang::new(PinScript::new(levels), DelayLog::new());
        assert_eq!(bus.read_rom(), Err(OneWireError::Crc));
    }

    /// A bus of simulated devices that answer a search the way real ones do: every
    /// device still in the running drives its bit and its complement, and drops out
    /// when the controller takes the other branch.
    struct SimulatedBus {
        devices: alloc::vec::Vec<RomCode>,
        active: alloc::vec::Vec<bool>,
        bit_index: u8,
        phase: u8,
    }

    impl SimulatedBus {
        fn new(devices: &[RomCode]) -> Self {
            SimulatedBus {
                devices: devices.to_vec(),
                active: alloc::vec![true; devices.len()],
                bit_index: 0,
                phase: 0,
            }
        }

        fn wired_and(&self, complement: bool) -> bool {
            self.devices
                .iter()
                .zip(&self.active)
                .filter(|(_, active)| **active)
                .all(|(device, _)| device.bit(self.bit_index) != complement)
        }
    }

    impl OneWireBus for SimulatedBus {
        type Error = Infallible;

        fn reset(&mut self) -> Result<bool, Infallible> {
            self.active.iter_mut().for_each(|active| *active = true);
            self.bit_index = 0;
            self.phase = 0;
            Ok(!self.devices.is_empty())
        }

        fn write_bit(&mut self, bit: bool) -> Result<(), Infallible> {
            if self.phase == 2 {
                let index = self.bit_index;
                for (device, active) in self.devices.iter().zip(self.active.iter_mut()) {
                    if device.bit(index) != bit {
                        *active = false;
                    }
                }
                self.bit_index += 1;
                self.phase = 0;
            }
            Ok(())
        }

        fn read_bit(&mut self) -> Result<bool, Infallible> {
            let bit = match self.phase {
                0 => self.wired_and(false),
                _ => self.wired_and(true),
            };
            self.phase += 1;
            Ok(bit)
        }
    }

    #[test]
    fn a_search_names_every_device_once_and_then_stops() {
        let devices = [
            RomCode::new(0x28, 0x0000_05E2_FDC3).unwrap(),
            RomCode::new(0x28, 0x0000_0A11_0042).unwrap(),
            RomCode::new(0x10, 0x0000_0000_0001).unwrap(),
            RomCode::new(0x28, 0x0000_05E2_FDC2).unwrap(),
        ];
        let mut bus = SimulatedBus::new(&devices);
        let mut search = Search::new();
        let mut found = alloc::vec::Vec::new();
        while let Some(rom) = search.next(&mut bus).unwrap() {
            found.push(rom);
        }
        assert_eq!(found.len(), devices.len());
        for device in &devices {
            assert!(found.contains(device), "{device:?} was not found");
        }
        assert_eq!(search.next(&mut bus).unwrap(), None);
    }

    #[test]
    fn a_search_of_an_empty_bus_finds_nothing() {
        let mut bus = SimulatedBus::new(&[]);
        assert_eq!(Search::new().next(&mut bus).unwrap(), None);
    }

    #[test]
    fn a_search_of_one_device_returns_it_and_finishes() {
        let only = RomCode::new(0x28, 7).unwrap();
        let mut bus = SimulatedBus::new(&[only]);
        let mut search = Search::new();
        assert_eq!(search.next(&mut bus).unwrap(), Some(only));
        assert_eq!(search.next(&mut bus).unwrap(), None);
    }
}
