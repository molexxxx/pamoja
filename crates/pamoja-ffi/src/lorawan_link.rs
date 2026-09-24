//! The C ABI for what keeps a LoRaWAN link running: the specification revision a device
//! follows, the timings and counts its regional parameters recommend, the back-off that
//! hands back what adaptive data rate tuned away once the network falls silent, and the
//! channel list a join accept can carry.
//!
//! A back-off is a counter that moves with every uplink, so it crosses as an opaque
//! handle. A channel list is sixteen bytes, so it crosses as those bytes and every call
//! reads or writes them in place.

use std::ptr;

use pamoja_lorawan::adr::{Backoff, Standing};
use pamoja_lorawan::{
    CfList, CfListKind, Version, CFLIST_FREQUENCIES, CFLIST_LEN, CFLIST_MASK_GROUPS,
};

use crate::{read_bytes, set_last_error, PamojaStatus};

/// LoRaWAN 1.0.3.
pub const PAMOJA_LORAWAN_VERSION_1_0_3: u8 = 3;
/// TS001-1.0.4, the LoRaWAN 1.0.4 link layer.
pub const PAMOJA_LORAWAN_VERSION_1_0_4: u8 = 4;

/// How long after an uplink the first receive window opens, RP002-1.0.5 section 3.3.
pub const PAMOJA_LORAWAN_RECEIVE_DELAY1_US: u32 = 1_000_000;
/// How long after an uplink the second receive window opens.
pub const PAMOJA_LORAWAN_RECEIVE_DELAY2_US: u32 = 2_000_000;
/// How long after a join request the first join accept window opens.
pub const PAMOJA_LORAWAN_JOIN_ACCEPT_DELAY1_US: u32 = 5_000_000;
/// How long after a join request the second join accept window opens.
pub const PAMOJA_LORAWAN_JOIN_ACCEPT_DELAY2_US: u32 = 6_000_000;
/// How far a receive window may open either side of its time, LoRaWAN 1.0.3 section 3.3.1.
pub const PAMOJA_LORAWAN_RECEIVE_WINDOW_TOLERANCE_US: u32 = 20;
/// The largest gap a frame counter may jump across and still be accepted.
pub const PAMOJA_LORAWAN_MAX_FCNT_GAP: u32 = 16_384;
/// How many unanswered uplinks before a device asks the network to answer.
pub const PAMOJA_LORAWAN_ADR_ACK_LIMIT: u32 = 64;
/// How many more before a device starts giving back what adaptive data rate took.
pub const PAMOJA_LORAWAN_ADR_ACK_DELAY: u32 = 32;
/// The shortest wait before a confirmed uplink is sent again.
pub const PAMOJA_LORAWAN_RETRANSMIT_TIMEOUT_MIN_US: u32 = 1_000_000;
/// The longest wait before a confirmed uplink is sent again.
pub const PAMOJA_LORAWAN_RETRANSMIT_TIMEOUT_MAX_US: u32 = 3_000_000;

/// The number of bytes a channel list occupies.
pub const PAMOJA_LORAWAN_CFLIST_LEN: usize = 16;
/// How many frequencies a type 0 channel list carries.
pub const PAMOJA_LORAWAN_CFLIST_FREQUENCIES: usize = 5;
/// How many sixteen-bit masks a type 1 channel list carries.
pub const PAMOJA_LORAWAN_CFLIST_MASK_GROUPS: usize = 6;
/// The CFListType byte of a list of frequencies.
pub const PAMOJA_LORAWAN_CFLIST_TYPE_FREQUENCIES: u8 = 0;
/// The CFListType byte of a list of channel mask groups.
pub const PAMOJA_LORAWAN_CFLIST_TYPE_CHANNEL_MASKS: u8 = 1;

/// What a back-off says to do with one uplink.
///
/// Each field is `1` for yes and `0` for no.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PamojaLorawanBackoffStep {
    /// Set the ADRACKReq bit, asking the network to answer.
    pub request_ack: u8,
    /// Go back to the default transmit power before sending. Only TS001-1.0.4 takes this
    /// step.
    pub restore_power: u8,
    /// Step the data rate down by the region's back-off table before sending.
    pub lower_data_rate: u8,
    /// Re-enable the default channels and set the repetition count back to one before
    /// sending. Only TS001-1.0.4 takes this step.
    pub restore_channels: u8,
}

