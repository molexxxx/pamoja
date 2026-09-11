//! Semtech SX1261, SX1262, and LLCC68 transceivers.
//!
//! These radios are run by commands. The host sends an opcode and its parameters in one
//! SPI transaction framed by NSS, and the chip holds its BUSY line high while it is still
//! processing, so the host waits for BUSY to fall before every command. Chapter 11 of the
//! SX1261/2 datasheet (Rev 2.2) lists the commands and chapter 13 details them. This
//! module carries them as data:
//!
//! - [`command`] - the opcodes and the bytes each command sends.
//! - [`irq`] - the interrupt bits of the IRQ register.
//! - [`status`] - the status byte, the device errors, the receive buffer, and the LoRa
//!   packet status, decoded with the datasheet's formulas.
//! - [`config`] - the frequency word, timeouts, image calibration, the LoRa modulation
//!   and packet parameters a [`LinkSettings`](pamoja_lora::LinkSettings) turns into, the
//!   power amplifier settings, and the registers the chapter 15 workarounds touch.
//!
//! The SX1261 has a low power amplifier for up to +15 dBm, and the SX1262 and the LLCC68
//! a high power one for up to +22 dBm; the SPI interface cannot tell them apart, so the
//! caller names the amplifier. With the `embedded-hal` feature, the `Sx126x` driver puts
//! the commands in the order chapter 14 gives for a transmission and a reception, with
//! the chapter 15 workarounds applied.

pub mod command;
pub mod config;
pub mod irq;
pub mod status;
