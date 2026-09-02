//! The C ABI for device profiles.
//!
//! These wrap [`pamoja_profile`] for callers that reach the SDK through the flat
//! C boundary. A profile is a named, pre-wired bundle: a control policy, a
//! publish topic, and a power schedule, so someone who can put a sensor to good
//! use does not also have to choose algorithms and tuning constants by hand.
//!
//! Two things cross. A [`PamojaProfile`] is the manifest, which loads from and
//! saves to JSON, so a profile ships as a file that a community can publish and
//! a device can read. A [`PamojaController`] is the decision logic that manifest
//! describes: hand it a reading and it says what the output should do and
//! whether the reading crossed a threshold worth raising.
//!
//! The whole presentation layer, which declares how a profile appears on a
//! dashboard, travels inside the manifest JSON rather than as its own set of
//! calls. That keeps one representation of a profile across every language, and
//! it is the same JSON the dashboard already consumes.
//!
//! Assembling a running node from a profile stays in Rust, because the Rust
//! `Node` is generic over its sensor, actuator, transport, and codec, and the
//! four together do not cross a C ABI. Nothing is lost: the controller holds the
//! decisions, and the caller drives their own hardware around it.

use std::ffi::c_char;
use std::ptr;

use pamoja_profile::{Alert, ControlSpec, Controller, PowerSchedule, Profile};

use crate::power::PamojaPowerPlan;
use crate::{read_str, set_last_error, PamojaStatus, PamojaString};

/// Which control policy a profile applies to each reading.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaControlKind {
    /// Hold a reading near a setpoint by switching an output on and off.
    Setpoint = 0,
    /// Watch a falling level and warn before it reaches empty.
    Level = 1,
    /// Warn when a reading changes faster than a limit.
    Surge = 2,
    /// Report readings only, with no output and no alerts.
    Monitor = 3,
}

/// A control policy, flattened so every variant crosses as one value.
///
/// Only the fields belonging to `kind` carry meaning; the rest are zero.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaControlSpec {
    /// Which policy this describes.
    pub kind: PamojaControlKind,
    /// The target reading, for [`PamojaControlKind::Setpoint`].
    pub setpoint: f32,
    /// Half the deadband width, for [`PamojaControlKind::Setpoint`].
    pub hysteresis: f32,
    /// Whether the output cools rather than heats, for
    /// [`PamojaControlKind::Setpoint`].
    pub cooling: bool,
    /// How far the reading may stray before an alert, for
    /// [`PamojaControlKind::Setpoint`].
    pub safe_band: f32,
    /// The level treated as empty, for [`PamojaControlKind::Level`].
    pub empty: f32,
    /// How many samples ahead to warn, for [`PamojaControlKind::Level`].
    pub warn_within: u32,
    /// Whether a rise rather than a fall is watched, for
    /// [`PamojaControlKind::Surge`].
    pub rising: bool,
    /// The largest safe change per sample, for [`PamojaControlKind::Surge`].
    pub limit: f32,
}

/// How often a node samples as its battery drains, in whole seconds.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaPowerSchedule {
    /// Seconds between samples at a healthy charge.
    pub active_secs: u64,
    /// Seconds between samples while conserving.
    pub saver_secs: u64,
    /// Seconds between samples when critically low.
    pub critical_secs: u64,
    /// Enter the saver cadence below this state of charge.
    pub saver_below: f32,
    /// Enter the critical cadence below this state of charge.
    pub critical_below: f32,
}

/// Which threshold a reading crossed, if any.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PamojaAlertKind {
    /// The reading raised nothing.
    None = 0,
    /// A controlled reading drifted outside its safe band.
    OutOfRange = 1,
    /// A falling level will reach empty within a few more samples.
    RunningOut = 2,
    /// A reading is changing faster than its safe rate.
    ChangingFast = 3,
}

