// Iris hinge (HW1): a compression-spring barrel ("flex") hinge whose temple
// end face is the detent cam. Torque vs opening angle, over-travel, clamp
// on the head, spring stress, Goodman safety factor and life for a stated
// usage — all from design.extra.mechanisms.hinge, which this tab edits.
import { useMemo } from 'react';
import { Link } from 'react-router-dom';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat, Toggle } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { HingeOut, HingeParams } from './types';
import { Checks, DEG, Kv, Notes, PlayBar, clamp, csvFrom, fmt, fmtCycles, sfTone, springPath, useClock } from './util';

export const HINGE_DEFAULTS: HingeParams = {
  material: 'stainless_302', wire_d_mm: 0.45, mean_d_mm: 2.0, active_coils: 6, ends: 'squared_ground', free_len_mm: 5.5, preload_mm: 0.6, peened: false,
  stop_offset_mm: null, inner_mm: null, outer_mm: null, inner_corner_r_mm: 1.1, outer_corner_r_mm: 0.3, max_overtravel_deg: 20, head_width_mm: 146,
  friction: 0.15, pin_d_mm: 1.4, uses_per_day: 12, wide_pulls_per_day: 2, design_years: 3,
};

/** Read / write design.extra.mechanisms.hinge. */
function useHingeParams(): [HingeParams, (q: Partial<HingeParams>) => void, () => void] {
  const project = useProject((s) => s.project);
  const patchDesign = useProject((s) => s.patchDesign);
  const extra = project.design.extra ?? {};
  const mech = (extra.mechanisms as Record<string, unknown> | undefined) ?? {};
  const stored = (mech.hinge as Partial<HingeParams> | undefined) ?? {};
  const params = { ...HINGE_DEFAULTS, ...stored };
  const set = (q: Partial<HingeParams>) => patchDesign('extra', { ...extra, mechanisms: { ...mech, hinge: { ...stored, ...q } } });
  const reset = () => patchDesign('extra', { ...extra, mechanisms: { ...mech, hinge: { ...HINGE_DEFAULTS } } });
  return [params, set, reset];
}

// Animation timeline (8 s): open → put on → hold → pulled wide → back → fold.
function phiAt(t: number, wear: number, max: number): number {
  const ease = (x: number) => 0.5 - 0.5 * Math.cos(Math.PI * clamp(x, 0, 1));
  if (t < 2) return 90 * ease(t / 2);
  if (t < 3.2) return 90 + wear * ease((t - 2) / 1.2);
  if (t < 4.2) return 90 + wear;
  if (t < 5.2) return 90 + wear + (max - wear) * ease(t - 4.2);
  if (t < 6.0) return 90 + max - max * ease((t - 5.2) / 0.8);
  return 90 - 90 * ease((t - 6) / 2);
}

