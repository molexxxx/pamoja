//! Building and verifying a signed, hash-chained audit log.

use pamoja_core::{Error, Result};
use pamoja_security::{DeviceIdentity, PublicIdentity};

use crate::entry::{digest, Entry};

// The previous-digest a log starts from, before any entry exists.
const GENESIS: [u8; 32] = [0u8; 32];

/// Appends signed, hash-chained entries to a tamper-evident log.
///
/// Each [`append`](AuditLog::append) signs the new entry's digest and links it to
/// the previous entry, so the log can later be proven complete and unaltered. The
/// log holds the signing identity and the chain head; it does not store the entries
/// itself, so the caller persists each entry's [`to_bytes`](Entry::to_bytes) to
/// durable storage (a file or SD card in the field) and rebuilds the chain from
/// there.
///
/// # Examples
///
/// ```
/// use pamoja_audit::{verify_chain, AuditLog};
/// use pamoja_security::DeviceIdentity;
///
/// let device = DeviceIdentity::from_seed(&[9u8; 32]);
/// let public = device.public();
///
/// let mut log = AuditLog::new(device);
/// let entries = [log.append(b"4.6C"), log.append(b"4.9C")];
///
/// assert!(verify_chain(&public, &entries).is_ok());
/// ```
pub struct AuditLog {
    identity: DeviceIdentity,
    head: [u8; 32],
    next_index: u64,
}

impl AuditLog {
    /// Starts a fresh log signed by `identity`.
    ///
    /// # Arguments
    ///
    /// * `identity` - the device identity that signs each entry.
    ///
    /// # Returns
    ///
    /// An empty log positioned at the first entry.
    pub fn new(identity: DeviceIdentity) -> Self {
        Self {
            identity,
            head: GENESIS,
            next_index: 0,
        }
    }

    /// Resumes a log after its last entry, to keep appending across a restart.
    ///
    /// # Arguments
    ///
    /// * `identity` - the device identity that signs each entry.
    /// * `last` - the most recent entry already in durable storage.
    ///
    /// # Returns
    ///
    /// A log positioned to append after `last`.
    pub fn resume(identity: DeviceIdentity, last: &Entry) -> Self {
        Self {
            identity,
            head: last.digest(),
            next_index: last.index() + 1,
        }
    }

    /// Appends `payload`, returning the new signed, chained entry.
    ///
    /// The caller persists the returned entry's [`to_bytes`](Entry::to_bytes).
    ///
    /// # Arguments
    ///
    /// * `payload` - the bytes to record, such as an encoded reading.
    ///
    /// # Returns
    ///
    /// The new [`Entry`].
    pub fn append(&mut self, payload: &[u8]) -> Entry {
        let index = self.next_index;
        let prev = self.head;
        let digest = digest(index, &prev, payload);
        let signature = self.identity.sign(&digest);
        self.head = digest;
        self.next_index += 1;
        Entry::new(index, prev, signature, payload.to_vec())
    }
}

/// Verifies a log's entries in sequence against the signer's public identity.
///
/// A verifier checks each entry's index, its link to the previous entry, and its
/// signature, advancing only on success. Feed it entries oldest first; the first
/// failure is the point the log was tampered with.
pub struct Verifier {
    public: PublicIdentity,
    expected_index: u64,
    expected_prev: [u8; 32],
}

impl Verifier {
    /// Creates a verifier for a log signed by `public`, starting at the first entry.
    ///
    /// # Arguments
    ///
    /// * `public` - the public identity expected to have signed the log.
    ///
    /// # Returns
    ///
    /// A verifier positioned at the first entry.
    pub fn new(public: PublicIdentity) -> Self {
        Self {
            public,
            expected_index: 0,
            expected_prev: GENESIS,
        }
    }

    /// Verifies the next entry in sequence, advancing the verifier on success.
    ///
    /// # Arguments
    ///
    /// * `entry` - the next entry in the log.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the entry is in sequence, correctly chained, and authentically
    /// signed.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Auth`](pamoja_core::Error::Auth) if the entry is out of
    /// sequence, its chain link is wrong, or its signature does not verify.
    pub fn check(&mut self, entry: &Entry) -> Result<()> {
        if entry.index() != self.expected_index {
            return Err(Error::Auth("audit entry is out of sequence".into()));
        }
        if entry.previous() != self.expected_prev {
            return Err(Error::Auth("audit chain is broken".into()));
        }
        let digest = entry.digest();
        self.public.verify(&digest, entry.signature())?;
        self.expected_index += 1;
        self.expected_prev = digest;
        Ok(())
    }
}

