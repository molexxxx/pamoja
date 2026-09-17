//! Remote multicast setup, TS005-2.0.0.
//!
//! Sending the same firmware image to a thousand devices one at a time would take the band
//! for days. Instead the server gives them all one multicast address and one key, then opens
//! a window in which they listen together. This package is how that group is set up: the
//! server creates it with [`McCommand::McGroupSetupReq`], carrying the group's key wrapped
//! under a key only the device holds, and later opens a Class C or Class B session on it.
//!
//! The keys come down a chain no session key ever leaves:
//!
//! - [`mc_root_key`] from the device's own root key, then [`mc_ke_key`] from that: the key
//!   the group keys travel under, which lives as long as the device does.
//! - [`mc_key`] unwraps the group key the setup carried.
//! - [`mc_app_s_key`] and [`mc_nwk_s_key`] derive the pair that reads the group's frames,
//!   from the group key and the group's address.
//!
//! # Examples
//!
//! A server wraps a group key, and the device unwraps it and derives the session:
//!
//! ```
//! use pamoja_lorawan::packages::multicast::{
//!     mc_app_s_key, mc_ke_key, mc_key, mc_nwk_s_key, mc_root_key, wrap_mc_key,
//! };
//!
//! let app_key = [0x2B; 16];
//! let group_key = [0x77; 16];
//! let ke_key = mc_ke_key(&mc_root_key(&app_key));
//!
//! let wrapped = wrap_mc_key(&ke_key, &group_key);
//! assert_eq!(mc_key(&ke_key, &wrapped), group_key, "the device gets it back");
//!
//! let addr = 0x2601_1BDA;
//! assert_ne!(mc_app_s_key(&group_key, addr), mc_nwk_s_key(&group_key, addr));
//! ```

use super::{payload, write, PackageVersion};
use crate::crypto::Cipher;
use crate::{Direction, LorawanError};

/// The port the remote multicast setup package is spoken on, section 4.
pub const PORT: u8 = 200;

/// The identifier of this package, section 4.
pub const PACKAGE: u8 = 2;

/// The version of this package implemented here, section 4.
pub const VERSION: u8 = 2;

/// How many multicast groups a device may hold at once, section 4.3.
pub const GROUPS: usize = 4;

/// The longest command of this package, in bytes.
pub const MAX_COMMAND: usize = 30;

const CID_PACKAGE_VERSION: u8 = 0x00;
const CID_GROUP_STATUS: u8 = 0x01;
const CID_GROUP_SETUP: u8 = 0x02;
const CID_GROUP_DELETE: u8 = 0x03;
const CID_CLASS_C_SESSION: u8 = 0x04;
const CID_CLASS_B_SESSION: u8 = 0x05;

/// Derives a device's multicast root key, section 4.3.
///
/// # Arguments
///
/// * `root_key` - the device's `GenAppKey` on LoRaWAN 1.0.x, or its `AppKey` on 1.1.
/// * `lorawan_11` - whether the device follows the 1.1 scheme, which uses a different
///   constant.
///
/// # Returns
///
/// The `McRootKey`, which nothing but [`mc_ke_key`] is taken from.
#[must_use]
pub fn mc_root_key_for(root_key: &[u8; 16], lorawan_11: bool) -> [u8; 16] {
    let mut block = [0u8; 16];
    block[0] = if lorawan_11 { 0x20 } else { 0x00 };
    Cipher::new(root_key).encrypt_block(&block)
}

/// Derives a device's multicast root key on the LoRaWAN 1.0.x scheme, section 4.3.
///
/// # Arguments
///
/// * `gen_app_key` - the `GenAppKey` provisioned into the device.
///
/// # Returns
///
/// The `McRootKey`.
#[must_use]
pub fn mc_root_key(gen_app_key: &[u8; 16]) -> [u8; 16] {
    mc_root_key_for(gen_app_key, false)
}

