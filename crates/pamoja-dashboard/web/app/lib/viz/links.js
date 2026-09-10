// viz/links.js - the link-type palette and connection chips.
//
// The presentation constants for a group's link (display name, accent color, and the
// nominal floor RSSI used to derive a believable signal reading) plus the small markup
// builders for the signal bars and the connection chip shown in card and modal headers.

/** Human-readable display names for each link kind, keyed by the wire enum value. */
export const LINK_NAMES = { lora: 'LoRa', wifi: 'Wi-Fi', cellular: 'Cellular', nbiot: 'NB-IoT', satellite: 'Satellite', ethernet: 'Ethernet', mesh: 'Mesh' };

/** Accent colors for each link kind, used for chips, edges, and packets. */
export const LINK_COLORS = { lora: 'var(--lk-lora)', wifi: 'var(--lk-wifi)', cellular: 'var(--lk-cellular)', nbiot: 'var(--lk-nbiot)', satellite: 'var(--lk-satellite)', ethernet: 'var(--lk-ethernet)', mesh: 'var(--lk-mesh)' };

/** Nominal floor RSSI (dBm) per link kind, the basis for a derived signal reading. */
export const LINK_RSSI = { lora: -112, wifi: -52, cellular: -84, nbiot: -102, satellite: -118, ethernet: -40, mesh: -96 };

/**
 * Renders the four-segment signal-strength bars.
 *
 * @param {number} strength - the lit-bar count in `0..=4`.
 * @param {boolean} online - whether the link is up; an offline link shows no lit bars.
 * @returns {string} the bars markup.
 */
export function bars(strength, online)
{
  let h = '<span class="bars">';
  for (let i = 1; i <= 4; i++) h += `<i class="${online && i <= strength ? 'on' : ''}"></i>`;
  return h + '</span>';
}

/**
 * Renders a group's connection chip: link name, a derived dBm reading, and signal bars.
 *
 * @param {{kind: string, strength: number, online: boolean}} link - the group's link.
 * @returns {string} the connection-chip markup.
 */
export function conn(link)
{
  const name = LINK_NAMES[link.kind] || link.kind;
  const color = LINK_COLORS[link.kind] || 'var(--key)';
  const dbm = link.online ? (LINK_RSSI[link.kind] ?? -90) + (link.strength - 2) * 6 : null;
  const sig = dbm != null ? `<span class="conn-speed">${dbm} dBm</span>` : '';
  return `<span class="conn ${link.online ? '' : 'off'}" style="--lc:${color}"><span class="conn-kind">${name}</span>${sig}${bars(link.strength, link.online)}</span>`;
}
