// Interactive Smith chart (SVG): impedance and admittance grids, a draggable
// load point, matching-network paths and band traces.
import { useRef, useState, type ReactNode } from 'react';
import type { C64, Element } from './types';
import { elLabel } from './util';

export interface Trace { name: string; re: number[]; im: number[]; color: string; dash?: string; width?: number }
export interface PathArcs { arcs: [number, number][][]; elements: Element[]; color: string }

const S = 400;
const C = S / 2;
const R = 180;
const X = (re: number) => C + re * R;
const Y = (im: number) => C - im * R;

export function zToGamma(z: C64, z0 = 50): C64 {
  const nr = z.re - z0;
  const dr = z.re + z0;
  const d = dr * dr + z.im * z.im;
  return { re: (nr * dr + z.im * z.im) / d, im: (z.im * dr - nr * z.im) / d };
}

export function gammaToZ(g: C64, z0 = 50): C64 {
  // z = z0 (1 + Γ)/(1 − Γ)
  const nr = 1 + g.re;
  const ni = g.im;
  const dr = 1 - g.re;
  const di = -g.im;
  const d = dr * dr + di * di;
  return { re: (z0 * (nr * dr + ni * di)) / d, im: (z0 * (ni * dr - nr * di)) / d };
}

function Grid({ admittance }: { admittance: boolean }) {
  const rs = [0.2, 0.5, 1, 2, 5];
  const xs = [0.2, 0.5, 1, 2, 5];
  const sgn = admittance ? -1 : 1;
  return (
    <g className={admittance ? 'rf-smith-adm' : 'rf-smith-grid'}>
      {rs.map((r) => (
        <circle key={`r${r}`} cx={X((sgn * r) / (1 + r))} cy={C} r={R / (1 + r)} />
      ))}
      {xs.flatMap((x) => [x, -x]).map((x) => (
        <circle key={`x${x}`} cx={X(sgn * 1)} cy={Y(sgn / x)} r={R / Math.abs(x)} />
      ))}
      <line x1={C - R} y1={C} x2={C + R} y2={C} />
    </g>
  );
}

type Box = [number, number, number, number]; // x, y, w, h in SVG units

const overlaps = (a: Box, b: Box, pad = 2) => a[0] < b[0] + b[2] + pad && b[0] < a[0] + a[2] + pad && a[1] < b[1] + b[3] + pad && b[1] < a[1] + a[3] + pad;

/**
 * Put each element label beside its arc without overlapping another label, the load
 * label, the arc end points or the chart edge (near the centre the arcs are short and
 * their midpoints crowd together). Candidates step along the arc and out along its
 * normal on both sides; the first free one wins, with a leader line when it sits away.
 */
function placeArcLabels(arcs: [number, number][][], texts: string[], avoid: Box[]) {
  const placed: Box[] = [...avoid];
  return arcs.map((arc, k) => {
    const text = texts[k] ?? '';
    const w = text.length * 6.6 + 4;
    const h = 14;
    const pts = arc.map(([re, im]) => [X(re), Y(im)] as [number, number]);
    let best: { cx: number; cy: number; px: number; py: number; box: Box; d: number } | null = null;
    outer: for (const f of [0.5, 0.35, 0.65, 0.2, 0.8, 0.05, 0.95]) {
      const i = Math.min(pts.length - 1, Math.max(0, Math.round(f * (pts.length - 1))));
      const a = pts[Math.max(0, i - 1)];
      const b = pts[Math.min(pts.length - 1, i + 1)];
      let tx = b[0] - a[0], ty = b[1] - a[1];
      const tl = Math.hypot(tx, ty) || 1;
      tx /= tl; ty /= tl;
      const [px, py] = pts[i];
      // try the side facing away from the chart centre first
      const out = (px - C) * -ty + (py - C) * tx >= 0 ? 1 : -1;
      for (const d of [8, 20, 34, 50]) {
        for (const side of [out, -out]) {
          const nx = -ty * side, ny = tx * side;
          const reach = d + 0.5 * (Math.abs(nx) * w + Math.abs(ny) * h);
          const cx = px + nx * reach, cy = py + ny * reach;
          const box: Box = [cx - w / 2, cy - h / 2, w, h];
          const inside = box[0] >= 2 && box[1] >= 2 && box[0] + w <= S - 2 && box[1] + h <= S - 2;
          if (!inside || placed.some((q) => overlaps(q, box))) continue;
          best = { cx, cy, px, py, box, d };
          break outer;
        }
      }
    }
    if (!best) {
      const [px, py] = pts[Math.floor(pts.length / 2)] ?? [C, C];
      const box: Box = [px + 6, py - 6 - h, w, h];
      best = { cx: px + 6 + w / 2, cy: py - 6 - h / 2, px, py, box, d: 8 };
    }
    placed.push(best.box);
    return { text, ...best };
  });
}

