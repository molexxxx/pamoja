//! The rules a device holds an update to, exercised end to end.
//!
//! Each test states one rule and the attack or accident it prevents. Together they
//! are the argument that an image reaching a device over any carrier at all is
//! still safe to run, and that a device which takes a bad one comes back.

use pamoja_security::DeviceIdentity;
use pamoja_update::{
    Boot, Delegation, Device, Envelope, Manifest, MemoryStore, PayloadFormat, Refusal, SlotState,
    SlotStore, Updater, DELEGATION_MAX, ENVELOPE_MAX, STRUCTURE_VERSION,
};
use sha2::{Digest, Sha256};

/// The vendor every test device belongs to.
const VENDOR: [u8; 16] = [0xab; 16];

/// The device class every test device is.
const CLASS: [u8; 16] = [0xcd; 16];

/// Builds the author whose releases the test devices trust.
fn author() -> DeviceIdentity {
    DeviceIdentity::from_seed(&[1u8; 32])
}

/// Builds a manifest describing `image`, with the fields a test wants to vary.
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

/// Signs a manifest into an envelope buffer.
fn release_manifest(manifest: &Manifest, by: &DeviceIdentity) -> ([u8; ENVELOPE_MAX], usize) {
    let mut buf = [0u8; ENVELOPE_MAX];
    let written = manifest.sign(by, &mut buf).expect("sign");
    (buf, written)
}

/// Builds a provisioned device already running sequence 1 in slot 0.
fn device_running_version_one() -> Updater<MemoryStore> {
    let device = Device {
        vendor_id: VENDOR,
        class_id: CLASS,
        anchor: author().public(),
    };
    let mut updater = Updater::new(device, MemoryStore::new(2, 4096));
    updater.provision(0, 1).expect("provision");
    updater
}

#[test]
fn an_update_is_staged_tried_and_confirmed() {
    let mut updater = device_running_version_one();
    let image = b"version two";
    let (envelope, len) = release_manifest(&manifest(image, 2, 1), &author());

    assert_eq!(updater.stage(&envelope[..len], image).expect("stage"), 1);
    assert_eq!(
        updater.store().record(1).expect("record").state,
        SlotState::Staged
    );

    // The bootloader tries it, which makes it pending until it says it is healthy.
    assert_eq!(updater.on_boot().expect("boot"), Boot::Trying(1));
    assert_eq!(updater.confirm().expect("confirm"), 1);

    // The slot it replaced is now free for the next update.
    assert_eq!(
        updater.store().record(0).expect("record").state,
        SlotState::Empty
    );
    assert_eq!(updater.on_boot().expect("boot"), Boot::Confirmed(1));
}

#[test]
fn an_image_that_never_confirms_is_reverted_on_the_next_boot() {
    let mut updater = device_running_version_one();
    let image = b"version two, which hangs";
    let (envelope, len) = release_manifest(&manifest(image, 2, 1), &author());
    updater.stage(&envelope[..len], image).expect("stage");

    // Tried once, and then the device resets without ever confirming.
    assert_eq!(updater.on_boot().expect("boot"), Boot::Trying(1));
    assert_eq!(
        updater.on_boot().expect("boot"),
        Boot::Reverted {
            failed: 1,
            fallback: 0
        },
        "an image that does not report itself healthy must not be tried forever"
    );

    // It is not tried again, and the device is back on what worked.
    assert_eq!(updater.on_boot().expect("boot"), Boot::Confirmed(0));
    assert_eq!(
        updater.store().record(1).expect("record").state,
        SlotState::Failed
    );
}

#[test]
fn a_running_image_can_give_up_on_itself() {
    let mut updater = device_running_version_one();
    let image = b"version two, which knows it is unwell";
    let (envelope, len) = release_manifest(&manifest(image, 2, 1), &author());
    updater.stage(&envelope[..len], image).expect("stage");
    updater.on_boot().expect("boot");

    assert_eq!(updater.revert().expect("revert"), 0);
    assert_eq!(updater.on_boot().expect("boot"), Boot::Confirmed(0));
}

#[test]
fn an_older_release_is_refused() {
    let mut updater = device_running_version_one();
    let image = b"version zero, captured and replayed";
    let (envelope, len) = release_manifest(&manifest(image, 1, 1), &author());

    assert_eq!(
        updater.stage(&envelope[..len], image),
        Err(Refusal::Rollback),
        "a replayed release must not take a device backwards to a known-bad image"
    );
}

