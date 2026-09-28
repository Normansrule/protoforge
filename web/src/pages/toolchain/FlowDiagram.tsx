// Animated map: Protoforge engine → generated files → real tools → vendors.
// Particles run along the connectors (requestAnimationFrame, paused for
// prefers-reduced-motion); hovering or focusing a tool highlights its path.
import { useEffect, useMemo, useRef, useState } from 'react';
import { useElementWidth } from '../../components/useElementWidth';
import { useReducedMotion } from '../home/motion';
import { TOOLS, VENDOR_STAGES } from './tools';

export const FILES: { id: string; label: string; tools: string[]; lab: string }[] = [
  { id: 'kicad', label: '.kicad_pcb · .kicad_pro', tools: ['kicad', 'kicanvas'], lab: 'PCB Studio' },
  { id: 'gerber', label: 'Gerber + drill', tools: ['tracespace'], lab: 'PCB Studio' },
  { id: 'step', label: 'STEP (AP214)', tools: ['onshape', 'freecad', '3dviewer'], lab: 'CAD Studio' },
  { id: 'scad', label: 'OpenSCAD .scad', tools: ['openscad'], lab: 'CAD Studio' },
  { id: 'stl', label: 'STL parts', tools: ['3dviewer', 'bambu'], lab: 'CAD Studio' },
  { id: 'gcode', label: 'G-code', tools: ['ncviewer'], lab: 'Print Lab' },
  { id: 'ctz', label: 'CircuitJS link', tools: ['falstad'], lab: 'Circuit Lab' },
  { id: 'cir', label: 'SPICE .cir', tools: ['ngspice'], lab: 'Circuit Lab' },
  { id: 'verilog', label: 'Verilog .v', tools: ['edaplayground', 'digitaljs', '8bitworkshop'], lab: 'Chip Lab' },
  { id: 'scene', label: 'Ray Optics scene', tools: ['rayoptics'], lab: 'Optics Lab' },
  { id: 'qasm', label: 'OpenQASM · Quirk URL', tools: ['ibmcomposer', 'quirk'], lab: 'Quantum Lab' },
  { id: 'sketch', label: 'ESP32 sketch', tools: ['wokwi'], lab: 'Prototype' },
];

interface Node { id: string; x: number; y: number; w: number; h: number; label: string; col: number }
interface Edge { id: string; from: string; to: string; d: string; tone: number }

function bez(x1: number, y1: number, x2: number, y2: number) {
  const dx = (x2 - x1) * 0.5;
  return `M${x1} ${y1} C${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`;
}

function point(d: string, t: number): [number, number] {
  const n = d.match(/-?\d+(\.\d+)?/g)!.map(Number);
  const [x1, y1, c1x, c1y, c2x, c2y, x2, y2] = n;
  const u = 1 - t;
  return [u * u * u * x1 + 3 * u * u * t * c1x + 3 * u * t * t * c2x + t * t * t * x2, u * u * u * y1 + 3 * u * u * t * c1y + 3 * u * t * t * c2y + t * t * t * y2];
}

