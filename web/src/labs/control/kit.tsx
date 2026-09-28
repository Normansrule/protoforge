// Shared helpers for the Control, Robotics and RL labs: an animation clock
// with play/pause/scrub, CSV building, colour scales and small SVG bits.
import { useEffect, useRef, useState, type ReactNode } from 'react';
import { loadEngine, tryCall } from '../../engine/engine';

export function usePrefersReducedMotion(): boolean {
  const [reduced, setReduced] = useState(() => typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches);
  useEffect(() => {
    const mq = window.matchMedia?.('(prefers-reduced-motion: reduce)');
    if (!mq) return;
    const f = () => setReduced(mq.matches);
    mq.addEventListener('change', f);
    return () => mq.removeEventListener('change', f);
  }, []);
  return reduced;
}

export interface Clock {
  t: number;
  playing: boolean;
  speed: number;
  setT: (t: number) => void;
  setPlaying: (p: boolean) => void;
  setSpeed: (s: number) => void;
  restart: () => void;
  duration: number;
}

/**
 * requestAnimationFrame clock over [0, duration] seconds of simulated time.
 * Starts paused when the user prefers reduced motion.
 */
export function useClock(duration: number, opts: { loop?: boolean; speed?: number; autoplay?: boolean; resetKey?: unknown } = {}): Clock {
  const reduced = usePrefersReducedMotion();
  const [t, setT] = useState(0);
  const [playing, setPlaying] = useState(opts.autoplay !== false && !reduced);
  const [speed, setSpeed] = useState(opts.speed ?? 1);
  const last = useRef<number | null>(null);
  const loop = opts.loop ?? true;
  const durRef = useRef(duration);
  durRef.current = duration;

  const first = useRef(true);
  useEffect(() => {
    setT(0);
    if (first.current) {
      first.current = false;
      return;
    }
    if (opts.autoplay !== false && !reduced) setPlaying(true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [opts.resetKey]);

  useEffect(() => {
    if (!playing) {
      last.current = null;
      return;
    }
    let raf = 0;
    const tick = (now: number) => {
      if (last.current !== null) {
        const dt = Math.min(0.1, (now - last.current) / 1000) * speed;
        setT((prev) => {
          const d = durRef.current;
          let n = prev + dt;
          if (n > d) {
            if (loop) n = 0;
            else {
              n = d;
              setPlaying(false);
            }
          }
          return n;
        });
      }
      last.current = now;
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [playing, speed, loop]);

  return {
    t: Math.min(t, duration),
    playing,
    speed,
    setT,
    setPlaying,
    setSpeed,
    restart: () => {
      setT(0);
      setPlaying(true);
    },
    duration,
  };
}

export function PlayBar({ clock, speeds = [0.25, 0.5, 1, 2, 4], label, extra }: { clock: Clock; speeds?: number[]; label?: (t: number) => string; extra?: ReactNode }) {
  return (
    <div className="row" style={{ gap: 8, flexWrap: 'nowrap', width: '100%' }}>
      <button className="btn small" onClick={() => clock.setPlaying(!clock.playing)} aria-label={clock.playing ? 'Pause' : 'Play'} style={{ minWidth: 66, justifyContent: 'center' }}>
        {clock.playing ? '❚❚ Pause' : '▶ Play'}
      </button>
      <button className="btn small" onClick={clock.restart} aria-label="Restart">↺</button>
      <input
        type="range"
        aria-label="Time"
        min={0}
        max={1000}
        value={clock.duration > 0 ? Math.round((clock.t / clock.duration) * 1000) : 0}
        onChange={(e) => {
          clock.setPlaying(false);
          clock.setT((Number(e.target.value) / 1000) * clock.duration);
        }}
        style={{ flex: 1, minWidth: 60 }}
      />
      <span className="mono small tnum" style={{ minWidth: 64, textAlign: 'right' }}>{label ? label(clock.t) : `${clock.t.toFixed(1)} s`}</span>
      <select aria-label="Speed" value={clock.speed} onChange={(e) => clock.setSpeed(Number(e.target.value))} style={{ width: 64, padding: '3px 4px' }}>
        {speeds.map((s) => <option key={s} value={s}>{s}×</option>)}
      </select>
      {extra}
    </div>
  );
}

/** Index of the last sample with t[i] <= t (binary search, t ascending). */
export function frameAt(ts: number[], t: number): number {
  if (ts.length === 0) return 0;
  let lo = 0, hi = ts.length - 1;
  if (t <= ts[0]) return 0;
  if (t >= ts[hi]) return hi;
  while (hi - lo > 1) {
    const m = (lo + hi) >> 1;
    if (ts[m] <= t) lo = m; else hi = m;
  }
  return lo;
}

/** Linear interpolation of ys at time t over ascending ts. */
export function lerpAt(ts: number[], ys: number[], t: number): number {
  const i = frameAt(ts, t);
  if (i >= ts.length - 1) return ys[ys.length - 1] ?? 0;
  const f = (t - ts[i]) / (ts[i + 1] - ts[i] || 1);
  return ys[i] + (ys[i + 1] - ys[i]) * Math.min(1, Math.max(0, f));
}

export function toCSV(header: string[], cols: (number[] | string[])[]): string {
  const n = Math.max(0, ...cols.map((c) => c.length));
  const rows = [header.join(',')];
  for (let i = 0; i < n; i++) rows.push(cols.map((c) => { const v = c[i]; return typeof v === 'number' ? (Number.isFinite(v) ? String(+v.toPrecision(8)) : '') : String(v ?? ''); }).join(','));
  return rows.join('\n') + '\n';
}

export const json = (v: unknown) => JSON.stringify(v, null, 2);

/** Mix two CSS colours (supports var()) via color-mix. */
export const mix = (a: string, b: string, pct: number) => `color-mix(in srgb, ${a} ${Math.round(Math.max(0, Math.min(100, pct)))}%, ${b})`;

/** Sequential single-hue scale on the chart surface, 0 → 1. */
export function seqColor(f: number, hue = 'var(--series-1)') {
  return mix(hue, 'var(--surface-2)', 8 + 88 * Math.max(0, Math.min(1, f)));
}

/** Diverging scale: negative → series-2 (orange), 0 → neutral, positive → series-1 (blue). */
export function divColor(v: number, maxAbs: number) {
  const f = maxAbs > 0 ? Math.max(-1, Math.min(1, v / maxAbs)) : 0;
  if (Math.abs(f) < 1e-9) return 'var(--surface-3)';
  return f > 0 ? mix('var(--series-1)', 'var(--surface-3)', 12 + 80 * f) : mix('var(--series-2)', 'var(--surface-3)', 12 + 80 * -f);
}

export function Legend({ items }: { items: { label: string; color: string; dash?: boolean }[] }) {
  return (
    <div className="plot-legend" style={{ paddingLeft: 0 }}>
      {items.map((it) => (
        <span className="lg" key={it.label}>
          <span className="lg-swatch" style={{ background: it.dash ? 'transparent' : it.color, borderTop: it.dash ? `2px dashed ${it.color}` : undefined }} />
          {it.label}
        </span>
      ))}
    </div>
  );
}

export function fmt(v: number | null | undefined, digits = 2, unit = '') {
  if (v === null || v === undefined || !Number.isFinite(v)) return '—';
  return `${v.toFixed(digits)}${unit ? ' ' + unit : ''}`;
}

/** A small key/value table used for numeric readouts. */
export function KV({ rows }: { rows: [ReactNode, ReactNode][] }) {
  return (
    <table className="table" style={{ fontSize: 12.5 }}>
      <tbody>
        {rows.map(([k, v], i) => (
          <tr key={i}>
            <td className="muted" style={{ padding: '5px 8px' }}>{k}</td>
            <td className="mono tnum" style={{ padding: '5px 8px', textAlign: 'right' }}>{v}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

export interface Complex { re: number; im: number }

export function fmtComplex(z: Complex, d = 3) {
  if (Math.abs(z.im) < 1e-9) return z.re.toFixed(d);
  return `${z.re.toFixed(d)} ${z.im >= 0 ? '+' : '−'} ${Math.abs(z.im).toFixed(d)}j`;
}

/** Resolve CSS custom properties (for three.js materials / canvas), updating on theme change. */
export function useCssColors<K extends string>(names: readonly K[]): Record<K, string> {
  const read = () => {
    const cs = getComputedStyle(document.documentElement);
    const out = {} as Record<K, string>;
    for (const n of names) out[n] = cs.getPropertyValue(n.startsWith('--') ? n : `--${n}`).trim() || '#888888';
    return out;
  };
  const [colors, setColors] = useState<Record<K, string>>(read);
  useEffect(() => {
    const obs = new MutationObserver(() => setColors(read()));
    obs.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme', 'class', 'style'] });
    return () => obs.disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return colors;
}

/**
 * Like useCompute, but runs the (expensive) engine call after the inputs have
 * been stable for `delayMs`, so dragging sliders stays responsive. Keeps the
 * previous result while recomputing.
 */
export function useIdleCompute<T>(method: string, input: unknown, delayMs = 250, enabled = true): { data: T | null; error: string | null; busy: boolean; ms: number } {
  const key = JSON.stringify(input ?? null);
  const [state, setState] = useState<{ data: T | null; error: string | null; key: string | null; ms: number }>({ data: null, error: null, key: null, ms: 0 });
  useEffect(() => {
    if (!enabled) return;
    let cancelled = false;
    const id = window.setTimeout(async () => {
      await loadEngine();
      if (cancelled) return;
      const t0 = performance.now();
      const r = tryCall<T>(method, JSON.parse(key));
      if (!cancelled) setState({ data: r.data, error: r.error, key, ms: performance.now() - t0 });
    }, state.key === null ? 0 : delayMs);
    return () => {
      cancelled = true;
      window.clearTimeout(id);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [method, key, enabled]);
  return { data: state.data, error: state.error, busy: state.key !== key, ms: state.ms };
}
