//! Where the concentrator keeps its registers.
//!
//! The chip is laid out in blocks, each with a base address: the receive buffer, the two
//! transmit chains, the common block, the front end, the two microcontrollers, the timestamp
//! counter, and the one-time programmable memory that carries the part number.
//!
//! Most registers are narrower than the byte they live in, so a register is an address plus
//! the bits it occupies inside it. Writing one means reading that byte, replacing those bits,
//! and writing it back, which is why [`Register`] carries the offset and the width rather
//! than just an address.

use super::spi;

/// The paged external memory, where the microcontroller firmware is written.
pub const EXTERNAL_MEMORY_BASE: u16 = 0x0000;

/// The buffer received packets are read out of.
pub const RX_BUFFER_BASE: u16 = 0x4000;

/// The first transmit chain.
pub const TX_TOP_A_BASE: u16 = 0x5200;

/// The second transmit chain.
pub const TX_TOP_B_BASE: u16 = 0x5400;

/// The block holding the version, the page selector, and the chip-wide controls.
pub const COMMON_BASE: u16 = 0x5600;

/// The general purpose pins.
pub const GPIO_BASE: u16 = 0x5640;

/// The built-in memory self test.
pub const MBIST_BASE: u16 = 0x56c0;

/// The radio front end.
pub const RADIO_FE_BASE: u16 = 0x5700;

/// The automatic gain control microcontroller.
pub const AGC_MCU_BASE: u16 = 0x5780;

/// The clock controls.
pub const CLK_CTRL_BASE: u16 = 0x57c0;

/// The receive chain.
pub const RX_TOP_BASE: u16 = 0x5800;

/// The single-spreading-factor receiver and the FSK receiver.
pub const RX_TOP_LORA_SERVICE_FSK_BASE: u16 = 0x5b00;

/// The capture memory.
pub const CAPTURE_RAM_BASE: u16 = 0x6000;

/// The arbiter microcontroller, which shares the radios between the receivers.
pub const ARB_MCU_BASE: u16 = 0x6080;

/// The counter a received packet is timestamped against.
pub const TIMESTAMP_BASE: u16 = 0x6100;

/// The one-time programmable memory, which carries the part number.
pub const OTP_BASE: u16 = 0x6180;

/// A register, which is some of the bits of a byte at an address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Register {
    /// The byte it lives in.
    pub address: u16,
    /// Which bit of that byte it starts at.
    pub offset: u8,
    /// How many bits it spans.
    pub width: u8,
    /// Whether the chip refuses to be written here.
    pub read_only: bool,
}

impl Register {
    /// Names a register by where it is and how wide it is.
    ///
    /// # Arguments
    ///
    /// * `address` - the byte it lives in.
    /// * `offset` - which bit it starts at.
    /// * `width` - how many bits it spans.
    /// * `read_only` - whether the chip refuses to be written here.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn new(address: u16, offset: u8, width: u8, read_only: bool) -> Register {
        Register {
            address,
            offset,
            width,
            read_only,
        }
    }

    /// Whether this register is the whole byte rather than a field inside one.
    ///
    /// A whole byte is written directly; anything narrower has to be read first, so the bits
    /// beside it survive the write.
    ///
    /// # Returns
    ///
    /// Whether a write can skip reading the byte first.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::register::{COMMON_VERSION, AGC_MCU_HOST_PROG};
    ///
    /// assert!(COMMON_VERSION.is_whole_byte());
    /// assert!(!AGC_MCU_HOST_PROG.is_whole_byte());
    /// ```
    #[must_use]
    pub const fn is_whole_byte(&self) -> bool {
        self.offset == 0 && self.width == 8
    }

    /// Reads this register out of the byte it shares.
    ///
    /// # Arguments
    ///
    /// * `byte` - the byte read from [`address`](Register::address).
    ///
    /// # Returns
    ///
    /// The value, shifted down to where it reads as a number.
    #[must_use]
    pub const fn decode(&self, byte: u8) -> u8 {
        spi::field(byte, self.offset, self.width)
    }

    /// Puts a value into this register, leaving the rest of the byte as it was.
    ///
    /// # Arguments
    ///
    /// * `byte` - the byte as it reads now.
    /// * `value` - what to put in this register.
    ///
    /// # Returns
    ///
    /// The byte to write back.
    #[must_use]
    pub const fn encode(&self, byte: u8, value: u8) -> u8 {
        spi::with_field(byte, self.offset, self.width, value)
    }
}

