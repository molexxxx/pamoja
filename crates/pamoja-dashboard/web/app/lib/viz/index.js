// viz/index.js - the visualization barrel and dispatch.
//
// Picks the visualization that suits each reading and renders it, and re-exports the
// helpers components reach for so the rest of the app imports visualizations from one
// place. Every visualization is hand-drawn inline SVG or CSS: no images, no chart
// library. The per-kind builders live alongside in gauges/glyphs/charts/links.

import { nf, fmt, t } from '../i18n.js';
import { catalog } from '../catalog.js';
import { esc, statusColor, trendArrow } from './util.js';
import { LINK_NAMES, LINK_COLORS, LINK_RSSI, bars, conn } from './links.js';
import { miniSpark, detailGraph, bannerRing, bigBars } from './charts.js';
import { radial, therm, droplet, battery, dial, wind, sun, barViz } from './gauges.js';
import { count, mesh, wave, chain, chip, valve } from './glyphs.js';

/**
 * Picks the visualization kind for a reading from its element key and unit.
 *
 * @param {string} key - the stable element key, such as `"soil_moisture"`.
 * @param {string} unit - the canonical unit, such as `"celsius"` or `"percent"`.
 * @returns {string} the visualization kind, such as `"therm"` or `"mesh"`.
 */
export function vizFor(key, unit)
{
  if (key.includes('mesh')) return 'mesh';
  if (unit === 'record') return 'chain';
  if (key.endsWith('valve')) return 'valve';
  if (unit === 'count') return 'count';
  if (unit === 'state') return 'chip';
  if (key.endsWith('_trend')) return 'spark';
  if (unit === 'celsius') return 'therm';
  if (key === 'humidity' || key.includes('moisture') || unit === 'millimeter') return 'droplet';
  if (unit === 'hectopascal' || unit === 'liter_per_minute') return 'dial';
  if (unit === 'meter_per_second') return 'wind';
  if (unit === 'lux') return 'sun';
  if (unit === 'decibel') return 'wave';
  if (unit === 'volt' || key === 'battery_voltage' || key === 'battery_level') return 'battery';
  if (unit === 'watt') return 'bar';
  if (unit === 'percent') return 'radial';
  return 'spark';
}

/** The visualization kinds the renderer can draw, for validating an explicit choice. */
const KNOWN_KINDS = new Set([
  'spark', 'radial', 'dial', 'bar', 'therm', 'droplet', 'battery', 'wind', 'sun',
  'wave', 'chip', 'valve', 'chain', 'mesh', 'count',
]);

/** The visualization kinds that read best spanning two columns. */
const WIDE_KINDS = new Set(['spark', 'chain', 'wave', 'mesh']);

/**
 * Picks the visualization kind for a reading, honoring an explicit choice before the
 * heuristic: the reading's own `viz`, else the catalog preset declared for its key, else
 * {@link vizFor} from the key and unit.
 *
 * @param {{key: string, unit: string, viz?: string}} r - the reading.
 * @returns {string} the visualization kind.
 */
export function vizOf(r)
{
  if (!r) return 'spark';
  if (r.viz && KNOWN_KINDS.has(r.viz)) return r.viz;
  const preset = catalog.sensorPresets.find((p) => p.key === r.key && p.viz);
  if (preset && KNOWN_KINDS.has(preset.viz)) return preset.viz;
  return vizFor(r.key, r.unit);
}

/**
 * Whether a reading's tile should span two columns: a wide visualization kind, or a
 * `span` flag on the reading or its catalog preset.
 *
 * @param {object} r - the reading.
 * @returns {boolean} `true` for a wide tile.
 */
export function isWide(r)
{
  if (r && r.span) return true;
  const preset = r && catalog.sensorPresets.find((p) => p.key === r.key);
  if (preset && preset.span) return true;
  return WIDE_KINDS.has(vizOf(r));
}

/**
 * Reports whether a reading uses a discrete glyph rather than a numeric gauge.
 *
 * @param {{key: string, unit: string, viz?: string}} r - the reading.
 * @returns {boolean} `true` for chip/valve/chain/mesh/count/wave visualizations.
 */
export function isDiscrete(r)
{
  const k = vizOf(r);
  return k === 'chip' || k === 'valve' || k === 'chain' || k === 'mesh' || k === 'count' || k === 'wave';
}

/**
 * Whether a sensor entry is a node or network stat (the device flags it), not a measurement.
 *
 * @param {object} s - the sensor entry.
 * @returns {boolean} `true` for a node stat.
 */
export const isStat = (s) => !!(s.reading && s.reading.stat);

