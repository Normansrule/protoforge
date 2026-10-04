// Standard frame for every lab page: title, teaching blurb and the live
// status of the requirements this lab's metrics verify.
import type { ReactNode } from 'react';
import { Link } from 'react-router-dom';
import { useEvaluation, useProject } from '../state/project';
import { Chip } from './ui';
import { fmtNum } from './Plot';
import { LabHelp } from '../ux/help/LabHelp';
import { LAB_HELP } from '../ux/help/labHelp';
import { LABS } from '../labs/registry';
import { useLocation } from 'react-router-dom';

export function LabPage(props: {
  kicker: string;
  title: string;
  subtitle: ReactNode;
  /** Metric key prefixes whose requirements should be shown, e.g. ['dsp.', 'chip.']. */
  metricPrefixes?: string[];
  actions?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="stack">
      <div className="page-head">
        <div style={{ flex: 1, minWidth: 280 }}>
          <div className="page-kicker">{props.kicker}</div>
          <h1 style={{ marginTop: 4 }}>{props.title}</h1>
          <div className="page-sub">{props.subtitle}</div>
          {props.metricPrefixes && <RequirementStrip prefixes={props.metricPrefixes} />}
          <ScopeNotice />
        </div>
        <LabHelp />
        {props.actions && <div className="row">{props.actions}</div>}
      </div>
      {props.children}
    </div>
  );
}

export function RequirementStrip({ prefixes }: { prefixes: string[] }) {
  const project = useProject((s) => s.project);
  const ev = useEvaluation();
  const reqs = project.requirements.filter((r) => r.verification.metric && prefixes.some((p) => r.verification.metric!.startsWith(p)));
  if (reqs.length === 0) return null;
  return (
    <div className="req-strip">
      {reqs.map((r) => {
        const res = ev.data?.results.find((x) => x.id === r.id);
        const tone = res?.status === 'pass' ? 'pass' : res?.status === 'fail' ? 'fail' : undefined;
        const icon = res?.status === 'pass' ? '✓' : res?.status === 'fail' ? '✕' : '○';
        return (
          <Link to={`/systems?req=${r.id}`} key={r.id} style={{ textDecoration: 'none' }}>
            <Chip tone={tone} title={r.text}>
              {icon} {r.id} · {res?.value != null ? `${fmtNum(res.value)} ${res.unit}` : '—'} ({res?.criterion ?? ''})
            </Chip>
          </Link>
        );
      })}
    </div>
  );
}

/** When your own product is loaded, say plainly which labs still analyse the Iris example. */
function ScopeNotice() {
  const { pathname } = useLocation();
  const id = useProject((s) => s.project.meta.id);
  const lab = LABS.find((l) => l.path === pathname);
  const help = lab ? LAB_HELP[lab.id] : undefined;
  if (id === 'iris' || !help || (help.scope !== 'iris' && help.scope !== 'mixed')) return null;
  return (
    <div className="callout small" role="note" style={{ marginTop: 10 }}>
      <strong>{help.scope === 'iris' ? 'This lab analyses the Iris example geometry, not your product. ' : 'Some tabs here use the Iris example. '}</strong>
      {help.scopeNote}
    </div>
  );
}
