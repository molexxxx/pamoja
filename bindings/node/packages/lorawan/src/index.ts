/**
 * Ergonomic facade over the generated LoRaWAN binding.
 *
 * A long-range public-band link is wide open, so LoRaWAN wraps every frame in two
 * guarantees: a message integrity code keyed to the network proves the frame is
 * authentic and intact, and the payload is encrypted to the application so only
 * its owner can read it. This builds and verifies exactly that.
 *
 * The frame direction is re-exported as a runtime {@link Direction} object,
 * because the generated enum is types-only.
 *
 * @packageDocumentation
 */

import type {
  LoraChannelPlan,
  LorawanBackoffStep,
  LorawanCfListKind as CfListKindName,
  LorawanChannel,
  LorawanDelivery,
  LorawanDeviceSettings,
  LorawanFrameCounters,
  LorawanFrequencySpan,
  LorawanHeard,
  LorawanNext,
  LorawanRx2,
  LorawanTransmission,
  LorawanWindow,
  LorawanDirection as DirectionName,
  LorawanMessageType as MessageTypeName,
  LorawanReceiveWindow as ReceiveWindowName,
  LorawanVersion as VersionName,
  LoraRelayChannel,
  LorawanAckWindow,
  LorawanAcknowledgment,
  LorawanCadPeriodicity as CadPeriodicityName,
  LorawanCadToRx as CadToRxName,
  LorawanCarrier,
  LorawanForwardedUplink,
  LorawanListen,
  LorawanRelayActivation as RelayActivationName,
  LorawanRelayExchange,
  LorawanRelayForward as RelayForwardName,
  LorawanRelayHeard,
  LorawanRelayHeardKind as RelayHeardKindName,
  LorawanRelayStatus,
  LorawanRelaySync as RelaySyncName,
  LorawanRxrDownlink,
  LorawanScan,
  LorawanStateSync,
  LorawanSynchronization,
  LorawanUplinkMetadata,
  LorawanWake,
  LorawanWakeKind as WakeKindName,
  LorawanWakeUp,
  LorawanWor,
  LorawanWorChannel as WorChannelName,
  LorawanWorKeys,
  LorawanWorKind as WorKindName,
  LorawanWorNext,
  LorawanWorSlot,
  LorawanXtalAccuracy as XtalAccuracyName,
} from '@pamoja/native'

import {
  LORAWAN_ADR_ACK_DELAY,
  LORAWAN_ADR_ACK_LIMIT,
  LORAWAN_JOIN_ACCEPT_DELAY1_US,
  LORAWAN_JOIN_ACCEPT_DELAY2_US,
  LORAWAN_MAX_FCNT_GAP,
  LORAWAN_MAX_FRAME,
  LORAWAN_MAX_PAYLOAD,
  LORAWAN_RECEIVE_DELAY1_US,
  LORAWAN_RECEIVE_DELAY2_US,
  LORAWAN_RECEIVE_WINDOW_TOLERANCE_US,
  LORAWAN_RETRANSMIT_TIMEOUT_MAX_US,
  LORAWAN_RETRANSMIT_TIMEOUT_MIN_US,
  LorawanBackoff,
  LorawanCfList,
  LorawanDevice,
  LorawanEndDevice,
  type LorawanGrant,
  LorawanJoinAccept,
  type LorawanMacCommand,
  LorawanRelay,
  type LorawanOptions,
  LorawanSession,
  lorawanGrantAccept,
  lorawanGrantSession,
  lorawanMacEncode,
  lorawanMacParse,
  lorawanParseHeader,
  lorawanParseJoinRequest,
  LORAWAN_FORWARD_OVERHEAD,
  LORAWAN_LA_FPORT_RELAY,
  LORAWAN_MIN_WOR_PREAMBLE_SYMBOLS,
  LORAWAN_RELAY_FWD_DELAY_US,
  LORAWAN_RXR_DELAY_US,
  LORAWAN_TRUSTED_ED_NUMBER,
  LORAWAN_WOR_ACK_DELAY_US,
  LORAWAN_WOR_ATTEMPTS_WO_ACK,
  LORAWAN_WOR_DATA_DELAY_US,
  lorawanRelayForwardEncode,
  lorawanRelayForwardParse,
  lorawanRelayNextWor,
  lorawanRelayRootWorSKey,
  lorawanRelaySecondChannel,
  lorawanRelaySynchronization,
  lorawanRelayTOffsetMs,
  lorawanRelayUnsynchronizedPreamble,
  lorawanRelayWorAck,
  lorawanRelayWorAckOpen,
  lorawanRelayWorJoinRequest,
  lorawanRelayWorKeys,
  lorawanRelayWorOpen,
  lorawanRelayWorParse,
  lorawanRelayWorUplink,
} from '@pamoja/native'

export {
  type LorawanAckWindow as AckWindow,
  type LorawanAcknowledgment as Acknowledgment,
  type LorawanCarrier as Carrier,
  type LorawanForwardedUplink as ForwardedUplink,
  type LorawanListen as Listen,
  type LorawanRelayExchange as RelayExchange,
  type LorawanRelayStatus as RelayStatus,
  type LorawanRxrDownlink as RxrDownlink,
  type LorawanScan as Scan,
  type LorawanStateSync as StateSync,
  type LorawanSynchronization as Synchronization,
  type LorawanUplinkMetadata as UplinkMetadata,
  type LorawanWakeUp as WakeUp,
  type LorawanWor as Wor,
  type LorawanWorKeys as WorKeys,
  type LorawanWorSlot as WorSlot,
  type LorawanOptions as Options,
  type LorawanBackoffStep as BackoffStep,
  type LorawanChannel as Channel,
  type LorawanDeviceSettings as DeviceSettings,
  type LorawanFrameCounters as FrameCounters,
  type LorawanFrequencySpan as FrequencySpan,
  type LorawanRx2 as Rx2,
  type LorawanTransmission as Transmission,
  type LorawanWindow as Window,
}

/** The largest application payload, in bytes, a single frame can carry. */
export const MAX_PAYLOAD = LORAWAN_MAX_PAYLOAD

/** The largest frame, in bytes, this build accepts. */
export const MAX_FRAME = LORAWAN_MAX_FRAME

/** How long after an uplink the first receive window opens, RP002-1.0.5 section 3.3. */
export const RECEIVE_DELAY1_US = LORAWAN_RECEIVE_DELAY1_US

/** How long after an uplink the second receive window opens. */
export const RECEIVE_DELAY2_US = LORAWAN_RECEIVE_DELAY2_US

/** How long after a join request the first join accept window opens. */
export const JOIN_ACCEPT_DELAY1_US = LORAWAN_JOIN_ACCEPT_DELAY1_US

/** How long after a join request the second join accept window opens. */
export const JOIN_ACCEPT_DELAY2_US = LORAWAN_JOIN_ACCEPT_DELAY2_US

/** How far a receive window may open either side of its time, LoRaWAN 1.0.3 section 3.3.1. */
export const RECEIVE_WINDOW_TOLERANCE_US = LORAWAN_RECEIVE_WINDOW_TOLERANCE_US

/** The largest gap a frame counter may jump across and still be accepted. */
export const MAX_FCNT_GAP = LORAWAN_MAX_FCNT_GAP

/** How many unanswered uplinks before a device asks the network to answer. */
export const ADR_ACK_LIMIT = LORAWAN_ADR_ACK_LIMIT

/** How many more before a device starts giving back what adaptive data rate took. */
export const ADR_ACK_DELAY = LORAWAN_ADR_ACK_DELAY

/** The shortest wait before a confirmed uplink is sent again. */
export const RETRANSMIT_TIMEOUT_MIN_US = LORAWAN_RETRANSMIT_TIMEOUT_MIN_US

/** The longest wait before a confirmed uplink is sent again. */
export const RETRANSMIT_TIMEOUT_MAX_US = LORAWAN_RETRANSMIT_TIMEOUT_MAX_US

/** A revision of the LoRaWAN link layer. */
export const Version = {
  /** LoRaWAN 1.0.3. */
  V1_0_3: 'V1_0_3' as VersionName,
  /** TS001-1.0.4, the LoRaWAN 1.0.4 link layer. */
  V1_0_4: 'V1_0_4' as VersionName,
} as const

/** One of the {@link Version} values. */
export type Version = VersionName

/** Which form a channel list takes, from its last byte. */
export const CfListKind = {
  /** Type 0: a list of frequencies. */
  Frequencies: 'Frequencies' as CfListKindName,
  /** Type 1: groups of channel mask bits. */
  ChannelMasks: 'ChannelMasks' as CfListKindName,
  /** A type the regional parameters reserve, which a device ignores. */
  Reserved: 'Reserved' as CfListKindName,
} as const

/** One of the {@link CfListKind} values. */
export type CfListKind = CfListKindName

/** Which receive window a frame arrived in. */
export const ReceiveWindow = {
  /** The first window, on the uplink's downlink channel. */
  Rx1: 'Rx1' as ReceiveWindowName,
  /** The second, on the fixed frequency and data rate. */
  Rx2: 'Rx2' as ReceiveWindowName,
  /** The relay window, which a device under a relay opens last, TS011-1.0.1 chapter 7. */
  Rxr: 'Rxr' as ReceiveWindowName,
} as const

/** One of the {@link ReceiveWindow} values. */
export type ReceiveWindow = ReceiveWindowName

/**
 * The optional channel list at the end of a join accept.
 *
 * A network that wants a device on more channels than its region's defaults says
 * so in sixteen bytes: five frequencies for a dynamic plan such as EU868, or six
 * groups of channel mask bits for a fixed plan such as US915. The list keeps the
 * bytes as they arrived and reads either form out of them.
 */
