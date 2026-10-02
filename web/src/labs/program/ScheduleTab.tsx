// Schedule: the work breakdown structure (WBS) network for the product,
// editable; critical path method (CPM) with total / free float; an animated
// Gantt chart and an activity-on-node network with the passes animated;
// owner load; exports (CSV, Mermaid gantt, JSON).
import { useMemo, useState } from 'react';
import { Card, Chip, Code, DownloadButton, ErrorBox, Explainer, Formula, NumberField, Select, Stat, Toggle } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useMetricMap } from '../../state/project';
import { Gantt } from './Gantt';
import { Network, networkSteps } from './Network';
import type { Schedule, Task } from './types';
import { CRIT, downloadName, num, PHASES, phaseColor, PlayBar, useClock, useProgram, usd } from './util';

const OWNERS = ['Systems', 'Optics', 'Electrical', 'Mechanical', 'Firmware', 'Silicon', 'Manufacturing', 'Test', 'Compliance', 'Program'];

export function ScheduleTab() {
  const { project, cfg, error, save, setTasks } = useProgram();
  const tasks = cfg?.tasks ?? null;
  const sched = useCompute<Schedule>('program.schedule', { project }, !!cfg);
  const metrics = useMetricMap();
  const [sel, setSel] = useState<string | null>(null);
  const [showFloat, setShowFloat] = useState(true);
  const [showLinks, setShowLinks] = useState(true);
  const s = sched.data;
  const finish = s?.finish_weeks ?? 1;
  const gclock = useClock(9, { loop: false, resetKey: s ? `${s.finish_weeks}-${s.tasks.length}` : 0 });
  const cursor = gclock.t >= gclock.duration - 1e-6 ? null : (gclock.t / gclock.duration) * (finish + 1);
  const nsteps = s ? networkSteps(s.tasks.length) : 1;
  const nclock = useClock(nsteps * 0.28, { loop: false, autoplay: false, resetKey: nsteps });
  const nstep = nclock.t >= nclock.duration - 1e-6 || (nclock.t === 0 && !nclock.playing) ? nsteps : (nclock.t / nclock.duration) * nsteps;
  const p80 = metrics.get('program.p80_weeks');

  const edit = (id: string, patch: Partial<Task>) => {
    if (!tasks) return;
    setTasks(tasks.map((t) => (t.id === id ? { ...t, ...patch } : t)));
  };
  const selTask = tasks?.find((t) => t.id === sel) ?? null;
  const selRow = s?.tasks.find((t) => t.id === sel) ?? null;

  const ownerSeries = useMemo(() => (s?.owners ?? []).map((o) => ({ name: `${o.owner} (peak ${num(o.peak, 1)})`, x: o.weeks.map((_, i) => i + 0.5), y: o.weeks, step: true })), [s]);

  return (
    <div className="stack">
      <ErrorBox error={error ?? sched.error} />
      <div className="grid grid-4">
        <Stat label="Plan to PVT exit" value={s ? num(s.finish_weeks, 1) : '…'} unit="weeks" note={s ? `${s.start_date} → ${s.finish_date} (most likely durations)` : ''} />
        <Stat label="80 % confidence" value={p80 ? num(p80.value, 1) : '…'} unit="weeks" note="Monte Carlo P80 — see Schedule risk" tone="warn" />
        <Stat label="Critical activities" value={s ? `${s.critical_count}` : '…'} unit={s ? `of ${s.tasks.length}` : ''} note="zero total float" />
        <Stat label="Budget at completion" value={s ? usd(s.bac_usd) : '…'} note={s ? `labour ${usd(s.labour_usd)} + builds, fees ${usd(s.expense_usd)}` : ''} />
      </div>

      <Card
        title="Gantt chart"
        subtitle="Bars grow as the plan unfolds; the critical path is orange with ◆, hatched tails are total float. Click a bar for details."
        actions={
          <>
            <Toggle label="Float" value={showFloat} onChange={setShowFloat} />
            <Toggle label="Links" value={showLinks} onChange={setShowLinks} />
          </>
        }
      >
        <PlayBar clock={gclock} label={() => (cursor === null ? `${num(finish, 1)} wk` : `wk ${num(cursor, 1)}`)} speeds={[0.5, 1, 2]} />
        <div style={{ marginTop: 10 }}>
          {s && <Gantt rows={s.tasks} finish={s.finish_weeks} start={s.start_date} cursor={cursor} selected={sel} onSelect={setSel} showFloat={showFloat} showLinks={showLinks} />}
        </div>
        <div className="row small muted" style={{ marginTop: 6 }}>
          <span><span className="pg-swatch" style={{ background: CRIT }} /> critical (◆, zero float)</span>
          <span><span className="pg-swatch pg-hatch-swatch" /> total float</span>
          <span>◇ milestone</span>
          {PHASES.map((p) => <span key={p}><span className="pg-swatch" style={{ background: phaseColor(p) }} />{p}</span>)}
        </div>
        {selRow && selTask && (
          <div className="pg-detail fade-in">
            <div className="row" style={{ justifyContent: 'space-between' }}>
              <div>
                <b>{selRow.id}</b> {selRow.name} {selRow.critical ? <Chip tone="warn">◆ critical</Chip> : <Chip tone="pass">float {num(selRow.total_float, 1)} wk</Chip>}
                <div className="small muted">WBS {selRow.wbs} · {selRow.phase} · {selRow.owner} · from {selRow.source} · {selRow.start_date} → {selRow.finish_date}</div>
              </div>
              <button className="btn small ghost" onClick={() => setSel(null)} aria-label="Close details">✕</button>
            </div>
            <div className="pg-kv">
              <span>ES / EF</span><b className="mono">{num(selRow.es, 1)} / {num(selRow.ef, 1)} wk</b>
              <span>LS / LF</span><b className="mono">{num(selRow.ls, 1)} / {num(selRow.lf, 1)} wk</b>
              <span>Total / free float</span><b className="mono">{num(selRow.total_float, 1)} / {num(selRow.free_float, 1)} wk</b>
              <span>Budget</span><b className="mono">{usd(selRow.budget_usd)}</b>
              <span>Predecessors</span><b className="mono">{selRow.preds.map(([p, l]) => (l ? `${p}${l > 0 ? '+' : ''}${l}` : p)).join(', ') || '—'}</b>
              <span>Successors</span><b className="mono">{selRow.succs.join(', ') || '—'}</b>
            </div>
            {!selTask.milestone && (
              <div className="grid grid-3" style={{ marginTop: 8 }}>
                <NumberField label="Optimistic" unit="wk" value={selTask.optimistic} onChange={(v) => edit(selTask.id, { optimistic: v })} />
                <NumberField label="Most likely" unit="wk" value={selTask.likely} onChange={(v) => edit(selTask.id, { likely: v })} />
                <NumberField label="Pessimistic" unit="wk" value={selTask.pessimistic} onChange={(v) => edit(selTask.id, { pessimistic: v })} />
              </div>
            )}
          </div>
        )}
      </Card>

      <Card title="Network diagram (activity on node)" subtitle="Press play to watch the forward pass fill ES / EF, the backward pass fill LS / LF, and the zero-float chain light up." actions={<button className="btn small" onClick={() => { nclock.setPlaying(false); nclock.setT(Math.min(nclock.duration, nclock.t + nclock.duration / nsteps)); }}>Step</button>}>
        <PlayBar clock={nclock} label={() => (nstep < (s?.tasks.length ?? 0) ? 'forward' : nstep < 2 * (s?.tasks.length ?? 0) ? 'backward' : 'critical')} speeds={[0.5, 1, 2, 4]} />
        {s && <div style={{ marginTop: 10 }}><Network rows={s.tasks} step={nstep} follow={nclock.playing} onSelect={setSel} selected={sel} /></div>}
      </Card>

      <div className="grid grid-2">
        <Card title="Stage lengths vs the Build Guide" subtitle="The plan's stage spans compared with the roadmap's typical weeks.">
          <div className="table-wrap"><table className="table">
            <thead><tr><th>Stage</th><th>Span</th><th>Plan</th><th>Typical</th><th>Verdict</th></tr></thead>
            <tbody>
              {s?.stage_checks.map((c) => {
                const ok = c.verdict.startsWith('within');
                return (
                  <tr key={c.stage}>
                    <td>{c.stage}</td>
                    <td className="mono small">{c.from} → {c.to}</td>
                    <td className="mono">{num(c.weeks, 1)} wk</td>
                    <td className="mono">{c.typical[0]}–{c.typical[1]} wk</td>
                    <td><Chip tone={ok ? 'pass' : 'warn'}>{ok ? '✓' : '!'} {c.verdict}</Chip></td>
                  </tr>
                );
              })}
            </tbody>
          </table></div>
        </Card>
        <Card title="Who is busy when" subtitle="Full-time equivalents per owner on the early-start schedule — peaks above the team you have mean the plan needs levelling.">
          {s && <Plot series={ownerSeries} height={230} xLabel="week" yLabel="FTE" yZero />}
        </Card>
      </div>

      <Card
        title="Work breakdown structure"
        subtitle={cfg?.edited ? 'Edited plan (saved in design.extra.program.tasks).' : 'Derived from the project: milestones, Build Guide roadmap, compliance plan and Manufacturing Hub costs. Edit anything — it is saved with the project.'}
        actions={
          <>
            <button className="btn small" onClick={() => tasks && setTasks([...tasks, { id: nextId(tasks), wbs: '', name: 'New task', phase: tasks[tasks.length - 1]?.phase ?? 'PVT', owner: 'Systems', optimistic: 1, likely: 2, pessimistic: 3, preds: [], milestone: false, fte: 1, expense_usd: 0, source: 'user' }])}>+ Task</button>
            <button className="btn small" disabled={!cfg?.edited} onClick={() => setTasks(null)}>Reset to project default</button>
          </>
        }
      >
        <TaskTable tasks={tasks ?? []} rows={s} onEdit={edit} onDelete={(id) => tasks && setTasks(tasks.filter((t) => t.id !== id).map((t) => ({ ...t, preds: t.preds.filter((p) => p.trim() !== id && !p.startsWith(`${id}+`) && !p.startsWith(`${id}-`)) })))} />
        <div className="grid grid-4" style={{ marginTop: 12 }}>
          <label className="field">
            <span className="field-top"><span className="field-label">Start date</span></span>
            <input type="date" value={cfg?.config.start_date ?? '2026-10-05'} onChange={(e) => e.target.value && save({ start_date: e.target.value })} />
          </label>
          <div className="pg-assumed"><NumberField label="Blended labour rate" unit="USD/h" value={cfg?.config.eng_rate_usd_h ?? 75} onChange={(v) => save({ eng_rate_usd_h: Math.max(0, v) })} /></div>
          <Select label="Frame tooling at PVT" value={(cfg?.config.frame_tooling ?? 'auto') as 'auto' | 'mold'} options={[{ value: 'auto', label: 'Follow the cost model (cheapest process)' }, { value: 'mold', label: 'Hard steel mold (roadmap)' }]} onChange={(v) => save({ frame_tooling: v, tasks: null })} />
          <div className="small muted" style={{ alignSelf: 'center' }}>Changing tooling regenerates the default plan.</div>
        </div>
      </Card>

      <div className="grid grid-2">
        <Card title="Exports" subtitle="CSV for spreadsheets / ProjectLibre / OpenProject import; Mermaid for GitHub READMEs and Pages.">
          <div className="row">
            {s && <DownloadButton filename={downloadName(project, 'schedule', 'csv')} content={s.csv} label="Schedule CSV" primary />}
            {s && <DownloadButton filename={downloadName(project, 'gantt', 'mmd')} content={s.mermaid} label="Mermaid gantt" />}
            {tasks && <DownloadButton filename={downloadName(project, 'wbs', 'json')} content={() => JSON.stringify(tasks, null, 2)} label="Tasks JSON" />}
          </div>
          {s && <div style={{ marginTop: 10 }}><Code code={s.mermaid.split('\n').slice(0, 18).join('\n') + '\n    …'} fullCode={s.mermaid} filename={downloadName(project, 'gantt', 'mmd')} maxHeight={240} /></div>}
          {s?.notes.map((n, i) => <p key={i} className="small muted">{n}</p>)}
        </Card>
        <Explainer
          title="Critical path method (CPM)"
          refs={[{ label: 'Kelley & Walker 1959' }, { label: 'PMBOK Guide, 7th ed. (2021)' }, { label: 'NASA SE Handbook SP-2016-6105 Rev 2' }]}
        >
          <p>Each activity has a duration d and finish-to-start predecessors (with an optional lag, written <code>T05+2</code>). The <b>forward pass</b> gives the earliest start and finish, the <b>backward pass</b> the latest ones that still meet the finish date T:</p>
          <Formula>ES<sub>j</sub> = max<sub>i→j</sub>(EF<sub>i</sub> + lag) · EF = ES + d · LF<sub>i</sub> = min<sub>i→j</sub>(LS<sub>j</sub> − lag) · LS = LF − d</Formula>
          <Formula>total float TF = LS − ES · free float FF<sub>i</sub> = min<sub>j</sub>(ES<sub>j</sub> − lag) − EF<sub>i</sub></Formula>
          <p>Zero total float marks the <b>critical path</b>, the longest chain through the network. For Iris the chain runs through firmware on dev kits, the EVT build and tests, the hinge flex cables, battery certification (IEC 62133-2 / UN 38.3) and the radio certification at PVT — shortening anything else does not move the finish.</p>
        </Explainer>
      </div>
    </div>
  );
}

