import { store } from '../store.js';
import { currentFleet } from '../lib/edits.js';
import { open, back } from '../nav.js';
import { t, nf, fmt } from '../lib/i18n.js';
import { catalog } from '../lib/catalog.js';
import { LINK_NAMES, LINK_COLORS, LINK_RSSI, meshStations, meshPeerCount, esc } from '../lib/viz/index.js';

// The map is laid out in a viewBox that turns portrait on a phone, so a narrow screen gets a
// tall graph that fills the panel instead of a short, wide one squeezed to the width.
function dims()
{
  const phone = typeof matchMedia === 'function' && matchMedia('(max-width: 560px)').matches;
  return phone ? { W: 620, H: 880 } : { W: 900, H: 560 };
}

/**
 * Finds a group and its owning org by group id.
 *
 * @param {object} f - the current fleet.
 * @param {string} gid - the group id.
 * @returns {{org: object, group: object}|null} the match, or null.
 */
function findGroup(f, gid)
{
  for (const o of f.orgs) for (const g of o.groups) if (g.id === gid) return { org: o, group: g };
  return null;
}

/**
 * Closes the mesh overlay one layer at a time: first a docked node, then the map.
 *
 * @returns {void}
 */
function closeMesh()
{
  if (store.state.meshNode) { store.dispatch('clearMeshNode'); open(() => { }, closeMesh); return; }
  store.dispatch('closeMeshView');
}

/**
 * Opens the mesh map overlay for a sensor id (gid/sid).
 *
 * @param {string} sid - the `gid/sid` key of the mesh sensor.
 * @returns {void}
 */
export function openMeshOverlay(sid)
{
  open(() => store.dispatch('openMeshView', sid), closeMesh);
}

