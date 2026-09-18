//! Fragmented data block transport, TS004-2.0.0.
//!
//! A firmware image does not fit in a LoRaWAN frame, so it goes across in pieces. The server
//! cuts the block into `nb_frag` fragments of `frag_size` bytes and sends them one per
//! downlink, then keeps sending: every fragment after the first `nb_frag` is a coded one,
//! the exclusive-or of a pseudo-random half of the originals. A device that missed some of
//! the originals can solve for them from any `nb_frag` independent fragments it did hear, so
//! a session survives losses without anyone asking for a retransmission.
//!
//! - [`parity_line`] and [`prbs23`] build the matrix rows those coded fragments come from.
//! - [`Fragmenter`] cuts a block up and produces any fragment of the session, coded or not.
//! - [`Defragmenter`] puts one back together from whatever arrives, in storage the caller
//!   provides: it never allocates, and it tells you how much it needs.
//! - [`FragCommand`] carries the session setup, status, delete and acknowledgment commands
//!   on port [`PORT`], and [`data_block_int_key`] with [`BlockMic`] check the reassembled
//!   block against the code the setup carried.
//!
//! The coded fragments a device must agree with its server about are seeded from the index
//! among the coded fragments, not the index within the whole session. That is what Semtech's
//! LoRa Basics Modem and ChirpStack's fragmentation server both do, and the two have to
//! agree for any of this to work. The MATLAB in the specification's appendix reads as though
//! the whole-session index were the seed.
//!
//! # Examples
//!
//! A block goes across in five fragments, two are lost, and two coded fragments rebuild it:
//!
//! ```
//! use pamoja_lorawan::packages::fragment::{Defragmenter, Fragmenter};
//!
//! let block = b"the quick brown fox jumps!";
//! let sender = Fragmenter::new(block, 6)?;
//! assert_eq!((sender.nb_frag(), sender.padding()), (5, 4));
//!
//! let mut store = [0u8; 30];
//! let mut matrix = [0u8; 64];
//! let mut receiver = Defragmenter::new(sender.nb_frag(), 6, &mut store, &mut matrix)?;
//!
//! // Fragments 3 and 4 never arrive; the coded ones that follow stand in for them.
//! let mut piece = [0u8; 6];
//! for n in [1, 2, 5, 6, 8] {
//!     sender.fragment(n, &mut piece)?;
//!     receiver.fragment(n, &piece)?;
//! }
//! assert!(receiver.done());
//! assert_eq!(&receiver.block()[..block.len()], block);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use super::{payload, write, PackageVersion};
use crate::crypto::Cipher;
use crate::{Direction, LorawanError};

/// The port the fragmented data block transport package is spoken on, section 3.
pub const PORT: u8 = 201;

/// The identifier of this package, section 3.
pub const PACKAGE: u8 = 3;

/// The version of this package implemented here, section 3.
pub const VERSION: u8 = 2;

/// How many fragmentation sessions a device may hold at once, section 3.3.
pub const SESSIONS: usize = 4;

/// The most fragments one session carries, which the fourteen-bit index bounds.
pub const MAX_FRAGMENTS: u16 = (1 << 14) - 1;

/// The longest command of this package other than a data fragment, in bytes.
pub const MAX_COMMAND: usize = 17;

const CID_PACKAGE_VERSION: u8 = 0x00;
const CID_SESSION_STATUS: u8 = 0x01;
const CID_SESSION_SETUP: u8 = 0x02;
const CID_SESSION_DELETE: u8 = 0x03;
const CID_BLOCK_RECEIVED: u8 = 0x04;
const CID_DATA_FRAGMENT: u8 = 0x08;

/// Steps the 23-bit pseudo-random sequence the parity matrix is drawn from, appendix A.1.
///
/// # Arguments
///
/// * `x` - the current value.
///
/// # Returns
///
/// The next one.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::packages::fragment::prbs23;
///
/// assert_eq!(prbs23(1), 1 << 22, "a one shifts out and feeds back");
/// ```
#[must_use]
pub const fn prbs23(x: u32) -> u32 {
    let b0 = x & 1;
    let b1 = (x >> 5) & 1;
    (x >> 1) + ((b0 ^ b1) << 22)
}

/// Builds one row of the parity matrix: which uncoded fragments a coded one is made of.
///
/// # Arguments
///
/// * `coded` - which coded fragment this is, counting from one. A session's fragment `n`
///   past its `nb_frag` uncoded ones is coded fragment `n`.
/// * `nb_frag` - how many uncoded fragments the block was cut into.
/// * `line` - a bit for every uncoded fragment, cleared and then filled in, little end
///   first. It must hold at least `nb_frag` bits.
///
/// # Returns
///
/// How many fragments the coded one is the exclusive-or of, which is half of `nb_frag`
/// rounded down.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::packages::fragment::parity_line;
///
/// let mut line = [0u8; 1];
/// assert_eq!(parity_line(1, 6, &mut line), 3);
/// assert_eq!(line[0] & 0b0011_1111, 0b0001_0011, "fragments 1, 2 and 5");
/// ```
pub fn parity_line(coded: u16, nb_frag: u16, line: &mut [u8]) -> usize {
    let m = u32::from(nb_frag);
    let bytes = (nb_frag as usize).div_ceil(8);
    for byte in line.iter_mut().take(bytes) {
        *byte = 0;
    }
    // A power of two makes the generator fall into patterns, so it draws from one more
    // than the count and throws away anything out of range, appendix A.1.
    let span = m + u32::from(m.is_power_of_two());
    let mut x = 1 + 1001 * u32::from(coded);
    let wanted = (nb_frag / 2) as usize;
    let mut set = 0;
    while set < wanted {
        let mut r = 1 << 16;
        while r >= m {
            x = prbs23(x);
            r = x % span;
        }
        let at = r as usize;
        if line[at / 8] & (1 << (at % 8)) == 0 {
            line[at / 8] |= 1 << (at % 8);
            set += 1;
        }
    }
    set
}

