//! Application layer clock synchronization, TS003-2.0.0.
//!
//! A device that has no GPS and no real-time clock still needs to know the time: a Class B
//! beacon, a multicast window and a scheduled reboot all depend on it. This package is how it
//! finds out. The device sends [`ClockCommand::AppTimeReq`] with the time it believes it is,
//! the server compares that against when the uplink actually arrived, and answers with
//! [`ClockCommand::AppTimeAns`] carrying the difference in seconds. Most of the time there is
//! nothing to say and the server stays quiet.
//!
//! The exchange is guarded by a four-bit token: the device raises it every time it applies a
//! correction, and an answer whose token does not match the request is ignored, so a late
//! answer cannot pull a corrected clock back.
//!
//! [`ClockSync`] holds that state for a device: it builds each request, reads what comes
//! back, and says what the device owes its server in return.
//!
//! # Examples
//!
//! A device asks for the time and applies the correction it gets back:
//!
//! ```
//! use pamoja_lorawan::packages::clock::{ClockCommand, ClockSync};
//! use pamoja_lorawan::Direction;
//!
//! let mut sync = ClockSync::new();
//! let mut frame = [0u8; 8];
//! let len = sync.app_time_req(1_000_000, false, &mut frame)?;
//!
//! // The server saw the uplink 12 seconds later than the device thinks it is.
//! let answer = ClockCommand::AppTimeAns { time_correction: 12, token: 0 };
//! let mut down = [0u8; 8];
//! let len = answer.encode(&mut down)?;
//!
//! let read = sync.heard(&down[..len])?;
//! assert_eq!(read.correction, Some(12));
//! assert_eq!(sync.token(), 1, "the token moves on, so a repeat is ignored");
//! # Ok::<(), pamoja_lorawan::LorawanError>(())
//! ```

use super::{payload, write, PackageVersion};
use crate::{Direction, LorawanError};

/// The port the clock synchronization package is spoken on, section 3.
pub const PORT: u8 = 202;

/// The identifier of this package, section 3.
pub const PACKAGE: u8 = 1;

/// The version of this package implemented here, section 3.
pub const VERSION: u8 = 2;

/// The longest clock synchronization command, in bytes.
pub const MAX_COMMAND: usize = 6;

const CID_PACKAGE_VERSION: u8 = 0x00;
const CID_APP_TIME: u8 = 0x01;
const CID_PERIODICITY: u8 = 0x02;
const CID_FORCE_RESYNC: u8 = 0x03;

/// One command of the clock synchronization package.
///
/// The identifier names a different command in each direction, so reading one takes the
/// direction the frame traveled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClockCommand {
    /// The server asking which version of this package the device implements.
    PackageVersionReq,
    /// The device's answer, section 3.1.
    PackageVersionAns(PackageVersion),
    /// The device asking for a correction, section 3.2.
    AppTimeReq {
        /// What the device believes the time is: seconds since the GPS epoch, modulo 2^32,
        /// read immediately before the frame goes out.
        device_time: u32,
        /// Whether the server must answer even when the clock is already right.
        ans_required: bool,
        /// The four-bit token the answer has to carry back.
        token: u8,
    },
    /// The server's correction, section 3.2.
    AppTimeAns {
        /// The seconds to add to the device's clock, which may be negative.
        time_correction: i32,
        /// The token of the request being answered.
        token: u8,
    },
    /// The server setting how often the device asks, section 3.3.
    DeviceAppTimePeriodicityReq {
        /// The coded period: 128 * 2^period seconds, plus or minus thirty.
        period: u8,
    },
    /// The device's answer, which also reports its clock, section 3.3.
    DeviceAppTimePeriodicityAns {
        /// Whether the device refuses a period set by the server and manages its own.
        not_supported: bool,
        /// What the device believes the time is.
        device_time: u32,
    },
    /// The server telling the device to synchronize now, section 3.4.
    ForceDeviceResyncCmd {
        /// How many requests to send, at most seven; zero means the command is discarded.
        transmissions: u8,
    },
}

