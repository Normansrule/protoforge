// Cut slices through the voxel temperature field: a canvas heat map of one
// plane of elements, with material boundaries outlined and a hover readout.
import { useEffect, useMemo, useRef, useState } from 'react';
import { useElementWidth } from '../../components/useElementWidth';
import { useTheme } from '../cad/viewer';
import type { Slices } from './types';
import { TEMP_GRADIENT, b64Bytes, temp } from './util';

export type Axis = 'x' | 'y' | 'z';
const MAT_NAMES = ['outside', 'shell (frame plastic)', 'cavity air', 'printed circuit board', 'heat spreader', 'micro-display', 'USB-C receptacle'];

/** Plane geometry: horizontal / vertical grid axes for a cut normal to `axis`. */
function planeAxes(axis: Axis): { h: 0 | 1 | 2; v: 0 | 1 | 2; hName: string; vName: string } {
  if (axis === 'x') return { h: 1, v: 2, hName: 'y (forward →)', vName: 'z (up)' };
  if (axis === 'y') return { h: 0, v: 2, hName: 'x (outward →)', vName: 'z (up)' };
  return { h: 1, v: 0, hName: 'y (forward →)', vName: 'x (outward)' };
}

export function SliceView({ s, axis, index, showMaterials }: { s: Slices; axis: Axis; index: number; showMaterials: boolean }) {
  const theme = useTheme();
  const [wrapRef, width] = useElementWidth<HTMLDivElement>();
  const canvas = useRef<HTMLCanvasElement>(null);
  const tq = useMemo(() => b64Bytes(s.temp_u8), [s]);
  const mq = useMemo(() => b64Bytes(s.mat_u8), [s]);
  const dims = [s.nx, s.ny, s.nz];
  const ax = { x: 0, y: 1, z: 2 }[axis];
  const { h, v, hName, vName } = planeAxes(axis);
  const nh = dims[h];
  const nv = dims[v];
  const idx = Math.max(0, Math.min(dims[ax] - 1, index));
  const cell = Math.max(2, Math.min(14, Math.floor((width || 600) / nh)));
  const [hover, setHover] = useState<{ x: number; y: number; t: number; m: number; p: [number, number] } | null>(null);
  const at = (a: number, b: number) => {
    const c = [0, 0, 0];
    c[ax] = idx;
    c[h] = a;
    c[v] = b;
    return c[0] + s.nx * (c[1] + s.ny * c[2]);
  };
  // stretch the colour ramp over the temperatures actually in this slice
  const [qlo, qhi] = useMemo(() => {
    let lo = 255;
    let hi = 0;
    for (let b = 0; b < nv; b++) {
      for (let a = 0; a < nh; a++) {
        const e = at(a, b);
        if (mq[e] === 0) continue;
        lo = Math.min(lo, tq[e]);
        hi = Math.max(hi, tq[e]);
      }
    }
    return hi > lo ? [lo, hi] : [0, 255];
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tq, mq, axis, idx, nh, nv]);
  const tOf = (q: number) => s.t_lo + (q / 255) * (s.t_hi - s.t_lo);
  useEffect(() => {
    const cv = canvas.current;
    if (!cv) return;
    const W = nh * cell;
    const H = nv * cell;
    cv.width = W;
    cv.height = H;
    const g = cv.getContext('2d');
    if (!g) return;
    const css = getComputedStyle(document.documentElement);
    const bg = css.getPropertyValue('--surface-2').trim() || '#222';
    const edge = theme === 'light' ? 'rgba(11,11,11,0.55)' : 'rgba(255,255,255,0.6)';
    const matCol = ['', css.getPropertyValue('--series-1').trim(), css.getPropertyValue('--surface-3').trim(), css.getPropertyValue('--series-3').trim(), css.getPropertyValue('--series-4').trim(), css.getPropertyValue('--series-7').trim(), css.getPropertyValue('--muted').trim()];
    g.fillStyle = bg;
    g.fillRect(0, 0, W, H);
    for (let b = 0; b < nv; b++) {
      for (let a = 0; a < nh; a++) {
        const e = at(a, b);
        const m = mq[e];
        if (m === 0) continue;
        if (showMaterials) {
          g.fillStyle = matCol[m] || '#888';
        } else {
          const [r, gg, bb] = temp((tq[e] - qlo) / Math.max(1, qhi - qlo));
          g.fillStyle = `rgb(${(r * 255) | 0},${(gg * 255) | 0},${(bb * 255) | 0})`;
        }
        g.fillRect(a * cell, (nv - 1 - b) * cell, cell, cell);
      }
    }
    // material boundaries
    g.strokeStyle = edge;
    g.lineWidth = 1;
    g.beginPath();
    for (let b = 0; b < nv; b++) {
      for (let a = 0; a < nh; a++) {
        const m = mq[at(a, b)];
        if (a + 1 < nh && mq[at(a + 1, b)] !== m) {
          const x = (a + 1) * cell + 0.5;
          g.moveTo(x, (nv - 1 - b) * cell);
          g.lineTo(x, (nv - b) * cell);
        }
        if (b + 1 < nv && mq[at(a, b + 1)] !== m) {
          const y = (nv - 1 - b) * cell + 0.5;
          g.moveTo(a * cell, y);
          g.lineTo((a + 1) * cell, y);
        }
      }
    }
    g.stroke();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tq, mq, axis, idx, cell, showMaterials, theme, nh, nv, qlo, qhi]);
  const pos = (c: number, d: 0 | 1 | 2) => s.origin[d] + (c + 0.5) * s.h_mm;
  return (
    <div ref={wrapRef} style={{ position: 'relative', width: '100%' }}>
      <div style={{ overflowX: 'auto' }}>
        <canvas
          ref={canvas}
          className="fl-slice"
          style={{ width: nh * cell, height: nv * cell }}
          onMouseMove={(ev) => {
            const r = (ev.target as HTMLCanvasElement).getBoundingClientRect();
            const a = Math.floor((ev.clientX - r.left) / cell);
            const b = nv - 1 - Math.floor((ev.clientY - r.top) / cell);
            if (a < 0 || b < 0 || a >= nh || b >= nv) return setHover(null);
            const e = at(a, b);
            setHover({ x: ev.clientX - r.left, y: ev.clientY - r.top, t: s.t_lo + (tq[e] / 255) * (s.t_hi - s.t_lo), m: mq[e], p: [pos(a, h), pos(b, v)] });
          }}
          onMouseLeave={() => setHover(null)}
          role="img"
          aria-label={`temperature slice normal to ${axis} at ${pos(idx, ax as 0 | 1 | 2).toFixed(1)} mm`}
        />
      </div>
      {hover && (
        <div className="plot-tooltip" style={{ left: Math.min(hover.x + 12, nh * cell - 160), top: hover.y + 12 }}>
          <b>{hover.m === 0 ? 'outside' : `${hover.t.toFixed(2)} °C`}</b>
          <div className="muted">{MAT_NAMES[hover.m]}</div>
          <div className="muted mono">
            {hName.split(' ')[0]} {hover.p[0].toFixed(1)} · {vName.split(' ')[0]} {hover.p[1].toFixed(1)} mm
          </div>
        </div>
      )}
      <div className="row small muted" style={{ justifyContent: 'space-between', marginTop: 4 }}>
        <span>→ {hName}</span>
        <span>↑ {vName}</span>
        <span className="mono">
          {axis} = {pos(idx, ax as 0 | 1 | 2).toFixed(1)} mm · {s.h_mm} mm voxels
        </span>
      </div>
      {!showMaterials && (
        <div className="row small" style={{ marginTop: 6, gap: 8 }}>
          <span className="mono">{tOf(qlo).toFixed(2)} °C</span>
          <span className="fl-ramp" style={{ width: 160, background: TEMP_GRADIENT }} />
          <span className="mono">{tOf(qhi).toFixed(2)} °C</span>
          <span className="muted">colour stretched over this slice</span>
        </div>
      )}
      {showMaterials && (
        <div className="row small" style={{ marginTop: 6 }}>
          {[1, 2, 3, 4, 5, 6].map((m) => (
            <span key={m} className="row" style={{ gap: 4 }}>
              <span className="fl-swatch" style={{ background: ['', 'var(--series-1)', 'var(--surface-3)', 'var(--series-3)', 'var(--series-4)', 'var(--series-7)', 'var(--muted)'][m] }} />
              {MAT_NAMES[m]}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
