//! Checking an image against the manifest that describes it.
//!
//! The image is hashed as it arrives rather than after it lands, so a device with
//! kilobytes of RAM can verify a payload of megabytes. Nothing is trusted until
//! [`ImageVerifier::finish`] agrees on both the length and the digest, which is
//! why the caller is handed a receipt it cannot forge rather than a boolean it
//! might forget to read.

use sha2::{Digest, Sha256};

use crate::error::{Refusal, Result};
use crate::manifest::{Manifest, DIGEST_LEN};

/// Proof that an image matched its manifest.
///
/// Only [`ImageVerifier::finish`] produces one, so a function that asks for a
/// [`Verified`] cannot be handed an unchecked image by mistake.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Verified {
    size: u32,
    digest: [u8; DIGEST_LEN],
}

impl Verified {
    /// Returns the verified image's length in bytes.
    ///
    /// # Returns
    ///
    /// The length the manifest declared and the image turned out to have.
    pub fn size(&self) -> u32 {
        self.size
    }

    /// Returns the verified image's digest.
    ///
    /// # Returns
    ///
    /// The SHA-256 the manifest committed to and the image hashed to.
    pub fn digest(&self) -> [u8; DIGEST_LEN] {
        self.digest
    }
}

/// Hashes a complete image, for a publisher filling in a manifest.
///
/// The manifest commits to a SHA-256 over the image, and this is that hash, so a
/// publisher does not have to add a hashing crate of its own just to name the image it
/// is releasing. A device receiving the image checks the same hash through
/// [`ImageVerifier`], which streams it rather than holding the whole image.
///
/// # Arguments
///
/// * `image` - the complete image the release carries.
///
/// # Returns
///
/// The SHA-256 of `image`, ready to put in a [`Manifest`](crate::Manifest).
///
/// # Examples
///
/// ```
/// use pamoja_update::image_digest;
///
/// let digest = image_digest(b"firmware");
/// assert_eq!(digest.len(), 32);
/// ```
pub fn image_digest(image: &[u8]) -> [u8; DIGEST_LEN] {
    Sha256::digest(image).into()
}

/// Hashes an image as it arrives and checks it against a manifest.
///
/// # Examples
///
/// ```
/// use pamoja_update::{ImageVerifier, Manifest, PayloadFormat, STRUCTURE_VERSION};
/// use sha2::{Digest, Sha256};
///
/// let image = b"firmware bytes";
/// let manifest = Manifest {
///     structure_version: STRUCTURE_VERSION,
///     sequence: 1,
///     vendor_id: [0; 16],
///     class_id: [0; 16],
///     format: PayloadFormat::Raw,
///     storage: 0,
///     digest: Sha256::digest(image).into(),
///     size: image.len() as u32,
///     expires: 0,
/// };
///
/// let mut verifier = ImageVerifier::new(&manifest);
/// for chunk in image.chunks(4) {
///     verifier.update(chunk).unwrap();
/// }
/// assert!(verifier.finish().is_ok());
/// ```
pub struct ImageVerifier {
    hasher: Sha256,
    expected_digest: [u8; DIGEST_LEN],
    expected_size: u32,
    seen: u64,
}

impl ImageVerifier {
    /// Starts verifying an image against `manifest`.
    ///
    /// # Arguments
    ///
    /// * `manifest` - the manifest whose digest and size the image must match.
    ///
    /// # Returns
    ///
    /// A verifier awaiting the image.
    pub fn new(manifest: &Manifest) -> Self {
        Self {
            hasher: Sha256::new(),
            expected_digest: manifest.digest,
            expected_size: manifest.size,
            seen: 0,
        }
    }

    /// Folds the next chunk of the image in.
    ///
    /// # Arguments
    ///
    /// * `chunk` - the next bytes of the image, in order.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the chunk is hashed.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Size`] as soon as more bytes arrive than the manifest
    /// declared, so an oversized payload is stopped while it is arriving rather
    /// than after it has filled the slot.
    pub fn update(&mut self, chunk: &[u8]) -> Result<()> {
        self.seen += chunk.len() as u64;
        if self.seen > u64::from(self.expected_size) {
            return Err(Refusal::Size);
        }
        self.hasher.update(chunk);
        Ok(())
    }