/// Whether a bit is set in a little-end-first bitset.
const fn bit(bits: &[u8], at: usize) -> bool {
    bits[at / 8] & (1 << (at % 8)) != 0
}

/// Sets or clears a bit in a little-end-first bitset.
const fn set_bit(bits: &mut [u8], at: usize, on: bool) {
    if on {
        bits[at / 8] |= 1 << (at % 8);
    } else {
        bits[at / 8] &= !(1 << (at % 8));
    }
}

/// Cuts a data block into the fragments of a session, and builds the coded ones.
///
/// The block is left where it is: nothing is copied until a fragment is asked for.
#[derive(Clone, Copy, Debug)]
pub struct Fragmenter<'a> {
    block: &'a [u8],
    frag_size: u8,
    nb_frag: u16,
    padding: u8,
}

impl<'a> Fragmenter<'a> {
    /// Prepares a block for a session.
    ///
    /// # Arguments
    ///
    /// * `block` - the data to send.
    /// * `frag_size` - how many bytes each fragment carries.
    ///
    /// # Returns
    ///
    /// The fragmenter, which says how many fragments the block takes and how much padding
    /// the last one needs.
    ///
    /// # Errors
    ///
    /// [`LorawanError::MalformedFrame`] for a fragment size of zero or a block that needs
    /// more than [`MAX_FRAGMENTS`] fragments.
    pub fn new(block: &'a [u8], frag_size: u8) -> Result<Fragmenter<'a>, LorawanError> {
        if frag_size == 0 {
            return Err(LorawanError::MalformedFrame);
        }
        let size = usize::from(frag_size);
        let nb_frag = block.len().div_ceil(size);
        if nb_frag == 0 || nb_frag > MAX_FRAGMENTS as usize {
            return Err(LorawanError::MalformedFrame);
        }
        Ok(Fragmenter {
            block,
            frag_size,
            nb_frag: nb_frag as u16,
            padding: (nb_frag * size - block.len()) as u8,
        })
    }

    /// How many uncoded fragments the block was cut into.
    ///
    /// # Returns
    ///
    /// The count, which the session setup carries as `NbFrag`.
    #[must_use]
    pub const fn nb_frag(&self) -> u16 {
        self.nb_frag
    }

    /// How many bytes of padding the last fragment carries.
    ///
    /// # Returns
    ///
    /// The count, which the session setup carries and the receiver strips.
    #[must_use]
    pub const fn padding(&self) -> u8 {
        self.padding
    }

    /// How many bytes each fragment carries.
    ///
    /// # Returns
    ///
    /// The size.
    #[must_use]
    pub const fn frag_size(&self) -> u8 {
        self.frag_size
    }

    /// Builds one fragment of the session.
    ///
    /// # Arguments
    ///
    /// * `n` - which fragment, counting from one. Up to `nb_frag` it is a piece of the
    ///   block; past that it is a coded fragment, the exclusive-or of a pseudo-random half
    ///   of the pieces.
    /// * `out` - where to write it, at least `frag_size` bytes.
    ///
    /// # Returns
    ///
    /// How many bytes were written, which is always `frag_size`.
    ///
    /// # Errors
    ///
    /// [`LorawanError::MalformedFrame`] for a fragment number of zero, and
    /// [`LorawanError::PayloadTooLong`] when the buffer is too small.
    pub fn fragment(&self, n: u16, out: &mut [u8]) -> Result<usize, LorawanError> {
        let size = usize::from(self.frag_size);
        let room = out.get_mut(..size).ok_or(LorawanError::PayloadTooLong)?;
        if n == 0 {
            return Err(LorawanError::MalformedFrame);
        }
        room.fill(0);
        if n <= self.nb_frag {
            self.piece(usize::from(n - 1), room);
            return Ok(size);
        }
        let mut line = [0u8; (MAX_FRAGMENTS as usize).div_ceil(8)];
        parity_line(n - self.nb_frag, self.nb_frag, &mut line);
        for index in 0..usize::from(self.nb_frag) {
            if bit(&line, index) {
                let mut piece = [0u8; 255];
                self.piece(index, &mut piece[..size]);
                for (out, byte) in room.iter_mut().zip(&piece[..size]) {
                    *out ^= byte;
                }
            }
        }
        Ok(size)
    }

    /// Copies one uncoded fragment out, padding the last one with zeros.
    fn piece(&self, index: usize, out: &mut [u8]) {
        let size = usize::from(self.frag_size);
        let at = index * size;
        let end = (at + size).min(self.block.len());
        out.fill(0);
        if at < self.block.len() {
            out[..end - at].copy_from_slice(&self.block[at..end]);
        }
    }
}

/// Why a fragmentation session could not be put back together.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FragError {
    /// The session parameters do not describe a session this build can run.
    Session,
    /// The caller's storage is too small for the session it was given.
    Storage,
    /// More uncoded fragments were lost than the caller left room to solve for.
    Memory,
    /// A fragment's number is outside the session.
    Fragment,
}

impl core::fmt::Display for FragError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FragError::Session => f.write_str("the fragmentation session is not one this runs"),
            FragError::Storage => f.write_str("the storage is too small for the session"),
            FragError::Memory => {
                f.write_str("more fragments were lost than there is room to solve for")
            }
            FragError::Fragment => f.write_str("the fragment is not part of the session"),
        }
    }
}

impl core::error::Error for FragError {}