/// Verifies a whole chain of entries from the start against `public`.
///
/// A chain proves nothing is missing before its last entry, not that its last entry is
/// the last one written: a log cut short at the end is still a valid chain. To catch
/// that, compare the last entry's index and digest with the ones the device last
/// reported, in a heartbeat or a telemetry snapshot.
///
/// # Arguments
///
/// * `public` - the public identity expected to have signed the log.
/// * `entries` - the log's entries, oldest first.
///
/// # Returns
///
/// `Ok(())` if every entry is in sequence, correctly chained, and authentic.
///
/// # Errors
///
/// Returns [`Error::Auth`](pamoja_core::Error::Auth) at the first entry that is out
/// of sequence, broken in the chain, or not authentically signed.
pub fn verify_chain(public: &PublicIdentity, entries: &[Entry]) -> Result<()> {
    let mut verifier = Verifier::new(*public);
    for entry in entries {
        verifier.check(entry)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device() -> DeviceIdentity {
        DeviceIdentity::from_seed(&[1u8; 32])
    }

    fn sample_log() -> (alloc::vec::Vec<Entry>, PublicIdentity) {
        let signer = device();
        let public = signer.public();
        let mut log = AuditLog::new(signer);
        let entries = alloc::vec![log.append(b"r0"), log.append(b"r1"), log.append(b"r2")];
        (entries, public)
    }

    #[test]
    fn a_genuine_chain_verifies() {
        let (entries, public) = sample_log();
        assert!(verify_chain(&public, &entries).is_ok());
    }

    #[test]
    fn a_tampered_payload_is_detected() {
        let (mut entries, public) = sample_log();
        let mut bytes = entries[1].to_bytes();
        *bytes.last_mut().expect("non-empty entry") ^= 0xff;
        entries[1] = Entry::from_bytes(&bytes).expect("parse");
        assert!(matches!(
            verify_chain(&public, &entries),
            Err(Error::Auth(_))
        ));
    }

    #[test]
    fn a_reordered_chain_is_detected() {
        let (mut entries, public) = sample_log();
        entries.swap(1, 2);
        assert!(matches!(
            verify_chain(&public, &entries),
            Err(Error::Auth(_))
        ));
    }

    #[test]
    fn a_dropped_entry_is_detected() {
        let (entries, public) = sample_log();
        let gap = alloc::vec![entries[0].clone(), entries[2].clone()];
        assert!(matches!(verify_chain(&public, &gap), Err(Error::Auth(_))));
    }

    #[test]
    fn another_signer_does_not_verify() {
        let (entries, _) = sample_log();
        let stranger = DeviceIdentity::from_seed(&[2u8; 32]).public();
        assert!(verify_chain(&stranger, &entries).is_err());
    }

    #[test]
    fn an_empty_chain_is_trivially_valid() {
        let public = device().public();
        assert!(verify_chain(&public, &[]).is_ok());
    }

    #[test]
    fn a_chain_that_does_not_start_at_the_beginning_is_rejected() {
        let (entries, public) = sample_log();
        // The slice's first entry has index 1, but verification expects to begin at 0.
        assert!(matches!(
            verify_chain(&public, &entries[1..]),
            Err(Error::Auth(_))
        ));
    }

    #[test]
    fn resume_continues_the_chain() {
        let public = device().public();
        let mut log = AuditLog::new(device());
        let e0 = log.append(b"r0");
        let e1 = log.append(b"r1");

        // A restart: rebuild the log from the last stored entry and keep appending.
        let mut resumed = AuditLog::resume(device(), &e1);
        let e2 = resumed.append(b"r2");

        assert!(verify_chain(&public, &[e0, e1, e2]).is_ok());
    }

    #[test]
    fn a_chain_cut_short_at_the_end_still_verifies() {
        let (entries, public) = sample_log();
        // What is left is a valid chain, so the tail is caught only against what the
        // device last reported.
        assert!(verify_chain(&public, &entries[..2]).is_ok());
        assert_ne!(entries[1].index(), entries[2].index());
    }
}
