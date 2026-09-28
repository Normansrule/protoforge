// CAD Studio: parametric Iris frame generator, face fit, mass properties,
// exports and a generic enclosure generator.
import { useMemo, useRef, useState, type ReactNode } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import { LabPage } from '../../components/LabPage';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat, StatusChip, Tabs } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { call } from '../../engine/engine';
import { useProject } from '../../state/project';
import type { FrameParams, Project } from '../../engine/types';
import { CadCanvas, PartEdges, PartMesh, PresetButtons, prefersReducedMotion, useTheme, type Bounds, type ViewPreset } from './viewer';
import { MATERIALS, download3mf, downloadAllZip, downloadPartStl, type PartOut, type PartsOut } from './api';
import { FitTab } from './FitTab';
import { EnclosureTab } from './EnclosureTab';
import { LearnTab } from './LearnTab';
import './cadprint.css';

type TabId = 'design' | 'fit' | 'mass' | 'enclosure' | 'learn';

export default function Lab() {
  const [tab, setTab] = useState<TabId>('design');
  return (
    <LabPage
      kicker="Optics & Mechanics · Lab"
      title="CAD Studio"
      subtitle={
        <>
          A dependency-free parametric modeller running in WebAssembly. It builds the Iris frame from the boxing-system
          sizes — rims, bridge, hinges, the electronics and battery pods, lids and the display optics housing — as
          watertight, molding-drafted meshes, weighs them with the divergence theorem and exports STL/OBJ/3MF.
        </>
      }
      metricPrefixes={['cad.', 'mass.', 'fit.']}
    >
      <Tabs<TabId>
        tabs={[
          { id: 'design', label: 'Frame designer' },
          { id: 'fit', label: 'Face fit' },
          { id: 'mass', label: 'Mass & export' },
          { id: 'enclosure', label: 'Enclosure generator' },
          { id: 'learn', label: 'How it works' },
        ]}
        value={tab}
        onChange={setTab}
      />
      {tab === 'design' && <DesignTab />}
      {tab === 'fit' && <FitTab />}
      {tab === 'mass' && <MassTab />}
      {tab === 'enclosure' && <EnclosureTab />}
      {tab === 'learn' && <LearnTab />}
    </LabPage>
  );
}

// ------------------------------------------------------------------ data

export function useFrameParts(orientation: 'assembly' | 'print' = 'assembly') {
  const project = useProject((s) => s.project);
  return useCompute<PartsOut>('cad.parts', { project, orientation, display: orientation === 'assembly' });
}

function boundsOf(d: PartsOut): Bounds {
  return { min: d.total.bbox_min, max: d.total.bbox_max };
}

// ------------------------------------------------------------------ 3D scene

const tmpV = new THREE.Vector3();

/** Group whose position eases towards `to` every frame. */
function Glide(props: { to: [number, number, number]; children: ReactNode }) {
  const ref = useRef<THREE.Group>(null);
  const reduced = useMemo(prefersReducedMotion, []);
  useFrame((_, dt) => {
    const g = ref.current;
    if (!g) return;
    tmpV.set(...props.to);
    if (reduced) g.position.copy(tmpV);
    else g.position.lerp(tmpV, 1 - Math.exp(-dt * 6));
  });
  return <group ref={ref}>{props.children}</group>;
}

/** Rotation about the engine Z axis through (px, py), eased. */
function Hinge(props: { px: number; py: number; angle: number; children: ReactNode }) {
  const ref = useRef<THREE.Group>(null);
  const reduced = useMemo(prefersReducedMotion, []);
  useFrame((_, dt) => {
    const g = ref.current;
    if (!g) return;
    const k = reduced ? 1 : 1 - Math.exp(-dt * 5);
    g.rotation.z += (props.angle - g.rotation.z) * k;
  });
  return (
    <group position={[props.px, props.py, 0]}>
      <group ref={ref}>
        <group position={[-props.px, -props.py, 0]}>{props.children}</group>
      </group>
    </group>
  );
}

interface SceneOpts {
  explode: number;
  fold: number;
  hidden: Set<string>;
  xray: boolean;
  wire: boolean;
  edges: boolean;
  pcb: boolean;
  lenses: boolean;
  hovered: string | null;
  setHovered: (n: string | null) => void;
}