#[test]
fn re_releasing_the_same_sequence_is_refused() {
    let mut updater = device_running_version_one();
    let image = b"version two";
    let (envelope, len) = release_manifest(&manifest(image, 2, 1), &author());
    updater.stage(&envelope[..len], image).expect("stage");
    updater.on_boot().expect("boot");
    updater.confirm().expect("confirm");

    // Even a genuinely different image cannot reuse a sequence number, because a
    // device has no way to tell which of the two it is being handed.
    let fixed = b"version two, fixed";
    let (envelope, len) = release_manifest(&manifest(fixed, 2, 0), &author());
    assert_eq!(
        updater.stage(&envelope[..len], fixed),
        Err(Refusal::Rollback)
    );
}

#[test]
fn a_sequence_that_already_failed_is_still_spent() {
    let mut updater = device_running_version_one();
    let bad = b"version two, broken";
    let (envelope, len) = release_manifest(&manifest(bad, 2, 1), &author());
    updater.stage(&envelope[..len], bad).expect("stage");
    updater.on_boot().expect("boot");
    updater.on_boot().expect("boot"); // never confirmed, so it fails

    let fixed = b"version two, repaired";
    let (envelope, len) = release_manifest(&manifest(fixed, 2, 1), &author());
    assert_eq!(
        updater.stage(&envelope[..len], fixed),
        Err(Refusal::Rollback),
        "a fix has to be released under a new number, or the broken one could return"
    );
}

#[test]
fn an_update_for_another_vendor_or_class_is_refused() {
    let image = b"someone else's firmware";

    let mut updater = device_running_version_one();
    let mut wrong_vendor = manifest(image, 2, 1);
    wrong_vendor.vendor_id = [0x99; 16];
    let (envelope, len) = release_manifest(&wrong_vendor, &author());
    assert_eq!(
        updater.stage(&envelope[..len], image),
        Err(Refusal::WrongDevice)
    );

    let mut wrong_class = manifest(image, 2, 1);
    wrong_class.class_id = [0x99; 16];
    let (envelope, len) = release_manifest(&wrong_class, &author());
    assert_eq!(
        updater.stage(&envelope[..len], image),
        Err(Refusal::WrongDevice),
        "firmware for a different device must not brick this one"
    );
}

#[test]
fn a_release_from_an_untrusted_author_is_refused() {
    let mut updater = device_running_version_one();
    let image = b"version two, from a stranger";
    let impostor = DeviceIdentity::from_seed(&[2u8; 32]);
    let (envelope, len) = release_manifest(&manifest(image, 2, 1), &impostor);

    assert_eq!(
        updater.stage(&envelope[..len], image),
        Err(Refusal::Signature)
    );
}

#[test]
fn an_image_that_does_not_match_its_manifest_leaves_nothing_bootable() {
    let mut updater = device_running_version_one();
    let image = b"version two";
    let (envelope, len) = release_manifest(&manifest(image, 2, 1), &author());

    let mut altered = *image;
    altered[0] ^= 0x01;
    assert_eq!(
        updater.stage(&envelope[..len], &altered),
        Err(Refusal::Digest)
    );

    // The slot took bytes and keeps them for a retry, but was never marked
    // bootable, so a device that lost power here still runs the image it confirmed.
    assert_eq!(
        updater.store().record(1).expect("record").state,
        SlotState::Receiving
    );
    assert_eq!(updater.on_boot().expect("boot"), Boot::Confirmed(0));
}

#[test]
fn an_interrupted_transfer_leaves_nothing_bootable() {
    let mut updater = device_running_version_one();
    let image = b"version two, arriving slowly";
    let (envelope, len) = release_manifest(&manifest(image, 2, 1), &author());

    {
        let mut staging = updater.begin(&envelope[..len]).expect("begin");
        staging.write(&image[..5]).expect("write");
        // The link drops here and the staging is dropped without finishing.
    }

    assert_eq!(
        updater.store().record(1).expect("record").state,
        SlotState::Receiving
    );
    assert_eq!(updater.on_boot().expect("boot"), Boot::Confirmed(0));
}

#[test]
fn an_image_larger_than_declared_is_stopped_while_it_arrives() {
    let mut updater = device_running_version_one();
    let image = b"version two";
    let (envelope, len) = release_manifest(&manifest(image, 2, 1), &author());

    let mut staging = updater.begin(&envelope[..len]).expect("begin");
    staging.write(image).expect("write");
    assert_eq!(staging.write(b"and more"), Err(Refusal::Size));
}

