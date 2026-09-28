// Design calculators: snap fits, fasteners (bolted joints, thread stripping,
// bosses and inserts), beams and buckling, fits (ISO 286 and press fits),
// thermal expansion mismatch, stress concentrations and living hinges — each
// with a to-scale or schematic SVG diagram and the formula it uses.
import { useId, useMemo, useState, type ReactNode } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { BeamOut, BoltOut, BossOut, BuckleOut, FitOut, HingeOut, KtOut, PressOut, Section, SnapOut, StructMaterial, ThermalOut } from './types';
import { fmt, prefersReducedMotion, sfTone, sfWord, toCsv, useRaf } from './util';

type CalcId = 'snap' | 'bolt' | 'boss' | 'beam' | 'buckle' | 'fits' | 'press' | 'thermal' | 'kt' | 'hinge';

const CALCS: { id: CalcId; label: string; iris: string }[] = [
  { id: 'snap', label: 'Snap fit', iris: 'pod lid clips' },
  { id: 'bolt', label: 'Bolted joint', iris: 'hinge screw' },
  { id: 'boss', label: 'Screw boss & inserts', iris: 'lid screws' },
  { id: 'beam', label: 'Beam bending', iris: 'temple arm' },
  { id: 'buckle', label: 'Buckling', iris: 'pod ribs' },
  { id: 'fits', label: 'ISO 286 fits', iris: 'hinge pin' },
  { id: 'press', label: 'Press fit', iris: 'pin in knuckle' },
  { id: 'thermal', label: 'Thermal mismatch', iris: 'PCB in pod' },
  { id: 'kt', label: 'Stress concentration', iris: 'holes & notches' },
  { id: 'hinge', label: 'Living hinge', iris: 'cable flap' },
];

export function CalculatorsTab() {
  const [calc, setCalc] = useState<CalcId>('snap');
  const mats = useCompute<StructMaterial[]>('structures.materials', {}).data ?? [];
  const matOpts = mats.map((m) => ({ value: m.id, label: m.name }));
  return (
    <div className="stack">
      <div className="st-presets" role="tablist" aria-label="Calculator">
        {CALCS.map((c) => (
          <button key={c.id} role="tab" aria-selected={calc === c.id} className={`st-preset ${calc === c.id ? 'active' : ''}`} onClick={() => setCalc(c.id)}>
            <b>{c.label}</b>
            <span className="small muted">Iris: {c.iris}</span>
          </button>
        ))}
      </div>
      {calc === 'snap' && <SnapCalc matOpts={matOpts} />}
      {calc === 'bolt' && <BoltCalc matOpts={matOpts} />}
      {calc === 'boss' && <BossCalc matOpts={matOpts} />}
      {calc === 'beam' && <BeamCalc matOpts={matOpts} />}
      {calc === 'buckle' && <BuckleCalc matOpts={matOpts} />}
      {calc === 'fits' && <FitsCalc />}
      {calc === 'press' && <PressCalc matOpts={matOpts} />}
      {calc === 'thermal' && <ThermalCalc matOpts={matOpts} />}
      {calc === 'kt' && <KtCalc matOpts={matOpts} />}
      {calc === 'hinge' && <HingeCalc matOpts={matOpts} />}
    </div>
  );
}

type MatOpts = { value: string; label: string }[];

/** Inputs on the left, diagram + results on the right. */
function Shell(props: { inputs: ReactNode; title: string; subtitle?: string; diagram: ReactNode; results: ReactNode; extra?: ReactNode; error?: string | null }) {
  return (
    <div className="st-calc">
      <Card title="Inputs">{props.inputs}</Card>
      <div className="stack" style={{ minWidth: 0 }}>
        <Card title={props.title} subtitle={props.subtitle}>
          <ErrorBox error={props.error ?? null} />
          {props.diagram}
          <div style={{ marginTop: 12 }}>{props.results}</div>
        </Card>
        {props.extra}
      </div>
    </div>
  );
}

function Verdict({ tone, children }: { tone: 'pass' | 'warn' | 'fail'; children: ReactNode }) {
  const icon = tone === 'pass' ? '✓' : tone === 'warn' ? '!' : '✕';
  return (
    <div className={`st-verdict ${tone}`}>
      <b>{icon}</b> {children}
    </div>
  );
}

function Hatch({ x, y, w, h }: { x: number; y: number; w: number; h: number }) {
  const id = `st-hatch-${useId().replace(/[^a-zA-Z0-9_-]/g, '')}`;
  const lines = [];
  for (let k = -h; k < w; k += 7) lines.push(<line key={k} className="hatch" x1={x + k} y1={y + h} x2={x + k + h} y2={y} />);
  return (
    <g>
      <defs>
        <clipPath id={id}>
          <rect x={x} y={y} width={w} height={h} />
        </clipPath>
      </defs>
      <rect x={x} y={y} width={w} height={h} fill="var(--surface-3)" stroke="var(--text-2)" />
      <g clipPath={`url(#${id})`}>{lines}</g>
    </g>
  );
}

function Arrow({ x1, y1, x2, y2 }: { x1: number; y1: number; x2: number; y2: number }) {
  const a = Math.atan2(y2 - y1, x2 - x1);
  const h = 7;
  return (
    <g className="load">
      <line x1={x1} y1={y1} x2={x2} y2={y2} />
      <polygon points={`${x2},${y2} ${x2 - h * Math.cos(a - 0.4)},${y2 - h * Math.sin(a - 0.4)} ${x2 - h * Math.cos(a + 0.4)},${y2 - h * Math.sin(a + 0.4)}`} stroke="none" />
    </g>
  );
}

function usePlay(period: number): [number, boolean, (b: boolean) => void] {
  const [t, setT] = useState(prefersReducedMotion() ? 0.55 : 0);
  const [playing, setPlaying] = useState(!prefersReducedMotion());
  useRaf(playing, (dt) => setT((v) => (v + dt / period) % 1));
  return [t, playing, setPlaying];
}

function PlayButton({ playing, setPlaying }: { playing: boolean; setPlaying: (b: boolean) => void }) {
  return (
    <button className="btn small" onClick={() => setPlaying(!playing)} aria-pressed={playing}>
      {playing ? '❚❚ Pause' : '▶ Animate'}
    </button>
  );
}

// ------------------------------------------------------------------ snap fit

