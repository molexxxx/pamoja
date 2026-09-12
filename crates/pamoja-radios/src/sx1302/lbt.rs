//! Listening before talking, on the SX1261 beside the concentrator.
//!
//! Some bands forbid transmitting into a channel somebody else is already using. The
//! concentrator cannot check for itself while it is busy receiving, so a gateway that needs
//! this carries a second radio, an SX1261, wired to the same bus behind the concentrator. It
//! samples the channel, compares what it hears against a threshold, and the concentrator asks
//! it for permission before a trigger is armed.
//!
//! That radio takes commands rather than register writes: an opcode byte, then its payload.
//! This module builds those frames and reads the answers back. It opens no bus.
//!
//! Two conversions here are worth stating plainly, because neither is what the numbers look
//! like. A threshold in dBm is written as a doubled negative, so -80 dBm goes out as 160. And
//! a scan time is not a duration the radio counts: it is a number of samples, and only two
//! durations have one, so anything else is refused rather than rounded.

/// The command that starts a carrier check.
pub const OP_LBT_START: u8 = 0x9a;

/// The command that starts a spectral scan.
pub const OP_SPECTRAL_SCAN: u8 = 0x9b;

/// The command that writes one of the radio's own registers.
pub const OP_WRITE_REGISTER: u8 = 0x0d;

/// The command that puts the radio in frequency synthesis mode, which ends a check.
pub const OP_SET_FS: u8 = 0xc1;

/// The register that holds the radio to the carrier check, cleared to stop it.
pub const REG_LBT_ENABLE: u16 = 0x089b;

/// How long the radio waits between the samples it takes, as the radio counts it.
///
/// Eleven is about 8.2 microseconds. Ten and twelve are the neighbouring settings the
/// reference mentions, at roughly 7.68 and 8.68.
pub const SAMPLE_INTERVAL: u8 = 11;

/// Which of the radio's pins reports the answer back to the concentrator.
pub const GPIO_ID: u8 = 1;

/// How many results a spectral scan returns.
pub const SCAN_RESULTS: usize = 33;

/// How long the radio listens for before it answers.
///
/// The radio counts samples rather than time, and only these two durations have a count, so
/// they are the only ones it takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanTime {
    /// A short check, long enough for the channel to be plainly busy or plainly clear.
    Short,
    /// A long check, for a band whose rules ask for one.
    Long,
}

impl ScanTime {
    /// How long this check listens, in microseconds.
    ///
    /// # Returns
    ///
    /// The duration, which is also how long a caller waits before asking for the answer.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::lbt::ScanTime;
    ///
    /// assert_eq!(ScanTime::Short.micros(), 128);
    /// assert_eq!(ScanTime::Long.micros(), 5000);
    /// ```
    #[must_use]
    pub const fn micros(&self) -> u16 {
        match self {
            ScanTime::Short => 128,
            ScanTime::Long => 5000,
        }
    }

    /// How many samples the radio takes to fill that duration.
    ///
    /// # Returns
    ///
    /// The count the radio is programmed with. It is not the duration divided by anything a
    /// caller can work out, so it is carried rather than computed.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::lbt::ScanTime;
    ///
    /// assert_eq!(ScanTime::Short.samples(), 24);
    /// assert_eq!(ScanTime::Long.samples(), 715);
    /// ```
    #[must_use]
    pub const fn samples(&self) -> u16 {
        match self {
            ScanTime::Short => 24,
            ScanTime::Long => 715,
        }
    }

    /// Reads a duration a caller asked for.
    ///
    /// # Arguments
    ///
    /// * `micros` - the duration in microseconds.
    ///
    /// # Returns
    ///
    /// The check that takes that long, or `None` for a duration the radio has no count for.
    /// A near miss is refused rather than rounded, because a check shorter than the rules ask
    /// for is worse than no check at all.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::lbt::ScanTime;
    ///
    /// assert_eq!(ScanTime::of_micros(128), Some(ScanTime::Short));
    /// assert_eq!(ScanTime::of_micros(5000), Some(ScanTime::Long));
    /// assert_eq!(ScanTime::of_micros(4999), None, "a near miss is not rounded");
    /// ```
    #[must_use]
    pub const fn of_micros(micros: u16) -> Option<ScanTime> {
        match micros {
            128 => Some(ScanTime::Short),
            5000 => Some(ScanTime::Long),
            _ => None,
        }
    }
}

