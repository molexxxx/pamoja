//! Putting a packet on the air.
//!
//! Transmitting is three things in order: the payload goes into a buffer belonging to one of
//! the two chains, the chip is told how long to wait before it starts, and then a trigger is
//! armed. Which trigger decides when: now, at a counter value, or on the pulse from a GPS.
//!
//! The waiting is the part worth care. A gateway answers a device inside a receive window a
//! second wide, and the chip does not begin radiating the instant it is triggered: it has a
//! front end to settle, a filter to fill, and a modulator to start. So a timed send is
//! programmed early by exactly those delays, and [`start_delay`] works out by how much. Get it
//! wrong and nothing fails anywhere: the packet goes out, the device is no longer listening,
//! and the join silently never completes.
//!
//! Nothing here opens a bus. The buffer address, the delay, and the trigger word are values,
//! so they can be checked against the reference implementation with no hardware present.

use pamoja_lora::LinkSettings;

use super::register::{Register, TX_TOP_A_BASE, TX_TOP_B_BASE};

/// Where the payload for the first chain is written.
pub const TX_BUFFER_A: u16 = 0x5300;

/// Where the payload for the second chain is written.
pub const TX_BUFFER_B: u16 = 0x5500;

/// The delay the chip starts from, in its own ticks, before the parts below are taken off.
///
/// The reference keeps this as microseconds and multiplies by the thirty-two ticks each one
/// takes, which is why it is not a round number here.
pub const START_DELAY_BASE: u16 = 1500 * 32;

/// How many ticks of the concentrator counter go by in a microsecond.
pub const TICKS_PER_US: u32 = 32;

/// The value the status register reads when a chain is idle.
pub const STATUS_FREE: u8 = 0x80;

/// Which of the two transmit chains a packet goes out on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chain {
    /// The first chain.
    A,
    /// The second chain.
    B,
}

impl Chain {
    /// Where this chain keeps the payload it is about to send.
    ///
    /// # Returns
    ///
    /// The address the first payload byte is written to.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::tx::Chain;
    ///
    /// assert_eq!(Chain::A.buffer(), 0x5300);
    /// assert_eq!(Chain::B.buffer(), 0x5500);
    /// ```
    #[must_use]
    pub const fn buffer(&self) -> u16 {
        match self {
            Chain::A => TX_BUFFER_A,
            Chain::B => TX_BUFFER_B,
        }
    }

    /// The block of registers that belong to this chain.
    ///
    /// # Returns
    ///
    /// The base address the chain's registers are offsets from.
    #[must_use]
    pub const fn base(&self) -> u16 {
        match self {
            Chain::A => TX_TOP_A_BASE,
            Chain::B => TX_TOP_B_BASE,
        }
    }

    /// The control that hands this chain's buffer to the host to write.
    ///
    /// # Returns
    ///
    /// The register, set before the payload is written and cleared after it.
    #[must_use]
    pub const fn write_buffer(&self) -> Register {
        Register::new(self.base() + 7, 0, 1, false)
    }

    /// Where the chain reports what it is doing.
    ///
    /// # Returns
    ///
    /// The register [`TxStatus::of`] reads.
    #[must_use]
    pub const fn status(&self) -> Register {
        Register::new(self.base() + 17, 0, 8, true)
    }

    /// The bit that starts a send of this kind.
    ///
    /// # Arguments
    ///
    /// * `trigger` - what the send waits for.
    ///
    /// # Returns
    ///
    /// The register, which is cleared and then set to arm it.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::tx::{Chain, Trigger};
    ///
    /// // All three share a byte and differ only in which bit they are.
    /// let immediate = Chain::A.trigger(Trigger::Immediate);
    /// let gps = Chain::A.trigger(Trigger::OnGps);
    /// assert_eq!(immediate.address, gps.address);
    /// assert_eq!((immediate.offset, gps.offset), (0, 2));
    /// ```
    #[must_use]
    pub const fn trigger(&self, trigger: Trigger) -> Register {
        let offset = match trigger {
            Trigger::Immediate => 0,
            Trigger::At(_) => 1,
            Trigger::OnGps => 2,
        };
        Register::new(self.base(), offset, 1, false)
    }

    /// One of the four bytes a timed send is programmed across.
    ///
    /// The bytes run backward: the least significant sits at the highest address. So index
    /// zero is the first byte [`trigger_bytes`] hands back, and each one after it is written
    /// an address lower.
    ///
    /// # Arguments
    ///
    /// * `index` - which byte, from zero to three. Anything higher is clamped to three.
    ///
    /// # Returns
    ///
    /// The register that byte is written to.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::tx::{trigger_bytes, Chain};
    ///
    /// let bytes = trigger_bytes(0x1234_5678);
    /// // The least significant byte goes to the highest of the four addresses.
    /// assert_eq!(bytes[0], 0x78);
    /// assert!(Chain::A.timer_byte(0).address > Chain::A.timer_byte(3).address);
    /// ```
    #[must_use]
    pub const fn timer_byte(&self, index: u8) -> Register {
        let index = if index > 3 { 3 } else { index };
        Register::new(self.base() + 4 - index as u16, 0, 8, false)
    }

    /// The high byte of the delay the chain starts early by.
    ///
    /// # Returns
    ///
    /// The register the first of [`start_delay_bytes`] is written to.
    #[must_use]
    pub const fn start_delay_msb(&self) -> Register {
        Register::new(self.base() + 5, 0, 8, false)
    }

