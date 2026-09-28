import { useMemo } from 'react';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat, StatusChip, Toggle } from '../../components/ui';
import { Plot, type Series } from '../../components/Plot';
import { KV, fmt, json, mix, toCSV, useIdleCompute } from '../control/kit';
import type { EvalSummary, PowerConfig, PowerResult } from './types';

const CTX_COLORS = ['var(--surface-3)', 'var(--series-1)', 'var(--series-3)', 'var(--series-7)'];
const DISPLAY_FILL = (m: number) => (m === 0 ? 'var(--surface-2)' : m === 1 ? mix('var(--series-4)', 'var(--surface-2)', 50) : 'var(--series-4)');

interface FrontierPoint { lambda: number; life_h: number; served: number; day_utility: number; fixed_life_h: number }

function Strip({ values, colors, t, maxT, label, titleOf }: { values: number[]; colors: (v: number) => string; t: number[]; maxT: number; label: string; titleOf: (v: number) => string }) {
  // merge runs of equal values into rectangles
  const runs: { a: number; b: number; v: number }[] = [];
  for (let i = 0; i < values.length; i++) {
    const v = values[i];
    const a = t[i];
    const b = t[i + 1] ?? t[i];
    if (runs.length && runs[runs.length - 1].v === v) runs[runs.length - 1].b = b;
    else runs.push({ a, b, v });
  }
  return (
    <div className="row" style={{ flexWrap: 'nowrap', gap: 8, alignItems: 'center' }}>
      <span className="small muted" style={{ width: 62, flex: '0 0 62px', textAlign: 'right' }}>{label}</span>
      <svg viewBox="0 0 1000 16" preserveAspectRatio="none" style={{ width: '100%', height: 16, display: 'block', borderRadius: 4, background: 'var(--surface-2)' }} role="img" aria-label={label}>
        {runs.map((r, i) => (
          <rect key={i} x={(1000 * r.a) / maxT} width={Math.max(0.5, (1000 * (r.b - r.a)) / maxT)} y={0} height={16} fill={colors(r.v)}>
            <title>{`${titleOf(r.v)} · ${r.a.toFixed(2)}–${r.b.toFixed(2)} h`}</title>
          </rect>
        ))}
      </svg>
    </div>
  );
}

function DayTimeline({ e, maxT, res }: { e: EvalSummary; maxT: number; res: PowerResult }) {
  const tl = e.timeline;
  return (
    <div className="stack" style={{ gap: 4 }}>
      <div className="row small" style={{ justifyContent: 'space-between' }}>
        <b>{e.name}</b>
        <span className="muted">battery lasts <b className="tnum" style={{ color: 'var(--text)' }}>{e.life_h.toFixed(2)} h</b> · utility served {(e.served * 100).toFixed(1)} % · avg {e.avg_power_mw.toFixed(1)} mW</span>
      </div>
      <Strip label="context" values={tl.ctx} t={tl.t_h} maxT={maxT} colors={(v) => CTX_COLORS[v]} titleOf={(v) => res.contexts[v]} />
      <Strip label="display" values={tl.display} t={tl.t_h} maxT={maxT} colors={DISPLAY_FILL} titleOf={(v) => res.displays[v]} />
      <Strip label="radio" values={tl.radio} t={tl.t_h} maxT={maxT} colors={(v) => (v === 0 ? 'var(--series-2)' : mix('var(--series-2)', 'var(--surface-2)', 30))} titleOf={(v) => `radio ${res.radios[v]}`} />
    </div>
  );
}

