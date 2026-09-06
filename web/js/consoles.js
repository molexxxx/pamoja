// consoles.js - the front page's dioramas: nine nodes doing their jobs, each drawn as a
// small console of live tiles and a log, driven by a script that plays the scenario on a
// loop. A console is mounted into a `figure[data-diorama=<key>]` whose key names a spec
// below; `cargo xtask site` fails when web/home.toml names a scenario with no spec here.
// DOM and SVG only, paused off-screen and under reduced motion.

const prefersReducedMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;

// Each console's accent, a token from theme.css so the palette lives in one place.
const ACCENT = { farm: 'var(--teal)', clinic: 'var(--amber)', water: 'var(--sky)', conservation: 'var(--forest)', village: 'var(--teal)', storm: 'var(--coral)', robot: 'var(--coral)', arm: 'var(--amber)', fleet: 'var(--sky)' };

const el = (tag, cls, html) => { const n = document.createElement(tag); if (cls) n.className = cls; if (html != null) n.innerHTML = html; return n; };
const lerp = (a, b, k) => a + (b - a) * k;
const pad = (n) => (n < 10 ? '0' + n : '' + n);
const clamp = (v, a, b) => Math.max(a, Math.min(b, v));

// ---- tile renderers: each returns { node, update(s) } ------------------

function tRadial(spec)
{
  const node = el('div', 'tile t-radial');
  const C = 2 * Math.PI * 30, sweep = 0.75; // 270deg gauge
  node.innerHTML = `
    <svg viewBox="0 0 80 72" class="gauge">
      <circle cx="40" cy="38" r="30" class="g-track" transform="rotate(135 40 38)"
        stroke-dasharray="${(C * sweep).toFixed(1)} ${C.toFixed(1)}" stroke-linecap="round"/>
      <circle cx="40" cy="38" r="30" class="g-val" transform="rotate(135 40 38)"
        stroke-dasharray="0 ${C.toFixed(1)}" stroke-linecap="round"/>
      <text x="40" y="40" class="g-num">0</text>
      <text x="40" y="55" class="g-unit">${spec.unit}</text>
    </svg>
    <div class="t-label">${spec.label}</div>`;
  const valC = node.querySelector('.g-val');
  const num = node.querySelector('.g-num');
  const lbl = node.querySelector('.t-label');
  return {
    node,
    update(s)
    {
      const v = s.v[spec.key] || 0;
      const pct = clamp(v / spec.max, 0, 1);
      valC.setAttribute('stroke-dasharray', `${(C * sweep * pct).toFixed(1)} ${C.toFixed(1)}`);
      num.textContent = spec.fmt ? spec.fmt(v) : Math.round(v);
      if (spec.status)
      {
        const [txt, tone] = spec.status(s);
        lbl.innerHTML = `${spec.label} <i class="t-tag ${tone}">${txt}</i>`;
      }
    },
  };
}

function tBar(spec)
{
  const node = el('div', 'tile t-bar');
  node.innerHTML = `
    <div class="t-row"><span class="t-label">${spec.label}</span><span class="t-val"><b>0</b>${spec.unit}</span></div>
    <div class="bar"><i></i></div>`;
  const fill = node.querySelector('.bar i');
  const val = node.querySelector('.t-val b');
  return {
    node,
    update(s)
    {
      const v = s.v[spec.key] || 0; const pct = clamp(v / spec.max, 0, 1);
      fill.style.width = (pct * 100).toFixed(1) + '%';
      val.textContent = spec.fmt ? spec.fmt(v) : Math.round(v);
    },
  };
}

function tKpi(spec)
{
  const node = el('div', 'tile t-kpi');
  node.innerHTML = `<div class="t-label">${spec.label}</div><div class="kpi"><b>0</b><span>${spec.unit}</span></div>`;
  const b = node.querySelector('.kpi b');
  const lbl = node.querySelector('.t-label');
  return {
    node,
    update(s)
    {
      const v = s.v[spec.key] || 0;
      b.textContent = spec.fmt ? spec.fmt(v) : Math.round(v);
      if (spec.status) { const [txt, tone] = spec.status(s); lbl.innerHTML = `${spec.label} <i class="t-tag ${tone}">${txt}</i>`; }
    },
  };
}

function tChip(spec)
{
  const node = el('div', 'tile t-chip');
  node.innerHTML = `<div class="t-label">${spec.label}</div><div class="chip"><span class="chip-dot"></span><b></b></div>`;
  const dot = node.querySelector('.chip-dot'); const b = node.querySelector('.chip b'); const chip = node.querySelector('.chip');
  return {
    node,
    update(s) { const [txt, tone] = spec.state(s); b.textContent = txt; chip.dataset.tone = tone; },
  };
}

function tSpark(spec)
{
  const node = el('div', 'tile t-spark');
  node.innerHTML = `<div class="t-row"><span class="t-label">${spec.label}</span><span class="t-val"><b>0</b>${spec.unit || ''}</span></div>
    <svg viewBox="0 0 200 46" preserveAspectRatio="none" class="spark"><polyline class="sp-line" points=""/><polyline class="sp-area" points=""/></svg>`;
  const line = node.querySelector('.sp-line'); const area = node.querySelector('.sp-area'); const val = node.querySelector('.t-val b');
  const buf = new Array(48).fill(spec.start ?? 50);
  let acc = 0;
  return {
    node,
    update(s, dt)
    {
      acc += dt; if (acc < 0.22) { val.textContent = spec.fmt ? spec.fmt(s.v[spec.key]) : Math.round(s.v[spec.key]); return; } acc = 0;
      buf.push(s.v[spec.key] || 0); buf.shift();
      const max = spec.max || 100;
      const pts = buf.map((v, i) => `${(i / (buf.length - 1) * 200).toFixed(1)},${(44 - clamp(v / max, 0, 1) * 40).toFixed(1)}`).join(' ');
      line.setAttribute('points', pts);
      area.setAttribute('points', `0,46 ${pts} 200,46`);
      val.textContent = spec.fmt ? spec.fmt(s.v[spec.key]) : Math.round(s.v[spec.key]);
    },
  };
}