    /// The low byte of that delay.
    ///
    /// # Returns
    ///
    /// The register the second of [`start_delay_bytes`] is written to.
    #[must_use]
    pub const fn start_delay_lsb(&self) -> Register {
        Register::new(self.base() + 6, 0, 8, false)
    }

    /// Which modulator the chain runs.
    ///
    /// # Returns
    ///
    /// The register that takes [`MODULATION_LORA`] or [`MODULATION_FSK`].
    #[must_use]
    pub const fn modulation_type(&self) -> Register {
        Register::new(self.base() + 9, 0, 1, false)
    }

    /// Where the modulator takes its samples from.
    ///
    /// # Returns
    ///
    /// The register that takes [`IF_SOURCE_LORA`] or [`IF_SOURCE_FSK`]. This is what parts
    /// LoRa from an unmodulated carrier, since both name the same modulator.
    #[must_use]
    pub const fn if_source(&self) -> Register {
        Register::new(self.base() + 32, 0, 2, false)
    }

    /// The bandwidth the gain control works the transmission out against.
    ///
    /// # Returns
    ///
    /// The register that takes what [`bandwidth_value`] gives.
    #[must_use]
    pub const fn agc_bandwidth(&self) -> Register {
        Register::new(self.base() + 12, 0, 8, false)
    }

    /// The power the chain transmits at.
    ///
    /// # Returns
    ///
    /// The register that takes what [`Gain::power_value`] gives, which is the amplifier and
    /// the power index together rather than a number of decibels.
    #[must_use]
    pub const fn power(&self) -> Register {
        Register::new(self.base() + 13, 0, 8, false)
    }

    /// The digital gain applied before the front end.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn digital_gain(&self) -> Register {
        Register::new(self.base() + 34, 0, 2, false)
    }

    /// The calibrated offset of the in-phase channel.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn offset_i(&self) -> Register {
        Register::new(self.base() + 35, 0, 8, false)
    }

    /// The calibrated offset of the quadrature channel.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn offset_q(&self) -> Register {
        Register::new(self.base() + 36, 0, 8, false)
    }

    /// One byte of the carrier the chain transmits on, most significant first.
    ///
    /// # Arguments
    ///
    /// * `index` - which byte, 0 to 2. Anything higher is taken as 2.
    ///
    /// # Returns
    ///
    /// The register that byte of [`frequency_value`] is written to.
    #[must_use]
    pub const fn frequency_byte(&self, index: u8) -> Register {
        let index = if index > 2 { 2 } else { index };
        Register::new(self.base() + 37 + index as u16, 0, 8, false)
    }

    /// The high bits of the frequency deviation, which is four bits rather than eight.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn deviation_msb(&self) -> Register {
        Register::new(self.base() + 40, 0, 4, false)
    }

    /// The low bits of it.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn deviation_lsb(&self) -> Register {
        Register::new(self.base() + 41, 0, 8, false)
    }

    /// The bandwidth the modulator runs at.
    ///
    /// # Returns
    ///
    /// The register, which shares a byte with the spreading factor.
    #[must_use]
    pub const fn bandwidth(&self) -> Register {
        Register::new(self.base() + 96, 4, 4, false)
    }

    /// The spreading factor it runs at.
    ///
    /// # Returns
    ///
    /// The register, which is the factor itself rather than an index.
    #[must_use]
    pub const fn spreading_factor(&self) -> Register {
        Register::new(self.base() + 96, 0, 4, false)
    }

    /// Whether the header carries the low data rate setting.
    ///
    /// # Returns
    ///
    /// The register, which the reference leaves at zero.
    #[must_use]
    pub const fn header_control(&self) -> Register {
        Register::new(self.base() + 97, 6, 2, false)
    }

    /// Whether low data rate optimization is on.
    ///
    /// # Returns
    ///
    /// The register. A symbol longer than sixteen milliseconds needs it, and the airtime
    /// assumes it, so the two have to agree.
    #[must_use]
    pub const fn ppm_offset(&self) -> Register {
        Register::new(self.base() + 97, 4, 2, false)
    }

    /// The coding rate it runs at.
    ///
    /// # Returns
    ///
    /// The register, which takes the denominator less four.
    #[must_use]
    pub const fn coding_rate(&self) -> Register {
        Register::new(self.base() + 97, 0, 3, false)
    }

    /// Whether the fine synchronization the two lowest spreading factors need is on.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn fine_sync(&self) -> Register {
        Register::new(self.base() + 98, 7, 1, false)
    }

    /// Whether the modulator runs at all.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn modem_enable(&self) -> Register {
        Register::new(self.base() + 98, 6, 1, false)
    }

    /// Which of listening, transmitting and channel activity detection the modem does.
    ///
    /// # Returns
    ///
    /// The register, which takes [`CAD_RX_TX_TRANSMIT`] for a send.
    #[must_use]
    pub const fn cad_rx_tx(&self) -> Register {
        Register::new(self.base() + 98, 4, 2, false)
    }

    /// Whether the packet goes out without a header.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn implicit_header(&self) -> Register {
        Register::new(self.base() + 98, 1, 1, false)
    }

    /// Whether the packet carries a physical layer checksum.
    ///
    /// # Returns
    ///
    /// The register. A LoRaWAN downlink goes out without one.
    #[must_use]
    pub const fn crc_enable(&self) -> Register {
        Register::new(self.base() + 98, 0, 1, false)
    }

    /// How many bytes the packet carries.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn payload_length(&self) -> Register {
        Register::new(self.base() + 99, 0, 8, false)
    }

    /// The bit that starts the modulator once it is configured.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn modem_start(&self) -> Register {
        Register::new(self.base() + 101, 7, 1, false)
    }

    /// The low bits of the preamble length, in symbols.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn preamble_lsb(&self) -> Register {
        Register::new(self.base() + 102, 0, 8, false)
    }

    /// The high bits of it.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn preamble_msb(&self) -> Register {
        Register::new(self.base() + 103, 0, 8, false)
    }

    /// How hard the chirp is filtered, which the start delay is worked from.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn chirp_lowpass(&self) -> Register {
        Register::new(self.base() + 105, 4, 3, false)
    }

    /// Whether the chirp runs continuously.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn continuous_chirp(&self) -> Register {
        Register::new(self.base() + 105, 2, 1, false)
    }

    /// Whether the chirp sweeps downward, which a LoRaWAN downlink does.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn chirp_invert(&self) -> Register {
        Register::new(self.base() + 105, 1, 1, false)
    }

    /// Whether the chain transmits continuously rather than one packet.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn continuous(&self) -> Register {
        Register::new(self.base() + 105, 0, 1, false)
    }

    /// Where the first sync symbol goes, which says which network the packet belongs to.
    ///
    /// # Returns
    ///
    /// The register. It powers up holding the private position, as the receive side does.
    #[must_use]
    pub const fn sync_peak1(&self) -> Register {
        Register::new(self.base() + 109, 0, 5, false)
    }

    /// Where the second one goes.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn sync_peak2(&self) -> Register {
        Register::new(self.base() + 110, 0, 5, false)
    }
}