/// What a controller decided about one reading.
///
/// Only the field belonging to `alert` carries meaning; the rest are zero.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PamojaReaction {
    /// Whether the profile drives an output at all.
    ///
    /// `false` means the profile observes rather than controls, and `actuator`
    /// should be ignored.
    pub has_actuator: bool,
    /// The setting the output should take, when `has_actuator` is `true`.
    pub actuator: bool,
    /// Which threshold the reading crossed.
    pub alert: PamojaAlertKind,
    /// The offending reading, for [`PamojaAlertKind::OutOfRange`].
    pub reading: f32,
    /// The estimated samples until empty, for [`PamojaAlertKind::RunningOut`].
    pub samples: u32,
    /// The change since the previous sample, for
    /// [`PamojaAlertKind::ChangingFast`].
    pub rate: f32,
}

impl From<ControlSpec> for PamojaControlSpec {
    fn from(spec: ControlSpec) -> Self {
        let mut flat = Self {
            kind: PamojaControlKind::Monitor,
            setpoint: 0.0,
            hysteresis: 0.0,
            cooling: false,
            safe_band: 0.0,
            empty: 0.0,
            warn_within: 0,
            rising: false,
            limit: 0.0,
        };
        match spec {
            ControlSpec::Setpoint {
                setpoint,
                hysteresis,
                cooling,
                safe_band,
            } => {
                flat.kind = PamojaControlKind::Setpoint;
                flat.setpoint = setpoint;
                flat.hysteresis = hysteresis;
                flat.cooling = cooling;
                flat.safe_band = safe_band;
            }
            ControlSpec::Level { empty, warn_within } => {
                flat.kind = PamojaControlKind::Level;
                flat.empty = empty;
                flat.warn_within = warn_within;
            }
            ControlSpec::Surge { rising, limit } => {
                flat.kind = PamojaControlKind::Surge;
                flat.rising = rising;
                flat.limit = limit;
            }
            ControlSpec::Monitor => {}
        }
        flat
    }
}

impl From<PowerSchedule> for PamojaPowerSchedule {
    fn from(schedule: PowerSchedule) -> Self {
        Self {
            active_secs: schedule.active_secs,
            saver_secs: schedule.saver_secs,
            critical_secs: schedule.critical_secs,
            saver_below: schedule.saver_below,
            critical_below: schedule.critical_below,
        }
    }
}

impl PamojaReaction {
    /// Flattens a reaction into the shape that crosses the boundary.
    fn flatten(reaction: pamoja_profile::Reaction) -> Self {
        let mut flat = Self {
            has_actuator: reaction.actuator.is_some(),
            actuator: reaction.actuator.unwrap_or(false),
            alert: PamojaAlertKind::None,
            reading: 0.0,
            samples: 0,
            rate: 0.0,
        };
        match reaction.alert {
            None => {}
            Some(Alert::OutOfRange { reading }) => {
                flat.alert = PamojaAlertKind::OutOfRange;
                flat.reading = reading;
            }
            Some(Alert::RunningOut { samples }) => {
                flat.alert = PamojaAlertKind::RunningOut;
                flat.samples = samples;
            }
            Some(Alert::ChangingFast { rate }) => {
                flat.alert = PamojaAlertKind::ChangingFast;
                flat.rate = rate;
            }
        }
        flat
    }
}

/// An opaque handle to a device profile.
pub struct PamojaProfile {
    inner: Profile,
}

impl PamojaProfile {
    /// Wraps a profile in a handle for the caller to own.
    fn into_raw(inner: Profile) -> *mut Self {
        Box::into_raw(Box::new(Self { inner }))
    }
}

/// Creates a cold-chain fridge monitor, which holds 5 C and flags an excursion.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_profile_free`].
#[no_mangle]
pub extern "C" fn pamoja_profile_vaccine_fridge_monitor() -> *mut PamojaProfile {
    PamojaProfile::into_raw(Profile::vaccine_fridge_monitor())
}