export class CfList {
  readonly #inner: LorawanCfList

  /**
   * Wraps a channel list.
   *
   * @param inner - The generated list this facade delegates to.
   */
  constructor(inner: LorawanCfList) {
    this.#inner = inner
  }

  /**
   * Builds a type 0 list from frequencies.
   *
   * @param frequenciesHz - Five frequencies in hertz, with 0 for a slot left unused.
   * @returns The channel list.
   * @throws If there are not five, or a frequency is not a whole number of hundreds
   *   of hertz from 100 MHz to just under 1.678 GHz.
   *
   * @example
   * ```ts
   * const list = CfList.frequencies([867_100_000, 867_300_000, 867_500_000, 867_700_000, 867_900_000])
   * list.bytes.subarray(0, 3) // <Buffer 18 4f 84>
   * ```
   */
  static frequencies(frequenciesHz: readonly number[]): CfList {
    return new CfList(LorawanCfList.fromFrequencies([...frequenciesHz]))
  }

  /**
   * Builds a type 1 list from channel mask groups.
   *
   * @param masks - Six sixteen-bit groups, where bit n of group g enables channel
   *   g * 16 + n.
   * @returns The channel list.
   * @throws If there are not six, or a group does not fit sixteen bits.
   */
  static channelMasks(masks: readonly number[]): CfList {
    return new CfList(LorawanCfList.fromChannelMasks([...masks]))
  }

  /**
   * Keeps a channel list exactly as it arrived, whatever its type byte says.
   *
   * @param bytes - The sixteen CFList bytes.
   * @returns The channel list.
   * @throws If there are not sixteen bytes.
   */
  static fromBytes(bytes: Uint8Array): CfList {
    return new CfList(LorawanCfList.fromBytes(Buffer.from(bytes)))
  }

  /** The sixteen bytes, as a join accept carries them. */
  get bytes(): Buffer {
    return this.#inner.bytes
  }

  /** Which form the list takes. */
  get kind(): CfListKind {
    return this.#inner.kind
  }

  /** The CFListType byte the list ends with. */
  get typeByte(): number {
    return this.#inner.typeByte
  }

  /**
   * Reads the frequencies out of a type 0 list.
   *
   * @returns Five frequencies in hertz, 0 for an unused slot, or `null` for a list of
   *   any other type.
   */
  frequenciesHz(): number[] | null {
    return this.#inner.frequenciesHz() ?? null
  }

  /**
   * Reads the mask groups out of a type 1 list.
   *
   * @returns Six groups, or `null` for a list of any other type.
   */
  channelMaskGroups(): number[] | null {
    return this.#inner.channelMaskGroups() ?? null
  }

  /**
   * Reports whether a type 1 list enables a channel.
   *
   * @param channel - The channel number, group * 16 + bit.
   * @returns Whether its bit is set, or `null` for a list of any other type or a
   *   channel past the 96 the groups cover.
   */
  enables(channel: number): boolean | null {
    return this.#inner.enables(channel) ?? null
  }

  /**
   * Lists the channels a type 1 list enables.
   *
   * @returns The channel numbers, lowest first, which is empty for a list of any
   *   other type.
   */
  enabledChannels(): number[] {
    return this.#inner.enabledChannels()
  }
}

/**
 * A device's count of how long the network has been silent.
 *
 * A network running adaptive data rate moves a device to the fastest rate and
 * lowest power that still reach it. Once uplinks go unanswered, this says when to
 * ask the network to answer and which of those settings to give back, a step at a
 * time, the way LoRaWAN 1.0.3 or TS001-1.0.4 describes.
 *
 * @example
 * ```ts
 * const backoff = new Backoff(Version.V1_0_4)
 * const step = backoff.uplink(dataRate === 0)
 * if (step.lowerDataRate) dataRate -= 1
 * ```
 */
export class Backoff {
  readonly #inner: LorawanBackoff

  /**
   * Starts a count from zero.
   *
   * @param version - The revision whose steps to follow, TS001-1.0.4 by default.
   * @param limit - How many unanswered uplinks before asking, 64 by default.
   * @param delay - How many more before the first step, and between steps after
   *   that, 32 by default. Zero is taken as one.
   */
  constructor(version: Version = Version.V1_0_4, limit?: number, delay?: number) {
    this.#inner = new LorawanBackoff(version, limit, delay)
  }

  /**
   * Counts one new uplink, and says what to do before sending it.
   *
   * Call it once per uplink the frame counter moves for; a repeat of the same uplink
   * does not count.
   *
   * @param atDefaultDataRate - Whether the device is already at its default data
   *   rate, the slowest it uses, so there is no lower rate to step to.
   * @returns Whether to ask for an answer, and which step this uplink takes.
   */
  uplink(atDefaultDataRate: boolean): LorawanBackoffStep {
    return this.#inner.uplink(atDefaultDataRate)
  }

  /** Counts a Class A downlink, which proves the network still hears the device. */
  downlink(): void {
    this.#inner.downlink()
  }

  /** How many uplinks have gone unanswered. */
  get counter(): number {
    return this.#inner.counter
  }

  /** The revision whose steps this count follows. */
  get version(): Version {
    return this.#inner.version
  }
}

/**
 * The direction a frame traveled, which its MIC and encryption both fold in.
 *
 * Provided as a runtime object plus a matching string-union type so it works as
 * both a value (`Direction.Uplink`) and a type annotation.
 */
export const Direction = {
  /** From an end device up to the network. */
  Uplink: 'Uplink' as DirectionName,
  /** From the network down to an end device. */
  Downlink: 'Downlink' as DirectionName,
} as const

/** One of the {@link Direction} values. */
export type Direction = DirectionName

/** A decoded data frame, with its payload decrypted. */
export interface RxData {
  /** The direction the frame traveled. */
  direction: Direction
  /** The device address the frame carries. */
  devAddr: number
  /** The low 16 bits of the frame counter. */
  fcnt: number
  /** Whether the frame asks to be acknowledged. */
  confirmed: boolean
  /** Whether the frame takes part in adaptive data rate. */
  adr: boolean
  /** Whether the frame acknowledges the last confirmed one. */
  ack: boolean
  /** Whether the network has more downlink data waiting. */
  fpending: boolean
  /** Whether an uplink asks the network to answer. */
  adrAckReq: boolean
  /** Whether an uplink comes from a device running Class B. */
  classB: boolean
  /** The port the frame was sent on, or `null` when it carries only options. */
  fport: number | null
  /** The MAC commands the header carried. */
  fopts: Buffer
  /** The decrypted application payload. */
  payload: Buffer
}

/** An activated LoRaWAN session: a device address and its two session keys. */
export class Session {
  readonly #inner: LorawanSession

  /**
   * Wraps an activated session.
   *
   * @param inner - The generated session this facade delegates to.
   */
  constructor(inner: LorawanSession) {
    this.#inner = inner
  }

  /** The device address this session is bound to. */
  get devAddr(): number {
    return this.#inner.devAddr
  }

  /**
   * The root relay session key a network sends a relay for this device, TS011-1.0.1
   * section 4.4, derived from the network session key.
   *
   * @returns The 16-byte key.
   */
  rootWorSKey(): Buffer {
    return this.#inner.rootWorSKey()
  }

  /**
   * The keys this device's wake-on-radio frames are protected with, TS011-1.0.1 section 4.5.
   *
   * @returns The integrity and encryption keys.
   */
  worKeys(): LorawanWorKeys {
    return this.#inner.worKeys()
  }

  /**
   * Encodes an uplink, encrypting the payload and appending the MIC.
   *
   * @param fcnt - The frame counter for this uplink.
   * @param fport - The port; 0 for MAC commands, otherwise an application port.
   * @param payload - The application payload to carry.
   * @param options - The header flags and frame options to set.
   * @returns The frame to transmit.
   * @throws If the payload and options do not fit a single frame.
   */
  encodeUplink(
    fcnt: number,
    fport: number,
    payload: Uint8Array,
    options?: LorawanOptions,
  ): Buffer {
    return this.#inner.encodeUplink(fcnt, fport, Buffer.from(payload), options)
  }

  /**
   * Encodes a downlink, encrypting the payload and appending the MIC.
   *
   * @param fcnt - The frame counter for this downlink.
   * @param fport - The port; 0 for MAC commands, otherwise an application port.
   * @param payload - The application payload to carry.
   * @param options - The header flags and frame options to set.
   * @returns The frame to transmit.
   * @throws If the payload and options do not fit a single frame.
   */
  encodeDownlink(
    fcnt: number,
    fport: number,
    payload: Uint8Array,
    options?: LorawanOptions,
  ): Buffer {
    return this.#inner.encodeDownlink(fcnt, fport, Buffer.from(payload), options)
  }

  /**
   * Verifies a received frame, then decrypts it.
   *
   * @param bytes - The frame exactly as it came off the radio.
   * @param fcnt - The full 32-bit counter expected for this frame; its low 16
   *   bits must match the counter the frame carries.
   * @returns The decoded frame.
   * @throws If the MIC does not verify, the counter does not match, or the frame
   *   is not a data frame.
   */
  decode(bytes: Uint8Array, fcnt: number): RxData {
    // The generated object leaves an absent port undefined; null says the same
    // thing in the shape the rest of this package uses.
    const rx = this.#inner.decode(Buffer.from(bytes), fcnt)
    return { ...rx, fport: rx.fport ?? null }
  }
}

/** An accepted join: the network settings, and the session it grants. */
export class JoinAccept {
  readonly #inner: LorawanJoinAccept

  /**
   * Wraps an accepted join.
   *
   * @param inner - The generated join this facade delegates to.
   */
  constructor(inner: LorawanJoinAccept) {
    this.#inner = inner
  }

