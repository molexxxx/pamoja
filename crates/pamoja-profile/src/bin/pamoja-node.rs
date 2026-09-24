//! Runs a profile from a site's wiring file, with no program to write.
//!
//! Run: `pamoja-node /etc/pamoja/coop.json`
//!
//! The wiring file names the profile it runs, the part that takes the readings, the line
//! the output drives, and the link the readings go over. The node ticks at the cadence the
//! profile sets for the battery's charge until it is stopped, and a tick that fails is
//! reported and tried again at the next interval rather than ending the node. `--check`
//! reads and checks both files and says what would run without touching any hardware.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use pamoja_codec::JsonCodec;
use pamoja_core::{Actuator, Error, Result, Sensor, Transport};
use pamoja_hal::bus::{BusDelay, I2cBus};
use pamoja_mqtt::{MqttConfig, MqttTransport, Tls};
use pamoja_profile::wiring::{
    BatteryWiring, LinkWiring, OutputWiring, Part, Plan, SensorWiring, Wiring,
};
use pamoja_profile::{Node, Profile, Tick};
use pamoja_sensors::driver::I2cRegisters;
use pamoja_sensors::{
    bme280, bmp280, ds18b20, hdc1080, ina219, ina226, opt3001, scd4x, sht3x, tmp117,
};

const USAGE: &str = "usage: pamoja-node <wiring.json> [--check] [--ticks N] [--fast]

  <wiring.json>  the site's wiring file, which names the profile it runs
  --check        read and check both files and say what would run, touching no hardware
  --ticks N      stop after N ticks
  --fast         tick again at once rather than waiting the interval the battery allows";

