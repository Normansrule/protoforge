// Top-view ray diagram of the folded display path: micro-display → lens →
// 45° combiner → eye (reduced-eye model), with animated light pulses.
import { forwardRef, useEffect, useMemo, useRef } from 'react';
import { useElementWidth } from '../../components/useElementWidth';
import type { TraceOut } from './types';

export const FIELD_COLORS = ['var(--series-2)', 'var(--series-4)', 'var(--series-3)', 'var(--series-1)', 'var(--series-7)', 'var(--series-5)', 'var(--series-8)'];
export const fieldColor = (i: number, n: number) => (n === 5 ? FIELD_COLORS[i] : FIELD_COLORS[Math.round((i / Math.max(1, n - 1)) * 4)]);

function polyLen(pts: [number, number][]) {
  let s = 0;
  for (let i = 1; i < pts.length; i++) s += Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]);
  return s;
}

export interface RayDiagramProps {
  trace: TraceOut;
  playing: boolean;
  /** Show only chief rays + marginal rays of each field (cleaner for eyebox demos). */
  sparse?: boolean;
  showBack?: boolean;
  showWorld?: boolean;
  highlightField?: number | null;
  /** Called with a new lateral eye offset (mm) while dragging the eye. */
  onDragEye?: (offsetMm: number) => void;
  maxHeight?: number;
}

