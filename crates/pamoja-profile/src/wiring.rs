//! A wiring file: which part reads what a profile reads, which line its output drives, and
//! which link its readings go over, for one site.
//!
//! A profile is portable: it says what to measure and how to decide, and the same file
//! runs a brooder in one village and another. A wiring file is the rest, and it belongs to
//! one installation: the part on this board's bus, the relay on this board's header, the
//! broker on this farm's network. `pamoja-node` reads the two and runs the node, with no
//! program to write.

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

use crate::presentation::refuse;
use crate::{format, ControlSpec, Profile};

/// A part the stock runner reads, as a wiring file names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Part {
    /// Bosch BME280: temperature, relative humidity, and pressure, on I2C.
    Bme280,
    /// Bosch BMP280: temperature and pressure, on I2C.
    Bmp280,
    /// Sensirion SHT3x: temperature and relative humidity, on I2C.
    Sht3x,
    /// TI HDC1080: temperature and relative humidity, on I2C at 0x40.
    Hdc1080,
    /// TI TMP117: temperature, on I2C.
    Tmp117,
    /// Sensirion SCD4x: carbon dioxide, temperature, and relative humidity, on I2C at 0x62.
    Scd4x,
    /// TI OPT3001: illuminance, on I2C.
    Opt3001,
    /// TI INA219: bus voltage, on I2C.
    Ina219,
    /// TI INA226: bus voltage, on I2C.
    Ina226,
    /// Maxim DS18B20: temperature, through the Linux kernel's 1-Wire files.
    Ds18b20,
    /// A list of readings played back in turn, for trying a profile with nothing wired.
    Replay,
}

impl Part {
    /// Every part the runner reads, in the order the documentation lists them.
    pub const ALL: [Part; 11] = [
        Part::Bme280,
        Part::Bmp280,
        Part::Sht3x,
        Part::Hdc1080,
        Part::Tmp117,
        Part::Scd4x,
        Part::Opt3001,
        Part::Ina219,
        Part::Ina226,
        Part::Ds18b20,
        Part::Replay,
    ];

    /// Returns the part as a wiring file names it.
    ///
    /// # Returns
    ///
    /// The name, such as `"bme280"`.
    pub fn name(self) -> &'static str {
        match self {
            Part::Bme280 => "bme280",
            Part::Bmp280 => "bmp280",
            Part::Sht3x => "sht3x",
            Part::Hdc1080 => "hdc1080",
            Part::Tmp117 => "tmp117",
            Part::Scd4x => "scd4x",
            Part::Opt3001 => "opt3001",
            Part::Ina219 => "ina219",
            Part::Ina226 => "ina226",
            Part::Ds18b20 => "ds18b20",
            Part::Replay => "replay",
        }
    }

    /// Returns what the part measures and the unit each comes out in.
    ///
    /// # Returns
    ///
    /// Each quantity with its unit, as a profile's `reads` names them. A replay part
    /// reads whatever the profile reads, so it lists nothing.
    pub fn measures(self) -> &'static [(&'static str, &'static str)] {
        const TEMPERATURE: (&str, &str) = ("temperature", "celsius");
        const HUMIDITY: (&str, &str) = ("relative_humidity", "percent");
        const PRESSURE: (&str, &str) = ("pressure", "hectopascal");
        match self {
            Part::Bme280 => &[TEMPERATURE, HUMIDITY, PRESSURE],
            Part::Bmp280 => &[TEMPERATURE, PRESSURE],
            Part::Sht3x | Part::Hdc1080 => &[TEMPERATURE, HUMIDITY],
            Part::Tmp117 | Part::Ds18b20 => &[TEMPERATURE],
            Part::Scd4x => &[("co2", "ppm"), TEMPERATURE, HUMIDITY],
            Part::Opt3001 => &[("illuminance", "lux")],
            Part::Ina219 | Part::Ina226 => &[("voltage", "volt")],
            Part::Replay => &[],
        }
    }

    /// Returns the I2C address the part answers at unless the wiring names another.
    ///
    /// # Returns
    ///
    /// The address, or `None` for a part that is not on I2C.
    pub fn default_address(self) -> Option<u8> {
        match self {
            Part::Bme280 | Part::Bmp280 => Some(0x76),
            Part::Sht3x | Part::Opt3001 => Some(0x44),
            Part::Hdc1080 | Part::Ina219 | Part::Ina226 => Some(0x40),
            Part::Tmp117 => Some(0x48),
            Part::Scd4x => Some(0x62),
            Part::Ds18b20 | Part::Replay => None,
        }
    }

    /// Whether the part answers only at its one address.
    fn fixed_address(self) -> bool {
        matches!(self, Part::Hdc1080 | Part::Scd4x)
    }
}

