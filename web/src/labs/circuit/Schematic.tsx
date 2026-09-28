// Animated SVG schematic. Nets are coloured by voltage, dots flow along the
// wires at a speed set by the branch current (direction by sign), and every
// part can be hovered/tapped to inspect its V, I and P.
import { useEffect, useMemo, useRef, type ReactNode } from 'react';
import type { ElementInfo, Layout, Part, Snapshot, Wire } from './types';
import { elemVIP, prefersReducedMotion, rampColor, si, switchOn } from './util';

const SW = 0.13; // stroke width in grid units

// ---------------------------------------------------------------------------
// Symbols (drawn with pin 1 at (0,−2) and pin 2 at (0,+2); rotated by `dir`)
// ---------------------------------------------------------------------------

const ROT: Record<string, number> = { down: 0, up: 180, right: -90, left: 90 };

function Lead({ y0, y1 }: { y0: number; y1: number }) {
  return <line x1={0} y1={y0} x2={0} y2={y1} />;
}

function SymbolBody({ sym, el, snap, glow }: { sym: string; el?: ElementInfo; snap?: Snapshot; glow: number }) {
  switch (sym) {
    case 'resistor': {
      const ys = [-1.3, -1.083, -0.65, -0.217, 0.217, 0.65, 1.083, 1.3];
      const xs = [0, 0.45, -0.45, 0.45, -0.45, 0.45, -0.45, 0];
      return (
        <g>
          <Lead y0={-2} y1={-1.3} />
          <polyline points={ys.map((y, k) => `${xs[k]},${y}`).join(' ')} fill="none" strokeLinejoin="round" />
          <Lead y0={1.3} y1={2} />
        </g>
      );
    }
    case 'capacitor':
      return (
        <g>
          <Lead y0={-2} y1={-0.28} />
          <line x1={-0.9} y1={-0.28} x2={0.9} y2={-0.28} strokeWidth={SW * 1.5} />
          <line x1={-0.9} y1={0.28} x2={0.9} y2={0.28} strokeWidth={SW * 1.5} />
          <Lead y0={0.28} y1={2} />
        </g>
      );
    case 'inductor':
      return (
        <g>
          <Lead y0={-2} y1={-1.3} />
          <path d="M0,-1.3 a0.325,0.325 0 0 1 0,0.65 a0.325,0.325 0 0 1 0,0.65 a0.325,0.325 0 0 1 0,0.65 a0.325,0.325 0 0 1 0,0.65" fill="none" />
          <Lead y0={1.3} y1={2} />
        </g>
      );
    case 'vsource': {
      const w = el?.wave ?? 'dc';
      return (
        <g>
          <Lead y0={-2} y1={-1.1} />
          <circle r={1.1} fill="var(--surface-1)" />
          <Lead y0={1.1} y1={2} />
          {w === 'sin' ? (
            <path d="M-0.55,0.05 q0.275,-0.6 0.55,0 t0.55,0" fill="none" />
          ) : w === 'pulse' || w === 'pwl' ? (
            <path d="M-0.6,0.3 h0.3 v-0.6 h0.5 v0.6 h0.4" fill="none" strokeWidth={SW * 0.9} />
          ) : (
            <>
              <path d="M-0.25,-0.45 h0.5 M0,-0.7 v0.5" />
              <path d="M-0.25,0.5 h0.5" />
            </>
          )}
        </g>
      );
    }
    case 'isource':
      return (
        <g>
          <Lead y0={-2} y1={-1.1} />
          <circle r={1.1} fill="var(--surface-1)" />
          <Lead y0={1.1} y1={2} />
          <path d="M0,-0.6 V0.55 M-0.3,0.2 L0,0.6 L0.3,0.2" fill="none" strokeLinejoin="round" />
        </g>
      );
    case 'battery':
      return (
        <g>
          <Lead y0={-2} y1={-0.55} />
          <line x1={-0.95} y1={-0.55} x2={0.95} y2={-0.55} />
          <line x1={-0.5} y1={-0.2} x2={0.5} y2={-0.2} strokeWidth={SW * 2.2} />
          <line x1={-0.95} y1={0.2} x2={0.95} y2={0.2} />
          <line x1={-0.5} y1={0.55} x2={0.5} y2={0.55} strokeWidth={SW * 2.2} />
          <Lead y0={0.55} y1={2} />
          <path d="M0.75,-1.25 h0.45 M0.975,-1.475 v0.45" strokeWidth={SW * 0.8} />
        </g>
      );
    case 'diode':
    case 'led':
      return (
        <g>
          <Lead y0={-2} y1={-0.6} />
          <path d="M-0.7,-0.6 H0.7 L0,0.55 Z" fill={sym === 'led' ? `color-mix(in srgb, var(--series-5) ${Math.round(15 + 85 * glow)}%, var(--surface-1))` : 'var(--surface-1)'} strokeLinejoin="round" />
          <line x1={-0.7} y1={0.55} x2={0.7} y2={0.55} />
          <Lead y0={0.55} y1={2} />
          {sym === 'led' && (
            <g strokeWidth={SW * 0.8} opacity={0.5 + 0.5 * glow}>
              <path d="M0.85,-0.35 L1.55,-0.95 M1.2,-0.95 H1.55 V-0.6" fill="none" />
              <path d="M0.85,0.15 L1.55,-0.45 M1.2,-0.45 H1.55 V-0.1" fill="none" />
            </g>
          )}
        </g>
      );
    case 'switch': {
      const on = el && snap ? switchOn(el, snap) : false;
      return (
        <g>
          <Lead y0={-2} y1={-0.9} />
          <circle cx={0} cy={-0.9} r={0.14} fill="var(--surface-1)" />
          <circle cx={0} cy={0.9} r={0.14} fill="var(--surface-1)" />
          <line x1={0} y1={0.9} x2={on ? 0 : -0.8} y2={on ? -0.9 : -0.65} strokeWidth={SW * 1.3} style={{ transition: 'all 0.08s' }} />
          <Lead y0={0.9} y1={2} />
        </g>
      );
    }
    case 'vcvs':
    case 'vccs':
      return (
        <g>
          <Lead y0={-2} y1={-1.1} />
          <path d="M0,-1.1 L1.1,0 L0,1.1 L-1.1,0 Z" fill="var(--surface-1)" strokeLinejoin="round" />
          <Lead y0={1.1} y1={2} />
          {sym === 'vcvs' ? <path d="M-0.22,-0.45 h0.44 M0,-0.67 v0.44 M-0.22,0.45 h0.44" /> : <path d="M0,-0.55 V0.5 M-0.28,0.18 L0,0.55 L0.28,0.18" fill="none" />}
        </g>
      );
    default:
      return <circle r={1} fill="none" strokeDasharray="0.2 0.2" />;
  }
}

