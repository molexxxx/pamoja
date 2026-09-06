import { store } from '../store.js';
import { t, nf, availableLocales, setLocale, localeName } from '../lib/i18n.js';
import { SCENARIOS, demo, live } from '../lib/feed.js';
import { currentFleet } from '../lib/edits.js';
import { open } from '../nav.js';
import { openNetworkOverlay } from './network-view.js';
import { problems } from './alarm-bar.js';
import { unlocked, lock } from '../lib/pair.js';
import { esc } from '../lib/viz/index.js';

const SVG = (d) => `<svg class="ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">${d}</svg>`;
const ICON =
{
  bell: SVG('<path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.7 21a2 2 0 0 1-3.4 0"/>'),
  network: SVG('<circle cx="18" cy="5" r="2.6"/><circle cx="6" cy="12" r="2.6"/><circle cx="18" cy="19" r="2.6"/><path d="M8.3 10.7 15.7 6.3M8.3 13.3 15.7 17.7"/>'),
  globe: SVG('<circle cx="12" cy="12" r="9"/><path d="M3 12h18"/><path d="M12 3a15 15 0 0 1 4 9 15 15 0 0 1-4 9 15 15 0 0 1-4-9 15 15 0 0 1 4-9z"/>'),
  sun: SVG('<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>'),
  moon: SVG('<path d="M21 12.8A9 9 0 1 1 11.2 3 7 7 0 0 0 21 12.8z"/>'),
  scenario: SVG('<line x1="4" y1="8.5" x2="20" y2="8.5"/><line x1="4" y1="15.5" x2="20" y2="15.5"/><circle cx="9" cy="8.5" r="2.4" fill="var(--bg-1)"/><circle cx="15" cy="15.5" r="2.4" fill="var(--bg-1)"/>'),
  lock: SVG('<rect x="5" y="11" width="14" height="9" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/>'),
  unlock: SVG('<rect x="5" y="11" width="14" height="9" rx="2"/><path d="M8 11V7a4 4 0 0 1 7.6-1.5"/>'),
  lite: SVG('<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 9.5h18M3 14.5h18"/>'),
};
const CHEVRON = '<svg class="chev" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M6 9l6 6 6-6"/></svg>';

