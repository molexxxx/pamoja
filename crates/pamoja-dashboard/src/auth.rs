//! Authenticating control over an open hotspot.
//!
//! Reading the dashboard is anonymous; moving an actuator or changing the fleet is not.
//! Because the serving hotspot is unencrypted, control cannot rely on a bearer token a
//! sniffer could capture and replay. Instead the device holds a pairing secret shown out
//! of band (its own screen, a QR code, or the dev server's console). A client that knows
//! the secret derives a per-session key from it and a server nonce, and authenticates
//! every command with a counter and an HMAC, so an on-network attacker can neither forge
//! a command nor replay a captured one. The secret itself never crosses the network.
//!
//! The keyed-hash primitives are reused from [`pamoja_session`] so this shares one
//! audited, vector-pinned crypto path. Sessions live in memory; a server restart simply
//! requires re-pairing.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use pamoja_session::{hkdf_sha256, hmac_sha256};

/// The HKDF context binding a derived key to this protocol and version.
const INFO: &[u8] = b"pamoja/dashboard/cmd v1";

/// How long a paired session stays valid before the client must pair again.
const SESSION_TTL: Duration = Duration::from_secs(30 * 60);

/// Why a control request was refused. The [`code`](AuthError::code) is a stable,
/// language-neutral string the page localizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthError {
    /// No session with that id exists; the client must pair first.
    UnknownSession,
    /// The session exists but has not completed pairing.
    NotPaired,
    /// The session has expired; the client must pair again.
    Expired,
    /// The counter is not greater than the last accepted one (a replay or reorder).
    Replayed,
    /// The supplied MAC does not match; the client does not hold the session key.
    BadMac,
}

impl AuthError {
    /// Returns the stable error code for this failure.
    ///
    /// # Returns
    ///
    /// A dotted, language-neutral code such as `"auth.bad_mac"`.
    pub fn code(self) -> &'static str {
        match self {
            AuthError::UnknownSession => "auth.unknown_session",
            AuthError::NotPaired => "auth.not_paired",
            AuthError::Expired => "auth.expired",
            AuthError::Replayed => "auth.replayed",
            AuthError::BadMac => "auth.bad_mac",
        }
    }
}

// One pairing session: the derived key, the highest command counter accepted, whether
// pairing has been confirmed, and when it lapses.
struct Entry {
    key: [u8; 32],
    last_counter: u64,
    paired: bool,
    expires: Instant,
}

/// A pairing challenge handed to a client in the clear.
pub struct Challenge {
    /// The opaque session identifier the client echoes on confirm and every command.
    pub session_id: String,
    /// The per-session salt the client mixes with the pairing secret to derive the key.
    pub nonce: String,
}

/// Gatekeeper for control actions: it issues pairing challenges and verifies commands.
pub struct Auth {
    secret: Vec<u8>,
    sessions: Mutex<HashMap<String, Entry>>,
}

