//! Over-the-air activation: the join exchange that turns root keys into a session.

use crate::crypto::Cipher;
use crate::error::LorawanError;
use crate::frame::{PhyPayload, MTYPE_JOIN_ACCEPT, MTYPE_JOIN_REQUEST, MTYPE_MASK};
use crate::session::Session;

// A join-request is a fixed 23 bytes: MHDR, AppEUI, DevEUI, DevNonce, and MIC.
pub(crate) const JOIN_REQUEST_LEN: usize = 1 + 8 + 8 + 2 + 4;

/// An end device's root credentials for over-the-air activation.
///
/// Where a [`Session`] is the state of an already-activated device, a `Device` holds what
/// it takes to activate: the device and application identifiers and the application root
/// key. It builds the [`join_request`](Device::join_request) a device broadcasts and turns
/// the network's reply into a ready [`Session`] with [`accept_join`](Device::accept_join),
/// deriving the session keys the spec prescribes.
///
/// The 8-byte identifiers are given most-significant byte first, as they are written; the
/// join-request transmits them little-endian, as the spec requires.
pub struct Device {
    dev_eui: [u8; 8],
    app_eui: [u8; 8],
    app_key: [u8; 16],
}

impl Device {
    /// Creates a device from its identifiers and application root key.
    ///
    /// # Arguments
    ///
    /// * `dev_eui` - the device identifier, most-significant byte first.
    /// * `app_eui` - the application (join) identifier, most-significant byte first.
    /// * `app_key` - the application root key.
    ///
    /// # Returns
    ///
    /// The device.
    pub fn new(dev_eui: [u8; 8], app_eui: [u8; 8], app_key: [u8; 16]) -> Self {
        Device {
            dev_eui,
            app_eui,
            app_key,
        }
    }

    /// Builds a join-request to broadcast.
    ///
    /// # Arguments
    ///
    /// * `dev_nonce` - a nonce the device must not reuse; keep it for the matching
    ///   [`accept_join`](Device::accept_join), which needs it to derive the keys.
    ///
    /// # Returns
    ///
    /// The join-request frame.
    pub fn join_request(&self, dev_nonce: u16) -> PhyPayload {
        let mut buf = [0u8; JOIN_REQUEST_LEN];
        buf[0] = MTYPE_JOIN_REQUEST;
        copy_reversed(&mut buf[1..9], &self.app_eui);
        copy_reversed(&mut buf[9..17], &self.dev_eui);
        buf[17..19].copy_from_slice(&dev_nonce.to_le_bytes());
        let tag = Cipher::new(&self.app_key).cmac(&buf[..19]);
        buf[19..23].copy_from_slice(&tag[..4]);
        PhyPayload::new(&buf).expect("a join-request always fits a frame")
    }

    /// Accepts a join-accept, deriving the activated session.
    ///
    /// Decrypts the reply, verifies its MIC against the application root key, and derives
    /// the network and application session keys from the nonces it carries.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the raw join-accept as it came off the radio.
    /// * `dev_nonce` - the same nonce passed to the [`join_request`](Device::join_request)
    ///   this reply answers.
    ///
    /// # Returns
    ///
    /// The activation, including the ready-to-use [`Session`].
    ///
    /// # Errors
    ///
    /// Returns [`LorawanError::FrameTooShort`] or [`LorawanError::MalformedFrame`] if the
    /// reply is not a valid join-accept shape, [`LorawanError::UnsupportedMType`] if it is
    /// not a join-accept, or [`LorawanError::MicMismatch`] if its MIC does not verify.
    pub fn accept_join(&self, bytes: &[u8], dev_nonce: u16) -> Result<JoinAccept, LorawanError> {
        if bytes.is_empty() {
            return Err(LorawanError::FrameTooShort);
        }
        if bytes[0] & MTYPE_MASK != MTYPE_JOIN_ACCEPT {
            return Err(LorawanError::UnsupportedMType(bytes[0] & MTYPE_MASK));
        }
        let encrypted = &bytes[1..];
        // The encrypted part is one block, or two when a channel list is attached.
        if encrypted.len() != 16 && encrypted.len() != 32 {
            return Err(LorawanError::MalformedFrame);
        }

        let cipher = Cipher::new(&self.app_key);
        // The network "encrypts" with AES decryption, so the device decrypts by encrypting.
        let mut clear = [0u8; 32];
        for (i, chunk) in encrypted.chunks(16).enumerate() {
            let block: [u8; 16] = chunk.try_into().map_err(|_| LorawanError::MalformedFrame)?;
            clear[i * 16..i * 16 + 16].copy_from_slice(&cipher.encrypt_block(&block));
        }
        let clear = &clear[..encrypted.len()];

        // The MIC covers the MHDR and the decrypted body up to the MIC itself.
        let mic_at = clear.len() - 4;
        let mut signed = [0u8; 1 + 28];
        signed[0] = bytes[0];
        signed[1..1 + mic_at].copy_from_slice(&clear[..mic_at]);
        let tag = cipher.cmac(&signed[..1 + mic_at]);
        if clear[mic_at..] != tag[..4] {
            return Err(LorawanError::MicMismatch);
        }

        let app_nonce = &clear[0..3];
        let net_id_bytes = &clear[3..6];
        let dev_addr = u32::from_le_bytes([clear[6], clear[7], clear[8], clear[9]]);
        let dl_settings = clear[10];
        let rx_delay = clear[11];
        let net_id = u32::from_le_bytes([net_id_bytes[0], net_id_bytes[1], net_id_bytes[2], 0]);

        let nwk_skey = derive_key(&cipher, 0x01, app_nonce, net_id_bytes, dev_nonce);
        let app_skey = derive_key(&cipher, 0x02, app_nonce, net_id_bytes, dev_nonce);

        Ok(JoinAccept {
            session: Session::new(dev_addr, nwk_skey, app_skey),
            net_id,
            dev_addr,
            dl_settings,
            rx_delay,
        })
    }
}

