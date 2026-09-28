// Shared visuals for the Quantum Lab: Bloch spheres (SVG projection with
// drag-to-rotate), amplitude bars with a phase colour wheel, histograms,
// a play/step/scrub bar, a gauge and a bitmap of random bits.
import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { Icon } from '../../components/Icon';

// ---------------------------------------------------------------------------
// Motion helpers
// ---------------------------------------------------------------------------

export function useReducedMotion(): boolean {
  const [r, setR] = useState(() => typeof window !== 'undefined' && !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches);
  useEffect(() => {
    const mq = window.matchMedia?.('(prefers-reduced-motion: reduce)');
    if (!mq) return;
    const f = () => setR(mq.matches);
    mq.addEventListener('change', f);
    return () => mq.removeEventListener('change', f);
  }, []);
  return r;
}

export interface Player {
  i: number;
  /** Fractional position between frame i and i+1 (0..1), for tweening. */
  frac: number;
  n: number;
  playing: boolean;
  setI: (i: number) => void;
  setPlaying: (p: boolean) => void;
  speed: number;
  setSpeed: (s: number) => void;
}

/**
 * Frame player over `n` frames at `fps` frames per second (× speed). Starts
 * paused when the user prefers reduced motion. `resetKey` rewinds.
 */
export function usePlayer(n: number, fps = 1.2, opts: { loop?: boolean; autoplay?: boolean; resetKey?: unknown; hold?: number; /** After the first load, jump to the last frame (paused) when resetKey changes. */ resetToEnd?: boolean } = {}): Player {
  const reduced = useReducedMotion();
  const [pos, setPos] = useState(0);
  const [playing, setPlaying] = useState(opts.autoplay !== false && !reduced);
  const [speed, setSpeed] = useState(1);
  const nRef = useRef(n);
  nRef.current = n;
  const first = useRef(true);
  useEffect(() => {
    if (first.current) {
      first.current = false;
      return;
    }
    if (opts.resetToEnd) {
      setPlaying(false);
      setPos(1e9); // clamped to the last frame
      return;
    }
    setPos(0);
    if (opts.autoplay !== false && !reduced) setPlaying(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [opts.resetKey]);
  useEffect(() => {
    if (!playing || n <= 1) return;
    let raf = 0;
    let last = performance.now();
    const hold = opts.hold ?? 1.5; // frames to pause at the end before looping
    const tick = (now: number) => {
      const dt = Math.min(0.1, (now - last) / 1000);
      last = now;
      setPos((p) => {
        let q = Math.min(p, nRef.current - 1 + (opts.hold ?? 1.5)) + dt * fps * speed;
        const end = nRef.current - 1;
        if (q > end + hold) {
          if (opts.loop === false) {
            setPlaying(false);
            return end;
          }
          q = 0;
        }
        return q;
      });
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [playing, fps, speed, n, opts.loop, opts.hold]);
  const clamped = Math.max(0, Math.min(n - 1, pos));
  const i = Math.floor(clamped);
  return {
    i,
    frac: i >= n - 1 ? 0 : clamped - i,
    n,
    playing,
    setI: (k) => { setPlaying(false); setPos(Math.max(0, Math.min(n - 1, k))); },
    setPlaying: (v) => { if (v && pos >= n - 1) setPos(0); setPlaying(v); },
    speed,
    setSpeed,
  };
}

export function PlayBar({ p, label, extra }: { p: Player; label?: (i: number) => ReactNode; extra?: ReactNode }) {
  return (
    <div className="qx-play">
      <button className="btn small" onClick={() => p.setI(p.i - 1)} aria-label="previous step" disabled={p.i <= 0}>◀</button>
      <button className="btn small primary" onClick={() => p.setPlaying(!p.playing)} aria-label={p.playing ? 'pause' : 'play'}>
        {p.playing ? '❚❚ Pause' : <><Icon name="play" size={12} /> Play</>}
      </button>
      <button className="btn small" onClick={() => p.setI(p.i + 1)} aria-label="next step" disabled={p.i >= p.n - 1}>▶</button>
      <input type="range" min={0} max={Math.max(0, p.n - 1)} step={1} value={p.i} onChange={(e) => p.setI(Number(e.target.value))} aria-label="step" />
      <span className="small tnum qx-play-label">{label ? label(p.i) : `${p.i} / ${p.n - 1}`}</span>
      <select value={p.speed} onChange={(e) => p.setSpeed(Number(e.target.value))} aria-label="speed" className="qx-speed">
        <option value={0.5}>0.5×</option>
        <option value={1}>1×</option>
        <option value={2}>2×</option>
        <option value={4}>4×</option>
      </select>
      {extra}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Colour helpers
// ---------------------------------------------------------------------------

/** Phase (radians) → colour on the wheel used by Quirk-style amplitude displays. */
export function phaseColor(phase: number, light = 52) {
  const deg = ((phase * 180) / Math.PI + 360) % 360;
  return `hsl(${deg.toFixed(0)} 78% ${light}%)`;
}

export function PhaseWheel({ size = 46 }: { size?: number }) {
  const r = size / 2 - 3;
  const segs = 36;
  return (
    <svg width={size} height={size} viewBox={`${-size / 2} ${-size / 2} ${size} ${size}`} role="img" aria-label="phase colour wheel: 0 at right, counter-clockwise">
      {Array.from({ length: segs }, (_, k) => {
        const a0 = (k / segs) * 2 * Math.PI;
        const a1 = ((k + 1) / segs) * 2 * Math.PI;
        return (
          <path key={k} d={`M0 0 L${r * Math.cos(a0)} ${-r * Math.sin(a0)} A${r} ${r} 0 0 0 ${r * Math.cos(a1)} ${-r * Math.sin(a1)} Z`} fill={phaseColor((a0 + a1) / 2)} />
        );
      })}
      <circle r={r * 0.42} fill="var(--surface-1)" />
      <text x={r + 1} y={3} fontSize={7} fill="var(--muted)" textAnchor="start">0</text>
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Bloch sphere
// ---------------------------------------------------------------------------

type V3 = [number, number, number];

function project(p: V3, az: number, el: number, R: number, c: number) {
  const [x, y, z] = p;
  const u = x * Math.cos(az) + y * Math.sin(az);
  const v = -x * Math.sin(az) + y * Math.cos(az);
  const sx = c + R * v;
  const sy = c - R * (z * Math.cos(el) - u * Math.sin(el));
  const depth = u * Math.cos(el) + z * Math.sin(el);
  return { sx, sy, depth };
}

function ringPath(pts: V3[], az: number, el: number, R: number, c: number) {
  // split into front (solid) and back (dashed) runs
  const front: string[] = [];
  const back: string[] = [];
  let prev: ReturnType<typeof project> | null = null;
  for (const p of pts) {
    const q = project(p, az, el, R, c);
    if (prev) {
      const seg = `M${prev.sx.toFixed(1)} ${prev.sy.toFixed(1)}L${q.sx.toFixed(1)} ${q.sy.toFixed(1)}`;
      ((prev.depth + q.depth) / 2 >= 0 ? front : back).push(seg);
    }
    prev = q;
  }
  return { front: front.join(''), back: back.join('') };
}

const circle = (f: (t: number) => V3, n = 72): V3[] => Array.from({ length: n + 1 }, (_, k) => f((k / n) * 2 * Math.PI));

export function BlochSphere(props: {
  v: V3;
  size?: number;
  label?: ReactNode;
  trail?: V3[];
  /** Affine image of the sphere under a channel: r → M r + t. */
  ellipsoid?: { m: number[][]; t: number[] } | null;
  ghost?: V3 | null;
  color?: string;
  sub?: ReactNode;
}) {
  const size = props.size ?? 150;
  const c = size / 2;
  const R = size * 0.36;
  const [az, setAz] = useState(-0.55);
  const el = 0.32;
  const drag = useRef<{ x: number; az: number } | null>(null);
  const rings = useMemo(() => ({
    eq: ringPath(circle((t) => [Math.cos(t), Math.sin(t), 0]), az, el, R, c),
    xz: ringPath(circle((t) => [Math.sin(t), 0, Math.cos(t)]), az, el, R, c),
    yz: ringPath(circle((t) => [0, Math.sin(t), Math.cos(t)]), az, el, R, c),
  }), [az, R, c]);
  const ell = useMemo(() => {
    if (!props.ellipsoid) return null;
    const { m, t } = props.ellipsoid;
    const map = (p: V3): V3 => [0, 1, 2].map((i) => m[i][0] * p[0] + m[i][1] * p[1] + m[i][2] * p[2] + t[i]) as V3;
    const paths: { front: string; back: string }[] = [];
    for (let k = 1; k < 6; k++) {
      const lat = -Math.PI / 2 + (k * Math.PI) / 6;
      paths.push(ringPath(circle((s) => map([Math.cos(lat) * Math.cos(s), Math.cos(lat) * Math.sin(s), Math.sin(lat)]), 48), az, el, R, c));
    }
    for (let k = 0; k < 6; k++) {
      const lon = (k * Math.PI) / 6;
      paths.push(ringPath(circle((s) => map([Math.sin(s) * Math.cos(lon), Math.sin(s) * Math.sin(lon), Math.cos(s)]), 48), az, el, R, c));
    }
    return paths;
  }, [props.ellipsoid, az, R, c]);
  const tip = project(props.v, az, el, R, c);
  const base = project([props.v[0], props.v[1], 0], az, el, R, c);
  const o = project([0, 0, 0], az, el, R, c);
  const axes: { p: V3; t: string }[] = [
    { p: [0, 0, 1.22], t: '|0⟩' },
    { p: [0, 0, -1.25], t: '|1⟩' },
    { p: [1.3, 0, 0], t: '|+⟩' },
    { p: [0, 1.28, 0], t: '|+i⟩' },
  ];
  const col = props.color ?? 'var(--accent)';
  const r = Math.hypot(...props.v);
  return (
    <figure className="qx-bloch" style={{ width: size }}>
      <svg
        width={size}
        height={size}
        viewBox={`0 0 ${size} ${size}`}
        role="img"
        aria-label={`Bloch vector (${props.v.map((x) => x.toFixed(2)).join(', ')})`}
        onPointerDown={(e) => { drag.current = { x: e.clientX, az }; (e.target as Element).setPointerCapture?.(e.pointerId); }}
        onPointerMove={(e) => { if (drag.current) setAz(drag.current.az - (e.clientX - drag.current.x) / 60); }}
        onPointerUp={() => { drag.current = null; }}
        style={{ touchAction: 'pan-y', cursor: 'grab' }}
      >
        <circle cx={c} cy={c} r={R} fill="var(--qx-sphere)" stroke="var(--border-strong)" />
        {(['eq', 'xz', 'yz'] as const).map((k) => (
          <g key={k}>
            <path d={rings[k].back} stroke="var(--muted)" strokeWidth={0.7} strokeDasharray="2 3" fill="none" opacity={0.5} />
            <path d={rings[k].front} stroke="var(--muted)" strokeWidth={0.8} fill="none" opacity={0.75} />
          </g>
        ))}
        {ell && ell.map((p, k) => (
          <g key={k}>
            <path d={p.back} stroke="var(--series-2)" strokeWidth={0.7} fill="none" opacity={0.35} />
            <path d={p.front} stroke="var(--series-2)" strokeWidth={0.9} fill="none" opacity={0.8} />
          </g>
        ))}
        {axes.map((a) => {
          const q = project(a.p, az, el, R, c);
          return <text key={a.t} x={q.sx} y={q.sy + 3} fontSize={size < 130 ? 8 : 9.5} fill="var(--text-2)" textAnchor="middle">{a.t}</text>;
        })}
        {props.trail && props.trail.length > 1 && (
          <polyline
            points={props.trail.map((p) => { const q = project(p, az, el, R, c); return `${q.sx.toFixed(1)},${q.sy.toFixed(1)}`; }).join(' ')}
            fill="none" stroke={col} strokeWidth={1.2} opacity={0.45}
          />
        )}
        {props.ghost && (() => {
          const g = project(props.ghost, az, el, R, c);
          return <line x1={o.sx} y1={o.sy} x2={g.sx} y2={g.sy} stroke="var(--muted)" strokeWidth={1.4} strokeDasharray="3 3" />;
        })()}
        <line x1={o.sx} y1={o.sy} x2={base.sx} y2={base.sy} stroke={col} strokeWidth={0.8} strokeDasharray="2 2" opacity={0.6} />
        <line x1={base.sx} y1={base.sy} x2={tip.sx} y2={tip.sy} stroke={col} strokeWidth={0.8} strokeDasharray="2 2" opacity={0.6} />
        <line x1={o.sx} y1={o.sy} x2={tip.sx} y2={tip.sy} stroke={col} strokeWidth={2.6} strokeLinecap="round" />
        <circle cx={tip.sx} cy={tip.sy} r={r > 0.02 ? 4.2 : 3} fill={col} stroke="var(--surface-1)" strokeWidth={1.2} />
        <circle cx={o.sx} cy={o.sy} r={1.6} fill="var(--text-2)" />
      </svg>
      {(props.label || props.sub) && (
        <figcaption>
          {props.label && <div className="qx-bloch-label">{props.label}</div>}
          {props.sub && <div className="small muted tnum">{props.sub}</div>}
        </figcaption>
      )}
    </figure>
  );
}

// ---------------------------------------------------------------------------
// Amplitudes and histograms
// ---------------------------------------------------------------------------

/** Quirk-style amplitude display: disc radius ∝ |a|, colour = phase, bar = |a|². */
export function AmplitudeBars({ labels, re, im, highlight }: { labels: string[]; re: number[]; im: number[]; highlight?: number | null }) {
  const n = labels.length;
  const dense = n > 16;
  return (
    <div className={`qx-amps ${dense ? 'dense' : ''}`} role="list" aria-label="state amplitudes">
      {labels.map((l, k) => {
        const mag = Math.hypot(re[k], im[k]);
        const ph = Math.atan2(im[k], re[k]);
        const p = mag * mag;
        const zero = mag < 1e-9;
        return (
          <div key={l + k} className={`qx-amp ${highlight === k ? 'hl' : ''}`} role="listitem" title={`|${l}⟩  a = ${re[k].toFixed(3)} ${im[k] < 0 ? '−' : '+'} ${Math.abs(im[k]).toFixed(3)}i   P = ${(p * 100).toFixed(2)} %   phase ${((ph * 180) / Math.PI).toFixed(0)}°`}>
            <div className="qx-amp-bar"><div style={{ height: `${Math.min(100, p * 100)}%`, background: zero ? 'transparent' : phaseColor(ph) }} /></div>
            {!dense && (
              <svg width={22} height={22} viewBox="-11 -11 22 22" aria-hidden>
                <circle r={10} fill="none" stroke="var(--border-strong)" />
                {!zero && <circle r={Math.max(1.5, 10 * mag)} fill={phaseColor(ph)} opacity={0.9} />}
                {!zero && <line x1={0} y1={0} x2={10 * Math.cos(ph)} y2={-10 * Math.sin(ph)} stroke="var(--text)" strokeWidth={1.3} />}
              </svg>
            )}
            <div className="qx-amp-label mono">{l}</div>
          </div>
        );
      })}
    </div>
  );
}

export function Histogram({ entries, exact, height = 170, max }: { entries: { bits: string; count: number; prob: number }[]; exact?: { bits: string; prob: number }[]; height?: number; max?: number }) {
  const keys = useMemo(() => {
    const s = new Set<string>(entries.map((e) => e.bits));
    exact?.forEach((e) => s.add(e.bits));
    return [...s].sort();
  }, [entries, exact]);
  const pmax = max ?? Math.max(0.05, ...entries.map((e) => e.prob), ...(exact ?? []).map((e) => e.prob));
  const many = keys.length > 20;
  return (
    <div className="qx-hist" style={{ height }}>
      {keys.length === 0 && <div className="muted small">No outcomes yet.</div>}
      {keys.map((k) => {
        const e = entries.find((x) => x.bits === k);
        const x = exact?.find((y) => y.bits === k);
        const p = e?.prob ?? 0;
        return (
          <div key={k} className="qx-hbar" title={`${k}: ${e?.count ?? 0} shots (${(p * 100).toFixed(1)} %)${x ? `, exact ${(x.prob * 100).toFixed(2)} %` : ''}`}>
            <div className="qx-hcol">
              <div className="qx-hfill" style={{ height: `${(p / pmax) * 100}%` }} />
              {x && <div className="qx-hexact" style={{ bottom: `${(x.prob / pmax) * 100}%` }} />}
            </div>
            {!many && <div className="qx-hval tnum">{p >= 0.001 ? (p * 100).toFixed(p < 0.1 ? 1 : 0) : ''}</div>}
            <div className={`qx-hlabel mono ${many ? 'rot' : ''}`}>{k}</div>
          </div>
        );
      })}
    </div>
  );
}

/** Horizontal probability bars (used by Grover and the noise tab). */
export function ProbBars({ labels, values, compare, highlight, color }: { labels: string[]; values: number[]; compare?: number[]; highlight?: number; color?: string }) {
  return (
    <div className="qx-pbars">
      {labels.map((l, k) => (
        <div key={l} className={`qx-prow ${highlight === k ? 'hl' : ''}`}>
          <span className="mono small">{l}</span>
          <div className="qx-ptrack">
            <div className="qx-pfill" style={{ width: `${Math.min(100, values[k] * 100)}%`, background: highlight === k ? 'var(--series-3)' : color ?? 'var(--series-1)' }} />
            {compare && <div className="qx-pmark" style={{ left: `${Math.min(100, compare[k] * 100)}%` }} />}
          </div>
          <span className="tnum small">{(values[k] * 100).toFixed(1)}%</span>
        </div>
      ))}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Gauge
// ---------------------------------------------------------------------------

export function Gauge({ value, max, threshold, label, unit = '%', marks = [] }: { value: number; max: number; threshold?: number; label: string; unit?: string; marks?: { v: number; t: string }[] }) {
  const W = 200, H = 118, cx = 100, cy = 104, r = 80;
  const ang = (v: number) => Math.PI * (1 - Math.max(0, Math.min(1, v / max)));
  const pt = (v: number, rr = r) => [cx + rr * Math.cos(ang(v)), cy - rr * Math.sin(ang(v))];
  const arc = (a: number, b: number) => {
    const [x0, y0] = pt(a);
    const [x1, y1] = pt(b);
    return `M${x0} ${y0} A${r} ${r} 0 0 1 ${x1} ${y1}`;
  };
  const over = threshold !== undefined && value > threshold;
  const [nx, ny] = pt(value, r - 8);
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="qx-gauge" role="img" aria-label={`${label}: ${value.toFixed(1)} ${unit}`}>
      <path d={arc(0, threshold ?? max)} stroke="var(--good)" strokeWidth={12} fill="none" opacity={0.35} />
      {threshold !== undefined && <path d={arc(threshold, max)} stroke="var(--bad)" strokeWidth={12} fill="none" opacity={0.35} />}
      {marks.map((m) => {
        const [x0, y0] = pt(m.v, r + 7);
        const [x1, y1] = pt(m.v, r - 7);
        const [tx, ty] = pt(m.v, r + 15);
        return (
          <g key={m.t}>
            <line x1={x0} y1={y0} x2={x1} y2={y1} stroke="var(--text-2)" strokeWidth={1.2} />
            <text x={tx} y={ty} fontSize={8.5} textAnchor="middle" fill="var(--text-2)">{m.t}</text>
          </g>
        );
      })}
      <line x1={cx} y1={cy} x2={nx} y2={ny} stroke={over ? 'var(--bad)' : 'var(--text)'} strokeWidth={3} strokeLinecap="round" style={{ transition: 'all 0.4s ease' }} />
      <circle cx={cx} cy={cy} r={5} fill="var(--text)" />
      <text x={cx} y={cy - 26} textAnchor="middle" fontSize={18} fontWeight={750} fill={over ? 'var(--bad-text)' : 'var(--text)'}>{value.toFixed(1)}{unit}</text>
      <text x={cx} y={cy - 12} textAnchor="middle" fontSize={9} fill="var(--muted)">{label}</text>
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Bitmap of bits
// ---------------------------------------------------------------------------

export function BitCanvas({ bits, cols = 64, px = 4, title }: { bits: string; cols?: number; px?: number; title?: string }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const rows = Math.max(1, Math.ceil(bits.length / cols));
  useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const ctx = cv.getContext('2d');
    if (!ctx) return;
    const cs = getComputedStyle(cv);
    const on = cs.getPropertyValue('--text').trim() || '#eee';
    const off = cs.getPropertyValue('--surface-2').trim() || '#222';
    ctx.fillStyle = off;
    ctx.fillRect(0, 0, cv.width, cv.height);
    ctx.fillStyle = on;
    for (let k = 0; k < bits.length; k++) {
      if (bits.charCodeAt(k) === 49) ctx.fillRect((k % cols) * px, Math.floor(k / cols) * px, px, px);
    }
  }, [bits, cols, px]);
  return <canvas ref={ref} width={cols * px} height={rows * px} className="qx-bits" title={title} aria-label={title ?? 'bitmap of random bits'} />;
}

export function Kv({ rows }: { rows: [ReactNode, ReactNode][] }) {
  return (
    <div className="qx-kv">
      {rows.map(([k, v], i) => (
        <div key={i} className="qx-kv-row"><span className="muted">{k}</span><span className="tnum">{v}</span></div>
      ))}
    </div>
  );
}

export function Seg<T extends string | number>(props: { value: T; options: { value: T; label: ReactNode; title?: string }[]; onChange: (v: T) => void; label?: string }) {
  return (
    <div className="qx-seg-wrap">
      {props.label && <span className="field-label small">{props.label}</span>}
      <div className="qx-seg" role="radiogroup" aria-label={props.label}>
        {props.options.map((o) => (
          <button key={String(o.value)} role="radio" aria-checked={props.value === o.value} title={o.title} className={props.value === o.value ? 'on' : ''} onClick={() => props.onChange(o.value)}>{o.label}</button>
        ))}
      </div>
    </div>
  );
}

export const pct = (x: number, d = 1) => `${(x * 100).toFixed(d)} %`;
export const deg = (r: number) => `${((r * 180) / Math.PI).toFixed(0)}°`;

export function csv(header: string[], rows: (string | number)[][]) {
  return [header.join(','), ...rows.map((r) => r.join(','))].join('\n');
}
