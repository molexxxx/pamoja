//! Running a gateway on a concentrator.
//!
//! A gateway is a concentrator with an uplink. Bringing the concentrator up is a fixed
//! sequence and getting it out of order leaves a board that answers every register, passes
//! every check and hears nothing: the receivers have to know their channels before they are
//! switched on, the clock has to move to a front end that is listening, and the two
//! microcontrollers have to be configured after their firmware is loaded rather than before.
//!
//! That sequence is carried here as data, so it can be read, tested and explained without a
//! bus. The program that walks it lives beside this module and owns the sockets, the files
//! and the clock.

pub mod config;
pub mod forward;

pub use config::{Config, ConfigError, Upstream};

use std::path::Path;

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::SpiDevice;

use pamoja_radios::sx1302::channel::Plan;
use pamoja_radios::sx1302::chip::Model;
use pamoja_radios::sx1302::firmware::{self, LoadError, Mcu, FIRMWARE_LEN};
use pamoja_radios::sx1302::tx::{Chain, FrontEnd};
use pamoja_radios::sx1302::{ConcentratorError, Sx1302};

/// A step in bringing a concentrator up.
///
/// Every step is one call on the driver. The order is the reference order, and each step
/// says what it is for rather than what it writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bring {
    /// Pulse the reset line, which the chip does not do for itself.
    Reset,
    /// Read the version register, which says whether anything is answering at all.
    Check,
    /// Read the part number, which says what is answering.
    Identify,
    /// Hold a front end in reset and let it come back.
    ResetFrontEnd(Chain, FrontEnd),
    /// Calibrate a front end over the band its carrier falls in.
    Calibrate(Chain, u32),
    /// Tune a front end and leave it listening.
    Tune(Chain, u32, bool),
    /// Take the concentrator clock from a front end that is now listening.
    Clock(Chain),
    /// Hand the front ends from the host to the gain control.
    Release,
    /// Give the receivers their channels, and switch them on.
    Channels,
    /// Load a microcontroller image.
    Load(Mcu),
    /// Configure the gain control and let it run.
    GainControl(FrontEnd, bool),
    /// Configure the arbiter and let it run.
    Arbiter(u8),
}

/// The order a concentrator is brought up in.
///
/// The front ends come first because the concentrator takes its clock from one of them, and
/// a front end that is not listening does not clock anything. The channels are configured
/// before the microcontrollers are started, because the gain control begins moving gains as
/// soon as it runs and there is no sense in it working on receivers that are not pointed
/// anywhere yet.
///
/// # Arguments
///
/// * `config` - what the gateway was told.
///
/// # Returns
///
/// The steps, in the order a driver takes them.
///
/// # Examples
///
/// ```
/// use pamoja_gateway::daemon::{bring_up, Bring, Config};
/// # let text = r#"{
/// #   "gateway": "b827ebfffe010203",
/// #   "concentrator": {
/// #     "spi": "/dev/spidev0.0",
/// #     "gpio_chip": "/dev/gpiochip0",
/// #     "reset_line": 23,
/// #     "firmware": { "gain_control": "agc.bin", "arbiter": "arb.bin" }
/// #   },
/// #   "radio": { "carrier_hz": 867500000, "channels": [-400000, 0] },
/// #   "upstream": { "forwarder": "router.example.net" }
/// # }"#;
/// let config = Config::parse(text).expect("the configuration is complete");
/// let steps: Vec<Bring> = bring_up(&config).collect();
///
/// // Nothing is asked of the chip before it has been reset and has answered.
/// assert_eq!(steps[0], Bring::Reset);
/// assert_eq!(steps[1], Bring::Check);
///
/// // And the receivers know their channels before either microcontroller runs.
/// let channels = steps.iter().position(|step| *step == Bring::Channels).expect("channels");
/// let gain = steps
///     .iter()
///     .position(|step| matches!(step, Bring::GainControl(_, _)))
///     .expect("the gain control is started");
/// assert!(channels < gain);
/// ```
pub fn bring_up(config: &Config) -> impl Iterator<Item = Bring> + '_ {
    let front_end = config.concentrator.front_end;
    let carrier = config.radio.carrier_hz;
    let single = config.concentrator.single_input;
    let clock = config.concentrator.clock;
    let listening = config.concentrator.listen_before_talk;

    let chains = [Chain::A, Chain::B];
    let radios = chains.into_iter().flat_map(move |chain| {
        [
            Bring::ResetFrontEnd(chain, front_end),
            Bring::Calibrate(chain, carrier),
            Bring::Tune(chain, carrier, single),
        ]
    });

    [Bring::Reset, Bring::Check, Bring::Identify]
        .into_iter()
        .chain(radios)
        .chain([
            Bring::Clock(clock),
            Bring::Release,
            Bring::Channels,
            Bring::Load(Mcu::Agc),
            Bring::Load(Mcu::Arb),
            Bring::GainControl(front_end, listening),
            Bring::Arbiter(config.radio.dual_demodulation),
        ])
}

