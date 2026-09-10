// viz/glyphs.js - discrete and non-numeric visualizations.
//
// The instruments for readings that are not a single number on a scale: a plain count,
// a neighbor-mesh map, an acoustic waveform, a hash-chained tamper log, a state chip,
// and a drip valve. Each sizes to its own content rather than the fixed gauge slot.

import { nf, fmt, t } from '../i18n.js';
import { esc } from './util.js';

/**
 * Renders a plain numeric count.
 *
 * @param {{value: number}} r - the reading.
 * @param {boolean} big - whether this is the expanded view.
 * @returns {string} the count markup.
 */
export function count(r, big)
{
  return `<div class="cnt${big ? ' big' : ''}">${fmt(r.value)}</div>`;
}

/**
 * Renders a neighbor-mesh map: peers feeding a gateway with traveling packets.
 *
 * @param {{value?: number, state?: string}} r - the reading; `value` hints peer count.
 * @param {boolean} big - whether this is the expanded view.
 * @param {number} [nodeCount] - the real peer count from the group's other sensors.
 * @returns {string} the mesh-map SVG markup.
 */
export function mesh(r, big, nodeCount)
{
  const W = 220, H = 112;
  const peers = Math.max(2, Math.min(7, nodeCount != null ? nodeCount : (Math.round(r && r.value) || 5)));
  const learning = !!(r && r.state && r.state.endsWith('learning'));
  const gw = peers;
  const pts = [];
  for (let i = 0; i < peers; i++)
  {
    const tt = peers === 1 ? 0.5 : i / (peers - 1);
    pts.push([26 + tt * (W - 82), H * 0.5 + (i % 2 ? -24 : 24)]);
  }
  pts.push([W - 18, H * 0.5]);
  const links = [];
  for (let i = 0; i < peers - 1; i++) links.push([i, i + 1]);
  for (let i = 0; i < peers; i++) links.push([i, gw]);
  const packets = [];
  for (let i = 0; i < peers; i++) packets.push([i, gw]);
  if (!packets.length) packets.push([0, gw]);
  const linkSvg = links.map((l, i) => `<line x1="${pts[l[0]][0].toFixed(1)}" y1="${pts[l[0]][1].toFixed(1)}" x2="${pts[l[1]][0].toFixed(1)}" y2="${pts[l[1]][1].toFixed(1)}" class="msh-link${learning && i === links.length - 1 ? ' weak' : ''}"/>`).join('');
  const pkSvg = packets.map((p, i) =>
  {
    const path = p.map((k, j) => `${j ? 'L' : 'M'}${pts[k][0].toFixed(1)},${pts[k][1].toFixed(1)}`).join(' ');
    const dur = (2.2 + i * 0.5).toFixed(2), b = (i * 0.6).toFixed(2);
    return `<circle r="3" class="msh-pk" opacity="0"><animateMotion dur="${dur}s" begin="${b}s" repeatCount="indefinite" path="${path}"/><animate attributeName="opacity" values="0;1;1;0" keyTimes="0;0.12;0.88;1" dur="${dur}s" begin="${b}s" repeatCount="indefinite"/></circle>`;
  }).join('');
  const nodeSvg = pts.map((c, i) => `<circle cx="${c[0].toFixed(1)}" cy="${c[1].toFixed(1)}" r="${i === gw ? 7.5 : 5}" class="msh-node${i === gw ? ' gw' : ''}"/>`).join('');
  return `<svg class="msh${big ? ' big' : ''}" viewBox="0 0 ${W} ${H}" preserveAspectRatio="xMidYMid meet" aria-hidden="true">${linkSvg}${pkSvg}${nodeSvg}</svg>`;
}

/**
 * Renders an acoustic monitor: a fixed-envelope waveform that flips hot (tall, fast,
 * red) when the reading is a threat rather than ambient.
 *
 * @param {{status: string, state?: string}} r - the reading.
 * @param {boolean} big - whether this is the expanded view.
 * @returns {string} the waveform markup.
 */