    /// Finishes the check and reports whether the image is the one described.
    ///
    /// # Returns
    ///
    /// A [`Verified`] receipt when the image is exactly the length and content
    /// the manifest committed to.
    ///
    /// # Errors
    ///
    /// Returns [`Refusal::Size`] if fewer bytes arrived than declared, or
    /// [`Refusal::Digest`] if the content does not hash to the manifest's digest.
    pub fn finish(self) -> Result<Verified> {
        if self.seen != u64::from(self.expected_size) {
            return Err(Refusal::Size);
        }

        let digest: [u8; DIGEST_LEN] = self.hasher.finalize().into();
        // A wrong digest and a right one take the same work to reject here, but the
        // comparison is over a hash the attacker cannot steer, so an early return
        // leaks nothing worth having.
        if digest != self.expected_digest {
            return Err(Refusal::Digest);
        }

        Ok(Verified {
            size: self.expected_size,
            digest,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{PayloadFormat, STRUCTURE_VERSION};

    /// A manifest describing `image` exactly.
    fn manifest_for(image: &[u8]) -> Manifest {
        Manifest {
            structure_version: STRUCTURE_VERSION,
            sequence: 1,
            vendor_id: [0; 16],
            class_id: [0; 16],
            format: PayloadFormat::Raw,
            storage: 0,
            digest: Sha256::digest(image).into(),
            size: image.len() as u32,
            expires: 0,
        }
    }

    /// Feeds an image through a verifier in small chunks.
    fn run(manifest: &Manifest, image: &[u8]) -> Result<Verified> {
        let mut verifier = ImageVerifier::new(manifest);
        for chunk in image.chunks(7) {
            verifier.update(chunk)?;
        }
        verifier.finish()
    }

    #[test]
    fn the_described_image_verifies() {
        let image = b"the firmware, arriving in pieces";
        let manifest = manifest_for(image);
        let verified = run(&manifest, image).expect("verify");
        assert_eq!(verified.size(), image.len() as u32);
        assert_eq!(verified.digest(), manifest.digest);
    }

    #[test]
    fn an_empty_image_verifies_when_that_is_what_was_described() {
        let manifest = manifest_for(b"");
        assert!(run(&manifest, b"").is_ok());
    }

    #[test]
    fn a_tampered_image_is_refused() {
        let image = b"the firmware, arriving in pieces";
        let manifest = manifest_for(image);
        let mut altered = *image;
        altered[3] ^= 0x01;
        assert_eq!(run(&manifest, &altered), Err(Refusal::Digest));
    }

    #[test]
    fn a_short_image_is_refused() {
        let image = b"the firmware, arriving in pieces";
        let manifest = manifest_for(image);
        assert_eq!(
            run(&manifest, &image[..image.len() - 1]),
            Err(Refusal::Size)
        );
    }

    #[test]
    fn an_oversized_image_is_stopped_while_it_arrives() {
        let image = b"short";
        let manifest = manifest_for(image);
        let mut verifier = ImageVerifier::new(&manifest);
        // The refusal lands on the chunk that crosses the declared length, not at
        // the end, so nothing keeps writing past the slot.
        assert!(verifier.update(image).is_ok());
        assert_eq!(verifier.update(b"more"), Err(Refusal::Size));
    }

    #[test]
    fn reordered_chunks_are_refused() {
        let image = b"order matters to a hash";
        let manifest = manifest_for(image);
        let mut verifier = ImageVerifier::new(&manifest);
        verifier.update(&image[10..]).expect("update");
        verifier.update(&image[..10]).expect("update");
        assert_eq!(verifier.finish(), Err(Refusal::Digest));
    }
}
