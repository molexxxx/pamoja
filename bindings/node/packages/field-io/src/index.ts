/**
 * Field I/O: The wires a gateway actually has: framed serial packets, an RS485 request and the reply it draws, a CAN frame, and the address a chip answers on.
 *
 * Installing this package installs `@pamoja/serial`, `@pamoja/modbus`, `@pamoja/can`, `@pamoja/gpio`, and re-exports each under its own
 * name, so a name two of them share stays unambiguous.
 *
 * @packageDocumentation
 */

export * as serial from '@pamoja/serial'
export * as modbus from '@pamoja/modbus'
export * as can from '@pamoja/can'
export * as gpio from '@pamoja/gpio'
