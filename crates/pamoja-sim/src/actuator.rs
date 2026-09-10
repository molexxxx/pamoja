//! A fake actuator that records the commands it is given.

use std::sync::{Arc, Mutex};

use pamoja_core::{Actuator, Result};

/// An actuator that records every command instead of driving hardware.
///
/// This stands in for a relay, a valve, or a motor in a hardware-free test: it
/// implements the core [`Actuator`] trait and keeps an ordered log of every command
/// applied to it, so a test can assert what a control loop decided to do. Take a
/// [`log`](RecordingActuator::log) handle before moving the actuator into a `Node`,
/// then read the commands back through it afterwards.
///
/// # Examples
///
/// ```
/// use pamoja_core::Actuator;
/// use pamoja_sim::RecordingActuator;
///
/// # async fn demo() -> pamoja_core::Result<()> {
/// let mut relay = RecordingActuator::new();
/// let log = relay.log();
///
/// relay.apply(true).await?;
/// relay.apply(false).await?;
///
/// assert_eq!(log.commands(), vec![true, false]);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct RecordingActuator<C> {
    log: Arc<Mutex<Vec<C>>>,
}

impl<C> RecordingActuator<C> {
    /// Creates an actuator with an empty command log.
    ///
    /// # Returns
    ///
    /// A recording actuator.
    pub fn new() -> Self {
        Self {
            log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns a handle that reads this actuator's command log.
    ///
    /// The handle shares the same underlying log, so commands applied after it is
    /// taken are still visible through it.
    ///
    /// # Returns
    ///
    /// An [`ActuatorLog`] over the same recorded commands.
    pub fn log(&self) -> ActuatorLog<C> {
        ActuatorLog {
            log: Arc::clone(&self.log),
        }
    }
}

impl<C> Default for RecordingActuator<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Send> Actuator for RecordingActuator<C> {
    type Command = C;

    async fn apply(&mut self, command: C) -> Result<()> {
        self.log.lock().expect("actuator log lock").push(command);
        Ok(())
    }
}

/// A read handle over a [`RecordingActuator`]'s command log.
#[derive(Clone, Debug)]
pub struct ActuatorLog<C> {
    log: Arc<Mutex<Vec<C>>>,
}

impl<C: Clone> ActuatorLog<C> {
    /// Returns a snapshot of every command applied so far, in order.
    ///
    /// # Returns
    ///
    /// A copy of the recorded commands.
    pub fn commands(&self) -> Vec<C> {
        self.log.lock().expect("actuator log lock").clone()
    }
}

impl<C> ActuatorLog<C> {
    /// Returns how many commands have been applied.
    ///
    /// # Returns
    ///
    /// The number of recorded commands.
    pub fn len(&self) -> usize {
        self.log.lock().expect("actuator log lock").len()
    }

    /// Returns whether no command has been applied yet.
    ///
    /// # Returns
    ///
    /// `true` if the command log is empty.
    pub fn is_empty(&self) -> bool {
        self.log.lock().expect("actuator log lock").is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_records_each_command_in_order() {
        let mut relay = RecordingActuator::new();
        let log = relay.log();
        assert!(log.is_empty());

        relay.apply(true).await.unwrap();
        relay.apply(false).await.unwrap();
        relay.apply(true).await.unwrap();

        assert_eq!(log.commands(), vec![true, false, true]);
        assert_eq!(log.len(), 3);
        assert!(!log.is_empty());
    }

    #[tokio::test]
    async fn a_log_taken_early_sees_later_commands() {
        let relay = RecordingActuator::new();
        let log = relay.log();
        // The actuator can be moved on (here, cloned) and still feed the same log.
        let mut moved = relay.clone();
        moved.apply(42u8).await.unwrap();
        assert_eq!(log.commands(), vec![42u8]);
    }
}