  /** The device address the network assigned. */
  get devAddr(): number {
    return this.#inner.devAddr
  }

  /** The identifier of the network that accepted the join. */
  get netId(): number {
    return this.#inner.netId
  }

  /**
   * The downlink settings byte, carrying the second receive window data rate and
   * the first window offset.
   */
  get dlSettings(): number {
    return this.#inner.dlSettings
  }

  /** The delay byte before the first receive window, as it arrived. */
  get rxDelay(): number {
    return this.#inner.rxDelay
  }

  /** How far below the uplink's data rate the first receive window listens. */
  get rx1DrOffset(): number {
    return this.#inner.rx1DrOffset
  }

  /** The data rate the second receive window listens at. */
  get rx2DataRate(): number {
    return this.#inner.rx2DataRate
  }

  /**
   * The delay from the end of an uplink to the first receive window, in
   * microseconds, where a delay byte of zero means one second.
   */
  get receiveDelayUs(): number {
    return this.#inner.receiveDelayUs
  }

  /**
   * Reads the channel list the accept carried.
   *
   * @returns The list, or `null` when the accept carried none.
   */
  cflist(): CfList | null {
    const list = this.#inner.cflist()
    return list == null ? null : new CfList(list)
  }

  /**
   * Takes the activated session this join grants.
   *
   * @returns The session, with its keys already derived.
   */
  session(): Session {
    return new Session(this.#inner.session())
  }
}

/** The root credentials over-the-air activation is built on. */
export class Device {
  readonly #inner: LorawanDevice

  /**
   * Creates a device from its two EUIs and its application key.
   *
   * @param devEui - The 8-byte device EUI.
   * @param appEui - The 8-byte application (join) EUI.
   * @param appKey - The 16-byte application key the join is secured with.
   * @throws If any credential is the wrong length.
   */
  constructor(devEui: Uint8Array, appEui: Uint8Array, appKey: Uint8Array) {
    this.#inner = new LorawanDevice(
      Buffer.from(devEui),
      Buffer.from(appEui),
      Buffer.from(appKey),
    )
  }

  /** The 8-byte device EUI, most-significant byte first. */
  get devEui(): Buffer {
    return this.#inner.devEui
  }

  /**
   * Builds the join request this device broadcasts to activate.
   *
   * @param devNonce - A nonce that must never repeat for this device, since the
   *   network rejects a replayed one.
   * @returns The join request to transmit.
   */
  joinRequest(devNonce: number): Buffer {
    return this.#inner.joinRequest(devNonce)
  }

