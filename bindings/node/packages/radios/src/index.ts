/**
 * Ergonomic facade over the generated LoRa radio binding.
 *
 * `@pamoja/lora` works out what a link costs and how far it reaches; this puts a radio chip
 * on the air to match. For the Semtech SX1261, SX1262, and LLCC68 it builds the bytes of
 * every command and decodes every answer, and for the SX1276 family it gives the register
 * values and decodes the readings. It chooses the amplifier setting a regional EIRP
 * ceiling allows behind an antenna, and holds the radio silent for the off time a duty-cycle
 * limit owes after each frame.
 *
 * @packageDocumentation
 */

export * as sx126x from './sx126x'
export * as sx127x from './sx127x'

import type {
  LoraRadioFamily as NativeLoraRadioFamily,
  LoraReceptionOutcome as NativeLoraReceptionOutcome,
} from '@pamoja/native'

export type {
  LoraRadioConfig,
  LoraRadioWiring,
  LoraReception,
  Sx126xBoard,
  Sx127xBoard,
} from '@pamoja/native'

/**
 * Which family a radio's chip belongs to.
 *
 * Provided as a runtime object plus a matching string-union type, so it works as both a
 * value (`RadioFamily.Sx126x`) and a type annotation.
 */
export const RadioFamily = {
  /** The SX1261, SX1262, SX1268, and LLCC68. */
  Sx126x: 'Sx126x' as NativeLoraRadioFamily,
  /** The SX1276, SX1277, SX1278, and SX1279. */
  Sx127x: 'Sx127x' as NativeLoraRadioFamily,
} as const

/** One of the {@link RadioFamily} values. */
export type RadioFamily = NativeLoraRadioFamily

/** How a reception ended. */
export const ReceptionOutcome = {
  /** A frame arrived and checked. */
  Frame: 'Frame' as NativeLoraReceptionOutcome,
  /** No frame arrived before the timeout. */
  Timeout: 'Timeout' as NativeLoraReceptionOutcome,
  /** A frame arrived whose header or CRC failed its check, and was dropped. */
  Corrupt: 'Corrupt' as NativeLoraReceptionOutcome,
} as const

/** One of the {@link ReceptionOutcome} values. */
export type ReceptionOutcome = NativeLoraReceptionOutcome

export {
  /**
   * A LoRa radio opened on a Linux board.
   *
   * `LoraRadio.openSx126x` and `LoraRadio.openSx127x` open a module through the kernel's
   * spidev and GPIO character devices and reset it; every call after that is the same for
   * either family, and each waits on a worker thread rather than the event loop. Only Linux
   * reaches a radio, and opening one anywhere else throws.
   */
  LoraRadio,
  /**
   * The silence a radio owes after its transmissions under a duty-cycle limit.
   *
   * Record each transmission with `transmitted`, then ask `ready` or `waitUs` before the
   * next, against a microsecond clock the caller keeps. A limit of zero forbids
   * transmitting, and the guard never becomes ready.
   */
  RadioDutyCycle as DutyCycle,
} from '@pamoja/native'