/// The part that takes the readings, and where it is.
///
/// In a wiring file it is the `sensor` object:
///
/// ```json
/// { "part": "bme280", "bus": "/dev/i2c-1", "address": "0x76" }
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SensorWiring {
    /// The part.
    pub part: Part,
    /// The I2C bus the part is on, such as `/dev/i2c-1`, or `sim` for the part's simulated
    /// twin, which answers with one fixed measurement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bus: Option<String>,
    /// The part's I2C address, when it is not the part's usual one.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "address",
        serialize_with = "hex"
    )]
    pub address: Option<u8>,
    /// A DS18B20's 1-Wire serial, such as `28-0316a2795cff`, as the kernel names its folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serial: Option<String>,
    /// The readings a replay part plays back in turn, in the unit the profile reads.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub readings: Vec<f32>,
    /// Added to each reading after it is in the profile's unit, to correct a probe that
    /// reads high or low.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub offset: f32,
    /// Multiplied into each reading before the offset is added.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub scale: f32,
}

/// The output a profile drives, as a wiring file names it.
///
/// ```json
/// { "gpio": "/dev/gpiochip0", "line": 17, "active_low": true }
/// { "print": "heat lamp" }
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "OutputFile", into = "OutputFile")]
pub enum OutputWiring {
    /// A GPIO line, such as the input of a relay board.
    Gpio {
        /// The GPIO chip, such as `/dev/gpiochip0`.
        chip: String,
        /// The line on the chip, which on a Raspberry Pi is the GPIO number.
        line: u32,
        /// Whether the line is driven low to switch the output on, as most relay boards
        /// want.
        active_low: bool,
    },
    /// A line of text each time the output switches, for trying a profile with nothing
    /// wired.
    Print {
        /// What the output is called in the text, such as `heat lamp`.
        label: String,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OutputFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gpio: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active_low: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    print: Option<String>,
}

impl TryFrom<OutputFile> for OutputWiring {
    type Error = String;

    fn try_from(file: OutputFile) -> Result<Self, String> {
        match file {
            OutputFile {
                gpio: Some(chip),
                line: Some(line),
                active_low,
                print: None,
            } => Ok(OutputWiring::Gpio {
                chip,
                line,
                active_low: active_low.unwrap_or(false),
            }),
            OutputFile {
                gpio: Some(chip),
                line: None,
                ..
            } => Err(format!(
                "the output on `{chip}` needs a `line`, the GPIO number"
            )),
            OutputFile {
                gpio: None,
                line: None,
                active_low: None,
                print: Some(label),
            } => Ok(OutputWiring::Print { label }),
            OutputFile {
                gpio: Some(_),
                print: Some(_),
                ..
            } => Err("an output is one thing: a `gpio` line or `print`, not both".to_owned()),
            OutputFile { .. } => Err(
                "an output names a `gpio` chip with its `line`, or `print` with a label".to_owned(),
            ),
        }
    }
}

impl From<OutputWiring> for OutputFile {
    fn from(output: OutputWiring) -> Self {
        match output {
            OutputWiring::Gpio {
                chip,
                line,
                active_low,
            } => OutputFile {
                gpio: Some(chip),
                line: Some(line),
                active_low: Some(active_low),
                print: None,
            },
            OutputWiring::Print { label } => OutputFile {
                gpio: None,
                line: None,
                active_low: None,
                print: Some(label),
            },
        }
    }
}

/// A broker's TLS settings, as a wiring file names them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TlsWiring {
    /// The certificate authority to trust, as a PEM file; the system's own unless given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ca: Option<String>,
    /// The client certificate to present, as a PEM file, for a broker that asks for one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate: Option<String>,
    /// The client certificate's private key, as a PEM file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

/// The link the readings are published over, as a wiring file names it.
///
/// ```json
/// { "mqtt": "192.168.1.10", "port": 1883 }
/// { "print": true }
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "LinkFile", into = "LinkFile")]
pub enum LinkWiring {
    /// An MQTT broker.
    Mqtt {
        /// The broker's host name or address.
        host: String,
        /// The broker's port.
        port: u16,
        /// The client id; the site's name unless given.
        client_id: Option<String>,
        /// The username to sign in with.
        username: Option<String>,
        /// The password to sign in with.
        password: Option<String>,
        /// TLS to the broker, when it speaks it.
        tls: Option<TlsWiring>,
    },
    /// A line of text for each reading, for trying a profile with no broker.
    Print,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mqtt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    client_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tls: Option<TlsWiring>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    print: Option<bool>,
}

