//! Pointing a concentrator at the channels it listens on.
//!
//! A concentrator that has been reset, identified and given firmware still hears nothing. Its
//! receivers have not been told which intermediate frequencies to listen on, which radio each
//! takes its samples from, which spreading factors to look for, or that they may run at all.
//! This is what closes that gap.
//!
//! There are ten receivers. Eight take any spreading factor and share a bandwidth fixed in
//! hardware at 125 kHz. One more is fixed to a single spreading factor and bandwidth, which
//! is what a network calls its service channel. The last hears frequency shift keying.
//!
//! Every frequency here is an offset from the carrier a front end is tuned to, not a carrier
//! of its own, which is why a plan is described relative to the radio it sits on.
//!
//! Nothing here opens a bus. The order is data, so it can be checked against the reference
//! with no hardware present.

use super::register::{self, Bank, Register};
use super::tx::{bandwidth_value, Chain};

/// The bandwidth every multi-spreading-factor receiver runs at, in hertz.
///
/// It is fixed in hardware. Only the service receiver takes a bandwidth of its own.
pub const MULTI_BANDWIDTH_HZ: u32 = 125_000;

/// How much spectrum one radio hears in total, in hertz.
///
/// A concentrator tunes its radios once and every channel sits inside this window. It is a
/// property of the radio rather than of the register that carries a channel offset.
pub const RX_BANDWIDTH_HZ: u32 = 1_600_000;

/// How far from the carrier a channel can sit, in hertz.
///
/// Half the radio's window, less half the channel's own width. The register that carries an
/// offset is thirteen bits and reaches far past this, so a channel beyond it is accepted by
/// the chip and then hears nothing at all.
pub const MAX_OFFSET_HZ: i32 = (RX_BANDWIDTH_HZ as i32 - MULTI_BANDWIDTH_HZ as i32) / 2;

/// The sync symbol positions a public network uses.
const PUBLIC_PEAKS: (u8, u8) = (6, 8);

/// The ones a private network uses, which is also what the chip powers up looking for.
const PRIVATE_PEAKS: (u8, u8) = (2, 4);

/// How many receivers take any spreading factor.
pub const MULTI_CHANNELS: usize = 8;

/// The lowest spreading factor a receiver can be asked for.
pub const MIN_SPREADING_FACTOR: u8 = 5;

/// The highest.
pub const MAX_SPREADING_FACTOR: u8 = 12;

/// How many preamble symbols a receiver waits for.
///
/// Ten covers both cases the chip has to serve at once: twelve at the two lowest spreading
/// factors and eight above them.
pub const PREAMBLE_SYMBOLS: u16 = 10;

/// What the channel power reading starts from.
pub const RSSI_DEFAULT: u8 = 85;

/// How heavily that reading is filtered.
pub const RSSI_FILTER_ALPHA: u8 = 0x05;

/// Frequency tracking left to the receiver.
pub const TRACKING_AUTOMATIC: u8 = 0x03;

/// Frequency tracking switched off.
pub const TRACKING_OFF: u8 = 0x00;

/// Fine timing that follows the drift in a straight line.
pub const TIMING_LINEAR: u8 = 0x02;

/// Peak selection left to the receiver.
pub const DFT_PEAK_AUTOMATIC: u8 = 0x03;

/// How many preamble symbols the fixed-rate receiver expects at the usual spreading factors.
pub const SERVICE_PREAMBLE_SYMBOLS: u16 = 8;

/// How many it expects at the two fastest, which is what end device drivers send there.
pub const SERVICE_PREAMBLE_SYMBOLS_FAST: u16 = 12;

/// How far above the noise a peak has to be for the fixed-rate receiver to take it.
pub const SERVICE_PEAK_TO_NOISE: u8 = 52;

/// How many peaks the fixed-rate receiver counts before it believes a preamble.
pub const SERVICE_PEAK_COUNT: u8 = 7;

/// How many peaks it counts on the second pass.
pub const SERVICE_SECOND_PEAK_COUNT: u8 = 5;

/// One of the receivers that takes any spreading factor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Listener {
    /// How far from the carrier it listens, in hertz, which is signed.
    pub offset_hz: i32,
    /// Which radio it takes its samples from.
    pub chain: Chain,
    /// Whether it runs at all.
    pub enabled: bool,
}

impl Listener {
    /// A receiver on the first radio, listening this far from the carrier.
    ///
    /// # Arguments
    ///
    /// * `offset_hz` - how far from the carrier, which is signed.
    ///
    /// # Returns
    ///
    /// The receiver, enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::channel::Listener;
    /// use pamoja_radios::sx1302::tx::Chain;
    ///
    /// let channel = Listener::at(-400_000);
    /// assert_eq!(channel.chain, Chain::A);
    /// assert!(channel.enabled);
    /// ```
    #[must_use]
    pub const fn at(offset_hz: i32) -> Listener {
        Listener {
            offset_hz,
            chain: Chain::A,
            enabled: true,
        }
    }

    /// The same receiver, on the other radio.
    ///
    /// # Arguments
    ///
    /// * `chain` - the radio it takes its samples from.
    ///
    /// # Returns
    ///
    /// The receiver.
    #[must_use]
    pub const fn on(mut self, chain: Chain) -> Listener {
        self.chain = chain;
        self
    }

    /// The same receiver, switched off.
    ///
    /// # Returns
    ///
    /// The receiver, which will neither be clocked nor correlated.
    #[must_use]
    pub const fn off(mut self) -> Listener {
        self.enabled = false;
        self
    }
}

impl Default for Listener {
    fn default() -> Listener {
        Listener::at(0).off()
    }
}

/// What a receiver is told when the packets it hears carry no header of their own.
///
/// A header carries the length, the coding rate, and whether a checksum follows. Without one
/// the receiver has to be told all three up front, and it hears nothing from a sender that
/// disagrees with any of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Implicit {
    /// How long every packet is.
    pub payload_length: u8,
    /// The coding rate, as the denominator less four, so 4/5 is 1 and 4/8 is 4.
    pub coding_rate: u8,
    /// Whether a checksum follows the payload.
    pub crc: bool,
}

