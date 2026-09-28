// A small, dependency-free SVG line/step/scatter plot with a hover crosshair,
// legend, log axes, reference lines and shaded bands. Used by every lab.
import { useMemo, useRef, useState } from 'react';
import { useElementWidth } from './useElementWidth';

export interface Series {
  name: string;
  x: number[];
  y: number[];
  /** CSS colour; defaults to categorical slot by index (var(--series-N)). */
  color?: string;
  dash?: string;
  width?: number;
  /** Draw as a staircase (digital waveforms). */
  step?: boolean;
  /** Draw points only. */
  scatter?: boolean;
  /** Fill between the line and y = 0. */
  area?: boolean;
}

export interface RefLine {
  value: number;
  label?: string;
  color?: string;
}

export interface Band {
  from: number;
  to: number;
  color?: string;
  label?: string;
}

export interface PlotProps {
  series: Series[];
  height?: number;
  xLabel?: string;
  yLabel?: string;
  xLog?: boolean;
  yLog?: boolean;
  xDomain?: [number, number];
  yDomain?: [number, number];
  /** Always include y = 0 in the auto domain. */
  yZero?: boolean;
  vLines?: RefLine[];
  hLines?: RefLine[];
  xBands?: Band[];
  formatX?: (v: number) => string;
  formatY?: (v: number) => string;
  legend?: boolean;
  title?: string;
}

export const seriesColor = (i: number) => `var(--series-${(i % 8) + 1})`;

export function fmtNum(v: number): string {
  if (!isFinite(v)) return '—';
  const a = Math.abs(v);
  if (a === 0) return '0';
  if (a >= 1e9) return (v / 1e9).toFixed(a >= 1e10 ? 0 : 1) + 'G';
  if (a >= 1e6) return (v / 1e6).toFixed(a >= 1e7 ? 0 : 1) + 'M';
  if (a >= 1e4) return (v / 1e3).toFixed(a >= 1e5 ? 0 : 1) + 'k';
  if (a >= 100) return v.toFixed(0);
  if (a >= 10) return v.toFixed(1);
  if (a >= 1) return v.toFixed(2);
  if (a >= 1e-3) return v.toPrecision(3);
  const exp = Math.floor(Math.log10(a));
  const units: [number, string][] = [[-6, 'µ'], [-9, 'n'], [-12, 'p']];
  for (const [e, s] of units) if (exp >= e) return (v / 10 ** e).toPrecision(3) + s;
  return v.toExponential(2);
}

function niceStep(range: number, count: number) {
  const raw = range / Math.max(count, 1);
  const mag = 10 ** Math.floor(Math.log10(raw));
  const norm = raw / mag;
  const step = norm < 1.5 ? 1 : norm < 3 ? 2 : norm < 7 ? 5 : 10;
  return step * mag;
}

function linTicks(min: number, max: number, count: number) {
  if (!(max > min)) return [min];
  const step = niceStep(max - min, count);
  const out: number[] = [];
  for (let v = Math.ceil(min / step) * step; v <= max + step * 1e-6; v += step) out.push(Math.abs(v) < step * 1e-9 ? 0 : v);
  return out;
}

function logTicks(min: number, max: number) {
  const out: number[] = [];
  for (let e = Math.floor(Math.log10(min)); e <= Math.ceil(Math.log10(max)); e++) {
    const v = 10 ** e;
    if (v >= min * 0.999 && v <= max * 1.001) out.push(v);
  }
  return out;
}

/** Keep at most ~2 points per pixel column (min/max decimation). */
function decimate(x: number[], y: number[], maxPts: number): [number[], number[]] {
  const n = Math.min(x.length, y.length);
  if (n <= maxPts) return [x.slice(0, n), y.slice(0, n)];
  const bucket = n / (maxPts / 2);
  const ox: number[] = [];
  const oy: number[] = [];
  for (let b = 0; b < maxPts / 2; b++) {
    const s = Math.floor(b * bucket);
    const e = Math.min(n, Math.floor((b + 1) * bucket));
    let lo = s;
    let hi = s;
    for (let i = s; i < e; i++) {
      if (y[i] < y[lo]) lo = i;
      if (y[i] > y[hi]) hi = i;
    }
    const [a, c] = lo < hi ? [lo, hi] : [hi, lo];
    ox.push(x[a], x[c]);
    oy.push(y[a], y[c]);
  }
  return [ox, oy];
}

