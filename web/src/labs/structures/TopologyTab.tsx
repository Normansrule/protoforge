// Topology optimisation (SIMP, the 99/88-line method): density field evolving
// iteration by iteration on a canvas, compliance history, presets including an
// Iris hinge endpiece, SVG/OpenSCAD export of the final outline.
import { useEffect, useMemo, useRef, useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useTheme } from '../cad/viewer';
import type { TopoOut, TopoPreset } from './types';
import { b64Bytes, fmt, heat, prefersReducedMotion, toCsv, useRaf, useWorkerCompute } from './util';

export function TopologyTab() {
  const presets = useCompute<TopoPreset[]>('structures.topo_presets', {}).data ?? [];
  const [preset, setPreset] = useState('mbb');
  const p = presets.find((x) => x.id === preset);
  const [volfrac, setVolfrac] = useState<number | null>(null);
  const [rmin, setRmin] = useState<number | null>(null);
  const [penal, setPenal] = useState(3);
  const [res, setRes] = useState(1);
  const [filter, setFilter] = useState<'sensitivity' | 'density'>('sensitivity');
  const [maxIter, setMaxIter] = useState(80);
  const input = useMemo(() => {
    if (!p) return null;
    return {
      preset,
      nelx: Math.round(p.nelx * res),
      nely: Math.round(p.nely * res),
      volfrac: volfrac ?? p.volfrac,
      rmin: (rmin ?? p.rmin) * res,
      penal,
      filter,
      max_iter: maxIter,
      every: 2,
    };
  }, [p, preset, res, volfrac, rmin, penal, filter, maxIter]);
  const run = useWorkerCompute<TopoOut>('structures.topo', input);
  const o = run.data;
  const [frame, setFrame] = useState(0);
  const [playing, setPlaying] = useState(!prefersReducedMotion());
  const [view, setView] = useState<'density' | 'stress'>('density');
  const acc = useRef(0);
  const nFrames = o?.frames.length ?? 0;
  // Restart the animation whenever a new result arrives.
  useEffect(() => {
    if (!o) return;
    if (prefersReducedMotion()) {
      setFrame(o.frames.length - 1);
      setPlaying(false);
    } else {
      setFrame(0);
      setPlaying(true);
    }
  }, [o]);
  useRaf(playing && nFrames > 0, (dt) => {
    acc.current += dt;
    if (acc.current > 0.09) {
      acc.current = 0;
      setFrame((f) => {
        if (f >= nFrames - 1) {
          setPlaying(false);
          return f;
        }
        return f + 1;
      });
    }
  });
  const fr = o ? o.frames[Math.min(frame, nFrames - 1)] : null;
  const iterNow = fr?.iter ?? 0;
  return (
    <div className="stack">
      <div className="st-presets">
        {presets.map((x) => (
          <button
            key={x.id}
            className={`st-preset ${preset === x.id ? 'active' : ''}`}
            onClick={() => {
              setPreset(x.id);
              setVolfrac(null);
              setRmin(null);
            }}
            aria-pressed={preset === x.id}
          >
            <b>{x.label}</b>
            <span className="small muted">{x.nelx} × {x.nely} · {Math.round(x.volfrac * 100)} % material</span>
          </button>
        ))}
      </div>
      <div className="st-fea">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card
            title={p?.label ?? 'Topology optimisation'}
            subtitle={p?.description}
            actions={run.busy ? <span className="cp-spin" aria-label="optimising" /> : o ? <span className="small muted">{o.iterations} iterations · {(run.ms / 1000).toFixed(2)} s</span> : undefined}
          >
            <ErrorBox error={run.error} />
            {o && fr ? <DensityCanvas o={o} rhoB64={fr.rho_b64} view={view && frame === nFrames - 1 ? view : 'density'} /> : <div className="scene3d" style={{ height: 260 }} />}
            <div className="cp-toolbar">
              <button className="btn small" onClick={() => { if (frame >= nFrames - 1) setFrame(0); setPlaying(!playing); }} disabled={!o}>
                {playing ? '❚❚ Pause' : '▶ Play'}
              </button>
              <button className="btn small" onClick={() => { setPlaying(false); setFrame((f) => Math.max(0, f - 1)); }} disabled={!o}>◀ Step</button>
              <button className="btn small" onClick={() => { setPlaying(false); setFrame((f) => Math.min(nFrames - 1, f + 1)); }} disabled={!o}>Step ▶</button>
              <Slider label={`Iteration ${iterNow} of ${o?.iterations ?? 0}`} value={frame} min={0} max={Math.max(1, nFrames - 1)} step={1} format={() => ''} onChange={(v) => { setPlaying(false); setFrame(Math.round(v)); }} />
              <div className="cp-seg" role="group" aria-label="Show">
                {(['density', 'stress'] as const).map((m) => (
                  <button key={m} className={`cp-seg-btn ${view === m ? 'active' : ''}`} onClick={() => { setView(m); if (m === 'stress') { setPlaying(false); setFrame(nFrames - 1); } }}>
                    {m === 'density' ? 'density' : 'final stress'}
                  </button>
                ))}
              </div>
            </div>
          </Card>
          {o && (
            <div className="grid grid-4">
              <Stat label="Compliance" value={fmt(o.compliance[Math.min(iterNow, o.compliance.length) - 1] ?? 0, 1)} note={`from ${fmt(o.compliance[0], 1)} at iteration 1`} />
              <Stat label="Volume fraction" value={fmt(100 * (o.volume[Math.max(0, iterNow - 1)] ?? 0), 1)} unit="%" note={`target ${Math.round(o.volfrac * 100)} %`} />
              <Stat label="Max density change" value={fmt(o.change[Math.max(0, iterNow - 1)] ?? 0, 3)} note={o.converged ? '✓ converged (< 0.01)' : 'iteration limit'} />
              <Stat label="Design space" value={`${fmt(o.nelx * o.elem_mm, 1)} × ${fmt(o.nely * o.elem_mm, 1)}`} unit="mm" note={`${o.nelx} × ${o.nely} elements`} />
            </div>
          )}
        </div>
        <div className="stack cp-sticky">
          <Card title="Problem">
            <Slider label="Volume fraction" value={volfrac ?? p?.volfrac ?? 0.5} min={0.15} max={0.8} step={0.01} format={(v) => `${Math.round(v * 100)} %`} onChange={setVolfrac} />
            <Slider label="Filter radius r_min" value={rmin ?? p?.rmin ?? 1.5} min={1.1} max={4} step={0.1} unit="el" onChange={setRmin} />
            <Slider label="Penalty p" value={penal} min={1} max={4} step={0.1} onChange={setPenal} />
            <Slider label="Resolution" value={res} min={0.5} max={1.5} step={0.25} format={(v) => `×${v}`} onChange={setRes} />
            <Slider label="Max iterations" value={maxIter} min={10} max={200} step={10} onChange={setMaxIter} />
            <Select label="Filter" value={filter} options={[{ value: 'sensitivity', label: 'Sensitivity filter (99-line)' }, { value: 'density', label: 'Density filter (88-line)' }]} onChange={setFilter} />
          </Card>
          {o && (
            <Card title="Export">
              <div className="stack" style={{ gap: 8 }}>
                <DownloadButton small filename={`topology-${o.preset}.svg`} label="outline SVG (mm)" content={o.svg} />
                <DownloadButton small filename={`topology-${o.preset}.scad`} label="OpenSCAD extrusion" content={o.openscad} />
                <DownloadButton small filename={`topology-${o.preset}-density.csv`} label="density CSV" content={() => densityCsv(o)} />
                <div className="small muted">
                  The outline is the ρ = 0.5 contour ({o.outline.length} loop{o.outline.length === 1 ? '' : 's'}). Import the SVG into Onshape or FreeCAD as a sketch, smooth it and extrude — topology results are a starting point, not a finished part.
                </div>
              </div>
            </Card>
          )}
        </div>
      </div>
      <div className="grid grid-2">
        <Card title="Compliance vs iteration" subtitle="Compliance fᵀu (lower = stiffer) falls as material moves to where it carries load">
          {o ? (
            <Plot
              series={[{ name: 'compliance', x: o.compliance.map((_, i) => i + 1), y: o.compliance }]}
              xLabel="iteration"
              yLabel="compliance (E = 1, unit load)"
              vLines={[{ value: iterNow, label: `now` }]}
              height={220}
            />
          ) : (
            <div className="muted small">—</div>
          )}
        </Card>
        <Explainer
          title="How SIMP grows a structure"
          refs={[
            { label: 'O. Sigmund, "A 99 line topology optimization code written in Matlab", Struct. Multidisc. Optim. 21, 2001' },
            { label: 'E. Andreassen et al., "Efficient topology optimization in MATLAB using 88 lines of code", Struct. Multidisc. Optim. 43, 2011' },
          ]}
        >
          <p>Every element gets a density x between 0 (void) and 1 (solid). Its stiffness is penalised, so grey is a bad deal:</p>
          <Formula>{`E(x) = Emin + x^p (E0 − Emin)        minimise c = fᵀu = Σ E(x_e) u_eᵀ k0 u_e
∂c/∂x_e = −p x_e^(p−1) (E0 − Emin) u_eᵀ k0 u_e     subject to Σ x_e = V* · N
OC update: x_new = clamp( x · √(−∂c/∂x / λ),  x ± 0.2 )   (λ by bisection so the volume fits)`}</Formula>
          <p className="small">
            Each iteration is one plane-stress finite element solve (bilinear quads, banded Cholesky), a sensitivity per element, a filter over radius r_min
            (stops checkerboards and makes the answer mesh-independent) and the optimality-criteria update. Elements that carry strain energy thicken; idle ones fade.
            For Iris, the hinge endpiece preset shows where material is worth its weight between the rim and the knuckle.
          </p>
        </Explainer>
      </div>
    </div>
  );
}

