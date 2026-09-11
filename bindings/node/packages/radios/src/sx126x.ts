/**
 * The Semtech SX1261, SX1262, and LLCC68, as the bytes they take and the answers they give.
 *
 * Every command goes out in one SPI transaction framed by NSS, sent once the chip's BUSY
 * line is low, and a query's answer is read in the same transaction after its bytes. These
 * functions build the bytes and decode the answers from the SX1261/2 datasheet (Rev 2.2),
 * so a program with its own SPI access can drive the chip with nothing else.
 *
 * @module
 */

import {
  SX126X_ERROR_ADC_CALIBRATION,
  SX126X_ERROR_IMAGE_CALIBRATION,
  SX126X_ERROR_PA_RAMP,
  SX126X_ERROR_PLL_CALIBRATION,
  SX126X_ERROR_PLL_LOCK,
  SX126X_ERROR_RC13M_CALIBRATION,
  SX126X_ERROR_RC64K_CALIBRATION,
  SX126X_ERROR_XOSC_START,
  SX126X_IRQ_ALL,
  SX126X_IRQ_CAD_DETECTED,
  SX126X_IRQ_CAD_DONE,
  SX126X_IRQ_CRC_ERROR,
  SX126X_IRQ_HEADER_ERROR,
  SX126X_IRQ_HEADER_VALID,
  SX126X_IRQ_LR_FHSS_HOP,
  SX126X_IRQ_PREAMBLE_DETECTED,
  SX126X_IRQ_RX_DONE,
  SX126X_IRQ_SYNC_WORD_VALID,
  SX126X_IRQ_TIMEOUT,
  SX126X_IRQ_TX_DONE,
  SX126X_REGISTER_LORA_SYNC_WORD,
  SX126X_RX_CONTINUOUS,
  SX126X_SYNC_WORD_PRIVATE,
  SX126X_SYNC_WORD_PUBLIC,
  type LoraLink,
  type LoraLinkBudget,
  type Sx126xAmplifier as NativeSx126xAmplifier,
  type Sx126xChipMode as NativeSx126xChipMode,
  type Sx126xCommandStatus as NativeSx126xCommandStatus,
  type Sx126xPacketStatus,
  type Sx126xQuery,
  type Sx126xRxBufferStatus,
  type Sx126xStatus,
  type Sx126xTxPower,
  sx126xCalibrateImage,
  sx126xClearIrqStatus,
  sx126xDeviceErrors,
  sx126xFrequencyWord,
  sx126xGetDeviceErrors,
  sx126xGetIrqStatus,
  sx126xGetPacketStatus,
  sx126xGetRssiInst,
  sx126xGetRxBufferStatus,
  sx126xGetStatus,
  sx126xImageCalibration,
  sx126xIrq,
  sx126xLlcc68Supports,
  sx126xPacketStatus,
  sx126xRampTimeUs,
  sx126xReadBuffer,
  sx126xReadRegister,
  sx126xRssiInstDbm,
  sx126xRxBufferStatus,
  sx126xSetDioIrqParams,
  sx126xSetLoraModulationParams,
  sx126xSetLoraPacketParams,
  sx126xSetPaConfig,
  sx126xSetPacketTypeLora,
  sx126xSetRfFrequency,
  sx126xSetRx,
  sx126xSetRxContinuous,
  sx126xSetSleep,
  sx126xSetStandby,
  sx126xSetTx,
  sx126xSetTxParams,
  sx126xStatus,
  sx126xTimeoutSteps,
  sx126xTxPower,
  sx126xTxPowerUnderCeiling,
  sx126xWriteBuffer,
  sx126xWriteRegister,
} from '@pamoja/native'

/** The amplifier configuration and power setting that produce an output power. */
export type TxPower = Sx126xTxPower

/** A command the chip answers in the same SPI transaction, and how much answer to read. */
export type Query = Sx126xQuery

/** A decoded status byte: the chip's mode and how its last command went. */
export type Status = Sx126xStatus

/** The signal levels a LoRa packet was received with. */
export type PacketStatus = Sx126xPacketStatus

