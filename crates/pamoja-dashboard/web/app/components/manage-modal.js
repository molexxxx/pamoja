import { store } from '../store.js';
import { back } from '../nav.js';
import { t } from '../lib/i18n.js';
import { makeGroup, makeSensor, currentFleet, provision } from '../lib/edits.js';
import { catalog, scopeAllows } from '../lib/catalog.js';
import { LINK_NAMES, esc } from '../lib/viz/index.js';

$.component('manage-modal', {
  state: { name: '', linkKind: 'lora', sensorKind: 'temperature', value: '', binding: '', peer: '', error: null, last: null },

  /** Resets the form whenever the create target changes. */
  mounted() { this._un = store.subscribe(() => this.sync()); },
  /** Tears down the store subscription. */
  destroyed() { if (this._un) this._un(); },

  /** Resets the form fields when a new create dialog opens, then re-renders. */
  sync()
  {
    const c = store.state.create;
    const id = c ? c.mode + (c.orgId || c.groupId) : null;
    if (id !== this.state.last)
    {
      this.state.last = id;
      this.state.name = '';
      this.state.value = '';
      this.state.binding = '';
      this.state.peer = '';
      this.state.error = null;
      this.state.linkKind = 'lora';
      this.state.sensorKind = 'temperature';
    }
    this.setState({});
  },

  /**
   * Selects a link kind for the new group.
   *
   * @param {string} k - the chosen link kind.
   * @returns {void}
   */
  setLink(k) { this.state.linkKind = k; },
  /**
   * Selects a sensor preset for the new sensor.
   *
   * @param {string} k - the chosen preset id.
   * @returns {void}
   */
  setKind(k) { this.state.sensorKind = k; },

  /**
   * The sensor presets offered for a target group, gated by each preset's scope so a
   * profile's custom element appears only where it applies (and mesh-only presets only on
   * a mesh link).
   *
   * @param {string} groupId - the target group's id.
   * @returns {Array<object>} the offered presets.
   */
  presetsFor(groupId)
  {
    const kind = this.groupKind(groupId);
    return catalog.sensorPresets.filter((p) => scopeAllows(p, kind));
  },

  /**
   * The link kind of a target group, or null if not found.
   *
   * @param {string} groupId - the target group's id.
   * @returns {?string} the link kind, such as `'mesh'`.
   */
  groupKind(groupId)
  {
    const f = currentFleet();
    if (f) for (const o of f.orgs) for (const g of o.groups) if (g.id === groupId) return g.link.kind;
    return null;
  },

  /**
   * Renders the type picker, splitting real sensors from node-stat cards so the two are
   * never confused when provisioning a group.
   *
   * @param {string} groupId - the target group's id.
   * @param {string} selected - the currently selected preset id.
   * @returns {string} the type-field markup.
   */
  typeField(groupId, selected)
  {
    const presets = this.presetsFor(groupId);
    const chip = (p) => `<button type="button" class="chip-opt ${selected === p.id ? 'on' : ''}" @click="setKind('${p.id}')">${esc(t('label.' + p.key))}</button>`;
    const sensors = presets.filter((p) => !p.stat);
    const stats = presets.filter((p) => p.stat);
    return `<div class="field"><span>${t('ui.type')}</span>
        <div class="chips">${sensors.map(chip).join('')}</div>
        ${stats.length ? `<span class="chips-sub">${esc(t('ui.stats'))}</span><div class="chips">${stats.map(chip).join('')}</div>` : ''}
      </div>`;
  },

  /** Cancels the dialog by unwinding one history entry. */
  cancel() { back(); },
  /**
   * Cancels the dialog when the backdrop itself is clicked.
   *
   * @param {MouseEvent} e - the click event.
   * @returns {void}
   */
  onOverlay(e) { if (e.target.classList.contains('modal-overlay')) back(); },

  /** Creates the group or sensor from the form, then closes the dialog. */
  async submit()
  {
    const c = store.state.create; if (!c) return;
    const built = c.mode === 'group'
      ? makeGroup(c.orgId, this.state.name.trim() || t('ui.newGroup'), this.state.linkKind)
      : makeSensor(c.groupId, this.state.sensorKind, (() => { const v = parseFloat(this.state.value); return Number.isFinite(v) ? v : NaN; })());
    // A sensor may carry an optional hardware binding for a real gateway to bind a driver,
    // and on a mesh node an optional peer (station) name that groups it on the mesh map.
    if (c.mode === 'sensor')
    {
      built.binding = this.state.binding.trim() || undefined;
      const peer = this.state.peer.trim();
      if (peer) built.peer = peer;
    }
    const result = await provision(c.mode === 'group' ? 'addGroup' : 'addSensor', built);
    if (result.ok) { back(); return; }
    // A real device refuses a sensor type it cannot bind; say so rather than the generic failure.
    this.state.error = result.error === 'command.unknown_sensor' ? t('ui.sensorUnsupported') : t('ui.commandFailed');
    this.setState({});
  },

  /**
   * Renders the create dialog for the active target, or an empty placeholder when none.
   *
   * @returns {string} the dialog markup.
   */
  render()
  {
    const c = store.state.create;
    if (!c) return '<div hidden></div>';
    const s = this.state;
    const body = c.mode === 'group'
      ? `
        <label class="field"><span>${t('ui.name')}</span>
          <input class="field-input" type="text" z-model="name" placeholder="${esc(t('ui.newGroup'))}" /></label>
        <div class="field"><span>${t('ui.connection')}</span>
          <div class="chips">${catalog.linkKinds.map((k) => `<button type="button" class="chip-opt ${s.linkKind === k ? 'on' : ''}" @click="setLink('${k}')">${esc(LINK_NAMES[k] || k)}</button>`).join('')}</div>
        </div>`
      : `
        ${this.typeField(c.groupId, s.sensorKind)}
        <label class="field"><span>${t('ui.value')}</span>
          <input class="field-input" type="number" step="any" z-model="value" placeholder="${t('ui.auto')}" /></label>
        <label class="field"><span>${t('ui.binding')}</span>
          <input class="field-input" type="text" autocomplete="off" spellcheck="false" z-model="binding" placeholder="${esc(t('ui.bindingHint'))}" /></label>
        ${this.groupKind(c.groupId) === 'mesh' ? `<label class="field"><span>${t('ui.meshPeer')}</span>
          <input class="field-input" type="text" autocomplete="off" spellcheck="false" z-model="peer" placeholder="${esc(t('ui.meshPeerHint'))}" /></label>` : ''}`;
    return `
      <div class="modal-overlay" @click="onOverlay">
        <div class="modal modal-form" role="dialog" aria-modal="true">
          <div class="modal-head">
            <div class="modal-title">${c.mode === 'group' ? t('ui.addGroup') : t('ui.addSensor')}</div>
            <button class="modal-close" type="button" @click="cancel" aria-label="${esc(t('ui.cancel'))}">✕</button>
          </div>
          <div class="form">${body}${s.error ? `<p class="form-error">${esc(s.error)}</p>` : ''}</div>
          <div class="form-actions">
            <button class="seg" type="button" @click="cancel">${t('ui.cancel')}</button>
            <button class="seg primary" type="button" @click="submit">${t('ui.create')}</button>
          </div>
        </div>
      </div>`;
  },
});