function densityCsv(o: TopoOut): string {
  const rows: (string | number)[][] = [['ex', 'ey', 'x_mm', 'y_mm', 'density', 'von_mises_mpa_per_n']];
  for (let ex = 0; ex < o.nelx; ex++) {
    for (let ey = 0; ey < o.nely; ey++) {
      const e = ey + o.nely * ex;
      rows.push([ex, ey, +((ex + 0.5) * o.elem_mm).toFixed(3), +((ey + 0.5) * o.elem_mm).toFixed(3), +o.final_rho[e].toFixed(4), +o.final_vm[e].toFixed(4)]);
    }
  }
  return toCsv(rows);
}

function DensityCanvas({ o, rhoB64, view }: { o: TopoOut; rhoB64: string; view: 'density' | 'stress' }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const theme = useTheme();
  useEffect(() => {
    const c = ref.current;
    if (!c) return;
    const rho = b64Bytes(rhoB64);
    const cw = c.clientWidth || 600;
    const pad = 26;
    const cell = Math.max(2, Math.floor((cw - 2 * pad) / o.nelx));
    const w = o.nelx * cell + 2 * pad;
    const h = o.nely * cell + 2 * pad;
    const dpr = window.devicePixelRatio || 1;
    c.width = w * dpr;
    c.height = h * dpr;
    c.style.height = `${h}px`;
    const g = c.getContext('2d');
    if (!g) return;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    const cs = getComputedStyle(document.documentElement);
    const surface = cs.getPropertyValue('--surface-1').trim() || '#fff';
    const solid = theme === 'light' ? [13, 54, 107] : [134, 182, 239];
    const bg = parseColor(surface);
    g.fillStyle = surface;
    g.fillRect(0, 0, w, h);
    const vmax = view === 'stress' ? Math.max(...o.final_vm) || 1 : 1;
    for (let ex = 0; ex < o.nelx; ex++) {
      for (let ey = 0; ey < o.nely; ey++) {
        const e = ey + o.nely * ex;
        const x = pad + ex * cell;
        const y = pad + (o.nely - 1 - ey) * cell;
        if (view === 'stress') {
          const r = o.final_rho[e];
          if (r < 0.3) continue;
          const [cr, cg, cb] = heat(o.final_vm[e] / vmax);
          g.fillStyle = `rgb(${cr * 255},${cg * 255},${cb * 255})`;
        } else {
          const t = rho[e] / 255;
          g.fillStyle = `rgb(${bg[0] + (solid[0] - bg[0]) * t},${bg[1] + (solid[1] - bg[1]) * t},${bg[2] + (solid[2] - bg[2]) * t})`;
        }
        g.fillRect(x, y, cell + 0.5, cell + 0.5);
      }
    }
    // Frame of the design domain.
    g.strokeStyle = cs.getPropertyValue('--border-strong').trim() || '#999';
    g.lineWidth = 1;
    g.strokeRect(pad - 0.5, pad - 0.5, o.nelx * cell + 1, o.nely * cell + 1);
    // Supports: small triangles (pins) or hatch for clamps.
    const muted = cs.getPropertyValue('--text-2').trim() || '#666';
    const accent = cs.getPropertyValue('--series-2').trim() || '#d95926';
    const nodeXY = (i: number, j: number) => [pad + i * cell, pad + (o.nely - j) * cell];
    g.fillStyle = muted;
    g.strokeStyle = muted;
    for (const s of o.supports) {
      for (let i = s.i0; i <= s.i1; i++) {
        for (let j = s.j0; j <= s.j1; j++) {
          if ((i - s.i0 + j - s.j0) % Math.max(1, Math.round(4 / Math.max(1, cell / 3))) !== 0) continue;
          const [x, y] = nodeXY(i, j);
          g.beginPath();
          if (s.fix_x && s.fix_y) {
            g.rect(x - 3, y - 3, 6, 6);
          } else if (s.fix_x) {
            g.moveTo(x, y);
            g.lineTo(x - 8, y - 4);
            g.lineTo(x - 8, y + 4);
          } else {
            g.moveTo(x, y);
            g.lineTo(x - 4, y + 8);
            g.lineTo(x + 4, y + 8);
          }
          g.closePath();
          g.fill();
        }
      }
    }
    // Loads: arrows.
    g.strokeStyle = accent;
    g.fillStyle = accent;
    g.lineWidth = 2.5;
    for (const l of o.loads) {
      const [x, y] = nodeXY(l.i, l.j);
      const m = Math.hypot(l.fx, l.fy) || 1;
      const dx = l.fx / m;
      const dy = -l.fy / m;
      const len = 22;
      g.beginPath();
      g.moveTo(x - dx * len, y - dy * len);
      g.lineTo(x, y);
      g.stroke();
      g.beginPath();
      g.moveTo(x, y);
      g.lineTo(x - dx * 8 - dy * 4, y - dy * 8 + dx * 4);
      g.lineTo(x - dx * 8 + dy * 4, y - dy * 8 - dx * 4);
      g.closePath();
      g.fill();
    }
  }, [o, rhoB64, theme, view]);
  return <canvas ref={ref} className="cp-canvas2d" style={{ width: '100%' }} role="img" aria-label="Material density field" />;
}

function parseColor(c: string): [number, number, number] {
  if (c.startsWith('#')) {
    const n = parseInt(c.slice(1, 7), 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  }
  const m = c.match(/\d+(\.\d+)?/g);
  return m ? [Number(m[0]), Number(m[1]), Number(m[2])] : [255, 255, 255];
}
