//! The control contract: the authenticated actions a client can ask a node to take.
//!
//! Reading the dashboard needs no command; changing the node does. A [`Command`] arrives
//! over the authenticated `POST /command` path (see the serving layer) and is dispatched
//! to the [`StateSource`](crate::StateSource), which is the only thing that can move an
//! actuator or change the fleet. The wire form is a serde-tagged object, so the page
//! sends `{"type":"actuate", ...}` for a discrete actuator and `{"type":"set", ...}` for
//! a numeric one.

use serde::Deserialize;

use crate::state::{Group, Sensor};

/// A control action a client asks the node to take.
///
/// The provisioning variants carry the group or sensor the client built, so the device
/// records the structure the operator described; the device owns and shares it across
/// every client.
// A command is decoded once per request and queued a handful at a time, so the sensor
// the add variant carries stays inline rather than boxed for the wire type's sake.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Command {
    /// Set a discrete actuator to one of its actions, such as opening a valve.
    Actuate {
        /// The actuator's `"groupId/sensorId"` path.
        target: String,
        /// The action to apply, one of the reading's advertised actions (`"open"`).
        action: String,
    },
    /// Set a numeric actuator to a value inside its advertised range, such as a pump
    /// speed or a setpoint.
    Set {
        /// The actuator's `"groupId/sensorId"` path.
        target: String,
        /// The value to apply, in the reading's unit and inside its
        /// [`range`](crate::Reading::range).
        value: f32,
    },
    /// Add a group to an organization.
    AddGroup {
        /// The organization's id.
        org: String,
        /// The group to add.
        group: Group,
    },
    /// Remove a group by id.
    RemoveGroup {
        /// The group's id.
        id: String,
    },
    /// Add a sensor to a group.
    AddSensor {
        /// The group's id.
        group: String,
        /// The sensor to add.
        sensor: Sensor,
        /// An optional gateway-defined hardware binding the device uses to find the sensor,
        /// such as `"i2c:0x76"`, `"gpio:4"`, or `"lora:ab12"`. The dashboard only carries it
        /// through; binding a real driver is the gateway's job when it drains the command.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        binding: Option<String>,
    },
    /// Remove a sensor by its `"groupId/sensorId"` path.
    RemoveSensor {
        /// The sensor's `"groupId/sensorId"` path.
        target: String,
    },
}

/// Why a command could not be carried out. The [`code`](CommandError::code) is a stable,
/// language-neutral string the page localizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandError {
    /// The source does not handle this kind of command.
    Unsupported,
    /// No actuator or target matches the command.
    UnknownTarget,
    /// The target exists but does not accept the requested action.
    InvalidAction,
    /// The value lies outside the range the target advertises.
    OutOfRange,
    /// The sensor being added is not one this device supports, so it was not added. A real
    /// device only accepts the sensor types it can actually bind a driver to.
    UnknownSensor,
}

impl CommandError {
    /// Returns the stable error code for this failure.
    ///
    /// # Returns
    ///
    /// A dotted, language-neutral code such as `"command.unknown_target"`.
    pub fn code(self) -> &'static str {
        match self {
            CommandError::Unsupported => "command.unsupported",
            CommandError::UnknownTarget => "command.unknown_target",
            CommandError::InvalidAction => "command.invalid_action",
            CommandError::OutOfRange => "command.out_of_range",
            CommandError::UnknownSensor => "command.unknown_sensor",
        }
    }
}