/// How far a fragmentation session has got, small enough to write down.
///
/// A device that loses power partway through a firmware download keeps its block storage and
/// its working storage in flash; this is the handful of numbers it needs beside them to carry
/// on where it left off, rather than asking for the whole image again.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Progress {
    /// How many fragments arrived, coded, uncoded and repeated.
    pub received: u16,
    /// How many uncoded fragments were lost.
    pub lost: u16,
    /// How many rows of the system are filled in.
    pub rows: u16,
    /// The highest uncoded fragment number seen.
    pub highest: u16,
    /// Whether the block is whole.
    pub done: bool,
}

/// Puts a data block back together from the fragments that arrive, TS004-2.0.0 appendix A.2.
///
/// The uncoded fragments go straight into the caller's block storage. A coded fragment is
/// reduced against everything already known and, if it still says something new, kept as one
/// row of a triangular system over the fragments that were lost. Once there are as many rows
/// as there are losses, the system is solved back into the block.
///
/// Nothing is allocated. The block storage is `nb_frag * frag_size` bytes, and the matrix
/// storage bounds how many losses can be solved for: [`Defragmenter::matrix_len`] says how
/// much room a given number of losses takes.
pub struct Defragmenter<'a> {
    nb_frag: u16,
    frag_size: u8,
    block: &'a mut [u8],
    matrix: &'a mut [u8],
    max_lost: u16,
    lost: u16,
    rows: u16,
    received: u16,
    highest: u16,
    done: bool,
}

impl<'a> Defragmenter<'a> {
    /// How many bytes of matrix storage a session needs.
    ///
    /// # Arguments
    ///
    /// * `nb_frag` - how many uncoded fragments the block was cut into.
    /// * `max_lost` - the most uncoded fragments the session should survive losing.
    ///
    /// # Returns
    ///
    /// The byte count: a row for each loss, as wide as the losses; the map from row to
    /// fragment; and a bit for each uncoded fragment saying whether it is in hand.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::packages::fragment::Defragmenter;
    ///
    /// assert_eq!(Defragmenter::matrix_len(100, 8), 8 + 2 * 8 + 13);
    /// ```
    #[must_use]
    pub const fn matrix_len(nb_frag: u16, max_lost: u16) -> usize {
        let lost = max_lost as usize;
        lost * lost.div_ceil(8) + 2 * lost + (nb_frag as usize).div_ceil(8)
    }

