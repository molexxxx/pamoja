//! Generated Node bindings for device identity and signed telemetry.
//!
//! These mirror the `pamoja-security` Rust API one-to-one. Signing and verifying
//! are deterministic and need no runtime, so unlike the MQTT transport nothing
//! here is asynchronous.

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use pamoja_security::{DeviceIdentity as CoreIdentity, PublicIdentity, Signature};

/// The length in bytes of an identity seed and of a public key.
const KEY_LEN: usize = 32;

/// The length in bytes of a signature.
const SIGNATURE_LEN: usize = 64;

/// A device's private signing identity.
#[napi]
pub struct DeviceIdentity {
    pub(crate) inner: CoreIdentity,
}

#[napi]
impl DeviceIdentity {
    /// Creates an identity from a provisioned 32-byte secret seed.
    #[napi(constructor)]
    pub fn new(seed: Buffer) -> napi::Result<Self> {
        let seed = fixed::<KEY_LEN>(seed.as_ref(), "seed")?;
        Ok(Self {
            inner: CoreIdentity::from_seed(&seed),
        })
    }

    /// Returns the public key matching this identity, safe to share.
    #[napi]
    pub fn public_key(&self) -> Buffer {
        self.inner.public().to_bytes().to_vec().into()
    }

    /// Returns the short hex fingerprint of this identity, for logs and displays.
    #[napi]
    pub fn fingerprint(&self) -> String {
        self.inner.public().fingerprint()
    }

    /// Signs a payload, returning the 64-byte detached signature.
    #[napi]
    pub fn sign(&self, payload: Buffer) -> Buffer {
        self.inner.sign(payload.as_ref()).to_bytes().to_vec().into()
    }

    /// Signs a payload and returns the signature followed by the payload, as one message.
    #[napi]
    pub fn sign_message(&self, payload: Buffer) -> Buffer {
        self.inner.sign_message(payload.as_ref()).into()
    }
}

/// Verifies that a signature covers a payload and was made by a public key.
///
/// Returns `false` when the payload was altered or was signed by a different
/// device, and throws only when an argument is the wrong length.
#[napi]
pub fn verify(public_key: Buffer, payload: Buffer, signature: Buffer) -> napi::Result<bool> {
    let key = fixed::<KEY_LEN>(public_key.as_ref(), "publicKey")?;
    let signature = fixed::<SIGNATURE_LEN>(signature.as_ref(), "signature")?;
    let Ok(public) = PublicIdentity::from_bytes(&key) else {
        return Ok(false);
    };
    Ok(public
        .verify(payload.as_ref(), &Signature::from_bytes(&signature))
        .is_ok())
}

/// Verifies a signed message and returns the payload it carries.
///
/// Returns `null` when the message is too short to hold a signature, was altered,
/// or was signed by a different device, and throws only when the key is the wrong
/// length.
#[napi]
pub fn verify_message(public_key: Buffer, message: Buffer) -> napi::Result<Option<Buffer>> {
    let key = fixed::<KEY_LEN>(public_key.as_ref(), "publicKey")?;
    let Ok(public) = PublicIdentity::from_bytes(&key) else {
        return Ok(None);
    };
    Ok(public
        .verify_message(message.as_ref())
        .ok()
        .map(|payload| payload.to_vec().into()))
}

/// Returns the short hex fingerprint of a public key.
#[napi]
pub fn fingerprint(public_key: Buffer) -> napi::Result<String> {
    let key = fixed::<KEY_LEN>(public_key.as_ref(), "publicKey")?;
    PublicIdentity::from_bytes(&key)
        .map(|public| public.fingerprint())
        .map_err(|error| napi::Error::from_reason(error.to_string()))
}

/// Reads a fixed-width argument, naming it in the error when the length is wrong.
fn fixed<const N: usize>(bytes: &[u8], name: &str) -> napi::Result<[u8; N]> {
    <[u8; N]>::try_from(bytes)
        .map_err(|_| napi::Error::from_reason(format!("{name} must be exactly {N} bytes")))
}