$.component('top-bar', {
  state: { localeOpen: false, scenarioOpen: false },

  /** Re-renders on store changes and on each fleet frame (for the alarm count). */
  mounted()
  {
    this._un = store.subscribe(() => this.setState({}));
    this._eff = $.effect(() => { currentFleet(); unlocked.value; demo.value; live.value; this.setState({}); });
  },
  /** Tears down the store subscription and fleet effect. */
  destroyed() { if (this._un) this._un(); if (typeof this._eff === 'function') this._eff(); },

  /** Opens the locale menu and closes the scenario menu. */
  toggleLocale() { this.state.scenarioOpen = false; this.state.localeOpen = !this.state.localeOpen; },
  /** Opens the scenario menu and closes the locale menu. */
  toggleScenario() { this.state.localeOpen = false; this.state.scenarioOpen = !this.state.scenarioOpen; },
  /** Closes the locale menu. */
  closeLocale() { this.state.localeOpen = false; },
  /** Closes the scenario menu. */
  closeScenario() { this.state.scenarioOpen = false; },

  /**
   * Switches the active locale and closes the menu.
   *
   * @param {string} l - the chosen locale tag.
   * @returns {Promise<void>} resolves once the locale is active.
   */
  async pickLocale(l) { this.state.localeOpen = false; await setLocale(l); },
  /**
   * Switches the dev scenario and closes the menu.
   *
   * @param {string} s - the chosen scenario key.
   * @returns {void}
   */
  pickScenario(s) { this.state.scenarioOpen = false; store.dispatch('setScenario', s); },
  /** Opens the network overlay. */
  openNetwork() { openNetworkOverlay(); },
  /** Locks control if unlocked, otherwise opens the pairing dialog. */
  toggleControl()
  {
    if (unlocked.value) lock();
    else open(() => store.dispatch('openPairing'), () => store.dispatch('closePairing'));
  },
  /** Opens the alarm drawer through the overlay nav. */
  openAlarms() { open(() => store.dispatch('openAlarms'), () => store.dispatch('closeAlarms')); },
  /** Toggles between the night and day themes, persisting the choice. */
  toggleTheme()
  {
    const next = store.state.theme === 'night' ? 'day' : 'night';
    store.dispatch('setTheme', next);
    document.documentElement.dataset.theme = next;
  },

  /**
   * Renders the brand and the control deck.
   *
   * @returns {string} the top-bar markup.
   */
  render()
  {
    const s = this.state;
    const alarmCount = problems(currentFleet()).length;
    const night = store.state.theme === 'night';
    const code = store.state.locale.slice(0, 2).toUpperCase();
    const locales = availableLocales().map((l) => `<li class="dd-option" aria-selected="${l === store.state.locale}" @click="pickLocale('${l}')">${esc(localeName(l))}</li>`).join('');
    const scenarios = SCENARIOS.map((sc) => `<li class="dd-option" aria-selected="${sc === store.state.scenario}" @click="pickScenario('${sc}')">${esc(t('scenario.' + sc))}</li>`).join('');
    return `
      <header class="topbar">
        <a class="brand" z-link="/" aria-label="pamoja">
          <span class="brand-mark"></span>
          <span class="brand-text"><span class="brand-word">pamoja</span><span class="brand-sub">${t('ui.subtitle')}</span></span>
        </a>
        <nav class="deck" aria-label="${esc(t('ui.subtitle'))}">
          <button class="deck-seg bell ${alarmCount ? 'has' : ''}" type="button" @click="openAlarms" aria-label="${esc(t('ui.alarmsTitle'))}" title="${esc(t('ui.alarmsTitle'))}">
            ${ICON.bell}${alarmCount ? `<span class="bell-count">${nf(alarmCount)}</span>` : ''}
          </button>
          <button class="deck-seg" type="button" @click="openNetwork" aria-label="${esc(t('ui.network'))}" title="${esc(t('ui.network'))}">
            ${ICON.network}<span class="deck-label">${t('ui.network')}</span>
          </button>
          ${live.value ? `<button class="deck-seg control ${unlocked.value ? 'is-unlocked' : ''}" type="button" @click="toggleControl" aria-label="${esc(t('ui.control'))}" title="${esc(unlocked.value ? t('ui.lock') : t('ui.unlock'))}">
            ${unlocked.value ? ICON.unlock : ICON.lock}
          </button>` : ''}
          <span class="deck-div" aria-hidden="true"></span>
          <div class="deck-dd ${s.localeOpen ? 'open' : ''}" @click.outside="closeLocale">
            <button class="deck-seg" type="button" @click="toggleLocale" aria-label="${esc(t('ui.language'))}" title="${esc(localeName(store.state.locale))}">
              ${ICON.globe}<span class="deck-code">${esc(code)}</span>${CHEVRON}
            </button>
            <ul class="dd-menu" role="listbox" z-show="localeOpen">${locales}</ul>
          </div>
          <button class="deck-seg theme ${night ? 'is-night' : 'is-day'}" type="button" @click="toggleTheme" aria-label="${esc(t('ui.theme'))}" title="${night ? esc(t('ui.day')) : esc(t('ui.night'))}">
            ${night ? ICON.sun : ICON.moon}
          </button>
          <a class="deck-seg lite" href="lite.html" aria-label="${esc(t('ui.tierLite'))}" title="${esc(t('ui.tierLite'))}">
            ${ICON.lite}<span class="deck-label">${t('ui.tierLite')}</span>
          </a>
          ${demo.value ? `
          <span class="deck-div" aria-hidden="true"></span>
          <div class="deck-dd ${s.scenarioOpen ? 'open' : ''}" @click.outside="closeScenario">
            <button class="deck-seg" type="button" @click="toggleScenario" aria-label="${esc(t('ui.scenario'))}" title="${esc(t('scenario.' + store.state.scenario))}">
              ${ICON.scenario}<span class="deck-label deck-pick">${esc(t('scenario.' + store.state.scenario))}</span>${CHEVRON}
            </button>
            <ul class="dd-menu" role="listbox" z-show="scenarioOpen">${scenarios}</ul>
          </div>` : ''}
        </nav>
      </header>`;
  },
});
