//! Putting firmware into the two microcontrollers the concentrator runs.
//!
//! The chip does not receive anything on its own. Two microcontrollers inside it do the work,
//! and both start empty: the gain control one sets the front end gains, and the arbiter shares
//! the radios between the receivers. Each holds eight kilobytes, written over the same SPI bus
//! as everything else, into a memory window the host takes control of first.
//!
//! The order matters and the chip does not enforce it. The host holds the microcontroller in
//! reset, claims its memory, writes, reads the same bytes back and compares them, releases it,
//! and only then asks whether the parity check passed. Skipping the read-back means a bad bus
//! looks like a working one until the concentrator quietly hears nothing.
//!
//! The firmware images themselves are not in this crate. They belong to Semtech and are
//! carried by the caller, so a [`Load`] says what to do with the bytes rather than holding
//! them. The reference implementation distributes them as C source rather than as files of
//! bytes, so [`read_source`] reads that form directly and a gateway points at the file it
//! already has.

use super::register::{self, Register};

/// How many bytes of firmware each microcontroller holds.
///
/// The memory is fourteen bits wide, so this is twice the number of words.
pub const FIRMWARE_LEN: usize = 8192;

/// Where the gain control firmware is written.
pub const AGC_MEMORY: u16 = 0x0000;

/// Where the arbiter firmware is written.
pub const ARB_MEMORY: u16 = 0x2000;

/// Which microcontroller is being loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mcu {
    /// The gain control microcontroller, which drives the front end gains.
    Agc,
    /// The arbiter, which shares the radios between the receivers.
    Arb,
}

impl Mcu {
    /// Where this microcontroller keeps its firmware.
    ///
    /// # Returns
    ///
    /// The address the first byte is written to.
    #[must_use]
    pub const fn memory(&self) -> u16 {
        match self {
            Mcu::Agc => AGC_MEMORY,
            Mcu::Arb => ARB_MEMORY,
        }
    }

    /// The register that holds this microcontroller in reset.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn clear(&self) -> Register {
        match self {
            Mcu::Agc => register::AGC_MCU_CLEAR,
            Mcu::Arb => register::ARB_MCU_CLEAR,
        }
    }

    /// The register that gives the host this microcontroller memory.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn host_prog(&self) -> Register {
        match self {
            Mcu::Agc => register::AGC_MCU_HOST_PROG,
            Mcu::Arb => register::ARB_MCU_HOST_PROG,
        }
    }

    /// The register the chip reports a failed parity check in.
    ///
    /// # Returns
    ///
    /// The register.
    #[must_use]
    pub const fn parity_error(&self) -> Register {
        match self {
            Mcu::Agc => register::AGC_MCU_PARITY_ERROR,
            Mcu::Arb => register::ARB_MCU_PARITY_ERROR,
        }
    }

    /// What this microcontroller is called.
    ///
    /// # Returns
    ///
    /// The name, for a message a person reads.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Mcu::Agc => "gain control",
            Mcu::Arb => "arbiter",
        }
    }
}

/// A step in loading firmware.
///
/// The whole sequence is data, so a driver walks it against a bus and a test walks it against
/// nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Load {
    /// Write a value to a register.
    Write(Register, u8),
    /// Write the firmware to memory, starting here.
    WriteFirmware(u16),
    /// Read the same span back, to compare it with what went out.
    ReadBack(u16),
    /// Read the parity register, which must come back zero.
    CheckParity(Register),
}

/// Why firmware did not load.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadError {
    /// The image handed over is not the size the microcontroller holds.
    WrongSize {
        /// How many bytes the caller offered.
        offered: usize,
    },
    /// The bytes read back are not the bytes written, so the bus is not carrying them.
    ReadBackDiffers {
        /// Where the first difference is.
        at: usize,
    },
    /// The bytes arrived but the chip refused them, so the image itself is wrong.
    ParityFailed,
}

impl core::fmt::Display for LoadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LoadError::WrongSize { offered } => write!(
                f,
                "firmware is {offered} bytes, and a microcontroller holds {FIRMWARE_LEN}"
            ),
            LoadError::ReadBackDiffers { at } => write!(
                f,
                "the firmware read back differs at byte {at}, so the bus is not carrying it"
            ),
            LoadError::ParityFailed => {
                f.write_str("the concentrator refused the firmware on its own parity check")
            }
        }
    }
}

/// The steps that load one microcontroller, in order.
///
/// # Arguments
///
/// * `mcu` - which microcontroller to load.
///
/// # Returns
///
/// Hold it in reset, claim its memory, put the memory window on the first page, write, read
/// back, release it, and check the parity the chip reports.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::firmware::{steps, Load, Mcu};
/// use pamoja_radios::sx1302::register;
///
/// let plan = steps(Mcu::Agc);
///
/// // The microcontroller is stopped before its memory is touched.
/// assert_eq!(plan[0], Load::Write(register::AGC_MCU_CLEAR, 1));
/// assert_eq!(plan[1], Load::Write(register::AGC_MCU_HOST_PROG, 1));
///
/// // And the chip has the last word on whether the firmware took.
/// assert_eq!(plan[6], Load::CheckParity(register::AGC_MCU_PARITY_ERROR));
/// ```
#[must_use]
pub const fn steps(mcu: Mcu) -> [Load; 7] {
    [
        Load::Write(mcu.clear(), 1),
        Load::Write(mcu.host_prog(), 1),
        Load::Write(register::COMMON_PAGE, 0),
        Load::WriteFirmware(mcu.memory()),
        Load::ReadBack(mcu.memory()),
        Load::Write(mcu.host_prog(), 0),
        Load::CheckParity(mcu.parity_error()),
    ]
}