function MosBody({ pmos }: { pmos: boolean }) {
  // gate on the left (−x); drain/source at (0, ∓2)
  const topY = -0.7;
  const botY = 0.7;
  return (
    <g>
      <line x1={-2} y1={0} x2={pmos ? -1.15 : -0.8} y2={0} />
      {pmos && <circle cx={-0.97} cy={0} r={0.18} fill="var(--surface-1)" />}
      <line x1={-0.78} y1={-0.85} x2={-0.78} y2={0.85} />
      <line x1={-0.45} y1={-1.05} x2={-0.45} y2={-0.45} strokeWidth={SW * 1.4} />
      <line x1={-0.45} y1={-0.25} x2={-0.45} y2={0.25} strokeWidth={SW * 1.4} />
      <line x1={-0.45} y1={0.45} x2={-0.45} y2={1.05} strokeWidth={SW * 1.4} />
      <path d={`M-0.45,${topY} H0 V-2`} fill="none" />
      <path d={`M-0.45,${botY} H0 V2`} fill="none" />
      <path d="M-0.45,0 H0" fill="none" />
      {pmos ? (
        <path d={`M-0.1,${topY - 0.22} L-0.42,${topY} L-0.1,${topY + 0.22}`} fill="none" strokeLinejoin="round" />
      ) : (
        <path d={`M-0.32,${botY - 0.22} L0,${botY} L-0.32,${botY + 0.22}`} fill="none" strokeLinejoin="round" />
      )}
    </g>
  );
}

function OpampBody() {
  return (
    <g>
      <path d="M-2,-2 L-2,2 L2.4,0 Z" fill="var(--surface-1)" strokeLinejoin="round" />
      <line x1={-3} y1={-1} x2={-2} y2={-1} />
      <line x1={-3} y1={1} x2={-2} y2={1} />
      <line x1={2.4} y1={0} x2={3} y2={0} />
      <path d="M-1.7,-1 h0.45 M-1.475,-1.225 v0.45 M-1.7,1 h0.45" strokeWidth={SW * 0.9} />
    </g>
  );
}

