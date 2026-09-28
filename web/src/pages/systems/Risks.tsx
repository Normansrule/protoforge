// Risks: Failure Mode and Effects Analysis (FMEA) table sorted by risk
// priority number (RPN), a severity × occurrence heat map and an editor.
import { useMemo, useState } from 'react';
import { Card, Explainer, Formula } from '../../components/ui';
import type { Risk } from '../../engine/types';
import { Badge, DText, Eyebrow, REFS, nextId, useStudio } from './shared';

const rpnOf = (r: Risk) => r.severity * r.occurrence * r.detection;
export const levelOf = (r: Risk): 'high' | 'medium' | 'low' => {
  const rpn = rpnOf(r);
  return rpn >= 120 || r.severity >= 9 ? 'high' : rpn >= 60 ? 'medium' : 'low';
};
const LEVEL_STATUS = { high: 'fail', medium: 'warn', low: 'pass' } as const;

const SCALES = {
  severity: '1 = no effect … 10 = hazardous without warning',
  occurrence: '1 = extremely unlikely … 10 = almost certain',
  detection: '1 = certain to be caught … 10 = cannot be detected',
};

function Score({ value, onChange, label }: { value: number; onChange: (v: number) => void; label: string }) {
  return (
    <select className="ss-sod" value={value} aria-label={label} onChange={(e) => onChange(Number(e.target.value))} onClick={(e) => e.stopPropagation()}>
      {Array.from({ length: 10 }, (_, i) => i + 1).map((v) => <option key={v} value={v}>{v}</option>)}
    </select>
  );
}

