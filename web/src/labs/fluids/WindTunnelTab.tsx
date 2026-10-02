// Wind tunnel: a D2Q9 lattice-Boltzmann (LBM) session lives in the Rust
// engine; every animation frame advances it a few steps and returns a
// quantised field (base64 bytes) plus tracer positions, drawn on a canvas.
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { call, tryCall } from '../../engine/engine';
import { useCompute, useEngineReady } from '../../engine/hooks';
import { useElementWidth } from '../../components/useElementWidth';
import { useTheme } from '../cad/viewer';
import type { LbmConfig, LbmInitOut, LbmPreset, LbmStepOut } from './types';
import { DIV_DARK, DIV_LIGHT, SPEED_DARK, SPEED_LIGHT, b64Bytes, fmt, gradientCss, lut, prefersReducedMotion, toCsv, useRaf } from './util';

type Field = 'vorticity' | 'speed' | 'pressure' | 'ux';
type Tool = 'none' | 'draw' | 'erase';

const RES: Record<string, [number, number, number]> = {
  coarse: [240, 84, 14],
  medium: [320, 112, 20],
  fine: [440, 154, 28],
};

const FIELD_LABEL: Record<Field, string> = {
  vorticity: 'vorticity ω (blue: clockwise, orange: counter-clockwise)',
  speed: 'speed |u|',
  pressure: 'pressure p − p∞',
  ux: 'streamwise velocity uₓ',
};

