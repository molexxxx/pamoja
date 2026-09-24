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

use pamoja_radios::sx1302::lbt::{self, ScanTime};
use pamoja_radios::sx1302::sx1261::Bandwidth;
use pamoja_radios::sx1302::tx::{Chain, FrontEnd, Gain, DEFAULT_GAINS};

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
pub const DEFAULT_FORWARDER_PORT: u16 = crate::udp::DEFAULT_PORT;

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

/// How the host reaches the concentrator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Bus {
    /// A card on the host's own SPI, with its reset and its supply on GPIO lines.
    Spi {
        /// The SPI device it answers on.
        spi: String,
        /// The GPIO character device its lines are on.
        gpio_chip: String,
        /// The line its reset pin is wired to.
        reset_line: u32,
        /// The line that switches the concentrator's supply on, for a board that gates it.
        ///
        /// A board wired this way answers nothing at all until the line is raised, and the
        /// line has to stay raised, so a gateway that names one keeps it held for as long
        /// as it runs.
        power_enable_line: Option<u32>,
    },
    /// A USB card, whose bridge does the SPI and drives the pins itself.
    Usb {
        /// The serial device the card enumerates as.
        port: String,
    },
}

/// Where the concentrator is wired.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Concentrator {
    /// How the host reaches it.
    pub bus: Bus,
    /// Whether the board wires its front ends single ended rather than differential.
    pub single_input: bool,
    /// Which front end the board carries.
    pub front_end: FrontEnd,
    /// Which front end the concentrator takes its clock from.
    pub clock: Chain,
    /// The SX1261 beside the concentrator, for a gateway that listens before it talks, surveys
    /// the band, or both.
    pub sx1261: Option<Sx1261Radio>,
    /// The gain control microcontroller image.
    pub gain_control_firmware: String,
    /// The arbiter microcontroller image.
    pub arbiter_firmware: String,
}

impl Concentrator {
    /// The channels the gateway checks before it talks, where it checks any.
    ///
    /// # Returns
    ///
    /// The section, or `None` for a gateway that transmits unchecked.
    pub fn listen_before_talk(&self) -> Option<&ListenBeforeTalk> {
        self.sx1261
            .as_ref()
            .and_then(|radio| radio.listen_before_talk.as_ref())
    }

    /// The band survey the gateway runs, where it runs one.
    ///
    /// # Returns
    ///
    /// The section, or `None` for a gateway that surveys nothing.
    pub fn spectral_scan(&self) -> Option<&SpectralScan> {
        self.sx1261
            .as_ref()
            .and_then(|radio| radio.spectral_scan.as_ref())
    }
}

/// The SX1261 beside a concentrator, and the jobs it has.
///
/// Semtech's reference card carries this second radio for two things the concentrator cannot
/// do for itself: a carrier check before a transmission, where the rules ask for one, and a
/// survey of the band. Both run from the same patch, so the radio is named once and each job
/// is a section of its own; a radio with neither has nothing to do and is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sx1261Radio {
    /// For a card on SPI, the radio's own SPI device, `/dev/spidev0.1` on Semtech's reference
    /// card. A USB card reaches the radio through its bridge and names none.
    pub spi: Option<String>,
    /// For a card on SPI, the radio's reset line on the concentrator's GPIO chip, 22 in the
    /// reference's `reset_lgw.sh`.
    pub reset_line: Option<u32>,
    /// Where Semtech's `sx1261_pram.var` is, the patch that gives the radio its carrier check
    /// and its scan.
    pub patch: String,
    /// The board's correction to the levels the radio measures, in dB.
    pub rssi_offset_db: i8,
    /// The channels the gateway checks before it talks, for a gateway that has to.
    pub listen_before_talk: Option<ListenBeforeTalk>,
    /// The band the gateway surveys, for one that does.
    pub spectral_scan: Option<SpectralScan>,
}

/// The channels a gateway checks before it transmits.
///
/// Some rules forbid transmitting into a channel someone else is using: ARIB STD-T108 in
/// Japan, and Korea's rules for KR920-923. A gateway under them names the channels it checks,
/// the level above which a channel counts as busy, and how long one transmission may hold a
/// channel, and the SX1261 listens on each before the concentrator is let transmit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListenBeforeTalk {
    /// The level above which a channel counts as busy, in dBm.
    pub threshold_dbm: i8,
    /// The channels to check.
    pub channels: Vec<lbt::Channel>,
}

