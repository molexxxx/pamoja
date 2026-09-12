//! Turning what a concentrator heard into what a network server is sent.
//!
//! A concentrator reports a packet in its own terms: the channel it arrived on, a spreading
//! factor, a coding rate as a small number, levels in the units the chip counts in, and a
//! timestamp of 27 bits. A network server is told the same packet in the protocol's terms: a
//! carrier in hertz, a datarate identifier, levels in decibels, and a timestamp that has been
//! widened past its rollover.
//!
//! This does that translation and nothing else, so it can be checked against a packet built
//! by hand with no bus and no socket in sight.

use pamoja_lora::budget::Decibels;
use pamoja_lora::LinkSettings;
use pamoja_radios::sx1302::channel::MULTI_BANDWIDTH_HZ;
use pamoja_radios::sx1302::rx::Packet;
use pamoja_radios::sx1302::timestamp::Counter;

use crate::udp::{CrcStatus, Rxpk};

/// The carrier a packet arrived on.
///
/// A concentrator hears on offsets from one carrier, and reports which of its receivers heard
/// a packet rather than the frequency it was on, so the carrier is worked back out from the
/// channel it names.
///
/// # Arguments
///
/// * `carrier_hz` - what the radio is tuned to.
/// * `offsets_hz` - how far each receiver listens from it, in the order they were configured.
/// * `channel` - the receiver that heard it.
///
/// # Returns
///
/// The carrier the packet was on, or `None` for a channel that was never configured.
///
/// # Examples
///
/// ```
/// use pamoja_gateway::daemon::forward::carrier;
///
/// let offsets = [-400_000, -200_000, 0];
/// assert_eq!(carrier(867_500_000, &offsets, 0), Some(867_100_000));
/// assert_eq!(carrier(867_500_000, &offsets, 2), Some(867_500_000));
///
/// // A packet on a receiver nobody configured has no carrier to report.
/// assert_eq!(carrier(867_500_000, &offsets, 7), None);
/// ```
#[must_use]
pub fn carrier(carrier_hz: u32, offsets_hz: &[i32], channel: u8) -> Option<u32> {
    let offset = offsets_hz.get(usize::from(channel))?;
    let on = i64::from(carrier_hz) + i64::from(*offset);
    u32::try_from(on).ok()
}

/// What a packet the concentrator heard is reported as.
///
/// # Arguments
///
/// * `packet` - the packet as the concentrator wrote it.
/// * `carrier_hz` - what the radio is tuned to.
/// * `offsets_hz` - how far each receiver listens from it.
/// * `counter` - the state that widens the packet timestamp past a rollover.
///
/// # Returns
///
/// The uplink entry, or `None` for a packet on a channel that was never configured or for
/// one the frequency shift keying receiver heard, which carries no spreading factor.
///
/// # Examples
///
/// ```
/// use pamoja_gateway::daemon::forward::heard;
/// use pamoja_lora::budget::Decibels;
/// use pamoja_radios::sx1302::rx::{Modem, Packet, Quality};
/// use pamoja_radios::sx1302::timestamp::Counter;
///
/// let packet = Packet {
///     payload: &[1, 2, 3],
///     channel: 0,
///     modem: Modem::LoraMulti(0),
///     datarate: 7,
///     coding_rate: 1,
///     crc_enabled: true,
///     quality: Quality::default(),
///     frequency_offset: 0,
///     snr_average: 20,
///     rssi_channel: 200,
///     rssi_signal: 200,
///     timestamp: 1_000,
///     payload_crc: 0,
///     timing_metrics: 0,
///     checksum_ok: true,
/// };
///
/// let mut counter = Counter::new();
/// let entry = heard(&packet, 867_500_000, &[-400_000], &counter).expect("a LoRa packet");
///
/// // The chip counts the ratio in quarters of a decibel, so twenty is five.
/// assert_eq!(entry.snr_db.map(Decibels::hundredths), Some(500));
/// assert_eq!(entry.frequency_hz, 867_100_000);
/// ```
#[must_use]
pub fn heard(
    packet: &Packet<'_>,
    carrier_hz: u32,
    offsets_hz: &[i32],
    counter: &Counter,
) -> Option<Rxpk> {
    if !packet.modem.is_lora() {
        return None;
    }

    let frequency_hz = carrier(carrier_hz, offsets_hz, packet.channel)?;
    let link = LinkSettings::new(packet.datarate, MULTI_BANDWIDTH_HZ)
        .with_coding_rate(packet.coding_rate.saturating_add(4));

    let mut entry = Rxpk::new(frequency_hz, link, packet.payload.to_vec());
    entry.channel = packet.channel;
    entry.crc = crc_of(packet);
    entry.timestamp_us = Some(counter.widened_packet(packet.timestamp));

    // The chip reports its levels in counts of its own, and the reference converts none of
    // them for LoRa, so they are passed on as they came rather than shifted by a number that
    // would be wrong on every board.
    entry.rssi_dbm = Decibels::from_db(i32::from(packet.rssi_channel));
    entry.snr_db = Some(Decibels::from_hundredths(
        i32::from(packet.snr_average) * 100 / 4,
    ));

    Some(entry)
}

