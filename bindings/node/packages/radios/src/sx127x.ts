/**
 * The Semtech SX1276, SX1277, SX1278, and SX1279, and modules such as the RFM95W, as the
 * register values they take and the readings they give.
 *
 * These radios are driven through registers. Each SPI transaction starts with an address
 * byte whose top bit is set for a write, followed by the data, and the address advances with
 * each byte except at the FIFO. These functions give the addresses, the values a LoRa link
 * and an output power put in them, and the readings decoded, from the SX1276/77/78/79
 * datasheet (Rev 7), so a program with its own SPI access can drive the chip.
 *
 * @module
 */

import {
  type LoraLink,
  type LoraLinkBudget,
  type Sx127xHighBwOptimize,
  type Sx127xMode as NativeSx127xMode,
  type Sx127xModem,
  type Sx127xModemStatus,
  type Sx127xPaOutput as NativeSx127xPaOutput,
  type Sx127xPacketStatus,
  type Sx127xSpuriousReception,
  type Sx127xTxPower,
  sx127xAutomaticIf,
  sx127xConstants,
  sx127xFrequencyFromWord,
  sx127xFrequencyWord,
  sx127xFskOpMode,
  sx127xHighBwOptimize,
  sx127xImageCalStart,
  sx127xInvertIq,
  sx127xInvertIq2,
  sx127xIrqFlags,
  sx127xLoraOpMode,
  sx127xModeFromOpMode,
  sx127xModem,
  sx127xModemStatus,
  sx127xOcpRegister,
  sx127xPacketStatus,
  sx127xReadAddress,
  sx127xRegisters,
  sx127xRssiDbm,
  sx127xSpuriousReception,
  sx127xSymbolTimeout,
  sx127xTxPower,
  sx127xTxPowerUnderCeiling,
  sx127xWriteAddress,
} from '@pamoja/native'

/** The amplifier settings: RegPaConfig, RegPaDac, and RegOcp, and the power they produce. */
export type TxPower = Sx127xTxPower

/** The LoRa modem registers a link turns into. */
export type Modem = Sx127xModem

/** The signal levels a LoRa packet was received with. */
export type PacketStatus = Sx127xPacketStatus

/** The live state of the LoRa modem. */
export type ModemStatus = Sx127xModemStatus

/** The writes of the 500 kHz sensitivity erratum. */
export type HighBwOptimize = Sx127xHighBwOptimize

/** The receive settings of the spurious reception erratum. */
export type SpuriousReception = Sx127xSpuriousReception

const REGISTERS = sx127xRegisters()
const CONSTANTS = sx127xConstants()
const IRQ_FLAGS = sx127xIrqFlags()

function named(table: Record<string, number>, name: string): number {
  const value = table[name]
  if (value === undefined) {
    throw new Error(`the native binding has no SX127x value named ${name}`)
  }
  return value
}

