//! The error the actuator drivers share.

/// What can go wrong driving a part over a bus or a set of pins.
///
/// A driver returns this from every transfer sequence. It converts into the core
/// [`Error`](pamoja_core::Error) so a driver used as an
/// [`Actuator`](pamoja_core::Actuator) reports the way every other device does: a
/// bus or pin fault becomes [`Error::Io`](pamoja_core::Error::Io), and a command the
/// part cannot take becomes [`Error::Codec`](pamoja_core::Error::Codec).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriverError<E> {
    /// The bus or pin underneath the driver failed: a missing acknowledge, a bus
    /// fault, a pin that could not be driven.
    Bus(E),
    /// The command is outside what the part accepts, such as a channel the part does
    /// not have or a frequency its prescaler cannot reach.
    Command(&'static str),
}

impl<E: core::fmt::Debug> core::fmt::Display for DriverError<E> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DriverError::Bus(error) => write!(f, "bus error: {error:?}"),
            DriverError::Command(reason) => write!(f, "command refused: {reason}"),
        }
    }
}

impl<E: core::fmt::Debug> core::error::Error for DriverError<E> {}

impl<E: core::fmt::Debug> From<DriverError<E>> for pamoja_core::Error {
    fn from(error: DriverError<E>) -> Self {
        match error {
            DriverError::Bus(bus) => pamoja_core::Error::Io(alloc::format!("bus error: {bus:?}")),
            DriverError::Command(reason) => {
                pamoja_core::Error::Codec(alloc::format!("command refused: {reason}"))
            }
        }
    }
}
