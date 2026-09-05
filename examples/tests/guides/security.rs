//! The device identity guide example; see docs/guides/security.md.

/// A device signing its own reading and a gateway checking it, with both ways a check
/// fails: a reading edited in transit, and a signature offered under another device's key.
#[test]
fn a_signed_reading_and_the_gateway_that_checks_it() {
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
    // ANCHOR_END: example

    assert_eq!(device.sign(reading), signature);
    assert!(gateway.verify(reading, &signature).is_ok());
    assert!(gateway.verify(edited, &signature).is_err());
    assert!(impostor.public().verify(reading, &signature).is_err());
}