function Labels() {
  const rs = [0.2, 0.5, 1, 2, 5];
  return (
    <g className="rf-smith-label">
      {rs.map((r) => (
        <text key={r} x={X((r - 1) / (r + 1)) + 2} y={C - 3}>{r}</text>
      ))}
      {[0.5, 1, 2].map((x) => {
        // where the x arc meets the unit circle: Γ = (x² − 1 + 2jx)/(x² + 1)
        const gr = (x * x - 1) / (x * x + 1);
        const gi = (2 * x) / (x * x + 1);
        return (
          <g key={x}>
            <text x={X(gr * 1.07)} y={Y(gi * 1.07) + 4} textAnchor="middle">+j{x}</text>
            <text x={X(gr * 1.07)} y={Y(-gi * 1.07) + 4} textAnchor="middle">−j{x}</text>
          </g>
        );
      })}
    </g>
  );
}

export function SmithChart(props: {
  load: C64;
  z0?: number;
  onDrag?: (z: C64) => void;
  traces?: Trace[];
  path?: PathArcs | null;
  admittance?: boolean;
  rlCircleDb?: number;
  children?: ReactNode;
  caption?: string;
}) {
  const z0 = props.z0 ?? 50;
  const g = zToGamma(props.load, z0);
  const svgRef = useRef<SVGSVGElement>(null);
  const [drag, setDrag] = useState(false);
  const toGamma = (e: React.PointerEvent) => {
    const svg = svgRef.current!;
    const rect = svg.getBoundingClientRect();
    let re = (((e.clientX - rect.left) / rect.width) * S - C) / R;
    let im = -(((e.clientY - rect.top) / rect.height) * S - C) / R;
    const m = Math.hypot(re, im);
    if (m > 0.97) {
      re *= 0.97 / m;
      im *= 0.97 / m;
    }
    return { re, im };
  };
  const move = (e: React.PointerEvent) => {
    if (!drag || !props.onDrag) return;
    const z = gammaToZ(toGamma(e), z0);
    props.onDrag({ re: Math.max(0.5, z.re), im: z.im });
  };
  const rl = props.rlCircleDb ?? 10;
  const rlR = 10 ** (-rl / 20);
  const mag = Math.hypot(g.re, g.im);
  return (
    <div className="rf-smith-wrap">
      <svg
        ref={svgRef}
        viewBox={`0 0 ${S} ${S}`}
        className="rf-smith"
        onPointerMove={move}
        onPointerUp={() => setDrag(false)}
        onPointerLeave={() => setDrag(false)}
        onPointerDown={(e) => {
          if (!props.onDrag) return;
          e.currentTarget.setPointerCapture?.(e.pointerId);
          setDrag(true);
          const z = gammaToZ(toGamma(e), z0);
          props.onDrag({ re: Math.max(0.5, z.re), im: z.im });
        }}
        role="img"
        aria-label={`Smith chart. Load ${props.load.re.toFixed(1)} ${props.load.im >= 0 ? '+' : '−'} j${Math.abs(props.load.im).toFixed(1)} ohm, reflection coefficient magnitude ${mag.toFixed(2)}.`}
      >
        <defs>
          <clipPath id="rf-unit">
            <circle cx={C} cy={C} r={R} />
          </clipPath>
        </defs>
        <circle cx={C} cy={C} r={R} className="rf-smith-bg" />
        <circle cx={C} cy={C} r={rlR * R} className="rf-smith-target" />
        <g clipPath="url(#rf-unit)">
          {props.admittance && <Grid admittance />}
          <Grid admittance={false} />
        </g>
        <circle cx={C} cy={C} r={R} className="rf-smith-outer" />
        <Labels />
        <text x={C + rlR * R * 0.71 + 4} y={C + rlR * R * 0.71 + 12} className="rf-smith-label">{rl} dB</text>
        <circle cx={C} cy={C} r={mag * R} className="rf-smith-vswr" />
        {(props.traces ?? []).map((t, k) => (
          <path
            key={k}
            d={t.re.map((re, i) => `${i ? 'L' : 'M'}${X(re).toFixed(1)},${Y(t.im[i]).toFixed(1)}`).join('')}
            fill="none"
            stroke={t.color}
            strokeWidth={t.width ?? 3}
            strokeDasharray={t.dash}
            strokeLinecap="round"
          />
        ))}
        {props.path &&
          props.path.arcs.map((arc, k) => {
            const end = arc[arc.length - 1];
            return (
              <g key={k}>
                <path d={arc.map(([re, im], i) => `${i ? 'L' : 'M'}${X(re).toFixed(1)},${Y(im).toFixed(1)}`).join('')} fill="none" stroke={props.path!.color} strokeWidth={2.5} />
                <circle cx={X(end[0])} cy={Y(end[1])} r={4} fill={props.path!.color} stroke="var(--surface-1)" strokeWidth={2} />
              </g>
            );
          })}
        {props.path &&
          (() => {
            const path = props.path;
            const texts = path.arcs.map((_, k) => {
              const el = path.elements[path.elements.length - 1 - k];
              return el ? `${k + 1}. ${elLabel(el)}` : '';
            });
            const lx = X(g.re), ly = Y(g.im);
            const avoid: Box[] = [
              [lx - 8, ly - 8, 16, 16], // load point
              [lx + 9, ly - 21, 26, 14], // "Z_L"
              [C - 5, C - 5, 10, 10], // centre
              [C + rlR * R * 0.71 + 2, C + rlR * R * 0.71, 36, 15], // return-loss circle label
              ...path.arcs.map((arc) => { const e = arc[arc.length - 1]; return [X(e[0]) - 5, Y(e[1]) - 5, 10, 10] as Box; }),
            ];
            return placeArcLabels(path.arcs, texts, avoid).map((l, k) =>
              l.text ? (
                <g key={`l${k}`}>
                  {l.d > 8 && <line x1={l.px} y1={l.py} x2={Math.max(l.box[0], Math.min(l.box[0] + l.box[2], l.px))} y2={Math.max(l.box[1], Math.min(l.box[1] + l.box[3], l.py))} stroke={path.color} strokeWidth={1} opacity={0.7} />}
                  <text x={l.cx} y={l.cy + 4} textAnchor="middle" className="rf-smith-elabel">{l.text}</text>
                </g>
              ) : null,
            );
          })()}
        <circle cx={C} cy={C} r={3} className="rf-smith-centre" />
        <g transform={`translate(${X(g.re)},${Y(g.im)})`} style={{ cursor: props.onDrag ? 'grab' : 'default' }}>
          <circle r={14} fill="transparent" />
          <circle r={7} className="rf-smith-load" />
          <text x={10} y={-10} className="rf-smith-elabel">Z_L</text>
        </g>
        {props.children}
      </svg>
      {props.caption && <div className="small muted" style={{ textAlign: 'center' }}>{props.caption}</div>}
    </div>
  );
}

