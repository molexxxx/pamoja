/**
 * Ergonomic facade over the generated LoRa radio binding.
 *
 * `@pamoja/lora` works out what a link costs and how far it reaches; this puts a radio chip
 * on the air to match. For the Semtech SX1261, SX1262, and LLCC68 it builds the bytes of
 * every command and decodes every answer, chooses the amplifier setting a regional EIRP
 * ceiling allows behind an antenna, and holds the radio silent for the off time a duty-cycle
 * limit owes after each frame.
 *
 * @packageDocumentation
 */

export * as sx126x from './sx126x'

export {
  /**
   * The silence a radio owes after its transmissions under a duty-cycle limit.
   *
   * Record each transmission with `transmitted`, then ask `ready` or `waitUs` before the
   * next, against a microsecond clock the caller keeps. A limit of zero forbids
   * transmitting, and the guard never becomes ready.
   */
  RadioDutyCycle as DutyCycle,
} from '@pamoja/native'