impl ListenBeforeTalk {
    /// Finds the check channel a transmission goes out on.
    ///
    /// # Arguments
    ///
    /// * `frequency_hz` - the transmission's carrier.
    /// * `bandwidth_hz` - its bandwidth.
    ///
    /// # Returns
    ///
    /// The channel, or `None` for a transmission on a channel that is not checked.
    pub fn channel(&self, frequency_hz: u32, bandwidth_hz: u32) -> Option<lbt::Channel> {
        self.channels
            .iter()
            .find(|channel| channel.covers(frequency_hz, bandwidth_hz))
            .copied()
    }

    /// The threshold the radio is given, which is the configured one with the board's
    /// correction added, as the reference adds them.
    ///
    /// # Arguments
    ///
    /// * `rssi_offset_db` - the board's correction, from [`Sx1261Radio::rssi_offset_db`].
    ///
    /// # Returns
    ///
    /// The threshold in dBm, held to what the radio can be told.
    pub fn radio_threshold_dbm(&self, rssi_offset_db: i8) -> i8 {
        self.threshold_dbm
            .saturating_add(rssi_offset_db)
            .clamp(-127, 0)
    }
}

/// A survey of the band: a run of channels the SX1261 scans in turn between the gateway's
/// other work, as Semtech's packet forwarder does in a thread of its own.
///
/// The reference names these `freq_start`, `nb_chan`, `nb_scan` and `pace_s`, and steps
/// 200 kHz between channels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpectralScan {
    /// The first channel's carrier, in hertz.
    pub start_hz: u32,
    /// How many channels, 200 kHz apart, from there.
    pub channels: u8,
    /// How many samples each scan takes.
    pub samples: u16,
    /// How many seconds between scans. The reference holds this to at least one.
    pub every_s: u32,
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
    /// Whether the network is a public one, which decides the sync word the receivers look
    /// for. Every LoRaWAN network is public, and the chip powers up expecting a private one.
    pub lorawan_public: bool,
    /// The lowest and highest frequency the board may transmit on, in hertz.
    ///
    /// A board is built for a band, and asking it for a carrier outside that band radiates
    /// badly or not at all. A gateway that names no bounds is not checked against any.
    pub tx_bounds: Option<(u32, u32)>,
    /// The share of the time the band allows a transmitter, in parts per thousand.
    ///
    /// `Some(10)` is the 1% that most of the European band runs under. A gateway that names
    /// none is not held to one, which is right where the band has no such limit.
    pub duty_cycle_permille: Option<u32>,
    /// What the board reaches at each power, strongest last.
    ///
    /// Which amplifier setting and power step give which radiated power is a property of the
    /// board rather than of the chip, so a gateway with hardware unlike the reference design
    /// names its own.
    pub gains: Vec<Gain>,
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

    // A card is on one bus or the other. Naming both is refused rather than resolved, since
    // a gateway that quietly picked one would look wired to the other.
    let bus = match (held.get("usb"), held.get("spi")) {
        (Some(_), Some(_)) => {
            return Err(ConfigError::Refused {
                field: "concentrator".to_owned(),
                why: "names both a usb port and an spi device, and a card is on one or the other"
                    .to_owned(),
            })
        }
        (Some(_), None) => Bus::Usb {
            port: required_text(held, "concentrator.usb")?.to_owned(),
        },
        (None, _) => Bus::Spi {
            spi: required_text(held, "concentrator.spi")?.to_owned(),
            gpio_chip: required_text(held, "concentrator.gpio_chip")?.to_owned(),
            reset_line: required_whole(held, "concentrator.reset_line")?,
            power_enable_line: match held.get("power_enable_line") {
                None => None,
                Some(_) => Some(required_whole(held, "concentrator.power_enable_line")?),
            },
        },
    };

    Ok(Concentrator {
        bus: bus.clone(),
        single_input: held
            .get("single_input")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        front_end: front_end(held)?,
        clock: clock(held)?,
        sx1261: match held.get("sx1261") {
            None | Some(Value::Bool(false)) => {
                if held.contains_key("listen_before_talk") || held.contains_key("spectral_scan") {
                    return Err(refused(
                        "concentrator",
                        "listen_before_talk and spectral_scan are jobs of the SX1261, so they sit inside concentrator.sx1261",
                    ));
                }
                None
            }
            Some(Value::Object(section)) => Some(sx1261(section, &bus)?),
            Some(_) => {
                return Err(refused(
                    "concentrator.sx1261",
                    "this is an object naming the radio, its patch, and what it does",
                ))
            }
        },
        gain_control_firmware: required_text(firmware, "concentrator.firmware.gain_control")?
            .to_owned(),
        arbiter_firmware: required_text(firmware, "concentrator.firmware.arbiter")?.to_owned(),
    })
}

