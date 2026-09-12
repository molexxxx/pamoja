//! Telling the concentrator apart from its relatives, and from nothing at all.
//!
//! Two questions get asked of a board before anything is loaded onto it. The version register
//! answers the first: a concentrator that is powered, wired the right way round, and clocked
//! reads back [`EXPECTED_VERSION`], and a board that reads anything else is not talking.
//!
//! The part number answers the second, and it lives in one-time programmable memory rather
//! than a register, so it takes two transfers: select the byte, then read it. The SX1302 and
//! the SX1303 are the same silicon to the SPI bus, and only this byte separates them.

use super::register::{self, Register};

/// What the version register reads on a concentrator that is answering.
pub const EXPECTED_VERSION: u8 = 0x10;

/// Where the part number sits in the one-time programmable memory.
pub const MODEL_BYTE_ADDRESS: u8 = 0xd0;

/// Which concentrator this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    /// An SX1302, which reports itself as either zero or two.
    Sx1302,
    /// An SX1303, which adds the fine timestamping the SX1302 has not.
    Sx1303,
    /// A part number this build does not know, carried so the caller can report it.
    Unknown(u8),
}

impl Model {
    /// Reads the part number byte.
    ///
    /// # Arguments
    ///
    /// * `byte` - what the one-time programmable memory answered with.
    ///
    /// # Returns
    ///
    /// Which concentrator it is.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::chip::Model;
    ///
    /// // The SX1302 reports itself two ways, and both mean the same part.
    /// assert_eq!(Model::of(0x00), Model::Sx1302);
    /// assert_eq!(Model::of(0x02), Model::Sx1302);
    /// assert_eq!(Model::of(0x03), Model::Sx1303);
    /// assert_eq!(Model::of(0x7f), Model::Unknown(0x7f));
    /// ```
    #[must_use]
    pub const fn of(byte: u8) -> Model {
        match byte {
            0x00 | 0x02 => Model::Sx1302,
            0x03 => Model::Sx1303,
            other => Model::Unknown(other),
        }
    }

    /// The part number as a name.
    ///
    /// # Returns
    ///
    /// The name, or `unknown` for a part number this build does not know.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Model::Sx1302 => "SX1302",
            Model::Sx1303 => "SX1303",
            Model::Unknown(_) => "unknown",
        }
    }

    /// Whether this is a concentrator this crate drives.
    ///
    /// # Returns
    ///
    /// Whether the part number is one of the two known ones.
    #[must_use]
    pub const fn is_known(&self) -> bool {
        matches!(self, Model::Sx1302 | Model::Sx1303)
    }
}

/// A step in reading the part number.
///
/// Reading it takes two transfers in order, so the sequence is carried as data: a driver walks
/// it, and a test walks it without a bus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Write a value to a register.
    Write(Register, u8),
    /// Read a register, which is the answer when it is the last step.
    Read(Register),
}

/// The transfers that read the part number, in order.
///
/// # Returns
///
/// Select the model byte in the one-time programmable memory, then read it. What the last
/// step reads goes to [`Model::of`].
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::chip::{identify, Model, Step, MODEL_BYTE_ADDRESS};
/// use pamoja_radios::sx1302::register;
///
/// let steps = identify();
/// assert_eq!(steps[0], Step::Write(register::OTP_BYTE_ADDR, MODEL_BYTE_ADDRESS));
/// assert_eq!(steps[1], Step::Read(register::OTP_RD_DATA));
///
/// // A board that answered with three is an SX1303.
/// assert_eq!(Model::of(0x03), Model::Sx1303);
/// ```
#[must_use]
pub const fn identify() -> [Step; 2] {
    [
        Step::Write(register::OTP_BYTE_ADDR, MODEL_BYTE_ADDRESS),
        Step::Read(register::OTP_RD_DATA),
    ]
}

/// Whether the version register says a concentrator is answering.
///
/// # Arguments
///
/// * `version` - what [`COMMON_VERSION`](register::COMMON_VERSION) read.
///
/// # Returns
///
/// Whether it matches [`EXPECTED_VERSION`]. A board that is unpowered, miswired, or held in
/// reset reads zero or all ones instead.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::chip::answers;
///
/// assert!(answers(0x10));
/// assert!(!answers(0x00), "an unpowered board reads zero");
/// assert!(!answers(0xff), "a floating bus reads all ones");
/// ```
#[must_use]
pub const fn answers(version: u8) -> bool {
    version == EXPECTED_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_part_numbers_are_the_ones_the_chip_reports() {
        // The SX1302 leaves this byte unprogrammed on some parts, so zero means the same.
        assert_eq!(Model::of(0x00), Model::Sx1302);
        assert_eq!(Model::of(0x02), Model::Sx1302);
        assert_eq!(Model::of(0x03), Model::Sx1303);

        assert_eq!(Model::of(0x00).name(), "SX1302");
        assert_eq!(Model::of(0x03).name(), "SX1303");
        assert!(Model::of(0x02).is_known());
        assert!(Model::of(0x03).is_known());
    }

    #[test]
    fn a_part_number_we_do_not_know_is_carried_not_discarded() {
        // A caller reporting a board that will not run should be able to say what it read.
        let read = Model::of(0x2a);
        assert_eq!(read, Model::Unknown(0x2a));
        assert!(!read.is_known());
        assert_eq!(read.name(), "unknown");
        assert!(format!("{read:?}").contains("42"));
    }

    #[test]
    fn identifying_selects_the_byte_before_reading_it() {
        let steps = identify();
        assert_eq!(
            steps,
            [
                Step::Write(register::OTP_BYTE_ADDR, 0xd0),
                Step::Read(register::OTP_RD_DATA),
            ]
        );

        // The read is of a register the chip will not let anyone write.
        let Step::Read(answer) = steps[1] else {
            panic!("the last step reads the answer");
        };
        assert!(answer.read_only);
        assert_eq!(answer.address, register::OTP_BASE + 1);
    }

    #[test]
    fn only_the_documented_version_counts_as_answering() {
        assert!(answers(EXPECTED_VERSION));
        assert_eq!(EXPECTED_VERSION, 0x10);

        // The two ways a bus reads when nothing is driving it.
        assert!(!answers(0x00));
        assert!(!answers(0xff));
        // And a neighboring value is still not a match.
        assert!(!answers(0x11));
    }
}
