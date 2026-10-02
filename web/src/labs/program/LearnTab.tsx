// How it works: the critical path method step by step on a six-activity
// network you can check by hand, a PERT / triangular distribution explorer,
// an NPV / IRR calculator, the readiness scales, the review sequence, tools
// to graduate to and references.
import { useState } from 'react';
import { Card, Explainer, Formula, Slider } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { Network, networkSteps } from './Network';
import type { PertOut, Schedule } from './types';
import { num, PlayBar, useClock, usd } from './util';

const TRL = ['Basic principles observed', 'Technology concept formulated', 'Analytical and experimental proof of concept', 'Validated in a laboratory environment', 'Validated in a relevant environment', 'Prototype demonstrated in a relevant environment', 'Prototype demonstrated in an operational environment', 'System complete and qualified', 'Proven in operation'];
const MRL = ['Basic manufacturing implications identified', 'Manufacturing concepts identified', 'Manufacturing proof of concept developed', 'Capability to produce in a laboratory environment', 'Prototype components in a production-relevant environment', 'Prototype system in a production-relevant environment', 'Production-representative environment', 'Pilot line demonstrated; ready for low-rate production', 'Low-rate production demonstrated', 'Full-rate production demonstrated'];

const TOOLS: { name: string; href: string; learn: string }[] = [
  { name: 'GanttProject', href: 'https://github.com/bardsoftware/ganttproject', learn: 'Desktop Gantt / PERT charts and resource load; imports the CSV this lab exports.' },
  { name: 'OpenProject', href: 'https://github.com/opf/openproject', learn: 'Web-based project management with Gantt, work packages and baselines — the team-scale version of this tab.' },
  { name: 'frappe/gantt', href: 'https://github.com/frappe/gantt', learn: 'A small SVG Gantt library (bars, progress, dependency arrows). This lab adds CPM floats, the critical path, Monte Carlo and EVM on top of the same idea.' },
  { name: 'Mermaid', href: 'https://github.com/mermaid-js/mermaid', learn: 'Text-to-diagram; the Mermaid gantt export renders on GitHub and GitHub Pages.' },
  { name: 'TaskJuggler', href: 'https://github.com/taskjuggler/TaskJuggler', learn: 'Text-file project scheduler with resource levelling and reports — the next step when owners are over-loaded.' },
  { name: 'MPXJ', href: 'https://github.com/joniles/mpxj', learn: 'Reads and writes Microsoft Project, Primavera and ProjectLibre files — useful to move this plan into professional tools.' },
];

