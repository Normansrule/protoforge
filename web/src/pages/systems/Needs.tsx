// Stakeholder needs editor (used by the Concept and Requirements tabs).
import { Card } from '../../components/ui';
import { Badge, DText, nextId, useStudio } from './shared';

export function NeedsEditor({ compact }: { compact?: boolean }) {
  const { project, update, studio } = useStudio();
  const cov = new Map((studio?.coverage.needs ?? []).map((n) => [n.id, n]));
  const map = { satisfied: 'pass', failing: 'fail', partial: 'partial', unverified: 'unverified', uncovered: 'todo' } as const;
  return (
    <Card
      title="Stakeholder needs"
      subtitle="What users and other stakeholders need, in their words. Requirements derive from these; validation checks them at the end."
      actions={
        <button className="btn small" onClick={() => update((d) => { d.needs.push({ id: nextId('N-', d.needs.map((n) => n.id), 1), text: '', stakeholder: 'User', priority: 1 }); })}>+ Need</button>
      }
    >
      {project.needs.length === 0 && <div className="ss-empty">No needs yet. Start with “what must this product let someone do?”</div>}
      <div className="table-wrap">
        <table className="table">
          {!compact && project.needs.length > 0 && (
            <thead><tr><th>ID</th><th>Need</th><th>Stakeholder</th><th>Priority</th><th>Coverage</th><th /></tr></thead>
          )}
          <tbody>
            {project.needs.map((n, i) => {
              const c = cov.get(n.id);
              return (
                <tr key={i}>
                  <td className="ss-id" style={{ width: 54 }}>{n.id}</td>
                  <td style={{ minWidth: 220 }}><DText value={n.text} placeholder="e.g. Last a full working day on one charge" onCommit={(v) => update((d) => { d.needs[i].text = v; })} /></td>
                  <td style={{ width: 150 }}><DText value={n.stakeholder} placeholder="Stakeholder" onCommit={(v) => update((d) => { d.needs[i].stakeholder = v; })} /></td>
                  <td style={{ width: 128 }}>
                    <select value={n.priority} aria-label={`${n.id} priority`} onChange={(e) => update((d) => { d.needs[i].priority = Number(e.target.value); })}>
                      <option value={1}>1 · must</option><option value={2}>2 · should</option><option value={3}>3 · could</option>
                    </select>
                  </td>
                  <td style={{ whiteSpace: 'nowrap' }}>{c && <Badge status={map[c.status]} label={`${c.status} · ${c.requirements.length} req`} />}</td>
                  <td style={{ width: 40 }}>
                    <button className="btn ghost small ss-x" aria-label={`Delete ${n.id}`} title="Delete need (requirements keep a dangling link until you fix them)"
                      onClick={() => { if (confirm(`Delete need ${n.id}?`)) update((d) => { d.needs.splice(i, 1); }); }}>✕</button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </Card>
  );
}