export function wave(r, big)
{
  const n = big ? 40 : 28;
  const threat = r.status !== 'ok' || (r.state && !r.state.endsWith('ambient'));
  let bars = '';
  for (let i = 0; i < n; i++)
  {
    const env = 0.34 + 0.4 * Math.abs(Math.sin(i * 0.5)) + 0.22 * Math.abs(Math.sin(i * 1.3 + 0.7));
    const h = Math.max(0.16, Math.min(1, env));
    const delay = (-((i * 0.37) % 1.1)).toFixed(2);
    bars += `<i style="--h:${h.toFixed(2)};animation-delay:${delay}s"></i>`;
  }
  const tag = r.state ? t(r.state) : t('acoustic.ambient');
  const loc = threat ? '<span class="wv-loc">≈ 2.1 km · NE</span>' : '';
  return `<div class="wv${threat ? ' hot' : ''}${big ? ' big' : ''}"><div class="wv-bars" aria-hidden="true">${bars}</div><div class="wv-cap"><span class="wv-tag">${esc(tag)}</span>${loc}</div></div>`;
}

/**
 * Renders a tamper-evident log: a row of hash-chained blocks and the sealed-record count.
 *
 * @param {{value: number}} r - the reading; `value` is the record count.
 * @param {boolean} big - whether this is the expanded view.
 * @returns {string} the chain markup.
 */
export function chain(r, big)
{
  const count = Math.round(r.value);
  const n = 4;
  const hex = (k) => (((Math.abs(k) * 2654435761) >>> 0) % 0x10000).toString(16).padStart(4, '0');
  let row = '';
  for (let i = n - 1; i >= 0; i--)
  {
    row += `<span class="chn-blk${i === 0 ? ' new' : ''}">#${hex(count - i)}</span>`;
    if (i) row += '<span class="chn-arr">→</span>';
  }
  return `<div class="chn${big ? ' big' : ''}">
    <div class="chn-row">${row}</div>
    <div class="chn-foot"><span class="chn-count">${nf(count)}</span><span class="chn-seal">${t('ui.sealed')}</span></div>
  </div>`;
}

const ON_STATE = /(open|on|synced|online|up|nominal|active|ready|optimized|optimized)$/;

/**
 * Renders a labeled state chip, lit when the state code reads as "on".
 *
 * @param {{status: string, state?: string}} r - the reading.
 * @param {boolean} big - whether this is the expanded view.
 * @returns {string} the chip markup.
 */
export function chip(r, big)
{
  const txt = r.state ? t(r.state) : t('status.' + r.status);
  const on = r.state && ON_STATE.test(r.state) ? '1' : '0';
  return `<div class="vchip${big ? ' big' : ''}" data-status="${r.status}" data-on="${on}"><span class="vchip-dot"></span>${esc(txt)}</div>`;
}

/**
 * Renders a drip valve: a lever along the flow when open, across it when closed.
 *
 * @param {{state?: string}} r - the reading; an "open"/"on" state shows drops.
 * @param {boolean} big - whether this is the expanded view.
 * @returns {string} the valve markup.
 */
export function valve(r, big)
{
  const open = !!(r.state && (r.state.endsWith('open') || r.state.endsWith('on')));
  const drops = open ? '<circle class="vlv-drop d1" cx="28" cy="33" r="2.1"/><circle class="vlv-drop d2" cx="28" cy="33" r="2.1"/>' : '';
  return `<div class="vlv${big ? ' big' : ''}" data-on="${open ? 1 : 0}">
    <svg class="vlv-svg" viewBox="0 0 56 48" aria-hidden="true">
      <rect class="vlv-pipe" x="4" y="22" width="48" height="8" rx="4"/>
      <circle class="vlv-body" cx="28" cy="26" r="8.5"/>
      <line class="vlv-handle" x1="17" y1="26" x2="39" y2="26" transform="rotate(${open ? 0 : 90} 28 26)"/>
      <circle class="vlv-hub" cx="28" cy="26" r="2.4"/>
      ${drops}
    </svg>
    <span class="vlv-state">${esc(t(r.state))}</span>
  </div>`;
}
