import { fleet, live } from './feed.js';
import { store } from '../store.js';
import { catalog } from './catalog.js';
import { sendCommand, unlocked } from './pair.js';

/**
 * Generates a short, unique id with a caller-supplied prefix.
 *
 * @param {string} p - the id prefix, such as `"s"` or `"g"`.
 * @returns {string} a unique identifier.
 */
export const uid = (p) => p + Date.now().toString(36) + Math.random().toString(36).slice(2, 5);

/**
 * Classifies a value against its safe band into a health status.
 *
 * @param {number} value - the reading value.
 * @param {[number, number]} [band] - the safe band `[low, high]`; a missing band is OK.
 * @returns {string} one of `'ok'`, `'warn'`, or `'alarm'`.
 */
export function statusFor(value, band)
{
  if (!band) return 'ok';
  const [lo, hi] = band, margin = (hi - lo) * 0.18;
  if (value < lo - margin || value > hi + margin) return 'alarm';
  if (value < lo || value > hi) return 'warn';
  return 'ok';
}

/**
 * Returns the more urgent of two statuses.
 *
 * @param {string} a - the first status.
 * @param {string} b - the second status.
 * @returns {string} the more urgent status.
 */
const worst = (a, b) => (['ok', 'warn', 'alarm'].indexOf(a) >= ['ok', 'warn', 'alarm'].indexOf(b) ? a : b);

/**
 * Sorts items by a saved order of ids; anything not in the order keeps its relative
 * position after the ordered ones (stable), so new items append cleanly.
 *
 * @param {Array} items - the items to order.
 * @param {string[]} [order] - the saved id order; a missing/empty order is a no-op.
 * @param {(item: any) => string} idOf - reads an item's id.
 * @returns {Array} the items in the saved order.
 */
function applyOrder(items, order, idOf)
{
  if (!order || !order.length) return items;
  const idx = new Map(order.map((id, i) => [id, i]));
  return items.slice().sort((a, b) => (idx.has(idOf(a)) ? idx.get(idOf(a)) : 1e9) - (idx.has(idOf(b)) ? idx.get(idOf(b)) : 1e9));
}

/**
 * Builds a fresh custom sensor for a group (numeric, discrete-state, or chain log).
 *
 * @param {string} groupId - the id of the group the sensor belongs to.
 * @param {string} presetId - the catalog preset id; falls back to the first preset.
 * @param {number} [value] - an explicit numeric value; defaults to the band midpoint.
 * @returns {object} the new sensor, in the snapshot shape.
 */
export function makeSensor(groupId, presetId, value)
{
  const p = catalog.sensorPresets.find((x) => x.id === presetId) || catalog.sensorPresets[0];
  const base = { id: uid('s'), groupId, battery: null, mode: 'active', events: [], custom: true };
  // A custom preset may pin its graphic and tile width; carry them onto the reading so it
  // renders the same way the profile intends.
  const pres = { ...(p.viz ? { viz: p.viz } : {}), ...(p.span ? { span: true } : {}), ...(p.stat ? { stat: true } : {}) };
  // A discrete sensor (valve, uplink) carries a state code instead of a numeric reading.
  if (p.state)
  {
    return { ...base, reading: { key: p.key, value: p.value || 0, unit: p.unit || 'state', status: 'ok', state: p.state, ...pres }, history: [] };
  }
  const v = Number.isFinite(value) ? value : (p.band ? (p.band[0] + p.band[1]) / 2 : (p.value ?? 0));
  return {
    ...base,
    reading: { key: p.key, value: v, unit: p.unit, status: statusFor(v, p.band), band: p.band, trend: 'steady', ...pres },
    history: Array.from({ length: 12 }, () => v),
  };
}

/**
 * Builds a fresh custom group for an org.
 *
 * @param {string} orgId - the id of the org the group belongs to.
 * @param {string} name - the group's display name.
 * @param {string} kind - the link kind, such as `'lora'`.
 * @returns {object} the new group, in the snapshot shape.
 */
export function makeGroup(orgId, name, kind)
{
  return { id: uid('g'), orgId, name: name || 'New group', link: { kind: kind || 'lora', strength: 3, online: true }, status: 'ok', sensors: [], custom: true };
}

/**
 * Applies the store's edits to a raw fleet snapshot, returning a new fleet.
 *
 * Removes and adds groups/sensors, applies the saved orderings, and recomputes group and
 * fleet status, without mutating the input.
 *
 * @param {object} raw - the raw fleet snapshot from the device.
 * @param {object} edits - the store's edit set (added/removed groups & sensors, orders).
 * @returns {object} a new fleet with the edits applied, or `raw` if it is falsy.
 */
