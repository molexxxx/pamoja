//! What a gateway daemon is told to do.
//!
//! A gateway needs a handful of facts that no library can work out for itself: which SPI
//! device the concentrator answers on, which line resets it, what carrier the radios are
//! tuned to, which channels to listen on, and where uplinks are sent. This reads them out of
//! a JSON file and refuses anything it cannot use, naming the field rather than the file.
//!
//! The firmware images are named here rather than carried in the crate. They belong to
//! Semtech and are distributed with the reference implementation, so a gateway points at the
//! copies it already has instead of pamoja shipping them.
//!
//! Nothing here opens a file, a bus or a socket: this turns text into a configuration, so it
//! can be checked without any of them.

use serde_json::{Map, Value};

use pamoja_radios::sx1302::tx::{Chain, FrontEnd};

use crate::udp::Eui;

/// How many characters a gateway identifier is written with.
const EUI_CHARS: usize = 16;

/// How many receivers a concentrator listens with.
const CHANNELS: usize = 8;

/// The lowest spreading factor a receiver can be asked for.
const MIN_SPREADING_FACTOR: u8 = 5;

/// The highest.
const MAX_SPREADING_FACTOR: u8 = 12;

/// The port a packet forwarder speaks on when a configuration does not say.
pub const DEFAULT_FORWARDER_PORT: u16 = 1700;

/// Why a configuration was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// The text is not a JSON object.
    NotAnObject(String),
    /// A field the daemon cannot run without is absent.
    Missing {
        /// Which field, written the way it appears in the file.
        field: String,
    },
    /// A field carries something the daemon cannot use.
    Refused {
        /// Which field.
        field: String,
        /// What is wrong with it.
        why: String,
    },
}

impl core::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ConfigError::NotAnObject(what) => {
                write!(f, "the configuration is not a JSON object: {what}")
            }
            ConfigError::Missing { field } => write!(f, "the configuration needs {field}"),
            ConfigError::Refused { field, why } => write!(f, "{field}: {why}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Where the concentrator is wired.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Concentrator {
    /// The SPI device it answers on.
    pub spi: String,
    /// The GPIO character device its reset line is on.
    pub gpio_chip: String,
    /// The line its reset pin is wired to.
    pub reset_line: u32,
    /// Whether the board wires its front ends single ended rather than differential.
    pub single_input: bool,
    /// Which front end the board carries.
    pub front_end: FrontEnd,
    /// Which front end the concentrator takes its clock from.
    pub clock: Chain,
    /// Whether the radio beside the concentrator checks a channel before the gateway talks.
    pub listen_before_talk: bool,
    /// The gain control microcontroller image.
    pub gain_control_firmware: String,
    /// The arbiter microcontroller image.
    pub arbiter_firmware: String,
}

/// What the radios are tuned to and what the receivers listen for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Radio {
    /// The carrier, in hertz, that every channel offset is measured from.
    pub carrier_hz: u32,
    /// How far each receiver listens from it, in hertz, which is signed.
    pub channels: Vec<i32>,
    /// Which spreading factors to look for.
    pub spreading_factors: Vec<u8>,
    /// Which spreading factors are demodulated twice over, one bit each counting from SF5.
    ///
    /// Demodulating twice buys a finer timestamp and costs capacity, so a gateway that does
    /// not need one leaves this at zero.
    pub dual_demodulation: u8,
}

/// Where a gateway sends what it hears.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Upstream {
    /// The Semtech packet forwarder protocol, over UDP.
    Forwarder {
        /// The network server to send to.
        host: String,
        /// The port it listens on.
        port: u16,
    },
    /// The Basics Station protocol, over a websocket.
    Station {
        /// The discovery endpoint to ask.
        endpoint: String,
    },
}

/// Everything a gateway daemon is told.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// The identifier this gateway reports itself by.
    pub gateway: Eui,
    /// Where the concentrator is wired.
    pub concentrator: Concentrator,
    /// What it listens for.
    pub radio: Radio,
    /// Where uplinks are sent.
    pub upstream: Upstream,
}