/// Which of the four memory pages the external memory window shows.
pub const COMMON_PAGE: Register = Register::new(COMMON_BASE, 0, 2, false);

/// The chip version, which a working concentrator answers with
/// [`EXPECTED_VERSION`](super::chip::EXPECTED_VERSION).
pub const COMMON_VERSION: Register = Register::new(COMMON_BASE + 6, 0, 8, true);

/// Holds the gain control microcontroller in reset.
pub const AGC_MCU_CLEAR: Register = Register::new(AGC_MCU_BASE, 2, 1, false);

/// Gives the host the gain control microcontroller memory, so firmware can be written.
pub const AGC_MCU_HOST_PROG: Register = Register::new(AGC_MCU_BASE, 1, 1, false);

/// Runs the clock of the gain control microcontroller.
pub const AGC_MCU_CLK_EN: Register = Register::new(AGC_MCU_BASE, 4, 1, false);

/// Set by the chip when the firmware it holds does not check out.
pub const AGC_MCU_PARITY_ERROR: Register = Register::new(AGC_MCU_BASE, 0, 1, true);

/// Holds the arbiter microcontroller in reset.
pub const ARB_MCU_CLEAR: Register = Register::new(ARB_MCU_BASE, 2, 1, false);

/// Gives the host the arbiter microcontroller memory.
pub const ARB_MCU_HOST_PROG: Register = Register::new(ARB_MCU_BASE, 1, 1, false);

/// Runs the clock of the arbiter microcontroller.
pub const ARB_MCU_CLK_EN: Register = Register::new(ARB_MCU_BASE, 5, 1, false);

/// Set by the chip when the arbiter firmware does not check out.
pub const ARB_MCU_PARITY_ERROR: Register = Register::new(ARB_MCU_BASE, 0, 1, true);

/// Runs the radios from the host clock rather than their own.
///
/// A front end is reset with this cleared, so the concentrator is still clocked while the
/// radio it drives is held down.
pub const COMMON_CLK32_RIF_CTRL: Register = Register::new(COMMON_BASE + 1, 4, 1, false);

/// Whether the host drives the front ends directly rather than the gain control
/// microcontroller.
pub const COMMON_HOST_RADIO_CTRL: Register = Register::new(COMMON_BASE + 1, 3, 1, false);

/// Powers the front end on the first chain.
pub const RF_EN_A_RADIO_EN: Register = Register::new(AGC_MCU_BASE + 3, 2, 1, false);

/// Holds the front end on the first chain in reset.
pub const RF_EN_A_RADIO_RST: Register = Register::new(AGC_MCU_BASE + 3, 3, 1, false);

/// Powers the front end on the second chain.
pub const RF_EN_B_RADIO_EN: Register = Register::new(AGC_MCU_BASE + 4, 2, 1, false);

/// Holds the front end on the second chain in reset.
pub const RF_EN_B_RADIO_RST: Register = Register::new(AGC_MCU_BASE + 4, 3, 1, false);

/// Which byte of the one-time programmable memory the read register answers with.
pub const OTP_BYTE_ADDR: Register = Register::new(OTP_BASE, 0, 8, false);

/// The byte at the address [`OTP_BYTE_ADDR`] selected.
pub const OTP_RD_DATA: Register = Register::new(OTP_BASE + 1, 0, 8, true);

/// Whether the one-time programmable memory is ready to be read.
pub const OTP_FSM_READY: Register = Register::new(OTP_BASE + 2, 0, 1, true);

