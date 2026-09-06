let router;
const stack = [];

/**
 * Wires the substate listener. Call once with the router after it is created.
 *
 * @param {object} [r] - the router instance; falls back to `$.getRouter()`.
 * @returns {void}
 */
export function initNav(r)
{
  router = r || ($.getRouter && $.getRouter());
  if (!router) return;
  router.onSubstate(() =>
  {
    if (!stack.length) return false; // no overlay open: let normal route navigation happen
    const close = stack.pop();
    if (close) close();
    return true; // consume: do not re-resolve the route
  });
}

/**
 * Opens an overlay: runs `openFn`, records `closeFn`, and pushes a history entry.
 *
 * @param {() => void} openFn - opens the overlay (typically a store dispatch).
 * @param {() => void} closeFn - closes the overlay when the entry is popped.
 * @returns {void}
 */
export function open(openFn, closeFn)
{
  openFn();
  stack.push(closeFn);
  if (router) router.pushSubstate('ov');
}

/**
 * Closes the topmost overlay by going back one history entry.
 *
 * @returns {void}
 */
export function back()
{
  if (stack.length) history.back();
}
