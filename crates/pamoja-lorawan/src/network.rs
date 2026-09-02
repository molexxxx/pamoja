//! The network side of over-the-air activation.
//!
//! [`Device`](crate::Device) is the end device half of the join exchange: it broadcasts a
//! join-request and accepts the reply. This module is the other half, so a deployment can
//! run its own network rather than joining someone else's: verify the request a device
//! sent, then grant it an address and the session keys both sides derive independently.

use crate::crypto::Cipher;
use crate::error::LorawanError;
use crate::frame::{PhyPayload, MTYPE_JOIN_ACCEPT, MTYPE_JOIN_REQUEST, MTYPE_MASK};
use crate::join::{copy_reversed, derive_key, JOIN_REQUEST_LEN};
use crate::session::Session;

/// The number of bytes a channel list adds to a join-accept.
const CFLIST_LEN: usize = 16;

/// A join-request a device broadcast, with its integrity already verified.
///
/// [`parse`](JoinRequest::parse) checks the request against the application root key
/// before reporting anything, so the identifiers it hands back are the ones the key holder
/// actually sent rather than whatever arrived on the air.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::{Device, JoinRequest};
///
/// const APP_KEY: [u8; 16] = [0xAB; 16];
/// let device = Device::new([0x11; 8], [0x22; 8], APP_KEY);
/// let on_air = device.join_request(0x1234);
///
/// // The network recognises the device and the nonce it must not accept twice.
/// let request = JoinRequest::parse(on_air.as_bytes(), &APP_KEY)?;
/// assert_eq!(request.dev_eui(), [0x11; 8]);
/// assert_eq!(request.dev_nonce(), 0x1234);
/// # Ok::<(), pamoja_lorawan::LorawanError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JoinRequest {
    dev_eui: [u8; 8],
    app_eui: [u8; 8],
    dev_nonce: u16,
}

impl JoinRequest {
    /// Verifies a join-request and reads the identifiers out of it.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the raw join-request as it came off the radio.
    /// * `app_key` - the application root key the device shares with this network.
    ///
    /// # Returns
    ///
    /// The verified request.
    ///
    /// # Errors
    ///
    /// Returns [`LorawanError::FrameTooShort`] if the frame is empty,
    /// [`LorawanError::UnsupportedMType`] if it is not a join-request,
    /// [`LorawanError::MalformedFrame`] if it is not the fixed 23 bytes a join-request is,
    /// or [`LorawanError::MicMismatch`] if its MIC does not verify, which means it was not
    /// sent by a holder of `app_key`.
    pub fn parse(bytes: &[u8], app_key: &[u8; 16]) -> Result<JoinRequest, LorawanError> {
        if bytes.is_empty() {
            return Err(LorawanError::FrameTooShort);
        }
        if bytes[0] & MTYPE_MASK != MTYPE_JOIN_REQUEST {
            return Err(LorawanError::UnsupportedMType(bytes[0] & MTYPE_MASK));
        }
        if bytes.len() != JOIN_REQUEST_LEN {
            return Err(LorawanError::MalformedFrame);
        }

        let tag = Cipher::new(app_key).cmac(&bytes[..19]);
        if bytes[19..23] != tag[..4] {
            return Err(LorawanError::MicMismatch);
        }

        // The identifiers travel little-endian, so they reverse back to how they read.
        let mut app_eui = [0u8; 8];
        let mut dev_eui = [0u8; 8];
        copy_reversed(&mut app_eui, &bytes[1..9]);
        copy_reversed(&mut dev_eui, &bytes[9..17]);

        Ok(JoinRequest {
            dev_eui,
            app_eui,
            dev_nonce: u16::from_le_bytes([bytes[17], bytes[18]]),
        })
    }

    /// Returns the device identifier, most-significant byte first.
    ///
    /// # Returns
    ///
    /// The DevEUI, as it is written rather than as it was transmitted.
    pub fn dev_eui(&self) -> [u8; 8] {
        self.dev_eui
    }

    /// Returns the application identifier, most-significant byte first.
    ///
    /// # Returns
    ///
    /// The AppEUI, as it is written rather than as it was transmitted.
    pub fn app_eui(&self) -> [u8; 8] {
        self.app_eui
    }

    /// Returns the nonce this request carried.
    ///
    /// A network must remember the nonces a device has already used and refuse a repeat,
    /// since replaying one would re-derive the same session keys.
    ///
    /// # Returns
    ///
    /// The DevNonce.
    pub fn dev_nonce(&self) -> u16 {
        self.dev_nonce
    }
}

