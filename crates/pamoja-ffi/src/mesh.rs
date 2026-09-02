//! The C ABI for mesh packet framing.
//!
//! These functions wrap [`pamoja_mesh`] for callers that reach the SDK through the
//! flat C boundary: an addressed packet that hops node to node across a radio mesh,
//! and the duplicate suppressor that stops a flood from circulating forever.
//!
//! A frame carries a payload, so it crosses as an opaque handle like every other
//! payload-bearing type here. The duplicate cache is sized when it is built rather
//! than by the const generic the Rust crate uses, since a const generic cannot
//! cross a C ABI at all; [`PAMOJA_MESH_SEEN_DEFAULT_CAPACITY`] is what a caller
//! with no reason to choose should pass.

use std::ptr;

use pamoja_mesh::{crc16, DynamicSeenCache, Frame, MeshError};

use crate::{read_bytes, set_last_error, PamojaStatus};

/// The largest mesh frame, in bytes, including its header and checksum.
pub const PAMOJA_MESH_FRAME_MAX: usize = 250;

/// The length of a mesh frame header, in bytes.
pub const PAMOJA_MESH_HEADER_LEN: usize = 12;

/// The bytes a frame spends on its header and checksum together.
pub const PAMOJA_MESH_OVERHEAD: usize = 14;

/// The largest payload a single mesh frame can carry, in bytes.
pub const PAMOJA_MESH_PAYLOAD_MAX: usize = 236;

/// The mesh protocol version this build speaks.
pub const PAMOJA_MESH_VERSION: u8 = 1;

/// The hop limit a frame starts with unless one is set.
pub const PAMOJA_MESH_DEFAULT_HOP_LIMIT: u8 = 3;

/// The destination address that means every node.
pub const PAMOJA_MESH_BROADCAST: u32 = 0xFFFF_FFFF;

/// A reasonable duplicate-cache size for a caller with no reason to choose one.
pub const PAMOJA_MESH_SEEN_DEFAULT_CAPACITY: usize = 64;

/// An opaque handle to a mesh frame.
///
/// Read it with the `pamoja_mesh_frame_*` calls, then release it with
/// [`pamoja_mesh_frame_free`].
pub struct PamojaMeshFrame {
    frame: Frame,
}

/// An opaque handle to a cache of recently seen packets.
///
/// Feed it every frame a node receives; it answers whether that packet is new, so
/// a node relays each packet once however many copies reach it. Release it with
/// [`pamoja_mesh_seen_free`].
pub struct PamojaSeenCache {
    seen: DynamicSeenCache,
}

/// Builds a mesh frame addressed to one node.
///
/// # Arguments
///
/// * `src` - the address of this node.
/// * `dst` - the address the frame is for, or [`PAMOJA_MESH_BROADCAST`].
/// * `id` - the sequence number identifying this packet from this source.
/// * `payload` - the bytes to carry.
/// * `payload_len` - the payload length in bytes.
/// * `out_frame` - receives the new frame.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_frame` set to a handle the caller
/// must release with [`pamoja_mesh_frame_free`], or
/// [`PamojaStatus::InvalidArgument`] if the payload is larger than
/// [`PAMOJA_MESH_PAYLOAD_MAX`].
///
/// # Safety
///
/// `payload` must point to at least `payload_len` readable bytes when that length
/// is non-zero, and `out_frame` must point to a writable `*mut PamojaMeshFrame`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_frame_new(
    src: u32,
    dst: u32,
    id: u16,
    payload: *const u8,
    payload_len: usize,
    out_frame: *mut *mut PamojaMeshFrame,
) -> PamojaStatus {
    build(payload, payload_len, out_frame, |bytes| {
        Frame::new(src, dst, id, bytes)
    })
}

/// Builds a mesh frame addressed to every node.
///
/// # Arguments
///
/// * `src` - the address of this node.
/// * `id` - the sequence number identifying this packet from this source.
/// * `payload` - the bytes to carry.
/// * `payload_len` - the payload length in bytes.
/// * `out_frame` - receives the new frame.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_frame` set to a handle the caller
/// must release with [`pamoja_mesh_frame_free`], or
/// [`PamojaStatus::InvalidArgument`] if the payload is too long.
///
/// # Safety
///
/// `payload` must point to at least `payload_len` readable bytes when that length
/// is non-zero, and `out_frame` must point to a writable `*mut PamojaMeshFrame`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_frame_broadcast(
    src: u32,
    id: u16,
    payload: *const u8,
    payload_len: usize,
    out_frame: *mut *mut PamojaMeshFrame,
) -> PamojaStatus {
    build(payload, payload_len, out_frame, |bytes| {
        Frame::broadcast(src, id, bytes)
    })
}

