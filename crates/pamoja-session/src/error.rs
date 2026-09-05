//! The error type for secured sessions.

/// What can go wrong opening a sealed message on a session.
///
/// Sealing never fails, so this only describes the receive side. The two cases are
/// kept distinct on purpose: a forged or corrupted message is an attack or a wire
/// fault, while a replayed message is a captured-and-resent valid message, and an
/// operator may want to react to them differently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionError {
    /// The message did not authenticate: its tag does not match, so it was altered
    /// in flight, was sealed under a different key, or is an outright forgery. The
    /// plaintext is never revealed in this case.
    Inauthentic,
    /// The message's counter has already been seen, or is older than the replay
    /// window still tracks, so it is a replay of a message already accepted (or one
    /// too old to prove is not). It is rejected without revealing the plaintext.
    Replayed,
}

impl core::fmt::Display for SessionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SessionError::Inauthentic => f.write_str("session message failed authentication"),
            SessionError::Replayed => f.write_str("session message is a replay"),
        }
    }
}

// `core::error::Error` rather than `std::error::Error`, so a caller on a
// microcontroller gets the same trait a caller on a gateway does.
impl core::error::Error for SessionError {}