export function WindTunnelTab() {
  const ready = useEngineReady();
  const theme = useTheme();
  const presets = useCompute<LbmPreset[]>('fluids.lbm_presets', {}).data ?? [];
  const [presetId, setPresetId] = useState('karman');
  const preset = presets.find((p) => p.id === presetId);
  const [res, setRes] = useState<'coarse' | 'medium' | 'fine'>('medium');
  const [re, setRe] = useState<number | null>(null);
  const [angle, setAngle] = useState<number | null>(null);
  const [naca, setNaca] = useState<string | null>(null);
  const [field, setField] = useState<Field>('vorticity');
  const [contrast, setContrast] = useState(1);
  const [spf, setSpf] = useState(16);
  const [tracers, setTracers] = useState(true);
  const [tool, setTool] = useState<Tool>('none');
  const [brush, setBrush] = useState(3);
  const [playing, setPlaying] = useState(!prefersReducedMotion());
  const [error, setError] = useState<string | null>(null);
  const [session, setSession] = useState<LbmInitOut | null>(null);
  const [last, setLast] = useState<LbmStepOut | null>(null);
  const [hist, setHist] = useState<LbmStepOut['history']>(null);
  const [spec, setSpec] = useState<LbmStepOut['spectrum']>(null);
  const [rate, setRate] = useState(0);
  const [resetN, setResetN] = useState(0);
  const maskRef = useRef<Uint8Array | null>(null);
  const frameRef = useRef<Uint8Array | null>(null);
  const tracerRef = useRef<number[]>([]);
  const counter = useRef(0);
  const sessRef = useRef<number | null>(null);
  const [wrapRef, width] = useElementWidth<HTMLDivElement>();
  const canvas = useRef<HTMLCanvasElement>(null);
  const off = useRef<HTMLCanvasElement | null>(null);

  const config: LbmConfig | null = useMemo(() => {
    if (!preset) return null;
    const c = { ...preset.config };
    // resolution: keep the preset's geometry fractions, scale cells
    const isBench = preset.id.startsWith('schafer');
    if (!isBench) {
      const [nx, ny, d] = RES[res];
      const scale = d / 20;
      c.nx = nx;
      c.ny = ny;
      c.size_cells = preset.config.size_cells * scale;
    } else {
      const k = res === 'coarse' ? 0.5 : res === 'fine' ? 1.4 : 1;
      c.nx = Math.round(preset.config.nx * k);
      c.ny = Math.round(preset.config.ny * k);
      c.size_cells = preset.config.size_cells * k;
    }
    if (re != null) c.re = re;
    if (angle != null) c.angle_deg = angle;
    if (naca != null) c.naca = naca;
    c.tracers = 600;
    return c;
  }, [preset, res, re, angle, naca]);

  // (re)start a session whenever the configuration changes
  useEffect(() => {
    if (!ready || !config) return;
    const r = tryCall<LbmInitOut>('fluids.lbm_init', { config, replace: sessRef.current });
    if (r.error || !r.data) {
      setError(r.error);
      return;
    }
    setError(null);
    sessRef.current = r.data.session;
    setSession(r.data);
    maskRef.current = b64Bytes(r.data.mask);
    frameRef.current = null;
    setHist(null);
    setSpec(null);
    setLast(null);
    counter.current = 0;
    // first frame
    step(0, true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready, config, resetN]);

  const draw = useCallback(() => {
    const cv = canvas.current;
    const s = session;
    const f = frameRef.current;
    if (!cv || !s || !f) return;
    const { nx, ny } = s.info;
    if (!off.current) off.current = document.createElement('canvas');
    const o = off.current;
    o.width = nx;
    o.height = ny;
    const g = o.getContext('2d');
    if (!g) return;
    const img = g.createImageData(nx, ny);
    const dark = theme === 'dark';
    const table = field === 'speed' ? lut(dark ? SPEED_DARK : SPEED_LIGHT) : lut(dark ? DIV_DARK : DIV_LIGHT);
    const mask = maskRef.current;
    const solid = dark ? [214, 214, 208] : [60, 60, 58];
    for (let j = 0; j < ny; j++) {
      const row = ny - 1 - j;
      for (let i = 0; i < nx; i++) {
        const k = i + nx * j;
        const p = 4 * (i + nx * row);
        if (mask && mask[k]) {
          img.data[p] = solid[0];
          img.data[p + 1] = solid[1];
          img.data[p + 2] = solid[2];
        } else {
          const q = 4 * f[k];
          img.data[p] = table[q];
          img.data[p + 1] = table[q + 1];
          img.data[p + 2] = table[q + 2];
        }
        img.data[p + 3] = 255;
      }
    }
    g.putImageData(img, 0, 0);
    const W = Math.max(200, width || 600);
    const H = Math.round((W * ny) / nx);
    const dpr = Math.min(2, window.devicePixelRatio || 1);
    if (cv.width !== Math.round(W * dpr)) {
      cv.width = Math.round(W * dpr);
      cv.height = Math.round(H * dpr);
    }
    cv.style.width = `${W}px`;
    cv.style.height = `${H}px`;
    const c = cv.getContext('2d');
    if (!c) return;
    c.setTransform(dpr, 0, 0, dpr, 0, 0);
    c.imageSmoothingEnabled = true;
    c.drawImage(o, 0, 0, W, H);
    const sx = W / nx;
    const sy = H / ny;
    if (tracers && tracerRef.current.length) {
      c.fillStyle = dark ? 'rgba(255,255,255,0.75)' : 'rgba(11,11,11,0.7)';
      const t = tracerRef.current;
      for (let k = 0; k < t.length; k += 2) c.fillRect(t[k] * sx - 0.8, (ny - t[k + 1]) * sy - 0.8, 1.6, 1.6);
    }
    // probe marker
    const [px, py] = s.info.probe;
    c.strokeStyle = dark ? '#ffffff' : '#0b0b0b';
    c.lineWidth = 1.2;
    c.beginPath();
    c.arc((px + 0.5) * sx, (ny - py - 0.5) * sy, 4, 0, Math.PI * 2);
    c.stroke();
  }, [session, theme, field, tracers, width]);

  function step(n: number, withFrame = true) {
    const id = sessRef.current;
    if (id == null) return;
    counter.current++;
    const wantHist = counter.current % 12 === 1;
    const wantSpec = counter.current % 30 === 1;
    const r = tryCall<LbmStepOut>('fluids.lbm_step', {
      session: id,
      steps: n,
      field,
      contrast,
      tracers,
      frame: withFrame,
      history_points: wantHist ? 400 : 0,
      spectrum: wantSpec,
    });
    if (r.error || !r.data) {
      setError(r.error);
      setPlaying(false);
      return;
    }
    const d = r.data;
    if (d.frame) frameRef.current = b64Bytes(d.frame);
    tracerRef.current = d.tracers;
    setLast(d);
    if (d.history) setHist(d.history);
    if (d.spectrum) setSpec(d.spectrum);
    if (d.unstable) {
      setPlaying(false);
      setError('The simulation went unstable (τ too close to ½ or the flow too fast for the grid). Lower Re, raise the resolution, or reset.');
    }
  }

  const rateAcc = useRef({ t: 0, steps: 0 });
  useRaf(playing && !!session && !error, (dt) => {
    step(spf);
    draw();
    rateAcc.current.t += dt;
    rateAcc.current.steps += spf;
    if (rateAcc.current.t > 1) {
      setRate(rateAcc.current.steps / rateAcc.current.t);
      rateAcc.current = { t: 0, steps: 0 };
    }
  });
  // redraw when paused and something visual changes
  useEffect(() => {
    if (!playing && sessRef.current != null) {
      step(0);
      draw();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [field, contrast, tracers, theme, width, session]);
  useEffect(() => {
    draw();
  }, [draw, last]);

  // painting obstacles
  const painting = useRef(false);
  const paintAt = (ev: React.PointerEvent<HTMLCanvasElement>) => {
    const s = session;
    if (!s || tool === 'none' || sessRef.current == null) return;
    const r = ev.currentTarget.getBoundingClientRect();
    const x = ((ev.clientX - r.left) / r.width) * s.info.nx;
    const y = (1 - (ev.clientY - r.top) / r.height) * s.info.ny;
    try {
      const out = call<{ mask: string; solid_cells: number }>('fluids.lbm_paint', { session: sessRef.current, points: [[x, y]], radius: brush, solid: tool === 'draw' });
      maskRef.current = b64Bytes(out.mask);
      if (!playing) draw();
    } catch (e) {
      setError(String(e));
    }
  };
  const clearAll = () => {
    if (sessRef.current == null) return;
    const out = call<{ mask: string }>('fluids.lbm_paint', { session: sessRef.current, clear: true, points: [] });
    maskRef.current = b64Bytes(out.mask);
    draw();
  };

  const info = session?.info;
  const st = spec?.st ?? null;
  const ramp = field === 'speed' ? (theme === 'dark' ? SPEED_DARK : SPEED_LIGHT) : theme === 'dark' ? DIV_DARK : DIV_LIGHT;
  const png = () => {
    const cv = canvas.current;
    if (!cv) return new Blob();
    const data = atob(cv.toDataURL('image/png').split(',')[1]);
    const u8 = new Uint8Array(data.length);
    for (let i = 0; i < data.length; i++) u8[i] = data.charCodeAt(i);
    return new Blob([u8], { type: 'image/png' });
  };
  return (
    <div className="stack">
      <Card
        title={preset?.label ?? 'Wind tunnel'}
        subtitle={preset?.description}
        actions={
          <div className="row">
            <button className="btn small primary" onClick={() => setPlaying((p) => !p)} aria-pressed={playing} disabled={!!error && !session}>{playing ? '❚❚ Pause' : '▶ Run'}</button>
            <button className="btn small" onClick={() => { setPlaying(false); step(spf); draw(); }}>Step ▸</button>
            <button className="btn small" onClick={() => { setError(null); setResetN((n) => n + 1); }}>↺ Reset</button>
          </div>
        }
      >
        <div className="fl-presets">
          {presets.map((p) => (
            <button key={p.id} className={`st-preset ${p.id === presetId ? 'active' : ''}`} onClick={() => { setPresetId(p.id); setRe(null); setAngle(null); setNaca(null); setError(null); }}>
              <b className="small">{p.label}</b>
            </button>
          ))}
        </div>
        <ErrorBox error={error} />
        <div ref={wrapRef} style={{ position: 'relative', width: '100%', marginTop: 12 }}>
          <canvas
            ref={canvas}
            className={`fl-tunnel ${tool !== 'none' ? 'painting' : ''}`}
            onPointerDown={(e) => { if (tool === 'none') return; painting.current = true; e.currentTarget.setPointerCapture(e.pointerId); paintAt(e); }}
            onPointerMove={(e) => { if (painting.current) paintAt(e); }}
            onPointerUp={() => { painting.current = false; }}
            onPointerLeave={() => { painting.current = false; }}
            role="img"
            aria-label={`lattice-Boltzmann flow field: ${FIELD_LABEL[field]}`}
          />
          <div className="fl-flowtag small">flow →</div>
        </div>
        <div className="row small" style={{ justifyContent: 'space-between', marginTop: 6 }}>
          <span className="row" style={{ gap: 6 }}>
            <span className="fl-ramp" style={{ background: gradientCss(ramp) }} />
            <span className="muted">{FIELD_LABEL[field]}</span>
          </span>
          <span className="muted">○ probe for the Strouhal number · {info ? `${info.nx} × ${info.ny} cells` : ''} · {rate > 0 ? `${(rate / 1000).toFixed(1)}k steps/s` : ''}</span>
        </div>
        <div className="cp-toolbar">
          <div className="cp-seg" role="group" aria-label="Field">
            {(['vorticity', 'speed', 'pressure', 'ux'] as Field[]).map((f) => (
              <button key={f} className={`cp-seg-btn ${field === f ? 'active' : ''}`} onClick={() => setField(f)}>{f === 'ux' ? 'uₓ' : f}</button>
            ))}
          </div>
          <div className="cp-seg" role="group" aria-label="Resolution">
            {(['coarse', 'medium', 'fine'] as const).map((r) => (
              <button key={r} className={`cp-seg-btn ${res === r ? 'active' : ''}`} onClick={() => setRes(r)}>{r}</button>
            ))}
          </div>
          <div className="cp-seg" role="group" aria-label="Draw obstacles">
            {(['none', 'draw', 'erase'] as Tool[]).map((t) => (
              <button key={t} className={`cp-seg-btn ${tool === t ? 'active' : ''}`} onClick={() => setTool(t)}>{t === 'none' ? 'look' : t === 'draw' ? '✎ draw' : '⌫ erase'}</button>
            ))}
          </div>
          {tool !== 'none' && <button className="btn small" onClick={clearAll}>Clear obstacles</button>}
          <label className="cp-check"><input type="checkbox" checked={tracers} onChange={(e) => setTracers(e.target.checked)} /> particle tracers</label>
        </div>
        <div className="grid grid-3" style={{ marginTop: 10 }}>
          <Slider label="Reynolds number Re = U D / ν" value={re ?? config?.re ?? 150} min={5} max={1000} log format={(v) => v.toFixed(0)} onChange={(v) => setRe(Math.round(v))} />
          <Slider label="Steps per frame" value={spf} min={1} max={60} step={1} onChange={(v) => setSpf(Math.round(v))} />
          <Slider label="Colour contrast" value={contrast} min={0.3} max={4} step={0.05} onChange={setContrast} />
          {tool !== 'none' && <Slider label="Brush radius" value={brush} min={1} max={12} step={0.5} unit="cells" onChange={setBrush} />}
          {(preset?.config.obstacle === 'naca' || preset?.config.obstacle === 'rectangle' || preset?.config.obstacle === 'temple' || preset?.config.obstacle === 'pod') && (
            <Slider label="Angle of attack" value={angle ?? preset?.config.angle_deg ?? 0} min={-20} max={30} step={1} unit="°" onChange={(v) => setAngle(Math.round(v))} />
          )}
          {preset?.config.obstacle === 'naca' && (
            <Select label="NACA 4-digit section" value={naca ?? preset.config.naca} options={['0012', '2412', '4412', '0018', '6409'].map((x) => ({ value: x, label: `NACA ${x}` }))} onChange={setNaca} />
          )}
        </div>
      </Card>

      {info && last && (
        <div className="grid grid-4">
          <Stat label="Reynolds number" value={fmt(info.re, 0)} note={`τ = ${info.tau.toFixed(3)}, ν = ${info.nu.toFixed(4)}, Mach ${info.mach.toFixed(2)}`} tone={info.re < info.re_requested - 0.5 ? 'warn' : undefined} />
          <Stat label="Drag coefficient C_D" value={fmt(last.cd_mean, 3)} note={`mean of the last 4000 steps · now ${fmt(last.cd, 3)}`} />
          <Stat label="Lift C_L" value={`${fmt(last.cl_mean, 3)}`} note={`± ${fmt(last.cl_amp, 3)} oscillation`} />
          <Stat label="Strouhal number" value={st != null ? fmt(st, 3) : '—'} note={spec?.note || (spec ? `${spec.periods.toFixed(1)} periods in the record` : 'needs ≥ 512 steps')} />
        </div>
      )}
      {info && (info.warnings.length > 0 || preset?.reference) && (
        <div className="stack" style={{ gap: 6 }}>
          {preset?.reference && <div className="callout small"><b>Reference: </b>{preset.reference}</div>}
          {info.warnings.map((w, i) => <div key={i} className="callout small" style={{ borderLeftColor: 'var(--warn)' }}>! {w}</div>)}
          {info.re_walking != null && (
            <div className="small muted">
              The real {preset?.config.obstacle === 'pod' ? 'pod' : 'temple'} ({fmt(info.real_size_mm, 1)} mm across) at walking speed (1.4 m/s) has Re ≈ {fmt(info.re_walking, 0)} — {info.re_walking > 400 ? 'higher than this grid can resolve with BGK; the wake is turbulent-ish in 3-D anyway, so treat this 2-D picture as qualitative' : 'within reach of this tunnel'}.
            </div>
          )}
        </div>
      )}

      <div className="grid grid-2">
        <Card title="Force history" subtitle="Momentum exchange on every fluid–solid link, normalised by ½ ρ U² D">
          {hist ? (
            <Plot
              series={[
                { name: 'C_D (drag)', x: hist.step, y: hist.cd },
                { name: 'C_L (lift)', x: hist.step, y: hist.cl },
              ]}
              xLabel="time step"
              height={230}
              legend
            />
          ) : (
            <div className="muted small">Press Run.</div>
          )}
          {hist && (
            <DownloadButton
              filename="lbm-forces.csv"
              small
              content={() => toCsv([['step', 'cd', 'cl', 'probe_uy_over_U'], ...hist.step.map((s, i) => [s, +hist.cd[i].toFixed(5), +hist.cl[i].toFixed(5), +hist.probe[i].toFixed(5)])])}
            />
          )}
        </Card>
        <Card title="Wake spectrum at the probe" subtitle="Hann-windowed FFT of the transverse velocity; the peak is the shedding frequency">
          {spec && spec.st_axis.length > 0 ? (
            <Plot
              series={[{ name: 'power (normalised)', x: spec.st_axis, y: spec.power }]}
              xLabel="Strouhal number f D / U"
              height={230}
              vLines={st != null ? [{ value: st, label: `St ${st.toFixed(3)}` }] : []}
              yZero
            />
          ) : (
            <div className="muted small">{spec?.note ?? 'Collecting the probe signal…'}</div>
          )}
          <div className="row">
            <DownloadButton filename="lbm-frame.png" small content={png} label="frame PNG" />
            {last && <Chip tone={Math.abs(last.mass / (info ? info.nx * info.ny - info.solid_cells : 1) - 1) < 0.01 ? 'pass' : 'warn'}>mean density {(last.mass / Math.max(1, info ? info.nx * info.ny - info.solid_cells : 1)).toFixed(4)}</Chip>}
          </div>
        </Card>
      </div>

      <Explainer
        title="How the lattice-Boltzmann tunnel works"
        refs={[
          { label: 'Krüger et al., The Lattice Boltzmann Method (Springer, 2017)' },
          { label: 'Succi, The Lattice Boltzmann Equation (Oxford, 2001)' },
          { label: 'Ladd, J. Fluid Mech. 271 (1994) — momentum exchange' },
          { label: 'Schäfer & Turek (1996) — cylinder benchmark' },
          { label: 'FluidX3D (GPU LBM)', href: 'https://github.com/ProjectPhysX/FluidX3D' },
          { label: 'Palabos (open-source LBM library, University of Geneva)' },
        ]}
      >
        <p>Instead of solving the Navier–Stokes equations directly, LBM moves nine particle populations per cell along the lattice links and relaxes them towards a local equilibrium:</p>
        <Formula>f<sub>i</sub>(x + c<sub>i</sub>, t + 1) = f<sub>i</sub> − (f<sub>i</sub> − f<sub>i</sub><sup>eq</sup>) / τ,  f<sub>i</sub><sup>eq</sup> = w<sub>i</sub> ρ [1 + 3 c<sub>i</sub>·u + 9/2 (c<sub>i</sub>·u)² − 3/2 u²]</Formula>
        <p>
          Chapman–Enskog analysis shows this recovers incompressible flow with viscosity ν = (τ − ½)/3, so the slider sets τ = 3 U D / Re + ½. Walls bounce populations back (no-slip half-way between
          nodes); the force is the momentum those bounced populations carry. Above Re ≈ 47 a cylinder wake becomes unstable and sheds the alternating Kármán street; St = f D / U is about 0.16–0.21 for an
          unconfined cylinder between Re 100 and 1000 (Williamson 1996) — this tunnel's side walls raise it. Engine tests: Poiseuille profile within 1 %, mass conserved to 10⁻¹², Schäfer–Turek C_D within 10 % at
          only 10 cells per diameter.
        </p>
      </Explainer>
    </div>
  );
}