/// Which front end is wired to a chain, since each settles at its own pace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontEnd {
    /// The SX1250, the front end a current gateway pairs with this chip.
    Sx1250,
    /// The SX1255 or SX1257, which the earlier boards used.
    Sx125x,
}

/// What the transmit chain is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TxStatus {
    /// Idle, and ready to be given a packet.
    Free,
    /// Holding a packet, waiting for its trigger.
    Scheduled,
    /// On the air now.
    Emitting,
    /// A value the chip does not document, carried so a caller can report it.
    Unknown(u8),
}

impl TxStatus {
    /// Reads the transmit status register.
    ///
    /// # Arguments
    ///
    /// * `value` - what the status register answered with.
    ///
    /// # Returns
    ///
    /// What the chain is doing.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::tx::TxStatus;
    ///
    /// assert_eq!(TxStatus::of(0x80), TxStatus::Free);
    /// assert_eq!(TxStatus::of(0x91), TxStatus::Scheduled);
    /// assert_eq!(TxStatus::of(0x60), TxStatus::Emitting);
    /// ```
    #[must_use]
    pub const fn of(value: u8) -> TxStatus {
        match value {
            STATUS_FREE => TxStatus::Free,
            0x91 | 0x92 => TxStatus::Scheduled,
            0x30 | 0x50 | 0x60 | 0x70 => TxStatus::Emitting,
            other => TxStatus::Unknown(other),
        }
    }

    /// Whether a chain in this state will take another packet.
    ///
    /// # Returns
    ///
    /// Whether it is idle. A chain that is scheduled or emitting keeps what it holds, so
    /// writing to it now would lose one of the two packets.
    #[must_use]
    pub const fn is_free(&self) -> bool {
        matches!(self, TxStatus::Free)
    }
}

/// When a packet goes out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    /// As soon as the chip is told.
    Immediate,
    /// When the concentrator counter reaches a value, in microseconds.
    At(u32),
    /// On the next pulse from a GPS receiver.
    OnGps,
}

/// A step in sending a packet.
///
/// The order is carried as data, so a driver walks it against a bus and a test walks it
/// against nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Send {
    /// Open the transmit buffer for writing.
    OpenBuffer(Register),
    /// Write the payload, starting here. For frequency shift keying the length goes first.
    WritePayload(u16),
    /// Close the buffer again.
    CloseBuffer(Register),
    /// Set the delay the chip starts early by, as a pair of bytes.
    SetStartDelay(u16),
    /// Program the counter value a timed send goes out at.
    SetTimerTrigger(u32),
    /// Reset a trigger, which the chip needs before it will take a new one.
    ResetTrigger(Register),
    /// Arm it.
    ArmTrigger(Register),
}

/// How far ahead of its window a chain has to be started.
///
/// The chip does not radiate the moment it is triggered. The front end settles, the filter
/// fills, and the modulator starts, so a send meant for a given moment is programmed by that
/// much earlier.
///
/// # Arguments
///
/// * `front_end` - which front end is wired to the chain.
/// * `bandwidth_hz` - the bandwidth the packet goes out at.
/// * `chirp_lowpass` - the filter setting the chain is configured with.
///
/// # Returns
///
/// The delay in the chip's own ticks, or `None` for a bandwidth no front end supports.
/// Anything that is not LoRa needs no delay at all, so a caller sends those with zero.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::tx::{start_delay, FrontEnd};
///
/// // The usual case: an SX1250 at 125 kHz.
/// let delay = start_delay(FrontEnd::Sx1250, 125_000, 6).expect("a supported bandwidth");
/// assert!(delay < 1500 * 32, "the chip starts early, never late");
///
/// // A bandwidth no front end covers has no answer.
/// assert_eq!(start_delay(FrontEnd::Sx1250, 62_500, 6), None);
/// ```
#[must_use]
pub fn start_delay(front_end: FrontEnd, bandwidth_hz: u32, chirp_lowpass: u8) -> Option<u16> {
    let radio = match (front_end, bandwidth_hz) {
        (FrontEnd::Sx1250, 125_000) => 19,
        (FrontEnd::Sx1250, 250_000) => 24,
        (FrontEnd::Sx1250, 500_000) => 21,
        // The earlier front ends take a fixed time, and only 250 kHz adds to it.
        (FrontEnd::Sx125x, 125_000 | 500_000) => 3 * 32 + 4,
        (FrontEnd::Sx125x, 250_000) => 3 * 32 + 4 + 6,
        _ => return None,
    };

    let filter = ((1u32 << chirp_lowpass) - 1) * 1_000_000 / bandwidth_hz;
    let modem = 8 * (32_000_000 / (32 * bandwidth_hz));

    Some(START_DELAY_BASE.wrapping_sub((radio + filter + modem) as u16))
}