impl TryFrom<LinkFile> for LinkWiring {
    type Error = String;

    fn try_from(file: LinkFile) -> Result<Self, String> {
        match file {
            LinkFile {
                mqtt: Some(host),
                print: None,
                port,
                client_id,
                username,
                password,
                tls,
            } => Ok(LinkWiring::Mqtt {
                host,
                port: port.unwrap_or(if tls.is_some() { 8883 } else { 1883 }),
                client_id,
                username,
                password,
                tls,
            }),
            LinkFile {
                mqtt: None,
                print: Some(true),
                port: None,
                client_id: None,
                username: None,
                password: None,
                tls: None,
            } => Ok(LinkWiring::Print),
            LinkFile {
                mqtt: Some(_),
                print: Some(_),
                ..
            } => Err("a link is one thing: `mqtt` or `print`, not both".to_owned()),
            LinkFile {
                mqtt: None,
                print: Some(true),
                ..
            } => Err("`print` takes no broker settings".to_owned()),
            LinkFile { .. } => Err(
                "a link names an `mqtt` broker's host, or `print`: true to print each reading"
                    .to_owned(),
            ),
        }
    }
}

impl From<LinkWiring> for LinkFile {
    fn from(link: LinkWiring) -> Self {
        match link {
            LinkWiring::Mqtt {
                host,
                port,
                client_id,
                username,
                password,
                tls,
            } => LinkFile {
                mqtt: Some(host),
                port: Some(port),
                client_id,
                username,
                password,
                tls,
                print: None,
            },
            LinkWiring::Print => LinkFile {
                mqtt: None,
                port: None,
                client_id: None,
                username: None,
                password: None,
                tls: None,
                print: Some(true),
            },
        }
    }
}

/// The battery the node runs from, read as a voltage to set how often it samples.
///
/// A node with none samples at its profile's active cadence, as on mains power.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatteryWiring {
    /// The power monitor across the battery: `ina219` or `ina226`.
    pub part: Part,
    /// The I2C bus the monitor is on, or `sim`.
    pub bus: String,
    /// The monitor's I2C address, when it is not the part's usual one.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "address",
        serialize_with = "hex"
    )]
    pub address: Option<u8>,
    /// The voltage the battery reads when it is empty.
    pub empty_volts: f32,
    /// The voltage the battery reads when it is full.
    pub full_volts: f32,
}

impl BatteryWiring {
    /// Turns the battery's voltage into a state of charge.
    ///
    /// # Arguments
    ///
    /// * `volts` - the voltage the monitor read.
    ///
    /// # Returns
    ///
    /// The charge from 0 to 1, on a straight line from empty to full; a reading outside the
    /// two is held to the nearer end.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_profile::wiring::{BatteryWiring, Part};
    ///
    /// let battery = BatteryWiring {
    ///     part: Part::Ina219,
    ///     bus: "/dev/i2c-1".to_owned(),
    ///     address: None,
    ///     empty_volts: 3.3,
    ///     full_volts: 4.2,
    /// };
    /// assert!((battery.charge(3.75) - 0.5).abs() < 1e-6);
    /// assert_eq!(battery.charge(5.0), 1.0);
    /// ```
    pub fn charge(&self, volts: f32) -> f32 {
        ((volts - self.empty_volts) / (self.full_volts - self.empty_volts)).clamp(0.0, 1.0)
    }
}