/// What the command line asked for.
struct Options {
    wiring: PathBuf,
    check: bool,
    ticks: Option<u64>,
    fast: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    let options = match options(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match run(options).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("pamoja-node: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Reads the command line.
fn options(mut args: impl Iterator<Item = String>) -> std::result::Result<Options, String> {
    let mut wiring = None;
    let mut check = false;
    let mut ticks = None;
    let mut fast = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--fast" => fast = true,
            "--ticks" => {
                let count = args.next().ok_or("--ticks takes a number")?;
                ticks = Some(
                    count
                        .parse()
                        .map_err(|_| format!("--ticks takes a whole number, not {count}"))?,
                );
            }
            "-h" | "--help" => {
                return Err("pamoja-node runs a profile from a wiring file".to_owned())
            }
            flag if flag.starts_with('-') => return Err(format!("{flag} is not an option")),
            path if wiring.is_none() => wiring = Some(PathBuf::from(path)),
            extra => return Err(format!("one wiring file at a time, not {extra} as well")),
        }
    }
    Ok(Options {
        wiring: wiring.ok_or("name the wiring file to run")?,
        check,
        ticks,
        fast,
    })
}

/// Loads both files, opens the parts, and runs the node until it is stopped.
async fn run(options: Options) -> std::result::Result<(), String> {
    let wiring_path = &options.wiring;
    let wiring = Wiring::from_json(&read(wiring_path)?)
        .map_err(|error| format!("{}: {error}", wiring_path.display()))?;
    let profile_path = wiring_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(&wiring.profile);
    let profile = Profile::from_json(&read(&profile_path)?)
        .map_err(|error| format!("{}: {error}", profile_path.display()))?;
    let plan = wiring.fits(&profile).map_err(|error| {
        format!(
            "{} cannot run {}: {error}",
            wiring_path.display(),
            profile_path.display()
        )
    })?;
    println!("{}", describe(&wiring, &profile, &plan));
    if options.check {
        return Ok(());
    }

    let unit = plan.to.clone();
    let sensor = Reader::open(&wiring.sensor, plan)?;
    let output = Output::open(wiring.output.as_ref(), &wiring.site)?;
    let mut link = Link::open(&wiring)?;
    link.connect()
        .await
        .map_err(|error| format!("connecting the link: {error}"))?;
    let mut battery = Battery::open(wiring.battery.as_ref())?;
    let label = output.label();
    let mut node =
        Node::new(profile, sensor, output, link, JsonCodec).map_err(|error| error.to_string())?;

    let mut count = 0;
    loop {
        match node.tick().await {
            Ok(tick) => println!("{}", line(&tick, &unit, label.as_deref())),
            Err(Error::Closed) => {
                println!("the sensor has no more readings");
                break;
            }
            Err(error) => {
                eprintln!("pamoja-node: a tick failed, trying again at the next interval: {error}")
            }
        }
        count += 1;
        if options.ticks.is_some_and(|limit| count >= limit) {
            break;
        }
        let charge = battery.charge().await;
        let (_, wait) = node.schedule(charge, false);
        if options.fast {
            continue;
        }
        tokio::select! {
            () = tokio::time::sleep(wait) => {}
            _ = tokio::signal::ctrl_c() => break,
        }
    }
    node.transport_mut().close().await;
    Ok(())
}

/// Reads a file, naming it when it cannot.
fn read(path: &Path) -> std::result::Result<String, String> {
    std::fs::read_to_string(path).map_err(|error| format!("reading {}: {error}", path.display()))
}

/// Says in one line what the wiring runs.
fn describe(wiring: &Wiring, profile: &Profile, plan: &Plan) -> String {
    let sensor = &wiring.sensor;
    let part = sensor.part.name();
    let source = match sensor.part {
        Part::Replay => format!("{} replayed readings", sensor.readings.len()),
        Part::Ds18b20 => format!(
            "a ds18b20, serial {}",
            sensor.serial.as_deref().unwrap_or_default()
        ),
        _ => format!(
            "a {part} at {:#04x} on {}",
            address(sensor),
            sensor.bus.as_deref().unwrap_or_default()
        ),
    };
    let output = match &wiring.output {
        Some(OutputWiring::Gpio { chip, line, .. }) => {
            format!(", the output on {chip} line {line}")
        }
        Some(OutputWiring::Print { label }) => format!(", the {label} printed"),
        None => String::new(),
    };
    let link = match &wiring.link {
        LinkWiring::Mqtt { host, port, .. } => format!("mqtt {host}:{port}"),
        LinkWiring::Print => "print".to_owned(),
    };
    format!(
        "{} runs {}: {} in {} from {source}{output}, reporting on {} over {link}",
        wiring.site, profile.name, plan.quantity, plan.to, profile.topic
    )
}

/// Says in one line what a tick read and did.
fn line(tick: &Tick, unit: &str, label: Option<&str>) -> String {
    let mut text = format!("{} {unit}", tick.reading);
    if let (Some(on), Some(label)) = (tick.reaction.actuator, label) {
        text.push_str(&format!(", {label} {}", if on { "on" } else { "off" }));
    }
    if let Some(alert) = tick.reaction.alert {
        text.push_str(&format!(", alert {}", alert.kind()));
    }
    text
}

/// The address a sensor answers at: the one the wiring names, or the part's usual one.
fn address(sensor: &SensorWiring) -> u8 {
    sensor
        .address
        .or(sensor.part.default_address())
        .unwrap_or_default()
}

/// Opens an I2C bus by its path, or the part's simulated twin for `sim`.
fn bus(path: &str, part: Part, address: u8) -> std::result::Result<I2cBus, String> {
    if path == "sim" {
        let simulated: pamoja_hal::sim::Part = match part {
            Part::Bme280 => bme280::sim::part(address).into(),
            Part::Bmp280 => bmp280::sim::part(address).into(),
            Part::Sht3x => sht3x::sim::part(address).into(),
            Part::Hdc1080 => hdc1080::sim::part().into(),
            Part::Tmp117 => tmp117::sim::part(address).into(),
            Part::Scd4x => scd4x::sim::part().into(),
            Part::Opt3001 => opt3001::sim::part(address).into(),
            Part::Ina219 => ina219::sim::part(address).into(),
            Part::Ina226 => ina226::sim::part(address).into(),
            Part::Ds18b20 | Part::Replay => {
                return Err(format!("a {} is not on an I2C bus", part.name()))
            }
        };
        return Ok(I2cBus::simulated([simulated]));
    }
    I2cBus::open(path).map_err(|error| format!("opening {path}: {error}"))
}

/// A driver's failure, as text.
fn failed(part: Part, error: impl core::fmt::Debug) -> String {
    format!(
        "the {} did not answer as the datasheet says: {error:?}",
        part.name()
    )
}

/// The part behind the node's reading.
enum Probe {
    Bme280(bme280::Bme280<I2cRegisters<I2cBus>, BusDelay>),
    Bmp280(bmp280::Bmp280<I2cRegisters<I2cBus>, BusDelay>),
    Sht3x(sht3x::Sht3x<I2cBus, BusDelay>),
    Hdc1080(hdc1080::Hdc1080<I2cBus, BusDelay>),
    Tmp117(tmp117::Tmp117<I2cBus, BusDelay>),
    Scd4x(scd4x::Scd4x<I2cBus, BusDelay>),
    Opt3001(opt3001::Opt3001<I2cBus, BusDelay>),
    Ina219(ina219::Ina219<I2cBus, BusDelay>),
    Ina226(ina226::Ina226<I2cBus, BusDelay>),
    Ds18b20(ds18b20::linux::Thermometer),
    Replay(std::vec::IntoIter<f32>),
}

/// The node's sensor: the part, and how its reading reaches the profile's unit.
struct Reader {
    probe: Probe,
    plan: Plan,
}

impl Reader {
    /// Opens the wired part and readies it to measure.
    fn open(wiring: &SensorWiring, plan: Plan) -> std::result::Result<Reader, String> {
        let part = wiring.part;
        let at = address(wiring);
        let on = |address| bus(wiring.bus.as_deref().unwrap_or_default(), part, address);
        let probe = match part {
            Part::Bme280 => {
                let bus = on(at)?;
                let mut driver = bme280::Bme280::i2c(bus.clone(), at, bus.delay());
                driver.init().map_err(|error| failed(part, error))?;
                Probe::Bme280(driver)
            }
            Part::Bmp280 => {
                let bus = on(at)?;
                let mut driver = bmp280::Bmp280::i2c(bus.clone(), at, bus.delay());
                driver.init().map_err(|error| failed(part, error))?;
                Probe::Bmp280(driver)
            }
            Part::Sht3x => {
                let bus = on(at)?;
                let mut driver = sht3x::Sht3x::new(bus.clone(), at, bus.delay());
                driver.init().map_err(|error| failed(part, error))?;
                Probe::Sht3x(driver)
            }
            Part::Hdc1080 => {
                let bus = on(at)?;
                let mut driver = hdc1080::Hdc1080::new(bus.clone(), bus.delay());
                driver.init().map_err(|error| failed(part, error))?;
                Probe::Hdc1080(driver)
            }
            Part::Tmp117 => {
                let bus = on(at)?;
                let mut driver = tmp117::Tmp117::new(bus.clone(), at, bus.delay());
                driver.init().map_err(|error| failed(part, error))?;
                Probe::Tmp117(driver)
            }
            Part::Scd4x => {
                let bus = on(at)?;
                let mut driver = scd4x::Scd4x::new(bus.clone(), bus.delay());
                driver.init().map_err(|error| failed(part, error))?;
                driver.start().map_err(|error| failed(part, error))?;
                Probe::Scd4x(driver)
            }
            Part::Opt3001 => {
                let bus = on(at)?;
                let mut driver = opt3001::Opt3001::new(bus.clone(), at, bus.delay());
                driver.init().map_err(|error| failed(part, error))?;
                Probe::Opt3001(driver)
            }
            Part::Ina219 => {
                let bus = on(at)?;
                let mut driver = ina219::Ina219::new(bus.clone(), at, bus.delay());
                driver.init().map_err(|error| failed(part, error))?;
                Probe::Ina219(driver)
            }
            Part::Ina226 => {
                let bus = on(at)?;
                let mut driver = ina226::Ina226::new(bus.clone(), at, bus.delay());
                driver.init().map_err(|error| failed(part, error))?;
                Probe::Ina226(driver)
            }
            Part::Ds18b20 => Probe::Ds18b20(ds18b20::linux::Thermometer::new(
                wiring.serial.as_deref().unwrap_or_default(),
            )),
            Part::Replay => Probe::Replay(wiring.readings.clone().into_iter()),
        };
        Ok(Reader { probe, plan })
    }
}

impl Sensor for Reader {
    type Reading = f32;

