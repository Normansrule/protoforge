// Visual building blocks for the DSP and Chip labs: z-plane, polar plot,
// stem plot, heat map, animation hooks. Colours come from theme variables.
import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { useElementWidth } from '../../components/useElementWidth';
import { fmtNum, seriesColor } from '../../components/Plot';

export function useReducedMotion(): boolean {
  const [r, setR] = useState(() => typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches);
  useEffect(() => {
    const m = window.matchMedia?.('(prefers-reduced-motion: reduce)');
    if (!m) return;
    const on = () => setR(m.matches);
    m.addEventListener('change', on);
    return () => m.removeEventListener('change', on);
  }, []);
  return !!r;
}

/** Calls `fn(dtMs, tMs)` every animation frame while `running`. */
export function useFrame(running: boolean, fn: (dt: number, t: number) => void) {
  const ref = useRef(fn);
  ref.current = fn;
  useEffect(() => {
    if (!running) return;
    let raf = 0;
    let last = performance.now();
    const loop = (now: number) => {
      const dt = Math.min(100, now - last);
      last = now;
      ref.current(dt, now);
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  }, [running]);
}

/** Tween an array of numbers towards `target` (used to morph polar patterns). */
export function useTween(target: number[][], ms = 380): number[][] {
  const reduced = useReducedMotion();
  const [shown, setShown] = useState(target);
  const from = useRef(target);
  const cur = useRef(target);
  useEffect(() => {
    if (reduced || cur.current.length !== target.length || cur.current.some((a, i) => a.length !== target[i].length)) {
      cur.current = target;
      setShown(target);
      return;
    }
    from.current = cur.current;
    const t0 = performance.now();
    let raf = 0;
    const step = (now: number) => {
      const k = Math.min(1, (now - t0) / ms);
      const e = 1 - Math.pow(1 - k, 3);
      const next = target.map((arr, i) => arr.map((v, j) => from.current[i][j] + (v - from.current[i][j]) * e));
      cur.current = next;
      setShown(next);
      if (k < 1) raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [JSON.stringify(target), reduced]);
  return shown;
}

/** z-plane pole–zero plot with the unit circle. */
export function PoleZeroPlot(props: { poles: [number, number][]; zeros: [number, number][]; originPoles?: number; size?: number }) {
  const { poles, zeros } = props;
  const [wrapRef, width] = useElementWidth<HTMLDivElement>();
  const S = Math.max(180, Math.min(props.size ?? 300, width || 300));
  const all = [...poles, ...zeros];
  const extent = Math.max(1.25, ...all.map(([re, im]) => Math.max(Math.abs(re), Math.abs(im)) * 1.1)).toFixed(2);
  const R = Math.min(Number(extent), 2.2);
  const c = S / 2;
  const sc = (S / 2 - 18) / R;
  const X = (re: number) => c + re * sc;
  const Y = (im: number) => c - im * sc;
  const [hover, setHover] = useState<string | null>(null);
  const clip = (v: number) => Math.max(-R, Math.min(R, v));
  return (
    <div ref={wrapRef} style={{ width: '100%', display: 'flex', justifyContent: 'center', position: 'relative' }}>
      <svg width={S} height={S} role="img" aria-label="Pole-zero plot in the z-plane">
        <line x1={0} x2={S} y1={c} y2={c} stroke="var(--grid)" />
        <line x1={c} x2={c} y1={0} y2={S} stroke="var(--grid)" />
        <circle cx={c} cy={c} r={sc} fill="var(--accent-soft)" stroke="var(--accent)" strokeWidth={1.4} strokeDasharray="4 3" />
        <text x={c + sc * 0.72} y={c - sc * 0.72} fontSize={10.5} fill="var(--muted)">|z| = 1</text>
        <text x={S - 6} y={c - 5} fontSize={10.5} fill="var(--muted)" textAnchor="end">Re</text>
        <text x={c + 5} y={12} fontSize={10.5} fill="var(--muted)">Im</text>
        {zeros.map(([re, im], i) => (
          <circle key={`z${i}`} cx={X(clip(re))} cy={Y(clip(im))} r={5} fill="none" stroke="var(--series-1)" strokeWidth={2}
            onMouseEnter={() => setHover(`zero ${fmtNum(re)} ${im >= 0 ? '+' : '−'} j${fmtNum(Math.abs(im))}  |z| = ${fmtNum(Math.hypot(re, im))}`)} onMouseLeave={() => setHover(null)} />
        ))}
        {poles.map(([re, im], i) => {
          const r = Math.hypot(re, im);
          const col = r >= 1 ? 'var(--bad)' : 'var(--series-2)';
          return (
            <g key={`p${i}`} stroke={col} strokeWidth={2.2} onMouseEnter={() => setHover(`pole ${fmtNum(re)} ${im >= 0 ? '+' : '−'} j${fmtNum(Math.abs(im))}  |z| = ${fmtNum(r)}`)} onMouseLeave={() => setHover(null)}>
              <line x1={X(clip(re)) - 5} x2={X(clip(re)) + 5} y1={Y(clip(im)) - 5} y2={Y(clip(im)) + 5} />
              <line x1={X(clip(re)) - 5} x2={X(clip(re)) + 5} y1={Y(clip(im)) + 5} y2={Y(clip(im)) - 5} />
              <circle cx={X(clip(re))} cy={Y(clip(im))} r={9} fill="transparent" stroke="none" />
            </g>
          );
        })}
        {!!props.originPoles && (
          <text x={c + 8} y={c + 16} fontSize={11} fill="var(--series-2)">×{props.originPoles} at origin</text>
        )}
      </svg>
      <div className="muted small mono" style={{ position: 'absolute', bottom: -2, left: 0, right: 0, textAlign: 'center', minHeight: 16 }}>{hover ?? ' '}</div>
    </div>
  );
}

/** Polar pattern plot (dB rings), axisymmetric data 0…180° mirrored to 360°. */
export function PolarPlot(props: { traces: { label: string; angle_deg: number[]; gain_db: number[]; color?: string }[]; floorDb?: number; size?: number; marker?: { angle: number; label: string } | null; rotate?: number }) {
  const floor = props.floorDb ?? -40;
  const [wrapRef, width] = useElementWidth<HTMLDivElement>();
  const S = Math.max(220, Math.min(props.size ?? 360, width || 360));
  const c = S / 2;
  const R = S / 2 - 26;
  const rad = (db: number) => (Math.max(floor, Math.min(0, db)) - floor) / -floor * R;
  const rot = ((props.rotate ?? -90) * Math.PI) / 180; // 0° (endfire) points up by default
  const pt = (ang: number, db: number) => {
    const a = (ang * Math.PI) / 180 + rot;
    const r = rad(db);
    return [c + r * Math.cos(a), c + r * Math.sin(a)];
  };
  const rings = [0, -10, -20, -30, -40].filter((d) => d >= floor);
  return (
    <div ref={wrapRef} style={{ width: '100%', display: 'flex', justifyContent: 'center' }}>
      <svg width={S} height={S} role="img" aria-label="Polar directivity pattern">
        <defs>
          <radialGradient id="polar-bg">
            <stop offset="0%" stopColor="var(--accent-soft)" />
            <stop offset="100%" stopColor="transparent" />
          </radialGradient>
        </defs>
        <circle cx={c} cy={c} r={R} fill="url(#polar-bg)" />
        {rings.map((d) => (
          <g key={d}>
            <circle cx={c} cy={c} r={rad(d)} fill="none" stroke="var(--grid)" />
            <text x={c + 3} y={c - rad(d) + 11} fontSize={9.5} fill="var(--muted)">{d} dB</text>
          </g>
        ))}
        {Array.from({ length: 12 }, (_, k) => k * 30).map((a) => {
          const [x1, y1] = pt(a, 0);
          const [lx, ly] = [c + (R + 14) * Math.cos((a * Math.PI) / 180 + rot), c + (R + 14) * Math.sin((a * Math.PI) / 180 + rot)];
          return (
            <g key={a}>
              <line x1={c} y1={c} x2={x1} y2={y1} stroke="var(--grid)" />
              <text x={lx} y={ly + 3.5} fontSize={10} fill="var(--muted)" textAnchor="middle">{a > 180 ? a - 360 : a}°</text>
            </g>
          );
        })}
        {props.traces.map((t, i) => {
          const pts: string[] = [];
          for (let k = 0; k < t.angle_deg.length; k++) pts.push(pt(t.angle_deg[k], t.gain_db[k]).map((v) => v.toFixed(1)).join(','));
          for (let k = t.angle_deg.length - 2; k > 0; k--) pts.push(pt(360 - t.angle_deg[k], t.gain_db[k]).map((v) => v.toFixed(1)).join(','));
          const color = t.color ?? seriesColor(i);
          return <polygon key={t.label} points={pts.join(' ')} fill={color} fillOpacity={i === 0 ? 0.13 : 0.04} stroke={color} strokeWidth={i === 0 ? 2.4 : 1.6} strokeLinejoin="round" />;
        })}
        {props.marker && (() => {
          const [x, y] = pt(props.marker.angle, 0);
          return (
            <g>
              <line x1={c} y1={c} x2={x} y2={y} stroke="var(--copper)" strokeWidth={1.5} strokeDasharray="4 3" />
              <circle cx={x} cy={y} r={4.5} fill="var(--copper)" />
              <text x={x + 6} y={y - 6} fontSize={11} fill="var(--copper)">{props.marker.label}</text>
            </g>
          );
        })()}
      </svg>
    </div>
  );
}

/** Stem plot for impulse responses / coefficients. */
export function StemPlot(props: { y: number[]; height?: number; color?: string; highlight?: number | null; label?: string; xLabel?: string }) {
  const [wrapRef, width] = useElementWidth<HTMLDivElement>();
  const H = props.height ?? 180;
  const W = Math.max(width, 120);
  const n = props.y.length;
  const m = { l: 44, r: 10, t: 10, b: props.xLabel ? 30 : 18 };
  const max = Math.max(1e-12, ...props.y.map((v) => Math.abs(v)));
  const lo = Math.min(0, ...props.y);
  const hi = Math.max(0, ...props.y);
  const span = hi - lo || max;
  const iw = W - m.l - m.r;
  const ih = H - m.t - m.b;
  const X = (i: number) => m.l + (n <= 1 ? iw / 2 : (i / (n - 1)) * iw);
  const Y = (v: number) => m.t + ih - ((v - lo) / span) * ih;
  const col = props.color ?? 'var(--series-1)';
  return (
    <div ref={wrapRef} className="plot">
      {width > 0 && (
        <svg width={W} height={H} role="img" aria-label={props.label ?? 'stem plot'}>
          <line x1={m.l} x2={m.l + iw} y1={Y(0)} y2={Y(0)} stroke="var(--axis)" />
          <text x={m.l - 6} y={Y(hi) + 4} fontSize={10.5} textAnchor="end" fill="var(--muted)" fontFamily="var(--mono)">{fmtNum(hi)}</text>
          {lo < 0 && <text x={m.l - 6} y={Y(lo) + 4} fontSize={10.5} textAnchor="end" fill="var(--muted)" fontFamily="var(--mono)">{fmtNum(lo)}</text>}
          <text x={m.l - 6} y={Y(0) + 4} fontSize={10.5} textAnchor="end" fill="var(--muted)" fontFamily="var(--mono)">0</text>
          {props.y.map((v, i) => (
            <g key={i} opacity={props.highlight == null || props.highlight === i ? 1 : 0.55}>
              <line x1={X(i)} x2={X(i)} y1={Y(0)} y2={Y(v)} stroke={col} strokeWidth={props.highlight === i ? 2.5 : 1.4} />
              <circle cx={X(i)} cy={Y(v)} r={n > 120 ? 1.6 : props.highlight === i ? 4.5 : 3} fill={col} stroke="var(--surface-1)" strokeWidth={1} />
            </g>
          ))}
          {props.xLabel && <text className="axis-label" x={m.l + iw / 2} y={H - 6} textAnchor="middle" fill="var(--text-2)" fontSize={11.5}>{props.xLabel}</text>}
        </svg>
      )}
    </div>
  );
}

const HEAT_STOPS = [
  [222, 60, 18],
  [205, 75, 38],
  [170, 70, 42],
  [45, 90, 55],
  [30, 95, 82],
];

function heatHsl(t: number): [number, number, number] {
  const k = Math.max(0, Math.min(1, t));
  const p = k * (HEAT_STOPS.length - 1);
  const i = Math.min(HEAT_STOPS.length - 2, Math.floor(p));
  const f = p - i;
  const v = HEAT_STOPS[i].map((x, j) => x + (HEAT_STOPS[i + 1][j] - x) * f);
  return [v[0], v[1], v[2]];
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  s /= 100;
  l /= 100;
  const k = (n: number) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
  return [Math.round(255 * f(0)), Math.round(255 * f(8)), Math.round(255 * f(4))];
}

/** Sequential colour ramp (deep blue → teal → amber → warm white) for heat maps; reads in both themes. */
export function heatColor(t: number): string {
  const [h, s, l] = heatHsl(t);
  return `hsl(${h.toFixed(0)} ${s.toFixed(0)}% ${l.toFixed(0)}%)`;
}

/** Draw a row-major grid of values into a canvas (smoothed by CSS scaling). */
export function HeatCanvas(props: { nx: number; ny: number; values: number[]; min: number; max: number; style?: React.CSSProperties; flipY?: boolean }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    cv.width = props.nx;
    cv.height = props.ny;
    const g = cv.getContext('2d');
    if (!g) return;
    const img = g.createImageData(props.nx, props.ny);
    for (let j = 0; j < props.ny; j++) {
      for (let i = 0; i < props.nx; i++) {
        const v = props.values[j * props.nx + i];
        const [h, s, l] = heatHsl((v - props.min) / (props.max - props.min || 1));
        const rgb = hslToRgb(h, s, l);
        const row = props.flipY ? props.ny - 1 - j : j;
        const o = (row * props.nx + i) * 4;
        img.data[o] = rgb[0];
        img.data[o + 1] = rgb[1];
        img.data[o + 2] = rgb[2];
        img.data[o + 3] = 255;
      }
    }
    g.putImageData(img, 0, 0);
  }, [props.nx, props.ny, props.values, props.min, props.max, props.flipY]);
  return <canvas ref={ref} style={{ width: '100%', height: '100%', ...props.style }} />;
}

/** A labelled key/value row used in stat lists. */
export function KV(props: { k: ReactNode; v: ReactNode; hint?: string }) {
  return (
    <div className="row small" style={{ justifyContent: 'space-between', borderBottom: '1px solid var(--border)', padding: '5px 0', flexWrap: 'nowrap' }} title={props.hint}>
      <span className="muted">{props.k}</span>
      <span className="mono tnum" style={{ textAlign: 'right' }}>{props.v}</span>
    </div>
  );
}

/** Segmented control (small tab-like buttons). */
export function Seg<T extends string | number>(props: { value: T; options: { value: T; label: ReactNode }[]; onChange: (v: T) => void; label?: string }) {
  return (
    <div className="field">
      {props.label && <span className="field-top"><span className="field-label">{props.label}</span></span>}
      <div className="seg" role="radiogroup">
        {props.options.map((o) => (
          <button key={String(o.value)} role="radio" aria-checked={o.value === props.value} className={`seg-btn ${o.value === props.value ? 'on' : ''}`} onClick={() => props.onChange(o.value)}>
            {o.label}
          </button>
        ))}
      </div>
    </div>
  );
}

export function useMemoJSON<T>(v: T): T {
  const key = JSON.stringify(v);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  return useMemo(() => v, [key]);
}

export function csvOf(cols: Record<string, ArrayLike<number>>): string {
  const keys = Object.keys(cols);
  const n = Math.max(...keys.map((k) => cols[k].length));
  const rows = [keys.join(',')];
  for (let i = 0; i < n; i++) rows.push(keys.map((k) => (i < cols[k].length ? String(cols[k][i]) : '')).join(','));
  return rows.join('\n') + '\n';
}