/// An opaque handle to a device's count of how long the network has been silent.
///
/// Release it with [`pamoja_lorawan_backoff_free`].
pub struct PamojaLorawanBackoff {
    backoff: Backoff,
}

/// Reads a revision code that crossed the boundary.
pub(crate) fn version(code: u8) -> Result<Version, PamojaStatus> {
    match code {
        PAMOJA_LORAWAN_VERSION_1_0_3 => Ok(Version::V1_0_3),
        PAMOJA_LORAWAN_VERSION_1_0_4 => Ok(Version::V1_0_4),
        other => {
            set_last_error(format!(
                "{other} is not a LoRaWAN version; expected {PAMOJA_LORAWAN_VERSION_1_0_3} or {PAMOJA_LORAWAN_VERSION_1_0_4}"
            ));
            Err(PamojaStatus::InvalidArgument)
        }
    }
}

/// Starts a back-off count from zero.
///
/// # Arguments
///
/// * `version` - [`PAMOJA_LORAWAN_VERSION_1_0_3`] or [`PAMOJA_LORAWAN_VERSION_1_0_4`], the
///   revision whose steps to follow.
/// * `limit` - how many unanswered uplinks before the device starts asking, usually
///   [`PAMOJA_LORAWAN_ADR_ACK_LIMIT`].
/// * `delay` - how many more before its first step, and between each step after that,
///   usually [`PAMOJA_LORAWAN_ADR_ACK_DELAY`]. Zero is taken as one.
/// * `out_backoff` - receives the count.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success, with `*out_backoff` set to a handle the caller must
/// release with [`pamoja_lorawan_backoff_free`].
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `out_backoff` is null or `version` names no
/// revision.
///
/// # Safety
///
/// `out_backoff` must point to a writable `*mut PamojaLorawanBackoff`.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_backoff_new(
    version: u8,
    limit: u32,
    delay: u32,
    out_backoff: *mut *mut PamojaLorawanBackoff,
) -> PamojaStatus {
    if out_backoff.is_null() {
        set_last_error("out_backoff must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let slot = &mut *out_backoff;
    *slot = ptr::null_mut();
    let version = match self::version(version) {
        Ok(version) => version,
        Err(status) => return status,
    };
    *slot = Box::into_raw(Box::new(PamojaLorawanBackoff {
        backoff: Backoff::new(version, limit, delay),
    }));
    PamojaStatus::Ok
}

/// Counts one new uplink, and says what to do before sending it.
///
/// Call it once per uplink the frame counter moves for. A repeat of the same uplink does
/// not count.
///
/// # Arguments
///
/// * `backoff` - the count.
/// * `default_data_rate` - `1` if the device is already at its default data rate, the
///   slowest it uses, so there is no lower rate to step to.
/// * `out_step` - receives what to do.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if either pointer is null.
///
/// # Safety
///
/// `backoff` must be a live handle from [`pamoja_lorawan_backoff_new`], and `out_step` must
/// point to a writable [`PamojaLorawanBackoffStep`].
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_backoff_uplink(
    backoff: *mut PamojaLorawanBackoff,
    default_data_rate: u8,
    out_step: *mut PamojaLorawanBackoffStep,
) -> PamojaStatus {
    let (Some(backoff), false) = (backoff.as_mut(), out_step.is_null()) else {
        set_last_error("backoff and out_step must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    let step = backoff.backoff.uplink(Standing {
        default_data_rate: default_data_rate != 0,
    });
    *out_step = PamojaLorawanBackoffStep {
        request_ack: u8::from(step.request_ack),
        restore_power: u8::from(step.restore_power),
        lower_data_rate: u8::from(step.lower_data_rate),
        restore_channels: u8::from(step.restore_channels),
    };
    PamojaStatus::Ok
}

/// Counts a Class A downlink, which proves the network still hears the device and resets
/// the count.
///
/// # Arguments
///
/// * `backoff` - the count.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if `backoff` is null.
///
/// # Safety
///
/// `backoff` must be a live handle from [`pamoja_lorawan_backoff_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_backoff_downlink(
    backoff: *mut PamojaLorawanBackoff,
) -> PamojaStatus {
    let Some(backoff) = backoff.as_mut() else {
        set_last_error("backoff must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    };
    backoff.backoff.downlink();
    PamojaStatus::Ok
}

/// Returns how many uplinks have gone unanswered.
///
/// # Returns
///
/// The counter, or 0 if `backoff` is null.
///
/// # Safety
///
/// `backoff` must be a live handle from [`pamoja_lorawan_backoff_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_backoff_counter(
    backoff: *const PamojaLorawanBackoff,
) -> u32 {
    backoff
        .as_ref()
        .map_or(0, |backoff| backoff.backoff.counter())
}

/// Returns the revision whose steps a count follows.
///
/// # Returns
///
/// [`PAMOJA_LORAWAN_VERSION_1_0_3`] or [`PAMOJA_LORAWAN_VERSION_1_0_4`], or 0 if `backoff`
/// is null.
///
/// # Safety
///
/// `backoff` must be a live handle from [`pamoja_lorawan_backoff_new`], or null.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_backoff_version(
    backoff: *const PamojaLorawanBackoff,
) -> u8 {
    backoff
        .as_ref()
        .map_or(0, |backoff| match backoff.backoff.version() {
            Version::V1_0_3 => PAMOJA_LORAWAN_VERSION_1_0_3,
            Version::V1_0_4 => PAMOJA_LORAWAN_VERSION_1_0_4,
        })
}

/// Releases a back-off handle.
///
/// Passing null is a no-op.
///
/// # Safety
///
/// `backoff` must be a handle from [`pamoja_lorawan_backoff_new`] that has not already been
/// freed, or null. After this call it must not be used again.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_backoff_free(backoff: *mut PamojaLorawanBackoff) {
    if !backoff.is_null() {
        drop(Box::from_raw(backoff));
    }
}