/// Parses a frame received off a radio.
///
/// # Arguments
///
/// * `bytes` - the frame exactly as it arrived.
/// * `bytes_len` - its length.
/// * `out_frame` - receives the parsed frame.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_frame` set to a handle the caller
/// must release with [`pamoja_mesh_frame_free`], or [`PamojaStatus::Codec`] if the
/// frame is truncated, of an unknown version, or fails its checksum.
///
/// # Safety
///
/// `bytes` must point to at least `bytes_len` readable bytes when that length is
/// non-zero, and `out_frame` must point to a writable `*mut PamojaMeshFrame`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_frame_parse(
    bytes: *const u8,
    bytes_len: usize,
    out_frame: *mut *mut PamojaMeshFrame,
) -> PamojaStatus {
    build(bytes, bytes_len, out_frame, Frame::parse)
}

/// Sets the number of relays a frame may still take.
///
/// # Arguments
///
/// * `frame` - the frame to adjust.
/// * `hop_limit` - the new hop limit.
///
/// # Safety
///
/// `frame` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_frame_set_hop_limit(
    frame: *mut PamojaMeshFrame,
    hop_limit: u8,
) {
    if frame.is_null() {
        return;
    }
    let frame = &mut *frame;
    frame.frame = frame.frame.with_hop_limit(hop_limit);
}

/// Returns the protocol version a frame declares.
///
/// # Returns
///
/// The version, or 0 if `frame` is null.
///
/// # Safety
///
/// `frame` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_frame_version(frame: *const PamojaMeshFrame) -> u8 {
    if frame.is_null() {
        return 0;
    }
    (*frame).frame.version()
}

/// Returns the address of the node a frame came from.
///
/// # Returns
///
/// The source address, or 0 if `frame` is null.
///
/// # Safety
///
/// `frame` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_frame_src(frame: *const PamojaMeshFrame) -> u32 {
    if frame.is_null() {
        return 0;
    }
    (*frame).frame.src()
}

/// Returns the address a frame is addressed to.
///
/// # Returns
///
/// The destination address, or 0 if `frame` is null.
///
/// # Safety
///
/// `frame` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_frame_dst(frame: *const PamojaMeshFrame) -> u32 {
    if frame.is_null() {
        return 0;
    }
    (*frame).frame.dst()
}

/// Returns the sequence number that identifies a packet from its source.
///
/// # Returns
///
/// The sequence number, or 0 if `frame` is null.
///
/// # Safety
///
/// `frame` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_frame_id(frame: *const PamojaMeshFrame) -> u16 {
    if frame.is_null() {
        return 0;
    }
    (*frame).frame.id()
}

/// Returns how many further relays a frame may take.
///
/// # Returns
///
/// The hop limit, or 0 if `frame` is null.
///
/// # Safety
///
/// `frame` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_frame_hop_limit(frame: *const PamojaMeshFrame) -> u8 {
    if frame.is_null() {
        return 0;
    }
    (*frame).frame.hop_limit()
}

/// Reports whether a frame is addressed to every node.
///
/// # Returns
///
/// `true` for a broadcast, or `false` if `frame` is null.
///
/// # Safety
///
/// `frame` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_frame_is_broadcast(frame: *const PamojaMeshFrame) -> bool {
    !frame.is_null() && (*frame).frame.is_broadcast()
}

/// Returns a pointer to the payload a frame carries.
///
/// Use [`pamoja_mesh_frame_payload_len`] for the length. The pointer is valid
/// until the frame is freed.
///
/// # Returns
///
/// A pointer to the payload, or null if `frame` is null or the payload is empty.
///
/// # Safety
///
/// `frame` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_frame_payload(frame: *const PamojaMeshFrame) -> *const u8 {
    if frame.is_null() {
        return ptr::null();
    }
    let payload = (*frame).frame.payload();
    if payload.is_empty() {
        ptr::null()
    } else {
        payload.as_ptr()
    }
}

/// Returns the length in bytes of the payload a frame carries.
///
/// # Returns
///
/// The payload length, or 0 if `frame` is null.
///
/// # Safety
///
/// `frame` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_frame_payload_len(frame: *const PamojaMeshFrame) -> usize {
    if frame.is_null() {
        return 0;
    }
    (*frame).frame.payload().len()
}