/// What the one-time programmable memory made of its own checksum.
pub const OTP_CHECKSUM_STATUS: Register = Register::new(OTP_BASE + 2, 4, 4, true);

/// How many bytes the receive buffer is holding, the high bits of the count.
///
/// Read this pair twice and take the larger answer. A read of the two bytes can report a
/// count below the true one, and the reference guards against it the same way.
pub const RX_BUFFER_NB_BYTES_MSB: Register = Register::new(RX_TOP_BASE + 200, 0, 5, true);

/// The low bits of that count.
pub const RX_BUFFER_NB_BYTES_LSB: Register = Register::new(RX_TOP_BASE + 201, 0, 8, true);

/// Which radio each of the eight channels takes its samples from, one bit per channel.
pub const RX_RADIO_SELECT: Register = Register::new(RX_TOP_BASE + 16, 0, 8, false);

/// How heavily the channel power reading is filtered.
pub const RX_RSSI_FILTER_ALPHA: Register = Register::new(RX_TOP_BASE + 17, 3, 5, false);

/// What a channel power reading starts from before anything is heard.
pub const RX_RSSI_DEFAULT: Register = Register::new(RX_TOP_BASE + 18, 0, 8, false);

/// The level the channel gain control backs off above.
pub const RX_GAIN_THRESHOLD_HIGH: Register = Register::new(RX_TOP_BASE + 19, 0, 8, false);

/// The level it comes back below.
pub const RX_GAIN_THRESHOLD_LOW: Register = Register::new(RX_TOP_BASE + 20, 0, 8, false);

/// The most the channel gain control will attenuate.
pub const RX_GAIN_MAX_ATTENUATION: Register = Register::new(RX_TOP_BASE + 21, 4, 4, false);

/// The least it will attenuate.
pub const RX_GAIN_MIN_ATTENUATION: Register = Register::new(RX_TOP_BASE + 21, 0, 4, false);

/// Whether the gain control microcontroller drives the channel gain rather than the host.
pub const RX_GAIN_AUTOMATIC: Register = Register::new(RX_TOP_BASE + 23, 0, 2, false);

/// The fixed channel gain, which is taken only when the host drives it.
pub const RX_CHANNEL_GAIN: Register = Register::new(RX_TOP_BASE + 25, 0, 4, false);

/// Whether that fixed gain is to be taken.
pub const RX_CHANNEL_GAIN_VALID: Register = Register::new(RX_TOP_BASE + 25, 4, 1, false);

/// Which correlators are clocked, one bit per channel.
pub const RX_CORRELATOR_CLOCK_ENABLE: Register = Register::new(RX_TOP_BASE + 32, 0, 8, false);

/// Which spreading factors the correlators look for, one bit each counting from SF5.
pub const RX_CORRELATOR_SF_ENABLE: Register = Register::new(RX_TOP_BASE + 34, 0, 8, false);

/// Whether a correlator takes only the first edge it detects.
pub const RX_CORRELATOR_FIRST_EDGE: Register = Register::new(RX_TOP_BASE + 35, 0, 8, false);

/// Whether a correlator clears its accumulator between detections.
pub const RX_CORRELATOR_CLEAR: Register = Register::new(RX_TOP_BASE + 36, 0, 8, false);

/// Whether the notch that removes the carrier at zero is in circuit.
pub const RX_DC_NOTCH_ENABLE: Register = Register::new(RX_TOP_BASE + 96, 0, 1, false);

/// Whether the receive filter is held at its default rather than tracking the gain.
pub const RX_FORCE_DEFAULT_FILTER: Register = Register::new(RX_TOP_BASE + 117, 3, 1, false);

/// The level the demodulator gain control aims for.
pub const RX_GAIN_TARGET_LEVEL: Register = Register::new(RX_TOP_BASE + 120, 6, 2, false);

