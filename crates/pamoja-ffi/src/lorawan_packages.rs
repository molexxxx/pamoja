//! The C ABI for the LoRaWAN application layer packages: clock synchronization TS003-2.0.0,
//! fragmented data block transport TS004-2.0.0, remote multicast setup TS005-2.0.0, and
//! firmware management TS006-2.0.0.
//!
//! The key derivations and the parity matrix are pure functions of their arguments. What has
//! to remember something crosses as an opaque handle: a clock synchronization package, a
//! firmware manager, a fragmentation session being put back together, and the code taken
//! over a block as it arrives.

use std::ptr;

use pamoja_lorawan::packages::clock::{ClockSync, PORT as CLOCK_PORT};
use pamoja_lorawan::packages::firmware::{
    FirmwareManager, Image, UpImageStatus, PORT as FIRMWARE_PORT,
};
use pamoja_lorawan::packages::fragment::{
    data_block_int_key, parity_line, prbs23, BlockMic, BlockMicKey, Defragmenter, FragError,
    Fragmenter, Progress, MAX_FRAGMENTS, PORT as FRAGMENT_PORT,
};
use pamoja_lorawan::packages::multicast::{
    mc_app_s_key, mc_ke_key, mc_key, mc_nwk_s_key, mc_root_key_for, wrap_mc_key,
    PORT as MULTICAST_PORT,
};

use pamoja_lorawan::packages::clock::ClockCommand;
use pamoja_lorawan::packages::firmware::{DeleteStatus, FirmwareCommand};
use pamoja_lorawan::packages::fragment::{FragCommand, SetupStatus};
use pamoja_lorawan::packages::multicast::{McCommand, SessionStatus};
use pamoja_lorawan::packages::PackageVersion;
use pamoja_lorawan::Direction;

use crate::{read_bytes, set_last_error, PamojaStatus};

/// The port clock synchronization is spoken on, TS003-2.0.0.
pub const PAMOJA_LORAWAN_CLOCK_PORT: u8 = CLOCK_PORT;
/// The port fragmented data block transport is spoken on, TS004-2.0.0.
pub const PAMOJA_LORAWAN_FRAGMENT_PORT: u8 = FRAGMENT_PORT;
/// The port remote multicast setup is spoken on, TS005-2.0.0.
pub const PAMOJA_LORAWAN_MULTICAST_PORT: u8 = MULTICAST_PORT;
/// The port firmware management is spoken on, TS006-1.0.0.
pub const PAMOJA_LORAWAN_FIRMWARE_PORT: u8 = FIRMWARE_PORT;
/// The most fragments one session carries.
pub const PAMOJA_LORAWAN_MAX_FRAGMENTS: u16 = MAX_FRAGMENTS;

/// The device holds no firmware upgrade image.
pub const PAMOJA_LORAWAN_IMAGE_NONE: u8 = 0;
/// One is there, but it is corrupt or its signature does not verify.
pub const PAMOJA_LORAWAN_IMAGE_CORRUPT: u8 = 1;
/// One is there and authentic, but it is not for this hardware.
pub const PAMOJA_LORAWAN_IMAGE_WRONG_HARDWARE: u8 = 2;
/// One is there, and it can be installed.
pub const PAMOJA_LORAWAN_IMAGE_VALID: u8 = 3;

/// Derives a device's multicast root key, TS005-2.0.0 section 4.3.
///
/// # Arguments
///
/// * `root_key` - the device's sixteen-byte `GenAppKey` on LoRaWAN 1.0.x, or its `AppKey`
///   on 1.1.
/// * `lorawan_11` - `1` for the 1.1 scheme, which starts from another constant.
/// * `out_key` - receives the sixteen-byte `McRootKey`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// `root_key` must point to sixteen readable bytes and `out_key` to sixteen writable ones.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_mc_root_key(
    root_key: *const u8,
    lorawan_11: u8,
    out_key: *mut u8,
) -> PamojaStatus {
    derive(root_key, out_key, |key| {
        mc_root_key_for(key, lorawan_11 != 0)
    })
}

/// Derives the key a multicast group's key travels under, section 4.3.
///
/// # Arguments
///
/// * `mc_root_key` - the sixteen-byte key [`pamoja_lorawan_mc_root_key`] derived.
/// * `out_key` - receives the sixteen-byte `McKEKey`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// Both pointers must be valid for sixteen bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_mc_ke_key(
    mc_root_key: *const u8,
    out_key: *mut u8,
) -> PamojaStatus {
    derive(mc_root_key, out_key, mc_ke_key)
}

/// Unwraps the group key a setup command carried, section 4.3.
///
/// # Arguments
///
/// * `mc_ke_key` - the sixteen-byte key it travels under.
/// * `wrapped` - the sixteen bytes the command carried.
/// * `out_key` - receives the group's sixteen-byte `McKey`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// Every pointer must be valid for sixteen bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_mc_key(
    mc_ke_key: *const u8,
    wrapped: *const u8,
    out_key: *mut u8,
) -> PamojaStatus {
    let (Some(ke), Some(wrapped), false) =
        (sixteen(mc_ke_key), sixteen(wrapped), out_key.is_null())
    else {
        return missing();
    };
    ptr::copy_nonoverlapping(mc_key(&ke, &wrapped).as_ptr(), out_key, 16);
    PamojaStatus::Ok
}

/// Wraps a group key for a device, which is what a server does before sending it.
///
/// # Arguments
///
/// * `mc_ke_key` - the device's sixteen-byte key encryption key.
/// * `mc_key` - the sixteen-byte group key.
/// * `out_wrapped` - receives the sixteen bytes a setup command carries.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// Every pointer must be valid for sixteen bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_wrap_mc_key(
    mc_ke_key: *const u8,
    mc_key: *const u8,
    out_wrapped: *mut u8,
) -> PamojaStatus {
    let (Some(ke), Some(key), false) = (sixteen(mc_ke_key), sixteen(mc_key), out_wrapped.is_null())
    else {
        return missing();
    };
    ptr::copy_nonoverlapping(wrap_mc_key(&ke, &key).as_ptr(), out_wrapped, 16);
    PamojaStatus::Ok
}

/// Derives the key that reads a multicast group's payloads, section 4.3.
///
/// # Arguments
///
/// * `mc_key` - the sixteen-byte group key.
/// * `mc_addr` - the group's address.
/// * `out_key` - receives the sixteen-byte `McAppSKey`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// Both pointers must be valid for sixteen bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_mc_app_s_key(
    mc_key: *const u8,
    mc_addr: u32,
    out_key: *mut u8,
) -> PamojaStatus {
    derive(mc_key, out_key, |key| mc_app_s_key(key, mc_addr))
}

/// Derives the key that verifies a multicast group's frames, section 4.3.
///
/// # Arguments
///
/// * `mc_key` - the sixteen-byte group key.
/// * `mc_addr` - the group's address.
/// * `out_key` - receives the sixteen-byte `McNwkSKey`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// Both pointers must be valid for sixteen bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_mc_nwk_s_key(
    mc_key: *const u8,
    mc_addr: u32,
    out_key: *mut u8,
) -> PamojaStatus {
    derive(mc_key, out_key, |key| mc_nwk_s_key(key, mc_addr))
}

/// Derives the key that signs a data block, TS004-2.0.0 section 3.3.
///
/// # Arguments
///
/// * `root_key` - the device's sixteen-byte `GenAppKey` or `AppKey`.
/// * `out_key` - receives the sixteen-byte `DataBlockIntKey`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// Both pointers must be valid for sixteen bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_data_block_int_key(
    root_key: *const u8,
    out_key: *mut u8,
) -> PamojaStatus {
    derive(root_key, out_key, data_block_int_key)
}

/// Steps the pseudo-random sequence the parity matrix is drawn from, appendix A.1.
///
/// # Arguments
///
/// * `x` - the current value.
///
/// # Returns
///
/// The next one.
#[no_mangle]
pub extern "C" fn pamoja_lorawan_frag_prbs23(x: u32) -> u32 {
    prbs23(x)
}

/// Builds one row of the parity matrix: which uncoded fragments a coded one is made of.
///
/// # Arguments
///
/// * `coded` - which coded fragment, counting from one past the uncoded ones.
/// * `nb_frag` - how many uncoded fragments the block was cut into.
/// * `out_line` - receives a bit for every uncoded fragment, little end first.
/// * `line_len` - how many bytes that buffer holds, at least `nb_frag` bits' worth.
/// * `out_ones` - receives how many fragments the coded one is made of.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer, a session of no fragments,
/// or a buffer too small.
///
/// # Safety
///
/// `out_line` must point to `line_len` writable bytes and `out_ones` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_frag_parity_line(
    coded: u16,
    nb_frag: u16,
    out_line: *mut u8,
    line_len: usize,
    out_ones: *mut usize,
) -> PamojaStatus {
    if out_line.is_null() || out_ones.is_null() || nb_frag == 0 {
        return missing();
    }
    let needed = usize::from(nb_frag).div_ceil(8);
    if line_len < needed {
        set_last_error(format!("the line needs {needed} bytes, not {line_len}"));
        return PamojaStatus::InvalidArgument;
    }
    let line = core::slice::from_raw_parts_mut(out_line, line_len);
    *out_ones = parity_line(coded, nb_frag, line);
    PamojaStatus::Ok
}

