//! The audit log guide example; see docs/guides/audit.md.

/// A controller keeping a signed record of what it did, and the two ways a tampered log
/// gives itself away: a record edited, and a record removed.
#[test]
fn a_signed_chain_and_the_two_ways_it_breaks() {
    // ANCHOR: example
    use pamoja_audit::{verify_chain, AuditLog, Entry};
    use pamoja_security::DeviceIdentity;

    // The controller signs its own log with a provisioned seed and an auditor holds only
    // the public half, so a log can be checked anywhere without the device present.
    let keeper = DeviceIdentity::from_seed(&[7u8; 32]);
    let auditor = keeper.public();

    let mut log = AuditLog::new(keeper);
    let lit = log.append(b"burner=on");
    let stopped = log.append(b"burner=off");
    println!("recorded  {} then {}", lit.index(), stopped.index());

    // Each record hashes its own index, the digest of the record before it, and what it
    // carries, so the chain fixes the order as well as the contents.
    println!("chained   {}", stopped.previous() == lit.digest());
    match verify_chain(&auditor, &[lit.clone(), stopped.clone()]) {
        Ok(()) => println!("verified  the whole log is authentic and in order"),
        Err(error) => println!("rejected  {error}"),
    }

    // Editing a stored record changes the digest its signature covers.
    let mut edited = stopped.to_bytes();
    *edited.last_mut().expect("a record with a payload") ^= 0xFF;
    let tampered = Entry::from_bytes(&edited).expect("a well-formed record");
    match verify_chain(&auditor, &[lit, tampered]) {
        Ok(()) => println!("an edited record verified, which should never happen"),
        Err(error) => println!("edited    caught: {error}"),
    }

    // Dropping the first record leaves the survivor chained to a link that is no longer
    // there, so a shortened log is caught as readily as an edited one.
    match verify_chain(&auditor, &[stopped]) {
        Ok(()) => println!("a shortened log verified, which should never happen"),
        Err(error) => println!("shortened caught: {error}"),
    }
    // ANCHOR_END: example
}
