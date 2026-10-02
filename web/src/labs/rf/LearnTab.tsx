// How it works: the ideas behind each tab, an animated standing-wave demo,
// the Iris RF chain, tools to graduate to and references.
import { useMemo, useState } from 'react';
import { Card, Explainer, Formula, Slider } from '../../components/ui';
import { useElementWidth } from '../../components/useElementWidth';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { IrisOut } from './types';
import { fmt, prefersReducedMotion, useRaf } from './util';
import { REFS } from './refs';

export function LearnTab() {
  const project = useProject((s) => s.project);
  const iris = useCompute<IrisOut>('rf.iris', { project });
  return (
    <div className="stack">
      <Card title="From Maxwell’s equations to a working Bluetooth link" subtitle="What each tab computes, in the order a radio designer works">
        <ol className="rf-steps">
          <li><b>Fields</b> — solve Maxwell’s equations on a grid to see how waves leave an antenna, bend round a head and reflect from metal (finite-difference time-domain, FDTD).</li>
          <li><b>Antennas</b> — size the radiator for 2.4 GHz and predict its pattern, directivity and efficiency (analytic patterns, array factor, Chu limit).</li>
          <li><b>Matching</b> — transform the antenna’s awkward impedance to the radio’s 50 Ω with two or three tiny inductors and capacitors (Smith chart, L/pi networks).</li>
          <li><b>Transmission lines</b> — draw the copper between radio, network and antenna as a 50 Ω line on the real board (microstrip, CPWG).</li>
          <li><b>Link budget</b> — add every gain and loss from the glasses to a phone and check there is margin left at the distance users care about.</li>
        </ol>
      </Card>
      <StandingWave />
      <div className="grid grid-2">
        <Explainer title="Maxwell’s equations in 2-D (TMz)" refs={[REFS.taflove, REFS.yee]}>
          With nothing varying along z, Maxwell’s curl equations split into two independent sets. The transverse-magnetic set used in the Fields tab is
          <Formula>{'μ ∂H_x/∂t = −∂E_z/∂y     μ ∂H_y/∂t = ∂E_z/∂x     ε ∂E_z/∂t = ∂H_y/∂x − ∂H_x/∂y − σE_z'}</Formula>
          Central differences on the staggered Yee grid are second-order accurate in space and time. Rule of thumb: at least 10–20 cells per wavelength in the densest material, and Δt at or below the Courant limit.
        </Explainer>
        <Explainer title="Tissue at 2.4 GHz" refs={[REFS.ieee1528, REFS.gabriel]}>
          Body tissue is mostly salty water: high permittivity and conductivity. Standard head-simulating liquid targets εr = 39.2 and σ = 1.80 S/m at 2450 MHz; muscle is around εr ≈ 53, σ ≈ 1.7 S/m (Gabriel 1996). The wavelength shrinks by √εr ≈ 6 inside and the field decays within a few centimetres, so a head next to the antenna both absorbs power and casts a radio shadow — the head loss in the link budget.
        </Explainer>
        <Explainer title="Why a matching network" refs={[REFS.pozar, REFS.ti]}>
          The radio is designed to drive 50 Ω. A chip antenna placed on a board smaller than the vendor’s reference board is detuned: its feed impedance moves away from 50 Ω (often low resistance with leftover reactance), and power reflects. A two-element L-network costs a few cents and typically recovers several dB; a three-pad pi footprint lets you re-tune on the bench after measuring with a vector network analyser (VNA).
          <Formula>{'P_delivered / P_available = 1 − |Γ|²   (minus the small loss of the inductors and capacitors)'}</Formula>
        </Explainer>
        <Explainer title="Units you will meet" refs={[REFS.pozar]}>
          dBm is power relative to 1 mW (0 dBm = 1 mW, −90 dBm = 1 pW). dBi is antenna gain relative to an isotropic radiator. Decibels add along a link: a 3 dB loss halves the power, 10 dB is a factor of ten, and every 6 dB of extra margin doubles the free-space range.
          <Formula>{'P(dBm) = 10·log₁₀(P / 1 mW)     range ∝ 10^(margin / (10·n))'}</Formula>
        </Explainer>
      </div>
      {iris.data && (
        <Card title="The Iris RF chain today" subtitle="Computed from the project — the same numbers the requirements see">
          <div className="table-wrap">
            <table className="table">
              <thead><tr><th>Link in the chain</th><th>Iris now</th><th>What the lab recommends</th></tr></thead>
              <tbody>
                <tr>
                  <td>Antenna feed trace (U1 RF → AE1)</td>
                  <td>{iris.data.current_feed.kind.toUpperCase()} {fmt(iris.data.current_feed.width_mm, 2)} mm → <b>{fmt(iris.data.current_feed_z0, 1)} Ω</b></td>
                  <td>CPWG {fmt(iris.data.recommended_feed.width_mm, 2)} mm, {fmt(iris.data.recommended_feed.gap_mm, 2)} mm gaps → {fmt(iris.data.recommended_feed_z0, 1)} Ω</td>
                </tr>
                <tr>
                  <td>Matching network</td>
                  <td>{iris.data.current_network.length ? iris.data.current_labels.join(' + ') : 'none'} → <b>{fmt(iris.data.current_summary.worst_rl_db, 1)} dB</b> worst return loss</td>
                  <td>{iris.data.recommended_labels.join(' + ')} → {fmt(iris.data.recommended_summary.worst_rl_db, 1)} dB</td>
                </tr>
                <tr>
                  <td>Antenna gain (assumed efficiency)</td>
                  <td>{fmt(iris.data.glasses_gain_dbi, 1)} dBi − {fmt(iris.data.match_loss_db, 1)} dB match loss</td>
                  <td>measure efficiency in an anechoic chamber or a reverberation chamber at a test lab</td>
                </tr>
                <tr>
                  <td>Link to a phone at {fmt(iris.data.design.link.distance_m, 0)} m</td>
                  <td><b>{fmt(iris.data.link.margin_db, 1)} dB</b> margin, {fmt(iris.data.link.range_m, 1)} m range</td>
                  <td>keep ≥ 0 dB with head and body loss and a 90 % fade margin</td>
                </tr>
              </tbody>
            </table>
          </div>
          <ul className="small rf-list" style={{ marginTop: 10 }}>{iris.data.notes.map((n, k) => <li key={k}>{n}</li>)}</ul>
        </Card>
      )}
      <Card title="Tools to graduate to" subtitle="Open-source tools that go further than this browser lab — closest equivalents for each tab">
        <div className="rf-grad">
          <div><b><a href="https://github.com/thliebig/openEMS" target="_blank" rel="noreferrer">openEMS</a></b> 3-D FDTD (equivalent-circuit FDTD) with a MATLAB/Octave and Python interface: simulate the real chip antenna, board and head in 3-D and get S11 and radiation patterns. The Fields tab is the 2-D teaching version of the same Yee scheme.</div>
          <div><b><a href="https://github.com/NanoComp/meep" target="_blank" rel="noreferrer">Meep</a></b> MIT’s FDTD package (Python/Scheme) with PML, dispersive materials and near-to-far-field transforms — great for the physics behind the Fields presets.</div>
          <div><b><a href="https://github.com/scikit-rf/scikit-rf" target="_blank" rel="noreferrer">scikit-rf</a></b> Python network analysis: load the Touchstone .s1p exported from Matching (or measured with a VNA), de-embed, plot Smith charts and design matches from real data.</div>
          <div><b><a href="https://github.com/ra3xdh/qucs_s" target="_blank" rel="noreferrer">Qucs-S</a></b> circuit simulator with S-parameter analysis and transmission-line models: put the matching network, the CPWG feed and a measured antenna file in one schematic and tune with real part libraries.</div>
        </div>
        <div className="small muted" style={{ marginTop: 10 }}>
          What this lab adds over them: everything runs in the browser, is tied to the Iris project file, and the numbers flow straight into requirement verification. What they add over this lab: 3-D geometry, measured data, real component models — use them before ordering boards.
        </div>
      </Card>
      <Card title="Next steps for the real Iris board">
        <ol className="rf-steps">
          <li>Add the recommended matching network (series + shunt, 0402) with a three-pad pi footprint at the antenna feed so it can be re-tuned.</li>
          <li>Give the RF net its own net class in PCB Studio: the CPWG width and gap above, ground stitching vias along both sides, no vias on the feed.</li>
          <li>Follow the chip-antenna vendor’s layout: clearance (no copper) under and around the antenna, the specified ground length, the feed position.</li>
          <li>Measure S11 of the assembled glasses with a VNA — on a head phantom or worn — and type the impedance into Matching; re-snap the values.</li>
          <li>Measure range with the phone app at 1, 5 and 10 m and compare with the Link budget; the gap tells you the real head and body loss.</li>
          <li>Pre-compliance: a module or chip with radio certification, and the regulatory path in Build Guide → Compliance.</li>
        </ol>
      </Card>
      <Card title="References">
        <ul className="small rf-list">
          {Object.values(REFS).map((r) => (
            <li key={r.label}>{'href' in r && r.href ? <a href={r.href} target="_blank" rel="noreferrer">{r.label}</a> : r.label}</li>
          ))}
        </ul>
      </Card>
    </div>
  );
}

