import { store } from '../store.js';
import { currentFleet } from '../lib/edits.js';
import { openOverlay } from '../nav.js';
import { sensorDetailBody, stickLog } from '../lib/detail.js';
import { t, nf } from '../lib/i18n.js';
import { catalog } from '../lib/catalog.js';
import { LINK_NAMES, LINK_COLORS, realSensors, esc } from '../lib/viz/index.js';

const SR = 60;

/**
 * Whether the viewport is phone-width, where the two views take different shapes.
 *
 * @returns {boolean} true on a narrow screen.
 */
function narrow()
{
  return typeof matchMedia === 'function' && matchMedia('(max-width: 700px)').matches;
}

function dims(map)
{
  return !map && narrow() ? { W: 680, H: 920 } : { W: 1040, H: 660 };
}

$.component('network-view', {
  state: { tick: 0 },

  /** Initializes pan/zoom state, subscriptions, and document pointer listeners. */
  mounted()
  {
    this._z = 1; this._px = 0; this._py = 0; this._drag = null;
    this._un = store.subscribe(() => this.setState({}));
    this._eff = $.effect(() => { currentFleet(); this.setState({}); });
    this._move = (e) => { if (!this._drag) return; this._px = this._drag.px + (e.clientX - this._drag.x); this._py = this._drag.py + (e.clientY - this._drag.y); this.applyTransform(); };
    this._up = () => { this._drag = null; };
    document.addEventListener('pointermove', this._move);
    document.addEventListener('pointerup', this._up);
  },
  /** Re-applies the pan/zoom transform and re-pins the event log after a re-render. */
  updated() { this.applyTransform(); stickLog(this._el); },
  /** Tears down subscriptions and document pointer listeners. */
  destroyed()
  {
    if (this._un) this._un();
    if (typeof this._eff === 'function') this._eff();
    document.removeEventListener('pointermove', this._move);
    document.removeEventListener('pointerup', this._up);
  },

  /** Writes the current pan/zoom onto the scene group. */
  /**
   * Applies the current pan and zoom. The scale is taken about the drawing's own center, so
   * zooming holds what you are looking at instead of pulling it toward a corner.
   *
   * @returns {void}
   */
  applyTransform()
  {
    const g = this._el && this._el.querySelector('.net-scene');
    if (!g) return;
    const { W, H } = dims(this.tab() === 'map');
    const z = this._z;
    const x = this._px + (W / 2) * (1 - z);
    const y = this._py + (H / 2) * (1 - z);
    g.setAttribute('transform', `translate(${x.toFixed(2)} ${y.toFixed(2)}) scale(${z})`);
  },
  /** Zooms in one step. */
  zoomIn() { this._z = $.clamp(this._z * 1.2, 0.25, 3); this.applyTransform(); },
  /** Zooms out one step. */
  zoomOut() { this._z = $.clamp(this._z / 1.2, 0.25, 3); this.applyTransform(); },
  /** Resets pan and zoom to the default view. */
  /**
   * Fits the whole drawing in the stage. The map fills its stage and crops, so on a narrow
   * screen part of it starts out of view; this brings all of it back, at any size.
   *
   * @returns {void}
   */
  resetView()
  {
    const svg = this._el && this._el.querySelector('.net-svg');
    const { W, H } = dims(this.tab() === 'map');
    let z = 1;
    if (svg)
    {
      const box = svg.getBoundingClientRect();
      if (box.width && box.height)
      {
        const fill = Math.max(box.width / W, box.height / H);
        const fit = Math.min(box.width / W, box.height / H);
        z = Math.min(1, fit / fill);
      }
    }
    this._z = z; this._px = 0; this._py = 0; this.applyTransform();
  },
  /**
   * Zooms toward the wheel direction.
   *
   * @param {WheelEvent} e - the wheel event.
   * @returns {void}
   */
  onWheel(e) { e.preventDefault(); this._z = $.clamp(this._z * (e.deltaY < 0 ? 1.12 : 0.89), 0.25, 3); this.applyTransform(); },
  /**
   * Begins a pan drag, unless the pointer landed on a node or leaf.
   *
   * @param {PointerEvent} e - the pointer-down event.
   * @returns {void}
   */
  onDown(e) { if (e.button !== 0) return; if (e.target.closest('[data-sid]') || e.target.closest('[data-gid]')) return; this._drag = { x: e.clientX, y: e.clientY, px: this._px, py: this._py }; },

  /**
   * Reads which view the route addresses, so a link carries the map as well as the page.
   *
   * @returns {string} `'map'` or `'topology'`.
   */
  tab() { return (this.props.$params && this.props.$params.tab) === 'map' ? 'map' : 'topology'; },

  /**
   * Docks the sensor panel for the clicked leaf.
   *
   * @param {MouseEvent} e - the click event.
   * @returns {void}
   */
  onLeaf(e) { const el = e.target.closest('[data-sid]'); if (el) store.dispatch('setNetSensor', el.dataset.sid); },
  /**
   * Docks the inspect panel for the clicked group node.
   *
   * @param {MouseEvent} e - the click event.
   * @returns {void}
   */
  onNode(e) { const el = e.target.closest('[data-gid]'); if (el) store.dispatch('setNetInspect', el.dataset.gid); },
  /** Closes the docked inspect panel. */
  closeInspect() { store.dispatch('clearNetInspect'); },
  /** Closes the docked sensor panel. */
  closeSensorPanel() { store.dispatch('clearNetSensor'); },

  /**
   * Flattens the fleet into a flat list of every group.
   *
   * @param {object} f - the current fleet.
   * @returns {object[]} every group across all orgs.
   */
  groupsOf(f) { const out = []; for (const o of f.orgs) for (const g of o.groups) out.push(g); return out; },

  /**
   * Computes hub and group node positions for the current tab.
   *
   * @param {object[]} groups - the groups to place.
   * @param {boolean} map - whether to place by geo coordinates (map) or a ring (topology).
   * @returns {{hub: object, gpos: Array}} the hub and per-group placements.
   */
  positions(groups, map)
  {
    const { W, H } = dims(map);
    if (map)
    {
      const ax = 100, ay = 70, aw = W - 200, ah = H - 170;
      const at = (id) => catalog.sitePositions[id] || [0.5, 0.5];
      const gpos = groups.map((g) => { const [nx, ny] = at(g.id); return { g, x: ax + nx * aw, y: ay + ny * ah, ang: 0 }; });
      const [hx, hy] = catalog.sitePositions.__gateway;
      const hub = { x: ax + hx * aw, y: ay + hy * ah };
      gpos.forEach((p) => { p.ang = Math.atan2(p.y - hub.y, p.x - hub.x); });
      return { hub, gpos };
    }
    const n = groups.length || 1, hub = { x: W / 2, y: H / 2 - 6 };
    const portrait = H > W;
    const rx = portrait ? W * 0.36 : 232, ry = portrait ? H * 0.34 : 232;
    const gpos = groups.map((g, i) => { const ang = -Math.PI / 2 + (i / n) * Math.PI * 2; return { g, x: hub.x + rx * Math.cos(ang), y: hub.y + ry * Math.sin(ang), ang }; });
    return { hub, gpos };
  },

  /**
   * Builds the topology/map scene: the hub, edges, packets, group nodes, and leaves.
   *
   * @param {object} f - the current fleet.
   * @param {boolean} map - whether to render the geo map tab.
   * @returns {string} the scene SVG group markup.
   */
  scene(f, map)
  {
    const groups = this.groupsOf(f);
    const owner = {};
    for (const o of f.orgs) for (const gg of o.groups) owner[gg.id] = o.name;
    const { hub, gpos } = this.positions(groups, map);
    const backdrop = map ? this.mapBackdrop() : '';
    let edges = '', packets = '', nodes = '';
    gpos.forEach((p, i) =>
    {
      const color = LINK_COLORS[p.g.link.kind] || '#38e1ff';
      const online = p.g.link.online;
      const path = `M${hub.x.toFixed(1)},${hub.y.toFixed(1)} L${p.x.toFixed(1)},${p.y.toFixed(1)}`;
      const st = p.g.status;
      const pkColor = st === 'alarm' ? 'var(--alarm)' : st === 'warn' ? 'var(--warn)' : color;
      const pkClass = st === 'alarm' ? 'net-pk hot' : 'net-pk';
      edges += `<line x1="${hub.x.toFixed(1)}" y1="${hub.y.toFixed(1)}" x2="${p.x.toFixed(1)}" y2="${p.y.toFixed(1)}" class="net-edge ${online ? '' : 'down'}" data-status="${st}" style="stroke:${color}"/>`;
      if (online) { const dur = (st === 'alarm' ? 1.2 : st === 'warn' ? 1.8 : 2.4 + i * 0.2).toFixed(2); for (let k = 0; k < 2; k++) { const b = (k * dur / 2).toFixed(2); packets += `<circle r="3.6" class="${pkClass}" style="fill:${pkColor}" opacity="0"><animateMotion dur="${dur}s" begin="${b}s" repeatCount="indefinite" path="${path}"/><animate attributeName="opacity" values="0;1;1;0" keyTimes="0;0.1;0.9;1" dur="${dur}s" begin="${b}s" repeatCount="indefinite"/></circle>`; } }
      if (!map)
      {
        const leaves = realSensors(p.g);
        const m = leaves.length || 1;
        leaves.forEach((s, j) =>
        {
          const la = p.ang + (-0.55 + (m === 1 ? 0.275 : (j / (m - 1)) * 1.1));
          const lx = p.x + SR * Math.cos(la), ly = p.y + SR * Math.sin(la);
          nodes += `<line x1="${p.x.toFixed(1)}" y1="${p.y.toFixed(1)}" x2="${lx.toFixed(1)}" y2="${ly.toFixed(1)}" class="net-twig"/>`;
          nodes += `<circle cx="${lx.toFixed(1)}" cy="${ly.toFixed(1)}" r="6" class="net-leaf" data-sid="${p.g.id}/${s.id}" data-status="${s.reading.status}" @click="onLeaf"><title>${esc(t('label.' + s.reading.key))}</title></circle>`;
        });
      }
      // On the ring a label falls outward by angle; on the map the positions are real, so
      // it falls away from the gateway instead, which is what stops the labels colliding
      // where several nodes sit close to the hub.
      const below = map ? p.y >= hub.y : Math.sin(p.ang) >= 0;
      nodes += `<g class="net-node ${store.state.netInspect === p.g.id ? 'sel' : ''}" data-status="${p.g.status}" data-gid="${p.g.id}" style="--lc:${color}" @click="onNode">
        <circle cx="${p.x.toFixed(1)}" cy="${p.y.toFixed(1)}" r="13" class="net-gn"/>
        <circle cx="${p.x.toFixed(1)}" cy="${p.y.toFixed(1)}" r="13" class="net-gn-ring"/>
        ${p.g.status !== 'ok' ? `<circle cx="${(p.x + 11).toFixed(1)}" cy="${(p.y - 11).toFixed(1)}" r="4.5" class="net-badge" data-status="${p.g.status}"/>` : ''}
      </g>
      <text class="net-label" x="${p.x.toFixed(1)}" y="${(p.y + (below ? 34 : -30)).toFixed(1)}" text-anchor="middle">${esc(p.g.name)}</text>
      <text class="net-sub" x="${p.x.toFixed(1)}" y="${(p.y + (below ? 50 : -14)).toFixed(1)}" text-anchor="middle">${esc(owner[p.g.id] || '')}${online ? '' : ' · ' + t('ui.offline')}</text>`;
    });
    return `<g class="net-scene">${backdrop}${edges}${packets}${nodes}
      <g class="net-hub"><circle cx="${hub.x.toFixed(1)}" cy="${hub.y.toFixed(1)}" r="26" class="net-hub-glow"/><circle cx="${hub.x.toFixed(1)}" cy="${hub.y.toFixed(1)}" r="18" class="net-hub-core"/><text x="${hub.x.toFixed(1)}" y="${(hub.y + 4).toFixed(1)}" text-anchor="middle" class="net-hub-t">⌂</text></g>
      <text x="${hub.x.toFixed(1)}" y="${(hub.y - 32).toFixed(1)}" text-anchor="middle" class="net-label">${t('ui.gateway')}</text></g>`;
  },

  /**
   * Renders the decorative rural site backdrop for the map tab (contours, water, fields,
   * roads, place labels). It pans and zooms with the scene.
   *
   * @returns {string} the backdrop SVG group markup.
   */
  mapBackdrop()
  {
    const { W, H } = dims(true);
    const ax = 100, ay = 70, aw = W - 200, ah = H - 170;
    const P = (nx, ny) => [+(ax + nx * aw).toFixed(1), +(ay + ny * ah).toFixed(1)];
    const hub = P(...catalog.sitePositions.__gateway);
    // Roads run to the two landmarks and to whichever mapped groups sit beyond them.
    const ends = [P(0.23, 0.21), P(0.80, 0.28), ...['solar', 'river'].filter((id) => catalog.sitePositions[id]).map((id) => P(...catalog.sitePositions[id]))];
    const road = (a, b) => `<path class="net-road" d="M${a[0]} ${a[1]} Q${(a[0] + b[0]) / 2 + 20} ${(a[1] + b[1]) / 2 - 20} ${b[0]} ${b[1]}"/>`;

    let grid = '';
    for (let i = 1; i < 8; i++) grid += `<line class="net-grat" x1="${(W / 8) * i}" y1="0" x2="${(W / 8) * i}" y2="${H}"/>`;
    for (let i = 1; i < 6; i++) grid += `<line class="net-grat" x1="0" y1="${(H / 6) * i}" x2="${W}" y2="${(H / 6) * i}"/>`;

    // A lake, a winding river feeding it, a forest patch, and irregular fields.
    const water = `<path class="net-water" d="M70 470 q70 -80 190 -55 q95 18 78 100 q-26 95 -160 88 q-150 -8 -108 -133 z"/>`;
    const river = `<path class="net-river" d="M300 60 q34 150 -36 224 q-66 70 -16 150 q34 56 -16 90"/>`;
    const forest = `<path class="net-forest" d="M768 372 q104 -34 156 50 q24 92 -78 124 q-126 22 -158 -70 q-22 -84 80 -128 z"/>`;
    const fields = `
      <path class="net-field" d="M642 110 q120 -28 196 26 q34 86 -52 132 q-150 40 -196 -44 q-30 -78 52 -114 z"/>
      <path class="net-field b" d="M150 120 q92 -26 168 18 q30 70 -40 120 q-130 36 -160 -44 q-22 -66 32 -94 z"/>`;
    const contours = `
      <path class="net-contour" d="M812 300 q96 50 50 168 q-66 100 -196 86"/>
      <path class="net-contour" d="M790 320 q70 48 36 138 q-50 78 -150 74"/>`;
    const roads = `<g class="net-roads">${ends.map((end) => road(hub, end)).join('')}</g>`;

    const compass = `<g class="net-compass" transform="translate(${W - 88} 120)"><circle r="22" class="net-comp-ring"/><path d="M0 -16 L5 4 L0 0 L-5 4 Z" class="net-comp-n"/><text y="-24" text-anchor="middle" class="net-comp-t">N</text></g>`;
    const scale = `<g class="net-scale" transform="translate(88 ${H - 108})"><line x1="0" y1="0" x2="120" y2="0"/><line x1="0" y1="-4" x2="0" y2="4"/><line x1="120" y1="-4" x2="120" y2="4"/><text x="60" y="-8" text-anchor="middle">2 km</text></g>`;
    const places = `
      <text class="net-place" x="200" y="118">North uplands</text>
      <text class="net-place" x="840" y="470">East ridge</text>
      <text class="net-place net-river-t" x="246" y="180">River</text>
      <text class="net-place net-river-t" x="120" y="420">Lake</text>`;
    return `<g class="net-bg">${grid}${water}${river}${forest}${fields}${contours}${roads}${compass}${scale}${places}</g>`;
  },

  /**
   * Resolves a sensor and its org/group by its `gid/sid` key.
   *
   * @param {object} f - the current fleet.
   * @param {string} sid - the `gid/sid` key.
   * @returns {{org: object, group: object, sensor: object}|null} the match, or null.
   */
  findSensor(f, sid)
  {
    if (!sid) return null;
    const [gid, sid2] = sid.split('/');
    for (const o of f.orgs) for (const g of o.groups) { if (g.id !== gid) continue; const s = g.sensors.find((x) => x.id === sid2); if (s) return { org: o, group: g, sensor: s }; }
    return null;
  },

  /**
   * Renders the docked sensor panel for the selected leaf.
   *
   * @param {object} f - the current fleet.
   * @returns {string} the panel markup, or an empty string when none is selected.
   */
  sensorPanel(f)
  {
    const found = this.findSensor(f, store.state.netSensor); if (!found) return '';
    const { group, sensor: s } = found;
    return `
      <div class="net-detail" data-status="${s.reading.status}">
        <div class="ins-head"><div><div class="ins-title">${esc(t('label.' + s.reading.key))}</div><div class="ins-sub">${esc(group.name)}</div></div>
          <button class="modal-close sm" type="button" @click="closeSensorPanel">✕</button></div>
        ${sensorDetailBody(s)}
      </div>`;
  },

  /**
   * Renders the docked inspect panel for the selected group node (link debug + issues).
   *
   * @param {object} f - the current fleet.
   * @returns {string} the panel markup, or an empty string when none is selected.
   */
  inspectPanel(f)
  {
    const id = store.state.netInspect; if (!id) return '';
    const g = this.groupsOf(f).find((x) => x.id === id); if (!g) return '';
    const spec = catalog.linkSpec[g.link.kind] || catalog.linkSpec.lora;
    const rssi = spec.rssi + (g.link.strength - 2) * 6;
    const issues = g.sensors.filter((s) => s.reading.status !== 'ok');
    const pkts = Math.round(4 + realSensors(g).length * 1.5);
    const row = (k, v) => `<div class="ins-row"><span>${k}</span><b>${v}</b></div>`;
    const issuesHtml = issues.length
      ? issues.map((s) => `<div class="ins-issue" data-level="${s.reading.status === 'alarm' ? 'error' : 'warn'}">${esc(t('label.' + s.reading.key))} · ${nf(s.reading.value)}${t('unit.' + s.reading.unit)}</div>`).join('')
      : `<div class="ins-ok">${t('ui.allClear')}</div>`;
    return `
      <div class="net-inspect" data-status="${g.status}">
        <div class="ins-head"><div><div class="ins-title">${esc(g.name)}</div><div class="ins-sub">${LINK_NAMES[g.link.kind] || g.link.kind} · ${t('status.' + g.status)}</div></div>
          <button class="modal-close sm" type="button" @click="closeInspect">✕</button></div>
        ${row(t('ui.link'), (g.link.online ? t('ui.online') : t('ui.offline')))}
        ${row(t('ui.signal'), `${rssi} dBm · ${g.link.strength}/4`)}
        ${row(t('ui.throughput'), spec.speed)}
        ${row(t('ui.latency'), `${spec.lat} ms`)}
        ${row(t('ui.packets'), `${pkts}/s`)}
        ${row(t('ui.sensors'), nf(realSensors(g).length))}
        <div class="ins-sec">${t('ui.issues')}</div>
        <div class="ins-issues">${issuesHtml}</div>
        <button class="ins-open" type="button" data-gid="${g.id}" @click="onOpenGroup">${t('ui.group')} →</button>
      </div>`;
  },

  /**
   * Opens the group view for the clicked node.
   *
   * @param {MouseEvent} e - the click event.
   * @returns {void}
   */
  onOpenGroup(e)
  {
    const el = e.target.closest('[data-gid]'); if (!el) return;
    const gid = el.dataset.gid;
    openOverlay(() => store.dispatch('setGroupView', gid), () => store.dispatch('clearGroupView'));
  },

  /**
   * Renders the network overlay with its tabs, scene, zoom controls, panels, and legend.
   *
   * @returns {string} the overlay markup, or an empty placeholder when closed.
   */
  render()
  {
    const f = currentFleet();
    if (!f) return '<div class="shell"></div>';
    const map = this.tab() === 'map';
    const { W, H } = dims(map);
    return `
      <div class="shell net-page">
        <header class="net-head">
          <div class="net-head-main">
            <a class="net-back" href="#/" z-link="/">${esc(t('ui.backToFleet'))}</a>
            <h1 class="net-title">${t('ui.network')}</h1>
            <p class="net-subtitle">${nf(this.groupsOf(f).length)} ${t('ui.groups')} · ${t('ui.networkHint')}</p>
          </div>
          <nav class="net-tabs" aria-label="${esc(t('ui.network'))}">
            <a class="net-tab ${!map ? 'on' : ''}" href="#/network" z-link="/network" aria-current="${!map}">${t('ui.topology')}</a>
            <a class="net-tab ${map ? 'on' : ''}" href="#/network/map" z-link="/network/map" aria-current="${map}">${t('ui.map')}</a>
          </nav>
        </header>
        <div class="net-stage">
          <svg class="net-svg${map ? ' fill' : ''}" viewBox="0 0 ${W} ${H}" preserveAspectRatio="${map ? 'xMidYMid slice' : 'xMidYMid meet'}" role="img" aria-label="${esc(t('ui.networkHint'))}" @wheel="onWheel" @pointerdown="onDown">
            ${this.scene(f, map)}
          </svg>
          <div class="net-zoom">
            <button type="button" @click="zoomIn" aria-label="${esc(t('ui.zoomIn'))}">+</button>
            <button type="button" @click="zoomOut" aria-label="${esc(t('ui.zoomOut'))}">\u2212</button>
            <button type="button" @click="resetView" aria-label="${esc(t('ui.zoomReset'))}">\u27f2</button>
          </div>
          ${this.inspectPanel(f)}
          ${this.sensorPanel(f)}
        </div>
        <p class="net-legend">${Object.keys(LINK_NAMES).map((k) => `<span class="net-leg"><i style="background:${LINK_COLORS[k]}"></i>${LINK_NAMES[k]}</span>`).join('')}</p>
      </div>`;
  },
});
