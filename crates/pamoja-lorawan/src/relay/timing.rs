//! How long a wake-on-radio preamble must be and when it goes out, TS011-1.0.1 section 5.2
//! and appendix 1.
//!
//! A relay wakes once a [`CadPeriodicity`] and needs to hear at least 6 preamble symbols and
//! its [`CadToRx`] symbols after it wakes. An end device that does not know when that is
//! sends a preamble spanning a whole period. Once a WOR ACK has told it when the relay
//! scanned, it predicts the next scan and sends only as much preamble as the two crystals
//! could have drifted since.

use super::ack::{CadPeriodicity, CadToRx, StateSync, XtalAccuracy};

/// The shortest WOR preamble, in symbols, TS011-1.0.1 appendix 1.
pub const MIN_WOR_PREAMBLE_SYMBOLS: u16 = 8;

/// The symbols a relay's radio needs to demodulate, besides its time to switch from
/// detecting to receiving.
const DEMODULATION_SYMBOLS: u64 = 6;

/// The preamble length of an end device that does not know when the relay scans,
/// TS011-1.0.1 section 5.2.
///
/// `floor(CADPeriodicity / TSYMB) + 1 + 6 + CadToRx`. An end device that has never heard
/// from the relay assumes a second between scans and eight symbols to receive.
///
/// # Arguments
///
/// * `cad_periodicity` - how often the relay scans.
/// * `symbol_us` - the symbol time of the WOR frame's data rate, in microseconds.
/// * `cad_to_rx` - how long the relay takes to start receiving.
///
/// # Returns
///
/// The preamble length in symbols, capped at what a radio's sixteen-bit preamble register
/// holds.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::relay::{unsynchronized_preamble_symbols, CadPeriodicity, CadToRx};
///
/// // TS011-1.0.1 appendix 1: SF10 at 125 kHz, 8.192 ms a symbol, a relay scanning every
/// // 500 ms and taking 4 symbols to receive.
/// assert_eq!(
///     unsynchronized_preamble_symbols(CadPeriodicity::Ms500, 8_192, CadToRx::Symbols4),
///     72
/// );
/// ```
#[must_use]
pub fn unsynchronized_preamble_symbols(
    cad_periodicity: CadPeriodicity,
    symbol_us: u64,
    cad_to_rx: CadToRx,
) -> u16 {
    preamble(cad_periodicity.period_us(), symbol_us, cad_to_rx)
}

/// The milliseconds a relay reports from the start of its scan to the end of the WOR
/// preamble, TS011-1.0.1 appendix 1.
///
/// `ceil(TEND - TSCAN - TOA(WOR) + (12 + 4.25) * TSYMB)`, where the 16.25 symbols take the
/// sync word and header off the frame's time on air.
///
/// # Arguments
///
/// * `scan_start_us` - when the scan that detected the frame started.
/// * `wor_end_us` - when the frame finished arriving.
/// * `wor_airtime_us` - the frame's time on air.
/// * `symbol_us` - the symbol time of its data rate.
///
/// # Returns
///
/// The offset in milliseconds, or `None` when it is negative or past the eleven bits a WOR
/// ACK carries.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::relay::t_offset_ms;
///
/// // TS011-1.0.1 appendix 1: detected at 87 654 ms, received by 88 734 ms, 321.536 ms on
/// // air at SF10.
/// assert_eq!(t_offset_ms(87_654_000, 88_734_000, 321_536, 8_192), Some(892));
/// ```
#[must_use]
pub fn t_offset_ms(
    scan_start_us: u64,
    wor_end_us: u64,
    wor_airtime_us: u64,
    symbol_us: u64,
) -> Option<u16> {
    let micros = i128::from(wor_end_us) - i128::from(scan_start_us) - i128::from(wor_airtime_us)
        + i128::from(symbol_us) * 65 / 4;
    if micros < 0 {
        return None;
    }
    let millis = (micros + 999) / 1000;
    u16::try_from(millis)
        .ok()
        .filter(|&offset| offset <= StateSync::MAX_T_OFFSET_MS)
}

/// When a synchronized end device's next WOR frame goes out, and with how long a preamble.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WorSlot {
    /// When to start sending, in microseconds.
    pub start_us: u64,
    /// The preamble length in symbols.
    pub preamble_symbols: u16,
}

/// What an end device knows of a relay's scans once a WOR ACK has arrived, TS011-1.0.1
/// section 3.9 and appendix 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Synchronization {
    /// When the relay scanned the channel the acknowledged frame went out on, `TREF`, in the
    /// device's microseconds.
    pub reference_us: u64,
    /// How often the relay scans.
    pub cad_periodicity: CadPeriodicity,
    /// How accurate its crystal is.
    pub relay_xtal: XtalAccuracy,
    /// How long it takes to start receiving.
    pub cad_to_rx: CadToRx,
}

