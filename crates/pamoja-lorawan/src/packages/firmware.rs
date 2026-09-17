//! Firmware management, TS006-1.0.0.
//!
//! This is the package a server uses to ask what a device is running, what upgrade image it
//! is holding, and when to reboot into it. Getting the image onto the device is another
//! matter, out of this specification's scope; what this covers is everything around it:
//! reporting the firmware and hardware versions, reporting whether the stored image is
//! whole, authentic and meant for this hardware, deleting one, and scheduling the reboot at
//! which an image is installed.
//!
//! A device stores one programmed reboot at a time. It can be set as a moment in time
//! ([`FirmwareCommand::DevRebootTimeReq`]) or as a countdown
//! ([`FirmwareCommand::DevRebootCountdownReq`]), and either one replaces whatever was
//! programmed before. Both have a value that cancels the reboot and a value that means
//! now, and a device that reboots immediately never answers, because it is gone.
//!
//! [`FirmwareManager`] holds that state for a device: the versions it reports, the image it
//! is holding, and the reboot it has been given.
//!
//! # Examples
//!
//! A server asks what the device would boot into, and schedules it:
//!
//! ```
//! use pamoja_lorawan::packages::firmware::{FirmwareManager, Image, UpImageStatus};
//!
//! let mut manager = FirmwareManager::new(0x0001_0203, 0xAABB_CCDD)
//!     .with_image(Image::valid(0x0001_0204));
//!
//! // DevUpgradeImageReq, then a reboot in an hour.
//! let mut out = [0u8; 16];
//! let len = manager.heard(&[0x04, 0x03, 0x10, 0x0E, 0x00], &mut out)?;
//! assert_eq!(out[0], 0x04, "the image answer comes first");
//! assert_eq!(out[1], UpImageStatus::Valid as u8);
//! assert_eq!(manager.reboot_in_s(), Some(3600));
//! # assert!(len > 6);
//! # Ok::<(), pamoja_lorawan::LorawanError>(())
//! ```

use super::{payload, write, PackageVersion};
use crate::{Direction, LorawanError};

/// The port the firmware management package is spoken on, section 3.
pub const PORT: u8 = 203;

/// The identifier of this package, section 3.
pub const PACKAGE: u8 = 4;

/// The version of this package implemented here, section 3.
pub const VERSION: u8 = 1;

/// The longest firmware management command, in bytes.
pub const MAX_COMMAND: usize = 9;

/// The reboot time that means as soon as possible, section 3.3.
pub const REBOOT_NOW: u32 = 0x0000_0000;

/// The reboot time that cancels a programmed reboot, section 3.3.
pub const REBOOT_CANCEL: u32 = 0xFFFF_FFFF;

/// The countdown that means as soon as possible, section 3.4.
pub const COUNTDOWN_NOW: u32 = 0x00_0000;

/// The countdown that cancels a programmed reboot, section 3.4.
pub const COUNTDOWN_CANCEL: u32 = 0xFF_FFFF;

const CID_PACKAGE_VERSION: u8 = 0x00;
const CID_DEV_VERSION: u8 = 0x01;
const CID_REBOOT_TIME: u8 = 0x02;
const CID_REBOOT_COUNTDOWN: u8 = 0x03;
const CID_UPGRADE_IMAGE: u8 = 0x04;
const CID_DELETE_IMAGE: u8 = 0x05;

/// What a device says about the firmware upgrade image it is holding, table 10.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum UpImageStatus {
    /// It is holding none.
    None = 0,
    /// One is there, but it is corrupt or its signature does not verify.
    Corrupt = 1,
    /// One is there and authentic, but it is not for this hardware.
    WrongHardware = 2,
    /// One is there, and it can be installed.
    Valid = 3,
}

impl UpImageStatus {
    /// Reads the coded status.
    ///
    /// # Arguments
    ///
    /// * `coded` - the two-bit field.
    ///
    /// # Returns
    ///
    /// The status.
    #[must_use]
    pub const fn from_bits(coded: u8) -> UpImageStatus {
        match coded & 0x03 {
            0 => UpImageStatus::None,
            1 => UpImageStatus::Corrupt,
            2 => UpImageStatus::WrongHardware,
            _ => UpImageStatus::Valid,
        }
    }
}

