//! The audit log guide example; see docs/guides/audit.md.
//!
//! Run: `cargo run -p pamoja-examples --example audit`

use std::error::Error;

/// A controller keeping a signed record of what it did, the ways a tampered log gives itself
/// away, and the log picking up where it left off after a restart.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use pamoja_audit::{verify_chain, AuditLog, Entry};
    use pamoja_security::DeviceIdentity;

    // The controller signs its own log with a provisioned seed and an auditor holds only
    // the public half, so a log can be checked anywhere without the device present.
    let seed = [7u8; 32];
    let keeper = DeviceIdentity::from_seed(&seed);
    let auditor = keeper.public();

    let mut log = AuditLog::new(keeper);
    let lit = log.append(b"burner=on");
    let stopped = log.append(b"burner=off");
    println!(
        "recorded  burner=on as record {} and burner=off as record {}",
        lit.index(),
        stopped.index()
    );

    // Each record hashes its own index, the digest of the record before it, and what it
    // carries, so the chain fixes the order as well as the contents.
    let linked = if stopped.previous() == lit.digest() {
        "carries"
    } else {
        "does not carry"
    };
    println!(
        "chained   record {} {linked} the digest of record {}",
        stopped.index(),
        lit.index()
    );
    match verify_chain(&auditor, &[lit.clone(), stopped.clone()]) {
        Ok(()) => println!("verified  the whole log is authentic and in order"),
        Err(error) => println!("rejected  {error}"),
    }

    // Editing a stored record changes the digest its signature covers.
    let mut edited = stopped.to_bytes();
    *edited.last_mut().expect("a record with a payload") ^= 0xFF;
    let tampered = Entry::from_bytes(&edited).expect("a well-formed record");
    match verify_chain(&auditor, &[lit.clone(), tampered]) {
        Ok(()) => println!("an edited record verified, which should never happen"),
        Err(error) => println!("edited    caught: {error}"),
    }

    // Dropping the first record, or swapping the two, leaves a record where its index says
    // it cannot be, so a shortened or reordered log is caught as readily as an edited one.
    let shortened = [stopped.clone()];
    match verify_chain(&auditor, &shortened) {
        Ok(()) => println!("a shortened log verified, which should never happen"),
        Err(error) => println!("shortened caught: {error}"),
    }
    match verify_chain(&auditor, &[stopped.clone(), lit.clone()]) {
        Ok(()) => println!("a reordered log verified, which should never happen"),
        Err(error) => println!("reordered caught: {error}"),
    }

    // A log checked against another device's key fails on the first signature.
    let stranger = DeviceIdentity::from_seed(&[8u8; 32]).public();
    match verify_chain(&stranger, &[lit.clone(), stopped.clone()]) {
        Ok(()) => println!("another device's key verified the log, which should never happen"),
        Err(error) => println!("stranger  caught: {error}"),
    }

    // After a restart the controller loads its seed again and resumes from the last record
    // in storage, so the log carries on as one chain rather than starting a second.
    let mut resumed = AuditLog::resume(DeviceIdentity::from_seed(&seed), &stopped);
    let relit = resumed.append(b"burner=on");
    match verify_chain(&auditor, &[lit.clone(), stopped.clone(), relit.clone()]) {
        Ok(()) => println!(
            "resumed   burner=on again as record {}, and the whole log still verifies",
            relit.index()
        ),
        Err(error) => println!("rejected  {error}"),
    }

    // What a chain cannot show is a record cut from its end, because what is left is still
    // a valid chain. The auditor catches it against the last index the device reported.
    let reported = relit.index();
    let cut = [lit, stopped];
    if verify_chain(&auditor, &cut).is_ok() {
        let ends = cut[cut.len() - 1].index();
        let verdict = if ends < reported {
            "a record is missing"
        } else {
            "nothing is missing"
        };
        println!(
            "cut       the log verifies but ends at record {ends}, and the device reported \
             record {reported}: {verdict}"
        );
    }
    // ANCHOR_END: example

    Ok(())
}