export function RisksTab() {
  const { project, update } = useStudio();
  const [sel, setSel] = useState<string | null>(project.risks[0]?.id ?? null);
  const [sortBy, setSortBy] = useState<'rpn' | 'id'>('rpn');
  const rows = useMemo(() => {
    const r = project.risks.map((x, i) => ({ x, i }));
    return sortBy === 'rpn' ? r.sort((a, b) => rpnOf(b.x) - rpnOf(a.x) || a.x.id.localeCompare(b.x.id)) : r;
  }, [project.risks, sortBy]);
  const set = (i: number, fn: (r: Risk) => void) => update((d) => { fn(d.risks[i]); });
  const selIdx = project.risks.findIndex((r) => r.id === sel);
  const selected = selIdx >= 0 ? project.risks[selIdx] : null;
  const counts = { high: 0, medium: 0, low: 0 };
  project.risks.forEach((r) => counts[levelOf(r)]++);
  const add = () => {
    const id = nextId('F-', project.risks.map((r) => r.id), 1);
    update((d) => { d.risks.push({ id, item: 'New item', failure_mode: 'Failure mode', effect: '', cause: '', severity: 5, occurrence: 5, detection: 5, mitigation: '' }); });
    setSel(id);
  };

  return (
    <>
      <div className="grid grid-2">
        <Card title="Risk matrix" subtitle="Severity × occurrence; each marker is a failure mode (detection shown in the RPN)">
          <Heatmap risks={project.risks} sel={sel} onSel={setSel} />
        </Card>
        <Card
          title={selected ? <span className="row" style={{ gap: 8 }}><span className="ss-id">{selected.id}</span> {selected.item}</span> : 'Failure mode'}
          subtitle={selected ? <>RPN {rpnOf(selected)} = {selected.severity} × {selected.occurrence} × {selected.detection}</> : 'Select a risk'}
          actions={selected && <Badge status={LEVEL_STATUS[levelOf(selected)]} label={`${levelOf(selected)} risk`} />}
        >
          {selected ? (
            <div className="ss-form">
              <div className="two">
                <label className="f">Item / function<DText value={selected.item} onCommit={(v) => set(selIdx, (r) => { r.item = v; })} /></label>
                <label className="f">Failure mode<DText value={selected.failure_mode} onCommit={(v) => set(selIdx, (r) => { r.failure_mode = v; })} /></label>
              </div>
              <div className="two">
                <label className="f">Effect<DText value={selected.effect} onCommit={(v) => set(selIdx, (r) => { r.effect = v; })} /></label>
                <label className="f">Cause<DText value={selected.cause} onCommit={(v) => set(selIdx, (r) => { r.cause = v; })} /></label>
              </div>
              <div className="three" style={{ gridTemplateColumns: '1fr 1fr 1fr' }}>
                {(['severity', 'occurrence', 'detection'] as const).map((k) => (
                  <label className="f" key={k} title={SCALES[k]}>{k[0].toUpperCase() + k.slice(1)} ({k[0].toUpperCase()})
                    <Score value={selected[k]} label={k} onChange={(v) => set(selIdx, (r) => { r[k] = v; })} />
                    <span className="muted small">{SCALES[k]}</span>
                  </label>
                ))}
              </div>
              <label className="f">Mitigation / recommended action<DText area rows={3} value={selected.mitigation} onCommit={(v) => set(selIdx, (r) => { r.mitigation = v; })} /></label>
              <div className="row" style={{ justifyContent: 'flex-end' }}>
                <button className="btn small" style={{ color: 'var(--bad-text)' }} onClick={() => { if (confirm(`Delete ${selected.id}?`)) { update((d) => { d.risks.splice(selIdx, 1); }); setSel(null); } }}>Delete risk</button>
              </div>
            </div>
          ) : <div className="ss-empty">No risk selected.</div>}
        </Card>
      </div>

      <Card
        title="Failure Mode and Effects Analysis (FMEA)"
        subtitle={<>{project.risks.length} failure modes · {counts.high} high · {counts.medium} medium · {counts.low} low</>}
        actions={
          <>
            <div className="ss-seg" role="group" aria-label="Sort">
              <button className={sortBy === 'rpn' ? 'on' : ''} onClick={() => setSortBy('rpn')}>By RPN</button>
              <button className={sortBy === 'id' ? 'on' : ''} onClick={() => setSortBy('id')}>By ID</button>
            </div>
            <button className="btn small primary" onClick={add}>+ Risk</button>
          </>
        }
      >
        <div className="table-wrap">
          <table className="table">
            <thead><tr><th>ID</th><th>Item — failure mode</th><th>Effect</th><th title="Severity">S</th><th title="Occurrence">O</th><th title="Detection">D</th><th>RPN</th><th>Level</th><th>Mitigation</th></tr></thead>
            <tbody>
              {rows.map(({ x, i }) => (
                <tr key={x.id} className={sel === x.id ? 'ss-sel-row' : ''} style={{ cursor: 'pointer' }} onClick={() => setSel(x.id)}>
                  <td className="ss-id">{x.id}</td>
                  <td style={{ minWidth: 180 }}><b style={{ fontWeight: 600 }}>{x.item}</b> — {x.failure_mode}<div className="muted small">cause: {x.cause || '—'}</div></td>
                  <td className="small" style={{ minWidth: 140 }}>{x.effect}</td>
                  <td><Score value={x.severity} label={`${x.id} severity`} onChange={(v) => set(i, (r) => { r.severity = v; })} /></td>
                  <td><Score value={x.occurrence} label={`${x.id} occurrence`} onChange={(v) => set(i, (r) => { r.occurrence = v; })} /></td>
                  <td><Score value={x.detection} label={`${x.id} detection`} onChange={(v) => set(i, (r) => { r.detection = v; })} /></td>
                  <td className="mono" style={{ fontWeight: 800, fontSize: 14 }}>{rpnOf(x)}</td>
                  <td><Badge status={LEVEL_STATUS[levelOf(x)]} label={levelOf(x)} /></td>
                  <td className="small muted" style={{ minWidth: 220 }}>{x.mitigation}</td>
                </tr>
              ))}
              {rows.length === 0 && <tr><td colSpan={9}><div className="ss-empty">No risks recorded yet.</div></td></tr>}
            </tbody>
          </table>
        </div>
      </Card>

      <div className="grid grid-2">
        <Explainer title="FMEA and the risk priority number" refs={[REFS.fmea, REFS.nasa]}>
          For every item, ask how it can fail (failure mode), what happens then (effect) and why (cause). Rate <b>severity</b> S of the effect, likelihood of
          <b> occurrence</b> O of the cause and how poorly current controls <b>detect</b> it before it reaches the user, D — each 1 to 10.
          <Formula>{'RPN = S × O × D        (1 … 1000)'}</Formula>
          Here a risk is <i>high</i> when RPN ≥ 120 or severity ≥ 9 (a safety effect is never averaged away), <i>medium</i> when RPN ≥ 60. The 2019 AIAG &amp; VDA
          handbook recommends an <i>Action Priority</i> table over raw RPN for the same reason: severity should dominate.
        </Explainer>
        <Explainer title="From risk to requirement" refs={[REFS.nasa, REFS.incose]}>
          <Eyebrow>Closing the loop</Eyebrow>
          Good mitigations become design changes and requirements: the Iris battery risk (F-1) is why the skin-temperature requirement exists and why the charger
          needs a thermistor input; the hinge-flex risk becomes a fold-cycle test in design validation. Re-rate S, O and D after each mitigation — only occurrence
          and detection can realistically go down.
        </Explainer>
      </div>
    </>
  );
}