/// Derives the key a group's key travels under, section 4.3.
///
/// # Arguments
///
/// * `mc_root_key` - the key [`mc_root_key`] derived.
///
/// # Returns
///
/// The `McKEKey`, which lives as long as the device.
#[must_use]
pub fn mc_ke_key(mc_root_key: &[u8; 16]) -> [u8; 16] {
    Cipher::new(mc_root_key).encrypt_block(&[0u8; 16])
}

/// Unwraps the group key a setup command carried, section 4.3.
///
/// # Arguments
///
/// * `mc_ke_key` - the key it travels under.
/// * `wrapped` - the sixteen bytes the command carried.
///
/// # Returns
///
/// The group's `McKey`.
#[must_use]
pub fn mc_key(mc_ke_key: &[u8; 16], wrapped: &[u8; 16]) -> [u8; 16] {
    Cipher::new(mc_ke_key).encrypt_block(wrapped)
}

/// Wraps a group key for a device, which is what a server does before sending it.
///
/// # Arguments
///
/// * `mc_ke_key` - the device's key encryption key.
/// * `mc_key` - the group key.
///
/// # Returns
///
/// The sixteen bytes the setup command carries.
#[must_use]
pub fn wrap_mc_key(mc_ke_key: &[u8; 16], mc_key: &[u8; 16]) -> [u8; 16] {
    Cipher::new(mc_ke_key).decrypt_block(mc_key)
}

/// Derives the key that reads a group's payloads, section 4.3.
///
/// # Arguments
///
/// * `mc_key` - the group key.
/// * `mc_addr` - the group's address.
///
/// # Returns
///
/// The `McAppSKey`.
#[must_use]
pub fn mc_app_s_key(mc_key: &[u8; 16], mc_addr: u32) -> [u8; 16] {
    session_key(mc_key, 0x01, mc_addr)
}

/// Derives the key that verifies a group's frames, section 4.3.
///
/// # Arguments
///
/// * `mc_key` - the group key.
/// * `mc_addr` - the group's address.
///
/// # Returns
///
/// The `McNwkSKey`.
#[must_use]
pub fn mc_nwk_s_key(mc_key: &[u8; 16], mc_addr: u32) -> [u8; 16] {
    session_key(mc_key, 0x02, mc_addr)
}

/// One of the two group session keys.
fn session_key(mc_key: &[u8; 16], tag: u8, mc_addr: u32) -> [u8; 16] {
    let mut block = [0u8; 16];
    block[0] = tag;
    block[1..5].copy_from_slice(&mc_addr.to_le_bytes());
    Cipher::new(mc_key).encrypt_block(&block)
}

/// What a device makes of a session a server asked it to open, table 20.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SessionStatus {
    /// Which group the session is on.
    pub mc_group_id: u8,
    /// The data rate is not one the device has.
    pub dr_error: bool,
    /// The frequency is not one the device can use.
    pub freq_error: bool,
    /// The device holds no such group.
    pub group_undefined: bool,
    /// The session was to start at a time already past.
    pub start_missed: bool,
}

impl SessionStatus {
    /// Whether the device will open the session.
    ///
    /// # Returns
    ///
    /// `true` when no error bit is set, in which case the answer carries when it starts.
    #[must_use]
    pub const fn accepted(&self) -> bool {
        !self.dr_error && !self.freq_error && !self.group_undefined && !self.start_missed
    }

    /// Reads the coded status.
    const fn from_bits(bits: u8) -> SessionStatus {
        SessionStatus {
            mc_group_id: bits & 0x03,
            dr_error: bits & 0x04 != 0,
            freq_error: bits & 0x08 != 0,
            group_undefined: bits & 0x10 != 0,
            start_missed: bits & 0x20 != 0,
        }
    }

    /// Writes the coded status.
    const fn bits(&self) -> u8 {
        (self.mc_group_id & 0x03)
            | ((self.dr_error as u8) << 2)
            | ((self.freq_error as u8) << 3)
            | ((self.group_undefined as u8) << 4)
            | ((self.start_missed as u8) << 5)
    }
}

