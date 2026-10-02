// System reliability: the Iris block diagram built from the bill of
// materials with typical failure rates, R(t), MTBF, a Monte Carlo check,
// importance and an animated replay of simulated units.
import { useMemo, useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat, Toggle } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { RbdDiagram, nodeFailed } from './RbdDiagram';
import type { BlockOut, IrisOut, RbdNode } from './types';
import { fmt, pct, prefersReducedMotion, toCsv, useRaf, useRelConfig } from './util';

function findNode(n: RbdNode, id: string): RbdNode | null {
  if (n.id === id) return n;
  for (const c of n.children ?? []) {
    const f = findNode(c, id);
    if (f) return f;
  }
  return null;
}

export function SystemTab() {
  const project = useProject((s) => s.project);
  const [cfg, setCfg] = useRelConfig();
  const { data: iris, error } = useCompute<IrisOut>('reliability.iris', { project, samples: 20000, seed: 1 });
  const rbd = iris?.rbd;
  const hpd = iris?.profile.hours_per_day ?? cfg.hours_per_day;
  const yearH = 365 * hpd;
  const missionH = rbd?.mission_h ?? 5840;
  const horizonH = 3 * missionH;
  const [tH, setTH] = useState(missionH);
  const [unitIx, setUnitIx] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  useRaf(playing, (dt) =>
    setTH((v) => {
      const n = v + (dt * horizonH) / 14;
      if (n >= horizonH) {
        setUnitIx((u) => (u + 1) % Math.max(1, rbd?.mc?.units.length ?? 1));
        return 0;
      }
      return n;
    }),
  );
  const unit = rbd?.mc?.units[unitIx] ?? null;
  const ttf = useMemo(() => {
    if (!unit || !rbd) return null;
    const m = new Map<string, number>();
    rbd.blocks.forEach((b, i) => m.set(b.id, unit.leaf_ttf_h[i]));
    return m;
  }, [unit, rbd]);
  const cause = useMemo(() => {
    if (!unit || !rbd || !iris) return null;
    const i = unit.leaf_ttf_h.findIndex((t) => Math.abs(t - unit.system_ttf_h) <= 1e-9 * Math.max(1, t));
    return i >= 0 ? rbd.blocks[i] : null;
  }, [unit, rbd, iris]);
  const sysFailed = iris && ttf ? nodeFailed(iris.tree, tH, ttf) : false;
  const sorted = useMemo(() => (rbd ? [...rbd.blocks].sort((a, b) => b.criticality - a.criticality) : []), [rbd]);
  const sel = selected && iris ? findNode(iris.tree, selected) : null;
  const selOut = selected && rbd ? rbd.blocks.find((b) => b.id === selected) ?? null : null;
  const blocksCsv = () =>
    rbd
      ? toCsv([
          ['refdes', 'part', 'class', 'group', 'model', 'fit_use', 'fit_ref_40C', 'temp_factor', 'beta', 'eta', 'life_unit', 'per_op_hour', 'qty', 'source', 'R_mission', 'birnbaum', 'criticality'],
          ...rbd.blocks.map((b) => [b.refdes, b.label, b.class, b.group, b.model, +b.fit.toFixed(3), b.fit_ref, +b.temp_factor.toFixed(4), b.beta || null, b.eta || null, b.life_unit, +b.per_hour.toFixed(5), b.qty, b.source, +b.r_mission.toFixed(6), +b.birnbaum.toFixed(6), +b.criticality.toFixed(6)]),
        ])
      : '';
  return (
    <div className="rl-layout">
      <div className="stack" style={{ minWidth: 0 }}>
        <ErrorBox error={error} />
        {rbd && iris && (
          <div className="grid grid-4">
            <Stat label="Reliability over 2 years" value={pct(iris.r_2yr, 1)} note={`${hpd} h/day worn · wear-out included`} tone={iris.r_2yr >= 0.9 ? 'pass' : 'warn'} />
            <Stat label="Parts-count MTBF" value={fmt(rbd.mtbf_random_h, 3)} unit="h" note={`${fmt(rbd.fit_total, 3)} FIT, random failures only`} />
            <Stat label="Mean time to failure" value={fmt(rbd.mttf_h / yearH, 3)} unit="years" note={`${fmt(rbd.mttf_h, 3)} operating h, all modes`} />
            <Stat label="B10 life (10 % failed)" value={fmt(rbd.b10_h / yearH, 3)} unit="years" note={`dominated by ${rbd.dominant}`} />
          </div>
        )}
        {rbd && iris && (
          <Card
            title="Iris reliability block diagram"
            subtitle="Subsystems in series; parts inside each box are in series unless shown as k of n. Bars show each block's reliability at the time below; ◷ marks wear-out items. Press play to replay simulated units."
            actions={
              <div className="row">
                <button className="btn small" onClick={() => setPlaying((p) => !p)} aria-pressed={playing}>{playing ? '❚❚ Pause' : '▶ Replay units'}</button>
                <DownloadButton small filename="iris-rbd.json" label="RBD JSON" content={() => JSON.stringify(iris.tree, null, 2)} />
              </div>
            }
          >
            <Slider label="Time in service" value={tH / yearH} min={0} max={horizonH / yearH} step={0.01} unit="years" onChange={(v) => { setPlaying(false); setTH(v * yearH); }} format={(v) => v.toFixed(2)} />
            <div className={`rl-unit-status ${sysFailed ? 'bad' : 'good'}`} aria-live="polite">
              {unit ? (
                sysFailed
                  ? `✕ Simulated unit ${unitIx + 1} failed at ${fmt(unit.system_ttf_h / yearH, 3)} years — ${cause ? `${cause.label} (${cause.refdes})` : 'system'}`
                  : `✓ Simulated unit ${unitIx + 1} working at ${(tH / yearH).toFixed(2)} years${unit.system_ttf_h < 1e14 ? ` · it will fail at ${fmt(unit.system_ttf_h / yearH, 3)} years (${cause?.label ?? ''})` : ''}`
              ) : 'Monte Carlo off'}
              <span className="row" style={{ marginLeft: 'auto' }}>
                <button className="btn small" onClick={() => setUnitIx((u) => (u + 7) % Math.max(1, rbd.mc?.units.length ?? 1))} aria-label="previous unit">‹</button>
                <button className="btn small" onClick={() => setUnitIx((u) => (u + 1) % Math.max(1, rbd.mc?.units.length ?? 1))} aria-label="next unit">›</button>
              </span>
            </div>
            <RbdDiagram root={iris.tree} tH={tH} ttf={ttf} flowing={playing && !prefersReducedMotion()} selected={selected} onSelect={setSelected} />
            {sel && selOut && <BlockEditor node={sel} out={selOut} onClose={() => setSelected(null)} />}
          </Card>
        )}
        {rbd && (
          <Card title="Reliability over time" subtitle="Analytic R(t) of the block diagram, the random-failure part alone, and the Monte Carlo estimate (20 000 simulated units).">
            <Plot
              series={[
                { name: 'system R(t)', x: rbd.t_h.map((t) => t / yearH), y: rbd.r_sys },
                { name: 'random failures only (parts count)', x: rbd.t_h.map((t) => t / yearH), y: rbd.r_random, dash: '5 3' },
                ...(rbd.mc ? [{ name: 'Monte Carlo', x: rbd.t_h.filter((_, i) => i % 6 === 0).map((t) => t / yearH), y: rbd.mc.r_curve.filter((_, i) => i % 6 === 0), scatter: true, color: 'var(--series-2)' }] : []),
              ]}
              xLabel="years in service"
              yLabel="reliability"
              legend
              height={250}
              yDomain={[0, 1.02]}
              vLines={[{ value: missionH / yearH, label: `${fmt(missionH / yearH)} y` }, { value: rbd.b10_h / yearH, label: 'B10' }]}
            />
            {rbd.mc && (
              <div className="small muted">
                Monte Carlo: R({fmt(missionH / yearH)} y) = {pct(rbd.mc.r_mission, 2)} (95 % interval {pct(rbd.mc.ci_lo, 2)} – {pct(rbd.mc.ci_hi, 2)}) against the analytic {pct(rbd.r_mission, 2)}
                {rbd.r_mission >= rbd.mc.ci_lo && rbd.r_mission <= rbd.mc.ci_hi ? ' ✓ agree' : ' ! outside the interval'}.
              </div>
            )}
            {rbd.notes.map((n, i) => (
              <div key={i} className="small muted">• {n}</div>
            ))}
          </Card>
        )}
        {rbd && (
          <Card title="What dominates? Importance at the mission time" subtitle="Criticality importance = the share of system failures a block is responsible for; Monte Carlo counts which block actually failed first.">
            <div className="rl-imp">
              {sorted.slice(0, 10).map((b) => (
                <div key={b.id} className="rl-imp-row">
                  <button className="rl-imp-name" onClick={() => setSelected(b.id)} title={b.class}>{b.refdes} · {b.label}</button>
                  <div className="rl-imp-bar"><div style={{ width: `${Math.max(0.5, b.criticality * 100)}%` }} /></div>
                  <span className="tnum small">{pct(b.criticality, 1)}</span>
                  <span className="tnum small muted" title="Monte Carlo first-failure share">MC {pct(b.mc_first_share, 1)}</span>
                </div>
              ))}
            </div>
            {sorted[0] && sorted[0].criticality > 0.5 && (
              <div className="rl-verdict warn" style={{ marginTop: 10 }}>
                ! {sorted[0].label} causes {pct(sorted[0].criticality, 0)} of failures in {fmt(missionH / yearH)} years. {sorted[0].model === 'weibull' && sorted[0].refdes.startsWith('BT')
                  ? 'The cell wears out before the electronics: a larger cell (shallower cycles), a lower charge voltage or a cell rated for more cycles moves this — or treat the battery as a service item and say so in the requirement.'
                  : 'Improve this item first; the rest barely matter.'}
              </div>
            )}
          </Card>
        )}
        {rbd && (
          <Card title="Blocks" subtitle="Typical failure rates at 40 °C (in failures in time, FIT = failures per 10⁹ device-hours), scaled to the board temperature for thermally activated parts — typical, replace with supplier data. Click a row to edit." actions={<DownloadButton small filename="iris-rbd-blocks.csv" label="blocks CSV" content={blocksCsv} />}>
            <div className="table-wrap" style={{ maxHeight: 380 }}>
              <table className="table">
                <thead><tr><th>Ref</th><th>Part</th><th>Class</th><th>Model</th><th>FIT (use)</th><th>β / η</th><th>R(mission)</th><th>Source</th></tr></thead>
                <tbody>
                  {rbd.blocks.map((b) => (
                    <tr key={b.id} onClick={() => setSelected(b.id)} className={selected === b.id ? 'rl-row-active' : undefined} style={{ cursor: 'pointer' }}>
                      <td className="mono small">{b.refdes}</td>
                      <td>{b.label}{b.qty !== 1 ? ` ×${b.qty}` : ''}</td>
                      <td className="small muted">{b.class}</td>
                      <td>{b.model === 'weibull' ? 'wear-out' : 'constant'}</td>
                      <td className="tnum">{b.model === 'weibull' ? '—' : fmt(b.fit, 3)}</td>
                      <td className="tnum">{b.model === 'weibull' ? `${fmt(b.beta)} / ${fmt(b.eta, 4)} ${b.life_unit}` : '—'}</td>
                      <td className="tnum">{(b.r_mission * 100).toFixed(3)} %</td>
                      <td className="small">{b.source === 'typical' ? 'typical' : `✎ ${b.source}`}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {iris && iris.excluded.length > 0 && (
              <div className="small muted" style={{ marginTop: 6 }}>Not modelled: {iris.excluded.map((e) => `${e.refdes} ${e.name} — ${e.reason}`).join('; ')}.</div>
            )}
            {Object.keys(cfg.blocks).length > 0 && (
              <button className="btn small" style={{ marginTop: 8 }} onClick={() => setCfg({ blocks: {} })}>Reset all {Object.keys(cfg.blocks).length} edited blocks to typical</button>
            )}
          </Card>
        )}
      </div>
      <div className="stack rl-side">
        <Card title="Use profile" subtitle="Saved with the project (design.extra.reliability) — the metrics follow">
          <Slider label="Mission" value={cfg.mission_years} min={0.5} max={5} step={0.5} unit="years" onChange={(v) => setCfg({ mission_years: v })} />
          <Slider label="Worn per day" value={cfg.hours_per_day} min={1} max={16} step={0.5} unit="h" onChange={(v) => setCfg({ hours_per_day: v })} />
          <Slider label="Open/close per day" value={cfg.hinge_cycles_per_day} min={1} max={40} step={1} onChange={(v) => setCfg({ hinge_cycles_per_day: v })} />
          <Slider label="Activation energy for part FITs" value={cfg.ea_ev} min={0.3} max={1} step={0.05} unit="eV" onChange={(v) => setCfg({ ea_ev: v })} />
          <Select label="Microphones" value={String(cfg.mic_k)} onChange={(v) => setCfg({ mic_k: Number(v) })} options={[{ value: '2', label: 'Both needed (beamformer) — series' }, { value: '1', label: 'One is enough (degraded captions) — 1 of 2' }]} />
          <Toggle label="Count the battery reaching 80 % capacity as a failure" value={cfg.battery_counts} onChange={(v) => setCfg({ battery_counts: v })} />
          {iris && (
            <dl className="rl-kv">
              <dt>Battery life per charge</dt><dd>{fmt(iris.profile.battery_life_h, 3)} h</dd>
              <dt>Charge cycles per day</dt><dd>{fmt(iris.profile.charge_cycles_per_day, 3)}</dd>
              <dt>Operating hours in 2 years</dt><dd>{fmt(2 * 365 * iris.profile.hours_per_day)}</dd>
              <dt>Board temperature in use</dt><dd>{iris.profile.board_c.toFixed(1)} °C</dd>
              <dt>FIT temperature factor</dt><dd>× {iris.profile.temp_factor.toFixed(2)} vs 40 °C</dd>
            </dl>
          )}
          <div className="small muted">Battery life and board temperature come from the Power & Thermal lab's model of this project.</div>
        </Card>
        <Explainer title="Block-diagram algebra" refs={[{ label: 'O’Connor & Kleyner, Practical Reliability Engineering, 5th ed., ch. 6' }, { label: 'Rausand & Høyland, System Reliability Theory, 2nd ed., ch. 4–5' }]}>
          <Formula>{`series      R = Π Rᵢ
parallel    R = 1 − Π (1 − Rᵢ)
k of n      R = P(at least k work)
MTTF = ∫₀^∞ R(t) dt;  series of constants: MTBF = 10⁹ / Σ FIT`}</Formula>
        </Explainer>
        <Explainer title="Failure rates and where to get real ones" refs={[{ label: 'MIL-HDBK-217F, Reliability Prediction of Electronic Equipment (1991, Notice 2 1995)' }, { label: 'Z. W. Birnbaum, "On the importance of different components in a multicomponent system", 1969' }]}>
          The FIT values here are typical orders of magnitude for commercial parts, not predictions: replace them with the reliability reports most semiconductor makers publish per product family, or with your own test data (Weibull tab → "Use this fit").
          Parts-count methods such as MIL-HDBK-217F are dated and pessimistic for modern parts; their value is ranking, not absolute numbers.
          <Formula>{`Birnbaum  I_B = R_sys(Rᵢ=1) − R_sys(Rᵢ=0)
criticality  I_C = I_B (1 − Rᵢ) / (1 − R_sys)`}</Formula>
        </Explainer>
      </div>
    </div>
  );
}

function BlockEditor({ node, out, onClose }: { node: RbdNode; out: BlockOut; onClose: () => void }) {
  const [cfg, setCfg] = useRelConfig();
  const ov = cfg.blocks[node.refdes || node.id] ?? {};
  const key = node.refdes || node.id;
  const setOv = (p: { fit?: number; beta?: number; eta?: number }) => setCfg({ blocks: { ...cfg.blocks, [key]: { ...ov, ...p, source: 'user' } } });
  return (
    <div className="rl-editor">
      <div className="row" style={{ justifyContent: 'space-between' }}>
        <b>{node.refdes} · {node.label}</b>
        <button className="btn small" onClick={onClose} aria-label="close editor">✕</button>
      </div>
      <div className="small muted">{node.class}. {node.note}</div>
      {node.model === 'weibull' ? (
        <div className="grid grid-2">
          <Slider label="Shape β" value={node.beta} min={0.5} max={8} step={0.1} onChange={(v) => setOv({ beta: v })} />
          <Slider label={`Characteristic life η (${node.life_unit})`} value={node.eta} min={Math.max(10, node.eta / 10)} max={node.eta * 10} log onChange={(v) => setOv({ eta: Math.round(v) })} />
          <div className="small">Driven at {fmt(node.per_hour, 3)} {node.life_unit} per operating hour.</div>
        </div>
      ) : (
        <div className="grid grid-2">
          <Slider label="Failure rate at use conditions" value={Math.max(0.01, node.fit)} min={0.01} max={5000} log unit="FIT" onChange={(v) => setOv({ fit: +v.toPrecision(3) })} />
          <div className="small">Typical {fmt(node.fit_ref)} FIT at 40 °C{node.temp_factor !== 1 ? ` × ${node.temp_factor.toFixed(2)} (Arrhenius)` : ''}. MTBF of this part alone: {fmt(1e9 / Math.max(1e-9, node.fit * node.qty), 3)} h.</div>
        </div>
      )}
      <div className="row small" style={{ marginTop: 6, flexWrap: 'wrap' }}>
        <span>R(mission) {(out.r_mission * 100).toFixed(4)} % · Birnbaum {out.birnbaum.toFixed(4)} · criticality {pct(out.criticality, 2)}</span>
        {cfg.blocks[key] && <button className="btn small" onClick={() => { const b = { ...cfg.blocks }; delete b[key]; setCfg({ blocks: b }); }}>Back to typical</button>}
      </div>
    </div>
  );
}
