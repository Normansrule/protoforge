// Concept: the product identity, problem, vision, users, use cases and constraints.
import { Card, Explainer } from '../../components/ui';
import { DText, Eyebrow, ListEditor, REFS, nextId, useStudio } from './shared';
import { NeedsEditor } from './Needs';

export function ConceptTab() {
  const { project, update } = useStudio();
  const m = project.meta;
  const c = project.concept;
  return (
    <>
      <div className="grid grid-2">
        <Card title="Product" subtitle="Name, codename and version appear on every report and export">
          <div className="ss-form">
            <div className="two">
              <label className="f">Name<DText value={m.name} onCommit={(v) => update((d) => { d.meta.name = v; })} /></label>
              <label className="f">Codename<DText value={m.codename} onCommit={(v) => update((d) => { d.meta.codename = v; })} /></label>
            </div>
            <div className="two">
              <label className="f">Version<DText value={m.version} onCommit={(v) => update((d) => { d.meta.version = v; })} /></label>
              <label className="f">Author<DText value={m.author} onCommit={(v) => update((d) => { d.meta.author = v; })} /></label>
            </div>
            <label className="f">Tagline<DText value={m.tagline} onCommit={(v) => update((d) => { d.meta.tagline = v; })} /></label>
            <label className="f">Description<DText area rows={4} value={m.description} onCommit={(v) => update((d) => { d.meta.description = v; })} /></label>
          </div>
        </Card>
        <Card title="Problem & vision" subtitle="Why the product should exist and what success looks like">
          <div className="ss-form">
            <label className="f">Problem statement<DText area rows={4} value={c.problem} placeholder="Who has what problem, and why current solutions fall short" onCommit={(v) => update((d) => { d.concept.problem = v; })} /></label>
            <label className="f">Vision<DText area rows={4} value={c.vision} placeholder="The product in one or two sentences, as the user would describe it" onCommit={(v) => update((d) => { d.concept.vision = v; })} /></label>
          </div>
        </Card>
      </div>

      <div className="grid grid-2">
        <Card title="Users" subtitle="The people the product serves (stakeholders)">
          <ListEditor items={c.users} placeholder="user" addLabel="User" onChange={(v) => update((d) => { d.concept.users = v; })} />
        </Card>
        <Card title="Constraints" subtitle="Limits the solution must respect — mass, process, budget, regulation">
          <ListEditor items={c.constraints} placeholder="constraint" addLabel="Constraint" onChange={(v) => update((d) => { d.concept.constraints = v; })} />
        </Card>
      </div>

      <Card
        title="Use cases"
        subtitle="Operational scenarios: how the product is used, step by step — the concept of operations"
        actions={<button className="btn small" onClick={() => update((d) => { d.concept.use_cases.push({ id: nextId('UC-', d.concept.use_cases.map((u) => u.id), 1), title: '', narrative: '' }); })}>+ Use case</button>}
      >
        {c.use_cases.length === 0 && <div className="ss-empty">No use cases yet.</div>}
        <div className="grid grid-2">
          {c.use_cases.map((u, i) => (
            <div key={i} className="stat" style={{ display: 'grid', gap: 8 }}>
              <div className="row" style={{ flexWrap: 'nowrap' }}>
                <span className="ss-id">{u.id}</span>
                <DText value={u.title} placeholder="Title" onCommit={(v) => update((d) => { d.concept.use_cases[i].title = v; })} />
                <button className="btn ghost small ss-x" aria-label={`Delete ${u.id}`} onClick={() => update((d) => { d.concept.use_cases.splice(i, 1); })}>✕</button>
              </div>
              <DText area rows={3} value={u.narrative} placeholder="Narrative: trigger, steps, outcome" onCommit={(v) => update((d) => { d.concept.use_cases[i].narrative = v; })} />
            </div>
          ))}
        </div>
      </Card>

      <NeedsEditor />

      <div className="grid grid-2">
        <Explainer title="Concept of operations first" refs={[REFS.nasa, REFS.iso15288]}>
          <Eyebrow>Stakeholder needs definition</Eyebrow>
          The first technical process in ISO/IEC/IEEE 15288 is defining stakeholder needs: who uses the product, in which scenarios, under which
          constraints. The NASA handbook calls the result the <i>concept of operations</i> (ConOps). Everything later — requirements, architecture,
          verification — traces back here, and validation at the end asks whether these needs were met.
        </Explainer>
        <Explainer title="Needs are not requirements" refs={[REFS.gtwr, REFS.incose]}>
          A need is in the stakeholder's language (“last a full working day”); a requirement is the engineer's measurable translation
          (“battery life in the reference profile shall be at least 8 h”). One need usually yields several requirements, and every
          requirement should trace to at least one need — otherwise nobody asked for it.
        </Explainer>
      </div>
    </>
  );
}