/// Reads a channel list that crossed the boundary.
///
/// # Safety
///
/// `cflist` must point to at least `cflist_len` readable bytes when that length is
/// non-zero.
unsafe fn cflist(cflist: *const u8, cflist_len: usize) -> Result<CfList, PamojaStatus> {
    let bytes = read_bytes(cflist, cflist_len)?;
    let Ok(bytes) = <[u8; CFLIST_LEN]>::try_from(&bytes[..]) else {
        set_last_error(format!(
            "a channel list is exactly {CFLIST_LEN} bytes, not {cflist_len}"
        ));
        return Err(PamojaStatus::InvalidArgument);
    };
    Ok(CfList::from_bytes(bytes))
}

/// Writes a channel list's bytes back out.
///
/// # Safety
///
/// `out_cflist` must point to at least [`PAMOJA_LORAWAN_CFLIST_LEN`] writable bytes.
unsafe fn write_cflist(list: CfList, out_cflist: *mut u8) {
    let bytes = list.to_bytes();
    ptr::copy_nonoverlapping(bytes.as_ptr(), out_cflist, bytes.len());
}

/// Builds a type 0 channel list from frequencies.
///
/// # Arguments
///
/// * `frequencies_hz` - the frequencies in hertz, with `0` for a slot left unused.
/// * `len` - how many `frequencies_hz` points at, which must be
///   [`PAMOJA_LORAWAN_CFLIST_FREQUENCIES`].
/// * `out_cflist` - receives the sixteen bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null or `len` is not five, and
/// [`PamojaStatus::Codec`] if a frequency is not a whole number of hundreds of hertz, lies
/// below the 100 MHz RP002-1.0.5 reserves, or does not fit three bytes.
///
/// # Safety
///
/// `frequencies_hz` must point to `len` readable values and `out_cflist` to at least
/// [`PAMOJA_LORAWAN_CFLIST_LEN`] writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_cflist_from_frequencies(
    frequencies_hz: *const u32,
    len: usize,
    out_cflist: *mut u8,
) -> PamojaStatus {
    if frequencies_hz.is_null() || out_cflist.is_null() || len != CFLIST_FREQUENCIES {
        set_last_error(format!(
            "a channel list takes {CFLIST_FREQUENCIES} frequencies and room for {CFLIST_LEN} bytes"
        ));
        return PamojaStatus::InvalidArgument;
    }
    let mut slots = [0u32; CFLIST_FREQUENCIES];
    slots.copy_from_slice(std::slice::from_raw_parts(frequencies_hz, len));
    match CfList::frequencies(slots) {
        Ok(list) => {
            write_cflist(list, out_cflist);
            PamojaStatus::Ok
        }
        Err(error) => {
            set_last_error(format!(
                "{error}: a channel list frequency is a whole number of hundreds of hertz from 100 MHz to just under 1.678 GHz"
            ));
            PamojaStatus::Codec
        }
    }
}

