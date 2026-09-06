import { mountConsoles } from './consoles.js';

let toast = null;
let listening = false;

/**
 * Shows a short notice at the foot of the viewport.
 *
 * @param {string} message - the notice.
 * @returns {void}
 */
const notice = (message) =>
{
  if (!toast || !toast.isConnected)
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

/**
 * Binds the front page: the stage, the wall of cards, and the backing preview.
 *
 * @returns {void}
 */
export function init()
{
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
  if (!listening)
  {
    listening = true;
    document.addEventListener('click', (e) =>
    {
      if (!e.target.closest('.bento-card')) document.querySelectorAll('.bento-card.pinned').forEach((card) => card.classList.remove('pinned'));
    });
  }

  const chips = [...document.querySelectorAll('.chip-btn')];
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

  // The first example opens capped; one click shows the whole of it.
  document.querySelectorAll('.reveal').forEach((button) => button.addEventListener('click', () =>
  {
    const panel = button.closest('.lang-panel');
    if (panel) panel.classList.remove('capped');
    button.remove();
  }));

  // Backing is a preview: every button says so rather than pretending.
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
}
