//! Scripted buses: a part's side of the conversation, for tests with nothing plugged in.
//!
//! A driver is the sequence of transfers its datasheet prescribes: read this register,
//! write that one, wait, read the result. [`I2cScript`] holds that sequence as
//! [`I2cStep`]s, checks every transfer the driver makes against the next step, and
//! answers with the bytes the part would have sent. A step can also fail on purpose,
//! so the error path is tested too. [`PinScript`] and [`DelayLog`] do the same for a
//! GPIO line and a delay: the levels the driver set and the time it waited are kept
//! for the test to assert against the datasheet's timing.
//!
//! [`block_on`] runs the future a driver's `read` or `apply` returns to completion, so
//! a test of a scripted driver needs no executor.

use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::fmt;
use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{
    ErrorType as PinErrorType, InputPin, OutputPin, PinState, StatefulOutputPin,
};
use embedded_hal::i2c::{ErrorKind, ErrorType, I2c, Operation, SevenBitAddress};
use embedded_hal::spi::{self, SpiDevice};

/// One transfer a scripted I2C part expects, and what it answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum I2cStep {
    /// The driver writes exactly `bytes` to `address`.
    Write {
        /// The 7-bit address the write must go to.
        address: u8,
        /// The bytes the driver must send.
        bytes: Vec<u8>,
    },
    /// The driver reads from `address` and receives `reply`, whose length is the
    /// length it must ask for.
    Read {
        /// The 7-bit address the read must come from.
        address: u8,
        /// The bytes the part answers with.
        reply: Vec<u8>,
    },
    /// The driver writes `bytes` then reads `reply.len()` bytes from `address` in one
    /// transaction, the shape of a register read.
    WriteRead {
        /// The 7-bit address of the part.
        address: u8,
        /// The bytes the driver must send first, usually a register address.
        bytes: Vec<u8>,
        /// The bytes the part answers with.
        reply: Vec<u8>,
    },
    /// The part fails the next transfer to `address` with `kind`, the way a missing
    /// or busy part does.
    Fault {
        /// The 7-bit address the failing transfer must go to.
        address: u8,
        /// The failure the driver sees.
        kind: ErrorKind,
    },
}

impl I2cStep {
    /// A write of exactly `bytes` to `address`.
    ///
    /// # Arguments
    ///
    /// * `address` - the 7-bit address.
    /// * `bytes` - the bytes the driver must send.
    ///
    /// # Returns
    ///
    /// The step.
    pub fn write(address: u8, bytes: impl Into<Vec<u8>>) -> I2cStep {
        I2cStep::Write {
            address,
            bytes: bytes.into(),
        }
    }

    /// A read from `address` answered with `reply`.
    ///
    /// # Arguments
    ///
    /// * `address` - the 7-bit address.
    /// * `reply` - the bytes the part answers with; the driver must ask for exactly
    ///   this many.
    ///
    /// # Returns
    ///
    /// The step.
    pub fn read(address: u8, reply: impl Into<Vec<u8>>) -> I2cStep {
        I2cStep::Read {
            address,
            reply: reply.into(),
        }
    }

    /// A write of `bytes` followed by a read answered with `reply`, in one transaction.
    ///
    /// # Arguments
    ///
    /// * `address` - the 7-bit address.
    /// * `bytes` - the bytes the driver must send first.
    /// * `reply` - the bytes the part answers with.
    ///
    /// # Returns
    ///
    /// The step.
    pub fn write_read(
        address: u8,
        bytes: impl Into<Vec<u8>>,
        reply: impl Into<Vec<u8>>,
    ) -> I2cStep {
        I2cStep::WriteRead {
            address,
            bytes: bytes.into(),
            reply: reply.into(),
        }
    }

    /// A transfer to `address` that fails with `kind`.
    ///
    /// # Arguments
    ///
    /// * `address` - the 7-bit address.
    /// * `kind` - the failure the driver sees.
    ///
    /// # Returns
    ///
    /// The step.
    pub fn fault(address: u8, kind: ErrorKind) -> I2cStep {
        I2cStep::Fault { address, kind }
    }