#[test]
fn an_image_too_large_for_the_slot_is_refused_before_any_of_it_arrives() {
    let device = Device {
        vendor_id: VENDOR,
        class_id: CLASS,
        anchor: author().public(),
    };
    let mut updater = Updater::new(device, MemoryStore::new(2, 16));
    updater.provision(0, 1).expect("provision");

    let image = [0u8; 64];
    let (envelope, len) = release_manifest(&manifest(&image, 2, 1), &author());
    assert_eq!(
        updater.stage(&envelope[..len], &image),
        Err(Refusal::SlotTooSmall),
        "the declared size is checked against the slot before a byte is written"
    );
}

#[test]
fn an_update_may_not_overwrite_the_slot_it_would_fall_back_to() {
    let mut updater = device_running_version_one();
    let image = b"version two, aimed at the running slot";
    let (envelope, len) = release_manifest(&manifest(image, 2, 0), &author());

    assert_eq!(
        updater.stage(&envelope[..len], image),
        Err(Refusal::WrongState),
        "writing over the confirmed image would leave nothing to come back to"
    );
}

#[test]
fn a_slot_the_device_does_not_have_is_refused() {
    let mut updater = device_running_version_one();
    let image = b"version two";
    let (envelope, len) = release_manifest(&manifest(image, 2, 7), &author());

    assert_eq!(
        updater.stage(&envelope[..len], image),
        Err(Refusal::NoSuchSlot)
    );
}

#[test]
fn confirming_when_nothing_is_pending_does_nothing() {
    let mut updater = device_running_version_one();
    assert_eq!(updater.confirm(), Err(Refusal::WrongState));

    let image = b"version two";
    let (envelope, len) = release_manifest(&manifest(image, 2, 1), &author());
    updater.stage(&envelope[..len], image).expect("stage");
    updater.on_boot().expect("boot");
    updater.confirm().expect("confirm");

    // A second confirmation has nothing to act on, rather than confirming again.
    assert_eq!(updater.confirm(), Err(Refusal::WrongState));
}

#[test]
fn a_device_with_nothing_installed_has_nothing_to_boot() {
    let device = Device {
        vendor_id: VENDOR,
        class_id: CLASS,
        anchor: author().public(),
    };
    let mut updater = Updater::new(device, MemoryStore::new(2, 4096));
    assert_eq!(updater.on_boot(), Err(Refusal::NothingToRevert));
}

#[test]
fn a_garbled_envelope_is_refused_before_anything_else_is_read() {
    let mut updater = device_running_version_one();
    assert!(updater.stage(b"not an envelope at all", b"image").is_err());
    assert!(Envelope::decode(&[0xff, 0xff, 0xff]).is_err());
}

#[test]
fn two_updates_in_a_row_alternate_slots() {
    let mut updater = device_running_version_one();

    let second = b"version two";
    let (envelope, len) = release_manifest(&manifest(second, 2, 1), &author());
    assert_eq!(updater.stage(&envelope[..len], second).expect("stage"), 1);
    updater.on_boot().expect("boot");
    updater.confirm().expect("confirm");

    // Slot 0 is free again, so the next release goes there.
    let third = b"version three";
    let (envelope, len) = release_manifest(&manifest(third, 3, 0), &author());
    assert_eq!(updater.stage(&envelope[..len], third).expect("stage"), 0);
    assert_eq!(updater.on_boot().expect("boot"), Boot::Trying(0));
    assert_eq!(updater.confirm().expect("confirm"), 0);
    assert_eq!(updater.installed_sequence().expect("sequence"), 3);
}

#[test]
fn an_expired_release_is_refused_even_though_it_is_newer() {
    // The threat RFC 9124 calls THREAT.IMG.EXPIRED.OFFLINE: a device that has been
    // out of contact is handed a release genuinely newer than the one it runs, but
    // old enough to have a known flaw. Its sequence number alone looks fine.
    let mut updater = device_running_version_one();
    let image = b"version two, superseded months ago";
    let mut expiring = manifest(image, 2, 1);
    expiring.expires = 1_000;
    let (envelope, len) = release_manifest(&expiring, &author());

    assert_eq!(
        updater.stage_at(&envelope[..len], image, Some(2_000)),
        Err(Refusal::Expired),
        "an expiry bounds how long a stale release stays usable"
    );
}