impl Config {
    /// Reads a configuration.
    ///
    /// # Arguments
    ///
    /// * `text` - the JSON a gateway was configured with.
    ///
    /// # Returns
    ///
    /// The configuration.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] naming the field that is missing or unusable, so an operator
    /// is told what to fix rather than that the file is wrong.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_gateway::daemon::config::{Config, Upstream};
    ///
    /// let text = r#"{
    ///   "gateway": "b827ebfffe010203",
    ///   "concentrator": {
    ///     "spi": "/dev/spidev0.0",
    ///     "gpio_chip": "/dev/gpiochip0",
    ///     "reset_line": 23,
    ///     "firmware": {
    ///       "gain_control": "/usr/share/pamoja/agc.bin",
    ///       "arbiter": "/usr/share/pamoja/arb.bin"
    ///     }
    ///   },
    ///   "radio": {
    ///     "carrier_hz": 867500000,
    ///     "channels": [-700000, -500000, -300000, -100000, 100000, 300000, 500000, 700000]
    ///   },
    ///   "upstream": { "forwarder": "router.example.net" }
    /// }"#;
    ///
    /// let config = Config::parse(text).expect("the configuration is complete");
    /// assert_eq!(config.radio.channels.len(), 8);
    /// assert!(matches!(config.upstream, Upstream::Forwarder { .. }));
    /// ```
    pub fn parse(text: &str) -> Result<Config, ConfigError> {
        let value: Value = serde_json::from_str(text)
            .map_err(|error| ConfigError::NotAnObject(error.to_string()))?;
        let object = object(&value, "the configuration")?;

        Ok(Config {
            gateway: eui(object)?,
            concentrator: concentrator(object)?,
            radio: radio(object)?,
            upstream: upstream(object)?,
        })
    }
}

/// Reads the gateway identifier.
fn eui(object: &Map<String, Value>) -> Result<Eui, ConfigError> {
    let text = required_text(object, "gateway")?;
    let trimmed: String = text.chars().filter(|c| *c != ':' && *c != '-').collect();
    if trimmed.len() != EUI_CHARS {
        return Err(refused(
            "gateway",
            &format!(
                "an identifier is {EUI_CHARS} hexadecimal characters, not {}",
                trimmed.len()
            ),
        ));
    }

    Eui::from_hex(&trimmed).ok_or_else(|| refused("gateway", &format!("{text} is not hexadecimal")))
}

/// Reads where the concentrator is wired.
fn concentrator(object: &Map<String, Value>) -> Result<Concentrator, ConfigError> {
    let held = required_object(object, "concentrator")?;
    let firmware = required_object(held, "concentrator.firmware")?;

    Ok(Concentrator {
        spi: required_text(held, "concentrator.spi")?.to_owned(),
        gpio_chip: required_text(held, "concentrator.gpio_chip")?.to_owned(),
        reset_line: required_whole(held, "concentrator.reset_line")?,
        single_input: held
            .get("single_input")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        front_end: front_end(held)?,
        clock: clock(held)?,
        listen_before_talk: held
            .get("listen_before_talk")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        gain_control_firmware: required_text(firmware, "concentrator.firmware.gain_control")?
            .to_owned(),
        arbiter_firmware: required_text(firmware, "concentrator.firmware.arbiter")?.to_owned(),
    })
}