    fn address(&self) -> u8 {
        match self {
            I2cStep::Write { address, .. }
            | I2cStep::Read { address, .. }
            | I2cStep::WriteRead { address, .. }
            | I2cStep::Fault { address, .. } => *address,
        }
    }
}

/// One operation of a transaction, as the driver issued it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Transfer {
    /// The driver wrote these bytes.
    Write {
        /// The 7-bit address written to.
        address: u8,
        /// The bytes written.
        bytes: Vec<u8>,
    },
    /// The driver asked to read this many bytes.
    Read {
        /// The 7-bit address read from.
        address: u8,
        /// How many bytes were asked for.
        len: usize,
    },
}

/// What a scripted I2C part refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptError {
    /// A transfer arrived that does not match the next step of the script.
    Mismatch {
        /// The index of the step the transfer was checked against.
        step: usize,
        /// The step that stood there, or `None` once the script had been used up.
        expected: Option<I2cStep>,
        /// The whole transaction as the driver issued it.
        actual: Vec<Transfer>,
    },
    /// The next step was an [`I2cStep::Fault`], so the part failed as scripted.
    Fault(ErrorKind),
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptError::Mismatch {
                step,
                expected,
                actual,
            } => write!(
                f,
                "i2c script step {step}: expected {expected:?}, the driver issued {actual:?}"
            ),
            ScriptError::Fault(kind) => write!(f, "i2c script fault: {kind}"),
        }
    }
}

impl core::error::Error for ScriptError {}

impl embedded_hal::i2c::Error for ScriptError {
    fn kind(&self) -> ErrorKind {
        match self {
            ScriptError::Fault(kind) => *kind,
            ScriptError::Mismatch { .. } => ErrorKind::Other,
        }
    }
}

/// An I2C bus that plays a script of transfers and replies.
///
/// Each transaction the driver issues is matched against the next step. A register
/// read (a write followed by a read in one transaction) consumes one
/// [`I2cStep::WriteRead`]; a plain write or read consumes a [`I2cStep::Write`] or
/// [`I2cStep::Read`]; a longer transaction consumes steps in order. A transfer that
/// does not match fails with [`ScriptError::Mismatch`], which names the step and what
/// arrived, and a test ends by checking [`done`](I2cScript::done) so an unfinished
/// script is a failure too.
///
/// # Examples
///
/// ```
/// use pamoja_hal::i2c::I2c;
/// use pamoja_hal::script::{I2cScript, I2cStep};
///
/// // A part at 0x48 whose 16-bit result register 0x00 reads 0x0C80.
/// let mut bus = I2cScript::new([
///     I2cStep::write(0x48, [0x01, 0x60, 0x20]),
///     I2cStep::write_read(0x48, [0x00], [0x0C, 0x80]),
/// ]);
///
/// bus.write(0x48, &[0x01, 0x60, 0x20])?;
/// let mut result = [0u8; 2];
/// bus.write_read(0x48, &[0x00], &mut result)?;
/// assert_eq!(u16::from_be_bytes(result), 0x0C80);
/// assert!(bus.done());
/// # Ok::<(), pamoja_hal::script::ScriptError>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct I2cScript {
    steps: VecDeque<I2cStep>,
    consumed: usize,
}

impl I2cScript {
    /// Creates a bus that expects `steps` in order.
    ///
    /// # Arguments
    ///
    /// * `steps` - the transfers the driver is expected to make, and their replies.
    ///
    /// # Returns
    ///
    /// The scripted bus.
    pub fn new(steps: impl IntoIterator<Item = I2cStep>) -> I2cScript {
        I2cScript {
            steps: steps.into_iter().collect(),
            consumed: 0,
        }
    }