// Live acoustic monitor: a bar equaliser that spikes on an alert event.
function tWave(spec)
{
  const node = el('div', 'tile t-wave wide');
  const N = 32;
  node.innerHTML = `<div class="t-row"><span class="t-label">${spec.label}</span><span class="t-val acoustic"><b>OK</b></span></div>
    <div class="wave">${Array.from({ length: N }, () => '<i></i>').join('')}</div>`;
  const bars = [...node.querySelectorAll('.wave i')]; const tag = node.querySelector('.acoustic');
  let alert = 0;
  return {
    node,
    update(s, dt)
    {
      if (s.flags.alert) { alert = 1; s.flags.alert = false; }
      alert = Math.max(0, alert - dt * 0.5);
      tag.classList.toggle('hot', alert > 0.1);
      tag.querySelector('b').textContent = alert > 0.1 ? 'CHAINSAW' : 'ambient';
      bars.forEach((b, i) =>
      {
        const base = 0.12 + 0.18 * Math.abs(Math.sin(s.t * 3 + i));
        const spike = alert * (0.5 + 0.5 * Math.sin(s.t * 22 + i * 1.3)) * (1 - Math.abs(i - N / 2) / N);
        b.style.transform = `scaleY(${(base + spike * 2).toFixed(3)})`;
        b.style.opacity = 0.5 + (base + spike) * 0.8;
      });
    },
  };
}

// The tamper-evident, hash-chained log: blocks prepend as records are signed.
function tChain(spec)
{
  const node = el('div', 'tile t-chain wide');
  node.innerHTML = `<div class="t-row"><span class="t-label">${spec.label}</span><span class="t-val mono chain-c">0</span></div><div class="chain"></div>`;
  const row = node.querySelector('.chain'); const count = node.querySelector('.chain-c');
  const hex = () => Math.floor(Math.random() * 0xffff).toString(16).padStart(4, '0');
  let n = 1041;
  const add = () =>
  {
    const blk = el('span', 'block', `#${hex()}`); row.prepend(blk);
    requestAnimationFrame(() => blk.classList.add('in'));
    while (row.children.length > 5) row.lastChild.remove();
    count.textContent = (++n).toLocaleString();
  };
  for (let i = 0; i < 5; i++) add();
  return {
    node,
    update(s) { if (s.flags.sign) { s.flags.sign = false; add(); } },
  };
}

// A compact mesh / signal-flow schematic with travelling packets (SMIL).
function tMesh(spec)
{
  const node = el('div', 'tile t-mesh wide');
  const { nodes, links, packets, sat } = spec.cfg;
  const linkSvg = links.map((l) => `<line x1="${nodes[l[0]][0]}" y1="${nodes[l[0]][1]}" x2="${nodes[l[1]][0]}" y2="${nodes[l[1]][1]}" class="ml${l[2] ? ' weak' : ''}"/>`).join('');
  const pktSvg = packets.map((p, i) =>
  {
    const path = p.map((k, j) => `${j ? 'L' : 'M'}${nodes[k][0]},${nodes[k][1]}`).join(' ');
    return `<circle r="3.2" class="pk"><animateMotion dur="${(1.6 + i * 0.2).toFixed(2)}s" repeatCount="indefinite" begin="${(i * 0.5).toFixed(2)}s" path="${path}"/><animate attributeName="opacity" values="0;1;1;0" dur="${(1.6 + i * 0.2).toFixed(2)}s" repeatCount="indefinite" begin="${(i * 0.5).toFixed(2)}s"/></circle>`;
  }).join('');
  const satSvg = sat ? `<line x1="${nodes[sat.from][0]}" y1="${nodes[sat.from][1]}" x2="${sat.xy[0]}" y2="${sat.xy[1]}" class="ml up"/>
    <circle r="3" class="pk up"><animateMotion dur="1.8s" repeatCount="indefinite" path="M${nodes[sat.from][0]},${nodes[sat.from][1]} L${sat.xy[0]},${sat.xy[1]}"/></circle>
    <g transform="translate(${sat.xy[0] - 7},${sat.xy[1] - 5})" class="sat"><rect width="14" height="9" rx="1.5"/><rect x="-6" y="2.5" width="5" height="4"/><rect x="15" y="2.5" width="5" height="4"/></g>` : '';
  const nodeSvg = nodes.map((n2, i) => `<circle cx="${n2[0]}" cy="${n2[1]}" r="${n2[2] === 'gw' ? 6 : 4.5}" class="mn${n2[2] === 'gw' ? ' gw' : ''}"/>`).join('');
  node.innerHTML = `<div class="t-row"><span class="t-label">${spec.label}</span>${spec.note ? `<span class="t-val mono mesh-note"></span>` : ''}</div>
    <svg viewBox="0 0 220 120" class="mesh">${linkSvg}${satSvg}${pktSvg}${nodeSvg}</svg>`;
  const note = node.querySelector('.mesh-note');
  return {
    node,
    update(s) { if (note && spec.note) note.textContent = spec.note(s); },
  };
}