/// One site's wiring: the part, the output, the link, and the battery a profile runs with.
///
/// ```json
/// {
///   "$schema": "https://pamoja.molex.cloud/schema/wiring-1.json",
///   "site": "coop-2",
///   "profile": "brooder-heater.json",
///   "sensor": { "part": "bme280", "bus": "/dev/i2c-1", "address": "0x76" },
///   "output": { "gpio": "/dev/gpiochip0", "line": 17, "active_low": true },
///   "link": { "mqtt": "192.168.1.10" }
/// }
/// ```
///
/// Like a profile it is refused, with the field it was probably meant to be, when it holds a
/// field its format does not have.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "WiringFile", into = "WiringFile")]
pub struct Wiring {
    /// The site's name, which the node's logs and its MQTT client id carry.
    pub site: String,
    /// The profile to run, as a path relative to the wiring file.
    pub profile: String,
    /// The part that takes the readings.
    pub sensor: SensorWiring,
    /// The output the profile drives, if it drives one.
    pub output: Option<OutputWiring>,
    /// The link the readings are published over.
    pub link: LinkWiring,
    /// The battery the node runs from, if it runs from one.
    pub battery: Option<BatteryWiring>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WiringFile {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    schema: Option<String>,
    site: String,
    profile: String,
    sensor: SensorWiring,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    output: Option<OutputWiring>,
    link: LinkWiring,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    battery: Option<BatteryWiring>,
}

impl TryFrom<WiringFile> for Wiring {
    type Error = String;

    fn try_from(file: WiringFile) -> Result<Self, String> {
        format::check_schema(file.schema.as_deref(), "wiring", Wiring::SCHEMA)?;
        Ok(Wiring {
            site: file.site,
            profile: file.profile,
            sensor: file.sensor,
            output: file.output,
            link: file.link,
            battery: file.battery,
        })
    }
}

impl From<Wiring> for WiringFile {
    fn from(wiring: Wiring) -> Self {
        WiringFile {
            schema: Some(Wiring::SCHEMA.to_owned()),
            site: wiring.site,
            profile: wiring.profile,
            sensor: wiring.sensor,
            output: wiring.output,
            link: wiring.link,
            battery: wiring.battery,
        }
    }
}

/// How a profile's reading is taken from a wired part: which of the part's quantities, and
/// how it is brought into the profile's unit.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    /// The quantity, as the profile names it.
    pub quantity: String,
    /// The unit the part reads it in.
    pub from: String,
    /// The unit the profile's numbers are in.
    pub to: String,
    /// The correction the wiring adds after the unit is right.
    pub offset: f32,
    /// The factor the wiring multiplies in before the offset.
    pub scale: f32,
}

impl Plan {
    /// Brings one of the part's readings into the profile's unit and applies the wiring's
    /// correction.
    ///
    /// # Arguments
    ///
    /// * `reading` - the reading in the part's unit.
    ///
    /// # Returns
    ///
    /// The reading the profile judges.
    pub fn apply(&self, reading: f32) -> f32 {
        let converted = convert(reading, &self.from, &self.to).unwrap_or(reading);
        converted * self.scale + self.offset
    }
}

impl Wiring {
    /// The address of the published JSON Schema for the wiring file format this build
    /// writes.
    pub const SCHEMA: &'static str = "https://pamoja.molex.cloud/schema/wiring-1.json";

