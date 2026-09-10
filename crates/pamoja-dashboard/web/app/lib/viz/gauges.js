// viz/gauges.js - per-quantity analog gauges.
//
// Each sensor kind gets the instrument that suits its quantity - an arch gauge, a
// thermometer, a droplet, a battery cell, a half-dial, an anemometer, a sun, or a bar -
// so a wall of tiles reads as a varied instrument panel rather than identical graphs.
// All hand-drawn inline SVG; sizing is driven by the `big` (expanded) flag.

import { nf, fmt, t } from '../i18n.js';
import { fracOf, unitSup, ends, gaugeTicks } from './util.js';

/**
 * Renders a 270-degree arch gauge for a fractional/percentage reading.
 *
 * @param {{value: number, band?: [number, number], unit: string}} r - the reading.
 * @param {boolean} big - whether this is the expanded (labeled) view.
 * @returns {string} the gauge SVG markup.
 */
export function radial(r, big)
{
  const v = r.value, band = r.band, rad = 20, c = 2 * Math.PI * rad, sweep = c * 0.75, f = fracOf(v, band);
  let ticks = '';
  for (const p of [0, 0.25, 0.5, 0.75, 1])
  {
    const a = p * 270 * Math.PI / 180;
    ticks += `<line class="vv-tick" x1="${(28 + 15.5 * Math.cos(a)).toFixed(1)}" y1="${(28 + 15.5 * Math.sin(a)).toFixed(1)}" x2="${(28 + 21 * Math.cos(a)).toFixed(1)}" y2="${(28 + 21 * Math.sin(a)).toFixed(1)}"/>`;
  }
  const center = `<text class="vv-num${big ? '' : ' sm'}" x="28" y="31" text-anchor="middle">${fmt(v)}</text>`;
  return `<svg aria-hidden="true" class="tv" viewBox="0 0 56 52">
    <g transform="rotate(135 28 28)">
      <circle class="vv-track" cx="28" cy="28" r="${rad}" stroke-dasharray="${sweep.toFixed(1)} ${c.toFixed(1)}"/>
      ${ticks}
      <circle class="vv-arc" cx="28" cy="28" r="${rad}" stroke-dasharray="${(f * sweep).toFixed(1)} ${c.toFixed(1)}"/>
    </g>${center}${ends(band, big, t('unit.' + r.unit))}</svg>`;
}

/**
 * Renders a thermometer for a temperature reading.
 *
 * @param {{value: number, band?: [number, number], unit: string}} r - the reading.
 * @param {boolean} big - whether this is the expanded (labeled) view.
 * @returns {string} the thermometer SVG markup.
 */
export function therm(r, big)
{
  const v = r.value, band = r.band, f = fracOf(v, band), top = 6, bottom = 34, h = bottom - top, mh = f * h;
  let ticks = '';
  for (const p of [0, 0.33, 0.66, 1]) ticks += `<line class="vv-tick" x1="18" y1="${(bottom - p * h).toFixed(1)}" x2="21" y2="${(bottom - p * h).toFixed(1)}"/>`;
  const lab = big && band ? `<text class="vv-end" x="23" y="${(top + 3).toFixed(1)}">${nf(band[1])}</text><text class="vv-end" x="23" y="${bottom}">${nf(band[0])}</text>${unitSup(t('unit.' + r.unit), 38, 8)}` : '';
  return `<svg aria-hidden="true" class="tv" viewBox="0 0 ${big ? 40 : 26} 50">
    <rect class="vv-tube" x="9" y="4" width="8" height="34" rx="4"/>
    ${ticks}
    <rect class="vv-merc" x="11" y="${(bottom - mh).toFixed(1)}" width="4" height="${(mh + 8).toFixed(1)}" rx="2"/>
    <circle class="vv-bulb" cx="13" cy="40" r="6.5"/>${lab}</svg>`;
}

/**
 * Renders a liquid-filled droplet for a humidity/moisture reading.
 *
 * @param {{value: number, band?: [number, number], unit: string}} r - the reading.
 * @param {string} uid - a unique suffix for the SVG clip-path id.
 * @param {boolean} big - whether this is the expanded (labeled) view.
 * @returns {string} the droplet SVG markup.
 */