// Top-down rover: follows a waypoint path with carrot lookahead, integrates
// odometry from its own motion, and holds when an obstacle is in the way. It
// is the simulation - it writes v, omega, heading and odometry back into the
// console state so the read-out tiles beside it show the rover's real values.
function tRover(spec)
{
  const node = el('div', 'tile t-rover wide');
  const W = 220, H = 120;
  const WP = spec.path;
  const OBS = spec.obstacle;
  const PX2M = spec.scale || 6 / W;
  const origin = WP[0];

  const seg = [];
  let total = 0;
  for (let i = 0; i < WP.length - 1; i++)
  {
    const len = Math.hypot(WP[i + 1][0] - WP[i][0], WP[i + 1][1] - WP[i][1]);
    seg.push({ at: total, len, a: WP[i], b: WP[i + 1] });
    total += len;
  }
  const pointAt = (d) =>
  {
    d = clamp(d, 0, total);
    let sg = seg[seg.length - 1];
    for (const s2 of seg) { if (d <= s2.at + s2.len) { sg = s2; break; } }
    const t = sg.len ? (d - sg.at) / sg.len : 0;
    return { x: lerp(sg.a[0], sg.b[0], t), y: lerp(sg.a[1], sg.b[1], t), h: Math.atan2(sg.b[1] - sg.a[1], sg.b[0] - sg.a[0]) };
  };

  let grid = '';
  for (let x = 22; x < W; x += 22) grid += `<line x1="${x}" y1="0" x2="${x}" y2="${H}"/>`;
  for (let y = 24; y < H; y += 24) grid += `<line x1="0" y1="${y}" x2="${W}" y2="${y}"/>`;
  const plan = WP.map((p, i) => `${i ? 'L' : 'M'}${p[0]},${p[1]}`).join(' ');
  const wpDots = WP.map((p, i) => `<circle cx="${p[0]}" cy="${p[1]}" r="${i === WP.length - 1 ? 4 : 2.6}" class="rv-wp${i === WP.length - 1 ? ' goal' : ''}"/>`).join('');
  const obs = OBS ? `<circle cx="${OBS.x}" cy="${OBS.y}" r="${OBS.r}" class="rv-obs"/><circle cx="${OBS.x}" cy="${OBS.y}" r="2.4" class="rv-obs-c"/>` : '';
  node.innerHTML = `
    <div class="t-row"><span class="t-label">${spec.label}</span><span class="t-val mono rv-pose"></span></div>
    <svg viewBox="0 0 ${W} ${H}" class="rover" preserveAspectRatio="xMidYMid meet">
      <g class="rv-grid">${grid}</g>
      <path d="${plan}" class="rv-plan"/>
      ${obs}
      <polyline class="rv-trail" points=""/>
      ${wpDots}
      <circle r="3.4" class="rv-carrot"/>
      <g class="rv-bot"><path d="M7,0 L-5,-4.5 L-2,0 L-5,4.5 Z"/></g>
    </svg>`;
  const pose = node.querySelector('.rv-pose');
  const trailEl = node.querySelector('.rv-trail');
  const carrot = node.querySelector('.rv-carrot');
  const bot = node.querySelector('.rv-bot');

  let dist = 0, dir = 1, hold = 0, prevH = pointAt(0).h, lastWp = 0, prevNear = false, obsHold = 0;
  const trail = [];
  const base = spec.speed || 20;

  return {
    node,
    update(s, dt)
    {
      const here = pointAt(dist);
      const near = OBS ? Math.hypot(here.x - OBS.x, here.y - OBS.y) < OBS.r + 9 : false;
      if (near && !prevNear) obsHold = 1.1;
      prevNear = near;
      if (obsHold > 0) obsHold = Math.max(0, obsHold - dt);
      let v = near ? (obsHold > 0 ? 0 : base * 0.3) : base;
      if (hold > 0) { hold = Math.max(0, hold - dt); v = 0; }
      dist += dir * v * dt;
      if (dir > 0 && dist >= total) { dist = total; dir = -1; hold = 1.0; }
      else if (dir < 0 && dist <= 0) { dist = 0; dir = 1; hold = 1.0; }

      const q = pointAt(dist);
      const heading = q.h + (dir < 0 ? Math.PI : 0);
      let dh = heading - prevH; while (dh > Math.PI) dh -= 2 * Math.PI; while (dh < -Math.PI) dh += 2 * Math.PI;
      prevH = heading;

      s.v.vx = v * PX2M;
      s.v.omega = dt > 0 ? lerp(s.v.omega || 0, dh / dt, 0.3) : (s.v.omega || 0);
      s.v.odo = (s.v.odo || 0) + Math.abs(v * dt) * PX2M;
      s.flags.near = near;

      let wp = 0;
      for (let i = 0; i < seg.length; i++) if (dist >= seg[i].at + seg[i].len * 0.5) wp = i + 1;
      if (dir > 0 && wp !== lastWp) s.flags.wp = wp;
      lastWp = wp;

      const degRaw = heading * 180 / Math.PI;
      const deg = ((degRaw % 360) + 360) % 360;
      pose.textContent = `${((q.x - origin[0]) * PX2M).toFixed(1)}, ${((origin[1] - q.y) * PX2M).toFixed(1)} m · ${Math.round(deg)}°`;
      bot.setAttribute('transform', `translate(${q.x.toFixed(2)},${q.y.toFixed(2)}) rotate(${degRaw.toFixed(1)})`);
      bot.classList.toggle('slow', near);
      const c = pointAt(dist + dir * 22);
      carrot.setAttribute('cx', c.x.toFixed(2));
      carrot.setAttribute('cy', c.y.toFixed(2));
      trail.push(`${q.x.toFixed(1)},${q.y.toFixed(1)}`);
      if (trail.length > 24) trail.shift();
      trailEl.setAttribute('points', trail.join(' '));
    },
  };
}

