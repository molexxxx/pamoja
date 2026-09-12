//! Putting a packet on the air.
//!
//! Transmitting is three things in order: the payload goes into a buffer belonging to one of
//! the two chains, the chip is told how long to wait before it starts, and then a trigger is
//! armed. Which trigger decides when: now, at a counter value, or on the pulse from a GPS.
//!
//! The waiting is the part worth care. A gateway answers a device inside a receive window a
//! second wide, and the chip does not begin radiating the instant it is triggered: it has a
//! front end to settle, a filter to fill, and a modulator to start. So a timed send is
//! programmed early by exactly those delays, and [`start_delay`] works out by how much. Get it
//! wrong and nothing fails anywhere: the packet goes out, the device is no longer listening,
//! and the join silently never completes.
//!
//! Nothing here opens a bus. The buffer address, the delay, and the trigger word are values,
//! so they can be checked against the reference implementation with no hardware present.

use super::register::{Register, TX_TOP_A_BASE, TX_TOP_B_BASE};

/// Where the payload for the first chain is written.
pub const TX_BUFFER_A: u16 = 0x5300;

/// Where the payload for the second chain is written.
pub const TX_BUFFER_B: u16 = 0x5500;

/// The delay the chip starts from, in its own ticks, before the parts below are taken off.
///
/// The reference keeps this as microseconds and multiplies by the thirty-two ticks each one
/// takes, which is why it is not a round number here.
pub const START_DELAY_BASE: u16 = 1500 * 32;

/// How many ticks of the concentrator counter go by in a microsecond.
pub const TICKS_PER_US: u32 = 32;

/// The value the status register reads when a chain is idle.
pub const STATUS_FREE: u8 = 0x80;

/// Which of the two transmit chains a packet goes out on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chain {
    /// The first chain.
    A,
    /// The second chain.
    B,
}

impl Chain {
    /// Where this chain keeps the payload it is about to send.
    ///
    /// # Returns
    ///
    /// The address the first payload byte is written to.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::tx::Chain;
    ///
    /// assert_eq!(Chain::A.buffer(), 0x5300);
    /// assert_eq!(Chain::B.buffer(), 0x5500);
    /// ```
    #[must_use]
    pub const fn buffer(&self) -> u16 {
        match self {
            Chain::A => TX_BUFFER_A,
            Chain::B => TX_BUFFER_B,
        }
    }

    /// The block of registers that belong to this chain.
    ///
    /// # Returns
    ///
    /// The base address the chain's registers are offsets from.
    #[must_use]
    pub const fn base(&self) -> u16 {
        match self {
            Chain::A => TX_TOP_A_BASE,
            Chain::B => TX_TOP_B_BASE,
        }
    }
}

/// Which front end is wired to a chain, since each settles at its own pace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontEnd {
    /// The SX1250, the front end a current gateway pairs with this chip.
    Sx1250,
    /// The SX1255 or SX1257, which the earlier boards used.
    Sx125x,
}

/// What the transmit chain is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TxStatus {
    /// Idle, and ready to be given a packet.
    Free,
    /// Holding a packet, waiting for its trigger.
    Scheduled,
    /// On the air now.
    Emitting,
    /// A value the chip does not document, carried so a caller can report it.
    Unknown(u8),
}

impl TxStatus {
    /// Reads the transmit status register.
    ///
    /// # Arguments
    ///
    /// * `value` - what the status register answered with.
    ///
    /// # Returns
    ///
    /// What the chain is doing.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::tx::TxStatus;
    ///
    /// assert_eq!(TxStatus::of(0x80), TxStatus::Free);
    /// assert_eq!(TxStatus::of(0x91), TxStatus::Scheduled);
    /// assert_eq!(TxStatus::of(0x60), TxStatus::Emitting);
    /// ```
    #[must_use]
    pub const fn of(value: u8) -> TxStatus {
        match value {
            STATUS_FREE => TxStatus::Free,
            0x91 | 0x92 => TxStatus::Scheduled,
            0x30 | 0x50 | 0x60 | 0x70 => TxStatus::Emitting,
            other => TxStatus::Unknown(other),
        }
    }

    /// Whether a chain in this state will take another packet.
    ///
    /// # Returns
    ///
    /// Whether it is idle. A chain that is scheduled or emitting keeps what it holds, so
    /// writing to it now would lose one of the two packets.
    #[must_use]
    pub const fn is_free(&self) -> bool {
        matches!(self, TxStatus::Free)
    }
}

/// When a packet goes out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    /// As soon as the chip is told.
    Immediate,
    /// When the concentrator counter reaches a value, in microseconds.
    At(u32),
    /// On the next pulse from a GPS receiver.
    OnGps,
}

