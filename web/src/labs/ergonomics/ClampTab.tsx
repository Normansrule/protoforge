// Tab 3 — Temple clamp: temples as stepped cantilevers plus front bending →
// clamp force vs head breadth percentile, against a comfort band.
import { useMemo, useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useElementWidth } from '../../components/useElementWidth';
import { csv, f1, GROUP_LABEL, Kv, PlayButton, REFS, useClock, useErgo, usePlaying, VerdictChip } from './shared';
import type { ClampModel, ClampOut, ErgoConfig, Segment } from './types';

export function ClampTab() {
  const { project, cfg, setCfg } = useErgo();
  const r = useCompute<ClampOut>('ergonomics.clamp', { project }, !!cfg);
  const c = r.data;
  if (!cfg) return <div className="muted small">Loading the engine…</div>;
  const p50 = c?.rows.find((x) => x.group === 'combined' && x.percentile === 50);
  return (
    <div className="stack">
      <ErrorBox error={r.error} />
      {c && (
        <div className="grid grid-4 hf-stats">
          <Stat label="Clamp at P95 head" value={f1(c.force_p95_n, 2)} unit="N" tone={c.force_p95_n <= c.band.max_n ? 'pass' : 'fail'} note={`per side at ${f1(c.head_p95_mm, 0)} mm · ergonomics.clamp_force_n`} />
          <Stat label="Clamp at median head" value={f1(p50?.force_n, 2)} unit="N" note={p50 ? `${f1(p50.head_breadth_mm, 0)} mm · ${p50.verdict}` : ''} />
          <Stat label="Heads inside the band" value={f1(c.band_coverage_pct, 0)} unit="%" tone={c.band_coverage_pct >= 50 ? 'pass' : 'warn'} note={`band ${f1(c.band.min_n, 2)}–${f1(c.band.max_n, 2)} N ↔ ${f1(c.band_heads_mm[0], 0)}–${f1(c.band_heads_mm[1], 0)} mm`} />
          <Stat label="Splay that centres the band" value={f1(c.best_splay_deg)} unit="°" note={`→ ${f1(c.best_splay_coverage_pct, 0)} % of heads (now ${f1(cfg.temple_splay_deg)}°)`} />
        </div>
      )}
      {c && <TopView c={c} cfg={cfg} />}
      {c && (
        <div className="hf-split">
          <Card
            title="Clamp force vs head-breadth percentile"
            subtitle="Force per side; dashed lines are the comfort band (slip limit below, sustained-pressure limit above)."
            actions={
              <DownloadButton
                small
                filename="iris-clamp-force.csv"
                label="CSV"
                content={() => csv(['percentile', 'hb_men_mm', 'force_men_n', 'hb_women_mm', 'force_women_n', 'hb_combined_mm', 'force_combined_n'], c.percentiles.map((p, i) => [p, c.hb_vs_pct[0][i], c.force_vs_pct[0][i], c.hb_vs_pct[1][i], c.force_vs_pct[1][i], c.hb_vs_pct[2][i], c.force_vs_pct[2][i]]))}
              />
            }
          >
            <Plot
              height={260}
              xLabel="head breadth percentile"
              yLabel="clamp force per side (N)"
              series={[
                { name: 'Men', x: c.percentiles, y: c.force_vs_pct[0], color: 'var(--series-1)' },
                { name: 'Women', x: c.percentiles, y: c.force_vs_pct[1], color: 'var(--series-2)' },
                { name: 'Combined', x: c.percentiles, y: c.force_vs_pct[2], color: 'var(--text-2)', dash: '5 4' },
              ]}
              hLines={[
                { value: c.band.min_n, label: 'slip limit', color: 'var(--series-4)' },
                { value: c.band.max_n, label: 'pressure limit', color: 'var(--bad)' },
              ]}
              yZero
            />
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>Group</th><th>Percentile</th><th>Head breadth</th><th>Force</th><th>Verdict</th></tr></thead>
                <tbody>
                  {c.rows.map((row) => (
                    <tr key={`${row.group}${row.percentile}`}>
                      <td>{GROUP_LABEL[row.group]}</td>
                      <td>P{row.percentile}</td>
                      <td className="tnum">{f1(row.head_breadth_mm)} mm</td>
                      <td className="tnum">{f1(row.force_n, 2)} N</td>
                      <td><VerdictChip v={row.verdict} /></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
          <Card title="Band & geometry assumptions" subtitle="Saved in the project; the metrics use them." actions={<button className="btn small" disabled={Math.abs(c.best_splay_deg - cfg.temple_splay_deg) < 0.05} onClick={() => setCfg({ temple_splay_deg: Number(c.best_splay_deg.toFixed(1)) })}>Use {f1(c.best_splay_deg)}° splay</button>}>
            <Slider label="Temple splay at the hinge stop" value={cfg.temple_splay_deg} min={-2} max={16} step={0.5} unit="°" onChange={(v) => setCfg({ temple_splay_deg: v })} />
            <Slider label="Contact point in front of the ear bend" value={cfg.contact_before_bend_mm} min={0} max={30} step={1} unit="mm" onChange={(v) => setCfg({ contact_before_bend_mm: v })} />
            <Slider label="Temple length touching the head" value={cfg.temple_contact_length_mm} min={10} max={80} step={1} unit="mm" onChange={(v) => setCfg({ temple_contact_length_mm: v })} />
            <Slider label="Sustained-pressure limit" value={cfg.pressure_limit_kpa} min={1} max={15} step={0.1} unit="kPa" onChange={(v) => setCfg({ pressure_limit_kpa: v })} hint="≈ 4.3 kPa = 32 mmHg capillary pressure (Landis 1930)" />
            <Slider label="Skin friction coefficient μ" value={cfg.friction_mu} min={0.1} max={1} step={0.05} onChange={(v) => setCfg({ friction_mu: v })} />
            <Slider label="Look-down angle for the slip check" value={cfg.look_down_deg} min={0} max={70} step={1} unit="°" onChange={(v) => setCfg({ look_down_deg: v })} />
            <Kv
              rows={[
                ['Material', `${c.material}, E = ${f1(c.model.e_mpa / 1000, 2)} GPa (typical)`],
                ['Contact behind the hinge', `${f1(c.model.l_c)} mm`],
                ['Spring hinge', c.model.hinge && c.model.hinge_crack_n != null ? `opens past ${f1(c.model.hinge_crack_n, 2)} N (${f1(c.model.hinge.crack_torque_nmm, 1)} N·mm), stop at ${f1(c.model.hinge.stop_deg, 0)}° — Mechanisms Lab` : 'none (rigid at the stop)'],
                ['Unloaded spread at contact', `${f1(c.model.s0)} mm`],
                ['Root bending stress at P95', `${f1(c.root_stress_p95_mpa)} MPa`],
                ['Frame yaw from L/R asymmetry', `${f1(c.yaw_p95_deg, 2)}° at P95`],
              ]}
            />
          </Card>
        </div>
      )}
      {c && <ComplianceCard m={c.model} />}
      {c && c.notes.map((n) => <div key={n} className="hf-note">{n}</div>)}
      <div className="grid grid-2">
        <Explainer title="Temples are stepped cantilevers" refs={[REFS.roark]}>
          Built in at the hinge and pushed outwards at the contact, each temple deflects by the unit-load (Castigliano) integral over its sections — the stiff hollow pod, then the
          slender arm:
          <Formula>{'c = ∫₀^{L_c} (L_c − s)² / (E I(s)) ds    (uniform: c = L³ / 3EI)    I_arm = h w³/12,  I_box = (h w³ − h_i w_i³)/12'}</Formula>
        </Explainer>
        <Explainer title="…and the front bows" refs={[REFS.roark]}>
          The temple force acts on a lever L_c, so each hinge sees a moment M = F L_c. The front (rim bars and bridge) bends under these equal end moments and both hinges rotate:
          <Formula>{'θ = M Σ ℓᵢ/(E Iᵢ)   →   F = max(0, H − S₀) / (L_c² Σ ℓ/EI + c_R + c_L),   S₀ = 2x_in + 2L_c tan(splay)'}</Formula>
          A long lever makes the front the softest spring — and a stiffer front (bonded lenses, metal core) raises the clamp.
        </Explainer>
        <Explainer title="…unless the hinge gives first" refs={[REFS.roark]}>
          Iris's spring hinge stays on its stop until the moment F L_c reaches the spring's over-travel torque T(0) from the Mechanisms Lab; then the temple swings open by α
          with F L_c = T(α) and each contact moves out by L_c sin α:
          <Formula>{'H − S₀ = F c_total + 2 L_c sin α(F),   α = 0 while F L_c ≤ T(0)'}</Formula>
          So the hinge is a clamp-force limiter: wide heads are held near T(0)/L_c until the over-travel stop. Too weak a spring and every head is loose; no hinge and the wide heads
          are squeezed.
        </Explainer>
        <Explainer title="Where the comfort band comes from" refs={[REFS.landis, REFS.derler]}>
          Lower edge: the force that stops the glasses sliding down the nose when you look down {cfg.look_down_deg}° with μ = {cfg.friction_mu} (Weight & balance tab). Upper edge:
          the temple contact pressure F / (contact length × temple height) reaching the {cfg.pressure_limit_kpa} kPa sustained-pressure limit. Both are typical, assumption-driven
          values — confirm with a wear test on real heads. Head breadth (the widest part of the head, above the ears) stands in for the width the temples span, so forces here are
          upper bounds.
        </Explainer>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------- spring hinge

/** Mirrors `ClampModel::force` in the engine: frame bending in series with the spring hinge. */
function clampForce(m: ClampModel, H: number): { f: number; alpha: number } {
  const d = H - m.s0;
  if (d <= 0) return { f: 0, alpha: 0 };
  const h = m.hinge;
  if (!h) return { f: d / m.c_total, alpha: 0 };
  const rad = Math.PI / 180;
  const table = h.alpha_deg.map((a, i) => {
    const f = h.torque_nmm[i] / m.l_c;
    return [f * m.c_total + 2 * m.l_c * Math.sin(a * rad), f, a];
  });
  if (d <= table[0][0]) return { f: d / m.c_total, alpha: 0 };
  const last = table[table.length - 1];
  if (d >= last[0]) return { f: Math.max(last[1], (d - 2 * m.l_c * Math.sin(h.stop_deg * rad)) / m.c_total), alpha: h.stop_deg };
  let i = 1;
  while (i < table.length - 1 && table[i][0] <= d) i++;
  const [a, b] = [table[i - 1], table[i]];
  const w = b[0] > a[0] ? (d - a[0]) / (b[0] - a[0]) : 1;
  return { f: a[1] + w * (b[1] - a[1]), alpha: a[2] + w * (b[2] - a[2]) };
}

// ---------------------------------------------------------------- top view

/** Deflection shape of a stepped cantilever under tip load f at l: v(s) = ∫∫ M/EI. */
function shape(segs: Segment[], l: number, f: number, n = 60): (s: number) => number {
  const ei = (s: number) => segs.find((g) => s >= g.s0 && s < g.s1)?.ei ?? segs[segs.length - 1].ei;
  const ds = l / n;
  const v: number[] = [0];
  let th = 0;
  for (let i = 0; i < n; i++) {
    const s = (i + 0.5) * ds;
    th += ((f * (l - s)) / ei(s)) * ds;
    v.push(v[i] + th * ds);
  }
  return (s: number) => {
    if (s <= l) {
      const k = Math.min(n - 1, Math.floor(s / ds));
      const t = s / ds - k;
      return v[k] + (v[k + 1] - v[k]) * t;
    }
    return v[n] + th * (s - l); // straight beyond the load
  };
}

function TopView({ c, cfg }: { c: ClampOut; cfg: ErgoConfig }) {
  const [playing, setPlaying] = usePlaying();
  const [manual, setManual] = useState(95);
  const t = useClock(playing);
  const pct = playing ? 50 + 47 * Math.sin((t * 2 * Math.PI) / 6) : manual;
  const idx = Math.max(0, Math.min(c.percentiles.length - 1, Math.round(pct) - 1));
  const H = c.hb_vs_pct[2][idx];
  const m = c.model;
  const { f: F, alpha } = clampForce(m, H);
  const hingeOpen = (alpha * Math.PI) / 180; // spring-hinge over-travel per side
  const verdict = F < c.band.min_n ? 'too loose' : F > c.band.max_n ? 'too tight' : 'comfortable';
  const g = c.frame.geom;
  const theta = (F * m.l_c * m.front_flex) / 2; // per-side hinge rotation from front bending
  const splay = (m.splay_deg * Math.PI) / 180;
  const vr = useMemo(() => shape(m.right, m.l_c, F), [m, F]);
  const vl = useMemo(() => shape(m.left, m.l_c, F), [m, F]);
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const Wd = Math.max(300, width);
  const Hd = Math.min(520, Math.max(320, Wd * 0.7));
  const span = 220;
  const k = Math.min(Wd / span, (Hd - 64) / 250);
  const X = (x: number) => Wd / 2 + x * k;
  const Y = (y: number) => 58 + (14 - y) * k; // forward (+y) up
  const lenTotal = g.bend_from_pin_mm + 30;
  const temple = (side: 1 | -1) => {
    const v = side > 0 ? vr : vl;
    const pts: string[] = [];
    for (let i = 0; i <= 40; i++) {
      const s = (lenTotal * i) / 40;
      const x = side * (g.temple_inner_x + s * Math.tan(splay) + s * Math.sin(theta + hingeOpen) + v(s));
      const y = g.pin_y - s;
      pts.push(`${X(x).toFixed(1)},${Y(y).toFixed(1)}`);
    }
    return pts.join(' ');
  };
  const yC = g.pin_y - m.l_c;
  const rx = H / 2;
  const ry = 1.29 * rx;
  const headCy = -(g.front_thickness_mm + 9) - ry;
  const col = verdict === 'comfortable' ? 'var(--good)' : verdict === 'too tight' ? 'var(--bad)' : 'var(--series-4)';
  const aLen = Math.min(70, 14 + F * 26);
  return (
    <Card
      title="Top view: the temples spread to the head"
      subtitle="True scale. The head breadth sweeps the combined population (P3 → P97); temple shapes are the computed cantilever deflection plus the front's hinge rotation."
      actions={<PlayButton playing={playing} onChange={setPlaying} />}
    >
      {!playing && <Slider label="Head breadth percentile (combined)" value={manual} min={1} max={99} step={1} format={(v) => `P${v.toFixed(0)}`} onChange={setManual} />}
      <div ref={ref}>
        <svg width={Wd} height={Hd} className="hf-svg" role="img" aria-label={`Top view, head ${H.toFixed(0)} mm, clamp ${F.toFixed(2)} N, ${verdict}`}>
          <ellipse cx={X(0)} cy={Y(headCy)} rx={rx * k} ry={ry * k} className="hf-head" />
          <path d={`M${X(-9)},${Y(headCy + ry - 3)} L${X(0)},${Y(headCy + ry + 12)} L${X(9)},${Y(headCy + ry - 3)}`} className="hf-head" />
          {/* front */}
          <line x1={X(-g.total_width_mm / 2)} x2={X(g.total_width_mm / 2)} y1={Y(-g.front_thickness_mm / 2)} y2={Y(-g.front_thickness_mm / 2)} className="hf-frame" style={{ strokeWidth: Math.max(3, g.front_thickness_mm * k) }} />
          <polyline points={temple(1)} className="hf-temple" />
          <polyline points={temple(-1)} className="hf-temple" />
          {/* rest position (dashed) */}
          {[1, -1].map((s) => (
            <line key={s} x1={X(s * g.temple_inner_x)} y1={Y(g.pin_y)} x2={X(s * (g.temple_inner_x + lenTotal * Math.tan(splay)))} y2={Y(g.pin_y - lenTotal)} className="hf-ghost" />
          ))}
          {/* contact arrows: head pushes the temples outwards */}
          {[1, -1].map((s) => (
            <g key={`a${s}`}>
              <line x1={X(s * H / 2)} y1={Y(yC)} x2={X(s * (H / 2 + aLen / k))} y2={Y(yC)} stroke={col} strokeWidth={3} strokeLinecap="round" />
              <path d={`M${X(s * (H / 2 + aLen / k)) + s * 8},${Y(yC)} l${-s * 10},-6 l0,12 Z`} fill={col} />
            </g>
          ))}
          <text x={X(0)} y={Y(yC) + 4} textAnchor="middle" className="hf-big" style={{ fill: col }}>{F.toFixed(2)} N</text>
          <text x={X(0)} y={Y(yC) + 22} textAnchor="middle" className="hf-lbl">{verdict}</text>
          <text x={8} y={14} className="hf-lbl">head breadth P{Math.round(pct)} = {H.toFixed(0)} mm</text>
          <text x={8} y={29} className="hf-lbl">rest spread {m.s0.toFixed(0)} mm · splay {cfg.temple_splay_deg.toFixed(1)}°</text>
          <text x={X(0)} y={Y(14) - 6} textAnchor="middle" className="hf-lbl">front (bows {(theta * 180 / Math.PI).toFixed(1)}° per hinge){m.hinge ? ` · spring hinge open ${alpha.toFixed(1)}°` : ''}</text>
        </svg>
      </div>
      <div className="hf-legend">
        <span><i style={{ background: 'var(--good)' }} /> comfortable ✓</span>
        <span><i style={{ background: 'var(--bad)' }} /> too tight ✕</span>
        <span><i style={{ background: 'var(--series-4)' }} /> too loose !</span>
        <span className="muted">dashed: temples at rest</span>
      </div>
    </Card>
  );
}

// ---------------------------------------------------------------- compliance breakdown

function ComplianceCard({ m }: { m: ClampModel }) {
  const parts: [string, number, string][] = [
    ['Front bending (both hinges)', m.c_front, 'var(--series-1)'],
    ['Right temple (electronics pod)', m.c_right, 'var(--series-3)'],
    ['Left temple (battery pod)', m.c_left, 'var(--series-4)'],
  ];
  const rows: [string, Segment][] = [
    ...m.front.map((s) => ['front', s] as [string, Segment]),
    ...m.right.map((s) => ['right temple', s] as [string, Segment]),
    ...m.left.map((s) => ['left temple', s] as [string, Segment]),
  ];
  return (
    <Card title="Where the springiness lives" subtitle={`Total compliance ${f1(m.c_total, 2)} mm/N: 1 N of clamp needs ${f1(m.c_total, 1)} mm more head breadth.`}>
      <div className="hf-stack-bar" role="img" aria-label="Compliance split">
        {parts.map(([n, v, col]) => (
          <div key={n} style={{ width: `${(100 * v) / m.c_total}%`, background: col }} title={`${n}: ${v.toFixed(2)} mm/N`} />
        ))}
      </div>
      <div className="hf-legend">
        {parts.map(([n, v, col]) => (
          <span key={n}><i style={{ background: col }} /> {n}: <b className="tnum">{f1(v, 2)} mm/N ({f1((100 * v) / m.c_total, 0)} %)</b></span>
        ))}
      </div>
      <div className="table-wrap">
        <table className="table">
          <thead><tr><th>Beam</th><th>Segment</th><th>From–to (mm)</th><th>I (mm⁴)</th><th>EI (N·mm²)</th></tr></thead>
          <tbody>
            {rows.map(([b, s], i) => (
              <tr key={i}>
                <td>{b}</td>
                <td>{s.name}</td>
                <td className="tnum">{f1(s.s0)}–{s.s1 > 1e8 ? 'contact' : f1(s.s1)}</td>
                <td className="tnum">{f1(s.i_mm4, 1)}</td>
                <td className="tnum">{s.ei.toExponential(2)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Card>
  );
}
