//! A rolling median filter for rejecting spikes.

/// A median filter over the most recent `N` readings.
///
/// A single bad sample - a spike from electrical noise or a flaky contact - drags a mean or
/// an exponential average off course, because it is blended into the result. The median
/// ignores it: one outlier cannot move the middle value of a sorted window. That makes a
/// [`Median`] the right filter when the noise is occasional spikes rather than steady
/// jitter; for steady jitter reach for [`Smoother`](crate::Smoother) instead. Keep the
/// window small and odd (3, 5, 7) so there is a single middle reading; with an even `N` the
/// median is the average of the two middle readings. A wide window rejects longer bursts
/// and lags a real step by half its length, so a filter of 32 passes a new level 16
/// readings late. [`with_capacity`](Median::with_capacity) keeps fewer than `N`.
///
/// A reading that is not a finite number, such as the NaN a failed sensor reports, is not
/// kept.
///
/// # Examples
///
/// ```
/// use pamoja_kit::Median;
///
/// let mut filtered = Median::<5>::new();
/// // A lone spike among steady readings is rejected.
/// for reading in [10.0, 10.0, 99.0, 10.0, 10.0] {
///     filtered.update(reading);
/// }
/// assert_eq!(filtered.median(), Some(10.0));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Median<const N: usize> {
    samples: [f32; N],
    capacity: usize,
    len: usize,
    next: usize,
}

impl<const N: usize> Median<N> {
    /// Creates an empty median filter over a window of `N` readings.
    ///
    /// # Returns
    ///
    /// A filter holding no readings yet.
    pub fn new() -> Self {
        Self::with_capacity(N)
    }

    /// Creates an empty median filter that keeps at most `capacity` readings.
    ///
    /// The storage is still `N` readings; this is how a filter sized at run time keeps
    /// fewer.
    ///
    /// # Arguments
    ///
    /// * `capacity` - the most readings to keep. One above `N` is taken as `N`.
    ///
    /// # Returns
    ///
    /// A filter holding no readings yet.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            samples: [0.0; N],
            capacity: capacity.min(N),
            len: 0,
            next: 0,
        }
    }

    /// Adds a reading and returns the median of the current window.
    ///
    /// # Arguments
    ///
    /// * `reading` - the latest raw reading.
    ///
    /// # Returns
    ///
    /// The median of the readings now in the window. With a zero-length window the reading
    /// passes through unchanged, and so does one that is not a finite number while the
    /// window is empty.
    pub fn update(&mut self, reading: f32) -> f32 {
        self.push(reading);
        self.median().unwrap_or(reading)
    }

    /// Adds a reading to the window, evicting the oldest once it is full.
    ///
    /// # Arguments
    ///
    /// * `reading` - the latest raw reading. One that is not a finite number is not kept.
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

    /// Returns the most readings the filter keeps: `N`, or less when made with
    /// [`with_capacity`](Median::with_capacity).
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the median of the readings in the window, or [`None`] if it is empty.
    ///
    /// # Returns
    ///
    /// The middle reading of the sorted window for an odd count, the average of the two
    /// middle readings for an even count, or [`None`] before any reading.
    pub fn median(&self) -> Option<f32> {
        if self.len == 0 {
            return None;
        }
        let mut sorted = [0.0f32; N];
        sorted[..self.len].copy_from_slice(&self.samples[..self.len]);
        let window = &mut sorted[..self.len];
        window.sort_unstable_by(f32::total_cmp);
        let mid = self.len / 2;
        if self.len % 2 == 1 {
            Some(window[mid])
        } else {
            Some((window[mid - 1] + window[mid]) / 2.0)
        }
    }

    /// Returns the number of readings currently held, at most the capacity.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the filter holds no readings.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<const N: usize> Default for Median<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odd_window_takes_the_middle_value() {
        let mut median = Median::<3>::new();
        median.update(3.0);
        median.update(1.0);
        assert_eq!(median.update(2.0), 2.0); // sorted [1, 2, 3] -> 2
    }

    #[test]
    fn even_window_averages_the_two_middle_values() {
        // The standard convention: the median of {1, 2, 3, 4} is (2 + 3) / 2 = 2.5.
        let mut median = Median::<4>::new();
        for reading in [1.0, 2.0, 3.0, 4.0] {
            median.push(reading);
        }
        assert_eq!(median.median(), Some(2.5));
    }

    #[test]
    fn a_single_spike_is_rejected() {
        let mut median = Median::<5>::new();
        for reading in [10.0, 10.0, 99.0, 10.0, 10.0] {
            median.push(reading);
        }
        assert_eq!(median.median(), Some(10.0));
    }

    #[test]
    fn an_empty_filter_has_no_median() {
        let median = Median::<3>::new();
        assert!(median.is_empty());
        assert_eq!(median.median(), None);
    }

    #[test]
    fn the_window_evicts_oldest_when_full() {
        let mut median = Median::<3>::new();
        for reading in [1.0, 2.0, 3.0, 100.0, 100.0] {
            median.push(reading);
        }
        // The window holds the last three readings, 3, 100, 100: median 100.
        assert_eq!(median.median(), Some(100.0));
        assert_eq!(median.len(), 3);
    }

    #[test]
    fn a_smaller_capacity_follows_a_step_sooner() {
        let mut small = Median::<32>::with_capacity(3);
        let mut wide = Median::<32>::new();
        for _ in 0..10 {
            small.update(10.0);
            wide.update(10.0);
        }
        small.update(20.0);
        wide.update(20.0);
        assert_eq!(small.update(20.0), 20.0);
        assert_eq!(wide.update(20.0), 10.0);
        assert_eq!(small.capacity(), 3);
    }

    #[test]
    fn a_reading_that_is_not_a_number_is_not_kept() {
        let mut median = Median::<3>::new();
        median.update(10.0);
        assert_eq!(median.update(f32::NAN), 10.0);
        assert_eq!(median.update(f32::INFINITY), 10.0);
        assert_eq!(median.len(), 1);
        assert!(Median::<3>::new().update(f32::NAN).is_nan());
    }
}
