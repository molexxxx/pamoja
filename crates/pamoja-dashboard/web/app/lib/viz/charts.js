// viz/charts.js - line, area, and histogram charts plus the status ring.
//
// The history visualizations: the tile sparkline, the expanded detail graph and its
// safe-band limits, the big sparkline and per-sample histogram for the expanded view,
// and the banner status ring. All hand-drawn inline SVG, no chart library.

import { nf } from '../i18n.js';

/**
 * Maps a history series to a polyline points string fitted to a box.
 *
 * @param {number[]} history - the values, oldest first.
 * @param {number} w - the box width.
 * @param {number} h - the box height.
 * @param {number} [pad=3] - the vertical padding inside the box.
 * @returns {string|null} the `"x,y x,y ..."` points, or `null` for fewer than two values.
 */
export function sparkPoints(history, w, h, pad = 3)
{
  if (!history || history.length < 2) return null;
  const min = Math.min(...history), max = Math.max(...history), span = max - min || 1;
  const step = w / (history.length - 1);
  return history.map((v, i) => `${(i * step).toFixed(1)},${(h - pad - ((v - min) / span) * (h - pad * 2)).toFixed(1)}`).join(' ');
}

/**
 * Renders the compact tile sparkline (line over an area).
 *
 * @param {number[]} history - the values, oldest first.
 * @returns {string} the sparkline SVG markup.
 */
export function miniSpark(history)
{
  const pts = sparkPoints(history, 100, 26);
  if (!pts) return '<svg aria-hidden="true" class="spark" viewBox="0 0 100 26" preserveAspectRatio="none"></svg>';
  return `<svg aria-hidden="true" class="spark" viewBox="0 0 100 26" preserveAspectRatio="none">
    <polyline class="sp-area" points="0,26 ${pts} 100,26"/><polyline class="sp-line" points="${pts}"/></svg>`;
}

/**
 * Renders the expanded detail graph: a line over an area, a midline, the current dot,
 * and any safe-band limit lines that fall within the plotted range.
 *
 * @param {number[]} history - the values, oldest first.
 * @param {[number, number]} [band] - the safe band `[low, high]` to draw as limits.
 * @returns {string} the detail-graph markup.
 */
export function detailGraph(history, band)
{
  const w = 320, h = 100, pad = 12;
  const pts = sparkPoints(history, w, h, pad);
  if (!pts) return `<div class="graph"></div>`;
  const min = Math.min(...history), max = Math.max(...history), span = max - min || 1;
  const y = (v) => h - pad - ((v - min) / span) * (h - pad * 2);
  // Draw a safe-band limit only where it actually falls within the plotted range, so a
  // band far outside the data does not paint a flat box over the whole graph.
  let limits = '';
  if (band)
  {
    for (const edge of band)
    {
      if (edge > min && edge < max)
      {
        const ly = y(edge).toFixed(1);
        limits += `<line class="dg-limit" x1="0" y1="${ly}" x2="${w}" y2="${ly}"/><text class="dg-limit-t" x="${w - 4}" y="${(+ly - 3).toFixed(1)}" text-anchor="end">${nf(edge)}</text>`;
      }
    }
  }
  const grid = [0.5].map((g) => `<line class="dg-grid" x1="0" y1="${(h * g).toFixed(1)}" x2="${w}" y2="${(h * g).toFixed(1)}"/>`).join('');
  const last = pts.split(' ').pop().split(',');
  return `<div class="graph">
    <svg aria-hidden="true" class="detail-graph" viewBox="0 0 ${w} ${h}" preserveAspectRatio="none">
      ${grid}${limits}
      <polyline class="dg-area" points="0,${h} ${pts} ${w},${h}"/>
      <polyline class="dg-line" points="${pts}"/>
      <circle class="dg-dot" cx="${last[0]}" cy="${last[1]}" r="2.6"/>
    </svg>
    <span class="dg-ax dg-max">${nf(max)}</span><span class="dg-ax dg-min">${nf(min)}</span>
  </div>`;
}

/**
 * Renders the banner status ring: a progress arc with a tick or alert mark.
 *
 * @param {string} status - the fleet status, one of `'ok'`, `'warn'`, or `'alarm'`.
 * @returns {string} the status-ring SVG markup.
 */
