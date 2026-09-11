/**
 * Ergonomic facade over the generated LoRa binding.
 *
 * LoRa buys kilometers of range on license-free bands at tiny power, and the
 * price is time: a transmission occupies the channel for a duration the radio
 * settings fix, and the regional rules cap how much of the time a node may
 * transmit. This is the arithmetic that keeps a node inside that budget, with no
 * radio and no floating point in the answer.
 *
 * It also works out how far a link reaches: the EIRP an antenna and cable leave,
 * the free-space loss of a path, the first Fresnel zone, the sensitivity of a
 * receiver, and the power 47 CFR 15.247 allows through an antenna, in decibels
 * resolved to a hundredth.
 *
 * @packageDocumentation
 */

import {
  type LoraLink,
  type LoraLinkBudget,
  loraAirtimeUs,
  loraDemodulatorSnrDb,
  loraEirpDbm,
  loraFccMaxConductedDbm,
  loraFreeSpaceLossDb,
  loraFresnelRadiusMm,
  loraLinkBudgetDefault,
  loraLinkDefault,
  loraMarginDb,
  loraMaxPathLossDb,
  loraMaxTransmitPowerDbm,
  loraMinOffTimeUs,
  loraNoiseFloorDbm,
  loraReceivedDbm,
  loraSensitivityDbm,
  loraSymbolTimeUs,
} from '@pamoja/native'
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
} from '@pamoja/native'

export { type LoraLink, type LoraLinkBudget }

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

/**
 * A typical noise figure for a Semtech sub-GHz LoRa radio, in dB.
 *
 * Semtech AN1200.22 takes 6 dB as the receiver behind the SX1272 and SX1276
 * datasheet sensitivities.
 */
export const RADIO_NOISE_FIGURE_DB = 6

/** A typical noise figure for a LoRaWAN gateway receiver, in dB, from Semtech TN1300.05. */
export const GATEWAY_NOISE_FIGURE_DB = 3

/**
 * Describes the gains and losses of a link, from the transmitting radio to the
 * receiving one.
 *
 * Parts left out keep their defaults: 0 dBm between isotropic antennas with no
 * cable loss, heard with {@link RADIO_NOISE_FIGURE_DB}. Every value is in decibels
 * and resolves to a hundredth of a decibel.
 *
 * @param parts - The parts of the link that differ from the defaults.
 * @returns The link budget.
 *
 * @example
 * ```ts
 * const node = linkBudget({ transmitPowerDbm: 14, transmitAntennaGainDbi: 2.15 })
 * eirpDbm(node) // 16.15
 * ```
 */
export function linkBudget(parts: Partial<LoraLinkBudget> = {}): LoraLinkBudget {
  return { ...loraLinkBudgetDefault(), ...parts }
}

/**
 * Returns the equivalent isotropically radiated power of a budget, in dBm.
 *
 * This is the transmit power plus the transmitting antenna gain, less the
 * transmitting cable loss, and it is the figure regional power ceilings limit.
 *
 * @param budget - The link budget.
 * @returns The EIRP in dBm.
 */
export function eirpDbm(budget: LoraLinkBudget): number {
  return loraEirpDbm(budget)
}

/**
 * Returns the power that reaches the receiving radio across a path, in dBm.
 *
 * @param budget - The link budget.
 * @param pathLossDb - The loss between the two antennas, such as
 *   {@link freeSpaceLossDb}.
 * @returns The received power in dBm.
 */
export function receivedDbm(budget: LoraLinkBudget, pathLossDb: number): number {
  return loraReceivedDbm(budget, pathLossDb)
}

/**
 * Returns the weakest signal the receiver of a budget can demodulate on a link.
 *
 * Sensitivity is the {@link noiseFloorDbm | noise floor} of the channel, raised by
 * the noise figure of the receiver and lowered by the
 * {@link demodulatorSnrDb | demodulator SNR}. With a 6 dB noise figure it lands
 * within about half a decibel of the SX1261/2 datasheet at 125 and 250 kHz.
 *
 * @param budget - The link budget, whose noise figure applies.
 * @param settings - The link settings, whose spreading factor and bandwidth set
 *   the floor.
 * @returns The sensitivity in dBm.
 */
export function sensitivityDbm(budget: LoraLinkBudget, settings: LoraLink): number {
  return loraSensitivityDbm(budget, settings)
}

/**
 * Returns the most path loss a link survives, in dB.
 *
 * @param budget - The link budget.
 * @param settings - The link settings, whose spreading factor and bandwidth set
 *   the sensitivity.
 * @returns The loss above which the link does not close.
 */
