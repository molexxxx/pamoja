//! A survey of the band, taken by the SX1261 between the gateway's other work.
//!
//! Semtech's packet forwarder runs a thread for this: every few seconds it points the SX1261
//! at the next channel of a run of them, 200 kHz apart, has it count how many samples were
//! at or above each of thirty-three levels, prints the counts, and moves on. It stands aside
//! for a downlink, since the radio has to be free for a carrier check, and it gives up on a
//! scan that has run for two seconds.
//!
//! A [`Sweep`] holds those decisions as data: which channel comes next, whether a scan is due,
//! and whether one has run too long. The program that owns the radio asks it on every poll and
//! does what it says, so the schedule can be tested without a radio and the radio driven
//! without a thread.

use std::time::{Duration, Instant};

use pamoja_radios::sx1302::sx1261::{Level, SCAN_LEVELS};

/// How far apart the scanned channels are, in hertz, which the reference fixes at 200 kHz.
pub const STEP_HZ: u32 = 200_000;

/// The shortest time between two scans. The reference paces its thread no faster than once a
/// second whatever it is configured with.
pub const LEAST_PACE: Duration = Duration::from_secs(1);

/// How long a scan may run before it is abandoned, the reference's two seconds.
pub const TIMEOUT: Duration = Duration::from_secs(2);

/// How many levels a scan counts at, four decibels apart.
pub const LEVELS: usize = SCAN_LEVELS;

/// Where a sweep is between one scan and the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// Between scans, with the next one due at a moment.
    Idle { due: Instant },
    /// A scan is running on one channel.
    Running { since: Instant, hertz: u32 },
}

/// The channels a gateway surveys and the pace it surveys them at.
///
/// A sweep starts at one carrier and steps up [`STEP_HZ`] at a time for as many channels as it
/// was given, then starts over. Every scan is asked to take the same number of samples.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sweep {
    start_hz: u32,
    channels: u8,
    samples: u16,
    pace: Duration,
    next: u8,
    state: State,
}

impl Sweep {
    /// Lays out a sweep.
    ///
    /// # Arguments
    ///
    /// * `start_hz` - the first channel's carrier.
    /// * `channels` - how many channels, [`STEP_HZ`] apart, from there.
    /// * `samples` - how many samples each scan takes.
    /// * `pace` - how long to wait between scans, held to at least [`LEAST_PACE`].
    /// * `now` - the moment the sweep is laid out; the first scan is due a pace after it.
    ///
    /// # Returns
    ///
    /// The sweep, idle.
    #[must_use]
    pub fn new(start_hz: u32, channels: u8, samples: u16, pace: Duration, now: Instant) -> Sweep {
        let pace = pace.max(LEAST_PACE);
        Sweep {
            start_hz,
            channels,
            samples,
            pace,
            next: 0,
            state: State::Idle { due: now + pace },
        }
    }

    /// How many samples each scan takes.
    ///
    /// # Returns
    ///
    /// The count the radio is asked for.
    #[must_use]
    pub const fn samples(&self) -> u16 {
        self.samples
    }

    /// The channel the next scan is of.
    ///
    /// # Returns
    ///
    /// Its carrier in hertz.
    #[must_use]
    pub fn next_hz(&self) -> u32 {
        self.start_hz + u32::from(self.next) * STEP_HZ
    }

    /// The channel a running scan is of, if one is running.
    ///
    /// # Returns
    ///
    /// Its carrier, or `None` between scans.
    #[must_use]
    pub const fn running(&self) -> Option<u32> {
        match self.state {
            State::Running { hertz, .. } => Some(hertz),
            State::Idle { .. } => None,
        }
    }

