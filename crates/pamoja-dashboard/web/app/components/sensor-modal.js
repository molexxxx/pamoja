import { store } from '../store.js';
import { currentFleet } from '../lib/edits.js';
import { back, open } from '../nav.js';
import { sensorDetailBody, stickLog } from '../lib/detail.js';
import { t } from '../lib/i18n.js';
import { sendCommand, unlocked } from '../lib/pair.js';
import { conn, esc } from '../lib/viz/index.js';

$.component('sensor-modal', {
  state: { busy: false, error: null },

  /** Re-renders on store changes, on each fleet frame, and on lock state changes. */
  mounted()
  {
    this._un = store.subscribe(() => this.setState({}));
    this._eff = $.effect(() => { currentFleet(); unlocked.value; this.setState({}); });
  },
  /** Tears down the store subscription and fleet effect. */
  destroyed() { if (this._un) this._un(); if (typeof this._eff === 'function') this._eff(); },
  /** Keeps the event log pinned to its newest line after a re-render. */
  updated() { stickLog(this._el); },

  /** Closes the modal by unwinding one history entry. */
  close() { back(); },
  /**
   * Closes the modal when the backdrop itself is clicked.
   *
   * @param {MouseEvent} e - the click event.
   * @returns {void}
   */
  onOverlay(e) { if (e.target.classList.contains('modal-overlay')) back(); },

  /**
   * Resolves the selected sensor and its org/group from the current fleet.
   *
   * @returns {{org: object, group: object, sensor: object}|null} the selection, or null.
   */
  find()
  {
    const sel = store.state.selected; const f = currentFleet();
    if (!sel || !f) return null;
    const [gid, sid] = sel.split('/');
    for (const o of f.orgs) for (const g of o.groups) { if (g.id !== gid) continue; const s = g.sensors.find((x) => x.id === sid); if (s) return { org: o, group: g, sensor: s }; }
    return null;
  },

  /**
   * Sends an authenticated actuation command for the selected actuator.
   *
   * @param {string} action - the action to apply, such as `"open"`.
   * @returns {Promise<void>} resolves once the command has been sent and the UI updated.
   */
  async actuate(action)
  {
    if (this.state.busy) return;
    const found = this.find();
    if (!found) return;
    this.state.busy = true;
    this.state.error = null;
    this.setState({});
    const result = await sendCommand({ type: 'actuate', target: found.group.id + '/' + found.sensor.id, action });
    this.state.busy = false;
    this.state.error = result.ok ? null : t('ui.commandFailed');
    this.setState({});
  },

  /** Opens the pairing dialog so a locked actuator can be unlocked. */
  unlockPrompt() { open(() => store.dispatch('openPairing'), () => store.dispatch('closePairing')); },

  /**
   * Renders the modal for the selected sensor, or an empty placeholder when none.
   *
   * @returns {string} the modal markup.
   */
  render()
  {
    const found = this.find();
    if (!found) return '<div hidden></div>';
    const { org, group, sensor: s } = found;
    const actions = s.reading.actions || [];
    const control = actions.length ? `
      <div class="actuator">
        <span class="actuator-label">${esc(t('ui.control'))}</span>
        ${unlocked.value
          ? `<div class="actuator-actions">${actions.map((a) => `<button class="seg ${s.reading.state === 'state.' + a ? 'on' : ''}" type="button" @click="actuate('${a}')" ${this.state.busy ? 'disabled' : ''}>${esc(t('state.' + a))}</button>`).join('')}</div>`
          : `<button class="seg" type="button" @click="unlockPrompt">${esc(t('ui.unlock'))}</button>`}
        ${this.state.error ? `<span class="form-error">${esc(this.state.error)}</span>` : ''}
      </div>` : '';
    return `
      <div class="modal-overlay" @click="onOverlay">
        <div class="modal" data-status="${s.reading.status}" role="dialog" aria-modal="true">
          <div class="modal-head">
            <div class="modal-head-main">
              <div class="modal-title">${esc(t('label.' + s.reading.key))}</div>
              <div class="modal-sub">${esc(org.name)} · ${esc(group.name)}</div>
            </div>
            <div class="modal-head-side">${conn(group.link)}<button class="modal-close" type="button" @click="close" aria-label="Close">✕</button></div>
          </div>
          ${sensorDetailBody(s)}
          ${control}
        </div>
      </div>`;
  },
});