impl ClockCommand {
    /// The identifier this command travels under.
    ///
    /// # Returns
    ///
    /// The identifier, 0x00 to 0x03.
    #[must_use]
    pub const fn cid(&self) -> u8 {
        match self {
            ClockCommand::PackageVersionReq | ClockCommand::PackageVersionAns(_) => {
                CID_PACKAGE_VERSION
            }
            ClockCommand::AppTimeReq { .. } | ClockCommand::AppTimeAns { .. } => CID_APP_TIME,
            ClockCommand::DeviceAppTimePeriodicityReq { .. }
            | ClockCommand::DeviceAppTimePeriodicityAns { .. } => CID_PERIODICITY,
            ClockCommand::ForceDeviceResyncCmd { .. } => CID_FORCE_RESYNC,
        }
    }

    /// Which way this command travels.
    ///
    /// # Returns
    ///
    /// [`Direction::Uplink`] for what a device sends, [`Direction::Downlink`] for what a
    /// server sends.
    #[must_use]
    pub const fn direction(&self) -> Direction {
        match self {
            ClockCommand::PackageVersionAns(_)
            | ClockCommand::AppTimeReq { .. }
            | ClockCommand::DeviceAppTimePeriodicityAns { .. } => Direction::Uplink,
            _ => Direction::Downlink,
        }
    }

    /// Reads one command from the front of a message.
    ///
    /// # Arguments
    ///
    /// * `direction` - which way the frame carrying it traveled.
    /// * `bytes` - the message, from this command's identifier on.
    ///
    /// # Returns
    ///
    /// The command and how many bytes it took.
    ///
    /// # Errors
    ///
    /// [`LorawanError::MalformedFrame`] when the message ends inside the command, and
    /// [`LorawanError::UnknownCommand`] for an identifier this package does not define.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::packages::clock::ClockCommand;
    /// use pamoja_lorawan::Direction;
    ///
    /// let (command, taken) = ClockCommand::parse(Direction::Downlink, &[0x03, 0x02])?;
    /// assert_eq!(command, ClockCommand::ForceDeviceResyncCmd { transmissions: 2 });
    /// assert_eq!(taken, 2);
    /// # Ok::<(), pamoja_lorawan::LorawanError>(())
    /// ```
    pub fn parse(
        direction: Direction,
        bytes: &[u8],
    ) -> Result<(ClockCommand, usize), LorawanError> {
        let cid = *bytes.first().ok_or(LorawanError::MalformedFrame)?;
        let up = matches!(direction, Direction::Uplink);
        match (cid, up) {
            (CID_PACKAGE_VERSION, false) => Ok((ClockCommand::PackageVersionReq, 1)),
            (CID_PACKAGE_VERSION, true) => {
                let (fields, taken) = payload(bytes, 2)?;
                Ok((
                    ClockCommand::PackageVersionAns(PackageVersion {
                        package: fields[0],
                        version: fields[1],
                    }),
                    taken,
                ))
            }
            (CID_APP_TIME, true) => {
                let (fields, taken) = payload(bytes, 5)?;
                Ok((
                    ClockCommand::AppTimeReq {
                        device_time: u32::from_le_bytes([
                            fields[0], fields[1], fields[2], fields[3],
                        ]),
                        ans_required: fields[4] & 0x10 != 0,
                        token: fields[4] & 0x0F,
                    },
                    taken,
                ))
            }
            (CID_APP_TIME, false) => {
                let (fields, taken) = payload(bytes, 5)?;
                Ok((
                    ClockCommand::AppTimeAns {
                        time_correction: i32::from_le_bytes([
                            fields[0], fields[1], fields[2], fields[3],
                        ]),
                        token: fields[4] & 0x0F,
                    },
                    taken,
                ))
            }
            (CID_PERIODICITY, false) => {
                let (fields, taken) = payload(bytes, 1)?;
                Ok((
                    ClockCommand::DeviceAppTimePeriodicityReq {
                        period: fields[0] & 0x0F,
                    },
                    taken,
                ))
            }
            (CID_PERIODICITY, true) => {
                let (fields, taken) = payload(bytes, 5)?;
                Ok((
                    ClockCommand::DeviceAppTimePeriodicityAns {
                        not_supported: fields[0] & 0x01 != 0,
                        device_time: u32::from_le_bytes([
                            fields[1], fields[2], fields[3], fields[4],
                        ]),
                    },
                    taken,
                ))
            }
            (CID_FORCE_RESYNC, false) => {
                let (fields, taken) = payload(bytes, 1)?;
                Ok((
                    ClockCommand::ForceDeviceResyncCmd {
                        transmissions: fields[0] & 0x07,
                    },
                    taken,
                ))
            }
            _ => Err(LorawanError::UnknownCommand(cid)),
        }
    }

