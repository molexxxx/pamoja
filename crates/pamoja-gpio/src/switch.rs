//! The parts that are one line: a switch driven, and a contact read, through a pin.
//!
//! A relay, a solenoid valve, a pump through a transistor, an indicator lamp: each is
//! an output line that is either asserted or not, and whether "asserted" is the high
//! or the low level depends on the board. A button, a float switch, a reed contact, a
//! motion detector: each is an input line read the same way. [`Switch`] and
//! [`Contact`] carry the [`Polarity`] so a caller says `on` and `off` and never
//! inverts by hand, and they implement the core [`Actuator`] and [`Sensor`] traits so
//! a valve takes a profile's `bool` and a float switch feeds a rule directly.

use embedded_hal::digital::{InputPin, OutputPin, PinState};
use pamoja_core::{Actuator, Error, Result, Sensor};

use crate::pin::{Level, Polarity};

fn state(level: Level) -> PinState {
    if level.is_high() {
        PinState::High
    } else {
        PinState::Low
    }
}

fn level(high: bool) -> Level {
    Level::from_bool(high)
}

/// An on/off part on an output line: a relay, a valve, a lamp.
///
/// # Examples
///
/// ```
/// use pamoja_core::Actuator;
/// use pamoja_gpio::pin::Polarity;
/// use pamoja_gpio::switch::Switch;
/// use pamoja_hal::digital::PinState;
/// use pamoja_hal::script::{block_on, PinScript};
///
/// // A relay board whose inputs are pulled low to energize the coil.
/// let mut valve = Switch::new(PinScript::new([]), Polarity::ActiveLow);
/// block_on(valve.apply(true))?;
/// block_on(valve.apply(false))?;
/// assert_eq!(valve.release().driven(), [PinState::Low, PinState::High]);
/// # Ok::<(), pamoja_core::Error>(())
/// ```
#[derive(Debug)]
pub struct Switch<P> {
    pin: P,
    polarity: Polarity,
    asserted: bool,
}

impl<P> Switch<P> {
    /// Wraps an output line, without driving it.
    ///
    /// # Arguments
    ///
    /// * `pin` - the output line.
    /// * `polarity` - which level asserts the part.
    ///
    /// # Returns
    ///
    /// The switch, recorded as not asserted until it is first driven.
    pub fn new(pin: P, polarity: Polarity) -> Switch<P> {
        Switch {
            pin,
            polarity,
            asserted: false,
        }
    }

    /// Wraps a line that asserts high.
    ///
    /// # Arguments
    ///
    /// * `pin` - the output line.
    ///
    /// # Returns
    ///
    /// The switch.
    pub fn active_high(pin: P) -> Switch<P> {
        Switch::new(pin, Polarity::ActiveHigh)
    }

    /// Wraps a line that asserts low, the shape of most relay boards.
    ///
    /// # Arguments
    ///
    /// * `pin` - the output line.
    ///
    /// # Returns
    ///
    /// The switch.
    pub fn active_low(pin: P) -> Switch<P> {
        Switch::new(pin, Polarity::ActiveLow)
    }

    /// Returns the polarity the switch drives with.
    pub fn polarity(&self) -> Polarity {
        self.polarity
    }

    /// Reports whether the part was last driven asserted.
    ///
    /// # Returns
    ///
    /// `true` after a successful `set(true)`, `false` after `set(false)` or before
    /// the line is first driven.
    pub fn is_asserted(&self) -> bool {
        self.asserted
    }

    /// Gives back the line.
    ///
    /// # Returns
    ///
    /// The pin the switch was built from.
    pub fn release(self) -> P {
        self.pin
    }
}

impl<P: OutputPin> Switch<P> {
    /// Drives the part asserted or not, at the level its polarity maps to.
    ///
    /// # Arguments
    ///
    /// * `asserted` - `true` to energize, `false` to release.
    ///
    /// # Errors
    ///
    /// Returns the pin's error if the line cannot be driven; the recorded state is
    /// left unchanged.
    pub fn set(&mut self, asserted: bool) -> core::result::Result<(), P::Error> {
        self.pin.set_state(state(self.polarity.level(asserted)))?;
        self.asserted = asserted;
        Ok(())
    }
}

impl<P> Actuator for Switch<P>
where
    P: OutputPin,
    P::Error: core::fmt::Debug,
{
    type Command = bool;

    async fn apply(&mut self, asserted: bool) -> Result<()> {
        self.set(asserted)
            .map_err(|error| Error::Io(alloc::format!("pin error: {error:?}")))
    }
}

