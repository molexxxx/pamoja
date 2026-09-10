// dialog.js - the modal contract every overlay in the app shares.
//
// A dialog has to name itself, take focus when it opens, keep the keyboard inside it while
// it is open, hold the page still behind it, and hand focus back to whatever opened it.
// Components call sync() from mounted() and updated(); it wires or releases as the dialog
// appears and disappears, so no component carries the bookkeeping itself.

const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

let wired = null;
let opener = null;
let openerKey = null;
let onKey = null;
let scrollLock = 0;

/**
 * Builds a selector that can find an element again after its component re-renders.
 *
 * @param {HTMLElement} el - the element that opened the dialog.
 * @returns {string|null} a selector that re-finds it, or null when nothing identifies it.
 */
function keyOf(el)
{
  if (!el) return null;
  if (el.id) return '#' + CSS.escape(el.id);
  for (const attr of ['data-sid', 'data-gid'])
  {
    const v = el.getAttribute && el.getAttribute(attr);
    if (v) return `[${attr}="${CSS.escape(v)}"]`;
  }
  return null;
}

/**
 * Returns a dialog's focusable children in tab order.
 *
 * @param {HTMLElement} dialog - the dialog element.
 * @returns {HTMLElement[]} the focusable descendants that are currently rendered.
 */
function focusable(dialog)
{
  return [...dialog.querySelectorAll(FOCUSABLE)].filter((el) => el.offsetParent !== null || getComputedStyle(el).position === 'fixed');
}

/**
 * Gives the dialog an accessible name taken from its own title element.
 *
 * @param {HTMLElement} dialog - the dialog element.
 * @returns {void}
 */
function name(dialog)
{
  if (dialog.getAttribute('aria-labelledby') || dialog.getAttribute('aria-label')) return;
  const title = dialog.querySelector('.modal-title, .net-title');
  if (!title) return;
  if (!title.id) title.id = 'dlg-title-' + Math.random().toString(36).slice(2, 8);
  dialog.setAttribute('aria-labelledby', title.id);
}

/** Releases the current dialog: unlocks the page and returns focus to whatever opened it. */
function release()
{
  if (!wired) return;
  document.removeEventListener('keydown', onKey, true);
  if (--scrollLock <= 0) { scrollLock = 0; document.body.style.removeProperty('overflow'); }
  // The opener is often replaced when its component re-renders, so fall back to finding it
  // again by key, and to the main landmark, rather than dropping focus at the document.
  let back = opener && document.contains(opener) ? opener : null;
  if (!back && openerKey) back = document.querySelector(openerKey);
  if (!back) back = document.getElementById('main');
  wired = null;
  opener = null;
  openerKey = null;
  onKey = null;
  if (back) back.focus({ preventScroll: true });
}

/**
 * Wires or releases the modal contract for a component's rendered tree.
 *
 * @param {HTMLElement} [root] - the component's root element; a tree with no dialog releases.
 * @returns {void}
 */
export function sync(root)
{
  const dialog = root && root.querySelector('[role="dialog"]');
  if (!dialog)
  {
    if (wired && !document.contains(wired)) release();
    return;
  }
  name(dialog);
  if (wired === dialog) return;
  if (wired) release();

  wired = dialog;
  opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  openerKey = keyOf(opener);
  scrollLock++;
  document.body.style.overflow = 'hidden';
  if (!dialog.hasAttribute('tabindex')) dialog.setAttribute('tabindex', '-1');
  const first = focusable(dialog)[0];
  (first || dialog).focus({ preventScroll: true });

  // Tab cycles inside the dialog: the page behind it is inert while it is open.
  onKey = (e) =>
  {
    if (e.key !== 'Tab' || !wired) return;
    const items = focusable(wired);
    if (!items.length) { e.preventDefault(); wired.focus({ preventScroll: true }); return; }
    const at = items.indexOf(document.activeElement);
    if (e.shiftKey && at <= 0) { e.preventDefault(); items[items.length - 1].focus(); }
    else if (!e.shiftKey && (at === -1 || at === items.length - 1)) { e.preventDefault(); items[0].focus(); }
  };
  document.addEventListener('keydown', onKey, true);
}

/** Releases the dialog contract when a component that owned one is torn down. */
export function drop()
{
  if (wired) release();
}
