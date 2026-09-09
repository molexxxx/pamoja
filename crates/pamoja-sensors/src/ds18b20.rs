//! Maxim DS18B20 1-Wire digital thermometer.
//!
//! The DS18B20 reports temperature as a 16-bit two's-complement number in a nine-byte
//! scratchpad, with a CRC byte that covers the rest. This module decodes that
//! temperature exactly as the datasheet's temperature/data table specifies, reads the
//! resolution out of the configuration byte, and verifies the scratchpad's CRC with
//! the Maxim 1-Wire polynomial so a read corrupted on the bus is caught rather than
//! trusted.
//!
//! It is pure logic: a caller drives the 1-Wire transactions (convert, then read
//! scratchpad) and hands the nine bytes to [`Scratchpad::parse`].

use crate::SensorError;

#[cfg(feature = "embedded-hal")]
mod driver;

#[cfg(feature = "embedded-hal")]
pub use driver::Ds18b20;

/// The function commands a DS18B20 answers once it has been addressed.
pub mod command {
    /// Start a temperature conversion; the result lands in the scratchpad.
    pub const CONVERT_T: u8 = 0x44;
    /// Send the nine scratchpad bytes, CRC last.
    pub const READ_SCRATCHPAD: u8 = 0xBE;
    /// Take the alarm thresholds and the configuration byte, three bytes in that order.
    pub const WRITE_SCRATCHPAD: u8 = 0x4E;
    /// Copy those three bytes from the scratchpad to the EEPROM.
    pub const COPY_SCRATCHPAD: u8 = 0x48;
    /// Reload the three EEPROM bytes into the scratchpad.
    pub const RECALL_EEPROM: u8 = 0xB8;
    /// Answer one bit: 1 for external power, 0 for parasite power.
    pub const READ_POWER_SUPPLY: u8 = 0xB4;
}

/// How long a copy to the EEPROM takes, in microseconds.
pub const EEPROM_WRITE_MICROS: u32 = 10_000;

/// The 1-Wire family code that identifies a DS18B20 in the first ROM byte.
pub const FAMILY_CODE: u8 = 0x28;

/// The conversion resolution, selected by the R1/R0 bits of the configuration byte.
///
/// Higher resolution resolves smaller steps but takes longer to convert, a tradeoff
/// the datasheet spells out. The temperature register always carries 1/16 °C per
/// count; at lower resolutions the unused low bits simply read as zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution {
    /// 9-bit, 0.5 °C steps, up to 93.75 ms per conversion.
    Bits9,
    /// 10-bit, 0.25 °C steps, up to 187.5 ms per conversion.
    Bits10,
    /// 11-bit, 0.125 °C steps, up to 375 ms per conversion.
    Bits11,
    /// 12-bit, 0.0625 °C steps, up to 750 ms per conversion. The power-on default.
    Bits12,
}

impl Resolution {
    /// Returns the number of significant bits this resolution produces.
    ///
    /// # Returns
    ///
    /// `9`, `10`, `11`, or `12`.
    pub fn bits(self) -> u8 {
        match self {
            Resolution::Bits9 => 9,
            Resolution::Bits10 => 10,
            Resolution::Bits11 => 11,
            Resolution::Bits12 => 12,
        }
    }

    /// Returns the configuration-register byte that selects this resolution.
    ///
    /// The byte places R1/R0 in bits 6:5 over the datasheet's fixed surrounding
    /// pattern (bit 7 clear, bit 4 set, bits 3:0 set), so 12-bit is `0x7F`, 11-bit
    /// `0x5F`, 10-bit `0x3F`, and 9-bit `0x1F`.
    ///
    /// # Returns
    ///
    /// The configuration byte written to scratchpad byte 4.
    pub fn config_byte(self) -> u8 {
        let r1r0 = match self {
            Resolution::Bits9 => 0b00,
            Resolution::Bits10 => 0b01,
            Resolution::Bits11 => 0b10,
            Resolution::Bits12 => 0b11,
        };
        0b0001_1111 | (r1r0 << 5)
    }