/// An on/off part on an input line: a button, a float switch, a motion detector.
///
/// # Examples
///
/// ```
/// use pamoja_core::Sensor;
/// use pamoja_gpio::pin::Polarity;
/// use pamoja_gpio::switch::Contact;
/// use pamoja_hal::digital::PinState;
/// use pamoja_hal::script::{block_on, PinScript};
///
/// // A float switch pulled up, which closes to ground when the tank is full.
/// let line = PinScript::new([PinState::High, PinState::Low]);
/// let mut full = Contact::new(line, Polarity::ActiveLow);
/// assert!(!block_on(full.read())?);
/// assert!(block_on(full.read())?);
/// # Ok::<(), pamoja_core::Error>(())
/// ```
#[derive(Debug)]
pub struct Contact<P> {
    pin: P,
    polarity: Polarity,
}

impl<P> Contact<P> {
    /// Wraps an input line.
    ///
    /// # Arguments
    ///
    /// * `pin` - the input line.
    /// * `polarity` - which level means the contact is asserted.
    ///
    /// # Returns
    ///
    /// The contact.
    pub fn new(pin: P, polarity: Polarity) -> Contact<P> {
        Contact { pin, polarity }
    }

    /// Wraps a line that reads high when asserted.
    ///
    /// # Arguments
    ///
    /// * `pin` - the input line.
    ///
    /// # Returns
    ///
    /// The contact.
    pub fn active_high(pin: P) -> Contact<P> {
        Contact::new(pin, Polarity::ActiveHigh)
    }

    /// Wraps a line pulled up that a closing contact takes low, the common wiring for
    /// a button or a float switch.
    ///
    /// # Arguments
    ///
    /// * `pin` - the input line.
    ///
    /// # Returns
    ///
    /// The contact.
    pub fn active_low(pin: P) -> Contact<P> {
        Contact::new(pin, Polarity::ActiveLow)
    }

    /// Returns the polarity the contact reads with.
    pub fn polarity(&self) -> Polarity {
        self.polarity
    }

    /// Gives back the line.
    ///
    /// # Returns
    ///
    /// The pin the contact was built from.
    pub fn release(self) -> P {
        self.pin
    }
}

impl<P: InputPin> Contact<P> {
    /// Reads the physical level of the line.
    ///
    /// # Returns
    ///
    /// The level, before the polarity is applied.
    ///
    /// # Errors
    ///
    /// Returns the pin's error if the line cannot be read.
    pub fn level(&mut self) -> core::result::Result<Level, P::Error> {
        self.pin.is_high().map(level)
    }

    /// Reads whether the contact is asserted.
    ///
    /// # Returns
    ///
    /// `true` when the line sits at the polarity's asserted level.
    ///
    /// # Errors
    ///
    /// Returns the pin's error if the line cannot be read.
    pub fn is_asserted(&mut self) -> core::result::Result<bool, P::Error> {
        self.level().map(|level| self.polarity.is_asserted(level))
    }
}

impl<P> Sensor for Contact<P>
where
    P: InputPin,
    P::Error: core::fmt::Debug,
{
    type Reading = bool;

    async fn read(&mut self) -> Result<bool> {
        self.is_asserted()
            .map_err(|error| Error::Io(alloc::format!("pin error: {error:?}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pamoja_hal::script::{block_on, PinScript};

    #[test]
    fn a_switch_drives_the_level_its_polarity_maps_to_and_remembers_it() {
        let mut lamp = Switch::active_high(PinScript::new([]));
        assert!(!lamp.is_asserted());
        lamp.set(true).unwrap();
        assert!(lamp.is_asserted());
        lamp.set(false).unwrap();
        assert_eq!(lamp.release().driven(), [PinState::High, PinState::Low]);

        let mut relay = Switch::active_low(PinScript::new([]));
        relay.set(true).unwrap();
        assert_eq!(relay.polarity(), Polarity::ActiveLow);
        assert_eq!(relay.release().driven(), [PinState::Low]);
    }

    #[test]
    fn a_switch_is_an_actuator_taking_a_bool() {
        let mut valve = Switch::new(PinScript::new([]), Polarity::ActiveLow);
        block_on(valve.apply(true)).unwrap();
        assert!(valve.is_asserted());
        block_on(valve.apply(false)).unwrap();
        assert_eq!(valve.release().driven(), [PinState::Low, PinState::High]);
    }

    #[test]
    fn a_contact_applies_its_polarity_to_what_it_reads() {
        let line = PinScript::new([PinState::High, PinState::Low, PinState::Low]);
        let mut button = Contact::active_low(line);
        assert!(!button.is_asserted().unwrap());
        assert!(button.is_asserted().unwrap());
        assert_eq!(button.level().unwrap(), Level::Low);
        assert_eq!(button.polarity(), Polarity::ActiveLow);

        let mut motion = Contact::active_high(PinScript::new([PinState::High]));
        assert!(block_on(motion.read()).unwrap());
        assert!(motion.release().driven().is_empty());
    }
}