/// A step in sending a packet.
///
/// The order is carried as data, so a driver walks it against a bus and a test walks it
/// against nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Send {
    /// Open the transmit buffer for writing.
    OpenBuffer(Register),
    /// Write the payload, starting here. For frequency shift keying the length goes first.
    WritePayload(u16),
    /// Close the buffer again.
    CloseBuffer(Register),
    /// Set the delay the chip starts early by, as a pair of bytes.
    SetStartDelay(u16),
    /// Program the counter value a timed send goes out at.
    SetTimerTrigger(u32),
    /// Reset a trigger, which the chip needs before it will take a new one.
    ResetTrigger(Register),
    /// Arm it.
    ArmTrigger(Register),
}

/// How far ahead of its window a chain has to be started.
///
/// The chip does not radiate the moment it is triggered. The front end settles, the filter
/// fills, and the modulator starts, so a send meant for a given moment is programmed by that
/// much earlier.
///
/// # Arguments
///
/// * `front_end` - which front end is wired to the chain.
/// * `bandwidth_hz` - the bandwidth the packet goes out at.
/// * `chirp_lowpass` - the filter setting the chain is configured with.
///
/// # Returns
///
/// The delay in the chip's own ticks, or `None` for a bandwidth no front end supports.
/// Anything that is not LoRa needs no delay at all, so a caller sends those with zero.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::tx::{start_delay, FrontEnd};
///
/// // The usual case: an SX1250 at 125 kHz.
/// let delay = start_delay(FrontEnd::Sx1250, 125_000, 6).expect("a supported bandwidth");
/// assert!(delay < 1500 * 32, "the chip starts early, never late");
///
/// // A bandwidth no front end covers has no answer.
/// assert_eq!(start_delay(FrontEnd::Sx1250, 62_500, 6), None);
/// ```
#[must_use]
pub fn start_delay(front_end: FrontEnd, bandwidth_hz: u32, chirp_lowpass: u8) -> Option<u16> {
    let radio = match (front_end, bandwidth_hz) {
        (FrontEnd::Sx1250, 125_000) => 19,
        (FrontEnd::Sx1250, 250_000) => 24,
        (FrontEnd::Sx1250, 500_000) => 21,
        // The earlier front ends take a fixed time, and only 250 kHz adds to it.
        (FrontEnd::Sx125x, 125_000 | 500_000) => 3 * 32 + 4,
        (FrontEnd::Sx125x, 250_000) => 3 * 32 + 4 + 6,
        _ => return None,
    };

    let filter = ((1u32 << chirp_lowpass) - 1) * 1_000_000 / bandwidth_hz;
    let modem = 8 * (32_000_000 / (32 * bandwidth_hz));

    Some(START_DELAY_BASE.wrapping_sub((radio + filter + modem) as u16))
}

/// The counter value a timed send is programmed at.
///
/// The counter runs in the chip's own ticks rather than microseconds, and the send is moved
/// earlier by the start delay so the packet is on the air at the moment asked for.
///
/// # Arguments
///
/// * `at_us` - when the packet should be heard, in microseconds of the counter.
/// * `start_delay` - what [`start_delay`] worked out.
///
/// # Returns
///
/// The value to write across the four trigger bytes.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::tx::trigger_value;
///
/// // A send a whole second out, brought forward by the delay.
/// assert_eq!(trigger_value(1_000_000, 48_000), 1_000_000 * 32 - 48_000);
/// ```
#[must_use]
pub const fn trigger_value(at_us: u32, start_delay: u16) -> u32 {
    at_us
        .wrapping_mul(TICKS_PER_US)
        .wrapping_sub(start_delay as u32)
}