impl Auth {
    /// Creates an authenticator for a pairing secret.
    ///
    /// # Arguments
    ///
    /// * `secret` - the canonical pairing secret string (the client normalizes a typed
    ///   code to the same value).
    ///
    /// # Returns
    ///
    /// An authenticator with no sessions yet.
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into().into_bytes(),
            sessions: Mutex::new(HashMap::new()),
        }
    }

    /// Generates a fresh high-entropy pairing secret as lowercase hex.
    ///
    /// # Returns
    ///
    /// A 128-bit secret rendered as 32 hex characters.
    pub fn generate_secret() -> String {
        to_hex(&random_bytes::<16>())
    }

    /// Starts a pairing exchange, returning a challenge and recording an unconfirmed
    /// session.
    ///
    /// # Returns
    ///
    /// The [`Challenge`] to send to the client.
    pub fn challenge(&self) -> Challenge {
        let session_id = to_hex(&random_bytes::<16>());
        let nonce = to_hex(&random_bytes::<16>());
        let mut key = [0u8; 32];
        hkdf_sha256(nonce.as_bytes(), &self.secret, INFO, &mut key);
        let mut sessions = self.sessions.lock().expect("sessions lock");
        prune(&mut sessions);
        sessions.insert(
            session_id.clone(),
            Entry {
                key,
                last_counter: 0,
                paired: false,
                expires: Instant::now() + SESSION_TTL,
            },
        );
        Challenge { session_id, nonce }
    }

    /// Confirms a pairing by checking the client proved it derived the session key.
    ///
    /// # Arguments
    ///
    /// * `session_id` - the challenge's session id.
    /// * `mac_hex` - `HMAC(key, "confirm\n" + session_id)` as lowercase hex.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the proof is valid and the session is now paired.
    ///
    /// # Errors
    ///
    /// [`AuthError::UnknownSession`], [`AuthError::Expired`], or [`AuthError::BadMac`].
    pub fn confirm(&self, session_id: &str, mac_hex: &str) -> Result<(), AuthError> {
        let mut sessions = self.sessions.lock().expect("sessions lock");
        let entry = sessions
            .get_mut(session_id)
            .ok_or(AuthError::UnknownSession)?;
        if Instant::now() > entry.expires {
            sessions.remove(session_id);
            return Err(AuthError::Expired);
        }
        let expected = hmac_hex(&entry.key, format!("confirm\n{session_id}").as_bytes());
        if !ct_eq(expected.as_bytes(), mac_hex.as_bytes()) {
            return Err(AuthError::BadMac);
        }
        entry.paired = true;
        Ok(())
    }

    /// Verifies an authenticated command and advances the session's replay counter.
    ///
    /// The MAC covers the counter and the exact command string, so the server checks the
    /// same bytes the client signed without re-serializing.
    ///
    /// # Arguments
    ///
    /// * `session_id` - the paired session's id.
    /// * `counter` - the strictly increasing per-session command counter.
    /// * `command` - the exact command payload string the client signed.
    /// * `mac_hex` - `HMAC(key, counter + "\n" + command)` as lowercase hex.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the command is authentic and fresh; the counter is then recorded.
    ///
    /// # Errors
    ///
    /// [`AuthError::UnknownSession`], [`AuthError::NotPaired`], [`AuthError::Expired`],
    /// [`AuthError::Replayed`], or [`AuthError::BadMac`].
    pub fn verify_command(
        &self,
        session_id: &str,
        counter: u64,
        command: &str,
        mac_hex: &str,
    ) -> Result<(), AuthError> {
        let mut sessions = self.sessions.lock().expect("sessions lock");
        let entry = sessions
            .get_mut(session_id)
            .ok_or(AuthError::UnknownSession)?;
        if Instant::now() > entry.expires {
            sessions.remove(session_id);
            return Err(AuthError::Expired);
        }
        if !entry.paired {
            return Err(AuthError::NotPaired);
        }
        if counter <= entry.last_counter {
            return Err(AuthError::Replayed);
        }
        let expected = hmac_hex(&entry.key, format!("{counter}\n{command}").as_bytes());
        if !ct_eq(expected.as_bytes(), mac_hex.as_bytes()) {
            return Err(AuthError::BadMac);
        }
        entry.last_counter = counter;
        Ok(())
    }
}

// Drops sessions whose time has passed, so the table cannot grow without bound.
fn prune(sessions: &mut HashMap<String, Entry>) {
    let now = Instant::now();
    sessions.retain(|_, entry| entry.expires > now);
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).expect("system RNG");
    bytes
}

fn hmac_hex(key: &[u8], message: &[u8]) -> String {
    to_hex(&hmac_sha256(key, message))
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from_digit((byte >> 4) as u32, 16).expect("nibble"));
        out.push(char::from_digit((byte & 0xf) as u32, 16).expect("nibble"));
    }
    out
}

