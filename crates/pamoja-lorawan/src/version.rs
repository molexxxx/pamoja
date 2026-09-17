//! Which revision of the LoRaWAN 1.0 link layer a device follows.

/// A revision of the LoRaWAN 1.0 link layer specification.
///
/// Both revisions frame and secure data the same way, and a network is told which one a
/// device follows when the device is registered. They differ in a handful of device rules,
/// and wherever this crate follows one or the other, the documentation says which.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Version {
    /// LoRaWAN 1.0.3, published in 2018.
    V1_0_3,
    /// LoRaWAN L2 1.0.4 (TS001-1.0.4), published in 2020, which settled several rules 1.0.3
    /// left open: a join nonce that counts rather than repeats at random, a frame counter
    /// with no gap limit, a staged adaptive data rate back-off, and an answer to every
    /// `LinkADRReq` rather than one per block.
    #[default]
    V1_0_4,
}