/// The receiver fixed to one spreading factor, which is what a network calls its service
/// channel.
///
/// The eight receivers beside it take any spreading factor and share a bandwidth fixed in
/// hardware at 125 kHz. This one takes exactly one spreading factor and a bandwidth of its
/// own, which is what lets a network carry a faster rate on a channel of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Service {
    /// How far from the carrier it listens, in hertz, which is signed.
    pub offset_hz: i32,
    /// Which radio it takes its samples from.
    pub chain: Chain,
    /// The one spreading factor it looks for.
    pub spreading_factor: u8,
    /// The bandwidth it runs at, in hertz. It is the only receiver that takes its own.
    pub bandwidth_hz: u32,
    /// What it assumes when a packet carries no header, or `None` when packets carry one.
    pub implicit: Option<Implicit>,
}

impl Service {
    /// A service channel on the first radio, at 125 kHz, hearing packets that carry a header.
    ///
    /// # Arguments
    ///
    /// * `offset_hz` - how far from the carrier it listens, which is signed.
    /// * `spreading_factor` - the one spreading factor it looks for.
    ///
    /// # Returns
    ///
    /// The service channel.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::channel::Service;
    /// use pamoja_radios::sx1302::tx::Chain;
    ///
    /// // The channel a European network runs at its fast rate: SF7 on 250 kHz.
    /// let service = Service::at(300_000, 7).bandwidth(250_000);
    /// assert_eq!(service.chain, Chain::A);
    /// assert!(service.implicit.is_none(), "packets carry their own header");
    /// ```
    #[must_use]
    pub const fn at(offset_hz: i32, spreading_factor: u8) -> Service {
        Service {
            offset_hz,
            chain: Chain::A,
            spreading_factor,
            bandwidth_hz: MULTI_BANDWIDTH_HZ,
            implicit: None,
        }
    }

    /// The same channel, on the other radio.
    ///
    /// # Arguments
    ///
    /// * `chain` - the radio it takes its samples from.
    ///
    /// # Returns
    ///
    /// The service channel.
    #[must_use]
    pub const fn on(mut self, chain: Chain) -> Service {
        self.chain = chain;
        self
    }

    /// The same channel, at a bandwidth of its own.
    ///
    /// # Arguments
    ///
    /// * `bandwidth_hz` - 125000, 250000, or 500000. Anything else leaves a plan a
    ///   concentrator refuses rather than one it quietly narrows.
    ///
    /// # Returns
    ///
    /// The service channel.
    #[must_use]
    pub const fn bandwidth(mut self, bandwidth_hz: u32) -> Service {
        self.bandwidth_hz = bandwidth_hz;
        self
    }

    /// The same channel, hearing packets that carry no header.
    ///
    /// # Arguments
    ///
    /// * `implicit` - what every packet is, since nothing on the air will say.
    ///
    /// # Returns
    ///
    /// The service channel.
    #[must_use]
    pub const fn without_header(mut self, implicit: Implicit) -> Service {
        self.implicit = Some(implicit);
        self
    }

    /// Whether the concentrator can be set to this channel at all.
    ///
    /// # Returns
    ///
    /// Whether the spreading factor and the bandwidth are ones the chip takes, and the
    /// offset is one the radio hears.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::channel::Service;
    ///
    /// assert!(Service::at(300_000, 7).bandwidth(250_000).supported());
    /// assert!(!Service::at(300_000, 7).bandwidth(200_000).supported(), "not a bandwidth");
    /// assert!(!Service::at(300_000, 13).supported(), "not a spreading factor");
    /// assert!(!Service::at(900_000, 7).supported(), "past what the radio hears");
    /// ```
    #[must_use]
    pub const fn supported(&self) -> bool {
        reachable(self.offset_hz)
            && bandwidth_value(self.bandwidth_hz).is_some()
            && self.spreading_factor >= MIN_SPREADING_FACTOR
            && self.spreading_factor <= MAX_SPREADING_FACTOR
    }
}

/// What a concentrator is to listen for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    /// The carrier the first radio is tuned to, in hertz, which every offset is measured
    /// from.
    pub carrier_hz: u32,
    /// The eight receivers that take any spreading factor.
    pub channels: [Listener; MULTI_CHANNELS],
    /// Which spreading factors to look for, one bit each counting from
    /// [`MIN_SPREADING_FACTOR`].
    pub spreading_factors: u8,
    /// Whether the network is a public one, which decides the sync word the receivers look
    /// for. LoRaWAN networks are public; the chip powers up expecting a private one.
    pub public: bool,
    /// The receiver fixed to one spreading factor, or `None` to leave it where reset left it.
    pub service: Option<Service>,
}

impl Plan {
    /// A plan on one carrier, listening on these offsets.
    ///
    /// # Arguments
    ///
    /// * `carrier_hz` - what the radio is tuned to.
    /// * `offsets_hz` - how far from it each receiver listens. Anything past the eighth is
    ///   ignored, and any receiver left over is switched off.
    ///
    /// # Returns
    ///
    /// The plan, looking for every spreading factor.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::channel::Plan;
    ///
    /// // The eight channels a European gateway usually runs, 867.1 MHz through 868.5 MHz.
    /// // The carrier sits in the middle of them, because a radio only hears so far either
    /// // side of what it is tuned to.
    /// let plan = Plan::new(
    ///     867_800_000,
    ///     &[-700_000, -500_000, -300_000, -100_000, 100_000, 300_000, 500_000, 700_000],
    /// );
    /// assert!(plan.channels[0].enabled);
    /// assert_eq!(plan.channels[4].offset_hz, 100_000);
    /// ```
    #[must_use]
    pub fn new(carrier_hz: u32, offsets_hz: &[i32]) -> Plan {
        let mut channels = [Listener::default(); MULTI_CHANNELS];
        let mut at = 0;
        while at < MULTI_CHANNELS && at < offsets_hz.len() {
            channels[at] = Listener::at(offsets_hz[at]);
            at += 1;
        }
        Plan {
            carrier_hz,
            channels,
            spreading_factors: every_spreading_factor(),
            public: true,
            service: None,
        }
    }

