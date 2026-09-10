// theme.js - which sheet the dashboard is printed on.
//
// One list, one resolver. `system` is not a sheet of its own: it follows the reader's
// operating system and resolves to the datasheet or its dark twin, and it keeps following
// while it is selected, so changing the system setting changes the page without a reload.

/** The sheets on offer, in the order the menu lists them: the light one, then the darks. */
export const THEMES = ['system', 'sheet', 'sheet-dark', 'slate', 'blueprint', 'phosphor', 'safelight'];

/** Names the dashboard used before the sheets, mapped onto their nearest one. */
const RETIRED = { night: 'sheet-dark', day: 'sheet', daylight: 'sheet' };

/**
 * Whether the operating system asks for a dark interface.
 *
 * @returns {boolean} true when the system prefers dark.
 */
function systemDark()
{
  return typeof matchMedia === 'function' && matchMedia('(prefers-color-scheme: dark)').matches;
}

/**
 * Normalizes a stored preference to a name this build knows.
 *
 * @param {*} stored - whatever was persisted, possibly from an older build.
 * @returns {string} a name in {@link THEMES}.
 */
export function known(stored)
{
  if (THEMES.includes(stored)) return stored;
  return RETIRED[stored] || 'system';
}

/**
 * Resolves a preference to the sheet actually painted.
 *
 * @param {string} name - a name in {@link THEMES}.
 * @returns {string} the sheet to write to the root element.
 */
export function resolve(name)
{
  return name === 'system' ? (systemDark() ? 'sheet-dark' : 'sheet') : name;
}

/**
 * Paints a preference onto the document.
 *
 * @param {string} name - a name in {@link THEMES}.
 * @returns {void}
 */
export function apply(name)
{
  document.documentElement.dataset.theme = resolve(name);
}

/**
 * Keeps the page in step with the operating system while `system` is the preference.
 *
 * @param {() => string} current - reads the preference at the moment the system changes.
 * @returns {void}
 */
export function followSystem(current)
{
  if (typeof matchMedia !== 'function') return;
  const query = matchMedia('(prefers-color-scheme: dark)');
  const onChange = () => { if (current() === 'system') apply('system'); };
  if (query.addEventListener) query.addEventListener('change', onChange);
  else if (query.addListener) query.addListener(onChange);
}