    /// Reads the resolution out of a configuration byte's R1/R0 bits.
    ///
    /// # Arguments
    ///
    /// * `byte` - the configuration register (scratchpad byte 4).
    ///
    /// # Returns
    ///
    /// The resolution selected by bits 6:5.
    pub fn from_config_byte(byte: u8) -> Resolution {
        match (byte >> 5) & 0b11 {
            0b00 => Resolution::Bits9,
            0b01 => Resolution::Bits10,
            0b10 => Resolution::Bits11,
            _ => Resolution::Bits12,
        }
    }

    /// Returns the temperature step this resolution resolves, in micro-degrees Celsius.
    ///
    /// # Returns
    ///
    /// `500000` (0.5 °C) for 9-bit down to `62500` (0.0625 °C) for 12-bit.
    pub fn step_micro_celsius(self) -> u32 {
        match self {
            Resolution::Bits9 => 500_000,
            Resolution::Bits10 => 250_000,
            Resolution::Bits11 => 125_000,
            Resolution::Bits12 => 62_500,
        }
    }

    /// Returns the datasheet's maximum conversion time, in microseconds.
    ///
    /// # Returns
    ///
    /// `93750` for 9-bit, doubling up to `750000` for 12-bit.
    pub fn max_conversion_micros(self) -> u32 {
        match self {
            Resolution::Bits9 => 93_750,
            Resolution::Bits10 => 187_500,
            Resolution::Bits11 => 375_000,
            Resolution::Bits12 => 750_000,
        }
    }
}

/// Converts a raw temperature register value to micro-degrees Celsius, exactly.
///
/// Each count is 1/16 °C, which is 62500 micro-degrees, so the conversion is exact in
/// integer arithmetic and needs no floating point.
///
/// # Arguments
///
/// * `raw` - the 16-bit two's-complement temperature register, as a signed value.
///
/// # Returns
///
/// The temperature in micro-degrees Celsius (millionths of a degree).
pub fn temperature_to_micro_celsius(raw: i16) -> i32 {
    raw as i32 * 62_500
}

/// Converts a raw temperature register value to degrees Celsius.
///
/// # Arguments
///
/// * `raw` - the 16-bit two's-complement temperature register, as a signed value.
///
/// # Returns
///
/// The temperature in degrees Celsius.
pub fn temperature_to_celsius(raw: i16) -> f32 {
    raw as f32 / 16.0
}

/// Converts micro-degrees Celsius to a raw temperature register value.
///
/// The result is truncated to the step `resolution` resolves, since the datasheet
/// specifies that the bits below a selected resolution read as zero.
///
/// # Arguments
///
/// * `micro_celsius` - the temperature in millionths of a degree Celsius.
/// * `resolution` - the resolution the part is configured for.
///
/// # Returns
///
/// The 16-bit two's-complement temperature register, as a signed value.
pub fn temperature_from_micro_celsius(micro_celsius: i32, resolution: Resolution) -> i16 {
    let counts = micro_celsius.div_euclid(62_500) as i16;
    let unused = 12 - resolution.bits();
    (counts >> unused) << unused
}

/// Converts degrees Celsius to a raw temperature register value.
///
/// # Arguments
///
/// * `celsius` - the temperature in degrees Celsius.
/// * `resolution` - the resolution the part is configured for.
///
/// # Returns
///
/// The 16-bit two's-complement temperature register, as a signed value.
pub fn temperature_from_celsius(celsius: f32, resolution: Resolution) -> i16 {
    temperature_from_micro_celsius((celsius * 1_000_000.0) as i32, resolution)
}

