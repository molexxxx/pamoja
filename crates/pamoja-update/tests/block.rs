//! An update that arrives as one block, the way a fragmented transport delivers it.
//!
//! The transport hands over bytes and vouches for nothing. These tests are the argument that
//! what comes out of one is held to exactly the rules an update fetched any other way is:
//! the framing only says where the manifest ends and the image begins, and every decision
//! after that is the manifest's.

use pamoja_security::DeviceIdentity;
use pamoja_update::block::{frame, split, DESCRIPTOR, HEADER_LEN};
use pamoja_update::{
    Device, Manifest, MemoryStore, PayloadFormat, Refusal, SlotState, SlotStore, Updater,
    ENVELOPE_MAX, STRUCTURE_VERSION,
};
use sha2::{Digest, Sha256};

/// The vendor every test device belongs to.
const VENDOR: [u8; 16] = [0xab; 16];

/// The device class every test device is.
const CLASS: [u8; 16] = [0xcd; 16];

/// The author whose releases the test devices trust.
fn author() -> DeviceIdentity {
    DeviceIdentity::from_seed(&[1u8; 32])
}

/// A manifest describing an image.
fn manifest(image: &[u8], sequence: u64, slot: u8) -> Manifest {
    Manifest {
        structure_version: STRUCTURE_VERSION,
        sequence,
        vendor_id: VENDOR,
        class_id: CLASS,
        format: PayloadFormat::Raw,
        storage: slot,
        digest: Sha256::digest(image).into(),
        size: image.len() as u32,
        expires: 0,
    }
}

/// A device already running sequence 1 in slot 0.
fn device() -> Updater<MemoryStore> {
    let device = Device {
        vendor_id: VENDOR,
        class_id: CLASS,
        anchor: author().public(),
    };
    let mut updater = Updater::new(device, MemoryStore::new(2, 4096));
    updater.provision(0, 1).expect("provision");
    updater
}

/// Frames a release the way a server would put it into one block.
fn release(image: &[u8], sequence: u64, slot: u8) -> ([u8; 4096], usize) {
    let mut envelope = [0u8; ENVELOPE_MAX];
    let written = manifest(image, sequence, slot)
        .sign(&author(), &mut envelope)
        .expect("sign");
    let mut block = [0u8; 4096];
    let len = frame(&envelope[..written], image, &mut block).expect("frame");
    (block, len)
}

#[test]
fn a_block_carries_the_manifest_and_the_image_it_describes() {
    let image = b"version two";
    let (block, len) = release(image, 2, 1);
    assert_eq!(&block[..4], &DESCRIPTOR, "the block names its convention");

    let (envelope, carried) = split(&block[..len]).expect("split");
    assert_eq!(carried, image);
    assert_eq!(len, HEADER_LEN + envelope.len() + image.len());

    let mut updater = device();
    assert_eq!(updater.stage(envelope, carried).expect("stage"), 1);
    assert_eq!(
        updater.store().record(1).expect("record").state,
        SlotState::Staged
    );
}

#[test]
fn a_block_that_is_not_this_convention_is_refused_before_anything_is_read() {
    assert_eq!(split(b"short"), Err(Refusal::Malformed));
    assert_eq!(
        split(b"OTHER\x00\x01\x02"),
        Err(Refusal::Malformed),
        "another vendor's block on the same transport"
    );

    let mut block = [0u8; 16];
    block[..4].copy_from_slice(&DESCRIPTOR);
    block[4..6].copy_from_slice(&999u16.to_le_bytes());
    assert_eq!(
        split(&block),
        Err(Refusal::Size),
        "a header that claims more than the block holds"
    );
}

#[test]
fn an_image_altered_in_the_block_is_refused_by_the_manifest() {
    let image = b"version two";
    let (mut block, len) = release(image, 2, 1);
    // Flip a byte of the image, which the transport would have no way to notice.
    block[len - 1] ^= 0xFF;

    let (envelope, carried) = split(&block[..len]).expect("split");
    let mut updater = device();
    assert_eq!(
        updater.stage(envelope, carried),
        Err(Refusal::Digest),
        "the digest the manifest commits to does not match"
    );
}

#[test]
fn a_manifest_altered_in_the_block_is_refused_by_its_signature() {
    let image = b"version two";
    let (mut block, len) = release(image, 2, 1);
    block[HEADER_LEN + 2] ^= 0xFF;

    let (envelope, carried) = split(&block[..len]).expect("split");
    let mut updater = device();
    assert!(
        matches!(
            updater.stage(envelope, carried),
            Err(Refusal::Signature | Refusal::Malformed)
        ),
        "a manifest nobody signed does not stage"
    );
}

#[test]
fn a_replayed_block_does_not_take_a_device_backward() {
    let image = b"version one again";
    let (block, len) = release(image, 1, 1);
    let (envelope, carried) = split(&block[..len]).expect("split");

    let mut updater = device();
    assert_eq!(
        updater.stage(envelope, carried),
        Err(Refusal::Rollback),
        "broadcasting an old release to everyone does not downgrade anyone"
    );
}

#[test]
fn a_block_arriving_in_pieces_is_staged_as_it_goes() {
    // A device short of memory writes each piece into the slot as it is reassembled,
    // rather than holding the whole image twice.
    let image: [u8; 512] = core::array::from_fn(|i| (i * 7 + 3) as u8);
    let (block, len) = release(&image, 2, 1);
    let (envelope, carried) = split(&block[..len]).expect("split");

    let mut updater = device();
    let mut staging = updater.begin(envelope).expect("begin");
    for piece in carried.chunks(48) {
        staging.write(piece).expect("write");
    }
    let slot = staging.finish().expect("finish");
    assert_eq!(slot, 1);
    assert_eq!(
        updater.store().record(1).expect("record").state,
        SlotState::Staged
    );
}