    async fn read(&mut self) -> Result<f32> {
        let quantity = self.plan.quantity.as_str();
        let raw = match &mut self.probe {
            Probe::Bme280(driver) => {
                let measurement = driver.read().await?;
                match quantity {
                    "relative_humidity" => measurement.relative_humidity_percent(),
                    "pressure" => measurement.hectopascals(),
                    _ => measurement.celsius(),
                }
            }
            Probe::Bmp280(driver) => {
                let reading = driver.read().await?;
                match quantity {
                    "pressure" => reading.hectopascals(),
                    _ => reading.celsius(),
                }
            }
            Probe::Sht3x(driver) => {
                let measurement = driver.read().await?;
                match quantity {
                    "relative_humidity" => measurement.relative_humidity(),
                    _ => measurement.temperature_celsius(),
                }
            }
            Probe::Hdc1080(driver) => {
                let measurement = driver.read().await?;
                match quantity {
                    "relative_humidity" => measurement.relative_humidity(),
                    _ => measurement.celsius(),
                }
            }
            Probe::Tmp117(driver) => driver.read().await?.celsius(),
            Probe::Scd4x(driver) => {
                let measurement = driver.read().await?;
                match quantity {
                    "co2" => f32::from(measurement.co2_ppm),
                    "relative_humidity" => measurement.relative_humidity_percent(),
                    _ => measurement.celsius(),
                }
            }
            Probe::Opt3001(driver) => driver.read().await?.lux(),
            Probe::Ina219(driver) => driver.read().await?.bus_millivolts() as f32 / 1000.0,
            Probe::Ina226(driver) => driver.read().await?.bus_volts(),
            Probe::Ds18b20(thermometer) => thermometer.read().await?.temperature_celsius(),
            Probe::Replay(readings) => readings.next().ok_or(Error::Closed)?,
        };
        Ok(self.plan.apply(raw))
    }
}

/// The output the profile drives.
enum Output {
    #[cfg(target_os = "linux")]
    Gpio(pamoja_gpio::switch::Switch<pamoja_hal::linux::CdevPin>),
    Print(String),
    Nothing,
}

impl Output {
    /// Takes the wired line, driven to its resting level the moment it is taken.
    fn open(wiring: Option<&OutputWiring>, site: &str) -> std::result::Result<Output, String> {
        match wiring {
            None => Ok(Output::Nothing),
            Some(OutputWiring::Print { label }) => Ok(Output::Print(label.clone())),
            #[cfg(target_os = "linux")]
            Some(OutputWiring::Gpio {
                chip,
                line,
                active_low,
            }) => {
                use pamoja_gpio::pin::Polarity;
                use pamoja_hal::digital::PinState;
                let (resting, polarity) = if *active_low {
                    (PinState::High, Polarity::ActiveLow)
                } else {
                    (PinState::Low, Polarity::ActiveHigh)
                };
                let pin =
                    pamoja_hal::linux::output(chip, *line, &format!("pamoja-node {site}"), resting)
                        .map_err(|error| format!("taking {chip} line {line}: {error}"))?;
                Ok(Output::Gpio(pamoja_gpio::switch::Switch::new(
                    pin, polarity,
                )))
            }
            #[cfg(not(target_os = "linux"))]
            Some(OutputWiring::Gpio { chip, line, .. }) => {
                let _ = site;
                Err(format!(
                    "taking {chip} line {line} needs Linux's GPIO character devices; on this machine the runner drives a `print` output"
                ))
            }
        }
    }