    /// Checks the wiring for what a runner could not open.
    ///
    /// # Returns
    ///
    /// Nothing when every part of the wiring can be opened.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`](pamoja_core::Error::Codec) naming the first problem: an
    /// empty site or profile path; an I2C part with no bus, a DS18B20 with no serial, a
    /// replay with no readings, an address past 0x7f or on a part that answers at only one,
    /// or a calibration that is not a finite number; a GPIO output with no chip; an MQTT link
    /// with no host, port 0, or a password without a username; or a battery on a part that
    /// is not a power monitor, or whose full voltage is not above its empty one.
    pub fn check(&self) -> pamoja_core::Result<()> {
        if self.site.trim().is_empty() {
            return refuse("the wiring needs a `site` name".to_owned());
        }
        if self.profile.trim().is_empty() {
            return refuse("the wiring needs the `profile` it runs".to_owned());
        }
        let sensor = &self.sensor;
        let part = sensor.part.name();
        match sensor.part {
            Part::Replay if sensor.readings.is_empty() => {
                return refuse("a replay sensor needs `readings` to play back".to_owned())
            }
            Part::Replay => {}
            Part::Ds18b20 if sensor.serial.as_deref().is_none_or(str::is_empty) => {
                return refuse(format!(
                    "a {part} needs its 1-Wire `serial`, such as 28-0316a2795cff"
                ))
            }
            Part::Ds18b20 => {}
            _ if sensor.bus.as_deref().is_none_or(str::is_empty) => {
                return refuse(format!(
                    "a {part} needs the I2C `bus` it is on, such as /dev/i2c-1, or sim"
                ))
            }
            _ => {}
        }
        for address in [
            sensor.address,
            self.battery.as_ref().and_then(|battery| battery.address),
        ]
        .into_iter()
        .flatten()
        {
            if address > 0x7f {
                return refuse(format!(
                    "the address {address:#04x} is past 0x7f, the last of I2C's seven-bit addresses"
                ));
            }
        }
        if let (Some(address), Some(usual)) = (sensor.address, sensor.part.default_address()) {
            if sensor.part.fixed_address() && address != usual {
                return refuse(format!(
                    "a {part} answers only at {usual:#04x}, not {address:#04x}"
                ));
            }
        }
        if !(sensor.offset.is_finite() && sensor.scale.is_finite() && sensor.scale != 0.0) {
            return refuse(
                "a sensor's `offset` and `scale` are finite numbers, and the scale is not 0"
                    .to_owned(),
            );
        }
        if let Some(OutputWiring::Gpio { chip, .. }) = &self.output {
            if chip.trim().is_empty() {
                return refuse(
                    "a GPIO output needs its `gpio` chip, such as /dev/gpiochip0".to_owned(),
                );
            }
        }
        if let LinkWiring::Mqtt {
            host,
            port,
            username,
            password,
            ..
        } = &self.link
        {
            if host.trim().is_empty() {
                return refuse("an MQTT link needs the broker's host".to_owned());
            }
            if *port == 0 {
                return refuse("an MQTT link's port is 1 to 65535".to_owned());
            }
            if password.is_some() && username.is_none() {
                return refuse("an MQTT password goes with a username".to_owned());
            }
        }
        if let LinkWiring::Mqtt { tls: Some(tls), .. } = &self.link {
            if tls.certificate.is_some() != tls.key.is_some() {
                return refuse(
                    "a TLS client `certificate` and its `key` are given together".to_owned(),
                );
            }
        }
        if let Some(battery) = &self.battery {
            if !matches!(battery.part, Part::Ina219 | Part::Ina226) {
                return refuse(format!(
                    "a battery is read through a power monitor, an ina219 or an ina226, not a {}",
                    battery.part.name()
                ));
            }
            if battery.bus.trim().is_empty() {
                return refuse("a battery's monitor needs the I2C `bus` it is on".to_owned());
            }
            if !(battery.empty_volts.is_finite()
                && battery.full_volts.is_finite()
                && battery.empty_volts < battery.full_volts)
            {
                return refuse(format!(
                    "a battery's `full_volts` ({}) is above its `empty_volts` ({})",
                    battery.full_volts, battery.empty_volts
                ));
            }
        }
        Ok(())
    }

    /// Checks that this wiring can run a profile, and says how its reading is taken.
    ///
    /// # Arguments
    ///
    /// * `profile` - the profile to run.
    ///
    /// # Returns
    ///
    /// The plan for the reading: the part's quantity and how it reaches the profile's unit.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`](pamoja_core::Error::Codec) when the profile does not say
    /// what it reads, the part does not measure it, the part's unit does not convert to the
    /// profile's, the profile's control kind is one only a program of its own can decide,
    /// a profile that switches an output has none wired, or one that switches nothing has
    /// one wired that would never move.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_profile::wiring::Wiring;
    /// use pamoja_profile::Profile;
    ///
    /// let wiring = Wiring::from_json(r#"{
    ///     "site": "coop-2",
    ///     "profile": "brooder-heater.json",
    ///     "sensor": { "part": "bme280", "bus": "/dev/i2c-1" },
    ///     "output": { "gpio": "/dev/gpiochip0", "line": 17, "active_low": true },
    ///     "link": { "mqtt": "192.168.1.10" }
    /// }"#)?;
    /// let fridge = Profile::vaccine_fridge_monitor();
    /// let plan = wiring.fits(&fridge)?;
    /// assert_eq!((plan.quantity.as_str(), plan.from.as_str()), ("temperature", "celsius"));
    ///
    /// // A soil probe's profile cannot run on a thermometer.
    /// let refused = wiring.fits(&Profile::irrigation_node()).unwrap_err().to_string();
    /// assert!(refused.contains("a bme280 measures temperature, relative_humidity, or pressure"));
    /// # Ok::<(), pamoja_core::Error>(())
    /// ```
    pub fn fits(&self, profile: &Profile) -> pamoja_core::Result<Plan> {
        let name = &profile.name;
        let Some(reads) = &profile.reads else {
            return Err(pamoja_core::Error::Codec(format!(
                "the profile `{name}` does not say what it reads, so the runner cannot pick a reading from the part; give it a `reads`"
            )));
        };
        let part = self.sensor.part;
        let from = if part == Part::Replay {
            reads.unit.clone()
        } else {
            match part
                .measures()
                .iter()
                .find(|(quantity, _)| *quantity == reads.quantity)
            {
                Some((_, unit)) => (*unit).to_owned(),
                None => {
                    let offered: Vec<&str> = part
                        .measures()
                        .iter()
                        .map(|(quantity, _)| *quantity)
                        .collect();
                    return Err(pamoja_core::Error::Codec(format!(
                        "the profile `{name}` reads {}, and a {} measures {}",
                        reads.quantity,
                        part.name(),
                        words(&offered)
                    )));
                }
            }
        };
        if convert(0.0, &from, &reads.unit).is_none() {
            return Err(pamoja_core::Error::Codec(format!(
                "a {} reads {} in {from}, and the runner does not convert that to the profile's {}",
                part.name(),
                reads.quantity,
                reads.unit
            )));
        }
        if let ControlSpec::Custom { kind, .. } = &profile.control {
            return Err(pamoja_core::Error::Codec(format!(
                "the profile `{name}` names the control kind `{kind}`, which only a program of its own can decide; the runner runs the built-in kinds"
            )));
        }
        let drives = matches!(profile.control, ControlSpec::Setpoint { .. });
        match (drives, &self.output) {
            (true, None) => Err(pamoja_core::Error::Codec(format!(
                "the profile `{name}` switches an output, and the wiring has no `output` for it"
            ))),
            (false, Some(_)) => Err(pamoja_core::Error::Codec(format!(
                "the profile `{name}` switches nothing, so the wiring's `output` would never move; leave it out"
            ))),
            _ => Ok(Plan {
                quantity: reads.quantity.clone(),
                from,
                to: reads.unit.clone(),
                offset: self.sensor.offset,
                scale: self.sensor.scale,
            }),
        }
    }
}

