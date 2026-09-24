/**
 * The name a misspelled one was probably meant to be, worked out the way the library's
 * own parser does, so a refusal reads the same in every language.
 *
 * @packageDocumentation
 */

/** The control kinds the library ships, as a manifest names them. */
export const BUILT_IN_KINDS: readonly string[] = ['setpoint', 'level', 'surge', 'monitor']

/**
 * Picks the allowed name a given one is most likely a misspelling of.
 *
 * @param given - the name the file or the program used.
 * @param allowed - the names it may use there, in the order to prefer them.
 * @returns the closest allowed name, or `undefined` when none is within two edits and
 *   closer than half the given name's length.
 */
export function nearest(given: string, allowed: Iterable<string>): string | undefined {
  const limit = Math.min(2, Math.max(1, Math.floor([...given].length / 2)))
  let best: string | undefined
  let fewest = Number.POSITIVE_INFINITY
  for (const name of allowed) {
    const edits = distance(given, name)
    if (edits <= limit && edits < fewest) {
      best = name
      fewest = edits
    }
  }
  return best
}

/**
 * The reason a control kind with no policy behind it is refused.
 *
 * @param kind - the custom kind the profile names.
 * @param registered - the kinds a registry knows, in name order.
 * @returns the reason, worded as the library words it.
 */
export function unresolved(kind: string, registered: readonly string[]): string {
  const near = nearest(kind, [...BUILT_IN_KINDS, ...registered])
  const hint =
    near !== undefined
      ? `; did you mean \`${near}\`?`
      : registered.length === 0
        ? '; resolve the profile through a PolicyRegistry that registers it'
        : `; the registry knows \`${registered.join('`, `')}\``
  return `codec error: no policy decides the control kind \`${kind}\`, which is not built in${hint}`
}

function distance(a: string, b: string): number {
  const right = [...b]
  let previous = Array.from({ length: right.length + 1 }, (_, index) => index)
  let index = 0
  for (const left of a) {
    const current = [index + 1]
    right.forEach((char, at) => {
      const substitute = previous[at] + (left === char ? 0 : 1)
      current.push(Math.min(substitute, previous[at + 1] + 1, current[at] + 1))
    })
    previous = current
    index += 1
  }
  return previous[right.length]
}