/// Reads a whole number of decibels, or the default where the field is absent and has one.
fn decibels(
    held: &Map<String, Value>,
    field: &str,
    name: &str,
    default: Option<i8>,
) -> Result<i8, ConfigError> {
    let field = format!("{field}.{name}");
    match (held.get(name), default) {
        (None, Some(default)) => Ok(default),
        (None, None) => Err(missing(&field)),
        (Some(value), _) => value
            .as_i64()
            .and_then(|held| i8::try_from(held).ok())
            .ok_or_else(|| refused(&field, "this is a whole number of decibels")),
    }
}

/// Reads the SX1261 beside the concentrator and what it does.
fn sx1261(held: &Map<String, Value>, bus: &Bus) -> Result<Sx1261Radio, ConfigError> {
    const FIELD: &str = "concentrator.sx1261";

    let (spi, reset_line) = match bus {
        Bus::Spi { .. } => (
            Some(required_text(held, &format!("{FIELD}.spi"))?.to_owned()),
            Some(required_whole(held, &format!("{FIELD}.reset_line"))?),
        ),
        Bus::Usb { .. } => {
            if held.contains_key("spi") || held.contains_key("reset_line") {
                return Err(refused(
                    FIELD,
                    "a USB card reaches its SX1261 through the bridge, so it names no spi device or reset line",
                ));
            }
            (None, None)
        }
    };

    let section = |name: &str, what: &str| -> Result<Option<&Map<String, Value>>, ConfigError> {
        match held.get(name) {
            None | Some(Value::Bool(false)) => Ok(None),
            Some(Value::Object(section)) => Ok(Some(section)),
            Some(_) => Err(refused(&format!("{FIELD}.{name}"), what)),
        }
    };
    let listen_before_talk = section(
        "listen_before_talk",
        "this is an object naming the threshold and the channels the radio checks",
    )?
    .map(listen_before_talk)
    .transpose()?;
    let spectral_scan = section(
        "spectral_scan",
        "this is an object naming where the survey starts, how many channels it covers, how many samples each takes and how often",
    )?
    .map(spectral_scan)
    .transpose()?;
    if listen_before_talk.is_none() && spectral_scan.is_none() {
        return Err(refused(
            FIELD,
            "the radio has nothing to do: name listen_before_talk, spectral_scan, or both",
        ));
    }

    Ok(Sx1261Radio {
        spi,
        reset_line,
        patch: required_text(held, &format!("{FIELD}.patch"))?.to_owned(),
        rssi_offset_db: decibels(held, FIELD, "rssi_offset_db", Some(0))?,
        listen_before_talk,
        spectral_scan,
    })
}

/// Reads the channels the SX1261 checks before the gateway talks.
fn listen_before_talk(held: &Map<String, Value>) -> Result<ListenBeforeTalk, ConfigError> {
    const FIELD: &str = "concentrator.sx1261.listen_before_talk";

    let threshold_dbm = decibels(held, FIELD, "threshold_dbm", None)?;
    if !(-127..=0).contains(&threshold_dbm) {
        return Err(refused(
            &format!("{FIELD}.threshold_dbm"),
            &format!("{threshold_dbm} dBm is not a level from -127 to 0 dBm"),
        ));
    }

    let listed = held
        .get("channels")
        .and_then(Value::as_array)
        .ok_or_else(|| missing(&format!("{FIELD}.channels")))?;
    if listed.is_empty() {
        return Err(refused(
            &format!("{FIELD}.channels"),
            "checking no channels checks nothing",
        ));
    }
    let channels = listed
        .iter()
        .map(check_channel)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ListenBeforeTalk {
        threshold_dbm,
        channels,
    })
}

/// Reads the band the SX1261 surveys.
fn spectral_scan(held: &Map<String, Value>) -> Result<SpectralScan, ConfigError> {
    const FIELD: &str = "concentrator.sx1261.spectral_scan";

    let channels = required_whole(held, &format!("{FIELD}.channels"))?;
    let channels = u8::try_from(channels)
        .ok()
        .filter(|count| *count > 0)
        .ok_or_else(|| {
            refused(
                &format!("{FIELD}.channels"),
                &format!("a survey covers 1 to 255 channels, not {channels}"),
            )
        })?;
    let samples = required_whole(held, &format!("{FIELD}.samples"))?;
    let samples = u16::try_from(samples)
        .ok()
        .filter(|count| *count > 0)
        .ok_or_else(|| {
            refused(
                &format!("{FIELD}.samples"),
                &format!("a scan takes 1 to 65535 samples, not {samples}"),
            )
        })?;

    Ok(SpectralScan {
        start_hz: required_whole(held, &format!("{FIELD}.start_hz"))?,
        channels,
        samples,
        every_s: match held.get("every_s") {
            None => 10,
            Some(_) => required_whole(held, &format!("{FIELD}.every_s"))?,
        },
    })
}

