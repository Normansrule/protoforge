// Heat-spreading trade: graphite sheet, copper foil, aluminium plate or none,
// at the pod or behind the display — each option a full 3-D solve.
import { useMemo, useState } from 'react';
import { Card, ErrorBox, Explainer, Formula, Select, Slider, Stat, DownloadButton } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useProject } from '../../state/project';
import { useCompute } from '../../engine/hooks';
import { configOf, type PodControls } from './controls';
import type { ThermalMaterial, TradeOut, TradeRow } from './types';
import { LOCATION_LABEL, fluidsDesign, fmt, toCsv, useWorkerCompute } from './util';

const SHEETS: Record<string, { k: number; label: string }> = {
  graphite: { k: 1500, label: 'graphite sheet (typ. 700–1900 W/m K in-plane)' },
  copper: { k: 390, label: 'copper foil (≈ 390 W/m K)' },
  aluminium: { k: 230, label: 'aluminium (≈ 230 W/m K)' },
};

function SpreadingDemo({ kWall, tWall }: { kWall: number; tWall: number }) {
  const [kind, setKind] = useState('graphite');
  const [um, setUm] = useState(25);
  const [h, setH] = useState(11);
  const g0 = kWall * tWall * 1e-3; // W/K per metre width (k·t)
  const gs = g0 + SHEETS[kind].k * um * 1e-6;
  const lam0 = Math.sqrt(g0 / h) * 1e3;
  const lam = Math.sqrt(gs / h) * 1e3;
  const xs = Array.from({ length: 121 }, (_, i) => -30 + i * 0.5);
  // line source of strength q' on a wall cooled on one side: θ(x) = q'/(2√(hG)) e^{−|x|/λ}
  const peak0 = 1 / (2 * Math.sqrt(h * g0));
  const peakS = 1 / (2 * Math.sqrt(h * gs));
  return (
    <div className="grid grid-2">
      <div>
        <Select label="Sheet" value={kind} options={Object.entries(SHEETS).map(([v, s]) => ({ value: v, label: s.label }))} onChange={setKind} />
        <Slider label="Sheet thickness" value={um} min={10} max={1000} log unit="µm" format={(v) => v.toFixed(0)} onChange={setUm} />
        <Slider label="Outside h (convection + radiation)" value={h} min={4} max={40} step={0.5} unit="W/m² K" onChange={setH} />
        <div className="grid grid-2">
          <Stat label="λ without" value={fmt(lam0, 1)} unit="mm" note={`k t = ${(g0 * 1e3).toFixed(2)} mW/K`} />
          <Stat label="λ with sheet" value={fmt(lam, 1)} unit="mm" note={`k t = ${(gs * 1e3).toFixed(1)} mW/K`} />
        </div>
        <p className="small muted">Peak rise falls by √(G₀/G) = {fmt(Math.sqrt(g0 / gs), 3)} — a {fmt((1 - Math.sqrt(g0 / gs)) * 100, 0)} % cut, until the sheet is wider than the part (then the area, not the sheet, limits).</p>
      </div>
      <Plot
        series={[
          { name: 'plastic wall only', x: xs, y: xs.map((x) => (peak0 * Math.exp(-Math.abs(x) / lam0)) / peak0) },
          { name: `with ${um.toFixed(0)} µm ${kind}`, x: xs, y: xs.map((x) => (peakS * Math.exp(-Math.abs(x) / lam)) / peak0) },
        ]}
        xLabel="distance from the heat source, mm"
        yLabel="θ / θ_peak (no sheet)"
        height={240}
        yDomain={[0, 1.05]}
        legend
      />
    </div>
  );
}

