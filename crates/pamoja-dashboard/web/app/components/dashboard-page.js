import { store } from '../store.js';
import { open } from '../nav.js';
import { t, nf, fmt } from '../lib/i18n.js';
import { currentFleet, provision, hasLocalEdits } from '../lib/edits.js';
import { live } from '../lib/feed.js';
import { unlocked, promptUnlock } from '../lib/pair.js';
import { conn, tileViz, bannerRing, trendArrow, isDiscrete, isStat, realSensors, groupStats, meshPeerCount, vizOf, isWide, esc } from '../lib/viz/index.js';
import { isNew } from '../lib/discovery.js';
import { openMeshOverlay } from './mesh-modal.js';

const ICON_EDIT = '<svg class="ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h9"/><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19.5l-4 1 1-4z"/></svg>';
const ICON_DONE = '<svg class="ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>';
const ICON_EXPAND = '<svg class="ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 3H5a2 2 0 0 0-2 2v3M16 3h3a2 2 0 0 1 2 2v3M21 16v3a2 2 0 0 1-2 2h-3M3 16v3a2 2 0 0 0 2 2h3"/></svg>';
const ICON_DRAG = '<svg class="ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 9 2 12l3 3M9 5l3-3 3 3M15 19l-3 3-3-3M19 9l3 3-3 3M2 12h20M12 2v20"/></svg>';
const ICON_RESET = '<svg class="ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 3-6.7L3 8"/><path d="M3 3v5h5"/></svg>';

/**
 * Counts the non-mesh sensors in a group, the peer count the mesh preview draws.
 *
 * @param {object} g - the group.
 * @returns {number} the count of non-mesh sensors.
 */
/**
 * Computes the worst sensor status across an org, so its dot can carry the org's health.
 *
 * @param {object} o - the org.
 * @returns {string} the worst status, one of `'ok'`, `'warn'`, or `'alarm'`.
 */
function orgStatus(o)
{
  let worst = 'ok';
  for (const g of o.groups) for (const s of g.sensors)
  {
    if (s.reading.status === 'alarm') return 'alarm';
    if (s.reading.status === 'warn') worst = 'warn';
  }
  return worst;
}

/**
 * Reports whether a `gid/sid` key refers to a mesh-map sensor in the current fleet.
 *
 * @param {string} sid - the `gid/sid` key.
 * @returns {boolean} `true` if the sensor uses the mesh visualization.
 */
function isMeshSensor(sid)
{
  const f = currentFleet();
  if (!f) return false;
  const [gid, sd] = sid.split('/');
  for (const o of f.orgs) for (const g of o.groups) if (g.id === gid)
  {
    const s = g.sensors.find((x) => x.id === sd);
    return !!s && vizOf(s.reading) === 'mesh';
  }
  return false;
}