/// Whether a gain drop is compensated for in the demodulator.
pub const RX_GAIN_DROP_COMPENSATION: Register = Register::new(RX_TOP_BASE + 120, 4, 1, false);

/// How many preamble symbols a receiver expects, low bits.
pub const RX_PREAMBLE_SYMBOLS_LSB: Register = Register::new(RX_TOP_BASE + 131, 0, 8, false);

/// The high bits of that count.
pub const RX_PREAMBLE_SYMBOLS_MSB: Register = Register::new(RX_TOP_BASE + 132, 0, 8, false);

/// Which peaks the frequency transform takes.
pub const RX_DFT_PEAK_MODE: Register = Register::new(RX_TOP_BASE + 135, 4, 2, false);

/// The proportional gain the fine timing uses while it is finding a packet.
pub const RX_FINE_TIMING_GAIN_AUTO: Register = Register::new(RX_TOP_BASE + 146, 6, 2, false);

/// The proportional gain it uses through the payload.
pub const RX_FINE_TIMING_GAIN_PAYLOAD: Register = Register::new(RX_TOP_BASE + 146, 3, 3, false);

/// Whether the fine timing runs its integrator at SF11.
pub const RX_FINE_TIMING_INTEGRATE_SF11: Register = Register::new(RX_TOP_BASE + 150, 4, 2, false);

/// Whether it runs it at SF12.
pub const RX_FINE_TIMING_INTEGRATE_SF12: Register = Register::new(RX_TOP_BASE + 150, 6, 2, false);

/// Whether the timestamping bank rounds its result.
pub const RX_TIMING_ROUNDING: Register = Register::new(RX_TOP_BASE + 153, 6, 1, false);

/// How the timestamping bank tracks timing.
pub const RX_TIMING_MODE: Register = Register::new(RX_TOP_BASE + 153, 0, 2, false);

/// The proportional gain the timestamping bank uses while finding a packet.
pub const RX_TIMING_GAIN_AUTO: Register = Register::new(RX_TOP_BASE + 154, 6, 2, false);

/// The proportional gain it uses through the payload.
pub const RX_TIMING_GAIN_PAYLOAD: Register = Register::new(RX_TOP_BASE + 154, 3, 3, false);

/// The proportional gain it uses through the preamble.
pub const RX_TIMING_GAIN_PREAMBLE: Register = Register::new(RX_TOP_BASE + 154, 0, 3, false);

/// The integral gain it uses while finding a packet.
pub const RX_TIMING_INTEGRAL_AUTO: Register = Register::new(RX_TOP_BASE + 155, 6, 2, false);

/// The integral gain it uses through the payload.
pub const RX_TIMING_INTEGRAL_PAYLOAD: Register = Register::new(RX_TOP_BASE + 155, 3, 3, false);

/// The integral gain it uses through the preamble.
pub const RX_TIMING_INTEGRAL_PREAMBLE: Register = Register::new(RX_TOP_BASE + 155, 0, 3, false);

/// Whether the timestamping bank runs its integrator at SF11.
pub const RX_TIMING_INTEGRATE_SF11: Register = Register::new(RX_TOP_BASE + 158, 4, 2, false);

/// Whether it runs it at SF12.
pub const RX_TIMING_INTEGRATE_SF12: Register = Register::new(RX_TOP_BASE + 158, 6, 2, false);

/// The high bits of the drift that turns a frequency error into a timing one.
///
/// The field is four bits wide, not eight. The mantissa never reaches twelve bits, so the
/// high half always fits, but a wider write would land in the register beside it.
pub const RX_DRIFT_MANTISSA_MSB: Register = Register::new(RX_TOP_BASE + 160, 0, 4, false);

/// The low bits of that drift.
pub const RX_DRIFT_MANTISSA_LSB: Register = Register::new(RX_TOP_BASE + 161, 0, 8, false);

/// How far the drift mantissa is shifted.
pub const RX_DRIFT_EXPONENT: Register = Register::new(RX_TOP_BASE + 162, 0, 3, false);