/// Builds a type 1 channel list from channel mask groups.
///
/// # Arguments
///
/// * `masks` - the groups, where bit *n* of group *g* enables channel `g * 16 + n`.
/// * `len` - how many `masks` points at, which must be
///   [`PAMOJA_LORAWAN_CFLIST_MASK_GROUPS`].
/// * `out_cflist` - receives the sixteen bytes.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null or `len` is not six.
///
/// # Safety
///
/// `masks` must point to `len` readable values and `out_cflist` to at least
/// [`PAMOJA_LORAWAN_CFLIST_LEN`] writable bytes.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_cflist_from_channel_masks(
    masks: *const u16,
    len: usize,
    out_cflist: *mut u8,
) -> PamojaStatus {
    if masks.is_null() || out_cflist.is_null() || len != CFLIST_MASK_GROUPS {
        set_last_error(format!(
            "a channel list takes {CFLIST_MASK_GROUPS} mask groups and room for {CFLIST_LEN} bytes"
        ));
        return PamojaStatus::InvalidArgument;
    }
    let mut groups = [0u16; CFLIST_MASK_GROUPS];
    groups.copy_from_slice(std::slice::from_raw_parts(masks, len));
    write_cflist(CfList::channel_masks(groups), out_cflist);
    PamojaStatus::Ok
}

/// Reads which form a channel list takes.
///
/// # Arguments
///
/// * `cflist` - the sixteen bytes.
/// * `cflist_len` - their length, which must be [`PAMOJA_LORAWAN_CFLIST_LEN`].
/// * `out_type` - receives the CFListType byte: [`PAMOJA_LORAWAN_CFLIST_TYPE_FREQUENCIES`],
///   [`PAMOJA_LORAWAN_CFLIST_TYPE_CHANNEL_MASKS`], or a type the regional parameters
///   reserve, which a device ignores.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null or the list is not
/// sixteen bytes.
///
/// # Safety
///
/// `cflist` must point to `cflist_len` readable bytes and `out_type` to a writable byte.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_cflist_type(
    cflist: *const u8,
    cflist_len: usize,
    out_type: *mut u8,
) -> PamojaStatus {
    if out_type.is_null() {
        set_last_error("out_type must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    match self::cflist(cflist, cflist_len) {
        Ok(list) => {
            *out_type = list.kind().to_byte();
            PamojaStatus::Ok
        }
        Err(status) => status,
    }
}

/// Reads the frequencies out of a type 0 channel list.
///
/// # Arguments
///
/// * `cflist` - the sixteen bytes.
/// * `cflist_len` - their length, which must be [`PAMOJA_LORAWAN_CFLIST_LEN`].
/// * `out_frequencies_hz` - receives the frequencies in hertz, `0` for an unused slot.
/// * `len` - room at `out_frequencies_hz`, which must be
///   [`PAMOJA_LORAWAN_CFLIST_FREQUENCIES`].
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, a length is wrong, or
/// the list is not type 0.
///
/// # Safety
///
/// `cflist` must point to `cflist_len` readable bytes and `out_frequencies_hz` to `len`
/// writable values.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_cflist_frequencies(
    cflist: *const u8,
    cflist_len: usize,
    out_frequencies_hz: *mut u32,
    len: usize,
) -> PamojaStatus {
    if out_frequencies_hz.is_null() || len != CFLIST_FREQUENCIES {
        set_last_error(format!(
            "out_frequencies_hz must have room for {CFLIST_FREQUENCIES} frequencies"
        ));
        return PamojaStatus::InvalidArgument;
    }
    let list = match self::cflist(cflist, cflist_len) {
        Ok(list) => list,
        Err(status) => return status,
    };
    let Some(frequencies) = list.frequencies_hz() else {
        set_last_error(format!(
            "the channel list is type {}, not a list of frequencies",
            list.kind().to_byte()
        ));
        return PamojaStatus::InvalidArgument;
    };
    ptr::copy_nonoverlapping(frequencies.as_ptr(), out_frequencies_hz, len);
    PamojaStatus::Ok
}

