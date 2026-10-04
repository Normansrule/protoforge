// Animated idea → factory pipeline. Every node shows the live verification
// status of the requirements its labs' metrics verify; particles travel the
// connectors (SVG <animateMotion>, disabled for prefers-reduced-motion).
import { useMemo, useState, type KeyboardEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import type { Evaluation, Project } from '../../engine/types';
import { useElementWidth } from '../../components/useElementWidth';
import { plural } from '../../components/reqStatus';
import { GROUPS, LABS, groupPrefixes, health, healthIcon, systemPrefixes, tallyFor, type Health, type Tally } from './status';
import { reducedMotion } from './motion';

const GLYPH: Record<string, string> = {
  idea: 'M9 18h6M10 21h4M12 3a6 6 0 00-3.5 10.9c.6.5 1 1.2 1 2V17h5v-1.1c0-.8.4-1.5 1-2A6 6 0 0012 3z',
  needs: 'M16 20v-1.5a3.5 3.5 0 00-3.5-3.5h-5A3.5 3.5 0 004 18.5V20M10 11a3.5 3.5 0 100-7 3.5 3.5 0 000 7zM20 20v-1.5a3.5 3.5 0 00-2.5-3.3M15.5 4.2a3.5 3.5 0 010 6.6',
  reqs: 'M10 6h10M10 12h10M10 18h10M3.5 6l1.5 1.5L7.5 5M3.5 12l1.5 1.5L7.5 11M3.5 18l1.5 1.5L7.5 17',
  arch: 'M4 4h6v6H4zM14 14h6v6h-6zM14 4h6v6h-6zM7 10v4h7M17 10v4',
  verify: 'M12 3l8 3v6c0 4.5-3.4 8.3-8 9-4.6-.7-8-4.5-8-9V6zM8.5 12l2.5 2.5 4.5-5',
  factory: 'M21 8l-9-5-9 5v8l9 5 9-5zM3 8l9 5 9-5M12 13v8',
};

export const SHORT: Record<string, string> = {
  circuit: 'Circuit', power: 'Power', pcb: 'PCB', dsp: 'DSP', chip: 'Chip', control: 'Control', robotics: 'Robotics',
  rl: 'RL', optics: 'Optics', cad: 'CAD', print: 'Print', molding: 'Molding', mfg: 'Mfg',
};

interface Box {
  id: string;
  x: number; // left
  y: number; // top
  w: number;
  h: number;
}
const cx = (b: Box) => b.x + b.w / 2;
const cy = (b: Box) => b.y + b.h / 2;

interface Layout {
  W: number;
  H: number;
  boxes: Record<string, Box>;
  edges: { id: string; from: string; to: string; d: string; tone: number }[];
  vertical: boolean;
}

const STAGES = ['idea', 'needs', 'reqs', 'arch'] as const;

function horizontal(): Layout {
  const W = 1180;
  const H = 440;
  const boxes: Record<string, Box> = {};
  const sw = 128;
  const sh = 74;
  STAGES.forEach((id, i) => (boxes[id] = { id, x: 8 + i * 146, y: H / 2 - sh / 2, w: sw, h: sh }));
  const gw = 236;
  const gh = 66;
  const gx = 610;
  const gap = (H - 16 - GROUPS.length * gh) / (GROUPS.length - 1);
  GROUPS.forEach((g, i) => (boxes[g] = { id: g, x: gx, y: 8 + i * (gh + gap), w: gw, h: gh }));
  boxes.verify = { id: 'verify', x: 924, y: H / 2 - 48, w: 116, h: 96 };
  boxes.factory = { id: 'factory', x: 1066, y: H / 2 - sh / 2, w: 110, h: sh };
  const link = (a: Box, b: Box) => {
    const x1 = a.x + a.w;
    const y1 = cy(a);
    const x2 = b.x;
    const y2 = cy(b);
    const dx = (x2 - x1) * 0.5;
    return `M${x1} ${y1} C${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`;
  };
  const edges: Layout['edges'] = [];
  for (let i = 0; i < STAGES.length - 1; i++) edges.push({ id: `${STAGES[i]}-${STAGES[i + 1]}`, from: STAGES[i], to: STAGES[i + 1], d: link(boxes[STAGES[i]], boxes[STAGES[i + 1]]), tone: 0 });
  GROUPS.forEach((g, i) => {
    edges.push({ id: `arch-${g}`, from: 'arch', to: g, d: link(boxes.arch, boxes[g]), tone: i + 1 });
    edges.push({ id: `${g}-verify`, from: g, to: 'verify', d: link(boxes[g], boxes.verify), tone: i + 1 });
  });
  edges.push({ id: 'verify-factory', from: 'verify', to: 'factory', d: link(boxes.verify, boxes.factory), tone: 0 });
  return { W, H, boxes, edges, vertical: false };
}

function vertical(width: number): Layout {
  const W = Math.max(300, Math.min(width, 520));
  const pad = 26; // room for the side rails
  const boxes: Record<string, Box> = {};
  const sw = (W - 2 * pad - 12) / 2;
  const sh = 70;
  // 2 × 2 grid for the four concept stages, read in Z order.
  boxes.idea = { id: 'idea', x: pad, y: 4, w: sw, h: sh };
  boxes.needs = { id: 'needs', x: pad + sw + 12, y: 4, w: sw, h: sh };
  boxes.reqs = { id: 'reqs', x: pad, y: 4 + sh + 26, w: sw, h: sh };
  boxes.arch = { id: 'arch', x: pad + sw + 12, y: 4 + sh + 26, w: sw, h: sh };
  let y = boxes.arch.y + sh + 44;
  const gh = 66;
  GROUPS.forEach((g) => {
    boxes[g] = { id: g, x: pad, y, w: W - 2 * pad, h: gh };
    y += gh + 14;
  });
  y += 30;
  boxes.verify = { id: 'verify', x: pad, y, w: sw, h: 84 };
  boxes.factory = { id: 'factory', x: pad + sw + 12, y: y + 7, w: sw, h: sh };
  const H = y + 90;
  const edges: Layout['edges'] = [];
  const h = (a: Box, b: Box) => `M${a.x + a.w} ${cy(a)} L${b.x} ${cy(b)}`;
  edges.push({ id: 'idea-needs', from: 'idea', to: 'needs', d: h(boxes.idea, boxes.needs), tone: 0 });
  // needs (top right) → reqs (bottom left): a Z stroke
  const n = boxes.needs;
  const r = boxes.reqs;
  const midY = n.y + n.h + 13;
  edges.push({ id: 'needs-reqs', from: 'needs', to: 'reqs', d: `M${cx(n)} ${n.y + n.h} L${cx(n)} ${midY} L${cx(r)} ${midY} L${cx(r)} ${r.y}`, tone: 0 });
  edges.push({ id: 'reqs-arch', from: 'reqs', to: 'arch', d: h(boxes.reqs, boxes.arch), tone: 0 });
  const a = boxes.arch;
  const L = 9;
  const R = W - 9;
  GROUPS.forEach((g, i) => {
    const b = boxes[g];
    const top = a.y + a.h + 18;
    edges.push({
      id: `arch-${g}`, from: 'arch', to: g, tone: i + 1,
      d: `M${cx(a)} ${a.y + a.h} L${cx(a)} ${top} L${L + 10} ${top} Q${L} ${top} ${L} ${top + 10} L${L} ${cy(b) - 8} Q${L} ${cy(b)} ${L + 8} ${cy(b)} L${b.x} ${cy(b)}`,
    });
    const v = boxes.verify;
    const bot = v.y - 16;
    edges.push({
      id: `${g}-verify`, from: g, to: 'verify', tone: i + 1,
      d: `M${b.x + b.w} ${cy(b)} L${R - 8} ${cy(b)} Q${R} ${cy(b)} ${R} ${cy(b) + 8} L${R} ${bot - 10} Q${R} ${bot} ${R - 10} ${bot} L${cx(v)} ${bot} L${cx(v)} ${v.y}`,
    });
  });
  edges.push({ id: 'verify-factory', from: 'verify', to: 'factory', d: h(boxes.verify, boxes.factory), tone: 0 });
  return { W, H, boxes, edges, vertical: true };
}

const tone = (h: Health) => (h === 'pass' ? 'var(--good)' : h === 'fail' ? 'var(--bad)' : h === 'goal' || h === 'partial' ? 'var(--warn)' : 'var(--border-strong)');
const series = (i: number) => (i === 0 ? 'var(--accent)' : `var(--series-${i})`);

function onKey(fn: () => void) {
  return (e: KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      fn();
    }
  };
}