/// A successful activation: the session to use, plus the network parameters the accept
/// carried.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JoinAccept {
    session: Session,
    net_id: u32,
    dev_addr: u32,
    dl_settings: u8,
    rx_delay: u8,
}

impl JoinAccept {
    /// Returns the activated session, ready to secure data frames.
    ///
    /// # Returns
    ///
    /// The [`Session`].
    pub fn session(&self) -> Session {
        self.session
    }

    /// Returns the device address the network assigned.
    ///
    /// # Returns
    ///
    /// The device address.
    pub fn dev_addr(&self) -> u32 {
        self.dev_addr
    }

    /// Returns the network identifier, a 24-bit value.
    ///
    /// # Returns
    ///
    /// The NetID.
    pub fn net_id(&self) -> u32 {
        self.net_id
    }

    /// Returns the downlink settings byte, which selects the downlink data rates.
    ///
    /// # Returns
    ///
    /// The DLSettings byte.
    pub fn dl_settings(&self) -> u8 {
        self.dl_settings
    }

    /// Returns the delay, in seconds, before the first receive window.
    ///
    /// # Returns
    ///
    /// The RxDelay value.
    pub fn rx_delay(&self) -> u8 {
        self.rx_delay
    }
}

// Copies `src` into `dst` reversed, turning a most-significant-byte-first identifier into
// the little-endian order the air interface uses.
pub(crate) fn copy_reversed(dst: &mut [u8], src: &[u8]) {
    for (d, s) in dst.iter_mut().zip(src.iter().rev()) {
        *d = *s;
    }
}

