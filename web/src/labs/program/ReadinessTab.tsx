// Readiness & reviews: technology / manufacturing readiness levels per
// subsystem suggested from project evidence (radar chart, climbing ladders,
// evidence editors) and the SRR / PDR / CDR / TRR / PRR review gates with
// entry / exit criteria tied to live requirement status.
import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Stat } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { useEvaluation } from '../../state/project';
import type { Evidence, ReadinessOut, Review, SubsystemReadiness } from './types';
import { downloadName, num, useClock, useProgram } from './util';

const STAGE_LABEL: Record<string, string> = {
  none: 'nothing built', breadboard: 'breadboard / dev kits', brassboard: 'brassboard (real form factor)', evt: 'EVT prototype', dvt: 'DVT units', pvt: 'PVT pilot line', production: 'in production',
};

export function ReadinessTab() {
  const { project, cfg, save } = useProgram();
  const ev = useEvaluation();
  const results = ev.data?.results.map((r) => ({ id: r.id, status: r.status }));
  const r = useCompute<ReadinessOut>('program.readiness', { project, results }, !!cfg && !!results);
  const d = r.data;
  const [open, setOpen] = useState<string | null>(null);
  const clock = useClock(3.5, { loop: false, resetKey: d ? d.subsystems.map((s) => `${s.trl}${s.mrl}`).join('') : 0 });
  const grow = Math.min(1, clock.t / clock.duration);
  const setEv = (id: string, patch: Partial<Evidence>) => {
    if (!cfg) return;
    const cur: Evidence = cfg.config.readiness[id] ?? { heritage: '', stage: 'none', trl_override: null, mrl_override: null, note: '' };
    save({ readiness: { ...cfg.config.readiness, [id]: { ...cur, ...patch } } });
  };
  const toggleCrit = (key: string, v: boolean) => cfg && save({ reviews: { ...cfg.config.reviews, [key]: v } });

  return (
    <div className="stack">
      <ErrorBox error={r.error} />
      {!d && <p className="muted small">Waiting for live requirement verification…</p>}
      {d && (
        <div className="grid grid-4">
          <Stat label="Lowest technology readiness (TRL)" value={d.min_trl} unit="/ 9" tone={d.min_trl >= 6 ? 'pass' : d.min_trl >= 4 ? 'warn' : 'fail'} note={d.summary.min_trl_subsystems.join(', ')} />
          <Stat label="Lowest manufacturing readiness (MRL)" value={d.min_mrl} unit="/ 10" tone={d.min_mrl >= 8 ? 'pass' : d.min_mrl >= 4 ? 'warn' : 'fail'} note="pilot line = MRL 8" />
          <Stat label="Next gate" value={nextGate(d.reviews)?.id ?? '—'} note={nextGate(d.reviews) ? `${nextGate(d.reviews)!.met} of ${nextGate(d.reviews)!.total} criteria met` : 'all gates passed'} />
          <Stat label="Requirements passing" value={ev.data ? `${ev.data.pass}` : '…'} unit={ev.data ? `of ${ev.data.results.length}` : ''} note={ev.data ? `${ev.data.fail} failing (${ev.data.open_goals ?? 0} open goals)` : ''} />
        </div>
      )}

      {d && (
        <div className="grid grid-2">
          <Card title="Readiness radar" subtitle="TRL (out of 9) and MRL (out of 10) per subsystem, suggested from evidence in the project." actions={<button className="btn small" onClick={clock.restart} aria-label="Replay the animation">↺ Replay</button>}>
            <Radar subs={d.subsystems} grow={grow} />
          </Card>
          <Card title="Review gates on the schedule" subtitle="When each systems-engineering review falls on the plan, and how ready it is.">
            <GateLine reviews={d.reviews} finish={d.summary.cpm_weeks} />
            <p className="small muted">SRR, PDR and CDR sit on the Iris milestones M-1, M-2 and M-3 (EVT exit); TRR opens DVT testing; PRR closes DVT before the PVT pilot build.</p>
          </Card>
        </div>
      )}

      {d && (
        <Card title="Subsystem evidence" subtitle="Tell the lab what exists; the suggested levels follow. Click a row to see the ladder and why it stops where it does."
          actions={<DownloadButton small filename={downloadName(project, 'readiness', 'md')} content={() => readinessMd(project.meta.name, d)} label="Readiness report (Markdown)" />}>
          <div className="table-wrap">
            <table className="table pg-ready">
              <thead><tr><th>Subsystem</th><th>Heritage</th><th>Built so far</th><th>TRL</th><th>MRL</th><th>Requirements</th></tr></thead>
              <tbody>
                {d.subsystems.map((s) => (
                  <SubRow key={s.id} s={s} open={open === s.id} onToggle={() => setOpen(open === s.id ? null : s.id)} stages={cfg?.stages ?? []} onEv={(p) => setEv(s.id, p)} />
                ))}
              </tbody>
            </table>
          </div>
        </Card>
      )}

      {d && (
        <div className="pg-reviews">
          {d.reviews.map((rv) => <ReviewCard key={rv.id} rv={rv} onToggle={toggleCrit} />)}
        </div>
      )}

      <Explainer title="Readiness levels and review gates" refs={[{ label: 'U.S. DoD Technology Readiness Assessment Guidebook' }, { label: 'DoD Manufacturing Readiness Level (MRL) Deskbook' }, { label: 'NASA SE Handbook SP-2016-6105 Rev 2' }, { label: 'Mankins 1995, TRL white paper' }]}>
        <p><b>Technology readiness levels (TRL)</b> run from 1 (basic principles observed) to 9 (proven in operation); the jump that matters is TRL 6, a prototype demonstrated in a relevant environment. <b>Manufacturing readiness levels (MRL)</b> run from 1 to 10; MRL 8 means a pilot line has shown the process can be ready for low-rate production. The lab suggests a level from what the project can prove — a requirement bound to a lab metric is an analytical proof of concept (TRL 3), vendor evaluation kits validate commercial off-the-shelf parts on the bench (TRL 4), TRL 6 needs a prototype and at least one requirement verified by test — and an assessor can override it with a note.</p>
        <p>The reviews follow the NASA life cycle: the <b>system requirements review (SRR)</b> baselines what to build, the <b>preliminary design review (PDR)</b> checks the design concept closes, the <b>critical design review (CDR)</b> releases the design for production-intent builds, the <b>test readiness review (TRR)</b> opens formal verification and the <b>production readiness review (PRR)</b> checks the line and the business case. NASA practice expects critical technologies at TRL 6 by PDR; this consumer-product plan asks for TRL 4 at PDR and 6 at CDR.</p>
      </Explainer>
    </div>
  );
}