/// Computes the Maxim 1-Wire CRC-8 over `data`.
///
/// This is the CRC the DS18B20 (and every Maxim 1-Wire device) appends to its ROM
/// code and scratchpad. The polynomial is X^8 + X^5 + X^4 + 1, processed
/// least-significant-bit first from a zero shift register, which is the reflected
/// form `0x8C`.
///
/// # Arguments
///
/// * `data` - the bytes the CRC covers, in transmission order.
///
/// # Returns
///
/// The 8-bit CRC; for a correctly received message followed by its CRC byte, running
/// this over all of them yields zero.
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

/// A decoded, CRC-verified DS18B20 scratchpad.
///
/// The scratchpad is nine bytes: temperature LSB and MSB, the high and low alarm
/// thresholds, the configuration byte, three reserved bytes, and a CRC. [`parse`]
/// checks the CRC before exposing any of it.
///
/// [`parse`]: Scratchpad::parse
///
/// # Examples
///
/// ```
/// use pamoja_sensors::ds18b20::{temperature_from_celsius, Resolution, Scratchpad};
///
/// // What a part sitting at +25.0625 °C, set to 12-bit steps with its thresholds at
/// // +75/-10 °C, puts on the bus. A driver reads these nine bytes off the wire.
/// let bytes = Scratchpad::new(
///     temperature_from_celsius(25.0625, Resolution::Bits12),
///     Resolution::Bits12,
///     75,
///     -10,
/// )
/// .to_bytes();
///
/// let scratchpad = Scratchpad::parse(&bytes)?;
/// assert_eq!(scratchpad.temperature_micro_celsius(), 25_062_500);
/// assert_eq!(scratchpad.resolution(), Resolution::Bits12);
/// # Ok::<(), pamoja_sensors::SensorError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scratchpad {
    raw_temperature: i16,
    alarm_high: i8,
    alarm_low: i8,
    resolution: Resolution,
}

impl Scratchpad {
    /// Parses and CRC-checks a nine-byte scratchpad.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the nine scratchpad bytes in the order the device sends them, the
    ///   ninth being the CRC.
    ///
    /// # Returns
    ///
    /// The decoded scratchpad.
    ///
    /// # Errors
    ///
    /// Returns [`SensorError::Crc`] if the CRC byte does not match the first eight,
    /// which means the read was corrupted and should be repeated.
    pub fn parse(bytes: &[u8; 9]) -> Result<Scratchpad, SensorError> {
        if crc8(&bytes[..8]) != bytes[8] {
            return Err(SensorError::Crc);
        }
        let raw_temperature = i16::from_le_bytes([bytes[0], bytes[1]]);
        Ok(Scratchpad {
            raw_temperature,
            alarm_high: bytes[2] as i8,
            alarm_low: bytes[3] as i8,
            resolution: Resolution::from_config_byte(bytes[4]),
        })
    }

    /// Builds the scratchpad a part in the given state reports.
    ///
    /// This is the inverse of [`parse`](Self::parse), so a node can be developed and
    /// tested against the bytes a thermometer would send without one attached.
    ///
    /// # Arguments
    ///
    /// * `raw_temperature` - the 16-bit two's-complement temperature register, as a
    ///   signed value; [`temperature_from_celsius`] builds one from a temperature.
    /// * `resolution` - the resolution the part is configured for.
    /// * `alarm_high` - the high alarm threshold in whole degrees Celsius (TH).
    /// * `alarm_low` - the low alarm threshold in whole degrees Celsius (TL).
    ///
    /// # Returns
    ///
    /// The scratchpad holding those values.
    pub fn new(
        raw_temperature: i16,
        resolution: Resolution,
        alarm_high: i8,
        alarm_low: i8,
    ) -> Scratchpad {
        Scratchpad {
            raw_temperature,
            alarm_high,
            alarm_low,
            resolution,
        }
    }

