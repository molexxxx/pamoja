//! The import lines the install page shows. They are spliced from here, so a symbol the
//! page names is one the compiler resolved.

/// The two ways into the framework reach the same types: `pamoja::modbus::Adu` is
/// `pamoja_modbus::Adu`, so code moves between them with no other change.
#[test]
fn the_umbrella_crate_and_the_capability_crate_are_the_same_types() {
    // ANCHOR: rust
    use pamoja::modbus::Adu; // the same type as pamoja_modbus::Adu
    use pamoja_codec::CborCodec;
    // ANCHOR_END: rust

    let frame = Adu::from_pdu(0x11, &[0x03, 0x00, 0x6b, 0x00, 0x03]).expect("a valid request");
    let same: pamoja_modbus::Adu = frame;
    assert_eq!(same.address(), 0x11);

    let _codec = CborCodec;
}