/** Where a received payload sits in the chip's data buffer. */
export type RxBufferStatus = Sx126xRxBufferStatus

/** The mode a status byte reports. */
export type ChipMode = NativeSx126xChipMode

/** How the last command went, as a status byte reports it. */
export type CommandStatus = NativeSx126xCommandStatus

/**
 * Which power amplifier a chip has.
 *
 * Provided as a runtime object, as the generated string enum is erased at compile time.
 * The SPI interface cannot tell the chips apart, so the caller names the amplifier.
 */
export const Amplifier = {
  /** The low power amplifier of the SX1261, up to +15 dBm. */
  LowPower: 'LowPower' as NativeSx126xAmplifier,
  /** The high power amplifier of the SX1262 and the LLCC68, up to +22 dBm. */
  HighPower: 'HighPower' as NativeSx126xAmplifier,
} as const

/** One of the {@link Amplifier} values. */
export type Amplifier = NativeSx126xAmplifier

/** The interrupt bits of the IRQ register, from Table 13-29 of the datasheet. */
export const Irq = {
  /** A packet has been sent. */
  TxDone: SX126X_IRQ_TX_DONE,
  /** A packet has been received. */
  RxDone: SX126X_IRQ_RX_DONE,
  /** A preamble has been detected. */
  PreambleDetected: SX126X_IRQ_PREAMBLE_DETECTED,
  /** A valid (G)FSK sync word has been detected. */
  SyncWordValid: SX126X_IRQ_SYNC_WORD_VALID,
  /** A valid LoRa header has been received. */
  HeaderValid: SX126X_IRQ_HEADER_VALID,
  /** A LoRa header failed its CRC. */
  HeaderError: SX126X_IRQ_HEADER_ERROR,
  /** A packet failed its CRC. */
  CrcError: SX126X_IRQ_CRC_ERROR,
  /** Channel activity detection has finished. */
  CadDone: SX126X_IRQ_CAD_DONE,
  /** Channel activity detection heard LoRa. */
  CadDetected: SX126X_IRQ_CAD_DETECTED,
  /** A transmission or a reception timed out. */
  Timeout: SX126X_IRQ_TIMEOUT,
  /** A long-range FHSS hop is due. */
  LrFhssHop: SX126X_IRQ_LR_FHSS_HOP,
  /** Every bit the chip defines. */
  All: SX126X_IRQ_ALL,
} as const

/** The device error bits GetDeviceErrors answers with. */
export const DeviceError = {
  /** The RC64k oscillator failed to calibrate. */
  Rc64kCalibration: SX126X_ERROR_RC64K_CALIBRATION,
  /** The RC13M oscillator failed to calibrate. */
  Rc13mCalibration: SX126X_ERROR_RC13M_CALIBRATION,
  /** The PLL failed to calibrate. */
  PllCalibration: SX126X_ERROR_PLL_CALIBRATION,
  /** The ADC failed to calibrate. */
  AdcCalibration: SX126X_ERROR_ADC_CALIBRATION,
  /** Image rejection failed to calibrate. */
  ImageCalibration: SX126X_ERROR_IMAGE_CALIBRATION,
  /** The crystal oscillator failed to start, which a TCXO raises until it is powered. */
  XoscStart: SX126X_ERROR_XOSC_START,
  /** The PLL failed to lock. */
  PllLock: SX126X_ERROR_PLL_LOCK,
  /** The power amplifier failed to ramp. */
  PaRamp: SX126X_ERROR_PA_RAMP,
} as const

/** The receive timeout word that keeps the chip listening until another command stops it. */
export const RX_CONTINUOUS = SX126X_RX_CONTINUOUS

/** The LoRa sync word of a public network such as LoRaWAN. */
export const SYNC_WORD_PUBLIC = SX126X_SYNC_WORD_PUBLIC

/** The LoRa sync word of a private network, and the chip's reset value. */
export const SYNC_WORD_PRIVATE = SX126X_SYNC_WORD_PRIVATE

/** The register that holds the most significant byte of the LoRa sync word. */
export const REGISTER_LORA_SYNC_WORD = SX126X_REGISTER_LORA_SYNC_WORD