export function maxPathLossDb(budget: LoraLinkBudget, settings: LoraLink): number {
  return loraMaxPathLossDb(budget, settings)
}

/**
 * Returns how far above the sensitivity a signal arrives across a path, in dB.
 *
 * @param budget - The link budget.
 * @param settings - The link settings, whose spreading factor and bandwidth set
 *   the sensitivity.
 * @param pathLossDb - The loss between the two antennas.
 * @returns The link margin, which is negative where the path loses more than the
 *   link survives.
 */
export function marginDb(
  budget: LoraLinkBudget,
  settings: LoraLink,
  pathLossDb: number,
): number {
  return loraMarginDb(budget, settings, pathLossDb)
}

/**
 * Returns the most transmit power that keeps the EIRP of a budget at or under a
 * ceiling, in dBm.
 *
 * A higher-gain antenna leaves less power for the radio.
 *
 * @param budget - The link budget, whose transmitting antenna and cable apply.
 * @param eirpCeilingDbm - The EIRP limit, such as a channel plan's
 *   `maxEirpDbm` for a frequency.
 * @returns The transmit power in dBm.
 */
export function maxTransmitPowerDbm(budget: LoraLinkBudget, eirpCeilingDbm: number): number {
  return loraMaxTransmitPowerDbm(budget, eirpCeilingDbm)
}

/**
 * Returns the thermal noise power in a channel, in dBm.
 *
 * This is -174 dBm/Hz plus `10 log10` of the bandwidth, as Semtech AN1200.22
 * derives it.
 *
 * @param bandwidthHz - The channel bandwidth in hertz.
 * @returns The noise power in dBm.
 */
export function noiseFloorDbm(bandwidthHz: number): number {
  return loraNoiseFloorDbm(bandwidthHz)
}

/**
 * Returns the signal-to-noise ratio the LoRa demodulator needs at a spreading
 * factor, in dB.
 *
 * These are the typical figures of Table 6-1 in the SX1261/2 datasheet, from
 * -2.5 dB at SF5 to -20 dB at SF12.
 *
 * @param spreadingFactor - The spreading factor, clamped to 5 to 12.
 * @returns The required SNR in dB.
 */
export function demodulatorSnrDb(spreadingFactor: number): number {
  return loraDemodulatorSnrDb(spreadingFactor)
}

/**
 * Returns the free-space basic transmission loss between isotropic antennas, in dB.
 *
 * This is Recommendation ITU-R P.525-5, equation (5): the loss with nothing in the
 * way, which grows by 6.02 dB each time the distance doubles.
 *
 * @param distanceM - The distance between the antennas in meters.
 * @param frequencyHz - The carrier frequency in hertz.
 * @returns The loss in dB.
 */
export function freeSpaceLossDb(distanceM: number, frequencyHz: number): number {
  return loraFreeSpaceLossDb(distanceM, frequencyHz)
}

/**
 * Returns the radius of the first Fresnel ellipsoid at a point on a path, in
 * millimeters.
 *
 * This is Recommendation ITU-R P.526-16, equation (2). The radius is widest halfway
 * along the path, and the diffraction zone starts where the clearance falls to 60%
 * of it.
 *
 * @param nearM - The distance from one antenna to the point, in meters.
 * @param farM - The distance from the point to the other antenna, in meters.
 * @param frequencyHz - The carrier frequency in hertz.
 * @returns The radius in millimeters.
 */
export function fresnelRadiusMm(nearM: number, farM: number, frequencyHz: number): number {
  return loraFresnelRadiusMm(nearM, farM, frequencyHz)
}

/**
 * Returns the most conducted power 47 CFR 15.247 allows a 902-928 MHz transmitter
 * through an antenna, in dBm.
 *
 * The limit is 1 W for digital modulation and for hopping on at least 50 channels,
 * and 0.25 W for hopping on 25 to 49, less every decibel the antenna gain exceeds
 * 6 dBi.
 *
 * @param antennaGainDbi - The directional gain of the transmitting antenna.
 * @param hoppingChannels - The number of hopping channels, left out for a system
 *   using digital modulation.
 * @returns The limit in dBm, or `null` for hopping on fewer than 25 channels, which
 *   paragraph (b)(2) sets no limit for.
 */
export function fccMaxConductedDbm(antennaGainDbi: number, hoppingChannels?: number): number | null {
  return loraFccMaxConductedDbm(antennaGainDbi, hoppingChannels)
}