/** Small ladder schematic: 50 Ω port on the left, antenna on the right. */
export function Ladder({ net, width = 300 }: { net: Element[]; width?: number }) {
  const n = Math.max(1, net.length);
  const step = (width - 110) / n;
  const y = 28;
  const gy = 78;
  const ind = (x0: number, x1: number, yy: number) => {
    const w = (x1 - x0) / 4;
    let d = `M${x0},${yy}`;
    for (let k = 0; k < 4; k++) d += ` a${w / 2},${w / 2} 0 0 1 ${w},0`;
    return d;
  };
  const vind = (xx: number, y0: number, y1: number) => {
    const w = (y1 - y0) / 4;
    let d = `M${xx},${y0}`;
    for (let k = 0; k < 4; k++) d += ` a${w / 2},${w / 2} 0 0 1 0,${w}`;
    return d;
  };
  return (
    <svg viewBox={`0 0 ${width} 96`} width="100%" style={{ maxWidth: width }} className="rf-ladder" role="img" aria-label={`matching network: ${net.map(elLabel).join(', ') || 'none'}`}>
      <circle cx={14} cy={(y + gy) / 2} r={11} className="rf-ladder-port" />
      <text x={14} y={(y + gy) / 2 + 4} textAnchor="middle" className="rf-ladder-t">50Ω</text>
      <line x1={14} y1={y} x2={14} y2={(y + gy) / 2 - 11} />
      <line x1={14} y1={gy} x2={14} y2={(y + gy) / 2 + 11} />
      <line x1={14} y1={y} x2={40} y2={y} />
      <line x1={14} y1={gy} x2={width - 50} y2={gy} className="rf-ladder-gnd" />
      {net.map((e, k) => {
        const x0 = 40 + k * step;
        const xm = x0 + step / 2;
        const l = e.kind.endsWith('_l');
        const label = elLabel(e).replace('series ', '').replace('shunt ', '');
        if (e.kind.startsWith('series')) {
          return (
            <g key={k}>
              <line x1={x0} y1={y} x2={xm - 16} y2={y} />
              {l ? <path d={ind(xm - 16, xm + 16, y)} className="rf-ladder-el" /> : (
                <>
                  <line x1={xm - 16} y1={y} x2={xm - 3} y2={y} />
                  <line x1={xm - 3} y1={y - 9} x2={xm - 3} y2={y + 9} className="rf-ladder-el" />
                  <line x1={xm + 3} y1={y - 9} x2={xm + 3} y2={y + 9} className="rf-ladder-el" />
                  <line x1={xm + 3} y1={y} x2={xm + 16} y2={y} />
                </>
              )}
              <line x1={xm + 16} y1={y} x2={x0 + step} y2={y} />
              <text x={xm} y={y - 12} textAnchor="middle" className="rf-ladder-t">{label}</text>
            </g>
          );
        }
        return (
          <g key={k}>
            <line x1={x0} y1={y} x2={x0 + step} y2={y} />
            <circle cx={xm} cy={y} r={2.5} className="rf-ladder-dot" />
            {l ? (
              <path d={vind(xm, (y + gy) / 2 - 16, (y + gy) / 2 + 16)} className="rf-ladder-el" />
            ) : (
              <>
                <line x1={xm - 9} y1={(y + gy) / 2 - 3} x2={xm + 9} y2={(y + gy) / 2 - 3} className="rf-ladder-el" />
                <line x1={xm - 9} y1={(y + gy) / 2 + 3} x2={xm + 9} y2={(y + gy) / 2 + 3} className="rf-ladder-el" />
              </>
            )}
            <line x1={xm} y1={y} x2={xm} y2={(y + gy) / 2 - (l ? 16 : 3)} />
            <line x1={xm} y1={(y + gy) / 2 + (l ? 16 : 3)} x2={xm} y2={gy} />
            <text x={xm + 12} y={(y + gy) / 2 + 4} className="rf-ladder-t">{label}</text>
          </g>
        );
      })}
      <line x1={40 + n * step} y1={y} x2={width - 50} y2={y} />
      <path d={`M${width - 50},${y} l0,-14 m-10,0 l10,10 l10,-10`} className="rf-ladder-el" />
      <text x={width - 36} y={y + 4} className="rf-ladder-t">AE1</text>
      <line x1={width - 50} y1={y} x2={width - 50} y2={y + 14} />
      <rect x={width - 58} y={y + 14} width={16} height={12} className="rf-ladder-ant" />
      <text x={width - 50} y={gy + 14} textAnchor="middle" className="rf-ladder-t">GND</text>
    </svg>
  );
}
