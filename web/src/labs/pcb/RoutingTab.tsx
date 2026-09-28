import { useState } from 'react';
import { Card, Select, Stat } from '../../components/ui';
import { fmtNum } from '../../components/Plot';
import { useProject } from '../../state/project';
import type { Layout, WaveTrace } from './types';
import { useWorkerCompute } from './useWorker';
import { Wavefront } from './Wavefront';

export function RoutingTab({ layout }: { layout: Layout }) {
  const project = useProject((s) => s.project);
  const candidates = layout.nets.filter((n) => n.class !== 'ground' && n.pins.length >= 2);
  const [net, setNet] = useState(candidates.find((n) => n.name === 'I2C_SDA')?.name ?? candidates[0]?.name ?? '');
  const [go, setGo] = useState<string | null>(null);
  const tr = useWorkerCompute<WaveTrace>('pcb.route_trace', go ? { project, net: go } : null);
  const [wi, setWi] = useState(0);
  const [playing, setPlaying] = useState(true);
  const [speed, setSpeed] = useState('1');
  const trace = tr.data && tr.data.net === go ? tr.data : null;
  const cells = trace ? trace.waves.reduce((a, w) => a + w.expanded.length / 4, 0) : 0;
  return (
    <div className="stack">
      <Card
        title="Lee's wavefront, live"
        subtitle="The net is ripped up and re-routed on the finished board while every visited grid cell is recorded (Dijkstra without the A* heuristic, so the flood is visible)"
        actions={<div className="row">
          <Select value={net} onChange={(v) => setNet(v)} options={candidates.map((n) => ({ value: n.name, label: `${n.name} (${n.pins.length} pins)` }))} />
          <button className="btn primary small" disabled={tr.busy} onClick={() => { setGo(net); setWi(0); setPlaying(true); }}>{tr.busy ? 'Re-routing board…' : 'Trace'}</button>
        </div>}
      >
        {!trace && !tr.busy && <div className="muted small" style={{ padding: 20 }}>Pick a net and press <b>Trace</b>. The worker re-runs the whole negotiated routing (a few seconds), then replays how the wave floods out from one pin until it touches the copper already on the net.</div>}
        {tr.busy && <div className="muted small" style={{ padding: 20 }}>Negotiating the board in a background worker…</div>}
        {tr.error && <div className="small" style={{ color: 'var(--bad-text)' }}>{tr.error}</div>}
        {trace && trace.waves.length === 0 && <div className="small muted">That net has no connection to search (single pin or pin-swapped only).</div>}
        {trace && trace.waves.length > 0 && (
          <div className="stack">
            <div className="row">
              <Select label="Connection" value={String(wi)} onChange={(v) => { setWi(Number(v)); setPlaying(true); }} options={trace.waves.map((w, i) => ({ value: String(i), label: `${i + 1}. from ${w.from_pin}${w.found ? '' : ' (failed)'}` }))} />
              <Select label="Speed" value={speed} onChange={setSpeed} options={[{ value: '0.4', label: 'slow' }, { value: '1', label: 'normal' }, { value: '3', label: 'fast' }]} />
              <button className="btn small" onClick={() => setPlaying(!playing)}>{playing ? 'Pause' : 'Play'}</button>
            </div>
            <Wavefront layout={layout} trace={trace} waveIndex={wi} playing={playing} speed={Number(speed)} onDone={() => setPlaying(false)} />
            <div className="grid grid-4">
              <Stat label="Connections" value={trace.waves.length} />
              <Stat label="Cells visited" value={cells.toLocaleString()} note={`grid ${fmtNum(trace.grid_mm)} mm, 2 layers`} />
              <Stat label="Grid" value={`${trace.nx} × ${trace.ny}`} unit="×2" />
              <Stat label="Search time" value={fmtNum(tr.ms / 1000)} unit="s" note="whole board re-negotiated" />
            </div>
          </div>
        )}
      </Card>
      <Card title="How this router works">
        <ol className="small" style={{ margin: 0, paddingLeft: 18, lineHeight: 1.7 }}>
          <li><b>Occupancy grid.</b> Every grid point on both layers knows whether it is free, within clearance of exactly one net (only that net may run a centre-line there) or blocked. Grid <i>edges</i> are checked too, so off-grid fine-pitch pads are respected exactly.</li>
          <li><b>Lee / A* search.</b> Each new pin floods outward (cost: 1 per top step, more on the bottom which carries the ground pour, extra for bends and vias) until it reaches any copper already on the net — a rectilinear Steiner-tree heuristic.</li>
          <li><b>Pin swapping.</b> A SoC GPIO signal may end on <i>any</i> free GPIO pad; the router claims the pad its wave reaches first.</li>
          <li><b>Negotiated congestion (PathFinder).</b> All nets are routed even if they overlap; overlaps get more expensive every iteration and remember past contention, so nets negotiate who gets which channel. Converged here after a few dozen passes.</li>
          <li><b>Ground.</b> Pads drop to the pour through the cheapest nearby via; pours on both layers are stitched with vias until ground is one island.</li>
          <li><b>Independent DRC.</b> Connectivity and clearances are re-extracted from exact geometry, not trusted from the router.</li>
        </ol>
      </Card>
    </div>
  );
}
