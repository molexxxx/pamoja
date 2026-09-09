//! Adapters that reshape what a sensor reads and what an actuator is told.
//!
//! A driver's reading is whatever the part measures, often several channels at once,
//! and a driver's command is whatever the part accepts. The pieces that consume a
//! sensor or drive an actuator usually want one plain value: a controller takes a
//! single `f32`, a profile drives a `bool`. [`Map`] and [`MapCommand`] bridge the two
//! without a wrapper type per part: [`Sensor::map`] selects or converts each reading
//! as it is taken, and [`Actuator::map_command`] converts each command before it
//! reaches the part.

use core::marker::PhantomData;

use crate::device::{Actuator, Sensor};
use crate::error::Result;

/// A sensor whose readings pass through a function before they are returned.
///
/// Created by [`Sensor::map`]. The function runs on every read, so it can select one
/// channel of a multi-channel measurement, convert a unit, or fold several fields into
/// one value.
///
/// # Examples
///
/// ```
/// use pamoja_core::{Result, Sensor};
///
/// struct Climate;
/// impl Sensor for Climate {
///     type Reading = (f32, f32);
///     async fn read(&mut self) -> Result<(f32, f32)> {
///         Ok((21.5, 48.0))
///     }
/// }
///
/// # async fn run() -> Result<()> {
/// let mut humidity = Climate.map(|(_, humidity)| humidity);
/// assert_eq!(humidity.read().await?, 48.0);
/// # Ok(())
/// # }
/// ```
pub struct Map<S, F> {
    inner: S,
    select: F,
}

impl<S, F> Map<S, F> {
    /// Wraps `inner` so each reading passes through `select`.
    ///
    /// # Arguments
    ///
    /// * `inner` - the sensor being adapted.
    /// * `select` - the function applied to each reading.
    ///
    /// # Returns
    ///
    /// The adapted sensor.
    pub fn new(inner: S, select: F) -> Map<S, F> {
        Map { inner, select }
    }

    /// Borrows the wrapped sensor.
    ///
    /// # Returns
    ///
    /// The sensor underneath the adapter.
    pub fn inner(&self) -> &S {
        &self.inner
    }

    /// Mutably borrows the wrapped sensor.
    ///
    /// # Returns
    ///
    /// The sensor underneath the adapter.
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    /// Unwraps the sensor, discarding the adapter.
    ///
    /// # Returns
    ///
    /// The sensor underneath the adapter.
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S, F, T> Sensor for Map<S, F>
where
    S: Sensor,
    F: FnMut(S::Reading) -> T,
{
    type Reading = T;

    async fn read(&mut self) -> Result<T> {
        let reading = self.inner.read().await?;
        Ok((self.select)(reading))
    }
}

/// An actuator whose commands are converted before they reach the part.
///
/// Created by [`Actuator::map_command`]. The function runs on every command, so a part
/// that takes a pulse width or a duty can be driven by a `bool`, an angle, or a
/// percentage, whichever the caller has.
///
/// # Examples
///
/// ```
/// use pamoja_core::{Actuator, Result};
///
/// struct Pump {
///     duty: u8,
/// }
/// impl Actuator for Pump {
///     type Command = u8;
///     async fn apply(&mut self, duty: u8) -> Result<()> {
///         self.duty = duty;
///         Ok(())
///     }
/// }
///
/// # async fn run() -> Result<()> {
/// // A profile drives the pump as on or off; on means full speed.
/// let mut switch = Pump { duty: 0 }.map_command(|on: bool| if on { 100 } else { 0 });
/// switch.apply(true).await?;
/// assert_eq!(switch.inner().duty, 100);
/// # Ok(())
/// # }
/// ```
pub struct MapCommand<A, F, C> {
    inner: A,
    convert: F,
    command: PhantomData<fn(C)>,
}

impl<A, F, C> MapCommand<A, F, C> {
    /// Wraps `inner` so each command passes through `convert` first.
    ///
    /// # Arguments
    ///
    /// * `inner` - the actuator being adapted.
    /// * `convert` - the function turning the caller's command into the part's.
    ///
    /// # Returns
    ///
    /// The adapted actuator.
    pub fn new(inner: A, convert: F) -> MapCommand<A, F, C> {
        MapCommand {
            inner,
            convert,
            command: PhantomData,
        }
    }

    /// Borrows the wrapped actuator.
    ///
    /// # Returns
    ///
    /// The actuator underneath the adapter.
    pub fn inner(&self) -> &A {
        &self.inner
    }

    /// Mutably borrows the wrapped actuator.
    ///
    /// # Returns
    ///
    /// The actuator underneath the adapter.
    pub fn inner_mut(&mut self) -> &mut A {
        &mut self.inner
    }

    /// Unwraps the actuator, discarding the adapter.
    ///
    /// # Returns
    ///
    /// The actuator underneath the adapter.
    pub fn into_inner(self) -> A {
        self.inner
    }
}

impl<A, F, C> Actuator for MapCommand<A, F, C>
where
    A: Actuator,
    F: FnMut(C) -> A::Command,
{
    type Command = C;

    async fn apply(&mut self, command: C) -> Result<()> {
        let converted = (self.convert)(command);
        self.inner.apply(converted).await
    }
}