    /// Starts a session.
    ///
    /// # Arguments
    ///
    /// * `nb_frag` - how many uncoded fragments the block was cut into.
    /// * `frag_size` - how many bytes each fragment carries.
    /// * `block` - where the block is put back together, `nb_frag * frag_size` bytes.
    /// * `matrix` - the working storage, whose size decides how many losses can be solved
    ///   for; [`Defragmenter::matrix_len`] sizes it.
    ///
    /// # Returns
    ///
    /// The session, with nothing received.
    ///
    /// # Errors
    ///
    /// [`FragError::Session`] for a session this build does not run, and
    /// [`FragError::Storage`] when the block storage is too small.
    pub fn new(
        nb_frag: u16,
        frag_size: u8,
        block: &'a mut [u8],
        matrix: &'a mut [u8],
    ) -> Result<Defragmenter<'a>, FragError> {
        Defragmenter::opened(nb_frag, frag_size, block, matrix, true)
    }

    /// Opens a session, clearing the storage or taking it as it is.
    fn opened(
        nb_frag: u16,
        frag_size: u8,
        block: &'a mut [u8],
        matrix: &'a mut [u8],
        clear: bool,
    ) -> Result<Defragmenter<'a>, FragError> {
        if nb_frag == 0 || nb_frag > MAX_FRAGMENTS || frag_size == 0 {
            return Err(FragError::Session);
        }
        let needed = usize::from(nb_frag) * usize::from(frag_size);
        if block.len() < needed {
            return Err(FragError::Storage);
        }
        // The largest number of losses this much matrix storage can carry.
        let mut max_lost = 0u16;
        while max_lost < nb_frag && Defragmenter::matrix_len(nb_frag, max_lost + 1) <= matrix.len()
        {
            max_lost += 1;
        }
        if Defragmenter::matrix_len(nb_frag, max_lost) > matrix.len() {
            return Err(FragError::Storage);
        }
        if clear {
            matrix.fill(0);
            block[..needed].fill(0);
        }
        Ok(Defragmenter {
            nb_frag,
            frag_size,
            block,
            matrix,
            max_lost,
            lost: 0,
            rows: 0,
            received: 0,
            highest: 0,
            done: false,
        })
    }

    /// Starts a session again where one left off.
    ///
    /// The storage must be the same two buffers the earlier session held, with whatever they
    /// had in them, and the progress must be the one it reported. Nothing is cleared.
    ///
    /// # Arguments
    ///
    /// * `nb_frag` - how many uncoded fragments the block was cut into.
    /// * `frag_size` - how many bytes each fragment carries.
    /// * `block` - the block storage, as the earlier session left it.
    /// * `matrix` - the working storage, as the earlier session left it.
    /// * `progress` - what [`Defragmenter::progress`] reported.
    ///
    /// # Returns
    ///
    /// The session, carrying on.
    ///
    /// # Errors
    ///
    /// [`FragError::Session`] for a session this build does not run, and
    /// [`FragError::Storage`] when the storage is too small.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_lorawan::packages::fragment::{Defragmenter, Fragmenter};
    ///
    /// let block = b"a block that outlives the power";
    /// let sender = Fragmenter::new(block, 8)?;
    /// let mut store = [0u8; 32];
    /// let mut matrix = [0u8; 64];
    ///
    /// let mut piece = [0u8; 8];
    /// let mut session = Defragmenter::new(sender.nb_frag(), 8, &mut store, &mut matrix)?;
    /// sender.fragment(1, &mut piece)?;
    /// session.fragment(1, &piece)?;
    /// let progress = session.progress();
    ///
    /// // The power goes. The two buffers were in flash; this is all that was beside them.
    /// let mut session =
    ///     Defragmenter::resumed(sender.nb_frag(), 8, &mut store, &mut matrix, progress)?;
    /// for n in 2..=sender.nb_frag() {
    ///     sender.fragment(n, &mut piece)?;
    ///     session.fragment(n, &piece)?;
    /// }
    /// assert!(session.done());
    /// assert_eq!(&session.block()[..block.len()], block);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn resumed(
        nb_frag: u16,
        frag_size: u8,
        block: &'a mut [u8],
        matrix: &'a mut [u8],
        progress: Progress,
    ) -> Result<Defragmenter<'a>, FragError> {
        let mut session = Defragmenter::opened(nb_frag, frag_size, block, matrix, false)?;
        session.received = progress.received;
        session.lost = progress.lost;
        session.rows = progress.rows;
        session.highest = progress.highest;
        session.done = progress.done;
        Ok(session)
    }

    /// How far the session has got, to be kept beside its storage.
    ///
    /// # Returns
    ///
    /// The numbers [`Defragmenter::resumed`] takes.
    #[must_use]
    pub const fn progress(&self) -> Progress {
        Progress {
            received: self.received,
            lost: self.lost,
            rows: self.rows,
            highest: self.highest,
            done: self.done,
        }
    }

    /// The block, as far as it has been put back together.
    ///
    /// # Returns
    ///
    /// The bytes, which are the whole block once [`Defragmenter::done`] is true, padding
    /// and all. The padding the session setup named is at the end.
    #[must_use]
    pub fn block(&self) -> &[u8] {
        let needed = usize::from(self.nb_frag) * usize::from(self.frag_size);
        &self.block[..needed]
    }

    /// Whether the whole block is there.
    ///
    /// # Returns
    ///
    /// `true` once every uncoded fragment has arrived or been solved for.
    #[must_use]
    pub const fn done(&self) -> bool {
        self.done
    }

    /// How many fragments arrived, coded and uncoded, including repeats.
    ///
    /// # Returns
    ///
    /// The count, which a status answer reports as `NbFragReceived`.
    #[must_use]
    pub const fn received(&self) -> u16 {
        self.received
    }

    /// How many uncoded fragments are still missing.
    ///
    /// # Returns
    ///
    /// The count, which is also the least number of further independent fragments needed.
    /// A status answer reports it as `MissingFrag`, capped at 255.
    #[must_use]
    pub const fn missing(&self) -> u16 {
        if self.done {
            0
        } else {
            self.lost - self.rows + (self.nb_frag - self.highest)
        }
    }

    /// How many losses this session can still solve for.
    ///
    /// # Returns
    ///
    /// The count the caller's matrix storage allows.
    #[must_use]
    pub const fn max_lost(&self) -> u16 {
        self.max_lost
    }

    /// Takes one fragment of the session.
    ///
    /// # Arguments
    ///
    /// * `n` - which fragment, counting from one.
    /// * `data` - its bytes, `frag_size` of them.
    ///
    /// # Returns
    ///
    /// Whether the block is now complete.
    ///
    /// # Errors
    ///
    /// [`FragError::Fragment`] for a fragment outside the session or of the wrong size, and
    /// [`FragError::Memory`] when more uncoded fragments were lost than there is room to
    /// solve for.
    pub fn fragment(&mut self, n: u16, data: &[u8]) -> Result<bool, FragError> {
        let size = usize::from(self.frag_size);
        if n == 0 || data.len() < size {
            return Err(FragError::Fragment);
        }
        self.received = self.received.saturating_add(1);
        if self.done {
            return Ok(true);
        }
        if n <= self.nb_frag {
            self.uncoded(n, &data[..size]);
            return Ok(self.done);
        }
        self.coded(n - self.nb_frag, &data[..size])
    }

    /// Stores an uncoded fragment and notes what was lost before it.
    fn uncoded(&mut self, n: u16, data: &[u8]) {
        let size = usize::from(self.frag_size);
        let index = usize::from(n - 1);
        if !self.arrived(index) {
            self.block[index * size..index * size + size].copy_from_slice(data);
            self.mark(index);
        }
        if n > self.highest {
            // Everything between the last one seen and this one was lost.
            for missing in self.highest..n - 1 {
                if !self.arrived(usize::from(missing)) {
                    self.note_lost(missing);
                }
            }
            self.highest = n;
        }
        if self.highest == self.nb_frag && self.lost == 0 {
            self.done = true;
        }
    }

    /// Takes a coded fragment into the system, solving the block when it completes it.
    fn coded(&mut self, coded: u16, data: &[u8]) -> Result<bool, FragError> {
        // Every uncoded fragment has been sent by now, so anything not seen is lost.
        for missing in self.highest..self.nb_frag {
            if !self.arrived(usize::from(missing)) {
                self.note_lost(missing);
            }
        }
        self.highest = self.nb_frag;
        if self.lost == 0 {
            self.done = true;
            return Ok(true);
        }
        if self.lost > self.max_lost {
            return Err(FragError::Memory);
        }

        let size = usize::from(self.frag_size);
        let mut line = [0u8; (MAX_FRAGMENTS as usize).div_ceil(8)];
        parity_line(coded, self.nb_frag, &mut line);

        // Reduce against the fragments already in hand, leaving a combination of the lost
        // ones only, and fold their data into the coded fragment as we go.
        let mut piece = [0u8; 255];
        piece[..size].copy_from_slice(data);
        let mut row = [0u8; ((MAX_FRAGMENTS as usize) / 8) + 1];
        let lost = usize::from(self.lost);
        for at in 0..lost {
            let fragment = usize::from(self.lost_at(at));
            if bit(&line, fragment) {
                set_bit(&mut row, at, true);
            }
        }
        for index in 0..usize::from(self.nb_frag) {
            if bit(&line, index) && self.arrived(index) {
                for (out, byte) in piece[..size]
                    .iter_mut()
                    .zip(&self.block[index * size..index * size + size])
                {
                    *out ^= byte;
                }
            }
        }

        // Reduce against the rows already stored, which are in echelon form.
        for at in 0..lost {
            if !bit(&row, at) || !self.row_used(at) {
                continue;
            }
            self.xor_row(at, &mut row);
            let slot = usize::from(self.lost_at(at));
            for (out, byte) in piece[..size]
                .iter_mut()
                .zip(&self.block[slot * size..slot * size + size])
            {
                *out ^= byte;
            }
        }

        // What is left either says nothing new, or becomes a new row.
        let Some(first) = (0..lost).find(|at| bit(&row, *at)) else {
            return Ok(false);
        };
        self.store_row(first, &row);
        let slot = usize::from(self.lost_at(first));
        self.block[slot * size..slot * size + size].copy_from_slice(&piece[..size]);
        self.rows += 1;
        if self.rows == self.lost {
            self.solve();
            self.done = true;
        }
        Ok(self.done)
    }

    /// Runs the triangular system back into the block, appendix A.2 step 5.
    fn solve(&mut self) {
        let size = usize::from(self.frag_size);
        let lost = usize::from(self.lost);
        for at in (0..lost).rev() {
            let slot = usize::from(self.lost_at(at));
            for other in at + 1..lost {
                if self.row_bit(at, other) {
                    let source = usize::from(self.lost_at(other));
                    for byte in 0..size {
                        self.block[slot * size + byte] ^= self.block[source * size + byte];
                    }
                }
            }
            self.mark(slot);
        }
    }

    /// The layout of the working storage: the row bits, then the lost list, then the
    /// bitset of which uncoded fragments arrived.
    const fn row_bytes(&self) -> usize {
        (self.max_lost as usize).div_ceil(8)
    }

    /// Where the list of lost fragments starts.
    const fn lost_at_offset(&self) -> usize {
        self.max_lost as usize * self.row_bytes()
    }

    /// Where the bitset of arrived fragments starts.
    const fn arrived_offset(&self) -> usize {
        self.lost_at_offset() + 2 * self.max_lost as usize
    }

    /// Whether an uncoded fragment has arrived or been solved for.
    fn arrived(&self, index: usize) -> bool {
        let at = self.arrived_offset() + index / 8;
        self.matrix[at] & (1 << (index % 8)) != 0
    }

    /// Notes that an uncoded fragment is in hand.
    fn mark(&mut self, index: usize) {
        let at = self.arrived_offset() + index / 8;
        self.matrix[at] |= 1 << (index % 8);
    }

    /// Notes that an uncoded fragment was lost, giving it a row of the system.
    fn note_lost(&mut self, fragment: u16) {
        if self.lost >= self.max_lost {
            self.lost += 1;
            return;
        }
        let base = self.lost_at_offset() + 2 * usize::from(self.lost);
        self.matrix[base] = (fragment & 0xFF) as u8;
        self.matrix[base + 1] = (fragment >> 8) as u8;
        self.lost += 1;
    }

    /// Which fragment a row of the system stands for.
    fn lost_at(&self, at: usize) -> u16 {
        let base = self.lost_at_offset() + 2 * at;
        u16::from(self.matrix[base]) | (u16::from(self.matrix[base + 1]) << 8)
    }

    /// Whether a row of the system has been filled in.
    fn row_used(&self, at: usize) -> bool {
        self.row_bit(at, at)
    }

    /// One bit of a stored row.
    fn row_bit(&self, row: usize, column: usize) -> bool {
        let base = row * self.row_bytes();
        self.matrix[base + column / 8] & (1 << (column % 8)) != 0
    }

    /// Exclusive-ors a stored row into a working one.
    fn xor_row(&self, row: usize, into: &mut [u8]) {
        let base = row * self.row_bytes();
        let stored = &self.matrix[base..base + self.row_bytes()];
        for (out, byte) in into.iter_mut().zip(stored) {
            *out ^= byte;
        }
    }

    /// Writes a working row into the system.
    fn store_row(&mut self, row: usize, from: &[u8]) {
        let width = self.row_bytes();
        let base = row * width;
        self.matrix[base..base + width].copy_from_slice(&from[..width]);
    }
}