/// What a network grants a device that joined: an address, and the settings to answer on.
///
/// Build one, then [`accept`](JoinGrant::accept) it into the frame to transmit and take
/// the matching [`session`](JoinGrant::session) to secure traffic with. Both sides derive
/// the same keys from the same nonces, so the session here and the one the device computes
/// agree without either sending a key.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::{Device, JoinGrant, JoinRequest, Uplink};
///
/// const APP_KEY: [u8; 16] = [0xAB; 16];
/// let device = Device::new([0x11; 8], [0x22; 8], APP_KEY);
/// let request = JoinRequest::parse(device.join_request(0x1234).as_bytes(), &APP_KEY)?;
///
/// // The network assigns an address and replies.
/// let grant = JoinGrant::new(0x0003_0201, 0x0006_0504, 0x2601_1BDA);
/// let reply = grant.accept(&APP_KEY, request.dev_nonce());
///
/// // The device activates, and the two sessions agree.
/// let activated = device.accept_join(reply.as_bytes(), 0x1234)?;
/// assert_eq!(activated.dev_addr(), 0x2601_1BDA);
///
/// let uplink = activated.session().encode_uplink(&Uplink::new(1, 1, b"joined"))?;
/// let heard = grant.session(&APP_KEY, 0x1234).decode(uplink.as_bytes(), 1)?;
/// assert_eq!(heard.payload(), b"joined");
/// # Ok::<(), pamoja_lorawan::LorawanError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JoinGrant {
    app_nonce: u32,
    net_id: u32,
    dev_addr: u32,
    dl_settings: u8,
    rx_delay: u8,
    cflist: Option<[u8; CFLIST_LEN]>,
}

impl JoinGrant {
    /// Creates a grant with no channel list and the default downlink settings.
    ///
    /// # Arguments
    ///
    /// * `app_nonce` - a nonce this network must not reuse for the device, since the
    ///   session keys are derived from it; only the low 24 bits are carried.
    /// * `net_id` - the network identifier; only the low 24 bits are carried.
    /// * `dev_addr` - the address to assign the device.
    ///
    /// # Returns
    ///
    /// The grant.
    pub fn new(app_nonce: u32, net_id: u32, dev_addr: u32) -> Self {
        JoinGrant {
            app_nonce,
            net_id,
            dev_addr,
            dl_settings: 0,
            rx_delay: 0,
            cflist: None,
        }
    }

    /// Sets the downlink settings byte, which selects the downlink data rates.
    ///
    /// # Arguments
    ///
    /// * `dl_settings` - the DLSettings byte.
    ///
    /// # Returns
    ///
    /// The updated grant, for chaining.
    pub fn with_dl_settings(mut self, dl_settings: u8) -> Self {
        self.dl_settings = dl_settings;
        self
    }

    /// Sets the delay, in seconds, before the first receive window.
    ///
    /// # Arguments
    ///
    /// * `rx_delay` - the RxDelay value.
    ///
    /// # Returns
    ///
    /// The updated grant, for chaining.
    pub fn with_rx_delay(mut self, rx_delay: u8) -> Self {
        self.rx_delay = rx_delay;
        self
    }

    /// Attaches the optional channel list, which tells the device where else to transmit.
    ///
    /// # Arguments
    ///
    /// * `cflist` - the 16-byte CFList, whose meaning is regional.
    ///
    /// # Returns
    ///
    /// The updated grant, for chaining. The accept it builds is 33 bytes rather than 17.
    pub fn with_cflist(mut self, cflist: [u8; CFLIST_LEN]) -> Self {
        self.cflist = Some(cflist);
        self
    }

    /// Returns the address this grant assigns.
    ///
    /// # Returns
    ///
    /// The device address.
    pub fn dev_addr(&self) -> u32 {
        self.dev_addr
    }

    /// Returns the network identifier this grant carries.
    ///
    /// # Returns
    ///
    /// The NetID, in its low 24 bits.
    pub fn net_id(&self) -> u32 {
        self.net_id
    }

    /// Builds the signed join-accept to transmit.
    ///
    /// # Arguments
    ///
    /// * `app_key` - the application root key the device shares with this network.
    /// * `dev_nonce` - the nonce the matching [`JoinRequest`] carried.
    ///
    /// # Returns
    ///
    /// The join-accept frame, encrypted and with its MIC in place.
    pub fn accept(&self, app_key: &[u8; 16], dev_nonce: u16) -> PhyPayload {
        let _ = dev_nonce;
        let cipher = Cipher::new(app_key);
        let body = self.body_len();

        // The clear body, then the MIC over the MHDR and everything before it.
        let mut clear = [0u8; 32];
        clear[0..3].copy_from_slice(&self.app_nonce.to_le_bytes()[..3]);
        clear[3..6].copy_from_slice(&self.net_id.to_le_bytes()[..3]);
        clear[6..10].copy_from_slice(&self.dev_addr.to_le_bytes());
        clear[10] = self.dl_settings;
        clear[11] = self.rx_delay;
        if let Some(cflist) = self.cflist {
            clear[12..12 + CFLIST_LEN].copy_from_slice(&cflist);
        }

        let mic_at = body - 4;
        let mut signed = [0u8; 1 + 28];
        signed[0] = MTYPE_JOIN_ACCEPT;
        signed[1..1 + mic_at].copy_from_slice(&clear[..mic_at]);
        let tag = cipher.cmac(&signed[..1 + mic_at]);
        clear[mic_at..body].copy_from_slice(&tag[..4]);

        // The network encrypts with AES decryption, so the device decrypts by encrypting.
        let mut frame = [0u8; 1 + 32];
        frame[0] = MTYPE_JOIN_ACCEPT;
        for (index, chunk) in clear[..body].chunks(16).enumerate() {
            let block: [u8; 16] = chunk.try_into().expect("the body is whole blocks");
            frame[1 + index * 16..1 + index * 16 + 16]
                .copy_from_slice(&cipher.decrypt_block(&block));
        }
        PhyPayload::new(&frame[..1 + body]).expect("a join-accept always fits a frame")
    }

