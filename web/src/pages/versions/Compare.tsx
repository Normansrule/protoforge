// Diff view for two versions: story, requirement status with ✓/✕ glyphs,
// parameter table, BOM diff, Circuit Lab checks and a metric-change chart.
import { useState } from 'react';
import { Card, Chip, Stat } from '../../components/ui';
import { fmtNum } from '../../components/Plot';
import { useElementWidth } from '../../components/useElementWidth';

interface ValueChange { path: string; old: unknown; new: unknown; delta: number | null; rel_pct: number | null }
interface ItemChange { kind: 'added' | 'removed' | 'changed'; key: string; label: string; fields: ValueChange[] }
interface Section { id: string; title: string; items: ItemChange[] }
interface ReqRow { id: string; text: string; criterion: string; status_a: string | null; status_b: string | null; value_a: number | null; value_b: number | null; margin_a?: number | null; margin_b?: number | null; unit: string; change: string }
interface MetricChange { key: string; label: string; unit: string; source: string; old: number | null; new: number | null; delta: number | null; rel_pct: number | null; note: string }
interface CheckChange { preset: string; preset_title: string; key: string; label: string; unit: string; old: number | null; new: number | null; pass_old: boolean | null; pass_new: boolean | null; note: string; changed: boolean }
interface Sum { short: string; name: string; version: string; evaluation: { pass: number; fail: number; unverified: number } | null }
export interface Diff {
  a: Sum; b: Sum; identical: boolean; meta: ValueChange[]; concept: ValueChange[]; params: ValueChange[]; sections: Section[];
  requirements_status: ReqRow[]; metrics: MetricChange[]; circuit_checks: CheckChange[];
  counts: { params: number; items_added: number; items_removed: number; items_changed: number; verdicts_changed: number; metrics_changed: number; checks_changed: number };
  headline: string[];
}

const val = (v: unknown) => (v === null || v === undefined ? '—' : typeof v === 'number' ? fmtNum(v) : typeof v === 'string' ? v : JSON.stringify(v));

function Glyph({ s }: { s: string | null }) {
  if (s === 'pass') return <span className="vd-pass">✓ pass</span>;
  if (s === 'fail') return <span className="vd-fail">✕ fail</span>;
  if (s === 'unverified') return <span className="muted">○ unverified</span>;
  return <span className="muted">— absent</span>;
}

const CHANGE: Record<string, { tone?: 'pass' | 'fail' | 'warn' | 'info'; text: string }> = {
  improved: { tone: 'pass', text: '▲ improved' },
  regressed: { tone: 'fail', text: '▼ regressed' },
  added: { tone: 'info', text: '+ added' },
  removed: { tone: 'warn', text: '− removed' },
  value: { text: 'Δ value' },
  same: { text: '= same' },
};

function MetricChart({ metrics }: { metrics: MetricChange[] }) {
  const [ref, w] = useElementWidth<HTMLDivElement>();
  const rows = metrics.filter((m) => m.rel_pct !== null).slice(0, 12);
  if (rows.length === 0) return <div className="muted small">No numeric metric changed.</div>;
  const max = Math.min(100, Math.max(5, ...rows.map((m) => Math.abs(m.rel_pct!))));
  const W = Math.max(300, w);
  const labelW = Math.min(230, W * 0.42);
  const barW = W - labelW - 70;
  const cx = labelW + barW / 2;
  const H = rows.length * 26 + 22;
  return (
    <div ref={ref}>
      <svg width="100%" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Relative change of each metric">
        <line x1={cx} y1={4} x2={cx} y2={H - 16} stroke="var(--axis)" />
        {[-max, max].map((t) => <text key={t} x={cx + (t / max) * (barW / 2)} y={H - 4} textAnchor="middle" className="vd-tick">{t > 0 ? '+' : ''}{t.toFixed(0)} %</text>)}
        {rows.map((m, i) => {
          const r = Math.max(-max, Math.min(max, m.rel_pct!));
          const len = (Math.abs(r) / max) * (barW / 2);
          const y = 8 + i * 26;
          const up = r >= 0;
          return (
            <g key={m.key}>
              <title>{`${m.label}: ${fmtNum(m.old ?? NaN)} → ${fmtNum(m.new ?? NaN)} ${m.unit} (${m.rel_pct!.toFixed(1)} %)`}</title>
              <text x={labelW - 8} y={y + 13} textAnchor="end" className="vd-lbl">{m.label.length > 34 ? m.label.slice(0, 33) + '…' : m.label}</text>
              <rect x={up ? cx : cx - len} y={y + 2} width={Math.max(1.5, len)} height={16} rx={3} fill={up ? 'var(--series-1)' : 'var(--series-2)'} />
              <text x={up ? cx + len + 5 : cx - len - 5} y={y + 14} textAnchor={up ? 'start' : 'end'} className="vd-val">{up ? '+' : '−'}{Math.abs(m.rel_pct!) >= 999 ? '>999' : Math.abs(m.rel_pct!).toFixed(Math.abs(m.rel_pct!) < 10 ? 1 : 0)} %</text>
            </g>
          );
        })}
      </svg>
      <div className="small muted">Relative change B vs A, clipped at ±{max.toFixed(0)} % (blue ▲ increase, orange ▼ decrease; hover for values). Sign alone does not say better or worse — see the requirement table.</div>
    </div>
  );
}

