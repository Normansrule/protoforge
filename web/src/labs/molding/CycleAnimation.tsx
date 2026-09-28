// Animated injection-molding cycle: clamp → inject → pack → cool → eject,
// with phase durations taken from the engine's cycle estimate.
import { useEffect, useState } from 'react';
import type { CycleBreakdown } from './types';
import { prefersReducedMotion } from '../pcb/zip';

const PHASES = [
  { key: 'clamp', label: 'Clamp', color: 'var(--series-7)' },
  { key: 'fill', label: 'Inject (fill)', color: 'var(--series-2)' },
  { key: 'pack', label: 'Pack / hold', color: 'var(--series-4)' },
  { key: 'cool', label: 'Cool', color: 'var(--series-1)' },
  { key: 'eject', label: 'Open & eject', color: 'var(--series-3)' },
] as const;

export function CycleAnimation({ cycle, speedup = 4 }: { cycle: CycleBreakdown; speedup?: number }) {
  // split the dry cycle into clamp (40 %) and open/eject (60 %)
  const durs = [cycle.open_eject_close_s * 0.4, cycle.fill_s, cycle.pack_s, cycle.cool_s, cycle.open_eject_close_s * 0.6];
  const total = durs.reduce((a, b) => a + b, 0);
  const [t, setT] = useState(0);
  const [playing, setPlaying] = useState(!prefersReducedMotion());
  useEffect(() => {
    if (!playing) return;
    let raf = 0;
    let last = performance.now();
    const tick = (now: number) => {
      const dt = (now - last) / 1000;
      last = now;
      setT((v) => (v + dt * speedup) % total);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [playing, total, speedup]);
  let acc = 0;
  let phase = 0;
  for (let i = 0; i < durs.length; i++) {
    if (t < acc + durs[i]) { phase = i; break; }
    acc += durs[i];
  }
  const f = durs[phase] > 0 ? (t - acc) / durs[phase] : 1;
  // geometry (SVG units)
  const closed = phase === 0 ? f : phase === 4 ? 1 - Math.min(1, f * 2) : 1;
  const gap = 36 * (1 - closed);
  const fill = phase === 1 ? f : phase >= 2 && phase <= 3 ? 1 : phase === 4 ? 1 : 0;
  const heat = phase === 3 ? 1 - f : phase === 1 || phase === 2 ? 1 : 0;
  const partOut = phase === 4 ? Math.max(0, f * 2 - 1) : 0;
  const screw = phase === 1 ? f : phase === 2 ? 1 : phase === 3 ? 1 - f : 0;
  return (
    <div>
      <svg viewBox="0 0 420 170" width="100%" style={{ display: 'block', maxHeight: 240 }} role="img" aria-label={`Molding cycle, phase ${PHASES[phase].label}`}>
        {/* barrel + screw */}
        <rect x={10} y={72} width={150} height={26} rx={4} fill="var(--surface-3)" stroke="var(--border-strong)" />
        <rect x={16 + 70 * screw} y={78} width={70} height={14} rx={3} fill="var(--muted)" />
        {[0, 1, 2, 3, 4].map((k) => <line key={k} x1={24 + 70 * screw + k * 13} y1={78} x2={30 + 70 * screw + k * 13} y2={92} stroke="var(--surface-1)" strokeWidth={2} />)}
        <rect x={160} y={80} width={20} height={10} fill={fill > 0 ? 'var(--series-2)' : 'var(--surface-3)'} opacity={fill > 0 ? 0.9 : 1} />
        {/* fixed half (cavity) */}
        <rect x={180} y={30} width={60} height={110} rx={4} fill="var(--surface-3)" stroke="var(--border-strong)" />
        {/* moving half (core) */}
        <g transform={`translate(${gap},0)`}>
          <rect x={240} y={30} width={60} height={110} rx={4} fill="var(--surface-2)" stroke="var(--border-strong)" />
          <rect x={300} y={70} width={100 - gap} height={30} fill="var(--surface-3)" stroke="var(--border-strong)" />
          {/* ejector pins */}
          {[55, 105].map((y) => <rect key={y} x={262 - 14 * partOut} y={y} width={24} height={4} fill="var(--muted)" />)}
        </g>
        {/* cavity / part */}
        <g transform={`translate(${gap * 0.5 + partOut * 30},${partOut * 20})`}>
          <rect x={222} y={48} width={36} height={74} rx={3} fill="none" stroke="var(--text-2)" strokeDasharray="3 3" />
          <rect x={222} y={48 + 74 * (1 - fill)} width={36} height={74 * fill} rx={3} fill={`color-mix(in srgb, var(--series-2) ${Math.round(heat * 100)}%, var(--series-1))`} opacity={0.9} />
        </g>
        {/* labels */}
        <text x={85} y={118} textAnchor="middle" fontSize={11} fill="var(--muted)">barrel & screw</text>
        <text x={196} y={24} textAnchor="middle" fontSize={11} fill="var(--muted)">fixed half</text>
        <text x={286 + gap} y={24} textAnchor="middle" fontSize={11} fill="var(--muted)">moving half</text>
        <text x={410} y={160} textAnchor="end" fontSize={13} fill="var(--text)" fontWeight={700}>{PHASES[phase].label} · {(t).toFixed(1)} / {total.toFixed(1)} s</text>
      </svg>
      <div style={{ display: 'flex', height: 14, borderRadius: 7, overflow: 'hidden', gap: 2, marginTop: 6 }} aria-hidden>
        {durs.map((d, i) => (
          <div key={i} title={`${PHASES[i].label}: ${d.toFixed(1)} s`} style={{ flex: d, background: PHASES[i].color, opacity: i === phase ? 1 : 0.45, position: 'relative' }}>
            {i === phase && <div style={{ position: 'absolute', left: `${f * 100}%`, top: 0, bottom: 0, width: 2, background: 'var(--text)' }} />}
          </div>
        ))}
      </div>
      <div className="row small" style={{ marginTop: 6, gap: 12 }}>
        {PHASES.map((p, i) => (
          <span key={p.key} className="row" style={{ gap: 5 }}>
            <span style={{ width: 10, height: 10, borderRadius: 3, background: p.color }} />
            {p.label} <span className="mono muted">{durs[i].toFixed(1)} s</span>
          </span>
        ))}
        <button className="btn small" style={{ marginLeft: 'auto' }} onClick={() => setPlaying(!playing)}>{playing ? 'Pause' : 'Play'}</button>
      </div>
      <div className="small muted" style={{ marginTop: 4 }}>Animation runs {speedup}× faster than real time.</div>
    </div>
  );
}