    /// Whether a scan is due to start.
    ///
    /// # Arguments
    ///
    /// * `now` - the moment being asked about.
    ///
    /// # Returns
    ///
    /// The channel to scan, or `None` while a scan is running or the pace has not passed.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::{Duration, Instant};
    /// use pamoja_gateway::daemon::scan::Sweep;
    ///
    /// let start = Instant::now();
    /// let sweep = Sweep::new(867_100_000, 8, 2000, Duration::from_secs(10), start);
    /// assert_eq!(sweep.due(start + Duration::from_secs(9)), None);
    /// assert_eq!(sweep.due(start + Duration::from_secs(10)), Some(867_100_000));
    /// ```
    #[must_use]
    pub fn due(&self, now: Instant) -> Option<u32> {
        match self.state {
            State::Idle { due } if now >= due => Some(self.next_hz()),
            _ => None,
        }
    }

    /// Records that a scan of the due channel was started.
    ///
    /// # Arguments
    ///
    /// * `now` - when it started, from which the timeout is counted.
    pub fn started(&mut self, now: Instant) {
        self.state = State::Running {
            since: now,
            hertz: self.next_hz(),
        };
    }

    /// Whether a running scan has run longer than [`TIMEOUT`].
    ///
    /// # Arguments
    ///
    /// * `now` - the moment being asked about.
    ///
    /// # Returns
    ///
    /// `true` for a scan to abandon; `false` for one still within its time, or none.
    #[must_use]
    pub fn overdue(&self, now: Instant) -> bool {
        match self.state {
            State::Running { since, .. } => now.duration_since(since) >= TIMEOUT,
            State::Idle { .. } => false,
        }
    }

    /// Records that the running scan finished, so the sweep moves to the next channel.
    ///
    /// The channel after the last is the first again, which is how the reference's thread
    /// wraps.
    ///
    /// # Arguments
    ///
    /// * `now` - when it finished; the next scan is due a pace later.
    ///
    /// # Returns
    ///
    /// The channel that was scanned.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::{Duration, Instant};
    /// use pamoja_gateway::daemon::scan::{Sweep, STEP_HZ};
    ///
    /// let start = Instant::now();
    /// let mut sweep = Sweep::new(867_100_000, 2, 2000, Duration::from_secs(1), start);
    /// let mut now = start + Duration::from_secs(1);
    /// sweep.started(now);
    /// assert_eq!(sweep.finished(now), 867_100_000);
    /// now += Duration::from_secs(1);
    /// sweep.started(now);
    /// assert_eq!(sweep.finished(now), 867_100_000 + STEP_HZ);
    /// assert_eq!(sweep.next_hz(), 867_100_000, "and back to the first");
    /// ```
    pub fn finished(&mut self, now: Instant) -> u32 {
        let scanned = self.next_hz();
        self.next = (self.next + 1) % self.channels.max(1);
        self.state = State::Idle {
            due: now + self.pace,
        };
        scanned
    }

    /// Records that the running scan did not finish, or that a scan could not be started, so
    /// the same channel is tried again a pace later.
    ///
    /// # Arguments
    ///
    /// * `now` - when that was decided.
    pub fn stopped(&mut self, now: Instant) {
        self.state = State::Idle {
            due: now + self.pace,
        };
    }
}