/// The counter value a timed send is programmed at.
///
/// The counter runs in the chip's own ticks rather than microseconds, and the send is moved
/// earlier by the start delay so the packet is on the air at the moment asked for.
///
/// # Arguments
///
/// * `at_us` - when the packet should be heard, in microseconds of the counter.
/// * `start_delay` - what [`start_delay`] worked out.
///
/// # Returns
///
/// The value to write across the four trigger bytes.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::tx::trigger_value;
///
/// // A send a whole second out, brought forward by the delay.
/// assert_eq!(trigger_value(1_000_000, 48_000), 1_000_000 * 32 - 48_000);
/// ```
#[must_use]
pub const fn trigger_value(at_us: u32, start_delay: u16) -> u32 {
    at_us
        .wrapping_mul(TICKS_PER_US)
        .wrapping_sub(start_delay as u32)
}

/// Splits a trigger value into the four bytes the chip takes.
///
/// # Arguments
///
/// * `value` - what [`trigger_value`] worked out.
///
/// # Returns
///
/// The bytes, least significant first, which is the order the four registers are written in.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::tx::trigger_bytes;
///
/// assert_eq!(trigger_bytes(0x1234_5678), [0x78, 0x56, 0x34, 0x12]);
/// ```
#[must_use]
pub const fn trigger_bytes(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

/// Splits a start delay into the two bytes the chip takes.
///
/// # Arguments
///
/// * `delay` - what [`start_delay`] worked out.
///
/// # Returns
///
/// The bytes, most significant first, which is the order this one register takes them in.
/// It is the opposite of the trigger, which is easy to get backwards.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::tx::start_delay_bytes;
///
/// assert_eq!(start_delay_bytes(0x1234), [0x12, 0x34]);
/// ```
#[must_use]
pub const fn start_delay_bytes(delay: u16) -> [u8; 2] {
    delay.to_be_bytes()
}

/// The order a packet is loaded and sent in.
///
/// A timed send carries one step the other two do not, since only it programs the counter
/// value to go out at.
///
/// # Arguments
///
/// * `chain` - which transmit chain is sending.
/// * `trigger` - what the send waits for.
/// * `start_delay` - what [`start_delay`] worked out for this packet.
///
/// # Returns
///
/// The steps, in the order the chip takes them.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::tx::{steps, Chain, Send, Trigger};
///
/// // A send that goes out now: delay, buffer, trigger.
/// assert_eq!(steps(Chain::A, Trigger::Immediate, 1_000).count(), 6);
///
/// // A timed one programs the counter as well.
/// assert_eq!(steps(Chain::A, Trigger::At(1_000_000), 1_000).count(), 7);
///
/// let mut order = steps(Chain::A, Trigger::Immediate, 1_000);
/// assert!(matches!(order.next(), Some(Send::SetStartDelay(1_000))));
/// assert!(matches!(order.next(), Some(Send::OpenBuffer(_))));
/// assert!(matches!(order.next(), Some(Send::WritePayload(0x5300))));
/// ```
pub fn steps(chain: Chain, trigger: Trigger, start_delay: u16) -> impl Iterator<Item = Send> {
    let arm = chain.trigger(trigger);
    let timed = match trigger {
        Trigger::At(at_us) => Some(Send::SetTimerTrigger(trigger_value(at_us, start_delay))),
        Trigger::Immediate | Trigger::OnGps => None,
    };
    [
        Some(Send::SetStartDelay(start_delay)),
        Some(Send::OpenBuffer(chain.write_buffer())),
        Some(Send::WritePayload(chain.buffer())),
        Some(Send::CloseBuffer(chain.write_buffer())),
        timed,
        Some(Send::ResetTrigger(arm)),
        Some(Send::ArmTrigger(arm)),
    ]
    .into_iter()
    .flatten()
}

/// What the modulation register takes for LoRa, which is also what it takes for an
/// unmodulated carrier. [`Chain::if_source`] is what parts the two.
pub const MODULATION_LORA: u8 = 0x00;

/// What it takes for frequency shift keying.
pub const MODULATION_FSK: u8 = 0x01;

/// What the sample source takes for LoRa.
pub const IF_SOURCE_LORA: u8 = 0x01;

/// What it takes for frequency shift keying.
pub const IF_SOURCE_FSK: u8 = 0x02;

/// What the modem mode takes to transmit.
pub const CAD_RX_TX_TRANSMIT: u8 = 0x02;

/// The chirp filtering a spreading factor below ten is sent with.
pub const CHIRP_LOWPASS_FAST: u8 = 6;

/// And what the slower ones are sent with.
pub const CHIRP_LOWPASS_SLOW: u8 = 7;

/// The sync symbol positions a public network transmits with.
pub const PUBLIC_PEAKS: (u8, u8) = (6, 8);

/// The ones a private network uses, which is what the chain powers up holding.
pub const PRIVATE_PEAKS: (u8, u8) = (2, 4);

/// Turns a carrier into the value the transmit chain is tuned with.
///
/// This is the concentrator's own scaling and it is not the one a front end takes: the
/// divisor is the same 32 MHz but the shift is eighteen rather than twenty five, so using
/// [`sx1250::frequency_value`](super::sx1250::frequency_value) here would be wrong by a
/// factor of 128.
///
/// # Arguments
///
/// * `hertz` - the carrier.
///
/// # Returns
///
/// The value, of which three bytes are written most significant first.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::tx::frequency_value;
///
/// // Worked from the ratio rather than from the function: 868100000 * 2^18 / 32000000.
/// assert_eq!(frequency_value(868_100_000), 7_111_475);
/// ```
#[must_use]
pub const fn frequency_value(hertz: u32) -> u32 {
    ((hertz as u64 * (1 << 18)) / 32_000_000) as u32
}

/// Turns a bandwidth into the value the modulator takes.
///
/// The values do not start at zero and they are not hertz, so a plausible guess is wrong by
/// four.
///
/// # Arguments
///
/// * `bandwidth_hz` - the bandwidth.
///
/// # Returns
///
/// The value, or `None` for a bandwidth this chip does not transmit at.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::tx::bandwidth_value;
///
/// assert_eq!(bandwidth_value(125_000), Some(4));
/// assert_eq!(bandwidth_value(250_000), Some(5));
/// assert_eq!(bandwidth_value(500_000), Some(6));
/// assert_eq!(bandwidth_value(62_500), None);
/// ```
#[must_use]
pub const fn bandwidth_value(bandwidth_hz: u32) -> Option<u8> {
    match bandwidth_hz {
        125_000 => Some(4),
        250_000 => Some(5),
        500_000 => Some(6),
        _ => None,
    }
}

/// One entry of the table that turns a wanted power into what the chain is set to.
///
/// A concentrator is not told a number of decibels. It is told an amplifier setting and an
/// index into the front end's own power steps, and which pair gives which radiated power is a
/// property of the board: its amplifier, its filters, and its losses. So the table belongs to
/// whoever built the gateway, and the one here is the reference design's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gain {
    /// What this entry radiates, in dBm.
    pub radiated_dbm: i8,
    /// The external amplifier setting. Anything above zero switches it on.
    pub amplifier: u8,
    /// The index into the front end's power steps.
    pub power_index: u8,
    /// The digital gain applied ahead of the front end.
    pub digital_gain: u8,
    /// The calibrated offset of the in-phase channel.
    pub offset_i: i8,
    /// The calibrated offset of the quadrature channel.
    pub offset_q: i8,
}