/// Creates an irrigation node, which opens a valve as soil moisture falls.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_profile_free`].
#[no_mangle]
pub extern "C" fn pamoja_profile_irrigation_node() -> *mut PamojaProfile {
    PamojaProfile::into_raw(Profile::irrigation_node())
}

/// Creates a well-level monitor, which warns before a tank runs dry.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_profile_free`].
#[no_mangle]
pub extern "C" fn pamoja_profile_well_level() -> *mut PamojaProfile {
    PamojaProfile::into_raw(Profile::well_level())
}

/// Creates a flood sensor, which warns when a level rises too fast.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_profile_free`].
#[no_mangle]
pub extern "C" fn pamoja_profile_flood_sensor() -> *mut PamojaProfile {
    PamojaProfile::into_raw(Profile::flood_sensor())
}

/// Loads a profile from its JSON manifest.
///
/// # Arguments
///
/// * `manifest` - the manifest, as null-terminated UTF-8.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_profile_free`], or null if the
/// manifest is malformed or `manifest` is null, with the reason available from
/// [`pamoja_last_error_message`](crate::pamoja_last_error_message).
///
/// # Safety
///
/// `manifest` must be a valid null-terminated UTF-8 string for the duration of
/// the call, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_profile_from_json(manifest: *const c_char) -> *mut PamojaProfile {
    let Some(manifest) = read_str(manifest, "manifest") else {
        return ptr::null_mut();
    };
    match Profile::from_json(manifest) {
        Ok(profile) => PamojaProfile::into_raw(profile),
        Err(error) => {
            set_last_error(error.to_string());
            ptr::null_mut()
        }
    }
}

/// Serializes a profile to its JSON manifest.
///
/// # Arguments
///
/// * `profile` - the profile.
///
/// # Returns
///
/// A string the caller must release with
/// [`pamoja_string_free`](crate::pamoja_string_free), or null if `profile` is
/// null or the profile cannot be serialized.
///
/// # Safety
///
/// `profile` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_profile_to_json(
    profile: *const PamojaProfile,
) -> *mut PamojaString {
    let Some(profile) = profile_handle(profile) else {
        return ptr::null_mut();
    };
    match profile.inner.to_json() {
        Ok(manifest) => PamojaString::into_raw(manifest),
        Err(error) => {
            set_last_error(error.to_string());
            ptr::null_mut()
        }
    }
}

/// Returns a profile's stable name.
///
/// # Arguments
///
/// * `profile` - the profile.
///
/// # Returns
///
/// A null-terminated UTF-8 string, which the caller must release with
/// [`pamoja_string_free`](crate::pamoja_string_free), or null if `profile` is
/// null.
///
/// # Safety
///
/// `profile` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_profile_name(profile: *const PamojaProfile) -> *mut PamojaString {
    match profile_handle(profile) {
        Some(profile) => PamojaString::into_raw(profile.inner.name.clone()),
        None => ptr::null_mut(),
    }
}

/// Returns the topic a profile publishes each reading to.
///
/// # Arguments
///
/// * `profile` - the profile.
///
/// # Returns
///
/// A null-terminated UTF-8 string, which the caller must release with
/// [`pamoja_string_free`](crate::pamoja_string_free), or null if `profile` is
/// null.
///
/// # Safety
///
/// `profile` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_profile_topic(profile: *const PamojaProfile) -> *mut PamojaString {
    match profile_handle(profile) {
        Some(profile) => PamojaString::into_raw(profile.inner.topic.clone()),
        None => ptr::null_mut(),
    }
}