impl core::fmt::Display for Bring {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Bring::Reset => f.write_str("pulsing the reset line"),
            Bring::Check => f.write_str("reading the version register"),
            Bring::Identify => f.write_str("reading the part number"),
            Bring::ResetFrontEnd(chain, front_end) => {
                write!(
                    f,
                    "resetting the {front_end:?} front end on chain {chain:?}"
                )
            }
            Bring::Calibrate(chain, hertz) => {
                write!(f, "calibrating chain {chain:?} over {hertz} Hz")
            }
            Bring::Tune(chain, hertz, _) => write!(f, "tuning chain {chain:?} to {hertz} Hz"),
            Bring::Clock(chain) => write!(f, "taking the clock from chain {chain:?}"),
            Bring::Release => f.write_str("handing the front ends to the gain control"),
            Bring::Channels => f.write_str("giving the receivers their channels"),
            Bring::Load(mcu) => write!(f, "loading the {} firmware", mcu.name()),
            Bring::GainControl(_, _) => f.write_str("starting the gain control"),
            Bring::Arbiter(_) => f.write_str("starting the arbiter"),
        }
    }
}

/// A bring-up that stopped, and what it was doing.
///
/// A concentrator that fails halfway is not a concentrator that failed: it is one that got
/// as far as a particular step. Carrying the step means an operator is told what was being
/// asked of the board, rather than only what the bus answered.
#[derive(Debug)]
pub struct BringError<E> {
    /// What was being done.
    pub step: Bring,
    /// What the concentrator answered.
    pub error: ConcentratorError<E>,
}

impl<E: core::fmt::Debug> core::fmt::Display for BringError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}: {}", self.step, self.error)
    }
}

impl<E: core::fmt::Debug> std::error::Error for BringError<E> {}

/// Brings a concentrator up, in the order [`bring_up`] gives.
///
/// Every caller that drives a real board walks the same steps, so the order lives here rather
/// than in each program. A step that fails stops the walk, because each one rests on the ones
/// before it: tuning a front end that never came out of reset writes to nothing, and starting
/// the gain control over receivers with no channels leaves a board that hears silence and
/// reports no fault.
///
/// # Arguments
///
/// * `chip` - the concentrator, already open on its bus.
/// * `config` - what the gateway was told.
/// * `plan` - the channels the receivers are given.
/// * `gain_control` - the gain control firmware, which [`image`] reads from a file.
/// * `arbiter` - the arbiter firmware.
///
/// # Returns
///
/// The part number the chip reported on the way through, so a caller can say what answered
/// without asking the bus again.
///
/// # Errors
///
/// Returns [`BringError`] naming the step that stopped and what the concentrator answered.
pub fn walk<SPI, RESET, D>(
    chip: &mut Sx1302<SPI, RESET, D>,
    config: &Config,
    plan: &Plan,
    gain_control: &[u8],
    arbiter: &[u8],
) -> Result<Option<Model>, BringError<SPI::Error>>
where
    SPI: SpiDevice,
    RESET: OutputPin,
    D: DelayNs,
{
    let mut model = None;

    for step in bring_up(config) {
        let outcome = match step {
            Bring::Reset => chip.reset(),
            Bring::Check => chip.check(),
            Bring::Identify => chip.identify().map(|found| model = Some(found)),
            Bring::ResetFrontEnd(chain, front_end) => chip.reset_front_end(chain, front_end),
            Bring::Calibrate(chain, hertz) => chip.calibrate_front_end(chain, hertz),
            Bring::Tune(chain, hertz, single) => chip.setup_front_end(chain, hertz, single),
            Bring::Clock(chain) => chip.select_clock(chain),
            Bring::Release => chip.release_front_ends(),
            Bring::Channels => chip.configure_channels(plan),
            Bring::Load(Mcu::Agc) => chip.load_firmware(Mcu::Agc, gain_control),
            Bring::Load(Mcu::Arb) => chip.load_firmware(Mcu::Arb, arbiter),
            Bring::GainControl(front_end, listening) => {
                chip.start_gain_control(front_end, listening)
            }
            Bring::Arbiter(mask) => chip.start_arbiter(mask),
        };

        outcome.map_err(|error| BringError { step, error })?;
    }

    Ok(model)
}

/// Why a firmware image could not be taken from a file.
#[derive(Debug)]
pub enum ImageError {
    /// The file could not be read at all.
    Unreadable {
        /// The file named.
        path: String,
        /// What the operating system said.
        why: String,
    },
    /// The file holds neither an image nor the source of one.
    Unusable {
        /// The file named.
        path: String,
        /// What is wrong with what it holds.
        why: LoadError,
    },
}

impl core::fmt::Display for ImageError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ImageError::Unreadable { path, why } => write!(f, "{path}: {why}"),
            ImageError::Unusable { path, why } => write!(f, "{path}: {why}"),
        }
    }
}

impl std::error::Error for ImageError {}

