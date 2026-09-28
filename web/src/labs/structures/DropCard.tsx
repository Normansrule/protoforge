// Drop test estimate: a 1-degree-of-freedom spring impact built on the
// temple stiffness from the voxel FEA, animated (fall → compress → rebound).
import { useState } from 'react';
import { Card, Explainer, Formula, Stat } from '../../components/ui';
import type { IrisOut } from './types';
import { fmt, prefersReducedMotion, sfTone, useRaf } from './util';

export function DropCard({ iris }: { iris: IrisOut | null }) {
  const d = iris?.drop;
  const [t, setT] = useState(0.62);
  const [playing, setPlaying] = useState(!prefersReducedMotion());
  const period = 3.2;
  useRaf(playing, (dt) => setT((v) => (v + dt / period) % 1));
  // Phase: 0–0.55 free fall (y ∝ t²), 0.55–0.75 compression+rebound (sin), 0.75–1 bounce up and fade.
  let drop = 0;
  let comp = 0;
  if (t < 0.55) drop = 1 - (t / 0.55) ** 2;
  else if (t < 0.75) comp = Math.sin(((t - 0.55) / 0.2) * Math.PI);
  else drop = 0.25 * Math.sin(((t - 0.75) / 0.25) * Math.PI);
  const H = 150;
  const ground = 170;
  const springRest = 40;
  const springLen = springRest * (1 - 0.55 * comp);
  const massY = ground - springLen - 26 - drop * H * 0.8;
  const coils = 7;
  // The spring stands on the ground; its top follows the mass only in contact.
  const springTop = ground - springLen;
  const pts: string[] = [];
  for (let i = 0; i <= coils * 2; i++) {
    const y = springTop + ((ground - springTop) * i) / (coils * 2);
    const x = 120 + (i === 0 || i === coils * 2 ? 0 : i % 2 ? 12 : -12);
    pts.push(`${x},${y}`);
  }
  return (
    <Card
      title="Drop: 1.5 m onto the electronics pod"
      subtitle="Energy method on the FEA stiffness — an upper bound on the impact force"
      actions={<button className="btn small" onClick={() => setPlaying(!playing)}>{playing ? '❚❚ Pause' : '▶ Play'}</button>}
    >
      {!d ? (
        <div className="muted small">Computing…</div>
      ) : (
        <div className="stack" style={{ gap: 12 }}>
          <div className="st-drop">
            <svg viewBox="0 0 240 190" width="100%" style={{ maxHeight: 220 }} role="img" aria-label="Mass-spring impact animation">
              <line x1={20} x2={220} y1={ground} y2={ground} stroke="var(--text-2)" strokeWidth={2} />
              {Array.from({ length: 12 }, (_, i) => (
                <line key={i} x1={24 + i * 17} x2={14 + i * 17} y1={ground} y2={ground + 10} stroke="var(--muted)" />
              ))}
              <polyline points={pts.join(' ')} fill="none" stroke="var(--series-2)" strokeWidth={2} strokeLinejoin="round" />
              <rect x={96} y={massY} width={48} height={26} rx={5} fill="var(--series-1)" />
              <text x={120} y={massY + 17} textAnchor="middle" fontSize={11} fill="#fff" fontWeight={700}>
                {fmt(d.moving_mass_g, 0)} g
              </text>
              <text x={150} y={ground - springLen / 2 + 4} fontSize={10.5} fill="var(--text-2)">
                k = {fmt(d.stiffness_n_mm, 1)} N/mm
              </text>
              {t >= 0.55 && t < 0.75 && (
                <text x={20} y={20} fontSize={12} fill="var(--text)" fontWeight={700}>
                  F = {fmt(d.peak_force_n * comp, 0)} N
                </text>
              )}
              {t < 0.55 && (
                <text x={20} y={20} fontSize={12} fill="var(--text-2)">
                  v = {fmt(d.impact_speed_m_s * (t / 0.55), 2)} m/s
                </text>
              )}
            </svg>
          </div>
          <div className="grid grid-2" style={{ gap: 10 }}>
            <Stat label="Impact speed" value={fmt(d.impact_speed_m_s, 2)} unit="m/s" note={`${fmt(d.energy_j * 1000, 0)} mJ`} />
            <Stat label="Peak force" value={fmt(d.peak_force_n, 0)} unit="N" note={`${fmt(d.peak_accel_g, 0)} g, ${fmt(d.contact_time_ms, 2)} ms contact`} />
            <Stat label="Peak stress" value={fmt(d.peak_stress_mpa, 0)} unit="MPa" note={d.governing} />
            <Stat label="Drop safety factor" value={fmt(d.safety_factor, 2)} unit="×" tone={sfTone(d.safety_factor, 1)} note={d.safety_factor >= 1 ? '✓ survives the bound' : '✕ bound exceeds yield'} />
          </div>
          <Explainer title="The spring model">
            <Formula>{`½ m v² = ½ k δ²   →   δ = v √(m/k),   F = k δ = v √(k m),   v = √(2 g h)
contact time ≈ π √(m/k)      stress = (stress per newton from FEA) × F`}</Formula>
            <ul className="small" style={{ margin: 0, paddingLeft: 18 }}>
              {d.assumptions.map((a, i) => (
                <li key={i}>{a}</li>
              ))}
            </ul>
          </Explainer>
        </div>
      )}
    </Card>
  );
}
