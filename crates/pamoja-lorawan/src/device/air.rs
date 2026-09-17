//! When a device may next transmit, and a device-specific random sequence.
//!
//! Three limits stack. The region's sub-bands each allow a share of the air, which a device
//! keeps by staying silent in that sub-band for a while after each transmission. A network
//! can cap the device's share over every sub-band at once with `DutyCycleReq`. And a join
//! request, which a whole fleet may start sending at once after an outage, is held to the
//! back-off of TS001-1.0.4 section 7.

/// The most sub-bands a plan can describe and still have each one's off time kept.
pub(crate) const MAX_SUB_BANDS: usize = 8;

const HOUR_US: u64 = 3_600_000_000;

/// TS001-1.0.4 table 56: 36 seconds of join airtime in the first hour.
const JOIN_FIRST_HOUR_US: u64 = 36_000_000;
/// 36 seconds over the ten hours after that.
const JOIN_NEXT_TEN_HOURS_US: u64 = 36_000_000;
/// And 8.7 seconds in every day after the first eleven hours.
const JOIN_PER_DAY_US: u64 = 8_700_000;

/// The off times a device is keeping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Air {
    sub_band_free_at_us: [u64; MAX_SUB_BANDS],
    aggregated_free_at_us: u64,
    join_first_us: Option<u64>,
    join_period: u64,
    join_used_us: u64,
}

impl Air {
    pub(crate) const fn new() -> Air {
        Air {
            sub_band_free_at_us: [0; MAX_SUB_BANDS],
            aggregated_free_at_us: 0,
            join_first_us: None,
            join_period: 0,
            join_used_us: 0,
        }
    }

    /// When a sub-band is next free, or zero for one that is free now or never limited.
    pub(crate) fn sub_band_free_at(&self, sub_band: Option<usize>) -> u64 {
        sub_band
            .and_then(|index| self.sub_band_free_at_us.get(index))
            .copied()
            .unwrap_or(0)
    }

    /// When the network's aggregated limit next allows a transmission.
    pub(crate) const fn aggregated_free_at(&self) -> u64 {
        self.aggregated_free_at_us
    }

    /// Keeps the silence a transmission costs.
    ///
    /// A share of `permille` over a transmission of `airtime_us` means waiting
    /// `airtime * (1000 - permille) / permille` after it ends. The aggregated share of
    /// `DutyCycleReq` is one over two to `max_duty_cycle`, so its wait is
    /// `airtime * (2^n - 1)`, from TS001-1.0.4 section 5.3.
    pub(crate) fn transmitted(
        &mut self,
        started_us: u64,
        airtime_us: u64,
        sub_band: Option<(usize, u32)>,
        max_duty_cycle: u8,
    ) {
        let ended_us = started_us.saturating_add(airtime_us);
        if let Some((index, permille)) = sub_band {
            if let Some(slot) = self.sub_band_free_at_us.get_mut(index) {
                if permille > 0 && permille < 1000 {
                    let off =
                        airtime_us.saturating_mul(u64::from(1000 - permille)) / u64::from(permille);
                    *slot = (*slot).max(ended_us.saturating_add(off));
                }
            }
        }
        if max_duty_cycle > 0 {
            let factor = (1u64 << max_duty_cycle.min(15)) - 1;
            let off = airtime_us.saturating_mul(factor);
            self.aggregated_free_at_us =
                self.aggregated_free_at_us.max(ended_us.saturating_add(off));
        }
    }

    /// Checks a join request of `airtime_us` against the back-off budget at `now_us`.
    ///
    /// # Returns
    ///
    /// `Ok` when it fits, or `Err(until)` with the start of the period in which it would.
    pub(crate) fn join_allowed(&mut self, now_us: u64, airtime_us: u64) -> Result<(), u64> {
        let first = *self.join_first_us.get_or_insert(now_us);
        let (period, budget, ends_us) = join_period(now_us.saturating_sub(first));
        if period != self.join_period {
            self.join_period = period;
            self.join_used_us = 0;
        }
        if self.join_used_us.saturating_add(airtime_us) > budget {
            return Err(first.saturating_add(ends_us));
        }
        Ok(())
    }

    /// Counts a join request's airtime against the period it went out in.
    pub(crate) fn joined_air(&mut self, airtime_us: u64) {
        self.join_used_us = self.join_used_us.saturating_add(airtime_us);
    }

    /// Forgets the join back-off, once a join has been accepted.
    pub(crate) fn join_done(&mut self) {
        self.join_first_us = None;
        self.join_period = 0;
        self.join_used_us = 0;
    }
}

/// Which join period an offset from the first join falls in, its budget, and when it ends.
///
/// Period 0 is the first hour, period 1 the ten hours after, and period 2 + N the Nth day
/// after the first eleven hours.
fn join_period(since_first_us: u64) -> (u64, u64, u64) {
    if since_first_us < HOUR_US {
        (0, JOIN_FIRST_HOUR_US, HOUR_US)
    } else if since_first_us < 11 * HOUR_US {
        (1, JOIN_NEXT_TEN_HOURS_US, 11 * HOUR_US)
    } else {
        let day = (since_first_us - 11 * HOUR_US) / (24 * HOUR_US);
        (
            2 + day,
            JOIN_PER_DAY_US,
            11 * HOUR_US + (day + 1) * 24 * HOUR_US,
        )
    }
}

