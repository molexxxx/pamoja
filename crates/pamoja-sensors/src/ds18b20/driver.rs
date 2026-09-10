//! The DS18B20 driven over a 1-Wire bus: address, convert, wait, read the scratchpad.

use embedded_hal::delay::DelayNs;
use pamoja_core::Sensor;
use pamoja_hal::onewire::{OneWireBus, OneWireError, RomCode};

use super::{command, Resolution, Scratchpad, EEPROM_WRITE_MICROS};
use crate::error::DriverError;

/// A DS18B20 on a 1-Wire bus, measuring on demand.
///
/// [`Ds18b20::new`] addresses the only thermometer on the bus with `SKIP ROM`;
/// [`Ds18b20::addressed`] addresses one of several by its ROM code with `MATCH ROM`.
/// [`init`](Ds18b20::init) writes the resolution and alarm thresholds to the
/// scratchpad and reads it back to confirm the part answers.
/// [`measure`](Ds18b20::measure) issues `CONVERT T`, waits the datasheet's maximum
/// conversion time for the resolution in use, then reads the nine-byte scratchpad,
/// whose CRC is checked before a temperature is taken from it. As a [`Sensor`] its
/// reading is the [`Scratchpad`]; one channel is selected with [`Sensor::map`].
///
/// The driver assumes the part is powered from its VDD pin. A part on parasite
/// power needs a strong pull-up during every conversion, which the bus underneath
/// has to supply; [`powered_externally`](Ds18b20::powered_externally) reports which
/// wiring the part has.
///
/// # Examples
///
/// A bit-banged bus over a scripted pin: the pin answers the presence sample low,
/// then shifts out a scratchpad of a part at 25.0625 C.
///
/// ```
/// use pamoja_core::Sensor;
/// use pamoja_hal::digital::PinState::{High, Low};
/// use pamoja_hal::onewire::BitBang;
/// use pamoja_hal::script::{block_on, DelayLog, PinScript};
/// use pamoja_sensors::ds18b20::{temperature_from_celsius, Ds18b20, Resolution, Scratchpad};
///
/// let raw = temperature_from_celsius(25.0625, Resolution::Bits12);
/// let scratchpad = Scratchpad::new(raw, Resolution::Bits12, 75, -10).to_bytes();
/// let mut levels = vec![Low, Low];
/// for byte in scratchpad {
///     levels.extend((0..8).map(|bit| if (byte >> bit) & 1 == 1 { High } else { Low }));
/// }
///
/// let bus = BitBang::new(PinScript::new(levels), DelayLog::new());
/// let mut probe = Ds18b20::new(bus, DelayLog::new());
/// let mut celsius = probe.map(|scratchpad| scratchpad.temperature_celsius());
/// assert_eq!(block_on(celsius.read())?, 25.0625);
/// # Ok::<(), pamoja_core::Error>(())
/// ```
#[derive(Debug)]
pub struct Ds18b20<B, D> {
    bus: B,
    delay: D,
    rom: Option<RomCode>,
    resolution: Resolution,
    alarm_high: i8,
    alarm_low: i8,
}

impl<B, D> Ds18b20<B, D> {
    /// Wraps a bus carrying a single thermometer.
    ///
    /// # Arguments
    ///
    /// * `bus` - the 1-Wire bus.
    /// * `delay` - the timer that paces the conversions.
    ///
    /// # Returns
    ///
    /// The driver, at 12-bit resolution with the alarm thresholds at the part's
    /// limits.
    pub fn new(bus: B, delay: D) -> Self {
        Ds18b20 {
            bus,
            delay,
            rom: None,
            resolution: Resolution::Bits12,
            alarm_high: 125,
            alarm_low: -55,
        }
    }

