//! The SPI transactions the SX1302 answers.
//!
//! Every transfer starts with a target byte, because the concentrator sits in front of the
//! radios and passes a transfer through to whichever the first byte names. After that comes a
//! command byte carrying the access bit and the top seven bits of the address, then the low
//! eight bits, then the data.
//!
//! A read costs two bytes more than a write: the chip needs a byte to turn the bus around, so
//! the host clocks out a dummy and reads the answer in the last byte it receives.
//!
//! These functions build the bytes and read them back. They touch no bus, so a caller can
//! check a frame against the datasheet, or replay one captured from a working gateway,
//! without any hardware present.

/// The concentrator itself, rather than a radio behind it.
pub const TARGET_CONCENTRATOR: u8 = 0x00;

/// The radio on the first chain, reached through the concentrator.
pub const TARGET_RADIO_A: u8 = 0x01;

/// The radio on the second chain.
pub const TARGET_RADIO_B: u8 = 0x02;

/// The access bit for a read, which is the absence of the write bit.
pub const READ_ACCESS: u8 = 0x00;

/// The access bit for a write, the top bit of the command byte.
pub const WRITE_ACCESS: u8 = 0x80;

/// The most bytes one burst carries before the host starts another.
///
/// A longer transfer is split into chunks of this size, each with a header of its own and an
/// address advanced by however much has already gone out.
pub const BURST_CHUNK: usize = 1024;

/// The clock the concentrator is driven at, in hertz.
pub const CLOCK_HZ: u32 = 2_000_000;

/// How many bytes a single-byte write transfer takes.
pub const WRITE_LEN: usize = 4;

/// How many bytes a single-byte read transfer takes.
pub const READ_LEN: usize = 5;

/// How many bytes the header of a burst write takes, before the data.
pub const BURST_WRITE_HEADER_LEN: usize = 3;

/// How many bytes the header of a burst read takes, before the data.
pub const BURST_READ_HEADER_LEN: usize = 4;

/// Builds the transfer that writes one byte to a register.
///
/// # Arguments
///
/// * `target` - which chip the transfer is for, one of the `TARGET_` constants.
/// * `address` - the register address.
/// * `value` - the byte to write.
///
/// # Returns
///
/// The bytes to clock out, target byte first.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::spi::{write, TARGET_CONCENTRATOR};
///
/// // The version register lives at 0x5606.
/// assert_eq!(write(TARGET_CONCENTRATOR, 0x5606, 0x10), [0x00, 0xd6, 0x06, 0x10]);
/// ```
#[must_use]
pub const fn write(target: u8, address: u16, value: u8) -> [u8; WRITE_LEN] {
    [
        target,
        WRITE_ACCESS | ((address >> 8) as u8 & 0x7f),
        (address & 0xff) as u8,
        value,
    ]
}

/// Builds the transfer that reads one byte from a register.
///
/// # Arguments
///
/// * `target` - which chip the transfer is for.
/// * `address` - the register address.
///
/// # Returns
///
/// The bytes to clock out. The chip answers in the last byte of what comes back, which
/// [`read_value`] takes.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::spi::{read, read_value, TARGET_CONCENTRATOR};
///
/// let out = read(TARGET_CONCENTRATOR, 0x5606);
/// assert_eq!(out, [0x00, 0x56, 0x06, 0x00, 0x00]);
///
/// // What the bus returned while those bytes went out.
/// assert_eq!(read_value(&[0x00, 0x00, 0x00, 0x00, 0x10]), Some(0x10));
/// ```
#[must_use]
pub const fn read(target: u8, address: u16) -> [u8; READ_LEN] {
    [
        target,
        READ_ACCESS | ((address >> 8) as u8 & 0x7f),
        (address & 0xff) as u8,
        0x00,
        0x00,
    ]
}

/// Takes the byte a read transfer brought back.
///
/// # Arguments
///
/// * `received` - everything the bus returned during a [`read`] transfer.
///
/// # Returns
///
/// The register byte, or `None` when the transfer is too short to hold one.
#[must_use]
pub fn read_value(received: &[u8]) -> Option<u8> {
    received.last().copied()
}