/// How many fragments a block of a given size takes, and how much padding the last one needs.
///
/// # Arguments
///
/// * `block_len` - the block's length in bytes.
/// * `frag_size` - how many bytes each fragment carries.
/// * `out_nb_frag` - receives the fragment count.
/// * `out_padding` - receives the padding the last fragment carries.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer, a fragment size of zero, or
/// a block needing more than [`PAMOJA_LORAWAN_MAX_FRAGMENTS`] fragments.
///
/// # Safety
///
/// Both out pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_frag_session(
    block_len: usize,
    frag_size: u8,
    out_nb_frag: *mut u16,
    out_padding: *mut u8,
) -> PamojaStatus {
    if out_nb_frag.is_null() || out_padding.is_null() || frag_size == 0 || block_len == 0 {
        return missing();
    }
    let size = usize::from(frag_size);
    let nb_frag = block_len.div_ceil(size);
    if nb_frag > MAX_FRAGMENTS as usize {
        set_last_error(format!(
            "{nb_frag} fragments is more than a session carries"
        ));
        return PamojaStatus::InvalidArgument;
    }
    *out_nb_frag = nb_frag as u16;
    *out_padding = (nb_frag * size - block_len) as u8;
    PamojaStatus::Ok
}

/// Builds one fragment of a session out of a block held whole.
///
/// # Arguments
///
/// * `block` - the block to send.
/// * `block_len` - its length.
/// * `frag_size` - how many bytes each fragment carries.
/// * `n` - which fragment, counting from one; past the uncoded ones it is a coded fragment.
/// * `out_fragment` - receives the fragment.
/// * `out_len` - how many bytes that buffer holds, at least `frag_size`.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer, a session this build does
/// not run, or a buffer too small.
///
/// # Safety
///
/// `block` must point to `block_len` readable bytes and `out_fragment` to `out_len` writable
/// ones.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_frag_fragment(
    block: *const u8,
    block_len: usize,
    frag_size: u8,
    n: u16,
    out_fragment: *mut u8,
    out_len: usize,
) -> PamojaStatus {
    let bytes = match read_bytes(block, block_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    if out_fragment.is_null() {
        return missing();
    }
    let Ok(sender) = Fragmenter::new(&bytes, frag_size) else {
        set_last_error("the block and fragment size do not make a session".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let out = core::slice::from_raw_parts_mut(out_fragment, out_len);
    match sender.fragment(n, out) {
        Ok(_) => PamojaStatus::Ok,
        Err(error) => {
            set_last_error(error.to_string());
            PamojaStatus::InvalidArgument
        }
    }
}

/// A fragmentation session being put back together, released with
/// [`pamoja_lorawan_defrag_free`].
pub struct PamojaLorawanDefrag {
    block: Vec<u8>,
    matrix: Vec<u8>,
    nb_frag: u16,
    frag_size: u8,
    progress: Progress,
    missing: u16,
}

/// How many bytes of working storage a session needs.
///
/// # Arguments
///
/// * `nb_frag` - how many uncoded fragments the block was cut into.
/// * `max_lost` - the most uncoded fragments the session should survive losing.
///
/// # Returns
///
/// The byte count, which [`pamoja_lorawan_defrag_new`] allocates for itself.
#[no_mangle]
pub extern "C" fn pamoja_lorawan_defrag_matrix_len(nb_frag: u16, max_lost: u16) -> usize {
    Defragmenter::matrix_len(nb_frag, max_lost)
}

/// Opens a fragmentation session.
///
/// # Arguments
///
/// * `nb_frag` - how many uncoded fragments the block was cut into.
/// * `frag_size` - how many bytes each fragment carries.
/// * `max_lost` - the most uncoded fragments to be able to solve for.
/// * `out_session` - receives the session.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_session` set to a handle the caller must
/// release with [`pamoja_lorawan_defrag_free`].
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a session this build does not run.
///
/// # Safety
///
/// `out_session` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_defrag_new(
    nb_frag: u16,
    frag_size: u8,
    max_lost: u16,
    out_session: *mut *mut PamojaLorawanDefrag,
) -> PamojaStatus {
    if out_session.is_null() {
        return missing();
    }
    *out_session = ptr::null_mut();
    if nb_frag == 0 || nb_frag > MAX_FRAGMENTS || frag_size == 0 {
        set_last_error("the session is not one this build runs".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let block = vec![0u8; usize::from(nb_frag) * usize::from(frag_size)];
    let matrix = vec![0u8; Defragmenter::matrix_len(nb_frag, max_lost.min(nb_frag))];
    let session = PamojaLorawanDefrag {
        block,
        matrix,
        nb_frag,
        frag_size,
        progress: Progress::default(),
        missing: nb_frag,
    };
    *out_session = Box::into_raw(Box::new(session));
    PamojaStatus::Ok
}

/// Releases a fragmentation session. Passing null is a no-op.
///
/// # Arguments
///
/// * `session` - the session, which must not be used again.
///
/// # Safety
///
/// `session` must be a live handle from [`pamoja_lorawan_defrag_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_defrag_free(session: *mut PamojaLorawanDefrag) {
    if !session.is_null() {
        drop(Box::from_raw(session));
    }
}

/// Takes one fragment of a session.
///
/// # Arguments
///
/// * `session` - the session.
/// * `n` - which fragment, counting from one.
/// * `fragment` - its bytes.
/// * `fragment_len` - how many, which must be at least the session's fragment size.
/// * `out_done` - receives `1` once the block is whole.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a fragment outside the session, and
/// [`PamojaStatus::Other`] when more fragments were lost than there is room to solve for.
///
/// # Safety
///
/// `session` must be a live handle, `fragment` must point to `fragment_len` readable bytes,
/// and `out_done` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_defrag_fragment(
    session: *mut PamojaLorawanDefrag,
    n: u16,
    fragment: *const u8,
    fragment_len: usize,
    out_done: *mut u8,
) -> PamojaStatus {
    let (Some(session), false) = (session.as_mut(), out_done.is_null()) else {
        return missing();
    };
    let bytes = match read_bytes(fragment, fragment_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    // The session's buffers hold everything but a handful of numbers, so it is opened
    // again where it left off for each fragment and put away again after.
    let mut receiver = match Defragmenter::resumed(
        session.nb_frag,
        session.frag_size,
        &mut session.block,
        &mut session.matrix,
        session.progress,
    ) {
        Ok(receiver) => receiver,
        Err(error) => {
            set_last_error(error.to_string());
            return PamojaStatus::InvalidArgument;
        }
    };
    let outcome = receiver.fragment(n, &bytes);
    let progress = receiver.progress();
    let missing = receiver.missing();
    session.progress = progress;
    session.missing = missing;
    match outcome {
        Ok(done) => {
            *out_done = u8::from(done);
            PamojaStatus::Ok
        }
        Err(FragError::Memory) => {
            set_last_error(FragError::Memory.to_string());
            PamojaStatus::Other
        }
        Err(error) => {
            set_last_error(error.to_string());
            PamojaStatus::InvalidArgument
        }
    }
}

/// Reports what a session has taken so far.
///
/// # Arguments
///
/// * `session` - the session.
/// * `out_received` - receives how many fragments arrived, coded and uncoded.
/// * `out_missing` - receives how many uncoded fragments are still missing.
/// * `out_done` - receives `1` once the block is whole.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success. Every out pointer may be null.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null handle.
///
/// # Safety
///
/// `session` must be a live handle and every non-null out pointer writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_defrag_status(
    session: *const PamojaLorawanDefrag,
    out_received: *mut u16,
    out_missing: *mut u16,
    out_done: *mut u8,
) -> PamojaStatus {
    let Some(session) = session.as_ref() else {
        return missing();
    };
    if !out_received.is_null() {
        *out_received = session.progress.received;
    }
    if !out_missing.is_null() {
        *out_missing = session.missing;
    }
    if !out_done.is_null() {
        *out_done = u8::from(session.progress.done);
    }
    PamojaStatus::Ok
}

/// Reads the block a session has put back together.
///
/// # Arguments
///
/// * `session` - the session.
/// * `out_block` - receives the bytes, or null to ask only for the length.
/// * `capacity` - how many bytes that buffer holds.
/// * `out_len` - receives the block's length.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null handle or a buffer too small.
///
/// # Safety
///
/// `session` must be a live handle, `out_block` must point to `capacity` writable bytes or
/// be null, and `out_len` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_defrag_block(
    session: *const PamojaLorawanDefrag,
    out_block: *mut u8,
    capacity: usize,
    out_len: *mut usize,
) -> PamojaStatus {
    let (Some(session), false) = (session.as_ref(), out_len.is_null()) else {
        return missing();
    };
    let len = usize::from(session.nb_frag) * usize::from(session.frag_size);
    *out_len = len;
    if out_block.is_null() {
        return PamojaStatus::Ok;
    }
    if capacity < len {
        set_last_error(format!("the block needs {len} bytes, not {capacity}"));
        return PamojaStatus::InvalidArgument;
    }
    ptr::copy_nonoverlapping(session.block.as_ptr(), out_block, len);
    PamojaStatus::Ok
}

/// A code being taken over a data block, released with [`pamoja_lorawan_block_mic_free`].
pub struct PamojaLorawanBlockMic {
    key: BlockMicKey,
    mic: Option<BlockMic<'static>>,
}

/// Starts a code over one session's block, TS004-2.0.0 section 3.3.
///
/// # Arguments
///
/// * `data_block_int_key` - the sixteen-byte key [`pamoja_lorawan_data_block_int_key`]
///   derived.
/// * `session_cnt` - the session counter the setup carried.
/// * `frag_index` - which of the device's sessions this is.
/// * `descriptor` - the four bytes the setup described the block with.
/// * `block_len` - the block's length without its padding.
/// * `out_mic` - receives the code.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_mic` set to a handle the caller must release
/// with [`pamoja_lorawan_block_mic_free`].
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// `data_block_int_key` must point to sixteen readable bytes, `descriptor` to four, and
/// `out_mic` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_block_mic_start(
    data_block_int_key: *const u8,
    session_cnt: u16,
    frag_index: u8,
    descriptor: *const u8,
    block_len: u32,
    out_mic: *mut *mut PamojaLorawanBlockMic,
) -> PamojaStatus {
    let (Some(key), false, false) = (
        sixteen(data_block_int_key),
        descriptor.is_null(),
        out_mic.is_null(),
    ) else {
        return missing();
    };
    *out_mic = ptr::null_mut();
    let mut four = [0u8; 4];
    ptr::copy_nonoverlapping(descriptor, four.as_mut_ptr(), 4);

    let mut held = Box::new(PamojaLorawanBlockMic {
        key: BlockMicKey::new(&key),
        mic: None,
    });
    // The code borrows the key it is taken under, which lives in the same box, so the
    // borrow lasts exactly as long as the handle does.
    let borrowed: &'static BlockMicKey = &*ptr::from_ref(&held.key);
    held.mic = Some(borrowed.start(session_cnt, frag_index, four, block_len));
    *out_mic = Box::into_raw(held);
    PamojaStatus::Ok
}

