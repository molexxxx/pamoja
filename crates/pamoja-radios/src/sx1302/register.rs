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