/// Derives the key that signs a data block, section 3.3.
///
/// # Arguments
///
/// * `root_key` - the device's `GenAppKey` on LoRaWAN 1.0.x, or its `AppKey` on 1.1.
///
/// # Returns
///
/// The `DataBlockIntKey`, which only ever signs data blocks.
///
/// # Examples
///
/// ```
/// use pamoja_lorawan::packages::fragment::data_block_int_key;
///
/// let key = data_block_int_key(&[0x2B; 16]);
/// assert_ne!(key, [0x2B; 16], "it is a key of its own");
/// ```
#[must_use]
pub fn data_block_int_key(root_key: &[u8; 16]) -> [u8; 16] {
    let mut block = [0u8; 16];
    block[0] = 0x30;
    Cipher::new(root_key).encrypt_block(&block)
}

/// The integrity code over a whole data block, section 3.3.
///
/// The block is signed a piece at a time, so a device checks an image it never holds twice.
pub struct BlockMic<'a> {
    stream: crate::crypto::CmacStream<'a>,
}

/// The cipher a block code is taken under, which the code borrows.
pub struct BlockMicKey {
    cipher: Cipher,
}

impl BlockMicKey {
    /// Prepares the key a block's code is taken under.
    ///
    /// # Arguments
    ///
    /// * `data_block_int_key` - the key [`data_block_int_key`] derived.
    ///
    /// # Returns
    ///
    /// The prepared key.
    #[must_use]
    pub fn new(data_block_int_key: &[u8; 16]) -> BlockMicKey {
        BlockMicKey {
            cipher: Cipher::new(data_block_int_key),
        }
    }