/// Adds a piece of the block to a code.
///
/// # Arguments
///
/// * `mic` - the code.
/// * `data` - the next bytes of the block, in order, without any padding.
/// * `data_len` - how many.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null handle.
///
/// # Safety
///
/// `mic` must be a live handle and `data` must point to `data_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_block_mic_update(
    mic: *mut PamojaLorawanBlockMic,
    data: *const u8,
    data_len: usize,
) -> PamojaStatus {
    let Some(held) = mic.as_mut() else {
        return missing();
    };
    let bytes = match read_bytes(data, data_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    if let Some(mic) = held.mic.as_mut() {
        mic.update(&bytes);
    }
    PamojaStatus::Ok
}

/// Finishes a code and releases it.
///
/// # Arguments
///
/// * `mic` - the code, which must not be used again.
/// * `out_mic` - receives the four bytes a session setup carries.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer.
///
/// # Safety
///
/// `mic` must be a live handle and `out_mic` must point to four writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_block_mic_finish(
    mic: *mut PamojaLorawanBlockMic,
    out_mic: *mut u8,
) -> PamojaStatus {
    if mic.is_null() || out_mic.is_null() {
        return missing();
    }
    let mut held = Box::from_raw(mic);
    let Some(taken) = held.mic.take() else {
        return missing();
    };
    let code = taken.finish();
    ptr::copy_nonoverlapping(code.as_ptr(), out_mic, 4);
    PamojaStatus::Ok
}

/// Releases a code that was never finished. Passing null is a no-op.
///
/// # Arguments
///
/// * `mic` - the code, which must not be used again.
///
/// # Safety
///
/// `mic` must be a live handle from [`pamoja_lorawan_block_mic_start`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_block_mic_free(mic: *mut PamojaLorawanBlockMic) {
    if !mic.is_null() {
        let mut held = Box::from_raw(mic);
        held.mic = None;
    }
}

/// A clock synchronization package on a device, released with
/// [`pamoja_lorawan_clock_sync_free`].
pub struct PamojaLorawanClockSync {
    sync: ClockSync,
}

/// Starts the clock synchronization package on a device, TS003-2.0.0.
///
/// # Arguments
///
/// * `self_managed` - `1` for a device that keeps its own periodicity and refuses the
///   server's.
/// * `out_sync` - receives the package.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_sync` set to a handle the caller must release
/// with [`pamoja_lorawan_clock_sync_free`].
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null out pointer.
///
/// # Safety
///
/// `out_sync` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_clock_sync_new(
    self_managed: u8,
    out_sync: *mut *mut PamojaLorawanClockSync,
) -> PamojaStatus {
    if out_sync.is_null() {
        return missing();
    }
    let sync = if self_managed != 0 {
        ClockSync::self_managed()
    } else {
        ClockSync::new()
    };
    *out_sync = Box::into_raw(Box::new(PamojaLorawanClockSync { sync }));
    PamojaStatus::Ok
}

/// Releases a clock synchronization package. Passing null is a no-op.
///
/// # Arguments
///
/// * `sync` - the package, which must not be used again.
///
/// # Safety
///
/// `sync` must be a live handle from [`pamoja_lorawan_clock_sync_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_clock_sync_free(sync: *mut PamojaLorawanClockSync) {
    if !sync.is_null() {
        drop(Box::from_raw(sync));
    }
}

/// Builds the request that asks a server for a clock correction, section 3.2.
///
/// # Arguments
///
/// * `sync` - the package.
/// * `device_time` - what the device believes the time is, in seconds since the GPS epoch.
/// * `ans_required` - `1` to make the server answer even when the clock is right.
/// * `out_command` - receives the command.
/// * `capacity` - how many bytes that buffer holds, at least six.
/// * `out_len` - receives how many were written.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer or a buffer too small.
///
/// # Safety
///
/// `sync` must be a live handle, `out_command` must point to `capacity` writable bytes, and
/// `out_len` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_clock_sync_request(
    sync: *mut PamojaLorawanClockSync,
    device_time: u32,
    ans_required: u8,
    out_command: *mut u8,
    capacity: usize,
    out_len: *mut usize,
) -> PamojaStatus {
    let (Some(sync), false, false) = (sync.as_mut(), out_command.is_null(), out_len.is_null())
    else {
        return missing();
    };
    let out = core::slice::from_raw_parts_mut(out_command, capacity);
    match sync.sync.app_time_req(device_time, ans_required != 0, out) {
        Ok(written) => {
            *out_len = written;
            PamojaStatus::Ok
        }
        Err(error) => {
            set_last_error(error.to_string());
            PamojaStatus::InvalidArgument
        }
    }
}

/// Reads a downlink on the clock port and acts on it, section 3.2.
///
/// # Arguments
///
/// * `sync` - the package.
/// * `payload` - what arrived on port [`PAMOJA_LORAWAN_CLOCK_PORT`].
/// * `payload_len` - its length.
/// * `out_correction` - receives the seconds to add to the device's clock.
/// * `out_has_correction` - receives `1` when there was one.
/// * `out_more` - receives `1` when another correction is coming.
/// * `out_resync` - receives how many requests a resynchronization command asked for, or 0.
/// * `out_answer_due` - receives `1` when the device now owes an answer.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success. Every out pointer may be null.
///
/// # Errors
///
/// Returns [`PamojaStatus::Codec`] for a message this package cannot read.
///
/// # Safety
///
/// `sync` must be a live handle, `payload` must point to `payload_len` readable bytes, and
/// every non-null out pointer must be writable.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn pamoja_lorawan_clock_sync_heard(
    sync: *mut PamojaLorawanClockSync,
    payload: *const u8,
    payload_len: usize,
    out_correction: *mut i32,
    out_has_correction: *mut u8,
    out_more: *mut u8,
    out_resync: *mut u8,
    out_answer_due: *mut u8,
) -> PamojaStatus {
    let Some(sync) = sync.as_mut() else {
        return missing();
    };
    let bytes = match read_bytes(payload, payload_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let heard = match sync.sync.heard(&bytes) {
        Ok(heard) => heard,
        Err(error) => {
            set_last_error(error.to_string());
            return PamojaStatus::Codec;
        }
    };
    if !out_correction.is_null() {
        *out_correction = heard.correction.unwrap_or(0);
    }
    if !out_has_correction.is_null() {
        *out_has_correction = u8::from(heard.correction.is_some());
    }
    if !out_more.is_null() {
        *out_more = u8::from(heard.more_correction);
    }
    if !out_resync.is_null() {
        *out_resync = heard.resync.unwrap_or(0);
    }
    if !out_answer_due.is_null() {
        *out_answer_due = u8::from(heard.answer_due);
    }
    PamojaStatus::Ok
}

/// Writes the answer a device owes its clock server, if it owes one.
///
/// # Arguments
///
/// * `sync` - the package.
/// * `device_time` - what the device believes the time is.
/// * `out_command` - receives the command.
/// * `capacity` - how many bytes that buffer holds.
/// * `out_len` - receives how many were written, which is zero when nothing is owed.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null pointer or a buffer too small.
///
/// # Safety
///
/// `sync` must be a live handle, `out_command` must point to `capacity` writable bytes, and
/// `out_len` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_clock_sync_answer(
    sync: *mut PamojaLorawanClockSync,
    device_time: u32,
    out_command: *mut u8,
    capacity: usize,
    out_len: *mut usize,
) -> PamojaStatus {
    let (Some(sync), false, false) = (sync.as_mut(), out_command.is_null(), out_len.is_null())
    else {
        return missing();
    };
    let out = core::slice::from_raw_parts_mut(out_command, capacity);
    match sync.sync.answer(device_time, out) {
        Ok(written) => {
            *out_len = written;
            PamojaStatus::Ok
        }
        Err(error) => {
            set_last_error(error.to_string());
            PamojaStatus::InvalidArgument
        }
    }
}

/// Reports where a clock synchronization package stands.
///
/// # Arguments
///
/// * `sync` - the package.
/// * `out_token` - receives the token the next request carries.
/// * `out_period_s` - receives the seconds between requests the server last set.
/// * `out_answer_due` - receives `1` when the device owes an answer.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success. Every out pointer may be null.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null handle.
///
/// # Safety
///
/// `sync` must be a live handle and every non-null out pointer writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_clock_sync_status(
    sync: *const PamojaLorawanClockSync,
    out_token: *mut u8,
    out_period_s: *mut u32,
    out_answer_due: *mut u8,
) -> PamojaStatus {
    let Some(sync) = sync.as_ref() else {
        return missing();
    };
    if !out_token.is_null() {
        *out_token = sync.sync.token();
    }
    if !out_period_s.is_null() {
        *out_period_s = sync.sync.period_s();
    }
    if !out_answer_due.is_null() {
        *out_answer_due = u8::from(sync.sync.answer_due());
    }
    PamojaStatus::Ok
}

