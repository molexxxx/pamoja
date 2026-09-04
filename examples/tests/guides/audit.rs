//! The audit log guide example; see docs/guides/audit.md.

/// A controller recording what it did and an auditor checking the record afterwards,
/// anchored to the ed25519 key RFC 8032 publishes and to the SHA-256 digest the chain
/// construction fixes, so neither is round-tripped against itself.
#[test]
fn a_signed_chain_and_the_two_ways_it_breaks() {
    // ANCHOR: example
    use pamoja_audit::{verify_chain, AuditLog, Entry};
    use pamoja_security::DeviceIdentity;

    // The controller signs its own log with a provisioned seed. This one is RFC 8032 test
    // vector 1, so the key the records are checked against is a published constant.
    let keeper = DeviceIdentity::from_seed(&[
        0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c,
        0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae,
        0x7f, 0x60,
    ]);
    let public = keeper.public();
    assert_eq!(
        public.to_bytes(),
        [
            0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64,
            0x07, 0x3a, 0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68,
            0xf7, 0x07, 0x51, 0x1a,
        ]
    );

    let mut log = AuditLog::new(keeper);
    let lit = log.append(b"burner=on");
    let stopped = log.append(b"burner=off");

    // A record's digest is SHA-256 over its little-endian index, the digest of the record
    // before it, and its payload, so the first record hashes forty zero bytes and then
    // what it carries.
    assert_eq!(lit.index(), 0);
    assert_eq!(
        lit.digest(),
        [
            0xe5, 0x0c, 0x6a, 0x7a, 0x94, 0x4f, 0xab, 0x6d, 0xd1, 0x3f, 0xfd, 0xb7, 0x60, 0xca,
            0x19, 0x0e, 0x14, 0xea, 0x00, 0xc1, 0x68, 0xba, 0x7c, 0x94, 0x87, 0x45, 0xba, 0x0a,
            0xf1, 0x46, 0xc1, 0x59,
        ]
    );
    assert_eq!(stopped.previous(), lit.digest());
    assert!(verify_chain(&public, &[lit.clone(), stopped.clone()]).is_ok());

    // Editing a stored record changes the digest its signature covers.
    let mut edited = stopped.to_bytes();
    *edited.last_mut().expect("a record with a payload") ^= 0xFF;
    let tampered = Entry::from_bytes(&edited).expect("a well-formed record");
    assert!(verify_chain(&public, &[lit, tampered]).is_err());

    // Dropping the record before it leaves the survivor chained to a link that is no
    // longer there, so a shortened log is caught as readily as an edited one.
    assert!(verify_chain(&public, &[stopped]).is_err());
    // ANCHOR_END: example
}