    /// Starts a code over one session's block.
    ///
    /// # Arguments
    ///
    /// * `session_cnt` - the session counter the setup carried.
    /// * `frag_index` - which of the device's sessions this is.
    /// * `descriptor` - the four bytes the setup described the block with.
    /// * `block_len` - the length of the block in bytes, without its padding.
    ///
    /// # Returns
    ///
    /// The code, waiting for the block itself.
    #[must_use]
    pub fn start(
        &self,
        session_cnt: u16,
        frag_index: u8,
        descriptor: [u8; 4],
        block_len: u32,
    ) -> BlockMic<'_> {
        let mut b0 = [0u8; 16];
        b0[0] = 0x49;
        b0[1..3].copy_from_slice(&session_cnt.to_le_bytes());
        b0[3] = frag_index;
        b0[4..8].copy_from_slice(&descriptor);
        b0[12..16].copy_from_slice(&block_len.to_le_bytes());
        let mut stream = self.cipher.cmac_stream();
        stream.update(&b0);
        BlockMic { stream }
    }
}

impl BlockMic<'_> {
    /// Adds a piece of the block.
    ///
    /// # Arguments
    ///
    /// * `data` - the next bytes of the block, in order, without any padding.
    pub fn update(&mut self, data: &[u8]) {
        self.stream.update(data);
    }

    /// Finishes the code.
    ///
    /// # Returns
    ///
    /// The four bytes a session setup carries.
    #[must_use]
    pub fn finish(self) -> [u8; 4] {
        let full = self.stream.finish();
        [full[0], full[1], full[2], full[3]]
    }
}

/// What a device makes of a session setup, table 14.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SetupStatus {
    /// The device does not run the fragmentation algorithm the setup named.
    pub unsupported_algorithm: bool,
    /// The device has not the memory for a block that size.
    pub not_enough_memory: bool,
    /// The session index is not one the device keeps.
    pub unsupported_index: bool,
    /// The descriptor is not one the device accepts.
    pub wrong_descriptor: bool,
    /// The session counter repeats one already used for this index.
    pub session_replay: bool,
    /// Which session this answer is about.
    pub frag_index: u8,
}

impl SetupStatus {
    /// Whether the setup was accepted.
    ///
    /// # Returns
    ///
    /// `true` when no error bit is set.
    #[must_use]
    pub const fn accepted(&self) -> bool {
        !self.unsupported_algorithm
            && !self.not_enough_memory
            && !self.unsupported_index
            && !self.wrong_descriptor
            && !self.session_replay
    }

    /// Reads the coded status.
    const fn from_bits(bits: u8) -> SetupStatus {
        SetupStatus {
            unsupported_algorithm: bits & 0x01 != 0,
            not_enough_memory: bits & 0x02 != 0,
            unsupported_index: bits & 0x04 != 0,
            wrong_descriptor: bits & 0x08 != 0,
            session_replay: bits & 0x10 != 0,
            frag_index: bits >> 6,
        }
    }

    /// Writes the coded status.
    const fn bits(&self) -> u8 {
        (self.unsupported_algorithm as u8)
            | ((self.not_enough_memory as u8) << 1)
            | ((self.unsupported_index as u8) << 2)
            | ((self.wrong_descriptor as u8) << 3)
            | ((self.session_replay as u8) << 4)
            | ((self.frag_index & 0x03) << 6)
    }
}

/// One command of the fragmented data block transport package.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FragCommand<'a> {
    /// The server asking which version of this package the device implements.
    PackageVersionReq,
    /// The device's answer, section 3.1.
    PackageVersionAns(PackageVersion),
    /// The server asking how a session is going, section 3.2.
    FragSessionStatusReq {
        /// Which session.
        frag_index: u8,
        /// Whether every device answers, or only those still missing fragments.
        all_participants: bool,
    },
    /// The device's answer, section 3.2.
    FragSessionStatusAns {
        /// Which session.
        frag_index: u8,
        /// How many fragments arrived, coded, uncoded and repeated.
        received: u16,
        /// How many uncoded fragments are still missing, capped at 255.
        missing: u8,
        /// Whether the block's integrity code did not check out.
        mic_error: bool,
        /// Whether the session ran out of memory to defragment with.
        memory_error: bool,
        /// Whether the session does not exist on this device.
        no_session: bool,
    },
    /// The server setting a session up, section 3.3.
    FragSessionSetupReq {
        /// Which session, 0 to 3.
        frag_index: u8,
        /// Which multicast groups may feed it, a bit for each.
        mc_group_bit_mask: u8,
        /// How many uncoded fragments the block was cut into.
        nb_frag: u16,
        /// How many bytes each fragment carries.
        frag_size: u8,
        /// Whether the device reports the block once it has it.
        ack_reception: bool,
        /// Which fragmentation algorithm to run.
        frag_algo: u8,
        /// The coded spread of the delay before a device answers.
        block_ack_delay: u8,
        /// How many bytes of padding the last fragment carries.
        padding: u8,
        /// The four bytes the server describes the block with.
        descriptor: [u8; 4],
        /// The session counter, which must rise for each new block.
        session_cnt: u16,
        /// The code over the block the device checks once it has it all.
        mic: [u8; 4],
    },
    /// The device's answer, section 3.3.
    FragSessionSetupAns(SetupStatus),
    /// The server deleting a session, section 3.4.
    FragSessionDeleteReq {
        /// Which session.
        frag_index: u8,
    },
    /// The device's answer, section 3.4.
    FragSessionDeleteAns {
        /// Which session.
        frag_index: u8,
        /// Whether the device had no such session.
        no_session: bool,
    },
    /// The device reporting a block it has completely received, section 3.5.
    FragDataBlockReceivedReq {
        /// Which session.
        frag_index: u8,
        /// Whether the block's integrity code did not check out.
        mic_error: bool,
    },
    /// The server's acknowledgment, section 3.5.
    FragDataBlockReceivedAns {
        /// Which session.
        frag_index: u8,
    },
    /// One fragment of a block, section 3.6.
    DataFragment {
        /// Which session.
        frag_index: u8,
        /// Which fragment, counting from one.
        n: u16,
        /// Its bytes.
        data: &'a [u8],
    },
}