    /// Returns the nine bytes a part holding this scratchpad puts on the bus.
    ///
    /// Bytes 5 to 7 are the reserved bytes, which a part reports but which carry no
    /// reading; they are filled with the values its scratchpad figure shows so the
    /// frame is the one a device would actually send. The ninth byte is the CRC over
    /// the other eight, so the result parses.
    ///
    /// # Returns
    ///
    /// The nine scratchpad bytes in transmission order, CRC last.
    pub fn to_bytes(&self) -> [u8; 9] {
        let temperature = self.raw_temperature.to_le_bytes();
        let mut bytes = [
            temperature[0],
            temperature[1],
            self.alarm_high as u8,
            self.alarm_low as u8,
            self.resolution.config_byte(),
            0xFF,
            0x0C,
            0x10,
            0x00,
        ];
        bytes[8] = crc8(&bytes[..8]);
        bytes
    }

    /// Returns the raw temperature register value.
    ///
    /// # Returns
    ///
    /// The 16-bit two's-complement register, as a signed value.
    pub fn raw_temperature(&self) -> i16 {
        self.raw_temperature
    }

    /// Returns the temperature in micro-degrees Celsius.
    ///
    /// # Returns
    ///
    /// The temperature, exact, in millionths of a degree Celsius.
    pub fn temperature_micro_celsius(&self) -> i32 {
        temperature_to_micro_celsius(self.raw_temperature)
    }

    /// Returns the temperature in degrees Celsius.
    ///
    /// # Returns
    ///
    /// The temperature in degrees Celsius.
    pub fn temperature_celsius(&self) -> f32 {
        temperature_to_celsius(self.raw_temperature)
    }

    /// Returns the configured conversion resolution.
    pub fn resolution(&self) -> Resolution {
        self.resolution
    }

    /// Returns the high temperature alarm threshold (TH), in whole degrees Celsius.
    pub fn alarm_high(&self) -> i8 {
        self.alarm_high
    }

    /// Returns the low temperature alarm threshold (TL), in whole degrees Celsius.
    pub fn alarm_low(&self) -> i8 {
        self.alarm_low
    }
}

/// Decodes the text the Linux kernel's `w1_therm` driver serves for a thermometer.
///
/// The `w1_slave` file under `/sys/bus/w1/devices/28-*/` holds two lines: the nine
/// scratchpad bytes in hex followed by `: crc=xx YES` or `NO`, then the same bytes
/// followed by `t=` and the temperature in millidegrees. This takes the first line's
/// bytes and parses them as a [`Scratchpad`], so the CRC is checked here as well; a
/// `NO` from the kernel is reported without a second look.
///
/// # Arguments
///
/// * `text` - the file's contents.
///
/// # Returns
///
/// The scratchpad.
///
/// # Errors
///
/// Returns [`SensorError::Crc`] if the kernel or this decoder rejects the CRC, and
/// [`SensorError::Invalid`] if the text is not in the driver's format.
///
/// # Examples
///
/// ```
/// use pamoja_sensors::ds18b20::{parse_w1_slave, temperature_from_celsius, Resolution, Scratchpad};
///
/// // What the kernel prints for a part at 20.8125 C, built from the same library
/// // that decodes it.
/// let raw = temperature_from_celsius(20.8125, Resolution::Bits12);
/// let bytes = Scratchpad::new(raw, Resolution::Bits12, 75, -10).to_bytes();
/// let hex: Vec<String> = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
/// let text = format!("{} : crc={:02x} YES\n", hex.join(" "), bytes[8]);
///
/// let scratchpad = parse_w1_slave(&text)?;
/// assert_eq!(scratchpad.temperature_micro_celsius(), 20_812_500);
/// # Ok::<(), pamoja_sensors::SensorError>(())
/// ```
pub fn parse_w1_slave(text: &str) -> Result<Scratchpad, SensorError> {
    let line = text.lines().next().ok_or(SensorError::Invalid)?;
    let (hex, verdict) = line.split_once(": crc=").ok_or(SensorError::Invalid)?;
    let verdict = verdict.trim();
    if verdict.ends_with("NO") {
        return Err(SensorError::Crc);
    }
    if !verdict.ends_with("YES") {
        return Err(SensorError::Invalid);
    }
    let mut bytes = [0u8; 9];
    let mut count = 0;
    for token in hex.split_ascii_whitespace() {
        if count == bytes.len() {
            return Err(SensorError::Invalid);
        }
        bytes[count] = u8::from_str_radix(token, 16).map_err(|_| SensorError::Invalid)?;
        count += 1;
    }
    if count != bytes.len() {
        return Err(SensorError::Invalid);
    }
    Scratchpad::parse(&bytes)
}