/** The register addresses, from Table 41 of the datasheet with the LoRa page selected. */
export const Register = {
  /** RegFifo, the LoRa data buffer, read or written at RegFifoAddrPtr. */
  Fifo: named(REGISTERS, 'fifo'),
  /** RegOpMode: LoRa or FSK, the register page, and the operating mode. */
  OpMode: named(REGISTERS, 'opMode'),
  /** RegFrfMsb, the top byte of the carrier word. */
  FrfMsb: named(REGISTERS, 'frfMsb'),
  /** RegFrfMid, the middle byte of the carrier word. */
  FrfMid: named(REGISTERS, 'frfMid'),
  /** RegFrfLsb, the low byte of the carrier word. */
  FrfLsb: named(REGISTERS, 'frfLsb'),
  /** RegPaConfig: the amplifier output, its maximum, and the power. */
  PaConfig: named(REGISTERS, 'paConfig'),
  /** RegPaRamp, the amplifier ramp time. */
  PaRamp: named(REGISTERS, 'paRamp'),
  /** RegOcp, the amplifier current limit. */
  Ocp: named(REGISTERS, 'ocp'),
  /** RegLna, the LNA gain and current. */
  Lna: named(REGISTERS, 'lna'),
  /** RegFifoAddrPtr, where the next RegFifo access lands. */
  FifoAddrPtr: named(REGISTERS, 'fifoAddrPtr'),
  /** RegFifoTxBaseAddr, where a transmitted payload starts. */
  FifoTxBaseAddr: named(REGISTERS, 'fifoTxBaseAddr'),
  /** RegFifoRxBaseAddr, where received payloads start. */
  FifoRxBaseAddr: named(REGISTERS, 'fifoRxBaseAddr'),
  /** RegFifoRxCurrentAddr, where the last received packet starts. */
  FifoRxCurrentAddr: named(REGISTERS, 'fifoRxCurrentAddr'),
  /** RegIrqFlagsMask, the interrupts masked off. */
  IrqFlagsMask: named(REGISTERS, 'irqFlagsMask'),
  /** RegIrqFlags, the interrupts raised, each cleared by writing it back as a 1. */
  IrqFlags: named(REGISTERS, 'irqFlags'),
  /** RegRxNbBytes, the payload length of the last packet received. */
  RxNbBytes: named(REGISTERS, 'rxNbBytes'),
  /** RegModemStat, the live state of the modem. */
  ModemStat: named(REGISTERS, 'modemStat'),
  /** RegPktSnrValue, the SNR of the last packet in quarters of a decibel. */
  PktSnrValue: named(REGISTERS, 'pktSnrValue'),
  /** RegPktRssiValue, the RSSI of the last packet. */
  PktRssiValue: named(REGISTERS, 'pktRssiValue'),
  /** RegRssiValue, the RSSI the receiver hears right now. */
  RssiValue: named(REGISTERS, 'rssiValue'),
  /** RegHopChannel, the PLL lock and the CRC the last header announced. */
  HopChannel: named(REGISTERS, 'hopChannel'),
  /** RegModemConfig1: bandwidth, coding rate, and header mode. */
  ModemConfig1: named(REGISTERS, 'modemConfig1'),
  /** RegModemConfig2: spreading factor, CRC, and the top bits of the symbol timeout. */
  ModemConfig2: named(REGISTERS, 'modemConfig2'),
  /** RegSymbTimeoutLsb, the low byte of the symbol timeout. */
  SymbTimeoutLsb: named(REGISTERS, 'symbTimeoutLsb'),
  /** RegPreambleMsb, the high byte of the preamble length. */
  PreambleMsb: named(REGISTERS, 'preambleMsb'),
  /** RegPreambleLsb, the low byte of the preamble length. */
  PreambleLsb: named(REGISTERS, 'preambleLsb'),
  /** RegPayloadLength, the payload length to send. */
  PayloadLength: named(REGISTERS, 'payloadLength'),
  /** RegMaxPayloadLength, the longest payload a received header may announce. */
  MaxPayloadLength: named(REGISTERS, 'maxPayloadLength'),
  /** RegModemConfig3: low data rate optimization and the automatic gain control. */
  ModemConfig3: named(REGISTERS, 'modemConfig3'),
  /** RegRssiWideband, a wideband RSSI sample. */
  RssiWideband: named(REGISTERS, 'rssiWideband'),
  /** RegIfFreq2, which the spurious reception erratum sets. */
  IfFreq2: named(REGISTERS, 'ifFreq2'),
  /** RegIfFreq1, which the spurious reception erratum clears. */
  IfFreq1: named(REGISTERS, 'ifFreq1'),
  /** RegDetectOptimize: the automatic IF and the detection optimization. */
  DetectOptimize: named(REGISTERS, 'detectOptimize'),
  /** RegInvertIQ, the IQ polarity of each path. */
  InvertIq: named(REGISTERS, 'invertIq'),
  /** RegHighBwOptimize1, which the 500 kHz erratum sets. */
  HighBwOptimize1: named(REGISTERS, 'highBwOptimize1'),
  /** RegDetectionThreshold, the LoRa detection threshold. */
  DetectionThreshold: named(REGISTERS, 'detectionThreshold'),
  /** RegSyncWord, the LoRa sync word. */
  SyncWord: named(REGISTERS, 'syncWord'),
  /** RegHighBwOptimize2, which the 500 kHz erratum sets. */
  HighBwOptimize2: named(REGISTERS, 'highBwOptimize2'),
  /** RegInvertIQ2, which completes an IQ inversion. */
  InvertIq2: named(REGISTERS, 'invertIq2'),
  /** RegImageCal, at the address of RegInvertIQ2 on the FSK page. */
  ImageCal: named(REGISTERS, 'imageCal'),
  /** RegDioMapping1, the events DIO0 to DIO3 signal. */
  DioMapping1: named(REGISTERS, 'dioMapping1'),
  /** RegDioMapping2, the events DIO4 and DIO5 signal. */
  DioMapping2: named(REGISTERS, 'dioMapping2'),
  /** RegVersion, the silicon revision. */
  Version: named(REGISTERS, 'version'),
  /** RegTcxo, a crystal or a TCXO on XTA. */
  Tcxo: named(REGISTERS, 'tcxo'),
  /** RegPaDac, the +20 dBm setting of PA_BOOST. */
  PaDac: named(REGISTERS, 'paDac'),
} as const