/// A firmware management package on a device, released with
/// [`pamoja_lorawan_firmware_free`].
pub struct PamojaLorawanFirmware {
    manager: FirmwareManager,
}

/// Starts the firmware management package on a device, TS006-1.0.0.
///
/// # Arguments
///
/// * `firmware` - the version the device is running, as its manufacturer numbers it.
/// * `hardware` - the platform it runs on.
/// * `out_manager` - receives the package.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_manager` set to a handle the caller must
/// release with [`pamoja_lorawan_firmware_free`].
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null out pointer.
///
/// # Safety
///
/// `out_manager` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_firmware_new(
    firmware: u32,
    hardware: u32,
    out_manager: *mut *mut PamojaLorawanFirmware,
) -> PamojaStatus {
    if out_manager.is_null() {
        return missing();
    }
    *out_manager = Box::into_raw(Box::new(PamojaLorawanFirmware {
        manager: FirmwareManager::new(firmware, hardware),
    }));
    PamojaStatus::Ok
}

/// Releases a firmware management package. Passing null is a no-op.
///
/// # Arguments
///
/// * `manager` - the package, which must not be used again.
///
/// # Safety
///
/// `manager` must be a live handle from [`pamoja_lorawan_firmware_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_firmware_free(manager: *mut PamojaLorawanFirmware) {
    if !manager.is_null() {
        drop(Box::from_raw(manager));
    }
}

/// Says what firmware upgrade image the device is holding.
///
/// # Arguments
///
/// * `manager` - the package.
/// * `status` - one of [`PAMOJA_LORAWAN_IMAGE_NONE`], [`PAMOJA_LORAWAN_IMAGE_CORRUPT`],
///   [`PAMOJA_LORAWAN_IMAGE_WRONG_HARDWARE`] or [`PAMOJA_LORAWAN_IMAGE_VALID`].
/// * `version` - the version the device would run once it is installed, for a valid image.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null handle.
///
/// # Safety
///
/// `manager` must be a live handle.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_firmware_set_image(
    manager: *mut PamojaLorawanFirmware,
    status: u8,
    version: u32,
) -> PamojaStatus {
    let Some(manager) = manager.as_mut() else {
        return missing();
    };
    manager.manager.set_image(match status {
        PAMOJA_LORAWAN_IMAGE_NONE => None,
        PAMOJA_LORAWAN_IMAGE_VALID => Some(Image::valid(version)),
        other => Some(Image::refused(UpImageStatus::from_bits(other))),
    });
    PamojaStatus::Ok
}

/// Reads a downlink on the firmware port and writes the answers it calls for.
///
/// # Arguments
///
/// * `manager` - the package.
/// * `payload` - what arrived on port [`PAMOJA_LORAWAN_FIRMWARE_PORT`].
/// * `payload_len` - its length.
/// * `now_s` - what the device believes the time is, in seconds since the GPS epoch.
/// * `has_now` - `1` when the device knows the time; a device that does not refuses a reboot
///   set for a moment in time.
/// * `out_answers` - receives the answers.
/// * `capacity` - how many bytes that buffer holds.
/// * `out_len` - receives how many were written.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::Codec`] for a message this package cannot read, and
/// [`PamojaStatus::InvalidArgument`] for a null pointer or a buffer too small.
///
/// # Safety
///
/// `manager` must be a live handle, `payload` must point to `payload_len` readable bytes,
/// `out_answers` to `capacity` writable ones, and `out_len` must be writable.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn pamoja_lorawan_firmware_heard(
    manager: *mut PamojaLorawanFirmware,
    payload: *const u8,
    payload_len: usize,
    now_s: u32,
    has_now: u8,
    out_answers: *mut u8,
    capacity: usize,
    out_len: *mut usize,
) -> PamojaStatus {
    let (Some(manager), false, false) =
        (manager.as_mut(), out_answers.is_null(), out_len.is_null())
    else {
        return missing();
    };
    let bytes = match read_bytes(payload, payload_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let out = core::slice::from_raw_parts_mut(out_answers, capacity);
    let now = (has_now != 0).then_some(now_s);
    match manager.manager.heard_at(&bytes, now, out) {
        Ok(written) => {
            *out_len = written;
            PamojaStatus::Ok
        }
        Err(error) => {
            set_last_error(error.to_string());
            PamojaStatus::Codec
        }
    }
}

/// Reports the reboot a firmware management package is holding.
///
/// # Arguments
///
/// * `manager` - the package.
/// * `out_at_s` - receives the moment the device is to reboot, where one was set as a time.
/// * `out_has_at` - receives `1` when there is one.
/// * `out_in_s` - receives how long until it reboots, where one was set as a countdown.
/// * `out_has_in` - receives `1` when there is one.
/// * `out_now` - receives `1` when the device was told to reboot at once.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success. Every out pointer may be null.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] for a null handle.
///
/// # Safety
///
/// `manager` must be a live handle and every non-null out pointer writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_firmware_reboot(
    manager: *const PamojaLorawanFirmware,
    out_at_s: *mut u32,
    out_has_at: *mut u8,
    out_in_s: *mut u32,
    out_has_in: *mut u8,
    out_now: *mut u8,
) -> PamojaStatus {
    let Some(manager) = manager.as_ref() else {
        return missing();
    };
    let at = manager.manager.reboot_at_s();
    let left = manager.manager.reboot_in_s();
    if !out_at_s.is_null() {
        *out_at_s = at.unwrap_or(0);
    }
    if !out_has_at.is_null() {
        *out_has_at = u8::from(at.is_some());
    }
    if !out_in_s.is_null() {
        *out_in_s = left.unwrap_or(0);
    }
    if !out_has_in.is_null() {
        *out_has_in = u8::from(left.is_some());
    }
    if !out_now.is_null() {
        *out_now = u8::from(manager.manager.reboot_now());
    }
    PamojaStatus::Ok
}

/// Reads sixteen bytes from a pointer that may be null.
unsafe fn sixteen(from: *const u8) -> Option<[u8; 16]> {
    if from.is_null() {
        return None;
    }
    let mut key = [0u8; 16];
    ptr::copy_nonoverlapping(from, key.as_mut_ptr(), 16);
    Some(key)
}

/// Reports a null argument the same way everywhere.
fn missing() -> PamojaStatus {
    set_last_error("a required argument was null".to_owned());
    PamojaStatus::InvalidArgument
}

/// Runs a derivation that takes one key and writes another.
unsafe fn derive(
    from: *const u8,
    out: *mut u8,
    step: impl FnOnce(&[u8; 16]) -> [u8; 16],
) -> PamojaStatus {
    let (Some(key), false) = (sixteen(from), out.is_null()) else {
        return missing();
    };
    ptr::copy_nonoverlapping(step(&key).as_ptr(), out, 16);
    PamojaStatus::Ok
}

/// One command of an application layer package, whichever package it belongs to.
///
/// `port` says which package: [`PAMOJA_LORAWAN_CLOCK_PORT`],
/// [`PAMOJA_LORAWAN_FRAGMENT_PORT`], [`PAMOJA_LORAWAN_MULTICAST_PORT`] or
/// [`PAMOJA_LORAWAN_FIRMWARE_PORT`]. `cid` names the command within it, and `uplink` says
/// which way it travels; together they decide which of the other fields carry anything. The
/// rest are zero.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanPackageCommand {
    /// Which package this command belongs to, as its port.
    pub port: u8,
    /// Which command within that package.
    pub cid: u8,
    /// `1` for what a device sends, `0` for what a server sends.
    pub uplink: u8,
    /// The package identifier a version answer carries.
    pub package: u8,
    /// The package version it implements.
    pub version: u8,
    /// A device's own clock, in seconds since the GPS epoch.
    pub device_time: u32,
    /// The seconds to add to a device's clock.
    pub time_correction: i32,
    /// The token that pairs a clock answer with its request.
    pub token: u8,
    /// Whether a clock request must be answered.
    pub ans_required: u8,
    /// The coded period between clock requests.
    pub period: u8,
    /// Whether a device manages its own clock periodicity.
    pub not_supported: u8,
    /// How many requests a resynchronization command asks for.
    pub transmissions: u8,
    /// The firmware a device reports running.
    pub firmware: u32,
    /// The hardware it runs on.
    pub hardware: u32,
    /// The moment or the delay a reboot is set for.
    pub reboot: u32,
    /// What a device makes of the upgrade image it holds.
    pub image_status: u8,
    /// The version it would run once that image is installed.
    pub next_version: u32,
    /// Whether an image answer carried a version.
    pub has_next_version: u8,
    /// The version a delete command names.
    pub delete_version: u32,
    /// Whether a device holds no valid image.
    pub no_valid_image: u8,
    /// Whether the version named is not the one held.
    pub invalid_version: u8,
    /// Which fragmentation session, 0 to 3.
    pub frag_index: u8,
    /// Which multicast groups may feed it, a bit for each.
    pub mc_group_bit_mask: u8,
    /// How many uncoded fragments a block was cut into.
    pub nb_frag: u16,
    /// How many bytes each fragment carries.
    pub frag_size: u8,
    /// Whether a device reports the block once it has it.
    pub ack_reception: u8,
    /// Which fragmentation algorithm to run.
    pub frag_algo: u8,
    /// The coded spread of the delay before a device answers.
    pub block_ack_delay: u8,
    /// How many bytes of padding the last fragment carries.
    pub padding: u8,
    /// The four bytes a server describes a block with.
    pub descriptor: [u8; 4],
    /// The session counter, which must rise for each new block.
    pub session_cnt: u16,
    /// The code over the block a device checks once it has it all.
    pub mic: [u8; 4],
    /// How many fragments arrived, coded, uncoded and repeated.
    pub received: u16,
    /// How many uncoded fragments are still missing.
    pub missing: u8,
    /// Whether the block's code did not check out.
    pub mic_error: u8,
    /// Whether a session ran out of memory to defragment with.
    pub memory_error: u8,
    /// Whether the session or group named does not exist on the device.
    pub no_session: u8,
    /// Whether every device answers a status request, or only those still missing fragments.
    pub all_participants: u8,
    /// Which fragment of a session a data fragment carries, counting from one.
    pub fragment_n: u16,
    /// Which multicast group, 0 to 3.
    pub mc_group_id: u8,
    /// The address a group answers to.
    pub mc_addr: u32,
    /// A group's key, wrapped under the device's key encryption key.
    pub mc_key_encrypted: [u8; 16],
    /// The first frame counter a device accepts from a group.
    pub min_mc_fcnt: u32,
    /// The last one, which ends the group's life.
    pub max_mc_fcnt: u32,
    /// Which groups a status request or answer covers, a bit for each.
    pub group_mask: u8,
    /// How many groups a device holds in all.
    pub nb_total_groups: u8,
    /// Whether a device holds no group by the identifier named.
    pub id_error: u8,
    /// When a multicast window opens, in seconds since the GPS epoch.
    pub session_time: u32,
    /// How long it lasts at most, coded.
    pub time_out: u8,
    /// How often a device opens a ping slot inside a Class B window.
    pub periodicity: u8,
    /// Where a group listens, in hertz.
    pub dl_frequency_hz: u32,
    /// The data rate it listens at.
    pub data_rate: u8,
    /// How many seconds until a window opens.
    pub time_to_start: u32,
    /// Whether a session answer carried a start time.
    pub has_time_to_start: u8,
    /// Whether the data rate named is not one the device has.
    pub dr_error: u8,
    /// Whether the frequency named is not one it can use.
    pub freq_error: u8,
    /// Whether the window was to start at a time already past.
    pub start_missed: u8,
}