/**
 * Whether a sensor entry is the mesh-map tile (the topology view), not a sensor.
 *
 * @param {object} s - the sensor entry.
 * @returns {boolean} `true` for the mesh-map entry.
 */
export const isMeshMap = (s) => vizOf(s.reading) === 'mesh';

/**
 * A group's real sensors: its entries minus node stats and the mesh-map tile. This is what
 * every "N sensors" count and per-sensor visualization should use.
 *
 * @param {object} g - the group.
 * @returns {Array} the real sensor entries.
 */
export const realSensors = (g) => (g.sensors || []).filter((s) => !isStat(s) && !isMeshMap(s));

/**
 * A group's node stats, in declaration order.
 *
 * @param {object} g - the group.
 * @returns {Array} the stat entries.
 */
export const groupStats = (g) => (g.sensors || []).filter(isStat);

/**
 * Groups a node's real sensors into the mesh peers (stations) that host them: sensors that
 * share a `peer` name collapse onto one station; an unnamed sensor is its own station. Each
 * station carries its position from the first member that has one.
 *
 * @param {object} g - the group.
 * @returns {Array<{name: string, sensors: Array, lat: ?number, lon: ?number}>} the stations.
 */
export const meshStations = (g) =>
{
  const order = [];
  const byKey = new Map();
  for (const s of realSensors(g))
  {
    const key = s.peer || ('@' + s.id);
    if (!byKey.has(key)) { byKey.set(key, { name: s.peer || t('label.' + s.reading.key), sensors: [], lat: null, lon: null }); order.push(key); }
    const st = byKey.get(key);
    st.sensors.push(s);
    if (st.lat == null && s.lat != null) { st.lat = s.lat; st.lon = s.lon; }
  }
  return order.map((k) => byKey.get(k));
};

/**
 * The number of mesh peers to draw for a group: its declared `neighbors` stat, or the
 * number of peer stations it hosts when none is given. Shared by the tile preview and the
 * full mesh map so the two never disagree.
 *
 * @param {object} g - the group.
 * @returns {number} the peer count, 1 to 14.
 */
export const meshPeerCount = (g) =>
{
  const n = (g.sensors || []).find((s) => s.reading.key === 'neighbors');
  const stat = n ? Math.max(0, Math.round(n.reading.value)) : 0;
  return Math.min(14, Math.max(1, stat || Math.max(meshStations(g).length, 3)));
};

/**
 * Renders the chosen visualization for a sensor, sized by `big`.
 *
 * @param {object} s - the sensor, carrying `reading`, `history`, and `id`.
 * @param {boolean} [big=false] - whether to render the expanded view.
 * @param {number} [nodes] - the mesh peer count, for the mesh visualization.
 * @returns {string} the visualization markup, wrapped in its sizing container.
 */
export function tileViz(s, big = false, nodes)
{
  const r = s.reading;
  const uid = (big ? 'b' : 't') + (s.id || r.key).replace(/[^a-z0-9]/gi, '');
  const vk = vizOf(r);
  let inner, full = false, disc = false;
  switch (vk)
  {
    case 'therm': inner = therm(r, big); break;
    case 'droplet': inner = droplet(r, uid, big); break;
    case 'radial': inner = radial(r, big); break;
    case 'dial': inner = dial(r, big); break;
    case 'wind': inner = wind(r, big); break;
    case 'sun': inner = sun(r, big); break;
    case 'battery': inner = battery(r, uid); break;
    case 'chip': inner = chip(r, big); disc = true; break;
    case 'valve': inner = valve(r, big); disc = true; break;
    case 'chain': inner = chain(r, big); disc = true; break;
    case 'wave': inner = wave(r, big); disc = true; break;
    case 'mesh': inner = mesh(r, big, nodes); disc = true; break;
    case 'count': inner = count(r, big); disc = true; break;
    case 'bar': inner = barViz(r.value, r.band, big, t('unit.' + r.unit)); full = true; break;
    default: inner = big ? bigBars(s.history, r.band) : miniSpark(s.history); full = true;
  }
  // Discrete glyphs (chip/valve) size to their own content rather than the fixed gauge slot.
  const cls = disc ? `tv-wrap disc tv-${vk}` : `tv-wrap tv-${vk}${full ? ' full' : ''}${big ? ' big' : ''}`;
  return `<div class="${cls}">${inner}</div>`;
}

export { nf, fmt };
export { esc, statusColor, trendArrow };
export { LINK_NAMES, LINK_COLORS, LINK_RSSI, bars, conn };
export { detailGraph, bannerRing };