/// Splits a trigger value into the four bytes the chip takes.
///
/// # Arguments
///
/// * `value` - what [`trigger_value`] worked out.
///
/// # Returns
///
/// The bytes, least significant first, which is the order the four registers are written in.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::tx::trigger_bytes;
///
/// assert_eq!(trigger_bytes(0x1234_5678), [0x78, 0x56, 0x34, 0x12]);
/// ```
#[must_use]
pub const fn trigger_bytes(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

/// Splits a start delay into the two bytes the chip takes.
///
/// # Arguments
///
/// * `delay` - what [`start_delay`] worked out.
///
/// # Returns
///
/// The bytes, most significant first, which is the order this one register takes them in.
/// It is the opposite of the trigger, which is easy to get backwards.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::tx::start_delay_bytes;
///
/// assert_eq!(start_delay_bytes(0x1234), [0x12, 0x34]);
/// ```
#[must_use]
pub const fn start_delay_bytes(delay: u16) -> [u8; 2] {
    delay.to_be_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_chain_has_its_own_buffer_and_registers() {
        assert_eq!(Chain::A.buffer(), 0x5300);
        assert_eq!(Chain::B.buffer(), 0x5500);
        assert_ne!(Chain::A.buffer(), Chain::B.buffer());

        // The buffers sit inside the blocks that belong to each chain.
        assert!(Chain::A.buffer() > Chain::A.base());
        assert!(Chain::B.buffer() > Chain::B.base());
        assert!(Chain::A.buffer() < Chain::B.base());
    }

    #[test]
    fn the_status_values_are_the_ones_the_chip_reports() {
        assert_eq!(TxStatus::of(0x80), TxStatus::Free);
        assert!(TxStatus::of(0x80).is_free());

        for scheduled in [0x91, 0x92] {
            assert_eq!(TxStatus::of(scheduled), TxStatus::Scheduled);
            assert!(!TxStatus::of(scheduled).is_free());
        }
        for emitting in [0x30, 0x50, 0x60, 0x70] {
            assert_eq!(TxStatus::of(emitting), TxStatus::Emitting);
            assert!(!TxStatus::of(emitting).is_free());
        }

        // Anything else is carried rather than mistaken for idle.
        assert_eq!(TxStatus::of(0x00), TxStatus::Unknown(0x00));
        assert!(!TxStatus::of(0x00).is_free());
    }

    #[test]
    fn the_chip_always_starts_early_and_never_late() {
        // Whatever the settings, the delay comes off the base rather than adding to it.
        for bandwidth in [125_000, 250_000, 500_000] {
            for lowpass in 0..=8 {
                for front_end in [FrontEnd::Sx1250, FrontEnd::Sx125x] {
                    let delay = start_delay(front_end, bandwidth, lowpass)
                        .expect("these are supported bandwidths");
                    assert!(
                        delay < START_DELAY_BASE,
                        "{front_end:?} {bandwidth} {lowpass}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_front_ends_settle_at_their_own_pace() {
        // The two differ, which is the whole reason the front end is a parameter.
        let modern = start_delay(FrontEnd::Sx1250, 125_000, 6).expect("supported");
        let earlier = start_delay(FrontEnd::Sx125x, 125_000, 6).expect("supported");
        assert_ne!(modern, earlier);

        // The older part takes longer to settle, so it starts earlier still.
        assert!(earlier < modern);
    }

    #[test]
    fn a_bandwidth_no_front_end_covers_has_no_answer() {
        for bandwidth in [7_800, 62_500, 1_000_000] {
            assert_eq!(start_delay(FrontEnd::Sx1250, bandwidth, 6), None);
            assert_eq!(start_delay(FrontEnd::Sx125x, bandwidth, 6), None);
        }
    }

    #[test]
    fn a_wider_filter_takes_longer_to_fill() {
        // The filter term grows with the setting, so the chip has to start earlier.
        let narrow = start_delay(FrontEnd::Sx1250, 125_000, 2).expect("supported");
        let wide = start_delay(FrontEnd::Sx1250, 125_000, 8).expect("supported");
        assert!(wide < narrow, "a wider filter means an earlier start");
    }

    #[test]
    fn a_timed_send_is_programmed_earlier_than_the_moment_asked_for() {
        // The window is counted in microseconds and the chip in ticks.
        let delay = 48_000u16;
        let value = trigger_value(1_000_000, delay);
        assert_eq!(value, 1_000_000 * TICKS_PER_US - u32::from(delay));

        // Which is earlier than the plain conversion would be.
        assert!(value < 1_000_000 * TICKS_PER_US);
    }

    #[test]
    fn the_two_words_go_out_in_opposite_orders() {
        // The trigger is least significant first and the delay most significant first, which
        // is the kind of thing that reads correctly and transmits at the wrong moment.
        assert_eq!(trigger_bytes(0x1234_5678), [0x78, 0x56, 0x34, 0x12]);
        assert_eq!(start_delay_bytes(0x1234), [0x12, 0x34]);

        // Round trips, so neither is quietly reversed.
        let value = 0xdead_beefu32;
        assert_eq!(u32::from_le_bytes(trigger_bytes(value)), value);
        let delay = 0xbeefu16;
        assert_eq!(u16::from_be_bytes(start_delay_bytes(delay)), delay);
    }

    #[test]
    fn a_trigger_that_wraps_the_counter_still_reads_back() {
        // The counter is thirty-two bits and wraps about every thirty-six minutes, so a
        // send near the wrap must not panic or saturate.
        let near_wrap = trigger_value(u32::MAX / TICKS_PER_US, 48_000);
        assert_eq!(
            near_wrap,
            (u32::MAX / TICKS_PER_US)
                .wrapping_mul(TICKS_PER_US)
                .wrapping_sub(48_000)
        );

        // And a moment so early that the delay takes it below zero wraps rather than panics.
        let underflow = trigger_value(0, 48_000);
        assert_eq!(underflow, 0u32.wrapping_sub(48_000));
    }
}