impl Default for PamojaLorawanPackageCommand {
    fn default() -> PamojaLorawanPackageCommand {
        // Every field is a scalar or an array of them, so zero is the "carries nothing"
        // value the struct documents.
        unsafe { core::mem::zeroed() }
    }
}

/// Reads one command of an application layer package.
///
/// A data fragment takes the whole message, as TS004-2.0.0 section 3 asks, and its bytes are
/// left in the caller's buffer: `out_command.fragment_n` says which fragment it is, and the
/// data starts three bytes into the message.
///
/// # Arguments
///
/// * `port` - which package the message arrived on.
/// * `uplink` - `1` when a device sent it, `0` when a server did.
/// * `payload` - the message, from this command's identifier on.
/// * `payload_len` - its length.
/// * `out_command` - receives the command.
/// * `out_taken` - receives how many bytes it took.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::Codec`] for a message this package cannot read, and
/// [`PamojaStatus::InvalidArgument`] for a port that names no package or a null pointer.
///
/// # Safety
///
/// `payload` must point to `payload_len` readable bytes and both out pointers must be
/// writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_package_parse(
    port: u8,
    uplink: u8,
    payload: *const u8,
    payload_len: usize,
    out_command: *mut PamojaLorawanPackageCommand,
    out_taken: *mut usize,
) -> PamojaStatus {
    if out_command.is_null() || out_taken.is_null() {
        return missing();
    }
    let bytes = match read_bytes(payload, payload_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let direction = if uplink != 0 {
        Direction::Uplink
    } else {
        Direction::Downlink
    };
    let read = match port {
        CLOCK_PORT => ClockCommand::parse(direction, &bytes)
            .map(|(command, taken)| (clock_out(command), taken)),
        FIRMWARE_PORT => FirmwareCommand::parse(direction, &bytes)
            .map(|(command, taken)| (firmware_out(command), taken)),
        FRAGMENT_PORT => {
            FragCommand::parse(direction, &bytes).map(|(command, taken)| (frag_out(command), taken))
        }
        MULTICAST_PORT => {
            McCommand::parse(direction, &bytes).map(|(command, taken)| (mc_out(command), taken))
        }
        other => {
            set_last_error(format!("port {other} is not an application layer package"));
            return PamojaStatus::InvalidArgument;
        }
    };
    match read {
        Ok((command, taken)) => {
            *out_command = command;
            *out_taken = taken;
            PamojaStatus::Ok
        }
        Err(error) => {
            set_last_error(error.to_string());
            PamojaStatus::Codec
        }
    }
}

/// Writes one command of an application layer package.
///
/// A data fragment carries its bytes separately: pass them as `data`, and the command's
/// `fragment_n` and `frag_index` say where they belong.
///
/// # Arguments
///
/// * `command` - the command, whose `port`, `cid` and `uplink` decide which fields are read.
/// * `data` - the bytes a data fragment carries, or null for every other command.
/// * `data_len` - how many.
/// * `out_payload` - receives the message.
/// * `capacity` - how many bytes that buffer holds.
/// * `out_len` - receives how many were written.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::Codec`] for a command this build does not write, and
/// [`PamojaStatus::InvalidArgument`] for a null pointer or a buffer too small.
///
/// # Safety
///
/// `command` must be readable, `data` must point to `data_len` readable bytes or be null,
/// `out_payload` must point to `capacity` writable bytes, and `out_len` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_package_encode(
    command: *const PamojaLorawanPackageCommand,
    data: *const u8,
    data_len: usize,
    out_payload: *mut u8,
    capacity: usize,
    out_len: *mut usize,
) -> PamojaStatus {
    let (Some(command), false, false) =
        (command.as_ref(), out_payload.is_null(), out_len.is_null())
    else {
        return missing();
    };
    let carried = match read_bytes(data, data_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    let out = core::slice::from_raw_parts_mut(out_payload, capacity);
    let written = match command.port {
        CLOCK_PORT => clock_in(command).map(|c| c.encode(out)),
        FIRMWARE_PORT => firmware_in(command).map(|c| c.encode(out)),
        FRAGMENT_PORT => frag_in(command, &carried).map(|c| c.encode(out)),
        MULTICAST_PORT => mc_in(command).map(|c| c.encode(out)),
        other => {
            set_last_error(format!("port {other} is not an application layer package"));
            return PamojaStatus::InvalidArgument;
        }
    };
    match written {
        Some(Ok(len)) => {
            *out_len = len;
            PamojaStatus::Ok
        }
        Some(Err(error)) => {
            set_last_error(error.to_string());
            PamojaStatus::InvalidArgument
        }
        None => {
            set_last_error(format!(
                "command {:#04x} on port {} is not one this build writes",
                command.cid, command.port
            ));
            PamojaStatus::Codec
        }
    }
}

/// Describes a clock synchronization command the way C holds it.
fn clock_out(command: ClockCommand) -> PamojaLorawanPackageCommand {
    let mut flat = PamojaLorawanPackageCommand {
        port: CLOCK_PORT,
        cid: command.cid(),
        uplink: u8::from(matches!(command.direction(), Direction::Uplink)),
        ..PamojaLorawanPackageCommand::default()
    };
    match command {
        ClockCommand::PackageVersionAns(version) => {
            flat.package = version.package;
            flat.version = version.version;
        }
        ClockCommand::AppTimeReq {
            device_time,
            ans_required,
            token,
        } => {
            flat.device_time = device_time;
            flat.ans_required = u8::from(ans_required);
            flat.token = token;
        }
        ClockCommand::AppTimeAns {
            time_correction,
            token,
        } => {
            flat.time_correction = time_correction;
            flat.token = token;
        }
        ClockCommand::DeviceAppTimePeriodicityReq { period } => flat.period = period,
        ClockCommand::DeviceAppTimePeriodicityAns {
            not_supported,
            device_time,
        } => {
            flat.not_supported = u8::from(not_supported);
            flat.device_time = device_time;
        }
        ClockCommand::ForceDeviceResyncCmd { transmissions } => flat.transmissions = transmissions,
        ClockCommand::PackageVersionReq => {}
    }
    flat
}

/// Reads a clock synchronization command out of the record C holds.
fn clock_in(flat: &PamojaLorawanPackageCommand) -> Option<ClockCommand> {
    let up = flat.uplink != 0;
    Some(match (flat.cid, up) {
        (0x00, false) => ClockCommand::PackageVersionReq,
        (0x00, true) => ClockCommand::PackageVersionAns(PackageVersion {
            package: flat.package,
            version: flat.version,
        }),
        (0x01, true) => ClockCommand::AppTimeReq {
            device_time: flat.device_time,
            ans_required: flat.ans_required != 0,
            token: flat.token,
        },
        (0x01, false) => ClockCommand::AppTimeAns {
            time_correction: flat.time_correction,
            token: flat.token,
        },
        (0x02, false) => ClockCommand::DeviceAppTimePeriodicityReq {
            period: flat.period,
        },
        (0x02, true) => ClockCommand::DeviceAppTimePeriodicityAns {
            not_supported: flat.not_supported != 0,
            device_time: flat.device_time,
        },
        (0x03, false) => ClockCommand::ForceDeviceResyncCmd {
            transmissions: flat.transmissions,
        },
        _ => return None,
    })
}

/// Describes a firmware management command the way C holds it.
fn firmware_out(command: FirmwareCommand) -> PamojaLorawanPackageCommand {
    let mut flat = PamojaLorawanPackageCommand {
        port: FIRMWARE_PORT,
        cid: command.cid(),
        uplink: u8::from(matches!(command.direction(), Direction::Uplink)),
        ..PamojaLorawanPackageCommand::default()
    };
    match command {
        FirmwareCommand::PackageVersionAns(version) => {
            flat.package = version.package;
            flat.version = version.version;
        }
        FirmwareCommand::DevVersionAns { firmware, hardware } => {
            flat.firmware = firmware;
            flat.hardware = hardware;
        }
        FirmwareCommand::DevRebootTimeReq { reboot_time }
        | FirmwareCommand::DevRebootTimeAns { reboot_time } => flat.reboot = reboot_time,
        FirmwareCommand::DevRebootCountdownReq { countdown }
        | FirmwareCommand::DevRebootCountdownAns { countdown } => flat.reboot = countdown,
        FirmwareCommand::DevUpgradeImageAns {
            status,
            next_version,
        } => {
            flat.image_status = status as u8;
            flat.next_version = next_version.unwrap_or(0);
            flat.has_next_version = u8::from(next_version.is_some());
        }
        FirmwareCommand::DevDeleteImageReq { version } => flat.delete_version = version,
        FirmwareCommand::DevDeleteImageAns(status) => {
            flat.no_valid_image = u8::from(status.no_valid_image);
            flat.invalid_version = u8::from(status.invalid_version);
        }
        FirmwareCommand::PackageVersionReq
        | FirmwareCommand::DevVersionReq
        | FirmwareCommand::DevUpgradeImageReq => {}
    }
    flat
}

/// Reads a firmware management command out of the record C holds.
fn firmware_in(flat: &PamojaLorawanPackageCommand) -> Option<FirmwareCommand> {
    let up = flat.uplink != 0;
    Some(match (flat.cid, up) {
        (0x00, false) => FirmwareCommand::PackageVersionReq,
        (0x00, true) => FirmwareCommand::PackageVersionAns(PackageVersion {
            package: flat.package,
            version: flat.version,
        }),
        (0x01, false) => FirmwareCommand::DevVersionReq,
        (0x01, true) => FirmwareCommand::DevVersionAns {
            firmware: flat.firmware,
            hardware: flat.hardware,
        },
        (0x02, false) => FirmwareCommand::DevRebootTimeReq {
            reboot_time: flat.reboot,
        },
        (0x02, true) => FirmwareCommand::DevRebootTimeAns {
            reboot_time: flat.reboot,
        },
        (0x03, false) => FirmwareCommand::DevRebootCountdownReq {
            countdown: flat.reboot,
        },
        (0x03, true) => FirmwareCommand::DevRebootCountdownAns {
            countdown: flat.reboot,
        },
        (0x04, false) => FirmwareCommand::DevUpgradeImageReq,
        (0x04, true) => FirmwareCommand::DevUpgradeImageAns {
            status: UpImageStatus::from_bits(flat.image_status),
            next_version: (flat.has_next_version != 0).then_some(flat.next_version),
        },
        (0x05, false) => FirmwareCommand::DevDeleteImageReq {
            version: flat.delete_version,
        },
        (0x05, true) => FirmwareCommand::DevDeleteImageAns(DeleteStatus {
            no_valid_image: flat.no_valid_image != 0,
            invalid_version: flat.invalid_version != 0,
        }),
        _ => return None,
    })
}

/// Describes a fragmentation command the way C holds it.
fn frag_out(command: FragCommand<'_>) -> PamojaLorawanPackageCommand {
    let mut flat = PamojaLorawanPackageCommand {
        port: FRAGMENT_PORT,
        cid: command.cid(),
        uplink: u8::from(matches!(command.direction(), Direction::Uplink)),
        ..PamojaLorawanPackageCommand::default()
    };
    match command {
        FragCommand::PackageVersionAns(version) => {
            flat.package = version.package;
            flat.version = version.version;
        }
        FragCommand::FragSessionStatusReq {
            frag_index,
            all_participants,
        } => {
            flat.frag_index = frag_index;
            flat.all_participants = u8::from(all_participants);
        }
        FragCommand::FragSessionStatusAns {
            frag_index,
            received,
            missing,
            mic_error,
            memory_error,
            no_session,
        } => {
            flat.frag_index = frag_index;
            flat.received = received;
            flat.missing = missing;
            flat.mic_error = u8::from(mic_error);
            flat.memory_error = u8::from(memory_error);
            flat.no_session = u8::from(no_session);
        }
        FragCommand::FragSessionSetupReq {
            frag_index,
            mc_group_bit_mask,
            nb_frag,
            frag_size,
            ack_reception,
            frag_algo,
            block_ack_delay,
            padding,
            descriptor,
            session_cnt,
            mic,
        } => {
            flat.frag_index = frag_index;
            flat.mc_group_bit_mask = mc_group_bit_mask;
            flat.nb_frag = nb_frag;
            flat.frag_size = frag_size;
            flat.ack_reception = u8::from(ack_reception);
            flat.frag_algo = frag_algo;
            flat.block_ack_delay = block_ack_delay;
            flat.padding = padding;
            flat.descriptor = descriptor;
            flat.session_cnt = session_cnt;
            flat.mic = mic;
        }
        FragCommand::FragSessionSetupAns(status) => {
            flat.frag_index = status.frag_index;
            flat.frag_algo = u8::from(status.unsupported_algorithm);
            flat.memory_error = u8::from(status.not_enough_memory);
            flat.no_session = u8::from(status.unsupported_index);
            flat.invalid_version = u8::from(status.wrong_descriptor);
            flat.mic_error = u8::from(status.session_replay);
        }
        FragCommand::FragSessionDeleteReq { frag_index } => flat.frag_index = frag_index,
        FragCommand::FragSessionDeleteAns {
            frag_index,
            no_session,
        } => {
            flat.frag_index = frag_index;
            flat.no_session = u8::from(no_session);
        }
        FragCommand::FragDataBlockReceivedReq {
            frag_index,
            mic_error,
        } => {
            flat.frag_index = frag_index;
            flat.mic_error = u8::from(mic_error);
        }
        FragCommand::FragDataBlockReceivedAns { frag_index } => flat.frag_index = frag_index,
        FragCommand::DataFragment {
            frag_index,
            n,
            data: _,
        } => {
            flat.frag_index = frag_index;
            flat.fragment_n = n;
        }
        FragCommand::PackageVersionReq => {}
    }
    flat
}

/// Reads a fragmentation command out of the record C holds.
fn frag_in<'a>(flat: &PamojaLorawanPackageCommand, data: &'a [u8]) -> Option<FragCommand<'a>> {
    let up = flat.uplink != 0;
    Some(match (flat.cid, up) {
        (0x00, false) => FragCommand::PackageVersionReq,
        (0x00, true) => FragCommand::PackageVersionAns(PackageVersion {
            package: flat.package,
            version: flat.version,
        }),
        (0x01, false) => FragCommand::FragSessionStatusReq {
            frag_index: flat.frag_index,
            all_participants: flat.all_participants != 0,
        },
        (0x01, true) => FragCommand::FragSessionStatusAns {
            frag_index: flat.frag_index,
            received: flat.received,
            missing: flat.missing,
            mic_error: flat.mic_error != 0,
            memory_error: flat.memory_error != 0,
            no_session: flat.no_session != 0,
        },
        (0x02, false) => FragCommand::FragSessionSetupReq {
            frag_index: flat.frag_index,
            mc_group_bit_mask: flat.mc_group_bit_mask,
            nb_frag: flat.nb_frag,
            frag_size: flat.frag_size,
            ack_reception: flat.ack_reception != 0,
            frag_algo: flat.frag_algo,
            block_ack_delay: flat.block_ack_delay,
            padding: flat.padding,
            descriptor: flat.descriptor,
            session_cnt: flat.session_cnt,
            mic: flat.mic,
        },
        (0x02, true) => FragCommand::FragSessionSetupAns(SetupStatus {
            unsupported_algorithm: flat.frag_algo != 0,
            not_enough_memory: flat.memory_error != 0,
            unsupported_index: flat.no_session != 0,
            wrong_descriptor: flat.invalid_version != 0,
            session_replay: flat.mic_error != 0,
            frag_index: flat.frag_index,
        }),
        (0x03, false) => FragCommand::FragSessionDeleteReq {
            frag_index: flat.frag_index,
        },
        (0x03, true) => FragCommand::FragSessionDeleteAns {
            frag_index: flat.frag_index,
            no_session: flat.no_session != 0,
        },
        (0x04, true) => FragCommand::FragDataBlockReceivedReq {
            frag_index: flat.frag_index,
            mic_error: flat.mic_error != 0,
        },
        (0x04, false) => FragCommand::FragDataBlockReceivedAns {
            frag_index: flat.frag_index,
        },
        (0x08, false) => FragCommand::DataFragment {
            frag_index: flat.frag_index,
            n: flat.fragment_n,
            data,
        },
        _ => return None,
    })
}