function SnapCalc({ matOpts }: { matOpts: MatOpts }) {
  const [p, setP] = useState({ length_mm: 8, thickness_mm: 1.2, width_mm: 4, undercut_mm: 0.6, profile: 'constant', lead_deg: 30, return_deg: 45, friction: 0.3, material: 'PETG' });
  const set = (q: Partial<typeof p>) => setP((o) => ({ ...o, ...q }));
  const { data: r, error } = useCompute<SnapOut>('structures.snap_fit', p);
  const [t, playing, setPlaying] = usePlay(2.6);
  // 0–0.55: the mating part slides in and pushes the hook up; 0.55–0.62 it snaps back; then hold.
  const W = 380;
  const H = 200;
  const k = 230 / (p.length_mm + 2.5 * p.undercut_mm + 1);
  const x0 = 60;
  const yc = 80;
  const tpx = p.thickness_mm * k;
  const ypx = p.undercut_mm * k;
  const L = p.length_mm * k;
  const shape = (x: number) => (x * x * (3 * L - x)) / (2 * L * L * L);
  const thick = (x: number) => (p.profile === 'tapered' ? tpx * (1 - 0.5 * (x / L)) : tpx);
  // The latch ledge (a short block) slides left under the hook: it rides up the
  // lead-in ramp, holds the hook fully deflected, then the hook drops behind it.
  const leadLen = ypx / Math.tan(Math.max(0.2, (p.lead_deg * Math.PI) / 180));
  const backLen = ypx / Math.tan(Math.max(0.2, (p.return_deg * Math.PI) / 180));
  const ledge = 34;
  const xStart = x0 + L + leadLen + 40;
  const xEnd = x0 + L - backLen - ledge - 30;
  const mateX = xStart + (xEnd - xStart) * Math.min(1, t / 0.75);
  const push = mateX > x0 + L + leadLen ? 0 : mateX > x0 + L ? (x0 + L + leadLen - mateX) / leadLen : mateX + ledge > x0 + L - backLen ? 1 : 0;
  const latched = mateX + ledge <= x0 + L - backLen;
  const defl = -push * ypx;
  const top: string[] = [];
  const bot: string[] = [];
  for (let i = 0; i <= 30; i++) {
    const x = (L * i) / 30;
    const w = defl * shape(x);
    top.push(`${x0 + x},${yc - thick(x) / 2 + w}`);
    bot.push(`${x0 + x},${yc + thick(x) / 2 + w}`);
  }
  const tipW = defl;
  const hookBase = yc + thick(L) / 2 + tipW;
  const lead = (p.lead_deg * Math.PI) / 180;
  const ret = (p.return_deg * Math.PI) / 180;
  const hookFront = x0 + L;
  const hook = `${hookFront - ypx / Math.tan(Math.max(0.2, ret))},${hookBase} ${hookFront},${hookBase + ypx} ${hookFront + ypx / Math.tan(Math.max(0.2, lead))},${hookBase} `;
  const util = r?.utilisation ?? 0;
  const tone: 'pass' | 'warn' | 'fail' = util <= 0.6 ? 'pass' : util <= 1 ? 'warn' : 'fail';
  return (
    <Shell
      error={error}
      title="Cantilever snap fit (Bayer / Covestro design guide)"
      subtitle="The arm bends by the undercut y as the mating part slides past the hook; the root strain must stay below the material's allowable"
      inputs={
        <>
          <Select label="Material" value={p.material} options={matOpts} onChange={(v) => set({ material: v })} />
          <Slider label="Arm length L" value={p.length_mm} min={3} max={25} step={0.5} unit="mm" onChange={(v) => set({ length_mm: v })} />
          <Slider label="Root thickness t" value={p.thickness_mm} min={0.4} max={3} step={0.05} unit="mm" onChange={(v) => set({ thickness_mm: v })} />
          <Slider label="Width b" value={p.width_mm} min={1} max={12} step={0.5} unit="mm" onChange={(v) => set({ width_mm: v })} />
          <Slider label="Undercut y" value={p.undercut_mm} min={0.1} max={2} step={0.05} unit="mm" onChange={(v) => set({ undercut_mm: v })} />
          <Select label="Profile" value={p.profile} options={[{ value: 'constant', label: 'Constant thickness (Bayer type 1)' }, { value: 'tapered', label: 'Tapers to t/2 at the hook (type 2)' }]} onChange={(v) => set({ profile: v })} />
          <Slider label="Lead-in angle α" value={p.lead_deg} min={10} max={60} step={1} unit="°" onChange={(v) => set({ lead_deg: v })} />
          <Slider label="Return angle α′" value={p.return_deg} min={20} max={90} step={1} unit="°" onChange={(v) => set({ return_deg: v })} />
          <Slider label="Friction μ" value={p.friction} min={0.1} max={0.8} step={0.05} onChange={(v) => set({ friction: v })} />
        </>
      }
      diagram={
        <>
          <svg className="st-diagram" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Snap-fit arm deflecting over the mating part">
            <Hatch x={20} y={yc - 50} w={40} h={100} />
            <polygon className="part" points={[...top, ...bot.reverse()].join(' ')} />
            <polygon className="part" points={hook} />
            <rect className="part2" x={mateX} y={yc + tpx / 2 + ypx} width={ledge} height={46} />
            <line className="dim" x1={x0} x2={x0 + L} y1={yc - tpx / 2 - 22} y2={yc - tpx / 2 - 22} />
            <text x={x0 + L / 2} y={yc - tpx / 2 - 27} textAnchor="middle">L = {fmt(p.length_mm, 1)} mm</text>
            <text x={x0 + 6} y={yc + 4} className="lbl">t</text>
            <line className="dim" x1={hookFront + 18} x2={hookFront + 18} y1={yc + tpx / 2} y2={yc + tpx / 2 + ypx} />
            <text x={hookFront + 22} y={yc + tpx / 2 + ypx / 2 + 4}>y = {fmt(p.undercut_mm, 2)}</text>
            <text x={20} y={H - 10}>{push > 0.02 ? `deflecting: root strain ${fmt((r?.strain_pct ?? 0) * push, 2)} %` : latched ? 'latched ✓' : 'approaching'}</text>
          </svg>
          <div className="cp-toolbar">
            <PlayButton playing={playing} setPlaying={setPlaying} />
            <span className="small muted">The arm, hook and mating part are drawn to scale.</span>
          </div>
        </>
      }
      results={
        r && (
          <div className="stack" style={{ gap: 10 }}>
            <div className="grid grid-4">
              <Stat label="Root strain" value={fmt(r.strain_pct, 2)} unit="%" tone={tone === 'pass' ? 'pass' : tone === 'warn' ? 'warn' : 'fail'} note={`allowable ≈ ${fmt(r.allowable_strain_pct, 1)} % (single use, typical)`} />
              <Stat label="Deflection force P" value={fmt(r.deflection_force_n, 2)} unit="N" note={`E = ${fmt(r.e_mpa / 1000, 2)} GPa`} />
              <Stat label="Mating force W" value={fmt(r.mating_force_n, 2)} unit="N" note={`lead-in ${p.lead_deg}°`} />
              <Stat label="Separation force" value={r.self_locking ? '∞' : fmt(r.separation_force_n, 2)} unit={r.self_locking ? '' : 'N'} note={r.self_locking ? 'self-locking (release by hand)' : `return ${p.return_deg}°`} />
            </div>
            <Verdict tone={tone}>{r.verdict} Largest undercut for this arm at the allowable strain: {fmt(r.max_undercut_mm, 2)} mm.</Verdict>
          </div>
        )
      }
      extra={
        <Explainer
          title="Snap-fit equations"
          refs={[{ label: 'Bayer MaterialScience (Covestro), "Snap-Fit Joints for Plastics — A Design Guide"' }, { label: 'P. R. Bonenberger, "The First Snap-Fit Handbook", Hanser, 3rd ed., 2016' }]}
        >
          <Formula>{`ε = 1.5 · y t / L²   (constant section; 0.92 · y t / L² tapering to t/2)
P = b t² E_s ε / (6 L)                     deflection force at the hook
W = P (μ + tan α) / (1 − μ tan α)          mating (push-on) force`}</Formula>
          <p className="small">
            Strain grows with t/L², so a longer, thinner arm is the easiest fix. Iris's pod lids snap on with short clips moulded in PETG or PC; keep repeated-use clips below ~60 % of the single-use strain. E_s is the secant modulus at the working strain — for printed parts use the modulus in the direction the arm is printed.
          </p>
        </Explainer>
      }
    />
  );
}

// ------------------------------------------------------------------ bolted joint

const SIZES = ['M1.6', 'M2', 'M2.5', 'M3', 'M4', 'M5', 'M6', 'M8', 'M10', 'M12'];
const CLASSES = ['4.8', '5.8', '8.8', '10.9', '12.9', 'A2-70'];