impl Synchronization {
    /// Works out when the relay scanned from the WOR ACK that answered a frame.
    ///
    /// `TREF = TLAST + PreambleLength * TSYMB - TOFFSET`.
    ///
    /// # Arguments
    ///
    /// * `wor_start_us` - when the acknowledged WOR frame started going out, `TLAST`.
    /// * `preamble_symbols` - its preamble length.
    /// * `symbol_us` - its symbol time.
    /// * `state` - what the acknowledgment said.
    ///
    /// # Returns
    ///
    /// The synchronization, referenced to the channel that frame used.
    #[must_use]
    pub fn from_ack(
        wor_start_us: u64,
        preamble_symbols: u16,
        symbol_us: u64,
        state: &StateSync,
    ) -> Synchronization {
        let preamble_us = u64::from(preamble_symbols) * symbol_us;
        Synchronization {
            reference_us: (wor_start_us + preamble_us)
                .saturating_sub(u64::from(state.t_offset_ms) * 1000),
            cad_periodicity: state.cad_periodicity,
            relay_xtal: state.xtal_accuracy,
            cad_to_rx: state.cad_to_rx,
        }
    }

    /// Picks the next scan to aim a WOR frame at, TS011-1.0.1 appendix 1.
    ///
    /// `TNEXT = N * CADPeriodicity + TREF` with `N = ceil((TNOW - TREF) / CADPeriodicity)`,
    /// `DriftError = (RelayXtalAccuracy + EndDeviceXtalAccuracy) * (TNEXT - TREF) / 10^6`,
    /// `TSTART = TNEXT - DriftError / 2`, moved one period on while that is already past, and
    /// `PreambleLength = max(8, floor(DriftError / TSYMB) + 1 + 6 + CadToRx)`.
    ///
    /// # Arguments
    ///
    /// * `now_us` - the time, `TNOW`.
    /// * `device_xtal_ppm` - the end device's own crystal accuracy.
    /// * `symbol_us` - the symbol time of the WOR frame's data rate.
    /// * `other_channel` - whether the frame goes out on the relay's other channel, which it
    ///   scans half a period apart.
    ///
    /// # Returns
    ///
    /// When to send and how long a preamble, or `None` once the drift exceeds a whole
    /// period, when the device is no longer synchronized and falls back to
    /// [`unsynchronized_preamble_symbols`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::relay::{
    ///     CadPeriodicity, CadToRx, Forward, StateSync, Synchronization, XtalAccuracy,
    /// };
    ///
    /// // TS011-1.0.1 appendix 1: a 133-symbol SF10 frame sent at 1 234 ms, and an
    /// // acknowledgment reporting 892 ms from a 30 ppm relay scanning every 500 ms.
    /// let state = StateSync {
    ///     cad_to_rx: CadToRx::Symbols4,
    ///     forward: Forward::Available,
    ///     relay_data_rate: 3,
    ///     xtal_accuracy: XtalAccuracy::Ppm30,
    ///     cad_periodicity: CadPeriodicity::Ms500,
    ///     t_offset_ms: 892,
    /// };
    /// let sync = Synchronization::from_ack(1_234_000, 133, 8_192, &state);
    /// assert_eq!(sync.reference_us, 1_431_536);
    ///
    /// // A minute later, from a 20 ppm device, the drift is 3 ms.
    /// let slot = sync.next_wor(61_000_000, 20, 8_192, false).expect("still synchronized");
    /// assert_eq!(slot.start_us, 61_430_036);
    /// assert_eq!(slot.preamble_symbols, 11);
    ///
    /// // Ten hours on, the drift passes a whole period.
    /// assert_eq!(sync.next_wor(36_000_000_000, 20, 8_192, false), None);
    /// ```
    #[must_use]
    pub fn next_wor(
        &self,
        now_us: u64,
        device_xtal_ppm: u32,
        symbol_us: u64,
        other_channel: bool,
    ) -> Option<WorSlot> {
        let period = self.cad_periodicity.period_us();
        let reference = if other_channel {
            self.reference_us + period / 2
        } else {
            self.reference_us
        };
        let ppm = u64::from(self.relay_xtal.ppm()) + u64::from(device_xtal_ppm);
        let elapsed = now_us.saturating_sub(reference);
        let mut next = reference + elapsed.div_ceil(period) * period;
        loop {
            let drift = ppm.checked_mul(next - reference)? / 1_000_000;
            if drift > period {
                return None;
            }
            let start = next - drift / 2;
            if start >= now_us {
                return Some(WorSlot {
                    start_us: start,
                    preamble_symbols: preamble(drift, symbol_us, self.cad_to_rx)
                        .max(MIN_WOR_PREAMBLE_SYMBOLS),
                });
            }
            next += period;
        }
    }
}

/// `floor(span / TSYMB) + 1 + 6 + CadToRx`, capped at sixteen bits.
fn preamble(span_us: u64, symbol_us: u64, cad_to_rx: CadToRx) -> u16 {
    let whole = span_us.checked_div(symbol_us).unwrap_or(u64::MAX);
    let symbols = whole
        .saturating_add(1 + DEMODULATION_SYMBOLS)
        .saturating_add(u64::from(cad_to_rx.symbols()));
    u16::try_from(symbols).unwrap_or(u16::MAX)
}