    /// What the output is called in a tick's line, if the node drives one.
    fn label(&self) -> Option<String> {
        match self {
            #[cfg(target_os = "linux")]
            Output::Gpio(_) => Some("output".to_owned()),
            Output::Print(label) => Some(label.clone()),
            Output::Nothing => None,
        }
    }
}

impl Actuator for Output {
    type Command = bool;

    async fn apply(&mut self, on: bool) -> Result<()> {
        match self {
            #[cfg(target_os = "linux")]
            Output::Gpio(switch) => switch.apply(on).await,
            Output::Print(_) | Output::Nothing => {
                let _ = on;
                Ok(())
            }
        }
    }
}

/// The link the readings are published over.
enum Link {
    Mqtt(Box<MqttTransport>),
    Print,
}

impl Link {
    /// Builds the link the wiring names, not yet connected.
    fn open(wiring: &Wiring) -> std::result::Result<Link, String> {
        match &wiring.link {
            LinkWiring::Print => Ok(Link::Print),
            LinkWiring::Mqtt {
                host,
                port,
                client_id,
                username,
                password,
                tls,
            } => {
                let id = client_id.clone().unwrap_or_else(|| wiring.site.clone());
                let mut config = MqttConfig::new(id, host.clone(), *port);
                if let Some(username) = username {
                    config =
                        config.credentials(username.clone(), password.clone().unwrap_or_default());
                }
                if let Some(tls) = tls {
                    let mut settings = match &tls.ca {
                        Some(ca) => Tls::with_ca_pem(read(Path::new(ca))?),
                        None => Tls::system_roots(),
                    };
                    if let (Some(certificate), Some(key)) = (&tls.certificate, &tls.key) {
                        settings = settings.client_certificate(
                            read(Path::new(certificate))?,
                            read(Path::new(key))?,
                        );
                    }
                    config = config.tls(settings);
                }
                Ok(Link::Mqtt(Box::new(MqttTransport::new(config))))
            }
        }
    }