    /// Derives the session this grant activates, the same one the device computes.
    ///
    /// # Arguments
    ///
    /// * `app_key` - the application root key the device shares with this network.
    /// * `dev_nonce` - the nonce the matching [`JoinRequest`] carried.
    ///
    /// # Returns
    ///
    /// The [`Session`] to secure this device's traffic with.
    pub fn session(&self, app_key: &[u8; 16], dev_nonce: u16) -> Session {
        let cipher = Cipher::new(app_key);
        let app_nonce = self.app_nonce.to_le_bytes();
        let net_id = self.net_id.to_le_bytes();
        let nwk_skey = derive_key(&cipher, 0x01, &app_nonce[..3], &net_id[..3], dev_nonce);
        let app_skey = derive_key(&cipher, 0x02, &app_nonce[..3], &net_id[..3], dev_nonce);
        Session::new(self.dev_addr, nwk_skey, app_skey)
    }

    /// Returns the length of the encrypted body, which a channel list doubles.
    fn body_len(&self) -> usize {
        if self.cflist.is_some() {
            32
        } else {
            16
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Device, Uplink};

    const APP_KEY: [u8; 16] = [0xAB; 16];
    const DEV_EUI: [u8; 8] = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];
    const APP_EUI: [u8; 8] = [0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF];
    const DEV_NONCE: u16 = 0x1234;

    fn grant() -> JoinGrant {
        JoinGrant::new(0x0003_0201, 0x0006_0504, 0x2601_1BDA)
            .with_dl_settings(0x00)
            .with_rx_delay(0x01)
    }

    #[test]
    fn a_request_verifies_and_reads_back_the_identifiers() {
        let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
        let request =
            JoinRequest::parse(device.join_request(DEV_NONCE).as_bytes(), &APP_KEY).unwrap();
        assert_eq!(request.dev_eui(), DEV_EUI);
        assert_eq!(request.app_eui(), APP_EUI);
        assert_eq!(request.dev_nonce(), DEV_NONCE);
    }

    #[test]
    fn a_request_signed_with_another_key_is_refused() {
        let device = Device::new(DEV_EUI, APP_EUI, [0x00; 16]);
        assert_eq!(
            JoinRequest::parse(device.join_request(DEV_NONCE).as_bytes(), &APP_KEY),
            Err(LorawanError::MicMismatch)
        );
    }

    #[test]
    fn a_tampered_request_fails_its_mic() {
        let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
        let mut bytes = device.join_request(DEV_NONCE).as_bytes().to_vec();
        bytes[10] ^= 0xFF;
        assert_eq!(
            JoinRequest::parse(&bytes, &APP_KEY),
            Err(LorawanError::MicMismatch)
        );
    }

    #[test]
    fn a_data_frame_is_not_a_join_request() {
        assert_eq!(
            JoinRequest::parse(&[0x40; JOIN_REQUEST_LEN], &APP_KEY),
            Err(LorawanError::UnsupportedMType(0x40))
        );
    }

    #[test]
    fn a_truncated_request_is_malformed() {
        assert_eq!(
            JoinRequest::parse(&[MTYPE_JOIN_REQUEST; 20], &APP_KEY),
            Err(LorawanError::MalformedFrame)
        );
    }

    #[test]
    fn the_accept_this_network_builds_activates_the_device() {
        let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
        let grant = grant();
        let accepted = device
            .accept_join(grant.accept(&APP_KEY, DEV_NONCE).as_bytes(), DEV_NONCE)
            .expect("the device accepts what this network signed");

        assert_eq!(accepted.dev_addr(), grant.dev_addr());
        assert_eq!(accepted.net_id(), grant.net_id());
        assert_eq!(accepted.dl_settings(), 0x00);
        assert_eq!(accepted.rx_delay(), 0x01);
    }