function nextGate(rs: Review[]) {
  return rs.find((r) => r.met < r.total) ?? null;
}

function SubRow({ s, open, onToggle, stages, onEv }: { s: SubsystemReadiness; open: boolean; onToggle: () => void; stages: string[]; onEv: (p: Partial<Evidence>) => void }) {
  const pass = s.reqs.filter((r) => r.status === 'pass').length;
  const fail = s.reqs.filter((r) => r.status === 'fail').length;
  const tested = s.reqs.filter((r) => r.method !== 'analysis').length;
  return (
    <>
      <tr onClick={onToggle} style={{ cursor: 'pointer' }} aria-expanded={open}>
        <td><b className="mono small">{s.id}</b> {s.name}<div className="small muted">{s.domain}{s.lab ? <> · <Link to={`/${s.lab}`} onClick={(e) => e.stopPropagation()}>/{s.lab}</Link></> : null}</div></td>
        <td onClick={(e) => e.stopPropagation()}>
          <select aria-label={`${s.id} heritage`} value={s.heritage_suggested ? '' : s.heritage} onChange={(e) => onEv({ heritage: e.target.value })}>
            <option value="">{`suggested: ${s.heritage}`}</option>
            <option value="new">new</option>
            <option value="modified">modified</option>
            <option value="cots">COTS</option>
          </select>
        </td>
        <td onClick={(e) => e.stopPropagation()}>
          <select aria-label={`${s.id} stage`} value={s.stage} onChange={(e) => onEv({ stage: e.target.value })}>
            {stages.map((x) => <option key={x} value={x}>{STAGE_LABEL[x] ?? x}</option>)}
          </select>
        </td>
        <td><Level v={s.trl} max={9} suggested={s.trl_suggested} /></td>
        <td><Level v={s.mrl} max={10} suggested={s.mrl_suggested} /></td>
        <td className="small">{s.reqs.length} · <span style={{ color: 'var(--good-text)' }}>✓{pass}</span>{fail > 0 && <span style={{ color: 'var(--bad-text)' }}> ✕{fail}</span>} · {tested} by test</td>
      </tr>
      {open && (
        <tr>
          <td colSpan={6} style={{ background: 'var(--surface-2)' }}>
            <div className="grid grid-2">
              <Ladder title="Technology readiness ladder" rungs={s.trl_ladder} level={s.trl} />
              <Ladder title="Manufacturing readiness ladder" rungs={s.mrl_ladder} level={s.mrl} />
            </div>
            {s.warnings.map((w, i) => <div key={i} className="small" style={{ color: 'var(--warn)' }}>! {w}</div>)}
            <div className="row" style={{ marginTop: 8 }}>
              <label className="small">TRL override <input type="number" min={1} max={9} style={{ width: 56 }} value={s.trl === s.trl_suggested ? '' : s.trl} placeholder={`${s.trl_suggested}`} onChange={(e) => onEv({ trl_override: e.target.value ? Number(e.target.value) : null })} /></label>
              <label className="small">MRL override <input type="number" min={1} max={10} style={{ width: 56 }} value={s.mrl === s.mrl_suggested ? '' : s.mrl} placeholder={`${s.mrl_suggested}`} onChange={(e) => onEv({ mrl_override: e.target.value ? Number(e.target.value) : null })} /></label>
              <input aria-label="Assessor note" placeholder="assessor note (why the override)" value={s.note} onChange={(e) => onEv({ note: e.target.value })} style={{ flex: 1, minWidth: 160 }} />
            </div>
            <div className="row small" style={{ marginTop: 6 }}>
              {s.reqs.map((q) => (
                <Link key={q.id} to={`/systems?req=${q.id}`} style={{ textDecoration: 'none' }}>
                  <Chip tone={q.status === 'pass' ? 'pass' : q.status === 'fail' ? 'fail' : undefined} title={`${q.kind} · verified by ${q.method}${q.metric ? ` · ${q.metric}` : ''}`}>{q.status === 'pass' ? '✓' : q.status === 'fail' ? '✕' : '○'} {q.id} · {q.method}</Chip>
                </Link>
              ))}
            </div>
          </td>
        </tr>
      )}
    </>
  );
}