    /// Reports whether every step has been consumed.
    ///
    /// # Returns
    ///
    /// `true` once the driver has made every transfer the script expected.
    pub fn done(&self) -> bool {
        self.steps.is_empty()
    }

    /// Reports how many steps remain.
    ///
    /// # Returns
    ///
    /// The number of steps the driver has not yet reached.
    pub fn remaining(&self) -> usize {
        self.steps.len()
    }

    /// Reports how many steps the driver has consumed.
    ///
    /// # Returns
    ///
    /// The number of steps matched or faulted so far.
    pub fn consumed(&self) -> usize {
        self.consumed
    }

    fn mismatch(&self, actual: Vec<Transfer>) -> ScriptError {
        ScriptError::Mismatch {
            step: self.consumed,
            expected: self.steps.front().cloned(),
            actual,
        }
    }

    fn take(&mut self) -> Option<I2cStep> {
        let step = self.steps.pop_front();
        if step.is_some() {
            self.consumed += 1;
        }
        step
    }
}

impl ErrorType for I2cScript {
    type Error = ScriptError;
}

impl I2c<SevenBitAddress> for I2cScript {
    fn transaction(
        &mut self,
        address: SevenBitAddress,
        operations: &mut [Operation<'_>],
    ) -> Result<(), ScriptError> {
        let issued: Vec<Transfer> = operations
            .iter()
            .map(|operation| match operation {
                Operation::Write(bytes) => Transfer::Write {
                    address,
                    bytes: bytes.to_vec(),
                },
                Operation::Read(buffer) => Transfer::Read {
                    address,
                    len: buffer.len(),
                },
            })
            .collect();

        let mut index = 0;
        while index < operations.len() {
            let Some(step) = self.steps.front() else {
                return Err(self.mismatch(issued));
            };
            if step.address() != address {
                return Err(self.mismatch(issued));
            }
            if let I2cStep::Fault { kind, .. } = step {
                let kind = *kind;
                self.take();
                return Err(ScriptError::Fault(kind));
            }

            let (first, rest) = operations[index..].split_first_mut().expect("in range");
            match (first, rest.first_mut(), step) {
                (
                    Operation::Write(written),
                    Some(Operation::Read(buffer)),
                    I2cStep::WriteRead { bytes, reply, .. },
                ) if bytes.as_slice() == *written && reply.len() == buffer.len() => {
                    buffer.copy_from_slice(reply);
                    self.take();
                    index += 2;
                }
                (Operation::Write(written), _, I2cStep::Write { bytes, .. })
                    if bytes.as_slice() == *written =>
                {
                    self.take();
                    index += 1;
                }
                (Operation::Read(buffer), _, I2cStep::Read { reply, .. })
                    if reply.len() == buffer.len() =>
                {
                    buffer.copy_from_slice(reply);
                    self.take();
                    index += 1;
                }
                _ => return Err(self.mismatch(issued)),
            }
        }
        Ok(())
    }
}

/// One transfer a scripted SPI part expects, and what it answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpiStep {
    /// The driver writes exactly `bytes`; whatever the part shifts out meanwhile is
    /// discarded, as the driver asked.
    Write {
        /// The bytes the driver must send.
        bytes: Vec<u8>,
    },
    /// The driver reads `reply.len()` bytes and receives `reply`.
    Read {
        /// The bytes the part shifts out.
        reply: Vec<u8>,
    },
    /// The driver writes `bytes` and receives `reply` in the same clocks, so the two
    /// are the same length.
    Transfer {
        /// The bytes the driver must send.
        bytes: Vec<u8>,
        /// The bytes the part shifts out at the same time.
        reply: Vec<u8>,
    },
    /// The part fails the next transfer with `kind`.
    Fault {
        /// The failure the driver sees.
        kind: spi::ErrorKind,
    },
}

impl SpiStep {
    /// A write of exactly `bytes`.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the bytes the driver must send.
    ///
    /// # Returns
    ///
    /// The step.
    pub fn write(bytes: impl Into<Vec<u8>>) -> SpiStep {
        SpiStep::Write {
            bytes: bytes.into(),
        }
    }