/// Whether the symbol time is taken with the opposite sign.
pub const RX_DRIFT_INVERT_SYMBOL_TIME: Register = Register::new(RX_TOP_BASE + 163, 2, 1, false);

/// How close a frequency has to be before the receiver calls it synchronized.
pub const RX_FREQUENCY_SYNC_THRESHOLD: Register = Register::new(RX_TOP_BASE + 171, 0, 4, false);

/// How far apart the correlators and the demodulators are allowed to be, high bits.
pub const RX_MODEM_SYNC_DELTA_MSB: Register = Register::new(RX_TOP_BASE + 183, 0, 3, false);

/// The low bits of that difference.
pub const RX_MODEM_SYNC_DELTA_LSB: Register = Register::new(RX_TOP_BASE + 184, 0, 8, false);

/// Which of the first eight demodulators are enabled.
pub const MODEM_ENABLE_FULL: Register = Register::new(OTP_BASE + 8, 0, 8, false);

/// Which of the second eight are.
pub const MODEM_ENABLE_LIMITED: Register = Register::new(OTP_BASE + 9, 0, 8, false);

/// Whether the receivers run at all.
pub const COMMON_GLOBAL_ENABLE: Register = Register::new(COMMON_BASE + 5, 3, 1, false);

/// Whether the frequency shift keying demodulator runs.
pub const COMMON_FSK_MODEM_ENABLE: Register = Register::new(COMMON_BASE + 5, 2, 1, false);

/// Whether the eight multi-spreading-factor demodulators run.
pub const COMMON_MULTI_MODEM_ENABLE: Register = Register::new(COMMON_BASE + 5, 1, 1, false);

/// Whether the single demodulator fixed to one spreading factor runs.
pub const COMMON_SERVICE_MODEM_ENABLE: Register = Register::new(COMMON_BASE + 5, 0, 1, false);

/// The high bits of the intermediate frequency the fixed demodulator listens on.
pub const SERVICE_FREQUENCY_MSB: Register =
    Register::new(RX_TOP_LORA_SERVICE_FSK_BASE, 0, 5, false);

/// The low bits of it.
pub const SERVICE_FREQUENCY_LSB: Register =
    Register::new(RX_TOP_LORA_SERVICE_FSK_BASE + 1, 0, 8, false);

/// Which radio that demodulator takes its samples from.
pub const SERVICE_RADIO_SELECT: Register =
    Register::new(RX_TOP_LORA_SERVICE_FSK_BASE + 2, 0, 1, false);

/// The high bits of the intermediate frequency the keying demodulator listens on.
pub const FSK_FREQUENCY_MSB: Register =
    Register::new(RX_TOP_LORA_SERVICE_FSK_BASE + 80, 0, 5, false);

/// The low bits of it.
pub const FSK_FREQUENCY_LSB: Register =
    Register::new(RX_TOP_LORA_SERVICE_FSK_BASE + 81, 0, 8, false);

/// Which radio that demodulator takes its samples from.
pub const FSK_RADIO_SELECT: Register =
    Register::new(RX_TOP_LORA_SERVICE_FSK_BASE + 84, 1, 1, false);

/// The settings that decide how hard a correlator looks for one spreading factor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Correlator {
    /// The ratio the accumulated peak has to reach above the noise.
    pub accumulated_ratio: Register,
    /// The ratio a single peak has to reach.
    pub peak_ratio: Register,
    /// How many peaks are wanted.
    pub peaks: Register,
    /// How many are wanted on the second pass.
    pub second_peaks: Register,
}