function Ground({ x, y }: { x: number; y: number }) {
  return (
    <g transform={`translate(${x} ${y})`} className="cl-sym">
      <line x1={-0.65} y1={0} x2={0.65} y2={0} />
      <line x1={-0.4} y1={0.28} x2={0.4} y2={0.28} />
      <line x1={-0.16} y1={0.56} x2={0.16} y2={0.56} />
    </g>
  );
}

// ---------------------------------------------------------------------------
// Geometry helpers
// ---------------------------------------------------------------------------

interface WireGeom {
  pts: [number, number][];
  cum: number[];
  len: number;
}

function geom(pts: [number, number][]): WireGeom {
  const cum = [0];
  for (let k = 1; k < pts.length; k++) cum.push(cum[k - 1] + Math.hypot(pts[k][0] - pts[k - 1][0], pts[k][1] - pts[k - 1][1]));
  return { pts, cum, len: cum[cum.length - 1] };
}

function pointAt(g: WireGeom, s: number): [number, number] {
  let k = 1;
  while (k < g.cum.length - 1 && g.cum[k] < s) k++;
  const seg = g.cum[k] - g.cum[k - 1] || 1;
  const u = (s - g.cum[k - 1]) / seg;
  const a = g.pts[k - 1];
  const b = g.pts[k];
  return [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u];
}

const SPACING = 1.1;

/** Map |I| to a dot speed (grid units / s) on a log scale. */
function speedOf(i: number, imax: number): number {
  const a = Math.abs(i);
  const imin = Math.max(imax * 1e-4, 1e-12);
  if (!(a > imin) || !(imax > 0)) return 0;
  const f = Math.log10(a / imin) / Math.log10(imax / imin);
  return Math.sign(i) * (0.7 + 6.3 * Math.max(0, Math.min(1, f)));
}

/**
 * Imperative dot animation (no React re-render per frame). `currents` holds
 * the current of each path; updated by the parent on every render.
 */
function useFlowDots(groupRef: React.RefObject<SVGGElement | null>, paths: WireGeom[], currents: React.RefObject<number[]>, imax: React.RefObject<number>, active: boolean) {
  useEffect(() => {
    const g = groupRef.current;
    if (!g) return;
    const reduce = prefersReducedMotion();
    const offsets = paths.map(() => Math.random() * SPACING);
    let last = performance.now();
    let raf = 0;
    const draw = (now: number) => {
      const dt = Math.min(0.1, (now - last) / 1000);
      last = now;
      const groups = g.children;
      for (let w = 0; w < paths.length; w++) {
        const grp = groups[w] as SVGGElement | undefined;
        if (!grp) continue;
        const sp = speedOf(currents.current?.[w] ?? 0, imax.current ?? 0);
        if (!reduce && active) offsets[w] = (offsets[w] + sp * dt) % SPACING;
        const geomw = paths[w];
        const dots = grp.children;
        grp.setAttribute('opacity', sp === 0 ? '0' : '1');
        for (let k = 0; k < dots.length; k++) {
          let s = k * SPACING + offsets[w];
          s = ((s % geomw.len) + geomw.len) % geomw.len;
          const [x, y] = pointAt(geomw, s);
          dots[k].setAttribute('cx', x.toFixed(3));
          dots[k].setAttribute('cy', y.toFixed(3));
        }
      }
      if (!reduce && active) raf = requestAnimationFrame(draw);
    };
    raf = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(raf);
  }, [groupRef, paths, currents, imax, active]);
}

function FlowLayer({ paths, currents, imax, active }: { paths: WireGeom[]; currents: number[]; imax: number; active: boolean }) {
  const gref = useRef<SVGGElement>(null);
  const cref = useRef<number[]>(currents);
  const iref = useRef<number>(imax);
  cref.current = currents;
  iref.current = imax;
  useFlowDots(gref, paths, cref, iref, active);
  return (
    <g ref={gref} className="cl-dots" pointerEvents="none">
      {paths.map((p, w) => (
        <g key={w}>
          {Array.from({ length: Math.max(1, Math.floor(p.len / SPACING)) }, (_, k) => (
            <circle key={k} r={0.14} />
          ))}
        </g>
      ))}
    </g>
  );
}