export function bannerRing(status)
{
  const color = `var(--${status === 'ok' ? 'ok' : status === 'warn' ? 'warn' : 'alarm'})`;
  const r = 26, c = 2 * Math.PI * r, frac = status === 'ok' ? 1 : status === 'warn' ? 0.66 : 0.4;
  const mark = status === 'ok'
    ? `<path d="M22 33 L29 40 L43 24" fill="none" stroke="${color}" stroke-width="5" stroke-linecap="round" stroke-linejoin="round"/>`
    : `<path d="M33 19 L33 36 M33 44 L33 46" fill="none" stroke="${color}" stroke-width="5" stroke-linecap="round"/>`;
  return `<svg viewBox="0 0 66 66" width="100%" height="100%" aria-hidden="true">
    <circle cx="33" cy="33" r="${r}" fill="none" stroke="var(--track)" stroke-width="4"/>
    <circle cx="33" cy="33" r="${r}" fill="none" stroke="${color}" stroke-width="4" stroke-linecap="round" stroke-dasharray="${(frac * c).toFixed(1)} ${c.toFixed(1)}" transform="rotate(-90 33 33)" style="filter: var(--viz-glow)"/>
    ${mark}</svg>`;
}

/**
 * Renders the expanded-view sparkline: a thin line over an area with a baseline, sample
 * dots, an emphasized current point, and min/max labels.
 *
 * @param {number[]} history - the values, oldest first.
 * @returns {string} the big-sparkline SVG markup.
 */
export function bigSpark(history)
{
  const w = 300, h = 116, pad = 14;
  if (!history || history.length < 2) return `<svg aria-hidden="true" class="bspark" viewBox="0 0 ${w} ${h}"></svg>`;
  const min = Math.min(...history), max = Math.max(...history), span = max - min || 1;
  const step = w / (history.length - 1);
  const pts = history.map((v, i) => [+(i * step).toFixed(1), +(h - pad - ((v - min) / span) * (h - pad * 2)).toFixed(1)]);
  const line = pts.map((p) => p.join(',')).join(' ');
  const dots = pts.map((p, i) => `<circle class="bsp-dot${i === pts.length - 1 ? ' last' : ''}" cx="${p[0]}" cy="${p[1]}" r="${i === pts.length - 1 ? 3 : 1.6}"/>`).join('');
  return `<svg aria-hidden="true" class="bspark" viewBox="0 0 ${w} ${h}" preserveAspectRatio="none">
    <line class="bsp-grid" x1="0" y1="${h / 2}" x2="${w}" y2="${h / 2}"/>
    <polyline class="bsp-area" points="0,${h} ${line} ${w},${h}"/>
    <polyline class="bsp-line" points="${line}"/>
    ${dots}
    <text class="bsp-ax" x="${w - 4}" y="12" text-anchor="end">${nf(max)}</text>
    <text class="bsp-ax" x="${w - 4}" y="${h - 4}" text-anchor="end">${nf(min)}</text>
  </svg>`;
}

/**
 * Renders the expanded-view per-sample histogram, with the most recent bar emphasized
 * and the safe band shaded as a target zone.
 *
 * @param {number[]} history - the values, oldest first.
 * @param {[number, number]} [band] - the safe band `[low, high]` to shade.
 * @returns {string} the histogram markup, wrapped with min/max labels.
 */
export function bigBars(history, band)
{
  const w = 300, h = 120, pad = 12;
  if (!history || history.length < 2) return `<svg aria-hidden="true" class="bbars" viewBox="0 0 ${w} ${h}"></svg>`;
  const min = Math.min(...history), max = Math.max(...history), span = max - min || 1;
  const n = history.length, slot = (w - pad * 2) / n, bw = slot * 0.6;
  const y = (v) => h - pad - ((v - min) / span) * (h - pad * 2);
  let zone = '';
  if (band)
  {
    const lo = Math.max(0, Math.min(1, (band[0] - min) / span)), hi = Math.max(0, Math.min(1, (band[1] - min) / span));
    if (hi > 0 && lo < 1) { const yt = y(min + hi * span), yb = y(min + lo * span); zone = `<rect class="bbar-zone" x="0" y="${yt.toFixed(1)}" width="${w}" height="${Math.max(0, yb - yt).toFixed(1)}"/>`; }
  }
  const bars = history.map((v, i) =>
  {
    const by = y(v), bh = h - pad - by, x = pad + i * slot + (slot - bw) / 2;
    return `<rect class="bbar${i === n - 1 ? ' last' : ''}" x="${x.toFixed(1)}" y="${by.toFixed(1)}" width="${bw.toFixed(1)}" height="${(bh + 1.5).toFixed(1)}" rx="1.2"/>`;
  }).join('');
  // The min/max labels live in HTML, not the SVG, so the none-aspect stretch that fills
  // the bars to full width never warps the text.
  return `<div class="bbars-wrap">
    <svg aria-hidden="true" class="bbars" viewBox="0 0 ${w} ${h}" preserveAspectRatio="none">${zone}${bars}</svg>
    <span class="bb-ax bb-max">${nf(max)}</span><span class="bb-ax bb-min">${nf(min)}</span>
  </div>`;
}