function Items({ s }: { s: Section }) {
  return (
    <div className="table-wrap">
      <table className="table">
        <thead><tr><th>Change</th><th>Id</th><th>Item</th><th>Fields</th></tr></thead>
        <tbody>
          {s.items.map((it) => (
            <tr key={it.key + it.kind}>
              <td><Chip tone={it.kind === 'added' ? 'info' : it.kind === 'removed' ? 'warn' : undefined}>{it.kind === 'added' ? '+ added' : it.kind === 'removed' ? '− removed' : 'Δ changed'}</Chip></td>
              <td className="mono">{it.key}</td>
              <td className="small">{it.label}</td>
              <td className="small">
                {it.kind === 'changed'
                  ? it.fields.map((f) => <div key={f.path}><span className="mono">{f.path}</span>: <span className="vd-old">{val(f.old)}</span> → <b>{val(f.new)}</b></div>)
                  : it.kind === 'added' ? <span className="muted">{it.fields.length} fields</span> : null}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function CompareView({ d }: { d: Diff }) {
  const [all, setAll] = useState(false);
  const reqRows = d.requirements_status.filter((r) => all || r.change !== 'same');
  const bom = d.sections.find((s) => s.id === 'components')!;
  const others = d.sections.filter((s) => s.id !== 'components' && s.items.length > 0);
  const ea = d.a.evaluation;
  const eb = d.b.evaluation;
  return (
    <div className="stack">
      <div className="grid grid-4">
        <Stat label="Design parameters" value={d.counts.params} note="changed values in design.*" />
        <Stat label="Verdict changes" value={d.counts.verdicts_changed} note={ea && eb ? `A ✓${ea.pass} ✕${ea.fail} → B ✓${eb.pass} ✕${eb.fail}` : ''} tone={eb && ea && eb.fail > ea.fail ? 'fail' : d.counts.verdicts_changed ? 'pass' : undefined} />
        <Stat label="Items changed" value={d.counts.items_added + d.counts.items_removed + d.counts.items_changed} note={`+${d.counts.items_added} −${d.counts.items_removed} Δ${d.counts.items_changed} (requirements, BOM, risks …)`} />
        <Stat label="Metrics moved" value={d.counts.metrics_changed} note={`${d.counts.checks_changed} Circuit Lab checks changed`} />
      </div>
      <Card title="What changed, and what it did" subtitle={`A = ${d.a.name} v${d.a.version} (#${d.a.short}) → B = ${d.b.name} v${d.b.version} (#${d.b.short})`}>
        <ul className="vd-story">{d.headline.map((h, i) => <li key={i}>{h}</li>)}</ul>
      </Card>
      <Card title="Requirement status" subtitle="Both versions evaluated by every lab" actions={<button className="btn small" onClick={() => setAll(!all)}>{all ? 'Only changes' : `Show all ${d.requirements_status.length}`}</button>}>
        {reqRows.length === 0 ? <div className="muted small">No requirement changed status or value.</div> : (
          <div className="table-wrap">
            <table className="table">
              <thead><tr><th>Id</th><th>Requirement</th><th>A</th><th>B</th><th>Change</th></tr></thead>
              <tbody>
                {reqRows.map((r) => (
                  <tr key={r.id}>
                    <td className="mono">{r.id}</td>
                    <td className="small">{r.text}<div className="muted">{r.criterion}</div></td>
                    <td className="small"><Glyph s={r.status_a} />{r.value_a !== null && <div className="tnum">{fmtNum(r.value_a)} {r.unit}</div>}{r.margin_a != null && <div className="muted tnum">margin {(100 * r.margin_a).toFixed(1)} %</div>}</td>
                    <td className="small"><Glyph s={r.status_b} />{r.value_b !== null && <div className="tnum">{fmtNum(r.value_b)} {r.unit}</div>}{r.margin_b != null && <div className="muted tnum">margin {(100 * r.margin_b).toFixed(1)} %</div>}</td>
                    <td><Chip tone={CHANGE[r.change]?.tone}>{CHANGE[r.change]?.text ?? r.change}</Chip></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Card>
      <div className="grid grid-2">
        <Card title="Design parameters" subtitle="design.* values that differ">
          {d.params.length === 0 ? <div className="muted small">No design parameter changed.</div> : (
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>Parameter</th><th style={{ textAlign: 'right' }}>A</th><th style={{ textAlign: 'right' }}>B</th><th style={{ textAlign: 'right' }}>Δ</th></tr></thead>
                <tbody>
                  {d.params.map((p) => (
                    <tr key={p.path}>
                      <td className="mono small">{p.path.replace(/^design\./, '')}</td>
                      <td className="tnum vd-old" style={{ textAlign: 'right' }}>{val(p.old)}</td>
                      <td className="tnum" style={{ textAlign: 'right' }}><b>{val(p.new)}</b></td>
                      <td className="tnum small" style={{ textAlign: 'right' }}>{p.delta !== null ? `${p.delta > 0 ? '+' : ''}${fmtNum(p.delta)}${p.rel_pct !== null ? ` (${p.rel_pct > 0 ? '+' : ''}${p.rel_pct.toFixed(0)} %)` : ''}` : ''}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Card>
        <Card title="Metric changes" subtitle="Every lab metric that moved">
          <MetricChart metrics={d.metrics} />
        </Card>
      </div>
      <Card title="Bill of materials" subtitle={`${bom.items.length} line(s) differ`}>
        {bom.items.length === 0 ? <div className="muted small">Same BOM.</div> : <Items s={bom} />}
      </Card>
      {d.circuit_checks.length > 0 && (
        <Card title="Circuit Lab checks re-run" subtitle="Presets whose inputs come from a changed component value">
          <div className="table-wrap">
            <table className="table">
              <thead><tr><th>Circuit</th><th>Check</th><th style={{ textAlign: 'right' }}>A</th><th style={{ textAlign: 'right' }}>B</th><th>Note</th></tr></thead>
              <tbody>
                {d.circuit_checks.filter((c) => c.changed).map((c) => (
                  <tr key={c.preset + c.key}>
                    <td className="small">{c.preset_title}</td>
                    <td className="small">{c.label}</td>
                    <td className="tnum" style={{ textAlign: 'right' }}>{c.old !== null ? fmtNum(c.old) : '—'} {c.unit} {c.pass_old === true ? <span className="vd-pass">✓</span> : c.pass_old === false ? <span className="vd-fail">✕</span> : null}</td>
                    <td className="tnum" style={{ textAlign: 'right' }}>{c.new !== null ? fmtNum(c.new) : '—'} {c.unit} {c.pass_new === true ? <span className="vd-pass">✓</span> : c.pass_new === false ? <span className="vd-fail">✕</span> : null}</td>
                    <td className="small muted">{c.note}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      )}
      {others.map((s) => (
        <details key={s.id} className="card vd-sec" open={s.id === 'requirements'}>
          <summary><b>{s.title}</b> <span className="muted small">· {s.items.length} change(s)</span></summary>
          <div style={{ marginTop: 10 }}><Items s={s} /></div>
        </details>
      ))}
      {(d.meta.length > 0 || d.concept.length > 0) && (
        <details className="card vd-sec">
          <summary><b>Metadata & concept</b> <span className="muted small">· {d.meta.length + d.concept.length} field(s)</span></summary>
          <div className="small" style={{ marginTop: 10 }}>{[...d.meta, ...d.concept].map((f) => <div key={f.path}><span className="mono">{f.path}</span>: <span className="vd-old">{val(f.old)}</span> → <b>{val(f.new)}</b></div>)}</div>
        </details>
      )}
    </div>
  );
}
