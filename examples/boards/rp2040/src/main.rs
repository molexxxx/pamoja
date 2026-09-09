//! The first program on a Raspberry Pi Pico: a BME280 on I2C0, read through the driver
//! pamoja ships, reported over UART0 every two seconds with the LED toggling on each read.
//!
//! Wire the BME280's SDA to GP4, its SCL to GP5, VIN to 3V3(OUT), and GND to any ground
//! pin; a USB serial adapter on GP0 (TX) and GP1 (RX) at 115200 baud shows the readings.
//! Hold BOOTSEL while plugging the board in, then `cargo run --release` loads it through
//! picotool. See docs/boards/rp2040.md.

#![no_std]
#![no_main]

use core::fmt::Write;

use embedded_alloc::Heap;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::StatefulOutputPin;
use panic_halt as _;
use rp2040_hal as hal;
use rp2040_hal::fugit::RateExtU32;
use rp2040_hal::Clock;

/// The second-stage bootloader the RP2040 needs at the start of flash.
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

/// The core's own types hold an owned topic and payload, so the binary carries a heap;
/// the driver itself never allocates.
#[global_allocator]
static HEAP: Heap = Heap::empty();

/// The Pico's crystal.
const XTAL_FREQ_HZ: u32 = 12_000_000;

// ANCHOR: example
use hal::gpio::{FunctionI2C, Pin};
use hal::uart::{DataBits, StopBits, UartConfig, UartPeripheral};
use pamoja_sensors::bme280::{Bme280, I2C_ADDRESS_PRIMARY};

#[hal::entry]
fn main() -> ! {
    {
        const HEAP_SIZE: usize = 4 * 1024;
        static mut HEAP_MEM: [core::mem::MaybeUninit<u8>; HEAP_SIZE] =
            [core::mem::MaybeUninit::uninit(); HEAP_SIZE];
        // SAFETY: the heap is initialized once, here, before anything allocates.
        unsafe { HEAP.init(core::ptr::addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }
    }

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

    // I2C0 on the pins the board's own SDK calls its default I2C, GP4 and GP5, at the
    // 400 kHz the BME280 accepts.
    let sda: Pin<_, FunctionI2C, _> = pins.gpio4.reconfigure();
    let scl: Pin<_, FunctionI2C, _> = pins.gpio5.reconfigure();
    let i2c = hal::I2C::i2c0(
        pac.I2C0,
        sda,
        scl,
        400.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    // UART0 on GP0 and GP1 carries the readings to a serial adapter; the LED on GP25
    // toggles on each one.
    let uart_pins = (pins.gpio0.into_function(), pins.gpio1.into_function());
    let mut uart = UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(
            UartConfig::new(115_200.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .expect("a valid UART configuration");
    let mut led = pins.gpio25.into_push_pull_output();

    // The timer is the driver's delay: the datasheet's start-up and measurement waits.
    let mut timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let mut sensor = Bme280::i2c(i2c, I2C_ADDRESS_PRIMARY, timer);
    sensor.init().expect("the BME280 answers on I2C0");

    loop {
        let measurement = sensor.measure().expect("a measurement");
        let _ = writeln!(
            uart,
            "{:.2} C, {:.2} hPa, {:.2} % humidity\r",
            measurement.celsius(),
            measurement.hectopascals(),
            measurement.relative_humidity_percent()
        );
        let _ = led.toggle();
        timer.delay_ms(2000);
    }
}
// ANCHOR_END: example
