//! How an end device decides to use a relay, TS011-1.0.1 sections 3.9 and 10.2.

/// How an end device manages its relay mode, TS011-1.0.1 table 40.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RelayActivation {
    /// Never send through a relay.
    Disabled,
    /// Always send through a relay, whether or not one answers.
    Enabled,
    /// Start sending through a relay after enough uplinks go unanswered, with
    /// [`smart_enable_uplinks`] of them.
    Dynamic,
    /// Leave it to the device, which is where every device starts and returns to when it
    /// joins again.
    DeviceControlled,
}

impl RelayActivation {
    /// Reads a coded mode, 0 through 3.
    ///
    /// # Arguments
    ///
    /// * `code` - the two bits an `EndDeviceConfReq` carries; only those are read.
    ///
    /// # Returns
    ///
    /// The mode.
    #[must_use]
    pub const fn from_code(code: u8) -> RelayActivation {
        match code & 0x03 {
            0 => RelayActivation::Disabled,
            1 => RelayActivation::Enabled,
            2 => RelayActivation::Dynamic,
            _ => RelayActivation::DeviceControlled,
        }
    }

    /// Returns the value an `EndDeviceConfReq` carries.
    ///
    /// # Returns
    ///
    /// The code, 0 through 3.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }
}

/// How many uplinks may go unanswered before a device in [`RelayActivation::Dynamic`] mode
/// starts using a relay, TS011-1.0.1 table 41.
///
/// # Arguments
///
/// * `level` - the coded level; only its low two bits are read.
///
/// # Returns
///
/// 8, 16, 32 or 64.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::relay::smart_enable_uplinks;
///
/// assert_eq!(smart_enable_uplinks(0), 8);
/// assert_eq!(smart_enable_uplinks(3), 64);
/// ```
#[must_use]
pub const fn smart_enable_uplinks(level: u8) -> u16 {
    8 << (level & 0x03)
}

/// What an end device knows about when its relay listens, TS011-1.0.1 section 3.9.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RelaySync {
    /// Nothing: the device assumes a relay scanning every second with a crystal good to 40
    /// parts per million and eight symbols to start receiving, sends a preamble spanning
    /// that whole second, and alternates the region's default channels.
    Initialized,
    /// How the relay scans, but not when: the preamble spans one scan period.
    Unsynchronized,
    /// When the relay next scans, so a preamble need only cover the drift since the last
    /// acknowledgment.
    Synchronized,
}
