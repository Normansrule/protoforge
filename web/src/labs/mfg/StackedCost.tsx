// Stacked-area chart of cost per unit vs quantity (log x), with a hover
// crosshair + tooltip, crossover and target markers, legend and direct labels.
import { useMemo, useRef, useState } from 'react';
import { useElementWidth } from '../../components/useElementWidth';
import { fmtNum, seriesColor } from '../../components/Plot';

export function StackedCost(props: {
  quantities: number[];
  categories: string[];
  perUnit: number[][];
  target: number;
  crossover: number | null;
  frameChoice: string[];
  height?: number;
}) {
  const { quantities: qs, categories: cats, perUnit } = props;
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const svgRef = useRef<SVGSVGElement>(null);
  const [hover, setHover] = useState<number | null>(null);
  const H = props.height ?? 320;
  const W = Math.max(width, 200);
  const narrow = W < 560;
  const m = { l: 48, r: narrow ? 14 : 150, t: 14, b: 40 };
  const iw = W - m.l - m.r, ih = H - m.t - m.b;
  const totals = qs.map((_, i) => perUnit.reduce((s, c) => s + c[i], 0));
  const raw = Math.max(...totals) * 1.05;
  const step0 = 10 ** Math.floor(Math.log10(raw / 4));
  const step = [1, 2, 2.5, 5, 10].map((k) => k * step0).find((s) => raw / s <= 5) ?? step0 * 10;
  const ymax = Math.ceil(raw / step) * step;
  const lx0 = Math.log10(qs[0]), lx1 = Math.log10(qs[qs.length - 1]);
  const sx = (q: number) => m.l + ((Math.log10(q) - lx0) / (lx1 - lx0 || 1)) * iw;
  const sy = (v: number) => m.t + ih - (v / ymax) * ih;
  const bands = useMemo(() => {
    const out: { d: string; top: number[] }[] = [];
    const base = qs.map(() => 0);
    for (const c of perUnit) {
      const top = base.map((b, i) => b + c[i]);
      let d = `M${sx(qs[0])},${sy(base[0])}`;
      for (let i = 0; i < qs.length; i++) d += `L${sx(qs[i])},${sy(top[i])}`;
      for (let i = qs.length - 1; i >= 0; i--) d += `L${sx(qs[i])},${sy(base[i])}`;
      out.push({ d: d + 'Z', top });
      for (let i = 0; i < base.length; i++) base[i] = top[i];
    }
    return out;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [qs, perUnit, W, H, ymax]);
  const yt = Array.from({ length: Math.round(ymax / step) + 1 }, (_, k) => k * step);
  const xt = [];
  const xstep = iw / Math.max(1, Math.floor(lx1) - Math.ceil(lx0)) < 44 ? 2 : 1;
  for (let e = Math.ceil(lx0); e <= Math.floor(lx1); e += xstep) xt.push(10 ** e);
  const hi = hover;
  // direct labels at the right edge for the largest bands
  const last = qs.length - 1;
  const labels = bands.map((b, k) => ({ k, mid: (b.top[last] - perUnit[k][last] / 2), size: perUnit[k][last] })).filter((l) => !narrow && l.size / ymax > 0.06);
  return (
    <div ref={ref} className="plot" style={{ position: 'relative' }}>
      {width > 0 && (
        <svg ref={svgRef} width={W} height={H} role="img" aria-label="Cost per unit versus quantity, stacked by category"
          onMouseMove={(e) => {
            const r = svgRef.current!.getBoundingClientRect();
            const px = e.clientX - r.left;
            if (px < m.l || px > m.l + iw) { setHover(null); return; }
            let best = 0, bd = Infinity;
            qs.forEach((q, i) => { const d = Math.abs(sx(q) - px); if (d < bd) { bd = d; best = i; } });
            setHover(best);
          }}
          onMouseLeave={() => setHover(null)}
        >
          {yt.map((v) => <g key={v} className="tick"><line className="gridline" x1={m.l} x2={m.l + iw} y1={sy(v)} y2={sy(v)} /><text x={m.l - 8} y={sy(v) + 4} textAnchor="end">{fmtNum(v)}</text></g>)}
          {xt.map((v) => <g key={v} className="tick"><line className="gridline" x1={sx(v)} x2={sx(v)} y1={m.t} y2={m.t + ih} opacity={0.6} /><text x={sx(v)} y={m.t + ih + 16} textAnchor="middle">{fmtNum(v)}</text></g>)}
          {bands.map((b, k) => <path key={k} d={b.d} fill={seriesColor(k)} opacity={hi === null ? 0.88 : 0.8} stroke="var(--surface-1)" strokeWidth={1.5} strokeLinejoin="round" />)}
          <line className="axisline" x1={m.l} x2={m.l + iw} y1={m.t + ih} y2={m.t + ih} />
          <text className="axis-label" x={m.l + iw / 2} y={H - 6} textAnchor="middle">quantity (log scale)</text>
          <text className="axis-label" transform={`translate(13 ${m.t + ih / 2}) rotate(-90)`} textAnchor="middle">USD per unit</text>
          {props.target >= qs[0] && props.target <= qs[last] && (
            <g><line x1={sx(props.target)} x2={sx(props.target)} y1={m.t} y2={m.t + ih} stroke="var(--text)" strokeDasharray="5 4" strokeWidth={1.2} />
              <text x={sx(props.target) + 4} y={m.t + 12} fontSize={11} fill="var(--text-2)">target {fmtNum(props.target)}</text></g>
          )}
          {props.crossover && props.crossover >= qs[0] && props.crossover <= qs[last] && (
            <g><line x1={sx(props.crossover)} x2={sx(props.crossover)} y1={m.t} y2={m.t + ih} stroke="var(--muted)" strokeDasharray="2 3" strokeWidth={1.2} />
              <text x={sx(props.crossover) + (narrow ? -4 : 4)} y={m.t + 26} textAnchor={narrow ? 'end' : 'start'} fontSize={11} fill="var(--text-2)">molding wins ≥ {fmtNum(props.crossover)}</text></g>
          )}
          {labels.map((l) => (
            <g key={l.k}>
              <line x1={m.l + iw + 2} x2={m.l + iw + 10} y1={sy(l.mid)} y2={sy(l.mid)} stroke={seriesColor(l.k)} strokeWidth={2} />
              <text x={m.l + iw + 13} y={sy(l.mid) + 4} fontSize={11} fill="var(--text-2)">{cats[l.k].length > 20 ? cats[l.k].slice(0, 19) + '…' : cats[l.k]}</text>
            </g>
          ))}
          {hi !== null && <line x1={sx(qs[hi])} x2={sx(qs[hi])} y1={m.t} y2={m.t + ih} stroke="var(--border-strong)" />}
        </svg>
      )}
      {hi !== null && (
        <div className="plot-tooltip" style={{ left: Math.max(0, Math.min(sx(qs[hi]) + 12, W - 260)), top: 10 }}>
          <div className="muted small">{fmtNum(qs[hi])} units · frame by {props.frameChoice[hi]}</div>
          {cats.map((c, k) => (
            <div className="tt-row" key={c}><span className="tt-swatch" style={{ background: seriesColor(k), height: 10 }} /><span>{c}</span><b className="tnum" style={{ marginLeft: 'auto', paddingLeft: 10 }}>${perUnit[k][hi].toFixed(2)}</b></div>
          ))}
          <div className="tt-row" style={{ borderTop: '1px solid var(--border)', marginTop: 4, paddingTop: 4 }}><span>Total</span><b className="tnum" style={{ marginLeft: 'auto' }}>${totals[hi].toFixed(2)}</b></div>
        </div>
      )}
      <div className="plot-legend">
        {cats.map((c, k) => <span className="lg" key={c}><span className="lg-swatch" style={{ background: seriesColor(k), height: 10, width: 10 }} />{c}</span>)}
      </div>
    </div>
  );
}