export function Plot(props: PlotProps) {
  const { series, height = 260, xLabel, yLabel, xLog, yLog, legend, title } = props;
  const [wrapRef, width] = useElementWidth<HTMLDivElement>();
  const [hover, setHover] = useState<number | null>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const m = { l: 56, r: 16, t: title ? 26 : 12, b: xLabel ? 42 : 28 };
  const W = Math.max(width, 120);
  const H = height;
  const iw = W - m.l - m.r;
  const ih = H - m.t - m.b;

  const dom = useMemo(() => {
    let x0 = Infinity, x1 = -Infinity, y0 = Infinity, y1 = -Infinity;
    for (const s of series) {
      const n = Math.min(s.x.length, s.y.length);
      for (let i = 0; i < n; i++) {
        const xv = s.x[i], yv = s.y[i];
        if (!isFinite(xv) || !isFinite(yv)) continue;
        if (xLog && xv <= 0) continue;
        if (yLog && yv <= 0) continue;
        if (xv < x0) x0 = xv;
        if (xv > x1) x1 = xv;
        if (yv < y0) y0 = yv;
        if (yv > y1) y1 = yv;
      }
    }
    for (const h of props.hLines ?? []) { y0 = Math.min(y0, h.value); y1 = Math.max(y1, h.value); }
    if (!isFinite(x0)) { x0 = 0; x1 = 1; }
    if (!isFinite(y0)) { y0 = 0; y1 = 1; }
    if (props.yZero && !yLog) { y0 = Math.min(y0, 0); y1 = Math.max(y1, 0); }
    if (y1 === y0) { y0 -= 1; y1 += 1; }
    if (!yLog) { const pad = (y1 - y0) * 0.06; y0 -= pad; y1 += pad; if (props.yZero && y0 < 0 && Math.min(...series.flatMap(s => s.y.slice(0, 1))) >= 0) y0 = Math.max(y0, 0 - pad); }
    if (props.xDomain) [x0, x1] = props.xDomain;
    if (props.yDomain) [y0, y1] = props.yDomain;
    return { x0, x1, y0, y1 };
  }, [series, xLog, yLog, props.xDomain, props.yDomain, props.yZero, props.hLines]);

  const sx = (v: number) =>
    xLog ? m.l + ((Math.log10(v) - Math.log10(dom.x0)) / (Math.log10(dom.x1) - Math.log10(dom.x0))) * iw
         : m.l + ((v - dom.x0) / (dom.x1 - dom.x0 || 1)) * iw;
  const sy = (v: number) =>
    yLog ? m.t + ih - ((Math.log10(v) - Math.log10(dom.y0)) / (Math.log10(dom.y1) - Math.log10(dom.y0))) * ih
         : m.t + ih - ((v - dom.y0) / (dom.y1 - dom.y0 || 1)) * ih;
  const invX = (px: number) => {
    const f = (px - m.l) / iw;
    return xLog ? 10 ** (Math.log10(dom.x0) + f * (Math.log10(dom.x1) - Math.log10(dom.x0))) : dom.x0 + f * (dom.x1 - dom.x0);
  };

  const xt = xLog ? logTicks(dom.x0, dom.x1) : linTicks(dom.x0, dom.x1, Math.max(2, Math.floor(iw / 90)));
  const yt = yLog ? logTicks(dom.y0, dom.y1) : linTicks(dom.y0, dom.y1, Math.max(2, Math.floor(ih / 45)));
  const fx = props.formatX ?? fmtNum;
  const fy = props.formatY ?? fmtNum;

  const paths = useMemo(() => {
    return series.map((s) => {
      const [dx, dy] = decimate(s.x, s.y, Math.max(200, Math.floor(iw * 2)));
      let d = '';
      let started = false;
      let prevY = 0;
      const pts: [number, number][] = [];
      for (let i = 0; i < dx.length; i++) {
        const xv = dx[i], yv = dy[i];
        if (!isFinite(xv) || !isFinite(yv) || (xLog && xv <= 0) || (yLog && yv <= 0)) { started = false; continue; }
        const X = sx(xv), Y = sy(Math.min(Math.max(yv, yLog ? 1e-300 : -1e300), 1e300));
        pts.push([X, Y]);
        if (!started) { d += `M${X.toFixed(1)},${Y.toFixed(1)}`; started = true; }
        else if (s.step) d += `H${X.toFixed(1)}V${Y.toFixed(1)}`;
        else d += `L${X.toFixed(1)},${Y.toFixed(1)}`;
        prevY = Y;
      }
      void prevY;
      let area = '';
      if (s.area && pts.length > 1) {
        const base = sy(yLog ? dom.y0 : Math.max(dom.y0, Math.min(0, dom.y1)));
        area = `M${pts[0][0]},${base}` + pts.map(([X, Y]) => `L${X},${Y}`).join('') + `L${pts[pts.length - 1][0]},${base}Z`;
      }
      return { d, area, pts };
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [series, iw, ih, dom, xLog, yLog]);

  // hover lookup
  const hoverInfo = useMemo(() => {
    if (hover === null) return null;
    const xv = invX(hover);
    const rows = series.map((s, i) => {
      const n = Math.min(s.x.length, s.y.length);
      if (n === 0) return null;
      let lo = 0, hi = n - 1;
      const asc = s.x[n - 1] >= s.x[0];
      if (asc) {
        while (hi - lo > 1) { const mid = (lo + hi) >> 1; if (s.x[mid] < xv) lo = mid; else hi = mid; }
      } else { lo = 0; hi = 0; let best = Infinity; for (let k = 0; k < n; k++) { const dd = Math.abs(s.x[k] - xv); if (dd < best) { best = dd; lo = k; hi = k; } } }
      const idx = Math.abs(s.x[lo] - xv) <= Math.abs(s.x[hi] - xv) ? lo : hi;
      return { name: s.name, color: s.color ?? seriesColor(i), x: s.x[idx], y: s.y[idx] };
    }).filter(Boolean) as { name: string; color: string; x: number; y: number }[];
    return { xv, rows };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hover, series, dom, iw]);

  const showLegend = legend ?? series.length > 1;

  return (
    <div className="plot" ref={wrapRef}>
      {width > 0 && (
        <svg
          ref={svgRef}
          width={W}
          height={H}
          role="img"
          aria-label={title ?? yLabel ?? 'plot'}
          onMouseMove={(e) => {
            const r = svgRef.current!.getBoundingClientRect();
            const px = e.clientX - r.left;
            setHover(px >= m.l && px <= m.l + iw ? px : null);
          }}
          onMouseLeave={() => setHover(null)}
        >
          {title && <text x={m.l} y={14} fill="var(--text)" fontSize={12.5} fontWeight={600}>{title}</text>}
          <defs>
            <clipPath id={`clip-${W}-${H}-${m.t}`}><rect x={m.l} y={m.t} width={iw} height={ih} /></clipPath>
          </defs>
          {(props.xBands ?? []).map((b, i) => {
            const a = sx(Math.max(b.from, dom.x0)), c = sx(Math.min(b.to, dom.x1));
            return (
              <g key={`b${i}`}>
                <rect x={Math.min(a, c)} y={m.t} width={Math.abs(c - a)} height={ih} fill={b.color ?? 'var(--accent-soft)'} />
                {b.label && <text x={Math.min(a, c) + 4} y={m.t + 12} fontSize={10.5} fill="var(--muted)">{b.label}</text>}
              </g>
            );
          })}
          {yt.map((v) => (
            <g key={`y${v}`} className="tick">
              <line className="gridline" x1={m.l} x2={m.l + iw} y1={sy(v)} y2={sy(v)} />
              <text x={m.l - 8} y={sy(v) + 4} textAnchor="end">{fy(v)}</text>
            </g>
          ))}
          {xt.map((v) => (
            <g key={`x${v}`} className="tick">
              <line className="gridline" x1={sx(v)} x2={sx(v)} y1={m.t} y2={m.t + ih} opacity={0.6} />
              <text x={sx(v)} y={m.t + ih + 16} textAnchor="middle">{fx(v)}</text>
            </g>
          ))}
          <line className="axisline" x1={m.l} x2={m.l + iw} y1={m.t + ih} y2={m.t + ih} />
          <line className="axisline" x1={m.l} x2={m.l} y1={m.t} y2={m.t + ih} />
          {xLabel && <text className="axis-label" x={m.l + iw / 2} y={H - 6} textAnchor="middle">{xLabel}</text>}
          {yLabel && <text className="axis-label" transform={`translate(13 ${m.t + ih / 2}) rotate(-90)`} textAnchor="middle">{yLabel}</text>}
          <g clipPath={`url(#clip-${W}-${H}-${m.t})`}>
            {paths.map((p, i) => {
              const s = series[i];
              const color = s.color ?? seriesColor(i);
              return (
                <g key={s.name + i}>
                  {p.area && <path d={p.area} fill={color} opacity={0.14} />}
                  {s.scatter
                    ? p.pts.map(([X, Y], k) => <circle key={k} cx={X} cy={Y} r={3} fill={color} stroke="var(--surface-1)" strokeWidth={1} />)
                    : <path d={p.d} fill="none" stroke={color} strokeWidth={s.width ?? 2} strokeDasharray={s.dash} strokeLinejoin="round" strokeLinecap="round" />}
                </g>
              );
            })}
            {(props.hLines ?? []).map((h, i) => (
              <g key={`h${i}`}>
                <line x1={m.l} x2={m.l + iw} y1={sy(h.value)} y2={sy(h.value)} stroke={h.color ?? 'var(--muted)'} strokeDasharray="5 4" strokeWidth={1.2} />
                {h.label && <text x={m.l + iw - 4} y={sy(h.value) - 5} fontSize={10.5} textAnchor="end" fill="var(--text-2)">{h.label}</text>}
              </g>
            ))}
            {(props.vLines ?? []).map((v, i) => (
              <g key={`v${i}`}>
                <line x1={sx(v.value)} x2={sx(v.value)} y1={m.t} y2={m.t + ih} stroke={v.color ?? 'var(--muted)'} strokeDasharray="5 4" strokeWidth={1.2} />
                {v.label && <text x={sx(v.value) + 4} y={m.t + 12} fontSize={10.5} fill="var(--text-2)">{v.label}</text>}
              </g>
            ))}
          </g>
          {hoverInfo && hover !== null && (
            <g pointerEvents="none">
              <line x1={hover} x2={hover} y1={m.t} y2={m.t + ih} stroke="var(--border-strong)" />
              {hoverInfo.rows.map((r, i) =>
                isFinite(r.y) && (!yLog || r.y > 0) && r.y >= Math.min(dom.y0, dom.y1) && r.y <= Math.max(dom.y0, dom.y1) ? (
                  <circle key={i} cx={sx(r.x)} cy={sy(r.y)} r={4} fill={r.color} stroke="var(--surface-1)" strokeWidth={2} />
                ) : null,
              )}
            </g>
          )}
        </svg>
      )}
      {hoverInfo && hover !== null && hoverInfo.rows.length > 0 && (
        <div className="plot-tooltip" style={{ left: Math.min(hover + 12, W - 170), top: m.t + 4 }}>
          <div className="muted small">{xLabel ?? 'x'}: {fx(hoverInfo.rows[0].x)}</div>
          {hoverInfo.rows.map((r, i) => (
            <div className="tt-row" key={i}>
              <span className="tt-swatch" style={{ background: r.color }} />
              <span>{r.name}</span>
              <b className="tnum" style={{ marginLeft: 'auto', paddingLeft: 10 }}>{fy(r.y)}</b>
            </div>
          ))}
        </div>
      )}
      {showLegend && (
        <div className="plot-legend">
          {series.map((s, i) => (
            <span className="lg" key={s.name + i}>
              <span className="lg-swatch" style={{ background: s.color ?? seriesColor(i) }} />
              {s.name}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