function layout() {
  const W = 1120;
  const rowH = 30;
  const tools = TOOLS.map((t) => t.id);
  const H = Math.max(tools.length, FILES.length) * rowH + 40;
  const nodes: Record<string, Node> = {};
  nodes.engine = { id: 'engine', x: 8, y: H / 2 - 70, w: 170, h: 140, label: 'Protoforge engine', col: 0 };
  const fGap = (H - 20) / FILES.length;
  FILES.forEach((f, i) => (nodes[`f:${f.id}`] = { id: `f:${f.id}`, x: 250, y: 10 + i * fGap + (fGap - 24) / 2, w: 176, h: 24, label: f.label, col: 1 }));
  // order tools by their first file so edges do not cross much
  const order = FILES.flatMap((f) => f.tools).filter((t, i, a) => a.indexOf(t) === i);
  const tGap = (H - 20) / order.length;
  order.forEach((id, i) => {
    const t = TOOLS.find((x) => x.id === id)!;
    nodes[`t:${id}`] = { id: `t:${id}`, x: 540, y: 10 + i * tGap + (tGap - 24) / 2, w: 186, h: 24, label: t.name, col: 2 };
  });
  const vGap = (H - 20) / VENDOR_STAGES.length;
  VENDOR_STAGES.forEach((v, i) => (nodes[`v:${v.id}`] = { id: `v:${v.id}`, x: 870, y: 10 + i * vGap + (vGap - 40) / 2, w: 242, h: 40, label: v.label, col: 3 }));
  const edges: Edge[] = [];
  const e = nodes.engine;
  FILES.forEach((f, i) => {
    const b = nodes[`f:${f.id}`];
    edges.push({ id: `e-${f.id}`, from: 'engine', to: b.id, d: bez(e.x + e.w, e.y + e.h / 2, b.x, b.y + b.h / 2), tone: i });
    f.tools.forEach((t) => {
      const c = nodes[`t:${t}`];
      edges.push({ id: `${f.id}-${t}`, from: b.id, to: c.id, d: bez(b.x + b.w, b.y + b.h / 2, c.x, c.y + c.h / 2), tone: i });
    });
  });
  VENDOR_STAGES.forEach((v, i) =>
    v.tools.forEach((t) => {
      const c = nodes[`t:${t}`];
      const d = nodes[`v:${v.id}`];
      edges.push({ id: `${t}-${v.id}`, from: c.id, to: d.id, d: bez(c.x + c.w, c.y + c.h / 2, d.x, d.y + d.h / 2), tone: i + 3 });
    }),
  );
  return { W, H, nodes, edges };
}

/** Ids connected to `id` (upstream and downstream). */
function pathOf(id: string | null, edges: Edge[]): Set<string> {
  const s = new Set<string>();
  if (!id) return s;
  s.add(id);
  const walk = (cur: string, dir: 'up' | 'down') => {
    for (const e of edges) {
      const [a, b] = dir === 'down' ? [e.from, e.to] : [e.to, e.from];
      if (a === cur && !s.has(e.id)) {
        s.add(e.id);
        s.add(b);
        walk(b, dir);
      }
    }
  };
  walk(id, 'down');
  walk(id, 'up');
  return s;
}