// Length-checked constant-time comparison, so a MAC check does not leak via timing.
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mirrors what the browser does: derive the session key from the secret and nonce.
    fn client_key(secret: &str, nonce: &str) -> [u8; 32] {
        let mut key = [0u8; 32];
        hkdf_sha256(nonce.as_bytes(), secret.as_bytes(), INFO, &mut key);
        key
    }

    #[test]
    fn a_correct_secret_pairs_and_commands() {
        let secret = "s3cret";
        let auth = Auth::new(secret);
        let challenge = auth.challenge();
        let key = client_key(secret, &challenge.nonce);

        let confirm = to_hex(&hmac_sha256(
            &key,
            format!("confirm\n{}", challenge.session_id).as_bytes(),
        ));
        auth.confirm(&challenge.session_id, &confirm)
            .expect("paired");

        let cmd = r#"{"type":"actuate"}"#;
        let mac = to_hex(&hmac_sha256(&key, format!("1\n{cmd}").as_bytes()));
        auth.verify_command(&challenge.session_id, 1, cmd, &mac)
            .expect("first command");
    }

    #[test]
    fn a_wrong_secret_cannot_pair() {
        let auth = Auth::new("right");
        let challenge = auth.challenge();
        let key = client_key("wrong", &challenge.nonce);
        let confirm = to_hex(&hmac_sha256(
            &key,
            format!("confirm\n{}", challenge.session_id).as_bytes(),
        ));
        assert_eq!(
            auth.confirm(&challenge.session_id, &confirm),
            Err(AuthError::BadMac)
        );
    }

    #[test]
    fn a_replayed_counter_is_refused() {
        let secret = "s3cret";
        let auth = Auth::new(secret);
        let challenge = auth.challenge();
        let key = client_key(secret, &challenge.nonce);
        let confirm = to_hex(&hmac_sha256(
            &key,
            format!("confirm\n{}", challenge.session_id).as_bytes(),
        ));
        auth.confirm(&challenge.session_id, &confirm)
            .expect("paired");

        let cmd = r#"{"type":"actuate"}"#;
        let mac = to_hex(&hmac_sha256(&key, format!("5\n{cmd}").as_bytes()));
        auth.verify_command(&challenge.session_id, 5, cmd, &mac)
            .expect("counter 5");
        // Replaying counter 5, or any counter at or below it, is rejected.
        assert_eq!(
            auth.verify_command(&challenge.session_id, 5, cmd, &mac),
            Err(AuthError::Replayed)
        );
    }

    #[test]
    fn an_unpaired_session_cannot_command() {
        let auth = Auth::new("s3cret");
        let challenge = auth.challenge();
        let key = client_key("s3cret", &challenge.nonce);
        let cmd = "{}";
        let mac = to_hex(&hmac_sha256(&key, format!("1\n{cmd}").as_bytes()));
        assert_eq!(
            auth.verify_command(&challenge.session_id, 1, cmd, &mac),
            Err(AuthError::NotPaired)
        );
    }

    #[test]
    fn an_unknown_session_is_refused() {
        let auth = Auth::new("s3cret");
        assert_eq!(auth.confirm("nope", "00"), Err(AuthError::UnknownSession));
    }

    #[test]
    fn a_tampered_command_fails_the_mac() {
        let secret = "s3cret";
        let auth = Auth::new(secret);
        let challenge = auth.challenge();
        let key = client_key(secret, &challenge.nonce);
        let confirm = to_hex(&hmac_sha256(
            &key,
            format!("confirm\n{}", challenge.session_id).as_bytes(),
        ));
        auth.confirm(&challenge.session_id, &confirm)
            .expect("paired");

        let cmd = r#"{"type":"actuate","action":"open"}"#;
        let mac = to_hex(&hmac_sha256(&key, format!("1\n{cmd}").as_bytes()));
        let tampered = r#"{"type":"actuate","action":"close"}"#;
        assert_eq!(
            auth.verify_command(&challenge.session_id, 1, tampered, &mac),
            Err(AuthError::BadMac)
        );
    }
}
