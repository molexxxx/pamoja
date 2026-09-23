/**
 * Ergonomic facade over the generated bus binding.
 *
 * A driver is a conversation with a part, and a bus is what carries it. An {@link I2cBus} is
 * one bus that the program and every driver on it share, with one of three things on the
 * other end: the kernel's adapter on a Linux board, simulated parts that answer from their
 * registers, or a script of the transfers a driver is expected to make. A driver runs the
 * same way over all three, so a program is written and tested with nothing plugged in and
 * then pointed at `/dev/i2c-1`.
 *
 * @packageDocumentation
 */

import {
  CommandPart,
  I2cBus,
  type I2cBusKind as I2cBusKindName,
  type I2cFault as I2cFaultName,
  I2cPart,
  I2cStep,
  WordPart,
} from '@pamoja/native'

/**
 * One I2C bus, shared by the program and every driver built on it.
 *
 * `I2cBus.open('/dev/i2c-1')` opens the kernel's adapter on a Linux board, and throws
 * anywhere else, or when the interface is off or the process may not use it.
 * `I2cBus.simulated([parts])` puts simulated parts of any kind on a bus, each answering at its
 * own address, and `I2cBus.scripted([steps])` plays {@link I2cStep}s in order and refuses any
 * other transfer. `write`, `read`, and `writeRead` move bytes, and a failed transfer throws
 * with the reason: nothing answered at the address, the script expected something else, or
 * the kernel's own words. `part(address)` copies what a simulated part holds now, as the class
 * of part it is, `transfers` counts every transfer, `remaining` is what a script has left, and
 * `waitedMicros` is how long the drivers on the bus have asked to wait, which a simulated or
 * scripted bus counts without sleeping through.
 */
export { I2cBus }

/**
 * A part that is not there, answering from 256 registers a byte wide, as Bosch's parts do.
 *
 * `new I2cPart(address)` starts with every register at zero, and `load(first, bytes)` puts
 * bytes in from a register on. A write names a register and fills it and the ones after it;
 * a read takes them back from wherever the last write left off. What a driver writes stays
 * written, so `register(address)` reads a part's configuration back once a driver is done.
 */
export { I2cPart }

/**
 * A part that is not there, answering from 256 registers sixteen bits wide, as Texas
 * Instruments' parts do.
 *
 * `new WordPart(address)` starts with every register at zero; `set(register, value)` puts a
 * word in and `word(register)` reads one back. A register travels most significant byte
 * first. `readOnly(register, mask)` marks the bits the part sets for itself, such as a
 * conversion-ready flag, which then keep the part's value whatever a driver writes.
 */
export { WordPart }

/**
 * A part that is not there, answering commands with the replies it was given, as Sensirion's
 * parts do.
 *
 * `new CommandPart(address, width?)` takes commands `width` bytes long, two unless given, and
 * `answer(command, reply)` gives one command its reply. A read takes the reply the last
 * command left, once, padded with `0xFF`; a read with no reply waiting is not acknowledged,
 * as a real part refuses one. `received` lists every write, oldest first.
 */
export { CommandPart }

/** Any simulated part: a bus takes each kind and gives each back as its own class. */
export type SimulatedPart = I2cPart | WordPart | CommandPart

/**
 * One transfer a script expects, and what the part answers: `I2cStep.write(address, bytes)`,
 * `I2cStep.read(address, reply)`, `I2cStep.writeRead(address, bytes, reply)` for a register
 * read, and `I2cStep.fault(address, fault)` for a transfer that fails on purpose.
 */
export { I2cStep }

/** What answers on a bus. */
export const I2cBusKind = {
  /** The kernel's adapter, with real parts on real wires. */
  Adapter: 'Adapter' as I2cBusKindName,
  /** Simulated parts, answering from their registers. */
  Simulated: 'Simulated' as I2cBusKindName,
  /** A script of the transfers a driver is expected to make. */
  Scripted: 'Scripted' as I2cBusKindName,
} as const

/** What answers on a bus. */
export type I2cBusKind = I2cBusKindName

/** How a scripted step fails the transfer that reaches it. */
export const I2cFault = {
  /** Nothing acknowledged the address. */
  NoAcknowledgeAddress: 'NoAcknowledgeAddress' as I2cFaultName,
  /** The part did not acknowledge a data byte. */
  NoAcknowledgeData: 'NoAcknowledgeData' as I2cFaultName,
  /** A missing acknowledge, with no telling whether of the address or the data. */
  NoAcknowledge: 'NoAcknowledge' as I2cFaultName,
  /** A bus error, such as a misplaced start or stop condition. */
  Bus: 'Bus' as I2cFaultName,
  /** Another controller won the bus. */
  ArbitrationLoss: 'ArbitrationLoss' as I2cFaultName,
  /** Data arrived faster than it was taken. */
  Overrun: 'Overrun' as I2cFaultName,
  /** A failure of no more particular kind. */
  Other: 'Other' as I2cFaultName,
} as const

/** How a scripted step fails the transfer that reaches it. */
export type I2cFault = I2cFaultName
