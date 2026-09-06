// home.js - the front page's islands, over markup the site renders.
//
// The page reads without any of this: every scenario's caption, every card's links, and
// the first example in all four languages are in the HTML. This adds the stage that plays
// one console at a time, the wall of cards that opens on a click and filters by chapter,
// and the notice that backing is a preview. The consoles themselves are in consoles.js.

import { mountConsoles } from './consoles.js';

// One scene on the stage at a time; its tab carries the accent.
const stageTabs = [...document.querySelectorAll('.stage-tab')];
const scenes = [...document.querySelectorAll('.scene')];
if (stageTabs.length)
{
  const show = (key) =>
  {
    stageTabs.forEach((tab) => tab.setAttribute('aria-selected', String(tab.dataset.scene === key)));
    scenes.forEach((scene) => { scene.hidden = scene.id !== `scene-${key}`; });
  };
  stageTabs.forEach((tab, index) =>
  {
    tab.addEventListener('click', () => show(tab.dataset.scene));
    tab.addEventListener('keydown', (e) =>
    {
      if (e.key !== 'ArrowRight' && e.key !== 'ArrowLeft') return;
      e.preventDefault();
      const next = stageTabs[(index + (e.key === 'ArrowRight' ? 1 : -1) + stageTabs.length) % stageTabs.length];
      next.focus();
      show(next.dataset.scene);
    });
  });
}
mountConsoles();

// The wall of cards: a click pins a card open, and the chips narrow the wall to a chapter.
const cards = [...document.querySelectorAll('.bento-card')];
const unpin = () => cards.forEach((card) => card.classList.remove('pinned'));
cards.forEach((card) =>
{
  card.addEventListener('click', (e) =>
  {
    if (e.target.closest('a')) return;
    const open = card.classList.contains('pinned');
    unpin();
    if (!open) card.classList.add('pinned');
  });
  card.addEventListener('keydown', (e) =>
  {
    if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); card.click(); }
    if (e.key === 'Escape') unpin();
  });
});
document.addEventListener('click', (e) => { if (!e.target.closest('.bento-card')) unpin(); });

const chips = [...document.querelectorAllSafe?.('.chip-btn') ?? document.querySelectorAll('.chip-btn')];
chips.forEach((chip) => chip.addEventListener('click', () =>
{
  const chapter = chip.dataset.chapter;
  chips.forEach((other) =>
  {
    const on = other === chip;
    other.classList.toggle('active', on);
    other.setAttribute('aria-selected', String(on));
  });
  unpin();
  cards.forEach((card) => { card.hidden = chapter !== 'all' && card.dataset.chapter !== chapter; });
}));

// Backing is a preview: every button says so rather than pretending.
let toast = null;
const notice = (message) =>
{
  if (!toast)
  {
    toast = document.createElement('div');
    toast.className = 'toast';
    toast.setAttribute('role', 'status');
    toast.setAttribute('aria-live', 'polite');
    document.body.appendChild(toast);
  }
  toast.textContent = message;
  toast.classList.add('show');
  clearTimeout(notice.timer);
  notice.timer = setTimeout(() => toast.classList.remove('show'), 3200);
};
const PREVIEW = 'Backing is not open yet; this section is a preview of how it will work.';
document.querySelectorAll('.soon').forEach((button) => button.addEventListener('click', () => notice(PREVIEW)));

const form = document.querySelector('.pledge');
if (form)
{
  form.addEventListener('submit', (e) => { e.preventDefault(); notice(PREVIEW); });
  const roles = [...form.querySelectorAll('.role')];
  roles.forEach((button) => button.addEventListener('click', () =>
  {
    roles.forEach((other) =>
    {
      const on = other === button;
      other.classList.toggle('active', on);
      other.setAttribute('aria-selected', String(on));
    });
    form.querySelectorAll('[data-when]').forEach((field) => { field.hidden = field.dataset.when !== button.dataset.role; });
  }));
}