export function FlowDiagram({ onPick }: { onPick?: (toolId: string) => void }) {
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const reduced = useReducedMotion();
  const [playing, setPlaying] = useState(true);
  const [hover, setHover] = useState<string | null>(null);
  const L = useMemo(layout, []);
  const dots = useRef<SVGGElement>(null);
  const hi = useMemo(() => pathOf(hover, L.edges), [hover, L.edges]);
  const run = playing && !reduced;
  const narrow = width > 0 && width < 760;

  useEffect(() => {
    if (!run || !dots.current) return;
    let raf = 0;
    const g = dots.current;
    const circles = Array.from(g.querySelectorAll('circle'));
    const t0 = performance.now();
    const tick = (now: number) => {
      const s = (now - t0) / 1000;
      circles.forEach((c, i) => {
        const e = L.edges[i];
        const col = L.nodes[e.from].col;
        // a pulse leaves the engine, then fans out column by column
        const phase = ((s * 0.45 - col * 0.33 + (i % 5) * 0.07) % 1 + 1) % 1;
        const [x, y] = point(e.d, phase);
        c.setAttribute('cx', x.toFixed(1));
        c.setAttribute('cy', y.toFixed(1));
        c.setAttribute('opacity', (Math.sin(Math.PI * phase) * 0.9).toFixed(2));
      });
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [run, L, narrow]);

  return (
    <div ref={ref} className="tc-flow">
      {narrow ? <FlowList onPick={onPick} /> : <>
      <div className="row" style={{ justifyContent: 'space-between', marginBottom: 6 }}>
        <div className="tc-flow-cols small muted">
          <span>Engine</span><span>Files it writes</span><span>Real tools</span><span>Who builds it</span>
        </div>
        {!reduced && <button className="btn small" onClick={() => setPlaying((p) => !p)} aria-pressed={!playing}>{playing ? '❚❚ Pause' : '▶ Play'}</button>}
      </div>
      <svg viewBox={`0 0 ${L.W} ${L.H}`} width="100%" role="img" aria-label="Protoforge engine to files to tools to vendors">
        <g fill="none">
          {L.edges.map((e) => (
            <path key={e.id} d={e.d} stroke={hi.has(e.id) ? `var(--series-${(e.tone % 8) + 1})` : 'var(--border-strong)'} strokeWidth={hi.has(e.id) ? 2.2 : 1.1} opacity={hover && !hi.has(e.id) ? 0.25 : 0.9} />
          ))}
        </g>
        <g ref={dots}>
          {L.edges.map((e) => <circle key={e.id} r={2.6} fill={`var(--series-${(e.tone % 8) + 1})`} opacity={0} />)}
        </g>
        {Object.values(L.nodes).map((n) => {
          const dim = hover && !hi.has(n.id);
          const isTool = n.col === 2;
          const toolId = n.id.slice(2);
          if (n.id === 'engine') {
            return (
              <g key={n.id} opacity={dim ? 0.35 : 1}>
                <rect x={n.x} y={n.y} width={n.w} height={n.h} rx={14} fill="var(--accent-soft)" stroke="var(--accent)" />
                <text x={n.x + 14} y={n.y + 28} fill="var(--text)" fontWeight={750} fontSize={15}>Protoforge</text>
                <text x={n.x + 14} y={n.y + 48} fill="var(--text-2)" fontSize={12}>Rust engine → WebAssembly</text>
                {['13+ labs · 1 project.json', 'simulate · verify', 'export in the browser'].map((s, i) => <text key={s} x={n.x + 14} y={n.y + 76 + i * 18} fill="var(--muted)" fontSize={11.5}>{s}</text>)}
              </g>
            );
          }
          return (
            <g
              key={n.id}
              opacity={dim ? 0.35 : 1}
              onMouseEnter={() => setHover(n.id)}
              onMouseLeave={() => setHover(null)}
              onFocus={() => setHover(n.id)}
              onBlur={() => setHover(null)}
              onClick={() => isTool && onPick?.(toolId)}
              onKeyDown={(ev) => { if (isTool && (ev.key === 'Enter' || ev.key === ' ')) { ev.preventDefault(); onPick?.(toolId); } }}
              tabIndex={0}
              role={isTool ? 'button' : undefined}
              style={{ cursor: isTool ? 'pointer' : 'default', outline: 'none' }}
            >
              <rect x={n.x} y={n.y} width={n.w} height={n.h} rx={n.col === 3 ? 10 : 12} fill={n.col === 1 ? 'var(--surface-3)' : 'var(--surface-2)'} stroke={hi.has(n.id) ? 'var(--accent)' : 'var(--border-strong)'} />
              <text x={n.x + 10} y={n.y + n.h / 2 + 4} fill="var(--text)" fontSize={12} fontFamily={n.col === 1 ? 'var(--mono)' : undefined}>{n.label}</text>
            </g>
          );
        })}
      </svg>
      </>}
    </div>
  );
}

/** Phone layout: one row per file type with animated connectors. */
const FlowList = ({ onPick }: { onPick?: (id: string) => void }) => (
  <div className="tc-flowlist">
    {FILES.map((f, i) => (
      <div key={f.id} className="tc-flowrow">
        <span className="tc-file mono">{f.label}</span>
        <svg width="26" height="12" aria-hidden><line x1="0" y1="6" x2="26" y2="6" className="flow-line" stroke={`var(--series-${(i % 8) + 1})`} strokeWidth={2} /></svg>
        <span className="row" style={{ gap: 4 }}>
          {f.tools.map((t) => (
            <button key={t} className="chip" style={{ cursor: 'pointer' }} onClick={() => onPick?.(t)}>{TOOLS.find((x) => x.id === t)?.name}</button>
          ))}
        </span>
      </div>
    ))}
  </div>
);