/// Reads the mask groups out of a type 1 channel list.
///
/// # Arguments
///
/// * `cflist` - the sixteen bytes.
/// * `cflist_len` - their length, which must be [`PAMOJA_LORAWAN_CFLIST_LEN`].
/// * `out_masks` - receives the groups.
/// * `len` - room at `out_masks`, which must be [`PAMOJA_LORAWAN_CFLIST_MASK_GROUPS`].
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, a length is wrong, or
/// the list is not type 1.
///
/// # Safety
///
/// `cflist` must point to `cflist_len` readable bytes and `out_masks` to `len` writable
/// values.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_cflist_channel_masks(
    cflist: *const u8,
    cflist_len: usize,
    out_masks: *mut u16,
    len: usize,
) -> PamojaStatus {
    if out_masks.is_null() || len != CFLIST_MASK_GROUPS {
        set_last_error(format!(
            "out_masks must have room for {CFLIST_MASK_GROUPS} mask groups"
        ));
        return PamojaStatus::InvalidArgument;
    }
    let list = match self::cflist(cflist, cflist_len) {
        Ok(list) => list,
        Err(status) => return status,
    };
    let Some(groups) = list.channel_mask_groups() else {
        set_last_error(format!(
            "the channel list is type {}, not a list of channel masks",
            list.kind().to_byte()
        ));
        return PamojaStatus::InvalidArgument;
    };
    ptr::copy_nonoverlapping(groups.as_ptr(), out_masks, len);
    PamojaStatus::Ok
}

