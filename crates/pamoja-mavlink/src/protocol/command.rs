//! The command protocol: send a command and interpret its acknowledgement.
//!
//! A ground station sends a [`CommandLong`](crate::dialect::CommandLong) (or
//! [`CommandInt`](crate::dialect::CommandInt)) and waits for a
//! [`CommandAck`] carrying the same command id. The
//! acknowledgement's result may be
//! [`IN_PROGRESS`](crate::dialect::mav_result::IN_PROGRESS), which means a long-running
//! command is still executing and the sender should keep waiting rather than time out. If no
//! acknowledgement arrives, the command is resent with an incremented `confirmation` count,
//! up to a retry limit, exactly as the protocol prescribes.
//!
//! [`CommandProtocol`] holds that logic: it matches acknowledgements to the command in flight,
//! classifies each into an [`AckOutcome`], and tracks the `confirmation` count and remaining
//! retries. It performs no IO; the caller sends the messages and applies the timeout.

use crate::dialect::{mav_result, CommandAck};

/// What an incoming [`CommandAck`] means for the command in flight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AckOutcome {
    /// The acknowledgement was for a different command; ignore it and keep waiting.
    Unrelated,
    /// The command is still running; keep waiting. The value is the reported progress
    /// percent (`0..=100`), or `255` when the autopilot does not report one.
    InProgress(u8),
    /// The command finished with this [`MAV_RESULT`](crate::dialect::mav_result) value.
    Final(u8),
}

/// Tracks one command awaiting its acknowledgement: which command, the retransmission
/// `confirmation` count, and the retries left.
#[derive(Clone, Copy, Debug)]
pub struct CommandProtocol {
    command: u16,
    confirmation: u8,
    retries_left: u8,
}

impl CommandProtocol {
    /// Starts tracking a command, allowing `max_retries` retransmissions.
    ///
    /// # Arguments
    ///
    /// * `command` - the [`MAV_CMD`](crate::dialect::mav_cmd) id being sent.
    /// * `max_retries` - how many times the command may be resent after a timeout before the
    ///   caller gives up.
    ///
    /// # Returns
    ///
    /// The protocol tracker, with `confirmation` at zero.
    pub fn new(command: u16, max_retries: u8) -> Self {
        CommandProtocol {
            command,
            confirmation: 0,
            retries_left: max_retries,
        }
    }

    /// Returns the command id being tracked.
    ///
    /// # Returns
    ///
    /// The command id.
    pub fn command(&self) -> u16 {
        self.command
    }

    /// Returns the `confirmation` count to stamp on the command being sent.
    ///
    /// It is zero for the first transmission and increments on each retransmission, which is
    /// how an autopilot distinguishes a resend from a new command.
    ///
    /// # Returns
    ///
    /// The current confirmation count.
    pub fn confirmation(&self) -> u8 {
        self.confirmation
    }

    /// Classifies an incoming acknowledgement against the command in flight.
    ///
    /// # Arguments
    ///
    /// * `ack` - the decoded acknowledgement.
    ///
    /// # Returns
    ///
    /// [`AckOutcome::Unrelated`] if the ack is for another command,
    /// [`AckOutcome::InProgress`] if the command is still running, or
    /// [`AckOutcome::Final`] with the result otherwise.
    pub fn on_ack(&self, ack: &CommandAck) -> AckOutcome {
        if ack.command != self.command {
            return AckOutcome::Unrelated;
        }
        if ack.result == mav_result::IN_PROGRESS {
            AckOutcome::InProgress(ack.progress)
        } else {
            AckOutcome::Final(ack.result)
        }
    }

    /// Records a timeout and reports whether the command may be resent.
    ///
    /// On a resend the `confirmation` count is incremented so the next call to
    /// [`confirmation`](Self::confirmation) stamps the new value.
    ///
    /// # Returns
    ///
    /// `Some(confirmation)` with the new count if a retry remains, or [`None`] once the retry
    /// budget is exhausted.
    pub fn on_timeout(&mut self) -> Option<u8> {
        if self.retries_left == 0 {
            return None;
        }
        self.retries_left -= 1;
        self.confirmation = self.confirmation.wrapping_add(1);
        Some(self.confirmation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect::mav_cmd;

    fn ack(command: u16, result: u8, progress: u8) -> CommandAck {
        CommandAck {
            command,
            result,
            progress,
            result_param2: 0,
            target_system: 1,
            target_component: 1,
        }
    }

    #[test]
    fn an_ack_for_another_command_is_unrelated() {
        let protocol = CommandProtocol::new(mav_cmd::COMPONENT_ARM_DISARM, 5);
        let other = ack(mav_cmd::NAV_TAKEOFF, mav_result::ACCEPTED, 0);
        assert_eq!(protocol.on_ack(&other), AckOutcome::Unrelated);
    }

    #[test]
    fn an_accepted_ack_is_final() {
        let protocol = CommandProtocol::new(mav_cmd::COMPONENT_ARM_DISARM, 5);
        let accepted = ack(mav_cmd::COMPONENT_ARM_DISARM, mav_result::ACCEPTED, 0);
        assert_eq!(
            protocol.on_ack(&accepted),
            AckOutcome::Final(mav_result::ACCEPTED)
        );
    }

    #[test]
    fn an_in_progress_ack_keeps_waiting_with_the_progress() {
        let protocol = CommandProtocol::new(mav_cmd::NAV_TAKEOFF, 5);
        let running = ack(mav_cmd::NAV_TAKEOFF, mav_result::IN_PROGRESS, 42);
        assert_eq!(protocol.on_ack(&running), AckOutcome::InProgress(42));
    }

    #[test]
    fn a_timeout_resends_with_an_incremented_confirmation_until_the_budget_runs_out() {
        let mut protocol = CommandProtocol::new(mav_cmd::COMPONENT_ARM_DISARM, 2);
        assert_eq!(protocol.confirmation(), 0);
        assert_eq!(protocol.on_timeout(), Some(1));
        assert_eq!(protocol.confirmation(), 1);
        assert_eq!(protocol.on_timeout(), Some(2));
        assert_eq!(protocol.confirmation(), 2);
        // The retry budget is spent; a further timeout gives up.
        assert_eq!(protocol.on_timeout(), None);
    }
}
