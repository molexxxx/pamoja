//! An SX1262 on a Raspberry Pi Pico: the Waveshare Pico-LoRa-SX1262 board, beaconing a reading
//! and reporting every frame it hears over UART0.
//!
//! The board is a hat: slot the Pico into its header and the SPI1 pins, the reset and BUSY
//! lines, and the antenna are all wired for you. A USB serial adapter on GP0 (TX) and GP1 (RX)
//! at 115200 baud shows what it prints. Hold BOOTSEL, plug the board in, and run
//! `cargo run --release --bin radio`. See docs/boards/rp2040.md.

#![no_std]
#![no_main]

use core::fmt::Write;

use panic_halt as _;
use rp2040_hal as hal;
use rp2040_hal::fugit::RateExtU32;
use rp2040_hal::Clock;

/// The second-stage bootloader the RP2040 needs at the start of flash.
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

/// The Pico's crystal.
const XTAL_FREQ_HZ: u32 = 12_000_000;

// ANCHOR: example
use embedded_hal::spi::MODE_0;
use embedded_hal_bus::spi::ExclusiveDevice;
use hal::gpio::{FunctionSpi, Pin, PullNone};
use hal::uart::{DataBits, StopBits, UartConfig, UartPeripheral};
use pamoja_lora::LinkSettings;
use pamoja_radios::duty::DutyCycle;
use pamoja_radios::sx126x::config::{PowerAmplifier, TcxoVoltage, TxPower};
use pamoja_radios::sx126x::{Board, RadioConfig, Reception, Sx126x, DEFAULT_TCXO_SETTLE_US};

/// The channel this node uses, what it sends at, and the share of time it may hold the band.
const FREQUENCY_HZ: u32 = 868_100_000;
const OUTPUT_DBM: i8 = 14;
const DUTY_CYCLE_PERMILLE: u32 = 10;

/// How long the radio listens before it looks at its duty-cycle budget again, in microseconds.
const LISTEN_US: u64 = 10_000_000;

#[hal::entry]
fn main() -> ! {
    let mut pac = hal::pac::Peripherals::take().expect("the peripherals are taken once");
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .expect("the clocks come up from the crystal");
    let sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // SPI1 on the pins the board wires to the radio: GP10 the clock, GP11 out, GP12 in. Two
    // megahertz is far under the chip's 16 MHz limit and is what the reference drivers use.
    let sck: Pin<_, FunctionSpi, PullNone> = pins.gpio10.reconfigure();
    let mosi: Pin<_, FunctionSpi, PullNone> = pins.gpio11.reconfigure();
    let miso: Pin<_, FunctionSpi, PullNone> = pins.gpio12.reconfigure();
    let bus = hal::Spi::<_, _, _, 8>::new(pac.SPI1, (mosi, miso, sck)).init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        2.MHz(),
        MODE_0,
    );

    // The chip select is a plain output the bus drives around each transaction, which is what
    // an exclusive device does for a bus with one chip on it.
    let timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let cs = pins.gpio3.into_push_pull_output();
    let device = ExclusiveDevice::new(bus, cs, timer).expect("the chip select takes a level");

    // What the board wires around the chip, from its own example code: a 1.7 V TCXO powered
    // from DIO3, the antenna switch on DIO2, and the DC-DC regulator.
    let board = Board::new(PowerAmplifier::HighPower)
        .with_tcxo(TcxoVoltage::V1_7, DEFAULT_TCXO_SETTLE_US)
        .with_dio2_rf_switch()
        .with_dc_dc();
    let busy = pins.gpio2.into_floating_input();
    let reset = pins.gpio15.into_push_pull_output();
    let mut radio = Sx126x::new(device, busy, reset, timer, board);
    radio.init().expect("the SX1262 answers on SPI1");

    // SF9 at 125 kHz, which is DR3 in the European plan, at 14 dBm.
    let link = LinkSettings::new(9, 125_000);
    let power = TxPower::for_output(PowerAmplifier::HighPower, OUTPUT_DBM);
    radio
        .configure(RadioConfig::new(FREQUENCY_HZ, link, power))
        .expect("the chip takes the settings");

    // UART0 on GP0 and GP1 carries what the node hears and sends.
    let uart_pins = (pins.gpio0.into_function(), pins.gpio1.into_function());
    let mut uart = UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(
            UartConfig::new(115_200.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .expect("a valid UART configuration");
    let _ = writeln!(uart, "beacon on {FREQUENCY_HZ} Hz at SF9, {OUTPUT_DBM} dBm\r");

    // Each frame buys silence in proportion to its airtime, and the guard says when the next
    // one may go out. The chip's own timer counts microseconds, which is what it counts in.
    let mut duty = DutyCycle::new(DUTY_CYCLE_PERMILLE);
    let mut buffer = [0u8; 255];
    let mut reading = 0u32;

    loop {
        match radio.receive(&mut buffer, LISTEN_US) {
            Ok(Reception::Frame { len, status }) => {
                let heard = core::str::from_utf8(&buffer[..len]).unwrap_or("(not text)");
                let _ = writeln!(
                    uart,
                    "heard  {heard} at {} dBm, SNR {} dB\r",
                    status.rssi_dbm.round_db(),
                    status.snr_db.round_db()
                );
            }
            Ok(Reception::Corrupt) => {
                let _ = writeln!(uart, "heard  a frame whose CRC failed\r");
            }
            Ok(Reception::Timeout) => {}
            Err(_) => {
                let _ = writeln!(uart, "the radio stopped answering\r");
            }
        }

        let now_us = timer.get_counter().ticks();
        if duty.ready(now_us) {
            let mut frame = [0u8; 32];
            let len = beacon(&mut frame, reading);
            match radio.transmit(&frame[..len]) {
                Ok(airtime_us) => {
                    duty.transmitted(now_us, &link, len);
                    let _ = writeln!(uart, "sent   reading {reading} in {airtime_us} us on air\r");
                    reading += 1;
                }
                Err(_) => {
                    let _ = writeln!(uart, "the frame did not go out\r");
                }
            }
        }
    }
}

/// Writes a beacon into a buffer without allocating, and returns its length.
fn beacon(frame: &mut [u8; 32], reading: u32) -> usize {
    let mut cursor = Cursor { frame, len: 0 };
    let _ = write!(cursor, "pico reading {reading}");
    cursor.len
}

/// A writer over a fixed buffer, so a `no_std` node can format a frame.
struct Cursor<'a> {
    frame: &'a mut [u8; 32],
    len: usize,
}

impl Write for Cursor<'_> {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        for byte in text.as_bytes() {
            if self.len == self.frame.len() {
                return Err(core::fmt::Error);
            }
            self.frame[self.len] = *byte;
            self.len += 1;
        }
        Ok(())
    }
}
// ANCHOR_END: example