    /// A read answered with `reply`.
    ///
    /// # Arguments
    ///
    /// * `reply` - the bytes the part shifts out; the driver must ask for exactly
    ///   this many.
    ///
    /// # Returns
    ///
    /// The step.
    pub fn read(reply: impl Into<Vec<u8>>) -> SpiStep {
        SpiStep::Read {
            reply: reply.into(),
        }
    }

    /// A full-duplex transfer: `bytes` go out while `reply` comes in.
    ///
    /// # Arguments
    ///
    /// * `bytes` - the bytes the driver must send.
    /// * `reply` - the bytes the part shifts out, as many as `bytes`.
    ///
    /// # Returns
    ///
    /// The step.
    pub fn transfer(bytes: impl Into<Vec<u8>>, reply: impl Into<Vec<u8>>) -> SpiStep {
        SpiStep::Transfer {
            bytes: bytes.into(),
            reply: reply.into(),
        }
    }

    /// A transfer that fails with `kind`.
    ///
    /// # Arguments
    ///
    /// * `kind` - the failure the driver sees.
    ///
    /// # Returns
    ///
    /// The step.
    pub fn fault(kind: spi::ErrorKind) -> SpiStep {
        SpiStep::Fault { kind }
    }
}

/// One operation of an SPI transaction, as the driver issued it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpiTransfer {
    /// The driver wrote these bytes.
    Write {
        /// The bytes written.
        bytes: Vec<u8>,
    },
    /// The driver asked to read this many bytes.
    Read {
        /// How many bytes were asked for.
        len: usize,
    },
    /// The driver wrote these bytes and read as many back.
    Transfer {
        /// The bytes written.
        bytes: Vec<u8>,
    },
    /// The driver asked the bus to pause inside the transaction.
    Delay {
        /// The pause in nanoseconds.
        ns: u32,
    },
}

/// What a scripted SPI part refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpiScriptError {
    /// An operation arrived that does not match the next step of the script.
    Mismatch {
        /// The index of the step the operation was checked against.
        step: usize,
        /// The step that stood there, or `None` once the script had been used up.
        expected: Option<SpiStep>,
        /// The whole transaction as the driver issued it.
        actual: Vec<SpiTransfer>,
    },
    /// The next step was a [`SpiStep::Fault`], so the part failed as scripted.
    Fault(spi::ErrorKind),
}

impl fmt::Display for SpiScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpiScriptError::Mismatch {
                step,
                expected,
                actual,
            } => write!(
                f,
                "spi script step {step}: expected {expected:?}, the driver issued {actual:?}"
            ),
            SpiScriptError::Fault(kind) => write!(f, "spi script fault: {kind}"),
        }
    }
}

impl core::error::Error for SpiScriptError {}

impl spi::Error for SpiScriptError {
    fn kind(&self) -> spi::ErrorKind {
        match self {
            SpiScriptError::Fault(kind) => *kind,
            SpiScriptError::Mismatch { .. } => spi::ErrorKind::Other,
        }
    }
}

/// An SPI device that plays a script of transfers and replies.
///
/// The device is the bus plus the part's chip select, so a transaction is one
/// chip-select assertion and each operation inside it consumes one step. A delay
/// operation consumes nothing. A transfer that does not match fails with
/// [`SpiScriptError::Mismatch`], and a test ends by checking
/// [`done`](SpiScript::done).
///
/// # Examples
///
/// ```
/// use pamoja_hal::script::{SpiScript, SpiStep};
/// use pamoja_hal::spi::{Operation, SpiDevice};
///
/// // A Bosch part read over SPI: the control byte 0xD0 with its read bit set, then
/// // the chip id shifted out.
/// let mut device = SpiScript::new([SpiStep::write([0xD0]), SpiStep::read([0x60])]);
/// let mut id = [0u8; 1];
/// device.transaction(&mut [Operation::Write(&[0xD0]), Operation::Read(&mut id)])?;
/// assert_eq!(id, [0x60]);
/// assert!(device.done());
/// # Ok::<(), pamoja_hal::script::SpiScriptError>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct SpiScript {
    steps: VecDeque<SpiStep>,
    consumed: usize,
}