/// Reads what the radios are tuned to.
fn radio(object: &Map<String, Value>) -> Result<Radio, ConfigError> {
    let held = required_object(object, "radio")?;
    let carrier_hz = required_whole(held, "radio.carrier_hz")?;
    if carrier_hz == 0 {
        return Err(refused(
            "radio.carrier_hz",
            "a carrier of zero tunes nothing",
        ));
    }

    let listed = held
        .get("channels")
        .and_then(Value::as_array)
        .ok_or_else(|| missing("radio.channels"))?;
    if listed.is_empty() || listed.len() > CHANNELS {
        return Err(refused(
            "radio.channels",
            &format!(
                "a concentrator listens on 1 to {CHANNELS} channels, not {}",
                listed.len()
            ),
        ));
    }

    let mut channels = Vec::with_capacity(listed.len());
    for offset in listed {
        let offset = offset.as_i64().ok_or_else(|| {
            refused(
                "radio.channels",
                "a channel offset is a whole number of hertz",
            )
        })?;
        channels.push(i32::try_from(offset).map_err(|_| {
            refused(
                "radio.channels",
                &format!("{offset} Hz is not a channel offset"),
            )
        })?);
    }

    let spreading_factors = match held.get("spreading_factors") {
        None => (MIN_SPREADING_FACTOR..=MAX_SPREADING_FACTOR).collect(),
        Some(value) => {
            let listed = value.as_array().ok_or_else(|| {
                refused(
                    "radio.spreading_factors",
                    "the spreading factors are a list",
                )
            })?;
            let mut wanted = Vec::with_capacity(listed.len());
            for entry in listed {
                let factor = entry.as_u64().ok_or_else(|| {
                    refused("radio.spreading_factors", "a spreading factor is a number")
                })?;
                let factor = u8::try_from(factor).unwrap_or(u8::MAX);
                if !(MIN_SPREADING_FACTOR..=MAX_SPREADING_FACTOR).contains(&factor) {
                    return Err(refused(
                        "radio.spreading_factors",
                        &format!(
                            "{factor} is outside {MIN_SPREADING_FACTOR} to {MAX_SPREADING_FACTOR}"
                        ),
                    ));
                }
                wanted.push(factor);
            }
            if wanted.is_empty() {
                return Err(refused(
                    "radio.spreading_factors",
                    "a receiver looking for none hears nothing",
                ));
            }
            wanted
        }
    };

    let dual_demodulation = match held.get("dual_demodulation") {
        None => 0,
        Some(value) => {
            let mask = value
                .as_u64()
                .ok_or_else(|| refused("radio.dual_demodulation", "this is a mask of bits"))?;
            u8::try_from(mask).map_err(|_| {
                refused(
                    "radio.dual_demodulation",
                    &format!("{mask} is wider than the eight spreading factors"),
                )
            })?
        }
    };

    Ok(Radio {
        carrier_hz,
        channels,
        spreading_factors,
        dual_demodulation,
    })
}

/// Reads which front end the board carries.
fn front_end(held: &Map<String, Value>) -> Result<FrontEnd, ConfigError> {
    match text(held, "front_end") {
        None => Ok(FrontEnd::Sx1250),
        Some("sx1250") => Ok(FrontEnd::Sx1250),
        Some("sx1255") | Some("sx1257") | Some("sx125x") => Ok(FrontEnd::Sx125x),
        Some(named) => Err(refused(
            "concentrator.front_end",
            &format!("{named} is not a front end this crate drives"),
        )),
    }
}

/// Reads which front end carries the concentrator clock.
fn clock(held: &Map<String, Value>) -> Result<Chain, ConfigError> {
    match text(held, "clock") {
        None | Some("a") | Some("A") => Ok(Chain::A),
        Some("b") | Some("B") => Ok(Chain::B),
        Some(named) => Err(refused(
            "concentrator.clock",
            &format!("the clock comes from chain a or b, not {named}"),
        )),
    }
}

/// Reads a string field.
fn text<'a>(held: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    held.get(key).and_then(Value::as_str)
}

/// Reads where uplinks are sent.
fn upstream(object: &Map<String, Value>) -> Result<Upstream, ConfigError> {
    let held = required_object(object, "upstream")?;

    if let Some(value) = held.get("forwarder") {
        let host = value
            .as_str()
            .ok_or_else(|| refused("upstream.forwarder", "a forwarder is named by its host"))?;
        let port = match held.get("port") {
            None => DEFAULT_FORWARDER_PORT,
            Some(port) => {
                let port = port
                    .as_u64()
                    .ok_or_else(|| refused("upstream.port", "a port is a number"))?;
                u16::try_from(port)
                    .map_err(|_| refused("upstream.port", &format!("{port} is not a port")))?
            }
        };
        return Ok(Upstream::Forwarder {
            host: host.to_owned(),
            port,
        });
    }

    if let Some(value) = held.get("station") {
        let endpoint = value
            .as_str()
            .ok_or_else(|| refused("upstream.station", "a station is named by its endpoint"))?;
        return Ok(Upstream::Station {
            endpoint: endpoint.to_owned(),
        });
    }

    Err(refused(
        "upstream",
        "name either a forwarder or a station to send to",
    ))
}

/// Reads a value as an object.
fn object<'a>(value: &'a Value, what: &str) -> Result<&'a Map<String, Value>, ConfigError> {
    value
        .as_object()
        .ok_or_else(|| ConfigError::NotAnObject(what.to_owned()))
}