function Level({ v, max, suggested }: { v: number; max: number; suggested: number }) {
  return (
    <span className="pg-level" title={v !== suggested ? `override (suggested ${suggested})` : 'suggested from evidence'}>
      <b className="mono">{v}</b>
      <span className="pg-level-bar">{Array.from({ length: max }, (_, i) => <i key={i} className={i < v ? 'on' : ''} />)}</span>
      {v !== suggested && <span className="small muted">✎</span>}
    </span>
  );
}

function Ladder({ title, rungs, level }: { title: string; rungs: SubsystemReadiness['trl_ladder']; level: number }) {
  return (
    <div>
      <div className="small" style={{ fontWeight: 650, marginBottom: 4 }}>{title}</div>
      <ol className="pg-ladder" reversed>
        {[...rungs].reverse().map((r) => (
          <li key={r.level} className={`${r.met ? 'met' : ''} ${r.level === level ? 'here' : ''}`} style={{ animationDelay: `${(r.level - 1) * 60}ms` }}>
            <span className="mono">{r.level}</span>
            <span><b>{r.name}</b> <span className="muted">— {r.met ? '✓' : '✕'} {r.why}</span></span>
          </li>
        ))}
      </ol>
    </div>
  );
}

function Radar({ subs, grow }: { subs: SubsystemReadiness[]; grow: number }) {
  const n = subs.length;
  const S = 360, c = S / 2, R = 128;
  const ang = (i: number) => -Math.PI / 2 + (2 * Math.PI * i) / n;
  const pt = (i: number, f: number) => [c + Math.cos(ang(i)) * R * f, c + Math.sin(ang(i)) * R * f];
  const poly = (vals: number[], max: number) => vals.map((v, i) => pt(i, (v / max) * grow).join(',')).join(' ');
  const gates = useMemo(() => [{ v: 6, label: 'TRL 6' }], []);
  return (
    <svg viewBox={`0 0 ${S} ${S}`} className="pg-svg" style={{ maxWidth: 440, margin: '0 auto', display: 'block' }} role="img" aria-label="Radar chart of technology and manufacturing readiness per subsystem">
      {[0.2, 0.4, 0.6, 0.8, 1].map((f) => <polygon key={f} points={subs.map((_, i) => pt(i, f).join(',')).join(' ')} fill="none" stroke="var(--grid)" />)}
      {gates.map((g) => <polygon key={g.label} points={subs.map((_, i) => pt(i, g.v / 9).join(',')).join(' ')} fill="none" stroke="var(--good)" strokeDasharray="4 3" opacity={0.7} />)}
      {subs.map((s, i) => {
        const [x, y] = pt(i, 1.13);
        const [x2, y2] = pt(i, 1);
        return (
          <g key={s.id}>
            <line x1={c} y1={c} x2={x2} y2={y2} stroke="var(--grid)" />
            <text x={x} y={y + 4} fontSize={10.5} textAnchor={Math.abs(x - c) < 8 ? 'middle' : x > c ? 'start' : 'end'} fill="var(--text-2)" className="mono">{s.id.replace('SYS-', '')}</text>
          </g>
        );
      })}
      <polygon points={poly(subs.map((s) => s.mrl), 10)} fill="var(--series-4)" fillOpacity={0.18} stroke="var(--series-4)" strokeWidth={2} strokeDasharray="6 3" />
      <polygon points={poly(subs.map((s) => s.trl), 9)} fill="var(--series-1)" fillOpacity={0.22} stroke="var(--series-1)" strokeWidth={2.2} />
      {subs.map((s, i) => { const [x, y] = pt(i, (s.trl / 9) * grow); return <circle key={s.id} cx={x} cy={y} r={3.4} fill="var(--series-1)" />; })}
      <g fontSize={10.5}>
        <rect x={8} y={S - 46} width={12} height={4} fill="var(--series-1)" /><text x={24} y={S - 41} fill="var(--text-2)">TRL (÷ 9)</text>
        <rect x={8} y={S - 32} width={12} height={4} fill="var(--series-4)" /><text x={24} y={S - 27} fill="var(--text-2)">MRL (÷ 10, dashed)</text>
        <line x1={8} x2={20} y1={S - 16} y2={S - 16} stroke="var(--good)" strokeDasharray="4 3" /><text x={24} y={S - 13} fill="var(--text-2)">TRL 6 — needed at CDR</text>
      </g>
    </svg>
  );
}

