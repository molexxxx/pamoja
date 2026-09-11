//! Semtech SX1276, SX1277, SX1278, and SX1279 transceivers, and the modules built on them
//! such as the RFM95W.
//!
//! These radios are run through registers. The host writes the carrier, the modem, and the
//! amplifier settings into them, fills the data buffer through RegFifo, and sets the
//! operating mode in RegOpMode to transmit or receive, and RegIrqFlags reports how it went.
//! The SX1276/77/78/79 datasheet (Rev 7) describes the registers, and this module carries
//! them as data:
//!
//! - [`register`] - the register addresses, the SPI address byte, and the operating modes.
//! - [`irq`] - the LoRa interrupt flags.
//! - [`status`] - the signal levels of a received packet and the modem's live state.
//! - [`config`] - the frequency word, the LoRa modem settings a
//!   [`LinkSettings`](pamoja_lora::LinkSettings) turns into, the amplifier and current limit
//!   settings, the sync word, the IQ polarity, and the register values of the errata.
//!
//! The four chips share their registers and differ in the bands they cover, and the SPI
//! interface cannot see which amplifier output a module wires to its antenna, so the caller
//! names it. With the `embedded-hal` feature, the [`Sx127x`] driver resets the chip,
//! calibrates its receiver at the carrier, and follows the datasheet's transmit and receive
//! sequences.

pub mod config;
pub mod irq;
pub mod register;
pub mod status;

#[cfg(feature = "embedded-hal")]
mod driver;

#[cfg(feature = "embedded-hal")]
pub use driver::{
    Board, RadioConfig, RadioError, Reception, Sx127x, CALIBRATION_LIMIT_US, CALIBRATION_POLL_US,
    IRQ_POLL_US, RESET_HOLD_US, RESET_SETTLE_US, TIMEOUT_MARGIN_US,
};
