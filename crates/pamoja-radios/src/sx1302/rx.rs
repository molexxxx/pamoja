//! Reading packets out of the concentrator.
//!
//! Received packets arrive in one buffer, back to back, each wrapped in metadata the chip
//! writes around it: a sync word and nine bytes saying how long the payload is and how it was
//! demodulated, then the payload, then fourteen more bytes of what the receiver made of it.
//!
//! The metadata is in two halves and they are addressed differently. Everything before the
//! payload is counted from the start of the packet; everything after it is counted from the
//! start plus the payload length, which is why a reader has to know how long the payload is
//! before it can find the signal levels or the timestamp.
//!
//! A packet can also carry fine timestamp metrics after all of that, two bytes each, and the
//! count lives in the tail. A reader that ignores them finds the next packet in the wrong
//! place, so [`packet_len`] takes them into account and the walk in [`packets`] stays aligned.
//!
//! Nothing here reads a register. A buffer is bytes, and this turns those bytes into packets,
//! so a capture from a working gateway can be replayed and checked without hardware.

use super::spi;

/// The first byte of the word that marks the start of a packet.
pub const SYNC_BYTE_0: u8 = 0xa5;

/// The second byte of that word.
pub const SYNC_BYTE_1: u8 = 0xc0;

/// How many bytes of metadata come before the payload, the sync word included.
pub const HEAD_METADATA: usize = 9;

/// How many bytes of metadata come after it, before any timestamp metrics.
pub const TAIL_METADATA: usize = 14;

/// How many bytes each fine timestamp metric takes.
pub const METRIC_LEN: usize = 2;

/// How many bytes the receive buffer holds in all.
pub const BUFFER_LEN: usize = 4096;

/// The largest payload a packet carries.
pub const MAX_PAYLOAD: usize = 255;

/// The highest identifier belonging to a multi-spreading-factor LoRa demodulator.
pub const LORA_MULTI_MODEM_MAX: u8 = 15;

/// The identifier of the single-spreading-factor LoRa demodulator.
pub const LORA_STD_MODEM: u8 = 16;

/// The identifier of the frequency shift keying demodulator.
pub const FSK_MODEM: u8 = 17;

/// Which demodulator heard a packet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modem {
    /// One of the demodulators that take any spreading factor, by its number.
    LoraMulti(u8),
    /// The demodulator fixed to one spreading factor and bandwidth.
    LoraStandard,
    /// The frequency shift keying demodulator.
    Fsk,
    /// An identifier this build does not know, carried rather than dropped.
    Unknown(u8),
}

impl Modem {
    /// Reads the demodulator identifier the chip wrote.
    ///
    /// # Arguments
    ///
    /// * `id` - the identifier from the packet metadata.
    ///
    /// # Returns
    ///
    /// Which demodulator it names.
    ///
    /// # Examples
    ///
    /// ```
    /// use pamoja_radios::sx1302::rx::Modem;
    ///
    /// assert_eq!(Modem::of(3), Modem::LoraMulti(3));
    /// assert_eq!(Modem::of(16), Modem::LoraStandard);
    /// assert_eq!(Modem::of(17), Modem::Fsk);
    /// ```
    #[must_use]
    pub const fn of(id: u8) -> Modem {
        match id {
            0..=LORA_MULTI_MODEM_MAX => Modem::LoraMulti(id),
            LORA_STD_MODEM => Modem::LoraStandard,
            FSK_MODEM => Modem::Fsk,
            other => Modem::Unknown(other),
        }
    }

    /// Whether this demodulator carries LoRa rather than frequency shift keying.
    ///
    /// # Returns
    ///
    /// Whether the spreading factor and coding rate in the metadata mean anything.
    #[must_use]
    pub const fn is_lora(&self) -> bool {
        matches!(self, Modem::LoraMulti(_) | Modem::LoraStandard)
    }
}

/// What the receiver made of a packet it heard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Quality {
    /// Whether the payload failed its own check.
    pub crc_error: bool,
    /// Whether the preamble did not line up.
    pub sync_error: bool,
    /// Whether the header did not check out.
    pub header_error: bool,
    /// Whether the timestamp is one the chip stands behind.
    pub timing_set: bool,
}

impl Quality {
    /// Whether the packet arrived intact.
    ///
    /// # Returns
    ///
    /// Whether none of the three errors is set. A gateway forwards a packet that failed its
    /// check only when it is told to, so this is what decides.
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        !self.crc_error && !self.sync_error && !self.header_error
    }
}