    /// The same plan, on a public network or a private one.
    ///
    /// # Arguments
    ///
    /// * `public` - whether the network is public. Every LoRaWAN network is.
    ///
    /// # Returns
    ///
    /// The plan.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::channel::Plan;
    ///
    /// // A plan listens to a public network unless it is told otherwise.
    /// assert!(Plan::new(867_800_000, &[0]).public);
    /// assert!(!Plan::new(867_800_000, &[0]).network(false).public);
    /// ```
    #[must_use]
    pub const fn network(mut self, public: bool) -> Plan {
        self.public = public;
        self
    }

    /// The same plan, with the fixed-rate receiver pointed at a channel.
    ///
    /// That receiver is switched on whatever a plan says, so one never given a channel
    /// listens wherever reset left it. This is what gives it somewhere to listen.
    ///
    /// # Arguments
    ///
    /// * `service` - the channel it runs on.
    ///
    /// # Returns
    ///
    /// The plan.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::channel::{Plan, Service};
    ///
    /// let plan = Plan::new(867_800_000, &[-300_000])
    ///     .serving(Service::at(500_000, 7).bandwidth(250_000));
    /// assert_eq!(plan.service.map(|service| service.spreading_factor), Some(7));
    /// ```
    #[must_use]
    pub const fn serving(mut self, service: Service) -> Plan {
        self.service = Some(service);
        self
    }

    /// The same plan, looking only for these spreading factors.
    ///
    /// # Arguments
    ///
    /// * `wanted` - the spreading factors to look for. Anything outside 5 to 12 is ignored.
    ///
    /// # Returns
    ///
    /// The plan.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::channel::Plan;
    ///
    /// // A network that only runs the faster half of the range.
    /// let plan = Plan::new(867_500_000, &[-400_000]).looking_for(&[7, 8, 9]);
    /// assert_eq!(plan.spreading_factors, 0b0001_1100);
    /// ```
    #[must_use]
    pub fn looking_for(mut self, wanted: &[u8]) -> Plan {
        let mut mask = 0u8;
        let mut at = 0;
        while at < wanted.len() {
            let spreading_factor = wanted[at];
            if (MIN_SPREADING_FACTOR..=MAX_SPREADING_FACTOR).contains(&spreading_factor) {
                mask |= 1 << (spreading_factor - MIN_SPREADING_FACTOR);
            }
            at += 1;
        }
        self.spreading_factors = mask;
        self
    }

    /// Which radio each receiver takes its samples from, one bit per receiver.
    ///
    /// # Returns
    ///
    /// The mask the chip takes, with a bit set for every receiver on the second radio.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::channel::Plan;
    /// use pamoja_radios::sx1302::tx::Chain;
    ///
    /// let mut plan = Plan::new(867_500_000, &[0, 0]);
    /// plan.channels[1] = plan.channels[1].on(Chain::B);
    /// assert_eq!(plan.radios(), 0b0000_0010);
    /// ```
    #[must_use]
    pub const fn radios(&self) -> u8 {
        let mut mask = 0u8;
        let mut at = 0;
        while at < MULTI_CHANNELS {
            if matches!(self.channels[at].chain, Chain::B) {
                mask |= 1 << at;
            }
            at += 1;
        }
        mask
    }

    /// Which receivers run, one bit each.
    ///
    /// # Returns
    ///
    /// The mask the chip takes.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::channel::Plan;
    ///
    /// // Three offsets given, so three receivers run and five do not.
    /// let plan = Plan::new(867_500_000, &[-400_000, -200_000, 0]);
    /// assert_eq!(plan.running(), 0b0000_0111);
    /// ```
    #[must_use]
    pub const fn running(&self) -> u8 {
        let mut mask = 0u8;
        let mut at = 0;
        while at < MULTI_CHANNELS {
            if self.channels[at].enabled {
                mask |= 1 << at;
            }
            at += 1;
        }
        mask
    }
}

/// Every spreading factor, as the mask the chip takes.
///
/// # Returns
///
/// A bit for each of the eight, counting from [`MIN_SPREADING_FACTOR`].
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::channel::every_spreading_factor;
///
/// assert_eq!(every_spreading_factor(), 0xff);
/// ```
#[must_use]
pub const fn every_spreading_factor() -> u8 {
    0xff
}

/// Turns an offset from the carrier into the value a channel register takes.
///
/// The chip counts in steps of its own, not in hertz: the register is the offset times 32
/// over 15625, which divides exactly. A negative offset is carried as the low thirteen bits,
/// which is what the two registers hold between them.
///
/// # Arguments
///
/// * `offset_hz` - how far from the carrier, which is signed.
///
/// # Returns
///
/// The value, of which the high five bits and the low eight are written separately.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::channel::offset_value;
///
/// // A channel 400 kHz below the carrier.
/// assert_eq!(offset_value(-400_000), (-819i16) as u16 & 0x1fff);
///
/// // And one on it.
/// assert_eq!(offset_value(0), 0);
/// ```
#[must_use]
pub const fn offset_value(offset_hz: i32) -> u16 {
    let steps = (offset_hz as i64 * 32 / 15_625) as i32;
    (steps as u16) & 0x1fff
}

/// Splits a channel offset into the two bytes its registers take.
///
/// # Arguments
///
/// * `offset_hz` - how far from the carrier.
///
/// # Returns
///
/// The high bits and then the low bits.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::channel::offset_bytes;
///
/// let [high, low] = offset_bytes(-400_000);
/// assert_eq!(high, 0x1c);
/// assert_eq!(low, 0xcd);
/// ```
#[must_use]
pub const fn offset_bytes(offset_hz: i32) -> [u8; 2] {
    let value = offset_value(offset_hz);
    [((value >> 8) & 0x1f) as u8, (value & 0xff) as u8]
}

/// How fast a frequency error turns into a timing one, as a mantissa and a shift.
///
/// The receiver corrects timing from the frequency error it measures, and how much it should
/// correct depends on where the carrier sits: the same error is a larger fraction of a lower
/// carrier. The chip takes that as a mantissa of at least 2048 and the shift that got it
/// there.
///
/// # Arguments
///
/// * `bandwidth_hz` - the bandwidth the receiver runs at.
/// * `carrier_hz` - what the radio is tuned to.
///
/// # Returns
///
/// The mantissa and the shift, or `None` for a carrier of zero.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::channel::drift;
///
/// // At 125 kHz on 868.1 MHz the ratio starts at 150 and doubles four times to reach 2400.
/// assert_eq!(drift(125_000, 868_100_000), Some((2400, 4)));
/// ```
#[must_use]
pub const fn drift(bandwidth_hz: u32, carrier_hz: u32) -> Option<(u16, u8)> {
    if carrier_hz == 0 {
        return None;
    }
    let mut mantissa = (bandwidth_hz as u64) * (1 << 20) / (carrier_hz as u64);
    let mut shift = 0u8;
    while mantissa < 2048 {
        mantissa <<= 1;
        shift += 1;
    }
    Some((mantissa as u16, shift))
}

