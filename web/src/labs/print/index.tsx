// Print Lab: an FDM slicer for the Iris frame parts (and the enclosure).
import { useEffect, useMemo, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { LabPage } from '../../components/LabPage';
import { Card, Chip, Code, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat, StatusChip, Tabs, Toggle } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { call } from '../../engine/engine';
import { useProject } from '../../state/project';
import type { PrintParams } from '../../engine/types';
import { CadCanvas, PartMesh, PresetButtons, prefersReducedMotion, type ViewPreset } from '../cad/viewer';
import { downloadPartStl, type PartsOut } from '../cad/api';
import { KINDS, defaultSettings, fmtTime, layerCsv, type EstimateOut, type OverhangReport, type SliceOut, type SliceSettings } from './api';
import { LayerCanvas, useTrace } from './LayerCanvas';
import { ToolpathStack, buildToolpaths } from './Toolpath3D';
import '../cad/cadprint.css';

const PRESETS: { name: string; params: Partial<PrintParams>; settings: Partial<SliceSettings> }[] = [
  { name: 'Hobby i3-class (Iris default)', params: { layer_height_mm: 0.2, perimeters: 3, infill_pct: 25, print_speed_mm_s: 60, travel_speed_mm_s: 150 }, settings: { accel_mm_s2: 2000, corner_speed_mm_s: 5, min_layer_time_s: 6 } },
  { name: 'Fast CoreXY-class', params: { layer_height_mm: 0.2, perimeters: 3, infill_pct: 20, print_speed_mm_s: 200, travel_speed_mm_s: 400 }, settings: { accel_mm_s2: 10000, corner_speed_mm_s: 8, min_layer_time_s: 5 } },
  { name: 'Draft', params: { layer_height_mm: 0.28, perimeters: 2, infill_pct: 15, print_speed_mm_s: 80, travel_speed_mm_s: 180 }, settings: { accel_mm_s2: 3000 } },
  { name: 'Fine', params: { layer_height_mm: 0.12, perimeters: 3, infill_pct: 25, print_speed_mm_s: 50, travel_speed_mm_s: 150 }, settings: { accel_mm_s2: 2000 } },
];

export default function Lab() {
  const project = useProject((s) => s.project);
  const patch = useProject((s) => s.patchDesign);
  const pp = project.design.print;
  const [settings, setSettings] = useState<SliceSettings>(defaultSettings);
  const [part, setPart] = useState('temple_r');
  const [sliceMs, setSliceMs] = useState(0);
  const [result, setResult] = useState<SliceOut | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sliceKey, setSliceKey] = useState('');
  const hasEnclosure = !!project.design.extra?.enclosure;
  const parts = useCompute<PartsOut>('cad.parts', { project, orientation: 'print', display: false });
  const options = useMemo(() => {
    const o = (parts.data?.parts ?? []).map((p) => ({ value: p.name, label: `${p.label} (${p.name})` }));
    if (hasEnclosure) o.push({ value: 'enclosure_body', label: 'Enclosure body' }, { value: 'enclosure_lid', label: 'Enclosure lid' });
    return o;
  }, [parts.data, hasEnclosure]);
  const key = JSON.stringify([part, pp, settings, project.design.frame, project.design.optics, project.design.extra?.enclosure ?? null]);
  const stale = result !== null && key !== sliceKey;

  const slice = () => {
    setBusy(true);
    setError(null);
    const k = key;
    setTimeout(() => {
      try {
        const t0 = performance.now();
        const r = call<SliceOut>('print.slice', { project, part, params: pp, settings });
        setSliceMs(performance.now() - t0);
        setResult(r);
        setSliceKey(k);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setBusy(false);
      }
    }, 30);
  };
  // Slice on first load and whenever the part changes.
  const first = useRef(true);
  useEffect(() => {
    if (!parts.data) return;
    if (first.current || result?.part !== part) {
      first.current = false;
      slice();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [part, !!parts.data]);

  const [tab, setTab] = useState<'layers' | 'stats' | 'overhang' | 'gcode' | 'frame'>('layers');
  return (
    <LabPage
      kicker="Optics & Mechanics · Lab"
      title="Print Lab"
      subtitle={
        <>
          A planar FDM slicer in WebAssembly: it sections each part into layers, offsets perimeters, detects top and bottom
          skins, hatches infill, orders the moves and writes generic Marlin/Klipper G-code — with time, filament and cost
          estimates for the whole Iris frame set.
        </>
      }
      metricPrefixes={['print.']}
    >
      <div className="cp-designer">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card
            title="Slice a part"
            subtitle={result ? `${result.part}: ${result.stats.layers} layers · ${result.gcode_lines.toLocaleString()} G-code lines · sliced in ${sliceMs.toFixed(0)} ms` : 'choose a part'}
            actions={
              <div className="row">
                <Select value={part} options={options.length ? options : [{ value: part, label: part }]} onChange={setPart} />
                <button className="btn primary" onClick={slice} disabled={busy}>
                  {busy ? <span className="cp-spin" /> : '▶'} {busy ? 'Slicing…' : stale ? 'Re-slice' : 'Slice'}
                </button>
              </div>
            }
          >
            <ErrorBox error={error} />
            {stale && !busy && (
              <div className="small" style={{ marginBottom: 8 }}>
                <Chip tone="warn">! settings changed</Chip> the preview shows the previous slice — press Re-slice.
              </div>
            )}
            {result && <StatTiles r={result} />}
          </Card>
          <Tabs
            tabs={[
              { id: 'layers', label: 'Layers & toolpaths' },
              { id: 'stats', label: 'Time & material' },
              { id: 'overhang', label: 'Overhangs' },
              { id: 'gcode', label: 'G-code' },
              { id: 'frame', label: 'Whole frame set' },
            ]}
            value={tab}
            onChange={setTab}
          />
          {tab === 'layers' && result && <LayersView r={result} />}
          {tab === 'stats' && result && <StatsView r={result} />}
          {tab === 'overhang' && <OverhangView part={part} />}
          {tab === 'gcode' && result && <GcodeView r={result} />}
          {tab === 'frame' && <FrameSetView settings={settings} />}
          <Teaching />
        </div>
        <div className="cp-sticky">
          <SettingsPanel pp={pp} patch={(v) => patch('print', v)} settings={settings} setSettings={setSettings} />
        </div>
      </div>
    </LabPage>
  );
}

function StatTiles({ r }: { r: SliceOut }) {
  const s = r.stats;
  return (
    <div className="cp-stats">
      <Stat label="Print time" value={fmtTime(s.time_s)} note={s.cooling_s > 1 ? `incl. ${fmtTime(s.cooling_s)} cooling slow-down` : 'trapezoidal motion model'} />
      <Stat label="Filament" value={s.filament_m.toFixed(2)} unit="m" note={`${s.filament_g.toFixed(2)} g · ${s.filament_cm3.toFixed(2)} cm³`} />
      <Stat label="Cost" value={`$${s.cost_usd.toFixed(2)}`} note={`at $${r.settings.cost_per_kg}/kg`} />
      <Stat label="Layers" value={s.layers} note={`${r.params.layer_height_mm} mm · top ${s.max_z_mm.toFixed(1)} mm`} />
      <Stat label="Fill ratio" value={`${(s.fill_ratio * 100).toFixed(0)}%`} note="extruded ÷ solid volume" />
    </div>
  );
}

// ------------------------------------------------------------------ layers

function LayersView({ r }: { r: SliceOut }) {
  const n = r.layers.length;
  const [layer, setLayer] = useState(Math.max(0, Math.min(n - 1, Math.floor(n * 0.3))));
  const [playing, setPlaying] = useState(false);
  const [travel, setTravel] = useState(false);
  const [ghost, setGhost] = useState(true);
  const [preset, setPreset] = useState<ViewPreset>('iso');
  const [nonce, setNonce] = useState(0);
  const [trace, tracing, startTrace, stopTrace] = useTrace(4);
  const geoms = useMemo(() => buildToolpaths(r.layers), [r]);
  useEffect(() => setLayer((l) => Math.min(l, n - 1)), [n]);
  useEffect(() => {
    if (!playing) return;
    if (prefersReducedMotion()) {
      setLayer(n - 1);
      setPlaying(false);
      return;
    }
    const id = setInterval(() => {
      setLayer((l) => {
        if (l >= n - 1) {
          setPlaying(false);
          return l;
        }
        return l + 1;
      });
    }, Math.max(40, 2400 / n));
    return () => clearInterval(id);
  }, [playing, n]);
  const L = r.layers[layer] ?? null;
  const ls = r.per_layer[layer];
  const bounds = { min: r.bbox[0], max: r.bbox[1] } as { min: [number, number, number]; max: [number, number, number] };
  return (
    <div className="stack">
      <div className="stack">
        <Card
          title={`Layer ${layer + 1} of ${n}`}
          subtitle={L ? `z = ${L.z.toFixed(2)} mm · ${ls ? fmtTime(ls.time_s) : ''}${ls?.slowed ? ' (slowed for cooling)' : ''}` : ''}
          actions={
            <div className="row">
              <button className="btn small" onClick={tracing ? stopTrace : startTrace}>{tracing ? '■ Stop' : '▶ Trace layer'}</button>
            </div>
          }
        >
          <LayerCanvas layer={L} prev={ghost && layer > 0 ? r.layers[layer - 1] : null} bbox={r.bbox} lineWidth={r.params.line_width_mm} showTravel={travel} progress={trace} height={360} />
          <div style={{ marginTop: 10 }}>
            <Slider label="Layer" value={layer} min={0} max={Math.max(1, n - 1)} step={1} onChange={(v) => { setPlaying(false); setLayer(Math.round(v)); }} format={(v) => `${Math.round(v) + 1} / ${n}  (z ${r.layers[Math.round(v)]?.z.toFixed(2) ?? ''} mm)`} />
          </div>
          <div className="cp-legend" style={{ marginTop: 10 }}>
            {KINDS.map((k) => (
              <span key={k.code}>
                {k.code === 0 ? <span className="cp-legend-dash" /> : <span className="cp-legend-dot" style={{ background: `var(${k.color})`, height: 5 }} />}
                {k.label}
              </span>
            ))}
            <span><span className="cp-legend-dash" style={{ borderTopColor: 'var(--bad-text)' }} /> travel with retraction</span>
          </div>
        </Card>
        <Card
          title="Toolpath stack"
          subtitle="Every extrusion move up to the selected layer"
          actions={
            <button className="btn small" onClick={() => { if (layer >= n - 1) setLayer(0); setPlaying((p) => !p); }}>
              {playing ? '❚❚ Pause' : '▶ Build up'}
            </button>
          }
        >
          <CadCanvas
            bounds={bounds}
            preset={preset}
            nonce={nonce}
            height={400}
            shadows={false}
            fit={0.62}
            overlay={
              <div className="cp-overlay">
                <PresetButtons value={preset} onChange={(v) => { setPreset(v); setNonce((x) => x + 1); }} />
              </div>
            }
          >
            <ToolpathStack geoms={geoms} upto={layer} showTravel={travel} current={L} />
            <PartMesh mesh={r.mesh} color="#8b9db5" opacity={0.07} />
          </CadCanvas>
        </Card>
      </div>
      <Card title="This layer">
        <div className="row">
          <Toggle label="Show travel moves" value={travel} onChange={setTravel} />
          <Toggle label="Ghost previous layer" value={ghost} onChange={setGhost} />
        </div>
        {ls && (
          <div className="cp-legend">
            {KINDS.map((k) => (
              <span key={k.code}>
                <span className="cp-swatch" style={{ background: `var(${k.color})` }} /> {k.short}: {ls.length_mm[k.code].toFixed(0)} mm · {ls.time_by_kind_s[k.code].toFixed(1)} s
              </span>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}

// ------------------------------------------------------------------ stats

function StatsView({ r }: { r: SliceOut }) {
  const s = r.stats;
  const z = r.per_layer.map((l) => l.z);
  const total = s.time_by_kind_s.reduce((a, b) => a + b, 0) || 1;
  let cum = 0;
  const cumulative = r.per_layer.map((l) => (cum += l.time_s) / 60);
  return (
    <div className="stack">
      <div className="grid grid-2">
        <Card title="Time per layer" subtitle="Small layers are slowed to the minimum layer time so the plastic can cool">
          <Plot
            height={240}
            xLabel="z (mm)"
            yLabel="s"
            yZero
            series={[
              { name: 'layer time', x: z, y: r.per_layer.map((l) => l.time_s), step: true, area: true },
              { name: 'motion only', x: z, y: r.per_layer.map((l) => l.raw_time_s), step: true, dash: '4 3' },
            ]}
            hLines={[{ value: r.settings.min_layer_time_s, label: 'min layer time' }]}
          />
        </Card>
        <Card title="Cumulative time" subtitle={`${fmtTime(s.time_s)} total`}>
          <Plot height={240} xLabel="z (mm)" yLabel="min" yZero series={[{ name: 'elapsed', x: z, y: cumulative, area: true }]} />
        </Card>
      </div>
      <Card title="Where the time goes" subtitle="Motion time by move type (excluding retractions, layer changes and cooling waits)">
        <div className="cp-stackbar">
          {KINDS.map((k) => (
            <div key={k.code} title={`${k.label}: ${fmtTime(s.time_by_kind_s[k.code])}`} style={{ width: `${(s.time_by_kind_s[k.code] / total) * 100}%`, background: `var(${k.color})`, opacity: k.code === 0 ? 0.5 : 1 }} />
          ))}
        </div>
        <div className="table-wrap" style={{ marginTop: 12 }}>
          <table className="table">
            <thead>
              <tr><th>Move type</th><th>Length</th><th>Time</th><th>Share</th></tr>
            </thead>
            <tbody>
              {KINDS.map((k) => (
                <tr key={k.code}>
                  <td><span className="cp-swatch" style={{ background: `var(${k.color})` }} /> {k.label}</td>
                  <td className="mono">{(s.length_by_kind_mm[k.code] / 1000).toFixed(2)} m</td>
                  <td className="mono">{fmtTime(s.time_by_kind_s[k.code])}</td>
                  <td className="mono">{((s.time_by_kind_s[k.code] / total) * 100).toFixed(1)}%</td>
                </tr>
              ))}
              <tr><td>Retractions</td><td className="mono">{s.retracts}×</td><td className="mono">{fmtTime(s.retracts * 2 * r.settings.retract_mm / r.settings.retract_speed_mm_s)}</td><td /></tr>
              <tr><td>Cooling slow-down</td><td className="mono">{s.slowed_layers} layers</td><td className="mono">{fmtTime(s.cooling_s)}</td><td /></tr>
            </tbody>
          </table>
        </div>
        <div className="row" style={{ marginTop: 12 }}>
          <DownloadButton filename={`${r.part}_layers.csv`} label="Per-layer stats (.csv)" content={() => layerCsv(r.per_layer)} />
        </div>
      </Card>
    </div>
  );
}

// ------------------------------------------------------------------ overhangs

function OverhangView({ part }: { part: string }) {
  const project = useProject((s) => s.project);
  const [thr, setThr] = useState(45);
  const [orientation, setOrientation] = useState<'print' | 'assembly'>('print');
  // Overhangs face down: look from below by default.
  const [preset, setPreset] = useState<ViewPreset>('below');
  const [nonce, setNonce] = useState(0);
  const isFrame = !part.startsWith('enclosure');
  const { data, error } = useCompute<OverhangReport>('print.overhangs', { project, part, threshold_deg: thr, orientation: isFrame ? orientation : 'print' });
  const bounds = useMemo(() => {
    if (!data) return null;
    const p = data.positions;
    const min: [number, number, number] = [Infinity, Infinity, Infinity];
    const max: [number, number, number] = [-Infinity, -Infinity, -Infinity];
    for (let i = 0; i < p.length; i += 3) for (let k = 0; k < 3; k++) { min[k] = Math.min(min[k], p[i + k]); max[k] = Math.max(max[k], p[i + k]); }
    return { min, max };
    // refit on part/orientation change only
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [part, orientation, !!data]);
  const mesh = useMemo(() => (data ? { positions: data.positions, colors: data.colors } : null), [data]);
  const bins = data?.histogram_mm2 ?? [];
  return (
    <div className="grid grid-2">
      <Card
        title="Overhang heat map"
        subtitle={isFrame ? 'Compare the print orientation with the as-worn orientation' : 'Print orientation'}
        actions={
          isFrame && (
            <div className="cp-seg">
              {(['print', 'assembly'] as const).map((o) => (
                <button key={o} className={`cp-seg-btn ${orientation === o ? 'active' : ''}`} onClick={() => { setOrientation(o); setNonce((x) => x + 1); }}>
                  {o === 'print' ? 'print (flat)' : 'as worn'}
                </button>
              ))}
            </div>
          )
        }
      >
        <ErrorBox error={error} />
        {mesh && bounds && (
          <CadCanvas
            bounds={bounds}
            preset={preset}
            nonce={nonce}
            height={400}
            fit={0.95}
            grid={false}
            shadows={false}
            overlay={<div className="cp-overlay"><PresetButtons below value={preset} onChange={(v) => { setPreset(v); setNonce((x) => x + 1); }} /></div>}
          >
            <PartMesh mesh={mesh} vertexColors />
          </CadCanvas>
        )}
        <div className="cp-legend" style={{ marginTop: 8 }}>
          <span><span className="cp-swatch" style={{ background: '#2eae6b' }} /> on the bed</span>
          <span><span className="cp-swatch" style={{ background: '#99a8c2' }} /> self-supporting</span>
          <span><span className="cp-swatch" style={{ background: '#f5bd38' }} /> within 15° of the limit</span>
          <span><span className="cp-swatch" style={{ background: '#eb6b47' }} /> needs support</span>
        </div>
      </Card>
      <div className="stack">
        <Card title="Support check">
          <Slider label="Overhang limit (from vertical)" value={thr} min={20} max={80} step={1} unit="°" onChange={setThr} />
          {data && (
            <>
              <div className="cp-stats" style={{ marginBottom: 10 }}>
                <Stat label="Overhang area" value={data.overhang_area_mm2.toFixed(1)} unit="mm²" tone={data.needs_support ? 'fail' : 'pass'} note={`${data.overhang_faces} faces`} />
                <Stat label="Bed contact" value={data.bed_area_mm2.toFixed(0)} unit="mm²" note="first-layer adhesion area" />
                <Stat label="Steepest" value={data.max_angle_deg.toFixed(0)} unit="°" note="from vertical" />
              </div>
              <div className="row small">
                <StatusChip status={data.needs_support ? 'fail' : 'pass'} /> {data.suggestion}
              </div>
            </>
          )}
        </Card>
        <Card title="Down-facing area by angle" subtitle="mm² of downward surface per 10° band (excluding the bed)">
          <Plot
            height={200}
            xLabel="angle from vertical (°)"
            yLabel="mm²"
            yZero
            series={[{ name: 'area', x: bins.map((_, i) => i * 10 + 5), y: bins, step: true, area: true }]}
            vLines={[{ value: thr, label: 'limit' }]}
          />
        </Card>
        <Explainer title="The 45° rule" refs={[{ label: 'Gibson, Rosen & Stucker, Additive Manufacturing Technologies (2015), §5.5' }, { label: 'ISO/ASTM 52900 — AM terminology' }]}>
          Each new bead can hang over the one below by roughly half its width; with a 0.45 mm line on 0.2 mm layers that is
          about 45° from vertical. Steeper down-facing surfaces sag unless supported (or bridged if short). That is why every
          Iris part is generated to lie flat on its largest face — in print orientation none of them needs support, while as
          worn the temples and pods would.
        </Explainer>
      </div>
    </div>
  );
}

// ------------------------------------------------------------------ G-code

function GcodeView({ r }: { r: SliceOut }) {
  const lines = r.gcode.split('\n');
  const head = lines.slice(0, 220).join('\n') + (lines.length > 220 ? `\n; … ${lines.length - 220} more lines — download for the full file` : '');
  return (
    <div className="grid grid-2">
      <Card title="G-code" subtitle={`${r.gcode_lines.toLocaleString()} lines · ${(r.gcode.length / 1024).toFixed(0)} kB · generic Marlin/Klipper flavour`}>
        <div className="cp-gcode-head">
          <DownloadButton filename={`iris_${r.part}.gcode`} label={`iris_${r.part}.gcode`} content={() => r.gcode} primary />
          <DownloadButton filename={`${r.part}_layers.csv`} label="stats .csv" content={() => layerCsv(r.per_layer)} small />
        </div>
        <Code code={head} maxHeight={480} />
      </Card>
      <div className="stack">
        <Explainer title="Reading the G-code" refs={[{ label: 'RepRap G-code reference', href: 'https://reprap.org/wiki/G-code' }, { label: 'Marlin G-code index', href: 'https://marlinfw.org/meta/gcode/' }]}>
          <div className="cp-kv" style={{ gridTemplateColumns: 'auto 1fr' }}>
            <dt className="mono">M140 / M190</dt><dd style={{ textAlign: 'left', fontFamily: 'var(--font)' }}>set / wait for bed temperature</dd>
            <dt className="mono">M104 / M109</dt><dd style={{ textAlign: 'left', fontFamily: 'var(--font)' }}>set / wait for hot-end temperature</dd>
            <dt className="mono">G28</dt><dd style={{ textAlign: 'left', fontFamily: 'var(--font)' }}>home all axes</dd>
            <dt className="mono">G90 · M82/M83</dt><dd style={{ textAlign: 'left', fontFamily: 'var(--font)' }}>absolute XYZ · absolute/relative extrusion</dd>
            <dt className="mono">G0 X Y F</dt><dd style={{ textAlign: 'left', fontFamily: 'var(--font)' }}>travel (F in mm/min)</dd>
            <dt className="mono">G1 X Y E</dt><dd style={{ textAlign: 'left', fontFamily: 'var(--font)' }}>print move; E is the filament axis position</dd>
            <dt className="mono">G1 E−0.8</dt><dd style={{ textAlign: 'left', fontFamily: 'var(--font)' }}>retract before a long travel (then restore)</dd>
            <dt className="mono">M106 S</dt><dd style={{ textAlign: 'left', fontFamily: 'var(--font)' }}>part-cooling fan (0–255), on after layer 1</dd>
          </div>
        </Explainer>
        <div className="callout" style={{ borderLeftColor: 'var(--warn)' }}>
          <h4>Generic G-code — check before printing</h4>
          Bed centre is X{r.settings.bed_center[0]} Y{r.settings.bed_center[1]}, the purge line runs along Y = 5. Printers
          differ in start code, bed size and firmware features. <b>Bambu Lab printers</b> need G-code from Bambu Studio or
          OrcaSlicer (their start sequence, calibration and AMS commands) — use the STL/3MF export and the handoff steps below.
        </div>
      </div>
    </div>
  );
}

// ------------------------------------------------------------------ whole frame

function FrameSetView({ settings }: { settings: SliceSettings }) {
  const project = useProject((s) => s.project);
  const fast = useCompute<EstimateOut>('print.estimate', { project, settings });
  const [full, setFull] = useState<EstimateOut | null>(null);
  const [busy, setBusy] = useState(false);
  const req = project.requirements.find((r) => r.verification.metric === 'print.total_time_h');
  const target = req?.verification.target ?? 8;
  useEffect(() => setFull(null), [project.design.frame, project.design.print, settings]);
  const runFull = () => {
    setBusy(true);
    setTimeout(() => {
      try {
        setFull(call<EstimateOut>('print.estimate', { project, settings, method: 'full' }));
      } finally {
        setBusy(false);
      }
    }, 30);
  };
  const f = fast.data;
  return (
    <div className="stack">
      <Card
        title="Full Iris frame set"
        subtitle={f ? `Fast estimate (${fast.ms.toFixed(1)} ms): ${f.method}` : '…'}
        actions={
          <button className="btn" onClick={runFull} disabled={busy}>
            {busy ? <span className="cp-spin" /> : '▶'} Verify with a full slice of every part
          </button>
        }
      >
        {f && (
          <>
            <div className="cp-stats" style={{ marginBottom: 12 }}>
              <Stat label="Total print time" value={fmtTime(f.total_time_h * 3600)} tone={f.total_time_h <= target ? 'pass' : 'fail'} note={`${f.total_time_h <= target ? '✓' : '✕'} ≤ ${target} h (${req?.id ?? 'R-021'})`} />
              <Stat label="Filament" value={f.total_filament_g.toFixed(1)} unit="g" note={`${f.total_filament_m.toFixed(1)} m`} />
              <Stat label="Material cost" value={`$${f.total_cost_usd.toFixed(2)}`} note={`at $${settings.cost_per_kg}/kg`} />
              {full && <Stat label="Full slice" value={fmtTime(full.total_time_h * 3600)} note={`${full.total_filament_g.toFixed(1)} g · Δ ${(((f.total_time_h - full.total_time_h) / full.total_time_h) * 100).toFixed(1)}% time`} />}
            </div>
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr>
                    <th>Part</th><th>Time (fast)</th><th>Filament g</th><th>Layers</th>
                    {full && <th>Time (full)</th>}
                    {full && <th>Δ time</th>}
                    {full && <th>Δ mass</th>}
                    <th>STL</th>
                  </tr>
                </thead>
                <tbody>
                  {f.parts.map((p, i) => {
                    const q = full?.parts[i];
                    return (
                      <tr key={p.name}>
                        <td className="mono">{p.name}</td>
                        <td className="mono">{fmtTime(p.stats.time_s)}</td>
                        <td className="mono">{p.stats.filament_g.toFixed(2)}</td>
                        <td className="mono">{p.stats.layers}</td>
                        {q && <td className="mono">{fmtTime(q.stats.time_s)}</td>}
                        {q && <td className="mono">{(((p.stats.time_s - q.stats.time_s) / q.stats.time_s) * 100).toFixed(1)}%</td>}
                        {q && <td className="mono">{(((p.stats.filament_g - q.stats.filament_g) / q.stats.filament_g) * 100).toFixed(1)}%</td>}
                        <td><button className="btn small" onClick={() => downloadPartStl(project, p.name, 'print')}>↓ stl</button></td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </>
        )}
      </Card>
      <Explainer title="Why two estimates?" refs={[{ label: 'L. A. Santaló, Integral Geometry and Geometric Probability (1976)' }]}>
        The requirement check runs on every edit anywhere in Protoforge, so it uses a fast estimator: each layer is only
        sectioned, and toolpath lengths follow from Steiner's parallel-curve formulas — an inward offset by d leaves area
        A − L·d + πχd² and boundary length L − 2πχd (χ = outer loops − holes) — while Cauchy–Crofton counts infill lines
        (a boundary of length L meets lines spaced s apart about 2L/(πs) times). The full slice generates every move; for
        the Iris parts the two agree within about 10 % per part and a few percent in total.
      </Explainer>
    </div>
  );
}

// ------------------------------------------------------------------ settings

function SettingsPanel(props: { pp: PrintParams; patch: (p: Partial<PrintParams>) => void; settings: SliceSettings; setSettings: (f: (s: SliceSettings) => SliceSettings) => void }) {
  const { pp, patch, settings, setSettings } = props;
  const st = (k: keyof SliceSettings, v: number | boolean) => setSettings((s) => ({ ...s, [k]: v }));
  return (
    <Card title="Print settings" subtitle="Stored in the project (design.print)">
      <div className="cp-group">
        <div className="cp-group-title">Presets</div>
        <div className="row">
          {PRESETS.map((p) => (
            <button key={p.name} className="btn small" onClick={() => { patch(p.params); setSettings((s) => ({ ...s, ...p.settings })); }}>
              {p.name}
            </button>
          ))}
        </div>
      </div>
      <div className="cp-group">
        <div className="cp-group-title">Layers & walls</div>
        <Slider label="Layer height" value={pp.layer_height_mm} min={0.08} max={0.32} step={0.02} unit="mm" onChange={(v) => patch({ layer_height_mm: v })} />
        <Slider label="First layer" value={settings.first_layer_mm} min={0.1} max={0.4} step={0.02} unit="mm" onChange={(v) => st('first_layer_mm', v)} />
        <Slider label="Line width" value={pp.line_width_mm} min={0.3} max={0.8} step={0.01} unit="mm" onChange={(v) => patch({ line_width_mm: v })} />
        <Slider label="Perimeters" value={pp.perimeters} min={1} max={6} step={1} onChange={(v) => patch({ perimeters: Math.round(v) })} format={(v) => String(Math.round(v))} />
        <Slider label="Top / bottom layers" value={settings.top_layers} min={0} max={8} step={1} onChange={(v) => setSettings((s) => ({ ...s, top_layers: Math.round(v), bottom_layers: Math.round(v) }))} format={(v) => String(Math.round(v))} />
        <Slider label="Infill" value={pp.infill_pct} min={0} max={100} step={1} unit="%" onChange={(v) => patch({ infill_pct: v })} />
      </div>
      <div className="cp-group">
        <div className="cp-group-title">Speed & motion</div>
        <Slider label="Print speed" value={pp.print_speed_mm_s} min={20} max={300} step={5} unit="mm/s" onChange={(v) => patch({ print_speed_mm_s: v })} />
        <Slider label="Outer wall speed" value={settings.outer_wall_speed_pct} min={30} max={100} step={5} unit="%" onChange={(v) => st('outer_wall_speed_pct', v)} />
        <Slider label="Travel speed" value={pp.travel_speed_mm_s} min={60} max={600} step={10} unit="mm/s" onChange={(v) => patch({ travel_speed_mm_s: v })} />
        <Slider label="Acceleration" value={settings.accel_mm_s2} min={300} max={20000} step={100} unit="mm/s²" log onChange={(v) => st('accel_mm_s2', Math.round(v / 100) * 100)} />
        <Slider label="Min layer time" value={settings.min_layer_time_s} min={0} max={20} step={0.5} unit="s" onChange={(v) => st('min_layer_time_s', v)} />
      </div>
      <div className="cp-group">
        <div className="cp-group-title">Material & temperatures</div>
        <Select label="Nozzle" value={String(pp.nozzle_mm)} options={['0.2', '0.4', '0.6', '0.8'].map((v) => ({ value: v, label: `${v} mm` }))} onChange={(v) => patch({ nozzle_mm: Number(v), line_width_mm: Math.round(Number(v) * 1.125 * 100) / 100 })} />
        <Select label="Filament diameter" value={String(pp.filament_diameter_mm)} options={[{ value: '1.75', label: '1.75 mm' }, { value: '2.85', label: '2.85 mm' }]} onChange={(v) => patch({ filament_diameter_mm: Number(v) })} />
        <Slider label="Nozzle temperature" value={pp.nozzle_temp_c} min={180} max={300} step={5} unit="°C" onChange={(v) => patch({ nozzle_temp_c: v })} />
        <Slider label="Bed temperature" value={pp.bed_temp_c} min={0} max={110} step={5} unit="°C" onChange={(v) => patch({ bed_temp_c: v })} />
        <Slider label="Part fan" value={settings.fan_pct} min={0} max={100} step={5} unit="%" onChange={(v) => st('fan_pct', v)} />
        <Slider label="Retraction" value={settings.retract_mm} min={0} max={6} step={0.1} unit="mm" onChange={(v) => st('retract_mm', v)} />
        <Slider label="Filament price" value={settings.cost_per_kg} min={10} max={80} step={1} unit="$/kg" onChange={(v) => st('cost_per_kg', v)} />
      </div>
      <div className="cp-group">
        <div className="cp-group-title">G-code</div>
        <Toggle label="Relative extrusion (M83)" value={settings.relative_e} onChange={(v) => st('relative_e', v)} />
        <Toggle label="Skirt" value={settings.skirt} onChange={(v) => st('skirt', v)} />
      </div>
    </Card>
  );
}

// ------------------------------------------------------------------ teaching

function Teaching() {
  const [t, setT] = useState<'slice' | 'perim' | 'infill' | 'e' | 'time' | 'handoff'>('slice');
  return (
    <Card title="How a slicer works">
      <Tabs
        tabs={[
          { id: 'slice', label: 'Slicing' },
          { id: 'perim', label: 'Perimeters' },
          { id: 'infill', label: 'Infill & strength' },
          { id: 'e', label: 'Extrusion maths' },
          { id: 'time', label: 'Time estimate' },
          { id: 'handoff', label: 'Bambu / Orca handoff' },
        ]}
        value={t}
        onChange={setT}
      />
      {t === 'slice' && (
        <Explainer title="From triangles to contours" refs={[{ label: 'Gibson, Rosen & Stucker, Additive Manufacturing Technologies, 2nd ed. (2015), ch. 5' }, { label: 'de Berg et al., Computational Geometry (2008)' }]}>
          For every layer the plane z = layer mid-height cuts each triangle that straddles it in one segment. The segment is
          oriented with the solid on its left (direction ∝ ẑ × n from the face normal), so outer contours come out
          counter-clockwise and holes clockwise. Neighbouring triangles compute the crossing on their shared edge from the
          same two vertices, so endpoints match bit for bit and a hash map chains them into closed loops. Loops are cleaned
          with the winding-number rule (which also merges overlapping shells) and grouped into islands by containment.
        </Explainer>
      )}
      {t === 'perim' && (
        <Explainer title="Perimeters are polygon offsets" refs={[{ label: 'X. Chen & S. McMains, "Polygon Offsetting by Computing Winding Numbers", ASME IDETC 2005' }]}>
          Perimeter k is the island boundary moved inwards by w/2 + k·s, where the bead spacing s = w − h(1 − π/4) makes
          neighbouring rounded beads tile the plane. Every edge is shifted, corners are mitred (or joined with arcs), and where
          the shifted curve folds over itself — thin walls, narrow slots — the positive-winding rule throws the invalid parts
          away. A 1.2 mm pod wall only fits one bead from each side at 0.45 mm; the third perimeter simply vanishes.
          <Formula>{'offset curve: A(d) = A − L·d + π·χ·d²   (Steiner, valid until a feature vanishes)'}</Formula>
        </Explainer>
      )}
      {t === 'infill' && (
        <Explainer title="Infill, skins and strength" refs={[{ label: 'Gibson, Rosen & Stucker (2015), §5.6 — design and part strength' }]}>
          Inside the perimeters the slicer lays parallel lines spaced s/ρ apart (ρ = infill fraction), rotating 45°/135° every
          layer so consecutive layers cross like a grid. Near top and bottom surfaces the part must be closed, so material that
          is not covered within {'N'} layers above or below (solid = infill ∖ (outline[i−N] ∩ outline[i+N])) is filled at 100 %.
          Strength of thin shells like the Iris temples comes mostly from the perimeters: more walls beat more infill.
        </Explainer>
      )}
      {t === 'e' && (
        <Explainer title="How much filament to push" refs={[{ label: 'Slic3r manual, "Flow math"' }]}>
          A bead of width w and height h has the cross-section of a rectangle with semicircular sides:
          <Formula>{'A_bead = (w − h)·h + π(h/2)²        E = A_bead · L / (π d² / 4)'}</Formula>
          where d is the filament diameter and L the move length. With w = 0.45, h = 0.2 and d = 1.75 mm, every millimetre of
          bead needs 0.0339 mm of filament. The simpler w·h·L/(πd²/4) over-extrudes by about 10 %.
        </Explainer>
      )}
      {t === 'time' && (
        <Explainer title="Estimating print time" refs={[{ label: 'Klipper documentation — Kinematics (trapezoid generator)' }, { label: 'Marlin — junction deviation' }]}>
          Printers cannot jump to full speed: each move follows a trapezoidal velocity profile, accelerating at a from the
          corner speed v_j to the feed rate v.
          <Formula>{'t = L/v + (v − v_j)²/(a·v)   if L ≥ (v² − v_j²)/a,   else  t = 2(√(v_j² + a·L) − v_j)/a'}</Formula>
          Polylines are split into runs at corners sharper than 30°. Retractions, layer changes and the minimum layer time
          (cooling) are added. Short infill segments on small parts are acceleration-limited, which is why doubling the
          feed rate rarely halves the time.
        </Explainer>
      )}
      {t === 'handoff' && (
        <div className="grid grid-2">
          <div>
            <h4 style={{ marginBottom: 6 }}>Bambu Studio / OrcaSlicer</h4>
            <ol className="cp-steps">
              <li>In the <Link to="/cad">CAD Studio</Link> → Mass &amp; export, download the <b>print plate (.3mf)</b> or the STL zip.</li>
              <li>Open it in Bambu Studio or OrcaSlicer; select your printer and a PETG profile (e.g. the generic PETG preset) — PETG likes 230–250 °C and a 70–80 °C textured plate.</li>
              <li>Match this lab: 0.2 mm layers, 3 walls, 25 % infill (grid or gyroid), 4 top / 4 bottom layers; supports off — every part is support-free as oriented.</li>
              <li>Slice, compare the time and filament with the Whole frame set tab, then print via LAN/cloud or the SD card.</li>
            </ol>
          </div>
          <div>
            <h4 style={{ marginBottom: 6 }}>Other printers</h4>
            <ol className="cp-steps">
              <li>PrusaSlicer, Cura (CuraEngine) or OrcaSlicer with your printer profile will add proper start code, bed mesh and pressure advance.</li>
              <li>This lab's G-code is a teaching reference: it is valid Marlin/Klipper syntax centred at X128 Y128 — check bed size and start code before running it.</li>
              <li>For production the frame is injection-molded: see the Molding Lab (draft ≥ 1° is already built in).</li>
            </ol>
          </div>
        </div>
      )}
    </Card>
  );
}
