/**
 * Ergonomic facade over the generated Modbus RTU binding.
 *
 * Modbus over RS485 is what cheap industrial sensing speaks: energy meters, soil
 * probes, water-quality transmitters, pump controllers. A {@link ModbusClient} runs
 * whole transactions over a serial port, with the timing the serial line specification
 * sets, and a {@link ModbusServer} is a device a {@link ModbusLine} puts on the far end
 * of a simulated one. Underneath, each request builder here returns a complete frame
 * with its CRC, and a reply comes back through {@link parseFrame} as an object that
 * reads its own values.
 *
 * @packageDocumentation
 */

import type { SerialPort, SerialSettings } from '@pamoja/hal'
import {
  ModbusClient as NativeModbusClient,
  type ModbusFailure,
  ModbusLine,
  type ModbusOutcome,
  ModbusServer,
  modbusCoils,
  modbusCrc16,
  modbusParseFrame,
  modbusRaw,
  modbusReadCoils,
  modbusReadDiscreteInputs,
  modbusReadHoldingRegisters,
  modbusReadHoldingRegistersReply,
  modbusReadInputRegisters,
  modbusReadInputRegistersReply,
  modbusRegisters,
  modbusWriteMultipleCoils,
  modbusWriteMultipleRegisters,
  modbusWriteSingleCoil,
  modbusWriteSingleRegister,
} from '@pamoja/native'

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
 * Builds the reply a device sends to a read-holding-registers request.
 *
 * This is the answering half of {@link readHoldingRegisters}, so a client can be
 * written and tested against what a device sends without a device on the line.
 *
 * @param address - The unit address the reply comes from.
 * @param values - The register values the device reports, in address order.
 * @returns The frame the device would send.
 * @throws If there are no values, or more than one frame can carry.
 */
export function readHoldingRegistersReply(address: number, values: number[]): Buffer {
  return modbusReadHoldingRegistersReply(address, values)
}

/**
 * Builds the reply a device sends to a read-input-registers request.
 *
 * @param address - The unit address the reply comes from.
 * @param values - The register values the device reports, in address order.
 * @returns The frame the device would send.
 * @throws If there are no values, or more than one frame can carry.
 */