impl Gain {
    /// What the power register takes for this entry.
    ///
    /// # Returns
    ///
    /// The amplifier bit above the power index, which is how an SX1250 board is set.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::tx::DEFAULT_GAINS;
    ///
    /// // The lowest entry runs the amplifier off, so only the index is carried.
    /// assert_eq!(DEFAULT_GAINS[0].power_value(), 15);
    ///
    /// // The highest runs it on, which is the bit above.
    /// assert_eq!(DEFAULT_GAINS[15].power_value(), 0x40 | 14);
    /// ```
    #[must_use]
    pub const fn power_value(&self) -> u8 {
        let amplifier = if self.amplifier > 0 { 1 } else { 0 };
        (amplifier << 6) | (self.power_index & 0x3f)
    }
}

/// The table the reference design carries for an SX1250 board on the European band.
///
/// A gateway with different hardware has a different table, which is why a configuration can
/// name its own. Nothing here is a law of the chip.
pub const DEFAULT_GAINS: [Gain; 16] = [
    gain(12, 0, 15),
    gain(13, 0, 16),
    gain(14, 0, 17),
    gain(15, 0, 19),
    gain(16, 0, 20),
    gain(17, 0, 22),
    gain(18, 1, 1),
    gain(19, 1, 2),
    gain(20, 1, 3),
    gain(21, 1, 4),
    gain(22, 1, 5),
    gain(23, 1, 6),
    gain(24, 1, 7),
    gain(25, 1, 9),
    gain(26, 1, 11),
    gain(27, 1, 14),
];

/// One entry of the reference table, whose digital gain and offsets are all zero.
const fn gain(radiated_dbm: i8, amplifier: u8, power_index: u8) -> Gain {
    Gain {
        radiated_dbm,
        amplifier,
        power_index,
        digital_gain: 0,
        offset_i: 0,
        offset_q: 0,
    }
}

/// Picks the entry to transmit a wanted power at.
///
/// The strongest entry that does not exceed what was asked for, so a gateway told to
/// transmit at more than its table reaches transmits at the most it has rather than refusing
/// or exceeding it. A request below the whole table takes the weakest entry.
///
/// # Arguments
///
/// * `table` - the board's table, strongest last.
/// * `wanted_dbm` - the power asked for.
///
/// # Returns
///
/// The entry, or `None` for an empty table.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::tx::{gain_for, DEFAULT_GAINS};
///
/// // Exactly on an entry, and between two.
/// assert_eq!(gain_for(&DEFAULT_GAINS, 20).map(|g| g.radiated_dbm), Some(20));
/// assert_eq!(gain_for(&DEFAULT_GAINS, 16).map(|g| g.radiated_dbm), Some(16));
///
/// // Past the top of the table, and below the bottom of it.
/// assert_eq!(gain_for(&DEFAULT_GAINS, 30).map(|g| g.radiated_dbm), Some(27));
/// assert_eq!(gain_for(&DEFAULT_GAINS, 2).map(|g| g.radiated_dbm), Some(12));
/// ```
#[must_use]
pub fn gain_for(table: &[Gain], wanted_dbm: i8) -> Option<Gain> {
    table
        .iter()
        .rev()
        .find(|entry| entry.radiated_dbm <= wanted_dbm)
        .or_else(|| table.first())
        .copied()
}

