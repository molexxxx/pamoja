//! A SocketCAN raw socket, the one place this crate reaches the kernel's CAN stack.
//!
//! Frames cross as the kernel's `struct can_frame`, 16 bytes, and `struct canfd_frame`, 72,
//! from `include/uapi/linux/can.h`: the identifier with its format and remote flags in host
//! byte order, the payload length, and the payload at offset 8. Binding the socket and setting
//! its options take a `sockaddr_can` and option values the safe wrappers do not cover, so those
//! three calls are `unsafe`; everything else goes through `nix` and the standard library.

#![allow(unsafe_code)]

use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsFd, AsRawFd, OwnedFd};
use std::time::{Duration, Instant};

use nix::errno::Errno;
use nix::net::if_::if_nametoindex;
use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
use nix::sys::socket::{socket, AddressFamily, SockFlag, SockProtocol, SockType};

use super::Filter;
use crate::frame::Frame;
use crate::id::CanId;

const CAN_MTU: usize = libc::CAN_MTU;
const CANFD_MTU: usize = libc::CANFD_MTU;
const DATA: usize = 8;

/// One raw socket bound to one interface. Reads and writes take `&self`, so one thread may
/// receive while another sends.
pub(super) struct Socket {
    file: File,
}

fn check(result: libc::c_int) -> io::Result<()> {
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

impl Socket {
    /// Opens a raw socket on an interface, taking CAN FD frames too where the kernel allows it.
    pub(super) fn open(interface: &str) -> io::Result<Socket> {
        let index = if_nametoindex(interface)?;
        let fd: OwnedFd = socket(
            AddressFamily::Can,
            SockType::Raw,
            SockFlag::SOCK_CLOEXEC,
            SockProtocol::CanRaw,
        )?;
        let on: libc::c_int = 1;
        // SAFETY: the pointer and length describe `on`, an int that outlives the call, which
        // is the value CAN_RAW_FD_FRAMES takes. A kernel without CAN FD refuses the option,
        // which leaves the socket on classic frames, so the result is not an error here.
        let _ = unsafe {
            libc::setsockopt(
                fd.as_raw_fd(),
                libc::SOL_CAN_RAW,
                libc::CAN_RAW_FD_FRAMES,
                (&on as *const libc::c_int).cast(),
                size_of::<libc::c_int>() as libc::socklen_t,
            )
        };
        // SAFETY: sockaddr_can is plain integers and a union of plain integers, for which all
        // zero bytes is a valid value, and the fields that matter are set before it is used.
        let mut address: libc::sockaddr_can = unsafe { std::mem::zeroed() };
        address.can_family = libc::AF_CAN as libc::sa_family_t;
        address.can_ifindex = index as libc::c_int;
        // SAFETY: the pointer and length describe `address`, a sockaddr_can that outlives the
        // call, as bind expects for an AF_CAN socket.
        check(unsafe {
            libc::bind(
                fd.as_raw_fd(),
                (&address as *const libc::sockaddr_can).cast(),
                size_of::<libc::sockaddr_can>() as libc::socklen_t,
            )
        })?;
        Ok(Socket {
            file: File::from(fd),
        })
    }

    /// Writes one frame, in the kernel's layout for its kind.
    pub(super) fn send(&self, frame: &Frame) -> io::Result<()> {
        let mut bytes = [0u8; CANFD_MTU];
        let mut word = frame.id().raw();
        if frame.id().is_extended() {
            word |= libc::CAN_EFF_FLAG;
        }
        if frame.is_remote() {
            word |= libc::CAN_RTR_FLAG;
        }
        bytes[..4].copy_from_slice(&word.to_ne_bytes());
        bytes[4] = frame.len() as u8;
        bytes[DATA..DATA + frame.data().len()].copy_from_slice(frame.data());
        let size = if frame.is_fd() { CANFD_MTU } else { CAN_MTU };
        let written = (&self.file).write(&bytes[..size])?;
        if written == size {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "the kernel took part of a frame",
            ))
        }
    }

    /// Waits up to `timeout` for a frame and reads it, or returns `None` when none came.
    pub(super) fn receive(&self, timeout: Duration) -> io::Result<Option<Frame>> {
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let millis = i32::try_from(left.as_micros().div_ceil(1_000)).unwrap_or(i32::MAX);
            let wait = PollTimeout::try_from(millis).unwrap_or(PollTimeout::MAX);
            let mut ready = [PollFd::new(self.file.as_fd(), PollFlags::POLLIN)];
            match poll(&mut ready, wait) {
                Ok(0) => return Ok(None),
                Ok(_) => break,
                Err(Errno::EINTR) => continue,
                Err(error) => return Err(error.into()),
            }
        }
        let mut bytes = [0u8; CANFD_MTU];
        let got = (&self.file).read(&mut bytes)?;
        let word = u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let id = if word & libc::CAN_EFF_FLAG != 0 {
            CanId::extended(word)
        } else {
            CanId::standard(word as u16)
        };
        let len = usize::from(bytes[4]);
        let frame = match got {
            CAN_MTU if word & libc::CAN_RTR_FLAG != 0 => Ok(Frame::remote(id, len)),
            CAN_MTU => Frame::new(id, &bytes[DATA..DATA + len.min(8)]),
            CANFD_MTU => Frame::fd(id, &bytes[DATA..DATA + len.min(64)]),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("a read of {got} bytes is not a CAN frame"),
                ))
            }
        }
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
        Ok(Some(frame))
    }

    /// Sets the socket's receive filters: `None` for every frame, and an empty list for none.
    pub(super) fn set_filters(&self, filters: Option<&[Filter]>) -> io::Result<()> {
        let filters: Vec<libc::can_filter> = match filters {
            None => vec![libc::can_filter {
                can_id: 0,
                can_mask: 0,
            }],
            Some(filters) => filters
                .iter()
                .map(|filter| {
                    let format = if filter.id().is_extended() {
                        libc::CAN_EFF_FLAG
                    } else {
                        0
                    };
                    libc::can_filter {
                        can_id: filter.id().raw() | format,
                        can_mask: filter.mask() | libc::CAN_EFF_FLAG,
                    }
                })
                .collect(),
        };
        // SAFETY: the pointer and length describe `filters`, an array of can_filter that
        // outlives the call, which is the value CAN_RAW_FILTER takes; a length of zero with
        // any pointer sets no filter, which keeps nothing.
        check(unsafe {
            libc::setsockopt(
                self.file.as_raw_fd(),
                libc::SOL_CAN_RAW,
                libc::CAN_RAW_FILTER,
                filters.as_ptr().cast(),
                (filters.len() * size_of::<libc::can_filter>()) as libc::socklen_t,
            )
        })
    }
}
