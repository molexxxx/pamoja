//! The token buckets that bound what a relay forwards, TS011-1.0.1 section 8.8.

use crate::mac::RELAY_BUCKET_MULTIPLIER;

/// How often every bucket earns its reload rate, in microseconds: an hour.
pub const RELOAD_PERIOD_US: u64 = 3_600_000_000;

/// The reload rate a trusted end device's bucket is coded with to mean no limit,
/// TS011-1.0.1 table 54.
pub const UNLIMITED_DEVICE_RATE: u8 = 63;

/// The reload rate a relay-wide bucket is coded with to mean no limit, TS011-1.0.1 table 64.
pub const UNLIMITED_RELAY_RATE: u8 = 127;

/// A token bucket: it earns a number of tokens an hour up to its size, and a relay spends
/// one on each message it lets through.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TokenBucket {
    reload_rate: Option<u8>,
    size: u16,
    tokens: u16,
}

impl TokenBucket {
    /// A bucket that earns `reload_rate` tokens an hour and holds `size`, starting full.
    ///
    /// # Arguments
    ///
    /// * `reload_rate` - tokens earned an hour.
    /// * `size` - the most tokens it holds.
    ///
    /// # Returns
    ///
    /// The bucket.
    #[must_use]
    pub const fn new(reload_rate: u8, size: u16) -> TokenBucket {
        TokenBucket {
            reload_rate: Some(reload_rate),
            size,
            tokens: size,
        }
    }

    /// A bucket as a relay command codes it, starting full.
    ///
    /// # Arguments
    ///
    /// * `reload_rate` - tokens earned an hour, or `unlimited` for no limit.
    /// * `bucket_size` - the coded size multiplier, TS011-1.0.1 table 55; only its low two
    ///   bits are read.
    /// * `unlimited` - the rate that means no limit: [`UNLIMITED_DEVICE_RATE`] or
    ///   [`UNLIMITED_RELAY_RATE`].
    ///
    /// # Returns
    ///
    /// The bucket.
    #[must_use]
    pub const fn coded(reload_rate: u8, bucket_size: u8, unlimited: u8) -> TokenBucket {
        if reload_rate >= unlimited {
            return TokenBucket::unlimited();
        }
        let multiplier = RELAY_BUCKET_MULTIPLIER[(bucket_size & 0x03) as usize];
        TokenBucket::new(reload_rate, reload_rate as u16 * multiplier)
    }

    /// A bucket that never runs out.
    ///
    /// # Returns
    ///
    /// The bucket.
    #[must_use]
    pub const fn unlimited() -> TokenBucket {
        TokenBucket {
            reload_rate: None,
            size: 0,
            tokens: 0,
        }
    }

    /// Returns how many tokens the bucket earns an hour.
    ///
    /// # Returns
    ///
    /// The rate, or `None` for no limit.
    #[must_use]
    pub const fn reload_rate(&self) -> Option<u8> {
        self.reload_rate
    }

    /// Returns the most tokens the bucket holds.
    ///
    /// # Returns
    ///
    /// The size, 0 for a bucket with no limit.
    #[must_use]
    pub const fn size(&self) -> u16 {
        self.size
    }

    /// Returns the tokens the bucket holds now.
    ///
    /// # Returns
    ///
    /// The count, 0 for a bucket with no limit.
    #[must_use]
    pub const fn tokens(&self) -> u16 {
        self.tokens
    }

    /// Reports whether a message may pass.
    ///
    /// # Returns
    ///
    /// `true` for a bucket with no limit or a token left.
    #[must_use]
    pub const fn has_token(&self) -> bool {
        self.reload_rate.is_none() || self.tokens > 0
    }

    /// Reports whether the bucket is empty and never refills, which a relay reports as
    /// forwarding disabled.
    ///
    /// # Returns
    ///
    /// `true` for an empty bucket that earns nothing.
    #[must_use]
    pub const fn disabled(&self) -> bool {
        matches!(self.reload_rate, Some(0)) && self.tokens == 0
    }

    /// Spends a token.
    ///
    /// # Returns
    ///
    /// `false`, spending nothing, when the bucket is empty.
    pub fn take(&mut self) -> bool {
        match self.reload_rate {
            None => true,
            Some(_) if self.tokens > 0 => {
                self.tokens -= 1;
                true
            }
            Some(_) => false,
        }
    }

    /// Adds what the bucket earns over some hours, keeping no more than its size.
    ///
    /// # Arguments
    ///
    /// * `hours` - the whole hours since the last reload.
    pub fn reload(&mut self, hours: u64) {
        if let Some(rate) = self.reload_rate {
            let earned = u64::from(rate).saturating_mul(hours);
            let tokens = u64::from(self.tokens).saturating_add(earned);
            self.tokens = tokens.min(u64::from(self.size)) as u16;
        }
    }

    /// Sets the tokens, keeping no more than the bucket's size.
    ///
    /// # Arguments
    ///
    /// * `tokens` - the count.
    pub fn set_tokens(&mut self, tokens: u16) {
        if self.reload_rate.is_some() {
            self.tokens = tokens.min(self.size);
        }
    }

    /// Sets the tokens to one hour's reload, as a relay does when it starts scanning.
    pub fn refill_one_hour(&mut self) {
        if let Some(rate) = self.reload_rate {
            self.set_tokens(u16::from(rate));
        }
    }
}