/// Describes a multicast setup command the way C holds it.
fn mc_out(command: McCommand) -> PamojaLorawanPackageCommand {
    let mut flat = PamojaLorawanPackageCommand {
        port: MULTICAST_PORT,
        cid: command.cid(),
        uplink: u8::from(matches!(command.direction(), Direction::Uplink)),
        ..PamojaLorawanPackageCommand::default()
    };
    match command {
        McCommand::PackageVersionAns(version) => {
            flat.package = version.package;
            flat.version = version.version;
        }
        McCommand::McGroupStatusReq { req_group_mask } => flat.group_mask = req_group_mask,
        McCommand::McGroupStatusAns {
            ans_group_mask,
            nb_total_groups,
        } => {
            flat.group_mask = ans_group_mask;
            flat.nb_total_groups = nb_total_groups;
        }
        McCommand::McGroupStatusItem {
            mc_group_id,
            mc_addr,
        } => {
            flat.mc_group_id = mc_group_id;
            flat.mc_addr = mc_addr;
        }
        McCommand::McGroupSetupReq {
            mc_group_id,
            mc_addr,
            mc_key_encrypted,
            min_mc_fcnt,
            max_mc_fcnt,
        } => {
            flat.mc_group_id = mc_group_id;
            flat.mc_addr = mc_addr;
            flat.mc_key_encrypted = mc_key_encrypted;
            flat.min_mc_fcnt = min_mc_fcnt;
            flat.max_mc_fcnt = max_mc_fcnt;
        }
        McCommand::McGroupSetupAns {
            mc_group_id,
            id_error,
        } => {
            flat.mc_group_id = mc_group_id;
            flat.id_error = u8::from(id_error);
        }
        McCommand::McGroupDeleteReq { mc_group_id } => flat.mc_group_id = mc_group_id,
        McCommand::McGroupDeleteAns {
            mc_group_id,
            group_undefined,
        } => {
            flat.mc_group_id = mc_group_id;
            flat.no_session = u8::from(group_undefined);
        }
        McCommand::McClassCSessionReq {
            mc_group_id,
            session_time,
            time_out,
            dl_frequency_hz,
            data_rate,
        } => {
            flat.mc_group_id = mc_group_id;
            flat.session_time = session_time;
            flat.time_out = time_out;
            flat.dl_frequency_hz = dl_frequency_hz;
            flat.data_rate = data_rate;
        }
        McCommand::McClassBSessionReq {
            mc_group_id,
            session_time,
            time_out,
            periodicity,
            dl_frequency_hz,
            data_rate,
        } => {
            flat.mc_group_id = mc_group_id;
            flat.session_time = session_time;
            flat.time_out = time_out;
            flat.periodicity = periodicity;
            flat.dl_frequency_hz = dl_frequency_hz;
            flat.data_rate = data_rate;
        }
        McCommand::McClassCSessionAns {
            status,
            time_to_start,
        }
        | McCommand::McClassBSessionAns {
            status,
            time_to_start,
        } => {
            flat.mc_group_id = status.mc_group_id;
            flat.dr_error = u8::from(status.dr_error);
            flat.freq_error = u8::from(status.freq_error);
            flat.no_session = u8::from(status.group_undefined);
            flat.start_missed = u8::from(status.start_missed);
            flat.time_to_start = time_to_start.unwrap_or(0);
            flat.has_time_to_start = u8::from(time_to_start.is_some());
        }
        McCommand::PackageVersionReq => {}
    }
    flat
}

