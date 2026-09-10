import { store } from '../store.js';
import { back } from '../nav.js';
import { t } from '../lib/i18n.js';
import { makeGroup, makeSensor, currentFleet, provision } from '../lib/edits.js';
import { catalog, scopeAllows } from '../lib/catalog.js';
import { LINK_NAMES, esc } from '../lib/viz/index.js';
import { sync as syncDialog, drop as dropDialog } from '../lib/dialog.js';

/** The unit tokens the custom-sensor entry suggests, beyond whatever the presets carry. */
const UNIT_TOKENS = ['percent', 'celsius', 'volt', 'watt', 'hectopascal', 'meter_per_second', 'millimeter', 'lux', 'liter_per_minute', 'decibel', 'count'];

/**
 * Turns a typed name into a stable element key, such as `"Water turbidity"` into
 * `"water_turbidity"`.
 *
 * @param {string} text - the name or key as typed.
 * @returns {string} the key, or an empty string if nothing usable was typed.
 */
function slug(text)
{
  return String(text || '').trim().toLowerCase().replace(/[^a-z0-9]+/g, '_').replace(/^_+|_+$/g, '');
}

$.component('manage-modal', {
  state: { name: '', linkKind: 'lora', sensorKind: 'temperature', value: '', binding: '', peer: '', error: null, last: null, cName: '', cKey: '', cUnit: '', cViz: 'radial', cLow: '', cHigh: '' },

  /** Resets the form whenever the create target changes. */
  mounted() { this._un = store.subscribe(() => this.sync()); },
  /** Tears down the store subscription. */
  destroyed() { dropDialog(); if (this._un) this._un(); },

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
      this.state.cName = '';
      this.state.cKey = '';
      this.state.cUnit = '';
      this.state.cViz = 'radial';
      this.state.cLow = '';
      this.state.cHigh = '';
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
   * Selects the graphic a custom sensor is drawn with.
   *
   * @param {string} k - the visualization kind, such as `'bar'`.
   * @returns {void}
   */
  setViz(k) { this.state.cViz = k; },

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
    const custom = `<button type="button" class="chip-opt chip-custom ${selected === 'custom' ? 'on' : ''}" @click="setKind('custom')">${esc(t('ui.custom'))}</button>`;
    return `<div class="field"><span>${t('ui.type')}</span>
        <div class="chips">${sensors.map(chip).join('')}${custom}</div>
        ${stats.length ? `<span class="chips-sub">${esc(t('ui.stats'))}</span><div class="chips">${stats.map(chip).join('')}</div>` : ''}
      </div>`;
  },

  /**
   * Renders the fields that describe a sensor the catalog has never seen: its name, key,
   * unit, graphic, and safe band.
   *
   * @returns {string} the custom-entry markup.
   */
  customFields()
  {
    const s = this.state;
    const units = [...new Set([...catalog.sensorPresets.map((p) => p.unit), ...UNIT_TOKENS].filter((u) => u && u !== 'state' && u !== 'record'))];
    return `
        <p class="form-hint">${esc(t('ui.customHint'))}</p>
        <label class="field"><span>${t('ui.name')}</span>
          <input class="field-input" type="text" autocomplete="off" z-model="cName" placeholder="Turbidity" /></label>
        <label class="field"><span>${t('ui.key')}</span>
          <input class="field-input" type="text" autocomplete="off" spellcheck="false" z-model="cKey" placeholder="water_turbidity" /></label>
        <label class="field"><span>${t('ui.unit')}</span>
          <input class="field-input" type="text" autocomplete="off" spellcheck="false" list="unit-tokens" z-model="cUnit" placeholder="percent, celsius, ntu" />
          <datalist id="unit-tokens">${units.map((u) => `<option value="${esc(u)}"></option>`).join('')}</datalist></label>
        <div class="field"><span>${t('ui.graphic')}</span>
          <div class="chips">${catalog.graphics.map((k) => `<button type="button" class="chip-opt ${s.cViz === k ? 'on' : ''}" @click="setViz('${k}')">${esc(t('viz.' + k))}</button>`).join('')}</div>
        </div>
        <div class="field"><span>${t('ui.band')}</span>
          <div class="field-pair">
            <input class="field-input" type="number" step="any" z-model="cLow" placeholder="${esc(t('ui.low'))}" aria-label="${esc(t('ui.low'))}" />
            <input class="field-input" type="number" step="any" z-model="cHigh" placeholder="${esc(t('ui.high'))}" aria-label="${esc(t('ui.high'))}" />
          </div>
        </div>`;
  },

  /**
   * Reads the custom-entry fields into a sensor description, or explains what is missing.
   *
   * @returns {{spec?: object, error?: string}} the description, or the error to show.
   */
  customSpec()
  {
    const s = this.state;
    const key = slug(s.cKey) || slug(s.cName);
    if (!key) return { error: t('ui.keyHint') };
    const unit = slug(s.cUnit);
    if (!unit) return { error: t('ui.unitHint') };
    const low = parseFloat(s.cLow), high = parseFloat(s.cHigh);
    const band = Number.isFinite(low) && Number.isFinite(high) && high > low ? [low, high] : undefined;
    const label = s.cName.trim() || undefined;
    return { spec: { key, unit, viz: s.cViz, label, band } };
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
    let spec;
    if (c.mode === 'sensor' && this.state.sensorKind === 'custom')
    {
      const read = this.customSpec();
      if (read.error) { this.state.error = read.error; this.setState({}); return; }
      spec = read.spec;
    }
    const built = c.mode === 'group'
      ? makeGroup(c.orgId, this.state.name.trim() || t('ui.newGroup'), this.state.linkKind)
      : makeSensor(c.groupId, this.state.sensorKind, (() => { const v = parseFloat(this.state.value); return Number.isFinite(v) ? v : NaN; })(), spec);
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
  /** Keeps the modal contract in step with what this component just rendered. */
  updated() { syncDialog(this._el); },

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
        ${s.sensorKind === 'custom' ? this.customFields() : ''}
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
            <h2 class="modal-title">${c.mode === 'group' ? t('ui.addGroup') : t('ui.addSensor')}</h2>
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