    /// Writes this command out.
    ///
    /// # Arguments
    ///
    /// * `out` - where to write it, at least [`MAX_COMMAND`] bytes for any command.
    ///
    /// # Returns
    ///
    /// How many bytes were written.
    ///
    /// # Errors
    ///
    /// [`LorawanError::PayloadTooLong`] when the buffer is too small.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::packages::clock::ClockCommand;
    ///
    /// let mut out = [0u8; 8];
    /// let request = ClockCommand::AppTimeReq { device_time: 1, ans_required: true, token: 3 };
    /// assert_eq!(request.encode(&mut out)?, 6);
    /// assert_eq!(&out[..6], &[0x01, 0x01, 0x00, 0x00, 0x00, 0x13]);
    /// # Ok::<(), pamoja_lorawan::LorawanError>(())
    /// ```
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, LorawanError> {
        match self {
            ClockCommand::PackageVersionReq => write(out, CID_PACKAGE_VERSION, &[]),
            ClockCommand::PackageVersionAns(version) => write(
                out,
                CID_PACKAGE_VERSION,
                &[version.package, version.version],
            ),
            ClockCommand::AppTimeReq {
                device_time,
                ans_required,
                token,
            } => {
                let time = device_time.to_le_bytes();
                let param = (u8::from(*ans_required) << 4) | (token & 0x0F);
                write(
                    out,
                    CID_APP_TIME,
                    &[time[0], time[1], time[2], time[3], param],
                )
            }
            ClockCommand::AppTimeAns {
                time_correction,
                token,
            } => {
                let delta = time_correction.to_le_bytes();
                write(
                    out,
                    CID_APP_TIME,
                    &[delta[0], delta[1], delta[2], delta[3], token & 0x0F],
                )
            }
            ClockCommand::DeviceAppTimePeriodicityReq { period } => {
                write(out, CID_PERIODICITY, &[period & 0x0F])
            }
            ClockCommand::DeviceAppTimePeriodicityAns {
                not_supported,
                device_time,
            } => {
                let time = device_time.to_le_bytes();
                write(
                    out,
                    CID_PERIODICITY,
                    &[u8::from(*not_supported), time[0], time[1], time[2], time[3]],
                )
            }
            ClockCommand::ForceDeviceResyncCmd { transmissions } => {
                write(out, CID_FORCE_RESYNC, &[transmissions & 0x07])
            }
        }
    }
}

/// Walks the commands packed into one clock synchronization message.
///
/// Iteration stops at an identifier this package does not define, because a command carries
/// no length of its own and cannot be stepped over.
pub struct ClockCommands<'a> {
    direction: Direction,
    bytes: &'a [u8],
    at: usize,
    stopped: bool,
}

