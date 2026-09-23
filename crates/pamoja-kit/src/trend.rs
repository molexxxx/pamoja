//! Measuring whether a value is trending up or down.

/// Tracks the linear trend of the most recent `N` readings.
///
/// Is a value rising or falling, and how fast? A [`Trend`] fits a least-squares straight
/// line to the last `N` readings, taken as evenly spaced in time, and reports its slope in
/// units per sample: positive when rising, negative when falling, near zero when flat.
/// Because the fit uses the whole window, a single noisy reading does not masquerade as a
/// trend the way a bare difference between two samples can. The slope is the ordinary
/// least-squares estimate, the sum of `(x - mean_x) * (y - mean_y)` over the sum of
/// `(x - mean_x)` squared, with `x` taken as the sample index 0, 1, 2, and so on.
/// [`with_capacity`](Trend::with_capacity) keeps fewer than `N`.
///
/// A reading that is not a finite number, such as the NaN a failed sensor reports, is not
/// kept, so it cannot turn the slope into NaN for as long as it stays in the window.
///
/// # Examples
///
/// ```
/// use pamoja_kit::Trend;
///
/// // A tank level falling two units per reading.
/// let mut level = Trend::<4>::new();
/// for reading in [40.0, 38.0, 36.0, 34.0] {
///     level.push(reading);
/// }
/// assert!((level.slope().unwrap() + 2.0).abs() < 1e-4);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Trend<const N: usize> {
    samples: [f32; N],
    capacity: usize,
    len: usize,
    next: usize,
}

impl<const N: usize> Trend<N> {
    /// Creates an empty trend tracker over a window of `N` readings.
    ///
    /// # Returns
    ///
    /// A tracker holding no readings yet.
    pub fn new() -> Self {
        Self::with_capacity(N)
    }

    /// Creates an empty trend tracker that keeps at most `capacity` readings.
    ///
    /// The storage is still `N` readings; this is how a tracker sized at run time keeps
    /// fewer.
    ///
    /// # Arguments
    ///
    /// * `capacity` - the most readings to keep. One above `N` is taken as `N`.
    ///
    /// # Returns
    ///
    /// A tracker holding no readings yet.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            samples: [0.0; N],
            capacity: capacity.min(N),
            len: 0,
            next: 0,
        }
    }

    /// Adds a reading, evicting the oldest once the window is full.
    ///
    /// # Arguments
    ///
    /// * `reading` - the latest reading, taken one sample interval after the previous one.
    ///   One that is not a finite number is not kept.
    pub fn push(&mut self, reading: f32) {
        if self.capacity == 0 || !reading.is_finite() {
            return;
        }
        self.samples[self.next] = reading;
        self.next = (self.next + 1) % self.capacity;
        if self.len < self.capacity {
            self.len += 1;
        }
    }

    /// Returns the most readings the tracker keeps: `N`, or less when made with
    /// [`with_capacity`](Trend::with_capacity).
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the trend slope in units per sample, or [`None`] with fewer than two readings.
    ///
    /// # Returns
    ///
    /// The least-squares slope of the window: positive when rising, negative when falling.
    /// [`None`] until at least two readings are present, since a single point has no slope.
    pub fn slope(&self) -> Option<f32> {
        if self.len < 2 {
            return None;
        }
        let count = self.len as f32;
        let mean_x = (count - 1.0) / 2.0;
        let mut mean_y = 0.0;
        for i in 0..self.len {
            mean_y += self.ordered(i);
        }
        mean_y /= count;
        let mut covariance = 0.0;
        let mut variance_x = 0.0;
        for i in 0..self.len {
            let dx = i as f32 - mean_x;
            covariance += dx * (self.ordered(i) - mean_y);
            variance_x += dx * dx;
        }
        if variance_x == 0.0 {
            return None;
        }
        Some(covariance / variance_x)
    }

    /// Returns the number of readings currently held, at most the capacity.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the tracker holds no readings.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the reading at time position `index`, where `0` is the oldest still held.
    fn ordered(&self, index: usize) -> f32 {
        let physical = if self.len == self.capacity {
            (self.next + index) % self.capacity
        } else {
            index
        };
        self.samples[physical]
    }
}

impl<const N: usize> Default for Trend<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn a_perfect_rising_line_has_its_exact_slope() {
        // y = 2x + 1 at x = 0..=4.
        let mut trend = Trend::<5>::new();
        for reading in [1.0, 3.0, 5.0, 7.0, 9.0] {
            trend.push(reading);
        }
        assert!(approx(trend.slope().unwrap(), 2.0));
    }

    #[test]
    fn a_falling_line_has_a_negative_slope() {
        let mut trend = Trend::<3>::new();
        for reading in [10.0, 8.0, 6.0] {
            trend.push(reading);
        }
        assert!(approx(trend.slope().unwrap(), -2.0));
    }

    #[test]
    fn a_flat_signal_has_zero_slope() {
        let mut trend = Trend::<4>::new();
        for reading in [5.0, 5.0, 5.0, 5.0] {
            trend.push(reading);
        }
        assert!(approx(trend.slope().unwrap(), 0.0));
    }

    #[test]
    fn slope_matches_a_hand_computed_least_squares_fit() {
        // y = [4, 5, 7, 10, 15] at x = 0..=4. mean_x = 2, mean_y = 8.2.
        // sum dx*dy = 8.4 + 3.2 + 0 + 1.8 + 13.6 = 27; sum dx^2 = 10; slope = 2.7.
        let mut trend = Trend::<5>::new();
        for reading in [4.0, 5.0, 7.0, 10.0, 15.0] {
            trend.push(reading);
        }
        assert!(approx(trend.slope().unwrap(), 2.7));
    }

    #[test]
    fn fewer_than_two_readings_has_no_slope() {
        let mut trend = Trend::<3>::new();
        assert_eq!(trend.slope(), None);
        trend.push(5.0);
        assert_eq!(trend.slope(), None);
    }

    #[test]
    fn the_slope_follows_the_window_as_it_slides() {
        // Once it fills, only the last three readings count.
        let mut trend = Trend::<3>::new();
        for reading in [0.0, 0.0, 0.0, 10.0, 20.0, 30.0] {
            trend.push(reading);
        }
        // The window holds 10, 20, 30 in order: slope 10 per sample.
        assert!(approx(trend.slope().unwrap(), 10.0));
    }

    #[test]
    fn a_smaller_capacity_slides_the_same_way() {
        let mut trend = Trend::<32>::with_capacity(3);
        for reading in [0.0, 0.0, 0.0, 10.0, 20.0, 30.0] {
            trend.push(reading);
        }
        assert!(approx(trend.slope().unwrap(), 10.0));
        assert_eq!(trend.capacity(), 3);
    }

    #[test]
    fn a_reading_that_is_not_a_number_is_not_kept() {
        let mut trend = Trend::<4>::new();
        for reading in [10.0, f32::NAN, 8.0, f32::NEG_INFINITY, 6.0] {
            trend.push(reading);
        }
        assert_eq!(trend.len(), 3);
        assert!(approx(trend.slope().unwrap(), -2.0));
    }
}