/** The RegVersion value of an SX1276, SX1277, SX1278, or SX1279. */
export const VERSION = named(CONSTANTS, 'version')
/** The bit of an address byte that makes an access a write. */
export const WRITE = named(CONSTANTS, 'write')
/** The sync word the datasheet reserves for LoRaWAN networks. */
export const SYNC_WORD_PUBLIC = named(CONSTANTS, 'syncWordPublic')
/** The private sync word, and the chip's reset value. */
export const SYNC_WORD_PRIVATE = named(CONSTANTS, 'syncWordPrivate')
/** RegPaDac at its reset value. */
export const PA_DAC_DEFAULT = named(CONSTANTS, 'paDacDefault')
/** RegPaDac with the +20 dBm setting on PA_BOOST. */
export const PA_DAC_HIGH_POWER = named(CONSTANTS, 'paDacHighPower')
/** The RegImageCal bit that starts a calibration. */
export const IMAGE_CAL_START = named(CONSTANTS, 'imageCalStart')
/** The RegImageCal bit set while a calibration runs. */
export const IMAGE_CAL_RUNNING = named(CONSTANTS, 'imageCalRunning')
/** RegLna with maximum gain and the high frequency LNA boost. */
export const LNA_BOOSTED = named(CONSTANTS, 'lnaBoosted')
/** RegTcxo for a module clocked by a TCXO. */
export const TCXO_INPUT_ON = named(CONSTANTS, 'tcxoInputOn')

/** The RegDioMapping1 values that route an event to DIO0, from Table 18 of the datasheet. */
export const Dio0 = {
  /** DIO0 signals RxDone. */
  RxDone: named(CONSTANTS, 'dio0RxDone'),
  /** DIO0 signals TxDone. */
  TxDone: named(CONSTANTS, 'dio0TxDone'),
  /** DIO0 signals CadDone. */
  CadDone: named(CONSTANTS, 'dio0CadDone'),
} as const

/** The LoRa interrupt flags of RegIrqFlags. */
export const Irq = {
  /** A single reception timed out before a preamble arrived. */
  RxTimeout: named(IRQ_FLAGS, 'rxTimeout'),
  /** A packet has been received. */
  RxDone: named(IRQ_FLAGS, 'rxDone'),
  /** The payload failed its CRC. */
  PayloadCrcError: named(IRQ_FLAGS, 'payloadCrcError'),
  /** A valid header was received. */
  ValidHeader: named(IRQ_FLAGS, 'validHeader'),
  /** The payload has been transmitted. */
  TxDone: named(IRQ_FLAGS, 'txDone'),
  /** Channel activity detection finished. */
  CadDone: named(IRQ_FLAGS, 'cadDone'),
  /** Frequency hopping moved to the next channel. */
  FhssChangeChannel: named(IRQ_FLAGS, 'fhssChangeChannel'),
  /** Channel activity detection heard a LoRa signal. */
  CadDetected: named(IRQ_FLAGS, 'cadDetected'),
} as const

/**
 * The amplifier output a module wires to its antenna.
 *
 * Provided as a runtime object, as the generated string enum is erased at compile time.
 * The SPI interface cannot see which output a module uses, so the caller names it.
 */
export const PaOutput = {
  /** The high efficiency amplifier on RFO_LF or RFO_HF, -4 to +15 dBm. */
  Rfo: 'Rfo' as NativeSx127xPaOutput,
  /** The regulated amplifier on PA_BOOST, +2 to +20 dBm, as on the RFM95W. */
  PaBoost: 'PaBoost' as NativeSx127xPaOutput,
} as const