export function HingeTab() {
  const project = useProject((s) => s.project);
  const [h, set, reset] = useHingeParams();
  const { data: o, error } = useCompute<HingeOut>('mechanisms.hinge', { project, params: h });
  const clock = useClock(8, { loop: true });
  const wear = o?.wear.overtravel_deg ?? 5;
  const phi = phiAt(clock.t, wear, h.max_overtravel_deg);
  const k = o ? clamp(Math.round(phi / 0.25), 0, o.phi_deg.length - 1) : 0;
  const csv = () =>
    o ? csvFrom(['phi_deg', 'slider_travel_mm', 'spring_force_n', 'spring_torque_nmm', 'open_torque_nmm', 'close_torque_nmm', 'energy_nmm'], [o.phi_deg, o.travel_mm, o.force_n, o.torque_nmm, o.open_torque_nmm, o.close_torque_nmm, o.energy_nmm]) : '';
  const json = () => JSON.stringify({ hinge: h, result: o && { ...o, phi_deg: undefined, travel_mm: undefined, force_n: undefined, torque_nmm: undefined, open_torque_nmm: undefined, close_torque_nmm: undefined, energy_nmm: undefined } }, null, 2);
  const stage = clock.t < 2 ? 'opening over the detent' : clock.t < 4.2 ? 'on the head (over-travel)' : clock.t < 6 ? 'pulled wide to the stop' : 'folding — snaps shut';
  return (
    <div className="mx-split">
      <div className="stack" style={{ minWidth: 0 }}>
        <Card
          title="Iris spring hinge (HW1) — top view"
          subtitle="The temple's end face is the detent cam: a preloaded spring in the temple's barrel pulls the end face against the endpiece's stop"
          actions={o && (<><DownloadButton small filename="iris-hinge-torque.csv" label="torque CSV" content={csv} /><DownloadButton small filename="iris-hinge.json" label="design JSON" content={json} /></>)}
        >
          <ErrorBox error={error} />
          {o && (
            <>
              <div className="mx-hinge-grid">
                <HingeView o={o} phi={phi} />
                <div className="stack" style={{ gap: 10 }}>
                  <Kv
                    rows={[
                      ['Opening angle φ', `${fmt(phi, 1)}°`],
                      ['Slider travel δ', `${fmt(o.travel_mm[k], 3)} mm`],
                      ['Spring force', `${fmt(o.force_n[k], 2)} N`],
                      ['Spring torque dE/dφ', `${fmt(o.torque_nmm[k], 2)} N·mm`],
                      ['Shear stress τ', `${fmt(o.force_n[k] * o.spring.tau_per_n, 0)} MPa`],
                      ['Now', stage],
                    ]}
                  />
                  <div className="row">
                    <Chip>contact from {fmt(o.contact_start_deg, 0)}°</Chip>
                    <Chip>detent hump at {fmt(o.hump_deg, 0)}°</Chip>
                    <Chip>stop at {fmt(90 + h.max_overtravel_deg, 0)}°</Chip>
                  </div>
                </div>
              </div>
              <PlayBar clock={clock} label={() => `φ ${fmt(phi, 0)}°`} />
              <div className="grid grid-4" style={{ marginTop: 12 }}>
                <Stat label="Torque to open past detent" value={fmt(o.detent_open_torque_nmm, 2)} unit="N·mm" note={`closing needs ${fmt(o.detent_close_torque_nmm, 2)} N·mm`} />
                <Stat label="Spring fatigue SF (Goodman)" value={fmt(o.nf_min, 2)} tone={sfTone(o.nf_min, 1.5)} note={o.nf_min >= 1.5 ? '✓ ≥ 1.5' : '! below 1.5 — see fixes'} />
                <Stat label="Life (open/close uses)" value={o.runout ? '≥ 10⁷' : fmtCycles(o.life_uses)} tone={o.life_uses >= o.design_uses ? 'pass' : 'fail'} note={`${o.runout ? 'runout · ' : ''}need ${fmtCycles(o.design_uses)} (${h.uses_per_day}/day × ${h.design_years} y)`} />
                <Stat label="Clamp on the head" value={fmt(o.wear.clamp_n, 2)} unit="N/side" note={`rigid hinge: ${fmt(o.wear.clamp_rigid_n, 1)} N`} />
              </div>
            </>
          )}
        </Card>
        {o && (
          <div className="grid grid-2">
            <Card title="Torque vs opening angle" subtitle="Positive = resists opening. User torques include pin friction.">
              <Plot
                series={[
                  { name: 'spring dE/dφ', x: o.phi_deg, y: o.torque_nmm },
                  { name: 'to open', x: o.phi_deg, y: o.open_torque_nmm, dash: '5 3' },
                  { name: 'to close', x: o.phi_deg, y: o.close_torque_nmm.map((v) => -v), dash: '2 3' },
                ]}
                xLabel="opening angle φ, deg"
                yLabel="N·mm"
                legend
                vLines={[{ value: phi, label: `${fmt(phi, 0)}°` }, { value: 90, color: 'var(--grid)' }]}
                hLines={[{ value: 0, color: 'var(--axis)' }]}
                height={240}
              />
            </Card>
            <Card title="Slider travel and spring force" subtitle="The end-face cam: δ(φ) = max(0, y_corner + r − a) / sin φ">
              <Plot
                series={[{ name: 'δ, mm', x: o.phi_deg, y: o.travel_mm }, { name: 'F / 10, N', x: o.phi_deg, y: o.force_n.map((f) => f / 10) }]}
                xLabel="opening angle φ, deg"
                legend
                vLines={[{ value: phi }]}
                height={240}
              />
            </Card>
          </div>
        )}
        {o && <HeadCard o={o} h={h} />}
        {o && <FatigueCard o={o} h={h} />}
      </div>
      <div className="stack mx-side">
        <Card title="Spring (saved to the Iris project)" actions={<button className="btn small" onClick={reset}>Reset</button>}>
          <Select label="Wire" value={h.material} options={[{ value: 'stainless_302', label: 'Stainless 302 (A313)' }, { value: 'music_wire', label: 'Music wire (A228)' }, { value: 'hard_drawn', label: 'Hard-drawn (A227)' }]} onChange={(material) => set({ material })} />
          <Slider label="Wire diameter d" value={h.wire_d_mm} min={0.2} max={0.8} step={0.01} unit="mm" onChange={(wire_d_mm) => set({ wire_d_mm })} />
          <Slider label="Mean coil diameter D" value={h.mean_d_mm} min={0.8} max={3.5} step={0.05} unit="mm" onChange={(mean_d_mm) => set({ mean_d_mm })} />
          <Slider label="Active coils" value={h.active_coils} min={2} max={14} step={0.5} onChange={(active_coils) => set({ active_coils })} />
          <Slider label="Free length" value={h.free_len_mm} min={2} max={12} step={0.1} unit="mm" onChange={(free_len_mm) => set({ free_len_mm })} />
          <Slider label="Preload compression" value={h.preload_mm} min={0} max={2} step={0.05} unit="mm" onChange={(preload_mm) => set({ preload_mm })} />
          <Toggle label="Shot peened wire" value={h.peened} onChange={(peened) => set({ peened })} />
        </Card>
        <Card title="Detent geometry">
          <Slider label="Inner corner radius" value={h.inner_corner_r_mm} min={0} max={2} step={0.05} unit="mm" onChange={(inner_corner_r_mm) => set({ inner_corner_r_mm })} />
          <Slider label="Outer corner radius" value={h.outer_corner_r_mm} min={0} max={1.5} step={0.05} unit="mm" onChange={(outer_corner_r_mm) => set({ outer_corner_r_mm })} />
          <Slider label="Over-travel stop" value={h.max_overtravel_deg} min={0} max={35} step={0.5} unit="° past 90°" onChange={(max_overtravel_deg) => set({ max_overtravel_deg })} />
          <Slider label="Pin to stop face a" value={h.stop_offset_mm ?? o?.geom.stop_offset_mm ?? 2.7} min={1} max={5} step={0.05} unit="mm" onChange={(stop_offset_mm) => set({ stop_offset_mm })} />
          <Slider label="Pin to inner face" value={h.inner_mm ?? o?.geom.inner_mm ?? 2} min={0.5} max={4} step={0.05} unit="mm" onChange={(inner_mm) => set({ inner_mm })} />
          <Slider label="Pin to outer face" value={h.outer_mm ?? o?.geom.outer_mm ?? 2} min={0.5} max={4} step={0.05} unit="mm" onChange={(outer_mm) => set({ outer_mm })} />
          <Slider label="Pin friction μ" value={h.friction} min={0} max={0.5} step={0.01} onChange={(friction) => set({ friction })} />
          <p className="small muted">a and the face offsets default to the CAD knuckles (KNUCKLE_LEN/2 + gap/2) and half the temple width.</p>
        </Card>
        <Card title="Usage and the wearer">
          <Slider label="Uses per day (open, wear, close)" value={h.uses_per_day} min={1} max={60} step={1} onChange={(uses_per_day) => set({ uses_per_day })} />
          <Slider label="Pulls to the stop per day" value={h.wide_pulls_per_day} min={0} max={20} step={0.5} onChange={(wide_pulls_per_day) => set({ wide_pulls_per_day })} />
          <Slider label="Design life" value={h.design_years} min={0.5} max={10} step={0.5} unit="years" onChange={(design_years) => set({ design_years })} />
          <Slider label="Head width at the ear" value={h.head_width_mm} min={120} max={170} step={1} unit="mm" onChange={(head_width_mm) => set({ head_width_mm })} />
          <p className="small muted">Adult head breadth is roughly 14–16 cm; see anthropometric surveys (for example ANSUR II) or the Ergonomics Lab.</p>
        </Card>
        <Explainer
          title="How the hinge is modelled"
          refs={[{ label: 'Shigley ch. 10 (springs)' }, { label: 'A. M. Wahl, "Mechanical Springs"' }, { label: 'ISO 12870 — spectacle frames, requirements and test methods (includes hinge endurance testing; check the current edition)' }]}
        >
          <Formula>{`corner: y = (a − r) sin φ ∓ (t − r) cos φ
travel: δ = max(0, y + r − a) / sin φ
energy: E = ½ k [(δ₀ + δ)² − δ₀²]
torque: T = dE/dφ = k (δ₀ + δ) dδ/dφ  (+ μ F r_pin)
head:   L_c sin α + F c_total/2 = Δ,   F L_c = T(90° + α)`}</Formula>
          <p className="small">
            Snap fits, screws and living hinges are in the <Link to="/structures">Structures Lab</Link> (Calculators) — the Structures Lab also runs the temple's own plastic hinge-area fatigue.
          </p>
        </Explainer>
      </div>
    </div>
  );
}

