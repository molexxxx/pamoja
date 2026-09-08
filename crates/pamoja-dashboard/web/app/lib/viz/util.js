// viz/util.js - shared low-level helpers for the visualizations.
//
// Pure string/number helpers used across the gauge, glyph, and chart builders: HTML
// escaping, status-to-color mapping, trend arrows, and the safe-band geometry and
// gauge furniture the SVG gauges share.

import { nf } from '../i18n.js';

/**
 * Escapes the HTML-significant characters in a value for safe interpolation into markup.
 *
 * @param {*} s - the value to escape; coerced to a string first.
 * @returns {string} the escaped text.
 */
export const esc = (s) =>
  String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

/**
 * Maps a health status to its CSS color variable.
 *
 * @param {string} s - the status, one of `'ok'`, `'warn'`, or `'alarm'`.
 * @returns {string} the `var(--...)` color reference for that status.
 */
export const statusColor = (s) => `var(--${s === 'ok' ? 'ok' : s === 'warn' ? 'warn' : 'alarm'})`;

/**
 * Picks the arrow glyph for a reading trend.
 *
 * @param {string} tr - the trend, one of `'rising'`, `'falling'`, or `'steady'`.
 * @returns {string} the matching arrow character.
 */
export const trendArrow = (tr) => (tr === 'rising' ? '↑' : tr === 'falling' ? '↓' : '→');

/**
 * Widens a safe band into a display range, padded so the band sits inside the gauge.
 *
 * @param {[number, number]} band - the safe band `[low, high]`.
 * @returns {[number, number]} the padded `[min, max]` range to plot against.
 */
export function rangeOf(band)
{
  const [lo, hi] = band;
  const pad = Math.max((hi - lo) * 0.5, 1);
  return [lo - pad, hi + pad];
}

/**
 * Computes a value's fractional position within its padded display range.
 *
 * @param {number} v - the reading value.
 * @param {[number, number]} [band] - the safe band; a missing band centers the value.
 * @returns {number} the fraction in `[0, 1]`.
 */
export function fracOf(v, band)
{
  if (!band) return 0.5;
  const [min, max] = rangeOf(band);
  return Math.min(1, Math.max(0, (v - min) / (max - min || 1)));
}

/**
 * Renders the small unit superscript that floats in a gauge corner.
 *
 * @param {string} unit - the already-localized unit label; empty renders nothing.
 * @param {number} x - the text x coordinate.
 * @param {number} y - the text y coordinate.
 * @returns {string} the unit `<text>` markup, or an empty string.
 */
export const unitSup = (unit, x, y) =>
  (unit ? `<text class="vv-unit" x="${x}" y="${y}" text-anchor="end">${unit}</text>` : '');

/**
 * Renders the band-end labels for the 270-degree arch gauges (radial, wind, sun).
 *
 * The low value sits at the left arch end, the high value at the right, and the unit
 * floats in the top-right corner so it never widens a value or pushes it off its end.
 *
 * @param {[number, number]} band - the safe band `[low, high]`.
 * @param {boolean} big - whether this is the expanded (labeled) view.
 * @param {string} [unit] - the already-localized unit label.
 * @returns {string} the end-label markup, or an empty string when not big or bandless.
 */
export const ends = (band, big, unit = '') =>
  (big && band
    ? `<text class="vv-end" x="6" y="49">${nf(band[0])}</text><text class="vv-end" x="50" y="49" text-anchor="end">${nf(band[1])}</text>${unitSup(unit, 53, 9)}`
    : '');

/**
 * Builds the five evenly-spaced tick marks for a 270-degree gauge centered at (28,28).
 *
 * @returns {string} the tick `<line>` markup.
 */
export function gaugeTicks()
{
  let ticks = '';
  for (const p of [0, 0.25, 0.5, 0.75, 1])
  {
    const a = p * 270 * Math.PI / 180;
    ticks += `<line class="vv-tick" x1="${(28 + 15.5 * Math.cos(a)).toFixed(1)}" y1="${(28 + 15.5 * Math.sin(a)).toFixed(1)}" x2="${(28 + 21 * Math.cos(a)).toFixed(1)}" y2="${(28 + 21 * Math.sin(a)).toFixed(1)}"/>`;
  }
  return ticks;
}