impl<'a> ClockCommands<'a> {
    /// Reads the commands in a message.
    ///
    /// # Arguments
    ///
    /// * `direction` - which way the frame carrying them traveled.
    /// * `bytes` - the payload sent on port [`PORT`].
    ///
    /// # Returns
    ///
    /// The walk.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::packages::clock::{ClockCommand, ClockCommands};
    /// use pamoja_lorawan::Direction;
    ///
    /// let message = [0x00, 0x03, 0x01];
    /// let read: Vec<ClockCommand> = ClockCommands::new(Direction::Downlink, &message)
    ///     .map(|command| command.unwrap())
    ///     .collect();
    /// assert_eq!(read[0], ClockCommand::PackageVersionReq);
    /// assert_eq!(read[1], ClockCommand::ForceDeviceResyncCmd { transmissions: 1 });
    /// ```
    #[must_use]
    pub const fn new(direction: Direction, bytes: &'a [u8]) -> ClockCommands<'a> {
        ClockCommands {
            direction,
            bytes,
            at: 0,
            stopped: false,
        }
    }

    /// What was not read.
    ///
    /// # Returns
    ///
    /// The bytes left: empty once every command has been read, and whatever followed an
    /// identifier this package does not define.
    #[must_use]
    pub fn remaining(&self) -> &'a [u8] {
        &self.bytes[self.at.min(self.bytes.len())..]
    }
}

impl Iterator for ClockCommands<'_> {
    type Item = Result<ClockCommand, LorawanError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stopped || self.at >= self.bytes.len() {
            return None;
        }
        match ClockCommand::parse(self.direction, &self.bytes[self.at..]) {
            Ok((command, taken)) => {
                self.at += taken;
                Some(Ok(command))
            }
            Err(LorawanError::UnknownCommand(_)) => {
                self.stopped = true;
                None
            }
            Err(error) => {
                self.stopped = true;
                Some(Err(error))
            }
        }
    }
}

/// What a downlink on the clock port asked of the device.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Heard {
    /// The seconds to add to the device's clock, where an answer carried a correction that
    /// matched the outstanding request.
    pub correction: Option<i32>,
    /// Whether the correction was the largest the field carries, so the clock is not yet
    /// right and another request should follow, section 3.2.
    pub more_correction: bool,
    /// How many requests the server asked for at once, from a resynchronization command.
    pub resync: Option<u8>,
    /// Whether the device now owes an answer, which [`ClockSync::answer`] writes.
    pub answer_due: bool,
}

/// The clock synchronization package running on a device, TS003-2.0.0.
///
/// It keeps the token that pairs an answer with its request, the period the server set, and
/// whatever answer the device owes. It holds no clock: the caller passes the device's own
/// time into each request and applies the correction that comes back.
#[derive(Clone, Copy, Debug)]
pub struct ClockSync {
    token: u8,
    period: u8,
    self_managed: bool,
    owed: Option<ClockCommand>,
    waiting: bool,
}

impl Default for ClockSync {
    fn default() -> ClockSync {
        ClockSync::new()
    }
}

impl ClockSync {
    /// The period a device asks at until its server says otherwise, section 3.3.
    pub const DEFAULT_PERIOD: u8 = 0;

    /// Starts the package on a device, with the token at zero.
    ///
    /// # Returns
    ///
    /// The package, with nothing outstanding.
    #[must_use]
    pub const fn new() -> ClockSync {
        ClockSync {
            token: 0,
            period: ClockSync::DEFAULT_PERIOD,
            self_managed: false,
            owed: None,
            waiting: false,
        }
    }

    /// Starts the package on a device that manages its own periodicity.
    ///
    /// Such a device answers a server that tries to set a period with the not-supported bit,
    /// section 3.3.
    ///
    /// # Returns
    ///
    /// The package.
    #[must_use]
    pub const fn self_managed() -> ClockSync {
        ClockSync {
            self_managed: true,
            ..ClockSync::new()
        }
    }

    /// The token the next request will carry.
    ///
    /// # Returns
    ///
    /// A value 0 to 15, which rises every time a correction is applied.
    #[must_use]
    pub const fn token(&self) -> u8 {
        self.token
    }

    /// How often the device should ask, as the server last set it.
    ///
    /// # Returns
    ///
    /// The seconds between requests, before the random spread of thirty seconds either way
    /// that section 3.3 asks for.
    #[must_use]
    pub const fn period_s(&self) -> u32 {
        128u32 << self.period
    }

