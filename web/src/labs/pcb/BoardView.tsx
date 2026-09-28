// Interactive SVG board view: layers, zoom/pan, net highlighting, DRC markers.
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import type { BoardPad, Layout, PartPos } from './types';
import { useElementWidth } from '../../components/useElementWidth';

export type LayerKey = 'fcu' | 'bcu' | 'pour' | 'mask' | 'silk' | 'court' | 'rats' | 'keep' | 'drc';

export const LAYERS: { key: LayerKey; label: string; color: string }[] = [
  { key: 'fcu', label: 'F.Cu', color: 'var(--copper)' },
  { key: 'bcu', label: 'B.Cu', color: 'var(--series-1)' },
  { key: 'pour', label: 'GND pours', color: 'color-mix(in srgb, var(--copper) 45%, transparent)' },
  { key: 'mask', label: 'Mask', color: 'var(--mask)' },
  { key: 'silk', label: 'Silkscreen', color: '#f2f1ea' },
  { key: 'court', label: 'Courtyards', color: 'var(--muted)' },
  { key: 'rats', label: 'Ratsnest', color: 'var(--series-4)' },
  { key: 'keep', label: 'Keep-outs', color: 'var(--series-5)' },
  { key: 'drc', label: 'DRC', color: 'var(--bad)' },
];

export type LayerState = Record<LayerKey, boolean>;
export const DEFAULT_LAYERS: LayerState = { fcu: true, bcu: true, pour: true, mask: true, silk: true, court: false, rats: false, keep: true, drc: true };

/** Minimum-spanning-tree ratsnest lines for every non-ground net. */
export function ratsnest(pads: BoardPad[]): [number, number, number, number, string][] {
  const byNet = new Map<string, BoardPad[]>();
  for (const p of pads) {
    if (!p.net || p.net === 'GND') continue;
    if (!byNet.has(p.net)) byNet.set(p.net, []);
    byNet.get(p.net)!.push(p);
  }
  const out: [number, number, number, number, string][] = [];
  for (const [net, ps] of byNet) {
    if (ps.length < 2) continue;
    const inTree = [0];
    const rest = new Set(ps.map((_, i) => i).slice(1));
    while (rest.size) {
      let best = [0, 0, Infinity];
      for (const a of inTree) for (const b of rest) {
        const d = Math.abs(ps[a].x - ps[b].x) + Math.abs(ps[a].y - ps[b].y);
        if (d < best[2]) best = [a, b, d];
      }
      out.push([ps[best[0]].x, ps[best[0]].y, ps[best[1]].x, ps[best[1]].y, net]);
      inTree.push(best[1]);
      rest.delete(best[1]);
    }
  }
  return out;
}