/// Writes a threshold the way the radio takes it.
///
/// The radio counts in half decibels below zero, so a threshold in dBm goes out doubled and
/// with its sign dropped: -80 dBm becomes 160.
///
/// # Arguments
///
/// * `dbm` - the threshold, which is negative for any real one.
///
/// # Returns
///
/// The byte the radio is programmed with.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::lbt::threshold_byte;
///
/// assert_eq!(threshold_byte(-80), 160);
/// assert_eq!(threshold_byte(-60), 120);
/// ```
#[must_use]
pub const fn threshold_byte(dbm: i8) -> u8 {
    (dbm as i16 * -2) as u8
}

/// Reads a threshold byte back as decibels.
///
/// # Arguments
///
/// * `byte` - what the radio was programmed with.
///
/// # Returns
///
/// The threshold in dBm.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::lbt::{threshold_byte, threshold_dbm};
///
/// // Every threshold a caller would set survives the round trip.
/// for dbm in -120..0 {
///     assert_eq!(threshold_dbm(threshold_byte(dbm)), dbm);
/// }
/// ```
#[must_use]
pub const fn threshold_dbm(byte: u8) -> i8 {
    -((byte / 2) as i8)
}

/// Builds the command that starts a carrier check.
///
/// # Arguments
///
/// * `scan_time` - how long to listen for.
/// * `threshold_dbm` - the level above which the channel counts as busy.
///
/// # Returns
///
/// The opcode and its payload. A caller sends it, waits
/// [`micros`](ScanTime::micros), and only then asks the concentrator for a trigger.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::lbt::{start, ScanTime, OP_LBT_START};
///
/// let (opcode, payload) = start(ScanTime::Short, -80);
/// assert_eq!(opcode, OP_LBT_START);
/// assert_eq!(payload, [11, 0, 24, 160, 1]);
/// ```
#[must_use]
pub const fn start(scan_time: ScanTime, threshold_dbm: i8) -> (u8, [u8; 5]) {
    let samples = scan_time.samples();
    (
        OP_LBT_START,
        [
            SAMPLE_INTERVAL,
            (samples >> 8) as u8,
            (samples & 0xff) as u8,
            threshold_byte(threshold_dbm),
            GPIO_ID,
        ],
    )
}

/// Builds the two commands that end a carrier check.
///
/// # Returns
///
/// The register write that releases the radio, then the command that parks it. Both go out in
/// this order, and the second takes no payload.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::lbt::{stop, OP_SET_FS, OP_WRITE_REGISTER};
///
/// let ((release_op, release), (park_op, park)) = stop();
/// assert_eq!(release_op, OP_WRITE_REGISTER);
/// assert_eq!(release, [0x08, 0x9b, 0x00], "clear the register that holds the check");
/// assert_eq!(park_op, OP_SET_FS);
/// assert!(park.is_empty());
/// ```
#[must_use]
pub const fn stop() -> ((u8, [u8; 3]), (u8, [u8; 0])) {
    (
        (
            OP_WRITE_REGISTER,
            [
                (REG_LBT_ENABLE >> 8) as u8,
                (REG_LBT_ENABLE & 0xff) as u8,
                0x00,
            ],
        ),
        (OP_SET_FS, []),
    )
}