// A live ROS 2 over Zenoh line: the real rmw_zenoh key for the message, plus a
// status line and a meter on the right (a publish sequence number by default,
// or whatever the spec wants - action feedback, peer count). The default Twist
// byte count is the true wire size (4-byte CDR header + 6 x float64).
function tBridge(spec)
{
  const node = el('div', 'tile t-bridge wide');
  node.innerHTML = `
    <div class="t-row"><span class="t-label">${spec.label}</span><span class="t-val mono br-seq"></span></div>
    <div class="br-key">${spec.key}</div>
    <div class="br-status">${spec.status}</div>`;
  const meter = node.querySelector('.br-seq');
  return {
    node,
    update(s) { meter.textContent = spec.meter ? spec.meter(s) : `seq ${(1400 + Math.floor(s.t * 10)).toLocaleString()}`; },
  };
}

// Side-view two-link arm: it solves its own inverse kinematics to land the tip
// on a moving pick/place target (elbow up or down), while forward kinematics
// draw the linkage. Joint angles and reach are written back for the read-outs.
function tArm(spec)
{
  const node = el('div', 'tile t-arm wide');
  const W = 220, H = 120;
  const B = spec.base || [70, 104];
  const L1 = spec.l1 || 48, L2 = spec.l2 || 40, R = L1 + L2;
  const WP = spec.targets;
  const elbowSign = spec.elbow === 'down' ? -1 : 1;

  const seg = [];
  let total = 0;
  for (let i = 0; i < WP.length - 1; i++)
  {
    const len = Math.hypot(WP[i + 1][0] - WP[i][0], WP[i + 1][1] - WP[i][1]);
    seg.push({ at: total, len, a: WP[i], b: WP[i + 1] });
    total += len;
  }
  const goalAt = (d) =>
  {
    d = total ? ((d % total) + total) % total : 0;
    let sg = seg[seg.length - 1];
    for (const s2 of seg) { if (d <= s2.at + s2.len) { sg = s2; break; } }
    const t = sg.len ? (d - sg.at) / sg.len : 0;
    return [lerp(sg.a[0], sg.b[0], t), lerp(sg.a[1], sg.b[1], t)];
  };

  let arc = '';
  for (let a = 198; a <= 342; a += 6) { arc += `${a === 198 ? 'M' : 'L'}${(B[0] + R * Math.cos(a * Math.PI / 180)).toFixed(1)},${(B[1] + R * Math.sin(a * Math.PI / 180)).toFixed(1)}`; }
  const tgts = WP.slice(0, -1).map((t) => `<circle cx="${t[0]}" cy="${t[1]}" r="2.6" class="arm-tgt"/>`).join('');
  node.innerHTML = `
    <div class="t-row"><span class="t-label">${spec.label}</span><span class="t-val mono arm-read"></span></div>
    <svg viewBox="0 0 ${W} ${H}" class="arm" preserveAspectRatio="xMidYMid meet">
      <path class="arm-reach" d="${arc}"/>
      ${tgts}
      <line class="arm-link arm-l1"/>
      <line class="arm-link arm-l2"/>
      <circle class="arm-goal" r="5.5"/>
      <circle class="arm-base" cx="${B[0]}" cy="${B[1]}" r="4.5"/>
      <circle class="arm-joint arm-elbow" r="3"/>
      <circle class="arm-tip" r="3.6"/>
    </svg>`;
  const read = node.querySelector('.arm-read');
  const l1 = node.querySelector('.arm-l1'), l2 = node.querySelector('.arm-l2');
  const elbowEl = node.querySelector('.arm-elbow'), tipEl = node.querySelector('.arm-tip'), goalEl = node.querySelector('.arm-goal');
  let d = 0;

  return {
    node,
    update(s, dt)
    {
      d += (spec.speed || 26) * dt;
      const goal = goalAt(d);
      let grip = false;
      for (const t of WP.slice(0, -1)) if (Math.hypot(goal[0] - t[0], goal[1] - t[1]) < 6) grip = true;

      const gx = goal[0] - B[0], gy = B[1] - goal[1];
      const reachable = Math.hypot(gx, gy) <= R - 0.5;
      const r = clamp(Math.hypot(gx, gy), Math.abs(L1 - L2) + 0.5, R - 0.5);
      const c2 = clamp((r * r - L1 * L1 - L2 * L2) / (2 * L1 * L2), -1, 1);
      const th2 = elbowSign * Math.acos(c2);
      const th1 = Math.atan2(gy, gx) - Math.atan2(L2 * Math.sin(th2), L1 + L2 * Math.cos(th2));
      const ex = B[0] + L1 * Math.cos(th1), ey = B[1] - L1 * Math.sin(th1);
      const tx = ex + L2 * Math.cos(th1 + th2), ty = ey - L2 * Math.sin(th1 + th2);

      l1.setAttribute('x1', B[0]); l1.setAttribute('y1', B[1]); l1.setAttribute('x2', ex.toFixed(2)); l1.setAttribute('y2', ey.toFixed(2));
      l2.setAttribute('x1', ex.toFixed(2)); l2.setAttribute('y1', ey.toFixed(2)); l2.setAttribute('x2', tx.toFixed(2)); l2.setAttribute('y2', ty.toFixed(2));
      elbowEl.setAttribute('cx', ex.toFixed(2)); elbowEl.setAttribute('cy', ey.toFixed(2));
      tipEl.setAttribute('cx', tx.toFixed(2)); tipEl.setAttribute('cy', ty.toFixed(2));
      goalEl.setAttribute('cx', goal[0].toFixed(2)); goalEl.setAttribute('cy', goal[1].toFixed(2));
      tipEl.classList.toggle('grip', grip);
      goalEl.classList.toggle('reached', reachable);

      const th1deg = th1 * 180 / Math.PI, th2deg = th2 * 180 / Math.PI;
      s.v.th1 = th1deg; s.v.th2 = th2deg; s.v.reach = (r / R) * 100;
      s.flags.grip = grip;
      read.textContent = `θ1 ${Math.round(th1deg)}° · θ2 ${Math.round(th2deg)}° · elbow ${elbowSign > 0 ? 'up' : 'down'}`;
    },
  };
}