/** Forward + reflected wave on a line: the standing-wave envelope and VSWR. */
function StandingWave() {
  const [gm, setGm] = useState(0.5);
  const [ph, setPh] = useState(0);
  const [t, setT] = useState(0);
  const [play, setPlay] = useState(!prefersReducedMotion());
  useRaf(play, (dt) => setT((x) => x + dt));
  const [wrapRef, width] = useElementWidth<HTMLDivElement>();
  const W = Math.max(300, width);
  const H = 180;
  const n = 200;
  const pts = useMemo(() => {
    const w = 2 * Math.PI * 0.6;
    const fwd: string[] = [];
    const tot: string[] = [];
    const envU: string[] = [];
    const envL: string[] = [];
    for (let i = 0; i <= n; i++) {
      const x = i / n; // 0 = source, 1 = load (2 wavelengths)
      const kx = 2 * Math.PI * 2 * x;
      const phr = (ph * Math.PI) / 180;
      const f = Math.sin(w * t - kx);
      const r = gm * Math.sin(w * t + kx - 2 * 2 * Math.PI * 2 + phr);
      // envelope |1 + Γ e^{j(2k(x−L) + φ)}|
      const a = 2 * 2 * Math.PI * 2 * (x - 1) + phr;
      const env = Math.hypot(1 + gm * Math.cos(a), gm * Math.sin(a));
      const X = 20 + x * (W - 40);
      const Y = (v: number) => H / 2 - v * (H / 2 - 12) / 1.9;
      fwd.push(`${i ? 'L' : 'M'}${X.toFixed(1)},${Y(f).toFixed(1)}`);
      tot.push(`${i ? 'L' : 'M'}${X.toFixed(1)},${Y(f + r).toFixed(1)}`);
      envU.push(`${i ? 'L' : 'M'}${X.toFixed(1)},${Y(env).toFixed(1)}`);
      envL.push(`${i ? 'L' : 'M'}${X.toFixed(1)},${Y(-env).toFixed(1)}`);
    }
    return { fwd: fwd.join(''), tot: tot.join(''), envU: envU.join(''), envL: envL.join('') };
  }, [gm, ph, t, W]);
  const vswr = (1 + gm) / (1 - Math.min(0.999, gm));
  const rl = gm > 0 ? -20 * Math.log10(gm) : Infinity;
  return (
    <Card title="Standing waves: what a mismatch looks like on a line" subtitle={`|Γ| = ${gm.toFixed(2)} → VSWR ${fmt(vswr, 2)}, return loss ${isFinite(rl) ? fmt(rl, 1) + ' dB' : '∞'}, ${fmt(100 * gm * gm, 0)} % of the power reflected`} actions={<button className="btn small" onClick={() => setPlay((p) => !p)} aria-pressed={play}>{play ? '❚❚ Pause' : '▶ Play'}</button>}>
      <div ref={wrapRef}>
        <svg viewBox={`0 0 ${W} ${H}`} width="100%" className="rf-sw" role="img" aria-label={`standing wave with reflection coefficient ${gm.toFixed(2)}`}>
          <line x1={20} x2={W - 20} y1={H / 2} y2={H / 2} className="rf-sw-axis" />
          <path d={pts.envU} className="rf-sw-env" />
          <path d={pts.envL} className="rf-sw-env" />
          <path d={pts.fwd} className="rf-sw-fwd" />
          <path d={pts.tot} className="rf-sw-tot" />
          <text x={22} y={14} className="rf-xsec-t">radio (source)</text>
          <text x={W - 22} y={14} textAnchor="end" className="rf-xsec-t">antenna (load)</text>
        </svg>
        <div className="row small" style={{ gap: 14 }}>
          <span><i className="rf-sw-key fwd" /> forward wave</span>
          <span><i className="rf-sw-key tot" /> forward + reflected</span>
          <span><i className="rf-sw-key env" /> envelope (max/min ratio = VSWR)</span>
        </div>
      </div>
      <div className="grid grid-2" style={{ marginTop: 8 }}>
        <Slider label="Reflection magnitude |Γ|" value={gm} min={0} max={0.95} step={0.01} onChange={setGm} />
        <Slider label="Reflection phase ∠Γ" value={ph} min={-180} max={180} step={5} unit="°" onChange={setPh} />
      </div>
    </Card>
  );
}