export function BoardView(props: {
  layout: Layout;
  layers: LayerState;
  height?: number;
  /** Override part positions (annealing animation): parts are drawn as courtyards only. */
  ghostParts?: PartPos[];
  hideRouting?: boolean;
  focus?: { x: number; y: number } | null;
  selectedNet?: string | null;
  onSelectNet?: (net: string | null) => void;
  onPickViolation?: (i: number) => void;
  overlay?: ReactNode;
}) {
  const L = props.layout;
  const W = L.board.width;
  const H = L.board.height;
  const Y = (y: number) => H - y;
  const margin = 2;
  const full = useMemo(() => [-margin, -margin, W + 2 * margin, H + 2 * margin], [W, H]);
  const [vb, setVb] = useState<number[]>(full);
  const [hoverNet, setHoverNet] = useState<string | null>(null);
  const [hoverInfo, setHoverInfo] = useState<string>('');
  const svgRef = useRef<SVGSVGElement>(null);
  const [wrapRef, wrapW] = useElementWidth<HTMLDivElement>();
  const autoH = Math.max(Math.min(props.height ?? 160, Math.round(wrapW * 0.4)), Math.round(wrapW * (full[3] / full[2])));
  const drag = useRef<{ x: number; y: number; vb: number[] } | null>(null);

  useEffect(() => setVb(full), [full]);
  useEffect(() => {
    if (!props.focus) return;
    const w = 12;
    const h = (w * full[3]) / full[2];
    setVb([props.focus.x - w / 2, Y(props.focus.y) - h / 2, w, h]);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.focus]);

  const toSvg = useCallback((cx: number, cy: number) => {
    const r = svgRef.current!.getBoundingClientRect();
    return [vb[0] + ((cx - r.left) / r.width) * vb[2], vb[1] + ((cy - r.top) / r.height) * vb[3]];
  }, [vb]);

  useEffect(() => {
    const el = svgRef.current;
    if (!el) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const [mx, my] = toSvg(e.clientX, e.clientY);
      const k = Math.exp(e.deltaY * 0.0015);
      setVb((v) => {
        const w = Math.min(Math.max(v[2] * k, 2), full[2] * 1.5);
        const h = (w * v[3]) / v[2];
        return [mx - ((mx - v[0]) * w) / v[2], my - ((my - v[1]) * h) / v[3], w, h];
      });
    };
    el.addEventListener('wheel', onWheel, { passive: false });
    return () => el.removeEventListener('wheel', onWheel);
  }, [toSvg, full]);

  const net = hoverNet ?? props.selectedNet ?? null;
  const dim = (n: string) => (net && n !== net ? 0.22 : 1);
  // pads moved with the ghost parts (annealing animation) so the ratsnest follows
  const movedPads = useMemo(() => {
    if (!props.ghostParts) return L.pads;
    const gp = new Map(props.ghostParts.map((g) => [g.refdes, g]));
    return L.pads.map((p) => {
      const g = gp.get(p.refdes);
      const base = L.parts.find((q) => q.refdes === p.refdes);
      if (!g || !base) return p;
      let dx = p.x - base.x, dy = p.y - base.y;
      const turns = ((Math.round((g.rot - base.rot) / 90) % 4) + 4) % 4;
      for (let k = 0; k < turns; k++) [dx, dy] = [-dy, dx];
      return { ...p, x: g.x + dx, y: g.y + dy };
    });
  }, [L.pads, L.parts, props.ghostParts]);
  const rats = useMemo(() => ratsnest(movedPads), [movedPads]);
  const ly = props.layers;
  const stroke = Math.max(vb[2] / 900, 0.02);

  const zoom = (k: number) => setVb((v) => {
    const cx = v[0] + v[2] / 2, cy = v[1] + v[3] / 2;
    const w = Math.min(Math.max(v[2] * k, 2), full[2] * 1.5), h = (w * v[3]) / v[2];
    return [cx - w / 2, cy - h / 2, w, h];
  });

  const padEl = (p: BoardPad, i: number, fill: string) => {
    const common = {
      key: `${p.refdes}.${p.pin}.${i}`,
      fill,
      opacity: dim(p.net),
      onMouseEnter: () => { setHoverNet(p.net || null); setHoverInfo(`${p.refdes}.${p.pin}${p.net ? ` · ${p.net}` : ' · no connection'}`); },
      onMouseLeave: () => { setHoverNet(null); setHoverInfo(''); },
      onClick: () => props.onSelectNet?.(p.net || null),
      style: { cursor: 'pointer' },
    };
    return p.shape === 'circle'
      ? <circle {...common} cx={p.x} cy={Y(p.y)} r={p.w / 2} />
      : <rect {...common} x={p.x - p.w / 2} y={Y(p.y) - p.h / 2} width={p.w} height={p.h} rx={p.shape === 'round_rect' ? Math.min(p.w, p.h) * 0.2 : 0} />;
  };

  return (
    <div style={{ position: 'relative' }} ref={wrapRef}>
      <div className="row" style={{ position: 'absolute', right: 8, top: 8, zIndex: 2, gap: 6 }}>
        <button className="btn small" onClick={() => zoom(0.7)} aria-label="Zoom in">+</button>
        <button className="btn small" onClick={() => zoom(1.4)} aria-label="Zoom out">−</button>
        <button className="btn small" onClick={() => setVb(full)}>Fit</button>
      </div>
      {hoverInfo && <div className="plot-tooltip" style={{ left: 8, top: 8 }}>{hoverInfo}</div>}
      <svg
        ref={svgRef}
        viewBox={vb.join(' ')}
        width="100%"
        height={autoH}
        preserveAspectRatio="xMidYMid meet"
        style={{ display: 'block', background: 'var(--surface-2)', borderRadius: 12, border: '1px solid var(--border)', touchAction: 'none', cursor: drag.current ? 'grabbing' : 'grab' }}
        onPointerDown={(e) => { drag.current = { x: e.clientX, y: e.clientY, vb }; (e.target as Element).setPointerCapture?.(e.pointerId); }}
        onPointerMove={(e) => {
          if (!drag.current) return;
          const r = svgRef.current!.getBoundingClientRect();
          const d = drag.current;
          setVb([d.vb[0] - ((e.clientX - d.x) / r.width) * d.vb[2], d.vb[1] - ((e.clientY - d.y) / r.height) * d.vb[3], d.vb[2], d.vb[3]]);
        }}
        onPointerUp={() => { drag.current = null; }}
        role="img"
        aria-label="PCB layout"
      >
        <defs>
          <pattern id="keep-hatch" width="0.6" height="0.6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
            <line x1="0" y1="0" x2="0" y2="0.6" stroke="var(--series-5)" strokeWidth="0.12" opacity="0.7" />
          </pattern>
        </defs>
        {/* substrate */}
        <rect x={0} y={0} width={W} height={H} rx={0.3} fill={ly.mask ? 'var(--mask)' : '#6b6a5e'} stroke="var(--border-strong)" strokeWidth={stroke * 2} />
        {/* bottom layer first (seen through the board) */}
        {ly.pour && L.pour.rects.map((r, i) => <rect key={`pb${i}`} x={r[0]} y={Y(r[3])} width={r[2] - r[0]} height={r[3] - r[1]} fill="var(--series-1)" opacity={net && net !== 'GND' ? 0.05 : 0.16} />)}
        {ly.bcu && L.tracks.filter((t) => t.layer === 1).map((t, i) => (
          <line key={`tb${i}`} x1={t.x0} y1={Y(t.y0)} x2={t.x1} y2={Y(t.y1)} stroke="var(--series-1)" strokeWidth={t.width} strokeLinecap="round" opacity={0.85 * dim(t.net)}
            onMouseEnter={() => { setHoverNet(t.net); setHoverInfo(`${t.net} · B.Cu`); }} onMouseLeave={() => { setHoverNet(null); setHoverInfo(''); }} onClick={() => props.onSelectNet?.(t.net)} />
        ))}
        {ly.bcu && L.pads.filter((p) => p.bottom && !p.top).map((p, i) => padEl(p, i, 'var(--series-1)'))}
        {/* top layer */}
        {ly.pour && L.pour_top.rects.map((r, i) => <rect key={`pt${i}`} x={r[0]} y={Y(r[3])} width={r[2] - r[0]} height={r[3] - r[1]} fill="var(--copper)" opacity={net && net !== 'GND' ? 0.06 : 0.22} />)}
        {ly.fcu && !props.hideRouting && L.tracks.filter((t) => t.layer === 0).map((t, i) => (
          <line key={`tt${i}`} x1={t.x0} y1={Y(t.y0)} x2={t.x1} y2={Y(t.y1)} stroke="var(--copper)" strokeWidth={t.width} strokeLinecap="round" opacity={dim(t.net)}
            onMouseEnter={() => { setHoverNet(t.net); setHoverInfo(`${t.net} · F.Cu`); }} onMouseLeave={() => { setHoverNet(null); setHoverInfo(''); }} onClick={() => props.onSelectNet?.(t.net)} />
        ))}
        {ly.fcu && !props.ghostParts && L.pads.filter((p) => p.top).map((p, i) => padEl(p, i, 'var(--copper)'))}
        {(ly.fcu || ly.bcu) && !props.hideRouting && L.vias.map((v, i) => (
          <g key={`v${i}`} opacity={dim(v.net)} onMouseEnter={() => { setHoverNet(v.net); setHoverInfo(`via · ${v.net}`); }} onMouseLeave={() => { setHoverNet(null); setHoverInfo(''); }}>
            <circle cx={v.x} cy={Y(v.y)} r={v.pad / 2} fill="#c9c7bd" />
            <circle cx={v.x} cy={Y(v.y)} r={v.drill / 2} fill="var(--surface-1)" />
          </g>
        ))}
        {!props.ghostParts && L.pads.filter((p) => p.drill > 0).map((p, i) => <circle key={`d${i}`} cx={p.x} cy={Y(p.y)} r={p.drill / 2} fill="var(--surface-1)" />)}
        {L.holes.map((h, i) => <circle key={`h${i}`} cx={h.x} cy={Y(h.y)} r={h.d / 2} fill="var(--surface-1)" stroke="var(--muted)" strokeWidth={stroke} />)}
        {ly.keep && L.keepouts.map((k, i) => (
          <g key={`k${i}`}>
            <rect x={k.rect[0]} y={Y(k.rect[3])} width={k.rect[2] - k.rect[0]} height={k.rect[3] - k.rect[1]} fill="url(#keep-hatch)" stroke="var(--series-5)" strokeWidth={stroke * 1.5} strokeDasharray={`${stroke * 6} ${stroke * 4}`} />
          </g>
        ))}
        {ly.silk && !props.ghostParts && (
          <g stroke="#f2f1ea" strokeWidth={0.15} strokeLinecap="round" fill="none" opacity={0.9} pointerEvents="none">
            {L.silk.lines.map((s, i) => <line key={`s${i}`} x1={s[0]} y1={Y(s[1])} x2={s[2]} y2={Y(s[3])} />)}
            {L.silk.dots.map((d, i) => <circle key={`sd${i}`} cx={d[0]} cy={Y(d[1])} r={d[2]} fill="#f2f1ea" stroke="none" />)}
            {L.silk.labels.map((l, i) => <text key={`sl${i}`} x={l.x} y={Y(l.y)} fontSize={l.h * 1.25} fill="#f2f1ea" stroke="none" fontFamily="var(--mono)" fontWeight={700}>{l.text}</text>)}
          </g>
        )}
        {(ly.court || props.ghostParts) && L.parts.filter((p) => !props.ghostParts?.some((g) => g.refdes === p.refdes)).map((p) => (
          <rect key={`c${p.refdes}`} x={p.courtyard[0]} y={Y(p.courtyard[3])} width={p.courtyard[2] - p.courtyard[0]} height={p.courtyard[3] - p.courtyard[1]} fill="none" stroke="var(--muted)" strokeWidth={stroke * 1.2} strokeDasharray={`${stroke * 4} ${stroke * 3}`} pointerEvents="none" />
        ))}
        {props.ghostParts && props.ghostParts.map((gp) => {
          const base = L.parts.find((p) => p.refdes === gp.refdes);
          if (!base) return null;
          const c = base.courtyard;
          // rotate the courtyard extents by the delta rotation
          const w0 = c[2] - c[0], h0 = c[3] - c[1];
          const swap = Math.round(((gp.rot - base.rot) % 180 + 180) % 180) === 90;
          const w = swap ? h0 : w0, h = swap ? w0 : h0;
          return (
            <g key={`g${gp.refdes}`}>
              <rect x={gp.x - w / 2} y={Y(gp.y) - h / 2} width={w} height={h} rx={0.2} fill="color-mix(in srgb, var(--accent) 30%, transparent)" stroke="var(--accent)" strokeWidth={stroke * 1.5} />
              <text x={gp.x} y={Y(gp.y) + 0.35} fontSize={1} textAnchor="middle" fill="var(--text)" fontFamily="var(--mono)">{gp.refdes}</text>
            </g>
          );
        })}
        {(ly.rats || props.ghostParts) && rats.map((r, i) => <line key={`r${i}`} x1={r[0]} y1={Y(r[1])} x2={r[2]} y2={Y(r[3])} stroke="var(--series-4)" strokeWidth={stroke * 1.4} strokeDasharray={`${stroke * 5} ${stroke * 4}`} opacity={dim(r[4])} pointerEvents="none" />)}
        {ly.drc && L.drc.map((v, i) => (
          <g key={`drc${i}`} onClick={() => props.onPickViolation?.(i)} style={{ cursor: 'pointer' }}>
            <circle cx={v.x} cy={Y(v.y)} r={0.8} fill="none" stroke="var(--bad)" strokeWidth={stroke * 3} />
            <circle cx={v.x} cy={Y(v.y)} r={0.2} fill="var(--bad)" />
          </g>
        ))}
        {props.overlay}
      </svg>
    </div>
  );
}