/// Reads a multicast setup command out of the record C holds.
fn mc_in(flat: &PamojaLorawanPackageCommand) -> Option<McCommand> {
    let up = flat.uplink != 0;
    let status = SessionStatus {
        mc_group_id: flat.mc_group_id,
        dr_error: flat.dr_error != 0,
        freq_error: flat.freq_error != 0,
        group_undefined: flat.no_session != 0,
        start_missed: flat.start_missed != 0,
    };
    let time_to_start = (flat.has_time_to_start != 0).then_some(flat.time_to_start);
    Some(match (flat.cid, up) {
        (0x00, false) => McCommand::PackageVersionReq,
        (0x00, true) => McCommand::PackageVersionAns(PackageVersion {
            package: flat.package,
            version: flat.version,
        }),
        (0x01, false) => McCommand::McGroupStatusReq {
            req_group_mask: flat.group_mask,
        },
        (0x01, true) => McCommand::McGroupStatusAns {
            ans_group_mask: flat.group_mask,
            nb_total_groups: flat.nb_total_groups,
        },
        (0x02, false) => McCommand::McGroupSetupReq {
            mc_group_id: flat.mc_group_id,
            mc_addr: flat.mc_addr,
            mc_key_encrypted: flat.mc_key_encrypted,
            min_mc_fcnt: flat.min_mc_fcnt,
            max_mc_fcnt: flat.max_mc_fcnt,
        },
        (0x02, true) => McCommand::McGroupSetupAns {
            mc_group_id: flat.mc_group_id,
            id_error: flat.id_error != 0,
        },
        (0x03, false) => McCommand::McGroupDeleteReq {
            mc_group_id: flat.mc_group_id,
        },
        (0x03, true) => McCommand::McGroupDeleteAns {
            mc_group_id: flat.mc_group_id,
            group_undefined: flat.no_session != 0,
        },
        (0x04, false) => McCommand::McClassCSessionReq {
            mc_group_id: flat.mc_group_id,
            session_time: flat.session_time,
            time_out: flat.time_out,
            dl_frequency_hz: flat.dl_frequency_hz,
            data_rate: flat.data_rate,
        },
        (0x04, true) => McCommand::McClassCSessionAns {
            status,
            time_to_start,
        },
        (0x05, false) => McCommand::McClassBSessionReq {
            mc_group_id: flat.mc_group_id,
            session_time: flat.session_time,
            time_out: flat.time_out,
            periodicity: flat.periodicity,
            dl_frequency_hz: flat.dl_frequency_hz,
            data_rate: flat.data_rate,
        },
        (0x05, true) => McCommand::McClassBSessionAns {
            status,
            time_to_start,
        },
        _ => return None,
    })
}

