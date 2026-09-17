//! The LoRaWAN application layer packages: what a server and a device say to each other on
//! top of the link layer, TS003, TS004, TS005 and TS006.
//!
//! A package is a small command language spoken on a port of its own. Every package shares
//! the same shape: a message carries one or more commands, each an identifier byte followed
//! by a payload whose length the identifier fixes, and a device runs them in order and
//! answers each one. The identifier means one command going down and another coming up, so
//! reading a message takes the [`Direction`](crate::Direction) it traveled, exactly as a MAC
//! command does.
//!
//! Each package announces itself: [`PackageVersion`] is the answer every package gives to the
//! version request every package defines, carrying the identifier of the package and the
//! version of it the device implements.
//!
//! - [`clock`]: application layer clock synchronization, TS003-2.0.0, on port
//!   [`clock::PORT`]. A device tells the server what time it thinks it is and the server
//!   answers with the correction.
//! - [`firmware`]: firmware management, TS006-1.0.0, on port [`firmware::PORT`]. A server
//!   asks what a device is running, what upgrade image it holds, and schedules the reboot
//!   that installs it.
//! - [`fragment`]: fragmented data block transport, TS004-2.0.0, on port [`fragment::PORT`].
//!   A block too large for one frame goes across in pieces, with coded fragments that let a
//!   device solve for the ones it missed.
//! - [`multicast`]: remote multicast setup, TS005-2.0.0, on port [`multicast::PORT`]. A
//!   group of devices is given one address and one key, and a window in which they all
//!   listen at once.
//!
//! These messages are unicast: a device drops them silently when they arrive on a multicast
//! address.

use crate::LorawanError;

pub mod clock;
pub mod firmware;
pub mod fragment;
pub mod multicast;

#[cfg(test)]
mod fragment_tests;
#[cfg(test)]
mod multicast_tests;
#[cfg(test)]
mod tests;

/// The answer every package gives to its own version request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PackageVersion {
    /// Which package this is: 1 for clock synchronization, 4 for firmware management.
    pub package: u8,
    /// The version of that package the device implements.
    pub version: u8,
}

/// Reads a command's identifier and its payload of a fixed length.
///
/// # Arguments
///
/// * `bytes` - the message from the identifier on.
/// * `len` - how many payload bytes the command carries.
///
/// # Returns
///
/// The payload and how many bytes the whole command took.
///
/// # Errors
///
/// [`LorawanError::MalformedFrame`] when the message ends inside the command.
pub(crate) fn payload(bytes: &[u8], len: usize) -> Result<(&[u8], usize), LorawanError> {
    bytes
        .get(1..=len)
        .map(|payload| (payload, len + 1))
        .ok_or(LorawanError::MalformedFrame)
}

/// Writes a command's identifier and payload.
///
/// # Arguments
///
/// * `out` - where to write.
/// * `cid` - the command identifier.
/// * `fields` - its payload.
///
/// # Returns
///
/// How many bytes were written.
///
/// # Errors
///
/// [`LorawanError::PayloadTooLong`] when the buffer is too small.
pub(crate) fn write(out: &mut [u8], cid: u8, fields: &[u8]) -> Result<usize, LorawanError> {
    let len = fields.len() + 1;
    let room = out.get_mut(..len).ok_or(LorawanError::PayloadTooLong)?;
    room[0] = cid;
    room[1..].copy_from_slice(fields);
    Ok(len)
}