/// A step in pointing a concentrator at its channels.
///
/// The order is carried as data, so a driver walks it against a bus and a test walks it
/// against nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Write a value into one register.
    Write(Register, u8),
}

/// The order a concentrator is pointed at its channels in.
///
/// The receivers are given their frequencies and radios, then the correlators are tuned and
/// told which spreading factors to look for, then the demodulators are set up, and only at
/// the end is anything switched on. Enabling earlier would start receivers that have not been
/// told where to listen.
///
/// # Arguments
///
/// * `plan` - what to listen for.
///
/// # Returns
///
/// The steps, in the order the chip takes them.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::channel::{steps, Plan, Step};
/// use pamoja_radios::sx1302::register;
///
/// let plan = Plan::new(867_500_000, &[-400_000, -200_000, 0]);
/// let order: Vec<Step> = steps(&plan).collect();
///
/// // Nothing is switched on until the end.
/// let enable = order
///     .iter()
///     .position(|step| matches!(step, Step::Write(register, _) if *register == register::COMMON_GLOBAL_ENABLE))
///     .expect("the receivers are switched on");
/// assert_eq!(enable, order.len() - 1);
/// ```
pub fn steps(plan: &Plan) -> impl Iterator<Item = Step> + '_ {
    channelizer(plan)
        .chain(correlators(plan))
        .chain(demodulators(plan))
        .chain(syncword(plan))
        .chain(service(plan))
        .chain(enables())
}

/// Telling the receivers which network they are listening to.
///
/// A LoRa preamble ends in two symbols whose positions say which network a frame belongs to,
/// and the chip looks for them where it is told. It powers up looking for a private network,
/// so receivers that are never given this hear nothing from a public one however well
/// everything else is configured.
///
/// The two lowest spreading factors have no public form and always take the private
/// positions, which is what the reference does.
fn syncword(plan: &Plan) -> impl Iterator<Item = Step> {
    let (first, second) = if plan.public {
        PUBLIC_PEAKS
    } else {
        PRIVATE_PEAKS
    };
    let (low_first, low_second) = PRIVATE_PEAKS;

    // The fixed-rate receiver follows the network, except at the two fastest spreading
    // factors, which have no public form however public the network is.
    let (service_first, service_second) = match plan.service {
        Some(service) if matches!(service.spreading_factor, 5 | 6) => PRIVATE_PEAKS,
        _ => (first, second),
    };

    [
        Step::Write(register::RX_SYNC_SF5_PEAK1, low_first),
        Step::Write(register::RX_SYNC_SF5_PEAK2, low_second),
        Step::Write(register::RX_SYNC_SF6_PEAK1, low_first),
        Step::Write(register::RX_SYNC_SF6_PEAK2, low_second),
        Step::Write(register::RX_SYNC_SF7_TO_SF12_PEAK1, first),
        Step::Write(register::RX_SYNC_SF7_TO_SF12_PEAK2, second),
        Step::Write(register::SERVICE_SYNC_PEAK1, service_first),
        Step::Write(register::SERVICE_SYNC_PEAK2, service_second),
    ]
    .into_iter()
}

/// Whether a rate is slow enough to need the low data rate optimization.
///
/// A symbol that lasts long enough drifts against the receiver clock before it ends, and the
/// two ends have to agree to carry fewer bits in it. Which rates those are depends on the
/// bandwidth as well as the spreading factor.
///
/// # Arguments
///
/// * `spreading_factor` - the spreading factor.
/// * `bandwidth_hz` - the bandwidth, in hertz.
///
/// # Returns
///
/// Whether the optimization is on for that pair.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::channel::low_rate_optimize;
///
/// assert!(low_rate_optimize(11, 125_000));
/// assert!(low_rate_optimize(12, 125_000));
/// assert!(!low_rate_optimize(10, 125_000));
///
/// // The same spreading factor twice as wide halves the symbol, so it is only on at SF12.
/// assert!(!low_rate_optimize(11, 250_000));
/// assert!(low_rate_optimize(12, 250_000));
/// assert!(!low_rate_optimize(12, 500_000));
/// ```
#[must_use]
pub const fn low_rate_optimize(spreading_factor: u8, bandwidth_hz: u32) -> bool {
    match bandwidth_hz {
        125_000 => matches!(spreading_factor, 11 | 12),
        250_000 => spreading_factor == 12,
        _ => false,
    }
}

/// Whether a channel offset is one the radio can actually hear.
///
/// # Arguments
///
/// * `offset_hz` - how far from the carrier the channel sits.
///
/// # Returns
///
/// Whether it falls inside the radio's window.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::channel::{reachable, MAX_OFFSET_HZ};
///
/// assert_eq!(MAX_OFFSET_HZ, 737_500);
/// assert!(reachable(700_000));
///
/// // The register carries this happily. The radio does not hear it.
/// assert!(!reachable(1_000_000));
/// ```
#[must_use]
pub const fn reachable(offset_hz: i32) -> bool {
    offset_hz >= -MAX_OFFSET_HZ && offset_hz <= MAX_OFFSET_HZ
}