/// Builds the header that precedes the data of a burst write.
///
/// # Arguments
///
/// * `target` - which chip the transfer is for.
/// * `address` - where the first byte lands.
///
/// # Returns
///
/// The header. The data follows it in the same transfer, unbroken.
#[must_use]
pub const fn burst_write_header(target: u8, address: u16) -> [u8; BURST_WRITE_HEADER_LEN] {
    [
        target,
        WRITE_ACCESS | ((address >> 8) as u8 & 0x7f),
        (address & 0xff) as u8,
    ]
}

/// Builds the header that precedes the data of a burst read.
///
/// # Arguments
///
/// * `target` - which chip the transfer is for.
/// * `address` - where the first byte is read from.
///
/// # Returns
///
/// The header, which carries the turnaround byte the chip needs before it answers.
#[must_use]
pub const fn burst_read_header(target: u8, address: u16) -> [u8; BURST_READ_HEADER_LEN] {
    [
        target,
        READ_ACCESS | ((address >> 8) as u8 & 0x7f),
        (address & 0xff) as u8,
        0x00,
    ]
}

/// Walks a long transfer as the chunks it actually goes out in.
///
/// A burst longer than [`BURST_CHUNK`] is sent as several, each with a header of its own and
/// an address advanced by everything already written.
///
/// # Arguments
///
/// * `address` - where the first byte lands.
/// * `len` - how many bytes there are in all.
///
/// # Returns
///
/// The address and length of each chunk, in the order they go out.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::spi::chunks;
///
/// // The microcontroller firmware is eight kilobytes, which is eight full chunks.
/// let plan: Vec<_> = chunks(0x0000, 8192).collect();
/// assert_eq!(plan.len(), 8);
/// assert_eq!(plan[0], (0x0000, 1024));
/// assert_eq!(plan[7], (0x1c00, 1024));
/// ```
pub fn chunks(address: u16, len: usize) -> impl Iterator<Item = (u16, usize)> {
    let mut at = address;
    let mut left = len;
    core::iter::from_fn(move || {
        if left == 0 {
            return None;
        }
        let taken = if left > BURST_CHUNK {
            BURST_CHUNK
        } else {
            left
        };
        let chunk = (at, taken);
        at = at.wrapping_add(taken as u16);
        left -= taken;
        Some(chunk)
    })
}

/// Reads the bits a register holds out of the byte it lives in.
///
/// # Arguments
///
/// * `byte` - the byte the register was read from.
/// * `offset` - which bit the field starts at.
/// * `width` - how many bits it spans.
///
/// # Returns
///
/// The field, shifted down to where it can be read as a number.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::spi::field;
///
/// // The checksum status sits in the top four bits of the OTP status byte.
/// assert_eq!(field(0xa0, 4, 4), 0x0a);
/// ```
#[must_use]
pub const fn field(byte: u8, offset: u8, width: u8) -> u8 {
    if width >= 8 {
        return byte >> offset;
    }
    (byte >> offset) & ((1u8 << width) - 1)
}