function nextId(tasks: Task[]) {
  let k = tasks.length + 1;
  while (tasks.some((t) => t.id === `T${String(k).padStart(2, '0')}`)) k++;
  return `T${String(k).padStart(2, '0')}`;
}

function TaskTable({ tasks, rows, onEdit, onDelete }: { tasks: Task[]; rows: Schedule | null; onEdit: (id: string, p: Partial<Task>) => void; onDelete: (id: string) => void }) {
  return (
    <div className="table-wrap pg-tasktable">
      <table className="table">
        <thead>
          <tr><th>ID</th><th>Name</th><th>Owner</th><th title="optimistic / most likely / pessimistic">O / M / P wk</th><th>Predecessors</th><th>FTE</th><th>Expense</th><th>ES–EF</th><th>TF</th><th /></tr>
        </thead>
        <tbody>
          {tasks.map((t) => {
            const r = rows?.tasks.find((x) => x.id === t.id);
            return (
              <tr key={t.id} className={r?.critical ? 'pg-crit-row' : undefined}>
                <td className="mono small">{r?.critical ? '◆ ' : ''}{t.id}</td>
                <td><input aria-label={`${t.id} name`} value={t.name} onChange={(e) => onEdit(t.id, { name: e.target.value })} style={{ minWidth: 200 }} /></td>
                <td>
                  <select aria-label={`${t.id} owner`} value={t.owner} onChange={(e) => onEdit(t.id, { owner: e.target.value })}>
                    {[...new Set([...OWNERS, t.owner])].map((o) => <option key={o}>{o}</option>)}
                  </select>
                </td>
                <td>
                  {t.milestone ? <span className="muted small">milestone</span> : (
                    <span className="row" style={{ gap: 3, flexWrap: 'nowrap' }}>
                      <input aria-label={`${t.id} optimistic`} type="number" step={0.5} value={t.optimistic} onChange={(e) => onEdit(t.id, { optimistic: Number(e.target.value) })} style={{ width: 52 }} />
                      <input aria-label={`${t.id} most likely`} type="number" step={0.5} value={t.likely} onChange={(e) => onEdit(t.id, { likely: Number(e.target.value) })} style={{ width: 52 }} />
                      <input aria-label={`${t.id} pessimistic`} type="number" step={0.5} value={t.pessimistic} onChange={(e) => onEdit(t.id, { pessimistic: Number(e.target.value) })} style={{ width: 52 }} />
                    </span>
                  )}
                </td>
                <td><input aria-label={`${t.id} predecessors`} className="mono" value={t.preds.join(', ')} onChange={(e) => onEdit(t.id, { preds: e.target.value.split(/[,\s]+/).filter(Boolean) })} style={{ width: 130 }} /></td>
                <td>{t.milestone ? '' : <input aria-label={`${t.id} FTE`} type="number" step={0.5} value={t.fte} onChange={(e) => onEdit(t.id, { fte: Number(e.target.value) })} style={{ width: 52 }} />}</td>
                <td><input aria-label={`${t.id} expense`} type="number" step={100} value={t.expense_usd} onChange={(e) => onEdit(t.id, { expense_usd: Number(e.target.value) })} style={{ width: 84 }} /></td>
                <td className="mono small tnum">{r ? `${num(r.es, 1)}–${num(r.ef, 1)}` : ''}</td>
                <td className="mono small tnum" style={{ color: r?.critical ? CRIT : undefined }}>{r ? num(r.total_float, 1) : ''}</td>
                <td><button className="btn small ghost" onClick={() => onDelete(t.id)} aria-label={`Delete ${t.id}`}>✕</button></td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
