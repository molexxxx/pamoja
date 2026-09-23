//! Keeping a rolling window of recent readings.

/// A fixed-capacity window over the most recent `N` readings.
///
/// Many field decisions look not at the latest reading but at the recent run of them: the
/// lowest battery voltage in the last minute, the average flow over the last ten samples,
/// how widely a tank level is bouncing. A [`Window`] keeps the last `N` readings in a ring
/// buffer - no allocation, so it runs on a microcontroller - and reports their spread. It
/// is the base the forecasting helpers build on.
///
/// The population [`variance`](Window::variance) is given directly; the standard deviation
/// is its square root, left to the caller so the type stays dependency-free. Capacity `N`
/// should be at least one; a zero-capacity window simply holds nothing.
/// [`with_capacity`](Window::with_capacity) keeps fewer than `N`, for a window sized at run
/// time.
///
/// A reading that is not a finite number, such as the NaN a failed sensor reports, is not
/// kept, so one bad reading cannot turn the mean into NaN for as long as it stays in the
/// window.
///
/// # Examples
///
/// ```
/// use pamoja_kit::Window;
///
/// // Keep the last four tank-level readings and read their spread.
/// let mut levels = Window::<4>::new();
/// for reading in [40.0, 42.0, 38.0, 41.0] {
///     levels.push(reading);
/// }
/// assert!(levels.is_full());
/// assert_eq!(levels.min(), Some(38.0));
/// assert_eq!(levels.max(), Some(42.0));
/// assert_eq!(levels.range(), Some(4.0));
/// assert_eq!(levels.latest(), Some(41.0));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Window<const N: usize> {
    samples: [f32; N],
    capacity: usize,
    len: usize,
    next: usize,
}

impl<const N: usize> Window<N> {
    /// Creates an empty window with capacity `N`.
    ///
    /// # Returns
    ///
    /// A window holding no readings yet.
    pub fn new() -> Self {
        Self::with_capacity(N)
    }

    /// Creates an empty window that keeps at most `capacity` readings.
    ///
    /// The storage is still `N` readings; this is how a window sized at run time, such as
    /// one a configuration chooses, keeps fewer.
    ///
    /// # Arguments
    ///
    /// * `capacity` - the most readings to keep. One above `N` is taken as `N`.
    ///
    /// # Returns
    ///
    /// A window holding no readings yet.
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
    /// * `reading` - the value to record. One that is not a finite number is not kept.
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

    /// Returns the number of readings currently held, at most the capacity.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the window holds no readings.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns `true` if the window holds as many readings as its capacity.
    pub fn is_full(&self) -> bool {
        self.len == self.capacity
    }

    /// Returns the most readings the window keeps: `N`, or less when made with
    /// [`with_capacity`](Window::with_capacity).
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the most recent reading, or [`None`] if the window is empty.
    pub fn latest(&self) -> Option<f32> {
        if self.len == 0 {
            return None;
        }
        Some(self.samples[(self.next + self.capacity - 1) % self.capacity])
    }

    /// Returns the oldest reading still held, or [`None`] if the window is empty.
    pub fn oldest(&self) -> Option<f32> {
        if self.len == 0 {
            return None;
        }
        let index = if self.is_full() { self.next } else { 0 };
        Some(self.samples[index])
    }

    /// Returns the smallest reading in the window, or [`None`] if it is empty.
    pub fn min(&self) -> Option<f32> {
        self.samples[..self.len].iter().copied().reduce(f32::min)
    }

    /// Returns the largest reading in the window, or [`None`] if it is empty.
    pub fn max(&self) -> Option<f32> {
        self.samples[..self.len].iter().copied().reduce(f32::max)
    }

    /// Returns the spread (largest minus smallest), or [`None`] if the window is empty.
    pub fn range(&self) -> Option<f32> {
        Some(self.max()? - self.min()?)
    }

    /// Returns the mean of the readings, or [`None`] if the window is empty.
    pub fn mean(&self) -> Option<f32> {
        if self.len == 0 {
            return None;
        }
        let sum: f32 = self.samples[..self.len].iter().sum();
        Some(sum / self.len as f32)
    }