$.component('dashboard-page', {
  state: { tick: 0, orgOpen: false },

  /** Subscribes to fleet/store changes, observes layout, and wires drag reordering. */
  mounted()
  {
    this._place = {};
    this._stop = $.effect(() => { currentFleet(); if (!this._drag) this.setState({}); });
    this._un = store.subscribe(() => { if (!this._drag) this.setState({}); });
    this._ro = new ResizeObserver(() => this.scheduleLayout());
    this._onResize = () => this.scheduleLayout();
    window.addEventListener('resize', this._onResize);
    this.bindDrag();
  },
  /** Re-observes the grid and cards after each re-render so layout stays current. */
  updated()
  {
    const grid = this._el && this._el.querySelector('.groups');
    if (!grid) return;
    if (!grid._obs) { grid._obs = true; this._ro.observe(grid); }
    grid.querySelectorAll('.gcard').forEach((c) => { if (!c._obs) { c._obs = true; this._ro.observe(c); } });
  },
  /** Tears down subscriptions, the resize observer, and any pending layout frame. */
  destroyed()
  {
    if (typeof this._stop === 'function') this._stop();
    if (this._un) this._un();
    if (this._ro) this._ro.disconnect();
    if (this._onResize) window.removeEventListener('resize', this._onResize);
    if (this._raf) cancelAnimationFrame(this._raf);
  },

  /** Schedules a masonry relayout on the next animation frame, coalescing bursts. */
  scheduleLayout() { if (!this._raf) this._raf = requestAnimationFrame(() => this.layout()); },

  /**
   * Masonry: cards keep their reading order across columns (card i in column i%cols) and
   * each stacks at its column's running bottom, so the only thing removed is the gap between
   * rows; the add-group card, last in order, lands in the next cell after the final group.
   * Placement is cached so render() re-emits it inline, since the morph strips JS-set styles.
   * One column drops to natural stacking (see .groups.mono).
   *
   * @returns {void}
   */
  layout()
  {
    this._raf = 0;
    const grid = this._el && this._el.querySelector('.groups');
    if (!grid) return;
    const cards = [...grid.children].filter((c) => c.classList.contains('gcard'));
    if (!cards.length) return;
    const colGap = parseFloat(getComputedStyle(grid).columnGap) || 18;
    // Derive the column count from the container width and the CSS min track (372px), not
    // from the live grid - our explicit placements would otherwise pin the desktop column
    // count on a phone and squish the cards instead of collapsing.
    const cols = Math.max(1, Math.floor((grid.clientWidth + colGap) / (372 + colGap)));
    // _mono is re-emitted by render() so the morph keeps the class between ticks.
    this._mono = cols <= 1;
    grid.classList.toggle('mono', this._mono);
    if (this._mono)
    {
      this._place = {};
      cards.forEach((c) => { c.style.gridColumn = ''; c.style.gridRow = ''; });
      return;
    }
    const gap = 18;
    const colBottom = new Array(cols).fill(1);
    cards.forEach((c, i) =>
    {
      const col = i % cols;
      const h = Math.max(1, Math.ceil(c.offsetHeight));
      const row = colBottom[col];
      c.style.gridColumn = String(col + 1);
      c.style.gridRow = row + ' / span ' + h;
      colBottom[col] = row + h + gap;
      this._place[c.dataset.gid || '__add'] = { c: col + 1, r: row, s: h };
    });
  },

  /** Binds drag-and-drop reordering for group cards and sensor tiles on the root. */
  bindDrag()
  {
    const root = this._el;
    root.addEventListener('dragstart', (e) =>
    {
      const el = e.target.closest('[draggable="true"]');
      if (!el) return;
      const group = el.classList.contains('gcard');
      const stat = el.classList.contains('statcard');
      const sel = group ? '.gcard[draggable]' : stat ? '.statcard[draggable]' : '.stile[draggable]';
      this._drag = { el, container: el.parentNode, sel, group };
      el.classList.add('dragging');
      e.dataTransfer.effectAllowed = 'move';
      try { e.dataTransfer.setData('text/plain', ''); } catch (err) { /* Safari */ }
    });
    root.addEventListener('dragover', (e) =>
    {
      if (!this._drag) return;
      e.preventDefault();
      const over = e.target.closest(this._drag.sel);
      if (!over || over === this._drag.el || over.parentNode !== this._drag.container) return;
      const rect = over.getBoundingClientRect();
      const before = (e.clientX - rect.left) < rect.width / 2;
      const ref = before ? over : over.nextSibling;
      if (ref !== this._drag.el) { this._drag.container.insertBefore(this._drag.el, ref); if (this._drag.group) this.scheduleLayout(); }
    });
    root.addEventListener('drop', (e) => { if (this._drag) e.preventDefault(); });
    root.addEventListener('dragend', () => this.endDrag());
  },

  /** Commits a finished drag: persists the new group or sensor order and relays out. */
  endDrag()
  {
    if (!this._drag) return;
    const { el, container, group } = this._drag;
    el.classList.remove('dragging');
    this._drag = null;
    if (group)
    {
      const ids = [...container.querySelectorAll(':scope > .gcard[data-gid]')].map((c) => c.dataset.gid);
      const org = this.selectedOrg(currentFleet());
      if (org) store.dispatch('reorderGroups', { orgId: org.id, ids });
    } else
    {
      const card = container.closest('.gcard');
      const gid = card && card.dataset.gid;
      if (gid)
      {
        // Persist the whole group order: sensor tiles then stat cards (each section keeps
        // its own order; the data list is a single ordered list).
        const ids = [
          ...card.querySelectorAll('.sensors > .stile[data-sid]'),
          ...card.querySelectorAll('.statcards > .statcard[data-sid]'),
        ].map((c) => c.dataset.sid.split('/')[1]);
        store.dispatch('reorderSensors', { gid, ids });
      }
    }
    this.setState({});
    this.scheduleLayout();
  },

  /**
   * Opens a sensor's detail (or the mesh overlay for a mesh sensor) from a tile click.
   *
   * @param {MouseEvent} e - the click event.
   * @returns {void}
   */
  onSensor(e)
  {
    if (e.target.closest('.tile-rm')) return;
    const el = e.target.closest('[data-sid]'); if (!el) return;
    const sid = el.dataset.sid;
    if (isMeshSensor(sid)) { openMeshOverlay(sid); return; }
    open(() => store.dispatch('selectSensor', sid), () => store.dispatch('closeSensor'));
  },
  /**
   * Removes a sensor in Manage mode.
   *
   * @param {MouseEvent} e - the click event.
   * @returns {void}
   */
  onRemoveSensor(e) { e.stopPropagation(); const el = e.target.closest('[data-key]'); if (el) provision('removeSensor', el.dataset.key); },
  /**
   * Removes a group in Manage mode.
   *
   * @param {MouseEvent} e - the click event.
   * @returns {void}
   */
  onRemoveGroup(e) { const el = e.target.closest('[data-gid]'); if (el) provision('removeGroup', el.dataset.gid); },
  /**
   * Opens the group view for the clicked group's expand button.
   *
   * @param {MouseEvent} e - the click event.
   * @returns {void}
   */
  onOpenGroup(e) { const el = e.target.closest('[data-gid]'); if (el) { const gid = el.dataset.gid; open(() => store.dispatch('setGroupView', gid), () => store.dispatch('clearGroupView')); } },
  /**
   * Opens the add-sensor dialog for the clicked group.
   *
   * @param {MouseEvent} e - the click event.
   * @returns {void}
   */
  onAddSensor(e) { const el = e.target.closest('[data-gid]'); if (el) { const gid = el.dataset.gid; open(() => store.dispatch('openCreate', { mode: 'sensor', groupId: gid }), () => store.dispatch('closeCreate')); } },
  /**
   * Opens the add-group dialog for the clicked org.
   *
   * @param {MouseEvent} e - the click event.
   * @returns {void}
   */
  onAddGroup(e) { const el = e.target.closest('[data-oid]'); if (el) { const oid = el.dataset.oid; open(() => store.dispatch('openCreate', { mode: 'group', orgId: oid }), () => store.dispatch('closeCreate')); } },

  /**
   * Resolves the org addressed by the route, defaulting to the first org.
   *
   * @param {object} f - the current fleet.
   * @returns {object|null} the selected org, or null when there are none.
   */
  selectedOrg(f)
  {
    const orgs = f.orgs || [];
    const id = this.props.$params && this.props.$params.id;
    return orgs.find((o) => o.id === id) || orgs[0] || null;
  },

  /**
   * Renders the dashboard shell: banner, org tabs, and the selected org's groups.
   *
   * @returns {string} the page markup.
   */
  render()
  {
    const f = currentFleet();
    if (!f)
    {
      return `<div class="shell"><section class="banner"><div class="banner-text"><span class="banner-eyebrow">${t('ui.status')}</span><h1 class="banner-title">${t('ui.connecting')}</h1></div></section></div>`;
    }
    return `<div class="shell">${this.banner(f)}${this.orgtabs(f)}${this.groups(f)}</div>`;
  },

  /**
   * Renders the status banner with fleet-wide counts.
   *
   * @param {object} f - the current fleet.
   * @returns {string} the banner markup.
   */
  banner(f)
  {
    let groups = 0, sensors = 0, alarms = 0, warns = 0;
    for (const o of f.orgs) for (const g of o.groups)
    {
      groups++;
      sensors += realSensors(g).length;
      // Alarm and warn tallies still include stats - a stat (a lost relay, a dropped uplink)
      // can be the most urgent thing on a node.
      for (const s of g.sensors) { if (s.reading.status === 'alarm') alarms++; else if (s.reading.status === 'warn') warns++; }
    }
    return `
      <section class="banner" data-status="${f.status}">
        <div class="banner-ring">${bannerRing(f.status)}</div>
        <div class="banner-text">
          <span class="banner-eyebrow">${t('ui.status')}</span>
          <h1 class="banner-title">${t('ui.hero.' + f.status)}</h1>
          <span class="banner-sub">${nf(f.orgs.length)} ${t('ui.orgs')} · ${nf(groups)} ${t('ui.groups')} · ${t('ui.sensorsCount', { n: sensors })}</span>
        </div>
        <div class="banner-stats">
          <div class="bstat"><b>${nf(sensors)}</b><span>${t('ui.sensors')}</span></div>
          <div class="bstat" data-tone="${warns ? 'warn' : ''}"><b>${nf(warns)}</b><span>${t('status.warn')}</span></div>
          <div class="bstat" data-tone="${alarms ? 'alarm' : ''}"><b>${nf(alarms)}</b><span>${t('status.alarm')}</span></div>
        </div>
      </section>`;
  },

  /**
   * Renders the org selector dropdown and the Manage toggle.
   *
   * @param {object} f - the current fleet.
   * @returns {string} the org-bar markup.
   */
  orgtabs(f)
  {
    const sel = this.selectedOrg(f) || f.orgs[0];
    const open = this.state.orgOpen;
    const menu = f.orgs.map((o) => `<a class="orgsel-item ${sel && o.id === sel.id ? 'on' : ''}" z-link="/org/${o.id}" @click="closeOrg">
        <span class="dotc" data-status="${orgStatus(o)}"></span><span class="orgsel-iname">${esc(o.name)}</span><span class="count">${nf(o.groups.length)}</span></a>`).join('');
    return `<div class="orgbar">
      <div class="orgsel ${open ? 'open' : ''}" @click.outside="closeOrg">
        <button class="orgsel-btn" type="button" @click="toggleOrg" aria-expanded="${open ? 'true' : 'false'}">
          <span class="dotc" data-status="${sel ? orgStatus(sel) : 'ok'}"></span>
          <span class="orgsel-cur">${esc(sel ? sel.name : t('ui.orgs'))}</span>
          <span class="count">${sel ? nf(sel.groups.length) : ''}</span>
          <svg class="orgsel-chev" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M6 9l6 6 6-6"/></svg>
        </button>
        <div class="orgsel-menu">${menu}</div>
      </div>
      ${!live.value && hasLocalEdits() ? `<button class="manage-btn reset-btn" type="button" @click="onReset" title="${esc(t('ui.reset'))}">${ICON_RESET}<span>${t('ui.reset')}</span></button>` : ''}
      <button class="manage-btn ${store.state.editing ? 'on' : ''}" type="button" @click="onManage">${store.state.editing ? ICON_DONE : ICON_EDIT}<span>${store.state.editing ? t('ui.done') : t('ui.manage')}</span></button>
    </div>`;
  },

  /** Discards the demo's local edit overlay, restoring the original fleet. */
  onReset() { store.dispatch('resetEdits'); this.scheduleLayout(); this.setState({}); },

  /** Toggles the org selector dropdown. */
  toggleOrg() { this.setState({ orgOpen: !this.state.orgOpen }); },
  /** Closes the org selector dropdown. */
  closeOrg() { if (this.state.orgOpen) this.setState({ orgOpen: false }); },
  /** Toggles Manage mode. */
  onManage()
  {
    // Against a real device, managing the fleet sends authenticated commands, so entering
    // manage mode requires control to be unlocked first. A static host edits locally.
    if (!store.state.editing && live.value && !unlocked.value) { promptUnlock(); return; }
    store.dispatch('toggleEditing');
  },

  /**
   * Renders the masonry grid of group cards for the selected org, plus an add card in
   * Manage mode.
   *
   * @param {object} f - the current fleet.
   * @returns {string} the groups-grid markup.
   */
  groups(f)
  {
    const org = this.selectedOrg(f);
    if (!org) return '';
    const editing = store.state.editing;
    const ap = (this._place || {}).__add;
    const astyle = ap ? `grid-column:${ap.c};grid-row:${ap.r} / span ${ap.s}` : 'grid-row:auto / span 170';
    const add = editing ? `<button class="gcard add-card" data-oid="${org.id}" z-key="__add" @click="onAddGroup" style="${astyle}"><span class="add-plus">+</span> ${t('ui.addGroup')}</button>` : '';
    const touch = typeof window.matchMedia === 'function' && window.matchMedia('(pointer: coarse)').matches;
    const hint = editing ? `<div class="manage-hint">${ICON_DRAG}<span>${esc(t(touch ? 'ui.dragHintTouch' : 'ui.dragHint'))}</span></div>` : '';
    return `${hint}<div class="groups${this._mono ? ' mono' : ''}">${org.groups.map((g) => this.groupCard(g, editing)).join('')}${add}</div>`;
  },

  /**
   * Renders one group card with its header, sensor tiles, and footer.
   *
   * @param {object} g - the group.
   * @param {boolean} editing - whether Manage mode is active.
   * @returns {string} the group-card markup.
   */
  groupCard(g, editing)
  {
    const rm = editing ? `<button class="icon-btn danger" data-gid="${g.id}" @click="onRemoveGroup" aria-label="${esc(t('ui.remove'))}">✕</button>` : '';
    const addS = editing ? `<button class="stile add-tile" data-gid="${g.id}" z-key="__addtile" @click="onAddSensor"><span class="add-plus">+</span><span>${t('ui.addSensor')}</span></button>` : '';
    const p = this._place && this._place[g.id];
    const style = p ? `grid-column:${p.c};grid-row:${p.r} / span ${p.s}` : 'grid-row:auto / span 320';
    const fresh = isNew(g.id);
    return `
      <article class="gcard${fresh ? ' is-new' : ''}" data-status="${g.status}" data-gid="${g.id}" z-key="${g.id}" style="${style}"${editing ? ' draggable="true"' : ''}>
        <div class="ghead">
          <div class="gtitle-wrap">
            <div class="gtitle">${esc(g.name)}${fresh ? `<span class="new-badge">${esc(t('ui.new'))}</span>` : ''}</div>
            <div class="gmeta"><span class="gstatus"><span class="dotc"></span>${t('status.' + g.status)}</span></div>
          </div>
          <div class="ghead-conn">${conn(g.link)}</div>
          ${rm}
        </div>
        <div class="sensors">
          ${g.sensors.filter((s) => !isStat(s)).map((s) => this.sensorTile(g, s, editing)).join('')}
          ${addS}
        </div>
        ${this.statStrip(g, editing)}
        <div class="gfoot">
          <button class="gexpand" type="button" data-gid="${g.id}" @click="onOpenGroup" aria-label="${esc(t('ui.group'))}" title="${esc(t('ui.group'))}">${ICON_EXPAND}</button>
        </div>
      </article>`;
  },

  /**
   * Renders one sensor tile with its label, optional readout, and visualization.
   *
   * @param {object} g - the group the sensor belongs to.
   * @param {object} s - the sensor.
   * @param {boolean} editing - whether Manage mode is active.
   * @returns {string} the sensor-tile markup.
   */
  sensorTile(g, s, editing)
  {
    const r = s.reading;
    const sid = g.id + '/' + s.id;
    const battery = s.battery != null ? `<span class="sbatt">${nf(s.battery, { style: 'percent', maximumFractionDigits: 0 })}</span>` : '';
    const rm = editing ? `<button class="tile-rm" data-key="${sid}" @click="onRemoveSensor" aria-label="${esc(t('ui.remove'))}">✕</button>` : '';
    const readout = isDiscrete(r)
      ? ''
      : `<div><span class="sval">${fmt(r.value)}</span><span class="sunit">${t('unit.' + r.unit)}</span>${r.trend ? `<span class="strend" data-dir="${r.trend}">${trendArrow(r.trend)}</span>` : ''}</div>`;
    const vk = vizOf(r);
    const span = isWide(r) ? ' span' : '';
    const nodes = vk === 'mesh' ? meshPeerCount(g) : undefined;
    const fresh = isNew(sid);
    return `
      <article class="stile${span}${fresh ? ' is-new' : ''} ${store.state.selected === sid ? 'open' : ''}" data-status="${r.status}" data-sid="${sid}" z-key="${s.id}" @click="onSensor" tabindex="0" role="button"${editing ? ' draggable="true"' : ''}>
        ${rm}
        <div class="stop"><span class="slabel">${esc(t('label.' + r.key))}${fresh ? `<span class="new-badge">${esc(t('ui.new'))}</span>` : ''}</span><span class="sdot"></span></div>
        ${battery}
        ${readout}
        ${tileViz(s, false, nodes)}
      </article>`;
  },

  /**
   * Renders the group's node-stats strip (neighbours, hops, link/relay status, ...), shown
   * apart from sensors and never counted as one. Empty when the group has no stats.
   *
   * @param {object} g - the group.
   * @param {boolean} editing - whether Manage mode is active.
   * @returns {string} the stats-strip markup, or an empty string.
   */
  statStrip(g, editing)
  {
    const stats = groupStats(g);
    if (!stats.length) return '';
    return `<div class="statstrip">
        <span class="statstrip-h">${esc(t('ui.stats'))}</span>
        <div class="statcards">${stats.map((s) => this.statCard(g, s, editing)).join('')}</div>
      </div>`;
  },

  /**
   * Renders one compact node-stat card (label + value or state); clicking opens its detail.
   *
   * @param {object} g - the group the stat belongs to.
   * @param {object} s - the stat entry.
   * @param {boolean} editing - whether Manage mode is active.
   * @returns {string} the stat-card markup.
   */
  statCard(g, s, editing)
  {
    const r = s.reading;
    const sid = g.id + '/' + s.id;
    const rm = editing ? `<button class="tile-rm" data-key="${sid}" @click="onRemoveSensor" aria-label="${esc(t('ui.remove'))}">✕</button>` : '';
    const unit = t('unit.' + r.unit);
    const value = r.state ? t(r.state) : `${fmt(r.value)}${unit ? ' ' + unit : ''}`;
    const fresh = isNew(sid);
    return `<article class="statcard${fresh ? ' is-new' : ''}" data-status="${r.status}" data-sid="${sid}" z-key="${s.id}" @click="onSensor" tabindex="0" role="button"${editing ? ' draggable="true"' : ''}>
        ${rm}
        <span class="statcard-dot"></span>
        <div class="statcard-text"><span class="statlabel">${esc(t('label.' + r.key))}${fresh ? `<span class="new-badge">${esc(t('ui.new'))}</span>` : ''}</span>
        <span class="statval">${esc(value)}</span></div>
      </article>`;
  },
});
