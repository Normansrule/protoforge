// Every lab from the registry as a card with its live headline metric and the
// requirements it verifies.
import { useMemo } from 'react';
import { Link } from 'react-router-dom';
import { Icon } from '../../components/Icon';
import { SHOWN } from '../../components/reqStatus';
import type { Evaluation, Metric, Project } from '../../engine/types';
import { GROUPS, LABS, fmt, headline, health, healthIcon, healthWord, tallyFor, unitText } from './status';

export function LabGrid({ project, ev, filter }: { project: Project; ev: Evaluation | null; filter: string }) {
  const metrics = useMemo(() => {
    const m = new Map<string, Metric>();
    ev?.metrics.forEach((x) => m.set(x.key, x));
    return m;
  }, [ev]);
  const labs = LABS.filter((l) => filter === 'All' || l.group === filter);
  return (
    <div className="hm-labs">
      {labs.map((l) => {
        const gi = GROUPS.indexOf(l.group);
        const t = tallyFor(l.metrics, project, ev);
        const h = health(t);
        const hl = headline(l, metrics);
        const req = hl ? t.reqs.find((r) => r.metric === hl.metric.key) : undefined;
        return (
          <Link key={l.id} to={l.path} className="hm-lab" style={{ ['--g' as string]: `var(--series-${gi + 1})` }}>
            <div className="hm-lab-top">
              <span className="hm-lab-icon"><Icon name={l.icon} size={20} /></span>
              <span className="hm-lab-group">{l.group}</span>
              <span className={`hm-lab-state ${h}`} title={`${t.pass} of ${t.total} requirements verified${t.goal ? ` · ${t.goal === 1 ? 'an open goal' : `${t.goal} open goals`}` : ''}`}>
                {healthIcon(h)} {t.total ? `${t.pass}/${t.total}` : ''} <span className="hm-sr">{healthWord(h)}</span>
              </span>
            </div>
            <h3>{l.title}</h3>
            <p className="hm-lab-blurb">{l.blurb}</p>
            <div className="hm-lab-metric">
              {hl ? (
                <>
                  <div className="hm-lab-mlabel">{hl.label}</div>
                  <div className="hm-lab-mvalue tnum">
                    {hl.metric.unit === 'USD' ? (
                      `$${hl.metric.value.toFixed(2)}`
                    ) : (
                      <>
                        {fmt(hl.metric.value)}
                        <span>{unitText(hl.metric.unit)}</span>
                      </>
                    )}
                  </div>
                  {req?.result && <div className="hm-lab-need">need {req.result.criterion.replace(/\buA\b/, 'µA')}</div>}
                </>
              ) : (
                <div className="hm-lab-mlabel">{ev ? 'metrics coming soon' : 'computing…'}</div>
              )}
            </div>
            {t.reqs.length > 0 && (
              <div className="hm-lab-reqs">
                {t.reqs.map((r) => {
                  const s = SHOWN[r.shown];
                  return (
                    <span key={r.id} className={`chip ${s.tone ?? ''}`} title={`${r.text} (${s.word})`}>
                      {s.icon} {r.id}
                    </span>
                  );
                })}
              </div>
            )}
            <span className="hm-lab-go" aria-hidden><Icon name="arrow" size={16} /></span>
          </Link>
        );
      })}
    </div>
  );
}