    #[test]
    fn both_sides_derive_the_same_session() {
        let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
        let grant = grant();
        let accepted = device
            .accept_join(grant.accept(&APP_KEY, DEV_NONCE).as_bytes(), DEV_NONCE)
            .expect("the device activates");

        // Neither side sent a key, yet each can read what the other secures.
        let network = grant.session(&APP_KEY, DEV_NONCE);
        assert_eq!(accepted.session(), network);

        let uplink = accepted
            .session()
            .encode_uplink(&Uplink::new(1, 1, b"joined"))
            .unwrap();
        assert_eq!(
            network.decode(uplink.as_bytes(), 1).unwrap().payload(),
            b"joined"
        );
    }

    #[test]
    fn a_grant_with_a_channel_list_activates_too() {
        let device = Device::new(DEV_EUI, APP_EUI, APP_KEY);
        let grant = grant().with_cflist([0x11; CFLIST_LEN]);
        let reply = grant.accept(&APP_KEY, DEV_NONCE);
        assert_eq!(
            reply.as_bytes().len(),
            33,
            "a channel list doubles the body"
        );

        let accepted = device
            .accept_join(reply.as_bytes(), DEV_NONCE)
            .expect("the device accepts the longer form");
        assert_eq!(accepted.dev_addr(), grant.dev_addr());
        assert_eq!(accepted.session(), grant.session(&APP_KEY, DEV_NONCE));
    }

    #[test]
    fn a_different_nonce_derives_a_different_session() {
        let grant = grant();
        assert_ne!(
            grant.session(&APP_KEY, DEV_NONCE),
            grant.session(&APP_KEY, DEV_NONCE + 1),
            "replaying a nonce is what the network must refuse, so the keys must differ"
        );
    }
}

#[cfg(test)]
mod published_vector {
    use super::*;
    use crate::Device;

    // A real EU868 join-accept captured off the air, with the plaintext fields and the
    // session keys an independent implementation derived from it. Anchoring to a third
    // party's numbers is what stops this crate and its bindings from agreeing with each
    // other on an answer that is wrong.
    //
    // Published at https://github.com/anthonykirby/lora-packet/issues/10
    const FRAME: &str = "204dd85ae608b87fc4889970b7d2042c9e72959b0057aed6094b16003df12de145";
    const APP_KEY: &str = "b6b53f4a168a7a88bdf7ea135ce9cfca";
    const DEV_NONCE: u16 = 0xCC85;
    const APP_NONCE: u32 = 0x00E5_063A;
    const NET_ID: u32 = 0x0000_0013;
    const DEV_ADDR: u32 = 0x2601_2E43;
    const DL_SETTINGS: u8 = 0x03;
    const RX_DELAY: u8 = 0x01;
    const CFLIST: &str = "184f84e85684b85e84886684586e8400";
    const NWK_SKEY: &str = "2c96f7028184bb0be8aa49275290d4fc";
    const APP_SKEY: &str = "f3a5c8f0232a38c144029c165865802c";

    #[test]
    fn a_device_activates_from_a_captured_join_accept() {
        let accepted = Device::new([0; 8], [0; 8], key(APP_KEY))
            .accept_join(&hex(FRAME), DEV_NONCE)
            .expect("the captured accept verifies against its own key");

        assert_eq!(accepted.dev_addr(), DEV_ADDR);
        assert_eq!(accepted.net_id(), NET_ID);
        assert_eq!(accepted.dl_settings(), DL_SETTINGS);
        assert_eq!(accepted.rx_delay(), RX_DELAY);

        // The session keys are the real check: they fold in the AppNonce, NetID, and
        // DevNonce, so matching an independent derivation pins the whole construction.
        assert_eq!(
            accepted.session(),
            Session::new(DEV_ADDR, key(NWK_SKEY), key(APP_SKEY))
        );
    }

    #[test]
    fn this_network_rebuilds_that_join_accept_byte_for_byte() {
        let grant = JoinGrant::new(APP_NONCE, NET_ID, DEV_ADDR)
            .with_dl_settings(DL_SETTINGS)
            .with_rx_delay(RX_DELAY)
            .with_cflist(hex(CFLIST).try_into().expect("a 16-byte channel list"));

        assert_eq!(
            grant.accept(&key(APP_KEY), DEV_NONCE).as_bytes(),
            &hex(FRAME)[..],
            "the frame this network signs is the one that was captured"
        );
        assert_eq!(
            grant.session(&key(APP_KEY), DEV_NONCE),
            Session::new(DEV_ADDR, key(NWK_SKEY), key(APP_SKEY))
        );
    }

    fn hex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&text[index..index + 2], 16).expect("hex"))
            .collect()
    }

    fn key(text: &str) -> [u8; 16] {
        hex(text).try_into().expect("a 16-byte key")
    }
}