export function droplet(r, uid, big)
{
  const f = fracOf(r.value, r.band);
  const path = 'M22 6 C32 22 36 28 36 33 a14 14 0 1 1 -28 0 C8 28 12 22 22 6 Z';
  const pct = big ? `<text class="vv-num" x="22" y="36" text-anchor="middle">${Math.round(f * 100)}</text>` : '';
  const cap = big && r.band ? `<text class="vv-end" x="5" y="55">${nf(r.band[0])}</text><text class="vv-end" x="39" y="55" text-anchor="end">${nf(r.band[1])}</text>${unitSup(t('unit.' + r.unit), 42, 8)}` : '';
  return `<svg aria-hidden="true" class="tv" viewBox="0 0 44 ${big ? 58 : 50}"><defs><clipPath id="d${uid}"><path d="${path}"/></clipPath></defs>
    <path class="vv-back" d="${path}"/>
    <g clip-path="url(#d${uid})"><rect class="vv-fill" x="6" width="32" height="50" y="0" style="transform:translateY(${((1 - f) * 50).toFixed(1)}px)"/>
      <ellipse class="vv-wave" cx="22" cy="${((1 - f) * 50).toFixed(1)}" rx="17" ry="3"/></g>
    <path class="vv-shell" d="${path}"/>
    <ellipse class="vv-shine" cx="16" cy="20" rx="3.2" ry="5.5"/>${pct}${cap}</svg>`;
}

/**
 * Renders a battery cell: a segmented fill proportional to charge, with a charging bolt
 * when the reading is rising.
 *
 * @param {{value: number, band?: [number, number], trend?: string}} r - the reading.
 * @param {string} uid - a unique suffix for the SVG clip-path id.
 * @returns {string} the battery SVG markup.
 */
export function battery(r, uid)
{
  const f = fracOf(r.value, r.band);
  const x0 = 9, innerW = 52;
  const segs = [1, 2, 3].map((i) => `<line class="bat-seg" x1="${(x0 + innerW * i / 4).toFixed(1)}" y1="12" x2="${(x0 + innerW * i / 4).toFixed(1)}" y2="34"/>`).join('');
  const bolt = r.trend === 'rising' ? '<path class="bat-bolt" d="M40 13 L31 26 L38 26 L35 35 L47 21 L40 21 Z"/>' : '';
  return `<svg aria-hidden="true" class="tv" viewBox="0 0 80 46">
      <defs><clipPath id="bf${uid}"><rect x="${x0}" y="12" width="${innerW}" height="22" rx="3"/></clipPath></defs>
      <rect class="bat-body" x="6" y="9" width="58" height="28" rx="5"/>
      <rect class="bat-cap" x="65" y="16" width="5" height="14" rx="2"/>
      <g clip-path="url(#bf${uid})"><rect class="bat-fill" x="${x0}" y="12" width="${(f * innerW).toFixed(1)}" height="22"/></g>
      ${segs}${bolt}
    </svg>`;
}

/**
 * Renders a half-dial gauge with a needle for a pressure/flow reading.
 *
 * @param {{value: number, band?: [number, number], unit: string}} r - the reading.
 * @param {boolean} big - whether this is the expanded (labeled) view.
 * @returns {string} the dial SVG markup.
 */
export function dial(r, big)
{
  const v = r.value, band = r.band, f = fracOf(v, band), cx = 32, cy = 36, rad = 24;
  const a = Math.PI - f * Math.PI, nx = cx + Math.cos(a) * rad * 0.82, ny = cy - Math.sin(a) * rad * 0.82;
  let ticks = '';
  for (const p of [0, 0.25, 0.5, 0.75, 1]) { const t2 = Math.PI - p * Math.PI; ticks += `<line class="vv-tick" x1="${(cx + Math.cos(t2) * (rad - 4)).toFixed(1)}" y1="${(cy - Math.sin(t2) * (rad - 4)).toFixed(1)}" x2="${(cx + Math.cos(t2) * rad).toFixed(1)}" y2="${(cy - Math.sin(t2) * rad).toFixed(1)}"/>`; }
  const lab = big && band ? `<text class="vv-end" x="${cx - rad}" y="48">${nf(band[0])}</text><text class="vv-end" x="${cx + rad}" y="48" text-anchor="end">${nf(band[1])}</text>${unitSup(t('unit.' + r.unit), 60, 9)}` : '';
  return `<svg aria-hidden="true" class="tv" viewBox="0 0 64 50">
    <path class="vv-track" fill="none" d="M${cx - rad} ${cy} A ${rad} ${rad} 0 0 1 ${cx + rad} ${cy}"/>
    <path class="vv-arcline" fill="none" pathLength="100" stroke-dasharray="${(f * 100).toFixed(1)} 100" d="M${cx - rad} ${cy} A ${rad} ${rad} 0 0 1 ${cx + rad} ${cy}"/>
    ${ticks}
    <line class="vv-needle" x1="${cx}" y1="${cy}" x2="${nx.toFixed(1)}" y2="${ny.toFixed(1)}"/>
    <circle class="vv-hub" cx="${cx}" cy="${cy}" r="3"/>${lab}</svg>`;
}