/// One line saying what a scan counted, in the shape the reference prints: the channel, then
/// how many samples were at or above each level, strongest first.
///
/// The levels are four decibels apart from the strongest, so it is named once rather than
/// beside every count. The last count is of everything below the lowest level.
///
/// # Arguments
///
/// * `hertz` - the channel that was scanned.
/// * `levels` - what the radio counted, as [`Sx1261::scan_counts`] read it.
///
/// # Returns
///
/// The line, without a newline.
///
/// # Examples
///
/// ```
/// use pamoja_gateway::daemon::scan::line;
/// use pamoja_radios::sx1302::sx1261::{Level, SCAN_LEVELS};
///
/// let mut levels = [Level { dbm: 0, count: 0 }; SCAN_LEVELS];
/// for (at, level) in levels.iter_mut().enumerate() {
///     level.dbm = -4 * at as i16;
///     level.count = 2000 - at as u16;
/// }
/// let text = line(867_100_000, &levels);
/// assert!(text.starts_with("spectral scan 867100000 Hz, from 0 dBm down in 4 dB steps: 2000 1999 1998"));
/// assert!(text.ends_with(" 1968"));
/// ```
///
/// [`Sx1261::scan_counts`]: pamoja_radios::sx1302::Sx1261::scan_counts
#[must_use]
pub fn line(hertz: u32, levels: &[Level]) -> String {
    let top = levels.first().map_or(0, |level| level.dbm);
    let counts: Vec<String> = levels.iter().map(|level| level.count.to_string()).collect();
    format!(
        "spectral scan {hertz} Hz, from {top} dBm down in 4 dB steps: {}",
        counts.join(" ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pace_is_held_to_a_second_as_the_reference_holds_it() {
        let start = Instant::now();
        let sweep = Sweep::new(867_100_000, 8, 2000, Duration::ZERO, start);
        assert_eq!(sweep.due(start), None);
        assert_eq!(sweep.due(start + Duration::from_millis(999)), None);
        assert_eq!(sweep.due(start + LEAST_PACE), Some(867_100_000));
    }

    #[test]
    fn nothing_is_due_while_a_scan_runs_and_it_is_overdue_after_two_seconds() {
        let start = Instant::now();
        let mut sweep = Sweep::new(867_100_000, 8, 2000, Duration::from_secs(1), start);
        let began = start + Duration::from_secs(1);
        sweep.started(began);
        assert_eq!(sweep.running(), Some(867_100_000));
        assert_eq!(sweep.due(began + Duration::from_secs(30)), None);
        assert!(!sweep.overdue(began + Duration::from_millis(1999)));
        assert!(sweep.overdue(began + TIMEOUT));
    }

    #[test]
    fn a_stopped_scan_keeps_its_channel_and_a_finished_one_moves_on() {
        let start = Instant::now();
        let mut sweep = Sweep::new(867_100_000, 3, 2000, Duration::from_secs(1), start);
        let now = start + Duration::from_secs(1);

        sweep.started(now);
        sweep.stopped(now);
        assert_eq!(sweep.running(), None);
        assert_eq!(
            sweep.next_hz(),
            867_100_000,
            "an abandoned channel is tried again"
        );
        assert_eq!(sweep.due(now), None, "a pace later, not at once");
        assert_eq!(sweep.due(now + Duration::from_secs(1)), Some(867_100_000));

        sweep.started(now);
        assert_eq!(sweep.finished(now), 867_100_000);
        assert_eq!(sweep.next_hz(), 867_300_000);
    }

    #[test]
    fn the_sweep_wraps_after_its_last_channel() {
        let start = Instant::now();
        let mut sweep = Sweep::new(920_600_000, 4, 100, Duration::from_secs(1), start);
        let scanned: Vec<u32> = (0..6)
            .map(|_| {
                sweep.started(start);
                sweep.finished(start)
            })
            .collect();
        assert_eq!(
            scanned,
            [
                920_600_000,
                920_800_000,
                921_000_000,
                921_200_000,
                920_600_000,
                920_800_000
            ]
        );
    }

    #[test]
    fn one_channel_is_scanned_over_and_over() {
        let start = Instant::now();
        let mut sweep = Sweep::new(868_100_000, 1, 100, Duration::from_secs(1), start);
        sweep.started(start);
        sweep.finished(start);
        assert_eq!(sweep.next_hz(), 868_100_000);
    }

    #[test]
    fn the_line_counts_every_level_strongest_first() {
        let mut levels = [Level { dbm: 0, count: 0 }; LEVELS];
        for (at, level) in levels.iter_mut().enumerate() {
            level.dbm = -3 - 4 * at as i16;
            level.count = at as u16;
        }
        let text = line(868_100_000, &levels);
        assert!(
            text.starts_with("spectral scan 868100000 Hz, from -3 dBm down in 4 dB steps: 0 1 2 ")
        );
        assert_eq!(text.split(' ').count(), 11 + LEVELS);
    }
}