/// A packet as the concentrator wrote it into the buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packet<'a> {
    /// The payload, exactly as it arrived.
    pub payload: &'a [u8],
    /// Which of the concentrator channels heard it.
    pub channel: u8,
    /// Which demodulator heard it.
    pub modem: Modem,
    /// The spreading factor, for a LoRa packet.
    pub datarate: u8,
    /// The coding rate, for a LoRa packet.
    pub coding_rate: u8,
    /// Whether the sender asked for a payload check at all.
    pub crc_enabled: bool,
    /// What the receiver made of it.
    pub quality: Quality,
    /// How far off the carrier was, signed, in the units the chip counts.
    pub frequency_offset: i32,
    /// The average signal-to-noise ratio, in dB, which is negative for a weak signal.
    pub snr_average: i8,
    /// The average channel power, as the chip reports it.
    pub rssi_channel: u8,
    /// The average signal power, as the chip reports it.
    pub rssi_signal: u8,
    /// The concentrator counter the packet is stamped against.
    pub timestamp: u32,
    /// The check the sender put over the payload.
    pub payload_crc: u16,
    /// How many fine timestamp metrics follow the packet.
    pub timing_metrics: u8,
    /// Whether the chip's own sum over the packet matches what the bus delivered.
    pub checksum_ok: bool,
}

/// Why a buffer did not read as a packet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RxError {
    /// The bytes at this point do not start with the sync word.
    NotAPacket,
    /// The buffer ends before the packet it announced does.
    Truncated {
        /// How many bytes the packet needs.
        needs: usize,
        /// How many the buffer has left.
        has: usize,
    },
}

impl core::fmt::Display for RxError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RxError::NotAPacket => f.write_str("these bytes do not begin a packet"),
            RxError::Truncated { needs, has } => write!(
                f,
                "the packet needs {needs} bytes and the buffer has {has} left"
            ),
        }
    }
}

/// How many bytes a packet takes in the buffer.
///
/// # Arguments
///
/// * `payload_len` - the length the head metadata announced.
/// * `metrics` - how many fine timestamp metrics the tail announced.
///
/// # Returns
///
/// The whole span, which is where the next packet starts. The metrics are counted because a
/// reader that leaves them out finds the next packet in the wrong place.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::rx::packet_len;
///
/// // Four bytes of payload and no metrics: nine of head and fourteen of tail around it.
/// assert_eq!(packet_len(4, 0), 27);
///
/// // Each metric adds two bytes after the tail.
/// assert_eq!(packet_len(4, 3), 33);
/// ```
#[must_use]
pub const fn packet_len(payload_len: usize, metrics: u8) -> usize {
    HEAD_METADATA + payload_len + TAIL_METADATA + (METRIC_LEN * metrics as usize)
}