/// Reads one group record of a multicast status answer, TS005-2.0.0 section 4.2.
///
/// # Arguments
///
/// * `payload` - the message from the record on, five bytes or more.
/// * `payload_len` - its length.
/// * `out_command` - receives the record.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::Codec`] when the message ends inside the record.
///
/// # Safety
///
/// `payload` must point to `payload_len` readable bytes and `out_command` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_package_status_item(
    payload: *const u8,
    payload_len: usize,
    out_command: *mut PamojaLorawanPackageCommand,
) -> PamojaStatus {
    if out_command.is_null() {
        return missing();
    }
    let bytes = match read_bytes(payload, payload_len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    match McCommand::status_item(&bytes) {
        Ok((command, _)) => {
            *out_command = mc_out(command);
            PamojaStatus::Ok
        }
        Err(error) => {
            set_last_error(error.to_string());
            PamojaStatus::Codec
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads a command of each package across the boundary and writes it back.
    #[test]
    fn a_command_of_each_package_crosses_the_boundary_both_ways() {
        unsafe {
            // Bytes taken from the specifications and from ChirpStack's own vectors.
            let cases: [(u8, u8, &[u8]); 6] = [
                (
                    PAMOJA_LORAWAN_CLOCK_PORT,
                    1,
                    &[0x01, 0x78, 0x56, 0x34, 0x12, 0x1A],
                ),
                (PAMOJA_LORAWAN_CLOCK_PORT, 0, &[0x03, 0x02]),
                (PAMOJA_LORAWAN_FIRMWARE_PORT, 0, &[0x03, 0x10, 0x0E, 0x00]),
                (
                    PAMOJA_LORAWAN_FRAGMENT_PORT,
                    0,
                    &[
                        0x02, 0x31, 0x00, 0x04, 0x80, 0x0D, 0x40, 0x01, 0x02, 0x03, 0x04, 0x80,
                        0x00, 0x01, 0x02, 0x03, 0x04,
                    ],
                ),
                (
                    PAMOJA_LORAWAN_FRAGMENT_PORT,
                    1,
                    &[0x01, 0x05, 0x00, 0xC4, 0x80],
                ),
                (
                    PAMOJA_LORAWAN_MULTICAST_PORT,
                    0,
                    &[
                        0x04, 0x02, 0x00, 0x04, 0x00, 0x00, 0x0F, 0x28, 0x76, 0x84, 0x05,
                    ],
                ),
            ];

            for (port, uplink, bytes) in cases {
                let mut command = PamojaLorawanPackageCommand::default();
                let mut taken = 0usize;
                assert_eq!(
                    pamoja_lorawan_package_parse(
                        port,
                        uplink,
                        bytes.as_ptr(),
                        bytes.len(),
                        &mut command,
                        &mut taken,
                    ),
                    PamojaStatus::Ok,
                    "reading {bytes:02x?} on port {port}"
                );
                assert_eq!(taken, bytes.len());

                let mut out = [0u8; 64];
                let mut written = 0usize;
                assert_eq!(
                    pamoja_lorawan_package_encode(
                        &command,
                        ptr::null(),
                        0,
                        out.as_mut_ptr(),
                        out.len(),
                        &mut written,
                    ),
                    PamojaStatus::Ok,
                    "writing it back"
                );
                assert_eq!(&out[..written], bytes, "on port {port}");
            }
        }
    }

    /// A data fragment carries its bytes beside the command.
    #[test]
    fn a_data_fragment_carries_its_bytes_beside_the_command() {
        unsafe {
            let message = [0x08u8, 0x00, 0x84, 0x01, 0x02, 0x03, 0x04];
            let mut command = PamojaLorawanPackageCommand::default();
            let mut taken = 0usize;
            assert_eq!(
                pamoja_lorawan_package_parse(
                    PAMOJA_LORAWAN_FRAGMENT_PORT,
                    0,
                    message.as_ptr(),
                    message.len(),
                    &mut command,
                    &mut taken,
                ),
                PamojaStatus::Ok
            );
            assert_eq!((command.frag_index, command.fragment_n), (2, 1024));
            assert_eq!(taken, message.len());

            let data = [0x01u8, 0x02, 0x03, 0x04];
            let mut out = [0u8; 16];
            let mut written = 0usize;
            assert_eq!(
                pamoja_lorawan_package_encode(
                    &command,
                    data.as_ptr(),
                    data.len(),
                    out.as_mut_ptr(),
                    out.len(),
                    &mut written,
                ),
                PamojaStatus::Ok
            );
            assert_eq!(&out[..written], &message);
        }
    }

    /// A port that names no package, and a command this build does not write.
    #[test]
    fn a_port_that_names_no_package_is_refused() {
        unsafe {
            let bytes = [0x00u8];
            let mut command = PamojaLorawanPackageCommand::default();
            let mut taken = 0usize;
            assert_eq!(
                pamoja_lorawan_package_parse(
                    1,
                    0,
                    bytes.as_ptr(),
                    bytes.len(),
                    &mut command,
                    &mut taken,
                ),
                PamojaStatus::InvalidArgument
            );

            let unknown = PamojaLorawanPackageCommand {
                port: PAMOJA_LORAWAN_CLOCK_PORT,
                cid: 0x7F,
                ..PamojaLorawanPackageCommand::default()
            };
            let mut out = [0u8; 16];
            let mut written = 0usize;
            assert_eq!(
                pamoja_lorawan_package_encode(
                    &unknown,
                    ptr::null(),
                    0,
                    out.as_mut_ptr(),
                    out.len(),
                    &mut written,
                ),
                PamojaStatus::Codec
            );
        }
    }

    /// Runs a whole fragmentation session across the boundary, losing every third fragment.
    #[test]
    fn a_block_crosses_the_boundary_in_fragments_and_comes_back_whole() {
        unsafe {
            // A session of two dozen fragments, which is where this code starts to pay:
            // the specification warns it is weak below about twenty.
            let block: [u8; 384] = core::array::from_fn(|i| (i * 13 + 5) as u8);
            let (mut nb_frag, mut padding) = (0u16, 0u8);
            assert_eq!(
                pamoja_lorawan_frag_session(block.len(), 16, &mut nb_frag, &mut padding),
                PamojaStatus::Ok
            );
            assert_eq!((nb_frag, padding), (24, 0));

            let mut session: *mut PamojaLorawanDefrag = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_defrag_new(nb_frag, 16, 12, &mut session),
                PamojaStatus::Ok
            );

            let mut fragment = [0u8; 16];
            let mut done = 0u8;
            for n in 1..=nb_frag * 3 {
                if n % 3 == 0 {
                    continue;
                }
                assert_eq!(
                    pamoja_lorawan_frag_fragment(
                        block.as_ptr(),
                        block.len(),
                        16,
                        n,
                        fragment.as_mut_ptr(),
                        fragment.len(),
                    ),
                    PamojaStatus::Ok
                );
                assert_eq!(
                    pamoja_lorawan_defrag_fragment(
                        session,
                        n,
                        fragment.as_ptr(),
                        fragment.len(),
                        &mut done,
                    ),
                    PamojaStatus::Ok
                );
                if done == 1 {
                    break;
                }
            }
            assert_eq!(done, 1, "the coded fragments made up for the losses");

            let (mut received, mut missing, mut finished) = (0u16, 0u16, 0u8);
            assert_eq!(
                pamoja_lorawan_defrag_status(session, &mut received, &mut missing, &mut finished),
                PamojaStatus::Ok
            );
            assert_eq!((missing, finished), (0, 1));
            assert!(received >= nb_frag);

            let mut out = [0u8; 384];
            let mut len = 0usize;
            assert_eq!(
                pamoja_lorawan_defrag_block(session, out.as_mut_ptr(), out.len(), &mut len),
                PamojaStatus::Ok
            );
            assert_eq!(len, block.len());
            assert_eq!(out, block);

            pamoja_lorawan_defrag_free(session);
        }
    }

    /// The keys of a multicast group and the code over a block, across the boundary.
    #[test]
    fn a_group_key_and_a_block_code_cross_the_boundary() {
        unsafe {
            let app_key = [0x2Bu8; 16];
            let group_key = [0x77u8; 16];
            let mut root = [0u8; 16];
            let mut ke = [0u8; 16];
            assert_eq!(
                pamoja_lorawan_mc_root_key(app_key.as_ptr(), 0, root.as_mut_ptr()),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_lorawan_mc_ke_key(root.as_ptr(), ke.as_mut_ptr()),
                PamojaStatus::Ok
            );

            let mut wrapped = [0u8; 16];
            let mut unwrapped = [0u8; 16];
            assert_eq!(
                pamoja_lorawan_wrap_mc_key(ke.as_ptr(), group_key.as_ptr(), wrapped.as_mut_ptr()),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_lorawan_mc_key(ke.as_ptr(), wrapped.as_ptr(), unwrapped.as_mut_ptr()),
                PamojaStatus::Ok
            );
            assert_eq!(unwrapped, group_key, "the device gets the group key back");

            let mut app = [0u8; 16];
            let mut nwk = [0u8; 16];
            assert_eq!(
                pamoja_lorawan_mc_app_s_key(group_key.as_ptr(), 0x2601_1BDA, app.as_mut_ptr()),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_lorawan_mc_nwk_s_key(group_key.as_ptr(), 0x2601_1BDA, nwk.as_mut_ptr()),
                PamojaStatus::Ok
            );
            assert_ne!(app, nwk);

            // The block code, taken in two pieces, matches the one taken in a single call.
            let mut block_key = [0u8; 16];
            assert_eq!(
                pamoja_lorawan_data_block_int_key(app_key.as_ptr(), block_key.as_mut_ptr()),
                PamojaStatus::Ok
            );
            let block = [0xA5u8; 40];
            let descriptor = *b"PJU1";
            let mut whole: *mut PamojaLorawanBlockMic = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_block_mic_start(
                    block_key.as_ptr(),
                    7,
                    2,
                    descriptor.as_ptr(),
                    block.len() as u32,
                    &mut whole,
                ),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_lorawan_block_mic_update(whole, block.as_ptr(), block.len()),
                PamojaStatus::Ok
            );
            let mut one = [0u8; 4];
            assert_eq!(
                pamoja_lorawan_block_mic_finish(whole, one.as_mut_ptr()),
                PamojaStatus::Ok
            );

            let mut pieces: *mut PamojaLorawanBlockMic = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_block_mic_start(
                    block_key.as_ptr(),
                    7,
                    2,
                    descriptor.as_ptr(),
                    block.len() as u32,
                    &mut pieces,
                ),
                PamojaStatus::Ok
            );
            for chunk in block.chunks(7) {
                assert_eq!(
                    pamoja_lorawan_block_mic_update(pieces, chunk.as_ptr(), chunk.len()),
                    PamojaStatus::Ok
                );
            }
            let mut two = [0u8; 4];
            assert_eq!(
                pamoja_lorawan_block_mic_finish(pieces, two.as_mut_ptr()),
                PamojaStatus::Ok
            );
            assert_eq!(one, two);
        }
    }

    /// A clock correction and a firmware reboot, across the boundary.
    #[test]
    fn a_clock_correction_and_a_reboot_cross_the_boundary() {
        unsafe {
            let mut sync: *mut PamojaLorawanClockSync = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_clock_sync_new(0, &mut sync),
                PamojaStatus::Ok
            );

            let mut request = [0u8; 8];
            let mut len = 0usize;
            assert_eq!(
                pamoja_lorawan_clock_sync_request(
                    sync,
                    1_000,
                    1,
                    request.as_mut_ptr(),
                    request.len(),
                    &mut len,
                ),
                PamojaStatus::Ok
            );
            assert_eq!(len, 6);

            // The server answers the token the request carried.
            let answer = [0x01u8, 12, 0, 0, 0, 0];
            let (mut correction, mut has, mut more, mut resync, mut due) =
                (0i32, 0u8, 0u8, 0u8, 0u8);
            assert_eq!(
                pamoja_lorawan_clock_sync_heard(
                    sync,
                    answer.as_ptr(),
                    answer.len(),
                    &mut correction,
                    &mut has,
                    &mut more,
                    &mut resync,
                    &mut due,
                ),
                PamojaStatus::Ok
            );
            assert_eq!((correction, has, more), (12, 1, 0));
            let mut token = 0u8;
            assert_eq!(
                pamoja_lorawan_clock_sync_status(
                    sync,
                    &mut token,
                    ptr::null_mut(),
                    ptr::null_mut()
                ),
                PamojaStatus::Ok
            );
            assert_eq!(token, 1, "the token moved on with the correction");
            pamoja_lorawan_clock_sync_free(sync);

            let mut manager: *mut PamojaLorawanFirmware = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_firmware_new(0x0001_0203, 0xAABB_CCDD, &mut manager),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_lorawan_firmware_set_image(manager, PAMOJA_LORAWAN_IMAGE_VALID, 0x0001_0204),
                PamojaStatus::Ok
            );

            // DevUpgradeImageReq, then a reboot in an hour.
            let asked = [0x04u8, 0x03, 0x10, 0x0E, 0x00];
            let mut answers = [0u8; 32];
            let mut written = 0usize;
            assert_eq!(
                pamoja_lorawan_firmware_heard(
                    manager,
                    asked.as_ptr(),
                    asked.len(),
                    0,
                    0,
                    answers.as_mut_ptr(),
                    answers.len(),
                    &mut written,
                ),
                PamojaStatus::Ok
            );
            assert_eq!(&answers[..2], &[0x04, PAMOJA_LORAWAN_IMAGE_VALID]);

            let (mut left, mut has_in) = (0u32, 0u8);
            assert_eq!(
                pamoja_lorawan_firmware_reboot(
                    manager,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    &mut left,
                    &mut has_in,
                    ptr::null_mut(),
                ),
                PamojaStatus::Ok
            );
            assert_eq!((left, has_in), (3_600, 1));
            pamoja_lorawan_firmware_free(manager);
        }
    }
}
