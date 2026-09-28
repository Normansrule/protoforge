// Generic enclosure generator: box + lid with bosses, standoffs, cut-outs.
import { useMemo, useState } from 'react';
import JSZip from 'jszip';
import { Card, ErrorBox, Explainer, Formula, Slider, Stat, StatusChip, downloadBlob } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { call } from '../../engine/engine';
import { useProject } from '../../state/project';
import { CadCanvas, PartEdges, PartMesh, PresetButtons, useTheme, type ViewPreset } from './viewer';
import { base64ToBytes, fileToBlob, type Cutout, type EnclosureOut, type EnclosureParams, type FileOut } from './api';

function defaults(): EnclosureParams {
  return {
    inner_length_mm: 72, inner_width_mm: 52, inner_height_mm: 22, wall_mm: 2, floor_mm: 2, lid_mm: 2,
    corner_radius_mm: 5, fillet_mm: 1.5, draft_deg: 0, clearance_mm: 0.2, bosses: true, boss_d_mm: 6.5,
    boss_hole_mm: 2.5, screw_clear_mm: 3.4, lip_mm: 2,
    pcb: { enabled: true, length_mm: 60, width_mm: 40, thickness_mm: 1.6, hole_inset_mm: 3.5, standoff_h_mm: 4, standoff_d_mm: 5.5, standoff_hole_mm: 2.2 },
    cutouts: [
      { face: 'x-', offset_mm: 0, z_mm: 6, width_mm: 9.5, height_mm: 3.8, label: 'USB-C' },
      { face: 'y-', offset_mm: 18, z_mm: 10, width_mm: 3.2, height_mm: 3.2, label: 'LED' },
    ],
    density_g_cm3: 1.24,
  };
}

const PRESETS: { name: string; make: (pcbW: number, pcbH: number) => EnclosureParams }[] = [
  { name: 'Dev-board case', make: () => defaults() },
  {
    name: 'Iris PCB bench case',
    make: (w, h) => ({
      ...defaults(),
      inner_length_mm: Math.ceil(w + 8),
      inner_width_mm: Math.ceil(h + 14),
      inner_height_mm: 14,
      corner_radius_mm: 4,
      boss_d_mm: 6,
      pcb: { ...defaults().pcb, length_mm: w, width_mm: h, thickness_mm: 0.8, standoff_h_mm: 3, hole_inset_mm: 2.5, standoff_d_mm: 4.5, standoff_hole_mm: 1.8, enabled: false },
      cutouts: [{ face: 'x+', offset_mm: 0, z_mm: 3, width_mm: 9.5, height_mm: 3.8, label: 'USB-C' }],
    }),
  },
  {
    name: 'Sensor puck',
    make: () => ({
      ...defaults(),
      inner_length_mm: 40, inner_width_mm: 40, inner_height_mm: 12, corner_radius_mm: 12, fillet_mm: 1.8, boss_d_mm: 6,
      pcb: { ...defaults().pcb, length_mm: 30, width_mm: 30, standoff_h_mm: 3 },
      cutouts: [{ face: 'y+', offset_mm: 0, z_mm: 4, width_mm: 12, height_mm: 2, label: 'vent' }],
    }),
  },
];

