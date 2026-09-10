//! Firing once when a reading crosses a line, and not again until it has come back.

/// What a [`Trigger`] reports when a reading changes its state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    /// The reading just crossed the line: the condition became true.
    Set,
    /// The reading just came back past the release band: the condition stopped holding.
    Cleared,
}

/// A threshold with hysteresis that reports the moment it is crossed.
///
/// This is the helper behind "when the tank drops below 20 percent": it holds a line,
/// says once when a reading crosses it, and says once more when the reading has come
/// back far enough to count. The hysteresis is the release band on the far side of the
/// line, so a reading hovering at the threshold cannot fire the trigger over and over.
/// Between the line and the release band the trigger holds its state.
///
/// A [`Thermostat`](crate::Thermostat) answers the same question as a level to hold;
/// a trigger answers it as an event to act on, which is what a rule wants.
///
/// # Examples
///
/// ```
/// use pamoja_kit::{Edge, Trigger};
///
/// // Water when the bed is drier than 30, and stop once it is wetter than 35.
/// let mut dry = Trigger::below(30.0, 5.0);
/// assert_eq!(dry.update(42.0), None); // well above the line
/// assert_eq!(dry.update(28.0), Some(Edge::Set)); // crossed it: fire once
/// assert_eq!(dry.update(33.0), None); // inside the release band: hold
/// assert!(dry.is_set());
/// assert_eq!(dry.update(36.0), Some(Edge::Cleared)); // past the band: clear once
/// assert_eq!(dry.update(20.0), Some(Edge::Set)); // and it can fire again
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trigger {
    threshold: f32,
    hysteresis: f32,
    above: bool,
    set: bool,
}

impl Trigger {
    /// Creates a trigger that fires when a reading rises above a line.
    ///
    /// It sets when the reading exceeds `threshold` and clears once the reading has
    /// fallen below `threshold` less `hysteresis`.
    ///
    /// # Arguments
    ///
    /// * `threshold` - the line a rising reading crosses.
    /// * `hysteresis` - how far below the line the reading must fall to clear; its
    ///   magnitude is used.
    ///
    /// # Returns
    ///
    /// A trigger that starts cleared.
    pub fn above(threshold: f32, hysteresis: f32) -> Self {
        Self {
            threshold,
            hysteresis: magnitude(hysteresis),
            above: true,
            set: false,
        }
    }

    /// Creates a trigger that fires when a reading falls below a line.
    ///
    /// It sets when the reading drops under `threshold` and clears once the reading
    /// has risen above `threshold` plus `hysteresis`.
    ///
    /// # Arguments
    ///
    /// * `threshold` - the line a falling reading crosses.
    /// * `hysteresis` - how far above the line the reading must rise to clear; its
    ///   magnitude is used.
    ///
    /// # Returns
    ///
    /// A trigger that starts cleared.
    pub fn below(threshold: f32, hysteresis: f32) -> Self {
        Self {
            threshold,
            hysteresis: magnitude(hysteresis),
            above: false,
            set: false,
        }
    }

    /// Feeds a reading in and reports whether it changed the trigger's state.
    ///
    /// # Arguments
    ///
    /// * `reading` - the latest measured value.
    ///
    /// # Returns
    ///
    /// [`Edge::Set`] the moment the reading crosses the line, [`Edge::Cleared`] the
    /// moment it comes back past the release band, and `None` while nothing changed.
    pub fn update(&mut self, reading: f32) -> Option<Edge> {
        let crossed = if self.above {
            reading > self.threshold
        } else {
            reading < self.threshold
        };
        let released = if self.above {
            reading < self.threshold - self.hysteresis
        } else {
            reading > self.threshold + self.hysteresis
        };
        if crossed && !self.set {
            self.set = true;
            Some(Edge::Set)
        } else if released && self.set {
            self.set = false;
            Some(Edge::Cleared)
        } else {
            None
        }
    }

    /// Returns whether the condition currently holds.
    ///
    /// # Returns
    ///
    /// `true` from the reading that set the trigger until the one that clears it.
    pub fn is_set(&self) -> bool {
        self.set
    }

    /// Returns the line the trigger watches.
    ///
    /// # Returns
    ///
    /// The threshold it was created with.
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// Returns the release band on the far side of the line.
    ///
    /// # Returns
    ///
    /// The hysteresis it was created with, as a magnitude.
    pub fn hysteresis(&self) -> f32 {
        self.hysteresis
    }

    /// Returns whether the trigger watches a rising reading.
    ///
    /// # Returns
    ///
    /// `true` for [`above`](Trigger::above), `false` for [`below`](Trigger::below).
    pub fn watches_above(&self) -> bool {
        self.above
    }
}

// `f32::abs` lives in `std`, so this `no_std` crate takes the magnitude by hand.
fn magnitude(value: f32) -> f32 {
    if value < 0.0 {
        -value
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_falling_trigger_fires_once_and_clears_past_the_band() {
        let mut dry = Trigger::below(30.0, 5.0);
        assert!(!dry.is_set());
        assert_eq!(dry.update(42.0), None);
        assert_eq!(dry.update(31.0), None); // above the line: nothing yet
        assert_eq!(dry.update(28.0), Some(Edge::Set));
        assert_eq!(dry.update(25.0), None); // still under: no second event
        assert_eq!(dry.update(33.0), None); // in the band: holds
        assert_eq!(dry.update(36.0), Some(Edge::Cleared));
        assert_eq!(dry.update(36.0), None); // still clear: no second event
        assert_eq!(dry.update(29.9), Some(Edge::Set));
    }

    #[test]
    fn a_rising_trigger_watches_the_other_way() {
        let mut hot = Trigger::above(80.0, 3.0);
        assert_eq!(hot.update(80.0), None); // on the line: not over it
        assert_eq!(hot.update(80.1), Some(Edge::Set));
        assert_eq!(hot.update(78.0), None); // inside the band
        assert_eq!(hot.update(76.9), Some(Edge::Cleared));
        assert!(hot.watches_above());
        assert_eq!(hot.threshold(), 80.0);
        assert_eq!(hot.hysteresis(), 3.0);
    }

    #[test]
    fn a_negative_hysteresis_is_taken_by_magnitude() {
        let mut level = Trigger::below(20.0, -2.0);
        assert_eq!(level.hysteresis(), 2.0);
        assert_eq!(level.update(19.0), Some(Edge::Set));
        assert_eq!(level.update(21.0), None);
        assert_eq!(level.update(22.5), Some(Edge::Cleared));
    }

    #[test]
    fn no_hysteresis_still_fires_once_per_crossing() {
        let mut line = Trigger::above(1.0, 0.0);
        assert_eq!(line.update(1.5), Some(Edge::Set));
        assert_eq!(line.update(1.7), None);
        assert_eq!(line.update(1.0), None); // on the line: holds
        assert_eq!(line.update(0.5), Some(Edge::Cleared));
    }
}
