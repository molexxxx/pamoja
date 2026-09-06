import { t, nf, fmt, ago } from './i18n.js';
import { tileViz, detailGraph, trendArrow, isDiscrete, vizOf, esc } from './viz/index.js';

/**
 * Keeps each event log scrolled to its newest line unless the user has scrolled up.
 *
 * Binds a one-time scroll listener per log that tracks whether the view is pinned to the
 * bottom, then re-pins after a re-render. Call from the host component's `updated` hook.
 *
 * @param {HTMLElement} root - the component root to find `.dlog` logs within.
 * @returns {void}
 */
export function stickLog(root)
{
  if (!root) return;
  root.querySelectorAll('.dlog').forEach((d) =>
  {
    if (!d._bound)
    {
      d._bound = true;
      d._stick = true;
      d.addEventListener('scroll', () => { d._stick = d.scrollHeight - d.scrollTop - d.clientHeight < 40; });
    }
    if (d._stick) d.scrollTop = d.scrollHeight;
  });
}

/**
 * Renders the shared sensor detail body: hero visualization, history graph, stat grid,
 * and a merged recent-events log.
 *
 * @param {object} s - the sensor, carrying `reading`, `history`, `events`, and `mode`.
 * @returns {string} the detail-body markup (no header).
 */
export function sensorDetailBody(s)
{
  const r = s.reading;
  const h = s.history || [];
  const min = h.length ? Math.min(...h) : r.value;
  const max = h.length ? Math.max(...h) : r.value;
  const avg = h.length ? h.reduce((a, b) => a + b, 0) / h.length : r.value;
  const u = t('unit.' + r.unit);
  const stat = (label, val, unit) => `<div class="statbox"><span>${label}</span><b>${val}${unit ? `<i>${unit}</i>` : ''}</b></div>`;
  const samples = h.slice(0, -1).reverse().slice(0, 8).map((v, i) => ({ level: 'info', code: 'reading.ok', value: v, ageSecs: (i + 1) * 30 + 20 }));
  const merged = [...(s.events || []), ...samples].sort((a, b) => (b.ageSecs || 0) - (a.ageSecs || 0)).slice(-10);
  const events = merged.map((e) =>
  {
    const v = e.value != null ? ' · ' + fmt(e.value) : '';
    const time = e.ageSecs != null ? ago(e.ageSecs) : '';
    return `<div class="dlog-line" data-level="${e.level}"><span class="lt">${time}</span><span class="lm">${esc(t('event.' + e.code))}${v}</span></div>`;
  }).join('') || `<div class="dlog-line"><span class="lm">${t('ui.noEvents')}</span></div>`;

  if (isDiscrete(r) && vizOf(r) !== 'wave')
  {
    return `
      <div class="modal-hero hero-discrete">
        ${tileViz(s, true)}
      </div>
      <div class="modal-body">
        <div>
          <div class="statgrid">
            ${stat(t('ui.cadence'), t('mode.' + s.mode))}
            ${s.battery != null ? stat(t('ui.battery'), nf(s.battery, { style: 'percent', maximumFractionDigits: 0 })) : ''}
          </div>
        </div>
        <div class="span"><div class="dsection">${t('ui.events')}</div><div class="dlog">${events}</div></div>
        <div class="span dsection" style="margin:0">${t('ui.sensorId')}: ${esc(s.id)}</div>
      </div>`;
  }

  return `
    <div class="modal-hero">
      ${tileViz(s, true)}
      <div class="modal-readout"><b>${fmt(r.value)}</b><span>${u}</span>${r.trend ? `<span class="strend" data-dir="${r.trend}">${trendArrow(r.trend)}</span>` : ''}</div>
    </div>
    <div class="modal-body">
      <div class="span"><div class="dsection">${t('ui.history')}</div>${detailGraph(h, r.band)}</div>
      <div>
        <div class="statgrid">
          ${stat(t('ui.min'), fmt(min), u)}
          ${stat(t('ui.max'), fmt(max), u)}
          ${stat(t('ui.avg'), fmt(avg), u)}
          ${r.band ? stat(t('ui.band'), `${nf(r.band[0])}–${nf(r.band[1])}`, u) : ''}
          ${s.battery != null ? stat(t('ui.battery'), nf(s.battery, { style: 'percent', maximumFractionDigits: 0 })) : ''}
          ${stat(t('ui.cadence'), t('mode.' + s.mode))}
        </div>
      </div>
      <div class="span"><div class="dsection">${t('ui.events')}</div><div class="dlog">${events}</div></div>
      <div class="span dsection" style="margin:0">${t('ui.sensorId')}: ${esc(s.id)}</div>
    </div>`;
}