/**
 * Returns the word SetRfFrequency takes for a frequency.
 *
 * @param frequencyHz - The carrier frequency in hertz.
 * @returns The frequency times 2^25 over the 32 MHz crystal, rounded to the nearest step.
 */
export function frequencyWord(frequencyHz: number): number {
  return sx126xFrequencyWord(frequencyHz)
}

/**
 * Returns the 24-bit timeout word SetTx and SetRx take for a duration.
 *
 * @param timeoutUs - The duration in microseconds.
 * @returns The number of 15.625 us steps; a nonzero duration never becomes the zero word
 *   that disables the timeout.
 */
export function timeoutSteps(timeoutUs: number): number {
  return sx126xTimeoutSteps(timeoutUs)
}

/**
 * Returns the two CalibrateImage codes that cover a band.
 *
 * @param lowHz - The lower edge of the band in hertz.
 * @param highHz - The upper edge of the band in hertz.
 * @returns `freq1` and `freq2`, 4 MHz steps that always cover the band.
 */
export function imageCalibration(lowHz: number, highHz: number): Buffer {
  return sx126xImageCalibration(lowHz, highHz)
}

/**
 * Returns the shortest amplifier ramp time the chip offers that lasts at least a duration.
 *
 * @param atLeastUs - The least ramp time wanted, in microseconds.
 * @returns One of the eight ramp times of Table 13-41, in microseconds.
 */
export function rampTimeUs(atLeastUs: number): number {
  return sx126xRampTimeUs(atLeastUs)
}

/**
 * Chooses the amplifier settings for an output power.
 *
 * @param amplifier - The chip's amplifier.
 * @param outputDbm - The output power wanted at the antenna port, in dBm.
 * @returns The configuration and the setting, clamped to what the amplifier allows.
 */
export function txPower(amplifier: Amplifier, outputDbm: number): TxPower {
  return sx126xTxPower(amplifier, outputDbm)
}

/**
 * Chooses the amplifier settings that keep a link's EIRP at or under a ceiling.
 *
 * @param amplifier - The chip's amplifier.
 * @param budget - The link budget, whose transmitting antenna and cable apply.
 * @param eirpCeilingDbm - The EIRP limit, such as a channel plan's ceiling for the frequency.
 * @returns The configuration and the setting, rounded down to whole decibels so the EIRP
 *   stays under the ceiling.
 */
export function txPowerUnderCeiling(
  amplifier: Amplifier,
  budget: LoraLinkBudget,
  eirpCeilingDbm: number,
): TxPower {
  return sx126xTxPowerUnderCeiling(amplifier, budget, eirpCeilingDbm)
}

/**
 * SetStandby into STDBY_RC, which stops a transmission or a reception.
 *
 * @returns The command bytes.
 */
export function setStandby(): Buffer {
  return sx126xSetStandby()
}

/**
 * SetPacketType for LoRa, the first radio setting a configuration sends.
 *
 * @returns The command bytes.
 */
export function setPacketTypeLora(): Buffer {
  return sx126xSetPacketTypeLora()
}

/**
 * SetRfFrequency for a carrier frequency.
 *
 * @param frequencyHz - The carrier frequency in hertz.
 * @returns The command bytes.
 */
export function setRfFrequency(frequencyHz: number): Buffer {
  return sx126xSetRfFrequency(frequencyHz)
}

/**
 * CalibrateImage over a band.
 *
 * @param lowHz - The lower edge of the band in hertz.
 * @param highHz - The upper edge of the band in hertz.
 * @returns The command bytes.
 */
export function calibrateImage(lowHz: number, highHz: number): Buffer {
  return sx126xCalibrateImage(lowHz, highHz)
}

/**
 * SetPaConfig for a power setting.
 *
 * @param power - The settings from {@link txPower} or {@link txPowerUnderCeiling}.
 * @returns The command bytes.
 */
export function setPaConfig(power: TxPower): Buffer {
  return sx126xSetPaConfig(power)
}

/**
 * SetTxParams for a power setting and a ramp time.
 *
 * @param power - The power settings.
 * @param rampUs - The least amplifier ramp time wanted, in microseconds.
 * @returns The command bytes.
 */
