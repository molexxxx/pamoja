//! The device identity guide example; see docs/guides/security.md.
//!
//! Run: `cargo run -p pamoja-examples --example security`

use std::error::Error;

/// A device signing its own reading and a gateway checking it, with both ways a check
/// fails: a reading edited in transit, and a signature offered under another device's key.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_security::{DeviceIdentity, PublicIdentity};

    // The seed is provisioned into the device once and never leaves it. A real one comes
    // from the factory or a secure element; any 32 bytes stand in here.
    let device = DeviceIdentity::from_seed(&[7u8; 32]);

    // Only the 32-byte public key travels to the gateway. Its fingerprint is the short
    // form an operator reads off a screen to tell one device from another.
    let gateway = PublicIdentity::from_bytes(&device.public().to_bytes()).expect("a valid key");
    println!("device     {}", gateway.fingerprint());

    // Signing is deterministic, so the same reading always produces the same 64 bytes and
    // there is no randomness to get wrong on a microcontroller.
    let reading = b"meter-4 1182.750 kWh";
    let signature = device.sign(reading);
    match gateway.verify(reading, &signature) {
        Ok(()) => println!("accepted   {}", String::from_utf8_lossy(reading)),
        Err(error) => println!("rejected   {error}"),
    }

    // A digit changed in transit no longer matches what was signed.
    let edited = b"meter-4 1082.750 kWh";
    match gateway.verify(edited, &signature) {
        Ok(()) => println!("accepted   an edited reading, which should never happen"),
        Err(_) => println!("rejected   {}", String::from_utf8_lossy(edited)),
    }

    // Nor does the same reading offered under another device's key.
    let impostor = DeviceIdentity::from_seed(&[90u8; 32]);
    match impostor.public().verify(reading, &signature) {
        Ok(()) => println!("accepted   an impostor, which should never happen"),
        Err(_) => println!("rejected   a signature offered under another device's key"),
    }

    // On a link the signature and the reading usually travel as one message, signature
    // first, and the gateway gets the reading back only once it has checked it.
    let message = device.sign_message(reading);
    let size = message.len();
    println!("message    {size} bytes on the wire, the signature and the reading together");
    match gateway.verify_message(&message) {
        Ok(carried) => {
            let carried = String::from_utf8_lossy(carried);
            println!("accepted   {carried}, read out of the message");
        }
        Err(error) => println!("rejected   {error}"),
    }

    // A message that lost its last byte on the way is refused whole.
    match gateway.verify_message(&message[..size - 1]) {
        Ok(_) => println!("accepted   a message cut short, which should never happen"),
        Err(_) => println!("rejected   a message that lost its last byte on the way"),
    }
    // ANCHOR_END: example

    assert_eq!(device.sign(reading), signature);
    assert!(gateway.verify(reading, &signature).is_ok());
    assert!(gateway.verify(edited, &signature).is_err());
    assert!(impostor.public().verify(reading, &signature).is_err());
    assert_eq!(size, 64 + reading.len());
    assert_eq!(gateway.verify_message(&message)?, reading);
    assert!(gateway.verify_message(&message[..size - 1]).is_err());

    Ok(())
}