export function PowerPolicy() {
  const project = useProject((s) => s.project);
  const patch = useProject((s) => s.patchDesign);
  const saved = (project.design.extra?.rl_power ?? {}) as Partial<PowerConfig>;
  const setCfg = (p: Partial<PowerConfig>) => patch('extra', { rl_power: { ...saved, ...p } } as never);
  const req = project.requirements.find((r) => r.verification.metric === 'rl.policy_life_gain_pct');
  const goal = req?.verification.target ?? 5;
  const cfg = { lambda: 0.05, beta: 1, low_refresh_factor: 0.7, episodes: 400, seed: 11, all_actions: true, ...saved };
  const res = useCompute<PowerResult>('rl.power_policy', { project, config: { ...saved, with_dp: true } });
  const d = res.data;
  const frontier = useIdleCompute<FrontierPoint[]>('rl.power_frontier', { project, config: saved }, 350);

  const maxT = d ? Math.max(d.fixed.life_h, d.learned.life_h, d.dp?.life_h ?? 0) * 1.02 : 1;
  const socS = useMemo<Series[]>(() => (d ? [
    { name: 'Fixed (BOM duty cycle)', x: d.fixed.timeline.t_h, y: d.fixed.timeline.soc.map((v) => v * 100), color: 'var(--series-2)' },
    { name: 'Learned (Q-learning)', x: d.learned.timeline.t_h, y: d.learned.timeline.soc.map((v) => v * 100), color: 'var(--series-1)' },
  ] : []), [d]);
  const trainS = useMemo<Series[]>(() => {
    if (!d) return [];
    const x = d.training_curve.map((_, i) => i + 1);
    const ma = d.training_curve.map((_, i) => { const lo = Math.max(0, i - 24); const s = d.training_curve.slice(lo, i + 1); return s.reduce((a, b) => a + b, 0) / s.length; });
    return [{ name: 'mean reward / step', x, y: d.training_curve, color: 'var(--series-1)', width: 0.8 }, { name: '25-episode average', x, y: ma, color: 'var(--series-2)', width: 2 }];
  }, [d]);
  const frontS = useMemo<Series[]>(() => {
    const f = frontier.data ?? [];
    if (!f.length) return [];
    return [
      { name: 'learned policy for each λ', x: f.map((p) => p.life_h), y: f.map((p) => p.served * 100), color: 'var(--series-1)' },
      { name: 'λ points', x: f.map((p) => p.life_h), y: f.map((p) => p.served * 100), color: 'var(--series-1)', scatter: true },
      { name: 'fixed policy', x: [f[0].fixed_life_h], y: [100], color: 'var(--series-2)', scatter: true },
    ];
  }, [frontier.data]);

  const nb = d?.soc_buckets ?? 6;
  const actLabel = (a: number) => `${d?.displays[Math.floor(a / 2)] ?? ''} · ${d?.radios[a % 2] ?? ''}`;
  const policyJson = () => json({
    description: 'Iris power policy: state = SoC bucket × user context; action = display mode × BLE radio interval',
    soc_buckets: nb, contexts: d?.contexts, actions: [0, 1, 2, 3, 4, 5].map(actLabel), state_index: 'bucket * 4 + context',
    q: d?.q, policy: d?.policy, dp_policy: d?.dp_policy, config: { ...saved }, model: d?.model,
  });
  const timelineCsv = () => {
    if (!d) return '';
    const L = d.learned.timeline, F = d.fixed.timeline;
    return toCSV(['t_h', 'context', 'fixed_display', 'fixed_radio', 'fixed_soc', 'learned_display', 'learned_radio', 'learned_soc'],
      [L.t_h, L.ctx.map((c) => d.contexts[c]), F.display.map((v) => d.displays[v]), F.radio.map((v) => d.radios[v]), F.soc, L.display.map((v) => d.displays[v]), L.radio.map((v) => d.radios[v]), L.soc]);
  };

  const PolicyGrid = ({ pol, compare }: { pol: number[]; compare?: number[] }) => (
    <svg viewBox={`0 0 ${90 + nb * 64} ${4 * 34 + 26}`} style={{ width: '100%', display: 'block' }} role="img" aria-label="policy table">
      {d?.contexts.map((c, ci) => (
        <g key={c}>
          <text x={82} y={ci * 34 + 21} fontSize={12} textAnchor="end" fill="var(--text-2)">{c}</text>
          {Array.from({ length: nb }, (_, b) => {
            const s = b * 4 + ci;
            const a = pol[s];
            const disp = Math.floor(a / 2);
            const diff = compare && compare[s] !== a;
            const q = d.q[s];
            return (
              <g key={b}>
                <title>{`SoC ${(b * 100 / nb).toFixed(0)}–${((b + 1) * 100 / nb).toFixed(0)} %, ${c}: ${actLabel(a)}${q ? ` · Q = [${q.map((v) => v.toFixed(2)).join(', ')}]` : ''}`}</title>
                <rect x={90 + b * 64} y={ci * 34 + 2} width={60} height={30} rx={5} fill={DISPLAY_FILL(disp)} stroke={diff ? 'var(--warn)' : 'var(--border)'} strokeWidth={diff ? 2.5 : 1} />
                <text x={90 + b * 64 + 30} y={ci * 34 + 15} fontSize={10.5} textAnchor="middle" fill="var(--text)" fontWeight={600}>{d.displays[disp]}</text>
                <text x={90 + b * 64 + 30} y={ci * 34 + 27} fontSize={9.5} textAnchor="middle" fill="var(--text-2)">{a % 2 === 0 ? 'radio fast' : 'radio slow'}</text>
              </g>
            );
          })}
        </g>
      ))}
      {Array.from({ length: nb }, (_, b) => (
        <text key={b} x={90 + b * 64 + 30} y={4 * 34 + 18} fontSize={10.5} textAnchor="middle" fill="var(--muted)">{(b * 100 / nb).toFixed(0)}–{((b + 1) * 100 / nb).toFixed(0)} %</text>
      ))}
    </svg>
  );

  const m = d?.model;
  return (
    <div className="stack">
      <div className="grid grid-4">
        <Stat label="Battery-life gain" value={fmt(d?.metric_value, 1)} unit="%" tone={d ? (d.metric_value >= goal ? 'pass' : 'fail') : undefined} note={<span>{d && <StatusChip status={d.metric_value >= goal ? 'pass' : 'fail'} />} R-024 goal ≥ {goal} %</span>} />
        <Stat label="Battery life" value={d ? `${d.fixed.life_h.toFixed(1)} → ${d.learned.life_h.toFixed(1)}` : '—'} unit="h" note={`continuous use, ${cfg.episodes} training episodes`} />
        <Stat label="Utility served" value={d ? `${(d.learned.served * 100).toFixed(1)} %` : '—'} tone={d ? (d.utility_ok ? 'pass' : 'fail') : undefined} note={d ? `fixed ${(d.fixed.served * 100).toFixed(1)} % · ${d.utility_ok ? 'equal or better ✓' : 'utility dropped ✕'}` : ''} />
        <Stat label="Matches DP optimum" value={fmt((d?.dp_agreement ?? NaN) * 100, 0)} unit="% of states" note={d?.dp ? `DP policy life ${d.dp.life_h.toFixed(2)} h` : ''} />
      </div>

      <div className="grid grid-sidebar">
        <Card title="MDP & learner" subtitle="Saved in design.extra.rl_power — the R-024 metric uses them">
          <Slider label="Energy price λ" value={cfg.lambda} min={0} max={1.5} step={0.01} onChange={(v) => setCfg({ lambda: v })} hint="reward = utility − λ(soc)·P/P_ref" />
          <Slider label="Battery awareness β" value={cfg.beta} min={0} max={10} step={0.1} onChange={(v) => setCfg({ beta: v })} hint="λ(soc) = λ(1 + β(1 − soc)²)" />
          <Slider label="Low-refresh display power" value={cfg.low_refresh_factor} min={0.3} max={1} step={0.01} format={(v) => `${(v * 100).toFixed(0)} %`} onChange={(v) => setCfg({ low_refresh_factor: v })} hint="Assumption: fraction of full display power in low-refresh mode" />
          <Slider label="Training episodes" value={cfg.episodes} min={20} max={4000} log step={1} format={(v) => String(Math.round(v))} onChange={(v) => setCfg({ episodes: Math.round(v) })} />
          <Slider label="Seed" value={cfg.seed} min={1} max={99} step={1} format={(v) => String(Math.round(v))} onChange={(v) => setCfg({ seed: Math.round(v) })} />
          <Toggle label="Update all actions per sample" value={cfg.all_actions} onChange={(v) => setCfg({ all_actions: v })} />
          <div className="row">
            <button className="btn small ghost" onClick={() => patch('extra', { rl_power: undefined } as never)}>Reset to defaults</button>
            <DownloadButton filename="iris-power-policy.json" content={policyJson} label="Policy JSON" small />
            <DownloadButton filename="iris-power-day.csv" content={timelineCsv} label="Day CSV" small />
          </div>
          <div className="muted small" style={{ marginTop: 6 }}>{res.ms.toFixed(0)} ms for training, dynamic programming and {3 * 24} simulated days</div>
          <ErrorBox error={res.error} />
        </Card>
        <Card title="Learned policy: what Iris does in each state" subtitle="Rows = what the wearer is doing, columns = battery charge · orange outline = differs from dynamic programming">
          {d && <PolicyGrid pol={d.policy} compare={d.dp_policy} />}
          <div className="plot-legend" style={{ paddingLeft: 0 }}>
            {[0, 1, 2].map((k) => <span className="lg" key={k}><span className="lg-swatch" style={{ background: DISPLAY_FILL(k), height: 10, width: 14, border: '1px solid var(--border)' }} />display {d?.displays[k]}</span>)}
          </div>
          {d && d.dp_policy.length > 0 && (
            <details style={{ marginTop: 8 }}>
              <summary className="small" style={{ cursor: 'pointer' }}>Show the dynamic-programming policy</summary>
              <PolicyGrid pol={d.dp_policy} />
            </details>
          )}
        </Card>
      </div>

      {d && (
        <Card title="A simulated day, minute by minute" subtitle="Same wearer behaviour for both policies (common random numbers); strips end when the battery is empty" actions={<span className="small muted">0 – {maxT.toFixed(1)} h</span>}>
          <div className="stack" style={{ gap: 14 }}>
            <DayTimeline e={d.fixed} maxT={maxT} res={d} />
            <DayTimeline e={d.learned} maxT={maxT} res={d} />
            <div className="plot-legend" style={{ paddingLeft: 70 }}>
              {d.contexts.map((c, i) => <span className="lg" key={c}><span className="lg-swatch" style={{ background: CTX_COLORS[i], height: 10, width: 14, border: '1px solid var(--border)' }} />{c}</span>)}
              <span className="lg"><span className="lg-swatch" style={{ background: 'var(--series-2)', height: 10, width: 14 }} />radio fast</span>
              <span className="lg"><span className="lg-swatch" style={{ background: mix('var(--series-2)', 'var(--surface-2)', 30), height: 10, width: 14 }} />radio slow</span>
            </div>
            <Plot series={socS} height={200} xLabel="time (h)" yLabel="battery (%)" xDomain={[0, maxT]} yDomain={[0, 102]} />
          </div>
        </Card>
      )}

      <div className="grid grid-2">
        <Card title="Life vs quality: sweeping the energy price λ" subtitle={frontier.busy ? 'Training one policy per λ…' : 'Each point is a separately trained policy'}>
          <Plot series={frontS} height={230} xLabel="battery life (h)" yLabel="utility served (%)" legend={false} vLines={d ? [{ value: d.fixed.life_h * (1 + goal / 100), label: `+${goal} % goal`, color: 'var(--good)' }] : []} />
          <div className="small muted">{(frontier.data ?? []).map((p) => `λ=${p.lambda}: ${p.life_h.toFixed(1)} h / ${(p.served * 100).toFixed(1)} %`).join(' · ')}</div>
        </Card>
        <Card title="Training" subtitle="Mean reward per step over each (exploring-start) episode">
          <Plot series={trainS} height={230} xLabel="episode" yLabel="reward / step" />
        </Card>
      </div>

      {m && (
        <div className="grid grid-2">
          <Card title="Energy model from the project BOM">
            <KV rows={[
              [`display ${m.display_ref} off / low / full`, `${m.display_mw.map((v) => v.toFixed(1)).join(' / ')} mW`],
              ...d!.contexts.map((c, i) => [`SoC ${m.soc_ref} · ${c} (fast / slow)`, `${m.soc_mw[i][0].toFixed(2)} / ${m.soc_mw[i][1].toFixed(2)} mW (duty ${(m.soc_duty[i][0] * 100).toFixed(0)} / ${(m.soc_duty[i][1] * 100).toFixed(0)} %)`] as [string, string]),
              ['all other parts (BOM duty average)', `${m.const_mw.toFixed(2)} mW`],
              ['buck efficiency', `${(m.buck_efficiency * 100).toFixed(0)} %`],
              ['usable battery energy', `${m.battery_mwh.toFixed(0)} mWh`],
              ['fixed-policy average (= BOM duty cycles)', `${m.p_ref_mw.toFixed(2)} mW → ${(m.battery_mwh / m.p_ref_mw).toFixed(2)} h`],
            ]} />
          </Card>
          <Card title="Wearer model (Markov chain, 1-minute steps)">
            <div className="table-wrap">
            <table className="table" style={{ fontSize: 12.5 }}>
              <thead><tr><th>from \ to</th>{d!.contexts.map((c) => <th key={c}>{c}</th>)}<th>share</th></tr></thead>
              <tbody>
                {m.trans.map((row, i) => (
                  <tr key={i}><td>{d!.contexts[i]}</td>{row.map((p, j) => <td key={j} className="mono">{p.toFixed(3)}</td>)}<td className="mono">{(m.shares[i] * 100).toFixed(1)} %</td></tr>
                ))}
              </tbody>
            </table>
            </div>
            <p className="small muted">Non-idle share = the display's duty cycle in the BOM ({((1 - m.shares[0]) * 100).toFixed(0)} %). Glances last ~1.5 min, navigation ~8 min, captions ~10 min.</p>
          </Card>
        </div>
      )}

      <div className="grid grid-2">
        <Explainer title="The MDP" refs={[{ label: 'Sutton & Barto, RL: An Introduction, ch. 3 & 6', href: 'http://incompleteideas.net/book/the-book-2nd.html' }, { label: 'Benini, Bogliolo & De Micheli, "A survey of design techniques for system-level dynamic power management", IEEE Trans. VLSI 2000' }]}>
          <Formula>{`state   s = (battery bucket, context ∈ {idle, glance, navigate, caption})
action  a = (display ∈ {off, low, full}) × (BLE interval ∈ {fast, slow})
reward  r = u(context, a) − λ (1 + β (1 − soc)²) · P(a)/P_ref
update  Q(s,a) ← Q(s,a) + α [r + γ max_a' Q(s',a') − Q(s,a)]`}</Formula>
          <p>Utilities: full display serves any need; low refresh serves glances and navigation arrows (UC-1 only redraws on change) but only 60 % of captions; slow BLE delays notifications (−0.05) and captions (−0.5). Because the wearer's behaviour does not depend on our action, every action can be updated with the same sampled next context — a post-decision-state trick that removes most sampling noise (toggle it off to see plain Q-learning struggle).</p>
        </Explainer>
        <Explainer title="How honest is the number?">
          <p><b>Traceable</b>: the fixed policy reproduces the BOM duty-cycle average power exactly, and every milliwatt comes from <span className="mono">p.components</span>. <b>Assumed</b>: the low-refresh power fraction ({(cfg.low_refresh_factor * 100).toFixed(0)} %), the SoC activity per context and the utility weights — the gain is most sensitive to the low-refresh fraction, so measure it on the real panel. The metric is set to 0 whenever the learned policy serves less utility than the fixed one (0.5 % tolerance). For production, graduate to Gymnasium environments with Stable-Baselines3 and real usage logs.</p>
        </Explainer>
      </div>
    </div>
  );
}