    /// Returns the population variance of the readings, or [`None`] if the window is empty.
    ///
    /// # Returns
    ///
    /// The mean squared deviation from the mean. The standard deviation is its square root.
    pub fn variance(&self) -> Option<f32> {
        let mean = self.mean()?;
        let sum_squared: f32 = self.samples[..self.len]
            .iter()
            .map(|reading| {
                let deviation = reading - mean;
                deviation * deviation
            })
            .sum();
        Some(sum_squared / self.len as f32)
    }
}

impl<const N: usize> Default for Window<N> {
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
    fn an_empty_window_has_no_statistics() {
        let window = Window::<3>::new();
        assert!(window.is_empty());
        assert_eq!(window.len(), 0);
        assert_eq!(window.min(), None);
        assert_eq!(window.max(), None);
        assert_eq!(window.mean(), None);
        assert_eq!(window.range(), None);
        assert_eq!(window.variance(), None);
        assert_eq!(window.latest(), None);
        assert_eq!(window.oldest(), None);
    }

    #[test]
    fn fills_to_capacity_then_stays_full() {
        let mut window = Window::<3>::new();
        window.push(1.0);
        assert_eq!(window.len(), 1);
        assert!(!window.is_full());
        window.push(2.0);
        window.push(3.0);
        assert!(window.is_full());
        window.push(4.0);
        assert_eq!(window.len(), 3);
        assert_eq!(window.capacity(), 3);
    }

    #[test]
    fn reports_spread_over_the_held_readings() {
        let mut window = Window::<4>::new();
        for reading in [40.0, 42.0, 38.0, 41.0] {
            window.push(reading);
        }
        assert_eq!(window.min(), Some(38.0));
        assert_eq!(window.max(), Some(42.0));
        assert_eq!(window.range(), Some(4.0));
        assert!(approx(window.mean().unwrap(), 40.25));
    }

    #[test]
    fn variance_matches_the_hand_computed_value() {
        // Readings 2, 4, 6: mean 4, squared deviations 4 + 0 + 4 = 8, over 3 samples.
        let mut window = Window::<3>::new();
        for reading in [2.0, 4.0, 6.0] {
            window.push(reading);
        }
        assert!(approx(window.variance().unwrap(), 8.0 / 3.0));
    }

    #[test]
    fn the_oldest_reading_is_evicted_when_full() {
        let mut window = Window::<3>::new();
        for reading in [10.0, 20.0, 30.0, 40.0] {
            window.push(reading);
        }
        // 10 has been pushed out; the window holds 20, 30, 40.
        assert_eq!(window.oldest(), Some(20.0));
        assert_eq!(window.latest(), Some(40.0));
        assert_eq!(window.min(), Some(20.0));
        assert_eq!(window.max(), Some(40.0));
    }

    #[test]
    fn a_smaller_capacity_keeps_fewer_readings() {
        let mut window = Window::<32>::with_capacity(3);
        for reading in [10.0, 20.0, 30.0, 40.0] {
            window.push(reading);
        }
        assert_eq!(window.capacity(), 3);
        assert!(window.is_full());
        assert_eq!(window.oldest(), Some(20.0));
        assert_eq!(window.latest(), Some(40.0));
        assert_eq!(Window::<4>::with_capacity(9).capacity(), 4);
    }

    #[test]
    fn a_reading_that_is_not_a_number_is_not_kept() {
        let mut window = Window::<4>::new();
        window.push(2.0);
        window.push(f32::NAN);
        window.push(f32::INFINITY);
        window.push(4.0);
        assert_eq!(window.len(), 2);
        assert_eq!(window.mean(), Some(3.0));
        assert_eq!(window.max(), Some(4.0));
    }

    #[test]
    fn latest_and_oldest_track_before_the_window_fills() {
        let mut window = Window::<5>::new();
        window.push(7.0);
        assert_eq!(window.latest(), Some(7.0));
        assert_eq!(window.oldest(), Some(7.0));
        window.push(9.0);
        assert_eq!(window.latest(), Some(9.0));
        assert_eq!(window.oldest(), Some(7.0));
    }
}