/// Why a packet cannot be described to the chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransmitError {
    /// A bandwidth the transmit chain does not run at.
    Bandwidth {
        /// The bandwidth asked for, in hertz.
        hertz: u32,
    },
    /// A packet longer than the length register can describe.
    PayloadTooLong {
        /// How many bytes were offered.
        bytes: usize,
    },
}

impl core::fmt::Display for TransmitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TransmitError::Bandwidth { hertz } => {
                write!(f, "a chain does not transmit at {hertz} Hz")
            }
            TransmitError::PayloadTooLong { bytes } => write!(
                f,
                "a packet of {bytes} bytes is longer than the length register describes"
            ),
        }
    }
}

impl core::error::Error for TransmitError {}

/// A packet to put on the air, and how.
///
/// Everything the chain has to be told before a payload means anything. Built from the link
/// settings the rest of pamoja works in, so the airtime, the duty cycle and the transmission
/// itself all come from one description rather than three.
#[derive(Clone, Copy, Debug)]
pub struct Transmit<'a> {
    /// The carrier to transmit on.
    pub frequency_hz: u32,
    /// The spreading factor, bandwidth, coding rate, preamble, header and checksum.
    pub link: LinkSettings,
    /// What the chain is set to, from the board's table.
    pub gain: Gain,
    /// Whether the chirp sweeps the other way, which every LoRaWAN downlink does.
    pub invert_polarity: bool,
    /// Whether the network is a public one, which picks the sync word.
    pub public: bool,
    /// The packet.
    pub payload: &'a [u8],
}

