//! A UART self-test: the Pi's own serial port with TX jumpered to RX, sending COBS frames and
//! reading each one straight back.
//!
//! Turn the serial port hardware on and the serial console off (`raspi-config`, Interface
//! Options, Serial Port), reboot, and put one jumper between GPIO14 and GPIO15. Then run
//! `cargo run --release --bin uart`. See docs/guides/serial.md.

use std::error::Error;
use std::thread;
use std::time::{Duration, Instant};

// ANCHOR: example
use pamoja_hal::port::{SerialPort, Settings};
use pamoja_serial::cobs::{self, CobsDecoder};

/// The primary UART, on GPIO14 and GPIO15 on every model but the Raspberry Pi 5.
const PORT: &str = "/dev/serial0";

fn main() -> Result<(), Box<dyn Error>> {
    let port = SerialPort::open(PORT, Settings::new(115_200))?;
    let mut decoder: CobsDecoder<64> = CobsDecoder::new();
    let mut buffer = [0u8; 64];
    let mut frame = [0u8; cobs::max_encoded_len(32)];

    for sequence in 1u16..=5 {
        let mut payload = sequence.to_be_bytes().to_vec();
        payload.extend_from_slice(b"ping");
        let framed = cobs::encode(&payload, &mut frame)?;
        let started = Instant::now();
        port.write(&frame[..framed])?;

        // The frame comes back on RX as it goes out on TX. Read until the decoder has it, or
        // until the line has been quiet for 100 ms.
        let mut echoed = None;
        while echoed.is_none() {
            let got = port.read(&mut buffer, Duration::from_millis(100))?;
            if got == 0 {
                break;
            }
            for &byte in &buffer[..got] {
                if let Ok(Some(back)) = decoder.push(byte) {
                    echoed = Some(back.to_vec());
                }
            }
        }
        match echoed {
            Some(back) if back == payload => println!(
                "frame {sequence}  {framed} bytes back in {:.2} ms",
                started.elapsed().as_secs_f64() * 1e3
            ),
            Some(_) => {
                println!("frame {sequence}  came back changed: check the speed and the wiring")
            }
            None => println!(
                "frame {sequence}  nothing came back: check the jumper, and that the console is off"
            ),
        }
        thread::sleep(Duration::from_millis(500));
    }
    Ok(())
}
// ANCHOR_END: example