// ------------------------------------------------------------------ top view

function HingeView({ o, phi }: { o: HingeOut; phi: number }) {
  const g = o.geom;
  const p = o.params;
  const a = g.stop_offset_mm;
  const ti = g.inner_mm;
  const to = g.outer_mm;
  const ri = clamp(p.inner_corner_r_mm, 0, Math.min(ti, a));
  const ro = clamp(p.outer_corner_r_mm, 0, Math.min(to, a));
  const f = Math.max(0.05, phi) * DEG;
  const k = clamp(Math.round(phi / 0.25), 0, o.travel_mm.length - 1);
  const delta = o.travel_mm[k];
  const u = [-Math.cos(f), -Math.sin(f)];
  const n = [Math.sin(f), -Math.cos(f)];
  // world (x right, y forward) → SVG (x, −y)
  const W = (s: number, t: number): [number, number] => [(s + delta) * u[0] + t * n[0], -((s + delta) * u[1] + t * n[1])];
  const P = (q: [number, number]) => `${q[0].toFixed(3)},${q[1].toFixed(3)}`;
  const Lv = 19;
  const arcPts = (cs: number, ct: number, r: number, a0: number, a1: number) => {
    const out: string[] = [];
    for (let q = 0; q <= 8; q++) {
      const t = a0 + ((a1 - a0) * q) / 8;
      out.push(P(W(cs + r * Math.cos(t), ct + r * Math.sin(t))));
    }
    return out;
  };
  const outline = [
    P(W(Lv, to)),
    ...arcPts(-a + ro, to - ro, ro, Math.PI / 2, Math.PI),
    ...arcPts(-a + ri, -ti + ri, ri, Math.PI, 1.5 * Math.PI),
    P(W(Lv, -ti)),
  ];
  // barrel and spring in temple coordinates
  const sw = 1.6;
  const Linst = o.spring.installed_len_mm;
  const head = sw + Linst - delta;
  const rs = o.spring.od_mm / 2;
  const barrel = [W(sw, -rs - 0.1), W(sw + Linst + 1.2, -rs - 0.1), W(sw + Linst + 1.2, rs + 0.1), W(sw, rs + 0.1)];
  const sp0 = W(sw, 0);
  const sp1 = W(head, 0);
  const pin = W(-delta, 0);
  const headA = W(head, -rs);
  const headB = W(head, rs);
  // endpiece (fixed): stop face at y = a
  const ep = `M${-(ti + 13)},${-a} L${to + 1.2},${-a} L${to + 1.2},${-(a + 5)} L${-(ti + 13)},${-(a + 5)} Z`;
  const vb = [-21, -(a + 6), 32, 27];
  return (
    <svg className="mx-canvas" viewBox={vb.join(' ')} role="img" aria-label="Animated top view of the spring hinge">
      <path d={ep} className="mx-front" strokeWidth={0.15} />
      <text x={-(ti + 12.4)} y={-(a + 2)} fontSize={1.2} className="mx-svgtext">front endpiece · stop face ↓</text>
      <path d={`M${outline.join(' L')} Z`} className="mx-temple" strokeWidth={0.15} />
      <polygon points={barrel.map(P).join(' ')} className="mx-barrel" strokeWidth={0.1} />
      <path d={springPath(sp0[0], sp0[1], sp1[0], sp1[1], o.params.active_coils, rs * 0.85)} className="mx-spring" strokeWidth={0.18} fill="none" />
      <line x1={pin[0]} y1={pin[1]} x2={sp1[0]} y2={sp1[1]} className="mx-rod" strokeWidth={0.35} />
      <line x1={headA[0]} y1={headA[1]} x2={headB[0]} y2={headB[1]} className="mx-rod" strokeWidth={0.5} />
      {/* knuckle tab from the endpiece to the pin */}
      <rect x={-0.9} y={-a} width={1.8} height={a} className="mx-knuckle" strokeWidth={0.1} />
      <circle cx={0} cy={0} r={1.05} className="mx-knuckle" strokeWidth={0.12} />
      <circle cx={0} cy={0} r={o.params.pin_d_mm / 2} className="mx-pin" />
      <text x={1.4} y={1.9} fontSize={1.2} className="mx-svgtext">pin</text>
      <text x={W(Lv - 4, 0)[0]} y={W(Lv - 4, 0)[1]} fontSize={1.3} className="mx-svgtext" textAnchor="middle">temple</text>
      <text x={vb[0] + 0.8} y={vb[1] + vb[3] - 0.8} fontSize={1.3} className="mx-svgtext">δ = {fmt(delta, 2)} mm · φ = {fmt(phi, 0)}°</text>
    </svg>
  );
}