/// The kernel's own 1-Wire driver: thermometers as files under `/sys/bus/w1/devices`.
///
/// On a Raspberry Pi the `w1-gpio` overlay (`dtoverlay=w1-gpio` in `config.txt`)
/// puts the bus on a GPIO, and the kernel's `w1_therm` driver lists every DS18B20 it
/// finds as a directory named by its family code and serial, `28-000005e2fdc3`, with
/// a `w1_slave` file that runs a conversion and prints the scratchpad on every read.
/// A [`Thermometer`](linux::Thermometer) reads that file as a
/// [`Sensor`](pamoja_core::Sensor), which is the right way to reach a DS18B20 from a
/// Linux process, where the bit timing a [`Ds18b20`] needs cannot be held.
#[cfg(feature = "linux")]
pub mod linux {
    use std::path::{Path, PathBuf};
    use std::{fmt, fs, io};

    use pamoja_core::Sensor;

    use super::{parse_w1_slave, Scratchpad, FAMILY_CODE};
    use crate::SensorError;

    /// Where the kernel lists the devices on its 1-Wire buses.
    pub const DEVICES: &str = "/sys/bus/w1/devices";

    /// The name of the file that runs a conversion and prints the scratchpad.
    pub const W1_SLAVE: &str = "w1_slave";

    /// What can go wrong reading a thermometer through the kernel.
    #[derive(Debug)]
    pub enum ThermometerError {
        /// The file could not be read: the overlay is off, the device is gone, or the
        /// process lacks permission.
        Io(io::Error),
        /// The file was read but its scratchpad did not decode.
        Sensor(SensorError),
    }