    /// Builds the request that asks for a correction, section 3.2.
    ///
    /// # Arguments
    ///
    /// * `device_time` - what the device believes the time is, in seconds since the GPS
    ///   epoch, read immediately before the frame goes out.
    /// * `ans_required` - whether the server must answer even if the clock is right.
    /// * `out` - where to write the command.
    ///
    /// # Returns
    ///
    /// How many bytes were written.
    ///
    /// # Errors
    ///
    /// [`LorawanError::PayloadTooLong`] when the buffer is too small.
    pub fn app_time_req(
        &mut self,
        device_time: u32,
        ans_required: bool,
        out: &mut [u8],
    ) -> Result<usize, LorawanError> {
        self.waiting = true;
        ClockCommand::AppTimeReq {
            device_time,
            ans_required,
            token: self.token,
        }
        .encode(out)
    }

    /// Reads a downlink on the clock port and acts on it.
    ///
    /// A correction whose token does not match the outstanding request is ignored, as section
    /// 3.2 asks, and so is any answer with no request waiting on it.
    ///
    /// # Arguments
    ///
    /// * `payload` - what arrived on port [`PORT`].
    ///
    /// # Returns
    ///
    /// The correction to apply, whether more of one is coming, and what the device now owes.
    ///
    /// # Errors
    ///
    /// The first error a command in the message raised.
    pub fn heard(&mut self, payload: &[u8]) -> Result<Heard, LorawanError> {
        let mut heard = Heard::default();
        for command in ClockCommands::new(Direction::Downlink, payload) {
            match command? {
                ClockCommand::PackageVersionReq => {
                    self.owed = Some(ClockCommand::PackageVersionAns(PackageVersion {
                        package: PACKAGE,
                        version: VERSION,
                    }));
                    heard.answer_due = true;
                }
                ClockCommand::AppTimeAns {
                    time_correction,
                    token,
                } => {
                    if self.waiting && token == self.token {
                        self.token = (self.token + 1) % 16;
                        self.waiting = false;
                        heard.correction = Some(time_correction);
                        heard.more_correction =
                            time_correction == i32::MAX || time_correction == i32::MIN;
                    }
                }
                ClockCommand::DeviceAppTimePeriodicityReq { period } => {
                    if !self.self_managed {
                        self.period = period;
                    }
                    self.owed = Some(ClockCommand::DeviceAppTimePeriodicityAns {
                        not_supported: self.self_managed,
                        device_time: 0,
                    });
                    heard.answer_due = true;
                }
                ClockCommand::ForceDeviceResyncCmd { transmissions } if transmissions > 0 => {
                    heard.resync = Some(transmissions);
                }
                _ => {}
            }
        }
        Ok(heard)
    }

    /// Writes the answer the device owes, if it owes one.
    ///
    /// # Arguments
    ///
    /// * `device_time` - what the device believes the time is, for the answer that reports
    ///   it.
    /// * `out` - where to write the command.
    ///
    /// # Returns
    ///
    /// How many bytes were written, and zero when nothing is owed.
    ///
    /// # Errors
    ///
    /// [`LorawanError::PayloadTooLong`] when the buffer is too small, leaving the answer
    /// owed.
    pub fn answer(&mut self, device_time: u32, out: &mut [u8]) -> Result<usize, LorawanError> {
        let Some(owed) = self.owed else {
            return Ok(0);
        };
        let owed = match owed {
            ClockCommand::DeviceAppTimePeriodicityAns { not_supported, .. } => {
                ClockCommand::DeviceAppTimePeriodicityAns {
                    not_supported,
                    device_time,
                }
            }
            other => other,
        };
        let written = owed.encode(out)?;
        self.owed = None;
        Ok(written)
    }

    /// Whether the device owes its server an answer.
    ///
    /// # Returns
    ///
    /// `true` when [`ClockSync::answer`] would write one.
    #[must_use]
    pub const fn answer_due(&self) -> bool {
        self.owed.is_some()
    }
}