function FrameScene({ data, o }: { data: PartsOut; o: SceneOpts }) {
  const theme = useTheme();
  const L = data.layout;
  const edgeColor = theme === 'light' ? '#1d2735' : '#dce6f5';
  const byGroup = (g: string) => data.parts.filter((p) => p.group === g && !o.hidden.has(p.name));
  const disp = (g: string) => data.display.filter((d) => d.group === g && (d.kind === 'pcb' ? o.pcb : o.lenses));
  const tR = data.parts.find((p) => p.name === 'temple_r')?.explode ?? [0, 0, 0];
  const lR = data.parts.find((p) => p.name === 'lid_r')?.explode ?? [0, 0, 0];
  const pcbExplode: [number, number, number] = [(tR[0] + lR[0]) / 2, (tR[1] + lR[1]) / 2, (tR[2] + lR[2]) / 2];
  const front = data.parts.find((p) => p.name === 'front')?.explode ?? [0, 0, 0];
  const part = (p: PartOut) => (
    <Glide key={p.name} to={[p.explode[0] * o.explode, p.explode[1] * o.explode, p.explode[2] * o.explode]}>
      <PartMesh
        mesh={p.mesh}
        color={p.color}
        xray={o.xray}
        wireframe={o.wire}
        highlight={o.hovered === p.name}
        onHover={(on) => o.setHovered(on ? p.name : null)}
      />
      {o.edges && !o.wire && <PartEdges mesh={p.mesh} color={edgeColor} opacity={0.35} />}
    </Glide>
  );
  const item = (d: PartsOut['display'][number]) => {
    const e = d.kind === 'pcb' ? pcbExplode : front;
    return (
      <Glide key={d.name} to={[e[0] * o.explode, e[1] * o.explode, e[2] * o.explode]}>
        <PartMesh mesh={d.mesh} color={d.color} opacity={d.opacity} roughness={0.1} clearcoat={1} renderOrder={2} />
      </Glide>
    );
  };
  return (
    <>
      {byGroup('front').map(part)}
      {disp('front').map(item)}
      <Hinge px={L.pin_x} py={L.pin_y} angle={-o.fold}>
        {byGroup('right').map(part)}
        {disp('right').map(item)}
      </Hinge>
      <Hinge px={-L.pin_x} py={L.pin_y} angle={o.fold}>
        {byGroup('left').map(part)}
        {disp('left').map(item)}
      </Hinge>
    </>
  );
}

/** Parts in print orientation laid out on a 256 mm bed. */
function BedScene({ data }: { data: PartsOut }) {
  const theme = useTheme();
  const placed = useMemo(() => {
    const gap = 8;
    const items = data.parts
      .map((p) => ({ p, w: p.mass.size[0], h: p.mass.size[1] }))
      .sort((a, b) => b.h - a.h);
    let x = gap;
    let y = gap;
    let row = 0;
    return items.map((it) => {
      if (x + it.w > 256 - gap && x > gap) {
        x = gap;
        y += row + gap;
        row = 0;
      }
      const pos: [number, number, number] = [x + it.w / 2 - 128, y + it.h / 2 - 128, 0];
      x += it.w + gap;
      row = Math.max(row, it.h);
      return { ...it, pos };
    });
  }, [data]);
  return (
    <>
      <mesh position={[0, 0, -0.6]}>
        <boxGeometry args={[256, 256, 1.2]} />
        <meshStandardMaterial color={theme === 'light' ? '#d9dde3' : '#2a2f38'} roughness={0.8} metalness={0.2} />
      </mesh>
      {placed.map(({ p, pos }) => (
        <group key={p.name} position={pos}>
          <PartMesh mesh={p.mesh} color={p.color} />
        </group>
      ))}
    </>
  );
}

// ------------------------------------------------------------------ design tab

