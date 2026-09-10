import { mountConsoles } from './consoles.js';

/**
 * Binds the front page: the scenario figures, the language pin, the probe over a figure,
 * and the capped first listing.
 *
 * @returns {void}
 */
export function init()
{
  const root = document.documentElement;

  // One scene on the stage at a time.
  const stageTabs = [...document.querySelectorAll('.stage-tab')];
  const scenes = [...document.querySelectorAll('.scene')];
  if (stageTabs.length)
  {
    const show = (key) =>
    {
      stageTabs.forEach((tab) => tab.setAttribute('aria-selected', String(tab.dataset.scene === key)));
      scenes.forEach((scene) => { scene.hidden = scene.id !== `scene-${key}`; });
    };
    stageTabs.forEach((tab, i) =>
    {
      tab.addEventListener('click', () => show(tab.dataset.scene));
      tab.addEventListener('keydown', (e) =>
      {
        if (e.key !== 'ArrowRight' && e.key !== 'ArrowLeft') return;
        e.preventDefault();
        const next = stageTabs[(i + (e.key === 'ArrowRight' ? 1 : stageTabs.length - 1)) % stageTabs.length];
        show(next.dataset.scene);
        next.focus();
      });
    });
    // A link from the applications list opens its scene as well as scrolling to it.
    document.querySelectorAll('.applications a[href^="#scene-"]').forEach((a) => a.addEventListener('click', () =>
    {
      show(a.getAttribute('href').slice('#scene-'.length));
    }));
  }
  mountConsoles();

  // The language pin in the ordering table sets every listing on the site, the way the
  // tabs over a listing do.
  const pins = [...document.querySelectorAll('.pin')];
  const keyed = (pin) => (pin.dataset.lang === 'node' ? 'typescript' : pin.dataset.lang === 'dotnet' ? 'c' : pin.dataset.lang);
  const pinned = () => pins.forEach((pin) => pin.setAttribute('aria-pressed', String(keyed(pin) === root.dataset.lang)));
  pinned();
  pins.forEach((pin) => pin.addEventListener('click', () =>
  {
    const lang = keyed(pin);
    root.dataset.lang = lang;
    try { localStorage.setItem('pamoja:lang', lang); } catch (e) { /* storage may be unavailable */ }
    document.querySelectorAll('.lang-tab').forEach((tab) => tab.setAttribute('aria-selected', String(tab.dataset.lang === lang)));
    pinned();
  }));
  document.querySelectorAll('.lang-tab').forEach((tab) => tab.addEventListener('click', pinned));

  // The probe: pointing at one reading in a figure, or moving the keyboard to it, singles
  // it out. Every reading is focusable, so the probe is not a mouse-only affordance.
  document.querySelectorAll('.diorama').forEach((figure) =>
  {
    const clear = () =>
    {
      figure.classList.remove('probing');
      figure.querySelectorAll('.tile.probed').forEach((t) => t.classList.remove('probed'));
    };
    const probe = (tile) =>
    {
      figure.classList.toggle('probing', Boolean(tile));
      figure.querySelectorAll('.tile.probed').forEach((t) => { if (t !== tile) t.classList.remove('probed'); });
      if (tile) tile.classList.add('probed');
    };
    figure.addEventListener('pointerover', (e) => probe(e.target.closest('.tile')));
    figure.addEventListener('focusin', (e) => probe(e.target.closest('.tile')));
    figure.addEventListener('pointerleave', clear);
    figure.addEventListener('focusout', (e) => { if (!figure.contains(e.relatedTarget)) clear(); });
    figure.addEventListener('pointercancel', clear);
  });
  // A tap outside a figure lets go of the reading it was holding.
  document.addEventListener('pointerdown', (e) =>
  {
    if (e.target.closest('.diorama')) return;
    document.querySelectorAll('.diorama.probing').forEach((figure) =>
    {
      figure.classList.remove('probing');
      figure.querySelectorAll('.tile.probed').forEach((t) => t.classList.remove('probed'));
    });
  });

  // The first listing opens at a few lines; the control in its caption opens the rest and
  // closes it again.
  document.querySelectorAll('.reveal').forEach((button) => button.addEventListener('click', () =>
  {
    const panel = button.closest('.lang-panel');
    if (!panel) return;
    const capped = panel.classList.toggle('capped');
    button.setAttribute('aria-expanded', String(!capped));
    button.firstChild.nodeValue = capped ? 'Whole listing' : 'Collapse';
    if (capped) panel.scrollIntoView({ block: 'nearest' });
  }));
}