export function LearnTab() {
  const cpm = useCompute<Schedule>('program.cpm', {});
  const steps = cpm.data ? networkSteps(cpm.data.tasks.length) : 1;
  const clock = useClock(steps * 0.9, { loop: false, resetKey: steps });
  const step = clock.t >= clock.duration - 1e-6 || (clock.t === 0 && !clock.playing) ? steps : (clock.t / clock.duration) * steps;
  const [abm, setAbm] = useState<[number, number, number]>([2, 4, 12]);
  const pert = useCompute<PertOut>('program.pert', { a: abm[0], m: abm[1], b: abm[2] });
  const [cf, setCf] = useState('-100, 60, 60');
  const [rate, setRate] = useState(10);
  const flows = cf.split(/[,\s]+/).map(Number).filter((v) => Number.isFinite(v));
  const npv = useCompute<{ npv: number; irr: number | null }>('program.npv', { cashflows: flows, rate: rate / 100 });

  return (
    <div className="stack">
      <Card title="The critical path method, by hand" subtitle="Six activities you can check on paper: A(3), B(4), C(2)←A, D(5)←A, E(1)←B,C, F(2)←D,E. Press play." actions={<button className="btn small" onClick={() => { clock.setPlaying(false); clock.setT(Math.min(clock.duration, clock.t + clock.duration / steps)); }}>Step</button>}>
        <PlayBar clock={clock} label={() => (step < 6 ? 'forward' : step < 12 ? 'backward' : 'critical')} speeds={[0.5, 1, 2]} />
        {cpm.data && <div style={{ marginTop: 10 }}><Network rows={cpm.data.tasks} step={step} follow={false} /></div>}
        {cpm.data && (
          <div className="table-wrap" style={{ marginTop: 10 }}>
            <table className="table">
              <thead><tr><th>Activity</th><th>d</th><th>ES</th><th>EF</th><th>LS</th><th>LF</th><th>Total float</th><th>Free float</th></tr></thead>
              <tbody>
                {cpm.data.tasks.map((t) => (
                  <tr key={t.id} className={t.critical ? 'pg-crit-row' : undefined}>
                    <td><b>{t.id}</b> {t.name}</td>
                    <td className="mono">{t.duration}</td><td className="mono">{t.es}</td><td className="mono">{t.ef}</td><td className="mono">{t.ls}</td><td className="mono">{t.lf}</td>
                    <td className="mono">{t.total_float}{t.critical ? ' ◆' : ''}</td><td className="mono">{t.free_float}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        <p className="small muted">Finish T = 10 weeks; critical path A → D → F. B can slip 3 weeks (total float) but only 1 week without delaying E (free float). The engine's unit test checks every number in this table.</p>
      </Card>

      <div className="grid grid-2">
        <Card title="Three-point estimate explorer" subtitle="How PERT and triangular distributions turn a, m, b into a spread.">
          <Slider label="Optimistic a" unit="wk" value={abm[0]} min={0} max={abm[1]} step={0.5} onChange={(v) => setAbm([v, abm[1], abm[2]])} />
          <Slider label="Most likely m" unit="wk" value={abm[1]} min={abm[0]} max={abm[2]} step={0.5} onChange={(v) => setAbm([abm[0], v, abm[2]])} />
          <Slider label="Pessimistic b" unit="wk" value={abm[2]} min={abm[1]} max={30} step={0.5} onChange={(v) => setAbm([abm[0], abm[1], v])} />
          {pert.data && (
            <>
              <Plot series={[{ name: `beta-PERT (α ${num(pert.data.alpha, 2)}, β ${num(pert.data.beta, 2)})`, x: pert.data.x, y: pert.data.pert_pdf, area: true }, { name: 'triangular', x: pert.data.x, y: pert.data.tri_pdf, dash: '5 4' }]}
                height={220} xLabel="weeks" yLabel="density" vLines={[{ value: abm[1], label: 'm' }, { value: pert.data.mean, label: 'tₑ', color: 'var(--series-1)' }]} />
              <p className="small mono">PERT tₑ = (a + 4m + b)/6 = {num(pert.data.mean, 2)} wk, σ = (b − a)/6 = {num(pert.data.sd, 2)} wk · triangular mean = {num(pert.data.tri_mean, 2)} wk, σ = {num(pert.data.tri_sd, 2)} wk</p>
            </>
          )}
        </Card>
        <Card title="Net present value and internal rate of return" subtitle="Cash flows per period, first one undiscounted.">
          <label className="field"><span className="field-top"><span className="field-label">Cash flows</span></span><input className="mono" value={cf} onChange={(e) => setCf(e.target.value)} /></label>
          <Slider label="Discount rate per period" unit="%" value={rate} min={0} max={40} step={0.5} onChange={setRate} />
          {npv.data && (
            <div className="grid grid-2">
              <div className="stat"><div className="stat-label">NPV</div><div className="stat-value">{usd(npv.data.npv, 2)}</div></div>
              <div className="stat"><div className="stat-label">IRR</div><div className="stat-value">{npv.data.irr === null ? '—' : `${num(npv.data.irr * 100, 2)} %`}</div></div>
            </div>
          )}
          <Formula>NPV = Σ CFₜ / (1 + r)ᵗ · IRR is the r with NPV = 0</Formula>
          <p className="small muted">Worked example (also a unit test): −100, 60, 60 at 10 % → NPV = −100 + 54.55 + 49.59 = 4.13; IRR solves 60x + 60x² = 100 with x = 1/(1 + r) → 13.07 %.</p>
        </Card>
      </div>

      <div className="grid grid-2">
        <Card title="Technology readiness levels (TRL)">
          <ol className="pg-scale">{TRL.map((t, i) => <li key={t}><b className="mono">{i + 1}</b> {t}</li>)}</ol>
        </Card>
        <Card title="Manufacturing readiness levels (MRL)">
          <ol className="pg-scale">{MRL.map((t, i) => <li key={t}><b className="mono">{i + 1}</b> {t}</li>)}</ol>
        </Card>
      </div>

      <div className="grid grid-2">
        <Explainer title="How the Iris plan is built" refs={[{ label: 'NASA SE Handbook SP-2016-6105 Rev 2' }, { label: 'PMBOK Guide, 7th ed.' }]}>
          <p>The default work breakdown structure (WBS) is derived, not typed: the six Iris milestones (M-0 concept review … M-5 PVT) become zero-length milestones; the Build Guide roadmap names the EVT, DVT and PVT builds and their quantities; the compliance plan adds certification work packages grouped by stage and category with its indicative fees and weeks; the Manufacturing Hub prices each prototype build (n units × unit cost at n). Two review gates are added (TRR, PRR). Every duration is a three-point estimate you can edit; edits are saved in <code>design.extra.program</code> with the project.</p>
          <p>The Monte Carlo P80 to PVT exit becomes <code>program.p80_weeks</code>, the business case gives <code>program.gross_margin_pct</code> and <code>program.breakeven_units</code>, and readiness gives <code>program.min_trl</code> — numbers requirements can verify like any other lab's.</p>
        </Explainer>
        <Card title="Tools to graduate to" subtitle="Open-source schedulers studied for this lab.">
          <ul className="pg-tools">
            {TOOLS.map((t) => <li key={t.name}><a href={t.href} target="_blank" rel="noreferrer"><b>{t.name}</b></a> — <span className="small">{t.learn}</span></li>)}
            <li><b>ProjectLibre</b> — <span className="small">desktop alternative to Microsoft Project (hosted on SourceForge); opens CPM networks and Gantt charts like this one.</span></li>
          </ul>
        </Card>
      </div>

      <Card title="References">
        <ul className="small pg-refs">
          <li>Project Management Institute, <i>A Guide to the Project Management Body of Knowledge (PMBOK Guide)</i>, 7th ed. (2021).</li>
          <li>NASA, <i>Systems Engineering Handbook</i>, SP-2016-6105 Rev 2 (2016).</li>
          <li>NASA, <i>Cost Estimating Handbook</i>, version 4.0 (2015).</li>
          <li>U.S. Department of Defense, <i>Technology Readiness Assessment Guidebook</i>; <i>Manufacturing Readiness Level (MRL) Deskbook</i> (check the current editions).</li>
          <li>J. C. Mankins, “Technology readiness levels: a white paper”, NASA (1995).</li>
          <li>J. E. Kelley & M. R. Walker, “Critical-path planning and scheduling”, Proc. Eastern Joint Computer Conference (1959).</li>
          <li>D. G. Malcolm, J. H. Roseboom, C. E. Clark & W. Fazar, “Application of a technique for research and development program evaluation”, <i>Operations Research</i> 7(5) (1959).</li>
          <li>D. T. Hulett, <i>Practical Schedule Risk Analysis</i> (Gower, 2009).</li>
          <li>Q. W. Fleming & J. M. Koppelman, <i>Earned Value Project Management</i>, 4th ed. (PMI, 2010).</li>
          <li>W. Lipke, “Schedule is different”, <i>The Measurable News</i> (2003).</li>
          <li>D. S. Christensen & S. R. Heise, “Cost performance index stability”, <i>National Contract Management Journal</i> (1993).</li>
          <li>K. T. Ulrich & S. D. Eppinger, <i>Product Design and Development</i> (McGraw-Hill).</li>
          <li>G. Marsaglia & W. W. Tsang, “A simple method for generating gamma variables”, <i>ACM Transactions on Mathematical Software</i> 26(3) (2000).</li>
        </ul>
      </Card>
    </div>
  );
}
