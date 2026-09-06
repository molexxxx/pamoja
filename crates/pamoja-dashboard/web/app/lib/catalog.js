/**
 * The live layout catalog. Mutated in place by {@link mergeCatalog} so existing imports
 * keep seeing the current configuration.
 *
 * @type {{
 *   sensorPresets: Array<object>,
 *   linkKinds: string[],
 *   linkSpec: Object<string, {speed: string, lat: number, rssi: number}>,
 *   sitePositions: Object<string, [number, number]>,
 *   theme: Object<string, string>,
 * }}
 */
export const catalog = {
  // Sensor types a user can add, each carrying the canonical key/unit/band.
  sensorPresets: [
    { id: 'temperature', key: 'temperature', unit: 'celsius', band: [2, 8] },
    { id: 'humidity', key: 'humidity', unit: 'percent', band: [30, 60] },
    { id: 'soc', key: 'state_of_charge', unit: 'percent', band: [20, 100] },
    { id: 'power', key: 'pv_power', unit: 'watt', band: [0, 400] },
    { id: 'pressure', key: 'pressure', unit: 'hectopascal', band: [980, 1040] },
    { id: 'wind', key: 'wind_speed', unit: 'meter_per_second', band: [0, 20] },
    { id: 'light', key: 'illuminance', unit: 'lux', band: [0, 100000] },
    { id: 'voltage', key: 'battery_voltage', unit: 'volt', band: [11.8, 14.6] },
    // Field-kit sensors (Farm node, Health post).
    { id: 'soil', key: 'soil_moisture', unit: 'percent', band: [36, 100] },
    { id: 'well', key: 'well_level', unit: 'percent', band: [20, 100] },
    { id: 'soiltrend', key: 'soil_trend', unit: 'percent', band: [0, 100] },
    { id: 'fridge', key: 'fridge_temp', unit: 'celsius', band: [2, 8] },
    { id: 'wardpower', key: 'ward_power', unit: 'percent', band: [50, 100] },
    { id: 'oxygen', key: 'oxygen_stock', unit: 'percent', band: [30, 100] },
    { id: 'flow', key: 'flow_rate', unit: 'liter_per_minute', band: [0, 16] },
    { id: 'tank', key: 'storage_tank', unit: 'percent', band: [20, 100] },
    { id: 'flowtrend', key: 'flow_trend', unit: 'liter_per_minute', band: [0, 16] },
    // Ranger relay / mesh node.
    { id: 'acoustic', key: 'acoustic', unit: 'decibel', band: [0, 120] },
    { id: 'batterylevel', key: 'battery_level', unit: 'percent', band: [20, 100] },
    // Mesh-node stats: telemetry about the node, not measurements - flagged as stats and
    // only offered on mesh-link groups.
    { id: 'neighbours', key: 'neighbours', unit: 'count', value: 5, band: [1, 12], stat: true, meshOnly: true },
    { id: 'hops', key: 'hops', unit: 'count', value: 3, band: [1, 8], stat: true, meshOnly: true },
    { id: 'relayed', key: 'messages_relayed', unit: 'count', value: 300, band: [0, 99999], stat: true, meshOnly: true },
    // Discrete (state chip / valve / mesh) and the tamper-evident chain log.
    { id: 'valve', key: 'drip_valve', unit: 'state', state: 'state.closed' },
    { id: 'uplink', key: 'uplink', unit: 'state', state: 'state.synced', stat: true },
    { id: 'pump', key: 'pump_health', unit: 'state', state: 'state.nominal' },
    { id: 'relaystatus', key: 'relay_status', unit: 'state', state: 'state.online', stat: true, meshOnly: true },
    { id: 'routing', key: 'routing', unit: 'state', state: 'mesh.optimised', stat: true, meshOnly: true },
    // The mesh map is one sensor that only applies to mesh-link groups; its value is the
    // number of mesh nodes drawn.
    { id: 'meshrelay', key: 'mesh_relay', unit: 'state', state: 'mesh.optimised', value: 5, meshOnly: true },
    { id: 'tamper', key: 'tamper_log', unit: 'record', value: 1000, stat: true },
  ],

  // Link kinds a user can pick when creating a group.
  linkKinds: ['lora', 'wifi', 'cellular', 'satellite', 'ethernet', 'mesh'],

  // Nominal per-link characteristics for the network and mesh inspection panels:
  // throughput, round-trip latency (ms), and floor RSSI (dBm).
  linkSpec: {
    lora: { speed: '5 kbps', lat: 780, rssi: -112 },
    wifi: { speed: '24 Mbps', lat: 11, rssi: -52 },
    cellular: { speed: '1.4 Mbps', lat: 58, rssi: -84 },
    nbiot: { speed: '62 kbps', lat: 240, rssi: -102 },
    satellite: { speed: '128 kbps', lat: 640, rssi: -118 },
    ethernet: { speed: '100 Mbps', lat: 3, rssi: -40 },
    mesh: { speed: '42 kbps', lat: 130, rssi: -96 },
  },

  // Demo site positions (normalized 0..1) for the network map tab, spread so nodes and
  // labels never collide. A real build would use each group's GPS over a tile basemap.
  sitePositions: {
    __gateway: [0.47, 0.45],
    'cold-chain': [0.29, 0.15],
    maternity: [0.17, 0.27],
    'silo-3': [0.72, 0.20],
    weather: [0.88, 0.36],
    solar: [0.66, 0.78],
    river: [0.36, 0.69],
  },

  // Theme tokens a device/profile may override (accent, status colors, glyph stroke).
  // Empty by default, so the page keeps its built-in palette until a catalog supplies one.
  theme: {},
};