#[test]
fn a_release_inside_its_window_is_accepted() {
    let mut updater = device_running_version_one();
    let image = b"version two, still current";
    let mut expiring = manifest(image, 2, 1);
    expiring.expires = 5_000;
    let (envelope, len) = release_manifest(&expiring, &author());

    assert_eq!(
        updater
            .stage_at(&envelope[..len], image, Some(1_000))
            .expect("stage"),
        1
    );
}

#[test]
fn a_device_with_no_clock_refuses_a_release_that_expires() {
    let mut updater = device_running_version_one();
    let image = b"version two";
    let mut expiring = manifest(image, 2, 1);
    expiring.expires = 5_000;
    let (envelope, len) = release_manifest(&expiring, &author());

    // Accepting it would silently ignore a bound the author asked for, so the
    // refusal is the honest answer rather than the convenient one.
    assert_eq!(
        updater.stage(&envelope[..len], image),
        Err(Refusal::NoClock)
    );
}

#[test]
fn a_release_that_never_expires_needs_no_clock() {
    let mut updater = device_running_version_one();
    let image = b"version two, no expiry set";
    let (envelope, len) = release_manifest(&manifest(image, 2, 1), &author());
    assert_eq!(updater.stage(&envelope[..len], image).expect("stage"), 1);
}

#[test]
fn the_expiry_is_covered_by_the_signature() {
    // If the expiry were outside the signed body, an attacker could extend it and
    // keep a stale release alive, which would make the field decorative.
    let mut updater = device_running_version_one();
    let image = b"version two";
    let mut expiring = manifest(image, 2, 1);
    expiring.expires = 1_000;
    let (mut envelope, len) = release_manifest(&expiring, &author());

    let at = envelope[..len]
        .windows(2)
        .position(|pair| pair == [0x09, 0x19])
        .expect("the expiry key and its two-byte argument");
    envelope[at + 2] ^= 0xff;

    assert_eq!(
        updater.stage_at(&envelope[..len], image, Some(2_000)),
        Err(Refusal::Signature)
    );
}

#[test]
fn a_transfer_cut_off_by_a_dead_link_picks_up_where_it_stopped() {
    let mut updater = device_running_version_one();
    let image = b"version two, arriving over a slow radio that keeps dropping";
    let (envelope, len) = release_manifest(&manifest(image, 2, 1), &author());

    // The link carries a third of it, then dies.
    {
        let mut staging = updater.begin(&envelope[..len]).expect("begin");
        staging.write(&image[..20]).expect("write");
        assert_eq!(staging.progress(), (20, image.len() as u32));
    }

    // The device reboots and is offered the same release again. It should not ask
    // for the part it already has.
    {
        let mut staging = updater.resume_at(&envelope[..len], None).expect("resume");
        assert_eq!(
            staging.progress(),
            (20, image.len() as u32),
            "a resumed transfer must not start from zero"
        );
        staging.write(&image[20..]).expect("write");
        assert_eq!(staging.finish().expect("finish"), 1);
    }

    // And the resumed image is the real one, whole.
    assert_eq!(updater.on_boot().expect("boot"), Boot::Trying(1));
    assert_eq!(updater.confirm().expect("confirm"), 1);
}

#[test]
fn a_transfer_cut_off_after_its_last_byte_is_not_thrown_away() {
    let mut updater = device_running_version_one();
    let image = b"version two, whole but never settled";
    let (envelope, len) = release_manifest(&manifest(image, 2, 1), &author());

    // Every byte arrives, and then the device resets before the image is settled.
    {
        let mut staging = updater.begin(&envelope[..len]).expect("begin");
        staging.write(image).expect("write");
        assert_eq!(staging.progress(), (image.len() as u32, image.len() as u32));
    }

    // Half an hour of radio time is not worth throwing away for the sake of one
    // call, so the resumed transfer settles what already arrived.
    let staging = updater.resume_at(&envelope[..len], None).expect("resume");
    assert_eq!(
        staging.progress(),
        (image.len() as u32, image.len() as u32),
        "a complete but unsettled image must not be erased"
    );
    assert_eq!(staging.finish().expect("finish"), 1);

    assert_eq!(updater.on_boot().expect("boot"), Boot::Trying(1));
    assert_eq!(updater.confirm().expect("confirm"), 1);
}

