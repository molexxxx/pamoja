//! The MAC answers and requests a device owes its next uplinks.
//!
//! TS001-1.0.4 section 5 has answers sent in the order the commands arrived, and has four of
//! them, the ones that change how a device listens, repeated on every uplink until a Class A
//! downlink shows the network heard them. When a frame cannot hold everything, answers come
//! first, then commands the device starts itself, then the application payload, and a list
//! that still does not fit is cut after the last whole command.

use crate::mac::{MacCommand, MAX_COMMAND};

/// How many answers a device holds for one downlink's worth of commands.
///
/// A frame holds at most 242 bytes of them and most are one or two bytes, so a network that
/// sends more commands than this in one downlink has its surplus executed but unanswered.
pub(crate) const MAX_ANSWERS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    bytes: [u8; MAX_COMMAND],
    len: u8,
    sticky: bool,
}

/// What a device owes the network.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Answers {
    entries: [Entry; MAX_ANSWERS],
    count: usize,
    link_check: bool,
    device_time: bool,
}

impl Answers {
    pub(crate) const fn new() -> Answers {
        Answers {
            entries: [Entry {
                bytes: [0; MAX_COMMAND],
                len: 0,
                sticky: false,
            }; MAX_ANSWERS],
            count: 0,
            link_check: false,
            device_time: false,
        }
    }

    /// Queues an answer, sent once or, when `sticky`, on every uplink until a downlink.
    pub(crate) fn push(&mut self, answer: MacCommand, sticky: bool) {
        if self.count == MAX_ANSWERS {
            return;
        }
        let mut bytes = [0u8; MAX_COMMAND];
        if let Ok(len) = answer.encode(&mut bytes) {
            self.entries[self.count] = Entry {
                bytes,
                len: len as u8,
                sticky,
            };
            self.count += 1;
        }
    }

    /// Each owed answer's bytes and whether it repeats until a downlink, in order.
    pub(crate) fn owed(&self) -> impl Iterator<Item = (&[u8], bool)> + '_ {
        self.entries[..self.count]
            .iter()
            .map(|entry| (&entry.bytes[..usize::from(entry.len)], entry.sticky))
    }

    /// Whether a link check and a time request are still to go out.
    pub(crate) const fn requests(&self) -> (bool, bool) {
        (self.link_check, self.device_time)
    }

    /// Queues an answer already encoded, as a saved state carries it.
    ///
    /// # Returns
    ///
    /// `false` if the queue is full or the answer is longer than a command can be.
    pub(crate) fn push_encoded(&mut self, answer: &[u8], sticky: bool) -> bool {
        if self.count == MAX_ANSWERS || answer.is_empty() || answer.len() > MAX_COMMAND {
            return false;
        }
        let mut bytes = [0u8; MAX_COMMAND];
        bytes[..answer.len()].copy_from_slice(answer);
        self.entries[self.count] = Entry {
            bytes,
            len: answer.len() as u8,
            sticky,
        };
        self.count += 1;
        true
    }

    /// Asks the network for a link check with the next uplink.
    pub(crate) fn request_link_check(&mut self) {
        self.link_check = true;
    }

    /// Asks the network for the time with the next uplink.
    pub(crate) fn request_device_time(&mut self) {
        self.device_time = true;
    }

    /// Clears everything a Class A downlink settles: the answers, sticky ones included.
    ///
    /// Requests the device made stay queued if they have not gone out yet.
    pub(crate) fn heard_downlink(&mut self) {
        self.count = 0;
    }

    /// The bytes every answer and request takes, in order, if nothing were cut.
    pub(crate) fn len(&self) -> usize {
        self.entries[..self.count]
            .iter()
            .map(|entry| usize::from(entry.len))
            .sum::<usize>()
            + usize::from(self.link_check)
            + usize::from(self.device_time)
    }

    /// Writes as many whole answers, then requests, as fit in `out`.
    ///
    /// # Returns
    ///
    /// The bytes written, and whether the two requests went out.
    pub(crate) fn write(&self, out: &mut [u8]) -> (usize, bool, bool) {
        let mut at = 0;
        for entry in &self.entries[..self.count] {
            let len = usize::from(entry.len);
            if at + len > out.len() {
                return (at, false, false);
            }
            out[at..at + len].copy_from_slice(&entry.bytes[..len]);
            at += len;
        }
        let mut sent_link_check = false;
        if self.link_check && at < out.len() {
            out[at] = MacCommand::LinkCheckReq.cid();
            at += 1;
            sent_link_check = true;
        }
        let mut sent_device_time = false;
        if self.device_time && at < out.len() {
            out[at] = MacCommand::DeviceTimeReq.cid();
            at += 1;
            sent_device_time = true;
        }
        (at, sent_link_check, sent_device_time)
    }

    /// Settles what an uplink carried: answers sent once are done, sticky ones stay, and the
    /// requests that went out are no longer owed.
    pub(crate) fn sent(&mut self, link_check: bool, device_time: bool) {
        let mut kept = 0;
        for index in 0..self.count {
            if self.entries[index].sticky {
                self.entries[kept] = self.entries[index];
                kept += 1;
            }
        }
        self.count = kept;
        if link_check {
            self.link_check = false;
        }
        if device_time {
            self.device_time = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_keep_their_order_and_sticky_ones_stay() {
        let mut answers = Answers::new();
        answers.push(
            MacCommand::LinkAdrAns {
                power_ack: true,
                data_rate_ack: true,
                channel_mask_ack: true,
            },
            false,
        );
        answers.push(MacCommand::RxTimingSetupAns, true);
        answers.push(MacCommand::DutyCycleAns, false);
        answers.request_link_check();

        let mut out = [0u8; 15];
        let (len, link_check, _) = answers.write(&mut out);
        assert_eq!(&out[..len], &[0x03, 0x07, 0x08, 0x04, 0x02]);
        assert!(link_check);

        answers.sent(link_check, false);
        let (len, link_check, _) = answers.write(&mut out);
        assert_eq!(&out[..len], &[0x08], "only the timing answer repeats");
        assert!(!link_check);

        answers.heard_downlink();
        assert_eq!(answers.len(), 0);
    }

    #[test]
    fn a_list_that_does_not_fit_is_cut_after_the_last_whole_answer() {
        let mut answers = Answers::new();
        answers.push(
            MacCommand::DevStatusAns {
                battery: 255,
                margin: 10,
            },
            false,
        );
        answers.push(
            MacCommand::DevStatusAns {
                battery: 255,
                margin: 10,
            },
            false,
        );
        let mut out = [0u8; 5];
        let (len, _, _) = answers.write(&mut out);
        assert_eq!(
            len, 3,
            "the second three-byte answer does not fit in the last two bytes"
        );
        assert_eq!(answers.len(), 6);
    }
}