/// Everything a chain is told before a packet is loaded.
///
/// The order is the reference order. A modulator is configured and then started, and the
/// payload and the trigger that [`steps`] carries come after all of it.
///
/// # Arguments
///
/// * `chain` - the chain that sends it.
/// * `transmit` - what to send and how.
///
/// # Returns
///
/// The writes, in order.
///
/// # Errors
///
/// Returns [`TransmitError::Bandwidth`] for a bandwidth the chain does not run at, and
/// [`TransmitError::PayloadTooLong`] for a packet the length register cannot describe.
pub fn settings(
    chain: Chain,
    transmit: &Transmit<'_>,
) -> Result<[(Register, u8); 31], TransmitError> {
    let link = &transmit.link;
    let bandwidth = bandwidth_value(link.bandwidth_hz()).ok_or(TransmitError::Bandwidth {
        hertz: link.bandwidth_hz(),
    })?;
    let length =
        u8::try_from(transmit.payload.len()).map_err(|_| TransmitError::PayloadTooLong {
            bytes: transmit.payload.len(),
        })?;
    let carrier = frequency_value(transmit.frequency_hz);

    // The deviation is half the bandwidth in the chip's own frequency units, and its high
    // half is four bits wide, which every supported bandwidth fits inside.
    let deviation = frequency_value(link.bandwidth_hz() / 2);
    let preamble = link.preamble_symbols();
    let spreading_factor = link.spreading_factor();

    let (peak1, peak2) = if transmit.public {
        PUBLIC_PEAKS
    } else {
        PRIVATE_PEAKS
    };

    // The two lowest spreading factors have no public form and need the fine synchronization
    // the others do not.
    let low = spreading_factor <= 6;
    let (peak1, peak2) = if low { PRIVATE_PEAKS } else { (peak1, peak2) };

    let lowpass = if spreading_factor < 10 {
        CHIRP_LOWPASS_FAST
    } else {
        CHIRP_LOWPASS_SLOW
    };

    Ok([
        (chain.modulation_type(), MODULATION_LORA),
        (chain.if_source(), IF_SOURCE_LORA),
        (chain.offset_i(), transmit.gain.offset_i as u8),
        (chain.offset_q(), transmit.gain.offset_q as u8),
        (chain.power(), transmit.gain.power_value()),
        (chain.digital_gain(), transmit.gain.digital_gain),
        (chain.frequency_byte(0), ((carrier >> 16) & 0xff) as u8),
        (chain.frequency_byte(1), ((carrier >> 8) & 0xff) as u8),
        (chain.frequency_byte(2), (carrier & 0xff) as u8),
        (chain.agc_bandwidth(), bandwidth),
        (chain.deviation_msb(), ((deviation >> 8) & 0xff) as u8),
        (chain.deviation_lsb(), (deviation & 0xff) as u8),
        (chain.bandwidth(), bandwidth),
        (chain.preamble_msb(), ((preamble >> 8) & 0xff) as u8),
        (chain.preamble_lsb(), (preamble & 0xff) as u8),
        (chain.spreading_factor(), spreading_factor),
        (chain.chirp_lowpass(), lowpass),
        (
            chain.coding_rate(),
            link.coding_rate_denominator().saturating_sub(4),
        ),
        (chain.modem_enable(), 1),
        (chain.cad_rx_tx(), CAD_RX_TX_TRANSMIT),
        (chain.modem_start(), 1),
        (chain.continuous(), 0),
        (chain.chirp_invert(), u8::from(transmit.invert_polarity)),
        (chain.implicit_header(), u8::from(!link.explicit_header())),
        (chain.crc_enable(), u8::from(link.crc())),
        (chain.sync_peak1(), peak1),
        (chain.sync_peak2(), peak2),
        (chain.fine_sync(), u8::from(low)),
        (chain.payload_length(), length),
        (chain.header_control(), 0),
        (
            chain.ppm_offset(),
            u8::from(link.low_data_rate_optimization()),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a chain is told for one packet, so a test can look a register up by name.
    fn settings_for(link: LinkSettings, public: bool, payload: &[u8]) -> [(Register, u8); 31] {
        let request = Transmit {
            frequency_hz: 868_100_000,
            link,
            gain: DEFAULT_GAINS[8],
            invert_polarity: true,
            public,
            payload,
        };
        settings(Chain::A, &request).expect("a packet the chain can describe")
    }

    fn value_of(written: &[(Register, u8)], want: Register) -> u8 {
        written
            .iter()
            .find_map(|(named, value)| (*named == want).then_some(*value))
            .expect("the register is written")
    }

    #[test]
    fn a_packet_is_described_in_the_units_the_chip_counts_in() {
        let link = LinkSettings::new(7, 125_000)
            .with_coding_rate(5)
            .with_preamble(8);
        let written = settings_for(link, true, &[0; 12]);
        let chain = Chain::A;

        // None of these are the numbers a person would write. The bandwidth is 4 rather than
        // 125000 or an index from zero, the coding rate is the denominator less four, and the
        // spreading factor is the only one that is itself.
        assert_eq!(value_of(&written, chain.bandwidth()), 4);
        assert_eq!(value_of(&written, chain.agc_bandwidth()), 4);
        assert_eq!(value_of(&written, chain.coding_rate()), 1);
        assert_eq!(value_of(&written, chain.spreading_factor()), 7);

        // The carrier is three bytes of the chip's own scaling, most significant first.
        assert_eq!(value_of(&written, chain.frequency_byte(0)), 0x6c);
        assert_eq!(value_of(&written, chain.frequency_byte(1)), 0x83);
        assert_eq!(value_of(&written, chain.frequency_byte(2)), 0x33);

        // The deviation is half the bandwidth in those same units, and its high half is four
        // bits wide, which 512 fits inside.
        assert_eq!(value_of(&written, chain.deviation_msb()), 2);
        assert_eq!(value_of(&written, chain.deviation_lsb()), 0);

        assert_eq!(value_of(&written, chain.payload_length()), 12);
        assert_eq!(value_of(&written, chain.preamble_lsb()), 8);
        assert_eq!(value_of(&written, chain.preamble_msb()), 0);
        assert_eq!(value_of(&written, chain.chirp_invert()), 1);
    }

    #[test]
    fn the_modulator_is_named_by_its_sample_source_rather_than_its_type() {
        let link = LinkSettings::new(9, 125_000);
        let written = settings_for(link, true, &[0; 4]);

        // Writing one into the modulation type would select frequency shift keying. LoRa and
        // an unmodulated carrier share it, and the sample source is what parts them.
        assert_eq!(
            value_of(&written, Chain::A.modulation_type()),
            MODULATION_LORA
        );
        assert_eq!(value_of(&written, Chain::A.if_source()), IF_SOURCE_LORA);
        assert_ne!(MODULATION_LORA, MODULATION_FSK);
    }

    #[test]
    fn a_transmission_says_which_network_it_belongs_to() {
        let link = LinkSettings::new(7, 125_000);
        let public = settings_for(link, true, &[0; 4]);
        let private = settings_for(link, false, &[0; 4]);

        assert_eq!(value_of(&public, Chain::A.sync_peak1()), 6);
        assert_eq!(value_of(&public, Chain::A.sync_peak2()), 8);
        assert_eq!(value_of(&private, Chain::A.sync_peak1()), 2);
        assert_eq!(value_of(&private, Chain::A.sync_peak2()), 4);

        // The two lowest spreading factors have no public form, and take the fine
        // synchronization the others do not.
        let low = settings_for(LinkSettings::new(5, 125_000), true, &[0; 4]);
        assert_eq!(value_of(&low, Chain::A.sync_peak1()), 2);
        assert_eq!(value_of(&low, Chain::A.sync_peak2()), 4);
        assert_eq!(value_of(&low, Chain::A.fine_sync()), 1);
        assert_eq!(value_of(&public, Chain::A.fine_sync()), 0);
    }

    #[test]
    fn the_slow_spreading_factors_are_sent_the_way_their_airtime_assumes() {
        let fast = settings_for(LinkSettings::new(7, 125_000), true, &[0; 4]);
        let slow = settings_for(LinkSettings::new(11, 125_000), true, &[0; 4]);

        // Low data rate optimization is on exactly when a symbol runs past sixteen
        // milliseconds, which is what the airtime already assumes.
        assert_eq!(value_of(&fast, Chain::A.ppm_offset()), 0);
        assert_eq!(value_of(&slow, Chain::A.ppm_offset()), 1);

        // And the chirp is filtered harder from SF10 upward.
        assert_eq!(
            value_of(&fast, Chain::A.chirp_lowpass()),
            CHIRP_LOWPASS_FAST
        );
        assert_eq!(
            value_of(&slow, Chain::A.chirp_lowpass()),
            CHIRP_LOWPASS_SLOW
        );
    }

    #[test]
    fn a_packet_the_chain_cannot_describe_is_refused() {
        let narrow = Transmit {
            frequency_hz: 868_100_000,
            link: LinkSettings::new(7, 62_500),
            gain: DEFAULT_GAINS[0],
            invert_polarity: false,
            public: true,
            payload: &[0; 4],
        };
        assert_eq!(
            settings(Chain::A, &narrow),
            Err(TransmitError::Bandwidth { hertz: 62_500 })
        );

        // A packet longer than the length register describes is refused rather than sent
        // with a length that is not its own.
        let long = Transmit {
            frequency_hz: 868_100_000,
            link: LinkSettings::new(7, 125_000),
            gain: DEFAULT_GAINS[0],
            invert_polarity: false,
            public: true,
            payload: &[0; 300],
        };
        assert_eq!(
            settings(Chain::A, &long),
            Err(TransmitError::PayloadTooLong { bytes: 300 })
        );
    }

    #[test]
    fn each_chain_has_its_own_buffer_and_registers() {
        assert_eq!(Chain::A.buffer(), 0x5300);
        assert_eq!(Chain::B.buffer(), 0x5500);
        assert_ne!(Chain::A.buffer(), Chain::B.buffer());

        // The buffers sit inside the blocks that belong to each chain.
        assert!(Chain::A.buffer() > Chain::A.base());
        assert!(Chain::B.buffer() > Chain::B.base());
        assert!(Chain::A.buffer() < Chain::B.base());
    }

    #[test]
    fn the_status_values_are_the_ones_the_chip_reports() {
        assert_eq!(TxStatus::of(0x80), TxStatus::Free);
        assert!(TxStatus::of(0x80).is_free());

        for scheduled in [0x91, 0x92] {
            assert_eq!(TxStatus::of(scheduled), TxStatus::Scheduled);
            assert!(!TxStatus::of(scheduled).is_free());
        }
        for emitting in [0x30, 0x50, 0x60, 0x70] {
            assert_eq!(TxStatus::of(emitting), TxStatus::Emitting);
            assert!(!TxStatus::of(emitting).is_free());
        }

        // Anything else is carried rather than mistaken for idle.
        assert_eq!(TxStatus::of(0x00), TxStatus::Unknown(0x00));
        assert!(!TxStatus::of(0x00).is_free());
    }

    #[test]
    fn the_chip_always_starts_early_and_never_late() {
        // Whatever the settings, the delay comes off the base rather than adding to it.
        for bandwidth in [125_000, 250_000, 500_000] {
            for lowpass in 0..=8 {
                for front_end in [FrontEnd::Sx1250, FrontEnd::Sx125x] {
                    let delay = start_delay(front_end, bandwidth, lowpass)
                        .expect("these are supported bandwidths");
                    assert!(
                        delay < START_DELAY_BASE,
                        "{front_end:?} {bandwidth} {lowpass}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_front_ends_settle_at_their_own_pace() {
        // The two differ, which is the whole reason the front end is a parameter.
        let modern = start_delay(FrontEnd::Sx1250, 125_000, 6).expect("supported");
        let earlier = start_delay(FrontEnd::Sx125x, 125_000, 6).expect("supported");
        assert_ne!(modern, earlier);

        // The older part takes longer to settle, so it starts earlier still.
        assert!(earlier < modern);
    }

    #[test]
    fn a_bandwidth_no_front_end_covers_has_no_answer() {
        for bandwidth in [7_800, 62_500, 1_000_000] {
            assert_eq!(start_delay(FrontEnd::Sx1250, bandwidth, 6), None);
            assert_eq!(start_delay(FrontEnd::Sx125x, bandwidth, 6), None);
        }
    }

    #[test]
    fn a_wider_filter_takes_longer_to_fill() {
        // The filter term grows with the setting, so the chip has to start earlier.
        let narrow = start_delay(FrontEnd::Sx1250, 125_000, 2).expect("supported");
        let wide = start_delay(FrontEnd::Sx1250, 125_000, 8).expect("supported");
        assert!(wide < narrow, "a wider filter means an earlier start");
    }

    #[test]
    fn a_timed_send_is_programmed_earlier_than_the_moment_asked_for() {
        // The window is counted in microseconds and the chip in ticks.
        let delay = 48_000u16;
        let value = trigger_value(1_000_000, delay);
        assert_eq!(value, 1_000_000 * TICKS_PER_US - u32::from(delay));

        // Which is earlier than the plain conversion would be.
        assert!(value < 1_000_000 * TICKS_PER_US);
    }

    #[test]
    fn the_two_words_go_out_in_opposite_orders() {
        // The trigger is least significant first and the delay most significant first, which
        // is the kind of thing that reads correctly and transmits at the wrong moment.
        assert_eq!(trigger_bytes(0x1234_5678), [0x78, 0x56, 0x34, 0x12]);
        assert_eq!(start_delay_bytes(0x1234), [0x12, 0x34]);

        // Round trips, so neither is quietly reversed.
        let value = 0xdead_beefu32;
        assert_eq!(u32::from_le_bytes(trigger_bytes(value)), value);
        let delay = 0xbeefu16;
        assert_eq!(u16::from_be_bytes(start_delay_bytes(delay)), delay);
    }

    #[test]
    fn a_trigger_that_wraps_the_counter_still_reads_back() {
        // The counter is thirty-two bits and wraps about every thirty-six minutes, so a
        // send near the wrap must not panic or saturate.
        let near_wrap = trigger_value(u32::MAX / TICKS_PER_US, 48_000);
        assert_eq!(
            near_wrap,
            (u32::MAX / TICKS_PER_US)
                .wrapping_mul(TICKS_PER_US)
                .wrapping_sub(48_000)
        );

        // And a moment so early that the delay takes it below zero wraps rather than panics.
        let underflow = trigger_value(0, 48_000);
        assert_eq!(underflow, 0u32.wrapping_sub(48_000));
    }
}