/**
 * Renders an anemometer: the value fills a 270-degree arc and the rotor spins faster
 * the stronger the wind.
 *
 * @param {{value: number, band?: [number, number], unit: string}} r - the reading.
 * @param {boolean} big - whether this is the expanded (labeled) view.
 * @returns {string} the anemometer SVG markup.
 */
export function wind(r, big)
{
  const v = r.value, band = r.band, rad = 20, c = 2 * Math.PI * rad, sweep = c * 0.75, f = fracOf(v, band);
  const arm = (deg) => { const a = deg * Math.PI / 180, x = 28 + 8 * Math.cos(a), y = 28 + 8 * Math.sin(a); return `<line class="vv-arm" x1="28" y1="28" x2="${x.toFixed(1)}" y2="${y.toFixed(1)}"/><circle class="vv-cup" cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="2.8"/>`; };
  return `<svg aria-hidden="true" class="tv" viewBox="0 0 56 52">
    <g transform="rotate(135 28 28)">
      <circle class="vv-track" cx="28" cy="28" r="${rad}" stroke-dasharray="${sweep.toFixed(1)} ${c.toFixed(1)}"/>
      ${gaugeTicks()}
      <circle class="vv-arc" cx="28" cy="28" r="${rad}" stroke-dasharray="${(f * sweep).toFixed(1)} ${c.toFixed(1)}"/>
    </g>
    <g class="vv-rotor">${arm(-90)}${arm(30)}${arm(150)}</g>
    <circle class="vv-hub" cx="28" cy="28" r="2.8"/>${ends(band, big, t('unit.' + r.unit))}</svg>`;
}

/**
 * Renders a light gauge: illuminance fills a 270-degree arc and the sun's core grows
 * and its corona brightens the more light there is.
 *
 * @param {{value: number, band?: [number, number], unit: string}} r - the reading.
 * @param {boolean} big - whether this is the expanded (labeled) view.
 * @returns {string} the light-gauge SVG markup.
 */
export function sun(r, big)
{
  const v = r.value, band = r.band, rad = 20, c = 2 * Math.PI * rad, sweep = c * 0.75, f = fracOf(v, band);
  let rays = '';
  for (let i = 0; i < 8; i++) { const a = i * 45 * Math.PI / 180, x1 = 28 + 9 * Math.cos(a), y1 = 28 + 9 * Math.sin(a), x2 = 28 + 12.5 * Math.cos(a), y2 = 28 + 12.5 * Math.sin(a); rays += `<line class="vv-ray" x1="${x1.toFixed(1)}" y1="${y1.toFixed(1)}" x2="${x2.toFixed(1)}" y2="${y2.toFixed(1)}"/>`; }
  return `<svg aria-hidden="true" class="tv" viewBox="0 0 56 52">
    <g transform="rotate(135 28 28)">
      <circle class="vv-track" cx="28" cy="28" r="${rad}" stroke-dasharray="${sweep.toFixed(1)} ${c.toFixed(1)}"/>
      ${gaugeTicks()}
      <circle class="vv-arc" cx="28" cy="28" r="${rad}" stroke-dasharray="${(f * sweep).toFixed(1)} ${c.toFixed(1)}"/>
    </g>
    <g class="vv-rays" style="opacity:${(0.35 + f * 0.65).toFixed(2)}">${rays}</g>
    <circle class="vv-core" cx="28" cy="28" r="${(4 + f * 4).toFixed(1)}"/>${ends(band, big, t('unit.' + r.unit))}</svg>`;
}

/**
 * Renders a horizontal bar gauge with an optional safe-band tick and scale.
 *
 * @param {number} v - the reading value.
 * @param {[number, number]} [band] - the safe band `[low, high]`; sets the scale top.
 * @param {boolean} big - whether to draw the labeled scale.
 * @param {string} [unit] - the already-localized unit label for the scale.
 * @returns {string} the bar-gauge markup.
 */
export function barViz(v, band, big, unit = '')
{
  const max = band ? band[1] : (v * 1.4 || 1);
  const f = Math.max(0, Math.min(1, v / (max || 1)));
  const ticks = band ? `<span class="vb-tick" style="inset-inline-start:${(Math.max(0, Math.min(1, band[0] / (max || 1))) * 100).toFixed(1)}%"></span>` : '';
  const scale = big ? `<div class="vbar-scale"><span>0</span><span>${nf(max)}${unit ? `<sup class="vb-unit">${unit}</sup>` : ''}</span></div>` : '';
  return `<div class="vbar${big ? ' big' : ''}"><div class="vbar-track">${ticks}<i style="inline-size:${(f * 100).toFixed(1)}%"></i></div>${scale}</div>`;
}
