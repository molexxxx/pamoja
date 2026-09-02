/**
 * Ergonomic facade over the generated LoRa binding.
 *
 * LoRa buys kilometres of range on license-free bands at tiny power, and the
 * price is time: a transmission occupies the channel for a duration the radio
 * settings fix, and the regional rules cap how much of the time a node may
 * transmit. This is the arithmetic that keeps a node inside that budget, with no
 * radio and no floating point in the answer.
 *
 * @packageDocumentation
 */

import {
  type LoraLink,
  loraAirtimeUs,
  loraLinkDefault,
  loraMinOffTimeUs,
  loraSymbolTimeUs,
} from '../index'

export { type LoraLink }

/**
 * Returns the settings for a spreading factor and bandwidth, with LoRa defaults.
 *
 * The defaults are coding rate 4/5, an eight-symbol preamble, an explicit header,
 * and CRC on, which is a typical uplink. Adjust the returned object to change any
 * of them.
 *
 * @param spreadingFactor - The spreading factor, clamped to 5 (fastest) to 12
 *   (longest range).
 * @param bandwidthHz - The channel bandwidth in hertz, such as 125000.
 * @returns The link settings.
 */
export function link(spreadingFactor: number, bandwidthHz: number): LoraLink {
  return loraLinkDefault(spreadingFactor, bandwidthHz)
}

/**
 * Returns the duration of one symbol on a link, in microseconds.
 *
 * @param settings - The link settings.
 * @returns The symbol time in microseconds.
 */
export function symbolTimeUs(settings: LoraLink): number {
  return loraSymbolTimeUs(settings)
}

/**
 * Returns the time on air of a payload, in microseconds.
 *
 * This is the channel occupancy a transmission costs: how long the radio holds
 * the air, which sets both the duty-cycle budget and most of the energy the
 * transmission spends.
 *
 * @param settings - The link settings.
 * @param payloadLength - The payload length in bytes.
 * @returns The time on air in microseconds.
 */
export function airtimeUs(settings: LoraLink, payloadLength: number): number {
  return loraAirtimeUs(settings, payloadLength)
}

/**
 * Returns the minimum silence after a transmission to honor a duty-cycle limit.
 *
 * @param settings - The link settings.
 * @param payloadLength - The payload length in bytes.
 * @param dutyCyclePermille - The limit in parts per thousand, so 10 is 1%.
 * @returns The required off time in microseconds, or `null` when the limit is
 *   zero, which forbids transmitting at all.
 */
export function minOffTimeUs(
  settings: LoraLink,
  payloadLength: number,
  dutyCyclePermille: number,
): number | null {
  return loraMinOffTimeUs(settings, payloadLength, dutyCyclePermille)
}

/**
 * Returns how many transmissions of a payload fit in an hour under a duty-cycle
 * limit.
 *
 * The airtime plus the silence it forces is what one transmission really costs,
 * so this is the message budget a deployment plans against.
 *
 * @param settings - The link settings.
 * @param payloadLength - The payload length in bytes.
 * @param dutyCyclePermille - The limit in parts per thousand, so 10 is 1%.
 * @returns The number of whole transmissions per hour, or 0 when the limit
 *   forbids transmitting.
 */
export function messagesPerHour(
  settings: LoraLink,
  payloadLength: number,
  dutyCyclePermille: number,
): number {
  const offTime = loraMinOffTimeUs(settings, payloadLength, dutyCyclePermille)
  if (offTime === null) {
    return 0
  }
  return Math.floor(3_600_000_000 / (loraAirtimeUs(settings, payloadLength) + offTime))
}
