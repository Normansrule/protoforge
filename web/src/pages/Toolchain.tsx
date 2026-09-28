// Web toolchain: one-click hand-off of every Protoforge design to the real
// web-based and open-source engineering tools (KiCad, KiCanvas, Onshape,
// OpenSCAD, Falstad, ngspice, EDA Playground, Ray Optics …).
import { useMemo, useRef, useState } from 'react';
import { LabPage } from '../components/LabPage';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Stat, Tabs } from '../components/ui';
import { useCompute } from '../engine/hooks';
import { useProject } from '../state/project';
import { useWorkerCompute } from '../labs/pcb/useWorker';
import { FlowDiagram } from './toolchain/FlowDiagram';
import { KicadPreview, useBoard } from './toolchain/KicadPreview';
import { CodeView } from './toolchain/CodeView';
import { CircuitsTab } from './toolchain/CircuitsTab';
import { MoreTab } from './toolchain/MoreTab';
import { Verification, VERIFIED_COUNT } from './toolchain/Verification';
import { downloadFiles, useBundle } from './toolchain/bundle';
import { DISCIPLINES, TOOLS, type Tool } from './toolchain/tools';
import type { FalstadCircuit, KicadBoard, StepFile } from './toolchain/types';
import './toolchain/toolchain.css';

type Tab = 'map' | 'kicad' | 'cad' | 'circuits' | 'more' | 'verify';

function ToolCard({ tool, picked, onTab, bundle }: { tool: Tool; picked: boolean; onTab: (t: Tab) => void; bundle: ReturnType<typeof useBundle> }) {
  return (
    <section id={`tool-${tool.id}`} className={`card tc-tool ${picked ? 'picked' : ''}`}>
      <div className="row" style={{ justifyContent: 'space-between', alignItems: 'flex-start' }}>
        <h3 style={{ margin: 0 }}>{tool.name}</h3>
        <Chip>{tool.kind}</Chip>
      </div>
      <div className="small" style={{ margin: '6px 0 8px', color: 'var(--text-2)' }}>{tool.purpose}</div>
      <div className="row" style={{ gap: 5, marginBottom: 10 }}>{tool.formats.map((f) => <span key={f} className="tc-fmt mono">{f}</span>)}</div>
      <div className="row" style={{ gap: 6 }}>
        <a className="btn small primary" href={tool.url} target="_blank" rel="noreferrer">Open {tool.name} ↗</a>
        {tool.tab && <button className="btn small" onClick={() => onTab(tool.tab!)}>Preview & links</button>}
        {tool.downloads?.map((d) => (
          <button key={d.label} className="btn small" disabled={bundle.busy} onClick={() => bundle.get().then((files) => downloadFiles(files, d))}>↓ {d.label}</button>
        ))}
        {tool.lab && <a className="btn small ghost" href={tool.lab.href}>{tool.lab.label} →</a>}
      </div>
      <details className="tc-steps">
        <summary className="small">How to open Protoforge’s files</summary>
        <ol className="small">{tool.steps.map((s, i) => <li key={i}>{s}</li>)}</ol>
      </details>
    </section>
  );
}

function KiCanvasBuilder({ filename }: { filename: string }) {
  const [url, setUrl] = useState(`https://github.com/<you>/<repo>/blob/main/hardware/${filename}`);
  const ok = /^https:\/\/github\.com\/[^/<>]+\/[^/<>]+\/blob\/[^<>]+\.kicad_pcb$/.test(url.trim());
  const link = `https://kicanvas.org/?github=${encodeURIComponent(url.trim())}`;
  return (
    <div className="stack" style={{ gap: 8 }}>
      <label className="field" style={{ marginBottom: 0 }}>
        <span className="field-top"><span className="field-label">GitHub URL of your committed board file</span></span>
        <input type="text" value={url} onChange={(e) => setUrl(e.target.value)} spellCheck={false} />
      </label>
      <div className="small mono tc-url">{link}</div>
      <div className="row">
        <a className={`btn small ${ok ? 'primary' : ''}`} href={ok ? link : undefined} aria-disabled={!ok} target="_blank" rel="noreferrer" onClick={(e) => !ok && e.preventDefault()}>Open in KiCanvas ↗</a>
        {!ok && <span className="small muted">Replace &lt;you&gt;/&lt;repo&gt; with a public repository that contains the file.</span>}
      </div>
    </div>
  );
}