// ---------------------------------------------------------------------------
// Schematic
// ---------------------------------------------------------------------------

export interface SchematicProps {
  layout: Layout;
  elements?: ElementInfo[];
  snap?: Snapshot;
  vrange?: [number, number];
  imax?: number;
  animate?: boolean;
  thumb?: boolean;
  hovered?: string | null;
  onHover?: (el: string | null) => void;
  onPick?: (el: string) => void;
  maxHeight?: number;
}

function partLabelPos(p: Part): { x: number; y: number; anchor: 'start' | 'middle' | 'end'; stack: boolean } {
  const vertical = !(p.dir === 'left' || p.dir === 'right') || p.sym === 'nmos' || p.sym === 'pmos';
  if (p.lpos === 'l') return { x: p.x - (p.sym === 'nmos' || p.sym === 'pmos' ? 2.3 : 1.45), y: p.y - 0.15, anchor: 'end', stack: true };
  if (p.lpos === 'r') return { x: p.x + 1.45, y: p.y - 0.15, anchor: 'start', stack: true };
  if (p.lpos === 't') return { x: p.x, y: p.y - (vertical ? 2.6 : 1.25), anchor: 'middle', stack: false };
  if (p.lpos === 'b') return { x: p.x, y: p.y + (vertical ? 3.1 : 1.75), anchor: 'middle', stack: false };
  if (p.sym === 'opamp') return { x: p.x + 0.2, y: p.y - 2.5, anchor: 'middle', stack: false };
  if (p.sym === 'nmos' || p.sym === 'pmos') return { x: p.x + 0.7, y: p.y - 0.15, anchor: 'start', stack: true };
  if (p.dir === 'left' || p.dir === 'right') return { x: p.x, y: p.y - 1.25, anchor: 'middle', stack: false };
  return { x: p.x + 1.45, y: p.y - 0.15, anchor: 'start', stack: true };
}

