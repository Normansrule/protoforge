// Design verification test (DVT) plan generated from the project's
// requirements: legs with sample sizes, environmental test families, a
// resource-constrained Gantt / chamber schedule, the requirement matrix and
// CSV / Markdown exports.
import { Fragment, useMemo, useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat, Toggle } from '../../components/ui';
import { useElementWidth } from '../../components/useElementWidth';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { DvtOut, DvtTest } from './types';
import { fmt, prefersReducedMotion, toCsv, useRaf, useRelConfig } from './util';

const LEG_COLORS: Record<string, string> = { BL: 'var(--series-7)', A: 'var(--series-2)', B: 'var(--series-1)', C: 'var(--series-3)', D: 'var(--series-5)', E: 'var(--series-4)' };

export function DvtTab() {
  const project = useProject((s) => s.project);
  const [cfg, setCfg] = useRelConfig();
  const { data: d, error } = useCompute<DvtOut>('reliability.dvt', { project });
  const [lanes, setLanes] = useState<'leg' | 'resource'>('resource');
  const [showCells, setShowCells] = useState(false);
  const [open, setOpen] = useState<string | null>(null);
  const matrixCsv = () =>
    d ? toCsv([['requirement', 'text', 'method', 'dvt_activity', 'equipment', 'units', 'pass', 'tests'], ...d.matrix.map((m) => [m.id, m.text, m.method, m.dvt, m.equipment, m.units, m.pass, m.tests.join(' ')])]) : '';
  const slug = project.meta.id || 'project';
  return (
    <div className="rl-layout">
      <div className="stack" style={{ minWidth: 0 }}>
        <ErrorBox error={error} />
        {d && (
          <div className="grid grid-4">
            <Stat label="Product units for DVT" value={d.total_units} unit="units" note={d.legs.filter((l) => l.product_units && l.units > 0).map((l) => `${l.id} ${l.units}`).join(' + ')} />
            <Stat label="Product legs" value={fmt(d.duration_d, 3)} unit="days" note="baseline, then legs in parallel" />
            <Stat label="Bare-cell cycling" value={fmt(d.cell_duration_d, 3)} unit="days" note="start it in EVT" tone={d.cell_duration_d > d.duration_d * 2 ? 'warn' : undefined} />
            <Stat label="Target" value={`R ${d.reliability}`} note={`${Math.round(d.confidence * 100)} % confidence · β ${d.life_beta} · ${d.life_multiple} lives`} />
          </div>
        )}
        {d && (
          <Card
            title="Test schedule"
            subtitle="Each resource runs one test at a time; tests in a leg run in order on the same units. Press play to walk through the campaign."
            actions={
              <div className="row" style={{ flexWrap: 'wrap' }}>
                <div className="rl-seg">
                  <button className={lanes === 'resource' ? 'active' : ''} onClick={() => setLanes('resource')} aria-pressed={lanes === 'resource'}>by resource</button>
                  <button className={lanes === 'leg' ? 'active' : ''} onClick={() => setLanes('leg')} aria-pressed={lanes === 'leg'}>by test</button>
                </div>
                <DownloadButton small filename={`${slug}-dvt-plan.csv`} label="plan CSV" content={() => d.csv} />
                <DownloadButton small filename={`${slug}-dvt-plan.md`} label="plan Markdown" content={() => d.markdown} primary />
              </div>
            }
          >
            <Toggle label={`Show bare-cell cycling (${fmt(d.cell_duration_d, 3)} days — it stretches the axis)`} value={showCells} onChange={setShowCells} />
            <Gantt tests={d.tests.filter((t) => showCells || t.leg !== 'E')} lanes={lanes} onPick={setOpen} />
            <div className="rl-legend small">
              {d.legs.map((l) => (
                <span key={l.id}><i className="rl-line" style={{ background: LEG_COLORS[l.id], height: 8 }} /> {l.id} · {l.name}</span>
              ))}
            </div>
          </Card>
        )}
        {d && (
          <Card title="Legs and sample sizes">
            <div className="rl-legs">
              {d.legs.map((l) => (
                <div key={l.id} className="rl-leg" style={{ borderTopColor: LEG_COLORS[l.id] }}>
                  <div className="row" style={{ justifyContent: 'space-between' }}>
                    <b>{l.id} · {l.name}</b>
                    <span className="tnum">{l.id === 'BL' ? `all ${d.total_units}` : `${l.units} ${l.product_units ? 'units' : 'cells'}`}</span>
                  </div>
                  <div className="small muted">{l.sample_basis}</div>
                  <div className="rl-units small" aria-hidden>
                    {Array.from({ length: Math.min(l.id === 'BL' ? d.total_units : l.units, 40) }, (_, i) => (
                      <span key={i} className="rl-unit mini" style={{ background: LEG_COLORS[l.id] }} />
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </Card>
        )}
        {d && (
          <Card title="Tests" subtitle="Click a test for its procedure outline. Standards by designation only — severities are typical choices; check the current editions and your product standard.">
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>ID</th><th>Test</th><th>Standard / family</th><th>Severity</th><th>Units</th><th>Days</th><th>Pass criterion</th></tr></thead>
                <tbody>
                  {d.tests.map((t) => (
                    <Fragment key={t.id}>
                      <tr onClick={() => setOpen(open === t.id ? null : t.id)} style={{ cursor: 'pointer' }} className={open === t.id ? 'rl-row-active' : undefined}>
                        <td><span className="rl-legtag" style={{ background: LEG_COLORS[t.leg] }}>{t.id}</span></td>
                        <td>{t.name}</td>
                        <td className="small">{t.standard}</td>
                        <td className="small">{t.severity}</td>
                        <td className="tnum">{t.units}</td>
                        <td className="tnum">{t.duration_d.toFixed(1)}</td>
                        <td className="small">{t.pass}</td>
                      </tr>
                      {open === t.id && (
                        <tr>
                          <td colSpan={7}>
                            <div className="rl-proc">
                              <ol>{t.procedure.map((p, i) => <li key={i}>{p}</li>)}</ol>
                              <div className="small"><b>Equipment:</b> {t.equipment} · <b>Resource:</b> {t.resource} · <b>Days</b> {t.start_d.toFixed(1)}–{t.end_d.toFixed(1)}{t.linked.length > 0 && <> · <b>Linked:</b> {t.linked.join(', ')}</>}</div>
                              {t.basis && <div className="small muted">Basis: {t.basis}</div>}
                            </div>
                          </td>
                        </tr>
                      )}
                    </Fragment>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
        )}
        {d && (
          <Card title="Requirement verification matrix" subtitle={`Every ${project.meta.name} requirement, how DVT confirms it physically, and the pass criterion from the requirement.`} actions={<DownloadButton small filename={`${slug}-verification-matrix.csv`} label="matrix CSV" content={matrixCsv} />}>
            <div className="table-wrap" style={{ maxHeight: 420 }}>
              <table className="table">
                <thead><tr><th>Req</th><th>Requirement</th><th>Method</th><th>DVT activity</th><th>Units</th><th>Pass</th><th>Tests</th></tr></thead>
                <tbody>
                  {d.matrix.map((m) => (
                    <tr key={m.id}>
                      <td className="mono small">{m.id}</td>
                      <td className="small" style={{ minWidth: 180 }}>{m.text}</td>
                      <td className="small">{m.method}</td>
                      <td className="small" style={{ minWidth: 200 }}>{m.dvt}</td>
                      <td className="small">{m.units}</td>
                      <td className="small tnum">{m.pass}</td>
                      <td className="small">{m.tests.join(', ') || '—'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
        )}
      </div>
      <div className="stack rl-side">
        <Card title="Plan targets" subtitle="Saved with the project — reliability.dvt_units follows">
          <Slider label="Reliability to demonstrate" value={cfg.dvt_reliability} min={0.7} max={0.99} step={0.01} onChange={(v) => setCfg({ dvt_reliability: v })} />
          <Slider label="Confidence" value={cfg.dvt_confidence} min={0.5} max={0.95} step={0.05} onChange={(v) => setCfg({ dvt_confidence: v })} format={(v) => `${Math.round(v * 100)} %`} />
          <Slider label="Failures allowed per leg" value={cfg.dvt_failures} min={0} max={2} step={1} format={(v) => String(Math.round(v))} onChange={(v) => setCfg({ dvt_failures: Math.round(v) })} />
          <Slider label="Life-leg test length" value={cfg.life_multiple} min={1} max={3} step={0.25} unit="lives" onChange={(v) => setCfg({ life_multiple: v })} />
          <Slider label="Assumed β of the life mechanisms" value={cfg.life_beta} min={1} max={4} step={0.1} onChange={(v) => setCfg({ life_beta: v })} />
          <Slider label="Units per qualification check" value={cfg.qual_units} min={1} max={10} step={1} format={(v) => String(Math.round(v))} onChange={(v) => setCfg({ qual_units: Math.round(v) })} />
          <Slider label="Daily electronics temperature swing" value={cfg.daily_swing_c} min={5} max={40} step={1} format={(v) => String(Math.round(v))} unit="K" onChange={(v) => setCfg({ daily_swing_c: v })} />
        </Card>
        {d && (
          <Card title="Notes">
            {d.notes.map((n, i) => <div key={i} className="small" style={{ marginBottom: 6 }}>• {n}</div>)}
          </Card>
        )}
        <Explainer title="EVT, DVT, PVT" refs={[{ label: 'O’Connor & Kleyner, Practical Reliability Engineering, 5th ed., ch. 12–14' }, { label: 'IEC 60068-2 series (environmental testing) — check current editions' }, { label: 'IEC 60529 (degrees of protection, IP code)' }]}>
          Engineering validation (EVT) asks "does the design work?"; design validation (DVT) asks "does it meet every requirement and survive its life, on production-intent parts?"; production validation (PVT) asks "can the line build it repeatably?". The Iris milestones M-3 to M-5 follow that split.
          <Formula>{`attribute leg:  n = ln(1 − C) / ln R
life leg:       n = ln(1 − C) / (k^β ln R)
qualification:  a severity check, small n`}</Formula>
        </Explainer>
      </div>
    </div>
  );
}

function Gantt({ tests, lanes, onPick }: { tests: DvtTest[]; lanes: 'leg' | 'resource'; onPick: (id: string) => void }) {
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const rows = useMemo(() => {
    if (lanes === 'leg') return tests.map((t) => ({ key: t.id, label: `${t.id} ${t.name}`, items: [t] }));
    const m = new Map<string, DvtTest[]>();
    tests.forEach((t) => m.set(t.resource, [...(m.get(t.resource) ?? []), t]));
    return [...m.entries()].map(([k, v]) => ({ key: k, label: k, items: v }));
  }, [tests, lanes]);
  const end = Math.max(1, ...tests.map((t) => t.end_d));
  const [day, setDay] = useState(end);
  const [playing, setPlaying] = useState(false);
  useRaf(playing, (dt) => setDay((v) => (v + dt * end / 10 > end ? 0 : v + dt * end / 10)));
  const W = Math.max(width, 300);
  const labelW = W < 520 ? 96 : 190;
  const rowH = 22;
  const H = rows.length * rowH + 30;
  const sx = (d: number) => labelW + (d / end) * (W - labelW - 12);
  const ticks: number[] = [];
  const step = end > 60 ? 14 : end > 20 ? 5 : end > 8 ? 2 : 1;
  for (let d = 0; d <= end + 1e-9; d += step) ticks.push(d);
  const active = tests.filter((t) => day >= t.start_d && day < t.end_d).map((t) => t.id);
  return (
    <div ref={ref}>
      <div className="row" style={{ marginBottom: 6 }}>
        <button
          className="btn small"
          onClick={() => {
            if (prefersReducedMotion()) {
              setDay(end);
              return;
            }
            if (!playing && day >= end) setDay(0);
            setPlaying((p) => !p);
          }}
          aria-pressed={playing}
        >{playing ? '❚❚ Pause' : '▶ Play the campaign'}</button>
        <span className="small muted" aria-live="polite">Day {day.toFixed(1)}{active.length ? ` — running: ${active.join(', ')}` : ''}</span>
      </div>
      <svg width={W} height={H} role="img" aria-label="Test schedule Gantt chart">
        {ticks.map((d) => (
          <g key={d}>
            <line x1={sx(d)} x2={sx(d)} y1={0} y2={H - 20} stroke="var(--grid)" />
            <text x={sx(d)} y={H - 6} textAnchor="middle" className="rl-axis">d{d}</text>
          </g>
        ))}
        {rows.map((r, i) => (
          <g key={r.key} transform={`translate(0 ${i * rowH})`}>
            <text x={4} y={rowH / 2 + 4} className="rl-axis" style={{ fill: 'var(--text-2)' }}>{r.label.length > (labelW < 120 ? 14 : 30) ? r.label.slice(0, labelW < 120 ? 13 : 29) + '…' : r.label}</text>
            {r.items.map((t) => {
              const x = sx(t.start_d);
              const w = Math.max(2, sx(t.end_d) - x);
              const done = day >= t.end_d;
              const run = day >= t.start_d && day < t.end_d;
              return (
                <g key={t.id} onClick={() => onPick(t.id)} style={{ cursor: 'pointer' }}>
                  <title>{`${t.id} ${t.name}: day ${t.start_d.toFixed(1)}–${t.end_d.toFixed(1)} (${t.units} units)`}</title>
                  <rect x={x} y={3} width={w} height={rowH - 6} rx={4} fill={LEG_COLORS[t.leg]} opacity={done ? 0.95 : run ? 0.8 : 0.35} stroke={run ? 'var(--text)' : 'none'} />
                  {w > 26 && <text x={x + 4} y={rowH / 2 + 4} className="rl-axis" style={{ fill: 'var(--surface-1)', fontWeight: 600 }}>{t.id}</text>}
                </g>
              );
            })}
          </g>
        ))}
        <line x1={sx(day)} x2={sx(day)} y1={0} y2={H - 20} stroke="var(--accent)" strokeWidth={2} />
      </svg>
    </div>
  );
}
