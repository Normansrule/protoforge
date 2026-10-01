// Linkages: four-bar (Grashof class, Freudenstein position, velocity,
// acceleration, coupler curve, transmission angle, toggles) with draggable
// pivots and a crank-angle scrubber; offset slider-crank; three-position
// motion synthesis that loads straight into the analyser.
import { useMemo, useRef, useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, NumberField, Select, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import type { FourBarIn, FourBarOut, SliderOut, Synth3Out } from './types';
import { DEG, Kv, Notes, PlayBar, csvFrom, fmt, pathOf, useClock, useSvgDrag } from './util';

type Mode = 'fourbar' | 'slider' | 'synth';

const PRESETS: { id: string; label: string; v: Partial<FourBarIn> }[] = [
  { id: 'cr', label: 'Crank-rocker (Norton Ex. 4-1 sizes)', v: { a: 40, b: 120, c: 80, d: 100, p: 60, delta3_deg: 30, theta1_deg: 0, branch: 'open' } },
  { id: 'hoeken', label: 'Hoeken straight-line', v: { a: 20, b: 50, c: 50, d: 40, p: 100, delta3_deg: 0, theta1_deg: 0, branch: 'open' } },
  { id: 'dc', label: 'Double crank (drag link)', v: { a: 90, b: 110, c: 100, d: 40, p: 50, delta3_deg: 40, theta1_deg: 0, branch: 'open' } },
  { id: 'dr', label: 'Double rocker (coupler crank)', v: { a: 80, b: 30, c: 100, d: 90, p: 40, delta3_deg: 60, theta1_deg: 0, branch: 'open' } },
  { id: 'nong', label: 'Non-Grashof triple rocker', v: { a: 70, b: 90, c: 75, d: 100, p: 50, delta3_deg: 35, theta1_deg: 0, branch: 'open' } },
];

const DEFAULT_FB: FourBarIn = { a: 40, b: 120, c: 80, d: 100, o2: [0, 0], theta1_deg: 0, p: 60, delta3_deg: 30, branch: 'open', omega2: 10, alpha2: 0, samples: 360 };

export function LinkagesTab() {
  const [mode, setMode] = useState<Mode>('fourbar');
  const [fb, setFb] = useState<FourBarIn>(DEFAULT_FB);
  return (
    <div className="stack">
      <div className="mx-seg" role="tablist" aria-label="Linkage type">
        {([
          ['fourbar', 'Four-bar'],
          ['slider', 'Slider-crank'],
          ['synth', 'Three-position synthesis'],
        ] as [Mode, string][]).map(([id, l]) => (
          <button key={id} role="tab" aria-selected={mode === id} className={`btn small ${mode === id ? 'primary' : ''}`} onClick={() => setMode(id)}>
            {l}
          </button>
        ))}
      </div>
      {mode === 'fourbar' && <FourBar fb={fb} setFb={setFb} />}
      {mode === 'slider' && <SliderCrank />}
      {mode === 'synth' && (
        <Synthesis
          onLoad={(v) => {
            setFb({ ...fb, ...v });
            setMode('fourbar');
          }}
        />
      )}
    </div>
  );
}

// ------------------------------------------------------------------ four-bar

/** Contiguous run of valid samples containing `seed` (wrapping), as a list of indices. */
function validRun(valid: boolean[], seed: number): number[] {
  const n = valid.length;
  if (!n) return [];
  if (valid.every(Boolean)) return Array.from({ length: n }, (_, i) => i);
  let s = seed;
  if (!valid[s]) {
    s = valid.findIndex(Boolean);
    if (s < 0) return [];
  }
  let lo = s;
  while (valid[(lo - 1 + n) % n] && (lo - 1 + n) % n !== s) lo = (lo - 1 + n) % n;
  const out: number[] = [];
  let k = lo;
  for (let q = 0; q < n && valid[k]; q++) {
    out.push(k);
    k = (k + 1) % n;
  }
  return out;
}

function FourBar({ fb, setFb }: { fb: FourBarIn; setFb: (f: FourBarIn) => void }) {
  const { data: r, error } = useCompute<FourBarOut>('mechanisms.fourbar', fb);
  const [seed, setSeed] = useState(40);
  const run = useMemo(() => (r ? validRun(r.valid, seed) : []), [r, seed]);
  const full = r ? run.length === r.valid.length : true;
  // animation: full rotation at 90°/s, or ping-pong across the valid range
  const frames = useMemo(() => (full ? run : [...run, ...run.slice(1, -1).reverse()]), [run, full]);
  const duration = Math.max(1, (frames.length * (360 / Math.max(1, r?.valid.length ?? 360))) / 90);
  const clock = useClock(duration, { loop: true });
  const fi = frames.length ? Math.min(frames.length - 1, Math.floor((clock.t / duration) * frames.length)) : 0;
  const idx = frames[fi] ?? 0;
  const set = (q: Partial<FourBarIn>) => setFb({ ...fb, ...q });
  // stable view box (frozen while dragging)
  const frozen = useRef<[number, number, number, number] | null>(null);
  const drag = useSvgDrag((id, x, yv) => {
    const y = -yv;
    if (!r) return;
    const o2 = r.o2;
    const o4 = r.o4;
    if (id === 'o2') {
      const d = Math.hypot(o4[0] - x, o4[1] - y);
      if (d > 2) set({ o2: [x, y], d, theta1_deg: Math.atan2(o4[1] - y, o4[0] - x) / DEG });
    } else if (id === 'o4') {
      const d = Math.hypot(x - o2[0], y - o2[1]);
      if (d > 2) set({ d, theta1_deg: Math.atan2(y - o2[1], x - o2[0]) / DEG });
    } else if (id === 'a') {
      const a = Math.hypot(x - o2[0], y - o2[1]);
      const th = (((Math.atan2(y - o2[1], x - o2[0]) / DEG - fb.theta1_deg) % 360) + 360) % 360;
      if (a > 2) set({ a });
      const k = Math.round((th / 360) * r.valid.length) % r.valid.length;
      setSeed(k);
      const pos = frames.indexOf(k);
      clock.setPlaying(false);
      if (pos >= 0) clock.setT(((pos + 0.5) / frames.length) * duration);
    } else if (id === 'b') {
      const ax = r.ax[idx];
      const ay = r.ay[idx];
      const b = Math.hypot(x - ax, y - ay);
      const c = Math.hypot(x - o4[0], y - o4[1]);
      if (b > 2 && c > 2) set({ b, c });
    } else if (id === 'p') {
      const ax = r.ax[idx];
      const ay = r.ay[idx];
      const p = Math.hypot(x - ax, y - ay);
      const ang = Math.atan2(y - ay, x - ax) - Math.atan2(r.by[idx] - ay, r.bx[idx] - ax);
      set({ p, delta3_deg: ((ang / DEG + 540) % 360) - 180 });
    }
  });
  const vb = useMemo(() => {
    if (drag.dragging() && frozen.current) return frozen.current;
    if (!r) return [-60, -140, 260, 240] as [number, number, number, number];
    const xs = [r.o2[0], r.o4[0], ...r.px.filter((_, i) => r.valid[i]), ...r.ax, ...r.bx.filter((_, i) => r.valid[i])];
    const ys = [r.o2[1], r.o4[1], ...r.py.filter((_, i) => r.valid[i]), ...r.ay, ...r.by.filter((_, i) => r.valid[i])];
    const x0 = Math.min(...xs);
    const x1 = Math.max(...xs);
    const y0 = Math.min(...ys);
    const y1 = Math.max(...ys);
    const pad = 0.12 * Math.max(x1 - x0, y1 - y0, 40);
    const v: [number, number, number, number] = [x0 - pad, -(y1 + pad), x1 - x0 + 2 * pad, y1 - y0 + 2 * pad];
    frozen.current = v;
    return v;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [r, drag.dragging()]);
  const unit = Math.max(vb[2], vb[3]) / 100;
  const ok = r && r.valid[idx];
  const csv = () =>
    r
      ? csvFrom(
          ['theta2_deg', 'Px_mm', 'Py_mm', 'theta3_deg', 'theta4_deg', 'omega3_rad_s', 'omega4_rad_s', 'alpha3_rad_s2', 'alpha4_rad_s2', 'mu_deg', 'P_speed_mm_s', 'P_accel_mm_s2'],
          [r.theta2_deg, ...[r.px, r.py, r.theta3_deg, r.theta4_deg, r.omega3, r.omega4, r.alpha3, r.alpha4, r.mu_deg, r.p_speed, r.p_accel].map((c) => c.map((v, i) => (r.valid[i] ? v : NaN)))],
        )
      : '';
  const masked = (v: number[]) => (r ? v.map((x, i) => (r.valid[i] ? x : NaN)) : []);
  const vline = r ? [{ value: r.theta2_deg[idx], label: `θ₂ ${fmt(r.theta2_deg[idx], 0)}°` }] : [];
  const g = r?.grashof;
  return (
    <div className="mx-split">
      <div className="stack" style={{ minWidth: 0 }}>
        <Card
          title="Four-bar linkage"
          subtitle="Drag the pivots O₂ and O₄, the crank pin A, the rocker pin B or the coupler point P; scrub or play the crank."
          actions={r && <DownloadButton small filename="coupler-curve.csv" label="coupler curve CSV" content={csv} />}
        >
          <ErrorBox error={error} />
          {r && (
            <>
              <svg
                ref={drag.svgRef}
                className="mx-canvas"
                viewBox={vb.join(' ')}
                onPointerMove={drag.move}
                onPointerUp={drag.end}
                onPointerCancel={drag.end}
                role="img"
                aria-label="Animated four-bar linkage with its coupler curve"
                style={{ touchAction: 'none' }}
              >
                <path d={pathOf(masked(r.px), masked(r.py), full, (v) => v, (v) => -v)} className="mx-curve" strokeWidth={0.5 * unit} />
                <line x1={r.o2[0]} y1={-r.o2[1]} x2={r.o4[0]} y2={-r.o4[1]} className="mx-ground" strokeWidth={0.5 * unit} strokeDasharray={`${1.5 * unit} ${unit}`} />
                <Ground x={r.o2[0]} y={-r.o2[1]} u={unit} />
                <Ground x={r.o4[0]} y={-r.o4[1]} u={unit} />
                {ok && (
                  <>
                    <polygon
                      points={`${r.ax[idx]},${-r.ay[idx]} ${r.bx[idx]},${-r.by[idx]} ${r.px[idx]},${-r.py[idx]}`}
                      className="mx-coupler"
                      strokeWidth={0.5 * unit}
                    />
                    <line x1={r.o2[0]} y1={-r.o2[1]} x2={r.ax[idx]} y2={-r.ay[idx]} className="mx-link l1" strokeWidth={1.6 * unit} />
                    <line x1={r.o4[0]} y1={-r.o4[1]} x2={r.bx[idx]} y2={-r.by[idx]} className="mx-link l2" strokeWidth={1.6 * unit} />
                    <line x1={r.ax[idx]} y1={-r.ay[idx]} x2={r.bx[idx]} y2={-r.by[idx]} className="mx-link l3" strokeWidth={1.6 * unit} />
                    <MuArc r={r} i={idx} u={unit} />
                    <Handle x={r.ax[idx]} y={-r.ay[idx]} u={unit} label="A" onDown={drag.start('a')} />
                    <Handle x={r.bx[idx]} y={-r.by[idx]} u={unit} label="B" onDown={drag.start('b')} />
                    <Handle x={r.px[idx]} y={-r.py[idx]} u={unit} label="P" onDown={drag.start('p')} kind="p" />
                  </>
                )}
                <Handle x={r.o2[0]} y={-r.o2[1]} u={unit} label="O₂" onDown={drag.start('o2')} kind="g" />
                <Handle x={r.o4[0]} y={-r.o4[1]} u={unit} label="O₄" onDown={drag.start('o4')} kind="g" />
              </svg>
              <PlayBar clock={clock} label={() => `θ₂ ${fmt(r.theta2_deg[idx], 0)}°`} />
              <div className="grid grid-4" style={{ marginTop: 12 }}>
                <Stat label="Class" value={g?.code ?? '—'} note={g?.name} tone={g?.change_point ? 'warn' : undefined} />
                <Stat label="Transmission angle μ" value={`${fmt(r.mu_min_deg, 0)}–${fmt(r.mu_max_deg, 0)}`} unit="°" tone={r.mu_min_deg >= 40 ? 'pass' : 'warn'} note={r.mu_min_deg >= 40 ? '✓ ≥ 40° everywhere' : '! dips below 40°'} />
                <Stat label="Rocker swing" value={full ? fmt(r.rocker_max_deg - r.rocker_min_deg, 1) : '—'} unit="°" note={r.time_ratio > 0 ? `time ratio ${fmt(r.time_ratio, 3)}` : 'crank-rocker only'} />
                <Stat label="Now" value={ok ? `μ ${fmt(r.mu_deg[idx], 0)}°` : 'no assembly'} note={ok ? `ω₄ ${fmt(r.omega4[idx], 2)} rad/s` : 'outside the reachable range'} />
              </div>
            </>
          )}
        </Card>
        {r && (
          <div className="grid grid-2">
            <Card title="Positions" subtitle="Coupler θ₃ and rocker θ₄ against crank angle θ₂">
              <Plot series={[{ name: 'θ₃', x: r.theta2_deg, y: masked(r.theta3_deg) }, { name: 'θ₄', x: r.theta2_deg, y: masked(r.theta4_deg) }]} xLabel="θ₂, deg" yLabel="deg" legend vLines={vline} height={210} />
            </Card>
            <Card title="Transmission angle" subtitle="Keep μ above about 40° for smooth force transfer">
              <Plot series={[{ name: 'μ', x: r.theta2_deg, y: masked(r.mu_deg) }]} xLabel="θ₂, deg" yLabel="μ, deg" hLines={[{ value: 40, label: '40° guideline' }]} vLines={vline} height={210} yDomain={[0, 90]} />
            </Card>
            <Card title="Angular velocities" subtitle={`ω₂ = ${fb.omega2} rad/s`}>
              <Plot series={[{ name: 'ω₃', x: r.theta2_deg, y: masked(r.omega3) }, { name: 'ω₄', x: r.theta2_deg, y: masked(r.omega4) }]} xLabel="θ₂, deg" yLabel="rad/s" legend vLines={vline} height={210} />
            </Card>
            <Card title="Angular accelerations">
              <Plot series={[{ name: 'α₃', x: r.theta2_deg, y: masked(r.alpha3) }, { name: 'α₄', x: r.theta2_deg, y: masked(r.alpha4) }]} xLabel="θ₂, deg" yLabel="rad/s²" legend vLines={vline} height={210} />
            </Card>
          </div>
        )}
      </div>
      <div className="stack mx-side">
        <Card title="Links">
          <Select label="Preset" value="" options={[{ value: '', label: 'Choose a preset…' }, ...PRESETS.map((p) => ({ value: p.id, label: p.label }))]} onChange={(id) => { const p = PRESETS.find((x) => x.id === id); if (p) setFb({ ...fb, ...p.v, o2: [0, 0] }); }} />
          <Slider label="Crank a (link 2)" value={fb.a} min={5} max={200} step={0.5} unit="mm" onChange={(a) => set({ a })} />
          <Slider label="Coupler b (link 3)" value={fb.b} min={5} max={250} step={0.5} unit="mm" onChange={(b) => set({ b })} />
          <Slider label="Rocker c (link 4)" value={fb.c} min={5} max={250} step={0.5} unit="mm" onChange={(c) => set({ c })} />
          <Slider label="Ground d (link 1)" value={fb.d} min={5} max={250} step={0.5} unit="mm" onChange={(d) => set({ d })} />
          <Slider label="Coupler point distance p" value={fb.p} min={0} max={250} step={0.5} unit="mm" onChange={(p) => set({ p })} />
          <Slider label="Coupler point angle δ₃" value={fb.delta3_deg} min={-180} max={180} step={1} unit="°" onChange={(delta3_deg) => set({ delta3_deg })} />
          <Select label="Assembly branch" value={fb.branch} options={[{ value: 'open', label: 'Open' }, { value: 'crossed', label: 'Crossed' }]} onChange={(branch) => set({ branch })} />
          <NumberField label="Crank speed ω₂" unit="rad/s" value={fb.omega2} onChange={(omega2) => set({ omega2 })} />
        </Card>
        {g && (
          <Card title="Grashof condition">
            <Kv
              rows={[
                ['S + L', `${fmt(g.s)} + ${fmt(g.l)} = ${fmt(g.s_plus_l)}`],
                ['P + Q', `${fmt(g.p)} + ${fmt(g.q)} = ${fmt(g.p_plus_q)}`],
                ['Verdict', g.change_point ? '= change point' : g.grashof ? '≤ Grashof' : '> non-Grashof'],
              ]}
            />
            <p className="small" style={{ marginTop: 8 }}>
              <Chip tone={g.crank_full_rotation ? 'pass' : 'warn'}>{g.crank_full_rotation ? '✓ crank turns fully' : '! input rocks'}</Chip> {g.description}
            </p>
            {r && r.toggle_theta2_deg.length > 0 && <p className="small muted">Toggle positions of the input: θ₂ = {r.toggle_theta2_deg.map((t) => `${fmt(t, 1)}°`).join(', ')}.</p>}
            {r && <Notes notes={r.notes} />}
          </Card>
        )}
        <Explainer
          title="Vector loop and Freudenstein's equation"
          refs={[
            { label: 'F. Freudenstein, "Approximate synthesis of four-bar linkages", Trans. ASME 77, 1955' },
            { label: 'R. L. Norton, "Design of Machinery", ch. 2, 4, 6, 7' },
          ]}
        >
          <Formula>{`a e^{iθ₂} + b e^{iθ₃} − c e^{iθ₄} − d = 0
K₁ cos θ₄ − K₂ cos θ₂ + K₃ = cos(θ₂ − θ₄)
K₁ = d/a   K₂ = d/c   K₃ = (a² − b² + c² + d²)/(2ac)
θ₄ = 2 atan2(−B ∓ √(B² − 4AC), 2A)
ω₃ = a ω₂ sin(θ₄ − θ₂) / (b sin(θ₃ − θ₄))
Grashof: S + L ≤ P + Q`}</Formula>
          <p className="small">
            The loop closes for two assemblies (open and crossed). Differentiating it once gives the velocities, twice the accelerations. Where the square root goes negative the
            links cannot meet: that is a toggle limit, and the input must reverse.
          </p>
        </Explainer>
      </div>
    </div>
  );
}

function Ground({ x, y, u }: { x: number; y: number; u: number }) {
  const w = 5 * u;
  return (
    <g className="mx-groundsym">
      <path d={`M${x},${y} L${x - w / 2},${y + 3.5 * u} L${x + w / 2},${y + 3.5 * u} Z`} strokeWidth={0.4 * u} />
      {[0, 1, 2, 3].map((k) => (
        <line key={k} x1={x - w / 2 + (k * w) / 3.5} y1={y + 3.5 * u} x2={x - w / 2 + (k * w) / 3.5 - 1.2 * u} y2={y + 5 * u} strokeWidth={0.35 * u} />
      ))}
    </g>
  );
}

function Handle({ x, y, u, label, onDown, kind }: { x: number; y: number; u: number; label: string; onDown: (e: React.PointerEvent) => void; kind?: 'g' | 'p' }) {
  return (
    <g className={`mx-handle ${kind ?? ''}`} onPointerDown={onDown} tabIndex={-1}>
      <circle cx={x} cy={y} r={4.5 * u} className="mx-hit" />
      <circle cx={x} cy={y} r={1.6 * u} strokeWidth={0.45 * u} />
      <text x={x + 2.4 * u} y={y - 2.4 * u} fontSize={3.6 * u}>{label}</text>
    </g>
  );
}

function MuArc({ r, i, u }: { r: FourBarOut; i: number; u: number }) {
  const bx = r.bx[i];
  const by = r.by[i];
  const a1 = Math.atan2(r.ay[i] - by, r.ax[i] - bx);
  const a2 = Math.atan2(r.o4[1] - by, r.o4[0] - bx);
  let d = a2 - a1;
  while (d > Math.PI) d -= 2 * Math.PI;
  while (d < -Math.PI) d += 2 * Math.PI;
  const R = 7 * u;
  const p1 = [bx + R * Math.cos(a1), -(by + R * Math.sin(a1))];
  const p2 = [bx + R * Math.cos(a1 + d), -(by + R * Math.sin(a1 + d))];
  const mid = a1 + d / 2;
  return (
    <g className="mx-mu">
      <path d={`M${p1[0]},${p1[1]} A${R},${R} 0 0 ${d > 0 ? 0 : 1} ${p2[0]},${p2[1]}`} strokeWidth={0.4 * u} fill="none" />
      <text x={bx + (R + 2.5 * u) * Math.cos(mid)} y={-(by + (R + 2.5 * u) * Math.sin(mid))} fontSize={3.2 * u} textAnchor="middle">
        μ {fmt(r.mu_deg[i], 0)}°
      </text>
    </g>
  );
}

// ------------------------------------------------------------------ slider-crank

function SliderCrank() {
  const [s, setS] = useState({ r: 30, l: 100, e: 0, omega2: 10 });
  const { data: o, error } = useCompute<SliderOut>('mechanisms.slider_crank', { ...s, samples: 360 });
  const clock = useClock(4, { loop: true });
  const idx = o ? Math.min(o.x.length - 1, Math.floor((clock.t / 4) * o.x.length)) : 0;
  const th = (o?.theta2_deg[idx] ?? 0) * DEG;
  const A = [s.r * Math.cos(th), s.r * Math.sin(th)];
  const xB = o?.x[idx] ?? 0;
  const W = s.r + s.l + 30;
  const vb = [-s.r - 15, -Math.max(s.r, Math.abs(s.e)) - 25, W + 15, 2 * Math.max(s.r, Math.abs(s.e)) + 50];
  const u = vb[2] / 100;
  const vline = o ? [{ value: o.theta2_deg[idx] }] : [];
  const csv = () => (o ? csvFrom(['theta2_deg', 'x_mm', 'v_mm_s', 'a_mm_s2', 'rod_angle_deg'], [o.theta2_deg, o.x, o.v, o.a, o.theta3_deg]) : '');
  return (
    <div className="mx-split">
      <div className="stack" style={{ minWidth: 0 }}>
        <Card title="Offset slider-crank" subtitle="The engine and pump mechanism: x = r cos θ₂ + √(l² − (r sin θ₂ − e)²)" actions={o && <DownloadButton small filename="slider-crank.csv" label="CSV" content={csv} />}>
          <ErrorBox error={error} />
          {o && (
            <>
              <svg className="mx-canvas" viewBox={vb.join(' ')} role="img" aria-label="Animated slider-crank">
                <line x1={o.x_min - 10} y1={-s.e + 4.5 * u} x2={o.x_max + 14} y2={-s.e + 4.5 * u} className="mx-ground" strokeWidth={0.5 * u} />
                <line x1={o.x_min - 10} y1={-s.e - 4.5 * u} x2={o.x_max + 14} y2={-s.e - 4.5 * u} className="mx-ground" strokeWidth={0.5 * u} />
                <circle cx={0} cy={0} r={s.r} className="mx-ghost" strokeWidth={0.3 * u} />
                <Ground x={0} y={0} u={u} />
                <rect x={xB - 6 * u} y={-s.e - 4 * u} width={12 * u} height={8 * u} rx={u} className="mx-slider" strokeWidth={0.4 * u} />
                <line x1={0} y1={0} x2={A[0]} y2={-A[1]} className="mx-link l1" strokeWidth={1.8 * u} />
                <line x1={A[0]} y1={-A[1]} x2={xB} y2={-s.e} className="mx-link l3" strokeWidth={1.8 * u} />
                <circle cx={A[0]} cy={-A[1]} r={1.3 * u} className="mx-pin" />
                <circle cx={xB} cy={-s.e} r={1.3 * u} className="mx-pin" />
                <line x1={o.x_min} y1={-s.e + 9 * u} x2={o.x_max} y2={-s.e + 9 * u} className="mx-dim" strokeWidth={0.3 * u} />
                <text x={(o.x_min + o.x_max) / 2} y={-s.e + 13 * u} fontSize={3.4 * u} textAnchor="middle" className="mx-svgtext">stroke {fmt(o.stroke_mm, 1)} mm</text>
              </svg>
              <PlayBar clock={clock} label={() => `θ₂ ${fmt(o.theta2_deg[idx], 0)}°`} />
              <div className="grid grid-4" style={{ marginTop: 12 }}>
                <Stat label="Stroke" value={fmt(o.stroke_mm, 2)} unit="mm" note={s.e === 0 ? '= 2r for zero offset' : 'grows with offset'} />
                <Stat label="Time ratio" value={fmt(o.time_ratio, 3)} note="forward : return crank angle" />
                <Stat label="Peak acceleration" value={fmt(o.a_max / 1000, 2)} unit="m/s²" note={`r ω²(1 + r/l) ≈ ${fmt(o.a_approx / 1000, 2)}`} />
                <Stat label="Rod angle now" value={fmt(o.theta3_deg[idx], 1)} unit="°" />
              </div>
            </>
          )}
        </Card>
        {o && (
          <div className="grid grid-3">
            <Card title="Position"><Plot series={[{ name: 'x', x: o.theta2_deg, y: o.x }]} xLabel="θ₂, deg" yLabel="mm" vLines={vline} height={190} /></Card>
            <Card title="Velocity"><Plot series={[{ name: 'v', x: o.theta2_deg, y: o.v.map((v) => v / 1000) }]} xLabel="θ₂, deg" yLabel="m/s" vLines={vline} height={190} /></Card>
            <Card title="Acceleration"><Plot series={[{ name: 'a', x: o.theta2_deg, y: o.a.map((v) => v / 1000) }]} xLabel="θ₂, deg" yLabel="m/s²" vLines={vline} height={190} /></Card>
          </div>
        )}
      </div>
      <div className="stack mx-side">
        <Card title="Dimensions">
          <Slider label="Crank r" value={s.r} min={5} max={80} step={0.5} unit="mm" onChange={(r) => setS({ ...s, r })} />
          <Slider label="Connecting rod l" value={s.l} min={20} max={300} step={1} unit="mm" onChange={(l) => setS({ ...s, l })} />
          <Slider label="Offset e" value={s.e} min={-40} max={40} step={0.5} unit="mm" onChange={(e) => setS({ ...s, e })} />
          <NumberField label="Crank speed ω₂" unit="rad/s" value={s.omega2} onChange={(omega2) => setS({ ...s, omega2 })} />
          {o && <Notes notes={o.notes} />}
        </Card>
        <Explainer title="Exact, not the textbook series" refs={[{ label: 'R. L. Norton, "Design of Machinery", §4.6, §13 (slider-crank dynamics)' }]}>
          <Formula>{`x(θ) = r cos θ + √(l² − (r sin θ − e)²)
v = ω dx/dθ,   a = ω² d²x/dθ²
a_max ≈ r ω² (1 + r/l)   (e = 0)`}</Formula>
          <p className="small">The engine derivatives are exact; the classic approximation drops higher harmonics of r/l and is shown only for comparison.</p>
        </Explainer>
      </div>
    </div>
  );
}

// ------------------------------------------------------------------ synthesis

interface SynthState { p: [number, number][]; theta_deg: number[]; beta_deg: [number, number]; gamma_deg: [number, number] }

function Synthesis({ onLoad }: { onLoad: (v: Partial<FourBarIn>) => void }) {
  const [st, setSt] = useState<SynthState>({ p: [[0, 0], [40, 20], [80, 10]], theta_deg: [0, -20, -45], beta_deg: [40, 90], gamma_deg: [-25, -60] });
  const { data: s, error } = useCompute<Synth3Out>('mechanisms.synthesis3', st);
  const drag = useSvgDrag((id, x, y) => {
    const k = Number(id);
    const p = st.p.map((q, i) => (i === k ? [x, -y] : q)) as [number, number][];
    setSt({ ...st, p });
  });
  const fbIn = s ? { a: s.a, b: s.b, c: s.c, d: s.d, o2: s.o2, theta1_deg: s.theta1_deg, p: s.p, delta3_deg: s.delta3_deg, branch: s.branch, omega2: 10, alpha2: 0, samples: 360 } : null;
  const { data: fb } = useCompute<FourBarOut>('mechanisms.fourbar', fbIn ?? {}, !!fbIn);
  const frozen = useRef<number[] | null>(null);
  const vb = useMemo(() => {
    if (drag.dragging() && frozen.current) return frozen.current;
    const xs = st.p.map((q) => q[0]);
    const ys = st.p.map((q) => q[1]);
    if (s) {
      xs.push(s.o2[0], s.o4[0], s.a1[0], s.b1[0]);
      ys.push(s.o2[1], s.o4[1], s.a1[1], s.b1[1]);
      if (fb) {
        for (const t of s.theta2_deg) {
          const k = Math.round(t) % 360;
          if (fb.valid[k]) {
            xs.push(fb.ax[k], fb.bx[k]);
            ys.push(fb.ay[k], fb.by[k]);
          }
        }
      }
    }
    const x0 = Math.min(...xs), x1 = Math.max(...xs), y0 = Math.min(...ys), y1 = Math.max(...ys);
    const pad = 0.15 * Math.max(x1 - x0, y1 - y0, 60);
    const v = [x0 - pad, -(y1 + pad), x1 - x0 + 2 * pad, y1 - y0 + 2 * pad];
    frozen.current = v;
    return v;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [st, s, fb, drag.dragging()]);
  const u = Math.max(vb[2], vb[3]) / 100;
  const poseIdx = s && fb ? s.theta2_deg.map((t) => Math.round(t) % 360) : [];
  return (
    <div className="mx-split">
      <div className="stack" style={{ minWidth: 0 }}>
        <Card title="Three-position motion generation" subtitle="Drag the three coupler points; set each pose angle and the free choices. The synthesised four-bar carries the coupler through all three.">
          <ErrorBox error={error} />
          <svg ref={drag.svgRef} className="mx-canvas" viewBox={vb.join(' ')} onPointerMove={drag.move} onPointerUp={drag.end} onPointerCancel={drag.end} style={{ touchAction: 'none' }} role="img" aria-label="Three-position synthesis">
            {fb && <path d={pathOf(fb.px.map((v, i) => (fb.valid[i] ? v : NaN)), fb.py, false, (v) => v, (v) => -v)} className="mx-curve" strokeWidth={0.4 * u} />}
            {s && fb &&
              poseIdx.map((k, q) =>
                fb.valid[k] ? (
                  <g key={q} opacity={0.55}>
                    <line x1={s.o2[0]} y1={-s.o2[1]} x2={fb.ax[k]} y2={-fb.ay[k]} className="mx-link l1" strokeWidth={1.1 * u} />
                    <line x1={s.o4[0]} y1={-s.o4[1]} x2={fb.bx[k]} y2={-fb.by[k]} className="mx-link l2" strokeWidth={1.1 * u} />
                    <polygon points={`${fb.ax[k]},${-fb.ay[k]} ${fb.bx[k]},${-fb.by[k]} ${fb.px[k]},${-fb.py[k]}`} className="mx-coupler" strokeWidth={0.4 * u} />
                  </g>
                ) : null,
              )}
            {s && (
              <>
                <Ground x={s.o2[0]} y={-s.o2[1]} u={u} />
                <Ground x={s.o4[0]} y={-s.o4[1]} u={u} />
                <circle cx={s.o2[0]} cy={-s.o2[1]} r={1.4 * u} className="mx-pin" />
                <circle cx={s.o4[0]} cy={-s.o4[1]} r={1.4 * u} className="mx-pin" />
              </>
            )}
            {st.p.map((q, k) => {
              const t = st.theta_deg[k] * DEG;
              const L = 10 * u;
              return (
                <g key={k}>
                  <line x1={q[0]} y1={-q[1]} x2={q[0] + L * Math.cos(t)} y2={-(q[1] + L * Math.sin(t))} className="mx-pose" strokeWidth={0.8 * u} />
                  <Handle x={q[0]} y={-q[1]} u={u} label={`P${k + 1}`} onDown={drag.start(String(k))} kind="p" />
                </g>
              );
            })}
          </svg>
          {s && (
            <div className="grid grid-4" style={{ marginTop: 12 }}>
              <Stat label="Links a / b / c / d" value={`${fmt(s.a, 1)} / ${fmt(s.b, 1)} / ${fmt(s.c, 1)} / ${fmt(s.d, 1)}`} unit="mm" />
              <Stat label="Class" value={s.grashof.code} note={s.grashof.name} />
              <Stat label="Branch" value={s.branch} />
              <Stat label="Crank at P1 / P2 / P3" value={s.theta2_deg.map((t) => fmt(t, 0)).join(' / ')} unit="°" />
            </div>
          )}
          {s && <Notes notes={s.notes} />}
          {fbIn && (
            <button className="btn primary" style={{ marginTop: 10 }} onClick={() => onLoad(fbIn as Partial<FourBarIn>)}>
              Analyse this linkage →
            </button>
          )}
        </Card>
      </div>
      <div className="stack mx-side">
        <Card title="Poses and free choices">
          {[0, 1, 2].map((k) => (
            <Slider key={k} label={`Coupler angle at P${k + 1}`} value={st.theta_deg[k]} min={-180} max={180} step={1} unit="°" onChange={(v) => setSt({ ...st, theta_deg: st.theta_deg.map((x, i) => (i === k ? v : x)) })} />
          ))}
          <Slider label="Crank rotation β₂ (P1→P2)" value={st.beta_deg[0]} min={-180} max={180} step={1} unit="°" onChange={(v) => setSt({ ...st, beta_deg: [v, st.beta_deg[1]] })} />
          <Slider label="Crank rotation β₃ (P1→P3)" value={st.beta_deg[1]} min={-180} max={180} step={1} unit="°" onChange={(v) => setSt({ ...st, beta_deg: [st.beta_deg[0], v] })} />
          <Slider label="Rocker rotation γ₂" value={st.gamma_deg[0]} min={-180} max={180} step={1} unit="°" onChange={(v) => setSt({ ...st, gamma_deg: [v, st.gamma_deg[1]] })} />
          <Slider label="Rocker rotation γ₃" value={st.gamma_deg[1]} min={-180} max={180} step={1} unit="°" onChange={(v) => setSt({ ...st, gamma_deg: [st.gamma_deg[0], v] })} />
        </Card>
        <Explainer title="Dyad equations (Norton §5.8)" refs={[{ label: 'R. L. Norton, "Design of Machinery", ch. 5' }, { label: 'G. N. Sandor & A. G. Erdman, "Advanced Mechanism Design", 1984' }]}>
          <Formula>{`W(e^{iβ₂} − 1) + Z(e^{iα₂} − 1) = δ₂
W(e^{iβ₃} − 1) + Z(e^{iα₃} − 1) = δ₃
O₂ = P₁ − Z − W,   O₄ = P₁ − S − U`}</Formula>
          <p className="small">
            Each dyad (crank W + coupler arm Z, rocker U + arm S) is two complex linear equations. Pick the crank and rocker rotations freely — each choice gives a different linkage. Watch for
            branch defects (a position on the other assembly) and toggles between positions.
          </p>
        </Explainer>
      </div>
    </div>
  );
}