export const RayDiagram = forwardRef<SVGSVGElement, RayDiagramProps>(function RayDiagram(props, fwd) {
  const { trace: t, playing, sparse, showBack = true, showWorld = true } = props;
  const g = t.geometry;
  const a = t.analysis;
  const [wrapRef, width] = useElementWidth<HTMLDivElement>();
  const pulseRefs = useRef<(SVGPathElement | null)[]>([]);
  const svgRef = useRef<SVGSVGElement | null>(null);
  const dragging = useRef(false);

  // bounds in mm (y up)
  const w2 = Math.abs(g.display[0][1]);
  const xmin = Math.min(g.display[0][0], -a.f_mm) - 7;
  const xmax = Math.max(g.combiner[1][0], g.eye_x + g.eyeball_r) + 14;
  const ytop = Math.max(w2, g.lens_half, g.combiner[0][1]) + 16;
  const ybot = g.eye_y - 2 * g.eyeball_r - 3;
  const W = xmax - xmin;
  const H = ytop - ybot;
  const pxPerMm = width > 0 ? Math.min(width / W, (props.maxHeight ?? Infinity) / H) : 10;
  const narrow = width > 0 && width < 560;
  const fs = (narrow ? 9.5 : 11.5) / pxPerMm; // font size in mm → ~11.5 px
  const sw = 1.3 / pxPerMm;
  const Y = (y: number) => -y;
  const pt = (p: [number, number]) => `${p[0].toFixed(3)},${Y(p[1]).toFixed(3)}`;
  const nFields = t.fields.length;

  const rays = useMemo(() => {
    const out = t.rays.filter((r) => {
      if (!sparse) return true;
      if (r.chief) return true;
      const rs = t.rays.filter((q) => q.field === r.field && !q.chief);
      return r === rs[0] || r === rs[rs.length - 1];
    });
    return out.map((r) => ({ r, len: polyLen(r.points) }));
  }, [t, sparse]);
  const maxLen = Math.max(1, ...rays.map((x) => x.len));

  // animate pulses imperatively (no React re-render per frame)
  useEffect(() => {
    const period = maxLen + 40;
    const pulse = Math.max(2.5, maxLen * 0.05);
    const setAll = (s: number) => {
      pulseRefs.current.forEach((el) => {
        if (!el) return;
        el.setAttribute('stroke-dasharray', `${pulse} ${period}`);
        el.setAttribute('stroke-dashoffset', String(pulse - (s % period)));
      });
    };
    if (!playing) {
      setAll(maxLen * 0.55);
      return;
    }
    let raf = 0;
    const t0 = performance.now();
    const speed = maxLen / 1.6; // cross the diagram in ~1.6 s
    const tick = (now: number) => {
      setAll(((now - t0) / 1000) * speed);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [playing, maxLen, rays]);

  const lensPath = (() => {
    const h = g.lens_half;
    const bulge = Math.min(2.2, h * 0.28);
    return `M0,${Y(h)} Q${bulge * 2},0 0,${Y(-h)} Q${-bulge * 2},0 0,${Y(h)} Z`;
  })();

  const eyeCx = g.eye_x;
  const eyeCy = g.eye_y - g.eyeball_r;
  const onPointer = (e: React.PointerEvent<SVGSVGElement>) => {
    if (!props.onDragEye || !dragging.current || !svgRef.current) return;
    const r = svgRef.current.getBoundingClientRect();
    const xmm = xmin + ((e.clientX - r.left) / r.width) * W;
    props.onDragEye(Math.max(-15, Math.min(15, xmm - g.lc)));
  };
  const setRefs = (el: SVGSVGElement | null) => {
    svgRef.current = el;
    if (typeof fwd === 'function') fwd(el);
    else if (fwd) fwd.current = el;
  };
  const hi = props.highlightField ?? null;

  return (
    <div ref={wrapRef} className="op-diagram" style={{ maxHeight: props.maxHeight }}>
      {width > 0 && (
        <svg
          ref={setRefs}
          xmlns="http://www.w3.org/2000/svg"
          viewBox={`${xmin} ${-ytop} ${W} ${H}`}
          width="100%"
          style={{ display: 'block', maxHeight: props.maxHeight, touchAction: props.onDragEye ? 'none' : undefined }}
          role="img"
          aria-label={`Ray diagram: display ${a.d_mm.toFixed(1)} mm from a ${a.f_mm} mm lens, folded by the combiner into the eye; field of view ${a.hfov_deg.toFixed(1)} degrees`}
          onPointerMove={onPointer}
          onPointerUp={() => (dragging.current = false)}
          onPointerLeave={() => (dragging.current = false)}
          fontFamily="Inter, system-ui, sans-serif"
        >
          <defs>
            <linearGradient id="op-glass" x1="0" x2="1">
              <stop offset="0" stopColor="var(--accent)" stopOpacity={0.12} />
              <stop offset="0.5" stopColor="var(--accent)" stopOpacity={0.3} />
              <stop offset="1" stopColor="var(--accent)" stopOpacity={0.12} />
            </linearGradient>
            <radialGradient id="op-eye" cx="0.5" cy="0.35" r="0.7">
              <stop offset="0" stopColor="var(--surface-1)" />
              <stop offset="1" stopColor="var(--surface-3)" />
            </radialGradient>
            <filter id="op-glow" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation={0.6} /></filter>
          </defs>
          {/* temple pod outline */}
          <rect x={g.display[0][0] - 3} y={Y(Math.max(w2, g.lens_half) + 3)} width={-g.display[0][0] + 5} height={2 * (Math.max(w2, g.lens_half) + 3)} rx={2.5} fill="var(--surface-3)" opacity={0.45} stroke="var(--border-strong)" strokeWidth={sw * 0.8} strokeDasharray={`${sw * 4} ${sw * 3}`} />
          <text x={g.display[0][0] - 2} y={Y(Math.max(w2, g.lens_half) + 4.2)} fontSize={fs * 0.9} fill="var(--muted)">optics housing (temple)</text>
          {/* optical axis */}
          <polyline points={`${xmin + 2},0 ${g.lc},0 ${g.lc},${Y(g.eye_y - 2 * g.eyeball_r)}`} fill="none" stroke="var(--muted)" strokeWidth={sw * 0.7} strokeDasharray={`${sw * 6} ${sw * 3} ${sw} ${sw * 3}`} opacity={0.7} />
          {/* focal points */}
          {[-a.f_mm, a.f_mm].map((x) => (
            <g key={x}>
              <circle cx={x} cy={0} r={sw * 1.8} fill="var(--text-2)" />
              <text x={x} y={-fs * 0.6} fontSize={fs * 0.85} textAnchor="middle" fill="var(--text-2)">F</text>
            </g>
          ))}
          {/* world light through the combiner */}
          {showWorld && [-6, -2, 2, 6].map((dx) => (
            <line key={dx} x1={g.lc + dx} y1={Y(ytop - 1)} x2={g.lc + dx} y2={Y(g.eye_y)} stroke="var(--text-2)" strokeWidth={sw * 0.7} opacity={0.18} />
          ))}
          {showWorld && !narrow && <text x={g.lc + 7.5} y={Y(ytop - 3)} fontSize={fs * 0.85} fill="var(--muted)">world light, {a.see_through_pct.toFixed(0)} % through ↓</text>}
          {/* virtual back-extensions */}
          {showBack && rays.filter(({ r }) => r.through_pupil).map(({ r }, i) => (
            <line key={`b${i}`} x1={r.back[0][0]} y1={Y(r.back[0][1])} x2={r.back[1][0]} y2={Y(r.back[1][1])} stroke={fieldColor(r.field, nFields)} strokeWidth={sw * 0.8} strokeDasharray={`${sw * 3} ${sw * 3}`} opacity={hi === null || hi === r.field ? 0.55 : 0.12} />
          ))}
          {showBack && a.image_kind !== 'real' && (
            <text x={g.lc} y={Y(ytop - 7)} fontSize={fs} textAnchor="middle" fill="var(--text)" fontWeight={650}>
              ↑ virtual image {a.image_kind === 'collimated' ? 'at infinity' : `${a.virtual_image_m.toFixed(2)} m away`}{narrow ? '' : ' (dashed: where the eye thinks light comes from)'}
            </text>
          )}
          {/* eyeball body (under the rays so the focusing cone inside the eye shows) */}
          <circle cx={eyeCx} cy={Y(eyeCy)} r={g.eyeball_r} fill="url(#op-eye)" opacity={0.9} />
          {/* rays */}
          {rays.map(({ r }, i) => {
            const col = fieldColor(r.field, nFields);
            const dim = hi !== null && hi !== r.field;
            return (
              <g key={i} opacity={dim ? 0.15 : 1}>
                <polyline points={r.points.map(pt).join(' ')} fill="none" stroke={col} strokeWidth={r.chief ? sw * 1.4 : sw} opacity={r.through_pupil ? 0.8 : 0.28} strokeLinejoin="round" />
                {!r.through_pupil && <text x={r.points[3][0]} y={Y(r.points[3][1]) + fs * 0.9} fontSize={fs * 0.8} textAnchor="middle" fill={col}>×</text>}
                <path ref={(el) => { pulseRefs.current[i] = el; }} d={`M${r.points.map(pt).join(' L')}`} fill="none" stroke="var(--text)" strokeWidth={sw * 2.2} strokeLinecap="round" opacity={r.through_pupil ? 0.95 : 0.35} filter="url(#op-glow)" />
              </g>
            );
          })}
          {/* display */}
          <rect x={g.display[0][0] - 1.2} y={Y(w2)} width={1.2} height={2 * w2} fill="var(--surface-1)" stroke="var(--text-2)" strokeWidth={sw} />
          {t.fields.map((f, i) => <circle key={i} cx={g.display[0][0]} cy={Y(f.y0)} r={sw * 2.2} fill={fieldColor(i, nFields)} stroke="var(--surface-1)" strokeWidth={sw * 0.6} />)}
          <text x={g.display[0][0] - 1} y={Y(-w2) + fs * 1.4} fontSize={fs} fill="var(--text)" fontWeight={650} textAnchor="start">micro-display</text>
          <text x={g.display[0][0] - 1} y={Y(-w2) + fs * 2.5} fontSize={fs * 0.85} fill="var(--muted)">{(2 * w2).toFixed(1)} mm · {a.pixel_pitch_um.toFixed(1)} µm pixels</text>
          {/* lens */}
          <path d={lensPath} fill="url(#op-glass)" stroke="var(--accent)" strokeWidth={sw} />
          <text x={0} y={Y(g.lens_half) - fs * 0.5} fontSize={fs} textAnchor="middle" fill="var(--text)" fontWeight={650}>lens f = {a.f_mm.toFixed(1)} mm</text>
          <text x={0} y={Y(-g.lens_half) + fs * 1.2} fontSize={fs * 0.85} textAnchor="middle" fill="var(--muted)">⌀ {a.aperture_mm.toFixed(1)} mm · f/{a.f_number.toFixed(2)}</text>
          {/* combiner */}
          <line x1={g.combiner[0][0]} y1={Y(g.combiner[0][1])} x2={g.combiner[1][0]} y2={Y(g.combiner[1][1])} stroke="var(--series-7)" strokeWidth={sw * 3} strokeLinecap="round" opacity={0.85} />
          <text x={g.combiner[1][0] + 1.2} y={Y(g.combiner[1][1]) + fs * 0.4} fontSize={fs} fill="var(--text)" fontWeight={650}>combiner</text>
          <text x={g.combiner[1][0] + 1.2} y={Y(g.combiner[1][1]) + fs * 1.5} fontSize={fs * 0.85} fill="var(--muted)">R {(100 - a.see_through_pct).toFixed(0)} % · 45°</text>
          {/* eye */}
          <g style={{ cursor: props.onDragEye ? 'grab' : undefined }} onPointerDown={(e) => { if (props.onDragEye) { dragging.current = true; (e.target as Element).setPointerCapture?.(e.pointerId); } }}>
            <circle cx={eyeCx} cy={Y(eyeCy)} r={g.eyeball_r} fill="transparent" stroke="var(--border-strong)" strokeWidth={sw} />
            <path d={`M${eyeCx - 5.2},${Y(g.eye_y - 1.2)} Q${eyeCx},${Y(g.eye_y + 1.6)} ${eyeCx + 5.2},${Y(g.eye_y - 1.2)}`} fill="none" stroke="var(--text-2)" strokeWidth={sw} opacity={0.7} />
            {/* iris with the pupil gap */}
            <line x1={eyeCx - 5.5} y1={Y(g.eye_y)} x2={eyeCx - g.pupil_half} y2={Y(g.eye_y)} stroke="var(--series-3)" strokeWidth={sw * 2.5} />
            <line x1={eyeCx + g.pupil_half} y1={Y(g.eye_y)} x2={eyeCx + 5.5} y2={Y(g.eye_y)} stroke="var(--series-3)" strokeWidth={sw * 2.5} />
            {/* retina */}
            <path d={`M${eyeCx - 7},${Y(g.retina_y + 0.9)} Q${eyeCx},${Y(g.retina_y - 0.7)} ${eyeCx + 7},${Y(g.retina_y + 0.9)}`} fill="none" stroke="var(--series-5)" strokeWidth={sw * 1.8} opacity={0.8} />
            {t.fields.map((f, i) => f.retina_x !== null && f.pupil_fill > 0 ? <circle key={i} cx={f.retina_x} cy={Y(g.retina_y)} r={sw * 1.8} fill={fieldColor(i, nFields)} /> : null)}
            <text x={eyeCx + g.eyeball_r + 1} y={Y(eyeCy) - fs * 0.2} fontSize={fs} fill="var(--text)" fontWeight={650}>eye</text>
            <text x={eyeCx + g.eyeball_r + 1} y={Y(eyeCy) + fs * 0.9} fontSize={fs * 0.85} fill="var(--muted)">pupil {(2 * g.pupil_half).toFixed(1)} mm</text>
            <text x={eyeCx + g.eyeball_r + 1} y={Y(eyeCy) + fs * 2} fontSize={fs * 0.85} fill="var(--muted)">{props.onDragEye ? 'drag me ↔' : `relief ${g.eye_relief.toFixed(0)} mm`}</text>
          </g>
          {a.min_lens_to_combiner_mm > g.lc + 1e-6 && (
            <g>
              <circle cx={Math.max(0.5, g.lc - g.lens_half)} cy={Y(g.lens_half)} r={3.2} fill="none" stroke="var(--bad)" strokeWidth={sw * 1.6} strokeDasharray={`${sw * 3} ${sw * 2}`} />
              <text x={Math.max(0.5, g.lc - g.lens_half) + 3.8} y={Y(g.lens_half + 2.4)} fontSize={fs * 0.9} fill="var(--bad-text)" fontWeight={650}>✕ combiner hits the lens{narrow ? '' : ` (needs ≥ ${a.min_lens_to_combiner_mm.toFixed(1)} mm)`}</text>
            </g>
          )}
          {/* dimension: display to lens */}
          <g stroke="var(--muted)" strokeWidth={sw * 0.8} fill="none">
            <line x1={g.display[0][0]} y1={Y(-w2 - 4.2) - fs * 2.4} x2={0} y2={Y(-w2 - 4.2) - fs * 2.4} />
          </g>
          <text x={g.display[0][0] / 2} y={Y(-w2 - 4.2) - fs * 2.8} fontSize={fs * 0.85} textAnchor="middle" fill="var(--text-2)">d = {a.d_mm.toFixed(2)} mm</text>
        </svg>
      )}
    </div>
  );
});

/** To-scale unfolded view of the eye and its virtual image: the field-of-view cone. */
export function FarView({ trace: t, reqLo = 1, reqHi = 3 }: { trace: TraceOut; reqLo?: number; reqHi?: number }) {
  const a = t.analysis;
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const virtual = a.image_kind === 'virtual';
  const D = virtual ? a.virtual_image_m : 3.5;
  const maxX = Math.max(3.4, D * 1.08);
  const half = Math.tan((a.hfov_deg * Math.PI) / 360) * (virtual ? D : maxX * 0.95);
  const Wm = maxX + 0.25;
  const pxPerM = width > 0 ? width / Wm : 200;
  const fs = (width > 0 && width < 560 ? 9.5 : 11.5) / pxPerM;
  const halfH = Math.max(half + fs * 2.2, 0.35);
  const Hm = 2 * halfH;
  const sw = 1.3 / pxPerM;
  const n = t.fields.length;
  return (
    <div ref={ref}>
      {width > 0 && (
        <svg viewBox={`-0.15 ${-halfH} ${Wm} ${Hm}`} width="100%" style={{ display: 'block' }} role="img" aria-label="field of view cone to scale" fontFamily="Inter, system-ui, sans-serif">
          <rect x={reqLo} y={-halfH} width={reqHi - reqLo} height={Hm} fill="color-mix(in srgb, var(--good) 10%, transparent)" />
          <text x={(reqLo + reqHi) / 2} y={-halfH + fs * 1.3} fontSize={fs} textAnchor="middle" fill="var(--good-text)">R-006: image {reqLo}–{reqHi} m</text>
          {[0, 1, 2, 3].filter((m) => m <= maxX).map((m) => (
            <g key={m}>
              <line x1={m} x2={m} y1={halfH - fs * 1.6} y2={halfH - fs * 1.1} stroke="var(--muted)" strokeWidth={sw} />
              <text x={m} y={halfH - fs * 0.2} fontSize={fs * 0.9} textAnchor="middle" fill="var(--muted)">{m} m</text>
            </g>
          ))}
          <polygon points={`0,0 ${virtual ? D : maxX},${-half} ${virtual ? D : maxX},${half}`} fill="color-mix(in srgb, var(--accent) 12%, transparent)" stroke="var(--accent)" strokeWidth={sw} strokeDasharray={virtual ? undefined : `${sw * 4} ${sw * 3}`} />
          {t.fields.map((f, i) => {
            const ang = (f.angle_deg * Math.PI) / 180;
            const x = virtual ? D : maxX;
            return <line key={i} x1={0} y1={0} x2={x} y2={-Math.tan(ang) * x} stroke={fieldColor(i, n)} strokeWidth={sw} opacity={0.7} />;
          })}
          {virtual && <line x1={D} x2={D} y1={-half} y2={half} stroke="var(--text)" strokeWidth={sw * 2.2} strokeDasharray={`${sw * 4} ${sw * 2}`} />}
          <circle cx={0} cy={0} r={0.012 / 1} fill="var(--text)" />
          <text x={0.03} y={-fs * 0.5} fontSize={fs} fill="var(--text)">eye</text>
          <text x={0.45} y={fs * 0.4} fontSize={fs} fill="var(--text)" fontWeight={650}>FOV {a.hfov_deg.toFixed(1)}°</text>
          {virtual ? (
            <text x={D - 0.03} y={-half - fs * 0.5} fontSize={fs} textAnchor="end" fill="var(--text)">{width < 560 ? `${(a.image_width_mm / 1000).toFixed(2)} m wide` : `virtual image ${(a.image_width_mm / 1000).toFixed(2)} m wide at ${D.toFixed(2)} m`}</text>
          ) : (
            <text x={maxX - 0.03} y={-half - fs * 0.5} fontSize={fs} textAnchor="end" fill="var(--text)">{a.image_kind === 'collimated' ? 'image at infinity →' : 'real image — not viewable'}</text>
          )}
        </svg>
      )}
    </div>
  );
}

/** Serialise an SVG element with theme variables resolved, for download. */
export function svgForDownload(svg: SVGSVGElement | null): string {
  if (!svg) return '';
  const cs = getComputedStyle(document.documentElement);
  let s = new XMLSerializer().serializeToString(svg);
  s = s.replace(/var\((--[a-z0-9-]+)\)/gi, (_, name: string) => cs.getPropertyValue(name).trim() || '#888');
  const bg = cs.getPropertyValue('--surface-2').trim();
  return s.replace(/<svg([^>]*)>/, `<svg$1><rect x="-1000" y="-1000" width="3000" height="3000" fill="${bg}"/>`);
}