/** One of the {@link PaOutput} values. */
export type PaOutput = NativeSx127xPaOutput

/** The operating modes of RegOpMode, as a runtime object. */
export const Mode = {
  /** Only the SPI interface and the registers are powered; the only mode that may switch modems. */
  Sleep: 'Sleep' as NativeSx127xMode,
  /** The oscillator and the baseband are on. */
  Standby: 'Standby' as NativeSx127xMode,
  /** The PLL is locked for transmit. */
  FsTx: 'FsTx' as NativeSx127xMode,
  /** One packet goes out, then the chip returns to standby. */
  Tx: 'Tx' as NativeSx127xMode,
  /** The PLL is locked for receive. */
  FsRx: 'FsRx' as NativeSx127xMode,
  /** The receiver takes packet after packet. */
  RxContinuous: 'RxContinuous' as NativeSx127xMode,
  /** The receiver waits for one packet or the symbol timeout. */
  RxSingle: 'RxSingle' as NativeSx127xMode,
  /** Channel activity detection looks for a LoRa preamble. */
  Cad: 'Cad' as NativeSx127xMode,
} as const

/** One of the {@link Mode} values. */
export type Mode = NativeSx127xMode

/**
 * Returns the 24-bit RegFrf word for a frequency.
 *
 * @param frequencyHz - The carrier frequency in hertz.
 * @returns The frequency times 2^19 over the 32 MHz crystal, rounded to the nearest step.
 */
export function frequencyWord(frequencyHz: number): number {
  return sx127xFrequencyWord(frequencyHz)
}

/**
 * Returns the frequency a RegFrf word selects.
 *
 * @param word - The 24-bit frequency word.
 * @returns The carrier frequency in hertz.
 */
export function frequencyFromWord(word: number): number {
  return sx127xFrequencyFromWord(word)
}

/**
 * Returns the address byte that reads a register.
 *
 * @param address - The register address.
 * @returns The address with the write bit clear.
 */
export function readAddress(address: number): number {
  return sx127xReadAddress(address)
}

/**
 * Returns the address byte that writes a register.
 *
 * @param address - The register address.
 * @returns The address with the write bit set.
 */
export function writeAddress(address: number): number {
  return sx127xWriteAddress(address)
}

/**
 * Returns the RegOpMode value for a LoRa operating mode.
 *
 * @param mode - The operating mode.
 * @returns The register value, with the LoRa register page selected.
 */
export function loraOpMode(mode: Mode): number {
  return sx127xLoraOpMode(mode)
}

/**
 * Returns the RegOpMode value for an FSK operating mode, which image calibration needs.
 *
 * @param mode - The operating mode.
 * @returns The register value.
 */
export function fskOpMode(mode: Mode): number {
  return sx127xFskOpMode(mode)
}

/**
 * Returns the operating mode a RegOpMode value holds.
 *
 * @param opMode - The register value.
 * @returns The mode.
 */
export function modeFromOpMode(opMode: number): Mode {
  return sx127xModeFromOpMode(opMode)
}

/**
 * Returns the LoRa modem registers for a link at a carrier.
 *
 * @param link - The link settings.
 * @param frequencyHz - The carrier frequency, which rules out 250 and 500 kHz below 175 MHz.
 * @param symbolTimeout - A single reception timeout in symbols, whose top bits go in RegModemConfig2.
 * @returns RegModemConfig1 to 3 and the SF6 detection settings.
 * @throws If the SX127x cannot use the link at the carrier.
 */
export function modem(link: LoraLink, frequencyHz: number, symbolTimeout = 0): Modem {
  return sx127xModem(link, frequencyHz, symbolTimeout)
}

/**
 * Returns a single reception timeout in the link's symbols.
 *
 * @param link - The link settings, whose symbol time counts the timeout.
 * @param timeoutUs - How long to listen for a preamble, in microseconds.
 * @returns The timeout rounded up to whole symbols, from 4 to 1023.
 */
export function symbolTimeout(link: LoraLink, timeoutUs: number): number {
  return sx127xSymbolTimeout(link, timeoutUs)
}

/**
 * Chooses the amplifier settings for an output power.
 *
 * @param output - The amplifier output the module uses.
 * @param outputDbm - The output power wanted, in dBm.
 * @returns The settings, clamped to what the output delivers.
 */