/// What `ConfigureFwdLimitReq` does to the token counters, TS011-1.0.1 table 63.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CounterReset {
    /// Empty every bucket.
    Empty,
    /// Set each to its reload rate.
    ReloadRate,
    /// Fill each.
    Full,
    /// Leave the counters, trimmed to any smaller size. A bucket that had no limit starts
    /// full.
    Keep,
}

impl CounterReset {
    /// Reads a coded value, 0 through 3.
    ///
    /// # Arguments
    ///
    /// * `code` - the value; only its low two bits are read.
    ///
    /// # Returns
    ///
    /// The reset.
    #[must_use]
    pub const fn from_code(code: u8) -> CounterReset {
        match code & 0x03 {
            0 => CounterReset::Empty,
            1 => CounterReset::ReloadRate,
            2 => CounterReset::Full,
            _ => CounterReset::Keep,
        }
    }

    /// Returns the value a `ConfigureFwdLimitReq` carries.
    ///
    /// # Returns
    ///
    /// The code, 0 through 3.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    fn apply(self, bucket: &mut TokenBucket, previous: u16) {
        match self {
            CounterReset::Empty => bucket.set_tokens(0),
            CounterReset::ReloadRate => bucket.refill_one_hour(),
            CounterReset::Full => bucket.set_tokens(bucket.size()),
            CounterReset::Keep => bucket.set_tokens(previous),
        }
    }
}

/// A relay's four relay-wide limits, TS011-1.0.1 tables 23 and 24.
///
/// A join request spends a token of `join_request` and `overall`, an uplink from a trusted
/// end device one of its own bucket, `global_uplink` and `overall`, and a notification of an
/// unknown end device one of `notify` and `overall`.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::relay::ForwardLimits;
///
/// let mut limits = ForwardLimits::new();
/// assert_eq!(limits.join_request.reload_rate(), Some(4));
/// assert_eq!(limits.join_request.size(), 8);
///
/// // A relay starts with an hour's worth of tokens, so a fifth join request waits.
/// for _ in 0..4 {
///     assert!(limits.join_request.take());
/// }
/// assert!(!limits.join_request.has_token());
///
/// // An hour on, four more arrive.
/// limits.reload(1);
/// assert_eq!(limits.join_request.tokens(), 4);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ForwardLimits {
    /// Join requests forwarded.
    pub join_request: TokenBucket,
    /// Notifications of end devices the relay does not trust.
    pub notify: TokenBucket,
    /// Uplinks forwarded for every trusted end device together.
    pub global_uplink: TokenBucket,
    /// Everything the relay sends on behalf of others.
    pub overall: TokenBucket,
}

impl ForwardLimits {
    /// The limits of TS011-1.0.1 table 23, each holding one hour's reload.
    ///
    /// # Returns
    ///
    /// Four join requests and four notifications an hour, each bucket twice that; and eight
    /// uplinks and eight messages overall an hour, each bucket sixteen.
    #[must_use]
    pub const fn new() -> ForwardLimits {
        ForwardLimits {
            join_request: starting(4, 8),
            notify: starting(4, 8),
            global_uplink: starting(8, 16),
            overall: starting(8, 16),
        }
    }

    /// Takes a `ConfigureFwdLimitReq`, TS011-1.0.1 section 10.6.
    ///
    /// # Arguments
    ///
    /// * `reset` - what happens to the counters.
    /// * `reload_rates` - the coded rates, join request, notify, global uplink and overall,
    ///   127 meaning no limit.
    /// * `bucket_sizes` - the coded size multipliers in the same order, table 55.
    pub fn configure(&mut self, reset: CounterReset, reload_rates: [u8; 4], bucket_sizes: [u8; 4]) {
        for (at, bucket) in [
            &mut self.join_request,
            &mut self.notify,
            &mut self.global_uplink,
            &mut self.overall,
        ]
        .into_iter()
        .enumerate()
        {
            let previous = if bucket.reload_rate().is_none() {
                u16::MAX
            } else {
                bucket.tokens()
            };
            *bucket = TokenBucket::coded(reload_rates[at], bucket_sizes[at], UNLIMITED_RELAY_RATE);
            reset.apply(bucket, previous);
        }
    }

    /// Adds what every bucket earns over some hours.
    ///
    /// # Arguments
    ///
    /// * `hours` - the whole hours since the last reload.
    pub fn reload(&mut self, hours: u64) {
        self.join_request.reload(hours);
        self.notify.reload(hours);
        self.global_uplink.reload(hours);
        self.overall.reload(hours);
    }

    /// Sets every bucket to one hour's reload, as TS011-1.0.1 section 8.8 has a relay do
    /// when it starts scanning.
    pub fn restart(&mut self) {
        self.join_request.refill_one_hour();
        self.notify.refill_one_hour();
        self.global_uplink.refill_one_hour();
        self.overall.refill_one_hour();
    }
}

impl Default for ForwardLimits {
    fn default() -> ForwardLimits {
        ForwardLimits::new()
    }
}

const fn starting(reload_rate: u8, size: u16) -> TokenBucket {
    TokenBucket {
        reload_rate: Some(reload_rate),
        size,
        tokens: reload_rate as u16,
    }
}
