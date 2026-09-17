//! The timing and counting defaults every region shares.
//!
//! The link layer specification names these and leaves their values to the regional
//! parameters, and RP002-1.0.5 section 3.3 gives one set recommended for all regions. A
//! device that uses different ones has to tell its network out of band when it is
//! commissioned, so these are the values both sides assume unless told otherwise.
//!
//! Times are in microseconds, the unit a link's airtime is counted in.

/// The delay from the end of an uplink to the opening of the first receive window.
pub const RECEIVE_DELAY1_US: u32 = 1_000_000;

/// The delay to the second receive window, which RP002-1.0.5 fixes at one second after the
/// first.
pub const RECEIVE_DELAY2_US: u32 = RECEIVE_DELAY1_US + 1_000_000;

/// The delay from the end of a join request to the first window a join accept may arrive
/// in.
pub const JOIN_ACCEPT_DELAY1_US: u32 = 5_000_000;

/// The delay to the second join accept window.
pub const JOIN_ACCEPT_DELAY2_US: u32 = 6_000_000;

/// How far either side of the nominal opening a receive window may start.
///
/// LoRaWAN 1.0.3 section 3.3.1 opens each window within 20 microseconds of its delay.
pub const RECEIVE_WINDOW_TOLERANCE_US: u32 = 20;

/// How far ahead of the last counter it accepted a receiver follows a sender.
///
/// RP002-1.0.5 lists it for LoRaWAN 1.0.3 and earlier and notes it was removed from 1.0.4.
/// A frame further ahead than this is refused, which is what stops a captured frame being
/// replayed at a counter the sender has not reached yet.
pub const MAX_FCNT_GAP: u32 = 16_384;

/// How many uplinks go unanswered before a device asks the network to say something.
pub const ADR_ACK_LIMIT: u32 = 64;

/// How many more go unanswered before a device steps its data rate down, and between each
/// step after that.
pub const ADR_ACK_DELAY: u32 = 32;

/// The shortest wait before a confirmed uplink with no acknowledgment is sent again.
///
/// RP002-1.0.5 gives the timeout as two seconds either way of one, drawn at random, so two
/// devices that missed the same answer do not retry into each other.
pub const RETRANSMIT_TIMEOUT_MIN_US: u32 = 1_000_000;

/// The longest wait before that retry.
pub const RETRANSMIT_TIMEOUT_MAX_US: u32 = 3_000_000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_values_are_the_ones_the_regional_parameters_print() {
        // RP002-1.0.5 section 3.3, Default Settings: RECEIVE_DELAY1 1s, RECEIVE_DELAY2 2s
        // (RECEIVE_DELAY1 + 1s), JOIN_ACCEPT_DELAY1 5s, JOIN_ACCEPT_DELAY2 6s, MAX_FCNT_GAP
        // 16384, ADR_ACK_LIMIT 64, ADR_ACK_DELAY 32, RETRANSMIT_TIMEOUT 2s +/- 1s.
        assert_eq!(RECEIVE_DELAY1_US, 1_000_000);
        assert_eq!(RECEIVE_DELAY2_US, 2_000_000);
        assert_eq!(JOIN_ACCEPT_DELAY1_US, 5_000_000);
        assert_eq!(JOIN_ACCEPT_DELAY2_US, 6_000_000);
        assert_eq!(MAX_FCNT_GAP, 16_384);
        assert_eq!(ADR_ACK_LIMIT, 64);
        assert_eq!(ADR_ACK_DELAY, 32);
        assert_eq!(RETRANSMIT_TIMEOUT_MIN_US, 1_000_000);
        assert_eq!(RETRANSMIT_TIMEOUT_MAX_US, 3_000_000);

        // LoRaWAN 1.0.3 section 3.3.1: +/- 20 microseconds.
        assert_eq!(RECEIVE_WINDOW_TOLERANCE_US, 20);
    }
}