/// Giving each receiver its frequency, its radio, and the gain it reads power against.
fn channelizer(plan: &Plan) -> impl Iterator<Item = Step> + '_ {
    let frequencies = (0..MULTI_CHANNELS).flat_map(move |at| {
        let [high, low] = offset_bytes(plan.channels[at].offset_hz);
        let [msb, lsb] = register::channel_frequency(at as u8);
        [Step::Write(msb, high), Step::Write(lsb, low)]
    });

    [
        Step::Write(register::RX_RADIO_SELECT, plan.radios()),
        Step::Write(register::RX_RSSI_FILTER_ALPHA, RSSI_FILTER_ALPHA),
        Step::Write(register::RX_RSSI_DEFAULT, RSSI_DEFAULT),
        Step::Write(register::RX_GAIN_AUTOMATIC, 0x01),
        Step::Write(register::RX_GAIN_THRESHOLD_HIGH, 0xff),
        Step::Write(register::RX_GAIN_THRESHOLD_LOW, 0x00),
        Step::Write(register::RX_GAIN_MAX_ATTENUATION, 15),
        Step::Write(register::RX_GAIN_MIN_ATTENUATION, 0),
    ]
    .into_iter()
    .chain(frequencies)
}

/// Tuning detection for every spreading factor, then saying which to look for.
fn correlators(plan: &Plan) -> impl Iterator<Item = Step> + '_ {
    let tuned = (MIN_SPREADING_FACTOR..=MAX_SPREADING_FACTOR).flat_map(|spreading_factor| {
        let correlator = register::correlator(spreading_factor);
        [
            Step::Write(correlator.accumulated_ratio, 52),
            Step::Write(correlator.peak_ratio, 24),
            Step::Write(correlator.peaks, 7),
            Step::Write(correlator.second_peaks, 5),
        ]
    });

    tuned.chain([
        Step::Write(register::RX_CORRELATOR_FIRST_EDGE, 0xff),
        Step::Write(register::RX_CORRELATOR_CLEAR, 0xff),
        Step::Write(register::RX_CORRELATOR_SF_ENABLE, plan.spreading_factors),
        Step::Write(register::RX_CORRELATOR_CLOCK_ENABLE, plan.running()),
    ])
}

/// Setting up the demodulators behind the correlators.
fn demodulators(plan: &Plan) -> impl Iterator<Item = Step> + '_ {
    let sync = (0..MULTI_CHANNELS).map(|at| {
        // The four receivers on each radio are staggered so they do not all look at once.
        let offset = [1u8, 5, 9, 13][at % 4];
        Step::Write(register::channel_sync_offset(at as u8), offset)
    });

    let tracking = (MIN_SPREADING_FACTOR..=MAX_SPREADING_FACTOR).flat_map(|spreading_factor| {
        [
            Step::Write(
                register::frequency_tracking(Bank::Demodulation, spreading_factor),
                TRACKING_AUTOMATIC,
            ),
            Step::Write(
                register::frequency_tracking(Bank::Timestamping, spreading_factor),
                TRACKING_OFF,
            ),
        ]
    });

    let corrections = (MIN_SPREADING_FACTOR..=MAX_SPREADING_FACTOR).map(|spreading_factor| {
        // The two slowest spreading factors drift far enough across a symbol to need it.
        let correction = u8::from(spreading_factor >= 11);
        Step::Write(register::drift_correction(spreading_factor), correction)
    });

    let (mantissa, shift) = drift(MULTI_BANDWIDTH_HZ, plan.carrier_hz).unwrap_or((2048, 0));
    let preamble = PREAMBLE_SYMBOLS.to_be_bytes();

    [
        Step::Write(register::RX_DC_NOTCH_ENABLE, 0x00),
        Step::Write(register::RX_FORCE_DEFAULT_FILTER, 0x01),
        Step::Write(register::RX_GAIN_DROP_COMPENSATION, 0x01),
        Step::Write(register::RX_GAIN_TARGET_LEVEL, 0x01),
        Step::Write(register::MODEM_ENABLE_FULL, 0xff),
        Step::Write(register::MODEM_ENABLE_LIMITED, 0xff),
        Step::Write(register::RX_MODEM_SYNC_DELTA_MSB, 0),
        Step::Write(register::RX_MODEM_SYNC_DELTA_LSB, 126),
    ]
    .into_iter()
    .chain(sync)
    .chain(corrections)
    .chain([
        Step::Write(register::RX_FINE_TIMING_GAIN_AUTO, 3),
        Step::Write(register::RX_FINE_TIMING_GAIN_PAYLOAD, 3),
        Step::Write(register::RX_FINE_TIMING_INTEGRATE_SF11, 1),
        Step::Write(register::RX_FINE_TIMING_INTEGRATE_SF12, 1),
        Step::Write(register::RX_FREQUENCY_SYNC_THRESHOLD, 15),
    ])
    .chain(tracking)
    .chain([
        Step::Write(register::RX_TIMING_INTEGRATE_SF11, 0),
        Step::Write(register::RX_TIMING_INTEGRATE_SF12, 0),
        Step::Write(register::RX_TIMING_ROUNDING, 1),
        Step::Write(register::RX_TIMING_MODE, TIMING_LINEAR),
        Step::Write(register::RX_TIMING_GAIN_AUTO, 0),
        Step::Write(register::RX_TIMING_GAIN_PREAMBLE, 6),
        Step::Write(register::RX_TIMING_GAIN_PAYLOAD, 2),
        Step::Write(register::RX_TIMING_INTEGRAL_AUTO, 0),
        Step::Write(register::RX_TIMING_INTEGRAL_PREAMBLE, 1),
        Step::Write(register::RX_TIMING_INTEGRAL_PAYLOAD, 0),
        Step::Write(register::RX_PREAMBLE_SYMBOLS_MSB, preamble[0]),
        Step::Write(register::RX_PREAMBLE_SYMBOLS_LSB, preamble[1]),
        Step::Write(
            register::RX_DRIFT_MANTISSA_MSB,
            ((mantissa >> 8) & 0x0f) as u8,
        ),
        Step::Write(register::RX_DRIFT_MANTISSA_LSB, (mantissa & 0xff) as u8),
        Step::Write(register::RX_DRIFT_EXPONENT, shift),
        Step::Write(register::RX_DRIFT_INVERT_SYMBOL_TIME, 1),
        Step::Write(register::RX_DFT_PEAK_MODE, DFT_PEAK_AUTOMATIC),
    ])
}

