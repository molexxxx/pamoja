//! Why an update was refused.
//!
//! Every variant is a rule the update process enforces rather than a detail of how
//! it is implemented, because the reason a device rejected an image is the part an
//! operator has to act on. They map onto [`pamoja_core::Error`] so a caller that
//! already handles the SDK's errors handles these too.

use alloc::string::{String, ToString};

use pamoja_core::Error;

/// The reason an update was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The manifest is not valid CBOR, or ends before its fields do.
    Malformed,
    /// The manifest was written by a newer structure version than this device reads.
    UnsupportedVersion,
    /// The manifest signature is not from the key this device trusts.
    Signature,
    /// The image does not match the digest the manifest commits to.
    Digest,
    /// The image is not the size the manifest declares.
    Size,
    /// The manifest is for a different vendor or device class.
    WrongDevice,
    /// The sequence number is not greater than the one already installed, so this
    /// is a replay or a downgrade.
    Rollback,
    /// The image does not fit the slot it is bound for.
    SlotTooSmall,
    /// The named slot does not exist on this device.
    NoSuchSlot,
    /// The operation does not apply to the slot's current state.
    WrongState,
    /// There is no confirmed image to fall back to.
    NothingToRevert,
}

impl Refusal {
    /// Returns a human-readable description of the refusal.
    ///
    /// # Returns
    ///
    /// A short phrase naming the rule that refused the update.
    pub fn reason(self) -> &'static str {
        match self {
            Self::Malformed => "the manifest is malformed",
            Self::UnsupportedVersion => "the manifest structure version is not supported",
            Self::Signature => "the manifest signature is not from the trusted key",
            Self::Digest => "the image does not match the manifest digest",
            Self::Size => "the image is not the size the manifest declares",
            Self::WrongDevice => "the manifest is for a different vendor or device class",
            Self::Rollback => "the sequence number would roll the device back",
            Self::SlotTooSmall => "the image does not fit the target slot",
            Self::NoSuchSlot => "no such slot on this device",
            Self::WrongState => "the slot is not in a state that allows this",
            Self::NothingToRevert => "there is no confirmed image to revert to",
        }
    }
}

impl From<Refusal> for Error {
    fn from(value: Refusal) -> Self {
        let message: String = value.reason().to_string();
        match value {
            // A failed authenticity check is a security outcome, not a parse fault,
            // so it is reported as one.
            Refusal::Signature | Refusal::Digest | Refusal::WrongDevice | Refusal::Rollback => {
                Error::Auth(message)
            }
            Refusal::Malformed | Refusal::UnsupportedVersion => Error::Codec(message),
            _ => Error::Io(message),
        }
    }
}

/// The result of an update operation.
pub type Result<T> = core::result::Result<T, Refusal>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authenticity_failures_map_to_auth_errors() {
        for refusal in [
            Refusal::Signature,
            Refusal::Digest,
            Refusal::WrongDevice,
            Refusal::Rollback,
        ] {
            assert!(matches!(Error::from(refusal), Error::Auth(_)));
        }
    }

    #[test]
    fn parse_failures_map_to_codec_errors() {
        assert!(matches!(Error::from(Refusal::Malformed), Error::Codec(_)));
        assert!(matches!(
            Error::from(Refusal::UnsupportedVersion),
            Error::Codec(_)
        ));
    }

    #[test]
    fn every_refusal_names_its_rule() {
        assert!(!Refusal::Rollback.reason().is_empty());
        assert!(Refusal::Rollback.reason().contains("roll"));
    }
}
