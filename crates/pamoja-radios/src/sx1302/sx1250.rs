//! The SX1250 front ends a concentrator listens through.
//!
//! A concentrator has no radio of its own. An SX1250 sits on each chain, and that is what a
//! carrier is tuned to: the concentrator only ever sees an intermediate frequency. Both front
//! ends are reached through the concentrator, on the same SPI device, with the first byte of
//! every frame naming the front end rather than the concentrator.
//!
//! A front end gives the host no BUSY line to watch, so the wait before a frame is a flat
//! millisecond rather than a pin. The reference does the same.
//!
//! Nothing here opens a bus. The framing, the register value a carrier becomes, the bands
//! image calibration is run over, and the order a front end is brought up in are all data, so
//! each one can be checked against the reference with no hardware present.

use super::spi;
use super::tx::Chain;

/// How long to wait before a frame, in microseconds.
pub const WAIT_BUSY_US: u32 = 1_000;

/// How long a mode change is given to settle, in microseconds.
pub const SETTLE_US: u32 = 10_000;

/// How long image calibration is given to finish, in microseconds.
pub const IMAGE_CALIBRATE_US: u32 = 10_000;

/// How many bytes of framing come before a command payload.
pub const HEADER_LEN: usize = 2;

/// The crystal the front end counts its carrier in, in hertz.
pub const XTAL_HZ: u64 = 32_000_000;

/// Selects a standby mode.
pub const OP_SET_STANDBY: u8 = 0x80;

/// Reads the chip status, which carries the mode it is in.
pub const OP_GET_STATUS: u8 = 0xc0;

/// Runs the calibrations named by a mask.
pub const OP_CALIBRATE: u8 = 0x89;

/// Runs image calibration over a band.
pub const OP_CALIBRATE_IMAGE: u8 = 0x98;

/// Writes one of the registers behind the command set.
pub const OP_WRITE_REGISTER: u8 = 0x0d;

/// Sets the carrier.
pub const OP_SET_RF_FREQUENCY: u8 = 0x86;

/// Puts the front end in receive.
pub const OP_SET_RX: u8 = 0x82;

/// Reads what the chip believes went wrong.
pub const OP_GET_DEVICE_ERRORS: u8 = 0x17;

/// The mask that runs every calibration the chip has.
pub const CALIBRATE_ALL: u8 = 0x7f;

/// The timeout that means receive forever.
pub const RX_CONTINUOUS: [u8; 3] = [0xff, 0xff, 0xff];

/// The mode a front end reports once it is in standby on its own oscillator.
pub const MODE_STANDBY_RC: u8 = 0x02;

/// The mode it reports once it is in standby on the crystal.
pub const MODE_STANDBY_XOSC: u8 = 0x03;

/// Which clock a standby runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Standby {
    /// The internal oscillator, which is where calibration is run.
    Rc,
    /// The crystal, which is where the carrier is set.
    Xosc,
}

impl Standby {
    /// The byte the standby command takes.
    ///
    /// # Returns
    ///
    /// The value for this mode.
    #[must_use]
    pub const fn value(&self) -> u8 {
        match self {
            Standby::Rc => 0x00,
            Standby::Xosc => 0x01,
        }
    }

    /// What the status register reads once the chip is in this mode.
    ///
    /// # Returns
    ///
    /// The mode to expect from [`mode`].
    #[must_use]
    pub const fn expected_mode(&self) -> u8 {
        match self {
            Standby::Rc => MODE_STANDBY_RC,
            Standby::Xosc => MODE_STANDBY_XOSC,
        }
    }
}

/// A step in bringing a front end up.
///
/// The order is carried as data, so a driver walks it against a bus and a test walks it
/// against nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Put the front end in a standby mode.
    Standby(Standby),
    /// Wait, in microseconds.
    Wait(u32),
    /// Read the status and refuse anything but this mode.
    ExpectMode(u8),
    /// Run the calibrations named by this mask.
    Calibrate(u8),
    /// Write one byte to a register behind the command set.
    Register(u16, u8),
    /// Clear the three bytes of the frequency offset at this register.
    ClearOffset(u16),
    /// Tune the carrier, in hertz.
    Tune(u32),
    /// Listen until told otherwise, which is also what clocks the concentrator.
    ListenContinuously,
}

/// Which front end a chain talks to.
///
/// # Arguments
///
/// * `chain` - the chain the front end is on.
///
/// # Returns
///
/// The target byte every frame to that front end starts with.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1250::target;
/// use pamoja_radios::sx1302::tx::Chain;
///
/// assert_eq!(target(Chain::A), 0x01);
/// assert_eq!(target(Chain::B), 0x02);
/// ```
#[must_use]
pub const fn target(chain: Chain) -> u8 {
    match chain {
        Chain::A => spi::TARGET_RADIO_A,
        Chain::B => spi::TARGET_RADIO_B,
    }
}

