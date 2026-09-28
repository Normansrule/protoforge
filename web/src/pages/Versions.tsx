// Versions: Onshape-style history for the whole product model — named
// versions (baselines), branches, restore, compare with requirement verdicts,
// auto-versioning, and JSON export/import of the history.
import { useEffect, useMemo, useRef, useState } from 'react';
import { LabPage } from '../components/LabPage';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Stat, Toggle } from '../components/ui';
import { useEvaluation, useProject } from '../state/project';
import { workerCall } from '../labs/pcb/useWorker';
import { downloadText } from '../components/ui';
import { VersionGraph } from './versions/Graph';
import { CompareView, type Diff } from './versions/Compare';
import { quickSummary, seedIfEmpty, startAutoVersioning, useHistory, type Version } from './versions/history';
import './versions/versions.css';

const WORKING = 'HEAD';

function useWorkingSummary() {
  const project = useProject((s) => s.project);
  return useMemo(() => {
    try {
      return quickSummary(project);
    } catch {
      return null;
    }
  }, [project]);
}

function Detail({ v, onCompare }: { v: Version; onCompare: (a: string, b: string) => void }) {
  const h = useHistory();
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(v.name);
  const [desc, setDesc] = useState(v.description);
  useEffect(() => {
    setName(v.name);
    setDesc(v.description);
    setEditing(false);
  }, [v]);
  const e = v.summary.evaluation;
  return (
    <Card title={v.name} subtitle={`${v.kind === 'example' ? 'Example history · ' : ''}branch ${v.branch} · #${v.summary.short} · ${new Date(v.created).toLocaleString()}`}>
      {editing ? (
        <div className="stack" style={{ gap: 8 }}>
          <input type="text" value={name} onChange={(x) => setName(x.target.value)} aria-label="Version name" />
          <textarea value={desc} onChange={(x) => setDesc(x.target.value)} rows={3} aria-label="Description" />
          <div className="row"><button className="btn small primary" onClick={() => { h.rename(v.id, name.trim() || v.name, desc); setEditing(false); }}>Save</button><button className="btn small" onClick={() => setEditing(false)}>Cancel</button></div>
        </div>
      ) : (
        <div className="small" style={{ color: 'var(--text-2)', marginBottom: 10 }}>{v.description || <span className="muted">No description.</span>}</div>
      )}
      <div className="row" style={{ gap: 6, marginBottom: 12 }}>
        {e ? <><Chip tone="pass">✓ {e.pass}</Chip>{e.fail > 0 && <Chip tone="fail">✕ {e.fail}</Chip>}{e.unverified > 0 && <Chip>○ {e.unverified}</Chip>}</> : <Chip>evaluating…</Chip>}
        <Chip>{v.summary.requirements} requirements</Chip>
        <Chip>{v.summary.components} BOM lines · ${v.summary.bom_cost_usd.toFixed(2)}</Chip>
        <Chip>v{v.summary.version}</Chip>
      </div>
      <div className="row" style={{ gap: 6 }}>
        <button className="btn small primary" onClick={() => confirm(`Restore “${v.name}”? The working state is replaced (create a version first to keep it).`) && h.restore(v.id)}>Restore</button>
        <button className="btn small" onClick={() => { const n = prompt('Name of the new branch', `${v.summary.version}-experiment`); if (n) h.branchFrom(v.id, n); }}>Branch from here</button>
        <button className="btn small" onClick={() => onCompare(v.id, WORKING)}>Compare with working state</button>
        <button className="btn small ghost" onClick={() => setEditing(true)}>Rename</button>
        <DownloadButton filename={`${v.project.meta.id || 'project'}-${v.summary.short}.json`} content={() => JSON.stringify(v.project, null, 2)} label="project.json" small />
        <button className="btn small ghost" onClick={() => confirm(`Delete “${v.name}” from the history?`) && h.remove(v.id)}>Delete</button>
      </div>
    </Card>
  );
}