/// Pointing the fixed-rate receiver at its channel, when a plan gives it one.
///
/// A plan without one writes nothing here, which leaves that receiver switched on and
/// listening wherever reset left it.
fn service(plan: &Plan) -> impl Iterator<Item = Step> + '_ {
    plan.service.into_iter().flat_map(move |service| {
        let spreading_factor = service.spreading_factor;
        let [high, low] = offset_bytes(service.offset_hz);

        // A bandwidth the chip has no encoding for is refused before a plan reaches a bus,
        // so this only decides what a plan that never will looks like.
        let bandwidth = bandwidth_value(service.bandwidth_hz).unwrap_or(0);

        let fast = matches!(spreading_factor, 5 | 6);
        let preamble = if fast {
            SERVICE_PREAMBLE_SYMBOLS_FAST
        } else {
            SERVICE_PREAMBLE_SYMBOLS
        }
        .to_be_bytes();

        // A slower spreading factor holds a symbol longer, so timing is corrected harder
        // through the preamble and, at the two slowest, accumulated through the payload.
        let preamble_gain = match spreading_factor {
            5 | 6 => 4,
            7..=10 => 6,
            _ => 7,
        };
        let integral_gain = match (spreading_factor, service.bandwidth_hz) {
            (5..=10, _) => 0,
            (_, 125_000) => 1,
            (_, 250_000) => 2,
            (_, 500_000) => 3,
            _ => 0,
        };

        let implicit = service.implicit.unwrap_or(Implicit {
            payload_length: 0,
            coding_rate: 0,
            crc: false,
        });
        let (mantissa, shift) = drift(service.bandwidth_hz, plan.carrier_hz).unwrap_or((2048, 0));

        [
            Step::Write(register::SERVICE_FREQUENCY_MSB, high),
            Step::Write(register::SERVICE_FREQUENCY_LSB, low),
            Step::Write(
                register::SERVICE_RADIO_SELECT,
                u8::from(matches!(service.chain, Chain::B)),
            ),
            Step::Write(register::SERVICE_DETECT_PEAK_COUNT, SERVICE_PEAK_COUNT),
            Step::Write(
                register::SERVICE_DETECT_PEAK_COUNT2,
                SERVICE_SECOND_PEAK_COUNT,
            ),
            Step::Write(register::SERVICE_DETECT_USE_GAIN_SYMBOL, 1),
            Step::Write(register::SERVICE_FINE_SYNC, u8::from(fast)),
            Step::Write(
                register::SERVICE_DETECT_PEAK_TO_NOISE,
                SERVICE_PEAK_TO_NOISE,
            ),
            Step::Write(register::SERVICE_DC_NOTCH_ENABLE, 0x00),
            Step::Write(register::SERVICE_FORCE_DEFAULT_FIR, 0x01),
            Step::Write(register::SERVICE_DAGC_GAIN_DROP_COMP, 0x01),
            Step::Write(register::SERVICE_DAGC_TARGET_LEVEL, 0x01),
            Step::Write(register::SERVICE_TIMING_GAIN_AUTOMATIC, 0x03),
            Step::Write(register::SERVICE_TIMING_GAIN_PAYLOAD, 0x03),
            Step::Write(register::SERVICE_TIMING_GAIN_PREAMBLE, preamble_gain),
            Step::Write(register::SERVICE_TIMING_GAIN_INTEGRAL, integral_gain),
            Step::Write(
                register::SERVICE_IMPLICIT_HEADER,
                u8::from(service.implicit.is_some()),
            ),
            Step::Write(register::SERVICE_CRC_ENABLE, u8::from(implicit.crc)),
            Step::Write(register::SERVICE_CODING_RATE, implicit.coding_rate),
            Step::Write(register::SERVICE_PAYLOAD_LENGTH, implicit.payload_length),
            Step::Write(register::SERVICE_SPREADING_FACTOR, spreading_factor),
            Step::Write(register::SERVICE_BANDWIDTH, bandwidth),
            Step::Write(
                register::SERVICE_PPM_OFFSET,
                u8::from(low_rate_optimize(spreading_factor, service.bandwidth_hz)),
            ),
            Step::Write(register::SERVICE_PREAMBLE_MSB, preamble[0]),
            Step::Write(register::SERVICE_PREAMBLE_LSB, preamble[1]),
            Step::Write(
                register::SERVICE_DRIFT_MANTISSA_MSB,
                ((mantissa >> 8) & 0x0f) as u8,
            ),
            Step::Write(
                register::SERVICE_DRIFT_MANTISSA_LSB,
                (mantissa & 0xff) as u8,
            ),
            Step::Write(register::SERVICE_DRIFT_EXPONENT, shift),
            Step::Write(register::SERVICE_DRIFT_INVERT_SYMBOL_TIME, 1),
            Step::Write(register::SERVICE_DAGC_IN_COMP, 1),
            Step::Write(register::SERVICE_MODEM_ENABLE, 1),
            Step::Write(register::SERVICE_CAD_RX_TX, 1),
            Step::Write(register::SERVICE_MODEM_START, 1),
            Step::Write(register::SERVICE_DFT_PEAK_MODE, DFT_PEAK_AUTOMATIC),
        ]
        .into_iter()
    })
}