/// Reads the one packet that starts at the front of these bytes.
///
/// # Arguments
///
/// * `buffer` - the bytes read out of the receive buffer.
///
/// # Returns
///
/// The packet, and how many bytes of the buffer it took.
///
/// # Errors
///
/// Returns [`RxError::NotAPacket`] when the sync word is missing, and [`RxError::Truncated`]
/// when the buffer ends before the packet does.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::rx::{parse, Modem};
///
/// // Head metadata, payload, then the tail the chip writes after it.
/// let mut buffer = vec![0xa5, 0xc0, 4, 2, 0b0111_0001, 3, 0, 0, 0];
/// buffer.extend_from_slice(b"21.5");
/// buffer.extend_from_slice(&[0u8; 14]);
///
/// let (packet, took) = parse(&buffer).expect("it begins with the sync word");
/// assert_eq!(packet.payload, b"21.5");
/// assert_eq!(packet.channel, 2);
/// assert_eq!(packet.modem, Modem::LoraMulti(3));
/// assert_eq!(packet.datarate, 7);
/// assert!(packet.crc_enabled);
/// assert_eq!(took, buffer.len());
/// ```
pub fn parse(buffer: &[u8]) -> Result<(Packet<'_>, usize), RxError> {
    if buffer.len() < 2 || buffer[0] != SYNC_BYTE_0 || buffer[1] != SYNC_BYTE_1 {
        return Err(RxError::NotAPacket);
    }
    if buffer.len() < HEAD_METADATA {
        return Err(RxError::Truncated {
            needs: HEAD_METADATA,
            has: buffer.len(),
        });
    }

    // Everything after the payload is addressed from here, so the length comes first.
    let payload_len = buffer[2] as usize;
    let tail_at = payload_len;

    // The metric count lives in the tail, and the packet length depends on it.
    let counted = tail_at + 21;
    if buffer.len() <= counted {
        return Err(RxError::Truncated {
            needs: counted + 1,
            has: buffer.len(),
        });
    }
    let metrics = buffer[counted];

    let needs = packet_len(payload_len, metrics);
    if buffer.len() < needs {
        return Err(RxError::Truncated {
            needs,
            has: buffer.len(),
        });
    }

    let head = buffer[4];
    let status = buffer[tail_at + 9];

    // Twenty bits across three bytes, and the top of that range means a negative offset.
    let offset = i32::from(buffer[6])
        | (i32::from(buffer[7]) << 8)
        | (i32::from(spi::field(buffer[8], 0, 4)) << 16);
    let frequency_offset = if offset >= (1 << 19) {
        offset - (1 << 20)
    } else {
        offset
    };

    let timestamp = u32::from(buffer[tail_at + 15])
        | (u32::from(buffer[tail_at + 16]) << 8)
        | (u32::from(buffer[tail_at + 17]) << 16)
        | (u32::from(buffer[tail_at + 18]) << 24);

    let payload_crc = u16::from(buffer[tail_at + 19]) | (u16::from(buffer[tail_at + 20]) << 8);

    // The chip sums every byte of the packet but the last, and writes the sum into it.
    let written = buffer[needs - 1];
    let summed = buffer[..needs - 1]
        .iter()
        .fold(0u8, |sum, byte| sum.wrapping_add(*byte));

    let packet = Packet {
        payload: &buffer[HEAD_METADATA..HEAD_METADATA + payload_len],
        channel: buffer[3],
        modem: Modem::of(buffer[5]),
        datarate: spi::field(head, 4, 4),
        coding_rate: spi::field(head, 1, 3),
        crc_enabled: spi::field(head, 0, 1) == 1,
        quality: Quality {
            crc_error: spi::field(status, 0, 1) == 1,
            sync_error: spi::field(status, 2, 1) == 1,
            header_error: spi::field(status, 3, 1) == 1,
            timing_set: spi::field(status, 4, 1) == 1,
        },
        frequency_offset,
        snr_average: buffer[tail_at + 10] as i8,
        rssi_channel: buffer[tail_at + 11],
        rssi_signal: buffer[tail_at + 12],
        timestamp,
        payload_crc,
        timing_metrics: metrics,
        checksum_ok: written == summed,
    };

    Ok((packet, needs))
}

