//! The serial framing guide example: a weather mast whose node sends COBS frames up a UART to
//! a gateway; see docs/guides/serial.md.
//!
//! Run: `cargo run -p pamoja-examples --example serial`

use std::error::Error;

/// A node and a gateway on the two ends of one serial line, with nothing plugged in.
fn main() -> std::result::Result<(), Box<dyn Error>> {
    // ANCHOR: example
    use std::time::Duration;

    use pamoja_hal::port::{SerialPort, Settings};
    use pamoja_serial::cobs::{self, CobsDecoder};
    use pamoja_serial::slip;

    // A reading is a two-byte sequence number, most significant byte first, then its text.
    fn reading(sequence: u16, text: &str) -> Vec<u8> {
        let mut payload = sequence.to_be_bytes().to_vec();
        payload.extend_from_slice(text.as_bytes());
        payload
    }

    // The line: 115200 baud, eight data bits, no parity, one stop bit. Ten bits a character.
    let settings = Settings::new(115_200);
    println!(
        "line         {settings}, {} bits a character, {:.2} us each",
        settings.bits_per_character(),
        settings.character_nanos() as f64 / 1_000.0
    );

    // The two ends of the cable with nothing plugged in. On a Raspberry Pi the gateway's end
    // is SerialPort::open("/dev/serial0", settings) and nothing after this statement changes.
    let (gateway, node) = SerialPort::pair(settings);

    // A UART carries bytes, and nothing marks where a message ends, so the node frames each
    // reading with COBS: zero becomes the one byte that ends a frame and never appears inside
    // one, which matters here, since the sequence number is full of zeros.
    let texts = ["wind=12.4", "wind=13.1", "wind=11.8"];
    let mut sent = 0;
    let mut frame = [0u8; cobs::max_encoded_len(32)];
    for (sequence, text) in (1..).zip(texts) {
        let framed = cobs::encode(&reading(sequence, text), &mut frame)?;
        node.write(&frame[..framed])?;
        sent += framed;
    }
    println!(
        "node         {} readings of {} bytes, framed as {sent} bytes",
        texts.len(),
        reading(1, texts[0]).len()
    );

    // A read returns whatever has arrived, which is rarely one frame: here it is all three.
    // The decoder splits the stream back into payloads at each delimiter.
    let mut buffer = [0u8; 256];
    let got = gateway.read(&mut buffer, Duration::from_millis(100))?;
    println!("gateway      {got} bytes in one read");
    let mut decoder: CobsDecoder<64> = CobsDecoder::new();
    let mut payloads = Vec::new();
    for &byte in &buffer[..got] {
        if let Ok(Some(payload)) = decoder.push(byte) {
            payloads.push(payload.to_vec());
        }
    }
    for payload in &payloads {
        let (sequence, text) = payload.split_at(2);
        let sequence = u16::from_be_bytes([sequence[0], sequence[1]]);
        println!("reading {sequence}    {}", String::from_utf8_lossy(text));
    }

    // What one frame costs on the wire at this speed, start and stop bits included.
    let frame_len = sent / texts.len();
    println!(
        "on the wire  {:.2} ms for a {frame_len}-byte frame at {settings}",
        settings.transfer_micros(frame_len) as f64 / 1_000.0
    );

    // The node restarts partway through a frame. As it comes back up it sends a lone
    // delimiter, which closes off the half frame, so the gateway drops it rather than gluing
    // it to the next one, and then it sends the reading again.
    let framed = cobs::encode(&reading(4, "wind=12.9"), &mut frame)?;
    node.write(&frame[..framed / 2])?;
    node.write(&[cobs::DELIMITER])?;
    node.write(&frame[..framed])?;
    let got = gateway.read(&mut buffer, Duration::from_millis(100))?;
    let mut dropped = 0;
    let mut resent = Vec::new();
    for &byte in &buffer[..got] {
        match decoder.push(byte) {
            Ok(Some(payload)) => resent.push(payload.to_vec()),
            Ok(None) => {}
            Err(_) => dropped += 1,
        }
    }
    println!(
        "restart      {dropped} frame cut short and dropped, then {}",
        String::from_utf8_lossy(&resent[0][2..])
    );

    // SLIP, the older framing, ends a frame with one reserved byte and escapes that byte and
    // its own escape byte inside one. With no reserved bytes in a reading it costs a byte less
    // than COBS; a payload full of them costs up to twice its length under SLIP, and never
    // more than one byte in 254 over under COBS.
    let first = reading(1, texts[0]);
    let mut slip_frame = [0u8; slip::max_encoded_len(32)];
    let slip_len = slip::encode(&first, &mut slip_frame)?;
    let cobs_len = cobs::encode(&first, &mut frame)?;
    println!(
        "framing      {} payload bytes: {slip_len} under SLIP, {cobs_len} under COBS",
        first.len()
    );

    // The node goes quiet. A read waits for the first byte up to its timeout; on a port with
    // nothing plugged in it returns at once and counts the wait instead of sleeping through
    // it, so a test of a silent node takes no time.
    let before = gateway.waited_micros();
    let got = gateway.read(&mut buffer, Duration::from_millis(500))?;
    println!(
        "silence      {got} bytes in {} ms, counted and not slept",
        (gateway.waited_micros() - before) / 1_000
    );
    // ANCHOR_END: example

    assert_eq!(settings.character_nanos(), 86_806, "10 bits at 115200");
    assert_eq!(
        payloads,
        [
            reading(1, "wind=12.4"),
            reading(2, "wind=13.1"),
            reading(3, "wind=11.8")
        ]
    );
    assert_eq!(
        sent, 39,
        "each 11-byte payload gains one code byte and a delimiter"
    );
    assert_eq!(settings.transfer_micros(13), 1_129);
    assert_eq!(dropped, 1);
    assert_eq!(resent, [reading(4, "wind=12.9")]);
    assert_eq!((slip_len, cobs_len), (12, 13));
    assert_eq!(gateway.waited_micros(), 500_000);
    assert_eq!(node.written(), 39 + framed / 2 + 1 + framed);

    Ok(())
}