// Derives a session key by encrypting the spec's key-derivation block with the root key.
pub(crate) fn derive_key(
    cipher: &Cipher,
    kind: u8,
    app_nonce: &[u8],
    net_id: &[u8],
    dev_nonce: u16,
) -> [u8; 16] {
    let mut block = [0u8; 16];
    block[0] = kind;
    block[1..4].copy_from_slice(app_nonce);
    block[4..7].copy_from_slice(net_id);
    block[7..9].copy_from_slice(&dev_nonce.to_le_bytes());
    cipher.encrypt_block(&block)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Uplink;

    const APP_KEY: [u8; 16] = [0xAB; 16];
    const DEV_EUI: [u8; 8] = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];
    const APP_EUI: [u8; 8] = [0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
    const DEV_NONCE: u16 = 0x1234;

    // Builds a join-accept the way a network server would, so a device can accept it.
    fn make_join_accept(
        app_key: &[u8; 16],
        app_nonce: [u8; 3],
        net_id: [u8; 3],
        dev_addr: u32,
        dl_settings: u8,
        rx_delay: u8,
    ) -> [u8; 17] {
        let cipher = Cipher::new(app_key);
        let mut clear = [0u8; 16];
        clear[0..3].copy_from_slice(&app_nonce);
        clear[3..6].copy_from_slice(&net_id);
        clear[6..10].copy_from_slice(&dev_addr.to_le_bytes());
        clear[10] = dl_settings;
        clear[11] = rx_delay;
        let mut signed = [0u8; 13];
        signed[0] = MTYPE_JOIN_ACCEPT;
        signed[1..13].copy_from_slice(&clear[..12]);
        let tag = cipher.cmac(&signed);
        clear[12..16].copy_from_slice(&tag[..4]);

        let mut frame = [0u8; 17];
        frame[0] = MTYPE_JOIN_ACCEPT;
        frame[1..17].copy_from_slice(&cipher.decrypt_block(&clear));
        frame
    }

    #[test]
    fn a_join_request_is_well_formed() {
        let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
        let request = device.join_request(DEV_NONCE);
        let bytes = request.as_bytes();
        assert_eq!(bytes.len(), JOIN_REQUEST_LEN);
        assert_eq!(bytes[0], MTYPE_JOIN_REQUEST);
        // EUIs are transmitted little-endian, so reversed from how they are written.
        assert_eq!(
            &bytes[1..9],
            &[0xFF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA, 0x99, 0x88]
        );
        assert_eq!(&bytes[17..19], &DEV_NONCE.to_le_bytes());
    }

    #[test]
    fn a_join_activates_a_session_that_secures_data() {
        let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
        let frame = make_join_accept(
            &APP_KEY,
            [0x01, 0x02, 0x03],
            [0x04, 0x05, 0x06],
            0x2601_1BDA,
            0x00,
            0x01,
        );

        let accepted = device.accept_join(&frame, DEV_NONCE).unwrap();
        assert_eq!(accepted.dev_addr(), 0x2601_1BDA);
        assert_eq!(accepted.net_id(), 0x0006_0504);
        assert_eq!(accepted.rx_delay(), 0x01);

        // The derived session secures a data-frame round-trip.
        let session = accepted.session();
        let uplink = session
            .encode_uplink(&Uplink::new(1, 1, b"joined"))
            .unwrap();
        let rx = session.decode(uplink.as_bytes(), 1).unwrap();
        assert_eq!(rx.payload(), b"joined");
    }

    #[test]
    fn a_tampered_join_accept_fails_the_mic() {
        let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
        let mut frame = make_join_accept(
            &APP_KEY,
            [0x01, 0x02, 0x03],
            [0x04, 0x05, 0x06],
            0x2601_1BDA,
            0x00,
            0x01,
        );
        frame[5] ^= 0xff;
        assert_eq!(
            device.accept_join(&frame, DEV_NONCE),
            Err(LorawanError::MicMismatch)
        );
    }

    #[test]
    fn the_wrong_root_key_rejects_the_join() {
        let device = Device::new(DEV_EUI, APP_EUI, [0x00; 16]);
        let frame = make_join_accept(
            &APP_KEY,
            [0x01, 0x02, 0x03],
            [0x04, 0x05, 0x06],
            0x2601_1BDA,
            0x00,
            0x01,
        );
        assert_eq!(
            device.accept_join(&frame, DEV_NONCE),
            Err(LorawanError::MicMismatch)
        );
    }

    #[test]
    fn a_join_accept_of_the_wrong_length_is_malformed() {
        let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
        assert_eq!(
            device.accept_join(&[MTYPE_JOIN_ACCEPT; 20], DEV_NONCE),
            Err(LorawanError::MalformedFrame)
        );
    }

    #[test]
    fn a_non_join_frame_is_rejected() {
        let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
        let mut frame = [0u8; 17];
        frame[0] = MTYPE_JOIN_REQUEST; // 0x00, not a join-accept
        assert_eq!(
            device.accept_join(&frame, DEV_NONCE),
            Err(LorawanError::UnsupportedMType(0x00))
        );
    }

    // Builds a 33-byte join-accept carrying a 16-byte channel list, the two-block form.
    fn make_join_accept_with_cflist(app_key: &[u8; 16], cflist: [u8; 16]) -> [u8; 33] {
        let cipher = Cipher::new(app_key);
        let mut clear = [0u8; 32];
        clear[0..3].copy_from_slice(&[0x01, 0x02, 0x03]); // AppNonce
        clear[3..6].copy_from_slice(&[0x04, 0x05, 0x06]); // NetID
        clear[6..10].copy_from_slice(&0x2601_1BDAu32.to_le_bytes()); // DevAddr
        clear[10] = 0x00; // DLSettings
        clear[11] = 0x01; // RxDelay
        clear[12..28].copy_from_slice(&cflist);
        let mut signed = [0u8; 29];
        signed[0] = MTYPE_JOIN_ACCEPT;
        signed[1..29].copy_from_slice(&clear[..28]);
        let tag = cipher.cmac(&signed);
        clear[28..32].copy_from_slice(&tag[..4]);

        let mut frame = [0u8; 33];
        frame[0] = MTYPE_JOIN_ACCEPT;
        let first: [u8; 16] = clear[0..16].try_into().unwrap();
        let second: [u8; 16] = clear[16..32].try_into().unwrap();
        frame[1..17].copy_from_slice(&cipher.decrypt_block(&first));
        frame[17..33].copy_from_slice(&cipher.decrypt_block(&second));
        frame
    }

    #[test]
    fn a_join_accept_with_a_channel_list_activates() {
        let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
        let frame = make_join_accept_with_cflist(&APP_KEY, [0x11; 16]);
        let accepted = device.accept_join(&frame, DEV_NONCE).unwrap();
        assert_eq!(accepted.dev_addr(), 0x2601_1BDA);
        // The derived session still secures a data frame.
        let session = accepted.session();
        let uplink = session.encode_uplink(&Uplink::new(1, 1, b"cf")).unwrap();
        assert_eq!(
            session.decode(uplink.as_bytes(), 1).unwrap().payload(),
            b"cf"
        );
    }
}