export default function Versions() {
  const h = useHistory();
  const project = useProject((s) => s.project);
  const ev = useEvaluation();
  const working = useWorkingSummary();
  const base = h.versions.find((v) => v.id === h.base) ?? null;
  const modified = !!working && (!base || base.summary.hash !== working.hash);
  const [selected, setSelected] = useState<string | null>(null);
  const [cmp, setCmp] = useState<[string | null, string | null]>([null, null]);
  const [diff, setDiff] = useState<{ key: string; data: Diff | null; error: string | null; busy: boolean }>({ key: '', data: null, error: null, busy: false });
  const [name, setName] = useState('');
  const [desc, setDesc] = useState('');
  const fileRef = useRef<HTMLInputElement>(null);
  const [importErr, setImportErr] = useState<string | null>(null);

  useEffect(() => {
    seedIfEmpty();
    startAutoVersioning();
  }, []);
  // default comparison: the two oldest versions (the seeded v0.3 → v0.4 story)
  useEffect(() => {
    if (cmp[0] === null && h.versions.length >= 2) {
      const s = [...h.versions].sort((a, b) => a.created - b.created);
      setCmp([s[0].id, s[1].id]);
    }
  }, [h.versions, cmp]);

  const projectOf = (id: string | null) => (id === WORKING ? project : h.versions.find((v) => v.id === id)?.project ?? null);
  const labelOf = (id: string | null) => (id === WORKING ? 'Working state' : h.versions.find((v) => v.id === id)?.name ?? '—');
  const key = `${cmp[0]}|${cmp[1]}|${cmp.includes(WORKING) ? working?.hash : ''}`;
  useEffect(() => {
    const [a, b] = [projectOf(cmp[0]), projectOf(cmp[1])];
    if (!a || !b || diff.key === key) return;
    setDiff({ key, data: null, error: null, busy: true });
    workerCall<Diff>('bridge.diff', { a, b })
      .then((data) => setDiff((d) => (d.key === key ? { key, data, error: null, busy: false } : d)))
      .catch((e: Error) => setDiff((d) => (d.key === key ? { key, data: null, error: e.message, busy: false } : d)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  const sel = h.versions.find((v) => v.id === selected);
  const options = [{ id: WORKING, label: `Working state (${h.branch})` }, ...[...h.versions].sort((a, b) => b.created - a.created).map((v) => ({ id: v.id, label: `${v.name} · #${v.summary.short}` }))];
  const compare = (a: string, b: string) => {
    setCmp([a, b]);
    setTimeout(() => document.getElementById('vd-compare')?.scrollIntoView({ behavior: 'smooth', block: 'start' }), 50);
  };

  return (
    <LabPage
      kicker="Configuration management"
      title="Versions"
      subtitle={<>Onshape-style history for the whole product: save named versions (baselines) of {project.meta.name}, branch to try an idea, restore any version, and compare two versions — parameters, bill of materials (BOM), requirements and what every lab now says about them.</>}
      actions={
        <>
          <input ref={fileRef} type="file" accept="application/json,.json" style={{ display: 'none' }} onChange={async (e) => {
            const f = e.target.files?.[0];
            if (!f) return;
            try {
              h.importJson(await f.text());
              setImportErr(null);
            } catch (err) {
              setImportErr(`Import failed: ${err instanceof Error ? err.message : err}`);
            }
            e.target.value = '';
          }} />
          <button className="btn small" onClick={() => fileRef.current?.click()}>Import history</button>
          <button className="btn small" onClick={() => downloadText(`${project.meta.id || 'project'}-versions.json`, h.exportJson(), 'application/json')}>Export history</button>
        </>
      }
    >
      <ErrorBox error={h.storageError || importErr} />
      <div className="grid grid-4">
        <Stat label="Versions" value={h.versions.length} note={`${h.versions.filter((v) => v.kind === 'auto').length} automatic · stored in this browser`} />
        <Stat label="Branches" value={h.branches.length} note={h.branches.map((b) => b.name).join(' · ')} />
        <Stat label="Working state" value={working ? `#${working.short}` : '…'} note={modified ? `modified since ${base?.name ?? 'no version'}` : `same as ${base?.name ?? '—'}`} tone={modified ? 'warn' : 'pass'} />
        <Stat label="Live verification" value={ev.data ? `✓ ${ev.data.pass}` : '…'} note={ev.data ? `${ev.data.fail} failing · ${ev.data.unverified} unverified` : ''} tone={ev.data ? (ev.data.fail > 0 ? 'fail' : 'pass') : undefined} />
      </div>
      <div className="grid grid-2 vd-top">
        <Card title="Version graph" subtitle="Newest on top · one lane per branch · dashed = your working state">
          <div className="vd-create">
            <div className="row" style={{ gap: 8, alignItems: 'flex-end' }}>
              <label className="field" style={{ flex: '1 1 160px', marginBottom: 0 }}>
                <span className="field-top"><span className="field-label">New version name</span></span>
                <input type="text" value={name} placeholder={`v${project.meta.version} — what changed`} onChange={(e) => setName(e.target.value)} />
              </label>
              <label className="field" style={{ flex: '2 1 200px', marginBottom: 0 }}>
                <span className="field-top"><span className="field-label">Description</span></span>
                <input type="text" value={desc} placeholder="why, what the labs showed" onChange={(e) => setDesc(e.target.value)} />
              </label>
              <button className="btn primary" onClick={() => { const v = h.create(name.trim() || `v${project.meta.version} · ${new Date().toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })}`, desc.trim()); setName(''); setDesc(''); setSelected(v.id); }}>Create version</button>
            </div>
            <div className="row" style={{ gap: 12, marginTop: 10 }}>
              <label className="small muted" htmlFor="vd-branch">Branch</label>
              <select id="vd-branch" value={h.branch} onChange={(e) => { if (!modified || confirm('Switching branch loads its latest version and replaces the working state. Continue?')) h.switchBranch(e.target.value); }} style={{ maxWidth: 200 }}>
                {h.branches.map((b) => <option key={b.name} value={b.name}>{b.name}</option>)}
              </select>
              <Toggle label="Auto-version significant changes (≤ 1 per minute)" value={h.auto} onChange={h.setAuto} />
            </div>
          </div>
          <VersionGraph versions={h.versions} branches={h.branches} branch={h.branch} base={h.base} working={working} modified={modified} selected={selected} compare={cmp} onSelect={setSelected} />
        </Card>
        <div className="stack">
          {sel ? <Detail v={sel} onCompare={compare} /> : (
            <Card title={selected === WORKING ? 'Working state' : 'Select a version'} subtitle={selected === WORKING ? `branch ${h.branch}` : 'Click a node in the graph'}>
              <div className="small" style={{ color: 'var(--text-2)' }}>
                {selected === WORKING
                  ? <>This is the live project every lab reads. {modified ? <>It differs from <b>{base?.name ?? 'the last version'}</b> — create a version to keep this state as a baseline.</> : 'It matches its base version.'}</>
                  : 'Versions are immutable snapshots of the whole project.json. Restore one to load it into every lab, branch from it to explore an alternative without losing main, or compare any two.'}
              </div>
              {selected === WORKING && base && <button className="btn small" style={{ marginTop: 10 }} onClick={() => compare(base.id, WORKING)}>Compare with {base.name}</button>}
            </Card>
          )}
          <Explainer title="Versions are baselines" refs={[{ label: 'International Organization for Standardization (ISO) 10007:2017 Quality management — Guidelines for configuration management' }, { label: 'NASA/SP-2016-6105 Rev 2, NASA Systems Engineering Handbook — configuration management process' }, { label: 'Onshape Learning Center: versions, branching and merging', href: 'https://learn.onshape.com' }]}>
            Configuration management keeps every decision traceable: a <b>baseline</b> is an agreed, frozen description of the product (functional, allocated, product baselines), changes are proposed against it, and <b>status accounting</b> records what changed and why. Onshape brings the same idea to CAD — immutable <i>versions</i>, a mutable <i>workspace</i>, <i>branches</i> for alternatives, and <i>compare</i>. Here a version freezes the whole project.json: requirements, BOM, and every design parameter the labs read, so comparing two versions also re-runs the verification.
            The fingerprint is a Fowler–Noll–Vo (FNV-1a) hash of the whole file:
            <Formula>{'fingerprint = FNV-1a-64(project.json)\nh ← (h ⊕ byte) · 1099511628211  mod 2⁶⁴,  h₀ = 14695981039346656037'}</Formula>
          </Explainer>
        </div>
      </div>
      <Card title="Compare two versions" subtitle="Both are evaluated by every lab in a background worker" className="vd-compare">
        <div id="vd-compare" className="row" style={{ gap: 8 }}>
          <label className="small muted" htmlFor="vd-a">A (before)</label>
          <select id="vd-a" value={cmp[0] ?? ''} onChange={(e) => setCmp([e.target.value, cmp[1]])} style={{ maxWidth: 280 }}>{options.map((o) => <option key={o.id} value={o.id}>{o.label}</option>)}</select>
          <button className="btn small" onClick={() => setCmp([cmp[1], cmp[0]])} aria-label="Swap A and B">⇄</button>
          <label className="small muted" htmlFor="vd-b">B (after)</label>
          <select id="vd-b" value={cmp[1] ?? ''} onChange={(e) => setCmp([cmp[0], e.target.value])} style={{ maxWidth: 280 }}>{options.map((o) => <option key={o.id} value={o.id}>{o.label}</option>)}</select>
          {diff.busy && <span className="small muted">Evaluating both versions…</span>}
        </div>
        <div className="small muted" style={{ marginTop: 6 }}>{labelOf(cmp[0])} → {labelOf(cmp[1])}</div>
      </Card>
      <ErrorBox error={diff.error} />
      {diff.data && <CompareView d={diff.data} />}
      <div className="row" style={{ justifyContent: 'flex-end' }}>
        <button className="btn small ghost" onClick={() => confirm('Delete the whole version history in this browser? (Export it first to keep a copy.)') && h.reset()}>Clear history</button>
      </div>
    </LabPage>
  );
}