/// Switching the receivers on, which is the last thing done.
fn enables() -> impl Iterator<Item = Step> {
    [
        Step::Write(register::COMMON_MULTI_MODEM_ENABLE, 0x01),
        Step::Write(register::COMMON_SERVICE_MODEM_ENABLE, 0x01),
        Step::Write(register::COMMON_FSK_MODEM_ENABLE, 0x01),
        Step::Write(register::COMMON_GLOBAL_ENABLE, 0x01),
    ]
    .into_iter()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The eight channels a European gateway usually runs, around one carrier.
    ///
    /// The carrier sits in the middle of the band rather than at its low end, because a
    /// radio only hears so far either side of what it is tuned to.
    fn eu868() -> Plan {
        Plan::new(
            867_800_000,
            &[
                -700_000, -500_000, -300_000, -100_000, 100_000, 300_000, 500_000, 700_000,
            ],
        )
    }

    #[test]
    fn the_receivers_are_told_which_network_they_are_on() {
        let public: Vec<Step> = steps(&eu868()).collect();
        let private: Vec<Step> = steps(&eu868().network(false)).collect();

        let written = |order: &[Step], want: register::Register| {
            order
                .iter()
                .find_map(|Step::Write(named, value)| (*named == want).then_some(*value))
                .expect("the sync word is written")
        };

        // A public network moves both sync symbols out. A private one leaves them where the
        // chip powers up, which is why a gateway that never writes these hears nothing from
        // a public network however well everything else is configured.
        assert_eq!(written(&public, register::RX_SYNC_SF7_TO_SF12_PEAK1), 6);
        assert_eq!(written(&public, register::RX_SYNC_SF7_TO_SF12_PEAK2), 8);
        assert_eq!(written(&private, register::RX_SYNC_SF7_TO_SF12_PEAK1), 2);
        assert_eq!(written(&private, register::RX_SYNC_SF7_TO_SF12_PEAK2), 4);

        // The two lowest spreading factors have no public form, on either kind of network.
        assert_eq!(written(&public, register::RX_SYNC_SF5_PEAK1), 2);
        assert_eq!(written(&public, register::RX_SYNC_SF5_PEAK2), 4);
        assert_eq!(written(&public, register::RX_SYNC_SF6_PEAK1), 2);
        assert_eq!(written(&public, register::RX_SYNC_SF6_PEAK2), 4);
    }

    #[test]
    fn the_sync_word_is_written_before_anything_is_switched_on() {
        let order: Vec<Step> = steps(&eu868()).collect();

        let sync = order
            .iter()
            .position(|step| {
                matches!(step, Step::Write(named, _) if *named == register::RX_SYNC_SF7_TO_SF12_PEAK1)
            })
            .expect("the sync word is written");
        let enable = order
            .iter()
            .position(|step| {
                matches!(step, Step::Write(named, _) if *named == register::COMMON_GLOBAL_ENABLE)
            })
            .expect("the receivers are switched on");

        assert!(sync < enable);
    }

    #[test]
    fn a_channel_past_the_radio_window_is_not_reachable() {
        // Half the radio's window, less half a channel. The plan this file used to carry
        // reached a megahertz, which is past it on both counts.
        assert_eq!(MAX_OFFSET_HZ, 737_500);
        assert!(reachable(MAX_OFFSET_HZ));
        assert!(reachable(-MAX_OFFSET_HZ));
        assert!(!reachable(MAX_OFFSET_HZ + 1));
        assert!(!reachable(800_000));
        assert!(!reachable(-1_000_000));

        // And every channel the shipped plan uses is inside it.
        for listener in &eu868().channels {
            assert!(reachable(listener.offset_hz), "{}", listener.offset_hz);
        }
    }

    #[test]
    fn an_offset_is_counted_in_chip_steps() {
        // Worked out from the ratio rather than from the function: 400000 * 32 / 15625 is
        // 819 exactly, and below the carrier it is carried as a negative in thirteen bits.
        assert_eq!(offset_value(400_000), 819);
        assert_eq!(offset_value(-400_000), 0x1fff - 819 + 1);
        assert_eq!(offset_bytes(0), [0x00, 0x00]);
    }

    #[test]
    fn the_drift_reaches_the_range_the_chip_wants() {
        // The mantissa starts below 2048 and is doubled until it is not, and the count of
        // doublings is what the chip is told.
        let (mantissa, shift) = drift(125_000, 868_100_000).expect("a real carrier");
        assert_eq!((mantissa, shift), (2400, 4));
        assert!(mantissa >= 2048);

        // A higher carrier is a smaller ratio, so it takes one more doubling.
        let (_, higher) = drift(125_000, 1_500_000_000).expect("a real carrier");
        assert!(higher > shift);

        assert_eq!(drift(125_000, 0), None);
    }

    #[test]
    fn the_drift_mantissa_fits_the_field_it_is_written_to() {
        // The high half goes into four bits, so a mantissa over twelve bits would be cut.
        // Every carrier a concentrator runs on has to stay inside that.
        for carrier in [433_000_000, 470_000_000, 868_100_000, 915_000_000] {
            let (mantissa, _) = drift(MULTI_BANDWIDTH_HZ, carrier).expect("a real carrier");
            assert!(mantissa < 4096, "{carrier} gives {mantissa}");
        }
    }

    #[test]
    fn a_plan_switches_off_the_receivers_it_was_not_given() {
        let plan = Plan::new(867_500_000, &[-400_000, 0]);
        assert_eq!(plan.running(), 0b0000_0011);
        assert!(!plan.channels[2].enabled);

        // And a plan given more than there are receivers takes the first eight.
        let full = Plan::new(867_500_000, &[0; 12]);
        assert_eq!(full.running(), 0xff);
    }

    #[test]
    fn the_radios_are_read_off_the_receivers() {
        let mut plan = eu868();
        for at in 4..MULTI_CHANNELS {
            plan.channels[at] = plan.channels[at].on(Chain::B);
        }
        assert_eq!(plan.radios(), 0b1111_0000);
    }

    #[test]
    fn only_the_spreading_factors_asked_for_are_looked_for() {
        let plan = eu868().looking_for(&[7, 12]);
        assert_eq!(plan.spreading_factors, 0b1000_0100);

        // One outside the range is not a bit anywhere.
        let odd = eu868().looking_for(&[4, 13]);
        assert_eq!(odd.spreading_factors, 0);
    }

    #[test]
    fn nothing_runs_before_it_has_been_told_where_to_listen() {
        let plan = eu868();
        let order: Vec<Step> = steps(&plan).collect();

        let first_frequency = order
            .iter()
            .position(|step| {
                matches!(step, Step::Write(register, _) if register.address == register::channel_frequency(0)[0].address)
            })
            .expect("the receivers are given their frequencies");
        let enable = order
            .iter()
            .position(|step| {
                matches!(step, Step::Write(register, _) if *register == register::COMMON_GLOBAL_ENABLE)
            })
            .expect("the receivers are switched on");

        assert!(first_frequency < enable, "{order:?}");
        assert_eq!(enable, order.len() - 1);
    }

    #[test]
    fn every_receiver_is_given_a_frequency_even_when_it_is_off() {
        // A receiver that is off still has its registers set, because leaving them at
        // whatever they held is how a channel comes back in a strange place later.
        let plan = Plan::new(867_500_000, &[-400_000]);
        let order: Vec<Step> = steps(&plan).collect();

        for channel in 0..MULTI_CHANNELS {
            let [msb, _] = register::channel_frequency(channel as u8);
            assert!(
                order
                    .iter()
                    .any(|step| matches!(step, Step::Write(register, _) if *register == msb)),
                "channel {channel} was left as it was"
            );
        }
    }

    #[test]
    fn the_correlators_are_tuned_for_every_spreading_factor() {
        let order: Vec<Step> = steps(&eu868()).collect();

        for spreading_factor in MIN_SPREADING_FACTOR..=MAX_SPREADING_FACTOR {
            let correlator = register::correlator(spreading_factor);
            assert!(
                order.iter().any(|step| {
                    matches!(step, Step::Write(register, value) if *register == correlator.accumulated_ratio && *value == 52)
                }),
                "SF{spreading_factor} was left untuned"
            );
        }
    }

    /// What one register was written, out of a whole plan.
    fn written(plan: &Plan, want: register::Register) -> Option<u8> {
        steps(plan).find_map(|Step::Write(named, value)| (named == want).then_some(value))
    }

    /// The European plan, with the fast channel a network runs beside it.
    fn with_service() -> Plan {
        eu868().serving(Service::at(-200_000, 7).bandwidth(250_000))
    }

    #[test]
    fn a_service_channel_is_given_a_frequency_and_a_radio() {
        let plan = with_service();
        let [high, low] = offset_bytes(-200_000);

        assert_eq!(written(&plan, register::SERVICE_FREQUENCY_MSB), Some(high));
        assert_eq!(written(&plan, register::SERVICE_FREQUENCY_LSB), Some(low));
        assert_eq!(written(&plan, register::SERVICE_RADIO_SELECT), Some(0));
        assert_eq!(written(&plan, register::SERVICE_SPREADING_FACTOR), Some(7));
        assert_eq!(written(&plan, register::SERVICE_BANDWIDTH), Some(5));
        assert_eq!(written(&plan, register::SERVICE_MODEM_START), Some(1));

        let other = eu868().serving(Service::at(0, 7).on(Chain::B));
        assert_eq!(written(&other, register::SERVICE_RADIO_SELECT), Some(1));
    }

    #[test]
    fn a_plan_without_a_service_channel_leaves_its_rate_alone() {
        let plan = eu868();
        assert_eq!(plan.service, None);
        assert_eq!(written(&plan, register::SERVICE_FREQUENCY_MSB), None);
        assert_eq!(written(&plan, register::SERVICE_SPREADING_FACTOR), None);
        assert_eq!(written(&plan, register::SERVICE_MODEM_START), None);
    }

    #[test]
    fn the_two_fastest_spreading_factors_stay_private_on_a_public_network() {
        // Everything else on a public network moves its sync symbols out.
        let fast = eu868().serving(Service::at(0, 5));
        assert_eq!(written(&fast, register::SERVICE_SYNC_PEAK1), Some(2));
        assert_eq!(written(&fast, register::SERVICE_SYNC_PEAK2), Some(4));
        assert_eq!(written(&fast, register::RX_SYNC_SF7_TO_SF12_PEAK1), Some(6));

        let slow = eu868().serving(Service::at(0, 7));
        assert_eq!(written(&slow, register::SERVICE_SYNC_PEAK1), Some(6));
        assert_eq!(written(&slow, register::SERVICE_SYNC_PEAK2), Some(8));

        // A private network is private for every receiver.
        let private = eu868().network(false).serving(Service::at(0, 7));
        assert_eq!(written(&private, register::SERVICE_SYNC_PEAK1), Some(2));
    }

    #[test]
    fn the_fast_spreading_factors_expect_a_longer_preamble() {
        let fast = eu868().serving(Service::at(0, 6));
        let usual = eu868().serving(Service::at(0, 9));

        assert_eq!(written(&fast, register::SERVICE_PREAMBLE_LSB), Some(12));
        assert_eq!(written(&usual, register::SERVICE_PREAMBLE_LSB), Some(8));

        // They are also the only ones that synchronize finely.
        assert_eq!(written(&fast, register::SERVICE_FINE_SYNC), Some(1));
        assert_eq!(written(&usual, register::SERVICE_FINE_SYNC), Some(0));
    }

    #[test]
    fn the_low_rate_optimization_follows_the_bandwidth_as_well_as_the_rate() {
        let narrow = eu868().serving(Service::at(0, 11));
        let wide = eu868().serving(Service::at(0, 11).bandwidth(250_000));

        assert_eq!(written(&narrow, register::SERVICE_PPM_OFFSET), Some(1));
        assert_eq!(
            written(&wide, register::SERVICE_PPM_OFFSET),
            Some(0),
            "the same spreading factor twice as wide does not need it"
        );
    }

    #[test]
    fn a_packet_without_a_header_carries_what_the_header_would_have() {
        let explicit = eu868().serving(Service::at(0, 7));
        assert_eq!(
            written(&explicit, register::SERVICE_IMPLICIT_HEADER),
            Some(0)
        );
        assert_eq!(
            written(&explicit, register::SERVICE_PAYLOAD_LENGTH),
            Some(0)
        );

        let implicit = eu868().serving(Service::at(0, 7).without_header(Implicit {
            payload_length: 23,
            coding_rate: 1,
            crc: true,
        }));
        assert_eq!(
            written(&implicit, register::SERVICE_IMPLICIT_HEADER),
            Some(1)
        );
        assert_eq!(
            written(&implicit, register::SERVICE_PAYLOAD_LENGTH),
            Some(23)
        );
        assert_eq!(written(&implicit, register::SERVICE_CODING_RATE), Some(1));
        assert_eq!(written(&implicit, register::SERVICE_CRC_ENABLE), Some(1));
    }

    #[test]
    fn a_rate_the_receiver_cannot_run_is_refused_rather_than_narrowed() {
        assert!(Service::at(0, 7).supported());
        assert!(Service::at(0, 12).bandwidth(500_000).supported());

        assert!(!Service::at(0, 4).supported(), "below the slowest");
        assert!(!Service::at(0, 13).supported(), "past the fastest");
        assert!(
            !Service::at(0, 7).bandwidth(62_500).supported(),
            "not a width"
        );
        assert!(!Service::at(MAX_OFFSET_HZ + 1, 7).supported(), "not heard");
    }
}
