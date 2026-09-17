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

#[cfg(test)]
mod tests {
    use super::*;

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