// Top-down fleet: a few robots each patrol their own loop around a routerless
// hub, all reachable through one Zenoh key expression. Peer and rate counters
// are written back for the read-outs.
function tFleet(spec)
{
  const node = el('div', 'tile t-fleet wide');
  const W = 220, H = 120;
  const hub = spec.hub || [110, 60];
  const bots = spec.bots;
  const loops = bots.map((b) => `<circle cx="${b.cx}" cy="${b.cy}" r="${b.r}" class="fl-loop"/>`).join('');
  const links = bots.map((b) => `<line x1="${hub[0]}" y1="${hub[1]}" x2="${b.cx}" y2="${b.cy}" class="fl-link"/>`).join('');
  node.innerHTML = `
    <div class="t-row"><span class="t-label">${spec.label}</span><span class="t-val mono fl-note">${bots.length} peers</span></div>
    <svg viewBox="0 0 ${W} ${H}" class="fleet" preserveAspectRatio="xMidYMid meet">
      ${loops}${links}
      <g transform="translate(${hub[0]},${hub[1]})"><rect x="-6" y="-5" width="12" height="10" rx="2" class="fl-hub"/></g>
      ${bots.map(() => '<g class="fl-bot"><path d="M6,0 L-4,-3.5 L-1.5,0 L-4,3.5 Z"/></g>').join('')}
    </svg>`;
  const botEls = [...node.querySelectorAll('.fl-bot')];
  return {
    node,
    update(s)
    {
      botEls.forEach((g, i) =>
      {
        const b = bots[i];
        const a = s.t * b.speed * b.dir + b.phase;
        const x = b.cx + b.r * Math.cos(a), y = b.cy + b.r * Math.sin(a);
        const hd = Math.atan2(b.dir * Math.cos(a), -b.dir * Math.sin(a)) * 180 / Math.PI;
        g.setAttribute('transform', `translate(${x.toFixed(2)},${y.toFixed(2)}) rotate(${hd.toFixed(1)})`);
      });
      s.v.peers = bots.length;
      s.v.rate = Math.round(28 + 4 * Math.sin(s.t * 0.8));
      s.v.twins = spec.twins ?? 1;
    },
  };
}

const RENDER = { radial: tRadial, bar: tBar, kpi: tKpi, chip: tChip, spark: tSpark, wave: tWave, chain: tChain, mesh: tMesh, rover: tRover, bridge: tBridge, arm: tArm, fleet: tFleet };

// ---- scenario specs ----------------------------------------------------
function step(s, cycle, steps)
{
  const p = s.t % cycle; let idx = 0;
  for (let i = 0; i < steps.length; i++) if (p >= steps[i].at) idx = i;
  const fire = Math.floor(s.t / cycle) * 100 + idx;
  if (s._fire !== fire)
  {
    s._fire = fire; const st = steps[idx];
    if (st.set) for (const k in st.set) s.target[k] = st.set[k];
    if (st.ev) s.flags[st.ev] = true;
    if (st.msg) s.say(st.msg);
  }
}