function StepCard({ step, busy }: { step: StepFile | null; busy: boolean }) {
  if (!step) return <Card title="STEP (ISO 10303-21)"><div className="muted small">{busy ? 'Writing STEP…' : '—'}</div></Card>;
  const s = step.stats;
  const genus = (chi: number) => (2 - chi) / 2;
  return (
    <Card
      title="STEP (ISO 10303-21, AP214) — faceted B-rep"
      subtitle={`${s.parts.length} parts · ${s.shells} closed shells · ${s.faces.toLocaleString()} planar faces · ${(s.bytes / 1e6).toFixed(2)} MB`}
      actions={<DownloadButton filename={step.filename} content={() => new Blob([step.content], { type: 'application/step' })} label={step.filename} small primary />}
    >
      <div className="table-wrap">
        <table className="table">
          <thead><tr><th>Part</th><th style={{ textAlign: 'right' }}>Shells</th><th style={{ textAlign: 'right' }}>Faces</th><th style={{ textAlign: 'right' }}>Volume</th><th>χ = V − E + F</th></tr></thead>
          <tbody>
            {s.parts.map((p) => (
              <tr key={p.name}>
                <td className="mono">{p.name}</td>
                <td className="tnum" style={{ textAlign: 'right' }}>{p.closed_shells}/{p.shells} {p.closed_shells === p.shells ? '✓' : '✕'}</td>
                <td className="tnum" style={{ textAlign: 'right' }}>{p.faces.toLocaleString()}</td>
                <td className="tnum" style={{ textAlign: 'right' }}>{(p.volume_mm3 / 1000).toFixed(2)} cm³</td>
                <td className="small">{p.euler.map((c, i) => <span key={i} className="tc-chi" title={`genus ${genus(c)}`}>{c}{genus(c) > 0 ? ` (${genus(c)} hole${genus(c) > 1 ? 's' : ''})` : ''}</span>)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="small muted" style={{ marginTop: 8 }}>{step.notes.map((n, i) => <div key={i}>• {n}</div>)}</div>
    </Card>
  );
}

export default function Toolchain() {
  const project = useProject((s) => s.project);
  const [tab, setTab] = useState<Tab>('map');
  const [picked, setPicked] = useState<string | null>(null);
  const bundle = useBundle(project);
  const kicad = useWorkerCompute<KicadBoard>('bridge.kicad_pcb', { project });
  const { board, error: parseErr } = useBoard(kicad.data?.content);
  const [frame, setFrame] = useState<'assembly' | 'print'>('assembly');
  const step = useWorkerCompute<StepFile>('bridge.step', tab === 'cad' ? { project, frame, timestamp: '' } : null);
  const scad = useCompute<string>('bridge.openscad', { project }, tab === 'cad');
  const falstad = useCompute<FalstadCircuit[]>('bridge.falstad', { project });
  const topRef = useRef<HTMLDivElement>(null);
  const full = falstad.data?.filter((c) => c.support === 'full').length ?? 0;
  const formats = useMemo(() => new Set(TOOLS.flatMap((t) => t.formats)).size, []);

  const pick = (id: string) => {
    setTab('map');
    setPicked(id);
    setTimeout(() => document.getElementById(`tool-${id}`)?.scrollIntoView({ behavior: 'smooth', block: 'center' }), 50);
  };

  return (
    <LabPage
      kicker="Hand-off · Real tools"
      title="Web toolchain"
      subtitle={<>Every design in {project.meta.name} leaves Protoforge as a file a real tool opens: the routed printed circuit board (PCB) as a KiCad project, the frame as Standard for the Exchange of Product model data (STEP) and a parametric OpenSCAD program, every circuit as a one-click Falstad CircuitJS link and a Simulation Program with Integrated Circuit Emphasis (SPICE) deck, the heads-up display (HUD) as a Ray Optics scene, the co-processor as Verilog hardware description language (HDL). Each exporter is checked by re-reading its output with an independent parser.</>}
      actions={
        <button className="btn primary" disabled={bundle.busy} onClick={() => bundle.get().then((files) => downloadFiles(files, { label: 'bundle', filename: `${project.meta.codename || project.meta.id || 'product'}-toolchain-bundle.zip`, match: () => true, zip: true }))}>
          {bundle.busy ? 'Building bundle…' : '↓ Download toolchain bundle (.zip)'}
        </button>
      }
    >
      <div ref={topRef} />
      <ErrorBox error={bundle.error || kicad.error || parseErr || falstad.error} />
      <div className="grid grid-4">
        <Stat label="Real tools" value={TOOLS.length} note={`${DISCIPLINES.length} disciplines · ${TOOLS.filter((t) => t.kind.includes('web')).length} run in a browser`} />
        <Stat label="Hand-off formats" value={formats} note="KiCad, Gerber, STEP, OpenSCAD, stereolithography (STL), G-code, SPICE, Verilog …" />
        <Stat label="Falstad circuits" value={falstad.data?.length ?? '…'} note={falstad.data ? `${full} exact · ${falstad.data.length - full} with listed simplifications` : 'converting…'} tone={falstad.data?.every((c) => c.connectivity_ok) ? 'pass' : undefined} />
        <Stat label="Checked by an outside parser" value={`${VERIFIED_COUNT} exports`} note="kiutils · OpenCASCADE (gmsh) · OpenSCAD · lz-string (JS)" tone="pass" />
      </div>
      <Tabs<Tab>
        value={tab}
        onChange={setTab}
        tabs={[
          { id: 'map', label: 'Tool map' },
          { id: 'kicad', label: 'KiCad board' },
          { id: 'cad', label: 'CAD · STEP · OpenSCAD' },
          { id: 'circuits', label: 'Circuits · Falstad' },
          { id: 'more', label: 'Optics · chips · more' },
          { id: 'verify', label: 'What is verified' },
        ]}
      />

      {tab === 'map' && (
        <>
          <Card title="From one project file to the tools that build it" subtitle="Hover a tool to trace its files back to the engine; click it to jump to its card">
            <FlowDiagram onPick={pick} />
          </Card>
          {DISCIPLINES.map((d) => (
            <div key={d} className="stack" style={{ gap: 10 }}>
              <h2 className="tc-disc">{d}</h2>
              <div className="grid grid-3">{TOOLS.filter((t) => t.discipline === d).map((t) => <ToolCard key={t.id} tool={t} picked={picked === t.id} onTab={setTab} bundle={bundle} />)}</div>
            </div>
          ))}
          <Explainer
            title="Why hand off at all?"
            refs={[
              { label: 'KiCad file formats', href: 'https://dev-docs.kicad.org/en/file-formats/' },
              { label: 'ISO 10303-21 (STEP physical file)' },
              { label: 'OpenSCAD documentation', href: 'https://openscad.org/documentation.html' },
              { label: 'Falstad CircuitJS', href: 'https://www.falstad.com/circuit/' },
              { label: 'Onshape Learning Center', href: 'https://learn.onshape.com' },
            ]}
          >
            Protoforge is where you explore and verify; production happens in tools a factory trusts. Open, documented file formats are the contract between the two: a KiCad board is plain S-expressions, a STEP file is an ISO 10303-21 text of numbered entities, a CircuitJS circuit is one line per element. Because every format is text, each export here is re-read by an independent parser before you rely on it.
          </Explainer>
        </>
      )}

      {tab === 'kicad' && (
        <div className="grid grid-sidebar tc-kicad">
          <div className="stack">
            <Card title="KiCanvas-style preview, re-read from the exported file" subtitle={kicad.data ? `${kicad.data.filename} · ${kicad.data.stats.width_mm} × ${kicad.data.stats.height_mm} mm · ${kicad.data.stats.layers} layers` : 'writing the board…'}>
              {board && kicad.data ? <KicadPreview board={board} stats={kicad.data.stats} /> : <div className="muted small">{kicad.busy ? 'Routing / writing the board in the worker…' : '—'}</div>}
            </Card>
            {kicad.data && <Card title="The file itself"><CodeView code={kicad.data.content} filename={kicad.data.filename} lang="plain" maxLines={140} maxHeight={360} /></Card>}
          </div>
          <div className="stack">
            <Card title="Downloads">
              <div className="stack" style={{ gap: 8 }}>
                {kicad.data && <DownloadButton filename={kicad.data.filename} content={() => new Blob([kicad.data!.content], { type: 'text/plain' })} label={kicad.data.filename} primary />}
                <button className="btn" disabled={bundle.busy} onClick={() => bundle.get().then((f) => downloadFiles(f, TOOLS.find((t) => t.id === 'kicad')!.downloads![0]))}>↓ KiCad project (.zip: board, .kicad_pro, netlist, footprint library)</button>
              </div>
            </Card>
            <Card title="Open in KiCanvas (browser)" subtitle="kicanvas.org reads a public GitHub file URL from its ?github= parameter">
              <KiCanvasBuilder filename={kicad.data?.filename ?? 'iris.kicad_pcb'} />
            </Card>
            {kicad.data && <div className="callout small">{kicad.data.notes.map((n, i) => <div key={i}>• {n}</div>)}</div>}
            <Explainer title="Two coordinate systems" refs={[{ label: 'KiCad file formats: board file', href: 'https://dev-docs.kicad.org/en/file-formats/' }, { label: 'KiCanvas source (theacodes/kicanvas)', href: 'https://github.com/theacodes/kicanvas' }]}>
              Protoforge measures from the board's bottom-left corner with y up; KiCad measures from the sheet's top-left with y down and turns angles counter-clockwise on screen. Each point and pad is mapped as
              <Formula>{'X = X₀ + x        Y = Y₀ + H − y\npad (local) = (pₓ, −p_y),  angle unchanged\nabsolute = anchor + R(θ)·local'}</Formula>
              The test suite re-applies KiCad's rule to every pad in the file and checks it lands on the engine's pad (to 0.1 µm) with the same net.
            </Explainer>
          </div>
        </div>
      )}

      {tab === 'cad' && (
        <div className="stack">
          <div className="row">
            <span className="small muted">STEP orientation:</span>
            {(['assembly', 'print'] as const).map((f) => <button key={f} className={`chip ${frame === f ? 'info' : ''}`} style={{ cursor: 'pointer' }} aria-pressed={frame === f} onClick={() => setFrame(f)}>{f === 'assembly' ? 'as worn (assembly)' : 'flat, mold pull = +Z (print)'}</button>)}
          </div>
          <ErrorBox error={step.error} />
          <div className="grid grid-2">
            <StepCard step={step.data} busy={step.busy} />
            <Explainer title="Faceted STEP: a mesh in STEP clothing" refs={[{ label: 'ISO 10303-21 Clear text encoding of the exchange structure' }, { label: 'ISO 10303-42 Geometric and topological representation' }, { label: 'Onshape Learning Center', href: 'https://learn.onshape.com' }]}>
              Every triangle of the CAD mesh becomes a planar FACE_SURFACE bounded by a POLY_LOOP; each watertight piece becomes a CLOSED_SHELL inside a FACETED_BREP, in millimetres, with its colour. It imports everywhere and is fine for viewing, fit checks, quotes and printing services — but a mold maker wants native B-rep (real cylinders and splines): rebuild the parts in Onshape or FreeCAD for tooling.
              <Formula>{'χ = V − E + F = 2 − 2g   (closed, orientable shell of genus g)\nfront: two lens openings → g = 2 → χ = −2'}</Formula>
              The Euler characteristic of each exported shell is computed from the file's own vertices, edges and faces — a quick topological check that the shells are closed.
            </Explainer>
          </div>
          <Card title="OpenSCAD — the frame as an editable program" subtitle="Parameters come from design.frame and design.optics; edit them in the Customizer and re-render" actions={<a className="btn small primary" href="https://ochafik.com/openscad2" target="_blank" rel="noreferrer">OpenSCAD Playground ↗</a>}>
            {scad.data ? <CodeView code={scad.data} filename={`${project.meta.codename?.toLowerCase() || 'frame'}-frame.scad`} lang="scad" maxLines={400} maxHeight={460} /> : <div className="muted small">…</div>}
            <ol className="small" style={{ marginTop: 10 }}>
              <li>Open the OpenSCAD Playground (OpenSCAD compiled to WebAssembly) or desktop OpenSCAD.</li>
              <li>Paste the program, press F5 to preview, change e.g. <code className="mono">bridge = 20;</code>, F6 to render.</li>
              <li>Export STL/3MF and slice it — or compare it with the exact meshes from CAD Studio.</li>
            </ol>
            <div className="small muted">A simplified re-statement of CAD Studio's generator: same numbers, close shapes (superellipse rims, drafted extrusions via <code className="mono">linear_extrude(scale = (w − 2h·tan θ)/w)</code>, hollow pods, bent temples).</div>
          </Card>
        </div>
      )}

      {tab === 'circuits' && <CircuitsTab circuits={falstad.data} />}
      {tab === 'more' && <MoreTab bundle={bundle} />}
      {tab === 'verify' && <Verification />}
    </LabPage>
  );
}
