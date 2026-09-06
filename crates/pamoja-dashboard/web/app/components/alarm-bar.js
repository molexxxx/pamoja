import { store } from '../store.js';
import { currentFleet } from '../lib/edits.js';
import { open, back } from '../nav.js';
import { t, nf, fmt } from '../lib/i18n.js';
import { esc } from '../lib/viz/index.js';

/**
 * Collects every sensor needing attention plus offline groups, alarms first.
 *
 * @param {object} f - the current fleet, or a falsy value before the first frame.
 * @returns {Array<object>} the problem rows, each carrying `org`, `group`, and either
 *   `sensor` or `link: true`.
 */
export function problems(f)
{
  const out = [];
  if (!f) return out;
  for (const o of f.orgs) for (const g of o.groups)
  {
    for (const s of g.sensors)
    {
      if (s.reading.status !== 'ok') out.push({ org: o, group: g, sensor: s });
    }
    if (!g.link.online) out.push({ org: o, group: g, link: true });
  }
  return out.sort((a, b) => (b.sensor?.reading.status === 'alarm' ? 1 : 0) - (a.sensor?.reading.status === 'alarm' ? 1 : 0));
}

$.component('alarm-bar', {
  state: { tick: 0 },

  /** Re-renders on store changes and on each fleet frame. */
  mounted()
  {
    this._un = store.subscribe(() => this.setState({}));
    this._eff = $.effect(() => { currentFleet(); this.setState({}); });
  },
  /** Tears down the store subscription and fleet effect. */
  destroyed()
  {
    if (this._un) this._un();
    if (typeof this._eff === 'function') this._eff();
  },

  /** Closes the drawer by unwinding one history entry. */
  close() { back(); },
  /**
   * Closes the drawer when the scrim itself is clicked.
   *
   * @param {MouseEvent} e - the click event.
   * @returns {void}
   */
  onOverlay(e) { if (e.target.classList.contains('drawer-scrim')) back(); },

  /**
   * Opens the detail modal for the clicked alarm row's sensor.
   *
   * @param {MouseEvent} e - the click event.
   * @returns {void}
   */
  onGoSensor(e)
  {
    const el = e.target.closest('[data-sid]'); if (!el) return;
    open(() => store.dispatch('selectSensor', el.dataset.sid), () => store.dispatch('closeSensor'));
  },
  /**
   * Opens the group view for the clicked alarm row's group.
   *
   * @param {MouseEvent} e - the click event.
   * @returns {void}
   */
  onGoGroup(e)
  {
    const el = e.target.closest('[data-gid]'); if (!el) return;
    const gid = el.dataset.gid;
    open(() => store.dispatch('setGroupView', gid), () => store.dispatch('clearGroupView'));
  },

  /**
   * Renders the drawer with the current problem list, or an empty placeholder when closed.
   *
   * @returns {string} the drawer markup.
   */
  render()
  {
    if (!store.state.alarms) return '<div hidden></div>';
    const list = problems(currentFleet());
    const groupBtn = (gid) => `<button class="al-btn" data-gid="${gid}" @click="onGoGroup">${t('ui.group')}</button>`;
    const rows = list.length ? list.map((p) =>
    {
      if (p.link)
      {
        return `<div class="al-row" data-level="error">
          <span class="al-dot" data-level="error"></span>
          <span class="al-text"><span class="al-name">${t('event.link.lost')}</span><span class="al-meta">${esc(p.org.name)} · ${esc(p.group.name)}</span></span>
          <span class="al-actions">${groupBtn(p.group.id)}</span></div>`;
      }
      const s = p.sensor, lvl = s.reading.status === 'alarm' ? 'error' : 'warn';
      return `<div class="al-row" data-level="${lvl}">
        <span class="al-dot" data-level="${lvl}"></span>
        <span class="al-text"><span class="al-name">${esc(t('label.' + s.reading.key))} · ${fmt(s.reading.value)}${t('unit.' + s.reading.unit)}</span>
          <span class="al-meta">${esc(p.group.name)} · ${esc(p.org.name)}</span></span>
        <span class="al-actions">
          <button class="al-btn" data-sid="${p.group.id}/${s.id}" @click="onGoSensor">${t('ui.sensorId')}</button>
          ${groupBtn(p.group.id)}
        </span></div>`;
    }).join('') : `<div class="al-empty">${t('ui.allClear')}</div>`;

    return `
      <div class="drawer-scrim" @click="onOverlay">
        <aside class="drawer" role="dialog" aria-modal="true">
          <div class="drawer-head">
            <div class="drawer-title">${t('ui.alarmsTitle')} ${list.length ? `<span class="al-count">${nf(list.length)}</span>` : ''}</div>
            <button class="modal-close" type="button" @click="close" aria-label="${esc(t('ui.cancel'))}">✕</button>
          </div>
          <div class="drawer-body">${rows}</div>
        </aside>
      </div>`;
  },
});
