//! The C ABI for the MAVLink dialect's named values.
//!
//! An enum field rides on the wire as a plain integer, so a caller that sets `type` on a
//! heartbeat or reads `result` off an acknowledgment works in numbers. These calls turn the
//! names the dialect writes, such as `MAV_TYPE_QUADROTOR`, into those numbers and back, for
//! every enumeration a field of a typed message names, so a caller never copies a value out of
//! the dialect by hand.

use std::ffi::c_char;

use pamoja_mavlink::dialect::{entry_value, enum_named, EnumDescriptor, ENUMS};

use crate::{read_str, set_last_error, PamojaStatus, PamojaString};

unsafe fn enumeration(name: *const c_char) -> Result<&'static EnumDescriptor, PamojaStatus> {
    let Some(name) = read_str(name, "enumeration") else {
        return Err(PamojaStatus::InvalidArgument);
    };
    enum_named(name).ok_or_else(|| {
        set_last_error(format!("{name} is not a dialect enumeration"));
        PamojaStatus::InvalidArgument
    })
}

/// Looks up the value a dialect entry name stands for, whichever enumeration it belongs to.
///
/// # Arguments
///
/// * `entry` - the entry's name, such as `MAV_CMD_COMPONENT_ARM_DISARM`, as null-terminated
///   UTF-8.
/// * `out_value` - receives the value.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] if no entry has that name or an
/// argument is null.
///
/// # Safety
///
/// `entry` must be a valid null-terminated UTF-8 string or null, and `out_value` must be
/// valid for a write or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_enum_value(
    entry: *const c_char,
    out_value: *mut u64,
) -> PamojaStatus {
    if out_value.is_null() {
        set_last_error("out_value must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(entry) = read_str(entry, "entry") else {
        return PamojaStatus::InvalidArgument;
    };
    match entry_value(entry) {
        Some(value) => {
            *out_value = value;
            PamojaStatus::Ok
        }
        None => {
            set_last_error(format!(
                "{entry} is not an entry of any dialect enumeration"
            ));
            PamojaStatus::InvalidArgument
        }
    }
}

/// Names the entry of an enumeration that stands for a value.
///
/// # Arguments
///
/// * `enumeration` - the enumeration's name, such as `MAV_STATE`, as null-terminated UTF-8.
/// * `value` - the value a field carried.
/// * `out_name` - receives the entry's name, which the caller releases with
///   [`pamoja_string_free`](crate::pamoja_string_free), or null when no entry names the value.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], whether or not an entry names the value, or
/// [`PamojaStatus::InvalidArgument`] if the enumeration is unknown or an argument is null.
///
/// # Safety
///
/// `enumeration` must be a valid null-terminated UTF-8 string or null, and `out_name` must be
/// valid for a write or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_enum_entry(
    enumeration: *const c_char,
    value: u64,
    out_name: *mut *mut PamojaString,
) -> PamojaStatus {
    if out_name.is_null() {
        set_last_error("out_name must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let described = match self::enumeration(enumeration) {
        Ok(described) => described,
        Err(status) => return status,
    };
    *out_name = match described.entry(value) {
        Some(name) => PamojaString::into_raw(name.to_owned()),
        None => std::ptr::null_mut(),
    };
    PamojaStatus::Ok
}

/// Names the entries a value is made of: for a bitmask, each entry whose bits are set in it;
/// otherwise the one entry that names it.
///
/// # Arguments
///
/// * `enumeration` - the enumeration's name, as null-terminated UTF-8.
/// * `value` - the value a field carried.
/// * `out_names` - receives the names in dialect order joined by `|`, empty when none applies,
///   which the caller releases with [`pamoja_string_free`](crate::pamoja_string_free).
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] if the enumeration is unknown or
/// an argument is null.
///
/// # Safety
///
/// `enumeration` must be a valid null-terminated UTF-8 string or null, and `out_names` must be
/// valid for a write or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_enum_names(
    enumeration: *const c_char,
    value: u64,
    out_names: *mut *mut PamojaString,
) -> PamojaStatus {
    if out_names.is_null() {
        set_last_error("out_names must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let described = match self::enumeration(enumeration) {
        Ok(described) => described,
        Err(status) => return status,
    };
    let names: Vec<&str> = described.names(value).collect();
    *out_names = PamojaString::into_raw(names.join("|"));
    PamojaStatus::Ok
}

/// Returns how many enumerations the dialect table holds.
///
/// # Returns
///
/// The count, for walking the table with [`pamoja_mavlink_enum_at`].
#[no_mangle]
pub extern "C" fn pamoja_mavlink_enum_count() -> usize {
    ENUMS.len()
}

/// Returns the name of the enumeration at an index of the dialect table.
///
/// # Arguments
///
/// * `index` - the position, from `0` below [`pamoja_mavlink_enum_count`].
/// * `out_name` - receives the name, which the caller releases with
///   [`pamoja_string_free`](crate::pamoja_string_free).
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] if `index` is past the end or
/// `out_name` is null.
///
/// # Safety
///
/// `out_name` must be valid for a write or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_enum_at(
    index: usize,
    out_name: *mut *mut PamojaString,
) -> PamojaStatus {
    if out_name.is_null() {
        set_last_error("out_name must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let Some(described) = ENUMS.get(index) else {
        set_last_error(format!("{index} is past the end of the dialect table"));
        return PamojaStatus::InvalidArgument;
    };
    *out_name = PamojaString::into_raw(described.name.to_owned());
    PamojaStatus::Ok
}

/// Reports whether an enumeration's values combine as bits.
///
/// # Arguments
///
/// * `enumeration` - the enumeration's name, as null-terminated UTF-8.
/// * `out_bitmask` - receives `true` for a bitmask.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] if the enumeration is unknown or
/// an argument is null.
///
/// # Safety
///
/// `enumeration` must be a valid null-terminated UTF-8 string or null, and `out_bitmask` must
/// be valid for a write or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_enum_is_bitmask(
    enumeration: *const c_char,
    out_bitmask: *mut bool,
) -> PamojaStatus {
    if out_bitmask.is_null() {
        set_last_error("out_bitmask must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match self::enumeration(enumeration) {
        Ok(described) => {
            *out_bitmask = described.bitmask;
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Returns how many entries an enumeration names.
///
/// # Arguments
///
/// * `enumeration` - the enumeration's name, as null-terminated UTF-8.
/// * `out_len` - receives the count, for walking the entries with
///   [`pamoja_mavlink_enum_entry_at`].
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] if the enumeration is unknown or
/// an argument is null.
///
/// # Safety
///
/// `enumeration` must be a valid null-terminated UTF-8 string or null, and `out_len` must be
/// valid for a write or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_enum_len(
    enumeration: *const c_char,
    out_len: *mut usize,
) -> PamojaStatus {
    if out_len.is_null() {
        set_last_error("out_len must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match self::enumeration(enumeration) {
        Ok(described) => {
            *out_len = described.entries.len();
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Returns an enumeration's entry at an index, in dialect order.
///
/// # Arguments
///
/// * `enumeration` - the enumeration's name, as null-terminated UTF-8.
/// * `index` - the position, from `0` below the count [`pamoja_mavlink_enum_len`] gives.
/// * `out_name` - receives the entry's name, which the caller releases with
///   [`pamoja_string_free`](crate::pamoja_string_free).
/// * `out_value` - receives the value it names.
///
/// # Returns
///
/// [`PamojaStatus::Ok`], or [`PamojaStatus::InvalidArgument`] if the enumeration is unknown,
/// `index` is past the end, or an argument is null.
///
/// # Safety
///
/// `enumeration` must be a valid null-terminated UTF-8 string or null, and `out_name` and
/// `out_value` must be valid for a write or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_mavlink_enum_entry_at(
    enumeration: *const c_char,
    index: usize,
    out_name: *mut *mut PamojaString,
    out_value: *mut u64,
) -> PamojaStatus {
    if out_name.is_null() || out_value.is_null() {
        set_last_error("out_name and out_value must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let described = match self::enumeration(enumeration) {
        Ok(described) => described,
        Err(status) => return status,
    };
    let Some(entry) = described.entries.get(index) else {
        set_last_error(format!(
            "{index} is past the end of {}, which names {}",
            described.name,
            described.entries.len()
        ));
        return PamojaStatus::InvalidArgument;
    };
    *out_name = PamojaString::into_raw(entry.name.to_owned());
    *out_value = entry.value;
    PamojaStatus::Ok
}

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};
    use std::ptr;

    use super::*;

    unsafe fn text(string: *mut PamojaString) -> String {
        let read = CStr::from_ptr(crate::pamoja_string_data(string))
            .to_str()
            .expect("utf-8")
            .to_owned();
        crate::pamoja_string_free(string);
        read
    }

    #[test]
    fn a_name_becomes_its_value_and_back() {
        let entry = CString::new("MAV_TYPE_QUADROTOR").expect("static");
        let mut value = 0;
        assert_eq!(
            unsafe { pamoja_mavlink_enum_value(entry.as_ptr(), &mut value) },
            PamojaStatus::Ok
        );
        assert_eq!(value, 2);

        let enumeration = CString::new("MAV_TYPE").expect("static");
        let mut name = ptr::null_mut();
        assert_eq!(
            unsafe { pamoja_mavlink_enum_entry(enumeration.as_ptr(), 2, &mut name) },
            PamojaStatus::Ok
        );
        assert_eq!(unsafe { text(name) }, "MAV_TYPE_QUADROTOR");

        assert_eq!(
            unsafe { pamoja_mavlink_enum_entry(enumeration.as_ptr(), 250, &mut name) },
            PamojaStatus::Ok
        );
        assert!(name.is_null(), "no entry names 250");
    }

    #[test]
    fn a_bitmask_is_named_bit_by_bit() {
        let enumeration = CString::new("MAV_MODE_FLAG").expect("static");
        let mut names = ptr::null_mut();
        assert_eq!(
            unsafe { pamoja_mavlink_enum_names(enumeration.as_ptr(), 129, &mut names) },
            PamojaStatus::Ok
        );
        assert_eq!(
            unsafe { text(names) },
            "MAV_MODE_FLAG_SAFETY_ARMED|MAV_MODE_FLAG_CUSTOM_MODE_ENABLED"
        );
        let mut bitmask = false;
        assert_eq!(
            unsafe { pamoja_mavlink_enum_is_bitmask(enumeration.as_ptr(), &mut bitmask) },
            PamojaStatus::Ok
        );
        assert!(bitmask);
    }

    #[test]
    fn the_table_can_be_walked() {
        let count = pamoja_mavlink_enum_count();
        assert!(count > 0);
        let mut name = ptr::null_mut();
        assert_eq!(
            unsafe { pamoja_mavlink_enum_at(0, &mut name) },
            PamojaStatus::Ok
        );
        let first = CString::new(unsafe { text(name) }).expect("no interior nul");
        let mut len = 0;
        assert_eq!(
            unsafe { pamoja_mavlink_enum_len(first.as_ptr(), &mut len) },
            PamojaStatus::Ok
        );
        let mut value = 0;
        assert_eq!(
            unsafe { pamoja_mavlink_enum_entry_at(first.as_ptr(), len - 1, &mut name, &mut value) },
            PamojaStatus::Ok
        );
        unsafe { crate::pamoja_string_free(name) };
        assert_eq!(
            unsafe { pamoja_mavlink_enum_entry_at(first.as_ptr(), len, &mut name, &mut value) },
            PamojaStatus::InvalidArgument
        );
        assert_eq!(
            unsafe { pamoja_mavlink_enum_at(count, &mut name) },
            PamojaStatus::InvalidArgument
        );
    }

    #[test]
    fn an_unknown_name_is_refused() {
        let entry = CString::new("MAV_TYPE_TELEPORTER").expect("static");
        let mut value = 0;
        assert_eq!(
            unsafe { pamoja_mavlink_enum_value(entry.as_ptr(), &mut value) },
            PamojaStatus::InvalidArgument
        );
        let enumeration = CString::new("MAV_TYPES").expect("static");
        let mut name = ptr::null_mut();
        assert_eq!(
            unsafe { pamoja_mavlink_enum_entry(enumeration.as_ptr(), 2, &mut name) },
            PamojaStatus::InvalidArgument
        );
        assert_eq!(
            unsafe { pamoja_mavlink_enum_value(ptr::null(), &mut value) },
            PamojaStatus::InvalidArgument
        );
    }
}