// ------------------------------------------------------------------ on the head

function HeadCard({ o, h }: { o: HingeOut; h: HingeParams }) {
  const g = o.geom;
  const al = o.wear.overtravel_deg * DEG;
  const W = 260, H = 250;
  const sc = 1.25;
  const cx = W / 2;
  const fy = 30;
  const pinX = g.pin_x_mm * sc;
  const lc = g.contact_len_mm * sc;
  const temples = [1, -1].map((side) => {
    const x0 = cx + side * pinX;
    const x1 = x0 + side * lc * Math.sin(al);
    const y1 = fy + lc * Math.cos(al);
    return { x0, x1, y1, side };
  });
  const headW = (h.head_width_mm / 2) * sc;
  return (
    <Card title="On the head" subtitle="The spread needed at the ear is shared by the hinge's over-travel and the frame's bending (temple + front, the Human Factors Lab's model)">
      <div className="grid grid-2">
        <svg className="mx-diagram" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Top view of Iris on a head with the temples spread">
          <ellipse cx={cx} cy={fy + lc * 0.66} rx={headW} ry={lc * 0.6} className="mx-head" />
          <line x1={cx - pinX - 3} y1={fy} x2={cx + pinX + 3} y2={fy} className="mx-frontline" />
          {temples.map((t) => (
            <g key={t.side}>
              <line x1={t.x0} y1={fy} x2={t.x0} y2={fy + lc} className="mx-ghost" />
              <line x1={t.x0} y1={fy} x2={t.x1} y2={t.y1} className="mx-templeline" />
              <line x1={t.x1 + t.side * 2} y1={t.y1} x2={t.x1 + t.side * 18} y2={t.y1} className="mx-force" markerEnd="url(#mx-arrow2)" />
            </g>
          ))}
          <defs>
            <marker id="mx-arrow2" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
              <path d="M0,0 L10,5 L0,10 z" fill="var(--series-2)" />
            </marker>
          </defs>
          <text x={8} y={H - 22}>head {h.head_width_mm} mm · temples {fmt(2 * g.rest_half_spread_mm, 0)} mm apart</text>
          <text x={8} y={H - 8}>clamp {fmt(o.wear.clamp_n, 2)} N/side · α = {fmt(o.wear.overtravel_deg, 1)}°</text>
        </svg>
        <div>
          <Kv
            rows={[
              ['Spread needed per side Δ', `${fmt(o.wear.spread_mm, 2)} mm`],
              ['Hinge over-travel α', `${fmt(o.wear.overtravel_deg, 2)}° ${o.wear.hits_stop ? '(at the stop!)' : ''}`],
              ['Frame bending (temple + front)', `${fmt(o.wear.arm_bend_mm, 3)} mm`],
              ['Share taken by the hinge', `${fmt(o.wear.hinge_share_pct, 1)} %`],
              ['Clamp force per side', `${fmt(o.wear.clamp_n, 3)} N`],
              ['…if the hinge were rigid', `${fmt(o.wear.clamp_rigid_n, 2)} N`],
              ['Frame stiffness at the contact', `${fmt(g.arm_stiffness_n_mm, 3)} N/mm`],
              ['…temple alone (beam)', `${fmt(g.temple_stiffness_n_mm, 2)} N/mm`],
              ['Over-travel starts at', `${fmt(o.overtravel_start_torque_nmm, 2)} N·mm (+${fmt(o.overtravel_rate_nmm_deg, 3)} N·mm/°)`],
            ]}
          />
          <p className="small muted" style={{ marginTop: 8 }}>
            The spring hinge makes the fit forgiving: almost any head width is taken up by over-travel at a nearly constant torque, so it contributes little grip. Grip comes from
            the temples' inward bend (set by the optician), the ear bend and the nose pads.
          </p>
        </div>
      </div>
    </Card>
  );
}