/// The two bytes every frame to a front end starts with.
///
/// A command is this header and then its payload. A read sends the same frame and takes the
/// answer from where the payload would have been, which is why [`HEADER_LEN`] is what a
/// reader skips.
///
/// # Arguments
///
/// * `chain` - the chain the front end is on.
/// * `opcode` - the command.
///
/// # Returns
///
/// The header.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1250::{header, OP_SET_RF_FREQUENCY};
/// use pamoja_radios::sx1302::tx::Chain;
///
/// assert_eq!(header(Chain::A, OP_SET_RF_FREQUENCY), [0x01, 0x86]);
/// ```
#[must_use]
pub const fn header(chain: Chain, opcode: u8) -> [u8; HEADER_LEN] {
    [target(chain), opcode]
}

/// Turns a carrier into the value the frequency command takes.
///
/// The chip counts the carrier in steps of its crystal, so the value is the carrier times two
/// to the twenty-fifth over the crystal. That divides exactly, which keeps this integer.
///
/// # Arguments
///
/// * `hertz` - the carrier.
///
/// # Returns
///
/// The value, most significant byte first when it is written.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1250::frequency_value;
///
/// assert_eq!(frequency_value(868_100_000), 0x3641_9999);
/// assert_eq!(frequency_value(915_000_000), 0x3930_0000);
/// ```
#[must_use]
pub const fn frequency_value(hertz: u32) -> u32 {
    ((hertz as u64) * (1 << 25) / XTAL_HZ) as u32
}

/// Splits a carrier into the four bytes the frequency command takes.
///
/// # Arguments
///
/// * `hertz` - the carrier.
///
/// # Returns
///
/// The bytes, most significant first.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1250::frequency_bytes;
///
/// assert_eq!(frequency_bytes(868_100_000), [0x36, 0x41, 0x99, 0x99]);
/// ```
#[must_use]
pub const fn frequency_bytes(hertz: u32) -> [u8; 4] {
    frequency_value(hertz).to_be_bytes()
}

/// The mode a status byte reports.
///
/// # Arguments
///
/// * `status` - what the status command answered with.
///
/// # Returns
///
/// The three bits that name the mode.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1250::{mode, MODE_STANDBY_XOSC};
///
/// assert_eq!(mode(0b0011_0000), MODE_STANDBY_XOSC);
/// ```
#[must_use]
pub const fn mode(status: u8) -> u8 {
    (status >> 4) & 0b111
}

/// The pair of bytes image calibration takes for the band a carrier is in.
///
/// The chip calibrates a band rather than a frequency, and it knows five of them. A carrier
/// outside all five has no answer, which is a configuration mistake rather than a bus one.
///
/// # Arguments
///
/// * `hertz` - the carrier.
///
/// # Returns
///
/// The band, or `None` for a carrier outside every band the chip calibrates.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1250::image_band;
///
/// // The two bands a LoRaWAN gateway runs in.
/// assert_eq!(image_band(868_100_000), Some([0xd7, 0xdb]));
/// assert_eq!(image_band(915_000_000), Some([0xe1, 0xe9]));
///
/// // A carrier between the bands has none.
/// assert_eq!(image_band(880_000_000), None);
/// ```
#[must_use]
pub const fn image_band(hertz: u32) -> Option<[u8; 2]> {
    match hertz {
        430_000_001..=439_999_999 => Some([0x6b, 0x6f]),
        470_000_001..=509_999_999 => Some([0x75, 0x81]),
        779_000_001..=786_999_999 => Some([0xc1, 0xc5]),
        863_000_001..=869_999_999 => Some([0xd7, 0xdb]),
        902_000_001..=927_999_999 => Some([0xe1, 0xe9]),
        _ => None,
    }
}

/// Whether the device errors say image calibration failed.
///
/// # Arguments
///
/// * `third` - the third byte the device errors command answered with.
///
/// # Returns
///
/// Whether the image calibration bit is set.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1250::image_failed;
///
/// assert!(image_failed(0b0001_0000));
/// assert!(!image_failed(0b0000_0000));
/// ```
#[must_use]
pub const fn image_failed(third: u8) -> bool {
    (third >> 4) & 1 == 1
}

