import { store } from '../store.js';
import { back } from '../nav.js';
import { t } from '../lib/i18n.js';
import { pair } from '../lib/pair.js';
import { esc } from '../lib/viz/index.js';

$.component('pairing-modal', {
  state: { code: '', error: null, busy: false, remember: false },

  /** Resets the form whenever the dialog opens or closes. */
  mounted() { this._un = store.subscribe(() => this.sync()); },
  /** Tears down the store subscription. */
  destroyed() { if (this._un) this._un(); },

  /** Clears the form when the dialog is not open, then re-renders. */
  sync()
  {
    if (!store.state.pairing) { this.state.code = ''; this.state.error = null; this.state.busy = false; this.state.remember = false; }
    this.setState({});
  },

  /** Toggles whether this device is trusted (the session persists across restarts). */
  toggleRemember() { this.state.remember = !this.state.remember; this.setState({}); },

  /** Closes the dialog by unwinding one history entry. */
  cancel() { back(); },
  /**
   * Closes the dialog when the backdrop itself is clicked.
   *
   * @param {MouseEvent} e - the click event.
   * @returns {void}
   */
  onOverlay(e) { if (e.target.classList.contains('modal-overlay')) back(); },

  /** Attempts to pair with the entered code, showing an error or closing on success. */
  async submit()
  {
    if (this.state.busy) return;
    this.state.busy = true;
    this.state.error = null;
    this.setState({});
    const result = await pair(this.state.code, this.state.remember);
    if (result.ok) { back(); return; }
    this.state.error = result.error === 'auth.unreachable' ? t('ui.pairOffline') : t('ui.pairFailed');
    this.state.busy = false;
    this.setState({});
  },

  /**
   * Renders the pairing dialog, or an empty placeholder when closed.
   *
   * @returns {string} the dialog markup.
   */
  render()
  {
    if (!store.state.pairing) return '<div hidden></div>';
    const s = this.state;
    return `
      <div class="modal-overlay" @click="onOverlay">
        <div class="modal modal-form" role="dialog" aria-modal="true">
          <div class="modal-head">
            <div class="modal-title">${t('ui.pairTitle')}</div>
            <button class="modal-close" type="button" @click="cancel" aria-label="${esc(t('ui.cancel'))}">✕</button>
          </div>
          <div class="form">
            <p class="form-hint">${esc(t('ui.pairHint'))}</p>
            <label class="field"><span>${t('ui.pairCode')}</span>
              <input class="field-input pair-code" type="text" autocomplete="off" spellcheck="false" z-model="code" placeholder="0000-0000" /></label>
            <label class="check-row">
              <input type="checkbox" class="check-box" ${s.remember ? 'checked' : ''} @click="toggleRemember" />
              <span>${esc(t('ui.trustDevice'))}</span>
            </label>
            ${s.error ? `<p class="form-error">${esc(s.error)}</p>` : ''}
          </div>
          <div class="form-actions">
            <button class="seg" type="button" @click="cancel">${t('ui.cancel')}</button>
            <button class="seg primary" type="button" @click="submit" ${s.busy ? 'disabled' : ''}>${t('ui.unlock')}</button>
          </div>
        </div>
      </div>`;
  },
});