/// Returns the control policy a profile applies.
///
/// # Arguments
///
/// * `profile` - the profile.
/// * `out_control` - receives the policy.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, or [`PamojaStatus::InvalidArgument`] if
/// either pointer is null.
///
/// # Safety
///
/// `profile` must be a live handle from a call that produced one, and
/// `out_control` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_profile_control(
    profile: *const PamojaProfile,
    out_control: *mut PamojaControlSpec,
) -> PamojaStatus {
    if out_control.is_null() {
        set_last_error("out_control must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(profile) = profile_handle(profile) else {
        return PamojaStatus::InvalidArgument;
    };
    *out_control = profile.inner.control.into();
    PamojaStatus::Ok
}

/// Returns the sampling schedule a profile keeps as its battery drains.
///
/// # Arguments
///
/// * `profile` - the profile.
/// * `out_schedule` - receives the schedule.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, or [`PamojaStatus::InvalidArgument`] if
/// either pointer is null.
///
/// # Safety
///
/// `profile` must be a live handle from a call that produced one, and
/// `out_schedule` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_profile_power(
    profile: *const PamojaProfile,
    out_schedule: *mut PamojaPowerSchedule,
) -> PamojaStatus {
    if out_schedule.is_null() {
        set_last_error("out_schedule must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(profile) = profile_handle(profile) else {
        return PamojaStatus::InvalidArgument;
    };
    *out_schedule = profile.inner.power.into();
    PamojaStatus::Ok
}

/// Assembles a profile's schedule into a power governor.
///
/// The governor is the same one [`pamoja_power_plan_new`](crate::power::pamoja_power_plan_new)
/// builds, so the mode and interval calls in that module apply to it unchanged.
///
/// # Arguments
///
/// * `profile` - the profile.
/// * `out_plan` - receives the governor.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, or [`PamojaStatus::InvalidArgument`] if
/// either pointer is null.
///
/// # Safety
///
/// `profile` must be a live handle from a call that produced one, and `out_plan`
/// must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_profile_power_plan(
    profile: *const PamojaProfile,
    out_plan: *mut PamojaPowerPlan,
) -> PamojaStatus {
    if out_plan.is_null() {
        set_last_error("out_plan must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(profile) = profile_handle(profile) else {
        return PamojaStatus::InvalidArgument;
    };
    let schedule = profile.inner.power;
    *out_plan = PamojaPowerPlan {
        active_us: schedule.active_secs.saturating_mul(1_000_000),
        saver_us: schedule.saver_secs.saturating_mul(1_000_000),
        critical_us: schedule.critical_secs.saturating_mul(1_000_000),
        saver_below: schedule.saver_below,
        critical_below: schedule.critical_below,
    };
    PamojaStatus::Ok
}

/// Builds the decision logic a profile describes.
///
/// # Arguments
///
/// * `profile` - the profile.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_controller_free`], or null if
/// `profile` is null.
///
/// # Safety
///
/// `profile` must be a live handle from a call that produced one, or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_profile_controller(
    profile: *const PamojaProfile,
) -> *mut PamojaController {
    match profile_handle(profile) {
        Some(profile) => PamojaController::into_raw(profile.inner.controller()),
        None => ptr::null_mut(),
    }
}

/// Releases a profile handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `profile` must be a handle from a call that produced one and that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_profile_free(profile: *mut PamojaProfile) {
    if !profile.is_null() {
        drop(Box::from_raw(profile));
    }
}

/// An opaque handle to a profile's decision logic.
///
/// A controller carries state between readings, because a level estimate and a
/// rate of change both need the previous sample, so evaluate readings through
/// one controller in the order they were taken.
pub struct PamojaController {
    inner: Controller,
}

impl PamojaController {
    /// Wraps a controller in a handle for the caller to own.
    fn into_raw(inner: Controller) -> *mut Self {
        Box::into_raw(Box::new(Self { inner }))
    }
}

/// Creates a controller that holds a reading near a setpoint.
///
/// # Arguments
///
/// * `setpoint` - the target reading.
/// * `hysteresis` - half the deadband width, which stops the output chattering.
/// * `cooling` - whether the output cools rather than heats.
/// * `safe_band` - how far the reading may stray before an alert.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_controller_free`].
#[no_mangle]
pub extern "C" fn pamoja_controller_setpoint(
    setpoint: f32,
    hysteresis: f32,
    cooling: bool,
    safe_band: f32,
) -> *mut PamojaController {
    PamojaController::into_raw(Controller::setpoint(
        setpoint, hysteresis, cooling, safe_band,
    ))
}