/// Builds the command that starts a spectral scan.
///
/// # Arguments
///
/// * `scans` - how many sweeps to take.
///
/// # Returns
///
/// The opcode and its payload.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::lbt::{spectral_scan, OP_SPECTRAL_SCAN};
///
/// let (opcode, payload) = spectral_scan(1000);
/// assert_eq!(opcode, OP_SPECTRAL_SCAN);
/// assert_eq!(payload, [0x03, 0xe8, 11]);
/// ```
#[must_use]
pub const fn spectral_scan(scans: u16) -> (u8, [u8; 3]) {
    // The reference fills three bytes here and then asks its bus layer to send nine, which
    // reads past the end of its own buffer. Three is what it actually fills, and what the
    // radio is documented to take, so three is what goes out.
    (
        OP_SPECTRAL_SCAN,
        [(scans >> 8) as u8, (scans & 0xff) as u8, SAMPLE_INTERVAL],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_two_durations_have_a_sample_count() {
        assert_eq!(ScanTime::of_micros(128), Some(ScanTime::Short));
        assert_eq!(ScanTime::of_micros(5000), Some(ScanTime::Long));

        // Anything else is refused. A check shorter than the rules ask for would pass while
        // meaning nothing, which is worse than refusing to run one.
        for micros in [0, 127, 129, 1000, 4999, 5001, u16::MAX] {
            assert_eq!(ScanTime::of_micros(micros), None, "{micros}");
        }
    }

    #[test]
    fn a_duration_and_its_count_belong_together() {
        assert_eq!(ScanTime::Short.micros(), 128);
        assert_eq!(ScanTime::Short.samples(), 24);
        assert_eq!(ScanTime::Long.micros(), 5000);
        assert_eq!(ScanTime::Long.samples(), 715);

        // Every duration reads back as the check it names.
        for scan in [ScanTime::Short, ScanTime::Long] {
            assert_eq!(ScanTime::of_micros(scan.micros()), Some(scan));
        }
    }

    #[test]
    fn a_threshold_goes_out_doubled_and_positive() {
        // The radio counts half decibels below zero, so the sign is dropped on the way out.
        assert_eq!(threshold_byte(-80), 160);
        assert_eq!(threshold_byte(-60), 120);
        assert_eq!(threshold_byte(-1), 2);
        assert_eq!(threshold_byte(0), 0);

        // And every threshold a caller would set survives the round trip.
        for dbm in -127..=0 {
            assert_eq!(threshold_dbm(threshold_byte(dbm)), dbm, "{dbm} dBm");
        }
    }

    #[test]
    fn the_start_command_carries_the_count_not_the_duration() {
        let (opcode, payload) = start(ScanTime::Short, -80);
        assert_eq!(opcode, OP_LBT_START);
        assert_eq!(payload, [SAMPLE_INTERVAL, 0, 24, 160, GPIO_ID]);

        // The long check needs two bytes for its count, which is why it is not one byte wide.
        let (_, payload) = start(ScanTime::Long, -80);
        assert_eq!(payload[1], (715 >> 8) as u8);
        assert_eq!(payload[2], (715 & 0xff) as u8);
        assert_eq!(u16::from_be_bytes([payload[1], payload[2]]), 715);
    }

    #[test]
    fn the_threshold_reaches_the_command_unchanged() {
        for dbm in [-40i8, -70, -80, -90, -110] {
            let (_, payload) = start(ScanTime::Long, dbm);
            assert_eq!(payload[3], threshold_byte(dbm));
            assert_eq!(threshold_dbm(payload[3]), dbm);
        }
    }

    #[test]
    fn stopping_clears_the_register_then_parks_the_radio() {
        let ((release_op, release), (park_op, park)) = stop();

        // The register that holds the check is cleared by address, most significant first.
        assert_eq!(release_op, OP_WRITE_REGISTER);
        assert_eq!(release[0], (REG_LBT_ENABLE >> 8) as u8);
        assert_eq!(release[1], (REG_LBT_ENABLE & 0xff) as u8);
        assert_eq!(release[2], 0x00, "cleared, not set");
        assert_eq!(u16::from_be_bytes([release[0], release[1]]), REG_LBT_ENABLE);

        // Parking takes no payload at all.
        assert_eq!(park_op, OP_SET_FS);
        assert!(park.is_empty());
    }

    #[test]
    fn a_spectral_scan_carries_its_sweep_count() {
        let (opcode, payload) = spectral_scan(1000);
        assert_eq!(opcode, OP_SPECTRAL_SCAN);
        assert_eq!(u16::from_be_bytes([payload[0], payload[1]]), 1000);
        assert_eq!(payload[2], SAMPLE_INTERVAL);

        // The count spans both bytes, so a sweep past 255 is not truncated.
        let (_, payload) = spectral_scan(u16::MAX);
        assert_eq!(u16::from_be_bytes([payload[0], payload[1]]), u16::MAX);
    }

    #[test]
    fn the_commands_do_not_collide() {
        // Four opcodes, all different, none of them zero.
        let opcodes = [OP_LBT_START, OP_SPECTRAL_SCAN, OP_WRITE_REGISTER, OP_SET_FS];
        for (at, opcode) in opcodes.iter().enumerate() {
            assert_ne!(*opcode, 0);
            for other in &opcodes[at + 1..] {
                assert_ne!(opcode, other);
            }
        }
    }
}
