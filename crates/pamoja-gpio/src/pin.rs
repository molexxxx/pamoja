//! The GPIO pin model: levels, pull and drive configuration, interrupt edges, and active
//! polarity.
//!
//! A GPIO pin is the simplest interface on a board: one line that is either high or low.
//! The logic that still has to be right is the meaning of that level. A button wired to
//! ground through a pull-up reads low when pressed; a relay board sold as "active low"
//! switches on when its input is driven low. Treating "pressed" or "on" as if it always
//! meant a high level is a classic inversion bug. This module carries the small set of
//! GPIO concepts so that mapping is written down once rather than scattered through call
//! sites.

/// The physical voltage level on a pin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// A low level, near ground.
    Low,
    /// A high level, near the supply voltage.
    High,
}

impl Level {
    /// Returns the opposite level.
    pub fn inverted(self) -> Level {
        match self {
            Level::Low => Level::High,
            Level::High => Level::Low,
        }
    }

    /// Returns `true` if this is [`High`](Level::High).
    pub fn is_high(self) -> bool {
        matches!(self, Level::High)
    }

    /// Returns `true` if this is [`Low`](Level::Low).
    pub fn is_low(self) -> bool {
        matches!(self, Level::Low)
    }

    /// Returns the level a boolean names.
    ///
    /// # Arguments
    ///
    /// * `high` - `true` for [`High`](Level::High), `false` for [`Low`](Level::Low).
    pub fn from_bool(high: bool) -> Level {
        if high {
            Level::High
        } else {
            Level::Low
        }
    }
}

/// Whether a pin reads its line (input) or drives it (output).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// The pin reads the level on its line.
    Input,
    /// The pin drives the level on its line.
    Output,
}

/// The internal pull resistor applied to an input pin.
///
/// A floating input drifts and reads noise, so a pin reading a switch needs a defined
/// resting level from a pull resistor (internal where the chip offers one, external
/// otherwise).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pull {
    /// No internal pull; the line floats unless something external holds it.
    None,
    /// An internal pull-up holds the line high when nothing drives it.
    Up,
    /// An internal pull-down holds the line low when nothing drives it.
    Down,
}

/// How an output pin drives its two states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drive {
    /// Push-pull: the pin actively drives both high and low.
    PushPull,
    /// Open-drain: the pin actively drives low and floats when high, so an external
    /// pull-up sets the high level. This is what shared, multi-device lines like I2C use.
    OpenDrain,
}

/// The signal transition that triggers a pin interrupt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    /// A low-to-high transition.
    Rising,
    /// A high-to-low transition.
    Falling,
    /// Either transition.
    Both,
}

impl Edge {
    /// Returns `true` if a change from `from` to `to` is an edge this trigger fires on.
    ///
    /// # Arguments
    ///
    /// * `from` - the level before the change.
    /// * `to` - the level after the change.
    ///
    /// # Returns
    ///
    /// `true` if the transition matches this trigger; `false` for the other direction or
    /// for no change at all.
    pub fn triggered_by(self, from: Level, to: Level) -> bool {
        match (from, to) {
            (Level::Low, Level::High) => matches!(self, Edge::Rising | Edge::Both),
            (Level::High, Level::Low) => matches!(self, Edge::Falling | Edge::Both),
            _ => false,
        }
    }
}

/// Whether a signal is asserted by a high or a low physical level.
///
/// Active-low wiring is everywhere in cheap hardware: a button to ground with a pull-up
/// reads [`Level::Low`] when pressed, and many relay boards energize when their input is
/// driven low. This type maps between the logical idea of "asserted" and the physical
/// [`Level`] so the mapping lives in one place instead of in scattered inversions.
///
/// # Examples
///
/// ```
/// use pamoja_gpio::pin::{Level, Polarity};
///
/// // An active-low relay: asserting it (switching the relay on) drives the pin low.
/// let relay = Polarity::ActiveLow;
/// assert_eq!(relay.level(true), Level::Low);
/// assert_eq!(relay.level(false), Level::High);
/// assert!(relay.is_asserted(Level::Low));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Polarity {
    /// A high level means asserted (the direct mapping).
    ActiveHigh,
    /// A low level means asserted (the inverted mapping).
    ActiveLow,
}

impl Polarity {
    /// Returns the physical level for a logical state.
    ///
    /// # Arguments
    ///
    /// * `asserted` - whether the signal should be asserted.
    ///
    /// # Returns
    ///
    /// The [`Level`] that represents that state under this polarity.
    pub fn level(self, asserted: bool) -> Level {
        match self {
            Polarity::ActiveHigh => Level::from_bool(asserted),
            Polarity::ActiveLow => Level::from_bool(!asserted),
        }
    }

    /// Returns whether a physical level means the signal is asserted.
    ///
    /// # Arguments
    ///
    /// * `level` - the level read on the pin.
    ///
    /// # Returns
    ///
    /// `true` if `level` asserts the signal under this polarity.
    pub fn is_asserted(self, level: Level) -> bool {
        match self {
            Polarity::ActiveHigh => level.is_high(),
            Polarity::ActiveLow => level.is_low(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_helpers() {
        assert_eq!(Level::Low.inverted(), Level::High);
        assert_eq!(Level::High.inverted(), Level::Low);
        assert!(Level::High.is_high() && !Level::High.is_low());
        assert_eq!(Level::from_bool(true), Level::High);
        assert_eq!(Level::from_bool(false), Level::Low);
    }

    #[test]
    fn edges_fire_on_the_right_transition() {
        assert!(Edge::Rising.triggered_by(Level::Low, Level::High));
        assert!(!Edge::Rising.triggered_by(Level::High, Level::Low));
        assert!(Edge::Falling.triggered_by(Level::High, Level::Low));
        assert!(!Edge::Falling.triggered_by(Level::Low, Level::High));
        assert!(Edge::Both.triggered_by(Level::Low, Level::High));
        assert!(Edge::Both.triggered_by(Level::High, Level::Low));
        // No transition never fires.
        assert!(!Edge::Both.triggered_by(Level::High, Level::High));
        assert!(!Edge::Rising.triggered_by(Level::Low, Level::Low));
    }

    #[test]
    fn active_high_is_the_direct_mapping() {
        assert_eq!(Polarity::ActiveHigh.level(true), Level::High);
        assert_eq!(Polarity::ActiveHigh.level(false), Level::Low);
        assert!(Polarity::ActiveHigh.is_asserted(Level::High));
        assert!(!Polarity::ActiveHigh.is_asserted(Level::Low));
    }

    #[test]
    fn active_low_inverts() {
        assert_eq!(Polarity::ActiveLow.level(true), Level::Low);
        assert_eq!(Polarity::ActiveLow.level(false), Level::High);
        assert!(Polarity::ActiveLow.is_asserted(Level::Low));
        assert!(!Polarity::ActiveLow.is_asserted(Level::High));
    }

    #[test]
    fn level_and_is_asserted_are_inverses() {
        for polarity in [Polarity::ActiveHigh, Polarity::ActiveLow] {
            for asserted in [true, false] {
                assert_eq!(polarity.is_asserted(polarity.level(asserted)), asserted);
            }
        }
    }
}