export function applyEdits(raw, edits)
{
  if (!raw) return raw;
  const rmG = new Set(edits.rmGroups), rmS = new Set(edits.rmSensors);
  const gOrder = edits.groupOrder || {}, sOrder = edits.sensorOrder || {};
  let fleetStatus = 'ok';
  const orgs = raw.orgs.map((o) =>
  {
    let groups = o.groups.filter((g) => !rmG.has(g.id)).map((g) => ({ ...g }));
    groups = groups.concat(edits.addGroups.filter((ag) => ag.orgId === o.id));
    groups = applyOrder(groups, gOrder[o.id], (g) => g.id);
    groups = groups.map((g) =>
    {
      let sensors = (g.sensors || []).filter((s) => !rmS.has(g.id + '/' + s.id));
      sensors = sensors.concat(edits.addSensors.filter((s) => s.groupId === g.id));
      sensors = applyOrder(sensors, sOrder[g.id], (s) => s.id);
      let st = g.link && g.link.online === false ? 'warn' : 'ok';
      for (const s of sensors) st = worst(st, s.reading.status || 'ok');
      fleetStatus = worst(fleetStatus, st);
      return { ...g, sensors, status: st };
    });
    return { ...o, groups };
  });
  return { ...raw, orgs, status: fleetStatus };
}

/**
 * Reorders groups and sensors by the saved view order without adding or removing, so a
 * user's drag-to-reorder preference still applies when the device owns the structure.
 *
 * @param {object} raw - the raw fleet snapshot from the device.
 * @param {object} edits - the store's edit set; only its orderings are used.
 * @returns {object} a new fleet with only the orderings applied.
 */
function applyOrderOnly(raw, edits)
{
  const gOrder = edits.groupOrder || {}, sOrder = edits.sensorOrder || {};
  return {
    ...raw,
    orgs: raw.orgs.map((o) => ({
      ...o,
      groups: applyOrder(o.groups, gOrder[o.id], (g) => g.id).map((g) => ({ ...g, sensors: applyOrder(g.sensors || [], sOrder[g.id], (s) => s.id) })),
    })),
  };
}

/**
 * The fleet to render. Against a real device the device is the source of truth, so only
 * the local ordering preference is layered on; on a static host the full client-side edit
 * overlay applies. Recomputed on each read.
 *
 * @returns {object|null} the fleet to render, or `null` before the first frame.
 */
export const currentFleet = () =>
{
  const raw = fleet.value;
  if (!raw) return raw;
  return live.value ? applyOrderOnly(raw, store.state.edits) : applyEdits(raw, store.state.edits);
};

/**
 * Whether the local edit overlay holds any changes (added or removed groups or sensors, or a
 * custom ordering). Drives the demo's "reset" affordance.
 *
 * @returns {boolean} true if any local edit is present.
 */
export const hasLocalEdits = () =>
{
  const e = store.state.edits;
  return !!(e.addGroups.length || e.addSensors.length || e.rmGroups.length || e.rmSensors.length
    || Object.keys(e.groupOrder || {}).length || Object.keys(e.sensorOrder || {}).length);
};

/**
 * Builds the authenticated command for a provisioning operation.
 *
 * @param {string} kind - `'addGroup'`, `'addSensor'`, `'removeGroup'`, or `'removeSensor'`.
 * @param {object|string} payload - the built group/sensor, or the id/path to remove.
 * @returns {object} the command object to send.
 */
function toCommand(kind, payload)
{
  switch (kind)
  {
    case 'addGroup': return { type: 'addGroup', org: payload.orgId, group: payload };
    case 'addSensor':
    {
      // The hardware binding is device config, not display data, so it rides on the command
      // rather than the sensor the dashboard renders.
      const { binding, ...sensor } = payload;
      return { type: 'addSensor', group: sensor.groupId, sensor, binding: binding || undefined };
    }
    case 'removeGroup': return { type: 'removeGroup', id: payload };
    default: return { type: 'removeSensor', target: payload };
  }
}

/**
 * Applies a provisioning change: as an authenticated command when a device is present, or
 * as a local edit on a static host. Entering manage mode is gated on being unlocked, so a
 * live path here is already paired.
 *
 * @param {string} kind - the operation, matching a store edit action name.
 * @param {object|string} payload - the built group/sensor, or the id/path to remove.
 * @returns {Promise<{ok: boolean, error?: string}>} the outcome.
 */
export async function provision(kind, payload)
{
  if (!live.value) { store.dispatch(kind, payload); return { ok: true }; }
  if (!unlocked.value) return { ok: false, error: 'auth.not_paired' };
  return sendCommand(toCommand(kind, payload));
}