/// Returns a pointer to the whole frame as it goes on the air.
///
/// Use [`pamoja_mesh_frame_bytes_len`] for the length. The pointer is valid until
/// the frame is freed.
///
/// # Returns
///
/// A pointer to the encoded frame, or null if `frame` is null.
///
/// # Safety
///
/// `frame` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_frame_bytes(frame: *const PamojaMeshFrame) -> *const u8 {
    if frame.is_null() {
        return ptr::null();
    }
    (*frame).frame.as_bytes().as_ptr()
}

/// Returns the length in bytes of the whole frame.
///
/// # Returns
///
/// The encoded length, or 0 if `frame` is null.
///
/// # Safety
///
/// `frame` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_frame_bytes_len(frame: *const PamojaMeshFrame) -> usize {
    if frame.is_null() {
        return 0;
    }
    (*frame).frame.as_bytes().len()
}

/// Returns the same frame with one hop spent, ready to forward.
///
/// # Arguments
///
/// * `frame` - the frame just received.
/// * `out_frame` - receives the frame to forward.
///
/// # Returns
///
/// `true` when the frame still had a hop to spend, with `*out_frame` set to a
/// handle the caller must release with [`pamoja_mesh_frame_free`], or `false` when
/// its hops have run out and it must not be relayed further.
///
/// # Safety
///
/// `frame` must be a live handle from a call that produced one, or null, and
/// `out_frame` must point to a writable `*mut PamojaMeshFrame`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_frame_relayed(
    frame: *const PamojaMeshFrame,
    out_frame: *mut *mut PamojaMeshFrame,
) -> bool {
    if out_frame.is_null() {
        return false;
    }
    let slot = &mut *out_frame;
    *slot = ptr::null_mut();
    if frame.is_null() {
        return false;
    }
    match (*frame).frame.relayed() {
        Some(forwarded) => {
            *slot = Box::into_raw(Box::new(PamojaMeshFrame { frame: forwarded }));
            true
        }
        None => false,
    }
}

/// Releases a mesh frame handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `frame` must be a handle from a call that produced one and that has not already
/// been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_frame_free(frame: *mut PamojaMeshFrame) {
    if !frame.is_null() {
        drop(Box::from_raw(frame));
    }
}

/// Computes the CRC-16 a mesh frame carries.
///
/// # Arguments
///
/// * `data` - the bytes the checksum covers.
/// * `data_len` - their length.
///
/// # Returns
///
/// The checksum, or 0 if `data` is null with a non-zero length.
///
/// # Safety
///
/// `data` must point to at least `data_len` readable bytes when that length is
/// non-zero.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_crc16(data: *const u8, data_len: usize) -> u16 {
    match read_bytes(data, data_len) {
        Ok(bytes) => crc16(&bytes),
        Err(_) => 0,
    }
}

/// Creates an empty duplicate cache.
///
/// # Arguments
///
/// * `capacity` - how many recently seen packets to remember; pass
///   [`PAMOJA_MESH_SEEN_DEFAULT_CAPACITY`] when there is no reason to choose. A
///   capacity of zero remembers nothing, so every copy of a packet is relayed.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_mesh_seen_free`].
#[no_mangle]
pub extern "C" fn pamoja_mesh_seen_new(capacity: usize) -> *mut PamojaSeenCache {
    Box::into_raw(Box::new(PamojaSeenCache {
        seen: DynamicSeenCache::new(capacity),
    }))
}

/// Reports whether a packet is currently remembered, without recording it.
///
/// # Arguments
///
/// * `cache` - the duplicate cache.
/// * `src` - the address the packet came from.
/// * `id` - the sequence number the packet carries.
///
/// # Returns
///
/// `true` if the packet has been seen recently, or `false` if `cache` is null.
///
/// # Safety
///
/// `cache` must be a live handle from [`pamoja_mesh_seen_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_seen_contains(
    cache: *const PamojaSeenCache,
    src: u32,
    id: u16,
) -> bool {
    !cache.is_null() && (*cache).seen.contains((src, id))
}

/// Records a packet and reports whether it was new.
///
/// # Arguments
///
/// * `cache` - the duplicate cache.
/// * `src` - the address the packet came from.
/// * `id` - the sequence number the packet carries.
///
/// # Returns
///
/// `true` if the packet had not been seen, which is when a node should act on it
/// and relay it, or `false` for a duplicate or a null cache.
///
/// # Safety
///
/// `cache` must be a live handle from [`pamoja_mesh_seen_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_seen_record(
    cache: *mut PamojaSeenCache,
    src: u32,
    id: u16,
) -> bool {
    if cache.is_null() {
        return false;
    }
    (*cache).seen.record((src, id))
}