export function EnclosureTab() {
  const project = useProject((s) => s.project);
  const update = useProject((s) => s.update);
  const stored = project.design.extra?.enclosure as EnclosureParams | undefined;
  const [p, setP] = useState<EnclosureParams>(() => ({ ...defaults(), ...(stored ?? {}) }));
  const [lift, setLift] = useState(0.6);
  const [xray, setXray] = useState(false);
  const [edges, setEdges] = useState(false);
  const [preset, setPreset] = useState<ViewPreset>('iso');
  const [nonce, setNonce] = useState(0);
  const { data, error, ms } = useCompute<EnclosureOut>('cad.enclosure', { params: p });
  const theme = useTheme();
  const set = <K extends keyof EnclosureParams>(k: K, v: EnclosureParams[K]) => setP((o) => ({ ...o, [k]: v }));
  const setPcb = (k: keyof EnclosureParams['pcb'], v: number | boolean) => setP((o) => ({ ...o, pcb: { ...o.pcb, [k]: v } }));
  const setCut = (i: number, c: Partial<Cutout>) => setP((o) => ({ ...o, cutouts: o.cutouts.map((x, j) => (j === i ? { ...x, ...c } : x)) }));
  const bounds = useMemo(() => {
    const L = p.inner_length_mm + 2 * p.wall_mm;
    const W = p.inner_width_mm + 2 * p.wall_mm;
    const H = p.floor_mm + p.inner_height_mm + p.lid_mm + 30;
    return { min: [-L / 2, -W / 2, 0] as [number, number, number], max: [L / 2, W / 2, H] as [number, number, number] };
    // Refit only when the preset/nonce changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [nonce]);
  const saved = JSON.stringify(stored ?? null) === JSON.stringify(p);
  const stl = (part: 'body' | 'lid') => call<FileOut>('cad.stl', { enclosure: p, part, orientation: 'print', binary: true });
  return (
    <div className="cp-designer">
      <div className="stack" style={{ minWidth: 0 }}>
        <Card
          title="Enclosure"
          subtitle={data ? `${data.outer_size_mm.map((v) => v.toFixed(1)).join(' × ')} mm outside · generated in ${ms.toFixed(1)} ms` : '…'}
          actions={
            <div className="row">
              {PRESETS.map((pr) => (
                <button key={pr.name} className="btn small" onClick={() => { setP(pr.make(project.design.pcb.width_mm, project.design.pcb.height_mm)); setNonce((n) => n + 1); }}>
                  {pr.name}
                </button>
              ))}
            </div>
          }
        >
          <ErrorBox error={error} />
          {data && (
            <CadCanvas
              bounds={bounds}
              preset={preset}
              nonce={nonce}
              height="min(56vh, 480px)"
              fit={1.0}
              overlay={
                <div className="cp-overlay">
                  <PresetButtons value={preset} onChange={(v) => { setPreset(v); setNonce((n) => n + 1); }} />
                </div>
              }
            >
              {data.parts.map((part) => (
                <group key={part.name} position={part.name === 'lid' ? [0, 0, lift * 30] : [0, 0, 0]}>
                  <PartMesh mesh={part.assembly} color={part.color} xray={xray} />
                  {edges && <PartEdges mesh={part.assembly} color={theme === 'light' ? '#1d2735' : '#dce6f5'} opacity={0.3} />}
                </group>
              ))}
              {p.pcb.enabled && (
                <mesh position={data.pcb_center}>
                  <boxGeometry args={data.pcb_size} />
                  <meshPhysicalMaterial color="#2fbf71" transparent opacity={0.55} roughness={0.3} />
                </mesh>
              )}
            </CadCanvas>
          )}
          <div className="cp-toolbar">
            <Slider label="Lift lid" value={lift} min={0} max={1} step={0.01} onChange={setLift} format={(v) => `${Math.round(v * 30)} mm`} />
            <label className="cp-check"><input type="checkbox" checked={xray} onChange={(e) => setXray(e.target.checked)} /> X-ray</label>
            <label className="cp-check"><input type="checkbox" checked={edges} onChange={(e) => setEdges(e.target.checked)} /> Edges (shows the stacked slabs)</label>
          </div>
        </Card>
        {data && (
          <div className="cp-stats">
            <Stat label="Body" value={data.parts[0].mass.mass_g.toFixed(1)} unit="g" note={<><StatusChip status={data.parts[0].watertight ? 'pass' : 'fail'} /> watertight</>} />
            <Stat label="Lid" value={data.parts[1].mass.mass_g.toFixed(1)} unit="g" note={<><StatusChip status={data.parts[1].watertight ? 'pass' : 'fail'} /> watertight</>} />
            <Stat label="Total (solid)" value={data.total_mass_g.toFixed(1)} unit="g" note={`PLA-like ${p.density_g_cm3} g/cm³`} />
            <Stat label="Interior" value={data.interior_mm.map((v) => v.toFixed(0)).join('×')} unit="mm" />
          </div>
        )}
        {data && data.warnings.length > 0 && (
          <div className="callout" style={{ borderLeftColor: 'var(--warn)' }}>
            {data.warnings.map((w) => <div key={w}>! {w}</div>)}
          </div>
        )}
        <Card title="Export" subtitle="Both parts in print orientation (body upright, lid face-down)">
          <div className="row">
            <button className="btn" onClick={() => { const f = stl('body'); downloadBlob('enclosure_body.stl', fileToBlob(f)); }}>↓ enclosure_body.stl</button>
            <button className="btn" onClick={() => { const f = stl('lid'); downloadBlob('enclosure_lid.stl', fileToBlob(f)); }}>↓ enclosure_lid.stl</button>
            <button
              className="btn"
              onClick={async () => {
                const zip = new JSZip();
                for (const part of ['body', 'lid'] as const) zip.file(`enclosure_${part}.stl`, base64ToBytes(stl(part).base64 ?? ''));
                zip.file('enclosure_params.json', JSON.stringify(p, null, 2));
                downloadBlob('enclosure.zip', await zip.generateAsync({ type: 'blob' }));
              }}
            >
              ↓ Both + parameters (.zip)
            </button>
            <button
              className="btn primary"
              disabled={saved}
              onClick={() => update((d) => { d.design.extra = { ...(d.design.extra ?? {}), enclosure: p as unknown as Record<string, unknown> }; })}
              title="Stores the parameters in the project so the Print Lab can slice the body and lid"
            >
              {saved ? '✓ In the Print Lab' : 'Send to Print Lab'}
            </button>
          </div>
        </Card>
        <Explainer title="Design for printing" refs={[{ label: 'I. Gibson, D. Rosen, B. Stucker, Additive Manufacturing Technologies, 2nd ed. (2015), ch. 17' }]}>
          Walls of at least two extrusion widths (≥ 1.2 mm; 2 mm is sturdy) print cleanly. M3 thread-forming screws want a
          ≈ 2.5 mm pilot hole in a ≥ 6 mm boss; the lid holes are 3.4 mm clearance. Cut-outs are rectangular windows cut
          with 2D booleans, so the wall above a window is a bridge — keep windows under ~20 mm wide or add a chamfer. The
          bottom edge fillet is approximated by stacking offset outlines along a quarter circle:
          <Formula>{'offset(z) = −r·(1 − sin θ),   z = r·(1 − cos θ),   θ ∈ [0, π/2]'}</Formula>
        </Explainer>
      </div>
      <div className="cp-sticky">
        <Card title="Parameters">
          <div className="cp-group">
            <div className="cp-group-title">Interior</div>
            <Slider label="Length" value={p.inner_length_mm} min={20} max={200} step={0.5} unit="mm" onChange={(v) => set('inner_length_mm', v)} />
            <Slider label="Width" value={p.inner_width_mm} min={15} max={160} step={0.5} unit="mm" onChange={(v) => set('inner_width_mm', v)} />
            <Slider label="Height" value={p.inner_height_mm} min={5} max={80} step={0.5} unit="mm" onChange={(v) => set('inner_height_mm', v)} />
          </div>
          <div className="cp-group">
            <div className="cp-group-title">Shell</div>
            <Slider label="Wall" value={p.wall_mm} min={0.8} max={5} step={0.1} unit="mm" onChange={(v) => set('wall_mm', v)} />
            <Slider label="Floor" value={p.floor_mm} min={0.8} max={5} step={0.1} unit="mm" onChange={(v) => set('floor_mm', v)} />
            <Slider label="Lid" value={p.lid_mm} min={0.8} max={5} step={0.1} unit="mm" onChange={(v) => set('lid_mm', v)} />
            <Slider label="Corner radius" value={p.corner_radius_mm} min={0} max={20} step={0.5} unit="mm" onChange={(v) => set('corner_radius_mm', v)} />
            <Slider label="Edge fillet" value={p.fillet_mm} min={0} max={4} step={0.1} unit="mm" onChange={(v) => set('fillet_mm', v)} />
            <Slider label="Draft" value={p.draft_deg} min={0} max={3} step={0.1} unit="°" onChange={(v) => set('draft_deg', v)} />
          </div>
          <div className="cp-group">
            <div className="cp-group-title">
              Screw bosses
              <label className="cp-check"><input type="checkbox" checked={p.bosses} onChange={(e) => set('bosses', e.target.checked)} /> on</label>
            </div>
            {p.bosses ? (
              <>
                <Slider label="Boss diameter" value={p.boss_d_mm} min={4} max={12} step={0.1} unit="mm" onChange={(v) => set('boss_d_mm', v)} />
                <Slider label="Pilot hole" value={p.boss_hole_mm} min={1.5} max={4} step={0.05} unit="mm" onChange={(v) => set('boss_hole_mm', v)} />
              </>
            ) : (
              <Slider label="Lid lip height" value={p.lip_mm} min={0} max={5} step={0.1} unit="mm" onChange={(v) => set('lip_mm', v)} />
            )}
          </div>
          <div className="cp-group">
            <div className="cp-group-title">
              PCB standoffs
              <label className="cp-check"><input type="checkbox" checked={p.pcb.enabled} onChange={(e) => setPcb('enabled', e.target.checked)} /> on</label>
            </div>
            {p.pcb.enabled && (
              <>
                <Slider label="Board length" value={p.pcb.length_mm} min={10} max={190} step={0.5} unit="mm" onChange={(v) => setPcb('length_mm', v)} />
                <Slider label="Board width" value={p.pcb.width_mm} min={10} max={150} step={0.5} unit="mm" onChange={(v) => setPcb('width_mm', v)} />
                <Slider label="Standoff height" value={p.pcb.standoff_h_mm} min={0} max={15} step={0.5} unit="mm" onChange={(v) => setPcb('standoff_h_mm', v)} />
                <Slider label="Hole inset" value={p.pcb.hole_inset_mm} min={1.5} max={8} step={0.1} unit="mm" onChange={(v) => setPcb('hole_inset_mm', v)} />
              </>
            )}
          </div>
          <div className="cp-group">
            <div className="cp-group-title">
              Cut-outs
              <button className="btn small" onClick={() => set('cutouts', [...p.cutouts, { face: 'x+', offset_mm: 0, z_mm: 5, width_mm: 6, height_mm: 4, label: `hole ${p.cutouts.length + 1}` }])}>+ add</button>
            </div>
            {p.cutouts.map((c, i) => (
              <div className="cp-cutout" key={i}>
                <label className="field cp-cut-face">
                  <span className="field-top"><span className="field-label">{c.label} — wall</span></span>
                  <select value={c.face} onChange={(e) => setCut(i, { face: e.target.value as Cutout['face'] })}>
                    {['x+', 'x-', 'y+', 'y-'].map((f) => <option key={f} value={f}>{f}</option>)}
                  </select>
                </label>
                <button className="btn small ghost" aria-label="Remove cut-out" onClick={() => set('cutouts', p.cutouts.filter((_, j) => j !== i))}>✕ remove</button>
                {(['offset_mm', 'z_mm', 'width_mm', 'height_mm'] as const).map((k) => (
                  <label className="field" key={k}>
                    <span className="field-top"><span className="field-label">{k.replace('_mm', '')}</span></span>
                    <input type="number" step={0.5} value={c[k]} onChange={(e) => setCut(i, { [k]: Number(e.target.value) })} />
                  </label>
                ))}
              </div>
            ))}
          </div>
        </Card>
      </div>
    </div>
  );
}
