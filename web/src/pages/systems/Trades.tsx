// Trade studies: weighted decision matrices with live scores, the recorded
// decision vs the computed winner, a weight-sensitivity tornado and a radar.
import { useEffect, useRef, useState } from 'react';
import { Card, Explainer } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import type { TradeStudy } from '../../engine/types';
import { seriesColor } from '../../components/Plot';
import { Badge, DNum, DText, REFS, nextId, useStudio } from './shared';
import type { TradeDetail } from './types';

export function TradesTab() {
  const { project, update } = useStudio();
  const [idx, setIdx] = useState(0);
  const i = Math.min(idx, Math.max(0, project.trades.length - 1));
  const study = project.trades[i];
  const addStudy = () => {
    update((d) => {
      d.trades.push({
        id: nextId('T-', d.trades.map((t) => t.id), 1), title: 'New trade study', decision: '',
        criteria: [{ name: 'Performance', weight: 0.5, higher_is_better: true }, { name: 'Cost', weight: 0.5, higher_is_better: true }],
        options: [{ name: 'Option A', scores: [5, 5], notes: '' }, { name: 'Option B', scores: [5, 5], notes: '' }],
      });
    });
    setIdx(project.trades.length);
  };
  return (
    <>
      <div className="ss-toolbar">
        <div className="ss-seg" role="tablist" aria-label="Trade studies" style={{ flexWrap: 'wrap' }}>
          {project.trades.map((t, k) => (
            <button key={t.id} className={k === i ? 'on' : ''} onClick={() => setIdx(k)}>{t.id} · {t.title}</button>
          ))}
        </div>
        <button className="btn small" onClick={addStudy}>+ Trade study</button>
      </div>
      {study ? <StudyEditor key={study.id} index={i} study={study} /> : <div className="ss-empty">No trade studies yet. A trade study compares design options against weighted criteria — add one.</div>}
      <div className="grid grid-2">
        <Explainer title="Weighted decision matrices" refs={[REFS.saaty, REFS.nasa, REFS.incose]}>
          Each option is scored 1–10 against each criterion; the weighted score is Σ wᵢ·sᵢ / Σ wᵢ (criteria where lower raw values are better are flipped to 11 − s).
          The NASA handbook's decision-analysis process recommends recording the criteria, weights and rationale so the decision can be revisited. Saaty's Analytic
          Hierarchy Process (AHP) derives weights from pairwise comparisons; here you set them directly with sliders.
        </Explainer>
        <Explainer title="Is the decision robust?" refs={[REFS.nasa]}>
          Weights are judgements, so check how much they would have to change before a different option wins. The tornado scales each criterion's weight by
          ±50 % and plots the winner's lead over the runner-up; a bar crossing zero means that swing alone flips the decision. The engine also reports the
          smallest relative weight change that flips the winner (“flips at …”).
        </Explainer>
      </div>
    </>
  );
}