/// Checks an image before any of it goes out.
///
/// # Arguments
///
/// * `firmware` - the image the caller holds.
///
/// # Returns
///
/// `Ok(())` when it is the size a microcontroller holds.
///
/// # Errors
///
/// Returns [`LoadError::WrongSize`] otherwise, rather than writing a truncated image and
/// leaving the chip to fail its own parity check later.
pub const fn check_size(firmware: &[u8]) -> Result<(), LoadError> {
    if firmware.len() == FIRMWARE_LEN {
        Ok(())
    } else {
        Err(LoadError::WrongSize {
            offered: firmware.len(),
        })
    }
}

/// Reads an image out of the C source the reference implementation carries it in.
///
/// Semtech distributes each image as an array of byte literals rather than as a file of
/// bytes, so converting it by hand is the first thing that stands between a new board and a
/// running gateway. This reads the literals in the order they are written, which turns that
/// step into naming the file.
///
/// Anything that is not a byte literal is skipped, so the declaration around the array, the
/// commas, and the closing brace need no stripping.
///
/// # Arguments
///
/// * `text` - the file, as text.
/// * `into` - where the bytes go, which is exactly what a microcontroller holds.
///
/// # Returns
///
/// `Ok(())` once every byte is in place.
///
/// # Errors
///
/// Returns [`LoadError::WrongSize`] carrying how many literals the text holds when that is
/// not [`FIRMWARE_LEN`], rather than loading a partial image. A count below it means the file
/// is truncated, and one above means it carries more than the single array.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::firmware::{read_source, FIRMWARE_LEN};
///
/// // The shape Semtech ships, shortened here to what fits on the page.
/// let mut source = String::from("static uint8_t arb_firmware[8192] = {\n");
/// for at in 0..FIRMWARE_LEN {
///     source.push_str(&format!("0x{:02X}, ", at % 256));
/// }
/// source.push_str("\n};\n");
///
/// let mut image = [0u8; FIRMWARE_LEN];
/// read_source(&source, &mut image).expect("the array holds a whole image");
/// assert_eq!(image[0], 0x00);
/// assert_eq!(image[255], 0xFF);
/// ```
pub fn read_source(text: &str, into: &mut [u8; FIRMWARE_LEN]) -> Result<(), LoadError> {
    let raw = text.as_bytes();
    let mut found = 0usize;
    let mut at = 0usize;

    while at + 2 < raw.len() {
        let prefixed = raw[at] == b'0' && (raw[at + 1] | 0x20) == b'x';
        let Some(high) = hex_digit(raw[at + 2]).filter(|_| prefixed) else {
            at += 1;
            continue;
        };

        // A literal is written with two digits throughout the reference files, but one is
        // still a byte, so a single digit is read rather than silently dropped.
        let (value, width) = match raw.get(at + 3).copied().and_then(hex_digit) {
            Some(low) => (high * 16 + low, 4),
            None => (high, 3),
        };

        if let Some(slot) = into.get_mut(found) {
            *slot = value;
        }
        found += 1;
        at += width;
    }

    if found == FIRMWARE_LEN {
        Ok(())
    } else {
        Err(LoadError::WrongSize { offered: found })
    }
}

