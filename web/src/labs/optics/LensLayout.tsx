// 2D meridional layout of a sequential lens system: surface profiles, glass
// elements, the eye pupil (stop), the unfolded combiner, the display, and
// real refracted ray fans in three wavelengths. Light pulses travel along the
// rays when playing (off with prefers-reduced-motion).
import { forwardRef, useMemo } from 'react';
import type { Layout } from './proTypes';
import { waveColor } from './proTypes';

export interface LensLayoutProps {
  layout: Layout;
  waves: number;
  /** Which wavelengths to draw. */
  showWave?: boolean[];
  /** Highlighted surface index (table hover). */
  highlight?: number | null;
  onPick?: (surface: number) => void;
  playing?: boolean;
  maxHeight?: number;
  /** Labels for the HUD context (eye, combiner, display). */
  hud?: boolean;
  compact?: boolean;
}

function path(pts: [number, number][]) {
  return pts.map((p, k) => `${k ? 'L' : 'M'}${p[0].toFixed(4)},${(-p[1]).toFixed(4)}`).join(' ');
}

export const LensLayout = forwardRef<SVGSVGElement, LensLayoutProps>(function LensLayout(props, ref) {
  const { layout: L, waves, highlight, hud, compact } = props;
  const showWave = props.showWave ?? Array.from({ length: waves }, () => true);
  const box = useMemo(() => {
    const zmin = Math.min(L.z_min, 0) - (hud ? 6 : 2);
    const zmax = Math.max(L.z_max, L.z_image) + 3;
    const ym = Math.max(L.y_max, 1) * 1.18 + 1.2;
    return { zmin, zmax, ym, w: zmax - zmin, h: 2 * ym };
  }, [L, hud]);
  const unit = box.w / 100; // 1 % of the width in mm, for strokes and text
  const fs = Math.max(0.55, unit * (compact ? 2.4 : 1.9));
  const sw = Math.max(0.04, unit * 0.22);
  const bar = box.w > 60 ? 10 : box.w > 25 ? 5 : 2;
  return (
    <svg
      ref={ref}
      className="op-lens-svg"
      viewBox={`${box.zmin} ${-box.ym} ${box.w} ${box.h}`}
      preserveAspectRatio="xMidYMid meet"
      style={{ width: '100%', height: 'auto', maxHeight: props.maxHeight, display: 'block' }}
      role="img"
      aria-label="lens layout with refracted ray fans"
    >
      {/* optical axis */}
      <line x1={box.zmin} y1={0} x2={box.zmax} y2={0} stroke="var(--axis)" strokeWidth={sw * 0.7} strokeDasharray={`${unit * 1.5} ${unit}`} />
      {/* glass elements */}
      {L.elements.map((e, k) => (
        <path key={`e${k}`} d={path(e.outline) + ' Z'} className={`op-glass ${e.material === 'PC' || e.material === 'PS' || e.material.includes('SF') ? 'flint' : ''}`} strokeWidth={sw * 0.5} />
      ))}
      {/* surfaces */}
      {L.surfaces.map((s) => {
        const hi = highlight === s.index;
        if (s.index === 0) {
          const h = s.semi_diameter;
          const t = box.ym * 0.72;
          return (
            <g key={s.index} className={`op-surf ${hi ? 'hi' : ''}`} onClick={() => props.onPick?.(s.index)}>
              <line x1={s.z} y1={-t} x2={s.z} y2={-h} strokeWidth={sw * 2} />
              <line x1={s.z} y1={h} x2={s.z} y2={t} strokeWidth={sw * 2} />
              {hud && !compact && <path d={`M${s.z - 1.2},${-h * 2.4} A ${h * 2.6} ${h * 2.6} 0 0 0 ${s.z - 1.2},${h * 2.4}`} fill="none" strokeWidth={sw} opacity={0.5} />}
              <text x={s.z} y={t + fs * 1.3} fontSize={fs} textAnchor="middle" className="op-lbl">{hud ? 'eye pupil (stop)' : 'stop'}</text>
            </g>
          );
        }
        if (s.fold) {
          return (
            <g key={s.index} className={`op-surf fold ${hi ? 'hi' : ''}`} onClick={() => props.onPick?.(s.index)}>
              <line x1={s.z} y1={-box.ym * 0.8} x2={s.z} y2={box.ym * 0.8} strokeWidth={sw} strokeDasharray={`${unit} ${unit * 0.8}`} />
              {!compact && <text x={s.z} y={-box.ym * 0.84} fontSize={fs} textAnchor="middle" className="op-lbl">combiner (unfolded)</text>}
            </g>
          );
        }
        if (s.ideal) {
          const h = s.semi_diameter;
          return (
            <g key={s.index} className={`op-surf ${hi ? 'hi' : ''}`} onClick={() => props.onPick?.(s.index)}>
              <line x1={s.z} y1={-h} x2={s.z} y2={h} strokeWidth={sw * 1.6} />
              <path d={`M${s.z - unit},${-h + unit} L${s.z},${-h} L${s.z + unit},${-h + unit} M${s.z - unit},${h - unit} L${s.z},${h} L${s.z + unit},${h - unit}`} fill="none" strokeWidth={sw * 1.2} />
            </g>
          );
        }
        return (
          <g key={s.index} className={`op-surf ${hi ? 'hi' : ''} ${s.mirror ? 'mirror' : ''}`} onClick={() => props.onPick?.(s.index)}>
            <path d={path(s.profile)} fill="none" strokeWidth={hi ? sw * 2.4 : sw * 1.3} />
            <path d={path(s.profile)} fill="none" stroke="transparent" strokeWidth={unit * 2.5} />
          </g>
        );
      })}
      {/* rays */}
      <g className={`op-rays ${props.playing ? 'playing' : ''}`}>
        {L.rays.filter((r) => showWave[r.wave]).map((r, k) => (
          <g key={k}>
            <path d={path(r.pts)} fill="none" stroke={waveColor(r.wave, waves)} strokeWidth={sw * 0.8} strokeOpacity={0.75} strokeLinejoin="round" />
            {props.playing && <path d={path(r.pts)} fill="none" stroke={waveColor(r.wave, waves)} strokeWidth={sw * 2.2} className="op-pulse" pathLength={100} style={{ animationDelay: `${-(r.field * 0.37 + r.py * 0.05) % 1.6}s` }} />}
            {r.fail && r.pts.length > 0 && (
              <g transform={`translate(${r.pts[r.pts.length - 1][0]} ${-r.pts[r.pts.length - 1][1]})`} className="op-fail">
                <path d={`M${-unit * 0.6},${-unit * 0.6} L${unit * 0.6},${unit * 0.6} M${-unit * 0.6},${unit * 0.6} L${unit * 0.6},${-unit * 0.6}`} strokeWidth={sw} />
              </g>
            )}
          </g>
        ))}
      </g>
      {/* display / image plane */}
      <g className="op-imgplane">
        <line x1={L.z_image} y1={-Math.max(L.image_half_height, 1) * 1.15} x2={L.z_image} y2={Math.max(L.image_half_height, 1) * 1.15} strokeWidth={sw * 3.2} />
        <text x={L.z_image} y={Math.max(L.image_half_height, 1) * 1.15 + fs * 1.3} fontSize={fs} textAnchor="middle" className="op-lbl">{hud ? 'display' : 'image'}</text>
      </g>
      {/* scale bar */}
      {!compact && (
        <g className="op-scale">
          <line x1={box.zmin + unit * 2} y1={box.ym - fs * 0.9} x2={box.zmin + unit * 2 + bar} y2={box.ym - fs * 0.9} strokeWidth={sw * 1.4} />
          <text x={box.zmin + unit * 2 + bar / 2} y={box.ym - fs * 1.4} fontSize={fs * 0.9} textAnchor="middle" className="op-lbl">{bar} mm</text>
        </g>
      )}
    </svg>
  );
});