    /// Wraps a bus carrying several devices and addresses one by its ROM code.
    ///
    /// # Arguments
    ///
    /// * `bus` - the 1-Wire bus.
    /// * `delay` - the timer that paces the conversions.
    /// * `rom` - the thermometer's ROM code, as a
    ///   [`Search`](pamoja_hal::onewire::Search) returns it.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn addressed(bus: B, delay: D, rom: RomCode) -> Self {
        let mut probe = Ds18b20::new(bus, delay);
        probe.rom = Some(rom);
        probe
    }

    /// Sets the conversion resolution, which also sets the conversion time.
    ///
    /// The setting is written to the part by [`init`](Ds18b20::init).
    ///
    /// # Arguments
    ///
    /// * `resolution` - 9 to 12 bits.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_resolution(mut self, resolution: Resolution) -> Self {
        self.resolution = resolution;
        self
    }

    /// Sets the alarm thresholds the part compares each conversion against.
    ///
    /// The thresholds are written to the part by [`init`](Ds18b20::init) and matter
    /// to an [alarm search](pamoja_hal::onewire::Search::alarms).
    ///
    /// # Arguments
    ///
    /// * `high` - the upper threshold in whole degrees Celsius.
    /// * `low` - the lower threshold in whole degrees Celsius.
    ///
    /// # Returns
    ///
    /// The driver.
    pub fn with_alarms(mut self, high: i8, low: i8) -> Self {
        self.alarm_high = high;
        self.alarm_low = low;
        self
    }

    /// Returns the ROM code the driver addresses, or `None` for the only device.
    pub fn rom(&self) -> Option<RomCode> {
        self.rom
    }

    /// Returns the resolution the driver converts at.
    pub fn resolution(&self) -> Resolution {
        self.resolution
    }

    /// Gives back the bus and the delay.
    ///
    /// # Returns
    ///
    /// The bus and delay the driver was built from.
    pub fn release(self) -> (B, D) {
        (self.bus, self.delay)
    }
}

impl<B: OneWireBus, D: DelayNs> Ds18b20<B, D> {
    /// Writes the resolution and alarm thresholds and reads the scratchpad back.
    ///
    /// The sequence is the datasheet's `WRITE SCRATCHPAD` of the three configurable
    /// bytes, then a `READ SCRATCHPAD` whose CRC and configuration byte confirm the
    /// part took them. The settings live in the scratchpad until power is lost;
    /// [`save_settings`](Ds18b20::save_settings) copies them to the EEPROM.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if no device answers or the line fails,
    /// [`DriverError::Sensor`] if the scratchpad read back fails its CRC or does not
    /// hold the resolution written.
    pub fn init(&mut self) -> Result<(), DriverError<OneWireError<B::Error>>> {
        self.select()?;
        let scratchpad = [
            command::WRITE_SCRATCHPAD,
            self.alarm_high as u8,
            self.alarm_low as u8,
            self.resolution.config_byte(),
        ];
        self.bus
            .write_bytes(&scratchpad)
            .map_err(|error| DriverError::Bus(OneWireError::Pin(error)))?;
        let readback = self.read_scratchpad()?;
        if readback.resolution() != self.resolution {
            return Err(DriverError::Sensor(crate::SensorError::Invalid));
        }
        Ok(())
    }

    /// Runs one conversion and returns the scratchpad.
    ///
    /// The sequence is the datasheet's: address the part, `CONVERT T`, wait the
    /// maximum conversion time for the resolution, address the part again, and
    /// `READ SCRATCHPAD`. The resolution the driver waits for is the one it holds,
    /// so run [`init`](Ds18b20::init) first if the part's EEPROM setting differs.
    ///
    /// # Returns
    ///
    /// The CRC-checked scratchpad, with the temperature in it.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if no device answers or the line fails, and
    /// [`DriverError::Sensor`] if the scratchpad fails its CRC.
    pub fn measure(&mut self) -> Result<Scratchpad, DriverError<OneWireError<B::Error>>> {
        self.select()?;
        self.bus
            .write_byte(command::CONVERT_T)
            .map_err(|error| DriverError::Bus(OneWireError::Pin(error)))?;
        self.delay.delay_us(self.resolution.max_conversion_micros());
        self.read_scratchpad()
    }

    /// Reads the scratchpad without converting, for the last result or the settings.
    ///
    /// # Returns
    ///
    /// The CRC-checked scratchpad.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if no device answers or the line fails, and
    /// [`DriverError::Sensor`] if the scratchpad fails its CRC.
    pub fn read_scratchpad(&mut self) -> Result<Scratchpad, DriverError<OneWireError<B::Error>>> {
        self.select()?;
        self.bus
            .write_byte(command::READ_SCRATCHPAD)
            .map_err(|error| DriverError::Bus(OneWireError::Pin(error)))?;
        let mut bytes = [0u8; 9];
        self.bus
            .read_bytes(&mut bytes)
            .map_err(|error| DriverError::Bus(OneWireError::Pin(error)))?;
        Ok(Scratchpad::parse(&bytes)?)
    }

    /// Copies the scratchpad settings to the EEPROM, so they survive power loss.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if no device answers or the line fails.
    pub fn save_settings(&mut self) -> Result<(), DriverError<OneWireError<B::Error>>> {
        self.select()?;
        self.bus
            .write_byte(command::COPY_SCRATCHPAD)
            .map_err(|error| DriverError::Bus(OneWireError::Pin(error)))?;
        self.delay.delay_us(EEPROM_WRITE_MICROS);
        Ok(())
    }