/// Creates a controller that warns before a falling level reaches empty.
///
/// # Arguments
///
/// * `empty` - the level treated as empty.
/// * `warn_within` - warn once empty is this many samples away.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_controller_free`].
#[no_mangle]
pub extern "C" fn pamoja_controller_level(empty: f32, warn_within: u32) -> *mut PamojaController {
    PamojaController::into_raw(Controller::level(empty, warn_within))
}

/// Creates a controller that warns when a reading changes too fast.
///
/// # Arguments
///
/// * `rising` - watch a rapid rise rather than a rapid fall.
/// * `limit` - the largest safe change per sample.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_controller_free`].
#[no_mangle]
pub extern "C" fn pamoja_controller_surge(rising: bool, limit: f32) -> *mut PamojaController {
    PamojaController::into_raw(Controller::surge(rising, limit))
}

/// Creates a controller that reports readings without judging them.
///
/// # Returns
///
/// A handle the caller must release with [`pamoja_controller_free`].
#[no_mangle]
pub extern "C" fn pamoja_controller_monitor() -> *mut PamojaController {
    PamojaController::into_raw(Controller::monitor())
}

/// Decides what one reading calls for.
///
/// # Arguments
///
/// * `controller` - the decision logic.
/// * `reading` - the reading to evaluate.
/// * `out_reaction` - receives the decision.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, or [`PamojaStatus::InvalidArgument`] if
/// either pointer is null.
///
/// # Safety
///
/// `controller` must be a live handle from a call that produced one, and
/// `out_reaction` must be writable.
#[no_mangle]
pub unsafe extern "C" fn pamoja_controller_evaluate(
    controller: *mut PamojaController,
    reading: f32,
    out_reaction: *mut PamojaReaction,
) -> PamojaStatus {
    if controller.is_null() {
        set_last_error("controller must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    if out_reaction.is_null() {
        set_last_error("out_reaction must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let controller = &mut *controller;
    *out_reaction = PamojaReaction::flatten(controller.inner.evaluate(reading));
    PamojaStatus::Ok
}

/// Releases a controller handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `controller` must be a handle from a call that produced one and that has not
/// already been freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_controller_free(controller: *mut PamojaController) {
    if !controller.is_null() {
        drop(Box::from_raw(controller));
    }
}

/// Borrows a profile handle, rejecting a null pointer.
///
/// # Safety
///
/// `profile` must be a live handle from a call that produced one, or null.
unsafe fn profile_handle<'a>(profile: *const PamojaProfile) -> Option<&'a PamojaProfile> {
    if profile.is_null() {
        set_last_error("profile must not be null".to_owned());
        return None;
    }
    Some(&*profile)
}

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};

    use super::*;

    fn text_of(string: *mut PamojaString) -> String {
        assert!(!string.is_null(), "the call produced no string");
        let text = unsafe { CStr::from_ptr(crate::pamoja_string_data(string)) }
            .to_str()
            .expect("utf-8")
            .to_owned();
        unsafe { crate::pamoja_string_free(string) };
        text
    }

    fn reaction_for(controller: *mut PamojaController, reading: f32) -> PamojaReaction {
        let mut reaction = PamojaReaction {
            has_actuator: false,
            actuator: false,
            alert: PamojaAlertKind::None,
            reading: 0.0,
            samples: 0,
            rate: 0.0,
        };
        assert_eq!(
            unsafe { pamoja_controller_evaluate(controller, reading, &mut reaction) },
            PamojaStatus::Ok
        );
        reaction
    }

    #[test]
    fn a_warm_fridge_runs_the_cooler_and_flags_the_excursion() {
        let profile = pamoja_profile_vaccine_fridge_monitor();
        let controller = unsafe { pamoja_profile_controller(profile) };
        let reaction = reaction_for(controller, 9.0);

        assert!(reaction.has_actuator, "the profile drives a cooler");
        assert!(reaction.actuator, "which runs when the fridge is warm");
        assert_eq!(
            reaction.alert,
            PamojaAlertKind::OutOfRange,
            "and 9 C is outside the safe band"
        );
        assert_eq!(reaction.reading, 9.0);

        unsafe {
            pamoja_controller_free(controller);
            pamoja_profile_free(profile);
        }
    }

    #[test]
    fn a_monitoring_profile_drives_no_output() {
        let controller = pamoja_controller_monitor();
        let reaction = reaction_for(controller, 21.5);
        assert!(
            !reaction.has_actuator,
            "a monitor observes rather than acts"
        );
        assert_eq!(reaction.alert, PamojaAlertKind::None);
        unsafe { pamoja_controller_free(controller) };
    }

    #[test]
    fn a_manifest_survives_a_round_trip() {
        let profile = pamoja_profile_well_level();
        let manifest = text_of(unsafe { pamoja_profile_to_json(profile) });

        let text = CString::new(manifest).expect("no interior null");
        let loaded = unsafe { pamoja_profile_from_json(text.as_ptr()) };
        assert!(!loaded.is_null());

        assert_eq!(
            text_of(unsafe { pamoja_profile_name(loaded) }),
            text_of(unsafe { pamoja_profile_name(profile) })
        );

        let mut original = PamojaControlSpec::from(ControlSpec::Monitor);
        let mut restored = original;
        unsafe {
            assert_eq!(
                pamoja_profile_control(profile, &mut original),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_profile_control(loaded, &mut restored),
                PamojaStatus::Ok
            );
            pamoja_profile_free(loaded);
            pamoja_profile_free(profile);
        }
        assert_eq!(restored.kind, PamojaControlKind::Level);
        assert_eq!(restored, original, "the policy came back unchanged");
    }

    #[test]
    fn a_schedule_becomes_a_governor_in_microseconds() {
        let profile = pamoja_profile_flood_sensor();
        let mut schedule = PamojaPowerSchedule {
            active_secs: 0,
            saver_secs: 0,
            critical_secs: 0,
            saver_below: 0.0,
            critical_below: 0.0,
        };
        let mut plan = PamojaPowerPlan {
            active_us: 0,
            saver_us: 0,
            critical_us: 0,
            saver_below: 0.0,
            critical_below: 0.0,
        };
        unsafe {
            assert_eq!(
                pamoja_profile_power(profile, &mut schedule),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_profile_power_plan(profile, &mut plan),
                PamojaStatus::Ok
            );
            pamoja_profile_free(profile);
        }
        assert_eq!(plan.active_us, schedule.active_secs * 1_000_000);
        assert_eq!(plan.saver_below, schedule.saver_below);
    }

    #[test]
    fn a_null_argument_is_rejected_rather_than_dereferenced() {
        assert!(unsafe { pamoja_profile_from_json(ptr::null()) }.is_null());
        assert!(unsafe { pamoja_profile_to_json(ptr::null()) }.is_null());
        assert!(unsafe { pamoja_profile_name(ptr::null()) }.is_null());
        assert!(unsafe { pamoja_profile_topic(ptr::null()) }.is_null());
        assert!(unsafe { pamoja_profile_controller(ptr::null()) }.is_null());
        assert_eq!(
            unsafe { pamoja_profile_control(ptr::null(), ptr::null_mut()) },
            PamojaStatus::InvalidArgument
        );
        assert_eq!(
            unsafe { pamoja_controller_evaluate(ptr::null_mut(), 0.0, ptr::null_mut()) },
            PamojaStatus::InvalidArgument
        );
    }
}
