//! The MAC answers and requests a device owes its next uplinks.
//!
//! TS001-1.0.4 section 5 has answers sent in the order the commands arrived, and has four of
//! them, the ones that change how a device listens, repeated on every uplink until a Class A
//! downlink shows the network heard them. When a frame cannot hold everything, answers come
//! first, then commands the device starts itself, then the application payload, and a list
//! that still does not fit is cut after the last whole command.

use crate::mac::MacCommand;

/// How many answers a device holds for one downlink's worth of commands.
///
/// A frame holds at most 242 bytes of them and most are one or two bytes, so a network that
/// sends more commands than this in one downlink has its surplus executed but unanswered.
pub(crate) const MAX_ANSWERS: usize = 64;

/// The longest command a device owes or starts: a relay's `NotifyNewEndDeviceReq`, seven
/// bytes with its identifier.
pub(crate) const MAX_ANSWER: usize = 7;

/// How a queued command goes out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Kind {
    /// An answer sent on the next uplink.
    Once,
    /// An answer sent on every uplink until a downlink arrives.
    Sticky,
    /// A command the device starts, sent once after the answers, and kept until it has gone
    /// out whatever arrives in between.
    Started,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    bytes: [u8; MAX_ANSWER],
    len: u8,
    kind: Kind,
}