export function setTxParams(power: TxPower, rampUs: number): Buffer {
  return sx126xSetTxParams(power, rampUs)
}

/**
 * SetModulationParams for a LoRa link.
 *
 * @param link - The link settings, from `@pamoja/lora`.
 * @returns The command bytes.
 * @throws If the link's bandwidth is not one the SX126x offers.
 */
export function setLoraModulationParams(link: LoraLink): Buffer {
  return sx126xSetLoraModulationParams(link)
}

/**
 * SetPacketParams for a LoRa link and a payload.
 *
 * @param link - The link settings, whose preamble, header, and CRC the frame uses.
 * @param payloadLength - The payload length to send, or the most a receiver accepts.
 * @param invertIq - Whether the IQ polarity is inverted, as LoRaWAN downlinks use.
 * @returns The command bytes.
 */
export function setLoraPacketParams(
  link: LoraLink,
  payloadLength: number,
  invertIq: boolean,
): Buffer {
  return sx126xSetLoraPacketParams(link, payloadLength, invertIq)
}

/**
 * SetDioIrqParams: which interrupts are enabled, and which DIO lines raise them.
 *
 * @param irq - The interrupts to enable, as {@link Irq} bits.
 * @param dio1 - The interrupts routed to DIO1.
 * @param dio2 - The interrupts routed to DIO2, none by default.
 * @param dio3 - The interrupts routed to DIO3, none by default.
 * @returns The command bytes.
 */
export function setDioIrqParams(irq: number, dio1: number, dio2?: number, dio3?: number): Buffer {
  return sx126xSetDioIrqParams(irq, dio1, dio2, dio3)
}

/**
 * ClearIrqStatus for a set of interrupts.
 *
 * @param irq - The interrupts to clear, as {@link Irq} bits.
 * @returns The command bytes.
 */
export function clearIrqStatus(irq: number): Buffer {
  return sx126xClearIrqStatus(irq)
}

/**
 * SetTx with a timeout.
 *
 * @param timeoutUs - How long the chip may transmit before it raises TIMEOUT, in
 *   microseconds; 0 disables the timeout.
 * @returns The command bytes.
 */
export function setTx(timeoutUs: number): Buffer {
  return sx126xSetTx(timeoutUs)
}

/**
 * SetRx with a timeout.
 *
 * @param timeoutUs - How long the chip listens for a packet to start, in microseconds; 0
 *   listens for one packet with no timeout.
 * @returns The command bytes.
 */
export function setRx(timeoutUs: number): Buffer {
  return sx126xSetRx(timeoutUs)
}

/**
 * SetRx in continuous mode, receiving packet after packet until another command.
 *
 * @returns The command bytes.
 */
export function setRxContinuous(): Buffer {
  return sx126xSetRxContinuous()
}

/**
 * SetSleep, without an RTC wake-up.
 *
 * @param warmStart - Whether to keep the configuration in retention while asleep.
 * @returns The command bytes.
 */
export function setSleep(warmStart: boolean): Buffer {
  return sx126xSetSleep(warmStart)
}

/**
 * A whole WriteRegister transaction.
 *
 * @param address - The first register's address, such as {@link REGISTER_LORA_SYNC_WORD}.
 * @param values - The register values, one byte each.
 * @returns The opcode, the address, and the values.
 */
export function writeRegister(address: number, values: Uint8Array): Buffer {
  return sx126xWriteRegister(address, Buffer.from(values))
}

/**
 * A whole WriteBuffer transaction.
 *
 * @param offset - Where in the data buffer the first byte goes.
 * @param payload - The bytes to write.
 * @returns The opcode, the offset, and the payload.
 */
export function writeBuffer(offset: number, payload: Uint8Array): Buffer {
  return sx126xWriteBuffer(offset, Buffer.from(payload))
}

/**
 * GetStatus, answered by the status byte; decode it with {@link status}.
 *
 * @returns The query.
 */
export function getStatus(): Query {
  return sx126xGetStatus()
}