export function SpreaderTab({ ctl, setCtl }: { ctl: PodControls; setCtl: (f: (c: PodControls) => PodControls) => void }) {
  const project = useProject((s) => s.project);
  const patchDesign = useProject((s) => s.patchDesign);
  const input = useMemo(() => ({ project, config: { ...configOf(ctl), spreader: 'none', voxel_mm: Math.max(1.5, ctl.voxel_mm) }, thicknesses_um: [10, 17, 25, 40, 70, 100, 200, 500] }), [project, ctl]);
  const res = useWorkerCompute<TradeOut>('fluids.spreader_trade', input, 200);
  const r = res.data;
  const limit = project.design.thermal.skin_limit_c;
  const design = fluidsDesign(project);
  const mats = useCompute<{ shell: ThermalMaterial[] }>('fluids.materials', {}).data;
  const kWall = mats?.shell.find((m) => m.id.toLowerCase() === project.design.frame.material.toLowerCase())?.k_in ?? 0.2;
  const best = r ? r.rows.reduce((a, b) => (b.peak_skin_c < a.peak_skin_c ? b : a), r.rows[0]) : null;
  const apply = (row: TradeRow) => {
    const next = { ...design, spreader: row.spreader, spreader_um: row.thickness_um || design.spreader_um, spreader_location: row.location === '-' ? design.spreader_location : row.location };
    patchDesign('extra', { fluids: next });
    setCtl((c) => ({ ...c, spreader: next.spreader, spreader_um: next.spreader_um, spreader_location: next.spreader_location }));
  };
  const isCurrent = (row: TradeRow) =>
    row.spreader === design.spreader && (row.spreader === 'none' || (Math.abs(row.thickness_um - design.spreader_um) < 1e-9 && row.location === design.spreader_location));
  return (
    <div className="stack">
      <Card
        title="Heat-spreader trade"
        subtitle={r ? `Each row is a full 3-D steady solve (${r.voxel_mm} mm voxels, display-heat placement: ${ctl.display_heat_in === 'pod' ? 'pod board' : 'optics housing'}). Baseline hotspot: ${r.hotspot_location === 'optics' ? 'the optics housing behind the display' : 'the pod'}.` : 'solving every option…'}
        actions={res.busy ? <span className="cp-spin" aria-label="computing" /> : undefined}
      >
        <ErrorBox error={res.error} />
        {r && best && (
          <>
            <div className="grid grid-4" style={{ marginBottom: 12 }}>
              <Stat label="No spreader" value={fmt(r.baseline_peak_c, 1)} unit="°C" tone={r.baseline_peak_c > limit ? 'fail' : 'pass'} note={r.baseline_peak_c > limit ? `✕ above ${limit} °C` : `✓ under ${limit} °C`} />
              <Stat label="Best option" value={fmt(best.peak_skin_c, 1)} unit="°C" tone={best.peak_skin_c > limit ? 'fail' : 'pass'} note={best.label} />
              <Stat label="Gain" value={fmt(best.gain_c, 1)} unit="°C" note="peak reduction" />
              <Stat label="Added mass" value={fmt(best.added_mass_g, 3)} unit="g" note={`sheet ≈ ${(r.sheet_area_mm2 / 100).toFixed(1)} cm²`} />
            </div>
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr><th>Option</th><th>k along</th><th>k·t</th><th>Peak</th><th>Gain</th><th>Hotspot − mean</th><th>Board</th><th>Mass</th><th /></tr>
                </thead>
                <tbody>
                  {r.rows.map((row) => (
                    <tr key={row.id}>
                      <td>{row.label}</td>
                      <td className="tnum">{row.k_in ? `${row.k_in} W/m K` : '—'}</td>
                      <td className="tnum">{row.sheet_conductance_mw_k ? `${fmt(row.sheet_conductance_mw_k, 1)} mW/K` : '—'}</td>
                      <td className="tnum">
                        {fmt(row.peak_skin_c, 1)} °C {row.peak_skin_c > limit ? <span className="fl-bad">✕</span> : <span className="fl-good">✓</span>}
                      </td>
                      <td className="tnum">{row.gain_c > 0.05 ? `−${fmt(row.gain_c, 1)} °C` : '—'}</td>
                      <td className="tnum">{fmt(row.hotspot_over_mean_c, 1)} °C</td>
                      <td className="tnum">{fmt(row.board_max_c, 1)} °C</td>
                      <td className="tnum">{row.added_mass_g ? `${fmt(row.added_mass_g, 3)} g` : '—'}</td>
                      <td>
                        {isCurrent(row) ? <span className="small muted">✓ Iris</span> : <button className="btn small" onClick={() => apply(row)}>Use for Iris</button>}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <ul className="small muted" style={{ paddingLeft: 18 }}>
              {r.notes.map((n, i) => <li key={i}>{n}</li>)}
              <li>✓ / ✕: peak frame surface against the {limit} °C skin limit (R-008's limit). Mass assumes the sheet covers {LOCATION_LABEL[r.hotspot_location] ?? 'the wall'}.</li>
            </ul>
            <div className="row">
              <DownloadButton
                filename="iris-spreader-trade.csv"
                small
                content={() => toCsv([['option', 'spreader', 'thickness_um', 'location', 'k_in_W_mK', 'peak_c', 'mean_c', 'hotspot_minus_mean_c', 'board_max_c', 'gain_c', 'added_mass_g'], ...r.rows.map((x) => [x.label, x.spreader, x.thickness_um, x.location, x.k_in, +x.peak_skin_c.toFixed(3), +x.mean_skin_c.toFixed(3), +x.hotspot_over_mean_c.toFixed(3), +x.board_max_c.toFixed(3), +x.gain_c.toFixed(3), +x.added_mass_g.toFixed(4)])])}
              />
            </div>
          </>
        )}
      </Card>
      {r && (
        <Card title="Peak temperature vs sheet thickness" subtitle={`Sheet ${LOCATION_LABEL[r.hotspot_location] ?? ''} — diminishing returns once the sheet spreads heat over the whole wall`}>
          <Plot
            series={r.sweeps.map((s) => ({ name: `${s.spreader} (${s.spreader === 'graphite' ? 'typ. 1500' : '390'} W/m K)`, x: s.thickness_um, y: s.peak_skin_c }))}
            xLog
            xLabel="sheet thickness, µm"
            yLabel="peak frame surface, °C"
            height={260}
            legend
            hLines={[
              { value: r.baseline_peak_c, label: 'no spreader' },
              { value: limit, label: `${limit} °C limit`, color: 'var(--bad)' },
            ]}
          />
          <p className="small muted" style={{ marginBottom: 0 }}>
            Graphite sheets are typically 17–100 µm thick; per unit thickness they beat copper by about 4× along the sheet but conduct poorly across it (typically 5–20 W/m K), which is why they are bonded flat
            to the hot surface.
          </p>
        </Card>
      )}
      <Card title="Why a 25 µm sheet matters: the spreading length" subtitle="1-D fin model of a wall with a line heat source, cooled on the outside">
        <SpreadingDemo kWall={kWall} tWall={project.design.frame.wall_mm} />
        <Explainer title="Conductance, not conductivity" refs={[{ label: 'Incropera et al. (2011) §3.6, extended surfaces' }]}>
          <p>A thin wall of conductivity k and thickness t, losing heat h per unit area, behaves like a fin. Away from a line source of strength q′ the excess temperature decays as</p>
          <Formula>θ(x) = q′ / (2√(h G)) · e<sup>−|x|/λ</sup>,  λ = √(G / h),  G = k t + k<sub>s</sub> t<sub>s</sub></Formula>
          <p>
            Only the product k·t (the sheet's conductance) enters, so a 25 µm graphite sheet at 1500 W/m K (37 mW/K) outweighs a 1.2 mm plastic wall (0.24 mW/K) 150 times over. That is also why the
            3-D model can represent a sheet much thinner than a voxel: it keeps k·t exact.
          </p>
        </Explainer>
      </Card>
      {ctl.spreader === 'none' && design.spreader === 'none' && r && r.hotspot_location === 'optics' && (
        <p className="small muted">
          Tip: the Iris hotspot is on the optics housing, so sheets inside the pod cannot help it — the “behind the display” rows can.
        </p>
      )}
    </div>
  );
}
