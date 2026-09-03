/**
 * The pamoja engine's surface for Node: the runtime version and the transport every
 * link shares, over the compiled engine in `@pamoja/native`.
 *
 * This is the counterpart of the `pamoja-core` crate. Each capability is its own
 * package (`@pamoja/mqtt`, `@pamoja/security`, and so on), and `pamoja` bundles all
 * of them. The generated contract itself is `@pamoja/native`.
 *
 * @packageDocumentation
 */

export { version } from '@pamoja/native'

export { Transport, type TransportMessage } from './transport'