/// The pair of registers holding one channel intermediate frequency.
///
/// The eight pairs run from the start of the block, two bytes each, the high bits first.
///
/// # Arguments
///
/// * `channel` - which of the eight, counting from zero. Higher is clamped to seven.
///
/// # Returns
///
/// The high bits and then the low bits.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::register::channel_frequency;
///
/// let [high, low] = channel_frequency(0);
/// assert_eq!((high.address, high.width), (0x5800, 5));
/// assert_eq!((low.address, low.width), (0x5801, 8));
///
/// // The last channel is seven pairs along.
/// assert_eq!(channel_frequency(7)[0].address, 0x580e);
/// ```
#[must_use]
pub const fn channel_frequency(channel: u8) -> [Register; 2] {
    let channel = if channel > 7 { 7 } else { channel } as u16;
    [
        Register::new(RX_TOP_BASE + channel * 2, 0, 5, false),
        Register::new(RX_TOP_BASE + channel * 2 + 1, 0, 8, false),
    ]
}

/// The correlator settings for one spreading factor.
///
/// The eight sets run seven bytes apart, starting at SF5.
///
/// # Arguments
///
/// * `spreading_factor` - from 5 to 12. Anything outside is clamped into it.
///
/// # Returns
///
/// The four registers that tune detection for it.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::register::correlator;
///
/// // SF7 is two sets along from SF5, so fourteen bytes.
/// assert_eq!(correlator(5).accumulated_ratio.address + 14, correlator(7).accumulated_ratio.address);
/// assert_eq!(correlator(7).peaks.width, 3);
/// ```
#[must_use]
pub const fn correlator(spreading_factor: u8) -> Correlator {
    let spreading_factor = if spreading_factor < 5 {
        5
    } else if spreading_factor > 12 {
        12
    } else {
        spreading_factor
    };
    let base = RX_TOP_BASE + 38 + (spreading_factor as u16 - 5) * 7;
    Correlator {
        accumulated_ratio: Register::new(base, 0, 7, false),
        peak_ratio: Register::new(base + 2, 0, 7, false),
        peaks: Register::new(base + 4, 3, 3, false),
        second_peaks: Register::new(base + 5, 2, 3, false),
    }
}

/// Which bank of demodulator settings a register belongs to.
///
/// The chip keeps two: one tuned for reading a packet, one for timing it. Both are set, and
/// the second only matters where a receiver is demodulating twice over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bank {
    /// The bank tuned for reading the packet.
    Demodulation,
    /// The bank tuned for timing it.
    Timestamping,
}

/// Whether a bank tracks the frequency of one spreading factor.
///
/// Four spreading factors share a byte, two bits each, so the eight run across two.
///
/// # Arguments
///
/// * `bank` - which bank to set.
/// * `spreading_factor` - from 5 to 12. Anything outside is clamped into it.
///
/// # Returns
///
/// The register.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::register::{frequency_tracking, Bank};
///
/// // SF5 starts a byte and SF8 ends it; SF9 starts the next.
/// assert_eq!(frequency_tracking(Bank::Demodulation, 5).offset, 0);
/// assert_eq!(frequency_tracking(Bank::Demodulation, 8).offset, 6);
/// assert_eq!(
///     frequency_tracking(Bank::Demodulation, 9).address,
///     frequency_tracking(Bank::Demodulation, 5).address + 1
/// );
/// ```
#[must_use]
pub const fn frequency_tracking(bank: Bank, spreading_factor: u8) -> Register {
    let spreading_factor = if spreading_factor < 5 {
        5
    } else if spreading_factor > 12 {
        12
    } else {
        spreading_factor
    };
    let base = match bank {
        Bank::Demodulation => RX_TOP_BASE + 165,
        Bank::Timestamping => RX_TOP_BASE + 167,
    };
    let address = if spreading_factor <= 8 {
        base
    } else {
        base + 1
    };
    Register::new(address, ((spreading_factor - 5) % 4) * 2, 2, false)
}

