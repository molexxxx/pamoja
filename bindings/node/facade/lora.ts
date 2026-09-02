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
import {
  LoraChannelPlan,
  LoraPlanBuilder,
  type LoraBeacon,
  type LoraChannelBlock,
  type LoraChannelSet as NativeLoraChannelSet,
  type LoraDataRate,
  type LoraDirection as NativeLoraDirection,
  type LoraMaxPayload,
  type LoraModulation as NativeLoraModulation,
  type LoraPayloadTable as NativeLoraPayloadTable,
  type LoraPlanInfo,
  type LoraRegion as NativeLoraRegion,
  type LoraRx2,
  type LoraSubBand,
} from '../index'

export { type LoraLink }

export {
  LoraChannelPlan,
  LoraPlanBuilder,
  type LoraBeacon,
  type LoraChannelBlock,
  type LoraDataRate,
  type LoraMaxPayload,
  type LoraPlanInfo,
  type LoraRx2,
  type LoraSubBand,
}

/**
 * A band with a published channel plan.
 *
 * Provided as a runtime object, as the generated string enum is erased at
 * compile time and so has no value a JavaScript caller can reach. Members carry
 * the generated enum's type as well, so they pass straight into the native
 * methods.
 */
export const LoraRegion = {
  /** Europe, 863-870 MHz. */
  Eu868: 'Eu868' as NativeLoraRegion,
  /** North America, 902-928 MHz. */
  Us915: 'Us915' as NativeLoraRegion,
  /** Europe, 433 MHz. */
  Eu433: 'Eu433' as NativeLoraRegion,
  /** Australia, 915-928 MHz. */
  Au915: 'Au915' as NativeLoraRegion,
  /** China, 470-510 MHz. */
  Cn470: 'Cn470' as NativeLoraRegion,
  /** Asia, 923 MHz. */
  As923: 'As923' as NativeLoraRegion,
  /** South Korea, 920-923 MHz. */
  Kr920: 'Kr920' as NativeLoraRegion,
  /** India, 865-867 MHz. */
  In865: 'In865' as NativeLoraRegion,
  /** Russia, 864-870 MHz. */
  Ru864: 'Ru864' as NativeLoraRegion,
} as const

/** One of the {@link LoraRegion} values. */
export type LoraRegion = NativeLoraRegion

/** Which direction a data-rate table describes. */
export const LoraDirection = {
  /** From the device to the network. */
  Uplink: 'Uplink' as NativeLoraDirection,
  /** From the network to the device. */
  Downlink: 'Downlink' as NativeLoraDirection,
} as const

/** One of the {@link LoraDirection} values. */
export type LoraDirection = NativeLoraDirection

/** Which of a plan's payload tables to read. */
export const LoraPayloadTable = {
  /** Uplink, for a device that may sit behind a repeater. */
  UplinkRepeater: 'UplinkRepeater' as NativeLoraPayloadTable,
  /** Uplink, for a device that will not. */
  UplinkDirect: 'UplinkDirect' as NativeLoraPayloadTable,
  /** Downlink, for a device that may sit behind a repeater. */
  DownlinkRepeater: 'DownlinkRepeater' as NativeLoraPayloadTable,
  /** Downlink, for a device that will not. */
  DownlinkDirect: 'DownlinkDirect' as NativeLoraPayloadTable,
  /** The limits that apply under a dwell-time limit. */
  DwellLimited: 'DwellLimited' as NativeLoraPayloadTable,
} as const

/** One of the {@link LoraPayloadTable} values. */
export type LoraPayloadTable = NativeLoraPayloadTable

/** Which channels of a plan to read. */
export const LoraChannelSet = {
  /** The channels a device must use to send a join request. */
  Join: 'Join' as NativeLoraChannelSet,
  /** The channels a device starts with before a network adds any. */
  Default: 'Default' as NativeLoraChannelSet,
} as const

/** One of the {@link LoraChannelSet} values. */
export type LoraChannelSet = NativeLoraChannelSet

/** How a data rate is carried on the air. */
export const LoraModulation = {
  /** LoRa modulation, described by a spreading factor and bandwidth. */
  Lora: 'Lora' as NativeLoraModulation,
  /** Frequency-shift keying, described by its bitrate alone. */
  Fsk: 'Fsk' as NativeLoraModulation,
  /** Long-range frequency-hopping spread spectrum. */
  LrFhss: 'LrFhss' as NativeLoraModulation,
  /** A data-rate number the region reserves, which carries nothing. */
  Reserved: 'Reserved' as NativeLoraModulation,
} as const

/** One of the {@link LoraModulation} values. */
export type LoraModulation = NativeLoraModulation

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

/**
 * Returns the published channel plan for a region.
 *
 * A channel plan is what a regulator and the LoRa Alliance publish about one
 * band: which data rates exist, what each carries, how much of the time a node
 * may hold a frequency, and where it listens for a downlink. The plan reports
 * those facts and costs a transmission out against them; it never refuses one,
 * because a deployment may hold licensed spectrum or be working under emergency
 * provisions and only the operator knows which.
 *
 * @param region - The band to describe.
 * @returns The plan, which answers every question about that band.
 *
 * @example
 * ```ts
 * const plan = planFor(LoraRegion.Eu868)
 * const link = plan.linkSettings(5)
 * const permille = plan.dutyCyclePermille(868_100_000)
 * ```
 */
export function planFor(region: LoraRegion): LoraChannelPlan {
  return LoraChannelPlan.forRegion(region)
}

/**
 * Returns how many transmissions of a payload fit in an hour at a data rate the
 * region defines.
 *
 * This is the budget question a deployment actually asks: not what the radio can
 * do, but how often it may speak on this band at this setting. The duty cycle of
 * the frequency it transmits on decides the answer.
 *
 * @param plan - The channel plan to read.
 * @param dataRate - The uplink data-rate number.
 * @param payloadLength - The payload length in bytes.
 * @param frequencyHz - The frequency the node transmits on.
 * @returns The number of whole transmissions per hour, or `null` when the plan
 *   does not describe that data rate or frequency.
 */
export function messagesPerHourAt(
  plan: LoraChannelPlan,
  dataRate: number,
  payloadLength: number,
  frequencyHz: number,
): number | null {
  const settings = plan.linkSettings(dataRate)
  const permille = plan.dutyCyclePermille(frequencyHz)
  if (settings == null || permille == null) {
    return null
  }
  return messagesPerHour(settings, payloadLength, permille)
}
