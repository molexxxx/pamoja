let hoverEl = null;
let amt = 5;
let glow = false;
let tx = 0;
let ty = 0;
let mx = 50;
let loop = 0;

/** Clears the tilt custom properties from the previously hovered element. */
function clearHover()
{
  if (hoverEl) { hoverEl.style.removeProperty('--rx'); hoverEl.style.removeProperty('--ry'); }
  hoverEl = null;
}

/** Writes the current tilt onto the hovered element and reschedules itself. */
function applyTilt()
{
  if (!hoverEl) { loop = 0; return; }
  hoverEl.style.setProperty('--rx', tx.toFixed(2) + 'deg');
  hoverEl.style.setProperty('--ry', ty.toFixed(2) + 'deg');
  if (glow) hoverEl.style.setProperty('--mx', mx.toFixed(0) + '%');
  loop = requestAnimationFrame(applyTilt);
}

/**
 * Updates the tilt target and amounts from a pointer event.
 *
 * @param {PointerEvent} e - the pointer-move event.
 * @returns {void}
 */
function parallax(e)
{
  const t = e.target.closest ? e.target : null;
  const modal = t && t.closest('.modal');
  const card = !modal && t ? t.closest('.gcard') : null;
  const el = modal || card;
  if (!el) { clearHover(); return; }
  if (el !== hoverEl) { clearHover(); hoverEl = el; amt = modal ? 3 : 5; glow = !!card; }
  const b = el.getBoundingClientRect();
  const px = (e.clientX - b.left) / b.width - 0.5, py = (e.clientY - b.top) / b.height - 0.5;
  tx = px * amt; ty = -py * amt; mx = px * 100 + 50;
  if (!loop) loop = requestAnimationFrame(applyTilt);
}

/**
 * Starts the pointer-tilt effect on devices with a hovering, fine pointer, and keeps it
 * in sync if the pointer kind later changes (such as attaching a mouse to a tablet).
 *
 * @returns {void}
 */
export function initParallax()
{
  const pointer = window.matchMedia('(hover: hover) and (pointer: fine)');
  const sync = () =>
  {
    if (pointer.matches)
    {
      document.addEventListener('pointermove', parallax);
    }
    else
    {
      document.removeEventListener('pointermove', parallax);
      clearHover();
    }
  };
  sync();
  pointer.addEventListener?.('change', sync);
}