const SPECS = {
  farm: {
    id: 'farm node · meru-03', link: 'LoRa', dbm: -87,
    init: { soil: 52, well: 64, batt: 4.1, flow: 0 },
    tiles: [
      { type: 'radial', label: 'soil moisture', key: 'soil', unit: '%', max: 100, status: (s) => (s.v.soil < 36 ? ['dry', 'warn'] : ['ok', 'ok']) },
      { type: 'bar', label: 'well level', key: 'well', unit: '%', max: 100 },
      { type: 'chip', label: 'drip valve', state: (s) => (s.v.soil < 36 ? ['OPEN', 'ok'] : ['CLOSED', 'idle']) },
      { type: 'kpi', label: 'battery', key: 'batt', unit: 'V', fmt: (v) => v.toFixed(2) },
      { type: 'spark', label: 'soil trend · 24h', key: 'soil', max: 100, start: 50 },
    ],
    script(s)
    {
      s.v.batt = 3.9 + 0.25 * (0.5 + 0.5 * Math.sin(s.t * 0.2));
      step(s, 13, [
        { at: 0, set: { soil: 33, well: 60 }, msg: 'soil 33% — below threshold' },
        { at: 2.4, set: { soil: 33 }, msg: 'valve → OPEN' },
        { at: 3, set: { soil: 58, well: 55 }, msg: 'irrigating drip line' },
        { at: 7, set: { soil: 54 }, msg: 'soil 54% — target met' },
        { at: 8, msg: 'valve → CLOSED' },
        { at: 9, set: { soil: 44 }, msg: 'published 84 B ↑ LoRa' },
      ]);
    },
  },
  clinic: {
    id: 'health post · kano-01', link: 'NB-IoT', dbm: -102,
    init: { temp: 4.2, power: 88, o2: 71, queued: 0 },
    tiles: [
      { type: 'kpi', label: 'fridge temp', key: 'temp', unit: '°C', fmt: (v) => v.toFixed(1), status: (s) => (s.v.temp > 7.6 || s.v.temp < 2.2 ? ['excursion', 'alert'] : ['in range', 'ok']) },
      { type: 'bar', label: 'ward power', key: 'power', unit: '%', max: 100 },
      { type: 'bar', label: 'oxygen stock', key: 'o2', unit: '%', max: 100 },
      { type: 'chip', label: 'uplink', state: (s) => (s.flags.offline ? [`buffering · ${Math.round(s.v.queued)}`, 'warn'] : ['synced', 'ok']) },
      { type: 'chain', label: 'tamper-evident log' },
    ],
    script(s)
    {
      s.v.temp = 4.2 + 1.6 * Math.sin(s.t * 0.5) + 0.3 * Math.sin(s.t * 2.1);
      step(s, 16, [
        { at: 0, set: { o2: 74, power: 90, queued: 0 }, ev: 'sign', msg: 'temp logged · signed' },
        { at: 3, ev: 'sign', msg: '#hash chained to prior' },
        { at: 5, set: { queued: 6 }, msg: 'link lost — store & forward' },
        { at: 8, set: { queued: 11 }, msg: '11 records buffered' },
        { at: 11, set: { queued: 0 }, msg: 'link up — draining queue' },
        { at: 13, ev: 'sign', msg: 'synced ✓ nothing lost' },
      ]);
      s.flags.offline = (s.t % 16) >= 5 && (s.t % 16) < 11.5;
    },
  },
  water: {
    id: 'water point · jaipur-07', link: 'LoRa', dbm: -91,
    init: { flow: 0, well: 58, tank: 40, draws: 1204 },
    tiles: [
      { type: 'kpi', label: 'flow rate', key: 'flow', unit: 'L/min', fmt: (v) => v.toFixed(0) },
      { type: 'bar', label: 'well level', key: 'well', unit: '%', max: 100 },
      { type: 'bar', label: 'storage tank', key: 'tank', unit: '%', max: 100 },
      { type: 'chip', label: 'pump health', state: (s) => (s.flags.weak ? ['weakening', 'warn'] : ['nominal', 'ok']) },
      { type: 'spark', label: 'flow · today', key: 'flow', max: 16, start: 0 },
    ],
    script(s)
    {
      s.v.tank = clamp(s.v.tank + (s.flags.draw ? 0.4 : -0.05), 20, 92);
      step(s, 11, [
        { at: 0, set: { flow: 0, well: 60 }, msg: 'idle' },
        { at: 2, set: { flow: 12, well: 52 }, ev: 'draw', msg: 'draw detected' },
        { at: 4, set: { flow: 11, well: 49 }, msg: 'flow 11 L/min' },
        { at: 6, set: { flow: 0, well: 56 }, msg: `logged · ${(1204).toLocaleString()} draws today` },
        { at: 8, set: { flow: 9 }, ev: 'draw', msg: 'pump pressure −6% vs baseline' },
      ]);
      s.flags.draw = s.v.flow > 4;
      s.flags.weak = (s.t % 22) > 16;
    },
  },
  conservation: {
    id: 'ranger relay · luangwa-02', link: 'mesh', dbm: -95,
    init: { river: 38, batt: 92, audio: 20 },
    tiles: [
      { type: 'wave', label: 'acoustic monitor' },
      { type: 'bar', label: 'river level', key: 'river', unit: '%', max: 100 },
      { type: 'kpi', label: 'battery', key: 'batt', unit: '%', fmt: (v) => Math.round(v) },
      { type: 'mesh', label: 'relay → ranger post', note: (s) => (s.flags.relaying ? 'relaying' : 'idle'), cfg: { nodes: [[24, 80], [90, 58, 0], [150, 70, 0], [200, 56, 'gw']], links: [[0, 1], [1, 2], [2, 3]], packets: [[0, 1, 2, 3]] } },
    ],
    script(s)
    {
      s.v.batt = 90 + 6 * Math.sin(s.t * 0.1);
      s.v.river = 36 + 4 * Math.sin(s.t * 0.3);
      step(s, 9, [
        { at: 0, msg: 'ambient 41 dB — quiet' },
        { at: 4, ev: 'alert', msg: 'ALERT chainsaw · 2.1 km NE' },
        { at: 5, msg: 'relaying → ranger post' },
        { at: 7, msg: 'delivered · 3 hops · 0.4 s' },
      ]);
      s.flags.relaying = (s.t % 9) >= 5 && (s.t % 9) < 7.5;
    },
  },
  village: {
    id: 'mesh node · pokhara-04', link: 'mesh', dbm: -83,
    init: { neighbors: 5, hops: 3, relayed: 318 },
    tiles: [
      { type: 'mesh', label: 'neighbour mesh', note: (s) => (s.flags.reroute ? 'rerouting' : `cost ${s.v.hops}`), cfg: { nodes: [[20, 60], [80, 30], [80, 95], [150, 55], [200, 80, 'gw']], links: [[0, 1], [0, 2], [1, 3], [2, 3], [3, 4], [1, 4, 1]], packets: [[0, 1, 3, 4], [0, 2, 3, 4]] } },
      { type: 'kpi', label: 'neighbours', key: 'neighbors', unit: '' },
      { type: 'kpi', label: 'hops to gateway', key: 'hops', unit: '' },
      { type: 'chip', label: 'routing', state: (s) => (s.flags.reroute ? ['learning', 'warn'] : ['optimised', 'ok']) },
      { type: 'spark', label: 'messages relayed', key: 'relayed', max: 360, start: 300 },
    ],
    script(s)
    {
      s.v.relayed += s.dt * 1.4;
      step(s, 12, [
        { at: 0, set: { hops: 3, neighbors: 5 }, msg: 'heard A→B (cost 3)' },
        { at: 3, msg: 'flooding suppressed · seen once' },
        { at: 6, set: { hops: 4 }, msg: 'link B→GW lost' },
        { at: 7, set: { hops: 2, neighbors: 5 }, msg: 'learned route via C (cost 2)' },
        { at: 9, msg: 'route optimised · airtime saved' },
      ]);
      s.flags.reroute = (s.t % 12) >= 6 && (s.t % 12) < 9;
    },
  },
  storm: {
    id: 'coast relay · tacloban-09', link: 'sat', dbm: -118,
    init: { carried: 12, queue: 9, batt: 73 },
    tiles: [
      { type: 'chip', label: 'cell grid', state: () => ['DOWN', 'alert'] },
      { type: 'chip', label: 'pamoja mesh', state: () => ['UP', 'ok'] },
      { type: 'mesh', label: 'relay → shared gateway · sat', note: (s) => `${Math.round(s.v.queue)} queued`, cfg: { nodes: [[20, 92], [70, 80, 0], [120, 88, 0], [170, 74, 'gw']], links: [[0, 1], [1, 2], [2, 3]], packets: [[0, 1, 2, 3]], sat: { from: 3, xy: [205, 22] } } },
      { type: 'kpi', label: 'reports carried', key: 'carried', unit: '' },
      { type: 'bar', label: 'uplink queue', key: 'queue', unit: '', max: 16 },
    ],
    script(s)
    {
      s.v.batt = 70 + 5 * Math.sin(s.t * 0.15);
      step(s, 10, [
        { at: 0, set: { queue: 11 }, msg: 'cell tower lost — landfall' },
        { at: 2, msg: 'relaying via coastal mesh' },
        { at: 4, set: { queue: 4, carried: 31 }, msg: 'uplink ↑ one shared gateway (sat)' },
        { at: 7, set: { queue: 8, carried: 38 }, msg: 'carried 38 location reports' },
      ]);
    },
  },
  robot: {
    id: 'rover · kit robotics', link: 'Zenoh', dbm: -54,
    init: {},
    tiles: [
      { type: 'rover', label: 'waypoint patrol · top-down', path: [[26, 94], [58, 44], [104, 30], [150, 66], [192, 34]], obstacle: { x: 129, y: 49, r: 8.5 }, speed: 21 },
      { type: 'kpi', label: 'linear · v', key: 'vx', unit: 'm/s', fmt: (v) => v.toFixed(2) },
      { type: 'kpi', label: 'angular · ω', key: 'omega', unit: 'rad/s', fmt: (v) => v.toFixed(2) },
      { type: 'kpi', label: 'odometry', key: 'odo', unit: 'm', fmt: (v) => v.toFixed(1) },
      { type: 'chip', label: 'safety', state: (s) => (s.flags.near ? ['obstacle · hold', 'warn'] : ['armed', 'ok']) },
      {
        type: 'bridge', label: 'ROS 2 ⇄ Zenoh',
        key: '0/cmd_vel/<b>geometry_msgs::msg::dds_::Twist_</b>/RIHS01_&hellip;',
        status: 'Twist &rarr; CDR <b>52 B</b> &rarr; decoded <span class="ok">&check;</span> &middot; rmw_zenoh, routerless',
      },
    ],
    script(s)
    {
      if (s.flags.wp && s.flags.wp !== s._wp) { s._wp = s.flags.wp; s.say(`waypoint ${s.flags.wp}/5 reached`); }
      if (s.flags.near && !s._near) { s._near = true; s.say('obstacle 0.6 m ahead — holding'); }
      if (!s.flags.near && s._near) { s._near = false; s.say('clear — resuming path'); }
      step(s, 12, [
        { at: 0, msg: 'route loaded · 5 waypoints' },
        { at: 3, msg: 'cmd_vel → 0/cmd_vel/…Twist_' },
        { at: 6, msg: 'watchdog ok · link 41 ms' },
        { at: 9, msg: 'odometry fused · no GPS' },
      ]);
    },
  },
  arm: {
    id: 'arm · kit robotics', link: 'ROS 2', dbm: -49,
    init: {},
    tiles: [
      { type: 'arm', label: 'two-link arm · FK + IK', base: [70, 104], l1: 48, l2: 40, elbow: 'up', speed: 27, targets: [[140, 92], [140, 54], [30, 52], [95, 74], [140, 92]] },
      { type: 'kpi', label: 'joint θ1', key: 'th1', unit: '°', fmt: (v) => Math.round(v) },
      { type: 'kpi', label: 'joint θ2', key: 'th2', unit: '°', fmt: (v) => Math.round(v) },
      { type: 'kpi', label: 'reach', key: 'reach', unit: '%', fmt: (v) => Math.round(v) },
      { type: 'chip', label: 'gripper', state: (s) => (s.flags.grip ? ['closed', 'warn'] : ['open', 'ok']) },
      {
        type: 'bridge', label: 'ROS 2 action',
        key: '0/<b>follow_joint_trajectory</b>/_action/feedback',
        status: 'goal accepted &middot; analytic 2-link IK &middot; rmw_zenoh, routerless',
        meter: (s) => `fb ${Math.floor((s.t * 14) % 100)}%`,
      },
    ],
    script(s)
    {
      if (s.flags.grip && !s._grip) { s._grip = true; s.say('gripper closed · part acquired'); }
      if (!s.flags.grip && s._grip) { s._grip = false; s.say('gripper open · part placed'); }
      step(s, 12, [
        { at: 0, msg: 'trajectory goal · 4 points' },
        { at: 4, msg: 'IK solved · elbow up' },
        { at: 8, msg: 'within joint limits' },
      ]);
    },
  },
  fleet: {
    id: 'fleet · 3 robots', link: 'Zenoh', dbm: -57,
    init: {},
    tiles: [
      { type: 'fleet', label: 'fleet · top-down', twins: 1, bots: [{ cx: 54, cy: 46, r: 16, phase: 0, speed: 0.9, dir: 1 }, { cx: 152, cy: 42, r: 14, phase: 2, speed: 1.1, dir: -1 }, { cx: 118, cy: 90, r: 18, phase: 4, speed: 0.8, dir: 1 }] },
      { type: 'kpi', label: 'peers', key: 'peers', unit: '', fmt: (v) => Math.round(v) },
      { type: 'kpi', label: 'cmd rate', key: 'rate', unit: '/s', fmt: (v) => Math.round(v) },
      { type: 'kpi', label: 'sim twins', key: 'twins', unit: '/ 3', fmt: (v) => Math.round(v) },
      { type: 'chip', label: 'coordination', state: () => ['in sync', 'ok'] },
      {
        type: 'bridge', label: 'Zenoh key expression',
        key: '<b>*</b>/cmd_vel/geometry_msgs::msg::dds_::Twist_/<b>**</b>',
        status: 'one subscriber &middot; 3 peers matched &middot; service set_mode <span class="ok">&check;</span>',
        meter: (s) => `${Math.round(s.v.peers || 3)} peers`,
      },
    ],
    script(s)
    {
      step(s, 12, [
        { at: 0, msg: 'discovered 3 peers · routerless' },
        { at: 3, msg: 'sub **/cmd_vel · keyexpr match' },
        { at: 6, msg: 'service set_mode(AUTO) → 3 ✓' },
        { at: 9, msg: 'twin + real robots · sim ok' },
      ]);
    },
  },
};