// ------------------------------------------------------------------ fatigue

function FatigueCard({ o, h }: { o: HingeOut; h: HingeParams }) {
  const s = o.spring;
  const pts = useMemo(() => o.blocks.filter((b) => b.per_day > 0 && b.travel_mm > 1e-9), [o]);
  const Wd = 360, Hd = 230, pad = 42;
  const mx = s.ssu_mpa * 1.02;
  const my = Math.max(s.sse_mpa, ...pts.map((b) => b.tau_a_mpa)) * 1.25;
  const X = (v: number) => pad + (v / mx) * (Wd - pad - 12);
  const Y = (v: number) => Hd - pad + 8 - (v / my) * (Hd - pad - 18);
  return (
    <Card title="Spring stress and life" subtitle={`Usage: ${h.uses_per_day} uses/day + ${h.wide_pulls_per_day} pulls to the stop/day for ${h.design_years} years = ${fmtCycles(o.design_uses)} uses`}>
      <div className="grid grid-2">
        <div>
          <div className="table-wrap">
            <table className="table">
              <thead><tr><th>Block</th><th>Per day</th><th>τ min–max</th><th>SF</th><th>Cycles to fail</th></tr></thead>
              <tbody>
                {o.blocks.map((b, i) => (
                  <tr key={i}>
                    <td style={{ minWidth: 150 }}>{i + 1}. {b.name}</td>
                    <td className="tnum">{fmt(b.per_day, 0)}</td>
                    <td className="tnum">{fmt(b.tau_min_mpa, 0)}–{fmt(b.tau_max_mpa, 0)} MPa</td>
                    <td className="tnum">{b.travel_mm > 1e-9 ? fmt(b.nf, 2) : '—'}</td>
                    <td className="tnum">{b.travel_mm > 1e-9 ? (b.runout ? '∞' : fmtCycles(b.cycles_to_failure)) : '—'}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <Kv
            rows={[
              ['Wire', s.material],
              ['S_ut, S_sy, S_su', `${fmt(s.sut_mpa, 0)}, ${fmt(s.ssy_mpa, 0)}, ${fmt(s.ssu_mpa, 0)} MPa`],
              ['Rate k, preload', `${fmt(s.rate_n_mm, 2)} N/mm, ${fmt(s.preload_n, 2)} N`],
              ['C, K_B', `${fmt(s.index_c, 2)}, ${fmt(s.k_b, 3)}`],
              ['Peak τ (at the stop)', `${fmt(o.max_tau_mpa, 0)} MPa → n_s ${fmt(o.ns_static, 2)}`],
              ['Miner damage per day', o.damage_per_day > 0 ? o.damage_per_day.toExponential(2) : '0 (all runout)'],
            ]}
          />
        </div>
        <div>
          <svg className="mx-diagram" viewBox={`0 0 ${Wd} ${Hd}`} role="img" aria-label="Goodman diagram with the hinge usage blocks">
            <line className="mx-axis" x1={pad} y1={Y(0)} x2={Wd - 8} y2={Y(0)} />
            <line className="mx-axis" x1={pad} y1={Y(0)} x2={pad} y2={8} />
            <polygon points={`${X(0)},${Y(0)} ${X(0)},${Y(s.sse_mpa)} ${X(s.ssu_mpa)},${Y(0)}`} className="mx-safe" />
            <line x1={X(0)} y1={Y(s.sse_mpa)} x2={X(s.ssu_mpa)} y2={Y(0)} stroke="var(--series-1)" strokeWidth={2} />
            <line x1={X(0)} y1={Y(s.ssy_mpa)} x2={X(s.ssy_mpa)} y2={Y(0)} stroke="var(--series-4)" strokeWidth={1.4} strokeDasharray="5 3" />
            {pts.map((b, i) => (
              <g key={i}>
                <circle cx={X(b.tau_m_mpa)} cy={Y(b.tau_a_mpa)} r={5} fill={`var(--series-${i + 2})`} stroke="var(--surface-1)" />
                <text x={X(b.tau_m_mpa) + 7} y={Y(b.tau_a_mpa) + 4}>{o.blocks.indexOf(b) + 1}</text>
              </g>
            ))}
            <text x={X(0) + 4} y={Y(s.sse_mpa) - 5}>S_se = {fmt(s.sse_mpa, 0)} MPa (Zimmerli{h.peened ? ', peened' : ''})</text>
            <text x={X(s.ssu_mpa) - 2} y={Y(0) + 16} textAnchor="end">S_su = {fmt(s.ssu_mpa, 0)}</text>
            <text x={Wd - 10} y={Hd - 6} textAnchor="end">τ_m, MPa →</text>
            <text x={pad + 4} y={16}>τ_a, MPa</text>
          </svg>
          <div className="small muted">Blue line: Goodman (infinite life). Dashed: yield, τ_a + τ_m = S_sy. Preload makes the mean stress high, so every cycle sits far to the right.</div>
        </div>
      </div>
      <div style={{ marginTop: 12 }}>
        <Checks checks={o.checks} />
        <Notes notes={o.notes} />
        {o.nf_min < 1.5 && (
          <div className="callout" style={{ borderLeftColor: 'var(--warn)', marginTop: 8 }}>
            <h4>Ways to raise the safety factor</h4>
            Shot-peen the wire (Zimmerli's endurance rises from 241 to 398 MPa), lower the over-travel stop, reduce the preload, or use a larger coil index (longer, softer spring) — each
            trades against detent feel or space in the 4 mm temple.
          </div>
        )}
      </div>
    </Card>
  );
}
