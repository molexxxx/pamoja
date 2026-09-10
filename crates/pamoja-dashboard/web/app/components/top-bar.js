import { store } from '../store.js';
import { THEMES, apply as applySheet } from '../lib/theme.js';
import { t, nf, availableLocales, setLocale, localeName } from '../lib/i18n.js';
import { SCENARIOS, demo, live } from '../lib/feed.js';
import { currentFleet } from '../lib/edits.js';
import { openOverlay } from '../nav.js';
import { problems } from './alarm-bar.js';
import { unlocked, lock } from '../lib/pair.js';
import { esc } from '../lib/viz/index.js';

const SVG = (d) => `<svg class="ic" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${d}</svg>`;
const ICON =
{
  bell: SVG('<path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.7 21a2 2 0 0 1-3.4 0"/>'),
  network: SVG('<circle cx="18" cy="5" r="2.6"/><circle cx="6" cy="12" r="2.6"/><circle cx="18" cy="19" r="2.6"/><path d="M8.3 10.7 15.7 6.3M8.3 13.3 15.7 17.7"/>'),
  globe: SVG('<circle cx="12" cy="12" r="9"/><path d="M3 12h18"/><path d="M12 3a15 15 0 0 1 4 9 15 15 0 0 1-4 9 15 15 0 0 1-4-9 15 15 0 0 1 4-9z"/>'),
  theme: SVG('<circle cx="12" cy="12" r="9"/><path d="M12 3a9 9 0 0 0 0 18z" fill="currentColor" stroke="none"/>'),
  scenario: SVG('<line x1="4" y1="8.5" x2="20" y2="8.5"/><line x1="4" y1="15.5" x2="20" y2="15.5"/><circle cx="9" cy="8.5" r="2.4" fill="var(--bg-1)"/><circle cx="15" cy="15.5" r="2.4" fill="var(--bg-1)"/>'),
  lock: SVG('<rect x="5" y="11" width="14" height="9" rx="2"/><path d="M8 11V7a4 4 0 0 1 8 0v4"/>'),
  unlock: SVG('<rect x="5" y="11" width="14" height="9" rx="2"/><path d="M8 11V7a4 4 0 0 1 7.6-1.5"/>'),
  lite: SVG('<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 9.5h18M3 14.5h18"/>'),
};
const CHEVRON = '<svg class="chev" aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M6 9l6 6 6-6"/></svg>';

$.component('top-bar', {
  state: { localeOpen: false, scenarioOpen: false, themeOpen: false },

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
  /** Locks control if unlocked, otherwise opens the pairing dialog. */
  toggleControl()
  {
    if (unlocked.value) lock();
    else openOverlay(() => store.dispatch('openPairing'), () => store.dispatch('closePairing'));
  },
  /** Opens the alarm drawer through the overlay nav. */
  openAlarms() { openOverlay(() => store.dispatch('openAlarms'), () => store.dispatch('closeAlarms')); },
  /** Opens or closes the theme menu. */
  toggleThemeMenu() { this.state.themeOpen = !this.state.themeOpen; },
  /** Closes the theme menu. */
  closeThemeMenu() { this.state.themeOpen = false; },
  /** Applies a theme by name and persists the choice. */
  pickTheme(name)
  {
    this.state.themeOpen = false;
    store.dispatch('setTheme', name);
    applySheet(name);
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
    const code = store.state.locale.slice(0, 2).toUpperCase();
    const locales = availableLocales().map((l) => `<li class="dd-option" role="option" aria-selected="${l === store.state.locale}" @click="pickLocale('${l}')">${esc(localeName(l))}</li>`).join('');
    const themes = THEMES.map((th) => `<li class="dd-option" role="option" aria-selected="${th === store.state.theme}" @click="pickTheme('${th}')"><span class="swatch" data-theme="${th}" aria-hidden="true"></span>${esc(t('theme.' + th))}</li>`).join('');
    const scenarios = SCENARIOS.map((sc) => `<li class="dd-option" role="option" aria-selected="${sc === store.state.scenario}" @click="pickScenario('${sc}')">${esc(t('scenario.' + sc))}</li>`).join('');
    return `
      <header class="topbar">
        <a class="brand" href="#/" z-link="/" aria-label="pamoja">
          <svg class="brand-mark" viewBox="0 0 240 240" width="32" height="32" aria-hidden="true"><g fill="none" stroke="currentColor" stroke-width="10" opacity="0.3"><polygon points="120,40 188,80 188,160 120,200 52,160 52,80"/><path d="M120 120V40M120 120l68-40M120 120l68 40M120 120v80M120 120l-68 40M120 120l-68-40"/></g><g><circle cx="120" cy="40" r="13" fill="#FFB627"/><circle cx="188" cy="80" r="13" fill="#F26A4B"/><circle cx="188" cy="160" r="13" fill="#1FA995"/><circle cx="120" cy="200" r="13" fill="#FFB627"/><circle cx="52" cy="160" r="13" fill="#F26A4B"/><circle cx="52" cy="80" r="13" fill="#1FA995"/></g><circle cx="120" cy="120" r="22" fill="#FFB627"/></svg>
          <span class="brand-text"><span class="brand-word">pamoja</span><span class="brand-sub">${t('ui.subtitle')}</span></span>
        </a>
        <nav class="deck" aria-label="${esc(t('ui.subtitle'))}">
          <button class="deck-seg bell ${alarmCount ? 'has' : ''}" type="button" @click="openAlarms" aria-label="${esc(t('ui.alarmsTitle'))}" title="${esc(t('ui.alarmsTitle'))}">
            ${ICON.bell}${alarmCount ? `<span class="bell-count">${nf(alarmCount)}</span>` : ''}
          </button>
          <a class="deck-seg" href="#/network" z-link="/network" aria-label="${esc(t('ui.network'))}" title="${esc(t('ui.network'))}">
            ${ICON.network}<span class="deck-label">${t('ui.network')}</span>
          </a>
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
          <div class="deck-dd ${s.themeOpen ? 'open' : ''}" @click.outside="closeThemeMenu">
            <button class="deck-seg theme" type="button" @click="toggleThemeMenu" aria-label="${esc(t('ui.theme'))}" title="${esc(t('theme.' + store.state.theme))}">
              ${ICON.theme}${CHEVRON}
            </button>
            <ul class="dd-menu" role="listbox" z-show="themeOpen">${themes}</ul>
          </div>
          <a class="deck-seg lite" href="lite.html" aria-label="${esc(t('ui.textView'))}" title="${esc(t('ui.textViewHint'))}">
            ${ICON.lite}<span class="deck-label">${t('ui.textView')}</span>
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
      </header>
      ${demo.value ? `<p class="demo-note" role="note">${esc(t('ui.demoNotice'))}</p>` : ''}`;
  },
});
