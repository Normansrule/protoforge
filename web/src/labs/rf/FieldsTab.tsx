// Fields (FDTD): a 2-D TMz Yee solver with a Bérenger PML runs in the lab's
// Web Worker; the page animates the quantised E_z frames it returns.
import { useEffect, useMemo, useRef, useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat, downloadBlob } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useElementWidth } from '../../components/useElementWidth';
import type { FdtdOut } from './types';
import { b64Bytes, divergingLut, fmt, lutGradient, prefersReducedMotion, toCsv, useRaf, useTheme, useWorkerCompute } from './util';
import { REFS } from './refs';

type Preset = 'dipole' | 'array' | 'head' | 'ground' | 'slit' | 'waveguide';

const PRESETS: { id: Preset; name: string; blurb: string }[] = [
  { id: 'dipole', name: 'Dipole in free space', blurb: 'One line source — the H-plane cut of a vertical dipole' },
  { id: 'array', name: 'Phased array', blurb: 'N sources with a phase step steer a beam' },
  { id: 'head', name: 'Antenna next to a head', blurb: 'Tissue phantom εr 39.2, σ 1.80 S/m absorbs and shadows' },
  { id: 'ground', name: 'Ground plane + monopole', blurb: 'Image theory: a λ/4 spacing sends the beam away' },
  { id: 'slit', name: 'Slit diffraction', blurb: 'A plane wave through an aperture (Huygens)' },
  { id: 'waveguide', name: 'Waveguide', blurb: 'Parallel plates: propagating or cut off' },
];

const GRIDS = { coarse: { nx: 130, ny: 100, dx: 2.0 }, normal: { nx: 180, ny: 140, dx: 1.5 }, fine: { nx: 240, ny: 186, dx: 1.125 } } as const;
type GridId = keyof typeof GRIDS;

function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || '#888';
}

