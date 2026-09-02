/**
 * Ergonomic facade over the generated Zenoh key-expression binding.
 *
 * A key expression is how a Zenoh network addresses data: a slash-separated
 * path that may carry the `*` and `**` wildcards, so one subscriber names a
 * whole subtree of a fleet rather than each node in it.
 *
 * Only the naming rules cross. Running a Zenoh session needs the std-only zenoh
 * stack, which would land in every install, so it stays in the Rust crate.
 *
 * @packageDocumentation
 */

import {
  keyexprCanonize,
  keyexprIsCanon,
  keyexprIsValid,
  keyexprMatches,
} from '../index'

/** The rules a Zenoh key expression obeys. */
export const keyexpr = {
  /** Reports whether a key expression is well formed. */
  isValid: keyexprIsValid,
  /** Reports whether a key expression is already in its canonical form. */
  isCanon: keyexprIsCanon,
  /**
   * Rewrites a key expression into its canonical form, or `null` if it is
   * malformed.
   *
   * Two expressions that select the same data have one canonical form, so
   * canonizing before comparing or routing avoids treating `a/**\/**\/b` and
   * `a/**\/b` as different.
   */
  canonize: keyexprCanonize,
  /** Reports whether a pattern selects a key. */
  matches: keyexprMatches,
} as const
