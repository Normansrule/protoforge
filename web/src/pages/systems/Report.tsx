// Report & templates: the full systems-engineering report (rendered Markdown
// + downloads) and starter projects for building something new.
import { useState } from 'react';
import { Card, DownloadButton, Explainer } from '../../components/ui';
import { Icon } from '../../components/Icon';
import { call } from '../../engine/engine';
import { useCompute } from '../../engine/hooks';
import { Markdown } from '../../labs/mfg/Markdown';
import { useProject } from '../../state/project';
import { Modal, REFS, useStudio } from './shared';
import type { Template } from './types';

export function ReportTab() {
  const { project, ev, go } = useStudio();
  const setProject = useProject((s) => s.setProject);
  const report = useCompute<string>('systems.report_full', { project, evaluation: ev }, !!ev);
  const templates = useCompute<Template[]>('systems.templates', {});
  const [pick, setPick] = useState<Template | null>(null);
  const slug = project.meta.id || 'project';
  return (
    <>
      <Card
        title="Start a new product"
        subtitle="Templates are complete starting points — needs, requirements bound to lab metrics, subsystems, interfaces, parts, trades, risks and stage gates. Loading one replaces the current project."
      >
        <div className="ss-templates">
          {(templates.data ?? []).map((t) => (
            <button key={t.id} className="ss-template" onClick={() => setPick(t)}>
              <div className="row" style={{ justifyContent: 'space-between', flexWrap: 'nowrap' }}>
                <span className="ico"><Icon name={t.icon} size={20} /></span>
                {t.project.meta.id === project.meta.id && <span className="chip info">current</span>}
              </div>
              <b style={{ fontSize: 15 }}>{t.name}</b>
              <span className="muted small" style={{ flex: 1 }}>{t.description}</span>
              <span className="small">
                {t.project.needs.length} needs · {t.project.requirements.length} requirements · {t.project.subsystems.length} subsystems · {t.project.components.length} parts
              </span>
              <span>{t.tags.map((g) => <span className="ss-tag" key={g}>{g}</span>)}</span>
            </button>
          ))}
          {templates.error && <div className="callout">{templates.error}</div>}
        </div>
      </Card>

      <Card
        title="Systems engineering report"
        subtitle="Concept, needs coverage, the verification matrix, budgets, architecture, trades, FMEA, gates and every metric — generated live from the project."
        actions={
          <>
            <DownloadButton small primary filename={`${slug}-systems-report.md`} content={() => report.data ?? ''} label="Report .md" />
            <DownloadButton small filename={`${slug}-verification.md`} content={() => call<string>('systems.report', project)} label="Verification .md" />
            <DownloadButton small filename={`${slug}.json`} content={() => JSON.stringify(project, null, 2)} label="project.json" />
          </>
        }
      >
        <div className="ss-report">
          {report.data ? <Markdown src={report.data} /> : <div className="muted small">{report.error ?? 'generating…'}</div>}
        </div>
      </Card>

      <Explainer title="Building anything from scratch — the workflow" refs={[REFS.nasa, REFS.incose, REFS.iso15288]}>
        <ol style={{ margin: '4px 0', paddingLeft: 20 }}>
          <li><b>Concept</b>: write the problem, vision, users, use cases and constraints; list the stakeholder needs.</li>
          <li><b>Requirements</b>: derive measurable “shall” statements from each need; bind each to a lab metric and a criterion.</li>
          <li><b>Architecture</b>: split the product into subsystems, connect them with interfaces, allocate every requirement.</li>
          <li><b>Trades & risks</b>: decide the big choices with weighted matrices; run an FMEA and turn mitigations into requirements.</li>
          <li><b>Design in the labs</b>: every change re-verifies the requirements; watch the budgets and the V-model fill in.</li>
          <li><b>Gates</b>: pass each stage gate on the Roadmap, then export the report and the fabrication package from the Manufacturing Hub.</li>
        </ol>
      </Explainer>

      <Modal
        open={!!pick}
        onClose={() => setPick(null)}
        title={pick ? `Load “${pick.name}”?` : ''}
        actions={
          <>
            <DownloadButton small filename={`${slug}.json`} content={() => JSON.stringify(project, null, 2)} label="Save current first" />
            <button className="btn" onClick={() => setPick(null)}>Cancel</button>
            <button className="btn primary" onClick={() => { if (pick) { setProject(structuredClone(pick.project)); setPick(null); go('overview'); } }}>Replace project</button>
          </>
        }
      >
        <p className="muted">This replaces <b style={{ color: 'var(--text)' }}>{project.meta.name}</b> in the browser with the {pick?.name} template. Edits that are not saved to a project.json file are lost.</p>
        {pick && pick.id !== 'iris' && pick.id !== 'blank' && (
          <p className="muted small">The template's board parts start off the PCB (the PCB Studio's placement is tuned for the Iris board outline), so its PCB requirements read “unverified” until you place the parts.</p>
        )}
      </Modal>
    </>
  );
}