#[cfg(feature = "json")]
impl Wiring {
    /// Loads a wiring file.
    ///
    /// # Arguments
    ///
    /// * `text` - the file's contents.
    ///
    /// # Returns
    ///
    /// The wiring.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`](pamoja_core::Error::Codec) if the text is not a wiring file,
    /// is written in a format this build does not read, carries a field the format does not
    /// have, with the one it was probably meant to be, or describes wiring
    /// [`check`](Wiring::check) refuses.
    pub fn from_json(text: &str) -> pamoja_core::Result<Self> {
        let wiring: Wiring = serde_json::from_str(text)
            .map_err(|error| pamoja_core::Error::Codec(format::explain(&error)))?;
        wiring.check()?;
        Ok(wiring)
    }

    /// Writes the wiring as its file.
    ///
    /// # Returns
    ///
    /// The pretty-printed JSON text, which names its format with `$schema`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Codec`](pamoja_core::Error::Codec) if the wiring cannot be
    /// serialized.
    pub fn to_json(&self) -> pamoja_core::Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|error| pamoja_core::Error::Codec(error.to_string()))
    }
}

/// Converts a reading between two units of the same quantity.
///
/// # Arguments
///
/// * `value` - the reading.
/// * `from` - the unit it is in, such as `celsius`.
/// * `to` - the unit wanted, such as `fahrenheit`.
///
/// # Returns
///
/// The reading in `to`, or `None` when the runner does not convert between the two. A unit
/// converts to itself, and temperature (`celsius`, `fahrenheit`, `kelvin`), pressure
/// (`pascal`, `hectopascal`, `millibar`, `kilopascal`, `bar`), and electrical units
/// (`volt` and `millivolt`) convert among their own kind.
///
/// # Examples
///
/// ```
/// use pamoja_profile::wiring::convert;
///
/// assert_eq!(convert(100.0, "celsius", "fahrenheit"), Some(212.0));
/// assert_eq!(convert(1013.25, "hectopascal", "kilopascal"), Some(101.325));
/// assert_eq!(convert(20.0, "celsius", "percent"), None);
/// ```
pub fn convert(value: f32, from: &str, to: &str) -> Option<f32> {
    if from == to {
        return Some(value);
    }
    let celsius = match from {
        "celsius" => Some(value),
        "fahrenheit" => Some((value - 32.0) * 5.0 / 9.0),
        "kelvin" => Some(value - 273.15),
        _ => None,
    };
    if let Some(celsius) = celsius {
        return match to {
            "celsius" => Some(celsius),
            "fahrenheit" => Some(celsius * 9.0 / 5.0 + 32.0),
            "kelvin" => Some(celsius + 273.15),
            _ => None,
        };
    }
    let pascal = |unit: &str| match unit {
        "pascal" => Some(1.0),
        "hectopascal" | "millibar" => Some(100.0),
        "kilopascal" => Some(1000.0),
        "bar" => Some(100_000.0),
        _ => None,
    };
    let volt = |unit: &str| match unit {
        "volt" => Some(1000.0),
        "millivolt" => Some(1.0),
        _ => None,
    };
    let (a, b) = match (pascal(from), pascal(to), volt(from), volt(to)) {
        (Some(a), Some(b), _, _) | (_, _, Some(a), Some(b)) => (a, b),
        _ => return None,
    };
    Some((f64::from(value) * a / b) as f32)
}