#[test]
fn a_resume_that_completes_the_wrong_bytes_still_fails_the_digest() {
    let mut updater = device_running_version_one();
    let image = b"version two, arriving over a slow radio that keeps dropping";
    let (envelope, len) = release_manifest(&manifest(image, 2, 1), &author());

    {
        let mut staging = updater.begin(&envelope[..len]).expect("begin");
        staging.write(&image[..20]).expect("write");
    }

    // Resuming does not make the earlier bytes trusted; the whole image is still
    // judged by the digest at the end.
    let mut staging = updater.resume_at(&envelope[..len], None).expect("resume");
    let mut rest = image[20..].to_vec();
    rest[0] ^= 0x01;
    staging.write(&rest).expect("write");
    assert_eq!(staging.finish(), Err(Refusal::Digest));

    assert_eq!(updater.on_boot().expect("boot"), Boot::Confirmed(0));
}

#[test]
fn a_different_release_does_not_resume_a_partial_transfer() {
    let mut updater = device_running_version_one();

    // The two images are deliberately the same length, so only their digests tell
    // them apart. A length check alone would let one be spliced onto the other.
    let first = b"version two.....never finished arriving";
    let second = b"version three...a different release now";
    assert_eq!(first.len(), second.len());

    let (envelope, len) = release_manifest(&manifest(first, 2, 1), &author());
    {
        let mut staging = updater.begin(&envelope[..len]).expect("begin");
        staging.write(&first[..10]).expect("write");
    }

    let (envelope, len) = release_manifest(&manifest(second, 3, 1), &author());
    let staging = updater.resume_at(&envelope[..len], None).expect("resume");
    assert_eq!(
        staging.progress(),
        (0, second.len() as u32),
        "two images must never be spliced together"
    );
}

#[test]
fn a_partial_transfer_is_never_bootable() {
    let mut updater = device_running_version_one();
    let image = b"version two, still in flight";
    let (envelope, len) = release_manifest(&manifest(image, 2, 1), &author());

    let mut staging = updater.begin(&envelope[..len]).expect("begin");
    staging.write(&image[..8]).expect("write");
    drop(staging);

    assert_eq!(
        updater.store().record(1).expect("record").state,
        SlotState::Receiving
    );
    assert_eq!(
        updater.on_boot().expect("boot"),
        Boot::Confirmed(0),
        "a half-arrived image must never be selected to run"
    );
}

#[test]
fn a_partial_transfer_does_not_block_its_own_resume() {
    // The in-flight sequence must not count as installed, or offering the same
    // release again to continue it would be refused as a rollback.
    let mut updater = device_running_version_one();
    let image = b"version two";
    let (envelope, len) = release_manifest(&manifest(image, 2, 1), &author());
    {
        let mut staging = updater.begin(&envelope[..len]).expect("begin");
        staging.write(&image[..4]).expect("write");
    }
    assert_eq!(updater.installed_sequence().expect("sequence"), 1);
    assert!(updater.resume_at(&envelope[..len], None).is_ok());
}

/// Signs a delegation naming the release key derived from `seed`.
fn delegate(epoch: u64, seed: u8, by: &DeviceIdentity) -> ([u8; DELEGATION_MAX], usize) {
    let delegation = Delegation {
        epoch,
        release_key: DeviceIdentity::from_seed(&[seed; 32]).public().to_bytes(),
        expires: 0,
    };
    let mut buf = [0u8; DELEGATION_MAX];
    let written = delegation.sign(by, &mut buf).expect("sign");
    (buf, written)
}

#[test]
fn a_delegated_key_may_sign_releases_and_the_anchor_no_longer_does() {
    let mut updater = device_running_version_one();
    let release = DeviceIdentity::from_seed(&[7u8; 32]);

    let (delegation, len) = delegate(1, 7, &author());
    updater.adopt(&delegation[..len], None).expect("adopt");

    // The delegated key now carries the authority.
    let image = b"version two, signed by the release key";
    let (envelope, elen) = release_manifest(&manifest(image, 2, 1), &release);
    assert_eq!(updater.stage(&envelope[..elen], image).expect("stage"), 1);

    // And the anchor, having handed the job on, is no longer the signer for
    // releases. Keeping both live would defeat the point of moving the key.
    let (envelope, elen) = release_manifest(&manifest(image, 3, 0), &author());
    assert_eq!(
        updater.stage(&envelope[..elen], image),
        Err(Refusal::Signature)
    );
}

