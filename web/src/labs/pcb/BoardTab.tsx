import { useMemo, useState } from 'react';
import { Card, Chip, NumberField, StatusChip, Toggle } from '../../components/ui';
import { fmtNum } from '../../components/Plot';
import { call } from '../../engine/engine';
import { useProject } from '../../state/project';
import type { PcbParams } from '../../engine/types';
import { BoardView, DEFAULT_LAYERS, LAYERS, type LayerState } from './BoardView';
import type { Layout } from './types';
import { workerCall } from './useWorker';


export function BoardTab({ layout }: { layout: Layout }) {
  const [layers, setLayers] = useState<LayerState>(DEFAULT_LAYERS);
  const [net, setNet] = useState<string | null>(null);
  const [focus, setFocus] = useState<{ x: number; y: number } | null>(null);
  const [filter, setFilter] = useState('');
  const nets = useMemo(() => layout.nets.filter((n) => n.pins.length > 1 && n.name.toLowerCase().includes(filter.toLowerCase())), [layout.nets, filter]);
  const sel = layout.nets.find((n) => n.name === net);
  return (
    <div className="stack">
      <Card
        title="Board"
        subtitle={`${layout.board.width} × ${layout.board.height} mm · ${layout.board.layers} layers · scroll to zoom, drag to pan, click a pad or track to follow its net`}
        actions={<div className="row" style={{ gap: 6 }}>{LAYERS.map((l) => (
          <button key={l.key} className={`chip ${layers[l.key] ? 'info' : ''}`} style={{ cursor: 'pointer' }} onClick={() => setLayers({ ...layers, [l.key]: !layers[l.key] })} aria-pressed={layers[l.key]}>
            <span style={{ width: 10, height: 10, borderRadius: 3, background: l.color, display: 'inline-block', border: '1px solid var(--border-strong)' }} />
            {l.label}
          </button>
        ))}</div>}
      >
        <BoardView layout={layout} layers={layers} height={200} selectedNet={net} onSelectNet={setNet} focus={focus} onPickViolation={(i) => setFocus({ x: layout.drc[i].x, y: layout.drc[i].y })} />
        {sel && (
          <div className="callout" style={{ marginTop: 10 }}>
            <div className="row" style={{ justifyContent: 'space-between' }}>
              <b>{sel.name}</b>
              <span className="row">{sel.routed ? <StatusChip status="pass" /> : <StatusChip status="fail" />}<button className="btn small" onClick={() => setNet(null)}>Clear</button></span>
            </div>
            <div className="small">{sel.description}</div>
            <div className="small mono" style={{ marginTop: 4 }}>{sel.pins.join('  ')}</div>
            <div className="small" style={{ marginTop: 4 }}>
              {fmtNum(sel.length_mm)} mm of track · {sel.vias} vias · IPC-2221 width for {fmtNum(sel.current_a * 1000)} mA at 10 °C rise: {fmtNum(sel.ipc_width_mm)} mm (used {layout.rules.trace_mm} mm)
            </div>
          </div>
        )}
      </Card>
      <div className="grid grid-3">
        <Card title="Nets" subtitle={`${layout.stats.routed}/${layout.stats.connections} connections · pin-swapped SoC GPIOs`}>
          <input type="text" placeholder="Filter nets…" value={filter} onChange={(e) => setFilter(e.target.value)} style={{ marginBottom: 8 }} />
          <div style={{ maxHeight: 300, overflowY: 'auto' }}>
            <table className="table">
              <thead><tr><th>Net</th><th>Pins</th><th>mm</th><th /></tr></thead>
              <tbody>
                {nets.map((n) => (
                  <tr key={n.name} onClick={() => setNet(n.name === net ? null : n.name)} style={{ cursor: 'pointer', background: n.name === net ? 'var(--accent-soft)' : undefined }}>
                    <td><span className="mono">{n.name}</span> {n.class !== 'signal' && <Chip>{n.class}</Chip>}</td>
                    <td className="tnum">{n.pins.length}</td>
                    <td className="tnum">{n.class === 'ground' ? 'pour' : fmtNum(n.length_mm)}</td>
                    <td>{n.routed ? <span style={{ color: 'var(--good-text)' }}>✓</span> : <span style={{ color: 'var(--bad-text)' }}>✕</span>}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
        <Card title="Design-rule check" subtitle={layout.drc.length === 0 ? 'Every check passes' : `${layout.drc.length} violation(s) — click to zoom`}>
          {layout.drc.length === 0 ? (
            <div className="stack" style={{ gap: 8 }}>
              <StatusChip status="pass" />
              <div className="small muted">Checked independently of the router, on exact geometry: copper clearance (pad/track/via/pour) ≥ {layout.rules.clearance_mm} mm, width ≥ {layout.rules.trace_mm} mm, copper-to-edge ≥ {layout.rules.edge_clearance_mm} mm, drills ≥ {layout.rules.min_drill_mm} mm, annular ring ≥ {layout.rules.min_annular_mm} mm, hole spacing, antenna keep-out, courtyard overlaps and connectivity (union–find over copper + pours).</div>
            </div>
          ) : (
            <div style={{ maxHeight: 300, overflowY: 'auto' }} className="stack">
              {layout.drc.map((v, i) => (
                <button key={i} className="btn small" style={{ justifyContent: 'flex-start', whiteSpace: 'normal', textAlign: 'left' }} onClick={() => setFocus({ x: v.x, y: v.y })}>
                  <Chip tone="fail">{v.kind}</Chip> {v.message}
                </button>
              ))}
            </div>
          )}
          <h4 style={{ marginTop: 14, marginBottom: 6 }}>SoC pin assignment</h4>
          <div className="small muted" style={{ marginBottom: 6 }}>Chosen by the router (pin swapping) — re-map to your SoC's real pin-mux table.</div>
          <div className="row" style={{ gap: 4 }}>{layout.gpio.map(([n, p]) => <Chip key={n}><span className="mono">{n}→{p}</span></Chip>)}</div>
        </Card>
        <RulesCard layout={layout} />
      </div>
      {layout.notes.length > 0 && (
        <Card title="Engineering notes">
          <ul className="small" style={{ margin: 0, paddingLeft: 18 }}>{layout.notes.map((n, i) => <li key={i}>{n}</li>)}</ul>
        </Card>
      )}
    </div>
  );
}

function RulesCard({ layout }: { layout: Layout }) {
  const project = useProject((s) => s.project);
  const patch = useProject((s) => s.patchDesign);
  const [draft, setDraft] = useState<PcbParams>(project.design.pcb);
  const [busy, setBusy] = useState(false);
  const [preview, setPreview] = useState<{ routed: number; connections: number; drc: number; vias: number; ms: number; layout: unknown } | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [auto, setAuto] = useState(true);
  const dirty = JSON.stringify(draft) !== JSON.stringify(project.design.pcb);
  const set = (k: keyof PcbParams) => (v: number) => { setDraft({ ...draft, [k]: v }); setPreview(null); };
  const reroute = async () => {
    setBusy(true);
    setErr(null);
    const next = structuredClone(project);
    next.design.pcb = draft;
    const t0 = performance.now();
    try {
      const l = await workerCall<Layout>('pcb.layout', { project: next });
      setPreview({ routed: l.stats.routed, connections: l.stats.connections, drc: l.stats.drc_violations, vias: l.stats.via_count, ms: performance.now() - t0, layout: l });
      if (auto) {
        call('pcb.adopt', { project: next, layout: l });
        patch('pcb', draft);
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };
  const apply = () => {
    if (!preview) return;
    const next = structuredClone(project);
    next.design.pcb = draft;
    call('pcb.adopt', { project: next, layout: preview.layout });
    patch('pcb', draft);
  };
  return (
    <Card title="Design rules" subtitle="Re-routes in a background worker (the router negotiates for a few seconds)">
      <div className="grid grid-2" style={{ gap: 8 }}>
        <NumberField label="Min trace" unit="mm" step={0.01} value={draft.min_trace_mm} onChange={set('min_trace_mm')} />
        <NumberField label="Min clearance" unit="mm" step={0.01} value={draft.min_clearance_mm} onChange={set('min_clearance_mm')} />
        <NumberField label="Via drill" unit="mm" step={0.05} value={draft.via_drill_mm} onChange={set('via_drill_mm')} />
        <NumberField label="Via pad" unit="mm" step={0.05} value={draft.via_pad_mm} onChange={set('via_pad_mm')} />
        <NumberField label="Grid" unit="mm" step={0.05} value={draft.grid_mm} onChange={set('grid_mm')} />
        <NumberField label="Thickness" unit="mm" step={0.1} value={draft.thickness_mm} onChange={set('thickness_mm')} />
        <NumberField label="Board length" unit="mm" step={0.5} value={draft.width_mm} onChange={set('width_mm')} />
        <NumberField label="Board width" unit="mm" step={0.5} value={draft.height_mm} onChange={set('height_mm')} />
      </div>
      <Toggle label="Apply to the project when routing succeeds" value={auto} onChange={setAuto} />
      <div className="row">
        <button className="btn primary" disabled={busy || !dirty} onClick={reroute}>{busy ? 'Routing…' : 'Re-route with these rules'}</button>
        {!auto && preview && dirty && <button className="btn" onClick={apply}>Apply</button>}
        <button className="btn ghost" disabled={busy} onClick={() => { setDraft(project.design.pcb); setPreview(null); }}>Reset</button>
      </div>
      {preview && (
        <div className="small" style={{ marginTop: 8 }}>
          {preview.routed}/{preview.connections} routed · {preview.drc} DRC · {preview.vias} vias · {(preview.ms / 1000).toFixed(1)} s
        </div>
      )}
      {err && <div className="small" style={{ color: 'var(--bad-text)' }}>{err}</div>}
      <div className="small muted" style={{ marginTop: 8 }}>
        Routing grid = grid ÷ 2 ({fmtNum(layout.rules.grid_mm / 2)} mm). Strategy used: <i>{layout.stats.strategy}</i> ({layout.stats.attempts} of 4 tried){layout.stats.from_cache ? '; default Iris copper is pre-routed by this engine and re-verified live' : ''}.
      </div>
    </Card>
  );
}
