import { fleet } from './feed.js';

/** How long a freshly discovered group or sensor keeps its "new" highlight, in ms. */
const HIGHLIGHT_MS = 12000;

let known = null; // Set of ids seen so far; null until the first frame seeds it.
const newUntil = new Map(); // id -> timestamp the "new" state expires.

// Collects every group id and `groupId/sensorId` path in a snapshot.
function idsOf(snap)
{
  const ids = new Set();
  if (snap) for (const o of snap.orgs || []) for (const g of o.groups || [])
  {
    ids.add(g.id);
    for (const s of g.sensors || []) ids.add(g.id + '/' + s.id);
  }
  return ids;
}

// Watch the raw device snapshot (not the locally edited view): a remote discovery is what
// this signals, and a user's own local add does not need announcing.
$.effect(() =>
{
  const snap = fleet.value;
  if (!snap) return;
  const ids = idsOf(snap);
  if (known === null) { known = ids; return; }
  const now = Date.now();
  for (const id of ids) if (!known.has(id)) newUntil.set(id, now + HIGHLIGHT_MS);
  known = ids;
});

/**
 * Whether an id was discovered recently enough to still show the "new" cue.
 *
 * @param {string} id - a group id or a `groupId/sensorId` path.
 * @returns {boolean} true while the discovery highlight is active.
 */
export function isNew(id)
{
  const until = newUntil.get(id);
  if (until == null) return false;
  if (until < Date.now()) { newUntil.delete(id); return false; }
  return true;
}