/// What one uplink took from the queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) struct Written {
    /// The bytes written.
    pub(crate) len: usize,
    /// How many started commands went out.
    pub(crate) started: usize,
    /// Whether the link check request went out.
    pub(crate) link_check: bool,
    /// Whether the time request went out.
    pub(crate) device_time: bool,
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
                bytes: [0; MAX_ANSWER],
                len: 0,
                kind: Kind::Once,
            }; MAX_ANSWERS],
            count: 0,
            link_check: false,
            device_time: false,
        }
    }

    /// Queues an answer, sent once or, when `sticky`, on every uplink until a downlink.
    pub(crate) fn push(&mut self, answer: MacCommand, sticky: bool) {
        let kind = if sticky { Kind::Sticky } else { Kind::Once };
        self.push_command(answer, kind);
    }

    /// Queues a command the device starts itself.
    ///
    /// # Returns
    ///
    /// `false` if the queue is full.
    pub(crate) fn start(&mut self, command: MacCommand) -> bool {
        self.push_command(command, Kind::Started)
    }

    /// Reports whether a started command beginning with these bytes is still to go out.
    pub(crate) fn starts_with(&self, prefix: &[u8]) -> bool {
        self.entries[..self.count].iter().any(|entry| {
            entry.kind == Kind::Started && entry.bytes[..usize::from(entry.len)].starts_with(prefix)
        })
    }

    fn push_command(&mut self, command: MacCommand, kind: Kind) -> bool {
        let mut bytes = [0u8; MAX_ANSWER];
        match command.encode(&mut bytes) {
            Ok(len) => self.push_encoded(&bytes[..len], kind),
            Err(_) => false,
        }
    }

    /// Each queued command's bytes and how it goes out, in order.
    pub(crate) fn owed(&self) -> impl Iterator<Item = (&[u8], Kind)> + '_ {
        self.entries[..self.count]
            .iter()
            .map(|entry| (&entry.bytes[..usize::from(entry.len)], entry.kind))
    }

    /// Whether a link check and a time request are still to go out.
    pub(crate) const fn requests(&self) -> (bool, bool) {
        (self.link_check, self.device_time)
    }

    /// Queues a command already encoded, as a saved state carries it.
    ///
    /// # Returns
    ///
    /// `false` if the queue is full or the command is longer than a command can be.
    pub(crate) fn push_encoded(&mut self, command: &[u8], kind: Kind) -> bool {
        if self.count == MAX_ANSWERS || command.is_empty() || command.len() > MAX_ANSWER {
            return false;
        }
        let mut bytes = [0u8; MAX_ANSWER];
        bytes[..command.len()].copy_from_slice(command);
        self.entries[self.count] = Entry {
            bytes,
            len: command.len() as u8,
            kind,
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
    /// Commands and requests the device started stay queued if they have not gone out yet.
    pub(crate) fn heard_downlink(&mut self) {
        self.keep(|entry| entry.kind == Kind::Started);
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

    /// Writes as many whole answers, then started commands, then requests, as fit in `out`.
    ///
    /// # Returns
    ///
    /// What went out.
    pub(crate) fn write(&self, out: &mut [u8]) -> Written {
        let mut written = Written::default();
        for entry in self.entries[..self.count]
            .iter()
            .filter(|entry| entry.kind != Kind::Started)
        {
            if !put(
                out,
                &mut written.len,
                &entry.bytes[..usize::from(entry.len)],
            ) {
                return written;
            }
        }
        for entry in self.entries[..self.count]
            .iter()
            .filter(|entry| entry.kind == Kind::Started)
        {
            if !put(
                out,
                &mut written.len,
                &entry.bytes[..usize::from(entry.len)],
            ) {
                return written;
            }
            written.started += 1;
        }
        if self.link_check && put(out, &mut written.len, &[MacCommand::LinkCheckReq.cid()]) {
            written.link_check = true;
        }
        if self.device_time && put(out, &mut written.len, &[MacCommand::DeviceTimeReq.cid()]) {
            written.device_time = true;
        }
        written
    }

    /// Settles what an uplink carried: answers sent once are done, sticky ones stay, started
    /// commands that went out are done, and so are the requests that went out.
    pub(crate) fn sent(&mut self, written: Written) {
        let mut started = 0;
        self.keep(|entry| match entry.kind {
            Kind::Once => false,
            Kind::Sticky => true,
            Kind::Started => {
                started += 1;
                started > written.started
            }
        });
        if written.link_check {
            self.link_check = false;
        }
        if written.device_time {
            self.device_time = false;
        }
    }

    fn keep(&mut self, mut keep: impl FnMut(&Entry) -> bool) {
        let mut kept = 0;
        for index in 0..self.count {
            if keep(&self.entries[index]) {
                self.entries[kept] = self.entries[index];
                kept += 1;
            }
        }
        self.count = kept;
    }
}

/// Appends whole bytes if they fit.
fn put(out: &mut [u8], at: &mut usize, bytes: &[u8]) -> bool {
    let end = *at + bytes.len();
    if end > out.len() {
        return false;
    }
    out[*at..end].copy_from_slice(bytes);
    *at = end;
    true
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
        let written = answers.write(&mut out);
        assert_eq!(&out[..written.len], &[0x03, 0x07, 0x08, 0x04, 0x02]);
        assert!(written.link_check);

        answers.sent(written);
        let written = answers.write(&mut out);
        assert_eq!(
            &out[..written.len],
            &[0x08],
            "only the timing answer repeats"
        );
        assert!(!written.link_check);

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
        let written = answers.write(&mut out);
        assert_eq!(
            written.len, 3,
            "the second three-byte answer does not fit in the last two bytes"
        );
        assert_eq!(answers.len(), 6);
    }

    #[test]
    fn a_started_command_follows_the_answers_and_outlasts_a_downlink_until_it_goes_out() {
        let notice = MacCommand::NotifyNewEndDeviceReq {
            dev_addr: 0x2601_1BDA,
            rssi_dbm: -100,
            snr_db: 5,
        };
        let mut answers = Answers::new();
        assert!(answers.start(notice));
        assert!(answers.starts_with(&[0x46, 0xDA, 0x1B, 0x01, 0x26]));
        answers.push(MacCommand::DutyCycleAns, false);

        answers.heard_downlink();
        assert_eq!(
            answers.len(),
            7,
            "the downlink settles answers, not the notice"
        );

        answers.push(MacCommand::DutyCycleAns, false);
        let mut out = [0u8; 15];
        let written = answers.write(&mut out);
        assert_eq!(out[0], 0x04, "the answer goes first");
        assert_eq!(out[1], 0x46);
        assert_eq!((written.len, written.started), (8, 1));

        answers.sent(written);
        assert_eq!(answers.len(), 0);

        assert!(answers.start(notice));
        let mut short = [0u8; 6];
        let written = answers.write(&mut short);
        assert_eq!(written.started, 0, "seven bytes do not fit in six");
        answers.sent(written);
        assert_eq!(answers.len(), 7, "a notice that did not go out stays");
    }
}