$.component('mesh-modal', {
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
  /** Re-applies the pan/zoom transform after a re-render. */
  updated() { this.applyTransform(); },
  /** Tears down subscriptions and document pointer listeners. */
  destroyed()
  {
    if (this._un) this._un();
    if (typeof this._eff === 'function') this._eff();
    document.removeEventListener('pointermove', this._move);
    document.removeEventListener('pointerup', this._up);
  },

  /** Writes the current pan/zoom onto the scene group. */
  applyTransform() { const g = this._el && this._el.querySelector('.mm-scene'); if (g) g.setAttribute('transform', `translate(${this._px} ${this._py}) scale(${this._z})`); },
  /** Zooms in one step. */
  zoomIn() { this._z = $.clamp(this._z * 1.2, 0.5, 3); this.applyTransform(); },
  /** Zooms out one step. */
  zoomOut() { this._z = $.clamp(this._z / 1.2, 0.5, 3); this.applyTransform(); },
  /** Resets pan and zoom to the default view. */
  resetView() { this._z = 1; this._px = 0; this._py = 0; this.applyTransform(); },
  /**
   * Zooms toward the wheel direction.
   *
   * @param {WheelEvent} e - the wheel event.
   * @returns {void}
   */
  onWheel(e) { e.preventDefault(); this._z = $.clamp(this._z * (e.deltaY < 0 ? 1.12 : 0.89), 0.5, 3); this.applyTransform(); },
  /**
   * Begins a pan drag, unless the pointer landed on a node.
   *
   * @param {PointerEvent} e - the pointer-down event.
   * @returns {void}
   */
  onDown(e) { if (e.button !== 0) return; if (e.target.closest('[data-node]')) return; this._drag = { x: e.clientX, y: e.clientY, px: this._px, py: this._py }; },
  /** Closes the overlay and unwinds one history entry. */
  close() { store.dispatch('closeMeshView'); back(); },
  /**
   * Closes the overlay when the scrim itself is clicked.
   *
   * @param {MouseEvent} e - the click event.
   * @returns {void}
   */
  onOverlay(e) { if (e.target.classList.contains('net-overlay')) this.close(); },
  /**
   * Docks the inspector for the clicked node.
   *
   * @param {MouseEvent} e - the click event.
   * @returns {void}
   */
  onNode(e) { const el = e.target.closest('[data-node]'); if (el) store.dispatch('setMeshNode', el.dataset.node); },
  /** Closes the docked node inspector. */
  closeNode() { store.dispatch('clearMeshNode'); },
  /**
   * Opens the full detail modal for a sensor hosted on the inspected peer.
   *
   * @param {MouseEvent} e - the click event.
   * @returns {void}
   */
  viewSensor(e)
  {
    const el = e.target.closest('[data-sid]'); if (!el) return;
    open(() => store.dispatch('selectSensor', el.dataset.sid), () => store.dispatch('closeSensor'));
  },

  /**
   * Reads a node-stat value by reading key, rounded, or a default when the stat is absent.
   *
   * @param {object} group - the group to read from.
   * @param {string} key - the stat reading key, such as `"neighbours"`.
   * @param {number} dflt - the value to use when the stat is missing.
   * @returns {number} the rounded stat value.
   */
  statVal(group, key, dflt)
  {
    const s = (group.sensors || []).find((x) => x.reading.key === key);
    return s ? Math.max(0, Math.round(s.reading.value)) : dflt;
  },

  /**
   * The worst status across a station's sensors, so its node carries the alarming one.
   *
   * @param {Array} sensors - the station's sensors.
   * @returns {string} `'alarm'`, `'warn'`, or `'ok'`.
   */
  stationStatus(sensors)
  {
    if (sensors.some((s) => s.reading.status === 'alarm')) return 'alarm';
    if (sensors.some((s) => s.reading.status === 'warn')) return 'warn';
    return 'ok';
  },

  /**
   * Builds the real mesh topology for a node: this node (hub), the multi-hop route up to the
   * gateway (length from the `hops` stat), and its mesh peers. Each peer is a station hosting
   * the node's sensors (grouped by their `peer` name); when the node and its stations carry
   * coordinates they are placed by real position, otherwise fanned. Remaining slots up to the
   * `neighbours` count are routing-only peers, so a sensor relay and a bare routing node draw
   * different graphs.
   *
   * @param {object} group - the group to lay out.
   * @returns {{nodes: Array, links: Array, packets: Array, pos: object, neighbours: number, hops: number, geo: boolean}} the topology.
   */
  topology(group)
  {
    const { W, H } = dims();
    const cx = W * 0.5, hubY = H * 0.6, gwY = H * 0.12;
    const stations = meshStations(group);
    const neighbours = meshPeerCount(group);
    const hops = $.clamp(this.statVal(group, 'hops', 2), 1, 8);
    // Geo layout when the node and all its stations have coordinates; else the fanned arc.
    const geo = group.lat != null && stations.length > 0 && stations.every((st) => st.lat != null);
    let proj = null;
    if (geo)
    {
      const k = Math.cos((group.lat * Math.PI) / 180);
      const pts = stations.map((st) => ({ dx: (st.lon - group.lon) * k, dy: st.lat - group.lat }));
      const m = Math.max(1e-6, ...pts.map((p) => Math.max(Math.abs(p.dx), Math.abs(p.dy))));
      proj = pts.map((p) => ({ x: cx + (p.dx / m) * W * 0.32, y: hubY - (p.dy / m) * H * 0.26 }));
    }

    const hub = { key: 'hub', role: 'hub', x: cx, y: hubY, name: group.name, status: group.status, link: group.link, group };
    const gw = { key: 'gw', role: 'gateway', x: cx, y: gwY, status: 'ok' };
    const nodes = [gw, hub];

    // The route to the gateway: `hops` edges, so `hops - 1` relay nodes between hub and gw.
    const routeKeys = ['hub'];
    for (let i = 1; i < hops; i++)
    {
      const key = 'r' + i;
      routeKeys.push(key);
      nodes.push({
        key, role: 'relay', idx: i, total: hops, status: 'ok',
        x: cx + (i % 2 ? 1 : -1) * W * 0.055,
        y: hubY + (gwY - hubY) * (i / hops),
      });
    }
    routeKeys.push('gw');

    const fanPos = (j) =>
    {
      const tt = neighbours === 1 ? 0.5 : j / (neighbours - 1);
      const a = Math.PI * 0.14 + tt * Math.PI * 0.72;
      const jit = 0.9 + 0.1 * Math.abs(Math.sin(j * 2.3));
      return { x: cx + Math.cos(a) * W * 0.31 * jit, y: hubY + Math.sin(a) * H * 0.3 * jit };
    };

    // The first peers are the sensor stations; the rest are routing-only neighbours.
    for (let j = 0; j < neighbours; j++)
    {
      const st = stations[j] || null;
      const p = st && geo ? proj[j] : fanPos(j);
      nodes.push({
        key: 'n' + j, role: 'peer', idx: j + 1, status: st ? this.stationStatus(st.sensors) : 'ok',
        x: p.x, y: p.y,
        station: st,
        sensors: st ? st.sensors : null,
        name: st ? st.name : null,
        coords: st && st.lat != null ? `${st.lat.toFixed(4)}, ${st.lon.toFixed(4)}` : null,
        sid: st && st.sensors.length === 1 ? group.id + '/' + st.sensors[0].id : null,
      });
    }

    const links = [];
    for (let i = 0; i < routeKeys.length - 1; i++) links.push([routeKeys[i], routeKeys[i + 1]]);
    for (let j = 0; j < neighbours; j++) links.push(['hub', 'n' + j]);

    const packets = [routeKeys.slice()];
    for (let j = 0; j < neighbours; j++) packets.push(['n' + j, 'hub']);

    const pos = {}; nodes.forEach((nd) => { pos[nd.key] = nd; });
    return { nodes, links, packets, pos, neighbours, hops, geo };
  },

  /**
   * Renders the docked inspector for a selected node (gateway, hub, route relay, or peer
   * station). A station hosting several sensors lists each, openable.
   *
   * @param {object} group - the group being inspected.
   * @param {object} [node] - the selected node; an empty node renders nothing.
   * @param {{neighbours: number, hops: number}} topo - the topology counts.
   * @returns {string} the inspector markup.
   */
  inspector(group, node, topo)
  {
    if (!node) return '';
    const dbm = (LINK_RSSI[group.link.kind] ?? -90) + (group.link.strength - 2) * 6;
    const speed = catalog.linkSpec[group.link.kind]?.speed || '-';
    const row = (k, v) => `<div class="ins-row"><span>${k}</span><b>${v}</b></div>`;
    let title, sub, body, foot = '';
    if (node.role === 'gateway')
    {
      title = t('ui.gateway'); sub = LINK_NAMES[group.link.kind] || group.link.kind;
      body = row(t('ui.link'), t('ui.online')) + row(t('ui.throughput'), speed) + row(t('ui.latency'), '12 ms');
    } else if (node.role === 'hub')
    {
      title = group.name; sub = `${LINK_NAMES[group.link.kind] || group.link.kind} · ${t('status.' + group.status)}`;
      body = row(t('ui.signal'), `${dbm} dBm`) + row(t('label.neighbours'), nf(topo.neighbours)) + row(t('label.hops'), nf(topo.hops));
    } else if (node.role === 'relay')
    {
      title = t('ui.meshRelay'); sub = t('ui.meshHopOf', { n: node.idx, total: node.total });
      const lat = 18 + node.idx * 14;
      body = row(t('ui.link'), t('ui.online')) + row(t('ui.throughput'), speed) + row(t('ui.latency'), `${lat} ms`);
    } else if (node.station)
    {
      title = node.name;
      sub = node.coords ? `${t('ui.meshPeer')} · ${node.coords}` : `${t('ui.meshPeer')} · ${t('status.' + node.status)}`;
      body = node.sensors.map((s) =>
      {
        const rd = s.reading;
        const reading = rd.state ? t(rd.state) : `${fmt(rd.value)} ${t('unit.' + rd.unit)}`;
        return `<button class="ins-srow" type="button" data-sid="${group.id}/${s.id}" @click="viewSensor" data-status="${rd.status}"><span>${esc(t('label.' + rd.key))}</span><b>${esc(reading)}</b></button>`;
      }).join('');
    } else
    {
      title = t('ui.meshPeer'); sub = t('ui.meshPeerOf', { n: node.idx, total: topo.neighbours });
      const traffic = 3 + (node.idx % 5);
      body = row(t('ui.signal'), `${dbm + 4 - node.idx} dBm`) + row(t('ui.throughput'), `${traffic}/s`) + row(t('ui.latency'), `${40 + traffic * 6} ms`);
    }
    return `<div class="net-inspect" data-status="${node.status || 'ok'}">
        <div class="ins-head"><div><div class="ins-title">${esc(title)}</div><div class="ins-sub">${esc(sub)}</div></div>
          <button class="modal-close sm" type="button" @click="closeNode" aria-label="${esc(t('ui.cancel'))}">✕</button></div>
        ${body}${foot}
      </div>`;
  },

  /**
   * Renders the mesh overlay for the active mesh sensor's group.
   *
   * @returns {string} the overlay markup, or an empty placeholder when inactive.
   */
  render()
  {
    const id = store.state.meshView;
    if (!id) return '<div hidden></div>';
    const f = currentFleet();
    if (!f) return '<div hidden></div>';
    const found = findGroup(f, id.split('/')[0]);
    if (!found) return '<div hidden></div>';
    const group = found.group;
    const { W, H } = dims();
    const color = LINK_COLORS[group.link.kind] || '#38e1ff';
    const { nodes, links, packets, pos, neighbours, hops } = this.topology(group);
    const sel = store.state.meshNode;

    const linkSvg = links.map((l) => `<line x1="${pos[l[0]].x.toFixed(1)}" y1="${pos[l[0]].y.toFixed(1)}" x2="${pos[l[1]].x.toFixed(1)}" y2="${pos[l[1]].y.toFixed(1)}" class="mm-link"/>`).join('');
    const pkSvg = packets.map((p, i) =>
    {
      const path = p.map((k, j) => `${j ? 'L' : 'M'}${pos[k].x.toFixed(1)},${pos[k].y.toFixed(1)}`).join(' ');
      const dur = (2.4 + i * 0.4).toFixed(2), b = (i * 0.5).toFixed(2);
      return `<circle r="3.4" class="mm-pk" opacity="0"><animateMotion dur="${dur}s" begin="${b}s" repeatCount="indefinite" path="${path}"/><animate attributeName="opacity" values="0;1;1;0" keyTimes="0;0.12;0.88;1" dur="${dur}s" begin="${b}s" repeatCount="indefinite"/></circle>`;
    }).join('');
    const nodeSvg = nodes.map((nd) =>
    {
      const hasSensor = nd.role === 'peer' && !!nd.station;
      const r = nd.role === 'gateway' ? 16 : nd.role === 'hub' ? 18 : nd.role === 'relay' ? 9 : hasSensor ? 11 : 8;
      const cls = `mm-node ${nd.role}${hasSensor ? ' sensor' : ''}${sel === nd.key ? ' sel' : ''}`;
      const glyph = nd.role === 'gateway' ? `<text class="mm-gly" x="${nd.x.toFixed(1)}" y="${(nd.y + 4).toFixed(1)}" text-anchor="middle">⌂</text>` : '';
      const named = nd.role === 'gateway' || nd.role === 'hub' || hasSensor;
      const label = named ? `<text class="mm-label" x="${nd.x.toFixed(1)}" y="${(nd.y + r + 14).toFixed(1)}" text-anchor="middle">${esc(nd.role === 'gateway' ? t('ui.gateway') : nd.name)}</text>` : '';
      return `<g class="${cls}" data-node="${nd.key}" data-status="${nd.status || 'ok'}" @click="onNode"><circle cx="${nd.x.toFixed(1)}" cy="${nd.y.toFixed(1)}" r="${r}" class="mm-dot"/></g>${glyph}${label}`;
    }).join('');

    return `
      <div class="net-overlay" @click="onOverlay">
        <div class="net-panel" data-status="${group.status}" style="--lc:${color}" role="dialog" aria-modal="true">
          <div class="net-head">
            <div>
              <div class="net-title">${esc(group.name)}</div>
              <div class="net-subtitle">${LINK_NAMES[group.link.kind] || group.link.kind} · ${nf(neighbours)} ${t('label.neighbours')} · ${nf(hops)} ${t('label.hops')}</div>
            </div>
            <div class="spacer"></div>
            <button class="modal-close" type="button" @click="close" aria-label="${esc(t('ui.cancel'))}">✕</button>
          </div>
          <div class="net-stage">
            <svg class="net-svg" viewBox="0 0 ${W} ${H}" preserveAspectRatio="xMidYMid meet" @wheel="onWheel" @pointerdown="onDown">
              <g class="mm-scene">${linkSvg}${pkSvg}${nodeSvg}</g>
            </svg>
            <div class="net-zoom">
              <button type="button" @click="zoomIn" aria-label="zoom in">+</button>
              <button type="button" @click="zoomOut" aria-label="zoom out">−</button>
              <button type="button" @click="resetView" aria-label="reset">⟲</button>
            </div>
            ${this.inspector(group, sel ? pos[sel] : null, { neighbours, hops })}
          </div>
        </div>
      </div>`;
  },
});
