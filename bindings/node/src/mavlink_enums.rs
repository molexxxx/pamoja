//! The MAVLink dialect's named values.
//!
//! An enum field rides on the wire as a plain integer, so setting `type` on a heartbeat or
//! reading `result` off an acknowledgment works in numbers. These calls turn the names the
//! dialect writes, such as `MAV_TYPE_QUADROTOR`, into those numbers and back, for every
//! enumeration a field of a typed message names.

use napi_derive::napi;
use pamoja_mavlink::dialect::{entry_value, enum_named, EnumDescriptor, ENUMS};

/// The largest whole number a JavaScript number holds exactly, `2^53 - 1`.
const SAFE_INTEGER: u64 = (1 << 53) - 1;

/// One named value of a dialect enumeration.
#[napi(object)]
pub struct MavlinkEnumEntry {
    /// The name the dialect gives the value, such as `MAV_STATE_STANDBY`.
    pub name: String,
    /// The value on the wire.
    pub value: f64,
}

fn enumeration(name: &str) -> napi::Result<&'static EnumDescriptor> {
    enum_named(name)
        .ok_or_else(|| napi::Error::from_reason(format!("{name} is not a dialect enumeration")))
}

fn whole(value: f64) -> napi::Result<u64> {
    if value.fract() == 0.0 && (0.0..=SAFE_INTEGER as f64).contains(&value) {
        Ok(value as u64)
    } else {
        Err(napi::Error::from_reason(format!(
            "a value must be a whole number from 0 up, not {value}"
        )))
    }
}

/// Looks up the value a dialect entry name stands for, whichever enumeration it belongs to.
///
/// @param entry - the entry's name, such as `MAV_CMD_COMPONENT_ARM_DISARM`.
/// @returns The value.
/// @throws If no entry of any dialect enumeration has that name.
#[napi]
pub fn mavlink_enum_value(entry: String) -> napi::Result<f64> {
    entry_value(&entry)
        .map(|value| value as f64)
        .ok_or_else(|| {
            napi::Error::from_reason(format!(
                "{entry} is not an entry of any dialect enumeration"
            ))
        })
}

/// Names the entry of an enumeration that stands for a value, or `null` if none does.
///
/// @param enumeration - the enumeration's name, such as `MAV_STATE`.
/// @param value - the value a field carried.
/// @throws If the enumeration is unknown or the value is not a whole number from 0 up.
#[napi]
pub fn mavlink_enum_entry(enumeration: String, value: f64) -> napi::Result<Option<String>> {
    let described = self::enumeration(&enumeration)?;
    Ok(described.entry(whole(value)?).map(str::to_owned))
}

/// Names the entries a value is made of: for a bitmask, each entry whose bits are set in it,
/// in dialect order; otherwise the one entry that names it. Empty when none applies.
///
/// @param enumeration - the enumeration's name, such as `MAV_MODE_FLAG`.
/// @param value - the value a field carried.
/// @throws If the enumeration is unknown or the value is not a whole number from 0 up.
#[napi]
pub fn mavlink_enum_names(enumeration: String, value: f64) -> napi::Result<Vec<String>> {
    let described = self::enumeration(&enumeration)?;
    Ok(described.names(whole(value)?).map(str::to_owned).collect())
}

/// Every entry of an enumeration, in dialect order.
///
/// @param enumeration - the enumeration's name.
/// @throws If the enumeration is unknown.
#[napi]
pub fn mavlink_enum_entries(enumeration: String) -> napi::Result<Vec<MavlinkEnumEntry>> {
    let described = self::enumeration(&enumeration)?;
    Ok(described
        .entries
        .iter()
        .map(|entry| MavlinkEnumEntry {
            name: entry.name.to_owned(),
            value: entry.value as f64,
        })
        .collect())
}

/// Reports whether an enumeration's values combine as bits.
///
/// @param enumeration - the enumeration's name.
/// @throws If the enumeration is unknown.
#[napi]
pub fn mavlink_enum_is_bitmask(enumeration: String) -> napi::Result<bool> {
    Ok(self::enumeration(&enumeration)?.bitmask)
}

/// The names of every enumeration the dialect table holds, in the order the dialect defines
/// them.
#[napi]
pub fn mavlink_known_enums() -> Vec<String> {
    ENUMS
        .iter()
        .map(|described| described.name.to_owned())
        .collect()
}