/// Lists names the way a sentence does: `a`, `a or b`, `a, b, or c`.
fn words(names: &[&str]) -> String {
    match names {
        [] => "nothing".to_owned(),
        [one] => (*one).to_owned(),
        [first, second] => format!("{first} or {second}"),
        [rest @ .., last] => format!("{}, or {last}", rest.join(", ")),
    }
}

fn address<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<u8>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Written {
        Number(u8),
        Text(String),
    }
    match Option::<Written>::deserialize(deserializer)? {
        None => Ok(None),
        Some(Written::Number(address)) => Ok(Some(address)),
        Some(Written::Text(text)) => {
            let digits = text
                .strip_prefix("0x")
                .or_else(|| text.strip_prefix("0X"))
                .ok_or_else(|| {
                    de::Error::custom(format!(
                        "the address `{text}` is a number, or hexadecimal such as \"0x76\""
                    ))
                })?;
            u8::from_str_radix(digits, 16).map(Some).map_err(|_| {
                de::Error::custom(format!(
                    "the address `{text}` is not one byte of hexadecimal"
                ))
            })
        }
    }
}

fn hex<S: serde::Serializer>(address: &Option<u8>, serializer: S) -> Result<S::Ok, S::Error> {
    match address {
        Some(address) => serializer.serialize_str(&format!("{address:#04x}")),
        None => serializer.serialize_none(),
    }
}

fn one() -> f32 {
    1.0
}

fn is_one(value: &f32) -> bool {
    *value == 1.0
}

fn is_zero(value: &f32) -> bool {
    *value == 0.0
}

#[cfg(all(test, feature = "json"))]
mod tests {
    use super::*;