// ---- console + deck ----------------------------------------------------
class Console
{
  constructor(fig, name)
  {
    this.fig = fig; this.active = false;
    const spec = SPECS[name]; this.spec = spec;
    const accent = ACCENT[name];
    fig.style.setProperty('--accent', accent);
    fig.classList.add('console');

    this.s = { v: { ...spec.init }, target: { ...spec.init }, t: 0, dt: 0, flags: {}, min: 12 * 60 + 1, lines: [], say: null };
    this.s.say = (msg) =>
    {
      this.s.min += 1 + Math.floor(Math.random() * 2);
      this.s.lines.push({ t: `${pad((this.s.min / 60 | 0) % 24)}:${pad(this.s.min % 60)}`, msg });
      while (this.s.lines.length > 4) this.s.lines.shift();
      this.renderLog();
    };

    const root = el('div', 'cons');
    root.innerHTML = `
      <div class="cons-head">
        <span class="cons-dot"></span>
        <span class="cons-id">${spec.id}</span>
        <span class="cons-badges">
          <span class="cons-link" title="link">${spec.link} <b>${spec.dbm}</b>dBm</span>
          <span class="cons-sig"><i></i><i></i><i></i><i></i></span>
        </span>
      </div>
      <div class="cons-body"></div>
      <div class="cons-log"></div>`;
    const body = root.querySelector('.cons-body');
    this.logEl = root.querySelector('.cons-log');
    this.tiles = spec.tiles.map((ts) => { const t = RENDER[ts.type](ts); body.appendChild(t.node); return t; });
    fig.replaceChildren(root);

    // Seed a couple of log lines and an initial render.
    spec.script(this.s);
    this.tiles.forEach((t) => t.update(this.s, 0));
    this.renderLog();
    if (prefersReducedMotion) { const svgs = fig.querySelectorAll('svg'); svgs.forEach((sv) => sv.pauseAnimations && sv.pauseAnimations()); }
  }