function GateLine({ reviews, finish }: { reviews: Review[]; finish: number }) {
  const end = Math.max(finish, ...reviews.map((r) => r.week ?? 0)) + 2;
  return (
    <div className="pg-gates">
      <div className="pg-gates-track" />
      {reviews.map((r, i) => {
        const left = r.week === null ? 0 : (100 * r.week) / end;
        const tone = r.met === r.total ? 'pass' : r.ready ? 'warn' : 'fail';
        return (
          <div key={r.id} className={`pg-gate ${tone}`} style={{ left: `${left}%`, top: i % 2 ? 46 : 0 }}>
            <b>{r.id}</b>
            <span className="mono small">wk {r.week === null ? '—' : num(r.week, 0)}</span>
            <span className="small">{r.met}/{r.total} {tone === 'pass' ? '✓' : tone === 'warn' ? '◐' : '✕'}</span>
          </div>
        );
      })}
    </div>
  );
}

function ReviewCard({ rv, onToggle }: { rv: Review; onToggle: (key: string, v: boolean) => void }) {
  const pct = (100 * rv.met) / Math.max(1, rv.total);
  return (
    <Card title={<>{rv.id} — {rv.name}</>} subtitle={`${rv.purpose}${rv.milestone ? ` · milestone ${rv.milestone}` : ''}${rv.week !== null ? ` · week ${num(rv.week, 0)}` : ''}`}
      actions={<Chip tone={rv.met === rv.total ? 'pass' : rv.ready ? 'warn' : 'fail'}>{rv.met === rv.total ? '✓ ready to close' : rv.ready ? '◐ entry met' : '✕ entry not met'}</Chip>}>
      <div className="pg-progress"><div style={{ width: `${pct}%` }} /></div>
      {(['entry', 'exit'] as const).map((ph) => (
        <div key={ph}>
          <div className="small muted" style={{ textTransform: 'uppercase', letterSpacing: '0.06em', margin: '8px 0 4px' }}>{ph} criteria</div>
          <ul className="pg-checklist">
            {rv.criteria.filter((c) => c.phase === ph).map((c) => (
              <li key={c.key} className={c.met ? 'met' : ''}>
                {c.auto ? (
                  <span className={`pg-auto ${c.met ? 'ok' : 'no'}`} title="checked automatically from the project">{c.met ? '✓' : '✕'}</span>
                ) : (
                  <input type="checkbox" checked={c.met} onChange={(e) => onToggle(c.key, e.target.checked)} aria-label={c.text} style={{ accentColor: 'var(--accent)' }} />
                )}
                <span>
                  {c.text} {c.auto ? <span className="small muted">· auto</span> : <span className="small muted">· manual</span>}
                  {((c.detail && c.detail !== 'manual') || c.route) && <div className="small muted">{c.detail !== 'manual' ? c.detail : ''}{c.route && <> · <Link to={c.route}>open</Link></>}</div>}
                  {c.reqs.length > 0 && (
                    <div className="row" style={{ gap: 4, marginTop: 3 }}>
                      {c.reqs.slice(0, 12).map(([id, st]) => (
                        <Link key={id} to={`/systems?req=${id}`} style={{ textDecoration: 'none' }}><Chip tone={st === 'pass' ? 'pass' : st === 'fail' ? 'fail' : undefined}>{st === 'pass' ? '✓' : st === 'fail' ? '✕' : '○'} {id}</Chip></Link>
                      ))}
                      {c.reqs.length > 12 && <span className="small muted">+{c.reqs.length - 12}</span>}
                    </div>
                  )}
                </span>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </Card>
  );
}

function readinessMd(name: string, d: ReadinessOut) {
  let s = `# ${name} — readiness and review gates\n\nGenerated by Protoforge (Program & Business Lab). Levels are suggested from project evidence; an assessor decides.\n\n| Subsystem | Heritage | Built | TRL | MRL | Notes |\n|---|---|---|---|---|---|\n`;
  for (const x of d.subsystems) s += `| ${x.id} ${x.name} | ${x.heritage} | ${x.stage} | ${x.trl}${x.trl !== x.trl_suggested ? ` (suggested ${x.trl_suggested})` : ''} | ${x.mrl} | ${[...x.warnings, x.note].filter(Boolean).join('; ')} |\n`;
  for (const r of d.reviews) {
    s += `\n## ${r.id} — ${r.name} (${r.met}/${r.total})\n\n${r.purpose}\n\n`;
    for (const c of r.criteria) s += `- [${c.met ? 'x' : ' '}] (${c.phase}${c.auto ? ', auto' : ''}) ${c.text}${c.detail ? ` — ${c.detail}` : ''}${c.reqs.length ? ` [${c.reqs.map(([id, st]) => `${id} ${st}`).join(', ')}]` : ''}\n`;
  }
  return s;
}