/**
 * Tests whether a value is a plain object (and not an array), for the deep merge.
 *
 * @param {*} v - the value to test.
 * @returns {boolean} `true` for a plain object.
 */
const isPlainObject = (v) => v != null && typeof v === 'object' && !Array.isArray(v);

/**
 * Deep-merges a partial catalog from a device or profile manifest over the defaults.
 *
 * Plain-object maps (such as `linkSpec` and `sitePositions`) are merged key by key, so
 * a manifest can adjust one link without restating them all; arrays and primitives
 * (such as `sensorPresets` and `linkKinds`) are replaced wholesale when supplied. The
 * shared {@link catalog} is mutated in place, so every importer sees the update.
 *
 * @param {object} [partial] - the catalog fields to override; ignored if falsy.
 * @returns {object} the updated {@link catalog}.
 */
export function mergeCatalog(partial)
{
  if (!isPlainObject(partial)) return catalog;
  for (const [key, value] of Object.entries(partial))
  {
    if (isPlainObject(value) && isPlainObject(catalog[key]))
    {
      catalog[key] = { ...catalog[key], ...value };
    }
    else
    {
      catalog[key] = value;
    }
  }
  return catalog;
}

/**
 * Folds a device-served catalog into the defaults *additively*: custom presets are
 * appended (or merged by `id`) onto the built-in ones rather than replacing them, so a
 * profile's elements appear alongside the standard set. The theme and link maps merge by
 * key; `linkKinds`, when given, replaces wholesale.
 *
 * @param {object} [served] - the catalog a device serves at `GET /catalog`; ignored if falsy.
 * @returns {object} the updated {@link catalog}.
 */
export function extendCatalog(served)
{
  if (!isPlainObject(served)) return catalog;
  if (Array.isArray(served.sensorPresets))
  {
    const byId = new Map(catalog.sensorPresets.map((p) => [p.id, p]));
    for (const p of served.sensorPresets) byId.set(p.id, { ...byId.get(p.id), ...p });
    catalog.sensorPresets = [...byId.values()];
  }
  if (isPlainObject(served.theme)) catalog.theme = { ...catalog.theme, ...served.theme };
  if (isPlainObject(served.linkSpec)) catalog.linkSpec = { ...catalog.linkSpec, ...served.linkSpec };
  if (isPlainObject(served.sitePositions)) catalog.sitePositions = { ...catalog.sitePositions, ...served.sitePositions };
  if (Array.isArray(served.linkKinds)) catalog.linkKinds = served.linkKinds;
  return catalog;
}

/**
 * Whether a sensor preset is offered on a group with the given link kind. Honors the
 * declared `scope` (`'always'` or `{ links: [...] }`) and the legacy `meshOnly` flag.
 *
 * @param {object} p - the sensor preset.
 * @param {string} linkKind - the target group's link kind, such as `'mesh'`.
 * @returns {boolean} `true` when the preset should be offered on that group.
 */
export function scopeAllows(p, linkKind)
{
  if (p.meshOnly) return linkKind === 'mesh';
  const scope = p.scope;
  if (!scope || scope === 'always') return true;
  if (Array.isArray(scope.links)) return scope.links.includes(linkKind);
  return true;
}