    impl fmt::Display for ThermometerError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                ThermometerError::Io(error) => write!(f, "reading the w1_slave file: {error}"),
                ThermometerError::Sensor(error) => write!(f, "{error}"),
            }
        }
    }

    impl std::error::Error for ThermometerError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                ThermometerError::Io(error) => Some(error),
                ThermometerError::Sensor(error) => Some(error),
            }
        }
    }

    impl From<ThermometerError> for pamoja_core::Error {
        fn from(error: ThermometerError) -> Self {
            match error {
                ThermometerError::Io(io) => pamoja_core::Error::Io(io.to_string()),
                ThermometerError::Sensor(sensor) => pamoja_core::Error::Codec(sensor.to_string()),
            }
        }
    }

    /// A DS18B20 the kernel exposes as a `w1_slave` file.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pamoja_core::Sensor;
    /// use pamoja_sensors::ds18b20::linux::Thermometer;
    ///
    /// # async fn run() -> pamoja_core::Result<()> {
    /// let mut probe = Thermometer::new("000005e2fdc3");
    /// let scratchpad = probe.read().await?;
    /// println!("{:.4} C", scratchpad.temperature_celsius());
    /// # Ok(())
    /// # }
    /// ```
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Thermometer {
        path: PathBuf,
    }

    impl Thermometer {
        /// Names a thermometer by the serial in its directory name.
        ///
        /// # Arguments
        ///
        /// * `serial` - the twelve hex digits after `28-` in the device directory.
        ///
        /// # Returns
        ///
        /// The thermometer, reading `/sys/bus/w1/devices/28-<serial>/w1_slave`.
        pub fn new(serial: &str) -> Thermometer {
            let directory = format!("{FAMILY_CODE:02x}-{serial}");
            Thermometer {
                path: Path::new(DEVICES).join(directory).join(W1_SLAVE),
            }
        }

        /// Names a thermometer by the path of its `w1_slave` file.
        ///
        /// # Arguments
        ///
        /// * `path` - the file to read.
        ///
        /// # Returns
        ///
        /// The thermometer.
        pub fn at(path: impl Into<PathBuf>) -> Thermometer {
            Thermometer { path: path.into() }
        }

        /// Returns the path of the file the thermometer reads.
        pub fn path(&self) -> &Path {
            &self.path
        }

        /// Lists every DS18B20 the kernel has found.
        ///
        /// # Returns
        ///
        /// One thermometer per `28-*` directory under [`DEVICES`].
        ///
        /// # Errors
        ///
        /// Returns the I/O error if the directory cannot be listed, which usually
        /// means the 1-Wire overlay is not enabled.
        pub fn discover() -> io::Result<Vec<Thermometer>> {
            Thermometer::discover_in(Path::new(DEVICES))
        }

        /// Lists every DS18B20 directory under `devices`.
        ///
        /// # Arguments
        ///
        /// * `devices` - the directory the kernel lists its 1-Wire devices in.
        ///
        /// # Returns
        ///
        /// One thermometer per directory whose name starts with the DS18B20 family
        /// code, sorted by name.
        ///
        /// # Errors
        ///
        /// Returns the I/O error if the directory cannot be listed.
        pub fn discover_in(devices: &Path) -> io::Result<Vec<Thermometer>> {
            let prefix = format!("{FAMILY_CODE:02x}-");
            let mut found: Vec<Thermometer> = fs::read_dir(devices)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
                .map(|entry| Thermometer::at(entry.path().join(W1_SLAVE)))
                .collect();
            found.sort_by(|a, b| a.path.cmp(&b.path));
            Ok(found)
        }

        /// Reads the file, which makes the kernel run a conversion, and decodes it.
        ///
        /// # Returns
        ///
        /// The CRC-checked scratchpad.
        ///
        /// # Errors
        ///
        /// Returns [`ThermometerError::Io`] if the file cannot be read and
        /// [`ThermometerError::Sensor`] if it does not decode.
        pub fn read_scratchpad(&self) -> Result<Scratchpad, ThermometerError> {
            let text = fs::read_to_string(&self.path).map_err(ThermometerError::Io)?;
            parse_w1_slave(&text).map_err(ThermometerError::Sensor)
        }
    }

    impl Sensor for Thermometer {
        type Reading = Scratchpad;

        async fn read(&mut self) -> pamoja_core::Result<Scratchpad> {
            self.read_scratchpad().map_err(pamoja_core::Error::from)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::ds18b20::{temperature_from_celsius, Resolution};

        fn kernel_text(celsius: f32) -> String {
            let raw = temperature_from_celsius(celsius, Resolution::Bits12);
            let bytes = Scratchpad::new(raw, Resolution::Bits12, 75, -10).to_bytes();
            let hex: Vec<String> = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            let hex = hex.join(" ");
            format!(
                "{hex} : crc={:02x} YES\n{hex} t={}\n",
                bytes[8],
                (celsius * 1000.0) as i32
            )
        }

        #[test]
        fn a_thermometer_reads_and_decodes_its_file() {
            let dir = std::env::temp_dir().join(format!("pamoja-w1-{}", std::process::id()));
            let device = dir.join("28-000005e2fdc3");
            fs::create_dir_all(&device).unwrap();
            fs::write(device.join(W1_SLAVE), kernel_text(21.5)).unwrap();
            fs::create_dir_all(dir.join("w1_bus_master1")).unwrap();

            let found = Thermometer::discover_in(&dir).unwrap();
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].path(), device.join(W1_SLAVE));
            let scratchpad = found[0].read_scratchpad().unwrap();
            assert_eq!(scratchpad.temperature_celsius(), 21.5);
            assert_eq!(
                pamoja_hal::script::block_on(found[0].clone().read()).unwrap(),
                scratchpad
            );

            let missing = Thermometer::at(dir.join("28-none").join(W1_SLAVE));
            assert!(matches!(
                missing.read_scratchpad(),
                Err(ThermometerError::Io(_))
            ));
            fs::remove_dir_all(&dir).unwrap();
        }

        #[test]
        fn a_serial_names_the_kernel_path() {
            let probe = Thermometer::new("000005e2fdc3");
            assert_eq!(
                probe.path(),
                Path::new("/sys/bus/w1/devices/28-000005e2fdc3/w1_slave")
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temperature_table_matches_the_datasheet() {
        // The DS18B20 datasheet's temperature/data relationship table, in 1/16 °C.
        let table: &[(i16, i32)] = &[
            (0x07D0, 125_000_000),
            (0x0550, 85_000_000),
            (0x0191, 25_062_500),
            (0x00A2, 10_125_000),
            (0x0008, 500_000),
            (0x0000, 0),
            (i16::from_le_bytes([0xF8, 0xFF]), -500_000),
            (i16::from_le_bytes([0x5E, 0xFF]), -10_125_000),
            (i16::from_le_bytes([0x6F, 0xFE]), -25_062_500),
            (i16::from_le_bytes([0x90, 0xFC]), -55_000_000),
        ];
        for &(raw, micro) in table {
            assert_eq!(temperature_to_micro_celsius(raw), micro, "raw {raw:#06x}");
        }
    }

    #[test]
    fn crc8_matches_the_published_check_value() {
        // CRC-8/MAXIM-DOW check value for the ASCII string "123456789" is 0xA1.
        assert_eq!(crc8(b"123456789"), 0xA1);
        // An empty message leaves the zero-initialized register untouched.
        assert_eq!(crc8(&[]), 0x00);
    }

    #[test]
    fn crc_over_a_message_and_its_crc_is_zero() {
        let data = [0x28, 0xFF, 0x64, 0x1E, 0x0C, 0x00, 0x00, 0x00];
        let crc = crc8(&data);
        let mut with_crc = [0u8; 9];
        with_crc[..8].copy_from_slice(&data);
        with_crc[8] = crc;
        assert_eq!(crc8(&with_crc), 0x00);
    }

    #[test]
    fn a_scratchpad_round_trips_through_parse() {
        let mut bytes = [0x91, 0x01, 75, 0xF6, 0x7F, 0xFF, 0x00, 0x10, 0x00];
        bytes[8] = crc8(&bytes[..8]);
        let scratchpad = Scratchpad::parse(&bytes).expect("valid crc");
        assert_eq!(scratchpad.raw_temperature(), 0x0191);
        assert_eq!(scratchpad.temperature_micro_celsius(), 25_062_500);
        assert_eq!(scratchpad.resolution(), Resolution::Bits12);
        assert_eq!(scratchpad.alarm_high(), 75);
        assert_eq!(scratchpad.alarm_low(), -10);
    }

    #[test]
    fn a_corrupted_scratchpad_fails_the_crc() {
        let mut bytes = [0x91, 0x01, 75, 0xF6, 0x7F, 0xFF, 0x00, 0x10, 0x00];
        bytes[8] = crc8(&bytes[..8]);
        bytes[0] ^= 0x01; // flip a temperature bit after the CRC was computed
        assert_eq!(Scratchpad::parse(&bytes), Err(SensorError::Crc));
    }

    #[test]
    fn resolution_config_bytes_match_the_datasheet() {
        assert_eq!(Resolution::Bits9.config_byte(), 0x1F);
        assert_eq!(Resolution::Bits10.config_byte(), 0x3F);
        assert_eq!(Resolution::Bits11.config_byte(), 0x5F);
        assert_eq!(Resolution::Bits12.config_byte(), 0x7F);
        for resolution in [
            Resolution::Bits9,
            Resolution::Bits10,
            Resolution::Bits11,
            Resolution::Bits12,
        ] {
            assert_eq!(
                Resolution::from_config_byte(resolution.config_byte()),
                resolution
            );
        }
    }
    #[test]
    fn a_built_scratchpad_parses_back_to_what_it_was_built_from() {
        let built = Scratchpad::new(
            temperature_from_celsius(25.0625, Resolution::Bits12),
            Resolution::Bits12,
            75,
            -10,
        );
        let parsed = Scratchpad::parse(&built.to_bytes()).expect("a built scratchpad is valid");
        assert_eq!(parsed, built);
        assert_eq!(parsed.raw_temperature(), 0x0191);
        assert_eq!(parsed.temperature_micro_celsius(), 25_062_500);
        assert_eq!(parsed.alarm_high(), 75);
        assert_eq!(parsed.alarm_low(), -10);
    }

    #[test]
    fn a_built_scratchpad_carries_the_datasheet_temperature_bytes() {
        // The +25.0625 °C row of the datasheet's temperature/data table is 0x0191, sent
        // least-significant byte first, and byte 4 selects 12-bit resolution.
        let bytes = Scratchpad::new(0x0191, Resolution::Bits12, 75, -10).to_bytes();
        assert_eq!(bytes[0], 0x91);
        assert_eq!(bytes[1], 0x01);
        assert_eq!(bytes[4], 0x7F);
        assert_eq!(bytes[8], crc8(&bytes[..8]));
    }

    #[test]
    fn a_temperature_truncates_to_the_step_the_resolution_resolves() {
        // 12-bit resolves a sixteenth of a degree; the coarser settings read the bits
        // below their step as zero, so the same temperature lands on a lower count.
        assert_eq!(temperature_from_celsius(25.0625, Resolution::Bits12), 401);
        assert_eq!(temperature_from_celsius(25.0625, Resolution::Bits11), 400);
        assert_eq!(temperature_from_celsius(25.0625, Resolution::Bits10), 400);
        assert_eq!(temperature_from_celsius(25.0625, Resolution::Bits9), 400);
        assert_eq!(
            temperature_from_micro_celsius(-10_062_500, Resolution::Bits12),
            -161
        );
    }

    #[test]
    fn every_temperature_register_survives_a_round_trip() {
        for raw in [-880i16, -161, -1, 0, 1, 401, 1250] {
            let micro = temperature_to_micro_celsius(raw);
            assert_eq!(
                temperature_from_micro_celsius(micro, Resolution::Bits12),
                raw
            );
        }
    }
    #[test]
    fn the_kernel_text_decodes_and_its_verdict_is_honored() {
        let raw = temperature_from_celsius(20.8125, Resolution::Bits12);
        let bytes = Scratchpad::new(raw, Resolution::Bits12, 75, -10).to_bytes();
        let mut hex = alloc::string::String::new();
        for byte in bytes {
            hex.push_str(&alloc::format!("{byte:02x} "));
        }
        let text = alloc::format!("{hex}: crc={:02x} YES\n{hex}t=20812\n", bytes[8]);
        let scratchpad = parse_w1_slave(&text).unwrap();
        assert_eq!(scratchpad.temperature_micro_celsius(), 20_812_500);

        let rejected = alloc::format!("{hex}: crc=00 NO\n{hex}t=20812\n");
        assert_eq!(parse_w1_slave(&rejected), Err(SensorError::Crc));
        assert_eq!(parse_w1_slave(""), Err(SensorError::Invalid));
        assert_eq!(
            parse_w1_slave("4d 01 : crc=e8 YES\n"),
            Err(SensorError::Invalid)
        );
        assert_eq!(
            parse_w1_slave("zz 01 4b 46 7f ff 03 10 e8 : crc=e8 YES\n"),
            Err(SensorError::Invalid)
        );
    }
}
