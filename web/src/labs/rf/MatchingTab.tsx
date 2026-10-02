// Matching: drag the antenna impedance on a Smith chart, synthesise L and pi
// networks, snap to standard values with component Q, sweep the BLE band,
// export Touchstone, and put the chosen network on Iris.
import { useMemo, useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat, Toggle } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { AntennaModel, IrisOut, MatchOut, Solution } from './types';
import { RF_DEFAULTS, fmt, toCsv, useIrisRf } from './util';
import { Ladder, SmithChart, type Trace } from './SmithChart';
import { REFS } from './refs';

const SOL_COLORS = ['var(--series-1)', 'var(--series-2)', 'var(--series-3)', 'var(--series-4)'];

export function MatchingTab() {
  const project = useProject((s) => s.project);
  const [rf, patchRf, stored] = useIrisRf();
  const [ant, setAnt] = useState<AntennaModel>(rf.antenna);
  const [topology, setTopology] = useState<'l' | 'pi'>('l');
  const [piQ, setPiQ] = useState(3);
  const [series, setSeries] = useState<'E12' | 'E24'>(rf.series === 'E12' ? 'E12' : 'E24');
  const [ql, setQl] = useState(rf.q_l);
  const [qc, setQc] = useState(rf.q_c);
  const [adm, setAdm] = useState(false);
  const [sel, setSel] = useState<number | null>(null);
  const input = useMemo(
    () => ({ antenna: ant, topology, pi_q: piQ, series, q_l: ql, q_c: qc, z0: 50, plot_lo_ghz: 2.2, plot_hi_ghz: 2.7, current: rf.network }),
    [ant, topology, piQ, series, ql, qc, rf.network],
  );
  const res = useCompute<MatchOut>('rf.match', input);
  const iris = useCompute<IrisOut>('rf.iris', { project });
  const m = res.data;
  const idx = m ? Math.min(sel ?? m.best, m.solutions.length - 1) : 0;
  const chosen: Solution | null = m ? m.solutions[idx] : null;
  const traces: Trace[] = [];
  if (m) {
    traces.push({ name: 'bare antenna, BLE band', re: m.bare_sweep.s11_re.filter((_, i) => inBand(m.bare_sweep.f_ghz[i])), im: m.bare_sweep.s11_im.filter((_, i) => inBand(m.bare_sweep.f_ghz[i])), color: 'var(--muted)', width: 4 });
    if (chosen) traces.push({ name: 'matched, BLE band', re: chosen.band.s11_re, im: chosen.band.s11_im, color: SOL_COLORS[idx % 4], width: 5 });
    if (m.current) traces.push({ name: 'on Iris now', re: m.current.band.s11_re, im: m.current.band.s11_im, color: 'var(--series-5)', width: 3, dash: '4 3' });
  }
  const antChanged = JSON.stringify(ant) !== JSON.stringify(rf.antenna);
  const apply = (s: Solution) => {
    patchRf({ antenna: ant, network: s.snapped, series, q_l: ql, q_c: qc });
  };
  return (
    <div className="stack">
      <div className="rf-split">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card
            title="Smith chart — drag Z_L"
            subtitle={m ? `Z_L = ${fmt(m.z_load.re, 1)} ${m.z_load.im >= 0 ? '+' : '−'} j${fmt(Math.abs(m.z_load.im), 1)} Ω at ${fmt(ant.fc_ghz, 4)} GHz · |Γ| = ${fmt(Math.hypot(m.gamma_load.re, m.gamma_load.im), 3)} · series resonance at ${fmt(m.f0_ghz, 3)} GHz` : ''}
            actions={<Toggle label="admittance grid" value={adm} onChange={setAdm} />}
          >
            <ErrorBox error={res.error} />
            <div className="rf-smith-grid2">
              <SmithChart
                load={{ re: ant.r_ohm, im: ant.x_ohm }}
                onDrag={(z) => setAnt((a) => ({ ...a, r_ohm: Math.round(z.re * 10) / 10, x_ohm: Math.round(z.im * 10) / 10 }))}
                traces={traces}
                path={chosen ? { arcs: chosen.path, elements: chosen.snapped, color: SOL_COLORS[idx % 4] } : null}
                admittance={adm}
                rlCircleDb={10}
              />
              <div className="stack" style={{ gap: 8 }}>
                <div className="rf-smith-legend small">
                  <span><i style={{ background: 'var(--muted)' }} /> bare antenna across 2.400–2.4835 GHz</span>
                  {chosen && <span><i style={{ background: SOL_COLORS[idx % 4] }} /> {chosen.name}: element arcs, then the band</span>}
                  {m?.current && <span><i style={{ background: 'var(--series-5)' }} /> network stored on Iris</span>}
                  <span><i className="rf-ring" /> 10 dB return-loss circle (|Γ| = 0.316) — stay inside</span>
                </div>
                <div className="small muted">
                  A series element slides the point along a constant-resistance circle; a shunt element slides it along a constant-conductance circle (turn on the admittance grid). Two moves reach the centre — that is the L-network.
                </div>
                {m && (
                  <div className="grid grid-2" style={{ gap: 8 }}>
                    <Stat label="Bare antenna, worst" value={fmt(m.bare.worst_rl_db, 1)} unit="dB RL" note={`VSWR ${fmt(m.bare.worst_vswr, 2)}`} tone={m.bare.worst_rl_db >= 10 ? 'pass' : 'fail'} />
                    {chosen && <Stat label="Matched, worst in band" value={fmt(chosen.snapped_summary.worst_rl_db, 1)} unit="dB RL" note={`VSWR ${fmt(chosen.snapped_summary.worst_vswr, 2)} · loss ${fmt(-chosen.snapped_summary.worst_gt_db, 2)} dB`} tone={chosen.snapped_summary.worst_rl_db >= 10 ? 'pass' : 'fail'} />}
                  </div>
                )}
              </div>
            </div>
          </Card>
          {m && (
            <Card title="Return loss across frequency" subtitle="Standard values with component Q; shaded: the BLE band. Higher is better.">
              <Plot
                series={[
                  { name: 'bare antenna', x: m.bare_sweep.f_ghz, y: m.bare_sweep.rl_db, color: 'var(--muted)', dash: '5 4' },
                  ...m.solutions.map((s, k) => ({ name: s.name.replace(/ \(.*\)/, ''), x: s.sweep.f_ghz, y: s.sweep.rl_db, color: SOL_COLORS[k % 4], width: k === idx ? 2.5 : 1.5 })),
                  ...(m.current ? [{ name: 'on Iris now', x: m.current.sweep.f_ghz, y: m.current.sweep.rl_db, color: 'var(--series-5)', dash: '2 3' }] : []),
                ]}
                height={250}
                xLabel="frequency (GHz)"
                yLabel="return loss (dB)"
                xBands={[{ from: 2.4, to: 2.4835, label: 'BLE' }]}
                hLines={[{ value: 10, label: '10 dB (VSWR 1.9)' }]}
                yZero
                legend
              />
            </Card>
          )}
          {m && (
            <div className="rf-solutions">
              {m.solutions.map((s, k) => (
                <SolutionCard key={k} s={s} color={SOL_COLORS[k % 4]} active={k === idx} best={k === m.best} onSelect={() => setSel(k)} onApply={() => apply(s)} />
              ))}
            </div>
          )}
        </div>
        <div className="stack rf-sticky">
          <Card title="Antenna feed impedance" subtitle="ASSUMED — measure with a VNA">
            <div className="callout small" style={{ marginBottom: 10 }}>
              A chip antenna on a 10 mm-wide board is detuned from the vendor’s evaluation board. These values are a typical “before matching” picture, not a measurement: measure S11 of the assembled glasses with a vector network analyser (VNA) — on a head phantom — and type it in.
            </div>
            <Slider label="Resistance R" value={ant.r_ohm} min={2} max={150} step={0.5} unit="Ω" onChange={(v) => setAnt({ ...ant, r_ohm: v })} />
            <Slider label="Reactance X at band centre" value={ant.x_ohm} min={-150} max={150} step={0.5} unit="Ω" onChange={(v) => setAnt({ ...ant, x_ohm: v })} />
            <Slider label="Antenna Q (series RLC model)" value={ant.q} min={1} max={40} step={0.5} onChange={(v) => setAnt({ ...ant, q: v })} />
            <div className="row">
              <button className="btn small" onClick={() => setAnt(RF_DEFAULTS.antenna)}>Reset to the Iris assumption</button>
              {antChanged && <Chip tone="warn">! differs from Iris</Chip>}
            </div>
          </Card>
          <Card title="Network">
            <div className="rf-seg" role="group" aria-label="Topology" style={{ marginBottom: 10 }}>
              {(['l', 'pi'] as const).map((t) => (
                <button key={t} className={`rf-seg-btn ${topology === t ? 'active' : ''}`} onClick={() => { setTopology(t); setSel(null); }} aria-pressed={topology === t}>
                  {t === 'l' ? 'L-network' : 'Pi-network'}
                </button>
              ))}
            </div>
            {topology === 'pi' && (
              <Slider label="Loaded Q" value={piQ} min={Math.max(0.5, Math.ceil(((m?.l_network_q ?? 1) + 0.05) * 10) / 10)} max={12} step={0.1} onChange={setPiQ} hint="Must exceed the L-network’s Q" />
            )}
            {m && <div className="small muted" style={{ marginBottom: 10 }}>The L-network’s Q is fixed by the resistance ratio: Q = √(R_high/R_low − 1) = {fmt(m.l_network_q, 2)} (bandwidth ≈ f/Q).</div>}
            <div className="rf-seg" role="group" aria-label="Standard values" style={{ marginBottom: 10 }}>
              {(['E12', 'E24'] as const).map((t) => (
                <button key={t} className={`rf-seg-btn ${series === t ? 'active' : ''}`} onClick={() => setSeries(t)} aria-pressed={series === t}>{t} values</button>
              ))}
            </div>
            <Slider label="Inductor Q at 2.4 GHz (0402: typ. 20–60)" value={ql} min={5} max={120} step={1} onChange={setQl} />
            <Slider label="Capacitor Q at 2.4 GHz (C0G: typ. 100–400)" value={qc} min={10} max={500} step={5} onChange={setQc} />
          </Card>
          {iris.data && (
            <Card title="On Iris" subtitle={stored ? 'design.extra.rf' : 'no RF design stored yet'}>
              <div className="stack" style={{ gap: 8 }}>
                <div className="small">
                  {iris.data.current_network.length ? (
                    <>Network: <b>{iris.data.current_labels.join(' + ')}</b></>
                  ) : (
                    <>✕ <b>No matching network</b> — the bill of materials (BOM) has the chip antenna wired straight to the radio. PCB Studio flags the same gap.</>
                  )}
                </div>
                <Chip tone={iris.data.current_summary.worst_rl_db >= 10 ? 'pass' : 'fail'}>
                  {iris.data.current_summary.worst_rl_db >= 10 ? '✓' : '✕'} rf.match_return_loss_db = {fmt(iris.data.current_summary.worst_rl_db, 1)} dB
                </Chip>
                <div className="small muted">Parts to add (recommended network):</div>
                <table className="table">
                  <tbody>
                    {iris.data.bom.map((b) => (
                      <tr key={b.refdes} title={b.note}>
                        <td className="mono">{b.refdes}</td>
                        <td>{b.value}</td>
                        <td className="muted small">{b.footprint}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                <div className="row">
                  <button className="btn small primary" onClick={() => patchRf({ network: iris.data!.recommended_network })}>Use the recommended network on Iris</button>
                  {rf.network && <button className="btn small" onClick={() => patchRf({ network: null })}>Remove</button>}
                </div>
                <DownloadButton
                  small
                  filename="iris-rf-match-bom.csv"
                  content={() => toCsv([['refdes', 'name', 'value', 'footprint', 'note'], ...iris.data!.bom.map((b) => [b.refdes, b.name, b.value, b.footprint, b.note])])}
                  label="BOM rows CSV"
                />
              </div>
            </Card>
          )}
          {m && (
            <Card title="Notes">
              <ul className="small rf-list">{m.notes.map((n, k) => <li key={k}>{n}</li>)}</ul>
            </Card>
          )}
        </div>
      </div>
      <div className="grid grid-2">
        <Explainer title="Reflection, return loss and the Smith chart" refs={[REFS.pozar, REFS.smith]}>
          A load Z on a Z₀ line reflects Γ = (Z − Z₀)/(Z + Z₀). The Smith chart draws the Γ plane with circles of constant resistance and arcs of constant reactance, so impedance arithmetic becomes geometry:
          <Formula>{'RL = −20·log₁₀|Γ|     VSWR = (1 + |Γ|)/(1 − |Γ|)     mismatch loss = −10·log₁₀(1 − |Γ|²)'}</Formula>
          10 dB return loss means 10 % of the power bounces back (0.46 dB lost) — the usual target for a small antenna over its band.
        </Explainer>
        <Explainer title="L-network: two elements, two solutions" refs={[REFS.pozar, REFS.bowick]}>
          For R_L &lt; Z₀ put a series element at the load and a shunt one toward the source:
          <Formula>{'X = ±√(R_L(Z₀ − R_L)) − X_L,     B = ±√((Z₀ − R_L)/R_L) / Z₀'}</Formula>
          For R_L &gt; Z₀ swap them: B = (X_L ± √(R_L/Z₀)·√(R_L² + X_L² − Z₀R_L))/(R_L² + X_L²), X = 1/B + X_L·Z₀/R_L − Z₀/(B·R_L). The ± gives two networks; choose the one that keeps the band tight, uses friendly values (here: snapped to E-series and re-checked with component Q) and, if needed, blocks DC. A pi-network passes through a lower virtual resistance R_v = R_high/(Q² + 1) so you choose Q.
        </Explainer>
      </div>
    </div>
  );
}

function inBand(f: number) {
  return f >= 2.4 - 1e-9 && f <= 2.4835 + 1e-9;
}

function SolutionCard(props: { s: Solution; color: string; active: boolean; best: boolean; onSelect: () => void; onApply: () => void }) {
  const { s } = props;
  return (
    <div className={`rf-sol ${props.active ? 'active' : ''}`} style={{ borderLeftColor: props.color }}>
      <button className="rf-sol-head" onClick={props.onSelect} aria-pressed={props.active}>
        <b>{s.name}</b>
        {props.best && <Chip tone="pass">✓ recommended</Chip>}
      </button>
      <Ladder net={s.snapped} />
      <dl className="rf-kv">
        <dt>Ideal values</dt>
        <dd>{s.ideal.map((e) => (e.kind.endsWith('_l') ? `${fmt(e.value * 1e9, 2)} nH` : `${fmt(e.value * 1e12, 2)} pF`)).join(' · ')}</dd>
        <dt>Standard values</dt>
        <dd>{s.labels.join(' · ')}</dd>
        <dt>Worst RL in band</dt>
        <dd>{fmt(s.snapped_summary.worst_rl_db, 1)} dB (ideal {fmt(s.ideal_summary.worst_rl_db, 1)})</dd>
        <dt>Worst VSWR</dt>
        <dd>{fmt(s.snapped_summary.worst_vswr, 2)}</dd>
        <dt>Network loss + mismatch</dt>
        <dd>{fmt(-s.snapped_summary.worst_gt_db, 2)} dB</dd>
        {s.loaded_q > 0 && (
          <>
            <dt>Loaded Q</dt>
            <dd>{fmt(s.loaded_q, 2)}</dd>
          </>
        )}
      </dl>
      <div className="row">
        <button className="btn small primary" onClick={props.onApply}>Use on Iris</button>
        <DownloadButton small filename={`iris-antenna-${s.topology}-${s.name.match(/[A-Z](?= \()/)?.[0] ?? 'pi'}.s1p`} content={s.touchstone} label=".s1p" />
        <DownloadButton small filename="sweep.csv" content={() => toCsv([['f_ghz', 's11_re', 's11_im', 'rl_db', 'vswr'], ...s.sweep.f_ghz.map((f, k) => [f, s.sweep.s11_re[k], s.sweep.s11_im[k], s.sweep.rl_db[k], s.sweep.vswr[k]])])} label="CSV" />
      </div>
    </div>
  );
}