function DesignTab() {
  const project = useProject((s) => s.project);
  const [mode, setMode] = useState<'assembly' | 'print'>('assembly');
  const asm = useFrameParts('assembly');
  const prn = useFrameParts('print');
  const data = mode === 'assembly' ? asm.data : prn.data;
  const [preset, setPreset] = useState<ViewPreset>('iso');
  const [nonce, setNonce] = useState(0);
  const [explode, setExplode] = useState(0);
  const [fold, setFold] = useState(0);
  const [hidden, setHidden] = useState<Set<string>>(new Set());
  const [xray, setXray] = useState(false);
  const [wire, setWire] = useState(false);
  const [edges, setEdges] = useState(true);
  const [pcb, setPcb] = useState(true);
  const [lenses, setLenses] = useState(true);
  const [spin, setSpin] = useState(false);
  const [hovered, setHovered] = useState<string | null>(null);
  const bounds = useMemo<Bounds | null>(() => {
    if (!asm.data) return null;
    if (mode === 'print') return { min: [-128, -128, 0], max: [128, 128, 20] };
    return boundsOf(asm.data);
    // Only refit when switching mode, not on every parameter drag.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode, !!asm.data]);
  const hov = data?.parts.find((p) => p.name === hovered);
  const pickPreset = (p: ViewPreset) => {
    setPreset(p);
    setNonce((n) => n + 1);
  };
  const toggle = (name: string) =>
    setHidden((h) => {
      const n = new Set(h);
      if (n.has(name)) n.delete(name);
      else n.add(name);
      return n;
    });
  return (
    <div className="stack">
      <div className="cp-designer">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card
            title={mode === 'assembly' ? 'Iris frame — assembly' : 'Print layout — 256 × 256 mm bed'}
            subtitle={
              asm.data
                ? `${asm.data.parts.length} parts · ${asm.data.total.triangles.toLocaleString()} triangles · generated in ${asm.ms.toFixed(1)} ms`
                : 'generating…'
            }
            actions={
              <div className="cp-seg">
                {(['assembly', 'print'] as const).map((m) => (
                  <button key={m} className={`cp-seg-btn ${mode === m ? 'active' : ''}`} onClick={() => { setMode(m); setNonce((n) => n + 1); }}>
                    {m === 'assembly' ? 'as worn' : 'print bed'}
                  </button>
                ))}
              </div>
            }
          >
            <ErrorBox error={asm.error} />
            {data && bounds && (
              <CadCanvas
                bounds={bounds}
                preset={mode === 'print' ? 'top' : preset}
                nonce={nonce}
                autoRotate={spin}
                height="min(62vh, 560px)"
                fit={mode === 'print' ? 0.62 : 0.82}
                overlay={
                  <>
                    {mode === 'assembly' && (
                      <div className="cp-overlay">
                        <PresetButtons value={preset} onChange={pickPreset} />
                      </div>
                    )}
                    <div className="cp-hud">
                      {hov ? (
                        <>
                          <b>{hov.label}</b> · {hov.mass.mass_g.toFixed(2)} g · {hov.mass.volume_cm3.toFixed(2)} cm³ · {hov.shells} shells
                        </>
                      ) : mode === 'assembly' ? (
                        <>Drag to orbit · scroll to zoom · hover a part</>
                      ) : (
                        <>Every part lies on its largest face — no supports needed</>
                      )}
                    </div>
                  </>
                }
              >
                {mode === 'assembly' ? (
                  <FrameScene data={data} o={{ explode, fold: (fold * Math.PI) / 180, hidden, xray, wire, edges, pcb, lenses, hovered, setHovered }} />
                ) : (
                  <BedScene data={data} />
                )}
              </CadCanvas>
            )}
            {mode === 'assembly' && (
              <div className="cp-toolbar">
                <Slider label="Explode" value={explode} min={0} max={1} step={0.01} onChange={setExplode} format={(v) => `${Math.round(v * 100)}%`} />
                <Slider label="Fold temples" value={fold} min={0} max={85} step={1} unit="°" onChange={setFold} />
                <label className="cp-check"><input type="checkbox" checked={xray} onChange={(e) => setXray(e.target.checked)} /> X-ray</label>
                <label className="cp-check"><input type="checkbox" checked={wire} onChange={(e) => setWire(e.target.checked)} /> Wireframe</label>
                <label className="cp-check"><input type="checkbox" checked={edges} onChange={(e) => setEdges(e.target.checked)} /> Edges</label>
                <label className="cp-check"><input type="checkbox" checked={pcb} onChange={(e) => setPcb(e.target.checked)} /> PCB</label>
                <label className="cp-check"><input type="checkbox" checked={lenses} onChange={(e) => setLenses(e.target.checked)} /> Lenses &amp; combiner</label>
                <label className="cp-check"><input type="checkbox" checked={spin} onChange={(e) => setSpin(e.target.checked)} /> Auto-rotate</label>
              </div>
            )}
          </Card>
          {asm.data && <FrameStats data={asm.data} project={project} />}
          {asm.data && (
            <Card title="Parts" subtitle="Toggle visibility; hover to highlight in the 3D view">
              <div className="cp-parts-list">
                {asm.data.parts.map((p) => (
                  <div
                    key={p.name}
                    className={`cp-part-row ${hovered === p.name ? 'hover' : ''}`}
                    onMouseEnter={() => setHovered(p.name)}
                    onMouseLeave={() => setHovered(null)}
                  >
                    <input type="checkbox" checked={!hidden.has(p.name)} onChange={() => toggle(p.name)} aria-label={`Show ${p.label}`} style={{ accentColor: 'var(--accent)' }} />
                    <span className="cp-swatch" style={{ background: p.color }} />
                    <span>
                      {p.label} <span className="muted mono">{p.name}</span>
                    </span>
                    <span className="row" style={{ gap: 8 }}>
                      <span className="mono small">{p.mass.mass_g.toFixed(2)} g</span>
                      <StatusChip status={p.check.watertight ? 'pass' : 'fail'} />
                    </span>
                  </div>
                ))}
              </div>
            </Card>
          )}
          {asm.data && asm.data.warnings.length > 0 && (
            <div className="callout" style={{ borderLeftColor: 'var(--warn)' }}>
              <h4>Parameter warnings</h4>
              {asm.data.warnings.map((w) => <div key={w}>! {w}</div>)}
            </div>
          )}
          <Explainer
            title="How this frame is built"
            refs={[
              { label: 'ISO 8624 — spectacle frame measuring system' },
              { label: 'M. de Berg et al., Computational Geometry, 3rd ed. (2008), ch. 3' },
            ]}
          >
            Each part is a <b>stack of extrusions</b> of 2D regions: the front is the rim outline around two superellipse
            lens openings (|x/a|ⁿ + |y/b|ⁿ = 1), extruded {project.design.frame.front_thickness_mm} mm; each temple is its
            side profile — knuckle, pod and an arm swept along a centre line with the bend behind the ear — with the pod
            pocket opening outwards and a lid on top. Every wall tapers by the draft angle so the same geometry could be
            injection-molded, and every shell is a closed 2-manifold (each edge shared by exactly two triangles in opposite
            directions), so its volume is exact. Change any slider: the requirements above, the Print Lab's time estimate and
            the Molding Lab's draft check all update.
          </Explainer>
        </div>
        <div className="cp-sticky">
          <FrameParamsPanel />
        </div>
      </div>
    </div>
  );
}

function FrameStats({ data, project }: { data: PartsOut; project: Project }) {
  const L = data.layout;
  const margin = L.pod_usable_len_mm - project.design.pcb.width_mm;
  return (
    <div className="cp-stats">
      <Stat label="Frame mass" value={data.frame_mass_g.toFixed(1)} unit="g" note={`${data.total.volume_cm3.toFixed(2)} cm³ of ${data.material}`} />
      <Stat
        label="Wearable total"
        value={data.wearable_mass_g.toFixed(1)}
        unit="g"
        tone={data.wearable_mass_g <= 50 ? 'pass' : 'fail'}
        note={`${data.wearable_mass_g <= 50 ? '✓' : '✕'} ≤ 50 g (R-001) · parts ${data.components_mass_g.toFixed(1)} g`}
      />
      <Stat
        label="PCB fit margin"
        value={margin.toFixed(2)}
        unit="mm"
        tone={margin >= 1 ? 'pass' : 'fail'}
        note={`${margin >= 1 ? '✓' : '✕'} ≥ 1 mm (R-019) · usable ${L.pod_usable_len_mm.toFixed(1)} mm`}
      />
      <Stat label="Frame PD (A + DBL)" value={L.frame_pd_mm.toFixed(0)} unit="mm" note={`front ${L.total_width_mm.toFixed(0)} × ${L.front_height_mm.toFixed(0)} mm`} />
      <Stat label="Pod cavity" value={L.pod_cavity_len_floor_mm.toFixed(1)} unit="mm" note={`× ${L.pod_cavity_height_floor_mm.toFixed(1)} high × ${L.pod_cavity_depth_mm.toFixed(1)} deep`} />
    </div>
  );
}

// ------------------------------------------------------------------ parameters

type Key = keyof FrameParams;
interface Def { k: Key; label: string; min: number; max: number; step: number; unit: string; hint?: string }

const GROUPS: { title: string; defs: Def[] }[] = [
  {
    title: 'Fit (boxing system)',
    defs: [
      { k: 'lens_width_mm', label: 'Lens width A', min: 40, max: 62, step: 0.5, unit: 'mm', hint: 'Horizontal box width of one lens (ISO 8624)' },
      { k: 'bridge_mm', label: 'Bridge DBL', min: 12, max: 26, step: 0.5, unit: 'mm', hint: 'Distance between lenses' },
      { k: 'temple_length_mm', label: 'Temple length', min: 120, max: 165, step: 1, unit: 'mm' },
      { k: 'pantoscopic_tilt_deg', label: 'Pantoscopic tilt', min: 0, max: 15, step: 0.5, unit: '°', hint: 'Front tilted so the lens bottoms move towards the cheeks' },
    ],
  },
  {
    title: 'Front',
    defs: [
      { k: 'lens_height_mm', label: 'Lens height B', min: 26, max: 52, step: 0.5, unit: 'mm' },
      { k: 'lens_shape_exponent', label: 'Lens shape exponent n', min: 2, max: 6, step: 0.1, unit: '', hint: '2 = ellipse, larger = squarer' },
      { k: 'rim_width_mm', label: 'Rim width', min: 2.5, max: 8, step: 0.1, unit: 'mm' },
      { k: 'front_thickness_mm', label: 'Front thickness', min: 3, max: 8, step: 0.1, unit: 'mm' },
    ],
  },
  {
    title: 'Temples',
    defs: [
      { k: 'temple_width_mm', label: 'Temple width', min: 3, max: 6, step: 0.1, unit: 'mm' },
      { k: 'temple_height_mm', label: 'Temple height', min: 5, max: 12, step: 0.1, unit: 'mm' },
    ],
  },
  {
    title: 'Pods',
    defs: [
      { k: 'pod_length_mm', label: 'Electronics pod length', min: 40, max: 80, step: 0.5, unit: 'mm' },
      { k: 'pod_width_mm', label: 'Pod width (outward)', min: 8, max: 16, step: 0.1, unit: 'mm' },
      { k: 'pod_height_mm', label: 'Pod height', min: 10, max: 20, step: 0.1, unit: 'mm' },
      { k: 'battery_pod_length_mm', label: 'Battery pod length', min: 25, max: 60, step: 0.5, unit: 'mm' },
      { k: 'wall_mm', label: 'Wall thickness', min: 0.8, max: 2.5, step: 0.05, unit: 'mm' },
    ],
  },
  {
    title: 'Molding & material',
    defs: [{ k: 'draft_deg', label: 'Draft angle', min: 0, max: 5, step: 0.1, unit: '°', hint: 'Taper of every wall relative to the mold pull direction' }],
  },
];

function FrameParamsPanel() {
  const project = useProject((s) => s.project);
  const patch = useProject((s) => s.patchDesign);
  const reset = useProject((s) => s.reset);
  const f = project.design.frame;
  const set = (k: Key, v: number) => patch('frame', { [k]: v } as Partial<FrameParams>);
  return (
    <Card
      title="Parameters"
      subtitle={`${f.lens_width_mm} □ ${f.bridge_mm} – ${f.temple_length_mm}`}
      actions={
        <button
          className="btn small"
          title="Restore the Iris reference project"
          onClick={() => {
            if (confirm('Reset the whole project to the Iris reference?')) reset();
          }}
        >
          Reset
        </button>
      }
    >
      {GROUPS.map((g) => (
        <div className="cp-group" key={g.title}>
          <div className="cp-group-title">{g.title}</div>
          {g.defs.map((d) => (
            <Slider
              key={d.k}
              label={d.label}
              hint={d.hint}
              value={Number(f[d.k])}
              min={d.min}
              max={d.max}
              step={d.step}
              unit={d.unit}
              onChange={(v) => set(d.k, v)}
            />
          ))}
          {g.title.startsWith('Molding') && (
            <>
              <Select
                label="Material"
                value={f.material}
                options={MATERIALS.map((m) => ({ value: m.name, label: `${m.name} — ${m.density} g/cm³` }))}
                onChange={(v) => {
                  const m = MATERIALS.find((x) => x.name === v)!;
                  patch('frame', { material: m.name, density_g_cm3: m.density });
                }}
              />
              <div className="muted small">{MATERIALS.find((m) => m.name === f.material)?.note ?? ''}</div>
            </>
          )}
        </div>
      ))}
    </Card>
  );
}

// ------------------------------------------------------------------ mass & export tab

function MassTab() {
  const project = useProject((s) => s.project);
  const { data, error } = useFrameParts('assembly');
  const [orient, setOrient] = useState<'print' | 'assembly'>('print');
  const [busy, setBusy] = useState<string | null>(null);
  if (!data) return <ErrorBox error={error} />;
  const comps = new Map<string, number>();
  for (const c of project.components) {
    const k = c.location || 'other';
    comps.set(k, (comps.get(k) ?? 0) + c.mass_g * c.qty);
  }
  const segments = [
    ...data.parts.map((p) => ({ label: p.name, g: p.mass.mass_g, color: p.color })),
    ...[...comps.entries()].map(([k, g], i) => ({ label: `parts: ${k}`, g, color: `var(--series-${(i % 8) + 1})` })),
  ];
  const total = data.wearable_mass_g;
  const scale = Math.max(total, 50);
  const run = async (label: string, fn: () => Promise<void> | void) => {
    setBusy(label);
    await new Promise((r) => setTimeout(r, 20));
    try {
      await fn();
    } finally {
      setBusy(null);
    }
  };
  return (
    <div className="stack">
      <div className="grid grid-2">
        <Card title="Mass budget" subtitle="Frame parts (volume × density) plus the components list — requirement R-001: ≤ 50 g">
          <div className="cp-stats" style={{ marginBottom: 14 }}>
            <Stat label="Frame" value={data.frame_mass_g.toFixed(2)} unit="g" />
            <Stat label="Components" value={data.components_mass_g.toFixed(2)} unit="g" />
            <Stat label="Wearable" value={total.toFixed(2)} unit="g" tone={total <= 50 ? 'pass' : 'fail'} note={`${total <= 50 ? '✓' : '✕'} margin ${(50 - total).toFixed(1)} g`} />
          </div>
          <div className="cp-stackbar" role="img" aria-label="Mass breakdown">
            {segments.map((s) => (
              <div key={s.label} title={`${s.label}: ${s.g.toFixed(2)} g`} style={{ width: `${(s.g / scale) * 100}%`, background: s.color }} />
            ))}
          </div>
          <div className="small muted" style={{ display: 'flex', justifyContent: 'space-between', marginTop: 4 }}>
            <span>0 g</span>
            <span>50 g limit{scale > 50 ? ` (bar to ${scale.toFixed(0)} g)` : ''}</span>
          </div>
          <div className="cp-legend">
            {segments.map((s) => (
              <span key={s.label}>
                <span className="cp-swatch" style={{ background: s.color }} /> {s.label} {s.g.toFixed(1)} g
              </span>
            ))}
          </div>
        </Card>
        <Card title="Downloads" subtitle="Binary STL per part, a zip of all parts, OBJ with part groups, or a 3MF plate for Bambu Studio / OrcaSlicer">
          <div className="row" style={{ marginBottom: 10 }}>
            <span className="small muted">Orientation</span>
            <div className="cp-seg">
              {(['print', 'assembly'] as const).map((o) => (
                <button key={o} className={`cp-seg-btn ${orient === o ? 'active' : ''}`} onClick={() => setOrient(o)}>
                  {o === 'print' ? 'print (flat, Z = 0)' : 'as worn'}
                </button>
              ))}
            </div>
          </div>
          <div className="row" style={{ marginBottom: 12 }}>
            {data.parts.map((p) => (
              <button key={p.name} className="btn small" onClick={() => downloadPartStl(project, p.name, orient)}>
                ↓ {p.name}.stl
              </button>
            ))}
          </div>
          <div className="row">
            <button className="btn primary" disabled={!!busy} onClick={() => run('zip', () => downloadAllZip(project, data.parts.map((p) => p.name), orient))}>
              {busy === 'zip' ? <span className="cp-spin" /> : '↓'} All parts (.zip)
            </button>
            <button className="btn" disabled={!!busy} onClick={() => run('3mf', () => download3mf(project))}>
              {busy === '3mf' ? <span className="cp-spin" /> : '↓'} Print plate (.3mf)
            </button>
            <DownloadButton
              filename="iris_frame.obj"
              label="Assembly (.obj)"
              content={() => call<{ text: string }>('cad.obj', { project, orientation: 'assembly' }).text}
            />
          </div>
          <p className="small muted" style={{ marginTop: 12 }}>
            STL carries no units — every slicer assumes millimetres. The 3MF declares <code>unit="millimeter"</code>, names
            each part and lays them out on a 256 mm plate (Bambu Lab X1/P1 size).
          </p>
        </Card>
      </div>
      <Card title="Mass properties" subtitle={`${data.material} at ${data.density_g_cm3} g/cm³ · centroid and bounding box in assembly coordinates (mm)`}>
        <div className="table-wrap">
          <table className="table">
            <thead>
              <tr>
                <th>Part</th>
                <th>Volume cm³</th>
                <th>Area cm²</th>
                <th>Mass g</th>
                <th>Centroid (x, y, z)</th>
                <th>Size (x × y × z)</th>
                <th>Triangles</th>
                <th>Mesh check</th>
              </tr>
            </thead>
            <tbody>
              {data.parts.map((p) => (
                <tr key={p.name}>
                  <td><span className="cp-swatch" style={{ background: p.color }} /> {p.label}</td>
                  <td className="mono">{p.mass.volume_cm3.toFixed(3)}</td>
                  <td className="mono">{(p.mass.area_mm2 / 100).toFixed(1)}</td>
                  <td className="mono">{p.mass.mass_g.toFixed(2)}</td>
                  <td className="mono small">{p.mass.centroid.map((v) => v.toFixed(1)).join(', ')}</td>
                  <td className="mono small">{p.mass.size.map((v) => v.toFixed(1)).join(' × ')}</td>
                  <td className="mono">{p.mass.triangles.toLocaleString()}</td>
                  <td>
                    <span title={`${p.shells} closed shells, ${p.check.boundary_edges} open edges, ${p.check.misoriented_edges} flipped edges`}>
                      <StatusChip status={p.check.watertight ? 'pass' : 'fail'} /> <span className="small muted">{p.check.shells} shells</span>
                    </span>
                  </td>
                </tr>
              ))}
              <tr>
                <td><b>Total</b></td>
                <td className="mono"><b>{data.total.volume_cm3.toFixed(3)}</b></td>
                <td className="mono">{(data.total.area_mm2 / 100).toFixed(1)}</td>
                <td className="mono"><b>{data.total.mass_g.toFixed(2)}</b></td>
                <td className="mono small">{data.total.centroid.map((v) => v.toFixed(1)).join(', ')}</td>
                <td className="mono small">{data.total.size.map((v) => v.toFixed(1)).join(' × ')}</td>
                <td className="mono">{data.total.triangles.toLocaleString()}</td>
                <td />
              </tr>
            </tbody>
          </table>
        </div>
      </Card>
      <div className="grid grid-2">
        <Explainer
          title="Volume from the surface alone: the divergence theorem"
          refs={[{ label: 'C. Zhang & T. Chen, "Efficient feature extraction for 2D/3D objects in mesh representation", ICIP 2001' }]}
        >
          For a closed surface with outward normals, ∭ ∇·F dV = ∯ F·n dA. Choosing F = (x, y, z)/3 (so ∇·F = 1) turns the
          volume into a sum over triangles — each one forms a signed tetrahedron with the origin:
          <Formula>{'V = Σ_t  v0 · (v1 × v2) / 6        centroid = Σ_t V_t (v0 + v1 + v2) / 4 / V'}</Formula>
          Triangles facing away from the origin add volume, those facing it subtract, and everything outside the solid
          cancels. It only works if the mesh is <b>watertight and consistently oriented</b> — which is why every part
          above is validated edge by edge. Mass = V × ρ.
        </Explainer>
        <Explainer title="Why the frame mass matters for Iris" refs={[{ label: 'Iris requirement R-001 (≤ 50 g wearable)' }]}>
          Comfortable all-day glasses stay near 30–40 g; heavier frames slide down the nose and press on the ears. Iris
          carries ~{data.components_mass_g.toFixed(0)} g of electronics, optics and battery, which leaves about{' '}
          {(50 - data.components_mass_g).toFixed(0)} g for the frame. Thinner rims, a thinner front, a lighter material (ABS,
          PA12) or a shorter pod are the levers — try them in the Frame designer and watch R-001.
          {data.frame_mass_g > 28 && (
            <div style={{ marginTop: 6 }}>
              <Chip tone="warn">! heavy frame</Chip> The current frame ({data.frame_mass_g.toFixed(1)} g) is at the heavy end of
              printed frames; the 5 mm × 5 mm rim front and the 18 mm collimator housing dominate.
            </div>
          )}
        </Explainer>
      </div>
    </div>
  );
}