export function readInputRegistersReply(address: number, values: number[]): Buffer {
  return modbusReadInputRegistersReply(address, values)
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

/** The unit address every device acts on and none answers. */
export const BROADCAST = 0

/** The reason a device gives for refusing a request, as it appears on the wire. */
export const ExceptionCode = {
  /** The function code is not allowed for this device. */
  IllegalFunction: 0x01,
  /** The data address is not allowed for this device. */
  IllegalDataAddress: 0x02,
  /** A value in the request is not allowed for this device. */
  IllegalDataValue: 0x03,
  /** The device failed while serving the request. */
  ServerDeviceFailure: 0x04,
  /** The device accepted a long-running request and is still processing it. */
  Acknowledge: 0x05,
  /** The device is busy with a long-running request; retry later. */
  ServerDeviceBusy: 0x06,
  /** The device detected a parity error in its memory. */
  MemoryParityError: 0x08,
  /** A gateway could not route the request to the target path. */
  GatewayPathUnavailable: 0x0a,
  /** A gateway got no response from the target device, usually one not on the network. */
  GatewayTargetFailedToRespond: 0x0b,
} as const

/** One of the {@link ExceptionCode} values. */
export type ExceptionCode = (typeof ExceptionCode)[keyof typeof ExceptionCode]

/** Why a Modbus transaction failed. */
export type ModbusClientErrorKind =
  | 'Request'
  | 'BroadcastRead'
  | 'Port'
  | 'Timeout'
  | 'Frame'
  | 'WrongUnit'
  | 'WrongFunction'
  | 'Mismatch'
  | 'Exception'

/**
 * Thrown when a Modbus transaction fails. `kind` says why: `Request` for a request one frame
 * cannot carry, `BroadcastRead` for a read sent to unit 0, `Port` for a port failure, `Timeout`
 * for a device that did not answer in time, `Frame` for a reply that fails its CRC or its
 * shape, `WrongUnit` and `WrongFunction` for a reply to someone else's request, `Mismatch` for
 * a reply that does not answer this one, and `Exception` for a device that refused.
 *
 * @example
 * ```ts
 * try {
 *   await client.readHoldingRegisters(17, 120, 3)
 * } catch (error) {
 *   if (error instanceof ModbusClientError && error.exception === ExceptionCode.IllegalDataAddress) {
 *     console.log('the meter has no register there')
 *   }
 * }
 * ```
 */
export class ModbusClientError extends Error {
  /** Why the transaction failed. */
  readonly kind: ModbusClientErrorKind
  /** The unit the transaction asked. */
  readonly unit: number
  /** For `Exception` and `WrongFunction`, the function asked; otherwise `null`. */
  readonly functionCode: number | null
  /**
   * For `WrongUnit`, the unit that answered, and for `WrongFunction`, the function the reply
   * named; otherwise `null`.
   */
  readonly found: number | null
  /** For `Exception`, why the device refused; otherwise `null`. */
  readonly exception: ExceptionCode | null
  /** For `Timeout`, how many bytes of a reply had arrived; otherwise `null`. */
  readonly received: number | null

  /**
   * Wraps what the native client reported.
   *
   * @param failure - The failure, as the native client describes it.
   */
  constructor(failure: ModbusFailure) {
    super(failure.message)
    this.name = 'ModbusClientError'
    this.kind = failure.kind as ModbusClientErrorKind
    this.unit = failure.unit
    this.functionCode = failure.function ?? null
    this.found = failure.found ?? null
    this.exception = (failure.exception ?? null) as ExceptionCode | null
    this.received = failure.received ?? null
  }
}

/**
 * A Modbus device: a unit address, 1 to 247, and the four tables it serves, coils, discrete
 * inputs, holding registers, and input registers, each holding only the addresses it was
 * given. `setHoldingRegisters(start, values)` and its siblings fill a table,
 * `holdingRegister(address)` and its siblings read one entry back or `null`, and
 * `answer(frame)` answers one RTU frame as the device on the line does, or returns `null`
 * when it stays silent: the frame failed its CRC, is for another unit, or is a broadcast,
 * whose write the device still carries out. `served` counts the requests it carried out.
 */
export { ModbusServer }

/**
 * Several devices on one simulated line, as devices share an RS485 pair: every frame reaches
 * all of them, the one it is addressed to answers, and each carries out a broadcast write.
 * `attach(server)` puts a device on it, which the line then shares, and `port(settings)`
 * makes a {@link SerialPort} with the line on its far end for a {@link ModbusClient} to poll.
 * Nothing on that port waits: the silences and timeouts of a real line are counted in its
 * `waitedMicros`.
 */
export { ModbusLine }

/** How a {@link ModbusClient} paces a line. */
export interface ModbusClientOptions {
  /** How long to wait for a whole reply, 1000 ms unless given. */
  responseTimeoutMs?: number
  /** How long to leave the line quiet after a broadcast, 100 ms unless given. */
  turnaroundMs?: number
}

/**
 * A Modbus RTU client on a serial line: the gateway, the master in the specification's words,
 * that sends each request and waits for its reply.
 *
 * Each transaction follows the Modbus over Serial Line specification. The client leaves the
 * line silent for 3.5 characters, or 1.75 ms above 19200 baud; drops anything stale waiting in
 * the port; writes the request; and reads the reply to the length the request implies, against
 * the response timeout. The reply's CRC, unit, and function are checked before a value is read
 * out of it, and any failure rejects with a {@link ModbusClientError}. A write to
 * {@link BROADCAST} reaches every device and draws no reply, so the client waits out the
 * turnaround delay instead. Each call runs on a worker thread.
 *
 * @example
 * ```ts
 * const port = SerialPort.open('/dev/ttyUSB0', { baud: 19200, parity: Parity.Even })
 * const client = new ModbusClient(port)
 * const [volts, amps] = await client.readHoldingRegisters(17, 107, 2)
 * ```
 */
export class ModbusClient {
  readonly #native: NativeModbusClient

  /**
   * Makes a client on a port.
   *
   * @param port - The line, opened at the speed and format the devices on it use.
   * @param options - The response timeout and the turnaround delay.
   */
  constructor(port: SerialPort, options: ModbusClientOptions = {}) {
    this.#native = new NativeModbusClient(port)
    if (options.responseTimeoutMs !== undefined) {
      this.#native.setResponseTimeout(options.responseTimeoutMs)
    }
    if (options.turnaroundMs !== undefined) {
      this.#native.setTurnaround(options.turnaroundMs)
    }
  }

  /**
   * The silence that separates two frames: 3.5 characters at the line's speed and format, and
   * a fixed 1750 microseconds above 19200 baud.
   *
   * @param settings - The line's speed and character format.
   * @returns The silence, in nanoseconds, rounded up.
   */
  static frameGapNanos(settings: SerialSettings): number {
    return NativeModbusClient.frameGapNanos(settings)
  }

  /** How long the client waits for a whole reply, in milliseconds. */
  get responseTimeoutMs(): number {
    return this.#native.responseTimeoutMs
  }

  set responseTimeoutMs(ms: number) {
    this.#native.setResponseTimeout(ms)
  }

  /** How long the client leaves the line quiet after a broadcast, in milliseconds. */
  get turnaroundMs(): number {
    return this.#native.turnaroundMs
  }

  set turnaroundMs(ms: number) {
    this.#native.setTurnaround(ms)
  }

  /**
   * Reads coils, function `0x01`.
   *
   * @param unit - The device, 1 to 247.
   * @param start - The first coil's address.
   * @param quantity - How many, 1 to 2000.
   * @returns The coils' states, in address order.
   * @throws {@link ModbusClientError} When the transaction fails.
   */
  async readCoils(unit: number, start: number, quantity: number): Promise<boolean[]> {
    return settled(await this.#native.readCoils(unit, start, quantity)).bits ?? []
  }

  /**
   * Reads discrete inputs, function `0x02`.
   *
   * @param unit - The device, 1 to 247.
   * @param start - The first input's address.
   * @param quantity - How many, 1 to 2000.
   * @returns The inputs' states, in address order.
   * @throws {@link ModbusClientError} When the transaction fails.
   */
  async readDiscreteInputs(unit: number, start: number, quantity: number): Promise<boolean[]> {
    return settled(await this.#native.readDiscreteInputs(unit, start, quantity)).bits ?? []
  }

  /**
   * Reads holding registers, function `0x03`.
   *
   * @param unit - The device, 1 to 247.
   * @param start - The first register's address.
   * @param quantity - How many, 1 to 125.
   * @returns The registers' values, in address order.
   * @throws {@link ModbusClientError} When the transaction fails.
   */
  async readHoldingRegisters(unit: number, start: number, quantity: number): Promise<number[]> {
    return settled(await this.#native.readHoldingRegisters(unit, start, quantity)).registers ?? []
  }

  /**
   * Reads input registers, function `0x04`.
   *
   * @param unit - The device, 1 to 247.
   * @param start - The first register's address.
   * @param quantity - How many, 1 to 125.
   * @returns The registers' values, in address order.
   * @throws {@link ModbusClientError} When the transaction fails.
   */
  async readInputRegisters(unit: number, start: number, quantity: number): Promise<number[]> {
    return settled(await this.#native.readInputRegisters(unit, start, quantity)).registers ?? []
  }

  /**
   * Writes one coil, function `0x05`.
   *
   * @param unit - The device, 1 to 247, or {@link BROADCAST} for every device.
   * @param address - The coil's address.
   * @param on - The state to write.
   * @throws {@link ModbusClientError} When the transaction fails.
   */
  async writeSingleCoil(unit: number, address: number, on: boolean): Promise<void> {
    settled(await this.#native.writeSingleCoil(unit, address, on))
  }

  /**
   * Writes one holding register, function `0x06`.
   *
   * @param unit - The device, 1 to 247, or {@link BROADCAST} for every device.
   * @param address - The register's address.
   * @param value - The value to write.
   * @throws {@link ModbusClientError} When the transaction fails.
   */
  async writeSingleRegister(unit: number, address: number, value: number): Promise<void> {
    settled(await this.#native.writeSingleRegister(unit, address, value))
  }

  /**
   * Writes a run of coils, function `0x0F`.
   *
   * @param unit - The device, 1 to 247, or {@link BROADCAST} for every device.
   * @param start - The first coil's address.
   * @param values - The states to write, 1 to 1968 of them, in address order.
   * @throws {@link ModbusClientError} When the transaction fails.
   */
  async writeMultipleCoils(unit: number, start: number, values: readonly boolean[]): Promise<void> {
    settled(await this.#native.writeMultipleCoils(unit, start, [...values]))
  }

  /**
   * Writes a run of holding registers, function `0x10`.
   *
   * @param unit - The device, 1 to 247, or {@link BROADCAST} for every device.
   * @param start - The first register's address.
   * @param values - The values to write, 1 to 123 of them, in address order.
   * @throws {@link ModbusClientError} When the transaction fails.
   */
  async writeMultipleRegisters(
    unit: number,
    start: number,
    values: readonly number[],
  ): Promise<void> {
    settled(await this.#native.writeMultipleRegisters(unit, start, [...values]))
  }
}

function settled(outcome: ModbusOutcome): ModbusOutcome {
  if (outcome.failure) {
    throw new ModbusClientError(outcome.failure)
  }
  return outcome
}