impl SpiScript {
    /// Creates a device that expects `steps` in order.
    ///
    /// # Arguments
    ///
    /// * `steps` - the transfers the driver is expected to make, and their replies.
    ///
    /// # Returns
    ///
    /// The scripted device.
    pub fn new(steps: impl IntoIterator<Item = SpiStep>) -> SpiScript {
        SpiScript {
            steps: steps.into_iter().collect(),
            consumed: 0,
        }
    }

    /// Reports whether every step has been consumed.
    ///
    /// # Returns
    ///
    /// `true` once the driver has made every transfer the script expected.
    pub fn done(&self) -> bool {
        self.steps.is_empty()
    }

    /// Reports how many steps remain.
    ///
    /// # Returns
    ///
    /// The number of steps the driver has not yet reached.
    pub fn remaining(&self) -> usize {
        self.steps.len()
    }

    /// Reports how many steps the driver has consumed.
    ///
    /// # Returns
    ///
    /// The number of steps matched or faulted so far.
    pub fn consumed(&self) -> usize {
        self.consumed
    }

    fn mismatch(&self, actual: Vec<SpiTransfer>) -> SpiScriptError {
        SpiScriptError::Mismatch {
            step: self.consumed,
            expected: self.steps.front().cloned(),
            actual,
        }
    }

    fn take(&mut self) {
        if self.steps.pop_front().is_some() {
            self.consumed += 1;
        }
    }
}

impl spi::ErrorType for SpiScript {
    type Error = SpiScriptError;
}

impl SpiDevice<u8> for SpiScript {
    fn transaction(
        &mut self,
        operations: &mut [spi::Operation<'_, u8>],
    ) -> Result<(), SpiScriptError> {
        let issued: Vec<SpiTransfer> = operations
            .iter()
            .map(|operation| match operation {
                spi::Operation::Write(bytes) => SpiTransfer::Write {
                    bytes: bytes.to_vec(),
                },
                spi::Operation::Read(buffer) => SpiTransfer::Read { len: buffer.len() },
                spi::Operation::Transfer(_, bytes) => SpiTransfer::Transfer {
                    bytes: bytes.to_vec(),
                },
                spi::Operation::TransferInPlace(bytes) => SpiTransfer::Transfer {
                    bytes: bytes.to_vec(),
                },
                spi::Operation::DelayNs(ns) => SpiTransfer::Delay { ns: *ns },
            })
            .collect();

        for operation in operations.iter_mut() {
            if let spi::Operation::DelayNs(_) = operation {
                continue;
            }
            let Some(step) = self.steps.front() else {
                return Err(self.mismatch(issued));
            };
            if let SpiStep::Fault { kind } = step {
                let kind = *kind;
                self.take();
                return Err(SpiScriptError::Fault(kind));
            }
            match (operation, step) {
                (spi::Operation::Write(written), SpiStep::Write { bytes })
                    if bytes.as_slice() == *written =>
                {
                    self.take();
                }
                (spi::Operation::Read(buffer), SpiStep::Read { reply })
                    if reply.len() == buffer.len() =>
                {
                    buffer.copy_from_slice(reply);
                    self.take();
                }
                (spi::Operation::Transfer(read, written), SpiStep::Transfer { bytes, reply })
                    if bytes.as_slice() == *written && reply.len() == read.len() =>
                {
                    read.copy_from_slice(reply);
                    self.take();
                }
                (spi::Operation::TransferInPlace(buffer), SpiStep::Transfer { bytes, reply })
                    if bytes.as_slice() == *buffer && reply.len() == buffer.len() =>
                {
                    buffer.copy_from_slice(reply);
                    self.take();
                }
                _ => return Err(self.mismatch(issued)),
            }
        }
        Ok(())
    }
}

/// A GPIO line that records the levels it is driven to and answers reads from a script.
///
/// Driving the pin appends to [`driven`](PinScript::driven). Reading it returns the
/// next scripted level, or the level it was last driven to once the script is used up,
/// so a line that is only ever an output never runs dry.
///
/// # Examples
///
/// ```
/// use pamoja_hal::digital::{InputPin, OutputPin, PinState};
/// use pamoja_hal::script::PinScript;
///
/// let mut pin = PinScript::new([PinState::Low, PinState::High]);
/// pin.set_high()?;
/// pin.set_low()?;
/// assert_eq!(pin.driven(), [PinState::High, PinState::Low]);
/// assert!(pin.is_low()?);
/// assert!(pin.is_high()?);
/// # Ok::<(), core::convert::Infallible>(())
/// ```
#[derive(Clone, Debug)]
pub struct PinScript {
    driven: Vec<PinState>,
    inputs: VecDeque<PinState>,
    level: PinState,
}

impl Default for PinScript {
    fn default() -> Self {
        PinScript::new([])
    }
}

impl PinScript {
    /// Creates a released (high) line whose reads answer `inputs` in order.
    ///
    /// # Arguments
    ///
    /// * `inputs` - the levels each read returns, in order.
    ///
    /// # Returns
    ///
    /// The scripted pin.
    pub fn new(inputs: impl IntoIterator<Item = PinState>) -> PinScript {
        PinScript {
            driven: Vec::new(),
            inputs: inputs.into_iter().collect(),
            level: PinState::High,
        }
    }