/// Reads one microcontroller image, in either form it is distributed in.
///
/// Semtech ships these images as C source rather than as files of bytes, and a gateway
/// usually has whichever form it downloaded. This takes both: a file that is exactly
/// [`FIRMWARE_LEN`] bytes is the image itself, and anything else is read as the source it is
/// written in. Nothing has to be converted before a gateway runs.
///
/// # Arguments
///
/// * `path` - the file to read.
///
/// # Returns
///
/// The image, ready for [`walk`].
///
/// # Errors
///
/// Returns [`ImageError::Unreadable`] if the file cannot be read, and
/// [`ImageError::Unusable`] if what it holds is neither an image nor an array of one.
pub fn image(path: &Path) -> Result<Vec<u8>, ImageError> {
    let named = || path.display().to_string();
    let raw = std::fs::read(path).map_err(|error| ImageError::Unreadable {
        path: named(),
        why: error.to_string(),
    })?;

    if raw.len() == FIRMWARE_LEN {
        return Ok(raw);
    }

    // Anything else is source, which is text. A file of the wrong length that is not text
    // is neither, and is reported by its length rather than by where its bytes stop parsing.
    let text = core::str::from_utf8(&raw).map_err(|_| ImageError::Unusable {
        path: named(),
        why: LoadError::WrongSize { offered: raw.len() },
    })?;

    let mut held = [0u8; FIRMWARE_LEN];
    firmware::read_source(text, &mut held)
        .map_err(|why| ImageError::Unusable { path: named(), why })?;

    Ok(held.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_walk_that_stops_names_the_step_it_stopped_on() {
        use pamoja_hal::script::{DelayLog, PinScript, SpiScript};

        let config = configured();
        let plan = Plan::new(config.radio.carrier_hz, &config.radio.channels);
        let mut chip = Sx1302::new(SpiScript::new([]), PinScript::new([]), DelayLog::new());

        // Nothing is scripted, so the first transfer fails. Resetting the chip is a line and
        // a wait, which needs no bus, so the walk gets that far and stops on the register
        // read after it rather than reporting a board that never started.
        let stopped = walk(&mut chip, &config, &plan, &[], &[]).expect_err("nothing answers");
        assert_eq!(stopped.step, Bring::Check);
        assert!(stopped
            .to_string()
            .starts_with("reading the version register"));
    }

    fn configured() -> Config {
        Config::parse(
            r#"{
              "gateway": "b827ebfffe010203",
              "concentrator": {
                "spi": "/dev/spidev0.0",
                "gpio_chip": "/dev/gpiochip0",
                "reset_line": 23,
                "firmware": { "gain_control": "agc.bin", "arbiter": "arb.bin" }
              },
              "radio": { "carrier_hz": 867500000, "channels": [-400000, 0] },
              "upstream": { "forwarder": "router.example.net" }
            }"#,
        )
        .expect("the configuration is complete")
    }

    #[test]
    fn both_front_ends_are_tuned_before_the_clock_moves_to_one() {
        // The concentrator is clocked by a front end in receive, so moving the clock before
        // one is listening stops the chip.
        let steps: Vec<Bring> = bring_up(&configured()).collect();

        let clock = steps
            .iter()
            .position(|step| matches!(step, Bring::Clock(_)))
            .expect("the clock moves");
        for chain in [Chain::A, Chain::B] {
            let tuned = steps
                .iter()
                .position(|step| matches!(step, Bring::Tune(on, _, _) if *on == chain))
                .expect("both front ends are tuned");
            assert!(tuned < clock, "{chain:?} was tuned after the clock moved");
        }
    }

    #[test]
    fn a_front_end_is_calibrated_before_it_is_tuned() {
        let steps: Vec<Bring> = bring_up(&configured()).collect();

        let calibrated = steps
            .iter()
            .position(|step| matches!(step, Bring::Calibrate(Chain::A, _)))
            .expect("it is calibrated");
        let tuned = steps
            .iter()
            .position(|step| matches!(step, Bring::Tune(Chain::A, _, _)))
            .expect("it is tuned");
        assert!(calibrated < tuned);
    }

    #[test]
    fn each_microcontroller_is_loaded_before_it_is_started() {
        let steps: Vec<Bring> = bring_up(&configured()).collect();

        let loaded_gain = steps
            .iter()
            .position(|step| *step == Bring::Load(Mcu::Agc))
            .expect("the gain control is loaded");
        let started_gain = steps
            .iter()
            .position(|step| matches!(step, Bring::GainControl(_, _)))
            .expect("and started");
        let loaded_arbiter = steps
            .iter()
            .position(|step| *step == Bring::Load(Mcu::Arb))
            .expect("the arbiter is loaded");
        let started_arbiter = steps
            .iter()
            .position(|step| matches!(step, Bring::Arbiter(_)))
            .expect("and started");

        assert!(loaded_gain < started_gain);
        assert!(loaded_arbiter < started_arbiter);
    }

    #[test]
    fn the_host_lets_go_of_the_front_ends_before_the_gain_control_runs() {
        let steps: Vec<Bring> = bring_up(&configured()).collect();

        let released = steps
            .iter()
            .position(|step| *step == Bring::Release)
            .expect("the host lets go");
        let gain = steps
            .iter()
            .position(|step| matches!(step, Bring::GainControl(_, _)))
            .expect("the gain control runs");
        assert!(
            released < gain,
            "the gain control cannot drive what the host holds"
        );
    }
}