    const COOP: &str = r#"{
  "$schema": "https://pamoja.molex.cloud/schema/wiring-1.json",
  "site": "coop-2",
  "profile": "brooder-heater.json",
  "sensor": { "part": "bme280", "bus": "/dev/i2c-1", "address": "0x77", "offset": -0.4 },
  "output": { "gpio": "/dev/gpiochip0", "line": 17, "active_low": true },
  "link": { "mqtt": "192.168.1.10", "username": "coop", "password": "hunter2" },
  "battery": { "part": "ina219", "bus": "/dev/i2c-1", "empty_volts": 3.3, "full_volts": 4.2 }
}"#;

    fn refused(text: &str) -> String {
        match Wiring::from_json(text) {
            Err(pamoja_core::Error::Codec(reason)) => reason,
            other => panic!("expected the wiring to be refused, got {other:?}"),
        }
    }

    #[test]
    fn a_wiring_file_loads_and_writes_back_the_same() {
        let wiring = Wiring::from_json(COOP).expect("a usable wiring");
        assert_eq!(wiring.sensor.address, Some(0x77));
        assert_eq!(wiring.sensor.offset, -0.4);
        assert_eq!(
            wiring.output,
            Some(OutputWiring::Gpio {
                chip: "/dev/gpiochip0".to_owned(),
                line: 17,
                active_low: true
            })
        );
        assert!(matches!(wiring.link, LinkWiring::Mqtt { port: 1883, .. }));
        let written = wiring.to_json().unwrap();
        assert!(written.contains("\"address\": \"0x77\""), "{written}");
        assert!(written
            .starts_with("{\n  \"$schema\": \"https://pamoja.molex.cloud/schema/wiring-1.json\""));
        assert_eq!(Wiring::from_json(&written).unwrap(), wiring);

        let decimal = COOP.replace("\"0x77\"", "119");
        assert_eq!(
            Wiring::from_json(&decimal).unwrap().sensor.address,
            Some(0x77)
        );
        let tls = COOP.replace(
            "\"password\": \"hunter2\"",
            "\"password\": \"hunter2\", \"tls\": {}",
        );
        assert!(matches!(
            Wiring::from_json(&tls).unwrap().link,
            LinkWiring::Mqtt { port: 8883, .. }
        ));
    }

    #[test]
    fn a_wiring_file_is_refused_with_the_reason() {
        assert!(refused(&COOP.replace("\"site\"", "\"stie\""))
            .starts_with("unknown field `stie`, did you mean `site`?"));
        assert!(refused(&COOP.replace("\"0x77\"", "\"77\"")).contains("hexadecimal such as"));
        assert!(refused(&COOP.replace("\"bme280\"", "\"bme28O\""))
            .starts_with("unknown variant `bme28O`, did you mean `bme280`?"));
        assert!(refused(&COOP.replace(", \"line\": 17", "")).contains("needs a `line`"));
        assert!(
            refused(&COOP.replace("\"username\": \"coop\", ", "")).contains("goes with a username")
        );
        assert!(
            refused(&COOP.replace("\"bus\": \"/dev/i2c-1\", \"address\"", "\"address\""))
                .contains("needs the I2C `bus`")
        );
        assert!(
            refused(&COOP.replace("\"part\": \"ina219\"", "\"part\": \"tmp117\""))
                .contains("through a power monitor")
        );
        assert!(refused(&COOP.replace("4.2", "3.0")).contains("is above its `empty_volts`"));
        assert!(
            refused(&COOP.replace("wiring-1.json", "wiring-2.json")).contains("wiring format 2")
        );
        let fixed = COOP
            .replace("\"bme280\"", "\"scd4x\"")
            .replace("\"0x77\"", "\"0x61\"");
        assert!(refused(&fixed).contains("answers only at 0x62"));
        let both = COOP.replace(
            "\"active_low\": true",
            "\"active_low\": true, \"print\": \"lamp\"",
        );
        assert!(refused(&both).contains("not both"));
    }

    #[test]
    fn a_wiring_fits_a_profile_that_reads_what_its_part_measures() {
        let wiring = Wiring::from_json(COOP).unwrap();
        let brooder = Profile::vaccine_fridge_monitor();
        let plan = wiring
            .fits(&brooder)
            .expect("a thermometer runs a temperature profile");
        assert_eq!(plan.apply(20.44), 20.44 - 0.4);

        let fahrenheit = brooder.clone().with_reads("temperature", "fahrenheit");
        let plan = wiring.fits(&fahrenheit).unwrap();
        assert!((plan.apply(100.0) - (212.0 - 0.4)).abs() < 1e-3);

        let reason = |profile: &Profile| wiring.fits(profile).unwrap_err().to_string();
        assert!(reason(&Profile::irrigation_node())
            .contains("reads soil_moisture, and a bme280 measures temperature, relative_humidity, or pressure"));
        assert!(
            reason(&brooder.clone().with_reads("temperature", "percent"))
                .contains("does not convert")
        );
        let mut unread = brooder.clone();
        unread.reads = None;
        assert!(reason(&unread).contains("does not say what it reads"));
        let mut custom = brooder.clone();
        custom.control = ControlSpec::custom("brooder_guard", crate::Params::new()).unwrap();
        assert!(reason(&custom).contains("only a program of its own can decide"));
        let mut monitor = brooder.clone();
        monitor.control = ControlSpec::Monitor;
        assert!(reason(&monitor).contains("would never move"));
        let mut unwired = wiring.clone();
        unwired.output = None;
        assert!(unwired
            .fits(&brooder)
            .unwrap_err()
            .to_string()
            .contains("no `output` for it"));
    }

    #[test]
    fn units_convert_among_their_own_kind() {
        assert_eq!(convert(0.0, "celsius", "kelvin"), Some(273.15));
        assert_eq!(convert(212.0, "fahrenheit", "celsius"), Some(100.0));
        assert_eq!(convert(1.0, "bar", "hectopascal"), Some(1000.0));
        assert_eq!(convert(3.3, "volt", "millivolt"), Some(3300.0));
        assert_eq!(convert(1.0, "volt", "celsius"), None);
        assert_eq!(convert(7.0, "ntu", "ntu"), Some(7.0));
        assert_eq!(words(&["a", "b", "c"]), "a, b, or c");
    }
}
