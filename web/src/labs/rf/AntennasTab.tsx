// Antennas: first-cut resonant lengths, analytic dipole/array patterns on
// polar plots and in 3-D, directivity, and the Iris efficiency chain.
import { useMemo, useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat } from '../../components/ui';
import { Scene3D, EngineMesh, Segments } from '../../components/Scene3D';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { ElementKind, IrisOut, PatternOut, SizeOut } from './types';
import { fmt, prefersReducedMotion, seqBlue, toCsv, useIrisRf, useRaf, useTheme } from './util';
import { PolarPlot } from './viz';
import { REFS } from './refs';

const ELEMENTS: { value: ElementKind; label: string }[] = [
  { value: 'half_wave', label: 'Half-wave dipole' },
  { value: 'short', label: 'Short (Hertzian) dipole' },
  { value: 'dipole', label: 'Dipole of any length' },
  { value: 'isotropic', label: 'Isotropic point source' },
];

export function AntennasTab() {
  const project = useProject((s) => s.project);
  const [rf, patchRf] = useIrisRf();
  const [f, setF] = useState(2.44);
  const [er, setEr] = useState(4.4);
  const sizes = useCompute<SizeOut>('rf.antenna_sizes', { f_ghz: f, er, ground_len_mm: project.design.pcb.width_mm, ground_w_mm: project.design.pcb.height_mm, chip_len_mm: 3.2 });
  const iris = useCompute<IrisOut>('rf.iris', { project });
  const [el, setEl] = useState<ElementKind>('half_wave');
  const [len, setLen] = useState(1.25);
  const [n, setN] = useState(4);
  const [d, setD] = useState(0.5);
  const [beta, setBeta] = useState(0);
  const [sweep, setSweep] = useState(false);
  const [bodyLoss, setBodyLoss] = useState(rf.link.head_loss_db);
  const mismatch = iris.data ? iris.data.match_loss_db : 0;
  const input = useMemo(
    () => ({ element: el, length_wl: len, n, spacing_wl: d, phase_deg: Math.round(beta), rad_efficiency: rf.rad_efficiency, mismatch_db: mismatch, body_loss_db: bodyLoss, mesh_step_deg: 5, mesh_range_db: 30 }),
    [el, len, n, d, beta, rf.rad_efficiency, mismatch, bodyLoss],
  );
  const pat = useCompute<PatternOut>('rf.pattern', input);
  useRaf(sweep, (dt) => setBeta((b) => (b + dt * 60 > 180 ? -180 : b + dt * 60)));
  const p = pat.data;
  const s = sizes.data;
  return (
    <div className="stack">
      <div className="grid grid-2">
        <Card title="How long should it be?" subtitle={s ? `λ₀ = ${fmt(s.lambda0_mm, 1)} mm at ${fmt(f, 3)} GHz` : ''}>
          <ErrorBox error={sizes.error} />
          <div className="grid grid-2" style={{ gap: 10 }}>
            <Slider label="Frequency" value={f} min={0.8} max={6} step={0.01} unit="GHz" onChange={setF} />
            <Slider label="Board εr (FR-4 ≈ 4.3–4.5)" value={er} min={1} max={10} step={0.05} onChange={setEr} />
          </div>
          {s && (
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>Antenna</th><th>Length</th><th>εeff</th></tr></thead>
                <tbody>
                  {s.rows.map((r) => (
                    <tr key={r.name} title={r.note}>
                      <td>{r.name}<div className="muted small">{r.note}</div></td>
                      <td className="mono tnum">{fmt(r.length_mm, 1)} mm</td>
                      <td className="mono tnum">{fmt(r.eeff, 2)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Card>
        <Card title="The Iris chip antenna" subtitle="3216 ceramic chip (AE1) at the end of a 58 × 10 mm, 0.8 mm board in the right pod">
          {s && (
            <div className="grid grid-2" style={{ gap: 10, marginBottom: 10 }}>
              <Stat label="Ground length" value={fmt(s.ground_len_wl, 2)} unit="λ" note={`${project.design.pcb.width_mm} mm board: the other half of the antenna`} />
              <Stat label="Chu–McLean Q (chip alone)" value={fmt(s.chu_q_chip, 0)} note={`ka = ${fmt(s.ka_chip, 3)} → bandwidth ≈ ${fmt(100 / s.chu_q_chip, 2)} %`} tone="warn" />
              <Stat label="Chu–McLean Q (whole board)" value={fmt(s.chu_q_board, 2)} note={`ka = ${fmt(s.ka_board, 2)} → up to ~${fmt(Math.min(100, s.chu_bw_board_pct), 0)} % bandwidth`} tone="pass" />
              <Stat label="BLE band needs" value="3.4" unit="%" note="2.400–2.4835 GHz / 2.442 GHz" />
            </div>
          )}
          {iris.data && (
            <div className="rf-chain">
              <EffRow label="Radiation efficiency (assumed)" db={10 * Math.log10(rf.rad_efficiency)} />
              <EffRow label={`Match: ${iris.data.current_network.length ? iris.data.current_labels.join(' + ') : 'none on Iris yet'}`} db={-iris.data.match_loss_db} />
              <EffRow label="Head loss (assumed, see Fields → head)" db={-bodyLoss} />
              <EffRow label="Total" db={10 * Math.log10(rf.rad_efficiency) - iris.data.match_loss_db - bodyLoss} total />
            </div>
          )}
          <Slider label="Radiation efficiency (chip on small ground: typically 30–70 %)" value={rf.rad_efficiency * 100} min={10} max={100} step={1} unit="%" onChange={(v) => patchRf({ rad_efficiency: v / 100 })} />
          <Slider label="Head loss" value={bodyLoss} min={0} max={20} step={0.5} unit="dB" onChange={setBodyLoss} />
          {s && <div className="small muted">{s.notes[0]}</div>}
        </Card>
      </div>
      <div className="rf-split">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card title="Radiation pattern" subtitle="Element pattern × array factor (pattern multiplication), normalised to the peak">
            <ErrorBox error={pat.error} />
            {p && (
              <div className="grid grid-2">
                <PolarPlot title="E-plane (xz): top = +z (dipole axis), right = +x" series={[{ name: 'E-plane', angles: p.cut_angle_deg, db: p.e_plane_db }]} />
                <PolarPlot title="H-plane (xy): right = +x (array axis), top = +y" series={[{ name: 'H-plane', angles: p.cut_angle_deg.map((a) => (90 - a + 360) % 360), db: p.h_plane_db, color: 'var(--series-2)' }]} />
              </div>
            )}
          </Card>
          {p && <Pattern3D p={p} n={n} d={d} />}
        </div>
        <div className="stack rf-sticky">
          <Card title="Element & array">
            <Select label="Element" value={el} options={ELEMENTS} onChange={setEl} />
            {el === 'dipole' && <Slider label="Dipole length L" value={len} min={0.05} max={2} step={0.01} unit="λ" onChange={setLen} />}
            <Slider label="Elements N (along x)" value={n} min={1} max={16} step={1} onChange={setN} />
            {n > 1 && (
              <>
                <Slider label="Spacing d" value={d} min={0.05} max={1.5} step={0.01} unit="λ" onChange={setD} />
                <Slider label="Phase step β" value={beta} min={-180} max={180} step={1} unit="°" onChange={(v) => { setSweep(false); setBeta(v); }} />
                <button className="btn small" onClick={() => setSweep((x) => !x)} aria-pressed={sweep} disabled={prefersReducedMotion() && !sweep} style={{ marginBottom: 10 }}>
                  {sweep ? '❚❚ Stop steering' : '▶ Sweep β (beam steering)'}
                </button>
                <div className="small muted">Main beam where ψ = kd·cosγ + β = 0 → cosγ = −β/(kd). Spacing above λ brings grating lobes.</div>
              </>
            )}
          </Card>
          {p && (
            <div className="grid grid-2" style={{ gap: 10 }}>
              <Stat label="Directivity" value={fmt(p.directivity_dbi, 2)} unit="dBi" note={`${fmt(p.directivity, 2)} × isotropic`} />
              <Stat label="Realised gain" value={fmt(p.realized_gain_dbi, 1)} unit="dBi" note={`total efficiency ${fmt(p.total_efficiency_pct, 0)} %`} />
              <Stat label="HPBW E-plane" value={fmt(p.hpbw_e_deg, 0)} unit="°" />
              <Stat label="HPBW H-plane" value={p.hpbw_h_deg >= 359 ? 'omni' : fmt(p.hpbw_h_deg, 0)} unit={p.hpbw_h_deg >= 359 ? '' : '°'} />
            </div>
          )}
          {p && (
            <DownloadButton
              small
              filename={`pattern-${el}-N${n}.csv`}
              content={() => toCsv([['angle_deg', 'e_plane_db', 'h_plane_db'], ...p.cut_angle_deg.map((a, k) => [a, p.e_plane_db[k], p.h_plane_db[k]])])}
              label="pattern cuts CSV"
            />
          )}
        </div>
      </div>
      <div className="grid grid-2">
        <Explainer title="Dipole patterns and directivity" refs={[REFS.balanis]}>
          A thin dipole of length L carries a standing-wave current; its far field is
          <Formula>{'E(θ) ∝ [cos(kL/2·cosθ) − cos(kL/2)] / sinθ'}</Formula>
          which becomes sinθ for a short dipole (D = 1.5, 1.76 dBi) and cos(π/2·cosθ)/sinθ for a half-wave dipole (D = 1.64, 2.15 dBi, 78° beamwidth). Directivity D = 4π·U_max/∮U dΩ is integrated numerically here, so the same code checks arrays and odd lengths.
        </Explainer>
        <Explainer title="Arrays, efficiency and why small antennas are hard" refs={[REFS.balanis, REFS.chu, REFS.mclean, REFS.ti]}>
          N elements spaced d with a phase step β multiply the element pattern by
          <Formula>{'AF = sin(Nψ/2) / (N·sin(ψ/2)),   ψ = kd·cosγ + β'}</Formula>
          Realised gain = D + 10·log₁₀(η_rad · (1 − |Γ|²) · η_body). The Chu–McLean limit Q ≥ 1/(ka)³ + 1/(ka) says an antenna inside a sphere of radius a cannot be both tiny and broadband — the 3.2 mm chip only works because the whole board radiates with it.
        </Explainer>
      </div>
    </div>
  );
}

function EffRow({ label, db, total }: { label: string; db: number; total?: boolean }) {
  const pct = 100 * 10 ** (db / 10);
  return (
    <div className={`rf-chain-row ${total ? 'total' : ''}`}>
      <span className="small">{label}</span>
      <div className="rf-meter" aria-hidden><div style={{ width: `${Math.max(1, Math.min(100, pct))}%` }} /></div>
      <span className="mono small tnum">{db >= 0 ? '+' : ''}{fmt(db, 1)} dB · {fmt(pct, 0)} %</span>
    </div>
  );
}

function Pattern3D({ p, n, d }: { p: PatternOut; n: number; d: number }) {
  const light = useTheme();
  const mesh = useMemo(() => {
    const colors: number[] = [];
    for (const t of p.mesh_level) colors.push(...seqBlue(t, light));
    return { positions: p.mesh_positions, indices: p.mesh_indices, colors };
  }, [p, light]);
  const axis = useMemo(() => {
    const pts: number[] = [];
    for (let k = 0; k < n; k++) {
      const x = (k - (n - 1) / 2) * d * 20;
      pts.push(x, 0, -8, x, 0, 8);
    }
    pts.push(-85, 0, 0, 85, 0, 0);
    return pts;
  }, [n, d]);
  return (
    <Card title="3-D pattern" subtitle="Radius ∝ gain in dB (30 dB range); drag to orbit · dipoles along z, array along x">
      <Scene3D height={340} camera={[150, 110, 150]} grid={false}>
        <EngineMesh mesh={mesh} opacity={0.92} />
        <Segments points={axis} color={light ? '#3a3a40' : '#e0e0e0'} />
      </Scene3D>
    </Card>
  );
}