    /// Returns every level the pin was driven to, oldest first.
    ///
    /// # Returns
    ///
    /// The levels set through [`OutputPin`].
    pub fn driven(&self) -> &[PinState] {
        &self.driven
    }

    /// Returns the level the pin was last driven to.
    ///
    /// # Returns
    ///
    /// The current output level.
    pub fn level(&self) -> PinState {
        self.level
    }

    /// Reports how many scripted input levels remain unread.
    ///
    /// # Returns
    ///
    /// The number of reads the script still answers.
    pub fn remaining(&self) -> usize {
        self.inputs.len()
    }
}

impl PinErrorType for PinScript {
    type Error = core::convert::Infallible;
}

impl OutputPin for PinScript {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        self.level = PinState::Low;
        self.driven.push(PinState::Low);
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        self.level = PinState::High;
        self.driven.push(PinState::High);
        Ok(())
    }
}

impl StatefulOutputPin for PinScript {
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        Ok(self.level == PinState::High)
    }

    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        Ok(self.level == PinState::Low)
    }
}

impl InputPin for PinScript {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        let level = self.inputs.pop_front().unwrap_or(self.level);
        Ok(level == PinState::High)
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        self.is_high().map(|high| !high)
    }
}

/// A delay that records every wait instead of sleeping.
///
/// # Examples
///
/// ```
/// use pamoja_hal::delay::DelayNs;
/// use pamoja_hal::script::DelayLog;
///
/// let mut delay = DelayLog::new();
/// delay.delay_us(480);
/// delay.delay_ms(10);
/// assert_eq!(delay.total_micros(), 10_480);
/// ```
#[derive(Clone, Debug, Default)]
pub struct DelayLog {
    waits_ns: Vec<u32>,
    total_ns: u64,
}

impl DelayLog {
    /// Creates a log with nothing waited yet.
    ///
    /// # Returns
    ///
    /// The empty log.
    pub fn new() -> DelayLog {
        DelayLog::default()
    }

    /// Returns every wait in nanoseconds, oldest first.
    ///
    /// # Returns
    ///
    /// The waits as the driver requested them.
    pub fn waits_ns(&self) -> &[u32] {
        &self.waits_ns
    }