function Heatmap({ risks, sel, onSel }: { risks: Risk[]; sel: string | null; onSel: (id: string) => void }) {
  const zone = (s: number, o: number) => (s * o >= 50 || s >= 9 ? 'var(--bad)' : s * o >= 20 ? 'var(--warn)' : 'var(--good)');
  const alpha = (s: number, o: number) => 8 + Math.round(((s * o) / 100) * 30);
  return (
    <div>
      <div style={{ display: 'grid', gridTemplateColumns: '22px minmax(0, 1fr)', gap: 6 }}>
        <div style={{ writingMode: 'vertical-rl', transform: 'rotate(180deg)', textAlign: 'center', fontSize: 11, color: 'var(--muted)' }}>Severity →</div>
        <div>
          <div className="ss-heat" style={{ gridTemplateColumns: '18px repeat(10, minmax(0, 1fr))' }}>
            {Array.from({ length: 10 }, (_, r) => 10 - r).map((s) => (
              <div key={s} style={{ display: 'contents' }}>
                <div className="muted small mono" style={{ alignSelf: 'center', textAlign: 'right' }}>{s}</div>
                {Array.from({ length: 10 }, (_, c) => c + 1).map((o) => {
                  const here = risks.filter((x) => x.severity === s && x.occurrence === o);
                  return (
                    <div key={o} className="ss-heat-cell" style={{ background: `color-mix(in srgb, ${zone(s, o)} ${alpha(s, o)}%, var(--surface-2))` }} title={`S ${s} × O ${o}`}>
                      {here.map((x) => (
                        <button key={x.id} className={`ss-heat-mark ${sel === x.id ? 'sel' : ''}`} onClick={() => onSel(x.id)} title={`${x.id}: ${x.item} — ${x.failure_mode} (RPN ${rpnOf(x)})`}>{x.id.replace(/^F-/, '')}</button>
                      ))}
                    </div>
                  );
                })}
              </div>
            ))}
            <div />
            {Array.from({ length: 10 }, (_, c) => <div key={c} className="muted small mono" style={{ textAlign: 'center' }}>{c + 1}</div>)}
          </div>
          <div className="muted small" style={{ textAlign: 'center', marginTop: 2 }}>Occurrence →</div>
        </div>
      </div>
      <div className="ss-legend">
        <span className="i"><span className="ss-sw" style={{ background: 'color-mix(in srgb, var(--good) 30%, var(--surface-2))' }} />✓ low zone (S×O &lt; 20)</span>
        <span className="i"><span className="ss-sw" style={{ background: 'color-mix(in srgb, var(--warn) 35%, var(--surface-2))' }} />! watch (S×O ≥ 20)</span>
        <span className="i"><span className="ss-sw" style={{ background: 'color-mix(in srgb, var(--bad) 35%, var(--surface-2))' }} />✕ act (S×O ≥ 50 or S ≥ 9)</span>
      </div>
    </div>
  );
}