/// Why a device would not delete the image it was asked to, table 13.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct DeleteStatus {
    /// The device holds no valid upgrade image.
    pub no_valid_image: bool,
    /// The version asked for is not the one the device holds.
    pub invalid_version: bool,
}

impl DeleteStatus {
    /// Whether the delete succeeded.
    ///
    /// # Returns
    ///
    /// `true` when neither error bit is set.
    #[must_use]
    pub const fn deleted(&self) -> bool {
        !self.no_valid_image && !self.invalid_version
    }
}

/// The firmware upgrade image a device is holding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Image {
    /// What the device makes of it.
    pub status: UpImageStatus,
    /// The version the device would be running once it is installed, which the device
    /// reports only for an image it can install, section 3.5.
    pub version: u32,
}

impl Image {
    /// An image that is whole, authentic, and meant for this hardware.
    ///
    /// # Arguments
    ///
    /// * `version` - the version the device would run once it is installed.
    ///
    /// # Returns
    ///
    /// The image.
    #[must_use]
    pub const fn valid(version: u32) -> Image {
        Image {
            status: UpImageStatus::Valid,
            version,
        }
    }

    /// An image the device cannot install, and why.
    ///
    /// # Arguments
    ///
    /// * `status` - what is wrong with it.
    ///
    /// # Returns
    ///
    /// The image.
    #[must_use]
    pub const fn refused(status: UpImageStatus) -> Image {
        Image { status, version: 0 }
    }
}

/// One command of the firmware management package.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FirmwareCommand {
    /// The server asking which version of this package the device implements.
    PackageVersionReq,
    /// The device's answer, section 3.1.
    PackageVersionAns(PackageVersion),
    /// The server asking what the device is running, section 3.2.
    DevVersionReq,
    /// The device's answer, whose two fields the manufacturer allots, section 3.2.
    DevVersionAns {
        /// The firmware the device is running.
        firmware: u32,
        /// The hardware it is running on.
        hardware: u32,
    },
    /// The server programming a reboot at a moment in time, section 3.3.
    DevRebootTimeReq {
        /// Seconds since the GPS epoch, [`REBOOT_NOW`] for at once, or [`REBOOT_CANCEL`]
        /// to cancel.
        reboot_time: u32,
    },
    /// The device's answer: the seconds until it reboots, zero for a time it cannot keep,
    /// or [`REBOOT_CANCEL`] acknowledging a cancellation, section 3.3.
    DevRebootTimeAns {
        /// The seconds until the reboot.
        reboot_time: u32,
    },
    /// The server programming a reboot after a delay, section 3.4.
    DevRebootCountdownReq {
        /// The seconds to wait, [`COUNTDOWN_NOW`] for at once, or [`COUNTDOWN_CANCEL`] to
        /// cancel. Three octets on the air.
        countdown: u32,
    },
    /// The device's answer, in the same three octets, section 3.4.
    DevRebootCountdownAns {
        /// The seconds until the reboot.
        countdown: u32,
    },
    /// The server asking what upgrade image the device holds, section 3.5.
    DevUpgradeImageReq,
    /// The device's answer, which carries a version only for an image it can install.
    DevUpgradeImageAns {
        /// What the device makes of the image.
        status: UpImageStatus,
        /// The version it would run, present only for [`UpImageStatus::Valid`].
        next_version: Option<u32>,
    },
    /// The server asking the device to delete the image it holds, section 3.6.
    DevDeleteImageReq {
        /// The version the server believes is there; a device that holds another refuses.
        version: u32,
    },
    /// The device's answer, section 3.6.
    DevDeleteImageAns(DeleteStatus),
}