    /// Returns the total time waited, in nanoseconds.
    ///
    /// # Returns
    ///
    /// The sum of every wait.
    pub fn total_ns(&self) -> u64 {
        self.total_ns
    }

    /// Returns the total time waited, in whole microseconds.
    ///
    /// # Returns
    ///
    /// The sum of every wait, rounded down.
    pub fn total_micros(&self) -> u64 {
        self.total_ns / 1_000
    }

    /// Returns the total time waited, in whole milliseconds.
    ///
    /// # Returns
    ///
    /// The sum of every wait, rounded down.
    pub fn total_millis(&self) -> u64 {
        self.total_ns / 1_000_000
    }

    /// Forgets every recorded wait.
    pub fn clear(&mut self) {
        self.waits_ns.clear();
        self.total_ns = 0;
    }
}

impl DelayNs for DelayLog {
    fn delay_ns(&mut self, ns: u32) {
        self.waits_ns.push(ns);
        self.total_ns += u64::from(ns);
    }
}

/// Runs a future to completion by polling it, for futures that never wait on I/O.
///
/// A driver over a scripted bus finishes its `read` or `apply` in one poll, so a test
/// needs no executor to await it. A future that is genuinely pending is polled again
/// without yielding, so this is not for futures that wait on a timer or a socket.
///
/// # Arguments
///
/// * `future` - the future to run.
///
/// # Returns
///
/// The future's output.
///
/// # Examples
///
/// ```
/// use pamoja_hal::script::block_on;
///
/// let answer = block_on(async { 6 * 7 });
/// assert_eq!(answer, 42);
/// ```
pub fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut context) {
            return output;
        }
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn a_register_read_consumes_one_write_read_step() {
        let mut bus = I2cScript::new([I2cStep::write_read(0x76, [0xF7], [1, 2, 3])]);
        let mut data = [0u8; 3];
        bus.write_read(0x76, &[0xF7], &mut data).unwrap();
        assert_eq!(data, [1, 2, 3]);
        assert!(bus.done());
        assert_eq!(bus.consumed(), 1);
    }

    #[test]
    fn a_longer_transaction_consumes_steps_in_order() {
        let mut bus = I2cScript::new([
            I2cStep::write(0x40, [0x02]),
            I2cStep::read(0x40, [0xAA, 0xBB]),
        ]);
        let mut data = [0u8; 2];
        bus.transaction(
            0x40,
            &mut [Operation::Write(&[0x02]), Operation::Read(&mut data)],
        )
        .unwrap();
        assert_eq!(data, [0xAA, 0xBB]);
        assert!(bus.done());
    }

    #[test]
    fn a_wrong_address_or_payload_is_a_mismatch_naming_the_step() {
        let mut bus = I2cScript::new([
            I2cStep::write(0x76, [0xF4, 0x25]),
            I2cStep::write(0x76, [0xF5, 0x00]),
        ]);
        bus.write(0x76, &[0xF4, 0x25]).unwrap();
        let error = bus.write(0x77, &[0xF5, 0x00]).unwrap_err();
        assert_eq!(
            error,
            ScriptError::Mismatch {
                step: 1,
                expected: Some(I2cStep::write(0x76, [0xF5, 0x00])),
                actual: vec![Transfer::Write {
                    address: 0x77,
                    bytes: vec![0xF5, 0x00]
                }],
            }
        );
        let error = bus.write(0x76, &[0xF5, 0x04]).unwrap_err();
        assert!(matches!(error, ScriptError::Mismatch { step: 1, .. }));
        assert_eq!(bus.remaining(), 1);
    }

    #[test]
    fn a_read_of_the_wrong_length_is_a_mismatch() {
        let mut bus = I2cScript::new([I2cStep::read(0x48, [0x12, 0x34])]);
        let mut short = [0u8; 1];
        assert!(bus.read(0x48, &mut short).is_err());
    }

    #[test]
    fn an_exhausted_script_refuses_and_says_so() {
        let mut bus = I2cScript::new([]);
        let error = bus.write(0x48, &[0x00]).unwrap_err();
        assert!(matches!(
            error,
            ScriptError::Mismatch {
                step: 0,
                expected: None,
                ..
            }
        ));
    }

    #[test]
    fn a_fault_step_fails_with_its_kind_and_is_consumed() {
        use embedded_hal::i2c::{Error, NoAcknowledgeSource};

        let kind = ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address);
        let mut bus = I2cScript::new([
            I2cStep::fault(0x76, kind),
            I2cStep::write_read(0x76, [0xD0], [0x60]),
        ]);
        let mut id = [0u8; 1];
        let error = bus.write_read(0x76, &[0xD0], &mut id).unwrap_err();
        assert_eq!(error, ScriptError::Fault(kind));
        assert_eq!(error.kind(), kind);
        bus.write_read(0x76, &[0xD0], &mut id).unwrap();
        assert_eq!(id, [0x60]);
        assert!(bus.done());
    }

    #[test]
    fn an_spi_transaction_consumes_one_step_per_operation_and_skips_delays() {
        let mut device = SpiScript::new([
            SpiStep::write([0xF7]),
            SpiStep::read([1, 2, 3]),
            SpiStep::transfer([0xAA, 0xBB], [0x11, 0x22]),
        ]);
        let mut data = [0u8; 3];
        let mut exchanged = [0xAA, 0xBB];
        device
            .transaction(&mut [
                spi::Operation::Write(&[0xF7]),
                spi::Operation::DelayNs(10),
                spi::Operation::Read(&mut data),
                spi::Operation::TransferInPlace(&mut exchanged),
            ])
            .unwrap();
        assert_eq!(data, [1, 2, 3]);
        assert_eq!(exchanged, [0x11, 0x22]);
        assert!(device.done());
        assert_eq!(device.consumed(), 3);
    }

    #[test]
    fn an_spi_mismatch_or_fault_is_reported_like_the_i2c_ones() {
        let mut device = SpiScript::new([
            SpiStep::write([0x74, 0x25]),
            SpiStep::fault(spi::ErrorKind::ChipSelectFault),
        ]);
        let error = device.write(&[0x74, 0x26]).unwrap_err();
        assert!(matches!(
            error,
            SpiScriptError::Mismatch {
                step: 0,
                expected: Some(SpiStep::Write { .. }),
                ..
            }
        ));
        device.write(&[0x74, 0x25]).unwrap();
        assert_eq!(
            device.write(&[0x00]).unwrap_err(),
            SpiScriptError::Fault(spi::ErrorKind::ChipSelectFault)
        );
        assert!(device.done());
    }

    #[test]
    fn a_pin_records_what_it_was_driven_to_and_answers_its_script() {
        let mut pin = PinScript::new([PinState::Low]);
        pin.set_low().unwrap();
        pin.set_high().unwrap();
        assert_eq!(pin.driven(), [PinState::Low, PinState::High]);
        assert!(pin.is_low().unwrap());
        assert!(
            pin.is_high().unwrap(),
            "the script is used up, so the output level answers"
        );
        assert!(pin.is_set_high().unwrap());
        assert_eq!(pin.remaining(), 0);
    }

    #[test]
    fn a_delay_log_sums_every_unit() {
        let mut delay = DelayLog::new();
        delay.delay_ns(500);
        delay.delay_us(2);
        delay.delay_ms(1);
        assert_eq!(delay.total_ns(), 1_002_500);
        assert_eq!(delay.total_micros(), 1_002);
        assert_eq!(delay.total_millis(), 1);
        assert!(!delay.waits_ns().is_empty());
        delay.clear();
        assert_eq!(delay.total_ns(), 0);
    }

    #[test]
    fn block_on_runs_a_ready_future() {
        assert_eq!(block_on(async { 7 }), 7);
    }
}
