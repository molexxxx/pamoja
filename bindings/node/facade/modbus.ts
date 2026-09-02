/**
 * Ergonomic facade over the generated Modbus RTU binding.
 *
 * Modbus over RS485 is what cheap industrial sensing speaks: energy meters, soil
 * probes, water-quality transmitters, pump controllers. Each request builder here
 * returns a complete frame with its CRC, ready to write to a port, and a reply
 * comes back through {@link parseFrame} as an object that reads its own values.
 *
 * @packageDocumentation
 */

import {
  modbusCoils,
  modbusCrc16,
  modbusParseFrame,
  modbusRaw,
  modbusReadCoils,
  modbusReadDiscreteInputs,
  modbusReadHoldingRegisters,
  modbusReadInputRegisters,
  modbusRegisters,
  modbusWriteMultipleCoils,
  modbusWriteMultipleRegisters,
  modbusWriteSingleCoil,
  modbusWriteSingleRegister,
} from '../index'

/**
 * Computes the CRC-16/MODBUS that every RTU frame ends with.
 *
 * @param bytes - The frame contents, without the trailing checksum.
 * @returns The checksum.
 */
export function crc16(bytes: Uint8Array): number {
  return modbusCrc16(Buffer.from(bytes))
}

/**
 * Builds a read-coils request (function `0x01`).
 *
 * @param address - The unit address to ask.
 * @param start - The address of the first coil.
 * @param count - How many coils to read.
 * @returns The frame to send.
 */
export function readCoils(address: number, start: number, count: number): Buffer {
  return modbusReadCoils(address, start, count)
}

/**
 * Builds a read-discrete-inputs request (function `0x02`).
 *
 * @param address - The unit address to ask.
 * @param start - The address of the first input.
 * @param count - How many inputs to read.
 * @returns The frame to send.
 */
export function readDiscreteInputs(address: number, start: number, count: number): Buffer {
  return modbusReadDiscreteInputs(address, start, count)
}

/**
 * Builds a read-holding-registers request (function `0x03`).
 *
 * @param address - The unit address to ask.
 * @param start - The address of the first register.
 * @param count - How many registers to read.
 * @returns The frame to send.
 */
export function readHoldingRegisters(address: number, start: number, count: number): Buffer {
  return modbusReadHoldingRegisters(address, start, count)
}

/**
 * Builds a read-input-registers request (function `0x04`).
 *
 * @param address - The unit address to ask.
 * @param start - The address of the first register.
 * @param count - How many registers to read.
 * @returns The frame to send.
 */
export function readInputRegisters(address: number, start: number, count: number): Buffer {
  return modbusReadInputRegisters(address, start, count)
}

/**
 * Builds a write-single-coil request (function `0x05`).
 *
 * @param address - The unit address to write to.
 * @param coil - The coil address.
 * @param on - The state to write.
 * @returns The frame to send.
 */
export function writeSingleCoil(address: number, coil: number, on: boolean): Buffer {
  return modbusWriteSingleCoil(address, coil, on)
}

/**
 * Builds a write-single-register request (function `0x06`).
 *
 * @param address - The unit address to write to.
 * @param register - The register address.
 * @param value - The 16-bit value to write.
 * @returns The frame to send.
 */
export function writeSingleRegister(address: number, register: number, value: number): Buffer {
  return modbusWriteSingleRegister(address, register, value)
}

/**
 * Builds a write-multiple-registers request (function `0x10`).
 *
 * @param address - The unit address to write to.
 * @param start - The address of the first register.
 * @param values - The 16-bit values, at most 123 of them.
 * @returns The frame to send.
 * @throws If there are no values, or more than one request can carry.
 */
export function writeMultipleRegisters(
  address: number,
  start: number,
  values: readonly number[],
): Buffer {
  return modbusWriteMultipleRegisters(address, start, values as number[])
}

/**
 * Builds a write-multiple-coils request (function `0x0F`).
 *
 * @param address - The unit address to write to.
 * @param start - The address of the first coil.
 * @param values - One state per coil, at most 1968 of them.
 * @returns The frame to send.
 * @throws If there are no values, or more than one request can carry.
 */
export function writeMultipleCoils(
  address: number,
  start: number,
  values: readonly boolean[],
): Buffer {
  return modbusWriteMultipleCoils(address, start, values as boolean[])
}

/**
 * Builds a request from a raw function code and data, for the function codes this
 * SDK does not name.
 *
 * @param address - The unit address to send to.
 * @param functionCode - The function code byte.
 * @param data - The bytes that follow it, used verbatim.
 * @returns The frame to send.
 * @throws If the data is longer than a PDU may be.
 */
export function raw(address: number, functionCode: number, data: Uint8Array): Buffer {
  return modbusRaw(address, functionCode, Buffer.from(data))
}

/**
 * A received frame whose CRC has been verified, which reads its own values.
 *
 * @example
 * ```ts
 * const reply = parseFrame(await port.read())
 * if (reply.exception === null) console.log(reply.registers())
 * ```
 */
export class ModbusFrame {
  /** The unit address the frame is addressed to or came from. */
  readonly address: number

  /**
   * The function code. An exception response carries the request's code with its
   * high bit set, as it appeared on the wire.
   */
  readonly functionCode: number

  /** The exception a device reported, or `null` when it served the request. */
  readonly exception: number | null

  /** The protocol data unit: the function code and its data. */
  readonly pdu: Buffer

  /**
   * Wraps a parsed frame.
   *
   * @param address - The unit address.
   * @param functionCode - The function code as it appeared on the wire.
   * @param exception - The exception code, or `null`.
   * @param pdu - The protocol data unit.
   */
  constructor(address: number, functionCode: number, exception: number | null, pdu: Buffer) {
    this.address = address
    this.functionCode = functionCode
    this.exception = exception
    this.pdu = pdu
  }

  /**
   * Reads the 16-bit registers out of a read-registers reply.
   *
   * @returns The registers, in order.
   * @throws If this is not a well-formed read-registers reply.
   */
  registers(): number[] {
    return modbusRegisters(this.pdu)
  }

  /**
   * Reads the coils or discrete inputs out of a read-bits reply.
   *
   * @param count - How many bits to read, the quantity the request asked for.
   * @returns One state per coil, in order.
   * @throws If the reply does not carry that many bits.
   */
  coils(count: number): boolean[] {
    return modbusCoils(this.pdu, count)
  }
}

/**
 * Parses a received RTU frame, verifying its CRC.
 *
 * @param bytes - The frame as it came off the wire, checksum included.
 * @returns The validated frame.
 * @throws If the frame is truncated, oversized, or its CRC does not match.
 */
export function parseFrame(bytes: Uint8Array): ModbusFrame {
  const frame = modbusParseFrame(Buffer.from(bytes))
  return new ModbusFrame(
    frame.address,
    frame.functionCode,
    frame.exception ?? null,
    frame.pdu,
  )
}