/// Walks every packet the buffer holds, in the order the chip wrote them.
///
/// # Arguments
///
/// * `buffer` - the bytes read out of the receive buffer.
///
/// # Returns
///
/// Each packet in turn. The walk stops at the first thing that does not read as one, which is
/// how the end of the written part of the buffer announces itself.
///
/// # Examples
///
/// ```
/// use pamoja_radios::sx1302::rx::packets;
///
/// // Two packets back to back, then the unwritten rest of the buffer.
/// let mut buffer = Vec::new();
/// for payload in [&b"one"[..], &b"two"[..]] {
///     buffer.extend_from_slice(&[0xa5, 0xc0, payload.len() as u8, 0, 0, 0, 0, 0, 0]);
///     buffer.extend_from_slice(payload);
///     buffer.extend_from_slice(&[0u8; 14]);
/// }
/// buffer.extend_from_slice(&[0u8; 32]);
///
/// let read: Vec<_> = packets(&buffer).map(|packet| packet.payload.to_vec()).collect();
/// assert_eq!(read, [b"one".to_vec(), b"two".to_vec()]);
/// ```
pub fn packets(buffer: &[u8]) -> impl Iterator<Item = Packet<'_>> {
    let mut at = 0usize;
    core::iter::from_fn(move || match parse(buffer.get(at..)?) {
        Ok((packet, took)) => {
            at += took;
            Some(packet)
        }
        Err(_) => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lays out a packet the way the chip writes one, so a test can read it back.
    fn written(
        payload: &[u8],
        channel: u8,
        modem: u8,
        head: u8,
        status: u8,
        metrics: u8,
    ) -> Vec<u8> {
        let mut buffer = vec![
            SYNC_BYTE_0,
            SYNC_BYTE_1,
            payload.len() as u8,
            channel,
            head,
            modem,
            0,
            0,
            0,
        ];
        buffer.extend_from_slice(payload);
        buffer.extend_from_slice(&[0u8; TAIL_METADATA]);
        buffer.extend_from_slice(&vec![0u8; METRIC_LEN * metrics as usize]);

        // The tail is addressed from the payload length, not from the packet start.
        let tail_at = payload.len();
        buffer[tail_at + 9] = status;
        buffer[tail_at + 21] = metrics;

        // The chip sums every byte but the last, and writes the sum into it.
        let last = buffer.len() - 1;
        let sum = buffer[..last]
            .iter()
            .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
        buffer[last] = sum;
        buffer
    }

    #[test]
    fn a_packet_reads_back_the_way_the_chip_wrote_it() {
        let buffer = written(b"21.5", 2, 3, 0b0111_0001, 0, 0);
        let (packet, took) = parse(&buffer).expect("it begins with the sync word");

        assert_eq!(packet.payload, b"21.5");
        assert_eq!(packet.channel, 2);
        assert_eq!(packet.modem, Modem::LoraMulti(3));
        assert_eq!(packet.datarate, 7);
        assert_eq!(packet.coding_rate, 0);
        assert!(packet.crc_enabled);
        assert!(packet.quality.is_clean());
        assert!(packet.checksum_ok);
        assert_eq!(took, packet_len(4, 0));
    }

    #[test]
    fn the_modem_and_the_frequency_offset_are_different_bytes() {
        // These two sit next to each other, and reading one for the other is silent.
        let mut buffer = written(b"x", 0, 7, 0, 0, 0);
        buffer[6] = 0x34;
        buffer[7] = 0x12;
        buffer[8] = 0x00;
        let last = buffer.len() - 1;
        buffer[last] = buffer[..last]
            .iter()
            .fold(0u8, |sum, byte| sum.wrapping_add(*byte));

        let (packet, _) = parse(&buffer).expect("a packet");
        assert_eq!(packet.modem, Modem::LoraMulti(7), "the modem is byte five");
        assert_eq!(
            packet.frequency_offset, 0x1234,
            "the offset starts at byte six"
        );
    }

    #[test]
    fn a_carrier_below_the_channel_reads_as_a_negative_offset() {
        // The offset is twenty bits and signed, so the top of the range wraps.
        let mut buffer = written(b"x", 0, 0, 0, 0, 0);
        buffer[6] = 0xff;
        buffer[7] = 0xff;
        buffer[8] = 0x0f;
        let last = buffer.len() - 1;
        buffer[last] = buffer[..last]
            .iter()
            .fold(0u8, |sum, byte| sum.wrapping_add(*byte));

        let (packet, _) = parse(&buffer).expect("a packet");
        assert_eq!(packet.frequency_offset, -1);
    }

    #[test]
    fn the_tail_is_addressed_from_the_payload_length() {
        // A longer payload moves every tail field, which is what makes this easy to get wrong.
        for payload in [&b"x"[..], &b"a much longer payload than the first one"[..]] {
            let mut buffer = written(payload, 0, 0, 0, 0, 0);
            let tail_at = payload.len();
            buffer[tail_at + 10] = 0xf6; // ten below zero, as a signed byte
            buffer[tail_at + 11] = 200;
            buffer[tail_at + 12] = 180;
            buffer[tail_at + 15] = 0x78;
            buffer[tail_at + 16] = 0x56;
            buffer[tail_at + 17] = 0x34;
            buffer[tail_at + 18] = 0x12;
            let last = buffer.len() - 1;
            buffer[last] = buffer[..last]
                .iter()
                .fold(0u8, |sum, byte| sum.wrapping_add(*byte));

            let (packet, _) = parse(&buffer).expect("a packet");
            assert_eq!(packet.snr_average, -10);
            assert_eq!(packet.rssi_channel, 200);
            assert_eq!(packet.rssi_signal, 180);
            assert_eq!(packet.timestamp, 0x1234_5678);
        }
    }

    #[test]
    fn timestamp_metrics_move_where_the_next_packet_starts() {
        // A packet carrying metrics is longer than its head and tail suggest. A reader that
        // misses them lands in the middle of the next packet and finds nothing.
        let mut buffer = written(b"one", 0, 0, 0, 0, 4);
        let first_len = buffer.len();
        assert_eq!(first_len, packet_len(3, 4));

        buffer.extend_from_slice(&written(b"two", 0, 0, 0, 0, 0));

        let read: Vec<_> = packets(&buffer)
            .map(|packet| packet.payload.to_vec())
            .collect();
        assert_eq!(read, [b"one".to_vec(), b"two".to_vec()]);

        let (first, took) = parse(&buffer).expect("a packet");
        assert_eq!(first.timing_metrics, 4);
        assert_eq!(took, first_len);
    }

    #[test]
    fn what_does_not_begin_with_the_sync_word_is_not_a_packet() {
        assert_eq!(parse(&[]), Err(RxError::NotAPacket));
        assert_eq!(parse(&[0u8; 64]), Err(RxError::NotAPacket));
        assert_eq!(parse(&[0xa5, 0x00, 0, 0]), Err(RxError::NotAPacket));
    }

    #[test]
    fn a_packet_cut_short_says_how_much_it_needed() {
        let buffer = written(b"a longer payload than the buffer holds", 0, 0, 0, 0, 0);
        let cut = &buffer[..20];
        let Err(RxError::Truncated { has, .. }) = parse(cut) else {
            panic!("a cut packet is truncated");
        };
        assert_eq!(has, 20);

        // The head alone does not say how long the packet is.
        assert_eq!(
            parse(&[SYNC_BYTE_0, SYNC_BYTE_1, 4]),
            Err(RxError::Truncated {
                needs: HEAD_METADATA,
                has: 3
            })
        );
    }

    #[test]
    fn the_error_flags_are_read_apart() {
        // Each flag has a bit of its own, so one failure is never read as another.
        let buffer = written(b"x", 0, 0, 0, 0b0000_0001, 0);
        let crc = parse(&buffer).expect("a packet").0.quality;
        assert!(crc.crc_error);
        assert!(!crc.sync_error);
        assert!(!crc.is_clean());

        let buffer = written(b"x", 0, 0, 0, 0b0000_0100, 0);
        let sync = parse(&buffer).expect("a packet").0.quality;
        assert!(sync.sync_error);
        assert!(!sync.crc_error);

        let buffer = written(b"x", 0, 0, 0, 0b0000_1000, 0);
        let header = parse(&buffer).expect("a packet").0.quality;
        assert!(header.header_error);

        let buffer = written(b"x", 0, 0, 0, 0b0001_0000, 0);
        let timed = parse(&buffer).expect("a packet").0.quality;
        assert!(timed.timing_set);
        assert!(timed.is_clean(), "a timestamp is not a failure");
    }

    #[test]
    fn a_packet_the_bus_corrupted_is_told_apart_from_one_the_air_did() {
        let good = written(b"21.5", 1, 0, 0b0111_0001, 0, 0);
        let (packet, _) = parse(&good).expect("a packet");
        assert!(packet.checksum_ok, "the chip and the bus agree");
        assert!(packet.quality.is_clean(), "and the air was clean");

        // One byte flipped on the way out of the chip: the air was fine, the bus was not.
        let mut corrupted = good.clone();
        corrupted[10] ^= 0x01;
        let (packet, _) = parse(&corrupted).expect("a packet");
        assert!(!packet.checksum_ok);
        assert!(
            packet.quality.is_clean(),
            "the receiver still reports a clean reception"
        );
    }

    #[test]
    fn the_demodulators_are_told_apart() {
        for id in 0..=LORA_MULTI_MODEM_MAX {
            assert_eq!(Modem::of(id), Modem::LoraMulti(id));
            assert!(Modem::of(id).is_lora());
        }
        assert_eq!(Modem::of(LORA_STD_MODEM), Modem::LoraStandard);
        assert!(Modem::of(LORA_STD_MODEM).is_lora());

        assert_eq!(Modem::of(FSK_MODEM), Modem::Fsk);
        assert!(!Modem::of(FSK_MODEM).is_lora());

        assert_eq!(Modem::of(200), Modem::Unknown(200));
        assert!(!Modem::of(200).is_lora());
    }

    #[test]
    fn the_buffer_is_walked_until_it_stops_being_packets() {
        let mut buffer = Vec::new();
        for payload in [&b"one"[..], &b"two"[..], &b"three"[..]] {
            buffer.extend_from_slice(&written(payload, 0, 0, 0, 0, 0));
        }
        buffer.extend_from_slice(&[0u8; 64]);

        let read: Vec<_> = packets(&buffer)
            .map(|packet| packet.payload.to_vec())
            .collect();
        assert_eq!(read, [b"one".to_vec(), b"two".to_vec(), b"three".to_vec()]);

        assert_eq!(packets(&[]).count(), 0);
    }

    #[test]
    fn a_packet_takes_its_payload_plus_its_metadata() {
        assert_eq!(packet_len(0, 0), HEAD_METADATA + TAIL_METADATA);
        assert_eq!(packet_len(255, 0), HEAD_METADATA + 255 + TAIL_METADATA);
        assert_eq!(packet_len(4, 3), HEAD_METADATA + 4 + TAIL_METADATA + 6);
        assert!(packet_len(MAX_PAYLOAD, 0) < BUFFER_LEN);
    }
}
