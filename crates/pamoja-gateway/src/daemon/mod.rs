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

use pamoja_radios::sx1302::firmware::Mcu;
use pamoja_radios::sx1302::tx::{Chain, FrontEnd};

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

#[cfg(test)]
mod tests {
    use super::*;

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
