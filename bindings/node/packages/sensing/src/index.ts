/**
 * Sensing and actuation: The parts wired to a board: a thermometer that checks its own bytes, a servo pulse, a stepper walking its coils, and a part of your own.
 *
 * Installing this package installs `@pamoja/sensors`, `@pamoja/actuators`, `@pamoja/core`, and re-exports each under its own
 * name, so a name two of them share stays unambiguous.
 *
 * @packageDocumentation
 */

export * as sensors from '@pamoja/sensors'
export * as actuators from '@pamoja/actuators'
export * as core from '@pamoja/core'
