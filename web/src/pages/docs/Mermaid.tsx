// A small, dependency-free renderer for Mermaid `flowchart` / `graph`
// diagrams (the subset the docs use: nodes with labels and shapes, edges with
// optional labels, subgraphs, LR/RL/TD/TB directions), so the docs read well on
// GitHub Pages without the Mermaid library. Layout: layers by longest path
// (back edges of cycles ignored), sources pulled next to their first target,
// nodes of a subgraph kept together in each layer. Anything else (sequence,
// class, state diagrams …) returns null and the caller shows the source.
import type { ReactNode } from 'react';

interface MNode { id: string; label: string[]; shape: 'rect' | 'round' | 'stadium' | 'circle' | 'diamond' | 'db'; group: number | null; order: number }
interface MEdge { from: string; to: string; label: string; style: 'solid' | 'dotted' | 'thick'; arrow: boolean }
interface MGroup { id: string; title: string }
export interface Flow { dir: 'LR' | 'RL' | 'TD' | 'TB' | 'BT'; nodes: Map<string, MNode>; edges: MEdge[]; groups: MGroup[] }

const clean = (s: string) =>
  s
    .trim()
    .replace(/^["'`]|["'`]$/g, '')
    .replace(/#quot;/g, '"')
    .replace(/#35;/g, '#')
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>');

const SHAPES: [RegExp, MNode['shape']][] = [
  [/^\(\((.*)\)\)$/, 'circle'],
  [/^\(\[(.*)\]\)$/, 'stadium'],
  [/^\[\((.*)\)\]$/, 'db'],
  [/^\[(.*)\]$/, 'rect'],
  [/^\((.*)\)$/, 'round'],
  [/^\{(.*)\}$/, 'diamond'],
];

const NODE = /^([A-Za-z0-9_À-￿-]+)\s*(\(\(.*?\)\)|\(\[.*?\]\)|\[\(.*?\)\]|\[.*?\]|\(.*?\)|\{.*?\})?/;
const ARROW = /^\s*(?:(-{2,3}|={2,3}|-\.+-?)\s*(?:"([^"]*)"|([^-=>|"]+?))\s*)?(-->|==>|-\.->|---|===|-\.-|--o|--x)\s*(?:\|([^|]*)\|)?\s*/;

