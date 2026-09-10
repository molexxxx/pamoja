//! Device-model traits implemented by capability crates.
//!
//! These traits describe the roles a piece of hardware can play in an
//! application: a connectable [`Device`], a [`Sensor`] that produces readings, an
//! [`Actuator`] that accepts commands, and a [`Telemetry`] source that streams
//! frames. A single type may implement more than one of them.
//!
//! The futures these traits return are `Send`, as a [`Transport`](crate::Transport)'s
//! are, so a node built from any sensor and actuator can be driven from a task on a
//! multi-threaded runtime and erased behind a trait object that is. An implementation
//! written as `async fn` satisfies this as long as everything it holds across an await
//! is `Send`; a driver generic over a bus says so with a `Send` bound on the bus.

use core::future::Future;

use crate::adapt::{Map, MapCommand};
use crate::error::Result;

/// A connectable physical or virtual device.
///
/// Implementors manage the lifecycle of an underlying resource such as a serial
/// port, a network socket, or a vehicle link.
pub trait Device {
    /// Returns the stable identifier for this device.
    ///
    /// The identifier is expected to remain constant for the lifetime of the
    /// device, for example a serial number, MAC address, or vehicle URI.
    ///
    /// # Returns
    ///
    /// A string slice borrowing the device's identifier.
    fn id(&self) -> &str;

    /// Opens the device and prepares it for use.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the device is connected and ready.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](crate::Error::Io) if the underlying resource cannot
    /// be opened, or [`Error::Transport`](crate::Error::Transport) if a link to
    /// the device cannot be established.
    fn connect(&mut self) -> impl Future<Output = Result<()>> + Send;

    /// Releases the device and any resources it holds.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the device has been disconnected and its resources freed.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](crate::Error::Io) if the underlying resource cannot
    /// be released cleanly.
    fn disconnect(&mut self) -> impl Future<Output = Result<()>> + Send;
}

/// A source of typed readings, such as a thermometer, GPS receiver, or lidar.
pub trait Sensor {
    /// The value produced by a single read, for example a temperature or a fix.
    type Reading;

    /// Takes a single reading from the sensor.
    ///
    /// # Returns
    ///
    /// The next [`Reading`](Self::Reading) sampled from the sensor.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](crate::Error::Io) if the sensor cannot be read, or
    /// [`Error::Closed`](crate::Error::Closed) if the sensor has been
    /// disconnected.
    fn read(&mut self) -> impl Future<Output = Result<Self::Reading>> + Send;

    /// Wraps this sensor so every reading passes through `select` first.
    ///
    /// This is how a driver that measures several channels feeds a consumer that
    /// wants one number: `select` picks the channel, converts the unit, or combines
    /// fields, and the result is a [`Sensor`] whose reading is whatever `select`
    /// returns. See [`Map`] for an example.
    ///
    /// # Arguments
    ///
    /// * `select` - the function applied to each reading as it is taken.
    ///
    /// # Returns
    ///
    /// The adapted sensor.
    fn map<F, T>(self, select: F) -> Map<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Reading) -> T,
    {
        Map::new(self, select)
    }
}

/// A sink that accepts typed commands, such as a motor or a valve.
pub trait Actuator {
    /// The command accepted by a single application, for example a setpoint.
    type Command;

    /// Applies a command to the actuator.
    ///
    /// # Arguments
    ///
    /// * `command` - the command to apply, consumed by the call.
    ///
    /// # Returns
    ///
    /// `Ok(())` once the command has been accepted by the actuator.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](crate::Error::Io) if the command cannot be
    /// delivered, or [`Error::Closed`](crate::Error::Closed) if the actuator has
    /// been disconnected.
    fn apply(&mut self, command: Self::Command) -> impl Future<Output = Result<()>> + Send;

    /// Wraps this actuator so every command passes through `convert` first.
    ///
    /// This is how a part that takes its own command shape is driven by a consumer
    /// that has a simpler one: a profile's `bool` becomes a pulse width, a percentage
    /// becomes a duty. The result is an [`Actuator`] whose command is whatever
    /// `convert` accepts. See [`MapCommand`] for an example.
    ///
    /// # Arguments
    ///
    /// * `convert` - the function turning the caller's command into this part's.
    ///
    /// # Returns
    ///
    /// The adapted actuator.
    fn map_command<F, C>(self, convert: F) -> MapCommand<Self, F, C>
    where
        Self: Sized,
        F: FnMut(C) -> Self::Command,
    {
        MapCommand::new(self, convert)
    }
}

/// A device that emits a continuous stream of telemetry frames.
pub trait Telemetry {
    /// A single telemetry frame, for example a status or position report.
    type Frame;

    /// Awaits the next telemetry frame.
    ///
    /// # Returns
    ///
    /// `Some(frame)` when a frame is available, or `None` once the telemetry
    /// stream has ended.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Transport`](crate::Error::Transport) if the telemetry
    /// link fails while waiting.
    fn next_frame(&mut self) -> impl Future<Output = Result<Option<Self::Frame>>> + Send;
}