/// The order a front end is brought up in.
///
/// It is calibrated on its own oscillator, moved to the crystal, given the register settings
/// the reference sets, tuned, and left listening. The last part matters for more than
/// receiving: the concentrator takes its clock from a front end that is in receive, so a
/// front end left idle stops the chip beside it.
///
/// # Arguments
///
/// * `hertz` - the carrier to tune to.
/// * `single_input` - whether this board wires the front end single ended rather than
///   differential.
///
/// # Returns
///
/// The steps, in the order the chip takes them.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::sx1250::{setup, Standby, Step};
///
/// // A differential board takes one step fewer than a single ended one.
/// assert_eq!(setup(868_100_000, false).count(), 21);
/// assert_eq!(setup(868_100_000, true).count(), 22);
///
/// // Calibration runs on the internal oscillator, before the crystal is started.
/// let mut order = setup(868_100_000, false);
/// assert_eq!(order.next(), Some(Step::Standby(Standby::Rc)));
/// ```
pub fn setup(hertz: u32, single_input: bool) -> impl Iterator<Item = Step> {
    let single = single_input.then_some(Step::Register(0x08e2, 0x0d));
    [
        Some(Step::Standby(Standby::Rc)),
        Some(Step::Wait(SETTLE_US)),
        Some(Step::ExpectMode(MODE_STANDBY_RC)),
        Some(Step::Calibrate(CALIBRATE_ALL)),
        Some(Step::Wait(SETTLE_US)),
        Some(Step::Standby(Standby::Xosc)),
        Some(Step::Wait(SETTLE_US)),
        Some(Step::ExpectMode(MODE_STANDBY_XOSC)),
        // The bitrate is set to its maximum, which shortens the switch into transmit.
        Some(Step::Register(0x06a1, 0x01)),
        Some(Step::Register(0x06a2, 0x00)),
        Some(Step::Register(0x06a3, 0x00)),
        // The pins are driven weakly, with nothing enabled as an input and no pulls.
        Some(Step::Register(0x0582, 0x00)),
        Some(Step::Register(0x0583, 0x00)),
        Some(Step::Register(0x0584, 0x00)),
        Some(Step::Register(0x0585, 0x00)),
        Some(Step::Register(0x0580, 0x00)),
        Some(Step::Register(0x08b6, 0x2a)),
        Some(Step::Tune(hertz)),
        Some(Step::ClearOffset(0x088f)),
        Some(Step::ListenContinuously),
        single,
        Some(Step::Register(0x0587, 0x0b)),
    ]
    .into_iter()
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_carrier_is_counted_in_crystal_steps() {
        // Worked out from the crystal rather than from the function: 868.1 MHz is
        // 868100000 * 2^25 / 32000000, which is 910268825.
        assert_eq!(frequency_value(868_100_000), 910_268_825);
        assert_eq!(frequency_bytes(868_100_000), [0x36, 0x41, 0x99, 0x99]);
    }

    #[test]
    fn the_bands_stop_where_the_chip_stops() {
        // The edges are exclusive in the reference, so a carrier exactly on one is outside.
        assert_eq!(image_band(863_000_000), None);
        assert_eq!(image_band(870_000_000), None);
        assert_eq!(image_band(863_000_001), Some([0xd7, 0xdb]));
        assert_eq!(image_band(869_999_999), Some([0xd7, 0xdb]));
    }

    #[test]
    fn every_band_the_chip_knows_has_a_pair() {
        for (hertz, expected) in [
            (433_000_000, [0x6b, 0x6f]),
            (490_000_000, [0x75, 0x81]),
            (783_000_000, [0xc1, 0xc5]),
            (868_100_000, [0xd7, 0xdb]),
            (915_000_000, [0xe1, 0xe9]),
        ] {
            assert_eq!(image_band(hertz), Some(expected), "{hertz}");
        }
    }

    #[test]
    fn a_status_byte_carries_the_mode_in_three_bits() {
        assert_eq!(mode(0b0010_0000), MODE_STANDBY_RC);
        assert_eq!(mode(0b0011_1010), MODE_STANDBY_XOSC);
    }

    #[test]
    fn the_front_end_is_left_listening() {
        // The concentrator is clocked by a front end in receive, so this is the step that
        // keeps the chip beside it running, not just the one that hears packets.
        let steps: Vec<Step> = setup(868_100_000, false).collect();
        assert!(steps.contains(&Step::ListenContinuously));
        assert!(steps.contains(&Step::Tune(868_100_000)));
    }

    #[test]
    fn calibration_comes_before_the_crystal() {
        let steps: Vec<Step> = setup(868_100_000, false).collect();
        let calibrate = steps
            .iter()
            .position(|step| matches!(step, Step::Calibrate(_)))
            .expect("the front end is calibrated");
        let crystal = steps
            .iter()
            .position(|step| matches!(step, Step::Standby(Standby::Xosc)))
            .expect("the crystal is started");
        assert!(calibrate < crystal, "{steps:?}");
    }

    #[test]
    fn a_frame_names_the_front_end_first() {
        assert_eq!(header(Chain::A, OP_GET_STATUS), [0x01, 0xc0]);
        assert_eq!(header(Chain::B, OP_SET_RX), [0x02, 0x82]);
    }
}
