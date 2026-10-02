// Convection calculator: natural (Churchill–Chu plate and cylinder, horizontal
// plates), forced (Churchill–Bernstein, laminar flat plate), radiation; every
// Nusselt number turned into h and plotted against the surface excess.
import { useMemo, useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { ConvOut } from './types';
import { fmt, toCsv } from './util';

function PlumeDiagram({ dt, speed }: { dt: number; speed: number }) {
  // Boundary layer of a heated vertical plate: δ(x) ∝ x^(1/4) (laminar), thinned by a cross wind.
  const H = 180;
  const pts = Array.from({ length: 31 }, (_, i) => {
    const x = (i / 30) * H;
    const d = 6 + 26 * Math.pow(x / H, 0.25) * Math.pow(Math.max(dt, 0.5) / 10, -0.25) / (1 + speed * 0.6);
    return [40 + d, 200 - x];
  });
  const path = `M40,200 ${pts.map((p) => `L${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(' ')} L40,20`;
  const streaks = [0, 1, 2, 3];
  return (
    <svg viewBox="0 0 320 220" className="fl-diagram" role="img" aria-label="natural convection boundary layer on a heated plate and a cross-flow over the temple">
      <defs>
        <linearGradient id="fl-bl" x1="0" x2="1">
          <stop offset="0" stopColor="var(--series-2)" stopOpacity="0.45" />
          <stop offset="1" stopColor="var(--series-2)" stopOpacity="0" />
        </linearGradient>
      </defs>
      <path d={path} fill="url(#fl-bl)" stroke="var(--series-2)" strokeDasharray="4 3" />
      <rect x="30" y="20" width="10" height="180" rx="2" fill="var(--surface-3)" stroke="var(--text-2)" />
      {streaks.map((k) => (
        <line key={k} x1={46 + k * 5} y1="195" x2={46 + k * 5} y2="30" className="fl-rise" style={{ animationDelay: `${-k * 0.4}s`, animationDuration: `${2.6 / Math.sqrt(Math.max(dt, 1) / 10)}s` }} />
      ))}
      <text x="18" y="214" className="lbl">heated wall</text>
      <text x="80" y="60">boundary layer δ ∝ x^¼</text>
      <text x="80" y="76">warm air rises</text>
      {/* temple cross-section in a cross wind */}
      <rect x="226" y="96" width="18" height="36" rx="5" fill="var(--surface-3)" stroke="var(--text-2)" />
      {[0, 1, 2, 3, 4].map((k) => (
        <path key={k} d={`M170,${80 + k * 17} C 210,${80 + k * 17} 210,${k < 2 ? 70 + k * 10 : k > 2 ? 150 + (k - 3) * 10 : 114} 235,${k < 2 ? 70 + k * 10 : k > 2 ? 150 + (k - 3) * 10 : 114} S 270,${80 + k * 17} 310,${80 + k * 17}`} className="fl-stream" style={{ opacity: speed > 0 ? 0.9 : 0.25 }} />
      ))}
      <text x="312" y="196" textAnchor="end">wind {speed.toFixed(1)} m/s · Re = V D / ν</text>
    </svg>
  );
}

export function ConvectionTab() {
  const project = useProject((s) => s.project);
  const f = project.design.frame;
  const [ts, setTs] = useState(project.design.thermal.skin_limit_c);
  const [ta, setTa] = useState(project.design.thermal.ambient_c);
  const [eps, setEps] = useState(0.9);
  const [lv, setLv] = useState(f.pod_height_mm);
  const [d, setD] = useState(+((2 * f.temple_width_mm * f.temple_height_mm) / (f.temple_width_mm + f.temple_height_mm)).toFixed(2));
  const [lf, setLf] = useState(f.pod_length_mm);
  const [v, setV] = useState(1.4);
  const lap = (f.pod_length_mm * f.pod_width_mm) / (2 * (f.pod_length_mm + f.pod_width_mm));
  const input = useMemo(
    () => ({ project, t_surface_c: ts, t_ambient_c: ta, emissivity: eps, plate_height_mm: lv, plate_ap_mm: lap, diameter_mm: d, flow_length_mm: lf, speed_m_s: v, dt_max: 30 }),
    [project, ts, ta, eps, lv, lap, d, lf, v],
  );
  const res = useCompute<ConvOut>('fluids.convection', input);
  const r = res.data;
  const shown = ['vplate', 'hcyl', 'hplate_up', 'hplate_down', 'cb', 'fplate'];
  return (
    <div className="stack">
      <div className="fl-calc">
        <Card title="Inputs" subtitle="Defaults: the Iris pod and temple">
          <Slider label="Surface temperature" value={ts} min={ta + 0.5} max={ta + 40} step={0.1} unit="°C" onChange={setTs} />
          <Slider label="Ambient air" value={ta} min={0} max={45} step={0.5} unit="°C" onChange={setTa} />
          <Slider label="Emissivity ε" value={eps} min={0.02} max={1} step={0.01} onChange={setEps} />
          <Slider label="Vertical plate height (pod)" value={lv} min={2} max={300} log unit="mm" format={(x) => x.toFixed(1)} onChange={setLv} />
          <Slider label="Cylinder diameter (temple)" value={d} min={1} max={100} log unit="mm" format={(x) => x.toFixed(2)} onChange={setD} />
          <Slider label="Length along the flow (pod)" value={lf} min={5} max={300} log unit="mm" format={(x) => x.toFixed(0)} onChange={setLf} />
          <Slider label="Air speed" value={v} min={0} max={10} step={0.1} unit="m/s" onChange={setV} hint="walking ≈ 1.4 m/s, cycling ≈ 5 m/s" />
          <div className="row">
            <button className="btn small" onClick={() => { setLv(f.pod_height_mm); setD(+((2 * f.temple_width_mm * f.temple_height_mm) / (f.temple_width_mm + f.temple_height_mm)).toFixed(2)); setLf(f.pod_length_mm); }}>Iris dimensions</button>
            <button className="btn small" onClick={() => { setLv(300); setD(50); setLf(300); }}>Textbook scale (0.3 m)</button>
          </div>
        </Card>
        <div className="stack" style={{ minWidth: 0 }}>
          <ErrorBox error={res.error} />
          {r && (
            <>
              <div className="grid grid-4">
                <Stat label="Still air, pod side" value={fmt(r.h_still_total, 2)} unit="W/m² K" note={`convection ${fmt(r.rows[0].h, 2)} + radiation ${fmt(r.h_rad, 2)}`} />
                <Stat label={`Walking ${r.speed_m_s} m/s`} value={fmt(r.h_walking_total, 2)} unit="W/m² K" note="Nu³ = Nu_N³ + Nu_F³, + radiation" />
                <Stat label="Heat flux, still air" value={fmt(r.q_still_w_m2, 1)} unit="W/m²" note={`= ${fmt(r.q_still_w_m2 * 0.1, 2)} mW/cm²`} />
                <Stat label="Radiation share" value={fmt((r.h_rad / r.h_still_total) * 100, 0)} unit="%" note="of still-air cooling" />
              </div>
              <Card title="Correlations at this state" subtitle={`Film temperature ${fmt(r.t_film_c, 1)} °C: k = ${(r.film.k * 1e3).toFixed(2)} mW/m K, ν = ${(r.film.nu * 1e6).toFixed(2)} mm²/s, Pr = ${r.film.pr.toFixed(3)}, β = 1/T = ${(r.film.beta * 1e3).toFixed(3)} /kK`}>
                <div className="table-wrap">
                  <table className="table">
                    <thead><tr><th>Geometry</th><th>L</th><th>Group</th><th>Nu</th><th>h, W/m² K</th><th>Range</th></tr></thead>
                    <tbody>
                      {r.rows.map((row) => (
                        <tr key={row.id}>
                          <td>{row.name}<div className="small muted mono">{row.equation}</div></td>
                          <td className="tnum">{fmt(row.length_mm, 1)} mm</td>
                          <td className="tnum">{row.group_name} = {row.group < 0.01 ? row.group.toExponential(1) : row.group >= 1e5 ? row.group.toExponential(2) : fmt(row.group, 1)}</td>
                          <td className="tnum">{fmt(row.nu, 2)}</td>
                          <td className="tnum"><b>{fmt(row.h, 2)}</b></td>
                          <td>{row.valid ? <Chip tone="pass">✓ {row.range}</Chip> : <Chip tone="warn" title="outside the validated range: extrapolated">! {row.range}</Chip>}</td>
                        </tr>
                      ))}
                      <tr>
                        <td>Radiation to surroundings at {r.t_ambient_c} °C<div className="small muted mono">h_r = ε σ (T_s + T_sur)(T_s² + T_sur²)</div></td>
                        <td /><td /><td /><td className="tnum"><b>{fmt(r.h_rad, 2)}</b></td><td><Chip>ε = {r.emissivity}</Chip></td>
                      </tr>
                    </tbody>
                  </table>
                </div>
                {r.notes.length > 0 && <ul className="small muted" style={{ paddingLeft: 18 }}>{r.notes.map((n, i) => <li key={i}>{n}</li>)}</ul>}
              </Card>
            </>
          )}
        </div>
      </div>
      {r && (
        <div className="grid grid-2">
          <Card title="h against the surface excess ΔT = T_s − T_∞" subtitle="Natural convection grows like ΔT^¼ (laminar); forced convection does not depend on ΔT">
            <Plot
              series={[
                ...r.curves.filter((c) => shown.includes(c.id)).map((c) => ({ name: c.name, x: r.dt, y: c.h })),
                { name: 'radiation', x: r.dt, y: r.h_rad_curve, dash: '5 4' },
              ]}
              xLabel="ΔT, K"
              yLabel="h, W/m² K"
              height={300}
              legend
              yZero
              vLines={[{ value: r.t_surface_c - r.t_ambient_c, label: 'now' }]}
            />
            <div className="row">
              <DownloadButton
                filename="convection-correlations.csv"
                small
                content={() => toCsv([['dT_K', ...r.curves.map((c) => c.id), 'h_rad'], ...r.dt.map((x, i) => [x, ...r.curves.map((c) => +c.h[i].toFixed(4)), +r.h_rad_curve[i].toFixed(4)])])}
              />
            </div>
          </Card>
          <Card title="What the air is doing">
            <PlumeDiagram dt={r.t_surface_c - r.t_ambient_c} speed={r.speed_m_s} />
            <p className="small muted">
              A small body has a thin boundary layer, so h is larger than on a big panel at the same ΔT — the Iris pod (L ≈ {f.pod_height_mm} mm) sees roughly twice the textbook 0.3 m value. Walking adds a
              forced component that the mixed-convection blend combines with buoyancy.
            </p>
          </Card>
        </div>
      )}
      <Explainer
        title="From Nusselt number to h"
        refs={[
          { label: 'Churchill & Chu, Int. J. Heat Mass Transfer 18 (1975)' },
          { label: 'Churchill & Bernstein, J. Heat Transfer 99 (1977)' },
          { label: 'Incropera et al., ch. 7 and 9 (2011)' },
          { label: 'Holman, Heat Transfer, 10th ed. (simplified air equations)' },
        ]}
      >
        <p>Convection correlations are fits to experiments in dimensionless form. Natural convection depends on the Rayleigh number, forced convection on the Reynolds number:</p>
        <Formula>Ra_L = g β ΔT L³ / (ν α),  Re = V L / ν,  h = Nu · k / L</Formula>
        <Formula>Vertical plate: Nu = {'{'}0.825 + 0.387 Ra^(1/6) / [1 + (0.492/Pr)^(9/16)]^(8/27){'}'}²</Formula>
        <Formula>Cylinder in cross-flow: Nu = 0.3 + 0.62 Re^½ Pr^⅓ / [1 + (0.4/Pr)^⅔]^¼ · [1 + (Re/282 000)^⅝]^⅘</Formula>
        <p>
          Checks in the engine's tests: the full-range plate formula gives Nu = 122.8 at Ra = 10⁹ (hand calculation), sits within 12 % of Holman's simplified h = 1.42 (ΔT/L)^¼ for air, the cylinder within 12 % of
          Morgan's 0.48 Ra^¼, and Churchill–Bernstein within 10 % of Hilpert's 0.193 Re^0.618 Pr^⅓ at Re = 10⁴. Correlations disagree by 10–20 % among themselves — that is the honest uncertainty of any h.
        </p>
      </Explainer>
    </div>
  );
}
