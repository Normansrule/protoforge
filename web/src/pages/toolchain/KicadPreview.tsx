// Draws the exported .kicad_pcb after re-reading it with a small TypeScript
// S-expression parser — if the file were incomplete, this picture would be.
import { useEffect, useMemo, useRef, useState } from 'react';
import { Chip, Toggle } from '../../components/ui';
import { useReducedMotion } from '../home/motion';
import { readBoard, type KBoard } from './sexpr';
import type { KicadStats } from './types';

export function useBoard(content: string | undefined): { board: KBoard | null; error: string | null } {
  return useMemo(() => {
    if (!content) return { board: null, error: null };
    try {
      return { board: readBoard(content), error: null };
    } catch (e) {
      return { board: null, error: String(e) };
    }
  }, [content]);
}

export function KicadPreview({ board, stats }: { board: KBoard; stats: KicadStats }) {
  const [show, setShow] = useState({ front: true, back: true, zones: true, vias: true, refs: true });
  const [net, setNet] = useState<string | null>(null);
  const [zoom, setZoom] = useState(1);
  const reduced = useReducedMotion();
  const [drawn, setDrawn] = useState(board.segments.length);
  const raf = useRef(0);
  const replay = () => {
    cancelAnimationFrame(raf.current);
    if (reduced) {
      setDrawn(board.segments.length);
      return;
    }
    const t0 = performance.now();
    const n = board.segments.length;
    const tick = (now: number) => {
      const k = Math.min(1, (now - t0) / 2600);
      setDrawn(Math.round(n * k));
      if (k < 1) raf.current = requestAnimationFrame(tick);
    };
    raf.current = requestAnimationFrame(tick);
  };
  useEffect(() => () => cancelAnimationFrame(raf.current), []);
  useEffect(() => setDrawn(board.segments.length), [board]);

  const o = board.outline ?? [0, 0, 60, 10];
  const m = 1.5;
  const vb = [o[0] - m, o[1] - m, o[2] - o[0] + 2 * m, o[3] - o[1] + 2 * m];
  const netId = net ? [...board.nets.entries()].find(([, n]) => n === net)?.[0] : undefined;
  const padCount = board.footprints.reduce((s, f) => s + f.pads.length, 0);
  const checks: [string, number, number][] = [
    ['footprints', board.footprints.length, stats.footprints],
    ['pads', padCount, stats.pads],
    ['track segments', board.segments.length, stats.segments],
    ['vias', board.vias.length, stats.vias],
    ['nets', board.nets.size - 1, stats.nets],
    ['zones', board.zones.filter((z) => !z.keepout).length, stats.zones],
  ];
  const dimFor = (n: string | number) => (net === null ? 1 : (typeof n === 'number' ? n === netId : n === net) ? 1 : 0.18);
  const nets = useMemo(() => [...new Set(board.footprints.flatMap((f) => f.pads.map((p) => p.net)).filter(Boolean))].sort(), [board]);

  return (
    <div className="stack" style={{ gap: 10 }}>
      <div className="row" style={{ gap: 14 }}>
        <Toggle label="Top copper" value={show.front} onChange={(v) => setShow({ ...show, front: v })} />
        <Toggle label="Bottom copper" value={show.back} onChange={(v) => setShow({ ...show, back: v })} />
        <Toggle label="Zones & keep-outs" value={show.zones} onChange={(v) => setShow({ ...show, zones: v })} />
        <Toggle label="Vias" value={show.vias} onChange={(v) => setShow({ ...show, vias: v })} />
        <Toggle label="References" value={show.refs} onChange={(v) => setShow({ ...show, refs: v })} />
      </div>
      <div className="row">
        <label className="small muted" htmlFor="kc-net">Highlight net</label>
        <select id="kc-net" value={net ?? ''} onChange={(e) => setNet(e.target.value || null)} style={{ maxWidth: 180 }}>
          <option value="">all nets</option>
          {nets.map((n) => <option key={n} value={n}>{n}</option>)}
        </select>
        <label className="small muted" htmlFor="kc-zoom">Zoom</label>
        <input id="kc-zoom" type="range" min={1} max={4} step={0.25} value={zoom} onChange={(e) => setZoom(Number(e.target.value))} style={{ width: 110 }} />
        <button className="btn small" onClick={replay}>▶ Replay routing from the file</button>
      </div>
      <div className="tc-board" style={{ overflowX: zoom > 1 ? 'auto' : 'hidden' }}>
        <svg viewBox={vb.join(' ')} style={{ width: `${zoom * 100}%`, display: 'block' }} role="img" aria-label="Board re-drawn from the exported .kicad_pcb">
          <rect x={o[0]} y={o[1]} width={o[2] - o[0]} height={o[3] - o[1]} fill="var(--mask)" opacity={0.55} stroke="var(--warn)" strokeWidth={0.15} />
          {show.zones && board.zones.map((z, i) => (
            <polygon key={i} points={z.pts.map((p) => p.join(',')).join(' ')} fill={z.keepout ? 'color-mix(in srgb, var(--bad) 12%, transparent)' : 'none'} stroke={z.keepout ? 'var(--bad)' : z.layer === 'F.Cu' ? 'var(--copper)' : 'var(--series-1)'} strokeWidth={0.12} strokeDasharray={z.keepout ? '0.5 0.35' : '1 0.5'} opacity={0.9} />
          ))}
          {show.back && board.segments.slice(0, drawn).filter((s) => s.layer === 'B.Cu').map((s, i) => (
            <line key={`b${i}`} x1={s.x0} y1={s.y0} x2={s.x1} y2={s.y1} stroke="var(--series-1)" strokeWidth={s.w} strokeLinecap="round" opacity={0.75 * dimFor(s.net)} />
          ))}
          {board.footprints.map((f) => f.pads.map((p, j) => {
            if ((p.layer === 'F' && !show.front) || (p.layer === 'B' && !show.back)) return null;
            const col = p.layer === 'B' ? 'var(--series-1)' : 'var(--copper)';
            const op = dimFor(p.net);
            return (
              <g key={`${f.ref}-${j}`} transform={`rotate(${-p.angle} ${p.x} ${p.y})`} opacity={op}>
                <title>{`${f.ref}.${p.num} — ${p.net || 'no net'}`}</title>
                {p.shape === 'circle' ? <circle cx={p.x} cy={p.y} r={p.w / 2} fill={col} /> : <rect x={p.x - p.w / 2} y={p.y - p.h / 2} width={p.w} height={p.h} rx={p.shape === 'roundrect' ? Math.min(p.w, p.h) * 0.25 : 0} fill={col} />}
                {p.drill > 0 && <circle cx={p.x} cy={p.y} r={p.drill / 2} fill="var(--bg)" />}
              </g>
            );
          }))}
          {show.front && board.segments.slice(0, drawn).filter((s) => s.layer === 'F.Cu').map((s, i) => (
            <line key={`f${i}`} x1={s.x0} y1={s.y0} x2={s.x1} y2={s.y1} stroke="var(--copper)" strokeWidth={s.w} strokeLinecap="round" opacity={0.95 * dimFor(s.net)} />
          ))}
          {show.vias && board.vias.map((v, i) => (
            <g key={i} opacity={dimFor(v.net)}>
              <circle cx={v.x} cy={v.y} r={v.size / 2} fill="var(--text-2)" />
              <circle cx={v.x} cy={v.y} r={v.drill / 2} fill="var(--bg)" />
            </g>
          ))}
          {show.refs && board.footprints.map((f) => f.crtyd && (
            <g key={f.ref}>
              <rect x={f.crtyd[0]} y={f.crtyd[1]} width={f.crtyd[2] - f.crtyd[0]} height={f.crtyd[3] - f.crtyd[1]} fill="none" stroke="var(--text)" strokeWidth={0.05} opacity={0.35} />
              <text x={(f.crtyd[0] + f.crtyd[2]) / 2} y={(f.crtyd[1] + f.crtyd[3]) / 2 + 0.25} fontSize={0.75} textAnchor="middle" fill="var(--text)" style={{ pointerEvents: 'none' }}>{f.ref}</text>
            </g>
          ))}
        </svg>
      </div>
      <div className="row" style={{ gap: 6 }}>
        <span className="small muted">Re-read from the file:</span>
        {checks.map(([k, a, b]) => <Chip key={k} tone={a === b ? 'pass' : 'fail'} title={`parsed ${a}, engine wrote ${b}`}>{a === b ? '✓' : '✕'} {a} {k}</Chip>)}
        <Chip>{board.version} · {board.generator}</Chip>
      </div>
    </div>
  );
}