#[test]
fn rotating_to_a_new_release_key_retires_the_old_one() {
    let mut updater = device_running_version_one();
    let first = DeviceIdentity::from_seed(&[7u8; 32]);
    let second = DeviceIdentity::from_seed(&[8u8; 32]);

    let (delegation, len) = delegate(1, 7, &author());
    updater.adopt(&delegation[..len], None).expect("adopt");
    let (delegation, len) = delegate(2, 8, &author());
    updater.adopt(&delegation[..len], None).expect("adopt");

    let image = b"version two";
    let (envelope, elen) = release_manifest(&manifest(image, 2, 1), &second);
    assert_eq!(updater.stage(&envelope[..elen], image).expect("stage"), 1);

    // A key that was compromised and rotated away from must not still work.
    let (envelope, elen) = release_manifest(&manifest(image, 3, 0), &first);
    assert_eq!(
        updater.stage(&envelope[..elen], image),
        Err(Refusal::Signature),
        "rotating exists to take authority away, not merely to add another holder"
    );
}

#[test]
fn a_replayed_older_delegation_cannot_reinstate_a_retired_key() {
    let mut updater = device_running_version_one();
    let (old, old_len) = delegate(1, 7, &author());
    let (new, new_len) = delegate(2, 8, &author());

    updater.adopt(&old[..old_len], None).expect("adopt");
    updater.adopt(&new[..new_len], None).expect("adopt");

    assert_eq!(
        updater.adopt(&old[..old_len], None),
        Err(Refusal::Rollback),
        "a retired key must not come back by replaying the statement that authorised it"
    );
    assert_eq!(updater.delegation().expect("delegation").epoch, 2);
}

#[test]
fn only_the_anchor_may_delegate() {
    let mut updater = device_running_version_one();
    let impostor = DeviceIdentity::from_seed(&[9u8; 32]);
    let (delegation, len) = delegate(1, 7, &impostor);

    assert_eq!(
        updater.adopt(&delegation[..len], None),
        Err(Refusal::Signature),
        "a release key must not be able to appoint its own successor"
    );
    assert!(updater.delegation().is_none());
}

#[test]
fn a_delegated_key_cannot_delegate_onwards() {
    // Depth is deliberately one: the anchor appoints a release key, and that is
    // where the chain stops. A release key that could delegate would be as
    // dangerous to lose as the anchor itself.
    let mut updater = device_running_version_one();
    let release = DeviceIdentity::from_seed(&[7u8; 32]);
    let (delegation, len) = delegate(1, 7, &author());
    updater.adopt(&delegation[..len], None).expect("adopt");

    let (onward, onward_len) = delegate(2, 8, &release);
    assert_eq!(
        updater.adopt(&onward[..onward_len], None),
        Err(Refusal::Signature)
    );
}

#[test]
fn a_device_survives_a_restart_with_its_delegation() {
    let release = DeviceIdentity::from_seed(&[7u8; 32]);
    let (delegation, len) = delegate(1, 7, &author());

    // The caller kept the envelope and hands it back on the way up.
    let device = Device {
        vendor_id: VENDOR,
        class_id: CLASS,
        anchor: author().public(),
    };
    let mut updater = Updater::new(device, MemoryStore::new(2, 4096))
        .with_delegation(&delegation[..len], None)
        .expect("restore");
    updater.provision(0, 1).expect("provision");

    let image = b"version two";
    let (envelope, elen) = release_manifest(&manifest(image, 2, 1), &release);
    assert_eq!(updater.stage(&envelope[..elen], image).expect("stage"), 1);
}

#[test]
fn an_expired_delegation_is_refused() {
    let mut updater = device_running_version_one();
    let delegation = Delegation {
        epoch: 1,
        release_key: DeviceIdentity::from_seed(&[7u8; 32]).public().to_bytes(),
        expires: 1_000,
    };
    let mut buf = [0u8; DELEGATION_MAX];
    let len = delegation.sign(&author(), &mut buf).expect("sign");

    assert_eq!(
        updater.adopt(&buf[..len], Some(2_000)),
        Err(Refusal::Expired)
    );
    assert_eq!(updater.adopt(&buf[..len], None), Err(Refusal::NoClock));
    assert!(updater.adopt(&buf[..len], Some(500)).is_ok());
}