export function txPower(output: PaOutput, outputDbm: number): TxPower {
  return sx127xTxPower(output, outputDbm)
}

/**
 * Chooses the amplifier settings that keep a link's EIRP at or under a ceiling.
 *
 * @param output - The amplifier output the module uses.
 * @param budget - The link budget, whose transmitting antenna and cable apply.
 * @param eirpCeilingDbm - The EIRP limit in dBm.
 * @returns The settings, rounded down to whole decibels.
 */
export function txPowerUnderCeiling(
  output: PaOutput,
  budget: LoraLinkBudget,
  eirpCeilingDbm: number,
): TxPower {
  return sx127xTxPowerUnderCeiling(output, budget, eirpCeilingDbm)
}

/**
 * Returns RegOcp for a current limit.
 *
 * @param milliamps - The most current the amplifier may draw.
 * @returns The register value with the protection on.
 */
export function ocpRegister(milliamps: number): number {
  return sx127xOcpRegister(milliamps)
}

/**
 * Returns RegInvertIQ for the IQ polarity of each path.
 *
 * @param receive - Whether to invert the receive path, as a LoRaWAN device does for downlinks.
 * @param transmit - Whether to invert the transmit path, as a gateway does.
 * @returns The register value, with the transmit bit set for normal IQ as the reference drivers have it.
 */
export function invertIq(receive: boolean, transmit: boolean): number {
  return sx127xInvertIq(receive, transmit)
}

/**
 * Returns RegInvertIQ2 for the path in use.
 *
 * @param inverted - Whether that path is inverted.
 * @returns 0x19 when inverted, else 0x1D.
 */
export function invertIq2(inverted: boolean): number {
  return sx127xInvertIq2(inverted)
}

/**
 * Returns the writes of the 500 kHz sensitivity erratum.
 *
 * @param link - The link settings, whose bandwidth decides.
 * @param frequencyHz - The carrier frequency in hertz.
 * @returns RegHighBwOptimize1 and, where it is written, RegHighBwOptimize2.
 * @throws If the SX127x has no such bandwidth.
 */
export function highBwOptimize(link: LoraLink, frequencyHz: number): HighBwOptimize {
  return sx127xHighBwOptimize(link, frequencyHz)
}

/**
 * Returns the receive settings of the spurious reception erratum.
 *
 * @param link - The link settings, whose bandwidth decides.
 * @returns The automatic IF, the hand-set IF, and the carrier offset.
 * @throws If the SX127x has no such bandwidth.
 */
export function spuriousReception(link: LoraLink): SpuriousReception {
  return sx127xSpuriousReception(link)
}

/**
 * Returns RegImageCal to start a calibration.
 *
 * @param current - The register's current value.
 * @returns The value with ImageCalStart set and AutoImageCalOn clear.
 */
export function imageCalStart(current: number): number {
  return sx127xImageCalStart(current)
}

/**
 * Returns RegDetectOptimize with AutomaticIFOn set or clear.
 *
 * @param current - The register's current value.
 * @param on - Whether the automatic IF stays on.
 * @returns The register value.
 */
export function automaticIf(current: number, on: boolean): number {
  return sx127xAutomaticIf(current, on)
}

/**
 * Decodes RegPktSnrValue and RegPktRssiValue, read together.
 *
 * @param answer - The two register values, SNR first.
 * @param frequencyHz - The carrier the packet was heard at, which picks the RF port's offset.
 * @returns The three signal levels, exact to a hundredth of a decibel.
 * @throws If the answer is not two bytes.
 */
export function packetStatus(answer: Uint8Array, frequencyHz: number): PacketStatus {
  return sx127xPacketStatus(Buffer.from(answer), frequencyHz)
}

/**
 * Decodes RegRssiValue.
 *
 * @param byte - The register value.
 * @param frequencyHz - The carrier the receiver is tuned to.
 * @returns The signal power the receiver hears right now, in dBm.
 */
export function rssiDbm(byte: number, frequencyHz: number): number {
  return sx127xRssiDbm(byte, frequencyHz)
}

/**
 * Decodes RegModemStat.
 *
 * @param byte - The register value.
 * @returns The modem's live state.
 */
export function modemStatus(byte: number): ModemStatus {
  return sx127xModemStatus(byte)
}