/// Reports whether a type 1 channel list enables a channel.
///
/// # Arguments
///
/// * `cflist` - the sixteen bytes.
/// * `cflist_len` - their length, which must be [`PAMOJA_LORAWAN_CFLIST_LEN`].
/// * `channel` - the channel number, `group * 16 + bit`.
/// * `out_enabled` - receives `1` if its bit is set and `0` if not.
///
/// # Returns
///
/// [`PamojaStatus::Ok`] on success.
///
/// # Errors
///
/// Returns [`PamojaStatus::InvalidArgument`] if a pointer is null, the list is not sixteen
/// bytes or not type 1, or the channel is past the 96 the groups cover.
///
/// # Safety
///
/// `cflist` must point to `cflist_len` readable bytes and `out_enabled` to a writable byte.
#[no_mangle]
pub unsafe extern "C" fn pamoja_lorawan_cflist_enables(
    cflist: *const u8,
    cflist_len: usize,
    channel: u8,
    out_enabled: *mut u8,
) -> PamojaStatus {
    if out_enabled.is_null() {
        set_last_error("out_enabled must not be null".to_owned());
        return PamojaStatus::InvalidArgument;
    }
    let list = match self::cflist(cflist, cflist_len) {
        Ok(list) => list,
        Err(status) => return status,
    };
    match list.enables(channel) {
        Some(enabled) => {
            *out_enabled = u8::from(enabled);
            PamojaStatus::Ok
        }
        None if matches!(list.kind(), CfListKind::ChannelMasks) => {
            set_last_error(format!(
                "channel {channel} is past the {} a channel list covers",
                CFLIST_MASK_GROUPS * 16
            ));
            PamojaStatus::InvalidArgument
        }
        None => {
            set_last_error(format!(
                "the channel list is type {}, not a list of channel masks",
                list.kind().to_byte()
            ));
            PamojaStatus::InvalidArgument
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use pamoja_lorawan::defaults;

    #[test]
    fn the_constants_are_the_crate_values() {
        assert_eq!(
            PAMOJA_LORAWAN_RECEIVE_DELAY1_US,
            defaults::RECEIVE_DELAY1_US
        );
        assert_eq!(
            PAMOJA_LORAWAN_RECEIVE_DELAY2_US,
            defaults::RECEIVE_DELAY2_US
        );
        assert_eq!(
            PAMOJA_LORAWAN_JOIN_ACCEPT_DELAY1_US,
            defaults::JOIN_ACCEPT_DELAY1_US
        );
        assert_eq!(
            PAMOJA_LORAWAN_JOIN_ACCEPT_DELAY2_US,
            defaults::JOIN_ACCEPT_DELAY2_US
        );
        assert_eq!(
            PAMOJA_LORAWAN_RECEIVE_WINDOW_TOLERANCE_US,
            defaults::RECEIVE_WINDOW_TOLERANCE_US
        );
        assert_eq!(PAMOJA_LORAWAN_MAX_FCNT_GAP, defaults::MAX_FCNT_GAP);
        assert_eq!(PAMOJA_LORAWAN_ADR_ACK_LIMIT, defaults::ADR_ACK_LIMIT);
        assert_eq!(PAMOJA_LORAWAN_ADR_ACK_DELAY, defaults::ADR_ACK_DELAY);
        assert_eq!(
            PAMOJA_LORAWAN_RETRANSMIT_TIMEOUT_MIN_US,
            defaults::RETRANSMIT_TIMEOUT_MIN_US
        );
        assert_eq!(
            PAMOJA_LORAWAN_RETRANSMIT_TIMEOUT_MAX_US,
            defaults::RETRANSMIT_TIMEOUT_MAX_US
        );
        assert_eq!(PAMOJA_LORAWAN_CFLIST_LEN, CFLIST_LEN);
        assert_eq!(PAMOJA_LORAWAN_CFLIST_FREQUENCIES, CFLIST_FREQUENCIES);
        assert_eq!(PAMOJA_LORAWAN_CFLIST_MASK_GROUPS, CFLIST_MASK_GROUPS);
    }

    #[test]
    fn a_back_off_follows_table_9_across_the_boundary() {
        unsafe {
            let mut backoff = ptr::null_mut();
            assert_eq!(
                pamoja_lorawan_backoff_new(PAMOJA_LORAWAN_VERSION_1_0_4, 64, 32, &mut backoff),
                PamojaStatus::Ok
            );
            assert_eq!(
                pamoja_lorawan_backoff_version(backoff),
                PAMOJA_LORAWAN_VERSION_1_0_4
            );

            // TS001-1.0.4 table 9 from DR2: power at 96, a rate at 128 and 160, channels
            // at 192 once the default rate is reached.
            let mut data_rate = 2;
            let mut steps = Vec::new();
            let mut step = PamojaLorawanBackoffStep {
                request_ack: 0,
                restore_power: 0,
                lower_data_rate: 0,
                restore_channels: 0,
            };
            for sent in 1..=200 {
                assert_eq!(
                    pamoja_lorawan_backoff_uplink(backoff, u8::from(data_rate == 0), &mut step),
                    PamojaStatus::Ok
                );
                if sent == 64 {
                    assert_eq!(step.request_ack, 1, "the sixty-fourth asks");
                }
                if step.restore_power == 1 {
                    steps.push((sent, "power"));
                }
                if step.lower_data_rate == 1 {
                    data_rate -= 1;
                    steps.push((sent, "rate"));
                }
                if step.restore_channels == 1 {
                    steps.push((sent, "channels"));
                }
            }
            assert_eq!(
                steps,
                [
                    (96, "power"),
                    (128, "rate"),
                    (160, "rate"),
                    (192, "channels")
                ]
            );
            assert_eq!(pamoja_lorawan_backoff_counter(backoff), 200);

            assert_eq!(pamoja_lorawan_backoff_downlink(backoff), PamojaStatus::Ok);
            assert_eq!(pamoja_lorawan_backoff_counter(backoff), 0);
            pamoja_lorawan_backoff_free(backoff);

            assert_eq!(
                pamoja_lorawan_backoff_new(2, 64, 32, &mut backoff),
                PamojaStatus::InvalidArgument
            );
            assert!(backoff.is_null());
            assert_eq!(
                pamoja_lorawan_backoff_uplink(ptr::null_mut(), 0, &mut step),
                PamojaStatus::InvalidArgument
            );
        }
    }

    #[test]
    fn a_channel_list_crosses_in_both_forms() {
        unsafe {
            // The published EU868 join accept of lora-packet issue 10 carries these five.
            let frequencies = [
                867_100_000,
                867_300_000,
                867_500_000,
                867_700_000,
                867_900_000,
            ];
            let mut bytes = [0xAAu8; 16];
            assert_eq!(
                pamoja_lorawan_cflist_from_frequencies(frequencies.as_ptr(), 5, bytes.as_mut_ptr()),
                PamojaStatus::Ok
            );
            assert_eq!(
                bytes,
                [
                    0x18, 0x4F, 0x84, 0xE8, 0x56, 0x84, 0xB8, 0x5E, 0x84, 0x88, 0x66, 0x84, 0x58,
                    0x6E, 0x84, 0x00
                ]
            );
            let mut kind = 9;
            assert_eq!(
                pamoja_lorawan_cflist_type(bytes.as_ptr(), 16, &mut kind),
                PamojaStatus::Ok
            );
            assert_eq!(kind, PAMOJA_LORAWAN_CFLIST_TYPE_FREQUENCIES);
            let mut read = [0u32; 5];
            assert_eq!(
                pamoja_lorawan_cflist_frequencies(bytes.as_ptr(), 16, read.as_mut_ptr(), 5),
                PamojaStatus::Ok
            );
            assert_eq!(read, frequencies);
            let mut masks = [0u16; 6];
            assert_eq!(
                pamoja_lorawan_cflist_channel_masks(bytes.as_ptr(), 16, masks.as_mut_ptr(), 6),
                PamojaStatus::InvalidArgument
            );
            let mut enabled = 9;
            assert_eq!(
                pamoja_lorawan_cflist_enables(bytes.as_ptr(), 16, 0, &mut enabled),
                PamojaStatus::InvalidArgument
            );

            for refused in [867_100_050, 99_999_900, 1_677_721_600] {
                let one = [refused, 0, 0, 0, 0];
                assert_eq!(
                    pamoja_lorawan_cflist_from_frequencies(one.as_ptr(), 5, bytes.as_mut_ptr()),
                    PamojaStatus::Codec,
                    "{refused} Hz"
                );
            }

            // A US915 network on its second sub-band, channels 8 to 15 and 65.
            let groups = [0xFF00, 0, 0, 0, 0x0002, 0];
            assert_eq!(
                pamoja_lorawan_cflist_from_channel_masks(groups.as_ptr(), 6, bytes.as_mut_ptr()),
                PamojaStatus::Ok
            );
            assert_eq!(
                bytes,
                [0x00, 0xFF, 0, 0, 0, 0, 0, 0, 0x02, 0x00, 0, 0, 0, 0, 0, 0x01]
            );
            assert_eq!(
                pamoja_lorawan_cflist_channel_masks(bytes.as_ptr(), 16, masks.as_mut_ptr(), 6),
                PamojaStatus::Ok
            );
            assert_eq!(masks, groups);
            for (channel, want) in [(7, 0), (8, 1), (15, 1), (64, 0), (65, 1)] {
                assert_eq!(
                    pamoja_lorawan_cflist_enables(bytes.as_ptr(), 16, channel, &mut enabled),
                    PamojaStatus::Ok
                );
                assert_eq!(enabled, want, "channel {channel}");
            }
            assert_eq!(
                pamoja_lorawan_cflist_enables(bytes.as_ptr(), 16, 96, &mut enabled),
                PamojaStatus::InvalidArgument
            );
            assert_eq!(
                pamoja_lorawan_cflist_type(bytes.as_ptr(), 15, &mut kind),
                PamojaStatus::InvalidArgument
            );
        }
    }
}