/// The drift correction applied at one spreading factor.
///
/// Four share a byte, two bits each, the same way the tracking settings do.
///
/// # Arguments
///
/// * `spreading_factor` - from 5 to 12. Anything outside is clamped into it.
///
/// # Returns
///
/// The register.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::register::drift_correction;
///
/// assert_eq!(drift_correction(5).offset, 0);
/// assert_eq!(drift_correction(12).offset, 6);
/// ```
#[must_use]
pub const fn drift_correction(spreading_factor: u8) -> Register {
    let spreading_factor = if spreading_factor < 5 {
        5
    } else if spreading_factor > 12 {
        12
    } else {
        spreading_factor
    };
    let address = if spreading_factor <= 8 {
        RX_TOP_BASE + 185
    } else {
        RX_TOP_BASE + 186
    };
    Register::new(address, ((spreading_factor - 5) % 4) * 2, 2, false)
}

/// How far one channel is offset when the arbiter lines its receivers up.
///
/// Two channels share a byte, a nibble each, the even one low.
///
/// # Arguments
///
/// * `channel` - which of the eight, counting from zero. Higher is clamped to seven.
///
/// # Returns
///
/// The register.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::register::channel_sync_offset;
///
/// assert_eq!(channel_sync_offset(0).offset, 0);
/// assert_eq!(channel_sync_offset(1).offset, 4);
/// assert_eq!(
///     channel_sync_offset(2).address,
///     channel_sync_offset(0).address + 1
/// );
/// ```
#[must_use]
pub const fn channel_sync_offset(channel: u8) -> Register {
    let channel = if channel > 7 { 7 } else { channel };
    let address = ARB_MCU_BASE + 29 + (channel / 2) as u16;
    let offset = if channel % 2 == 0 { 0 } else { 4 };
    Register::new(address, offset, 4, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The block addresses the reference implementation lists, which every register is an
    // offset from.
    #[test]
    fn the_blocks_are_where_the_chip_puts_them() {
        assert_eq!(EXTERNAL_MEMORY_BASE, 0x0000);
        assert_eq!(RX_BUFFER_BASE, 0x4000);
        assert_eq!(TX_TOP_A_BASE, 0x5200);
        assert_eq!(TX_TOP_B_BASE, 0x5400);
        assert_eq!(COMMON_BASE, 0x5600);
        assert_eq!(GPIO_BASE, 0x5640);
        assert_eq!(MBIST_BASE, 0x56c0);
        assert_eq!(RADIO_FE_BASE, 0x5700);
        assert_eq!(AGC_MCU_BASE, 0x5780);
        assert_eq!(CLK_CTRL_BASE, 0x57c0);
        assert_eq!(RX_TOP_BASE, 0x5800);
        assert_eq!(RX_TOP_LORA_SERVICE_FSK_BASE, 0x5b00);
        assert_eq!(CAPTURE_RAM_BASE, 0x6000);
        assert_eq!(ARB_MCU_BASE, 0x6080);
        assert_eq!(TIMESTAMP_BASE, 0x6100);
        assert_eq!(OTP_BASE, 0x6180);
    }

    #[test]
    fn the_registers_sit_where_the_table_puts_them() {
        assert_eq!(COMMON_PAGE, Register::new(0x5600, 0, 2, false));
        assert_eq!(COMMON_VERSION, Register::new(0x5606, 0, 8, true));
        assert_eq!(OTP_BYTE_ADDR, Register::new(0x6180, 0, 8, false));
        assert_eq!(OTP_RD_DATA, Register::new(0x6181, 0, 8, true));
        assert_eq!(OTP_FSM_READY, Register::new(0x6182, 0, 1, true));
        assert_eq!(OTP_CHECKSUM_STATUS, Register::new(0x6182, 4, 4, true));
    }

    #[test]
    fn the_two_microcontrollers_share_one_control_byte_each() {
        // Every gain control bit is in the byte at the block base.
        for register in [
            AGC_MCU_CLK_EN,
            AGC_MCU_CLEAR,
            AGC_MCU_HOST_PROG,
            AGC_MCU_PARITY_ERROR,
        ] {
            assert_eq!(register.address, AGC_MCU_BASE);
            assert_eq!(register.width, 1);
        }
        for register in [
            ARB_MCU_CLK_EN,
            ARB_MCU_CLEAR,
            ARB_MCU_HOST_PROG,
            ARB_MCU_PARITY_ERROR,
        ] {
            assert_eq!(register.address, ARB_MCU_BASE);
            assert_eq!(register.width, 1);
        }

        // The chip reports these; a host that writes one is writing to a wall.
        for register in [
            AGC_MCU_PARITY_ERROR,
            ARB_MCU_PARITY_ERROR,
            COMMON_VERSION,
            OTP_RD_DATA,
            OTP_FSM_READY,
            OTP_CHECKSUM_STATUS,
        ] {
            assert!(register.read_only, "{register:?} is reported, not set");
        }

        // And these are the ones a host drives.
        for register in [
            AGC_MCU_HOST_PROG,
            ARB_MCU_HOST_PROG,
            COMMON_PAGE,
            OTP_BYTE_ADDR,
        ] {
            assert!(!register.read_only, "{register:?} is set by the host");
        }
    }

    #[test]
    fn the_front_end_controls_sit_where_the_table_puts_them() {
        assert_eq!(COMMON_CLK32_RIF_CTRL, Register::new(0x5601, 4, 1, false));
        assert_eq!(COMMON_HOST_RADIO_CTRL, Register::new(0x5601, 3, 1, false));
        assert_eq!(RF_EN_A_RADIO_EN, Register::new(0x5783, 2, 1, false));
        assert_eq!(RF_EN_A_RADIO_RST, Register::new(0x5783, 3, 1, false));
        assert_eq!(RF_EN_B_RADIO_EN, Register::new(0x5784, 2, 1, false));
        assert_eq!(RF_EN_B_RADIO_RST, Register::new(0x5784, 3, 1, false));

        // The two chains have controls of their own, one byte apart.
        assert_eq!(RF_EN_B_RADIO_EN.address - RF_EN_A_RADIO_EN.address, 1);
        assert_ne!(RF_EN_A_RADIO_RST, RF_EN_B_RADIO_RST);

        // Enable and reset share a byte, so writing one must not disturb the other.
        let byte = RF_EN_A_RADIO_EN.encode(0, 1);
        let byte = RF_EN_A_RADIO_RST.encode(byte, 1);
        assert_eq!(
            RF_EN_A_RADIO_EN.decode(byte),
            1,
            "the front end stayed powered"
        );
        assert_eq!(RF_EN_A_RADIO_RST.decode(byte), 1);
    }

    #[test]
    fn a_narrow_register_leaves_its_neighbors_alone() {
        // Taking the microcontroller for programming must not stop its clock.
        let byte = AGC_MCU_CLK_EN.encode(0x00, 1);
        assert_eq!(AGC_MCU_CLK_EN.decode(byte), 1);

        let byte = AGC_MCU_HOST_PROG.encode(byte, 1);
        assert_eq!(AGC_MCU_HOST_PROG.decode(byte), 1);
        assert_eq!(AGC_MCU_CLK_EN.decode(byte), 1, "the clock stayed on");

        let byte = AGC_MCU_HOST_PROG.encode(byte, 0);
        assert_eq!(AGC_MCU_HOST_PROG.decode(byte), 0);
        assert_eq!(
            AGC_MCU_CLK_EN.decode(byte),
            1,
            "and stayed on after release"
        );
    }

    #[test]
    fn a_whole_byte_needs_no_read_first() {
        assert!(COMMON_VERSION.is_whole_byte());
        assert!(OTP_BYTE_ADDR.is_whole_byte());
        assert!(!COMMON_PAGE.is_whole_byte());
        assert!(!AGC_MCU_CLEAR.is_whole_byte());

        // The status byte holds two registers, read from the same read.
        let status = 0b1010_0001;
        assert_eq!(OTP_FSM_READY.decode(status), 1);
        assert_eq!(OTP_CHECKSUM_STATUS.decode(status), 0b1010);
    }
}
