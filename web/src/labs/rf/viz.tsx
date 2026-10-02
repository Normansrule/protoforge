// Small SVG visualisations shared by the RF tabs: polar pattern plot.
import { useState } from 'react';

export interface PolarSeries { name: string; angles: number[]; db: number[]; color?: string; dash?: string }

/** Polar plot in dB. Angle 0° at the top, increasing clockwise. */
export function PolarPlot(props: { series: PolarSeries[]; range?: number; size?: number; title?: string; zeroLabel?: string; ninetyLabel?: string }) {
  const { series, range = 40, size = 260 } = props;
  const c = size / 2;
  const R = c - 26;
  const [hover, setHover] = useState<number | null>(null);
  const toXY = (deg: number, db: number) => {
    const r = (Math.max(0, db + range) / range) * R;
    const a = (deg * Math.PI) / 180;
    return [c + r * Math.sin(a), c - r * Math.cos(a)];
  };
  const rings = [0, -10, -20, -30].filter((d) => d > -range);
  return (
    <figure className="rf-polar" style={{ margin: 0 }}>
      {props.title && <figcaption className="small" style={{ fontWeight: 600, marginBottom: 4 }}>{props.title}</figcaption>}
      <svg
        viewBox={`0 0 ${size} ${size}`}
        width="100%"
        style={{ maxWidth: size, display: 'block', margin: '0 auto' }}
        onMouseMove={(e) => {
          const rect = e.currentTarget.getBoundingClientRect();
          const x = ((e.clientX - rect.left) / rect.width) * size - c;
          const y = ((e.clientY - rect.top) / rect.height) * size - c;
          let deg = (Math.atan2(x, -y) * 180) / Math.PI;
          if (deg < 0) deg += 360;
          setHover(Math.round(deg) % 360);
        }}
        onMouseLeave={() => setHover(null)}
        role="img"
        aria-label={props.title ?? 'polar pattern'}
      >
        {rings.map((d) => (
          <g key={d}>
            <circle cx={c} cy={c} r={((d + range) / range) * R} className="rf-polar-ring" />
            <text x={c + 3} y={c - ((d + range) / range) * R + 11} className="rf-polar-tick">{d} dB</text>
          </g>
        ))}
        {Array.from({ length: 12 }, (_, k) => k * 30).map((a) => {
          const [x, y] = toXY(a, 0);
          const [lx, ly] = [c + (R + 13) * Math.sin((a * Math.PI) / 180), c - (R + 13) * Math.cos((a * Math.PI) / 180)];
          return (
            <g key={a}>
              <line x1={c} y1={c} x2={x} y2={y} className="rf-polar-ring" />
              <text x={lx} y={ly + 4} textAnchor="middle" className="rf-polar-tick">{a}°</text>
            </g>
          );
        })}
        {series.map((s, k) => {
          const d = s.angles.map((a, i) => toXY(a, s.db[i])).map(([x, y], i) => `${i ? 'L' : 'M'}${x.toFixed(1)},${y.toFixed(1)}`).join('');
          return <path key={k} d={d} fill="none" stroke={s.color ?? `var(--series-${k + 1})`} strokeWidth={2} strokeDasharray={s.dash} strokeLinejoin="round" />;
        })}
        {hover !== null && (
          <g>
            <line x1={c} y1={c} x2={toXY(hover, 0)[0]} y2={toXY(hover, 0)[1]} stroke="var(--muted)" strokeDasharray="3 3" />
            {series.map((s, k) => {
              const i = s.angles.findIndex((a) => Math.round(a) === hover);
              if (i < 0) return null;
              const [x, y] = toXY(hover, s.db[i]);
              return <circle key={k} cx={x} cy={y} r={4} fill={s.color ?? `var(--series-${k + 1})`} stroke="var(--surface-1)" strokeWidth={2} />;
            })}
          </g>
        )}
      </svg>
      <div className="rf-polar-readout small tnum">
        {hover !== null
          ? series.map((s, k) => {
              const i = s.angles.findIndex((a) => Math.round(a) === hover);
              return (
                <span key={k}>
                  <i style={{ background: s.color ?? `var(--series-${k + 1})` }} /> {s.name} {hover}°: {i >= 0 ? `${s.db[i].toFixed(1)} dB` : '—'}
                </span>
              );
            })
          : series.map((s, k) => (
              <span key={k}>
                <i style={{ background: s.color ?? `var(--series-${k + 1})` }} /> {s.name}
              </span>
            ))}
      </div>
    </figure>
  );
}