impl<'a> FragCommand<'a> {
    /// The identifier this command travels under.
    ///
    /// # Returns
    ///
    /// The identifier, 0x00 to 0x04 or 0x08 for a data fragment.
    #[must_use]
    pub const fn cid(&self) -> u8 {
        match self {
            FragCommand::PackageVersionReq | FragCommand::PackageVersionAns(_) => {
                CID_PACKAGE_VERSION
            }
            FragCommand::FragSessionStatusReq { .. } | FragCommand::FragSessionStatusAns { .. } => {
                CID_SESSION_STATUS
            }
            FragCommand::FragSessionSetupReq { .. } | FragCommand::FragSessionSetupAns(_) => {
                CID_SESSION_SETUP
            }
            FragCommand::FragSessionDeleteReq { .. } | FragCommand::FragSessionDeleteAns { .. } => {
                CID_SESSION_DELETE
            }
            FragCommand::FragDataBlockReceivedReq { .. }
            | FragCommand::FragDataBlockReceivedAns { .. } => CID_BLOCK_RECEIVED,
            FragCommand::DataFragment { .. } => CID_DATA_FRAGMENT,
        }
    }

    /// Which way this command travels.
    ///
    /// # Returns
    ///
    /// [`Direction::Uplink`] for what a device sends, [`Direction::Downlink`] for what a
    /// server sends.
    #[must_use]
    pub const fn direction(&self) -> Direction {
        match self {
            FragCommand::PackageVersionAns(_)
            | FragCommand::FragSessionStatusAns { .. }
            | FragCommand::FragSessionSetupAns(_)
            | FragCommand::FragSessionDeleteAns { .. }
            | FragCommand::FragDataBlockReceivedReq { .. } => Direction::Uplink,
            _ => Direction::Downlink,
        }
    }

    /// Reads one command from the front of a message.
    ///
    /// A data fragment takes the whole message, as section 3 asks, so it reads to the end.
    ///
    /// # Arguments
    ///
    /// * `direction` - which way the frame carrying it traveled.
    /// * `bytes` - the message, from this command's identifier on.
    ///
    /// # Returns
    ///
    /// The command and how many bytes it took.
    ///
    /// # Errors
    ///
    /// [`LorawanError::MalformedFrame`] when the message ends inside the command, and
    /// [`LorawanError::UnknownCommand`] for an identifier this package does not define.
    pub fn parse(
        direction: Direction,
        bytes: &'a [u8],
    ) -> Result<(FragCommand<'a>, usize), LorawanError> {
        let cid = *bytes.first().ok_or(LorawanError::MalformedFrame)?;
        let up = matches!(direction, Direction::Uplink);
        match (cid, up) {
            (CID_PACKAGE_VERSION, false) => Ok((FragCommand::PackageVersionReq, 1)),
            (CID_PACKAGE_VERSION, true) => {
                let (fields, taken) = payload(bytes, 2)?;
                Ok((
                    FragCommand::PackageVersionAns(PackageVersion {
                        package: fields[0],
                        version: fields[1],
                    }),
                    taken,
                ))
            }
            (CID_SESSION_STATUS, false) => {
                let (fields, taken) = payload(bytes, 1)?;
                Ok((
                    FragCommand::FragSessionStatusReq {
                        frag_index: (fields[0] >> 1) & 0x03,
                        all_participants: fields[0] & 0x01 != 0,
                    },
                    taken,
                ))
            }
            (CID_SESSION_STATUS, true) => {
                let (fields, taken) = payload(bytes, 4)?;
                let received = u16::from_le_bytes([fields[1], fields[2]]);
                Ok((
                    FragCommand::FragSessionStatusAns {
                        frag_index: (received >> 14) as u8,
                        received: received & 0x3FFF,
                        missing: fields[3],
                        mic_error: fields[0] & 0x02 != 0,
                        memory_error: fields[0] & 0x01 != 0,
                        no_session: fields[0] & 0x04 != 0,
                    },
                    taken,
                ))
            }
            (CID_SESSION_SETUP, false) => {
                let (fields, taken) = payload(bytes, 16)?;
                Ok((
                    FragCommand::FragSessionSetupReq {
                        frag_index: (fields[0] >> 4) & 0x03,
                        mc_group_bit_mask: fields[0] & 0x0F,
                        nb_frag: u16::from_le_bytes([fields[1], fields[2]]),
                        frag_size: fields[3],
                        ack_reception: fields[4] & 0x40 != 0,
                        frag_algo: (fields[4] >> 3) & 0x07,
                        block_ack_delay: fields[4] & 0x07,
                        padding: fields[5],
                        descriptor: [fields[6], fields[7], fields[8], fields[9]],
                        session_cnt: u16::from_le_bytes([fields[10], fields[11]]),
                        mic: [fields[12], fields[13], fields[14], fields[15]],
                    },
                    taken,
                ))
            }
            (CID_SESSION_SETUP, true) => {
                let (fields, taken) = payload(bytes, 1)?;
                Ok((
                    FragCommand::FragSessionSetupAns(SetupStatus::from_bits(fields[0])),
                    taken,
                ))
            }
            (CID_SESSION_DELETE, false) => {
                let (fields, taken) = payload(bytes, 1)?;
                Ok((
                    FragCommand::FragSessionDeleteReq {
                        frag_index: fields[0] & 0x03,
                    },
                    taken,
                ))
            }
            (CID_SESSION_DELETE, true) => {
                let (fields, taken) = payload(bytes, 1)?;
                Ok((
                    FragCommand::FragSessionDeleteAns {
                        frag_index: fields[0] & 0x03,
                        no_session: fields[0] & 0x04 != 0,
                    },
                    taken,
                ))
            }
            (CID_BLOCK_RECEIVED, true) => {
                let (fields, taken) = payload(bytes, 1)?;
                Ok((
                    FragCommand::FragDataBlockReceivedReq {
                        frag_index: fields[0] & 0x03,
                        mic_error: fields[0] & 0x04 != 0,
                    },
                    taken,
                ))
            }
            (CID_BLOCK_RECEIVED, false) => {
                let (fields, taken) = payload(bytes, 1)?;
                Ok((
                    FragCommand::FragDataBlockReceivedAns {
                        frag_index: fields[0] & 0x03,
                    },
                    taken,
                ))
            }
            (CID_DATA_FRAGMENT, false) => {
                let fields = bytes.get(1..3).ok_or(LorawanError::MalformedFrame)?;
                let index = u16::from_le_bytes([fields[0], fields[1]]);
                Ok((
                    FragCommand::DataFragment {
                        frag_index: (index >> 14) as u8,
                        n: index & 0x3FFF,
                        data: &bytes[3..],
                    },
                    bytes.len(),
                ))
            }
            _ => Err(LorawanError::UnknownCommand(cid)),
        }
    }