/// One command of the remote multicast setup package.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum McCommand {
    /// The server asking which version of this package the device implements.
    PackageVersionReq,
    /// The device's answer, section 4.1.
    PackageVersionAns(PackageVersion),
    /// The server asking which groups the device holds, section 4.2.
    McGroupStatusReq {
        /// Which groups to report on, a bit for each.
        req_group_mask: u8,
    },
    /// The device's answer, section 4.2. The groups themselves follow in the message.
    McGroupStatusAns {
        /// Which groups this answer covers, a bit for each.
        ans_group_mask: u8,
        /// How many groups the device holds in all.
        nb_total_groups: u8,
    },
    /// One group of a status answer: its identifier and its address.
    McGroupStatusItem {
        /// Which group.
        mc_group_id: u8,
        /// Its address.
        mc_addr: u32,
    },
    /// The server creating or changing a group, section 4.3.
    McGroupSetupReq {
        /// Which group, 0 to 3.
        mc_group_id: u8,
        /// The address the group answers to.
        mc_addr: u32,
        /// The group key, wrapped under the device's key encryption key.
        mc_key_encrypted: [u8; 16],
        /// The first frame counter the device accepts from the group.
        min_mc_fcnt: u32,
        /// The last one, which ends the group's life.
        max_mc_fcnt: u32,
    },
    /// The device's answer, section 4.3.
    McGroupSetupAns {
        /// Which group.
        mc_group_id: u8,
        /// Whether the device holds no group by that identifier.
        id_error: bool,
    },
    /// The server deleting a group, section 4.4.
    McGroupDeleteReq {
        /// Which group.
        mc_group_id: u8,
    },
    /// The device's answer, section 4.4.
    McGroupDeleteAns {
        /// Which group.
        mc_group_id: u8,
        /// Whether the device held no such group.
        group_undefined: bool,
    },
    /// The server opening a Class C window on a group, section 4.5.
    McClassCSessionReq {
        /// Which group.
        mc_group_id: u8,
        /// When the window opens, in seconds since the GPS epoch.
        session_time: u32,
        /// How long it lasts at most: two to the power of this, in seconds.
        time_out: u8,
        /// Where the group listens, in hertz.
        dl_frequency_hz: u32,
        /// The data rate it listens at.
        data_rate: u8,
    },
    /// The device's answer, section 4.5.
    McClassCSessionAns {
        /// What the device made of it.
        status: SessionStatus,
        /// How many seconds until the window opens, for a session it will open.
        time_to_start: Option<u32>,
    },
    /// The server opening a Class B window on a group, section 4.6.
    McClassBSessionReq {
        /// Which group.
        mc_group_id: u8,
        /// When the window opens, a multiple of the beacon period.
        session_time: u32,
        /// How long it lasts at most: 128 seconds times two to the power of this.
        time_out: u8,
        /// How often the device opens a ping slot inside it.
        periodicity: u8,
        /// Where the group listens, in hertz.
        dl_frequency_hz: u32,
        /// The data rate it listens at.
        data_rate: u8,
    },
    /// The device's answer, section 4.6.
    McClassBSessionAns {
        /// What the device made of it.
        status: SessionStatus,
        /// How many seconds until the window opens, for a session it will open.
        time_to_start: Option<u32>,
    },
}