export function FieldsTab() {
  const [preset, setPreset] = useState<Preset>('head');
  const [grid, setGrid] = useState<GridId>('normal');
  const [pulse, setPulse] = useState(false);
  const [courant, setCourant] = useState(0.99);
  const [steps, setSteps] = useState(1000);
  const [fGhz, setFGhz] = useState(2.44);
  const [tissueEr, setTissueEr] = useState(39.2);
  const [tissueSigma, setTissueSigma] = useState(1.8);
  const [gap, setGap] = useState(10);
  const [nArr, setNArr] = useState(4);
  const [spacing, setSpacing] = useState(0.5);
  const [phase, setPhase] = useState(0);
  const [height, setHeight] = useState(30.7);
  const [slit, setSlit] = useState(40);
  const [guide, setGuide] = useState(80);
  const [source, setSource] = useState<[number, number] | null>(null);
  const [probe, setProbe] = useState<[number, number] | null>(null);
  const [clickMode, setClickMode] = useState<'source' | 'probe'>('source');
  const g = GRIDS[grid];
  const input = useMemo(
    () => ({
      preset,
      nx: g.nx,
      ny: g.ny,
      dx_mm: g.dx,
      f_ghz: fGhz,
      steps: Math.round((steps * 1.5) / g.dx),
      frames: 110,
      courant,
      pml: 12,
      pulse,
      source,
      probe,
      tissue_er: tissueEr,
      tissue_sigma: tissueSigma,
      gap_mm: gap,
      array_n: nArr,
      array_spacing_wl: spacing,
      array_phase_deg: phase,
      height_mm: height,
      slit_mm: slit,
      guide_mm: guide,
      gamma: 0.5,
    }),
    [preset, g, fGhz, steps, courant, pulse, source, probe, tissueEr, tissueSigma, gap, nArr, spacing, phase, height, slit, guide],
  );
  const res = useWorkerCompute<FdtdOut>('rf.fdtd', input);
  const r = res.data;
  const choose = (p: Preset) => {
    setPreset(p);
    setSource(null);
    setProbe(null);
  };
  return (
    <div className="stack">
      <div className="rf-split">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card
            title={`${PRESETS.find((p) => p.id === preset)?.name} — E_z field`}
            subtitle={
              r
                ? `${r.nx} × ${r.ny} Yee cells of ${r.dx_mm} mm · ${r.steps} steps of ${fmt(r.dt_ps, 3)} ps · ${fmt(r.cells_per_wavelength, 0)} cells per wavelength in air · ${(res.ms / 1000).toFixed(2)} s in WebAssembly`
                : res.busy
                  ? 'time-stepping Maxwell’s equations…'
                  : '—'
            }
            actions={res.busy ? <span className="rf-spin" aria-label="computing" /> : undefined}
          >
            <ErrorBox error={res.error} />
            {r ? (
              <FieldPlayer r={r} clickMode={clickMode} onPick={(xy) => (clickMode === 'source' ? setSource(xy) : setProbe(xy))} />
            ) : (
              <div className="rf-canvas-empty">{res.busy ? 'Solving the first run…' : 'Waiting for the engine…'}</div>
            )}
            <div className="rf-toolbar" style={{ marginTop: 8 }}>
              <div className="rf-seg" role="group" aria-label="Click places">
                <span className="muted small" style={{ padding: '0 6px' }}>Click places the</span>
                {(['source', 'probe'] as const).map((m) => (
                  <button key={m} className={`rf-seg-btn ${clickMode === m ? 'active' : ''}`} onClick={() => setClickMode(m)} aria-pressed={clickMode === m}>
                    {m}
                  </button>
                ))}
              </div>
              {(source || probe) && (
                <button className="btn small" onClick={() => { setSource(null); setProbe(null); }}>
                  Reset positions
                </button>
              )}
            </div>
          </Card>
          {r && (
            <div className="grid grid-4 rf-stats">
              <Stat label="Courant number S" value={fmt(r.courant, 2)} note={`${r.stable ? '✓' : '✕'} Δt ${fmt(r.dt_ps, 3)} ps vs limit ${fmt(r.dt_max_ps, 3)} ps`} tone={r.stable ? 'pass' : 'fail'} />
              <Stat label="Resolution (densest material)" value={fmt(r.cells_per_wavelength_min, 1)} unit="cells/λ" note={r.cells_per_wavelength_min >= 10 ? '✓ ≥ 10 cells per wavelength' : '! < 10: visible dispersion'} tone={r.cells_per_wavelength_min >= 10 ? 'pass' : 'warn'} />
              <Stat
                label="Power absorbed by tissue"
                value={r.absorbed_fraction != null ? fmt(100 * r.absorbed_fraction, 0) : '—'}
                unit={r.absorbed_fraction != null ? '%' : ''}
                note={r.absorbed_db != null ? `≈ ${fmt(r.absorbed_db, 1)} dB of radiated power lost (2-D estimate)` : 'no lossy material in this preset'}
              />
              <Stat label="Grid" value={`${r.nx}×${r.ny}`} note={`PML ${r.pml} cells · ${fmt((r.nx * r.dx_mm) / 10, 0)} × ${fmt((r.ny * r.dx_mm) / 10, 0)} cm`} />
            </div>
          )}
          {r && <Traces r={r} />}
        </div>
        <div className="stack rf-sticky">
          <Card title="Scene">
            <div className="rf-presets">
              {PRESETS.map((p) => (
                <button key={p.id} className={`rf-preset ${preset === p.id ? 'active' : ''}`} onClick={() => choose(p.id)} aria-pressed={preset === p.id}>
                  <b>{p.name}</b>
                  <span className="muted small">{p.blurb}</span>
                </button>
              ))}
            </div>
          </Card>
          <Card title="Parameters">
            {preset === 'head' && (
              <>
                <Slider label="Tissue permittivity εr" value={tissueEr} min={1} max={60} step={0.1} onChange={setTissueEr} />
                <Slider label="Tissue conductivity σ" value={tissueSigma} min={0} max={3} step={0.01} unit="S/m" onChange={setTissueSigma} />
                <Slider label="Antenna-to-head gap" value={gap} min={2} max={40} step={1} unit="mm" onChange={(v) => { setGap(v); setSource(null); }} />
                <div className="small muted" style={{ marginBottom: 10 }}>Defaults are the IEEE 1528 / IEC 62209 head-simulating liquid targets at 2450 MHz. Set σ = 0 to see a lossless dielectric lens instead.</div>
              </>
            )}
            {preset === 'array' && (
              <>
                <Slider label="Elements N" value={nArr} min={1} max={8} step={1} onChange={setNArr} />
                <Slider label="Spacing d" value={spacing} min={0.1} max={1.2} step={0.05} unit="λ" onChange={setSpacing} />
                <Slider label="Phase step β" value={phase} min={-180} max={180} step={5} unit="°" onChange={setPhase} />
              </>
            )}
            {preset === 'ground' && <Slider label="Height above the ground plane" value={height} min={5} max={70} step={0.5} unit="mm" onChange={(v) => { setHeight(v); setSource(null); }} />}
            {preset === 'slit' && <Slider label="Slit width" value={slit} min={6} max={150} step={1} unit="mm" onChange={setSlit} />}
            {preset === 'waveguide' && (
              <>
                <Slider label="Plate spacing a" value={guide} min={30} max={120} step={1} unit="mm" onChange={setGuide} />
                <div className="small muted" style={{ marginBottom: 10 }}>
                  Cutoff f_c = c/2a = {fmt(299.792458 / (2 * guide), 2)} GHz — {fGhz > 299.792458 / (2 * guide) ? '✓ propagates' : '✕ below cutoff (evanescent)'} at {fmt(fGhz, 2)} GHz.
                </div>
              </>
            )}
            <Slider label="Frequency" value={fGhz} min={1} max={6} step={0.01} unit="GHz" onChange={setFGhz} />
            <div className="rf-seg" role="group" aria-label="Excitation" style={{ marginBottom: 10 }}>
              {([false, true] as const).map((p) => (
                <button key={String(p)} className={`rf-seg-btn ${pulse === p ? 'active' : ''}`} onClick={() => setPulse(p)} aria-pressed={pulse === p}>
                  {p ? 'Gaussian pulse' : 'Continuous wave'}
                </button>
              ))}
            </div>
            <div className="rf-seg" role="group" aria-label="Grid" style={{ marginBottom: 10 }}>
              {(Object.keys(GRIDS) as GridId[]).map((k) => (
                <button key={k} className={`rf-seg-btn ${grid === k ? 'active' : ''}`} onClick={() => { setGrid(k); setSource(null); setProbe(null); }} aria-pressed={grid === k}>
                  {k} ({GRIDS[k].dx} mm)
                </button>
              ))}
            </div>
            <Slider label="Duration (at 1.5 mm cells)" value={steps} min={200} max={2000} step={50} unit="steps" onChange={setSteps} />
            <Slider label="Courant number S = cΔt√2/Δ" value={courant} min={0.3} max={1.03} step={0.01} onChange={setCourant} hint="Above 1 the leapfrog scheme is unstable — try 1.02" />
          </Card>
          {r && (
            <Card title="Downloads">
              <div className="row">
                <DownloadButton small filename={`fdtd-${r.preset}-probe.csv`} content={() => toCsv([['t_ns', 'Ez_probe'], ...r.probe_t_ns.map((t, k) => [t, r.probe_ez[k]])])} label="probe trace CSV" />
                <DownloadButton small filename={`fdtd-${r.preset}-energy.csv`} content={() => toCsv([['t_ns', 'energy_J_per_m'], ...r.frame_time_ns.map((t, k) => [t, r.energy[k]])])} label="energy CSV" />
                <DownloadButton small filename={`fdtd-${r.preset}-input.json`} content={() => JSON.stringify(input, null, 2)} label="run settings JSON" />
              </div>
            </Card>
          )}
          {r && r.notes.length > 0 && (
            <Card title="What you are seeing">
              <div className="stack" style={{ gap: 8 }}>
                {r.notes.map((n, k) => <div key={k} className="small">{n}</div>)}
              </div>
            </Card>
          )}
        </div>
      </div>
      <div className="grid grid-2">
        <Explainer title="The Yee grid and the leapfrog" refs={[REFS.yee, REFS.taflove]}>
          Maxwell’s curl equations are discretised with E_z on cell corners and H_x, H_y half a cell away, half a time step apart. Each field is updated from the curl of the other:
          <Formula>{'H_y ← H_y + Δt/(μ₀Δ)·(E_z[i+1] − E_z[i])     E_z ← C_a·E_z + C_b·(∂H_y/∂x − ∂H_x/∂y)Δ'}</Formula>
          <Formula>{'C_a = (1 − σΔt/2ε)/(1 + σΔt/2ε),   C_b = (Δt/εΔ)/(1 + σΔt/2ε)'}</Formula>
          Stability requires the Courant condition cΔt·√(1/Δx² + 1/Δy²) ≤ 1; push S above 1 and the checkerboard instability grows without bound.
        </Explainer>
        <Explainer title="Perfectly matched layer (PML)" refs={[REFS.berenger, REFS.taflove]}>
          The grid is surrounded by a 12-cell absorbing layer. Bérenger split E_z into E_zx + E_zy, each damped only by the conductivity normal to its own direction, with magnetic conductivity σ* = σμ₀/ε₀ so the layer’s impedance matches free space at every angle:
          <Formula>{'σ(ρ) = σ_max (ρ/N)³,   σ_max = −(m+1)·ln R₀ / (2η₀NΔ),   R₀ = 10⁻⁶'}</Formula>
          Waves leave through the dashed boundary as if the room went on forever — test it with the Gaussian pulse and watch the energy trace fall to zero.
        </Explainer>
      </div>
    </div>
  );
}