  renderLog()
  {
    this.logEl.innerHTML = this.s.lines.map((l, i) => `<div class="log-line${i === this.s.lines.length - 1 ? ' new' : ''}"><span>${l.t}</span>${l.msg}</div>`).join('');
  }

  update(t, dt)
  {
    const s = this.s; s.t = t; s.dt = dt;
    this.spec.script(s);
    const k = 1 - Math.exp(-dt * 3.2);
    for (const key in s.target) s.v[key] = lerp(s.v[key], s.target[key], k);
    for (const tile of this.tiles) tile.update(s, dt);
  }
}

let stop = null;

/**
 * Mounts a console into every `figure[data-diorama]` on the page and runs them while they
 * are in view. Calling it again, after the page has been swapped, stops the previous run.
 *
 * @returns {void}
 */
export function mountConsoles()
{
  if (stop) { stop(); stop = null; }
  const items = [];
  document.querySelectorAll('[data-diorama]').forEach((fig) =>
  {
    const name = fig.dataset.diorama;
    if (SPECS[name]) items.push(new Console(fig, name));
  });
  if (prefersReducedMotion || !items.length) return;

  const io = new IntersectionObserver(
    (es) => es.forEach((e) => { const it = items.find((i) => i.fig === e.target); if (it) it.active = e.isIntersecting; }),
    { rootMargin: '10% 0px', threshold: 0.01 },
  );
  items.forEach((i) => io.observe(i.fig));

  let last = performance.now();
  let frame = 0;
  const loop = (now) =>
  {
    const dt = Math.min(0.05, (now - last) / 1000); last = now;
    const t = now / 1000;
    for (const it of items) if (it.active) it.update(t, dt);
    frame = requestAnimationFrame(loop);
  };
  frame = requestAnimationFrame(loop);
  stop = () => { cancelAnimationFrame(frame); io.disconnect(); };
}