export function Schematic(props: SchematicProps) {
  const { layout, elements = [], snap, thumb, hovered } = props;
  const byName = useMemo(() => new Map(elements.map((e) => [e.name.toLowerCase(), e])), [elements]);
  const vr = props.vrange ?? [0, 1];
  const color = (node: string) => (snap && !thumb ? rampColor(snap.v(node), vr[0], vr[1]) : 'var(--muted)');
  const paths = useMemo(() => layout.wires.map((w) => geom(w.pts)), [layout]);
  const flowI = (w: Wire) => (snap ? w.flow.reduce((a, [el, s]) => a + s * snap.i(el), 0) : 0);
  const currents = layout.wires.map(flowI);

  // junction dots: points shared by ≥ 3 wire ends / pins
  const junctions = useMemo(() => {
    const cnt = new Map<string, { x: number; y: number; n: number; node: string }>();
    const add = (x: number, y: number, node: string, weight = 1) => {
      const k = `${x},${y}`;
      const e = cnt.get(k) ?? { x, y, n: 0, node };
      e.n += weight;
      cnt.set(k, e);
    };
    for (const w of layout.wires) {
      w.pts.forEach((p, k) => add(p[0], p[1], w.node, k === 0 || k === w.pts.length - 1 ? 1 : 2));
    }
    for (const p of layout.parts) for (const pin of p.pins) add(pin.x, pin.y, pin.node);
    return [...cnt.values()].filter((e) => e.n >= 3);
  }, [layout]);

  const pad = thumb ? 0.6 : 1.2;
  const vb = `${-pad} ${-pad} ${layout.w + 2 * pad} ${layout.h + 2 * pad}`;
  const imax = props.imax ?? 0;

  const partNode = (p: Part): ReactNode => {
    const el = byName.get(p.el.toLowerCase());
    const isHover = hovered && p.members.some((m) => m.toLowerCase() === hovered.toLowerCase());
    let glow = 0;
    if (p.sym === 'led' && el && snap) {
      const { i } = elemVIP(el, snap);
      glow = imax > 0 ? Math.max(0, Math.min(1, i / (imax * 0.8))) : 0;
    }
    const body =
      p.sym === 'nmos' || p.sym === 'pmos' ? (
        <g transform={p.dir === 'left' ? 'scale(-1 1)' : undefined}><MosBody pmos={p.sym === 'pmos'} /></g>
      ) : p.sym === 'opamp' ? (
        <OpampBody />
      ) : (
        <g transform={`rotate(${ROT[p.dir] ?? 0})`}><SymbolBody sym={p.sym} el={el} snap={snap} glow={glow} /></g>
      );
    const lp = partLabelPos(p);
    const value = el ? shortValue(el.value) + (p.sym === 'switch' ? ` · ⎍${el.nodes[2]}` : '') : '';
    return (
      <g
        key={p.el}
        className={`cl-part ${isHover ? 'hover' : ''}`}
        onMouseEnter={() => props.onHover?.(p.el)}
        onMouseLeave={() => props.onHover?.(null)}
        onClick={() => props.onPick?.(p.el)}
      >
        <rect x={p.x - 2.2} y={p.y - 2.2} width={4.4} height={4.4} fill="transparent" rx={0.6} className="cl-hit" />
        <g transform={`translate(${p.x} ${p.y})`} className="cl-sym">{body}</g>
        {!thumb && p.lpos !== 'none' && (
          <text x={lp.x} y={lp.y} textAnchor={lp.anchor} className="cl-plabel">
            <tspan className="cl-pname">{p.sym === 'opamp' ? 'op-amp' : p.el}</tspan>
            {value && (lp.stack ? <tspan x={lp.x} dy={0.72} className="cl-pval">{value}</tspan> : <tspan className="cl-pval"> {value}</tspan>)}
          </text>
        )}
      </g>
    );
  };

  return (
    <svg className={`cl-schem ${thumb ? 'thumb' : ''}`} viewBox={vb} preserveAspectRatio="xMidYMid meet" style={{ maxHeight: props.maxHeight, maxWidth: thumb ? undefined : (layout.w + 2 * pad) * 26, marginInline: 'auto' }} role="img" aria-label="circuit schematic">
      {!thumb && layout.boxes.map((b, k) => (
        <g key={`b${k}`} className="cl-box">
          <rect x={b.x} y={b.y} width={b.w} height={b.h} rx={0.5} />
          <text x={b.x + 0.35} y={b.y + b.h - 0.35}>{b.label}</text>
        </g>
      ))}
      <g className="cl-wires">
        {layout.wires.map((w, k) => (
          <polyline key={k} points={w.pts.map((p) => p.join(',')).join(' ')} stroke={color(w.node)} fill="none" strokeWidth={thumb ? 0.22 : 0.2} strokeLinejoin="round" strokeLinecap="round" />
        ))}
      </g>
      {!thumb && <FlowLayer paths={paths} currents={currents} imax={imax} active={props.animate !== false} />}
      {junctions.map((j, k) => <circle key={`j${k}`} cx={j.x} cy={j.y} r={0.22} fill={color(j.node)} />)}
      {layout.grounds.map((g, k) => <Ground key={`g${k}`} x={g[0]} y={g[1]} />)}
      {layout.parts.map(partNode)}
      {!thumb && layout.labels.map((l, k) => {
        const v = l.node && snap ? snap.v(l.node) : null;
        return (
          <g key={`l${k}`} className={`cl-label ${l.port ? 'port' : ''}`}>
            {l.port && <circle cx={l.x} cy={l.y} r={0.24} fill="var(--surface-1)" stroke={l.node ? color(l.node) : 'var(--muted)'} strokeWidth={0.12} />}
            <text x={l.x + (l.port ? 0.5 : 0)} y={l.y + (l.port ? 0.25 : 0)} textAnchor="start">
              {l.text}
              {v !== null && <tspan className="cl-lval" dx={0.35}>{si(v, 'V', 4)}</tspan>}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

/** Keep schematic labels short: drop waveform detail. */
function shortValue(v: string) {
  const sin = v.startsWith('SIN ');
  return v
    .replace(/ · AC .*$/, '')
    .replace(sin ? /$^/ : / @ .*$/, '')
    .replace(/ \(IC [^)]*\)/, '')
    .replace(/^PULSE /, '⎍ ')
    .replace(/^SIN /, '∿ ')
    .replace(/ V → /, '→')
    .replace(/ A → /, '→')
    .replace(/^\S+ Ron /, 'Ron ');
}

// ---------------------------------------------------------------------------
// Node graph for custom netlists (no layout): nodes on a circle, elements as
// labelled edges with animated current.
// ---------------------------------------------------------------------------

export function NodeGraph({ elements, nodes, snap, vrange, imax, animate, onHover, hovered }: {
  elements: ElementInfo[]; nodes: string[]; snap: Snapshot; vrange: [number, number]; imax: number; animate: boolean;
  onHover?: (el: string | null) => void; hovered?: string | null;
}) {
  const all = useMemo(() => ['0', ...nodes], [nodes]);
  const pos = useMemo(() => {
    const m = new Map<string, [number, number]>();
    const R = Math.max(6, all.length * 1.4);
    const n = all.length;
    all.forEach((node, k) => {
      // ground at the bottom, others clockwise around
      const a = Math.PI / 2 + (2 * Math.PI * k) / n;
      m.set(node, [R + R * Math.cos(a), R + R * Math.sin(a)]);
    });
    return { m, R };
  }, [all]);
  const edges = useMemo(() => {
    const seen = new Map<string, number>();
    return elements.map((e) => {
      const [a, b] = e.kind === 'nmos' || e.kind === 'pmos' ? [e.nodes[0], e.nodes[2]] : [e.nodes[0], e.nodes[1]];
      const key = [a, b].sort().join('|');
      const k = seen.get(key) ?? 0;
      seen.set(key, k + 1);
      const pa = pos.m.get(a) ?? [0, 0];
      const pb = pos.m.get(b) ?? [0, 0];
      // bend relative to a canonical direction so parallel elements fan out
      const [qa, qb] = a < b ? [pa, pb] : [pb, pa];
      const mx = (pa[0] + pb[0]) / 2;
      const my = (pa[1] + pb[1]) / 2;
      const dx = qb[0] - qa[0];
      const dy = qb[1] - qa[1];
      const L = Math.hypot(dx, dy) || 1;
      const bend = (k % 2 === 0 ? 1 : -1) * Math.ceil(k / 2) * 5;
      const c: [number, number] = [mx - (dy / L) * bend, my + (dx / L) * bend];
      // sample the quadratic curve as a polyline for the dot animation
      const pts: [number, number][] = [];
      for (let s = 0; s <= 16; s++) {
        const t = s / 16;
        pts.push([(1 - t) * (1 - t) * pa[0] + 2 * (1 - t) * t * c[0] + t * t * pb[0], (1 - t) * (1 - t) * pa[1] + 2 * (1 - t) * t * c[1] + t * t * pb[1]]);
      }
      const mid = pts[8];
      return { e, pts, mid, geom: geom(pts) };
    });
  }, [elements, pos]);
  const size = pos.R * 2;
  const pad = 3;
  const geoms = useMemo(() => edges.map((x) => x.geom), [edges]);
  const currents = edges.map(({ e }) => snap.i(e.name));
  return (
    <svg className="cl-schem" viewBox={`${-pad} ${-pad} ${size + 2 * pad} ${size + 2 * pad}`} style={{ maxHeight: 520 }} role="img" aria-label="node graph">
      {edges.map(({ e, pts }) => (
        <polyline key={e.name} points={pts.map((p) => p.join(',')).join(' ')} fill="none" stroke="var(--border-strong)" strokeWidth={0.18} />
      ))}
      <FlowLayer paths={geoms} currents={currents} imax={imax} active={animate} />
      {edges.map(({ e, mid }) => (
        <g key={`m${e.name}`} className={`cl-part ${hovered === e.name ? 'hover' : ''}`} onMouseEnter={() => onHover?.(e.name)} onMouseLeave={() => onHover?.(null)}>
          <rect x={mid[0] - 1.5} y={mid[1] - 0.55} width={3} height={1.1} rx={0.5} className="cl-pill" />
          <text x={mid[0]} y={mid[1] + 0.22} textAnchor="middle" className="cl-plabel"><tspan className="cl-pname">{e.name}</tspan></text>
        </g>
      ))}
      {all.map((n) => {
        const p = pos.m.get(n)!;
        return (
          <g key={n}>
            <circle cx={p[0]} cy={p[1]} r={0.75} fill={rampColor(snap.v(n), vrange[0], vrange[1])} stroke="var(--surface-1)" strokeWidth={0.15} />
            <text x={p[0]} y={p[1] - 1.1} textAnchor="middle" className="cl-plabel">
              <tspan className="cl-pname">{n === '0' ? 'gnd' : n}</tspan>
              <tspan className="cl-lval" dx={0.3}>{si(snap.v(n), 'V', 3)}</tspan>
            </text>
          </g>
        );
      })}
    </svg>
  );
}