/**
 * GetIrqStatus, answered by two IRQ bytes; decode them with {@link irq}.
 *
 * @returns The query.
 */
export function getIrqStatus(): Query {
  return sx126xGetIrqStatus()
}

/**
 * GetRxBufferStatus, answered by two bytes; decode them with {@link rxBufferStatus}.
 *
 * @returns The query.
 */
export function getRxBufferStatus(): Query {
  return sx126xGetRxBufferStatus()
}

/**
 * GetPacketStatus, answered by three bytes; decode them with {@link packetStatus}.
 *
 * @returns The query.
 */
export function getPacketStatus(): Query {
  return sx126xGetPacketStatus()
}

/**
 * GetRssiInst, answered by one byte; decode it with {@link rssiInstDbm}.
 *
 * @returns The query.
 */
export function getRssiInst(): Query {
  return sx126xGetRssiInst()
}

/**
 * GetDeviceErrors, answered by two bytes; decode them with {@link deviceErrors}.
 *
 * @returns The query.
 */
export function getDeviceErrors(): Query {
  return sx126xGetDeviceErrors()
}

/**
 * ReadRegister for a run of consecutive registers.
 *
 * @param address - The first register's address.
 * @param length - How many registers to read.
 * @returns The query.
 */
export function readRegister(address: number, length: number): Query {
  return sx126xReadRegister(address, length)
}

/**
 * ReadBuffer for a run of the data buffer.
 *
 * @param offset - Where in the buffer the first byte is.
 * @param length - How many bytes to read.
 * @returns The query.
 */
export function readBuffer(offset: number, length: number): Query {
  return sx126xReadBuffer(offset, length)
}

/**
 * Decodes a status byte.
 *
 * @param byte - The status byte.
 * @returns The chip's mode, how its last command went, and whether that was an error.
 */
export function status(byte: number): Status {
  return sx126xStatus(byte)
}

/**
 * Decodes a GetIrqStatus answer.
 *
 * @param answer - The two answer bytes.
 * @returns The pending interrupts, as {@link Irq} bits.
 * @throws If the answer is not two bytes.
 */
export function irq(answer: Uint8Array): number {
  return sx126xIrq(Buffer.from(answer))
}

/**
 * Decodes a GetDeviceErrors answer.
 *
 * @param answer - The two answer bytes.
 * @returns The flagged errors, as {@link DeviceError} bits.
 * @throws If the answer is not two bytes.
 */
export function deviceErrors(answer: Uint8Array): number {
  return sx126xDeviceErrors(Buffer.from(answer))
}

/**
 * Decodes a LoRa GetPacketStatus answer.
 *
 * @param answer - RssiPkt, SnrPkt, and SignalRssiPkt.
 * @returns The three signal levels, exact to a hundredth of a decibel.
 * @throws If the answer is not three bytes.
 */
export function packetStatus(answer: Uint8Array): PacketStatus {
  return sx126xPacketStatus(Buffer.from(answer))
}

/**
 * Decodes a GetRxBufferStatus answer.
 *
 * @param answer - PayloadLengthRx and RxStartBufferPointer.
 * @returns The payload length and where it starts in the data buffer.
 * @throws If the answer is not two bytes.
 */
export function rxBufferStatus(answer: Uint8Array): RxBufferStatus {
  return sx126xRxBufferStatus(Buffer.from(answer))
}

/**
 * Decodes a GetRssiInst answer.
 *
 * @param byte - RssiInst.
 * @returns The signal power the receiver hears right now, in dBm.
 */
export function rssiInstDbm(byte: number): number {
  return sx126xRssiInstDbm(byte)
}

/**
 * Reports whether an LLCC68 supports a link's spreading factor at its bandwidth.
 *
 * The LLCC68 takes the SX1262's commands but not all its settings: up to SF9 at 125 kHz,
 * SF10 at 250 kHz, and SF11 at 500 kHz, and no bandwidth below 125 kHz.
 *
 * @param link - The link settings.
 * @returns `true` when an LLCC68 can use the link.
 */
export function llcc68Supports(link: LoraLink): boolean {
  return sx126xLlcc68Supports(link)
}