/// A random sequence of its own for each device.
///
/// TS001-1.0.4 section 7 asks that retries follow "a different sequence for every
/// end-device", and channel choice benefits the same way. This is xorshift32, which needs
/// nothing but a nonzero seed and is not meant to be unpredictable, only uncorrelated
/// between devices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Sequence {
    state: u32,
}

impl Sequence {
    /// Seeds from a caller's value and the device's identifier, so two devices given the
    /// same seed still differ.
    pub(crate) fn new(seed: u32, dev_eui: &[u8; 8]) -> Sequence {
        let mut state = seed ^ 0x9E37_79B9;
        for chunk in dev_eui.chunks(4) {
            let mut word = [0u8; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            state = state.rotate_left(13) ^ u32::from_le_bytes(word);
        }
        Sequence {
            state: if state == 0 { 0x2545_F491 } else { state },
        }
    }

    pub(crate) fn next(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// A value from `0` to `bound - 1`, or zero when `bound` is zero.
    pub(crate) fn below(&mut self, bound: u32) -> u32 {
        if bound == 0 {
            0
        } else {
            self.next() % bound
        }
    }

    /// A value from `low` to `high` inclusive.
    pub(crate) fn between(&mut self, low: u32, high: u32) -> u32 {
        low + self.below(high.saturating_sub(low).saturating_add(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_one_percent_sub_band_costs_ninety_nine_times_the_airtime() {
        let mut air = Air::new();
        air.transmitted(1_000, 100_000, Some((0, 10)), 0);
        assert_eq!(air.sub_band_free_at(Some(0)), 1_000 + 100_000 + 9_900_000);
        assert_eq!(
            air.sub_band_free_at(Some(1)),
            0,
            "other sub-bands are untouched"
        );
        assert_eq!(air.sub_band_free_at(None), 0);
    }

    #[test]
    fn the_aggregated_limit_waits_two_to_the_n_less_one_airtimes() {
        // TS001-1.0.4 section 5.3: Toff = TimeOnAir x (2^MaxDutyCycle - 1).
        let mut air = Air::new();
        air.transmitted(0, 50_000, None, 3);
        assert_eq!(air.aggregated_free_at(), 50_000 + 50_000 * 7);
    }

    #[test]
    fn a_hundred_percent_sub_band_and_no_network_limit_cost_nothing() {
        let mut air = Air::new();
        air.transmitted(0, 50_000, Some((0, 1000)), 0);
        assert_eq!(air.sub_band_free_at(Some(0)), 0);
        assert_eq!(air.aggregated_free_at(), 0);
    }

    #[test]
    fn join_airtime_follows_table_56() {
        let mut air = Air::new();
        let start = 5_000_000;

        // 36 seconds fit in the first hour, and the next request waits for the second.
        assert!(air.join_allowed(start, 36_000_000).is_ok());
        air.joined_air(36_000_000);
        assert_eq!(air.join_allowed(start + 1, 1), Err(start + HOUR_US));

        // The ten hours after share another 36 seconds.
        assert!(air.join_allowed(start + HOUR_US, 30_000_000).is_ok());
        air.joined_air(30_000_000);
        assert_eq!(
            air.join_allowed(start + 2 * HOUR_US, 7_000_000),
            Err(start + 11 * HOUR_US)
        );

        // After eleven hours, 8.7 seconds a day.
        assert!(air.join_allowed(start + 11 * HOUR_US, 8_700_000).is_ok());
        air.joined_air(8_700_000);
        assert_eq!(
            air.join_allowed(start + 12 * HOUR_US, 1),
            Err(start + 35 * HOUR_US)
        );
        assert!(air.join_allowed(start + 35 * HOUR_US, 8_700_000).is_ok());
    }

    #[test]
    fn two_devices_with_the_same_seed_draw_different_sequences() {
        let mut one = Sequence::new(7, &[1, 2, 3, 4, 5, 6, 7, 8]);
        let mut two = Sequence::new(7, &[1, 2, 3, 4, 5, 6, 7, 9]);
        let first: Vec<u32> = (0..8).map(|_| one.next()).collect();
        let second: Vec<u32> = (0..8).map(|_| two.next()).collect();
        assert_ne!(first, second);
        assert!(first.iter().all(|value| *value != 0));
    }

    #[test]
    fn a_bounded_draw_stays_in_its_range() {
        let mut sequence = Sequence::new(0, &[0; 8]);
        for _ in 0..1000 {
            let value = sequence.between(1_000_000, 3_000_000);
            assert!((1_000_000..=3_000_000).contains(&value));
        }
        assert_eq!(sequence.below(0), 0);
    }
}