/// Returns how many packets a duplicate cache remembers.
///
/// # Returns
///
/// The capacity it was created with, or 0 if `cache` is null.
///
/// # Safety
///
/// `cache` must be a live handle from [`pamoja_mesh_seen_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_seen_capacity(cache: *const PamojaSeenCache) -> usize {
    if cache.is_null() {
        return 0;
    }
    (*cache).seen.capacity()
}

/// Releases a duplicate cache handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `cache` must be a handle from [`pamoja_mesh_seen_new`] that has not already
/// been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mesh_seen_free(cache: *mut PamojaSeenCache) {
    if !cache.is_null() {
        drop(Box::from_raw(cache));
    }
}

/// Runs a frame constructor over a borrowed buffer and hands back a handle.
///
/// # Safety
///
/// `bytes` must point to at least `len` readable bytes when that length is
/// non-zero, and `out_frame` must point to a writable `*mut PamojaMeshFrame`.
unsafe fn build(
    bytes: *const u8,
    len: usize,
    out_frame: *mut *mut PamojaMeshFrame,
    construct: impl FnOnce(&[u8]) -> Result<Frame, MeshError>,
) -> PamojaStatus {
    if out_frame.is_null() {
        set_last_error("out_frame must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_frame;
    *slot = ptr::null_mut();

    let bytes = match read_bytes(bytes, len) {
        Ok(bytes) => bytes,
        Err(status) => return status,
    };
    match construct(&bytes) {
        Ok(frame) => {
            *slot = Box::into_raw(Box::new(PamojaMeshFrame { frame }));
            PamojaStatus::Ok
        }
        Err(error) => failed(error),
    }
}

/// Records a mesh error and classifies it.
///
/// # Arguments
///
/// * `error` - the failure the mesh crate reported.
///
/// # Returns
///
/// [`PamojaStatus::InvalidArgument`] when the caller asked for something no frame
/// can hold, and [`PamojaStatus::Codec`] when a received frame could not be read.
fn failed(error: MeshError) -> PamojaStatus {
    set_last_error(error.to_string());
    match error {
        MeshError::PayloadTooLong | MeshError::FrameTooLong => PamojaStatus::InvalidArgument,
        _ => PamojaStatus::Codec,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_constants_match_the_mesh_crate() {
        assert_eq!(PAMOJA_MESH_FRAME_MAX, Frame::MAX_LEN);
        assert_eq!(PAMOJA_MESH_HEADER_LEN, Frame::HEADER_LEN);
        assert_eq!(PAMOJA_MESH_OVERHEAD, Frame::OVERHEAD);
        assert_eq!(PAMOJA_MESH_PAYLOAD_MAX, Frame::MAX_PAYLOAD);
        assert_eq!(PAMOJA_MESH_VERSION, Frame::VERSION);
        assert_eq!(PAMOJA_MESH_DEFAULT_HOP_LIMIT, Frame::DEFAULT_HOP_LIMIT);
        assert_eq!(PAMOJA_MESH_BROADCAST, pamoja_mesh::BROADCAST);
    }

    #[test]
    fn a_broadcast_round_trips_through_the_boundary() {
        let payload = b"level=high";
        let mut frame = ptr::null_mut();
        // Safety: the payload and out-pointer are both valid.
        unsafe {
            assert_eq!(
                pamoja_mesh_frame_broadcast(
                    0x1234_5678,
                    1,
                    payload.as_ptr(),
                    payload.len(),
                    &mut frame
                ),
                PamojaStatus::Ok
            );
            let on_air = std::slice::from_raw_parts(
                pamoja_mesh_frame_bytes(frame),
                pamoja_mesh_frame_bytes_len(frame),
            )
            .to_vec();
            pamoja_mesh_frame_free(frame);

            let mut received = ptr::null_mut();
            assert_eq!(
                pamoja_mesh_frame_parse(on_air.as_ptr(), on_air.len(), &mut received),
                PamojaStatus::Ok
            );
            assert!(pamoja_mesh_frame_is_broadcast(received));
            assert_eq!(pamoja_mesh_frame_src(received), 0x1234_5678);
            assert_eq!(pamoja_mesh_frame_id(received), 1);
            assert_eq!(pamoja_mesh_frame_version(received), PAMOJA_MESH_VERSION);
            let recovered = std::slice::from_raw_parts(
                pamoja_mesh_frame_payload(received),
                pamoja_mesh_frame_payload_len(received),
            );
            assert_eq!(recovered, payload);
            pamoja_mesh_frame_free(received);
        }
    }

    #[test]
    fn a_corrupt_frame_is_refused() {
        let payload = b"reading";
        let mut frame = ptr::null_mut();
        // Safety: the payload and out-pointer are both valid.
        unsafe {
            assert_eq!(
                pamoja_mesh_frame_new(1, 2, 7, payload.as_ptr(), payload.len(), &mut frame),
                PamojaStatus::Ok
            );
            let mut on_air = std::slice::from_raw_parts(
                pamoja_mesh_frame_bytes(frame),
                pamoja_mesh_frame_bytes_len(frame),
            )
            .to_vec();
            pamoja_mesh_frame_free(frame);
            on_air[PAMOJA_MESH_HEADER_LEN] ^= 0xFF;

            let mut received = ptr::null_mut();
            assert_eq!(
                pamoja_mesh_frame_parse(on_air.as_ptr(), on_air.len(), &mut received),
                PamojaStatus::Codec
            );
            assert!(received.is_null());
        }
    }

    #[test]
    fn a_frame_stops_relaying_when_its_hops_run_out() {
        let mut frame = ptr::null_mut();
        // Safety: the out-pointers are valid.
        unsafe {
            assert_eq!(
                pamoja_mesh_frame_broadcast(9, 1, ptr::null(), 0, &mut frame),
                PamojaStatus::Ok
            );
            assert_eq!(pamoja_mesh_frame_payload_len(frame), 0);
            assert!(pamoja_mesh_frame_payload(frame).is_null());
            pamoja_mesh_frame_set_hop_limit(frame, 1);

            let mut once = ptr::null_mut();
            assert!(pamoja_mesh_frame_relayed(frame, &mut once));
            assert_eq!(pamoja_mesh_frame_hop_limit(once), 0);

            let mut twice = ptr::null_mut();
            assert!(!pamoja_mesh_frame_relayed(once, &mut twice));
            assert!(twice.is_null());

            pamoja_mesh_frame_free(once);
            pamoja_mesh_frame_free(frame);
        }
    }

    #[test]
    fn the_cache_recognises_a_packet_it_has_already_seen() {
        let cache = pamoja_mesh_seen_new(PAMOJA_MESH_SEEN_DEFAULT_CAPACITY);
        // Safety: the cache handle was just created.
        unsafe {
            assert!(!pamoja_mesh_seen_contains(cache, 0x42, 1));
            assert!(pamoja_mesh_seen_record(cache, 0x42, 1));
            assert!(pamoja_mesh_seen_contains(cache, 0x42, 1));
            assert!(!pamoja_mesh_seen_record(cache, 0x42, 1));
            assert!(pamoja_mesh_seen_record(cache, 0x42, 2));
            assert_eq!(
                pamoja_mesh_seen_capacity(cache),
                PAMOJA_MESH_SEEN_DEFAULT_CAPACITY
            );
            pamoja_mesh_seen_free(cache);

            // A cache sized by the caller evicts at the size it was given.
            let small = pamoja_mesh_seen_new(2);
            assert_eq!(pamoja_mesh_seen_capacity(small), 2);
            assert!(pamoja_mesh_seen_record(small, 1, 1));
            assert!(pamoja_mesh_seen_record(small, 1, 2));
            assert!(pamoja_mesh_seen_record(small, 1, 3));
            assert!(!pamoja_mesh_seen_contains(small, 1, 1));
            pamoja_mesh_seen_free(small);
        }
    }

    #[test]
    fn the_checksum_matches_the_mesh_crate() {
        let data = b"level=high";
        // Safety: the buffer is valid for its length.
        let checksum = unsafe { pamoja_mesh_crc16(data.as_ptr(), data.len()) };
        assert_eq!(checksum, crc16(data));
    }

    #[test]
    fn null_handles_are_tolerated() {
        // Safety: every call below is documented to accept null.
        unsafe {
            assert_eq!(pamoja_mesh_frame_src(ptr::null()), 0);
            assert_eq!(pamoja_mesh_frame_dst(ptr::null()), 0);
            assert!(!pamoja_mesh_frame_is_broadcast(ptr::null()));
            assert!(pamoja_mesh_frame_bytes(ptr::null()).is_null());
            pamoja_mesh_frame_set_hop_limit(ptr::null_mut(), 3);
            pamoja_mesh_frame_free(ptr::null_mut());
            assert!(!pamoja_mesh_seen_record(ptr::null_mut(), 1, 1));
            pamoja_mesh_seen_free(ptr::null_mut());
        }
    }
}