function Traces({ r }: { r: FdtdOut }) {
  const emax = Math.max(1e-300, ...r.energy);
  const energy = r.energy.map((e) => (100 * e) / emax);
  const probe = r.probe_ez.map((v) => v / r.scale);
  return (
    <div className="grid grid-2">
      <Card title="Probe E_z vs time" subtitle={`at cell (${r.probe[0]}, ${r.probe[1]}) — the diamond marker`}>
        <Plot series={[{ name: 'E_z at probe', x: r.probe_t_ns, y: probe }]} height={200} xLabel="time (ns)" yLabel="E_z / colour-scale max" />
      </Card>
      <Card title="Field energy in the room" subtitle="Σ ½(εE² + μH²) inside the PML, as % of its peak">
        <Plot series={[{ name: 'energy', x: r.frame_time_ns, y: energy, area: true }]} height={200} xLabel="time (ns)" yLabel="energy (% of peak)" yZero />
      </Card>
    </div>
  );
}

function FieldPlayer({ r, clickMode, onPick }: { r: FdtdOut; clickMode: 'source' | 'probe'; onPick: (xy: [number, number]) => void }) {
  const light = useTheme();
  const frames = useMemo(() => b64Bytes(r.frames_b64), [r.frames_b64]);
  const mat = useMemo(() => b64Bytes(r.material_b64), [r.material_b64]);
  const lut = useMemo(() => divergingLut(light), [light]);
  const [wrapRef, width] = useElementWidth<HTMLDivElement>();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [frame, setFrame] = useState(0);
  const [playing, setPlaying] = useState(!prefersReducedMotion());
  const [fps, setFps] = useState(24);
  const acc = useRef(0);
  const off = useMemo(() => {
    const c = document.createElement('canvas');
    c.width = r.nx;
    c.height = r.ny;
    return c;
  }, [r.nx, r.ny]);
  useEffect(() => {
    // new run: jump to the last frame when paused so the steady state shows
    setFrame(prefersReducedMotion() ? r.n_frames - 1 : 0);
  }, [r]);
  useRaf(playing, (dt) => {
    acc.current += dt * fps;
    if (acc.current >= 1) {
      const n = Math.floor(acc.current);
      acc.current -= n;
      setFrame((f) => (f + n) % r.n_frames);
    }
  });
  const W = Math.max(200, width);
  const H = Math.round((W * r.ny) / r.nx);
  useEffect(() => {
    const cv = canvasRef.current;
    if (!cv) return;
    const dpr = window.devicePixelRatio || 1;
    cv.width = Math.round(W * dpr);
    cv.height = Math.round(H * dpr);
    const ctx = cv.getContext('2d');
    const octx = off.getContext('2d');
    if (!ctx || !octx) return;
    const n = r.nx * r.ny;
    const f = Math.min(frame, r.n_frames - 1);
    const img = octx.createImageData(r.nx, r.ny);
    const metal = light ? [60, 60, 58] : [215, 214, 205];
    for (let k = 0; k < n; k++) {
      const q = frames[f * n + k];
      const o = k * 4;
      if (mat[k] === 2) {
        img.data[o] = metal[0];
        img.data[o + 1] = metal[1];
        img.data[o + 2] = metal[2];
      } else {
        img.data[o] = lut[q * 4];
        img.data[o + 1] = lut[q * 4 + 1];
        img.data[o + 2] = lut[q * 4 + 2];
      }
      img.data[o + 3] = 255;
    }
    octx.putImageData(img, 0, 0);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.imageSmoothingEnabled = true;
    ctx.drawImage(off, 0, 0, W, H);
    const sx = W / r.nx;
    const sy = H / r.ny;
    // tissue outline
    ctx.fillStyle = light ? 'rgba(20,20,20,0.55)' : 'rgba(255,255,255,0.55)';
    for (let j = 1; j < r.ny - 1; j++) {
      for (let i = 1; i < r.nx - 1; i++) {
        const k = j * r.nx + i;
        if (mat[k] === 1 && (mat[k - 1] !== 1 || mat[k + 1] !== 1 || mat[k - r.nx] !== 1 || mat[k + r.nx] !== 1)) {
          ctx.fillRect(i * sx, j * sy, Math.max(1, sx * 0.7), Math.max(1, sy * 0.7));
        }
      }
    }
    // PML band
    const p = r.pml;
    ctx.fillStyle = light ? 'rgba(252,252,251,0.55)' : 'rgba(13,13,14,0.55)';
    ctx.fillRect(0, 0, W, p * sy);
    ctx.fillRect(0, H - p * sy, W, p * sy);
    ctx.fillRect(0, p * sy, p * sx, H - 2 * p * sy);
    ctx.fillRect(W - p * sx, p * sy, p * sx, H - 2 * p * sy);
    ctx.setLineDash([5, 4]);
    ctx.strokeStyle = cssVar('--muted');
    ctx.lineWidth = 1;
    ctx.strokeRect(p * sx, p * sy, W - 2 * p * sx, H - 2 * p * sy);
    ctx.setLineDash([]);
    ctx.fillStyle = cssVar('--text-2');
    ctx.font = '11px system-ui, sans-serif';
    ctx.fillText('PML', 4, Math.max(11, p * sy - 3));
    // sources
    const accent = cssVar('--text');
    for (const [i, j] of r.sources.length > 40 ? [r.sources[0], r.sources[r.sources.length - 1]] : r.sources) {
      ctx.beginPath();
      ctx.arc((i + 0.5) * sx, (j + 0.5) * sy, 4.5, 0, 2 * Math.PI);
      ctx.fillStyle = accent;
      ctx.fill();
      ctx.lineWidth = 2;
      ctx.strokeStyle = light ? '#fff' : '#000';
      ctx.stroke();
    }
    if (r.sources.length > 40) {
      const [i0, j0] = r.sources[0];
      const [, j1] = r.sources[r.sources.length - 1];
      ctx.strokeStyle = accent;
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo((i0 + 0.5) * sx, (j0 + 0.5) * sy);
      ctx.lineTo((i0 + 0.5) * sx, (j1 + 0.5) * sy);
      ctx.stroke();
    }
    // probe
    const [pi, pj] = r.probe;
    const px = (pi + 0.5) * sx;
    const py = (pj + 0.5) * sy;
    ctx.beginPath();
    ctx.moveTo(px, py - 7);
    ctx.lineTo(px + 7, py);
    ctx.lineTo(px, py + 7);
    ctx.lineTo(px - 7, py);
    ctx.closePath();
    ctx.lineWidth = 2;
    ctx.strokeStyle = accent;
    ctx.stroke();
  }, [frame, frames, mat, lut, off, r, W, H, light]);
  const pick = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const i = ((e.clientX - rect.left) / rect.width) * r.nx;
    const j = ((e.clientY - rect.top) / rect.height) * r.ny;
    onPick([Math.round(i), Math.round(j)]);
  };
  const t = r.frame_time_ns[Math.min(frame, r.n_frames - 1)] ?? 0;
  const snapshot = () => canvasRef.current?.toBlob((b) => b && downloadBlob(`fdtd-${r.preset}-frame${frame}.png`, b));
  return (
    <div ref={wrapRef} className="stack" style={{ gap: 8 }}>
      <div style={{ position: 'relative' }}>
        <canvas
          ref={canvasRef}
          className="rf-canvas"
          style={{ width: W, height: H, cursor: 'crosshair' }}
          onClick={pick}
          role="img"
          aria-label={`E_z field of the ${r.preset} scene at ${t.toFixed(2)} ns. Click to move the ${clickMode}.`}
        />
        <div className="rf-legend">
          <div className="small"><b>E_z</b> (signed, γ {r.gamma})</div>
          <div className="rf-legend-bar" style={{ background: lutGradient(light) }} />
          <div className="rf-legend-ticks"><span>−</span><span>0</span><span>+</span></div>
          <div className="rf-legend-keys small">
            <span><i className="rf-key-dot" /> source</span>
            <span><i className="rf-key-diamond" /> probe</span>
            {r.legend.some((l) => l.id === 2) && <span><i className="rf-key-metal" /> metal</span>}
            {r.legend.some((l) => l.id === 1) && <span><i className="rf-key-outline" /> tissue</span>}
          </div>
        </div>
      </div>
      <div className="rf-toolbar">
        <button className="btn small" onClick={() => setPlaying((p) => !p)} aria-pressed={playing}>{playing ? '❚❚ Pause' : '▶ Play'}</button>
        <button className="btn small" onClick={() => { setPlaying(false); setFrame((f) => (f - 1 + r.n_frames) % r.n_frames); }} aria-label="previous frame">◀ step</button>
        <button className="btn small" onClick={() => { setPlaying(false); setFrame((f) => (f + 1) % r.n_frames); }} aria-label="next frame">step ▶</button>
        <label className="rf-scrub">
          <span className="muted small tnum">t = {t.toFixed(2)} ns · step {r.frame_step[Math.min(frame, r.n_frames - 1)]}/{r.steps}</span>
          <input type="range" min={0} max={r.n_frames - 1} value={Math.min(frame, r.n_frames - 1)} onChange={(e) => { setPlaying(false); setFrame(Number(e.target.value)); }} aria-label="frame" />
        </label>
        <Slider label="Speed" value={fps} min={4} max={60} step={1} unit="fps" onChange={setFps} />
        <button className="btn small" onClick={snapshot}>↓ frame PNG</button>
      </div>
    </div>
  );
}