  /**
   * Turns the join accept a network sent into the settings it grants.
   *
   * @param bytes - The join accept exactly as it arrived.
   * @param devNonce - The nonce the matching join request carried.
   * @returns The accepted join, which grants a session.
   * @throws If the MIC does not verify, or the frame is not a join accept.
   */
  acceptJoin(bytes: Uint8Array, devNonce: number): JoinAccept {
    return new JoinAccept(this.#inner.acceptJoin(Buffer.from(bytes), devNonce))
  }
}

/**
 * Creates a session for a device already activated by personalization.
 *
 * @param devAddr - The device address the network assigned.
 * @param nwkSKey - The 16-byte network session key, which authenticates frames.
 * @param appSKey - The 16-byte application session key, which encrypts payloads.
 * @returns The session, ready to encode and decode data frames.
 * @throws If either key is not 16 bytes.
 */
export function session(
  devAddr: number,
  nwkSKey: Uint8Array,
  appSKey: Uint8Array,
): Session {
  return new Session(
    new LorawanSession(devAddr, Buffer.from(nwkSKey), Buffer.from(appSKey)),
  )
}

/**
 * Creates a device holding the root credentials for over-the-air activation.
 *
 * @param devEui - The 8-byte device EUI.
 * @param appEui - The 8-byte application (join) EUI.
 * @param appKey - The 16-byte application key the join exchange is secured with.
 * @returns The device, ready to build a join request.
 * @throws If any credential is the wrong length.
 */
export function device(
  devEui: Uint8Array,
  appEui: Uint8Array,
  appKey: Uint8Array,
): Device {
  return new Device(devEui, appEui, appKey)
}

/** What kind of message a frame is, read from its header. */
export const MessageType = {
  /** A device asking to join a network. */
  JoinRequest: 'JoinRequest' as MessageTypeName,
  /** A network admitting a device. */
  JoinAccept: 'JoinAccept' as MessageTypeName,
  /** Data from a device that does not need acknowledging. */
  UnconfirmedUp: 'UnconfirmedUp' as MessageTypeName,
  /** Data from a device that asks to be acknowledged. */
  ConfirmedUp: 'ConfirmedUp' as MessageTypeName,
  /** Data to a device that does not need acknowledging. */
  UnconfirmedDown: 'UnconfirmedDown' as MessageTypeName,
  /** Data to a device that asks to be acknowledged. */
  ConfirmedDown: 'ConfirmedDown' as MessageTypeName,
} as const

/** One of the {@link MessageType} values. */
export type MessageType = MessageTypeName

/** What a frame says about itself before any key is involved. */
export interface Header {
  /** What kind of message the frame is. */
  messageType: MessageType
  /** Whether this is a data frame rather than part of a join exchange. */
  isData: boolean
  /** The device address, or `null` for a join frame. */
  devAddr: number | null
  /** The low 16 bits of the frame counter, or `null` for a join frame. */
  fcnt: number | null
  /** The port, or `null` for a join frame or one carrying only options. */
  fport: number | null
  /** Whether the frame asks to be acknowledged. */
  confirmed: boolean
  /** Whether the frame takes part in adaptive data rate. */
  adr: boolean
  /** Whether the frame acknowledges the last confirmed one. */
  ack: boolean
  /** Whether the network has more downlink data waiting. */
  fpending: boolean
  /** Whether an uplink asks the network to answer. */
  adrAckReq: boolean
  /** Whether an uplink comes from a device running Class B. */
  classB: boolean
  /** How many bytes of frame options the header carries. */
  foptsLen: number
  /** The length of the still-encrypted payload. */
  payloadLen: number
}

/**
 * Reads a frame far enough to route it, without any key.
 *
 * A receiver holding many sessions uses this to find which one a frame belongs
 * to: the device address travels in the clear, so it can be read before the
 * session that would verify the frame is even known.
 *
 * Nothing this reports is authenticated. Treat it as a routing hint until
 * {@link Session.decode} has verified the frame.
 *
 * @param bytes - The raw frame as it came off the radio.
 * @returns What the header says the frame is.
 * @throws If the frame is truncated or carries a message type this build does not
 *   read.
 */
export function parseHeader(bytes: Uint8Array): Header {
  const header = lorawanParseHeader(Buffer.from(bytes))
  return {
    ...header,
    devAddr: header.devAddr ?? null,
    fcnt: header.fcnt ?? null,
    fport: header.fport ?? null,
  }
}

/** A join-request a device broadcast, with its integrity already verified. */
export interface JoinRequest {
  /** The device identifier, most-significant byte first. */
  devEui: Buffer
  /** The application identifier, most-significant byte first. */
  appEui: Buffer
  /** The nonce the request carried, which a network must not accept twice. */
  devNonce: number
}

/** What a network grants a device that joined. */
export interface Grant {
  /** A nonce this network must not reuse for the device; low 24 bits only. */
  appNonce: number
  /** The network identifier; low 24 bits only. */
  netId: number
  /** The address to assign the device. */
  devAddr: number
  /** The downlink settings byte, defaulting to 0. */
  dlSettings?: number
  /** The delay before the first receive window in seconds, defaulting to 0. */
  rxDelay?: number
  /** The optional channel list, as a {@link CfList} or its 16 bytes. */
  cflist?: Uint8Array | CfList
}

/**
 * Verifies a join-request and reads the identifiers out of it.
 *
 * This is the network side of activation: it proves the request came from a
 * holder of the application key before reporting who sent it.
 *
 * @param bytes - The raw join-request as it came off the radio.
 * @param appKey - The 16-byte application root key the device shares.
 * @returns The verified request.
 * @throws If the MIC does not verify or the frame is not a join-request.
 */
export function parseJoinRequest(bytes: Uint8Array, appKey: Uint8Array): JoinRequest {
  return lorawanParseJoinRequest(Buffer.from(bytes), Buffer.from(appKey))
}

/**
 * Builds the signed join-accept a network sends to admit a device.
 *
 * @param grant - The address and settings to grant.
 * @param appKey - The 16-byte application root key the device shares.
 * @param devNonce - The nonce the matching join-request carried.
 * @returns The join-accept to transmit.
 * @throws If the key or the channel list is the wrong length.
 */
export function grantAccept(
  grant: Grant,
  appKey: Uint8Array,
  devNonce: number,
): Buffer {
  return lorawanGrantAccept(nativeGrant(grant), Buffer.from(appKey), devNonce)
}

/**
 * Derives the session a grant activates, the same one the device computes.
 *
 * Neither side sends a key: both derive it from the nonces the join exchange
 * carried, so the network can read what the device it just admitted sends.
 *
 * @param grant - The address and settings granted.
 * @param appKey - The 16-byte application root key the device shares.
 * @param devNonce - The nonce the matching join-request carried.
 * @returns The session to secure this device's traffic with.
 * @throws If the key or the channel list is the wrong length.
 */
export function grantSession(
  grant: Grant,
  appKey: Uint8Array,
  devNonce: number,
): Session {
  return new Session(
    lorawanGrantSession(nativeGrant(grant), Buffer.from(appKey), devNonce),
  )
}

/** Renders a grant in the shape the generated binding takes. */
function nativeGrant(grant: Grant): LorawanGrant {
  return {
    appNonce: grant.appNonce,
    netId: grant.netId,
    devAddr: grant.devAddr,
    dlSettings: grant.dlSettings,
    rxDelay: grant.rxDelay,
    cflist:
      grant.cflist === undefined
        ? undefined
        : grant.cflist instanceof CfList
          ? grant.cflist.bytes
          : Buffer.from(grant.cflist),
  }
}

/**
 * One of the commands a network and a device configure each other with.
 *
 * `kind` names the command and `cid` is the identifier it travels under. Only the
 * fields that command carries are set; the rest are absent. The same identifier
 * means a different command in each direction, so `direction` decides which one
 * this is.
 */
export type MacCommand = LorawanMacCommand

/**
 * Reads the commands packed into a frame options field, or a payload sent on port 0.
 *
 * The same identifier means a different command in each direction, so the
 * direction decides what is read and there is no default: `0x03` going down is a
 * request to change data rate, and the same byte coming up answers one.
 *
 * A command does not carry its own length, so a reader that meets one this build
 * does not know cannot step over it. Reading stops there and returns what came
 * before, rather than guessing.
 *
 * @param direction - Which way the frame carrying them travels.
 * @param bytes - The options field, or the payload from port 0.
 * @returns The commands that were readable, in order.
 */
export function macParse(direction: Direction, bytes: Uint8Array): MacCommand[] {
  return lorawanMacParse(direction, Buffer.from(bytes))
}

/**
 * Writes one command out.
 *
 * @param command - The command; its `cid` and `direction` decide which fields are
 *   read.
 * @returns The bytes it goes out as.
 * @throws If the identifier and direction name no command, or a field will not fit
 *   what carries it, such as a frequency above 1.67 GHz.
 */
export function macEncode(command: MacCommand): Buffer {
  return lorawanMacEncode(command)
}

/** Why an end device could not do what it was asked, as a thrown error's `code`. */
export type DeviceErrorCode =
  | 'TooManyChannels'
  | 'NoCredentials'
  | 'NotJoined'
  | 'Busy'
  | 'NothingPending'
  | 'Wait'
  | 'NoChannel'
  | 'DataRate'
  | 'PayloadTooLong'
  | 'CounterExhausted'
  | 'Frame'
  | 'Foreign'
  | 'Replayed'
  | 'CounterGap'
  | 'Refused'
  | 'State'

/** An error an end device threw, with what goes with its code. */
export interface DeviceError extends Error {
  /** Why the call failed. */
  code: DeviceErrorCode
  /** For `Wait`, the earliest time to try again, in microseconds. */
  untilUs?: number
  /** For `PayloadTooLong`, the most the frame carries; for `TooManyChannels`, the most a device keeps. */
  max?: number
  /** For `DataRate`, the data rate. */
  dataRate?: number
  /** For `State`, why the saved state was not resumed. */
  state?: 'Length' | 'Corrupt' | 'Format' | 'Plan'
  /** For a `Format` state, the format it was saved in. */
  format?: number
}

/**
 * Reports whether a thrown value is an end device's error, and optionally which one.
 *
 * @param error - What was caught.
 * @param code - The code to match, or any when omitted.
 * @returns Whether it is that error.
 *
 * @example
 * ```ts
 * try {
 *   device.send(2, reading, nowUs)
 * } catch (error) {
 *   if (isDeviceError(error, 'Wait')) sleepUntil(error.untilUs)
 *   else throw error
 * }
 * ```
 */
export function isDeviceError(error: unknown, code?: DeviceErrorCode): error is DeviceError {
  return (
    error instanceof Error &&
    typeof (error as DeviceError).code === 'string' &&
    (code === undefined || (error as DeviceError).code === code)
  )
}

/** A downlink, read and acted on. */
export interface Delivery {
  /** The application port, or `null` for a frame that carried only MAC commands or nothing. */
  port: number | null
  /** The application payload, decrypted. */
  payload: Buffer
  /** Whether the network acknowledged the confirmed uplink this answered. */
  acknowledged: boolean
  /** Whether the network asked for this downlink to be acknowledged; the next uplink does. */
  confirmed: boolean
  /** Whether the network has more waiting. */
  morePending: boolean
  /** The answer to a link check the device asked for, or `null`. */
  linkCheck: { marginDb: number; gateways: number } | null
  /** The answer to a time request the device asked for, or `null`. */
  deviceTime: { gpsSeconds: number; fraction: number } | null
}

/** What a frame heard in a receive window turned out to be. */
export type Heard =
  | { kind: 'Joined'; devAddr: number }
  | { kind: 'Data'; devAddr: number; delivery: Delivery }

/** What to do once both receive windows closed with nothing for the device. */
export type Next =
  | { kind: 'Repeat'; notBeforeUs: number }
  | { kind: 'Done' }
  | { kind: 'Unacknowledged' }
  | { kind: 'JoinAgain'; notBeforeUs: number }

/**
 * Reads what a frame heard in a receive window turned out to be.
 *
 * @param heard - What the generated binding returned.
 * @returns The join, or the downlink read and acted on.
 */
function heardOut(heard: LorawanHeard): Heard {
  if (heard.kind === 'Joined' || heard.delivery == null) {
    return { kind: 'Joined', devAddr: heard.devAddr }
  }
  const delivery: LorawanDelivery = heard.delivery
  return {
    kind: 'Data',
    devAddr: heard.devAddr,
    delivery: {
      port: delivery.port ?? null,
      payload: delivery.payload,
      acknowledged: delivery.acknowledged,
      confirmed: delivery.confirmed,
      morePending: delivery.morePending,
      linkCheck: delivery.linkCheck ?? null,
      deviceTime: delivery.deviceTime ?? null,
    },
  }
}

/**
 * Reads what to do once both receive windows closed with nothing.
 *
 * @param next - What the generated binding returned.
 * @param nowUs - The time the second window closed, in microseconds.
 * @returns Whether to repeat the frame, join again, or move on.
 */
function nextOut(next: LorawanNext, nowUs: number): Next {
  switch (next.kind) {
    case 'Repeat':
      return { kind: 'Repeat', notBeforeUs: next.notBeforeUs ?? nowUs }
    case 'JoinAgain':
      return { kind: 'JoinAgain', notBeforeUs: next.notBeforeUs ?? nowUs }
    case 'Done':
      return { kind: 'Done' }
    default:
      return { kind: 'Unacknowledged' }
  }
}

/**
 * A LoRaWAN Class A end device, without a radio.
 *
 * It joins, chooses a channel and data rate for each uplink, says when and where to listen
 * for the answer, reads what comes back, and does what the network's MAC commands ask. It
 * owns no radio and no clock: every call takes the time in microseconds and hands back what
 * to put on the air, so the same device runs over any radio, or in a test with none.
 *
 * One exchange runs like this. {@link EndDevice.join} or {@link EndDevice.send} returns a
 * transmission and two receive windows timed from its end. A frame heard in either window
 * goes to {@link EndDevice.heard}. If neither held one, {@link EndDevice.nothingHeard} says
 * whether to {@link EndDevice.repeat} the frame, or move on.
 *
 * A call that cannot be done throws a {@link DeviceError}; {@link isDeviceError} tells which.
 *
 * @example
 * ```ts
 * const device = EndDevice.overTheAir(planFor(LoraRegion.Eu868), devEui, joinEui, appKey, {
 *   minOutputDbm: 2,
 *   maxOutputDbm: 14,
 * })
 * const request = device.join(nonce, clock.nowUs())
 * ```
 */
export class EndDevice {
  readonly #inner: LorawanEndDevice

  /**
   * Wraps a generated end device.
   *
   * @param inner - The generated device this facade delegates to.
   */
  constructor(inner: LorawanEndDevice) {
    this.#inner = inner
  }

  /**
   * Makes a device that joins over the air.
   *
   * @param plan - A published channel plan, from `planFor` or `cn470Plan`.
   * @param devEui - The 8-byte device EUI.
   * @param joinEui - The 8-byte join EUI.
   * @param appKey - The 16-byte root key.
   * @param settings - What the radio can do; only the output power range is required.
   * @param counters - Frame counters carried over a restart; a join starts them over.
   * @returns The device, not yet joined.
   * @throws If the plan was built rather than published, a credential is the wrong length,
   *   or the settings run backward.
   */
  static overTheAir(
    plan: LoraChannelPlan,
    devEui: Uint8Array,
    joinEui: Uint8Array,
    appKey: Uint8Array,
    settings: LorawanDeviceSettings,
    counters?: LorawanFrameCounters,
  ): EndDevice {
    const credentials = new LorawanDevice(Buffer.from(devEui), Buffer.from(joinEui), Buffer.from(appKey))
    return new EndDevice(LorawanEndDevice.overTheAir(plan, credentials, settings, counters))
  }

  /**
   * Makes a device activated by personalization.
   *
   * Such a device never resets its frame counters, so one that lost power passes the ones it
   * kept.
   *
   * @param plan - A published channel plan.
   * @param devAddr - The address the device was provisioned with.
   * @param nwkSKey - The 16-byte network session key.
   * @param appSKey - The 16-byte application session key.
   * @param settings - What the radio can do.
   * @param counters - The frame counters the device kept.
   * @returns The device, ready to send at its slowest data rate.
   * @throws If the plan was built rather than published, or a key is the wrong length.
   */
  static personalized(
    plan: LoraChannelPlan,
    devAddr: number,
    nwkSKey: Uint8Array,
    appSKey: Uint8Array,
    settings: LorawanDeviceSettings,
    counters?: LorawanFrameCounters,
  ): EndDevice {
    const session = new LorawanSession(devAddr, Buffer.from(nwkSKey), Buffer.from(appSKey))
    return new EndDevice(LorawanEndDevice.personalized(plan, session, settings, counters))
  }