function BoltCalc({ matOpts }: { matOpts: MatOpts }) {
  const [p, setP] = useState({ size: 'M2', class: 'A2-70', grip_mm: 4, nut_factor: 0.2, preload_fraction: 0.75, member_e_mpa: 68_900, load_n: 50, nut_material: 'AL6061-T6', engagement_mm: 3 });
  const set = (q: Partial<typeof p>) => setP((o) => ({ ...o, ...q }));
  const { data: r, error } = useCompute<BoltOut>('structures.bolt', p);
  const [ext, setExt] = useState(0.6);
  // Joint diagram (Shigley): bolt line rises with slope k_b from the origin to the preload; member line falls from δ_b to δ_b + δ_m.
  const jd = useMemo(() => {
    if (!r) return null;
    const db = r.preload_n / r.bolt_stiffness_n_mm;
    const dm = r.preload_n / r.member_stiffness_n_mm;
    const P = r.separation_load_n * ext;
    const dPlus = P / (r.bolt_stiffness_n_mm + r.member_stiffness_n_mm);
    return { db, dm, P, fb: r.preload_n + r.joint_constant * P, fm: Math.max(0, r.preload_n - (1 - r.joint_constant) * P), dPlus };
  }, [r, ext]);
  const W = 380;
  const H = 210;
  return (
    <Shell
      error={error}
      title="Bolted joint: preload, load sharing and thread stripping (Shigley ch. 8)"
      subtitle="Tightening stretches the bolt and squeezes the parts; an external load is shared in proportion to their stiffness"
      inputs={
        <>
          <Select label="Thread (ISO metric coarse)" value={p.size} options={SIZES.map((s) => ({ value: s, label: s }))} onChange={(v) => set({ size: v })} />
          <Select label="Property class" value={p.class} options={CLASSES.map((c) => ({ value: c, label: c === 'A2-70' ? 'A2-70 stainless' : `${c} steel` }))} onChange={(v) => set({ class: v })} />
          <Slider label="Grip length (clamped)" value={p.grip_mm} min={1} max={30} step={0.5} unit="mm" onChange={(v) => set({ grip_mm: v })} />
          <Slider label="Preload / proof load" value={p.preload_fraction} min={0.2} max={0.9} step={0.05} format={(v) => `${Math.round(v * 100)} %`} onChange={(v) => set({ preload_fraction: v })} />
          <Slider label="Nut factor K" value={p.nut_factor} min={0.1} max={0.3} step={0.01} onChange={(v) => set({ nut_factor: v })} />
          <Slider label="Member modulus" value={p.member_e_mpa} min={1000} max={210000} log unit="MPa" onChange={(v) => set({ member_e_mpa: v })} />
          <Slider label="External separating load" value={p.load_n} min={1} max={3000} log unit="N" onChange={(v) => set({ load_n: v })} />
          <Select label="Tapped part / insert material" value={p.nut_material} options={matOpts} onChange={(v) => set({ nut_material: v })} />
          <Slider label="Thread engagement" value={p.engagement_mm} min={0.5} max={15} step={0.25} unit="mm" onChange={(v) => set({ engagement_mm: v })} />
        </>
      }
      diagram={
        r &&
        jd && (
          <>
            <svg className="st-diagram" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Bolted joint diagram: force against deformation">
              {(() => {
                const x0 = 40;
                const y0 = H - 30;
                const sx = 300 / (jd.db + jd.dm);
                const sy = 150 / Math.max(r.preload_n * 1.4, jd.fb * 1.05);
                const X = (d: number) => x0 + d * sx;
                const Y = (f: number) => y0 - f * sy;
                return (
                  <g>
                    <line className="ground" x1={x0} y1={y0} x2={W - 20} y2={y0} />
                    <line className="ground" x1={x0} y1={y0} x2={x0} y2={14} />
                    <text x={W - 22} y={y0 + 16} textAnchor="end">deformation δ</text>
                    <text x={x0 + 4} y={16}>force</text>
                    <line x1={X(0)} y1={Y(0)} x2={X(jd.db + jd.dPlus)} y2={Y(jd.fb)} stroke="var(--series-1)" strokeWidth={2.2} />
                    <line x1={X(jd.db - jd.dPlus * 0)} y1={Y(r.preload_n)} x2={X(jd.db + jd.dm)} y2={Y(0)} stroke="var(--series-3)" strokeWidth={2.2} />
                    <line className="ghost" x1={X(jd.db)} y1={Y(r.preload_n)} x2={X(jd.db)} y2={y0} />
                    <line x1={X(jd.db + jd.dPlus)} y1={Y(jd.fb)} x2={X(jd.db + jd.dPlus)} y2={Y(jd.fm)} stroke="var(--series-2)" strokeWidth={3} />
                    <circle cx={X(jd.db)} cy={Y(r.preload_n)} r={3.5} fill="var(--text)" />
                    <text x={X(jd.db) - 6} y={Y(r.preload_n) - 8} textAnchor="end" className="lbl">F_i = {fmt(r.preload_n, 0)} N</text>
                    <text x={X(jd.db + jd.dPlus) + 6} y={(Y(jd.fb) + Y(jd.fm)) / 2 + 4}>P = {fmt(jd.P, 0)} N</text>
                    <text x={X(jd.db * 0.35)} y={Y(r.preload_n * 0.35) - 8} fill="var(--series-1)">bolt k_b</text>
                    <text x={X(jd.db + jd.dm * 0.55)} y={Y(r.preload_n * 0.45) - 6} fill="var(--series-3)">members k_m</text>
                  </g>
                );
              })()}
            </svg>
            <div className="cp-toolbar">
              <Slider label="Apply external load (share of the separation load)" value={ext} min={0} max={1} step={0.01} format={(v) => `${Math.round(v * 100)} %`} onChange={setExt} />
            </div>
          </>
        )
      }
      results={
        r && (
          <div className="stack" style={{ gap: 10 }}>
            <div className="grid grid-4">
              <Stat label="Preload F_i" value={fmt(r.preload_n, 0)} unit="N" note={`proof load ${fmt(r.proof_load_n, 0)} N (A_t ${fmt(r.stress_area_mm2, 2)} mm²)`} />
              <Stat label="Tightening torque" value={fmt(r.torque_nm, 3)} unit="N·m" note={`T = K F_i d, K = ${p.nut_factor}`} />
              <Stat label="Joint constant C" value={fmt(r.joint_constant, 3)} note={`bolt takes ${Math.round(r.joint_constant * 100)} % of P`} />
              <Stat label="Separation safety" value={fmt(r.separation_factor, 2)} unit="×" tone={sfTone(r.separation_factor, 1.5)} note={`separates at ${fmt(r.separation_load_n, 0)} N`} />
            </div>
            <div className="grid grid-4">
              <Stat label="Strip internal thread" value={fmt(r.strip_force_internal_n, 0)} unit="N" note={`A = ${fmt(r.strip_area_internal_mm2, 1)} mm² in ${r.nut_material}`} />
              <Stat label="Strip bolt thread" value={fmt(r.strip_force_external_n, 0)} unit="N" note={`A = ${fmt(r.strip_area_external_mm2, 1)} mm²`} />
              <Stat label="Break the bolt" value={fmt(r.tensile_capacity_n, 0)} unit="N" note="proof load" />
              <Stat label="Engagement for full strength" value={fmt(r.min_engagement_mm, 1)} unit="mm" tone={p.engagement_mm >= r.min_engagement_mm ? 'pass' : 'warn'} note={`${fmt(r.min_engagement_mm / r.d_mm, 1)} × d`} />
            </div>
            <Verdict tone={r.first_failure.startsWith('bolt breaks') ? 'pass' : 'warn'}>First to fail: {r.first_failure}.</Verdict>
            <ul className="small muted" style={{ margin: 0, paddingLeft: 18 }}>
              {r.notes.map((n, i) => (
                <li key={i}>{n}</li>
              ))}
            </ul>
          </div>
        )
      }
      extra={
        <Explainer title="Bolted-joint equations" refs={[{ label: 'R. G. Budynas & J. K. Nisbett, "Shigley’s Mechanical Engineering Design", 10th ed., ch. 8' }, { label: 'ISO 898-1 (steel bolt property classes); ISO 3506-1 (stainless)' }]}>
          <Formula>{`A_t = π/4 (d − 0.9382 p)²        F_i = f · S_p A_t        T = K F_i d
k_b = A E_b / l      k_m = E d A e^(B d / l)  (Wileman, A = 0.787, B = 0.629)
C = k_b / (k_b + k_m):   F_bolt = F_i + C P,   F_members = F_i − (1 − C) P
stripping:  A_int ≈ π d L_e · 7/8,   A_ext ≈ π D₁ L_e · 3/4,   τ = 0.577 S_y`}</Formula>
          <p className="small">
            The Iris hinge uses an M1.6–M2 stainless screw into a metal insert: torque it gently — the plastic around the insert, not the screw, sets the limit.
          </p>
        </Explainer>
      }
    />
  );
}

// ------------------------------------------------------------------ bosses

