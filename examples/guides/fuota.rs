//! The firmware update over the air guide example; see docs/guides/fuota.md.
//!
//! Run: `cargo run -p pamoja-examples --example fuota`

use std::error::Error;

/// A signed release broadcast to a group of devices over a link that drops a tenth of what
/// it carries: the group is set up, the image goes out in fragments, a device that missed
/// some solves for them, checks what it built, and stages it for the next boot.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_lorawan::packages::fragment::{
        data_block_int_key, BlockMicKey, Defragmenter, Fragmenter,
    };
    use pamoja_lorawan::packages::multicast::{
        mc_app_s_key, mc_ke_key, mc_key, mc_root_key, wrap_mc_key,
    };
    use pamoja_security::DeviceIdentity;
    use pamoja_update::block::{frame, split};
    use pamoja_update::{
        image_digest, Device, Manifest, MemoryStore, PayloadFormat, SlotState, SlotStore, Updater,
        ENVELOPE_MAX, STRUCTURE_VERSION,
    };

    // The publisher signs releases; the devices in the field are anchored to its public half
    // and will take firmware from nobody else, however it reaches them.
    let publisher = DeviceIdentity::from_seed(&[7u8; 32]);
    const VENDOR: [u8; 16] = [10; 16];
    const FLOW_METER: [u8; 16] = [11; 16];

    let image = b"firmware for a flow meter, version two, long enough to need fragmenting";
    let manifest = Manifest {
        structure_version: STRUCTURE_VERSION,
        sequence: 2,
        vendor_id: VENDOR,
        class_id: FLOW_METER,
        format: PayloadFormat::Raw,
        storage: 1,
        digest: image_digest(image),
        size: image.len() as u32,
        expires: 0,
    };
    let mut envelope = [0u8; ENVELOPE_MAX];
    let signed = manifest.sign(&publisher, &mut envelope)?;

    // One block carries the release: the signed manifest and the image behind a header that
    // says where each begins. The transport moves bytes and vouches for none of them.
    let mut block = [0u8; 512];
    let block_len = frame(&envelope[..signed], image, &mut block)?;
    println!(
        "release   {block_len} bytes: a signed manifest and {} of image",
        image.len()
    );

    // The group every device in the field belongs to. Its key travels wrapped under a key
    // each device derives from its own root key, so the broadcast key is never in the clear.
    let device_root_key = [0x2B; 16];
    let group_key = [0x77; 16];
    let group_addr = 0x2601_0042;
    let wrapped = wrap_mc_key(&mc_ke_key(&mc_root_key(&device_root_key)), &group_key);
    let unwrapped = mc_key(&mc_ke_key(&mc_root_key(&device_root_key)), &wrapped);
    println!(
        "group     {:#010X} keyed by a wrapped key the device unwraps: {}",
        group_addr,
        unwrapped == group_key
    );
    let _payload_key = mc_app_s_key(&group_key, group_addr);

    // The server cuts the block into fragments and sends more than there are, so a device
    // that misses some can still finish. Each coded fragment is the exclusive-or of a
    // pseudo-random half of the originals.
    let sender = Fragmenter::new(&block[..block_len], 32)?;
    println!(
        "session   {} fragments of {} bytes, {} of padding",
        sender.nb_frag(),
        sender.frag_size(),
        sender.padding()
    );

    // The device puts it back together in storage it set aside once: the block itself, and
    // room to solve for a handful of losses.
    let mut store = [0u8; 512];
    let mut matrix = [0u8; 256];
    let mut receiver = Defragmenter::new(
        sender.nb_frag(),
        sender.frag_size(),
        &mut store,
        &mut matrix,
    )?;

    // The link drops every fourth fragment. The session keeps going until the block is whole.
    let mut piece = [0u8; 32];
    let (mut sent, mut coded) = (0, 0);
    for n in 1..=sender.nb_frag() * 2 {
        if n % 4 == 0 {
            continue;
        }
        sender.fragment(n, &mut piece)?;
        sent += 1;
        coded += usize::from(n > sender.nb_frag());
        if receiver.fragment(n, &piece)? {
            break;
        }
    }
    println!("received  {sent} fragments, {coded} of them coded, and the block is whole");

    // What the device built is checked against the code the session setup carried, taken
    // over the block a piece at a time so the image is never held twice.
    let signer = BlockMicKey::new(&data_block_int_key(&device_root_key));
    let mut expected = signer.start(1, 0, *b"PJU1", block_len as u32);
    expected.update(&block[..block_len]);
    let mut built = signer.start(1, 0, *b"PJU1", block_len as u32);
    built.update(&receiver.block()[..block_len]);
    let intact = built.finish() == expected.finish();
    println!("checked   the block the device built is the one the server sent: {intact}");

    // Only now does the update itself get a say. The header says where the manifest ends;
    // everything after that is the manifest's decision, exactly as for a wired update.
    let (carried_envelope, carried_image) = split(&receiver.block()[..block_len])?;
    let mut updater = Updater::new(
        Device {
            vendor_id: VENDOR,
            class_id: FLOW_METER,
            anchor: publisher.public(),
        },
        MemoryStore::new(2, 4096),
    );
    updater.provision(0, 1).expect("the shipped image");
    let slot = updater.stage(carried_envelope, carried_image)?;
    println!("staged    into slot {slot}, leaving the running image alone");

    // A release broadcast to everyone is still refused by anyone it is not for. This one is
    // signed by another key.
    let impostor = DeviceIdentity::from_seed(&[90u8; 32]);
    let mut forged = [0u8; ENVELOPE_MAX];
    let forged_len = manifest.sign(&impostor, &mut forged)?;
    match updater.stage(&forged[..forged_len], image) {
        Ok(_) => println!("a forged release was accepted, which should never happen"),
        Err(error) => println!("forged    refused: {error}"),
    }
    // ANCHOR_END: example

    assert!(intact);
    assert_eq!(slot, 1);
    assert_eq!(carried_image, image);
    assert_eq!(
        updater.store().record(slot).expect("the slot").state,
        SlotState::Staged
    );
    Ok(())
}
