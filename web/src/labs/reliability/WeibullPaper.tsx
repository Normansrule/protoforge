// Weibull probability paper: log time on x, ln(−ln(1 − F)) on y, so a
// Weibull distribution is a straight line with slope β. Median-rank points,
// suspensions as ticks, the fit, confidence bounds and a hover read-out.
import { useMemo, useState } from 'react';
import { useElementWidth } from '../../components/useElementWidth';
import { fmt, wy } from './util';

export interface PaperLine {
  name: string;
  beta: number;
  eta: number;
  color: string;
  dash?: string;
}

const F_TICKS = [0.001, 0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.3, 0.5, 0.632, 0.8, 0.9, 0.95, 0.99, 0.999];

export function WeibullPaper(props: {
  points: { t: number; f: number }[];
  suspensions: number[];
  lines: PaperLine[];
  bounds?: { f: number[]; lo: number[]; hi: number[] } | null;
  unit: string;
  /** Number of points revealed (for the build-up animation). */
  reveal?: number;
  highlightB?: number;
  height?: number;
}) {
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const [hover, setHover] = useState<{ x: number; y: number } | null>(null);
  const W = Math.max(width, 280);
  const H = props.height ?? 360;
  const m = { l: 50, r: 14, t: 12, b: 40 };
  const iw = W - m.l - m.r;
  const ih = H - m.t - m.b;
  const main = props.lines[0];
  const dom = useMemo(() => {
    const ts = [...props.points.map((p) => p.t), ...props.suspensions];
    if (main) {
      ts.push(main.eta * (-Math.log(1 - 0.01)) ** (1 / main.beta));
      ts.push(main.eta * (-Math.log(1 - 0.99)) ** (1 / main.beta));
    }
    const pos = ts.filter((t) => t > 0 && isFinite(t));
    const lo = Math.min(...pos);
    const hi = Math.max(...pos);
    let x0 = Math.floor(Math.log10(lo / 1.3));
    let x1 = Math.ceil(Math.log10(hi * 1.3));
    if (x1 - x0 < 1) x1 = x0 + 1;
    if (x1 - x0 > 5) x0 = x1 - 5;
    return { x0, x1, y0: wy(0.001), y1: wy(0.999) };
  }, [props.points, props.suspensions, main]);
  const sx = (t: number) => m.l + ((Math.log10(t) - dom.x0) / (dom.x1 - dom.x0)) * iw;
  const sy = (f: number) => m.t + ih - ((wy(f) - dom.y0) / (dom.y1 - dom.y0)) * ih;
  const inside = (t: number) => t >= 10 ** dom.x0 && t <= 10 ** dom.x1;
  const linePath = (beta: number, eta: number) => {
    let d = '';
    for (let k = 0; k <= 80; k++) {
      const y = dom.y0 + ((dom.y1 - dom.y0) * k) / 80;
      const f = 1 - Math.exp(-Math.exp(y));
      const t = eta * (-Math.log(1 - f)) ** (1 / beta);
      if (!inside(t)) continue;
      d += `${d ? 'L' : 'M'}${sx(t).toFixed(1)},${sy(f).toFixed(1)}`;
    }
    return d;
  };
  const boundPath = (ts: number[]) => {
    if (!props.bounds) return '';
    let d = '';
    props.bounds.f.forEach((f, i) => {
      const t = ts[i];
      if (!(t > 0) || !inside(t) || f < 0.001 || f > 0.999) return;
      d += `${d ? 'L' : 'M'}${sx(t).toFixed(1)},${sy(f).toFixed(1)}`;
    });
    return d;
  };
  const xTicks: number[] = [];
  for (let e = dom.x0; e <= dom.x1; e++) xTicks.push(10 ** e);
  const minor: number[] = [];
  for (let e = dom.x0; e < dom.x1; e++) for (const k of [2, 3, 5, 7]) minor.push(k * 10 ** e);
  const shown = props.points.slice(0, props.reveal ?? props.points.length);
  const hv = hover && main
    ? (() => {
        const t = 10 ** (dom.x0 + ((hover.x - m.l) / iw) * (dom.x1 - dom.x0));
        const f = 1 - Math.exp(-((t / main.eta) ** main.beta));
        return { t, f };
      })()
    : null;
  const b = props.highlightB;
  const bt = b && main ? main.eta * (-Math.log(1 - b)) ** (1 / main.beta) : null;
  return (
    <div ref={ref} className="rl-paper">
      <svg
        width={W}
        height={H}
        role="img"
        aria-label="Weibull probability plot"
        onMouseMove={(e) => {
          const r = (e.currentTarget as SVGSVGElement).getBoundingClientRect();
          const x = e.clientX - r.left;
          if (x >= m.l && x <= W - m.r) setHover({ x, y: e.clientY - r.top });
          else setHover(null);
        }}
        onMouseLeave={() => setHover(null)}
      >
        {minor.map((t) => (
          <line key={`m${t}`} x1={sx(t)} x2={sx(t)} y1={m.t} y2={m.t + ih} stroke="var(--grid)" strokeWidth={0.6} />
        ))}
        {xTicks.map((t) => (
          <g key={t}>
            <line x1={sx(t)} x2={sx(t)} y1={m.t} y2={m.t + ih} stroke="var(--axis)" strokeWidth={0.8} />
            <text x={sx(t)} y={m.t + ih + 15} textAnchor="middle" className="rl-axis">{fmt(t)}</text>
          </g>
        ))}
        {F_TICKS.map((f) => (
          <g key={f}>
            <line x1={m.l} x2={m.l + iw} y1={sy(f)} y2={sy(f)} stroke={f === 0.632 ? 'var(--axis)' : 'var(--grid)'} strokeWidth={f === 0.632 ? 1 : 0.7} strokeDasharray={f === 0.632 ? '4 3' : undefined} />
            <text x={m.l - 5} y={sy(f) + 3.5} textAnchor="end" className="rl-axis">{f === 0.632 ? '63.2' : +(f * 100).toFixed(1)}</text>
          </g>
        ))}
        <text x={m.l + iw / 2} y={H - 4} textAnchor="middle" className="rl-axis-label">time to failure ({props.unit || 'units'}) — log scale</text>
        <text x={12} y={m.t + ih / 2} textAnchor="middle" transform={`rotate(-90 12 ${m.t + ih / 2})`} className="rl-axis-label">unreliability F, %</text>
        {props.bounds && (
          <>
            <path d={boundPath(props.bounds.lo)} fill="none" stroke="var(--series-1)" strokeWidth={1.2} strokeDasharray="5 4" opacity={0.8} />
            <path d={boundPath(props.bounds.hi)} fill="none" stroke="var(--series-1)" strokeWidth={1.2} strokeDasharray="5 4" opacity={0.8} />
          </>
        )}
        {props.lines.map((l, i) => (
          <path
            key={l.name}
            d={linePath(l.beta, l.eta)}
            fill="none"
            stroke={l.color}
            strokeWidth={i === 0 ? 2.4 : 1.4}
            strokeDasharray={l.dash}
            className={i === 0 && props.reveal != null && props.reveal < props.points.length ? 'rl-hidden' : l.dash ? undefined : 'rl-draw'}
          />
        ))}
        {bt && inside(bt) && (
          <g>
            <line x1={sx(bt)} x2={sx(bt)} y1={sy(b!)} y2={m.t + ih} stroke="var(--series-2)" strokeDasharray="3 3" />
            <line x1={m.l} x2={sx(bt)} y1={sy(b!)} y2={sy(b!)} stroke="var(--series-2)" strokeDasharray="3 3" />
            <text x={sx(bt) + 4} y={m.t + ih - 5} className="rl-axis" style={{ fill: 'var(--series-2)' }}>B{+(b! * 100).toFixed(1)} = {fmt(bt)}</text>
          </g>
        )}
        {props.suspensions.filter(inside).map((t, i) => (
          <g key={`s${i}`}>
            <line x1={sx(t)} x2={sx(t)} y1={m.t + ih - 10 - (i % 3) * 4} y2={m.t + ih} stroke="var(--muted)" strokeWidth={2} />
          </g>
        ))}
        {shown.filter((p) => inside(p.t) && p.f > 0.0005 && p.f < 0.9995).map((p, i) => (
          <circle key={i} cx={sx(p.t)} cy={sy(p.f)} r={4.5} className="rl-pt" fill="var(--surface-1)" stroke="var(--series-2)" strokeWidth={2} />
        ))}
        {hv && inside(hv.t) && (
          <g pointerEvents="none">
            <line x1={hover!.x} x2={hover!.x} y1={m.t} y2={m.t + ih} stroke="var(--muted)" strokeWidth={0.8} />
            <circle cx={hover!.x} cy={sy(Math.min(0.999, Math.max(0.001, hv.f)))} r={3.5} fill="var(--series-1)" />
            <rect x={Math.min(hover!.x + 8, W - 150)} y={m.t + 4} width={140} height={36} rx={6} fill="var(--surface-1)" stroke="var(--border-strong)" />
            <text x={Math.min(hover!.x + 16, W - 142)} y={m.t + 19} className="rl-axis" style={{ fill: 'var(--text)' }}>t = {fmt(hv.t)} {props.unit}</text>
            <text x={Math.min(hover!.x + 16, W - 142)} y={m.t + 33} className="rl-axis" style={{ fill: 'var(--text)' }}>F = {(hv.f * 100).toFixed(2)} %, R = {((1 - hv.f) * 100).toFixed(2)} %</text>
          </g>
        )}
      </svg>
      <div className="rl-legend small">
        <span><i className="rl-dot" style={{ borderColor: 'var(--series-2)' }} /> failures (median rank)</span>
        {props.suspensions.length > 0 && <span><i className="rl-tick" /> suspensions</span>}
        {props.lines.map((l) => (
          <span key={l.name}><i className="rl-line" style={{ background: l.color }} /> {l.name}</span>
        ))}
        {props.bounds && <span><i className="rl-line dashed" /> confidence bounds</span>}
      </div>
    </div>
  );
}