  /**
   * Builds a join request.
   *
   * @param devNonce - A nonce this device has never used with its join EUI.
   * @param nowUs - The time, in microseconds.
   * @returns What to transmit, with the join accept windows.
   * @throws A {@link DeviceError}: `NoCredentials`, `Busy` or `Wait`.
   */
  join(devNonce: number, nowUs: number): LorawanTransmission {
    return this.#inner.join(devNonce, nowUs)
  }

  /**
   * Builds an uplink carrying a payload.
   *
   * @param port - The application port, 1 to 223, or 224 for the certification test port.
   * @param payload - The payload, as bytes or text encoded as UTF-8.
   * @param nowUs - The time, in microseconds.
   * @param confirmed - Whether to ask the network to acknowledge it.
   * @returns What to transmit, with its receive windows.
   * @throws A {@link DeviceError}: `NotJoined`, `Busy`, `Wait`, `PayloadTooLong`, or `Frame`
   *   for port 0.
   */
  send(port: number, payload: Uint8Array | string, nowUs: number, confirmed = false): LorawanTransmission {
    return this.#inner.send(port, Buffer.from(payload), confirmed, nowUs)
  }

  /**
   * Builds an uplink with no payload, carrying the answers the device owes, an
   * acknowledgment, or an ADR acknowledgment request.
   *
   * @param nowUs - The time, in microseconds.
   * @returns What to transmit, with its receive windows.
   * @throws A {@link DeviceError}, as {@link EndDevice.send}.
   */
  sendEmpty(nowUs: number): LorawanTransmission {
    return this.#inner.sendEmpty(nowUs)
  }

  /**
   * Sends the last uplink again, the same frame on a channel chosen afresh.
   *
   * @param nowUs - The time, in microseconds.
   * @returns What to transmit, with its receive windows.
   * @throws A {@link DeviceError}: `NothingPending` or `Wait`.
   */
  repeat(nowUs: number): LorawanTransmission {
    return this.#inner.repeat(nowUs)
  }