/// Reads a field that has to be an object.
fn required_object<'a>(
    held: &'a Map<String, Value>,
    field: &str,
) -> Result<&'a Map<String, Value>, ConfigError> {
    let name = field.rsplit('.').next().unwrap_or(field);
    held.get(name)
        .ok_or_else(|| missing(field))?
        .as_object()
        .ok_or_else(|| refused(field, "this is a group of settings"))
}

/// Reads a field that has to be text.
fn required_text<'a>(held: &'a Map<String, Value>, field: &str) -> Result<&'a str, ConfigError> {
    let name = field.rsplit('.').next().unwrap_or(field);
    held.get(name)
        .ok_or_else(|| missing(field))?
        .as_str()
        .ok_or_else(|| refused(field, "this is written as text"))
}

/// Reads a field that has to be a whole number.
fn required_whole(held: &Map<String, Value>, field: &str) -> Result<u32, ConfigError> {
    let name = field.rsplit('.').next().unwrap_or(field);
    let value = held
        .get(name)
        .ok_or_else(|| missing(field))?
        .as_u64()
        .ok_or_else(|| refused(field, "this is a whole number"))?;
    u32::try_from(value).map_err(|_| refused(field, &format!("{value} is too large")))
}

/// Builds the error an absent field reports.
fn missing(field: &str) -> ConfigError {
    ConfigError::Missing {
        field: field.to_owned(),
    }
}