    /// Asks the part whether it is powered from VDD or parasitically from the line.
    ///
    /// # Returns
    ///
    /// `true` for external power, `false` for parasite power, which needs a strong
    /// pull-up during conversions.
    ///
    /// # Errors
    ///
    /// Returns [`DriverError::Bus`] if no device answers or the line fails.
    pub fn powered_externally(&mut self) -> Result<bool, DriverError<OneWireError<B::Error>>> {
        self.select()?;
        self.bus
            .write_byte(command::READ_POWER_SUPPLY)
            .map_err(|error| DriverError::Bus(OneWireError::Pin(error)))?;
        self.bus
            .read_bit()
            .map_err(|error| DriverError::Bus(OneWireError::Pin(error)))
    }

    fn select(&mut self) -> Result<(), DriverError<OneWireError<B::Error>>> {
        match self.rom {
            Some(rom) => self.bus.match_rom(&rom),
            None => self.bus.skip_rom(),
        }
        .map_err(DriverError::Bus)
    }
}

impl<B, D> Sensor for Ds18b20<B, D>
where
    B: OneWireBus + Send,
    B::Error: core::fmt::Debug,
    D: DelayNs + Send,
{
    type Reading = Scratchpad;

    async fn read(&mut self) -> pamoja_core::Result<Scratchpad> {
        self.measure().map_err(pamoja_core::Error::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ds18b20::temperature_from_celsius;
    use crate::SensorError;
    use alloc::vec;
    use alloc::vec::Vec;
    use core::convert::Infallible;
    use pamoja_hal::digital::PinState::{High, Low};
    use pamoja_hal::onewire::{command as rom_command, BitBang};
    use pamoja_hal::script::{block_on, DelayLog, PinScript};

    /// A thermometer on a simulated bus: it records every byte written after a reset
    /// and answers the function commands the way the datasheet describes.
    struct Simulated {
        rom: RomCode,
        present: bool,
        scratchpad: [u8; 9],
        written: Vec<u8>,
        reading: Vec<bool>,
        external_power: bool,
    }

    impl Simulated {
        fn new(celsius: f32) -> Simulated {
            let raw = temperature_from_celsius(celsius, Resolution::Bits12);
            Simulated {
                rom: RomCode::new(0x28, 0x0000_05E2_FDC3).unwrap(),
                present: true,
                scratchpad: Scratchpad::new(raw, Resolution::Bits12, 125, -55).to_bytes(),
                written: Vec::new(),
                reading: Vec::new(),
                external_power: true,
            }
        }

        fn queue_bytes(&mut self, bytes: &[u8]) {
            for &byte in bytes {
                self.reading
                    .extend((0..8).map(|bit| (byte >> bit) & 1 == 1));
            }
        }

        fn act(&mut self) {
            let addressed = match self.written.first() {
                Some(&rom_command::SKIP_ROM) => 1,
                Some(&rom_command::MATCH_ROM)
                    if self.written.len() >= 9 && self.written[1..9] == self.rom.bytes() =>
                {
                    9
                }
                _ => return,
            };
            match self.written.get(addressed) {
                Some(&command::READ_SCRATCHPAD) => {
                    let scratchpad = self.scratchpad;
                    self.queue_bytes(&scratchpad);
                }
                Some(&command::WRITE_SCRATCHPAD) if self.written.len() == addressed + 4 => {
                    self.scratchpad[2] = self.written[addressed + 1];
                    self.scratchpad[3] = self.written[addressed + 2];
                    self.scratchpad[4] = self.written[addressed + 3];
                    self.scratchpad[8] = crate::ds18b20::crc8(&self.scratchpad[..8]);
                }
                Some(&command::READ_POWER_SUPPLY) => {
                    let external = self.external_power;
                    self.reading.push(external);
                }
                _ => {}
            }
        }
    }

    impl OneWireBus for Simulated {
        type Error = Infallible;

        fn reset(&mut self) -> Result<bool, Infallible> {
            self.written.clear();
            self.reading.clear();
            Ok(self.present)
        }

        fn write_bit(&mut self, _bit: bool) -> Result<(), Infallible> {
            unreachable!("bytes are written whole")
        }

        fn read_bit(&mut self) -> Result<bool, Infallible> {
            Ok(if self.reading.is_empty() {
                true
            } else {
                self.reading.remove(0)
            })
        }

        fn write_byte(&mut self, byte: u8) -> Result<(), Infallible> {
            self.written.push(byte);
            self.act();
            Ok(())
        }
    }

    #[test]
    fn measure_converts_then_reads_a_crc_checked_scratchpad() {
        let mut probe = Ds18b20::new(Simulated::new(25.0625), DelayLog::new());
        let scratchpad = probe.measure().unwrap();
        assert_eq!(scratchpad.temperature_celsius(), 25.0625);
        assert_eq!(scratchpad.resolution(), Resolution::Bits12);
        let (bus, delay) = probe.release();
        assert_eq!(
            bus.written,
            [rom_command::SKIP_ROM, command::READ_SCRATCHPAD]
        );
        assert_eq!(delay.waits_ns(), [750_000_000]);
    }

    #[test]
    fn init_writes_the_settings_and_confirms_them_from_the_readback() {
        let mut probe = Ds18b20::new(Simulated::new(20.0), DelayLog::new())
            .with_resolution(Resolution::Bits9)
            .with_alarms(30, -5);
        probe.init().unwrap();
        let scratchpad = probe.read_scratchpad().unwrap();
        assert_eq!(scratchpad.resolution(), Resolution::Bits9);
        assert_eq!(scratchpad.alarm_high(), 30);
        assert_eq!(scratchpad.alarm_low(), -5);
        probe.measure().unwrap();
        let (_, delay) = probe.release();
        assert_eq!(
            delay.waits_ns(),
            [93_750_000],
            "a 9-bit conversion waits 93.75 ms"
        );
    }

    #[test]
    fn an_addressed_probe_matches_its_rom_and_a_stranger_is_ignored() {
        let bus = Simulated::new(18.5);
        let rom = bus.rom;
        let mut probe = Ds18b20::addressed(bus, DelayLog::new(), rom);
        assert_eq!(probe.rom(), Some(rom));
        assert_eq!(probe.measure().unwrap().temperature_celsius(), 18.5);

        let stranger = RomCode::new(0x28, 0x0000_0000_0001).unwrap();
        let mut wrong = Ds18b20::addressed(Simulated::new(18.5), DelayLog::new(), stranger);
        assert_eq!(wrong.measure(), Err(DriverError::Sensor(SensorError::Crc)));
    }

    #[test]
    fn an_empty_bus_and_a_corrupted_scratchpad_are_reported_apart() {
        let mut absent = Simulated::new(0.0);
        absent.present = false;
        let mut probe = Ds18b20::new(absent, DelayLog::new());
        assert_eq!(
            probe.measure(),
            Err(DriverError::Bus(OneWireError::NoDevice))
        );

        let mut noisy = Simulated::new(0.0);
        noisy.scratchpad[1] ^= 0x10;
        let mut probe = Ds18b20::new(noisy, DelayLog::new());
        assert_eq!(probe.measure(), Err(DriverError::Sensor(SensorError::Crc)));
    }

    #[test]
    fn power_supply_and_eeprom_copy_follow_their_commands() {
        let mut parasite = Simulated::new(0.0);
        parasite.external_power = false;
        let mut probe = Ds18b20::new(parasite, DelayLog::new());
        assert!(!probe.powered_externally().unwrap());
        probe.save_settings().unwrap();
        let (bus, delay) = probe.release();
        assert_eq!(
            bus.written,
            [rom_command::SKIP_ROM, command::COPY_SCRATCHPAD]
        );
        assert_eq!(delay.waits_ns(), [EEPROM_WRITE_MICROS * 1_000]);
    }

    #[test]
    fn over_a_bit_banged_pin_the_wire_carries_the_datasheet_sequence() {
        let raw = temperature_from_celsius(-10.125, Resolution::Bits12);
        let bytes = Scratchpad::new(raw, Resolution::Bits12, 125, -55).to_bytes();
        let mut levels = vec![Low, Low];
        for byte in bytes {
            levels.extend((0..8).map(|bit| if (byte >> bit) & 1 == 1 { High } else { Low }));
        }
        let bus = BitBang::new(PinScript::new(levels), DelayLog::new());
        let mut probe = Ds18b20::new(bus, DelayLog::new());
        let scratchpad = probe.measure().unwrap();
        assert_eq!(scratchpad.temperature_celsius(), -10.125);
        let (bus, _) = probe.release();
        let (pin, _) = bus.into_parts();
        assert_eq!(
            pin.driven().len(),
            2 + 16 + 16 + 2 + 16 + 16 + 144,
            "two resets, four command bytes, and 72 read slots, two edges each"
        );
        assert_eq!(pin.remaining(), 0);
    }

    #[test]
    fn as_a_sensor_it_reads_and_maps_to_celsius() {
        let mut celsius = Ds18b20::new(Simulated::new(85.0), DelayLog::new())
            .map(|scratchpad| scratchpad.temperature_celsius());
        assert_eq!(block_on(celsius.read()).unwrap(), 85.0);
    }
}