/// Reads one channel the SX1261 checks.
fn check_channel(entry: &Value) -> Result<lbt::Channel, ConfigError> {
    const FIELD: &str = "concentrator.sx1261.listen_before_talk.channels";
    let held = entry
        .as_object()
        .ok_or_else(|| refused(FIELD, "every channel is an object"))?;

    let frequency_hz = required_whole(held, &format!("{FIELD}.frequency_hz"))?;
    let bandwidth = match required_whole(held, &format!("{FIELD}.bandwidth_hz"))? {
        125_000 => Bandwidth::Khz125,
        250_000 => Bandwidth::Khz250,
        other => {
            return Err(refused(
                &format!("{FIELD}.bandwidth_hz"),
                &format!("the SX1261 checks 125000 or 250000 Hz channels, not {other}"),
            ))
        }
    };
    let scan_micros = required_whole(held, &format!("{FIELD}.scan_time_us"))?;
    let scan_time = u16::try_from(scan_micros)
        .ok()
        .and_then(ScanTime::of_micros)
        .ok_or_else(|| {
            refused(
                &format!("{FIELD}.scan_time_us"),
                &format!("the SX1261 scans for 128 or 5000 us, not {scan_micros}"),
            )
        })?;
    let transmit_ms = required_whole(held, &format!("{FIELD}.transmit_time_ms"))?;
    let transmit_time_ms = u16::try_from(transmit_ms)
        .ok()
        .filter(|ms| u32::from(*ms) * 1_000 > lbt::SENSE_LEAD_US)
        .ok_or_else(|| {
            refused(
                &format!("{FIELD}.transmit_time_ms"),
                &format!(
                    "{transmit_ms} ms leaves no time to transmit after the channel is sensed {} us ahead",
                    lbt::SENSE_LEAD_US
                ),
            )
        })?;

    Ok(lbt::Channel {
        frequency_hz,
        bandwidth,
        scan_time,
        transmit_time_ms,
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

    // Every LoRaWAN network is public, and a gateway that assumes otherwise hears none of
    // them, so this is the default rather than something each configuration has to say.
    let lorawan_public = held
        .get("lorawan_public")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    // Both bounds or neither: one on its own reads like a limit and enforces nothing on the
    // side it leaves open.
    let tx_bounds = match (held.get("tx_freq_min_hz"), held.get("tx_freq_max_hz")) {
        (None, None) => None,
        (Some(_), None) => return Err(missing("radio.tx_freq_max_hz")),
        (None, Some(_)) => return Err(missing("radio.tx_freq_min_hz")),
        (Some(_), Some(_)) => {
            let lowest = required_whole(held, "radio.tx_freq_min_hz")?;
            let highest = required_whole(held, "radio.tx_freq_max_hz")?;
            if lowest > highest {
                return Err(refused(
                    "radio.tx_freq_min_hz",
                    &format!("{lowest} Hz is above the highest, {highest} Hz"),
                ));
            }
            Some((lowest, highest))
        }
    };

    let duty_cycle_permille = match held.get("duty_cycle_permille") {
        None => None,
        Some(value) => {
            let permille = value.as_u64().ok_or_else(|| {
                refused(
                    "radio.duty_cycle_permille",
                    "this is a whole number of parts per thousand",
                )
            })?;
            if permille == 0 || permille > 1000 {
                return Err(refused(
                    "radio.duty_cycle_permille",
                    &format!("{permille} is not a share between 1 and 1000"),
                ));
            }
            Some(permille as u32)
        }
    };

    let gains = match held.get("gain_table") {
        None => DEFAULT_GAINS.to_vec(),
        Some(value) => {
            let listed = value
                .as_array()
                .ok_or_else(|| refused("radio.gain_table", "the gain table is a list"))?;
            if listed.is_empty() {
                return Err(refused(
                    "radio.gain_table",
                    "a table with no entries reaches no power at all",
                ));
            }
            listed.iter().map(gain).collect::<Result<Vec<_>, _>>()?
        }
    };

    Ok(Radio {
        carrier_hz,
        channels,
        spreading_factors,
        dual_demodulation,
        lorawan_public,
        tx_bounds,
        duty_cycle_permille,
        gains,
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

/// Reads one entry of the board's transmit gain table.
///
/// The names are this crate's rather than the reference implementation's: `radiated_dbm` is
/// its `rf_power`, `amplifier` its `pa_gain`, `power_index` its `pwr_idx`, and
/// `digital_gain` its `dig_gain`. The two offsets are the calibrated ones and default to
/// zero, which is what a board that has not been calibrated carries.
fn gain(entry: &Value) -> Result<Gain, ConfigError> {
    let held = entry
        .as_object()
        .ok_or_else(|| refused("radio.gain_table", "every entry is an object"))?;

    let signed = |field: &str, name: &str| -> Result<i8, ConfigError> {
        match held.get(name) {
            None => Ok(0),
            Some(value) => value
                .as_i64()
                .and_then(|held| i8::try_from(held).ok())
                .ok_or_else(|| refused(field, "this is a small whole number")),
        }
    };
    let small = |field: &str, name: &str| -> Result<u8, ConfigError> {
        match held.get(name) {
            None => Ok(0),
            Some(value) => value
                .as_u64()
                .and_then(|held| u8::try_from(held).ok())
                .ok_or_else(|| refused(field, "this is a small whole number")),
        }
    };

    Ok(Gain {
        radiated_dbm: held
            .get("radiated_dbm")
            .and_then(Value::as_i64)
            .and_then(|held| i8::try_from(held).ok())
            .ok_or_else(|| {
                refused(
                    "radio.gain_table.radiated_dbm",
                    "every entry says what it radiates, in dBm",
                )
            })?,
        amplifier: small("radio.gain_table.amplifier", "amplifier")?,
        power_index: small("radio.gain_table.power_index", "power_index")?,
        digital_gain: small("radio.gain_table.digital_gain", "digital_gain")?,
        offset_i: signed("radio.gain_table.offset_i", "offset_i")?,
        offset_q: signed("radio.gain_table.offset_q", "offset_q")?,
    })
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

    /// The same configuration with something added to its radio.
    fn radio_with(extra: &str) -> String {
        complete().replace(
            r#""spreading_factors": [7, 8, 9]"#,
            &format!(r#""spreading_factors": [7, 8, 9], {extra}"#),
        )
    }

    /// The supply line a configuration names, when its card is on SPI.
    fn supply_line(text: &str) -> Option<u32> {
        match Config::parse(text)
            .expect("the configuration parses")
            .concentrator
            .bus
        {
            Bus::Spi {
                power_enable_line, ..
            } => power_enable_line,
            Bus::Usb { .. } => panic!("a USB card has no supply line to name"),
        }
    }

    /// The same configuration with an SX1261 section on its concentrator.
    fn checking(section: &str) -> String {
        complete().replace(
            r#""single_input": true,"#,
            &format!(r#""single_input": true, "sx1261": {section},"#),
        )
    }

    /// A section as a gateway in Japan would write one, on Semtech's reference wiring.
    const JAPAN: &str = r#"{
      "spi": "/dev/spidev0.1",
      "reset_line": 22,
      "patch": "/opt/sx1302_hal/libloragw/src/sx1261_pram.var",
      "rssi_offset_db": -4,
      "listen_before_talk": {
        "threshold_dbm": -80,
        "channels": [
          { "frequency_hz": 920600000, "bandwidth_hz": 125000, "scan_time_us": 5000, "transmit_time_ms": 4000 },
          { "frequency_hz": 920800000, "bandwidth_hz": 250000, "scan_time_us": 128, "transmit_time_ms": 400 }
        ]
      }
    }"#;

    /// A section as a gateway in Europe surveying its band would write one.
    const SURVEY: &str = r#"{
      "spi": "/dev/spidev0.1",
      "reset_line": 22,
      "patch": "/opt/sx1302_hal/libloragw/src/sx1261_pram.var",
      "spectral_scan": { "start_hz": 867100000, "channels": 8, "samples": 2000, "every_s": 10 }
    }"#;

    #[test]
    fn listen_before_talk_reads_its_radio_threshold_and_channels() {
        let config = Config::parse(&checking(JAPAN)).expect("a complete section");
        let radio = config.concentrator.sx1261.expect("the radio is there");
        assert_eq!(radio.spi.as_deref(), Some("/dev/spidev0.1"));
        assert_eq!(radio.reset_line, Some(22));
        assert_eq!(radio.rssi_offset_db, -4);
        assert!(radio.spectral_scan.is_none());
        let section = radio.listen_before_talk.expect("the section is there");
        assert_eq!(section.threshold_dbm, -80);
        assert_eq!(
            section.radio_threshold_dbm(radio.rssi_offset_db),
            -84,
            "the board's offset is added, as the reference adds it"
        );
        assert_eq!(
            section.channels,
            [
                lbt::Channel {
                    frequency_hz: 920_600_000,
                    bandwidth: Bandwidth::Khz125,
                    scan_time: ScanTime::Long,
                    transmit_time_ms: 4000,
                },
                lbt::Channel {
                    frequency_hz: 920_800_000,
                    bandwidth: Bandwidth::Khz250,
                    scan_time: ScanTime::Short,
                    transmit_time_ms: 400,
                },
            ]
        );

        // A carrier that came through floating point still finds its channel.
        assert!(section.channel(920_600_004, 125_000).is_some());
        assert!(section.channel(920_600_000, 250_000).is_none());
        assert!(section.channel(923_200_000, 125_000).is_none());
    }

    #[test]
    fn the_radio_is_absent_or_false_or_a_section_with_something_to_do() {
        assert!(Config::parse(&complete())
            .expect("parses")
            .concentrator
            .sx1261
            .is_none());
        assert!(Config::parse(&checking("false"))
            .expect("parses")
            .concentrator
            .sx1261
            .is_none());
        assert!(matches!(
            Config::parse(&checking("true")),
            Err(ConfigError::Refused { field, .. }) if field == "concentrator.sx1261"
        ));

        // A radio named with neither job is refused rather than brought up for nothing.
        let idle = r#"{ "spi": "/dev/spidev0.1", "reset_line": 22, "patch": "p" }"#;
        assert!(matches!(
            Config::parse(&checking(idle)),
            Err(ConfigError::Refused { field, .. }) if field == "concentrator.sx1261"
        ));

        // A job named beside the radio rather than inside it is pointed at where it goes.
        let outside = complete().replace(
            r#""single_input": true,"#,
            r#""single_input": true, "listen_before_talk": { "threshold_dbm": -80, "channels": [] },"#,
        );
        assert!(matches!(
            Config::parse(&outside),
            Err(ConfigError::Refused { field, why }) if field == "concentrator" && why.contains("concentrator.sx1261")
        ));
    }

    #[test]
    fn each_part_of_a_check_the_radio_cannot_do_is_refused_by_name() {
        let refused_field = |section: String| match Config::parse(&checking(&section)) {
            Err(ConfigError::Refused { field, .. }) | Err(ConfigError::Missing { field }) => field,
            other => panic!("expected a refusal, got {other:?}"),
        };

        assert_eq!(
            refused_field(JAPAN.replace(r#""scan_time_us": 5000"#, r#""scan_time_us": 1000"#)),
            "concentrator.sx1261.listen_before_talk.channels.scan_time_us"
        );
        assert_eq!(
            refused_field(JAPAN.replace(r#""bandwidth_hz": 250000"#, r#""bandwidth_hz": 500000"#)),
            "concentrator.sx1261.listen_before_talk.channels.bandwidth_hz"
        );
        assert_eq!(
            refused_field(JAPAN.replace(r#""transmit_time_ms": 400"#, r#""transmit_time_ms": 1"#)),
            "concentrator.sx1261.listen_before_talk.channels.transmit_time_ms"
        );
        assert_eq!(
            refused_field(JAPAN.replace(r#""threshold_dbm": -80"#, r#""threshold_dbm": 10"#)),
            "concentrator.sx1261.listen_before_talk.threshold_dbm"
        );
        assert_eq!(
            refused_field(JAPAN.replace(r#""reset_line": 22,"#, "")),
            "concentrator.sx1261.reset_line"
        );
        assert_eq!(
            refused_field(JAPAN.replace(
                r#""patch": "/opt/sx1302_hal/libloragw/src/sx1261_pram.var","#,
                ""
            )),
            "concentrator.sx1261.patch"
        );
        let no_channels = r#"{ "spi": "/dev/spidev0.1", "reset_line": 22, "patch": "p",
            "listen_before_talk": { "threshold_dbm": -80, "channels": [] } }"#;
        assert_eq!(
            refused_field(no_channels.to_owned()),
            "concentrator.sx1261.listen_before_talk.channels"
        );
    }

    #[test]
    fn a_survey_reads_its_run_of_channels_and_its_pace() {
        let config = Config::parse(&checking(SURVEY)).expect("a complete section");
        let radio = config
            .concentrator
            .sx1261
            .as_ref()
            .expect("the radio is there");
        assert!(radio.listen_before_talk.is_none());
        assert_eq!(radio.rssi_offset_db, 0, "the offset defaults to none");
        assert_eq!(
            radio.spectral_scan,
            Some(SpectralScan {
                start_hz: 867_100_000,
                channels: 8,
                samples: 2000,
                every_s: 10,
            })
        );
        assert_eq!(
            config
                .concentrator
                .spectral_scan()
                .map(|scan| scan.channels),
            Some(8)
        );
        assert!(config.concentrator.listen_before_talk().is_none());

        // The pace defaults to the reference's ten seconds.
        let unpaced = SURVEY.replace(r#", "every_s": 10"#, "");
        assert_eq!(
            Config::parse(&checking(&unpaced))
                .expect("parses")
                .concentrator
                .spectral_scan()
                .map(|scan| scan.every_s),
            Some(10)
        );
    }

    #[test]
    fn each_part_of_a_survey_the_radio_cannot_run_is_refused_by_name() {
        let refused_field = |section: String| match Config::parse(&checking(&section)) {
            Err(ConfigError::Refused { field, .. }) | Err(ConfigError::Missing { field }) => field,
            other => panic!("expected a refusal, got {other:?}"),
        };

        assert_eq!(
            refused_field(SURVEY.replace(r#""channels": 8"#, r#""channels": 0"#)),
            "concentrator.sx1261.spectral_scan.channels"
        );
        assert_eq!(
            refused_field(SURVEY.replace(r#""channels": 8"#, r#""channels": 256"#)),
            "concentrator.sx1261.spectral_scan.channels"
        );
        assert_eq!(
            refused_field(SURVEY.replace(r#""samples": 2000"#, r#""samples": 0"#)),
            "concentrator.sx1261.spectral_scan.samples"
        );
        assert_eq!(
            refused_field(SURVEY.replace(r#""start_hz": 867100000, "#, "")),
            "concentrator.sx1261.spectral_scan.start_hz"
        );
        assert_eq!(
            refused_field(
                SURVEY.replace(r#""spectral_scan": {"#, r#""spectral_scan": true, "x": {"#)
            ),
            "concentrator.sx1261.spectral_scan"
        );
    }

    #[test]
    fn a_radio_can_check_and_survey_at_once() {
        let both = JAPAN.replace(
            r#""listen_before_talk": {"#,
            r#""spectral_scan": { "start_hz": 920600000, "channels": 4, "samples": 500 },
      "listen_before_talk": {"#,
        );
        let config = Config::parse(&checking(&both)).expect("both jobs parse");
        let radio = config.concentrator.sx1261.expect("the radio is there");
        assert_eq!(
            radio
                .listen_before_talk
                .as_ref()
                .map(|lbt| lbt.channels.len()),
            Some(2)
        );
        assert_eq!(
            radio.spectral_scan.as_ref().map(|scan| scan.channels),
            Some(4)
        );
    }

    #[test]
    fn a_usb_card_reaches_its_radio_through_the_bridge() {
        let usb = |section: &str| {
            checking(section).replace(
                r#""spi": "/dev/spidev0.0",
            "gpio_chip": "/dev/gpiochip0",
            "reset_line": 23,"#,
                r#""usb": "/dev/ttyACM0","#,
            )
        };
        let bridged = r#"{ "patch": "p", "listen_before_talk": { "threshold_dbm": -80, "channels": [
            { "frequency_hz": 922100000, "bandwidth_hz": 125000, "scan_time_us": 5000, "transmit_time_ms": 4000 }
        ] } }"#;
        let config = Config::parse(&usb(bridged)).expect("a USB section names no wiring");
        let radio = config.concentrator.sx1261.expect("present");
        assert_eq!((radio.spi, radio.reset_line), (None, None));

        assert!(matches!(
            Config::parse(&usb(JAPAN)),
            Err(ConfigError::Refused { field, .. }) if field == "concentrator.sx1261"
        ));
    }

    #[test]
    fn a_board_that_gates_its_supply_names_the_line() {
        assert_eq!(supply_line(&complete()), None);

        // Eighteen is the line the reference design gates its concentrator behind.
        let gated = complete().replace(
            r#""reset_line": 23,"#,
            r#""reset_line": 23, "power_enable_line": 18,"#,
        );
        assert_eq!(supply_line(&gated), Some(18));

        // A line that is not a number is refused by its name rather than quietly dropped,
        // which on a board wired this way would look like a concentrator that never answers.
        let wrong = complete().replace(
            r#""reset_line": 23,"#,
            r#""reset_line": 23, "power_enable_line": "eighteen","#,
        );
        assert!(
            matches!(Config::parse(&wrong), Err(ConfigError::Refused { field, .. }) if field == "concentrator.power_enable_line")
        );
    }

    #[test]
    fn the_transmit_bounds_are_named_together_or_not_at_all() {
        assert_eq!(
            Config::parse(&complete())
                .expect("complete")
                .radio
                .tx_bounds,
            None
        );

        let both = radio_with(r#""tx_freq_min_hz": 863000000, "tx_freq_max_hz": 870000000"#);
        assert_eq!(
            Config::parse(&both).expect("both bounds").radio.tx_bounds,
            Some((863_000_000, 870_000_000))
        );

        // One on its own reads like a limit while enforcing nothing on the side it leaves
        // open, so it is refused by the name of the one that is missing.
        let half = radio_with(r#""tx_freq_min_hz": 863000000"#);
        assert!(
            matches!(Config::parse(&half), Err(ConfigError::Missing { field }) if field == "radio.tx_freq_max_hz")
        );

        let upside_down = radio_with(r#""tx_freq_min_hz": 870000000, "tx_freq_max_hz": 863000000"#);
        assert!(
            matches!(Config::parse(&upside_down), Err(ConfigError::Refused { field, .. }) if field == "radio.tx_freq_min_hz")
        );
    }

    #[test]
    fn a_duty_cycle_is_a_share_of_the_time() {
        assert_eq!(
            Config::parse(&complete())
                .expect("complete")
                .radio
                .duty_cycle_permille,
            None
        );

        let one_percent = radio_with(r#""duty_cycle_permille": 10"#);
        assert_eq!(
            Config::parse(&one_percent)
                .expect("a share")
                .radio
                .duty_cycle_permille,
            Some(10)
        );

        // A share of none would mean never transmitting, and a share above all of the time
        // is not a share at all.
        for share in ["0", "1001"] {
            let refused = radio_with(&format!(r#""duty_cycle_permille": {share}"#));
            assert!(
                matches!(Config::parse(&refused), Err(ConfigError::Refused { field, .. }) if field == "radio.duty_cycle_permille"),
                "{share} was accepted"
            );
        }
    }

    #[test]
    fn a_board_can_name_the_powers_it_reaches() {
        assert_eq!(
            Config::parse(&complete())
                .expect("complete")
                .radio
                .gains
                .len(),
            DEFAULT_GAINS.len()
        );

        let own = radio_with(
            r#""gain_table": [{"radiated_dbm": 14, "amplifier": 0, "power_index": 17}]"#,
        );
        let named = Config::parse(&own).expect("a table");
        assert_eq!(named.radio.gains.len(), 1);
        assert_eq!(named.radio.gains[0].radiated_dbm, 14);
        assert_eq!(named.radio.gains[0].power_index, 17);

        // The calibrated offsets default to none, which is what an uncalibrated board holds.
        assert_eq!(named.radio.gains[0].offset_i, 0);
        assert_eq!(named.radio.gains[0].offset_q, 0);

        let empty = radio_with(r#""gain_table": []"#);
        assert!(
            matches!(Config::parse(&empty), Err(ConfigError::Refused { field, .. }) if field == "radio.gain_table")
        );
    }

    #[test]
    fn a_board_takes_the_usual_answers_when_it_says_nothing() {
        // An ordinary gateway names none of these, so the defaults have to be the ordinary
        // board: an SX1250 pair, clocked from the first chain, with neither listen before
        // talk nor double demodulation.
        let config = Config::parse(&complete()).expect("every field is there");

        assert_eq!(config.concentrator.front_end, FrontEnd::Sx1250);
        assert_eq!(config.concentrator.clock, Chain::A);
        assert!(config.concentrator.sx1261.is_none());
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

        assert_eq!(
            config.concentrator.bus,
            Bus::Spi {
                spi: "/dev/spidev0.0".to_owned(),
                gpio_chip: "/dev/gpiochip0".to_owned(),
                reset_line: 23,
                power_enable_line: None,
            }
        );
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
    fn a_usb_card_names_its_port_and_none_of_the_lines() {
        let usb = complete()
            .replace(r#""spi": "/dev/spidev0.0","#, r#""usb": "/dev/ttyACM0","#)
            .replace(r#""gpio_chip": "/dev/gpiochip0","#, "")
            .replace(r#""reset_line": 23,"#, "");
        let config = Config::parse(&usb).expect("a USB card needs no lines");

        assert_eq!(
            config.concentrator.bus,
            Bus::Usb {
                port: "/dev/ttyACM0".to_owned()
            }
        );
    }

    #[test]
    fn a_card_named_on_both_buses_is_refused() {
        let both = complete().replace(
            r#""spi": "/dev/spidev0.0","#,
            r#""spi": "/dev/spidev0.0", "usb": "/dev/ttyACM0","#,
        );
        assert!(
            matches!(Config::parse(&both), Err(ConfigError::Refused { field, .. }) if field == "concentrator")
        );
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