/// Builds the error an unusable field reports.
fn refused(field: &str, why: &str) -> ConfigError {
    ConfigError::Refused {
        field: field.to_owned(),
        why: why.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A configuration with everything a gateway needs.
    fn complete() -> String {
        r#"{
          "gateway": "b8:27:eb:ff:fe:01:02:03",
          "concentrator": {
            "spi": "/dev/spidev0.0",
            "gpio_chip": "/dev/gpiochip0",
            "reset_line": 23,
            "single_input": true,
            "firmware": {
              "gain_control": "/usr/share/pamoja/agc.bin",
              "arbiter": "/usr/share/pamoja/arb.bin"
            }
          },
          "radio": {
            "carrier_hz": 867500000,
            "channels": [-400000, -200000, 0, 200000],
            "spreading_factors": [7, 8, 9]
          },
          "upstream": { "forwarder": "router.example.net", "port": 1700 }
        }"#
        .to_owned()
    }

    #[test]
    fn a_board_takes_the_usual_answers_when_it_says_nothing() {
        // An ordinary gateway names none of these, so the defaults have to be the ordinary
        // board: an SX1250 pair, clocked from the first chain, with neither listen before
        // talk nor double demodulation.
        let config = Config::parse(&complete()).expect("every field is there");

        assert_eq!(config.concentrator.front_end, FrontEnd::Sx1250);
        assert_eq!(config.concentrator.clock, Chain::A);
        assert!(!config.concentrator.listen_before_talk);
        assert_eq!(config.radio.dual_demodulation, 0);
    }

    #[test]
    fn a_board_clocked_from_the_second_chain_says_so() {
        let named = complete().replace(
            "\"reset_line\": 23,",
            "\"reset_line\": 23, \"clock\": \"b\", \"front_end\": \"sx1257\",",
        );
        let config = Config::parse(&named).expect("both are named");

        assert_eq!(config.concentrator.clock, Chain::B);
        assert_eq!(config.concentrator.front_end, FrontEnd::Sx125x);
    }

    #[test]
    fn a_front_end_this_crate_does_not_drive_is_refused() {
        let odd = complete().replace(
            "\"reset_line\": 23,",
            "\"reset_line\": 23, \"front_end\": \"sx1280\",",
        );
        match Config::parse(&odd) {
            Err(ConfigError::Refused { field, why }) => {
                assert_eq!(field, "concentrator.front_end");
                assert!(why.contains("sx1280"), "{why}");
            }
            other => panic!("an unknown front end is refused: {other:?}"),
        }
    }

    #[test]
    fn a_complete_configuration_reads_back() {
        let config = Config::parse(&complete()).expect("every field is there");

        assert_eq!(config.concentrator.spi, "/dev/spidev0.0");
        assert_eq!(config.concentrator.reset_line, 23);
        assert!(config.concentrator.single_input);
        assert_eq!(config.radio.carrier_hz, 867_500_000);
        assert_eq!(config.radio.channels, [-400_000, -200_000, 0, 200_000]);
        assert_eq!(config.radio.spreading_factors, [7, 8, 9]);
        assert_eq!(
            config.upstream,
            Upstream::Forwarder {
                host: "router.example.net".to_owned(),
                port: 1700,
            }
        );
    }

    #[test]
    fn an_identifier_is_read_with_or_without_separators() {
        let plain = complete().replace("b8:27:eb:ff:fe:01:02:03", "b827ebfffe010203");
        let with = Config::parse(&complete()).expect("separators are allowed");
        let without = Config::parse(&plain).expect("and so is leaving them out");
        assert_eq!(with.gateway, without.gateway);
    }

    #[test]
    fn a_missing_field_names_itself() {
        let without = complete().replace("\"reset_line\": 23,", "");
        match Config::parse(&without) {
            Err(ConfigError::Missing { field }) => {
                assert_eq!(field, "concentrator.reset_line");
            }
            other => panic!("the field that is missing is named: {other:?}"),
        }
    }

    #[test]
    fn a_field_that_cannot_be_used_says_why() {
        let zero = complete().replace("867500000", "0");
        match Config::parse(&zero) {
            Err(ConfigError::Refused { field, why }) => {
                assert_eq!(field, "radio.carrier_hz");
                assert!(why.contains("zero"), "{why}");
            }
            other => panic!("a carrier of zero is refused: {other:?}"),
        }
    }

    #[test]
    fn more_channels_than_there_are_receivers_is_refused() {
        let many = complete().replace(
            "[-400000, -200000, 0, 200000]",
            "[1, 2, 3, 4, 5, 6, 7, 8, 9]",
        );
        match Config::parse(&many) {
            Err(ConfigError::Refused { field, .. }) => assert_eq!(field, "radio.channels"),
            other => panic!("nine channels are refused: {other:?}"),
        }
    }

    #[test]
    fn the_spreading_factors_default_to_all_of_them() {
        let without = complete().replace("\"spreading_factors\": [7, 8, 9]", "\"chatter\": 0");
        let config = Config::parse(&without).expect("they are optional");
        assert_eq!(config.radio.spreading_factors, [5, 6, 7, 8, 9, 10, 11, 12]);
    }

    #[test]
    fn a_spreading_factor_the_chip_does_not_have_is_refused() {
        let odd = complete().replace("[7, 8, 9]", "[7, 13]");
        match Config::parse(&odd) {
            Err(ConfigError::Refused { field, why }) => {
                assert_eq!(field, "radio.spreading_factors");
                assert!(why.contains("13"), "{why}");
            }
            other => panic!("SF13 does not exist: {other:?}"),
        }
    }

    #[test]
    fn a_forwarder_takes_the_usual_port_when_none_is_given() {
        let without = complete().replace(", \"port\": 1700", "");
        let config = Config::parse(&without).expect("the port is optional");
        assert_eq!(
            config.upstream,
            Upstream::Forwarder {
                host: "router.example.net".to_owned(),
                port: DEFAULT_FORWARDER_PORT,
            }
        );
    }

    #[test]
    fn a_station_endpoint_is_read_too() {
        let station = complete().replace(
            "{ \"forwarder\": \"router.example.net\", \"port\": 1700 }",
            "{ \"station\": \"wss://router.example.net/router-info\" }",
        );
        let config = Config::parse(&station).expect("a station is an upstream");
        assert_eq!(
            config.upstream,
            Upstream::Station {
                endpoint: "wss://router.example.net/router-info".to_owned(),
            }
        );
    }

    #[test]
    fn an_upstream_that_names_neither_is_refused() {
        let neither = complete().replace(
            "{ \"forwarder\": \"router.example.net\", \"port\": 1700 }",
            "{ \"somewhere\": \"else\" }",
        );
        match Config::parse(&neither) {
            Err(ConfigError::Refused { field, .. }) => assert_eq!(field, "upstream"),
            other => panic!("an upstream has to be one or the other: {other:?}"),
        }
    }

    #[test]
    fn something_that_is_not_json_says_so() {
        assert!(matches!(
            Config::parse("not json at all"),
            Err(ConfigError::NotAnObject(_))
        ));
    }
}