function StudyEditor({ index, study }: { index: number; study: TradeStudy }) {
  const { update } = useStudio();
  // local draft so slider drags stay smooth; committed to the project after a pause
  const [draft, setDraft] = useState<TradeStudy>(study);
  const draftRef = useRef(study);
  const dirty = useRef(false);
  const timer = useRef<number | null>(null);
  useEffect(() => { if (!dirty.current) { draftRef.current = study; setDraft(study); } }, [study]);
  useEffect(() => () => { if (timer.current) window.clearTimeout(timer.current); }, []);
  const edit = (fn: (t: TradeStudy) => void, immediate = false) => {
    const n = structuredClone(draftRef.current);
    fn(n);
    draftRef.current = n;
    dirty.current = true;
    setDraft(n);
    if (timer.current) window.clearTimeout(timer.current);
    const commit = () => { dirty.current = false; update((d) => { d.trades[index] = draftRef.current; }); };
    if (immediate) commit(); else timer.current = window.setTimeout(commit, 450);
  };
  const det = useCompute<TradeDetail>('systems.trade_detail', draft);
  const d = det.data;
  const winner = d ? draft.options[d.result.winner] : null;
  const decisionIdx = draft.options.findIndex((o) => o.name.trim() === draft.decision.trim());

  return (
    <>
      <Card
        title={<DText value={draft.title} ariaLabel="Study title" onCommit={(v) => edit((t) => { t.title = v; }, true)} style={{ fontWeight: 650, fontSize: 15 }} />}
        subtitle={<>{draft.id} · {draft.options.length} options × {draft.criteria.length} criteria</>}
        actions={
          <>
            <button className="btn small" onClick={() => edit((t) => { t.criteria.push({ name: `Criterion ${t.criteria.length + 1}`, weight: 0.2, higher_is_better: true }); t.options.forEach((o) => o.scores.push(5)); }, true)}>+ Criterion</button>
            <button className="btn small" onClick={() => edit((t) => { t.options.push({ name: `Option ${String.fromCharCode(65 + t.options.length)}`, scores: t.criteria.map(() => 5), notes: '' }); }, true)}>+ Option</button>
            <button className="btn small ghost" style={{ color: 'var(--bad-text)' }} onClick={() => { if (confirm(`Delete ${draft.id}?`)) update((p) => { p.trades.splice(index, 1); }); }}>Delete study</button>
          </>
        }
      >
        <div className="table-wrap">
          <table className="table ss-trade">
            <thead>
              <tr>
                <th style={{ minWidth: 230 }}>Option</th>
                {draft.criteria.map((c, j) => (
                  <th key={j} style={{ minWidth: 150, textTransform: 'none', letterSpacing: 0 }}>
                    <div className="row" style={{ flexWrap: 'nowrap', gap: 4 }}>
                      <span className="ss-sw" style={{ background: seriesColor(j), flex: '0 0 10px' }} />
                      <DText value={c.name} ariaLabel={`Criterion ${j + 1} name`} onCommit={(v) => edit((t) => { t.criteria[j].name = v; }, true)} />
                      <button className="btn ghost small ss-x" aria-label={`Remove ${c.name}`} onClick={() => edit((t) => { t.criteria.splice(j, 1); t.options.forEach((o) => o.scores.splice(j, 1)); }, true)}>✕</button>
                    </div>
                    <input type="range" min={0} max={1} step={0.01} value={c.weight} aria-label={`${c.name} weight`} onChange={(e) => edit((t) => { t.criteria[j].weight = Number(e.target.value); })} />
                    <div className="row small" style={{ justifyContent: 'space-between', fontWeight: 500 }}>
                      <span className="mono">w {c.weight.toFixed(2)} · {d ? `${Math.round(d.weights[j] * 100)} %` : ''}</span>
                      <button className="btn ghost small" style={{ padding: '0 4px' }} title="Toggle whether higher raw scores are better" onClick={() => edit((t) => { t.criteria[j].higher_is_better = !(t.criteria[j].higher_is_better ?? true); }, true)}>
                        {(c.higher_is_better ?? true) ? '↑ better' : '↓ better'}
                      </button>
                    </div>
                  </th>
                ))}
                <th>Score</th>
                <th>Rank</th>
              </tr>
            </thead>
            <tbody>
              {draft.options.map((o, k) => {
                const rank = d ? d.ranking.indexOf(k) + 1 : 0;
                const win = d?.result.winner === k;
                return (
                  <tr key={k} className={win ? 'win' : ''}>
                    <td>
                      <div className="row" style={{ flexWrap: 'nowrap', gap: 4 }}>
                        <span className="ss-sw" style={{ background: seriesColor(k), borderRadius: '50%', flex: '0 0 10px' }} />
                        <DText value={o.name} ariaLabel={`Option ${k + 1} name`} onCommit={(v) => edit((t) => { if (t.decision === t.options[k].name) t.decision = v; t.options[k].name = v; }, true)} />
                        <button className="btn ghost small ss-x" aria-label={`Remove ${o.name}`} onClick={() => edit((t) => { t.options.splice(k, 1); }, true)}>✕</button>
                      </div>
                      <DText value={o.notes} placeholder="notes" ariaLabel={`${o.name} notes`} className="small" onCommit={(v) => edit((t) => { t.options[k].notes = v; }, true)} style={{ marginTop: 4, fontSize: 12 }} />
                    </td>
                    {draft.criteria.map((_, j) => (
                      <td key={j}><DNum value={o.scores[j] ?? 0} min={1} max={10} step={1} ariaLabel={`${o.name} ${draft.criteria[j].name} score`} onCommit={(v) => edit((t) => { t.options[k].scores[j] = v; }, true)} /></td>
                    ))}
                    <td className="mono" style={{ fontWeight: 700 }}>{d ? d.result.scores[k].toFixed(2) : '…'}</td>
                    <td>{win ? <Badge status="pass" label="winner" /> : <span className="muted">#{rank}</span>}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
        <div className="row" style={{ marginTop: 12, gap: 12 }}>
          <label className="small" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            Recorded decision
            <select value={decisionIdx >= 0 ? draft.decision : ''} style={{ width: 'auto' }} onChange={(e) => edit((t) => { t.decision = e.target.value; }, true)}>
              <option value="">— undecided —</option>
              {draft.options.map((o, k) => <option key={k} value={o.name}>{o.name}</option>)}
            </select>
          </label>
          {d && winner && (
            d.decision_agrees
              ? (draft.decision ? <Badge status="pass" label={`decision matches the weighted winner (lead ${d.lead.toFixed(2)})`} /> : <Badge status="todo" label={`weighted winner: ${winner.name}`} />)
              : <Badge status="warn" label={`recorded “${draft.decision}” ≠ weighted winner “${winner.name}”`} title="Either the weights do not capture why this option was chosen, or the decision should be revisited" />
          )}
        </div>
      </Card>
      {d && (
        <div className="grid grid-3">
          <Card title="Weighted scores" subtitle="Each bar is split into the criteria's weighted contributions">
            <ScoreBars study={draft} d={d} />
          </Card>
          <Card title="Weight sensitivity (tornado)" subtitle={`Winner's lead over the runner-up when one weight is scaled ±${Math.round(d.swing * 100)} %`}>
            <Tornado study={draft} d={d} />
          </Card>
          <Card title="Option profiles" subtitle="Direction-adjusted scores (outer = better)">
            <Radar study={draft} />
          </Card>
        </div>
      )}
    </>
  );
}

function ScoreBars({ study, d }: { study: TradeStudy; d: TradeDetail }) {
  const [hover, setHover] = useState<string | null>(null);
  return (
    <div className="stack" style={{ gap: 10 }}>
      {d.ranking.map((k) => {
        const o = study.options[k];
        return (
          <div key={k}>
            <div className="row small" style={{ justifyContent: 'space-between' }}>
              <span style={{ fontWeight: 600 }}>{d.result.winner === k ? '★ ' : ''}{o.name}</span>
              <span className="mono">{d.result.scores[k].toFixed(2)}</span>
            </div>
            <div className="ss-stack" style={{ height: 14, background: 'transparent' }}>
              {d.contributions[k].map((c, j) => (
                <span key={j} style={{ width: `${(c / 10) * 100}%`, background: seriesColor(j), borderRadius: 3 }}
                  onMouseEnter={() => setHover(`${o.name} · ${study.criteria[j]?.name}: ${c.toFixed(2)} (weight ${Math.round(d.weights[j] * 100)} % × score ${o.scores[j]})`)}
                  onMouseLeave={() => setHover(null)} />
              ))}
            </div>
          </div>
        );
      })}
      <div className="ss-legend" style={{ marginTop: 2 }}>
        {study.criteria.map((c, j) => <span className="i" key={j}><span className="ss-sw" style={{ background: seriesColor(j) }} />{c.name}</span>)}
      </div>
      <div className="muted small" style={{ minHeight: 18 }}>{hover ?? 'Hover a segment for its contribution. Scale 0–10.'}</div>
    </div>
  );
}

function Tornado({ study, d }: { study: TradeStudy; d: TradeDetail }) {
  const rows = d.tornado;
  const vals = rows.flatMap((r) => [r.lo_lead, r.hi_lead, r.base_lead, 0]);
  const lo = Math.min(...vals), hi = Math.max(...vals);
  const span = Math.max(hi - lo, 0.2);
  const W = 300, L = 96, R = 10, RH = 30;
  const x = (v: number) => L + ((v - (lo - span * 0.08)) / (span * 1.16)) * (W - L - R);
  const H = rows.length * RH + 30;
  return (
    <div>
      <svg width="100%" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Tornado chart of weight sensitivity">
        <line x1={x(0)} x2={x(0)} y1={4} y2={H - 22} stroke="var(--bad)" strokeDasharray="3 3" />
        <text x={x(0)} y={H - 8} fontSize={9.5} textAnchor="middle" fill="var(--bad-text)">flip</text>
        <line x1={x(d.lead)} x2={x(d.lead)} y1={4} y2={H - 22} stroke="var(--text-2)" />
        <text x={x(d.lead)} y={H - 8} fontSize={9.5} textAnchor="middle" fill="var(--text-2)">now {d.lead.toFixed(2)}</text>
        {rows.map((r, i) => {
          const y = 8 + i * RH;
          const a = x(Math.min(r.lo_lead, r.base_lead)), b = x(Math.max(r.lo_lead, r.base_lead));
          const c = x(Math.min(r.hi_lead, r.base_lead)), e = x(Math.max(r.hi_lead, r.base_lead));
          return (
            <g key={r.criterion}>
              <title>{`${r.name}: weight −${Math.round(d.swing * 100)} % → lead ${r.lo_lead.toFixed(2)} (${study.options[r.lo_winner]?.name} wins); +${Math.round(d.swing * 100)} % → lead ${r.hi_lead.toFixed(2)} (${study.options[r.hi_winner]?.name} wins)${r.flip != null ? `; flips at ${r.flip > 0 ? '+' : ''}${Math.round(r.flip * 100)} %` : ''}`}</title>
              <text x={L - 6} y={y + 11} fontSize={10.5} textAnchor="end" fill="var(--text)">{r.name.length > 14 ? r.name.slice(0, 13) + '…' : r.name}</text>
              <rect x={a} y={y + 2} width={Math.max(1.5, b - a)} height={12} rx={3} fill="var(--series-1)" opacity={0.9} />
              <rect x={c} y={y + 2} width={Math.max(1.5, e - c)} height={12} rx={3} fill="var(--series-2)" opacity={0.9} />
              <text x={L - 6} y={y + 23} fontSize={9} textAnchor="end" fill="var(--muted)">{r.flip != null ? `flips at ${r.flip > 0 ? '+' : ''}${Math.round(r.flip * 100)} %` : 'robust'}</text>
            </g>
          );
        })}
      </svg>
      <div className="ss-legend">
        <span className="i"><span className="ss-sw" style={{ background: 'var(--series-1)' }} />weight −{Math.round(d.swing * 100)} %</span>
        <span className="i"><span className="ss-sw" style={{ background: 'var(--series-2)' }} />weight +{Math.round(d.swing * 100)} %</span>
      </div>
    </div>
  );
}

function Radar({ study }: { study: TradeStudy }) {
  const n = study.criteria.length;
  const S = 320, C = S / 2, R = 92;
  if (n < 3) return <div className="muted small">Add at least three criteria for a radar view.</div>;
  const ang = (j: number) => -Math.PI / 2 + (j / n) * 2 * Math.PI;
  const pt = (j: number, v: number) => [C + Math.cos(ang(j)) * (R * v) / 10, C + Math.sin(ang(j)) * (R * v) / 10];
  return (
    <div>
      <svg width="100%" viewBox={`0 0 ${S} ${S}`} style={{ maxWidth: 360, display: 'block', margin: '0 auto' }} role="img" aria-label="Radar chart of option scores">
        {[2, 4, 6, 8, 10].map((g) => (
          <polygon key={g} points={study.criteria.map((_, j) => pt(j, g).join(',')).join(' ')} fill="none" stroke="var(--grid)" />
        ))}
        {study.criteria.map((c, j) => {
          const [x, y] = pt(j, 10);
          const [lx, ly] = pt(j, 12.4);
          return (
            <g key={j}>
              <line x1={C} y1={C} x2={x} y2={y} stroke="var(--grid)" />
              <text x={lx} y={ly + 3} fontSize={10} textAnchor={Math.abs(lx - C) < 8 ? 'middle' : lx > C ? 'start' : 'end'} fill="var(--text-2)">{c.name.length > 18 ? c.name.slice(0, 17) + '…' : c.name}</text>
            </g>
          );
        })}
        {study.options.map((o, k) => (
          <polygon key={k} points={study.criteria.map((c, j) => pt(j, (c.higher_is_better ?? true) ? o.scores[j] ?? 0 : 11 - (o.scores[j] ?? 0)).join(',')).join(' ')}
            fill={seriesColor(k)} fillOpacity={0.1} stroke={seriesColor(k)} strokeWidth={2} strokeLinejoin="round">
            <title>{o.name}</title>
          </polygon>
        ))}
      </svg>
      <div className="ss-legend" style={{ justifyContent: 'center' }}>
        {study.options.map((o, k) => <span className="i" key={k}><span className="ss-sw" style={{ background: seriesColor(k), borderRadius: '50%' }} />{o.name}</span>)}
      </div>
    </div>
  );
}