impl McCommand {
    /// The identifier this command travels under.
    ///
    /// # Returns
    ///
    /// The identifier, 0x00 to 0x05.
    #[must_use]
    pub const fn cid(&self) -> u8 {
        match self {
            McCommand::PackageVersionReq | McCommand::PackageVersionAns(_) => CID_PACKAGE_VERSION,
            McCommand::McGroupStatusReq { .. }
            | McCommand::McGroupStatusAns { .. }
            | McCommand::McGroupStatusItem { .. } => CID_GROUP_STATUS,
            McCommand::McGroupSetupReq { .. } | McCommand::McGroupSetupAns { .. } => {
                CID_GROUP_SETUP
            }
            McCommand::McGroupDeleteReq { .. } | McCommand::McGroupDeleteAns { .. } => {
                CID_GROUP_DELETE
            }
            McCommand::McClassCSessionReq { .. } | McCommand::McClassCSessionAns { .. } => {
                CID_CLASS_C_SESSION
            }
            McCommand::McClassBSessionReq { .. } | McCommand::McClassBSessionAns { .. } => {
                CID_CLASS_B_SESSION
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
            McCommand::PackageVersionAns(_)
            | McCommand::McGroupStatusAns { .. }
            | McCommand::McGroupStatusItem { .. }
            | McCommand::McGroupSetupAns { .. }
            | McCommand::McGroupDeleteAns { .. }
            | McCommand::McClassCSessionAns { .. }
            | McCommand::McClassBSessionAns { .. } => Direction::Uplink,
            _ => Direction::Downlink,
        }
    }

    /// Reads one command from the front of a message.
    ///
    /// A status answer reads only its own byte: the groups it covers follow as
    /// [`McCommand::McGroupStatusItem`] records, five bytes each, which
    /// [`McCommand::status_item`] reads.
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
    pub fn parse(direction: Direction, bytes: &[u8]) -> Result<(McCommand, usize), LorawanError> {
        let cid = *bytes.first().ok_or(LorawanError::MalformedFrame)?;
        let up = matches!(direction, Direction::Uplink);
        match (cid, up) {
            (CID_PACKAGE_VERSION, false) => Ok((McCommand::PackageVersionReq, 1)),
            (CID_PACKAGE_VERSION, true) => {
                let (fields, taken) = payload(bytes, 2)?;
                Ok((
                    McCommand::PackageVersionAns(PackageVersion {
                        package: fields[0],
                        version: fields[1],
                    }),
                    taken,
                ))
            }
            (CID_GROUP_STATUS, false) => {
                let (fields, taken) = payload(bytes, 1)?;
                Ok((
                    McCommand::McGroupStatusReq {
                        req_group_mask: fields[0] & 0x0F,
                    },
                    taken,
                ))
            }
            (CID_GROUP_STATUS, true) => {
                let (fields, taken) = payload(bytes, 1)?;
                Ok((
                    McCommand::McGroupStatusAns {
                        ans_group_mask: fields[0] & 0x0F,
                        nb_total_groups: (fields[0] >> 4) & 0x07,
                    },
                    taken,
                ))
            }
            (CID_GROUP_SETUP, false) => {
                let (fields, taken) = payload(bytes, 29)?;
                let mut key = [0u8; 16];
                key.copy_from_slice(&fields[5..21]);
                Ok((
                    McCommand::McGroupSetupReq {
                        mc_group_id: fields[0] & 0x03,
                        mc_addr: u32::from_le_bytes([fields[1], fields[2], fields[3], fields[4]]),
                        mc_key_encrypted: key,
                        min_mc_fcnt: u32::from_le_bytes([
                            fields[21], fields[22], fields[23], fields[24],
                        ]),
                        max_mc_fcnt: u32::from_le_bytes([
                            fields[25], fields[26], fields[27], fields[28],
                        ]),
                    },
                    taken,
                ))
            }
            (CID_GROUP_SETUP, true) => {
                let (fields, taken) = payload(bytes, 1)?;
                Ok((
                    McCommand::McGroupSetupAns {
                        mc_group_id: fields[0] & 0x03,
                        id_error: fields[0] & 0x04 != 0,
                    },
                    taken,
                ))
            }
            (CID_GROUP_DELETE, false) => {
                let (fields, taken) = payload(bytes, 1)?;
                Ok((
                    McCommand::McGroupDeleteReq {
                        mc_group_id: fields[0] & 0x03,
                    },
                    taken,
                ))
            }
            (CID_GROUP_DELETE, true) => {
                let (fields, taken) = payload(bytes, 1)?;
                Ok((
                    McCommand::McGroupDeleteAns {
                        mc_group_id: fields[0] & 0x03,
                        group_undefined: fields[0] & 0x04 != 0,
                    },
                    taken,
                ))
            }
            (CID_CLASS_C_SESSION, false) => {
                let (fields, taken) = payload(bytes, 10)?;
                Ok((
                    McCommand::McClassCSessionReq {
                        mc_group_id: fields[0] & 0x03,
                        session_time: u32::from_le_bytes([
                            fields[1], fields[2], fields[3], fields[4],
                        ]),
                        time_out: fields[5] & 0x0F,
                        dl_frequency_hz: frequency(&fields[6..9]),
                        data_rate: fields[9],
                    },
                    taken,
                ))
            }
            (CID_CLASS_B_SESSION, false) => {
                let (fields, taken) = payload(bytes, 10)?;
                Ok((
                    McCommand::McClassBSessionReq {
                        mc_group_id: fields[0] & 0x03,
                        session_time: u32::from_le_bytes([
                            fields[1], fields[2], fields[3], fields[4],
                        ]),
                        time_out: fields[5] & 0x0F,
                        periodicity: (fields[5] >> 4) & 0x07,
                        dl_frequency_hz: frequency(&fields[6..9]),
                        data_rate: fields[9],
                    },
                    taken,
                ))
            }
            (CID_CLASS_C_SESSION | CID_CLASS_B_SESSION, true) => {
                let (fields, _) = payload(bytes, 1)?;
                let status = SessionStatus::from_bits(fields[0]);
                let (time_to_start, taken) = if status.accepted() {
                    let (fields, taken) = payload(bytes, 4)?;
                    (
                        Some(u32::from_le_bytes([fields[1], fields[2], fields[3], 0])),
                        taken,
                    )
                } else {
                    (None, 2)
                };
                Ok((
                    if cid == CID_CLASS_C_SESSION {
                        McCommand::McClassCSessionAns {
                            status,
                            time_to_start,
                        }
                    } else {
                        McCommand::McClassBSessionAns {
                            status,
                            time_to_start,
                        }
                    },
                    taken,
                ))
            }
            _ => Err(LorawanError::UnknownCommand(cid)),
        }
    }

    /// Reads one group record of a status answer, section 4.2.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the message from the record on, five bytes or more.
    ///
    /// # Returns
    ///
    /// The record and how many bytes it took.
    ///
    /// # Errors
    ///
    /// [`LorawanError::MalformedFrame`] when the message ends inside the record.
    pub fn status_item(bytes: &[u8]) -> Result<(McCommand, usize), LorawanError> {
        let fields = bytes.get(..5).ok_or(LorawanError::MalformedFrame)?;
        Ok((
            McCommand::McGroupStatusItem {
                mc_group_id: fields[0] & 0x03,
                mc_addr: u32::from_le_bytes([fields[1], fields[2], fields[3], fields[4]]),
            },
            5,
        ))
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
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, LorawanError> {
        match self {
            McCommand::PackageVersionReq => write(out, CID_PACKAGE_VERSION, &[]),
            McCommand::PackageVersionAns(version) => write(
                out,
                CID_PACKAGE_VERSION,
                &[version.package, version.version],
            ),
            McCommand::McGroupStatusReq { req_group_mask } => {
                write(out, CID_GROUP_STATUS, &[req_group_mask & 0x0F])
            }
            McCommand::McGroupStatusAns {
                ans_group_mask,
                nb_total_groups,
            } => write(
                out,
                CID_GROUP_STATUS,
                &[(ans_group_mask & 0x0F) | ((nb_total_groups & 0x07) << 4)],
            ),
            McCommand::McGroupStatusItem {
                mc_group_id,
                mc_addr,
            } => {
                let addr = mc_addr.to_le_bytes();
                let room = out.get_mut(..5).ok_or(LorawanError::PayloadTooLong)?;
                room[0] = mc_group_id & 0x03;
                room[1..5].copy_from_slice(&addr);
                Ok(5)
            }
            McCommand::McGroupSetupReq {
                mc_group_id,
                mc_addr,
                mc_key_encrypted,
                min_mc_fcnt,
                max_mc_fcnt,
            } => {
                let mut fields = [0u8; 29];
                fields[0] = mc_group_id & 0x03;
                fields[1..5].copy_from_slice(&mc_addr.to_le_bytes());
                fields[5..21].copy_from_slice(mc_key_encrypted);
                fields[21..25].copy_from_slice(&min_mc_fcnt.to_le_bytes());
                fields[25..29].copy_from_slice(&max_mc_fcnt.to_le_bytes());
                write(out, CID_GROUP_SETUP, &fields)
            }
            McCommand::McGroupSetupAns {
                mc_group_id,
                id_error,
            } => write(
                out,
                CID_GROUP_SETUP,
                &[(mc_group_id & 0x03) | (u8::from(*id_error) << 2)],
            ),
            McCommand::McGroupDeleteReq { mc_group_id } => {
                write(out, CID_GROUP_DELETE, &[mc_group_id & 0x03])
            }
            McCommand::McGroupDeleteAns {
                mc_group_id,
                group_undefined,
            } => write(
                out,
                CID_GROUP_DELETE,
                &[(mc_group_id & 0x03) | (u8::from(*group_undefined) << 2)],
            ),
            McCommand::McClassCSessionReq {
                mc_group_id,
                session_time,
                time_out,
                dl_frequency_hz,
                data_rate,
            } => {
                let fields = session_fields(
                    *mc_group_id,
                    *session_time,
                    time_out & 0x0F,
                    *dl_frequency_hz,
                    *data_rate,
                );
                write(out, CID_CLASS_C_SESSION, &fields)
            }
            McCommand::McClassBSessionReq {
                mc_group_id,
                session_time,
                time_out,
                periodicity,
                dl_frequency_hz,
                data_rate,
            } => {
                let fields = session_fields(
                    *mc_group_id,
                    *session_time,
                    (time_out & 0x0F) | ((periodicity & 0x07) << 4),
                    *dl_frequency_hz,
                    *data_rate,
                );
                write(out, CID_CLASS_B_SESSION, &fields)
            }
            McCommand::McClassCSessionAns {
                status,
                time_to_start,
            } => session_answer(out, CID_CLASS_C_SESSION, *status, *time_to_start),
            McCommand::McClassBSessionAns {
                status,
                time_to_start,
            } => session_answer(out, CID_CLASS_B_SESSION, *status, *time_to_start),
        }
    }
}

/// Reads a three-byte frequency, in hundreds of hertz as `NewChannelReq` codes it.
fn frequency(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0]) * 100
}

/// The fields a session request carries, which the two classes share.
fn session_fields(
    mc_group_id: u8,
    session_time: u32,
    coded_timeout: u8,
    dl_frequency_hz: u32,
    data_rate: u8,
) -> [u8; 10] {
    let mut fields = [0u8; 10];
    fields[0] = mc_group_id & 0x03;
    fields[1..5].copy_from_slice(&session_time.to_le_bytes());
    fields[5] = coded_timeout;
    let coded = (dl_frequency_hz / 100).to_le_bytes();
    fields[6..9].copy_from_slice(&coded[..3]);
    fields[9] = data_rate;
    fields
}

/// Writes a session answer, which carries its start only when the device will open it.
fn session_answer(
    out: &mut [u8],
    cid: u8,
    status: SessionStatus,
    time_to_start: Option<u32>,
) -> Result<usize, LorawanError> {
    match time_to_start.filter(|_| status.accepted()) {
        Some(start) => {
            let coded = start.to_le_bytes();
            write(out, cid, &[status.bits(), coded[0], coded[1], coded[2]])
        }
        None => write(out, cid, &[status.bits()]),
    }
}