    /// Writes this command out.
    ///
    /// A session setup writes its integrity code, which the caller takes over the block
    /// with [`BlockMicKey`].
    ///
    /// # Arguments
    ///
    /// * `out` - where to write it.
    ///
    /// # Returns
    ///
    /// How many bytes were written.
    ///
    /// # Errors
    ///
    /// [`LorawanError::PayloadTooLong`] when the buffer is too small.
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, LorawanError> {
        match self {
            FragCommand::PackageVersionReq => write(out, CID_PACKAGE_VERSION, &[]),
            FragCommand::PackageVersionAns(version) => write(
                out,
                CID_PACKAGE_VERSION,
                &[version.package, version.version],
            ),
            FragCommand::FragSessionStatusReq {
                frag_index,
                all_participants,
            } => write(
                out,
                CID_SESSION_STATUS,
                &[((frag_index & 0x03) << 1) | u8::from(*all_participants)],
            ),
            FragCommand::FragSessionStatusAns {
                frag_index,
                received,
                missing,
                mic_error,
                memory_error,
                no_session,
            } => {
                let coded = (received & 0x3FFF) | (u16::from(*frag_index) << 14);
                let status = u8::from(*memory_error)
                    | (u8::from(*mic_error) << 1)
                    | (u8::from(*no_session) << 2);
                let bytes = coded.to_le_bytes();
                write(
                    out,
                    CID_SESSION_STATUS,
                    &[status, bytes[0], bytes[1], *missing],
                )
            }
            FragCommand::FragSessionSetupReq {
                frag_index,
                mc_group_bit_mask,
                nb_frag,
                frag_size,
                ack_reception,
                frag_algo,
                block_ack_delay,
                padding,
                descriptor,
                session_cnt,
                mic,
            } => {
                let nb = nb_frag.to_le_bytes();
                let count = session_cnt.to_le_bytes();
                let control = (u8::from(*ack_reception) << 6)
                    | ((frag_algo & 0x07) << 3)
                    | (block_ack_delay & 0x07);
                write(
                    out,
                    CID_SESSION_SETUP,
                    &[
                        ((frag_index & 0x03) << 4) | (mc_group_bit_mask & 0x0F),
                        nb[0],
                        nb[1],
                        *frag_size,
                        control,
                        *padding,
                        descriptor[0],
                        descriptor[1],
                        descriptor[2],
                        descriptor[3],
                        count[0],
                        count[1],
                        mic[0],
                        mic[1],
                        mic[2],
                        mic[3],
                    ],
                )
            }
            FragCommand::FragSessionSetupAns(status) => {
                write(out, CID_SESSION_SETUP, &[status.bits()])
            }
            FragCommand::FragSessionDeleteReq { frag_index } => {
                write(out, CID_SESSION_DELETE, &[frag_index & 0x03])
            }
            FragCommand::FragSessionDeleteAns {
                frag_index,
                no_session,
            } => write(
                out,
                CID_SESSION_DELETE,
                &[(frag_index & 0x03) | (u8::from(*no_session) << 2)],
            ),
            FragCommand::FragDataBlockReceivedReq {
                frag_index,
                mic_error,
            } => write(
                out,
                CID_BLOCK_RECEIVED,
                &[(frag_index & 0x03) | (u8::from(*mic_error) << 2)],
            ),
            FragCommand::FragDataBlockReceivedAns { frag_index } => {
                write(out, CID_BLOCK_RECEIVED, &[frag_index & 0x03])
            }
            FragCommand::DataFragment {
                frag_index,
                n,
                data,
            } => {
                let coded = (n & 0x3FFF) | (u16::from(*frag_index) << 14);
                let bytes = coded.to_le_bytes();
                let len = data.len() + 3;
                let room = out.get_mut(..len).ok_or(LorawanError::PayloadTooLong)?;
                room[0] = CID_DATA_FRAGMENT;
                room[1] = bytes[0];
                room[2] = bytes[1];
                room[3..].copy_from_slice(data);
                Ok(len)
            }
        }
    }
}