    /// Says goodbye to the broker, so it does not publish a will for a node that stopped
    /// on purpose.
    async fn close(&mut self) {
        if let Link::Mqtt(mqtt) = self {
            let _ = mqtt.disconnect().await;
        }
    }
}

impl Transport for Link {
    async fn connect(&mut self) -> Result<()> {
        match self {
            Link::Mqtt(mqtt) => mqtt.connect().await,
            Link::Print => Ok(()),
        }
    }

    async fn send(&mut self, topic: &str, payload: &[u8]) -> Result<()> {
        match self {
            Link::Mqtt(mqtt) => mqtt.send(topic, payload).await,
            Link::Print => {
                println!("-> {topic} {}", String::from_utf8_lossy(payload));
                Ok(())
            }
        }
    }

    async fn subscribe(&mut self, topic: &str) -> Result<()> {
        match self {
            Link::Mqtt(mqtt) => mqtt.subscribe(topic).await,
            Link::Print => Ok(()),
        }
    }
}

/// The battery the node runs from, read as a state of charge.
enum Battery {
    Mains,
    Ina219(ina219::Ina219<I2cBus, BusDelay>, BatteryWiring),
    Ina226(ina226::Ina226<I2cBus, BusDelay>, BatteryWiring),
}

impl Battery {
    /// Opens the battery's power monitor, if the wiring names one.
    fn open(wiring: Option<&BatteryWiring>) -> std::result::Result<Battery, String> {
        let Some(wiring) = wiring else {
            return Ok(Battery::Mains);
        };
        let part = wiring.part;
        let at = wiring
            .address
            .or(part.default_address())
            .unwrap_or_default();
        let bus = bus(&wiring.bus, part, at)?;
        match part {
            Part::Ina219 => {
                let mut monitor = ina219::Ina219::new(bus.clone(), at, bus.delay());
                monitor.init().map_err(|error| failed(part, error))?;
                Ok(Battery::Ina219(monitor, wiring.clone()))
            }
            _ => {
                let mut monitor = ina226::Ina226::new(bus.clone(), at, bus.delay());
                monitor.init().map_err(|error| failed(part, error))?;
                Ok(Battery::Ina226(monitor, wiring.clone()))
            }
        }
    }

    /// Reads the state of charge; a node on mains is full, and a monitor that does not
    /// answer is taken as critical, the cadence that spares the battery most.
    async fn charge(&mut self) -> f32 {
        let volts = match self {
            Battery::Mains => return 1.0,
            Battery::Ina219(monitor, _) => monitor
                .read()
                .await
                .map(|reading| reading.bus_millivolts() as f32 / 1000.0),
            Battery::Ina226(monitor, _) => monitor.read().await.map(|reading| reading.bus_volts()),
        };
        match (volts, &*self) {
            (Ok(volts), Battery::Ina219(_, wiring) | Battery::Ina226(_, wiring)) => {
                wiring.charge(volts)
            }
            (Err(error), _) => {
                eprintln!("pamoja-node: the battery monitor did not answer, so the node samples as if critical: {error}");
                f32::NAN
            }
            (Ok(_), Battery::Mains) => 1.0,
        }
    }
}