/// Puts a value into the bits a register holds, leaving the rest of the byte alone.
///
/// A register narrower than a byte shares that byte with its neighbors, so writing one means
/// reading the byte, replacing those bits, and writing it back. This is the middle step.
///
/// # Arguments
///
/// * `byte` - the byte as it reads now.
/// * `offset` - which bit the field starts at.
/// * `width` - how many bits it spans.
/// * `value` - what to put there.
///
/// # Returns
///
/// The byte to write back.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::spi::with_field;
///
/// // Setting the host programming bit leaves the clock enable beside it untouched.
/// assert_eq!(with_field(0b0001_0000, 1, 1, 1), 0b0001_0010);
/// ```
#[must_use]
pub const fn with_field(byte: u8, offset: u8, width: u8, value: u8) -> u8 {
    let mask = if width >= 8 {
        0xffu8
    } else {
        ((1u8 << width) - 1) << offset
    };
    (byte & !mask) | ((value << offset) & mask)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The transfers the reference implementation clocks out, byte for byte.
    #[test]
    fn a_write_is_four_bytes_with_the_access_bit_set() {
        // The address splits across two bytes, and the top bit of the first says write.
        assert_eq!(
            write(TARGET_CONCENTRATOR, 0x5606, 0x10),
            [0x00, 0x80 | 0x56, 0x06, 0x10]
        );
        assert_eq!(
            write(TARGET_RADIO_A, 0x0000, 0xff),
            [0x01, 0x80, 0x00, 0xff]
        );
    }

    #[test]
    fn a_read_is_five_bytes_and_answers_in_the_last() {
        assert_eq!(read(TARGET_CONCENTRATOR, 0x6180), [0x00, 0x61, 0x80, 0, 0]);
        assert_eq!(read(TARGET_RADIO_B, 0x1234), [0x02, 0x12, 0x34, 0, 0]);

        // The bus returns as many bytes as went out; the register is the last of them.
        assert_eq!(read_value(&[0, 0, 0, 0, 0x42]), Some(0x42));
        assert_eq!(read_value(&[]), None);
    }

    #[test]
    fn the_address_keeps_only_fifteen_bits() {
        // The top bit of the command byte is the access bit, so an address cannot reach it.
        let framed = write(TARGET_CONCENTRATOR, 0xffff, 0x00);
        assert_eq!(framed[1], 0xff);
        assert_eq!(framed[1] & 0x80, WRITE_ACCESS);

        let framed = read(TARGET_CONCENTRATOR, 0xffff);
        assert_eq!(framed[1], 0x7f);
    }

    #[test]
    fn a_burst_header_carries_no_data_of_its_own() {
        assert_eq!(
            burst_write_header(TARGET_CONCENTRATOR, 0x2000),
            [0x00, 0x80 | 0x20, 0x00]
        );
        // A read needs one more byte, for the chip to turn the bus around.
        assert_eq!(
            burst_read_header(TARGET_CONCENTRATOR, 0x2000),
            [0x00, 0x20, 0x00, 0x00]
        );
    }

    #[test]
    fn a_long_burst_is_split_and_the_address_follows_it() {
        let plan: Vec<_> = chunks(0x0000, 8192).collect();
        assert_eq!(plan.len(), 8);
        for (index, (at, len)) in plan.iter().enumerate() {
            assert_eq!(*len, BURST_CHUNK);
            assert_eq!(*at as usize, index * BURST_CHUNK);
        }

        // A short burst is one chunk, and a partial tail keeps its real length.
        assert_eq!(chunks(0x4000, 10).collect::<Vec<_>>(), [(0x4000, 10)]);
        assert_eq!(
            chunks(0x0000, 1025).collect::<Vec<_>>(),
            [(0x0000, 1024), (0x0400, 1)]
        );
        assert_eq!(chunks(0x0000, 0).count(), 0);
    }

    #[test]
    fn a_field_reads_and_writes_without_touching_its_neighbors() {
        // The AGC control byte holds five registers, so each one has to be picked out.
        let byte = 0b0001_0101;
        assert_eq!(field(byte, 4, 1), 1); // clock enable
        assert_eq!(field(byte, 2, 1), 1); // clear
        assert_eq!(field(byte, 1, 1), 0); // host programming
        assert_eq!(field(byte, 0, 1), 1); // parity error

        let set = with_field(byte, 1, 1, 1);
        assert_eq!(set, 0b0001_0111);
        assert_eq!(field(set, 4, 1), 1);
        assert_eq!(field(set, 0, 1), 1);

        let cleared = with_field(set, 2, 1, 0);
        assert_eq!(cleared, 0b0001_0011);

        // A whole byte reads and writes as itself.
        assert_eq!(field(0xab, 0, 8), 0xab);
        assert_eq!(with_field(0x00, 0, 8, 0xab), 0xab);
    }
}