impl FirmwareCommand {
    /// The identifier this command travels under.
    ///
    /// # Returns
    ///
    /// The identifier, 0x00 to 0x05.
    #[must_use]
    pub const fn cid(&self) -> u8 {
        match self {
            FirmwareCommand::PackageVersionReq | FirmwareCommand::PackageVersionAns(_) => {
                CID_PACKAGE_VERSION
            }
            FirmwareCommand::DevVersionReq | FirmwareCommand::DevVersionAns { .. } => {
                CID_DEV_VERSION
            }
            FirmwareCommand::DevRebootTimeReq { .. } | FirmwareCommand::DevRebootTimeAns { .. } => {
                CID_REBOOT_TIME
            }
            FirmwareCommand::DevRebootCountdownReq { .. }
            | FirmwareCommand::DevRebootCountdownAns { .. } => CID_REBOOT_COUNTDOWN,
            FirmwareCommand::DevUpgradeImageReq | FirmwareCommand::DevUpgradeImageAns { .. } => {
                CID_UPGRADE_IMAGE
            }
            FirmwareCommand::DevDeleteImageReq { .. } | FirmwareCommand::DevDeleteImageAns(_) => {
                CID_DELETE_IMAGE
            }
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
            FirmwareCommand::PackageVersionAns(_)
            | FirmwareCommand::DevVersionAns { .. }
            | FirmwareCommand::DevRebootTimeAns { .. }
            | FirmwareCommand::DevRebootCountdownAns { .. }
            | FirmwareCommand::DevUpgradeImageAns { .. }
            | FirmwareCommand::DevDeleteImageAns(_) => Direction::Uplink,
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
    /// use pamoja_lorawan::packages::firmware::FirmwareCommand;
    /// use pamoja_lorawan::Direction;
    ///
    /// let (command, taken) = FirmwareCommand::parse(Direction::Downlink, &[0x03, 0x10, 0x0E, 0x00])?;
    /// assert_eq!(command, FirmwareCommand::DevRebootCountdownReq { countdown: 3_600 });
    /// assert_eq!(taken, 4);
    /// # Ok::<(), pamoja_lorawan::LorawanError>(())
    /// ```
    pub fn parse(
        direction: Direction,
        bytes: &[u8],
    ) -> Result<(FirmwareCommand, usize), LorawanError> {
        let cid = *bytes.first().ok_or(LorawanError::MalformedFrame)?;
        let up = matches!(direction, Direction::Uplink);
        match (cid, up) {
            (CID_PACKAGE_VERSION, false) => Ok((FirmwareCommand::PackageVersionReq, 1)),
            (CID_PACKAGE_VERSION, true) => {
                let (fields, taken) = payload(bytes, 2)?;
                Ok((
                    FirmwareCommand::PackageVersionAns(PackageVersion {
                        package: fields[0],
                        version: fields[1],
                    }),
                    taken,
                ))
            }
            (CID_DEV_VERSION, false) => Ok((FirmwareCommand::DevVersionReq, 1)),
            (CID_DEV_VERSION, true) => {
                let (fields, taken) = payload(bytes, 8)?;
                Ok((
                    FirmwareCommand::DevVersionAns {
                        firmware: u32::from_le_bytes([fields[0], fields[1], fields[2], fields[3]]),
                        hardware: u32::from_le_bytes([fields[4], fields[5], fields[6], fields[7]]),
                    },
                    taken,
                ))
            }
            (CID_REBOOT_TIME, _) => {
                let (fields, taken) = payload(bytes, 4)?;
                let value = u32::from_le_bytes([fields[0], fields[1], fields[2], fields[3]]);
                Ok((
                    if up {
                        FirmwareCommand::DevRebootTimeAns { reboot_time: value }
                    } else {
                        FirmwareCommand::DevRebootTimeReq { reboot_time: value }
                    },
                    taken,
                ))
            }
            (CID_REBOOT_COUNTDOWN, _) => {
                let (fields, taken) = payload(bytes, 3)?;
                let value = u32::from_le_bytes([fields[0], fields[1], fields[2], 0]);
                Ok((
                    if up {
                        FirmwareCommand::DevRebootCountdownAns { countdown: value }
                    } else {
                        FirmwareCommand::DevRebootCountdownReq { countdown: value }
                    },
                    taken,
                ))
            }
            (CID_UPGRADE_IMAGE, false) => Ok((FirmwareCommand::DevUpgradeImageReq, 1)),
            (CID_UPGRADE_IMAGE, true) => {
                let (status, _) = payload(bytes, 1)?;
                let status = UpImageStatus::from_bits(status[0]);
                if matches!(status, UpImageStatus::Valid) {
                    let (fields, taken) = payload(bytes, 5)?;
                    Ok((
                        FirmwareCommand::DevUpgradeImageAns {
                            status,
                            next_version: Some(u32::from_le_bytes([
                                fields[1], fields[2], fields[3], fields[4],
                            ])),
                        },
                        taken,
                    ))
                } else {
                    Ok((
                        FirmwareCommand::DevUpgradeImageAns {
                            status,
                            next_version: None,
                        },
                        2,
                    ))
                }
            }
            (CID_DELETE_IMAGE, false) => {
                let (fields, taken) = payload(bytes, 4)?;
                Ok((
                    FirmwareCommand::DevDeleteImageReq {
                        version: u32::from_le_bytes([fields[0], fields[1], fields[2], fields[3]]),
                    },
                    taken,
                ))
            }
            (CID_DELETE_IMAGE, true) => {
                let (fields, taken) = payload(bytes, 1)?;
                Ok((
                    FirmwareCommand::DevDeleteImageAns(DeleteStatus {
                        no_valid_image: fields[0] & 0x01 != 0,
                        invalid_version: fields[0] & 0x02 != 0,
                    }),
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
    /// use pamoja_lorawan::packages::firmware::{FirmwareCommand, UpImageStatus};
    ///
    /// let mut out = [0u8; 8];
    /// let answer = FirmwareCommand::DevUpgradeImageAns {
    ///     status: UpImageStatus::None,
    ///     next_version: None,
    /// };
    /// assert_eq!(answer.encode(&mut out)?, 2, "a version rides only with a valid image");
    /// # Ok::<(), pamoja_lorawan::LorawanError>(())
    /// ```
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, LorawanError> {
        match self {
            FirmwareCommand::PackageVersionReq => write(out, CID_PACKAGE_VERSION, &[]),
            FirmwareCommand::PackageVersionAns(version) => write(
                out,
                CID_PACKAGE_VERSION,
                &[version.package, version.version],
            ),
            FirmwareCommand::DevVersionReq => write(out, CID_DEV_VERSION, &[]),
            FirmwareCommand::DevVersionAns { firmware, hardware } => {
                let mut fields = [0u8; 8];
                fields[..4].copy_from_slice(&firmware.to_le_bytes());
                fields[4..].copy_from_slice(&hardware.to_le_bytes());
                write(out, CID_DEV_VERSION, &fields)
            }
            FirmwareCommand::DevRebootTimeReq { reboot_time }
            | FirmwareCommand::DevRebootTimeAns { reboot_time } => {
                write(out, CID_REBOOT_TIME, &reboot_time.to_le_bytes())
            }
            FirmwareCommand::DevRebootCountdownReq { countdown }
            | FirmwareCommand::DevRebootCountdownAns { countdown } => {
                let value = countdown.to_le_bytes();
                write(out, CID_REBOOT_COUNTDOWN, &value[..3])
            }
            FirmwareCommand::DevUpgradeImageReq => write(out, CID_UPGRADE_IMAGE, &[]),
            FirmwareCommand::DevUpgradeImageAns {
                status,
                next_version,
            } => match (status, next_version) {
                (UpImageStatus::Valid, Some(version)) => {
                    let mut fields = [0u8; 5];
                    fields[0] = UpImageStatus::Valid as u8;
                    fields[1..].copy_from_slice(&version.to_le_bytes());
                    write(out, CID_UPGRADE_IMAGE, &fields)
                }
                (status, _) => write(out, CID_UPGRADE_IMAGE, &[*status as u8]),
            },
            FirmwareCommand::DevDeleteImageReq { version } => {
                write(out, CID_DELETE_IMAGE, &version.to_le_bytes())
            }
            FirmwareCommand::DevDeleteImageAns(status) => write(
                out,
                CID_DELETE_IMAGE,
                &[u8::from(status.no_valid_image) | (u8::from(status.invalid_version) << 1)],
            ),
        }
    }
}

/// Walks the commands packed into one firmware management message.
///
/// Iteration stops at an identifier this package does not define, because a command carries
/// no length of its own and cannot be stepped over.
pub struct FirmwareCommands<'a> {
    direction: Direction,
    bytes: &'a [u8],
    at: usize,
    stopped: bool,
}

impl<'a> FirmwareCommands<'a> {
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
    #[must_use]
    pub const fn new(direction: Direction, bytes: &'a [u8]) -> FirmwareCommands<'a> {
        FirmwareCommands {
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

impl Iterator for FirmwareCommands<'_> {
    type Item = Result<FirmwareCommand, LorawanError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stopped || self.at >= self.bytes.len() {
            return None;
        }
        match FirmwareCommand::parse(self.direction, &self.bytes[self.at..]) {
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

/// When a device has been told to reboot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reboot {
    /// Nothing is programmed.
    None,
    /// At a moment in time, in seconds since the GPS epoch.
    At(u32),
    /// After a delay, counted from the command that set it.
    In(u32),
    /// As soon as the device can, which it does without answering.
    Now,
}

/// The firmware management package running on a device, TS006-1.0.0.
///
/// It reports the versions the device was built with, the upgrade image it is holding, and
/// keeps the one programmed reboot the specification allows. It owns no clock: the caller
/// passes the time into [`FirmwareManager::heard`] when the device knows it, and a device
/// that does not know the time refuses a reboot set for a moment in time, as section 3.3
/// asks.
#[derive(Clone, Copy, Debug)]
pub struct FirmwareManager {
    firmware: u32,
    hardware: u32,
    image: Option<Image>,
    reboot: Reboot,
    since_s: u32,
}

impl FirmwareManager {
    /// Starts the package on a device.
    ///
    /// # Arguments
    ///
    /// * `firmware` - the version the device is running, as its manufacturer numbers it.
    /// * `hardware` - the platform it runs on.
    ///
    /// # Returns
    ///
    /// The package, holding no image and no programmed reboot.
    #[must_use]
    pub const fn new(firmware: u32, hardware: u32) -> FirmwareManager {
        FirmwareManager {
            firmware,
            hardware,
            image: None,
            reboot: Reboot::None,
            since_s: 0,
        }
    }

    /// Returns it holding a firmware upgrade image.
    ///
    /// # Arguments
    ///
    /// * `image` - the image the device is holding.
    ///
    /// # Returns
    ///
    /// The package, for chaining.
    #[must_use]
    pub const fn with_image(mut self, image: Image) -> FirmwareManager {
        self.image = Some(image);
        self
    }

    /// The image the device is holding, if it holds one.
    ///
    /// # Returns
    ///
    /// The image.
    #[must_use]
    pub const fn image(&self) -> Option<Image> {
        self.image
    }

    /// Puts an image in the device's store, replacing whatever was there.
    ///
    /// # Arguments
    ///
    /// * `image` - the image, or `None` once one has been installed or deleted.
    pub const fn set_image(&mut self, image: Option<Image>) {
        self.image = image;
    }

    /// The moment the device is to reboot, where one was programmed as a time.
    ///
    /// # Returns
    ///
    /// Seconds since the GPS epoch, and `None` when the reboot is a countdown, immediate, or
    /// not programmed at all.
    #[must_use]
    pub const fn reboot_at_s(&self) -> Option<u32> {
        match self.reboot {
            Reboot::At(when) => Some(when),
            _ => None,
        }
    }

    /// How long until the device is to reboot, where one was programmed as a countdown.
    ///
    /// # Returns
    ///
    /// The seconds left, and `None` when the reboot is a moment in time, immediate, or not
    /// programmed at all.
    #[must_use]
    pub const fn reboot_in_s(&self) -> Option<u32> {
        match self.reboot {
            Reboot::In(delay) => Some(delay),
            _ => None,
        }
    }

    /// Whether the device was told to reboot at once.
    ///
    /// # Returns
    ///
    /// `true` when it should reboot as soon as it can, in which case it sends no answer.
    #[must_use]
    pub const fn reboot_now(&self) -> bool {
        matches!(self.reboot, Reboot::Now)
    }

    /// Forgets the programmed reboot, for a device that has carried it out.
    pub const fn rebooted(&mut self) {
        self.reboot = Reboot::None;
    }

    /// Reads a downlink on the firmware port and writes the answers it calls for.
    ///
    /// Commands run in the order they arrive, and each writes its answer after the one
    /// before it, as section 3 asks. A reboot the device was told to do at once is recorded
    /// on [`FirmwareManager::reboot_now`] and answered with nothing, because the device will
    /// be gone.
    ///
    /// # Arguments
    ///
    /// * `payload` - what arrived on port [`PORT`].
    /// * `now_s` - what the device believes the time is, in seconds since the GPS epoch, or
    ///   `None` when it does not know.
    /// * `out` - where to write the answers.
    ///
    /// # Returns
    ///
    /// How many bytes of answer were written.
    ///
    /// # Errors
    ///
    /// The first error a command raised, or [`LorawanError::PayloadTooLong`] when the
    /// answers do not fit.
    pub fn heard_at(
        &mut self,
        payload: &[u8],
        now_s: Option<u32>,
        out: &mut [u8],
    ) -> Result<usize, LorawanError> {
        let mut at = 0;
        for command in FirmwareCommands::new(Direction::Downlink, payload) {
            let answer = match command? {
                FirmwareCommand::PackageVersionReq => {
                    Some(FirmwareCommand::PackageVersionAns(PackageVersion {
                        package: PACKAGE,
                        version: VERSION,
                    }))
                }
                FirmwareCommand::DevVersionReq => Some(FirmwareCommand::DevVersionAns {
                    firmware: self.firmware,
                    hardware: self.hardware,
                }),
                FirmwareCommand::DevRebootTimeReq { reboot_time } => {
                    self.reboot_time(reboot_time, now_s)
                }
                FirmwareCommand::DevRebootCountdownReq { countdown } => self.countdown(countdown),
                FirmwareCommand::DevUpgradeImageReq => Some(FirmwareCommand::DevUpgradeImageAns {
                    status: self.image.map_or(UpImageStatus::None, |image| image.status),
                    next_version: self.image.and_then(|image| {
                        matches!(image.status, UpImageStatus::Valid).then_some(image.version)
                    }),
                }),
                FirmwareCommand::DevDeleteImageReq { version } => Some(self.delete(version)),
                _ => None,
            };
            if let Some(answer) = answer {
                at += answer.encode(out.get_mut(at..).ok_or(LorawanError::PayloadTooLong)?)?;
            }
        }
        Ok(at)
    }

    /// Reads a downlink on the firmware port from a device that does not know the time.
    ///
    /// # Arguments
    ///
    /// * `payload` - what arrived on port [`PORT`].
    /// * `out` - where to write the answers.
    ///
    /// # Returns
    ///
    /// How many bytes of answer were written.
    ///
    /// # Errors
    ///
    /// As [`FirmwareManager::heard_at`].
    pub fn heard(&mut self, payload: &[u8], out: &mut [u8]) -> Result<usize, LorawanError> {
        self.heard_at(payload, None, out)
    }

    /// Programs a reboot at a moment in time, section 3.3.
    fn reboot_time(&mut self, reboot_time: u32, now_s: Option<u32>) -> Option<FirmwareCommand> {
        match reboot_time {
            REBOOT_NOW => {
                self.reboot = Reboot::Now;
                None
            }
            REBOOT_CANCEL => {
                self.reboot = Reboot::None;
                Some(FirmwareCommand::DevRebootTimeAns {
                    reboot_time: REBOOT_CANCEL,
                })
            }
            when => {
                let left = now_s.filter(|now| when > *now).map(|now| when - now);
                match left {
                    Some(left) => {
                        self.reboot = Reboot::At(when);
                        self.since_s = now_s.unwrap_or(0);
                        Some(FirmwareCommand::DevRebootTimeAns { reboot_time: left })
                    }
                    None => Some(FirmwareCommand::DevRebootTimeAns {
                        reboot_time: REBOOT_NOW,
                    }),
                }
            }
        }
    }

    /// Programs a reboot after a delay, section 3.4.
    fn countdown(&mut self, countdown: u32) -> Option<FirmwareCommand> {
        match countdown {
            COUNTDOWN_NOW => {
                self.reboot = Reboot::Now;
                None
            }
            COUNTDOWN_CANCEL => {
                self.reboot = Reboot::None;
                Some(FirmwareCommand::DevRebootCountdownAns {
                    countdown: COUNTDOWN_CANCEL,
                })
            }
            delay => {
                self.reboot = Reboot::In(delay);
                Some(FirmwareCommand::DevRebootCountdownAns { countdown: delay })
            }
        }
    }

    /// Deletes the image the device holds, section 3.6.
    fn delete(&mut self, version: u32) -> FirmwareCommand {
        let held = self
            .image
            .filter(|image| matches!(image.status, UpImageStatus::Valid));
        let status = match held {
            None => DeleteStatus {
                no_valid_image: true,
                invalid_version: false,
            },
            Some(image) if image.version != version => DeleteStatus {
                no_valid_image: false,
                invalid_version: true,
            },
            Some(_) => {
                self.image = None;
                DeleteStatus::default()
            }
        };
        FirmwareCommand::DevDeleteImageAns(status)
    }
}