  /**
   * Reads a frame heard in one of the receive windows of the last transmission.
   *
   * TS001-1.0.4 section 4.1 has a device discard a frame whose MACPayload is longer than the
   * data rate it was received at carries. Name the window and the frame is held to that
   * window's limit; leave it out and it may be as long as the faster window allows.
   *
   * @param frame - The bytes the radio received.
   * @param snrDb - The frame's signal-to-noise ratio, which a `DevStatusAns` reports.
   * @param window - The window the radio heard it in, when known.
   * @returns The join, or the downlink read and acted on.
   * @throws A {@link DeviceError}: `NothingPending`, `Foreign`, `Replayed`, `CounterGap`,
   *   `Refused` or `Frame`. The windows stay open, so the second still listens.
   */
  heard(frame: Uint8Array, snrDb: number, window?: ReceiveWindow): Heard {
    return heardOut(this.#inner.heard(Buffer.from(frame), snrDb, window))
  }

  /**
   * Says what comes next once both receive windows closed with nothing for the device.
   *
   * @param nowUs - The time the second window closed, in microseconds.
   * @returns Whether to repeat the frame, join again, or move on.
   * @throws A {@link DeviceError}: `NothingPending`.
   */
  nothingHeard(nowUs: number): Next {
    return nextOut(this.#inner.nothingHeard(nowUs), nowUs)
  }

  /**
   * Sets what the device reports its battery as when a network asks.
   *
   * @param battery - A level from 1, empty, to 254, full; `'external'` on external power;
   *   or `'unknown'` or `null` when it cannot tell.
   */
  setBattery(battery: number | 'external' | 'unknown' | null): void {
    this.#inner.setBattery(battery)
  }

  /** Asks the network, with the next uplink, how well it hears the device. */
  requestLinkCheck(): void {
    this.#inner.requestLinkCheck()
  }

  /** Asks the network, with the next uplink, for the time. */
  requestDeviceTime(): void {
    this.#inner.requestDeviceTime()
  }

  /** Whether the device is on a network: once joined, or from the start when personalized. */
  get isJoined(): boolean {
    return this.#inner.isJoined
  }

  /** The address the device is on the network by, or `null` before joining. */
  get devAddr(): number | null {
    return this.#inner.devAddr ?? null
  }

  /** The data rate the next uplink goes out at, before any back-off step. */
  get dataRate(): number {
    return this.#inner.dataRate
  }

  /** The next uplink frame counter, which a device stores to carry over a restart. */
  get fcntUp(): number {
    return this.#inner.fcntUp
  }

  /** The last downlink frame counter accepted, or `null` before any downlink. */
  get fcntDown(): number | null {
    return this.#inner.fcntDown ?? null
  }

  /** How many times each uplink goes out, as the network last set it. */
  get transmissions(): number {
    return this.#inner.transmissions
  }

  /** Where the second receive window listens. */
  get rx2(): LorawanRx2 {
    return this.#inner.rx2
  }

  /** The delay from the end of an uplink to the first receive window, in microseconds. */
  get receiveDelayUs(): number {
    return this.#inner.receiveDelayUs
  }

  /** The lowest and highest frequency the device transmits or listens on. */
  get frequencySpan(): LorawanFrequencySpan {
    return this.#inner.frequencySpan
  }

  /**
   * Lists the channels the device may send on.
   *
   * @returns Each enabled channel, with its index.
   */
  channels(): LorawanChannel[] {
    return this.#inner.channels()
  }

  /**
   * Saves a joined device's state, to keep across a loss of power.
   *
   * The bytes hold the session keys, so keep them wherever the keys would be safe.
   *
   * @param nowUs - The time, in microseconds.
   * @returns The state.
   * @throws A {@link DeviceError}: `NotJoined` or `Busy`.
   */
  save(nowUs: number): Buffer {
    return this.#inner.save(nowUs)
  }

  /**
   * Puts a saved state back on a device made the same way.
   *
   * @param saved - The state {@link EndDevice.save} returned.
   * @param nowUs - The time on the clock the device woke to, in microseconds.
   * @throws A {@link DeviceError}: `State` for a state of the wrong length, a corrupt one, one
   *   in another format, or one from another plan, and `Busy`.
   */
  resume(saved: Uint8Array, nowUs: number): void {
    this.#inner.resume(Buffer.from(saved), nowUs)
  }

  /**
   * Turns relay mode on or off, TS011-1.0.1 section 10.2 and appendix 5.
   *
   * From here on the decision is the caller's rather than the device's own policy, until
   * the network takes it over with an `EndDeviceConfReq` or hands it back.
   *
   * @param on - Whether the next uplinks go through a relay.
   * @returns `false` when the network holds the decision, leaving the mode as it was.
   */
  useRelay(on: boolean): boolean {
    return this.#inner.useRelay(on)
  }

  /**
   * Reads the acknowledgment a relay answered the last WOR frame with.
   *
   * The device is now synchronized: it knows when the relay scans, so its next frames carry
   * only as much preamble as the two clocks could have drifted apart.
   *
   * @param frame - The seven bytes the radio received in the acknowledgment window.
   * @returns What the relay said about itself.
   * @throws A {@link DeviceError}: `NothingPending` with no exchange waiting, or `Frame` for
   *   an acknowledgment that does not verify.
   */
  heardWorAck(frame: Uint8Array): LorawanRelayStatus {
    return this.#inner.heardWorAck(Buffer.from(frame))
  }

  /**
   * Says what to do once the acknowledgment window closed with nothing in it.
   *
   * @param nowUs - The time the window closed, in microseconds.
   * @returns Whether to send the uplink at the time the exchange named anyway, or wake the
   *   relay again first, as the network's back-off asks.
   * @throws A {@link DeviceError}: `NothingPending`.
   */
  noWorAck(nowUs: number): WorNext {
    const next: LorawanWorNext = this.#inner.noWorAck(nowUs)
    return next.wakeUp == null ? { kind: 'Uplink' } : { kind: 'WakeUp', exchange: next.wakeUp }
  }

  /** Whether the next uplink goes through a relay. */
  get relaying(): boolean {
    return this.#inner.relaying
  }

  /** How the device decides whether to use a relay. */
  get relayActivation(): RelayActivation {
    return this.#inner.relayActivation
  }

  /** What the device knows of when its relay listens, TS011-1.0.1 section 3.9. */
  get relaySync(): RelaySync {
    return this.#inner.relaySync
  }

  /** What the relay's last acknowledgment said about itself, or `null` before one arrived. */
  get relayStatus(): LorawanRelayStatus | null {
    return this.#inner.relayStatus ?? null
  }

  /** The WOR frame counter the next frame will use, TS011-1.0.1 section 5.3.2. */
  get worCounter(): number {
    return this.#inner.worCounter
  }
}

/** The port every message between a relay and its network uses, TS011-1.0.1 section 9. */
export const LA_FPORT_RELAY = LORAWAN_LA_FPORT_RELAY

/** How many end devices a relay verifies wake-on-radio frames for, RP002-1.0.5 section 5.4.5. */
export const TRUSTED_ED_NUMBER = LORAWAN_TRUSTED_ED_NUMBER

/** How many WOR frames go without an acknowledgment before the uplink goes anyway. */
export const WOR_ATTEMPTS_WO_ACK = LORAWAN_WOR_ATTEMPTS_WO_ACK

/** The gap between a WOR frame, or its acknowledgment, and the LoRaWAN frame after it. */
export const WOR_DATA_DELAY_US = LORAWAN_WOR_DATA_DELAY_US

/** The gap between a WOR frame and its acknowledgment. */
export const WOR_ACK_DELAY_US = LORAWAN_WOR_ACK_DELAY_US

/** The gap between a relay hearing an uplink and forwarding it. */
export const RELAY_FWD_DELAY_US = LORAWAN_RELAY_FWD_DELAY_US

/** How long after an uplink an end device's RXR window opens at the latest. */
export const RXR_DELAY_US = LORAWAN_RXR_DELAY_US

/** The bytes a forwarded uplink adds in front of the end device's frame. */
export const FORWARD_OVERHEAD = LORAWAN_FORWARD_OVERHEAD

/** The shortest WOR preamble, in symbols. */
export const MIN_WOR_PREAMBLE_SYMBOLS = LORAWAN_MIN_WOR_PREAMBLE_SYMBOLS

/** Which WOR frame a relay heard. */
export const WorKind = {
  /** Ahead of a join request, which nothing protects. */
  JoinRequest: 'JoinRequest' as WorKindName,
  /** Ahead of a Class A uplink, sealed with the device's WOR keys. */
  Uplink: 'Uplink' as WorKindName,
} as const

/** One of the {@link WorKind} values. */
export type WorKind = WorKindName

/** How often a relay scans a channel for a WOR preamble, TS011-1.0.1 table 18. */
export const CadPeriodicity = {
  /** Once a second, the default. */
  Ms1000: 'Ms1000' as CadPeriodicityName,
  /** Every 500 milliseconds. */
  Ms500: 'Ms500' as CadPeriodicityName,
  /** Every 250 milliseconds. */
  Ms250: 'Ms250' as CadPeriodicityName,
  /** Every 100 milliseconds. */
  Ms100: 'Ms100' as CadPeriodicityName,
  /** Every 50 milliseconds. */
  Ms50: 'Ms50' as CadPeriodicityName,
  /** Every 20 milliseconds. */
  Ms20: 'Ms20' as CadPeriodicityName,
} as const

/** One of the {@link CadPeriodicity} values. */
export type CadPeriodicity = CadPeriodicityName

/** How many symbols a relay takes from detecting activity to receiving, table 15. */
export const CadToRx = {
  /** Two symbols. */
  Symbols2: 'Symbols2' as CadToRxName,
  /** Four symbols. */
  Symbols4: 'Symbols4' as CadToRxName,
  /** Six symbols. */
  Symbols6: 'Symbols6' as CadToRxName,
  /** Eight symbols, which a device assumes before it has heard from a relay. */
  Symbols8: 'Symbols8' as CadToRxName,
} as const

/** One of the {@link CadToRx} values. */
export type CadToRx = CadToRxName

/** How accurate a relay's crystal is, table 17. */
export const XtalAccuracy = {
  /** Better than 10 parts per million. */
  Ppm10: 'Ppm10' as XtalAccuracyName,
  /** Better than 20. */
  Ppm20: 'Ppm20' as XtalAccuracyName,
  /** Better than 30. */
  Ppm30: 'Ppm30' as XtalAccuracyName,
  /** Better than 40, which a device assumes before it has heard from a relay. */
  Ppm40: 'Ppm40' as XtalAccuracyName,
} as const

/** One of the {@link XtalAccuracy} values. */
export type XtalAccuracy = XtalAccuracyName

/** Whether a relay will forward the uplink after a WOR frame, table 16. */
export const RelayForward = {
  /** It has room to. */
  Available: 'Available' as RelayForwardName,
  /** A forwarding limit is reached; try again in 30 minutes. */
  RetryIn30Minutes: 'RetryIn30Minutes' as RelayForwardName,
  /** A forwarding limit is reached; try again in 60 minutes. */
  RetryIn60Minutes: 'RetryIn60Minutes' as RelayForwardName,
  /** Forwarding is off. */
  Disabled: 'Disabled' as RelayForwardName,
} as const

/** One of the {@link RelayForward} values. */
export type RelayForward = RelayForwardName

/** Which of a relay's channels a WOR frame arrived on, table 28. */
export const WorChannel = {
  /** The default channel. */
  Default: 'Default' as WorChannelName,
  /** The second channel a network configured. */
  Second: 'Second' as WorChannelName,
} as const

/** One of the {@link WorChannel} values. */
export type WorChannel = WorChannelName

/**
 * A LoRaWAN relay, TS011-1.0.1: what an end device and a relay say to each other.
 *
 * A relay sleeps, waking every scan period to look for radio activity. An end device out of
 * a gateway's reach first sends a wake-on-radio (WOR) frame whose preamble spans that sleep
 * and which says where its uplink follows. The relay may acknowledge with its own timing,
 * so the next preamble can be short, then forwards the uplink on port {@link LA_FPORT_RELAY}.
 *
 * @example
 * ```ts
 * const keys = session.worKeys()
 * const wor = { frequencyHz: 865_100_000, dataRate: 3 }
 * const uplink = { frequencyHz: 868_100_000, dataRate: 5 }
 * const frame = relay.worUplink(keys, session.devAddr, 1, uplink, wor)
 * relay.openWor(frame, keys, 1, wor) // { frequencyHz: 868100000, dataRate: 5 }
 * ```
 */
export const relay = {
  /**
   * Derives an end device's root relay session key from its network session key, section 4.4.
   *
   * @param networkKey - The 16-byte network session key.
   * @returns The 16-byte root key.
   */
  rootWorSKey(networkKey: Uint8Array): Buffer {
    return lorawanRelayRootWorSKey(Buffer.from(networkKey))
  },

  /**
   * Derives an end device's WOR keys from its root relay session key, section 4.5.
   *
   * @param rootKey - The 16-byte root key.
   * @param devAddr - The device's address.
   * @returns The integrity and encryption keys.
   */
  worKeys(rootKey: Uint8Array, devAddr: number): LorawanWorKeys {
    return lorawanRelayWorKeys(Buffer.from(rootKey), devAddr)
  },

  /**
   * Builds the WOR frame ahead of a join request, section 5.3.1.
   *
   * @param uplink - Where and how fast the join request follows.
   * @returns The five-byte frame.
   * @throws For a data rate past 15 or a frequency the field cannot carry.
   */
  worJoinRequest(uplink: LorawanCarrier): Buffer {
    return lorawanRelayWorJoinRequest(uplink)
  },

  /**
   * Builds the WOR frame ahead of a Class A uplink, section 5.3.2.
   *
   * @param keys - The device's WOR keys.
   * @param devAddr - Its address.
   * @param wfcnt - The WOR frame counter, raised for every WOR frame.
   * @param uplink - Where and how fast the uplink follows.
   * @param wor - The carrier this WOR frame goes out on.
   * @returns The fifteen-byte frame.
   * @throws For a carrier the fields cannot carry.
   */
  worUplink(
    keys: LorawanWorKeys,
    devAddr: number,
    wfcnt: number,
    uplink: LorawanCarrier,
    wor: LorawanCarrier,
  ): Buffer {
    return lorawanRelayWorUplink(keys, devAddr, wfcnt, uplink, wor)
  },

  /**
   * Reads a WOR frame, leaving an uplink's carrier sealed.
   *
   * @param frame - The bytes a relay received.
   * @returns The frame's kind, and a join request's carrier or an uplink's address and
   *   counter.
   * @throws For a reserved or proprietary type, or a length that is not the type's.
   */
  parseWor(frame: Uint8Array): LorawanWor {
    return lorawanRelayWorParse(Buffer.from(frame))
  },

  /**
   * Checks a WOR frame ahead of a Class A uplink and reads where the uplink follows.
   *
   * @param frame - The frame a relay received.
   * @param keys - The keys of the device it names.
   * @param wfcnt - The full 32-bit counter the relay takes it to carry.
   * @param wor - The carrier it arrived on.
   * @returns Where and how fast the uplink follows.
   * @throws When the frame is not an uplink WOR, or its integrity code does not verify.
   */
  openWor(
    frame: Uint8Array,
    keys: LorawanWorKeys,
    wfcnt: number,
    wor: LorawanCarrier,
  ): LorawanCarrier {
    return lorawanRelayWorOpen(Buffer.from(frame), keys, wfcnt, wor)
  },

  /**
   * Builds a relay's WOR ACK, section 6.2.
   *
   * @param keys - The end device's WOR keys.
   * @param devAddr - Its address.
   * @param wfcnt - The counter of the acknowledged WOR frame.
   * @param ack - The carrier the acknowledgment goes out on.
   * @param uplink - The carrier the WOR frame named for the uplink.
   * @param state - What the relay tells the device.
   * @returns The seven-byte acknowledgment.
   * @throws For a state or carrier the fields cannot carry.
   */
  worAck(
    keys: LorawanWorKeys,
    devAddr: number,
    wfcnt: number,
    ack: LorawanCarrier,
    uplink: LorawanCarrier,
    state: LorawanStateSync,
  ): Buffer {
    return lorawanRelayWorAck(keys, devAddr, wfcnt, ack, uplink, state)
  },

  /**
   * Checks and reads a WOR ACK, section 6.2.
   *
   * @param frame - The acknowledgment an end device received.
   * @param keys - The device's WOR keys.
   * @param devAddr - Its address.
   * @param wfcnt - The counter of the WOR frame it sent.
   * @param ack - The carrier the acknowledgment arrived on.
   * @param uplink - The carrier the WOR frame named.
   * @returns What the relay said about itself.
   * @throws When the integrity code does not verify, or the periodicity is reserved.
   */
  openWorAck(
    frame: Uint8Array,
    keys: LorawanWorKeys,
    devAddr: number,
    wfcnt: number,
    ack: LorawanCarrier,
    uplink: LorawanCarrier,
  ): LorawanStateSync {
    return lorawanRelayWorAckOpen(Buffer.from(frame), keys, devAddr, wfcnt, ack, uplink)
  },

  /**
   * Writes an uplink a relay forwards on port 226, section 9.1. A strength or ratio past what
   * its field carries goes out as the closest value.
   *
   * @param forwarded - The metadata, the frequency, and the end device's frame.
   * @returns The relay uplink's payload.
   * @throws For a data rate or frequency the fields cannot carry.
   */
  encodeForward(forwarded: LorawanForwardedUplink): Buffer {
    return lorawanRelayForwardEncode({
      ...forwarded,
      phyPayload: Buffer.from(forwarded.phyPayload),
    })
  },

  /**
   * Reads an uplink a relay forwarded, section 9.1.
   *
   * @param payload - The relay uplink's payload on port 226.
   * @returns The metadata, the frequency, and the end device's frame.
   * @throws For a payload too short or a reserved WOR channel.
   */
  parseForward(payload: Uint8Array): LorawanForwardedUplink {
    return lorawanRelayForwardParse(Buffer.from(payload))
  },

  /**
   * The WOR preamble of an end device that does not know when the relay scans, section 5.2.
   *
   * @param cadPeriodicity - How often the relay scans.
   * @param symbolUs - The symbol time of the WOR frame's data rate, in microseconds.
   * @param cadToRx - The relay's time to start receiving.
   * @returns The preamble length in symbols.
   */
  unsynchronizedPreamble(
    cadPeriodicity: CadPeriodicity,
    symbolUs: number,
    cadToRx: CadToRx,
  ): number {
    return lorawanRelayUnsynchronizedPreamble(cadPeriodicity, symbolUs, cadToRx)
  },

  /**
   * The offset a relay reports in a WOR ACK, appendix 1.
   *
   * @param scanStartUs - When the scan that detected the frame started.
   * @param preambleEndUs - When the frame's preamble ended: when it finished arriving,
   *   less the airtime of its sync word and payload.
   * @returns The offset in milliseconds, or null when the preamble ended before the scan
   *   or more than eleven bits of milliseconds after it.
   */
  tOffsetMs(scanStartUs: number, preambleEndUs: number): number | null {
    return lorawanRelayTOffsetMs(scanStartUs, preambleEndUs) ?? null
  },

  /**
   * Works out when a relay scanned from the WOR ACK that answered a frame, appendix 1.
   *
   * @param worStartUs - When the acknowledged WOR frame started going out.
   * @param preambleSymbols - Its preamble length.
   * @param symbolUs - Its symbol time.
   * @param state - What the acknowledgment said.
   * @returns The synchronization.
   */
  synchronization(
    worStartUs: number,
    preambleSymbols: number,
    symbolUs: number,
    state: LorawanStateSync,
  ): LorawanSynchronization {
    return lorawanRelaySynchronization(worStartUs, preambleSymbols, symbolUs, state)
  },

  /**
   * Picks the relay scan a synchronized end device aims its next WOR frame at, appendix 1.
   *
   * @param synchronization - What the device knows of the relay.
   * @param nowUs - The time.
   * @param deviceXtalPpm - The device's crystal accuracy.
   * @param symbolUs - The symbol time of the WOR frame's data rate.
   * @param otherChannel - Whether the frame goes out on the relay's other channel.
   * @returns When to send and how long a preamble, or null once the drift exceeds a period.
   */
  nextWor(
    synchronization: LorawanSynchronization,
    nowUs: number,
    deviceXtalPpm: number,
    symbolUs: number,
    otherChannel = false,
  ): LorawanWorSlot | null {
    return (
      lorawanRelayNextWor(synchronization, nowUs, deviceXtalPpm, symbolUs, otherChannel) ?? null
    )
  },

  /**
   * Reads the second channel a relay or end device configuration describes.
   *
   * @param secondChannelIndex - The coded index, 1 for a second channel.
   * @param dataRate - Its data rate.
   * @param ackOffset - The coded acknowledgment offset, table 35.
   * @param frequencyHz - Its frequency.
   * @returns The channel with its acknowledgment frequency, or null when none is named.
   */
  secondChannel(
    secondChannelIndex: number,
    dataRate: number,
    ackOffset: number,
    frequencyHz: number,
  ): LoraRelayChannel | null {
    return lorawanRelaySecondChannel(secondChannelIndex, dataRate, ackOffset, frequencyHz) ?? null
  },
} as const

/** How an end device decides whether to send through a relay, TS011-1.0.1 section 10.2. */
export const RelayActivation = {
  /** Never, the default. */
  Disabled: 'Disabled' as RelayActivationName,
  /** Always. */
  Enabled: 'Enabled' as RelayActivationName,
  /** Only once a run of uplinks went unanswered, as the network's smart-enable level sets. */
  Dynamic: 'Dynamic' as RelayActivationName,
  /** However the device itself decides, which {@link EndDevice.useRelay} sets. */
  DeviceControlled: 'DeviceControlled' as RelayActivationName,
} as const

/** One of the {@link RelayActivation} values. */
export type RelayActivation = RelayActivationName

/** What an end device knows of its relay's scans, TS011-1.0.1 section 3.9. */
export const RelaySync = {
  /** Nothing: no WOR frame has gone out yet. */
  Initialized: 'Initialized' as RelaySyncName,
  /** One went out unanswered, so the next preamble spans a whole scan period. */
  Unsynchronized: 'Unsynchronized' as RelaySyncName,
  /** It knows when the relay scans, so a short preamble reaches it. */
  Synchronized: 'Synchronized' as RelaySyncName,
} as const

/** One of the {@link RelaySync} values. */
export type RelaySync = RelaySyncName

/** What an end device does once its WOR frame went unanswered. */
export type WorNext =
  | { kind: 'Uplink' }
  | { kind: 'WakeUp'; exchange: LorawanRelayExchange }

/** What a WOR frame led a relay to do. */
export const WakeKind = {
  /** A join request from a device the relay's filters let through. */
  JoinRequest: 'JoinRequest' as WakeKindName,
  /** An uplink from a trusted device. */
  Uplink: 'Uplink' as WakeKindName,
  /** A frame from a device the relay does not know, which it tells its network about. */
  Notified: 'Notified' as WakeKindName,
} as const

/** What a WOR frame led a relay to do. */
export type Wake =
  | { kind: 'JoinRequest'; listen: LorawanListen }
  | {
      kind: 'Uplink'
      devAddr: number
      wfcnt: number
      forward: RelayForward
      acknowledgment: LorawanAcknowledgment | null
      listen: LorawanListen | null
    }
  | { kind: 'Notified'; devAddr: number }

/** What a frame a relay's own device heard turned out to be. */
export const RelayHeardKind = {
  /** The relay's own device read it. */
  Device: 'Device' as RelayHeardKindName,
  /** A downlink for an end device, to send in its relay window. */
  Downlink: 'Downlink' as RelayHeardKindName,
  /** A downlink for an end device the relay cannot send on. */
  Undeliverable: 'Undeliverable' as RelayHeardKindName,
} as const

/** What a frame a relay's own device heard turned out to be. */
export type RelayHeard =
  | { kind: 'Device'; heard: Heard }
  | { kind: 'Downlink'; downlink: LorawanRxrDownlink }
  | { kind: 'Undeliverable'; reason: string }

/** An end device a relay forwards for, as an `UpdateUplinkListReq` describes it. */
export interface TrustedDevice {
  /** The device's address. */
  devAddr: number
  /** Its root relay session key, from {@link relay.rootWorSKey}. */
  rootWorSKey: Uint8Array
  /** The WOR frame counter to expect from it next; 0 for a device that has not sent one. */
  nextWfcnt?: number
  /** How many of its uplinks are forwarded an hour, 63 for no limit, table 55. */
  reloadRate?: number
  /** The coded bucket size multiplier, table 55. */
  bucketSize?: number
}

/**
 * A LoRaWAN relay: an end device that also carries the uplinks of the devices around it.
 *
 * A relay sleeps, waking every scan period to look for radio activity on its WOR channel.
 * One turn runs like this: {@link Relay.nextScan} says when and where to listen, a frame
 * heard there goes to {@link Relay.heardWor}, the uplink it announced to
 * {@link Relay.heardUplink}, and {@link Relay.forward} wraps that in one of the relay's own
 * uplinks on port {@link LA_FPORT_RELAY}. What the relay's own receive windows hear goes to
 * {@link Relay.heardIn}, which turns a downlink meant for an end device into one to send in
 * that device's relay window.
 *
 * It owns no radio and no clock, so the same relay runs over any radio, or in a test with
 * none. The codecs behind it are on the {@link relay} object.
 *
 * @example
 * ```ts
 * const node = Relay.personalized(planFor(LoraRegion.Eu868), devAddr, nwkSKey, appSKey, {
 *   minOutputDbm: 2,
 *   maxOutputDbm: 14,
 * })
 * node.start(CadPeriodicity.Ms1000, 0)
 * node.trust(0, { devAddr: sensorAddr, rootWorSKey: relay.rootWorSKey(sensorNwkSKey) })
 * const scan = node.nextScan(clock.nowUs())
 * ```
 */
export class Relay {
  readonly #inner: LorawanRelay

  /**
   * Wraps a generated relay.
   *
   * @param inner - The generated relay this facade delegates to.
   */
  constructor(inner: LorawanRelay) {
    this.#inner = inner
  }

  /**
   * Makes a relay whose own device is activated by personalization.
   *
   * @param plan - A published channel plan, from `planFor` or `cn470Plan`.
   * @param devAddr - The address the relay was provisioned with.
   * @param nwkSKey - Its 16-byte network session key.
   * @param appSKey - Its 16-byte application session key.
   * @param settings - What its radio can do.
   * @param xtalAccuracy - How accurate its crystal is, which its acknowledgments report.
   * @param cadToRx - How long it takes to start receiving once it detects activity.
   * @returns The relay, stopped until {@link Relay.start}.
   * @throws If the plan was built rather than published, or a key is the wrong length.
   */
  static personalized(
    plan: LoraChannelPlan,
    devAddr: number,
    nwkSKey: Uint8Array,
    appSKey: Uint8Array,
    settings: LorawanDeviceSettings,
    xtalAccuracy: XtalAccuracy = XtalAccuracy.Ppm40,
    cadToRx: CadToRx = CadToRx.Symbols8,
  ): Relay {
    const session = new LorawanSession(devAddr, Buffer.from(nwkSKey), Buffer.from(appSKey))
    return new Relay(LorawanRelay.personalized(plan, session, settings, xtalAccuracy, cadToRx))
  }

  /**
   * Makes a relay whose own device joins over the air.
   *
   * @param plan - A published channel plan.
   * @param devEui - Its 8-byte device EUI.
   * @param joinEui - Its 8-byte join EUI.
   * @param appKey - Its 16-byte root key.
   * @param settings - What its radio can do.
   * @param xtalAccuracy - How accurate its crystal is.
   * @param cadToRx - How long it takes to start receiving once it detects activity.
   * @returns The relay, neither joined nor scanning.
   * @throws If the plan was built rather than published, or a credential is the wrong length.
   */
  static overTheAir(
    plan: LoraChannelPlan,
    devEui: Uint8Array,
    joinEui: Uint8Array,
    appKey: Uint8Array,
    settings: LorawanDeviceSettings,
    xtalAccuracy: XtalAccuracy = XtalAccuracy.Ppm40,
    cadToRx: CadToRx = CadToRx.Symbols8,
  ): Relay {
    const credentials = new LorawanDevice(
      Buffer.from(devEui),
      Buffer.from(joinEui),
      Buffer.from(appKey),
    )
    return new Relay(LorawanRelay.overTheAir(plan, credentials, settings, xtalAccuracy, cadToRx))
  }

  /**
   * Starts scanning, or changes what a running relay scans from its next scan on.
   *
   * @param cadPeriodicity - How often to scan.
   * @param defaultChannelIndex - Which of the region's WOR channels is the default one.
   * @param secondChannel - A second channel the network configured, from
   *   {@link relay.secondChannel}.
   * @throws For a channel index the region does not define.
   */
  start(
    cadPeriodicity: CadPeriodicity,
    defaultChannelIndex: number,
    secondChannel?: LoraRelayChannel,
  ): void {
    this.#inner.start(cadPeriodicity, defaultChannelIndex, secondChannel)
  }

  /** Stops scanning. A forwarded uplink already waiting still goes out. */
  stop(): void {
    this.#inner.stop()
  }

  /**
   * Trusts an end device, as an `UpdateUplinkListReq` with the same fields does.
   *
   * @param index - The entry, 0 to 15; there is room for {@link TRUSTED_ED_NUMBER}.
   * @param device - The device, its key, and what it may spend.
   * @throws For an index past 15.
   */
  trust(index: number, device: TrustedDevice): void {
    this.#inner.trust(
      index,
      device.devAddr,
      Buffer.from(device.rootWorSKey),
      device.nextWfcnt ?? 0,
      device.reloadRate ?? 63,
      device.bucketSize ?? 0,
    )
  }

  /**
   * Says when and where to scan next.
   *
   * @param nowUs - The time, in microseconds.
   * @returns The scan, or `null` while the relay is stopped.
   */
  nextScan(nowUs: number): LorawanScan | null {
    return this.#inner.nextScan(nowUs) ?? null
  }

  /**
   * Reads a WOR frame a scan heard.
   *
   * @param scan - The scan that heard it, as {@link Relay.nextScan} returned it.
   * @param frame - The bytes the radio received.
   * @param rssiDbm - Its received signal strength, which the forwarded uplink carries.
   * @param snrDb - Its signal-to-noise ratio, which the forwarded uplink carries.
   * @param endedUs - When the frame ended, in microseconds.
   * @returns What the frame led to: an acknowledgment to send, an uplink to listen for, or
   *   a notification for the network.
   * @throws For a frame that does not verify, a device the relay does not forward for, or a
   *   limit already spent.
   */
  heardWor(
    scan: LorawanScan,
    frame: Uint8Array,
    rssiDbm: number,
    snrDb: number,
    endedUs: number,
  ): Wake {
    const wake: LorawanWake = this.#inner.heardWor(scan, Buffer.from(frame), rssiDbm, snrDb, endedUs)
    switch (wake.kind) {
      case 'JoinRequest':
        return { kind: 'JoinRequest', listen: wake.listen as LorawanListen }
      case 'Notified':
        return { kind: 'Notified', devAddr: wake.devAddr as number }
      default:
        return {
          kind: 'Uplink',
          devAddr: wake.devAddr as number,
          wfcnt: wake.wfcnt as number,
          forward: wake.forward as RelayForward,
          acknowledgment: wake.acknowledgment ?? null,
          listen: wake.listen ?? null,
        }
    }
  }

  /**
   * Reads the uplink a WOR frame announced, and holds it to forward.
   *
   * @param frame - The bytes the radio received.
   * @param rssiDbm - Its received signal strength.
   * @param snrDb - Its signal-to-noise ratio.
   * @param endedUs - When the frame ended, in microseconds.
   * @returns When to {@link Relay.forward} it: fifty milliseconds after it ended.
   * @throws When nothing announced it, or the frame is not the one it announced.
   */
  heardUplink(frame: Uint8Array, rssiDbm: number, snrDb: number, endedUs: number): number {
    return this.#inner.heardUplink(Buffer.from(frame), rssiDbm, snrDb, endedUs)
  }

  /** Clears the uplink a WOR frame announced, once listening heard nothing. */
  uplinkMissed(): void {
    this.#inner.uplinkMissed()
  }

  /**
   * Sends the uplink the relay is holding, in one of its own on port
   * {@link LA_FPORT_RELAY}.
   *
   * @param nowUs - The time, in microseconds.
   * @returns What to transmit, with the relay's own receive windows.
   * @throws A {@link DeviceError} when the relay's own device cannot send it yet, or when
   *   nothing is waiting.
   */
  forward(nowUs: number): LorawanTransmission {
    return this.#inner.forward(nowUs)
  }

  /**
   * Reads a frame the relay's own device heard, acting on the relay commands in it.
   *
   * @param window - The window the radio heard it in.
   * @param frame - The bytes the radio received.
   * @param snrDb - Its signal-to-noise ratio.
   * @returns What it turned out to be: the relay's own downlink, one to pass on to an end
   *   device, or one that cannot be passed on.
   * @throws A {@link DeviceError}, as {@link EndDevice.heard}.
   */
  heardIn(window: ReceiveWindow, frame: Uint8Array, snrDb: number): RelayHeard {
    const heard: LorawanRelayHeard = this.#inner.heardIn(window, Buffer.from(frame), snrDb)
    switch (heard.kind) {
      case 'Downlink':
        return { kind: 'Downlink', downlink: heard.downlink as LorawanRxrDownlink }
      case 'Undeliverable':
        return { kind: 'Undeliverable', reason: heard.reason ?? 'undeliverable' }
      default:
        return { kind: 'Device', heard: heardOut(heard.heard as LorawanHeard) }
    }
  }

  /**
   * Says what comes next once the relay's own windows closed with nothing in them.
   *
   * @param nowUs - The time the second window closed, in microseconds.
   * @returns Whether to repeat the frame, join again, or move on.
   * @throws A {@link DeviceError}: `NothingPending`.
   */
  nothingHeard(nowUs: number): Next {
    return nextOut(this.#inner.nothingHeard(nowUs), nowUs)
  }

  /**
   * Builds the relay's own join request.
   *
   * @param devNonce - A nonce this relay has never used with its join EUI.
   * @param nowUs - The time, in microseconds.
   * @returns What to transmit, with the join accept windows.
   * @throws A {@link DeviceError}: `NoCredentials`, `Busy` or `Wait`.
   */
  join(devNonce: number, nowUs: number): LorawanTransmission {
    return this.#inner.join(devNonce, nowUs)
  }

  /**
   * Sends one of the relay's own uplinks, which also carries what it owes its network.
   *
   * @param port - The application port, 1 to 223.
   * @param payload - The payload, as bytes or text encoded as UTF-8.
   * @param nowUs - The time, in microseconds.
   * @param confirmed - Whether to ask the network to acknowledge it.
   * @returns What to transmit, with its receive windows.
   * @throws A {@link DeviceError}, as {@link EndDevice.send}.
   */
  send(
    port: number,
    payload: Uint8Array | string,
    nowUs: number,
    confirmed = false,
  ): LorawanTransmission {
    return this.#inner.send(port, Buffer.from(payload), confirmed, nowUs)
  }

  /**
   * Sends an uplink with no payload, carrying whatever the relay owes its network.
   *
   * @param nowUs - The time, in microseconds.
   * @returns What to transmit, with its receive windows.
   * @throws A {@link DeviceError}, as {@link EndDevice.send}.
   */
  sendEmpty(nowUs: number): LorawanTransmission {
    return this.#inner.sendEmpty(nowUs)
  }

  /** Whether the relay is scanning. */
  get running(): boolean {
    return this.#inner.running
  }

  /** When the forwarded uplink waiting to go out is due, or `null` with nothing waiting. */
  get forwardDue(): number | null {
    return this.#inner.forwardDue ?? null
  }

  /** The address the relay's own device is on the network by, or `null` before joining. */
  get devAddr(): number | null {
    return this.#inner.devAddr ?? null
  }

  /** Whether the relay's own device is on a network. */
  get joined(): boolean {
    return this.#inner.joined
  }

  /** The data rate the relay forwards at, which its acknowledgments report. */
  get dataRate(): number {
    return this.#inner.dataRate
  }
}