function BossCalc({ matOpts }: { matOpts: MatOpts }) {
  const [p, setP] = useState({ screw_d_mm: 2, engagement_mm: 4, wall_mm: 1.2, material: 'PETG', load_n: 20 });
  const set = (q: Partial<typeof p>) => setP((o) => ({ ...o, ...q }));
  const { data: r, error } = useCompute<BossOut>('structures.boss', p);
  const W = 380;
  const H = 190;
  const k = 110 / Math.max(r?.boss_od_mm ?? 4, 2);
  return (
    <Shell
      error={error}
      title="Screw boss for a thread-forming screw"
      subtitle="Pilot hole, boss diameter and a pull-out estimate for screws straight into plastic"
      inputs={
        <>
          <Select label="Part material" value={p.material} options={matOpts} onChange={(v) => set({ material: v })} />
          <Slider label="Screw diameter d" value={p.screw_d_mm} min={1.4} max={5} step={0.1} unit="mm" onChange={(v) => set({ screw_d_mm: v })} />
          <Slider label="Thread engagement" value={p.engagement_mm} min={1} max={12} step={0.5} unit="mm" onChange={(v) => set({ engagement_mm: v })} />
          <Slider label="Nominal wall" value={p.wall_mm} min={0.8} max={4} step={0.1} unit="mm" onChange={(v) => set({ wall_mm: v })} />
          <Slider label="Service pull-out load" value={p.load_n} min={1} max={200} log unit="N" onChange={(v) => set({ load_n: v })} />
        </>
      }
      diagram={
        r && (
          <svg className="st-diagram" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Screw boss cross-section">
            {(() => {
              const cx = W / 2;
              const top = 36;
              const hgt = Math.min(110, p.engagement_mm * k * 0.8 + 30);
              const ro = (r.boss_od_mm / 2) * k;
              const ri = (r.pilot_hole_mm / 2) * k;
              const rs = (p.screw_d_mm / 2) * k;
              const base = top + hgt;
              return (
                <g>
                  <rect className="part" x={cx - ro} y={top} width={ro - ri} height={hgt} />
                  <rect className="part" x={cx + ri} y={top} width={ro - ri} height={hgt} />
                  <rect className="part" x={30} y={base} width={W - 60} height={p.wall_mm * k * 0.6 + 6} />
                  <rect x={cx - rs} y={top - 22} width={2 * rs} height={hgt * 0.85 + 22} fill="var(--surface-3)" stroke="var(--text-2)" />
                  {Array.from({ length: 8 }, (_, i) => (
                    <line key={i} x1={cx - rs} x2={cx + rs} y1={top + 4 + i * ((hgt * 0.8) / 8)} y2={top + 10 + i * ((hgt * 0.8) / 8)} stroke="var(--text-2)" />
                  ))}
                  <rect x={cx - rs * 1.9} y={top - 30} width={rs * 3.8} height={9} rx={2} fill="var(--surface-3)" stroke="var(--text-2)" />
                  <line className="dim" x1={cx - ro} x2={cx + ro} y1={top - 38} y2={top - 38} />
                  <text x={cx} y={top - 42} textAnchor="middle">boss Ø {fmt(r.boss_od_mm, 1)} mm</text>
                  <text x={cx + ro + 8} y={top + hgt / 2}>pilot Ø {fmt(r.pilot_hole_mm, 2)}</text>
                  <text x={34} y={base + 20}>wall {fmt(p.wall_mm, 1)} mm · boss wall at base ≤ {fmt(r.max_boss_wall_at_base_mm, 2)} mm</text>
                </g>
              );
            })()}
          </svg>
        )
      }
      results={
        r && (
          <div className="stack" style={{ gap: 10 }}>
            <div className="grid grid-4">
              <Stat label="Pilot hole" value={fmt(r.pilot_hole_mm, 2)} unit="mm" />
              <Stat label="Boss Ø" value={fmt(r.boss_od_mm, 1)} unit="mm" note={`${fmt(r.boss_od_range_mm[0], 1)}–${fmt(r.boss_od_range_mm[1], 1)} mm`} />
              <Stat label="Pull-out estimate" value={fmt(r.pullout_force_n, 0)} unit="N" note={`τ ≈ ${fmt(r.shear_strength_mpa, 1)} MPa`} />
              <Stat label="Safety factor" value={fmt(r.safety_factor, 1)} unit="×" tone={sfTone(r.safety_factor, 3)} note={sfWord(r.safety_factor, 3)} />
            </div>
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr>
                    <th>Heat-set insert</th>
                    <th>Hole Ø</th>
                    <th>Insert Ø</th>
                    <th>Length</th>
                    <th>Min boss Ø</th>
                  </tr>
                </thead>
                <tbody>
                  {r.inserts.map((i) => (
                    <tr key={i.thread}>
                      <td>{i.thread}</td>
                      <td className="tnum">{i.hole_mm} mm</td>
                      <td className="tnum">{i.insert_od_mm} mm</td>
                      <td className="tnum">{i.length_mm} mm</td>
                      <td className="tnum">{i.min_boss_od_mm} mm</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <ul className="small muted" style={{ margin: 0, paddingLeft: 18 }}>
              {r.notes.map((n, i) => (
                <li key={i}>{n}</li>
              ))}
            </ul>
          </div>
        )
      }
      extra={
        <Explainer title="Boss rules and the pull-out estimate" refs={[{ label: 'R. A. Malloy, "Plastic Part Design for Injection Molding", Hanser, 2nd ed., 2010' }]}>
          <Formula>{`pilot ≈ 0.8 d (ductile) … 0.85 d (brittle)     boss Ø ≈ 2 d … 2.5 d
F_pull-out ≈ π D_p L τ,   D_p = (d + pilot)/2,   τ ≈ S_y / √3`}</Formula>
        </Explainer>
      }
    />
  );
}

// ------------------------------------------------------------------ beams

function SectionInputs({ s, onChange }: { s: Section; onChange: (s: Section) => void }) {
  return (
    <>
      <Select label="Section" value={s.shape} options={[{ value: 'rect', label: 'Rectangle' }, { value: 'round', label: 'Round bar' }, { value: 'tube', label: 'Round tube' }]} onChange={(v) => onChange({ ...s, shape: v })} />
      {s.shape === 'rect' && <Slider label="Width b (along the neutral axis)" value={s.b} min={0.5} max={20} step={0.1} unit="mm" onChange={(v) => onChange({ ...s, b: v })} />}
      <Slider label={s.shape === 'rect' ? 'Depth h (in the bending direction)' : 'Outer diameter'} value={s.h} min={0.5} max={20} step={0.1} unit="mm" onChange={(v) => onChange({ ...s, h: v })} />
      {s.shape === 'tube' && <Slider label="Wall t" value={s.t} min={0.2} max={Math.max(0.3, s.h / 2)} step={0.05} unit="mm" onChange={(v) => onChange({ ...s, t: v })} />}
    </>
  );
}

function BeamCalc({ matOpts }: { matOpts: MatOpts }) {
  const frame = useProject((s) => s.project.design.frame);
  const [p, setP] = useState({
    support: 'cantilever',
    load: 'point',
    length_mm: 75,
    force_n: 5,
    section: { shape: 'rect', b: frame.temple_height_mm, h: frame.temple_width_mm, t: 1 } as Section,
    material: frame.material,
  });
  const set = (q: Partial<typeof p>) => setP((o) => ({ ...o, ...q }));
  const { data: r, error } = useCompute<BeamOut>('structures.beam', p);
  const [t, playing, setPlaying] = usePlay(2.4);
  const phase = 0.5 - 0.5 * Math.cos(2 * Math.PI * t);
  const W = 380;
  const H = 170;
  const x0 = 40;
  const Lp = 300;
  const yb = 70;
  const dmax = r ? Math.max(...r.deflection.map(Math.abs)) : 1;
  const scale = dmax > 0 ? 40 / dmax : 0;
  const pts = r ? r.x.map((x, i) => `${x0 + (x / p.length_mm) * Lp},${yb + phase * scale * r.deflection[i]}`).join(' ') : '';
  const csv = () => (r ? toCsv([['x_mm', 'deflection_mm', 'moment_nmm'], ...r.x.map((x, i) => [+x.toFixed(3), +r.deflection[i].toExponential(5), +r.moment[i].toFixed(4)])]) : '');
  return (
    <Shell
      error={error}
      title="Euler–Bernoulli beam"
      subtitle="Closed-form deflection, moment and bending stress (Roark / Shigley beam tables); the default is an Iris temple arm under a 5 N spread"
      inputs={
        <>
          <Select label="Supports" value={p.support} options={[{ value: 'cantilever', label: 'Cantilever (fixed–free)' }, { value: 'simply_supported', label: 'Simply supported' }, { value: 'fixed_fixed', label: 'Fixed both ends' }]} onChange={(v) => set({ support: v })} />
          <Select label="Load" value={p.load} options={[{ value: 'point', label: p.support === 'cantilever' ? 'Point load at the tip' : 'Point load at mid-span' }, { value: 'udl', label: 'Uniformly distributed' }]} onChange={(v) => set({ load: v })} />
          <Slider label="Length L" value={p.length_mm} min={5} max={200} step={1} unit="mm" onChange={(v) => set({ length_mm: v })} />
          <Slider label="Total force" value={p.force_n} min={0.1} max={100} log unit="N" onChange={(v) => set({ force_n: v })} />
          <Select label="Material" value={p.material} options={matOpts} onChange={(v) => set({ material: v })} />
          <SectionInputs s={p.section} onChange={(s) => set({ section: s })} />
        </>
      }
      diagram={
        r && (
          <>
            <svg className="st-diagram" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Beam with supports and animated deflected shape">
              {p.support === 'cantilever' ? (
                <Hatch x={x0 - 22} y={yb - 30} w={22} h={60} />
              ) : (
                <>
                  <polygon className="ground" points={`${x0},${yb + 4} ${x0 - 9},${yb + 18} ${x0 + 9},${yb + 18}`} />
                  <polygon className="ground" points={`${x0 + Lp},${yb + 4} ${x0 + Lp - 9},${yb + 18} ${x0 + Lp + 9},${yb + 18}`} />
                  {p.support === 'fixed_fixed' && (
                    <>
                      <Hatch x={x0 - 22} y={yb - 30} w={22} h={60} />
                      <Hatch x={x0 + Lp} y={yb - 30} w={22} h={60} />
                    </>
                  )}
                </>
              )}
              <line className="ghost" x1={x0} y1={yb} x2={x0 + Lp} y2={yb} />
              <polyline points={pts} fill="none" stroke="var(--series-1)" strokeWidth={Math.max(3, Math.min(10, p.section.h * 1.2))} strokeLinecap="round" strokeLinejoin="round" />
              {p.load === 'point' ? (
                <Arrow x1={p.support === 'cantilever' ? x0 + Lp : x0 + Lp / 2} y1={18} x2={p.support === 'cantilever' ? x0 + Lp : x0 + Lp / 2} y2={yb - 8 + phase * scale * (p.support === 'cantilever' ? r.deflection[r.deflection.length - 1] : r.deflection[Math.floor(r.deflection.length / 2)])} />
              ) : (
                Array.from({ length: 9 }, (_, i) => <Arrow key={i} x1={x0 + (Lp * i) / 8} y1={24} x2={x0 + (Lp * i) / 8} y2={yb - 8} />)
              )}
              <text x={x0} y={H - 12}>deflection drawn ×{fmt(scale / (Lp / p.length_mm), 0)} · max {fmt(r.max_deflection_mm, 3)} mm</text>
            </svg>
            <div className="cp-toolbar">
              <PlayButton playing={playing} setPlaying={setPlaying} />
              <DownloadButton small filename="beam.csv" label="deflection & moment CSV" content={csv} />
            </div>
          </>
        )
      }
      results={
        r && (
          <div className="stack" style={{ gap: 10 }}>
            <div className="grid grid-4">
              <Stat label="Max deflection" value={fmt(r.max_deflection_mm, 3)} unit="mm" note={`stiffness ${fmt(r.stiffness_n_mm, 2)} N/mm`} />
              <Stat label="Max moment" value={fmt(r.max_moment_nmm, 1)} unit="N·mm" note={`at x = ${fmt(r.max_moment_x_mm, 1)} mm`} />
              <Stat label="Bending stress" value={fmt(r.max_stress_mpa, 2)} unit="MPa" note={`Z = ${fmt(r.section.z_mm3, 2)} mm³, I = ${fmt(r.section.i_mm4, 2)} mm⁴`} />
              <Stat label="Safety factor" value={fmt(r.safety_factor, 2)} unit="×" tone={sfTone(r.safety_factor)} note={`${sfWord(r.safety_factor)} (yield ${fmt(r.yield_mpa, 0)} MPa)`} />
            </div>
            <div className="grid grid-2">
              <Plot series={[{ name: 'deflection', x: r.x, y: r.deflection }]} xLabel="x, mm" yLabel="deflection, mm" height={170} yZero />
              <Plot series={[{ name: 'moment', x: r.x, y: r.moment, color: 'var(--series-2)', area: true }]} xLabel="x, mm" yLabel="moment, N·mm" height={170} yZero />
            </div>
            <div className="small muted">{r.formula}</div>
          </div>
        )
      }
      extra={
        <Explainer title="Beam theory" refs={[{ label: 'W. C. Young & R. G. Budynas, "Roark’s Formulas for Stress and Strain", 7th ed., 2002' }, { label: 'Budynas & Nisbett, "Shigley’s Mechanical Engineering Design", Table A-9' }]}>
          <Formula>{`E I w''''(x) = q(x)        M = −E I w''        σ = M c / I = M / Z
cantilever, tip load:  δ = F L³ / (3 E I),  σ_max = F L c / I`}</Formula>
          <p className="small">Compare with the Strength tab: the FEA of the real temple gives the same order of stress in the slim arm, plus the extra flexibility of the pod and the stress concentration where the arm meets it.</p>
        </Explainer>
      }
    />
  );
}

function BuckleCalc({ matOpts }: { matOpts: MatOpts }) {
  const [p, setP] = useState({ length_mm: 12, section: { shape: 'rect', b: 6, h: 0.8, t: 0 } as Section, ends: 'fixed_free', material: 'PETG', load_n: 10, plate_width_mm: 6, plate_edges: 'free' });
  const set = (q: Partial<typeof p>) => setP((o) => ({ ...o, ...q }));
  const { data: r, error } = useCompute<BuckleOut>('structures.buckling', p);
  const [t, playing, setPlaying] = usePlay(2.8);
  const amp = r ? Math.min(1, p.load_n / Math.max(1e-9, r.critical_load_n)) : 0;
  const bow = (0.5 - 0.5 * Math.cos(2 * Math.PI * t)) * amp;
  const W = 380;
  const H = 190;
  const x = 190;
  const y0 = 30;
  const y1 = 160;
  const shapeFn = (s: number) => {
    // s ∈ [0, 1] from the bottom; mode shapes for the end conditions.
    switch (p.ends) {
      case 'pinned_pinned':
        return Math.sin(Math.PI * s);
      case 'fixed_fixed':
        return 0.5 * (1 - Math.cos(2 * Math.PI * s));
      case 'fixed_pinned':
        return Math.sin(4.4934 * s) / 0.976 - s * Math.sin(4.4934) / 0.976;
      default:
        return 1 - Math.cos((Math.PI / 2) * s);
    }
  };
  const pts = Array.from({ length: 31 }, (_, i) => {
    const s = i / 30;
    return `${x + 38 * bow * shapeFn(s)},${y1 - s * (y1 - y0)}`;
  }).join(' ');
  return (
    <Shell
      error={error}
      title="Column and thin-wall buckling"
      subtitle="Euler for slender columns, Johnson's parabola for short ones, and plate buckling for thin walls and ribs"
      inputs={
        <>
          <Select label="End conditions" value={p.ends} options={[{ value: 'fixed_free', label: 'Fixed – free (K = 2)' }, { value: 'pinned_pinned', label: 'Pinned – pinned (K = 1)' }, { value: 'fixed_pinned', label: 'Fixed – pinned (K = 0.7)' }, { value: 'fixed_fixed', label: 'Fixed – fixed (K = 0.5)' }]} onChange={(v) => set({ ends: v })} />
          <Slider label="Length L" value={p.length_mm} min={2} max={100} step={0.5} unit="mm" onChange={(v) => set({ length_mm: v })} />
          <Slider label="Compressive load" value={p.load_n} min={0.1} max={2000} log unit="N" onChange={(v) => set({ load_n: v })} />
          <Select label="Material" value={p.material} options={matOpts} onChange={(v) => set({ material: v })} />
          <SectionInputs s={p.section} onChange={(s) => set({ section: s })} />
          <Slider label="Plate check: unloaded width" value={p.plate_width_mm} min={0} max={30} step={0.5} unit="mm" onChange={(v) => set({ plate_width_mm: v })} />
          <Select label="Plate edges" value={p.plate_edges} options={[{ value: 'free', label: 'One free edge (a rib), k = 0.425' }, { value: 'ss', label: 'All edges supported (a wall), k = 4' }]} onChange={(v) => set({ plate_edges: v })} />
        </>
      }
      diagram={
        r && (
          <>
            <svg className="st-diagram" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Column buckling mode">
              <Hatch x={x - 30} y={y1} w={60} h={14} />
              {(p.ends === 'fixed_fixed' || p.ends === 'fixed_pinned') && <Hatch x={x - 30} y={y0 - 14} w={60} h={14} />}
              <line className="ghost" x1={x} y1={y0} x2={x} y2={y1} />
              <polyline points={pts} fill="none" stroke="var(--series-1)" strokeWidth={6} strokeLinecap="round" />
              <Arrow x1={x + 38 * bow * shapeFn(1)} y1={4} x2={x + 38 * bow * shapeFn(1)} y2={y0 - 3} />
              <text x={20} y={24}>P = {fmt(p.load_n, 1)} N</text>
              <text x={20} y={42}>P_cr = {fmt(r.critical_load_n, 1)} N</text>
              <text x={250} y={H - 16}>{p.load_n >= r.critical_load_n ? '✕ buckles' : `load = ${Math.round(amp * 100)} % of P_cr`}</text>
            </svg>
            <div className="cp-toolbar">
              <PlayButton playing={playing} setPlaying={setPlaying} />
              <span className="small muted">Mode shape amplitude drawn ∝ P / P_cr (schematic).</span>
            </div>
          </>
        )
      }
      results={
        r && (
          <div className="stack" style={{ gap: 10 }}>
            <div className="grid grid-4">
              <Stat label="Critical load" value={fmt(r.critical_load_n, 1)} unit="N" note={r.regime} />
              <Stat label="Safety factor" value={fmt(r.safety_factor, 2)} unit="×" tone={sfTone(r.safety_factor, 3)} note={sfWord(r.safety_factor, 3)} />
              <Stat label="Slenderness K L / r" value={fmt(r.slenderness, 0)} note={`transition ${fmt(r.transition_slenderness, 0)}`} />
              <Stat label="Plate buckling stress" value={r.plate_stress_mpa > 0 ? fmt(r.plate_stress_mpa, 1) : '—'} unit={r.plate_stress_mpa > 0 ? 'MPa' : ''} note={r.plate_stress_mpa > 0 ? `applied ${fmt(r.applied_stress_mpa, 2)} MPa, load ${fmt(r.plate_load_n, 0)} N` : 'plate check off'} />
            </div>
            <Plot series={[{ name: 'P_cr', x: r.curve_l, y: r.curve_p }]} xLabel="length, mm" yLabel="critical load, N" yLog height={180} vLines={[{ value: p.length_mm, label: 'now' }]} hLines={[{ value: p.load_n, label: 'applied' }]} />
          </div>
        )
      }
      extra={
        <Explainer title="Buckling formulas" refs={[{ label: 'Budynas & Nisbett, "Shigley’s Mechanical Engineering Design", §4-11 to 4-14' }, { label: 'Young & Budynas, "Roark’s Formulas for Stress and Strain", ch. 15' }]}>
          <Formula>{`Euler:    P_cr = π² E I / (K L)²              (K L / r ≥ √(2π² E / S_y))
Johnson:  P_cr = A [S_y − (S_y K L / (2π r))² / E]
plate:    σ_cr = k π² E / (12 (1 − ν²)) · (t / b)²`}</Formula>
        </Explainer>
      }
    />
  );
}

// ------------------------------------------------------------------ ISO 286 fits

function FitsCalc() {
  const fits = useCompute<{ hole: string; shaft: string; name: string }[]>('structures.fits', {}).data ?? [];
  const [p, setP] = useState({ nominal_mm: 1.5, hole: 'H7', shaft: 'g6' });
  const set = (q: Partial<typeof p>) => setP((o) => ({ ...o, ...q }));
  const { data: r, error } = useCompute<FitOut>('structures.fit', p);
  const W = 380;
  const H = 210;
  const vals = r ? [r.hole_es_um, r.hole_ei_um, r.shaft_es_um, r.shaft_ei_um, 0] : [0];
  const lo = Math.min(...vals);
  const hi = Math.max(...vals);
  const span = Math.max(1, hi - lo);
  const Y = (um: number) => 30 + ((hi - um) / span) * 140;
  return (
    <Shell
      error={error}
      title="ISO 286 limits and fits"
      subtitle="The letter places the tolerance zone, the number sets its width (IT grade) — here drawn to scale in micrometres"
      inputs={
        <>
          <Slider label="Nominal size" value={p.nominal_mm} min={1} max={120} log unit="mm" onChange={(v) => set({ nominal_mm: +v.toFixed(2) })} />
          <Select label="Hole" value={p.hole} options={['D9', 'E8', 'F8', 'G7', 'H6', 'H7', 'H8', 'H9', 'H11', 'JS7', 'K7', 'M7', 'N7', 'P7'].map((h) => ({ value: h, label: h }))} onChange={(v) => set({ hole: v })} />
          <Select label="Shaft" value={p.shaft} options={['d9', 'e8', 'f7', 'g6', 'h6', 'h7', 'h9', 'js6', 'k6', 'm6', 'n6', 'p6', 'r6', 's6'].map((h) => ({ value: h, label: h }))} onChange={(v) => set({ shaft: v })} />
          <div className="small muted" style={{ margin: '8px 0 6px' }}>Preferred fits</div>
          <div className="row" style={{ gap: 6 }}>
            {fits.map((f) => (
              <button key={f.hole + f.shaft} className={`btn small ${p.hole === f.hole && p.shaft === f.shaft ? 'primary' : ''}`} onClick={() => set({ hole: f.hole, shaft: f.shaft })} title={f.name}>
                {f.hole}/{f.shaft}
              </button>
            ))}
          </div>
        </>
      }
      diagram={
        r && (
          <svg className="st-diagram" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Tolerance zones of hole and shaft">
            <line x1={20} x2={W - 20} y1={Y(0)} y2={Y(0)} stroke="var(--text)" strokeWidth={1.5} />
            <text x={24} y={Y(0) - 5}>zero line = {fmt(r.nominal_mm, 2)} mm</text>
            <rect className="part" x={110} y={Y(r.hole_es_um)} width={80} height={Math.max(2, Y(r.hole_ei_um) - Y(r.hole_es_um))} />
            <text x={150} y={Y(r.hole_es_um) - 6} textAnchor="middle" className="lbl">hole {r.hole}</text>
            <rect className="part2" x={210} y={Y(r.shaft_es_um)} width={80} height={Math.max(2, Y(r.shaft_ei_um) - Y(r.shaft_es_um))} />
            <text x={250} y={Y(r.shaft_ei_um) + 14} textAnchor="middle" className="lbl">shaft {r.shaft}</text>
            {[r.hole_es_um, r.hole_ei_um].map((v, i) => (
              <text key={`h${i}`} x={104} y={Y(v) + 4} textAnchor="end">{v > 0 ? '+' : ''}{fmt(v, 0)} µm</text>
            ))}
            {[r.shaft_es_um, r.shaft_ei_um].map((v, i) => (
              <text key={`s${i}`} x={296} y={Y(v) + 4}>{v > 0 ? '+' : ''}{fmt(v, 0)} µm</text>
            ))}
          </svg>
        )
      }
      results={
        r && (
          <div className="stack" style={{ gap: 10 }}>
            <div className="grid grid-4">
              <Stat label="Hole" value={`${fmt(r.hole_min_mm, 4)}–${fmt(r.hole_max_mm, 4)}`} unit="mm" />
              <Stat label="Shaft" value={`${fmt(r.shaft_min_mm, 4)}–${fmt(r.shaft_max_mm, 4)}`} unit="mm" />
              <Stat label="Clearance" value={`${fmt(r.min_clearance_um, 0)} … ${fmt(r.max_clearance_um, 0)}`} unit="µm" note="negative = interference" />
              <Stat label="Fit type" value={r.fit_type} tone={r.fit_type === 'clearance' ? 'pass' : r.fit_type === 'interference' ? 'warn' : undefined} />
            </div>
            <Verdict tone="pass">{r.description}</Verdict>
            <div className="small muted">
              Iris hinge pin (Ø 1.5 mm): a sliding fit H7/g6 lets the temple turn without play in a metal knuckle. Printed holes are far looser than IT7 (±0.1 mm is IT11–IT12 at this size) — ream them or use a metal bush.
            </div>
          </div>
        )
      }
      extra={
        <Explainer title="How an ISO fit is built" refs={[{ label: 'ISO 286-1 and ISO 286-2 (ISO code system for tolerances on linear sizes)' }, { label: 'Budynas & Nisbett, "Shigley’s Mechanical Engineering Design", §7-8' }]}>
          <Formula>{`shaft: es / ei = fundamental deviation (letter) ;  width = IT grade (number)
hole K, M, N (≤ IT8), P (≤ IT7):  ES = −ei + Δ,   Δ = IT_n − IT_(n−1)
clearance_max = ES − ei      clearance_min = EI − es`}</Formula>
        </Explainer>
      }
    />
  );
}

function PressCalc({ matOpts }: { matOpts: MatOpts }) {
  const [p, setP] = useState({ d_mm: 1.5, interference_mm: 0.02, hub_od_mm: 4, shaft_id_mm: 0, length_mm: 5, friction: 0.3, hub_material: 'PETG', shaft_material: 'SS304' });
  const set = (q: Partial<typeof p>) => setP((o) => ({ ...o, ...q }));
  const { data: r, error } = useCompute<PressOut>('structures.press_fit', p);
  const W = 380;
  const H = 200;
  const k = 80 / (p.hub_od_mm / 2);
  const cx = 110;
  const cy = 100;
  return (
    <Shell
      error={error}
      title="Press fit (Lamé thick cylinders)"
      subtitle="Interference squeezes the shaft and stretches the hub — the hoop stress at the hub bore governs"
      inputs={
        <>
          <Slider label="Interface diameter" value={p.d_mm} min={0.5} max={20} step={0.1} unit="mm" onChange={(v) => set({ d_mm: v, hub_od_mm: Math.max(p.hub_od_mm, v * 1.3) })} />
          <Slider label="Diametral interference" value={p.interference_mm} min={0.002} max={0.2} log unit="mm" onChange={(v) => set({ interference_mm: v })} />
          <Slider label="Hub outer diameter" value={p.hub_od_mm} min={p.d_mm * 1.1} max={p.d_mm * 6} step={0.1} unit="mm" onChange={(v) => set({ hub_od_mm: v })} />
          <Slider label="Engagement length" value={p.length_mm} min={1} max={30} step={0.5} unit="mm" onChange={(v) => set({ length_mm: v })} />
          <Slider label="Friction μ" value={p.friction} min={0.05} max={0.6} step={0.01} onChange={(v) => set({ friction: v })} />
          <Select label="Hub material" value={p.hub_material} options={matOpts} onChange={(v) => set({ hub_material: v })} />
          <Select label="Shaft material" value={p.shaft_material} options={matOpts} onChange={(v) => set({ shaft_material: v })} />
        </>
      }
      diagram={
        r && (
          <svg className="st-diagram" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Press-fit cross-section and hoop stress">
            <circle className="part" cx={cx} cy={cy} r={(p.hub_od_mm / 2) * k} />
            <circle className="part2" cx={cx} cy={cy} r={(p.d_mm / 2) * k} />
            {Array.from({ length: 12 }, (_, i) => {
              const a = (i / 12) * 2 * Math.PI;
              const r0 = (p.d_mm / 2) * k;
              return <Arrow key={i} x1={cx + (r0 + 16) * Math.cos(a)} y1={cy + (r0 + 16) * Math.sin(a)} x2={cx + (r0 + 3) * Math.cos(a)} y2={cy + (r0 + 3) * Math.sin(a)} />;
            })}
            <text x={cx} y={cy + 4} textAnchor="middle" className="lbl">p = {fmt(r.pressure_mpa, 1)}</text>
            <g transform="translate(215 30)">
              <text x={0} y={0} className="lbl">hub stress vs radius</text>
              {(() => {
                const w = 145;
                const h = 130;
                const mx = Math.max(...r.hoop, ...r.radial.map(Math.abs)) || 1;
                const X = (rr: number) => ((rr - r.r[0]) / (r.r[r.r.length - 1] - r.r[0])) * w;
                const Yv = (v: number) => h / 2 - (v / mx) * (h / 2 - 6) + 10;
                return (
                  <g>
                    <line className="dim" x1={0} x2={w} y1={Yv(0)} y2={Yv(0)} />
                    <polyline fill="none" stroke="var(--series-2)" strokeWidth={2} points={r.r.map((rr, i) => `${X(rr)},${Yv(r.hoop[i])}`).join(' ')} />
                    <polyline fill="none" stroke="var(--series-1)" strokeWidth={2} points={r.r.map((rr, i) => `${X(rr)},${Yv(r.radial[i])}`).join(' ')} />
                    <text x={w} y={Yv(r.hoop[r.hoop.length - 1]) - 6} textAnchor="end" fill="var(--series-2)">hoop</text>
                    <text x={w} y={Yv(r.radial[r.radial.length - 1]) + 14} textAnchor="end" fill="var(--series-1)">radial</text>
                  </g>
                );
              })()}
            </g>
          </svg>
        )
      }
      results={
        r && (
          <div className="grid grid-4">
            <Stat label="Contact pressure" value={fmt(r.pressure_mpa, 2)} unit="MPa" />
            <Stat label="Hub hoop stress" value={fmt(r.hub_hoop_mpa, 1)} unit="MPa" note={`von Mises ${fmt(r.hub_von_mises_mpa, 1)} MPa`} />
            <Stat label="Hub safety factor" value={fmt(r.hub_safety_factor, 2)} unit="×" tone={sfTone(r.hub_safety_factor)} note={sfWord(r.hub_safety_factor)} />
            <Stat label="Push-out force" value={fmt(r.axial_force_n, 1)} unit="N" note={`torque ${fmt(r.torque_nm * 1000, 1)} N·mm`} />
          </div>
        )
      }
      extra={
        <Explainer title="Lamé press-fit equations" refs={[{ label: 'Budynas & Nisbett, "Shigley’s Mechanical Engineering Design", §3-16' }]}>
          <Formula>{`p = δ / [ R/E_o ((r_o² + R²)/(r_o² − R²) + ν_o) + R/E_i ((R² + r_i²)/(R² − r_i²) − ν_i) ]
hub bore: σ_t = p (r_o² + R²)/(r_o² − R²),  σ_r = −p;   push-out F = μ p π d L`}</Formula>
          <p className="small">Plastics creep: a press-fit pin in PETG loses much of its grip over months. Knurled pins, heat staking or metal knuckles hold.</p>
        </Explainer>
      }
    />
  );
}

// ------------------------------------------------------------------ thermal

function ThermalCalc({ matOpts }: { matOpts: MatOpts }) {
  const [p, setP] = useState({ outer: 'PETG', inner: 'FR-4', length_mm: 60, clearance_mm: 0.4, t_assembly_c: 23, t_min_c: -10, t_max_c: 50, t_outer_mm: 1.2, t_inner_mm: 0.8 });
  const set = (q: Partial<typeof p>) => setP((o) => ({ ...o, ...q }));
  const { data: r, error } = useCompute<ThermalOut>('structures.thermal', p);
  const [temp, setTemp] = useState(-10);
  const W = 380;
  const H = 180;
  const dT = temp - p.t_assembly_c;
  const kappa = r ? r.bimetal_curvature_per_k * dT : 0;
  // Bow drawn exaggerated so it is visible.
  const bowPx = r ? Math.max(-40, Math.min(40, (kappa * (p.length_mm / 1000) ** 2 / 8) * 1000 * 400)) : 0;
  const clr = r ? p.clearance_mm + ((r.alpha_outer_ppm - r.alpha_inner_ppm) * 1e-6 * p.length_mm * dT) : 0;
  return (
    <Shell
      error={error}
      title="Thermal expansion mismatch"
      subtitle="Plastics expand 3–6× more than FR-4 or metals: gaps close in the cold, bonded layers bow"
      inputs={
        <>
          <Select label="Outer part (housing)" value={p.outer} options={matOpts} onChange={(v) => set({ outer: v })} />
          <Select label="Inner part (board, lens, insert)" value={p.inner} options={matOpts} onChange={(v) => set({ inner: v })} />
          <Slider label="Fit length" value={p.length_mm} min={5} max={200} step={1} unit="mm" onChange={(v) => set({ length_mm: v })} />
          <Slider label="Clearance at assembly" value={p.clearance_mm} min={-0.2} max={1} step={0.01} unit="mm" onChange={(v) => set({ clearance_mm: v })} />
          <Slider label="Cold limit" value={p.t_min_c} min={-40} max={20} step={1} unit="°C" onChange={(v) => set({ t_min_c: v })} />
          <Slider label="Hot limit" value={p.t_max_c} min={25} max={100} step={1} unit="°C" onChange={(v) => set({ t_max_c: v })} />
          <Slider label="Bonded layers: outer thickness" value={p.t_outer_mm} min={0.2} max={5} step={0.1} unit="mm" onChange={(v) => set({ t_outer_mm: v })} />
          <Slider label="Bonded layers: inner thickness" value={p.t_inner_mm} min={0.2} max={5} step={0.1} unit="mm" onChange={(v) => set({ t_inner_mm: v })} />
        </>
      }
      diagram={
        r && (
          <>
            <svg className="st-diagram" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Board in a housing and a bonded bi-material strip at the chosen temperature">
              <rect className="part" x={20} y={20} width={200} height={50} rx={6} />
              <rect x={30} y={30} width={180} height={30} fill="var(--surface-2)" />
              <rect className="part2" x={35} y={35} width={Math.max(10, 170 - (p.clearance_mm - clr) * 400)} height={20} />
              <text x={120} y={86} textAnchor="middle">clearance {fmt(clr, 3)} mm {clr < 0 ? '✕ pressed' : '✓'}</text>
              <path d={`M 240 ${120} Q 300 ${120 - 2 * bowPx} 360 ${120}`} fill="none" stroke="var(--series-1)" strokeWidth={7} />
              <path d={`M 240 ${127} Q 300 ${127 - 2 * bowPx} 360 ${127}`} fill="none" stroke="var(--series-3)" strokeWidth={7} />
              <text x={300} y={150} textAnchor="middle">bonded strip (bow exaggerated)</text>
              <text x={20} y={H - 10} className="lbl">{temp} °C</text>
            </svg>
            <div className="cp-toolbar">
              <Slider label="Temperature" value={temp} min={p.t_min_c} max={p.t_max_c} step={1} unit="°C" onChange={setTemp} />
            </div>
          </>
        )
      }
      results={
        r && (
          <div className="stack" style={{ gap: 10 }}>
            <div className="grid grid-4">
              <Stat label="Clearance change" value={fmt(r.dclear_um_per_k, 2)} unit="µm/K" note={`α ${fmt(r.alpha_outer_ppm, 0)} vs ${fmt(r.alpha_inner_ppm, 0)} ppm/K`} />
              <Stat label="Clearance, cold" value={fmt(r.clearance_cold_mm, 3)} unit="mm" tone={r.clearance_cold_mm >= 0 ? 'pass' : 'fail'} note={`at ${p.t_min_c} °C`} />
              <Stat label="Clearance, hot" value={fmt(r.clearance_hot_mm, 3)} unit="mm" tone={r.clearance_hot_mm >= 0 ? 'pass' : 'fail'} note={`at ${p.t_max_c} °C`} />
              <Stat label="Bonded-strip bow" value={fmt(Math.max(Math.abs(r.bimetal_bow_cold_mm), Math.abs(r.bimetal_bow_hot_mm)), 3)} unit="mm" note={`over ${p.length_mm} mm`} />
            </div>
            <Plot series={[{ name: 'clearance', x: r.temps_c, y: r.clearance_mm }]} xLabel="temperature, °C" yLabel="clearance, mm" height={170} hLines={[{ value: 0, label: 'contact' }]} vLines={[{ value: temp, label: 'now' }]} />
            <ul className="small muted" style={{ margin: 0, paddingLeft: 18 }}>
              {r.notes.map((n, i) => (
                <li key={i}>{n}</li>
              ))}
              <li>
                Fully constrained upper bound: {fmt(r.constrained_stress_mpa, 1)} MPa vs yield {fmt(r.outer_yield_mpa, 0)} MPa{r.closing_temp_c != null ? `; the gap closes at ${fmt(r.closing_temp_c, 0)} °C.` : '.'}
              </li>
            </ul>
          </div>
        )
      }
      extra={
        <Explainer title="Expansion equations" refs={[{ label: 'S. Timoshenko, "Analysis of bi-metal thermostats", J. Opt. Soc. Am. 11, 1925' }, { label: 'J. R. Davis (ed.), ASM handbooks — typical expansion coefficients' }]}>
          <Formula>{`Δc = (α_outer − α_inner) L ΔT          σ_constrained ≤ E Δα ΔT
bonded strip:  κ = 6 Δα ΔT (1 + m)² / ( h [3 (1 + m)² + (1 + m n)(m² + 1/(m n))] ),  m = t₁/t₂, n = E₁/E₂`}</Formula>
        </Explainer>
      }
    />
  );
}

// ------------------------------------------------------------------ stress concentration

function KtCalc({ matOpts }: { matOpts: MatOpts }) {
  const [p, setP] = useState({ geometry: 'hole', width_mm: 8, hole_mm: 1.6, a_mm: 1, b_mm: 0.5, radius_mm: 0.3, thickness_mm: 4, force_n: 100, q: 1, material: 'PETG' });
  const set = (q: Partial<typeof p>) => setP((o) => ({ ...o, ...q }));
  const { data: r, error } = useCompute<KtOut>('structures.stress_conc', p);
  const W = 380;
  const H = 190;
  const k = 150 / p.width_mm;
  const cx = 150;
  const cy = 95;
  const x = p.geometry === 'hole' ? p.hole_mm / p.width_mm : p.geometry === 'ellipse' ? p.a_mm / p.b_mm : p.a_mm / p.radius_mm;
  return (
    <Shell
      error={error}
      title="Stress concentration factor K_t"
      subtitle="Holes and notches raise the local stress above the nominal one — how much depends on the geometry ratio"
      inputs={
        <>
          <Select label="Geometry" value={p.geometry} options={[{ value: 'hole', label: 'Hole in a plate (tension)' }, { value: 'ellipse', label: 'Elliptical hole (wide plate)' }, { value: 'notch', label: 'Edge notch, depth a, root radius ρ' }]} onChange={(v) => set({ geometry: v })} />
          <Slider label="Plate width W" value={p.width_mm} min={2} max={40} step={0.5} unit="mm" onChange={(v) => set({ width_mm: v })} />
          {p.geometry === 'hole' && <Slider label="Hole diameter d" value={p.hole_mm} min={0.2} max={p.width_mm * 0.9} step={0.1} unit="mm" onChange={(v) => set({ hole_mm: v })} />}
          {p.geometry !== 'hole' && <Slider label={p.geometry === 'ellipse' ? 'Semi-axis a (across the load)' : 'Notch depth a'} value={p.a_mm} min={0.1} max={p.width_mm * 0.45} step={0.05} unit="mm" onChange={(v) => set({ a_mm: v })} />}
          {p.geometry === 'ellipse' && <Slider label="Semi-axis b (along the load)" value={p.b_mm} min={0.05} max={5} step={0.05} unit="mm" onChange={(v) => set({ b_mm: v })} />}
          {p.geometry === 'notch' && <Slider label="Root radius ρ" value={p.radius_mm} min={0.02} max={3} log unit="mm" onChange={(v) => set({ radius_mm: v })} />}
          <Slider label="Thickness" value={p.thickness_mm} min={0.5} max={10} step={0.1} unit="mm" onChange={(v) => set({ thickness_mm: v })} />
          <Slider label="Tensile force" value={p.force_n} min={1} max={2000} log unit="N" onChange={(v) => set({ force_n: v })} />
          <Slider label="Notch sensitivity q" value={p.q} min={0} max={1} step={0.05} onChange={(v) => set({ q: v })} />
          <Select label="Material" value={p.material} options={matOpts} onChange={(v) => set({ material: v })} />
        </>
      }
      diagram={
        r && (
          <svg className="st-diagram" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Plate with a stress raiser and the stress rise at its edge">
            <rect className="part" x={cx - 60} y={cy - (p.width_mm / 2) * k} width={120} height={p.width_mm * k} />
            {p.geometry === 'hole' && <circle cx={cx} cy={cy} r={(p.hole_mm / 2) * k} fill="var(--surface-2)" stroke="var(--series-1)" strokeWidth={1.5} />}
            {p.geometry === 'ellipse' && <ellipse cx={cx} cy={cy} rx={p.b_mm * k} ry={p.a_mm * k} fill="var(--surface-2)" stroke="var(--series-1)" strokeWidth={1.5} />}
            {p.geometry === 'notch' && (
              <path
                d={`M ${cx - 6 - p.radius_mm * k} ${cy - (p.width_mm / 2) * k} L ${cx - p.radius_mm * k} ${cy - (p.width_mm / 2) * k + p.a_mm * k - p.radius_mm * k} A ${p.radius_mm * k} ${p.radius_mm * k} 0 0 0 ${cx + p.radius_mm * k} ${cy - (p.width_mm / 2) * k + p.a_mm * k - p.radius_mm * k} L ${cx + 6 + p.radius_mm * k} ${cy - (p.width_mm / 2) * k} Z`}
                fill="var(--surface-2)"
                stroke="var(--series-1)"
                strokeWidth={1.5}
              />
            )}
            <Arrow x1={cx - 70} y1={cy} x2={cx - 100} y2={cy} />
            <Arrow x1={cx + 70} y1={cy} x2={cx + 100} y2={cy} />
            <text x={cx} y={H - 8} textAnchor="middle">σ_peak = K_t σ_nom = {fmt(r.peak_mpa, 1)} MPa</text>
            <g transform="translate(270 30)">
              <text x={0} y={0} className="lbl">σ across the section</text>
              {(() => {
                const h = 120;
                const w = 90;
                const pts = Array.from({ length: 21 }, (_, i) => {
                  const s = i / 20;
                  // Schematic decay from the peak at the edge of the raiser to ~nominal (Kirsch-like 1/r² falloff).
                  const v = 1 + (r.kt - 1) * Math.pow(1 / (1 + 4 * s), 2);
                  return `${(v / r.kt) * w},${12 + s * h}`;
                });
                return (
                  <g>
                    <line className="dim" x1={w / r.kt} x2={w / r.kt} y1={12} y2={12 + h} />
                    <polyline points={pts.join(' ')} fill="none" stroke="var(--series-2)" strokeWidth={2} />
                    <text x={w / r.kt + 3} y={12 + h}>nominal</text>
                  </g>
                );
              })()}
            </g>
          </svg>
        )
      }
      results={
        r && (
          <div className="stack" style={{ gap: 10 }}>
            <div className="grid grid-4">
              <Stat label="K_t" value={fmt(r.kt, 2)} note={r.basis} />
              <Stat label="K_f (fatigue)" value={fmt(r.kf, 2)} note={`q = ${p.q}`} />
              <Stat label="Safety factor on nominal" value={fmt(r.sf_nominal, 2)} unit="×" tone={sfTone(r.sf_nominal)} note={`σ_nom ${fmt(r.nominal_mpa, 1)} MPa — static, ductile`} />
              <Stat label="Safety factor on peak" value={fmt(r.sf_peak, 2)} unit="×" tone={sfTone(r.sf_peak, 1)} note="brittle / first yield" />
            </div>
            <Plot series={[{ name: 'K_t', x: r.curve_x, y: r.curve_kt }]} xLabel={r.curve_label} yLabel="K_t" height={170} vLines={[{ value: x, label: 'now' }]} yZero />
            <ul className="small muted" style={{ margin: 0, paddingLeft: 18 }}>
              {r.notes.map((n, i) => (
                <li key={i}>{n}</li>
              ))}
            </ul>
          </div>
        )
      }
      extra={
        <Explainer title="Where K_t comes from" refs={[{ label: 'W. D. Pilkey & D. F. Pilkey, "Peterson’s Stress Concentration Factors", 3rd ed., Wiley, 2008' }, { label: 'C. E. Inglis, "Stresses in a plate due to the presence of cracks and sharp corners", Trans. Inst. Naval Architects 55, 1913' }]}>
          <Formula>{`hole in a finite plate (net section):  K_tn ≈ 2 + (1 − d/W)³   (Heywood)
elliptical hole:  K_t = 1 + 2 a / b        notch as an ellipse:  K_t ≈ 1 + 2 √(a / ρ)
fatigue:  K_f = 1 + q (K_t − 1)`}</Formula>
          <p className="small">This is why the Strength tab reports a "corner peak" beside the design stress: the sharp step where the Iris temple leaves the pod is a notch with ρ → 0.</p>
        </Explainer>
      }
    />
  );
}

// ------------------------------------------------------------------ living hinge

function HingeCalc({ matOpts }: { matOpts: MatOpts }) {
  const [p, setP] = useState({ thickness_mm: 0.35, length_mm: 1.5, angle_deg: 180, material: 'PP' });
  const set = (q: Partial<typeof p>) => setP((o) => ({ ...o, ...q }));
  const { data: r, error } = useCompute<HingeOut>('structures.living_hinge', p);
  const [t, playing, setPlaying] = usePlay(3);
  const ang = ((0.5 - 0.5 * Math.cos(2 * Math.PI * t)) * p.angle_deg * Math.PI) / 180;
  const W = 380;
  const H = 190;
  const ox = 150;
  const oy = 130;
  const Lh = 30;
  // Arc of the hinge: radius R = L / θ (px); leaf rotates about the hinge.
  const R = ang > 1e-3 ? Lh / ang : 1e6;
  const ex = ang > 1e-3 ? ox + R * Math.sin(ang) : ox + Lh;
  const ey = ang > 1e-3 ? oy - R * (1 - Math.cos(ang)) : oy;
  const leaf = 110;
  const lx = ex + leaf * Math.cos(ang);
  const ly = ey - leaf * Math.sin(ang);
  const tone: 'pass' | 'fail' = r && r.strain_pct <= r.guidance_limit_pct ? 'pass' : 'fail';
  return (
    <Shell
      error={error}
      title="Living hinge"
      subtitle="A thin web bends into an arc of radius L/θ — the outer-fibre strain must stay within what the material tolerates"
      inputs={
        <>
          <Select label="Material" value={p.material} options={matOpts} onChange={(v) => set({ material: v })} />
          <Slider label="Web thickness t" value={p.thickness_mm} min={0.1} max={1.2} step={0.01} unit="mm" onChange={(v) => set({ thickness_mm: v })} />
          <Slider label="Web length L" value={p.length_mm} min={0.3} max={6} step={0.05} unit="mm" onChange={(v) => set({ length_mm: v })} />
          <Slider label="Fold angle θ" value={p.angle_deg} min={10} max={180} step={5} unit="°" onChange={(v) => set({ angle_deg: v })} />
        </>
      }
      diagram={
        r && (
          <>
            <svg className="st-diagram" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Living hinge folding">
              <rect className="part" x={ox - 120} y={oy - 4} width={120} height={14} />
              {ang > 1e-3 ? (
                <path d={`M ${ox} ${oy} A ${R} ${R} 0 0 0 ${ex} ${ey}`} fill="none" stroke="var(--series-2)" strokeWidth={4} />
              ) : (
                <line x1={ox} y1={oy} x2={ox + Lh} y2={oy} stroke="var(--series-2)" strokeWidth={4} />
              )}
              <line x1={ex} y1={ey} x2={lx} y2={ly} stroke="var(--series-1)" strokeWidth={14} />
              <text x={20} y={24}>ε = t θ / (2 L) = {fmt((r.strain_pct * ang) / Math.max(1e-6, (p.angle_deg * Math.PI) / 180), 1)} %</text>
            </svg>
            <div className="cp-toolbar">
              <PlayButton playing={playing} setPlaying={setPlaying} />
            </div>
          </>
        )
      }
      results={
        r && (
          <div className="stack" style={{ gap: 10 }}>
            <div className="grid grid-4">
              <Stat label="Outer-fibre strain" value={fmt(r.strain_pct, 1)} unit="%" tone={tone} note={`guidance ≈ ${fmt(r.guidance_limit_pct, 0)} %`} />
              <Stat label="Bend radius" value={fmt(r.bend_radius_mm, 2)} unit="mm" />
              <Stat label="Shortest web for the guidance" value={fmt(r.min_length_mm, 2)} unit="mm" />
              <Stat label="Fold angle" value={fmt(p.angle_deg, 0)} unit="°" />
            </div>
            <Verdict tone={tone}>{r.verdict}</Verdict>
          </div>
        )
      }
      extra={
        <Explainer title="Living-hinge strain" refs={[{ label: 'R. A. Malloy, "Plastic Part Design for Injection Molding", Hanser — living hinges' }]}>
          <Formula>{`R = L / θ          ε = t / (2 R) = t θ / (2 L)`}</Formula>
          <p className="small">Printed PETG or PLA hinges crack within a few folds; molded polypropylene survives very many because the flow orients the molecules across the hinge. For Iris, a TPU flap over the USB-C slot is the printable alternative.</p>
        </Explainer>
      }
    />
  );
}
