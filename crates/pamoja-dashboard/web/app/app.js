import { store } from './store.js';
import { initI18n, t, registerLabels, registerMessages } from './lib/i18n.js';
import { initNav, back } from './nav.js';
import { connectFeed, connected, fleet } from './lib/feed.js';
import { catalog, extendCatalog } from './lib/catalog.js';
import { initParallax } from './lib/parallax.js';
import { routes } from './routes.js';
import './components/top-bar.js';
import './components/dashboard-page.js';
import './components/sensor-modal.js';
import './components/manage-modal.js';
import './components/pairing-modal.js';
import './components/group-modal.js';
import './components/mesh-modal.js';
import './components/network-view.js';
import './components/alarm-bar.js';

/**
 * Tints the page from a catalog theme by setting CSS custom properties on the root.
 *
 * @param {{accent?: string, ok?: string, warn?: string, alarm?: string, track?: string}} theme - the theme tokens.
 * @returns {void}
 */
function applyTheme(theme)
{
  if (!theme) return;
  const root = document.documentElement.style;
  const set = (name, value) => { if (value) root.setProperty(name, value); };
  set('--cyan', theme.accent);
  set('--ok', theme.ok);
  set('--warn', theme.warn);
  set('--alarm', theme.alarm);
  set('--track', theme.track);
}

/**
 * Best-effort fetch of the device's presentation catalog, folded into the built-in one
 * before first paint. A static host (or a device with nothing custom) answers no catalog,
 * and the built-in defaults stand.
 *
 * @returns {Promise<void>} resolves once any served catalog has been applied.
 */
async function loadCatalog()
{
  try
  {
    const res = await fetch('/catalog', { cache: 'no-store' });
    if (!res.ok) return;
    const served = await res.json();
    extendCatalog(served);
    registerLabels(served.sensorPresets);
    registerMessages(served.messages);
    applyTheme(catalog.theme);
  } catch { /* no device endpoint here; the built-in catalog stands */ }
}

await initI18n();
await loadCatalog();
document.documentElement.dataset.theme = store.state.theme;

const router = $.router({ routes, mode: 'hash', fallback: 'dashboard-page' });
connectFeed();

$.effect(() =>
{
  const tag = document.getElementById('offline-tag');
  if (tag) { tag.hidden = connected.value; tag.textContent = t('ui.disconnected'); }
});

let mounted = false, revealed = false;

/** Removes the boot splash once the app is mounted and the first fleet frame has arrived. */
function reveal()
{
  if (revealed || !mounted || fleet.value == null) return;
  revealed = true;
  const boot = document.getElementById('boot');
  if (boot) { boot.classList.add('gone'); setTimeout(() => boot.remove(), 600); }
}
$.effect(() => { fleet.value; reveal(); });
setTimeout(() => { revealed = true; const b = document.getElementById('boot'); if (b) { b.classList.add('gone'); setTimeout(() => b.remove(), 600); } }, 1500);

$.ready(() =>
{
  initNav(router);
  $.mountAll();
  mounted = true;
  reveal();
  initParallax();
  document.addEventListener('keydown', (e) => { if (e.key === 'Escape') back(); });
});
