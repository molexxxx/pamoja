/**
 * Ergonomic facade over the generated store-and-forward binding.
 *
 * The queue a node writes into while it has nowhere to send. An in-memory
 * buffer suits a test or a process that will not outlive it; a file-backed one
 * survives a reboot, which is what a node somewhere without reliable power
 * actually needs.
 *
 * A full store refuses the next append rather than dropping anything, so a
 * record is never lost without the caller being told.
 *
 * @packageDocumentation
 */

export { Store } from '@pamoja/native'