/// What the receiver made of a packet, as the protocol writes it.
fn crc_of(packet: &Packet<'_>) -> CrcStatus {
    if !packet.crc_enabled {
        CrcStatus::Absent
    } else if packet.quality.crc_error {
        CrcStatus::Failed
    } else {
        CrcStatus::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pamoja_radios::sx1302::rx::{Modem, Quality};

    fn heard_packet<'a>(payload: &'a [u8]) -> Packet<'a> {
        Packet {
            payload,
            channel: 1,
            modem: Modem::LoraMulti(1),
            datarate: 9,
            coding_rate: 1,
            crc_enabled: true,
            quality: Quality::default(),
            frequency_offset: 0,
            snr_average: -20,
            rssi_channel: 180,
            rssi_signal: 180,
            timestamp: 5_000,
            payload_crc: 0,
            timing_metrics: 0,
            checksum_ok: true,
        }
    }

    #[test]
    fn the_carrier_comes_back_from_the_channel_that_heard_it() {
        let offsets = [-400_000, -200_000, 0, 200_000];
        assert_eq!(carrier(867_500_000, &offsets, 1), Some(867_300_000));
        assert_eq!(carrier(867_500_000, &offsets, 3), Some(867_700_000));
        assert_eq!(carrier(867_500_000, &offsets, 4), None);
    }

    #[test]
    fn a_coding_rate_is_reported_as_its_denominator() {
        // The chip counts 1 to 4 and the protocol writes 4/5 to 4/8.
        let payload = [1, 2, 3];
        let mut packet = heard_packet(&payload);
        let counter = Counter::new();

        for (counted, denominator) in [(1u8, 5u8), (2, 6), (3, 7), (4, 8)] {
            packet.coding_rate = counted;
            let entry = heard(&packet, 867_500_000, &[0, 0], &counter).expect("a LoRa packet");
            let link = entry.modulation.link().expect("LoRa carries link settings");
            assert_eq!(link.coding_rate_denominator(), denominator);
        }
    }

    #[test]
    fn a_weak_signal_is_reported_as_a_negative_ratio() {
        // The chip counts quarters of a decibel, signed, so minus twenty is minus five.
        let payload = [1];
        let packet = heard_packet(&payload);
        let counter = Counter::new();

        let entry = heard(&packet, 867_500_000, &[0, 0], &counter).expect("a LoRa packet");
        assert_eq!(entry.snr_db.map(Decibels::hundredths), Some(-500));
    }

    #[test]
    fn what_the_receiver_made_of_it_crosses_over() {
        let payload = [1];
        let counter = Counter::new();

        let mut packet = heard_packet(&payload);
        packet.quality.crc_error = true;
        let failed = heard(&packet, 867_500_000, &[0, 0], &counter).expect("a LoRa packet");
        assert_eq!(failed.crc, CrcStatus::Failed);

        let mut without = heard_packet(&payload);
        without.crc_enabled = false;
        let absent = heard(&without, 867_500_000, &[0, 0], &counter).expect("a LoRa packet");
        assert_eq!(absent.crc, CrcStatus::Absent);
    }

    #[test]
    fn a_packet_the_keying_receiver_heard_is_not_forwarded_as_lora() {
        let payload = [1];
        let mut packet = heard_packet(&payload);
        packet.modem = Modem::Fsk;
        let counter = Counter::new();

        assert!(heard(&packet, 867_500_000, &[0, 0], &counter).is_none());
    }

    #[test]
    fn a_packet_on_a_channel_nobody_configured_is_not_forwarded() {
        let payload = [1];
        let mut packet = heard_packet(&payload);
        packet.channel = 6;
        let counter = Counter::new();

        assert!(heard(&packet, 867_500_000, &[0, 0], &counter).is_none());
    }
}