export function Pipeline({ project, ev }: { project: Project; ev: Evaluation | null }) {
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const nav = useNavigate();
  const [hover, setHover] = useState<string | null>(null);
  const mode = width === 0 ? 0 : width < 760 ? Math.round(width / 10) * 10 : -1;
  const layout = useMemo(() => (mode > 0 ? vertical(mode) : horizontal()), [mode]);
  const motion = !reducedMotion();

  const groupTally = useMemo(() => Object.fromEntries(GROUPS.map((g) => [g, tallyFor(groupPrefixes(g), project, ev)])), [project, ev]);
  const labTally = useMemo(() => Object.fromEntries(LABS.map((l) => [l.id, tallyFor(l.metrics, project, ev)])), [project, ev]);
  const sysTally = useMemo(() => tallyFor(systemPrefixes(project), project, ev), [project, ev]);
  // `fail` = binding failures; open stretch goals are counted (and coloured) apart
  const total = ev ? { pass: ev.pass, fail: ev.fail - (ev.open_goals ?? 0), goal: ev.open_goals ?? 0, unverified: ev.unverified, n: ev.results.length } : null;

  const lit = (e: { from: string; to: string }) => hover != null && (e.from === hover || e.to === hover);

  const stageNode = (id: string, title: string, sub: string, to: string, extra?: React.ReactNode) => {
    const b = layout.boxes[id];
    const go = () => nav(to);
    return (
      <g
        key={id}
        className={`pf-node ${hover === id ? 'hot' : ''}`}
        transform={`translate(${b.x} ${b.y})`}
        role="link"
        tabIndex={0}
        aria-label={`${title}: ${sub}`}
        onClick={go}
        onKeyDown={onKey(go)}
        onMouseEnter={() => setHover(id)}
        onMouseLeave={() => setHover(null)}
      >
        <rect className="pf-box" width={b.w} height={b.h} rx={12} />
        <svg x={12} y={12} width={20} height={20} viewBox="0 0 24 24" className="pf-glyph">
          <path d={GLYPH[id]} />
        </svg>
        <text x={12} y={b.h - 26} className="pf-title">{title}</text>
        <text x={12} y={b.h - 11} className="pf-sub">{sub}</text>
        {extra}
      </g>
    );
  };

  const groupNode = (g: string, gi: number) => {
    const b = layout.boxes[g];
    const t: Tally = groupTally[g];
    const h = health(t);
    const labs = LABS.filter((l) => l.group === g);
    let x = 12;
    return (
      <g
        key={g}
        className={`pf-node pf-group ${hover === g ? 'hot' : ''}`}
        transform={`translate(${b.x} ${b.y})`}
        onMouseEnter={() => setHover(g)}
        onMouseLeave={() => setHover(null)}
      >
        <rect className="pf-box" width={b.w} height={b.h} rx={12} />
        <rect x={0} y={10} width={3} height={b.h - 20} rx={1.5} style={{ fill: series(gi + 1) }} />
        <text x={12} y={22} className="pf-title">{g}</text>
        <g transform={`translate(${b.w - 12} 22)`}>
          <text textAnchor="end" className="pf-status" style={{ fill: tone(h) }}>
            {healthIcon(h)} {t.total ? `${t.pass}/${t.total}` : '—'}
          </text>
        </g>
        {labs.map((l) => {
          const lt = labTally[l.id];
          const lh = health(lt);
          const name = SHORT[l.id] ?? l.title;
          const w = 24 + name.length * 6.6;
          const cxp = x;
          x += w + 6;
          const go = () => nav(l.path);
          return (
            <g
              key={l.id}
              className="pf-lab"
              transform={`translate(${cxp} ${b.h - 30})`}
              role="link"
              tabIndex={0}
              aria-label={`${l.title}: ${lt.total ? `${lt.pass} of ${lt.total} requirements verified${lt.goal ? `, ${plural(lt.goal, 'open goal')}` : ''}` : 'no bound requirements'}`}
              onClick={go}
              onKeyDown={onKey(go)}
            >
              <title>{l.blurb}</title>
              <rect width={w} height={20} rx={10} />
              <circle cx={11} cy={10} r={3.6} style={{ fill: tone(lh) }} />
              <text x={19} y={14}>{name}</text>
            </g>
          );
        })}
      </g>
    );
  };

  const vb = layout.boxes.verify;
  return (
    <div ref={ref} className="pf-pipeline">
      {width > 0 && (
        <svg viewBox={`0 0 ${layout.W} ${layout.H}`} width="100%" role="img" aria-label="Protoforge pipeline from idea to factory package with live verification status">
          <defs>
            <radialGradient id="pf-spark">
              <stop offset="0" stopColor="#fff" stopOpacity="1" />
              <stop offset="1" stopColor="#fff" stopOpacity="0" />
            </radialGradient>
          </defs>
          <g className="pf-edges">
            {layout.edges.map((e) => (
              <path key={e.id} id={`pf-e-${e.id.replace(/[^a-zA-Z0-9-]/g, '')}`} d={e.d} className={`pf-edge ${lit(e) ? 'lit' : ''}`} style={{ stroke: lit(e) ? series(e.tone) : undefined }} />
            ))}
          </g>
          {motion && (
            <g className="pf-particles" aria-hidden>
              {layout.edges.map((e, i) =>
                [0, 1].map((k) => {
                  const dur = (layout.vertical ? 3.6 : 2.6) + (i % 3) * 0.35;
                  return (
                    <circle key={`${e.id}-${k}`} r={layout.vertical ? 2.4 : 2.8} style={{ fill: series(e.tone) }} opacity={0}>
                      <animateMotion dur={`${dur}s`} begin={`${(k * dur) / 2 + (i % 5) * 0.21}s`} repeatCount="indefinite" rotate="auto">
                        <mpath href={`#pf-e-${e.id.replace(/[^a-zA-Z0-9-]/g, '')}`} />
                      </animateMotion>
                      <animate attributeName="opacity" values="0;1;1;0" keyTimes="0;0.12;0.85;1" dur={`${dur}s`} begin={`${(k * dur) / 2 + (i % 5) * 0.21}s`} repeatCount="indefinite" />
                    </circle>
                  );
                }),
              )}
            </g>
          )}
          {stageNode('idea', 'Idea', project.meta.codename ? `${project.meta.codename} concept` : 'concept', '/systems')}
          {stageNode('needs', 'Needs', `${project.needs.length} captured`, '/systems')}
          {stageNode('reqs', 'Requirements', `${project.requirements.length} → metrics`, '/systems')}
          {stageNode('arch', 'Architecture', `${project.subsystems.length} subsystems`, '/systems')}
          {GROUPS.map(groupNode)}
          {stageNode(
            'verify',
            'Verification',
            // short enough for the box: "51/54 pass · 2 fail", or "51/54 · 3 open goals" when only goals are open
            total ? (total.fail ? `${total.pass}/${total.n} pass · ${total.fail} fail` : total.goal ? `${total.pass}/${total.n} · ${plural(total.goal, 'open goal')}` : `${total.pass}/${total.n} pass`) : 'running…',
            '/systems',
            total && (
              <g transform={`translate(${vb.w - 26} 22)`} aria-hidden>
                <circle r={14} className="pf-ring-bg" />
                <circle
                  r={14}
                  className="pf-ring"
                  style={{ stroke: total.fail ? 'var(--bad)' : total.goal ? 'var(--warn)' : 'var(--good)' }}
                  strokeDasharray={`${(2 * Math.PI * 14 * total.pass) / Math.max(1, total.n)} 999`}
                  transform="rotate(-90)"
                />
              </g>
            ),
          )}
          {stageNode('factory', 'Factory', 'fab-ready zip', '/mfg')}
          {!layout.vertical && (
            <g className="pf-phase" aria-hidden>
              <text x={8} y={layout.boxes.idea.y - 34}>① Define</text>
              <text x={8} y={layout.boxes.idea.y - 18} className="pf-phase-sub">left side of the V: concept → architecture</text>
              <text x={vb.x} y={vb.y - 52}>③ Verify &amp; build</text>
              <text x={vb.x} y={vb.y - 36} className="pf-phase-sub">right side of the V</text>
              <text x={layout.boxes[GROUPS[0]].x + layout.boxes[GROUPS[0]].w + 14} y={layout.boxes[GROUPS[0]].y + 14} className="pf-phase-sub">② design &amp; analyse</text>
            </g>
          )}
          {sysTally.total > 0 && !layout.vertical && (
            <text x={cx(vb)} y={vb.y + vb.h + 20} textAnchor="middle" className="pf-note">
              + {sysTally.total} system-level (mass, fit, cost)
            </text>
          )}
        </svg>
      )}
    </div>
  );
}