/** Parse a Mermaid flowchart; null when the source is not one we can draw. */
export function parseFlow(src: string): Flow | null {
  const lines = src.split('\n').map((l) => l.replace(/%%.*$/, '').trim()).filter(Boolean);
  const head = /^(flowchart|graph)\s*(LR|RL|TD|TB|BT)?\s*;?$/i.exec(lines[0] ?? '');
  if (!head) return null;
  const flow: Flow = { dir: ((head[2] ?? 'TD').toUpperCase() as Flow['dir']), nodes: new Map(), edges: [], groups: [] };
  const stack: number[] = [];
  let order = 0;
  const node = (tok: string): string | null => {
    const m = NODE.exec(tok.trim());
    if (!m) return null;
    const id = m[1];
    let n = flow.nodes.get(id);
    if (!n) {
      n = { id, label: [id], shape: 'rect', group: stack.length ? stack[stack.length - 1] : null, order: order++ };
      flow.nodes.set(id, n);
    }
    if (m[2]) {
      for (const [re, shape] of SHAPES) {
        const s = re.exec(m[2]);
        if (s) {
          n.shape = shape;
          n.label = clean(s[1]).split(/<br\s*\/?>/i).map(clean);
          break;
        }
      }
    }
    return id;
  };
  for (const raw of lines.slice(1)) {
    const l = raw.replace(/;$/, '');
    const sg = /^subgraph\s+([^\s[]+)\s*(?:\[(.*)\])?\s*$/.exec(l) ?? /^subgraph\s+(.+)$/.exec(l);
    if (sg) {
      flow.groups.push({ id: sg[1], title: clean(sg[2] ?? sg[1]) });
      stack.push(flow.groups.length - 1);
      continue;
    }
    if (l === 'end') {
      stack.pop();
      continue;
    }
    if (/^(classDef|class|style|linkStyle|click|direction)\b/.test(l)) continue;
    // chain: A --> B -- text --> C
    let rest = l;
    const first = NODE.exec(rest);
    if (!first) return null;
    let prev = node(first[0]);
    rest = rest.slice(first[0].length);
    while (rest.trim().length) {
      const a = ARROW.exec(rest);
      if (!a) {
        const amp = /^\s*&\s*/.exec(rest);
        if (amp) return null; // "A & B --> C" is not supported
        break;
      }
      rest = rest.slice(a[0].length);
      const t = NODE.exec(rest);
      if (!t) return null;
      const next = node(t[0]);
      rest = rest.slice(t[0].length);
      const op = a[4];
      if (prev && next) {
        flow.edges.push({
          from: prev,
          to: next,
          label: clean(a[5] ?? a[2] ?? a[3] ?? ''),
          style: op.includes('.') ? 'dotted' : op.includes('=') ? 'thick' : 'solid',
          arrow: op.endsWith('>') || op.endsWith('o') || op.endsWith('x'),
        });
      }
      prev = next;
    }
  }
  return flow.nodes.size >= 2 ? flow : null;
}

const CH = 6.7; // average character width at 12 px
/** SVG path through points with rounded corners (orthogonal routing). */
function ortho(pts: [number, number][], r = 8): string {
  let d = `M${pts[0][0]} ${pts[0][1]}`;
  for (let i = 1; i < pts.length - 1; i++) {
    const [px, py] = pts[i - 1];
    const [x, y] = pts[i];
    const [nx, ny] = pts[i + 1];
    const d1 = Math.hypot(x - px, y - py);
    const d2 = Math.hypot(nx - x, ny - y);
    const k = Math.min(r, d1 / 2, d2 / 2);
    if (k < 0.5) {
      d += ` L${x} ${y}`;
      continue;
    }
    d += ` L${x - ((x - px) / d1) * k} ${y - ((y - py) / d1) * k} Q${x} ${y} ${x + ((nx - x) / d2) * k} ${y + ((ny - y) / d2) * k}`;
  }
  const [lx, ly] = pts[pts.length - 1];
  return `${d} L${lx} ${ly}`;
}
const LINE = 15;

/** Widest left-to-right diagram drawn as is; wider ones are turned top-down to stay legible in the doc column. */
const MAX_W = 940;

export function FlowDiagram({ flow, rotated = false }: { flow: Flow; rotated?: boolean }) {
  const nodes = [...flow.nodes.values()];
  const horizontal = flow.dir === 'LR' || flow.dir === 'RL';
  // --- layering (longest path on the DAG without back edges) ---
  const out = new Map<string, string[]>();
  nodes.forEach((n) => out.set(n.id, []));
  const back = new Set<number>();
  const state = new Map<string, number>();
  const dfs = (id: string) => {
    state.set(id, 1);
    flow.edges.forEach((e, i) => {
      if (e.from !== id) return;
      const s = state.get(e.to) ?? 0;
      if (s === 1) back.add(i);
      else if (s === 0) dfs(e.to);
    });
    state.set(id, 2);
  };
  nodes.forEach((n) => !state.get(n.id) && dfs(n.id));
  const fwd = flow.edges.filter((_, i) => !back.has(i));
  fwd.forEach((e) => out.get(e.from)!.push(e.to));
  const layer = new Map<string, number>();
  const depth = (id: string, seen = new Set<string>()): number => {
    if (layer.has(id)) return layer.get(id)!;
    if (seen.has(id)) return 0;
    seen.add(id);
    const ins = fwd.filter((e) => e.to === id).map((e) => depth(e.from, seen) + 1);
    const d = ins.length ? Math.max(...ins) : 0;
    layer.set(id, d);
    return d;
  };
  nodes.forEach((n) => depth(n.id));
  // pull pure sources next to their nearest target
  nodes.forEach((n) => {
    const hasIn = fwd.some((e) => e.to === n.id);
    const tgts = out.get(n.id)!;
    if (!hasIn && tgts.length) layer.set(n.id, Math.max(0, Math.min(...tgts.map((t) => layer.get(t)!)) - 1));
  });
  const L = Math.max(...layer.values()) + 1;
  const cols: MNode[][] = Array.from({ length: L }, () => []);
  nodes.forEach((n) => cols[layer.get(n.id)!].push(n));
  cols.forEach((c) => c.sort((a, b) => (a.group ?? 99) - (b.group ?? 99) || a.order - b.order));
  // --- sizes and positions ---
  const size = (n: MNode) => {
    const w = Math.max(...n.label.map((s) => s.length)) * CH + 26;
    const h = n.label.length * LINE + 14;
    return n.shape === 'circle' ? { w: Math.max(w, h), h: Math.max(w, h) } : n.shape === 'diamond' ? { w: w + 24, h: h + 18 } : { w, h };
  };
  const GAP_CROSS = 22;
  const TOP = 30;
  const lw = (e: MEdge) => (e.label ? Math.max(...e.label.split(/<br\s*\/?>/i).map((l) => l.length)) * 6 + 22 : 0);
  // gap after each layer: wide enough for the labels of edges ending in the next layer
  const gaps = cols.map((_, i) => {
    const ls = flow.edges.filter((e) => !back.has(flow.edges.indexOf(e)) && layer.get(e.to) === i + 1 && layer.get(e.from)! <= i).map(lw);
    const starts = !horizontal && cols[i + 1]?.some((n) => n.group !== null && Math.min(...nodes.filter((m) => m.group === n.group).map((m) => layer.get(m.id)!)) === i + 1);
    return Math.max(horizontal ? 56 : 44, ...ls.map((w) => (horizontal ? w + 16 : 34))) + (starts ? 30 : 0);
  });
  const sz = new Map(nodes.map((n) => [n.id, size(n)]));
  const along = (n: MNode) => (horizontal ? sz.get(n.id)!.w : sz.get(n.id)!.h);
  const across = (n: MNode) => (horizontal ? sz.get(n.id)!.h : sz.get(n.id)!.w);
  // main-axis offset of each layer
  const thick = cols.map((c) => Math.max(0, ...c.map(along)));
  const mainAt: number[] = [];
  let main = 12 + (!horizontal && cols[0].some((n) => n.group !== null) ? 26 : 0);
  cols.forEach((_, i) => {
    mainAt.push(main);
    main += thick[i] + gaps[i];
  });
  main -= gaps[gaps.length - 1];
  if (horizontal && main + 24 > MAX_W) return <FlowDiagram flow={{ ...flow, dir: flow.dir === 'RL' ? 'BT' : 'TD' }} rotated />;
  // bands: one per subgraph, and one per layer for ungrouped nodes; bands whose
  // layer ranges do not overlap share a track on the cross axis
  interface Band { key: string; members: MNode[]; lo: number; hi: number; size: number; track: number; title: number | null }
  const bands: Band[] = [];
  flow.groups.forEach((_, gi) => {
    const ms = nodes.filter((n) => n.group === gi);
    if (ms.length) bands.push({ key: `g${gi}`, members: ms, lo: 0, hi: 0, size: 0, track: 0, title: gi });
  });
  cols.forEach((c, i) => {
    const ms = c.filter((n) => n.group === null);
    if (ms.length) bands.push({ key: `u${i}`, members: ms, lo: i, hi: i, size: 0, track: 0, title: null });
  });
  bands.forEach((bd) => {
    const ls = bd.members.map((n) => layer.get(n.id)!);
    bd.lo = Math.min(...ls);
    bd.hi = Math.max(...ls);
    bd.size = Math.max(
      ...cols.map((c) => {
        const ms = c.filter((n) => bd.members.includes(n));
        return ms.length ? ms.reduce((t, n) => t + across(n), 0) + GAP_CROSS * (ms.length - 1) : 0;
      }),
    ) + (horizontal && bd.title !== null ? 26 : 0);
  });
  const tracks: Band[][] = [];
  bands.forEach((bd) => {
    let t = tracks.findIndex((tr) => tr.every((o) => bd.hi < o.lo || bd.lo > o.hi));
    if (t < 0) {
      tracks.push([]);
      t = tracks.length - 1;
    }
    tracks[t].push(bd);
    bd.track = t;
  });
  const trackSize = tracks.map((tr) => Math.max(...tr.map((bd) => bd.size)));
  const trackAt: number[] = [];
  let cross = horizontal ? TOP - 26 : 16;
  trackSize.forEach((ts) => {
    trackAt.push(cross);
    cross += ts + 34;
  });
  const maxCross = cross - 34 + 8;
  const pos = new Map<string, { x: number; y: number; w: number; h: number }>();
  bands.forEach((bd) => {
    const tAllow = horizontal && bd.title !== null ? 26 : 0;
    const off0 = trackAt[bd.track] + tAllow;
    const inner = trackSize[bd.track] - tAllow;
    cols.forEach((c, i) => {
      const ms = c.filter((n) => bd.members.includes(n));
      if (!ms.length) return;
      const len = ms.reduce((t, n) => t + across(n), 0) + GAP_CROSS * (ms.length - 1);
      let at = off0 + (inner - len) / 2;
      ms.forEach((n) => {
        const { w, h } = sz.get(n.id)!;
        const m = mainAt[i] + (thick[i] - along(n)) / 2;
        pos.set(n.id, horizontal ? { x: m, y: at, w, h } : { x: at, y: m, w, h });
        at += across(n) + GAP_CROSS;
      });
    });
  });
  if (flow.dir === 'RL' || flow.dir === 'BT') {
    pos.forEach((p) => {
      if (horizontal) p.x = main - p.x - p.w;
      else p.y = main - p.y - p.h;
    });
  }
  const W = horizontal ? main + 24 : maxCross + 30;
  const H = horizontal ? maxCross + 30 : main + 24;
  // --- subgraph boxes ---
  const boxes = flow.groups
    .map((g, gi) => {
      const ms = nodes.filter((n) => n.group === gi).map((n) => pos.get(n.id)!);
      if (!ms.length) return null;
      const x0 = Math.min(...ms.map((p) => p.x)) - 12;
      const y0 = Math.min(...ms.map((p) => p.y)) - 24;
      const x1 = Math.max(...ms.map((p) => p.x + p.w)) + 12;
      const y1 = Math.max(...ms.map((p) => p.y + p.h)) + 10;
      return { g, gi, x0, y0, x1, y1 };
    })
    .filter(Boolean) as { g: MGroup; gi: number; x0: number; y0: number; x1: number; y1: number }[];
  // --- edges ---
  const port = (id: string, side: 'out' | 'in') => {
    const p = pos.get(id)!;
    if (horizontal) {
      const rightward = flow.dir === 'LR';
      const right = (side === 'out') === rightward;
      return { x: right ? p.x + p.w : p.x, y: p.y + p.h / 2 };
    }
    const down = flow.dir !== 'BT';
    const bottom = (side === 'out') === down;
    return { x: p.x + p.w / 2, y: bottom ? p.y + p.h : p.y };
  };
  let extX = 0;
  let extY = 0;
  const edgeEls: ReactNode[] = [];
  const labelEls: ReactNode[] = [];
  flow.edges.forEach((e, i) => {
    let d: string;
    let mx: number;
    let my: number;
    const span = layer.get(e.to)! - layer.get(e.from)!;
    if (back.has(i) || span <= 0) {
      // back edge: orthogonal route outside everything in the layers it spans
      const a = pos.get(e.from)!;
      const b = pos.get(e.to)!;
      const l0 = Math.min(layer.get(e.from)!, layer.get(e.to)!);
      const l1 = Math.max(layer.get(e.from)!, layer.get(e.to)!);
      const spanned = nodes.filter((n) => layer.get(n.id)! >= l0 && layer.get(n.id)! <= l1).map((n) => pos.get(n.id)!);
      const grp = boxes.filter((bx) => spanned.some((p) => p.x >= bx.x0 && p.x <= bx.x1 && p.y >= bx.y0 && p.y <= bx.y1));
      if (horizontal) {
        const yb = Math.max(...spanned.map((p) => p.y + p.h), ...grp.map((g) => g.y1)) + 16;
        const [ax, bx] = [a.x + a.w / 2, b.x + b.w / 2];
        d = ortho([[ax, a.y + a.h], [ax, yb], [bx, yb], [bx, b.y + b.h]]);
        mx = (ax + bx) / 2;
        my = yb;
        extY = Math.max(extY, yb + 16);
      } else {
        const xb = Math.max(...spanned.map((p) => p.x + p.w), ...grp.map((g) => g.x1)) + 16;
        const [ay, by] = [a.y + a.h / 2, b.y + b.h / 2];
        d = ortho([[a.x + a.w, ay], [xb, ay], [xb, by], [b.x + b.w, by]]);
        mx = xb + lw(e) / 2 + 4;
        my = (ay + by) / 2;
        extX = Math.max(extX, xb + lw(e) + 12);
      }
    } else if (span > 1) {
      // skip edge: orthogonal detour beside the intermediate layers it jumps
      const s = port(e.from, 'out');
      const t = port(e.to, 'in');
      const mid = nodes.filter((n) => layer.get(n.id)! > layer.get(e.from)! && layer.get(n.id)! < layer.get(e.to)!).map((n) => pos.get(n.id)!);
      const g1 = gaps[layer.get(e.from)!] / 2;
      const g2 = gaps[layer.get(e.to)! - 1] / 2;
      const dirv = flow.dir === 'LR' || flow.dir === 'TD' || flow.dir === 'TB' ? 1 : -1;
      if (horizontal) {
        const lo = Math.min(s.y, t.y);
        const over = mid.filter((p) => p.y < lo + 20 && p.y + p.h > lo - 200);
        const yr = Math.min(lo, ...over.map((p) => p.y)) - 14;
        d = ortho([[s.x, s.y], [s.x + dirv * g1, s.y], [s.x + dirv * g1, yr], [t.x - dirv * g2, yr], [t.x - dirv * g2, t.y], [t.x, t.y]]);
        mx = t.x - dirv * g2 - dirv * 40;
        my = yr;
      } else {
        const hi = Math.max(s.x, t.x);
        const over = mid.filter((p) => p.x < hi + 20 && p.x + p.w > Math.min(s.x, t.x) - 20);
        const xr = Math.max(hi, ...over.map((p) => p.x + p.w)) + 14;
        d = ortho([[s.x, s.y], [s.x, s.y + dirv * g1], [xr, s.y + dirv * g1], [xr, t.y - dirv * g2], [t.x, t.y - dirv * g2], [t.x, t.y]]);
        mx = xr + lw(e) / 2 + 4;
        my = (s.y + t.y) / 2;
        extX = Math.max(extX, xr + lw(e) + 12);
      }
    } else {
      const s = port(e.from, 'out');
      const t = port(e.to, 'in');
      if (horizontal) {
        const k = (t.x - s.x) / 2;
        d = `M${s.x} ${s.y} C${s.x + k} ${s.y}, ${t.x - k} ${t.y}, ${t.x} ${t.y}`;
      } else {
        const k = (t.y - s.y) / 2;
        d = `M${s.x} ${s.y} C${s.x} ${s.y + k}, ${t.x} ${t.y - k}, ${t.x} ${t.y}`;
      }
      mx = (s.x + t.x) / 2;
      my = (s.y + t.y) / 2;
    }
    edgeEls.push(
      <path key={`e${i}`} d={d} fill="none" stroke="var(--muted)" strokeWidth={e.style === 'thick' ? 2.6 : 1.4} strokeDasharray={e.style === 'dotted' ? '4 4' : undefined} markerEnd={e.arrow ? 'url(#md-arrow)' : undefined} />,
    );
    if (e.label) {
      const lines = e.label.split(/<br\s*\/?>/i);
      const w = Math.max(...lines.map((l) => l.length)) * 6 + 10;
      const h = lines.length * 13 + 6;
      labelEls.push(
        <g key={`l${i}`}>
          <rect x={mx - w / 2} y={my - h / 2} width={w} height={h} rx={5} fill="var(--surface-1)" stroke="var(--border)" />
          {lines.map((l, j) => <text key={j} x={mx} y={my - h / 2 + 14 + j * 13} textAnchor="middle" fontSize={10.5} fill="var(--text-2)">{l}</text>)}
        </g>,
      );
    }
  });
  return (
    <svg viewBox={`0 0 ${Math.max(W, extX)} ${Math.max(H, extY)}`} data-rotated={rotated || undefined} className="md-flow" role="img" aria-label="Diagram" style={{ width: '100%', maxWidth: Math.max(W, extX), minWidth: Math.round(Math.max(W, extX) * 0.72), height: 'auto', display: 'block' }}>
      <defs>
        <marker id="md-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
          <path d="M0 0 L10 5 L0 10 Z" fill="var(--muted)" />
        </marker>
      </defs>
      {boxes.map((b) => (
        <g key={b.g.id}>
          <rect x={b.x0} y={b.y0} width={b.x1 - b.x0} height={b.y1 - b.y0} rx={12} fill={`color-mix(in srgb, var(--series-${(b.gi % 8) + 1}) 7%, transparent)`} stroke={`var(--series-${(b.gi % 8) + 1})`} strokeDasharray="5 4" />
          <text x={b.x0 + 10} y={b.y0 + 15} fontSize={11.5} fontWeight={700} fill="var(--text-2)">{b.g.title}</text>
        </g>
      ))}
      {edgeEls}
      {nodes.map((n) => {
        const p = pos.get(n.id)!;
        const cx = p.x + p.w / 2;
        const cy = p.y + p.h / 2;
        const common = { fill: 'var(--surface-2)', stroke: 'var(--border-strong)', strokeWidth: 1.3 };
        let shape: ReactNode;
        if (n.shape === 'circle') shape = <circle cx={cx} cy={cy} r={p.w / 2} {...common} />;
        else if (n.shape === 'diamond') shape = <path d={`M${cx} ${p.y} L${p.x + p.w} ${cy} L${cx} ${p.y + p.h} L${p.x} ${cy} Z`} {...common} />;
        else shape = <rect x={p.x} y={p.y} width={p.w} height={p.h} rx={n.shape === 'rect' ? 6 : n.shape === 'round' ? 12 : p.h / 2} {...common} />;
        return (
          <g key={n.id}>
            {shape}
            {n.label.map((l, j) => (
              <text key={j} x={cx} y={cy - ((n.label.length - 1) * LINE) / 2 + j * LINE + 4} textAnchor="middle" fontSize={12} fill={j === 0 ? 'var(--text)' : 'var(--text-2)'} fontWeight={j === 0 ? 600 : 400}>{l}</text>
            ))}
          </g>
        );
      })}
      {labelEls}
    </svg>
  );
}