/// Reads one hexadecimal digit.
const fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Compares what was read back with what was written.
///
/// # Arguments
///
/// * `written` - the image that went out.
/// * `read_back` - the same span read from the chip.
///
/// # Returns
///
/// `Ok(())` when every byte matches.
///
/// # Errors
///
/// Returns [`LoadError::ReadBackDiffers`] naming the first byte that does not, which says
/// where on the bus the trouble starts rather than only that there is some.
pub fn compare(written: &[u8], read_back: &[u8]) -> Result<(), LoadError> {
    for (at, (out, back)) in written.iter().zip(read_back.iter()).enumerate() {
        if out != back {
            return Err(LoadError::ReadBackDiffers { at });
        }
    }
    if written.len() == read_back.len() {
        Ok(())
    } else {
        Err(LoadError::ReadBackDiffers {
            at: written.len().min(read_back.len()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_image_is_read_out_of_the_source_it_is_distributed_as() {
        let mut source = String::from("static uint8_t agc_firmware_sx1250[8192] = { \n");
        for at in 0..FIRMWARE_LEN {
            source.push_str(&format!("0x{:02X}, ", at % 251));
            if at % 16 == 15 {
                source.push('\n');
            }
        }
        source.push_str("};\n");

        let mut image = [0u8; FIRMWARE_LEN];
        read_source(&source, &mut image).expect("the array holds a whole image");

        // The declaration, the commas and the closing brace carry no literals, so none of
        // them shift the bytes.
        for (at, byte) in image.iter().enumerate() {
            assert_eq!(*byte, (at % 251) as u8, "byte {at}");
        }
    }

    #[test]
    fn a_truncated_file_is_refused_with_what_it_held() {
        let source = "static uint8_t arb_firmware[8192] = { 0x01, 0x02, 0x03 };";

        let mut image = [0u8; FIRMWARE_LEN];
        assert_eq!(
            read_source(source, &mut image),
            Err(LoadError::WrongSize { offered: 3 })
        );
    }

    #[test]
    fn a_file_carrying_more_than_one_array_is_refused() {
        let mut source = String::new();
        for _ in 0..=FIRMWARE_LEN {
            source.push_str("0x00, ");
        }

        // Counting past the end rather than stopping at it is what lets the error say how
        // much the file actually holds.
        let mut image = [0u8; FIRMWARE_LEN];
        assert_eq!(
            read_source(&source, &mut image),
            Err(LoadError::WrongSize {
                offered: FIRMWARE_LEN + 1
            })
        );
    }

    #[test]
    fn the_case_of_a_literal_does_not_matter() {
        let mut source = String::new();
        for at in 0..FIRMWARE_LEN {
            source.push_str(if at % 2 == 0 { "0xab, " } else { "0XCD, " });
        }

        let mut image = [0u8; FIRMWARE_LEN];
        read_source(&source, &mut image).expect("both cases are literals");
        assert_eq!(image[0], 0xAB);
        assert_eq!(image[1], 0xCD);
    }

    #[test]
    fn each_microcontroller_has_its_own_memory_and_controls() {
        assert_eq!(Mcu::Agc.memory(), 0x0000);
        assert_eq!(Mcu::Arb.memory(), 0x2000);

        // The two never share a register, or loading one would disturb the other.
        assert_ne!(Mcu::Agc.clear(), Mcu::Arb.clear());
        assert_ne!(Mcu::Agc.host_prog(), Mcu::Arb.host_prog());
        assert_ne!(Mcu::Agc.parity_error(), Mcu::Arb.parity_error());

        assert_eq!(Mcu::Agc.name(), "gain control");
        assert_eq!(Mcu::Arb.name(), "arbiter");
    }

    #[test]
    fn the_firmware_spans_the_memory_exactly() {
        // Eight kilobytes at each address, and the two spans do not overlap.
        assert_eq!(FIRMWARE_LEN, 8192);
        assert_eq!(AGC_MEMORY as usize + FIRMWARE_LEN, ARB_MEMORY as usize);
    }

    #[test]
    fn the_sequence_stops_the_chip_before_writing_and_checks_after() {
        for mcu in [Mcu::Agc, Mcu::Arb] {
            let plan = steps(mcu);

            // Stopped and claimed before the memory is touched.
            assert_eq!(plan[0], Load::Write(mcu.clear(), 1));
            assert_eq!(plan[1], Load::Write(mcu.host_prog(), 1));
            assert_eq!(plan[2], Load::Write(register::COMMON_PAGE, 0));

            // Written, then read back before anything is released.
            assert_eq!(plan[3], Load::WriteFirmware(mcu.memory()));
            assert_eq!(plan[4], Load::ReadBack(mcu.memory()));

            // Released, then asked whether it took.
            assert_eq!(plan[5], Load::Write(mcu.host_prog(), 0));
            assert_eq!(plan[6], Load::CheckParity(mcu.parity_error()));
        }
    }

    #[test]
    fn an_image_of_the_wrong_size_never_reaches_the_bus() {
        assert_eq!(check_size(&[0u8; FIRMWARE_LEN]), Ok(()));
        assert_eq!(
            check_size(&[0u8; 16]),
            Err(LoadError::WrongSize { offered: 16 })
        );
        assert_eq!(check_size(&[]), Err(LoadError::WrongSize { offered: 0 }));
    }

    #[test]
    fn a_read_back_says_where_the_bus_went_wrong() {
        let written = [0xa5u8; 64];
        assert_eq!(compare(&written, &written), Ok(()));

        // One byte dropped, and the caller is told which.
        let mut back = written;
        back[40] = 0x00;
        assert_eq!(
            compare(&written, &back),
            Err(LoadError::ReadBackDiffers { at: 40 })
        );

        // A short read is a difference too, at the point it stopped.
        assert_eq!(
            compare(&written, &written[..20]),
            Err(LoadError::ReadBackDiffers { at: 20 })
        );
    }

    #[test]
    fn every_failure_says_what_to_do_about_it() {
        // A person reading the log should be able to tell a wiring fault from a bad image.
        let wiring = LoadError::ReadBackDiffers { at: 7 };
        let image = LoadError::ParityFailed;
        assert!(format!("{wiring}").contains("bus is not carrying"));
        assert!(format!("{image}").contains("parity"));
        assert!(format!("{}", LoadError::WrongSize { offered: 3 }).contains("8192"));
    }
}
