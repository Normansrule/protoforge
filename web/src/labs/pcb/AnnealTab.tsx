import { useEffect, useMemo, useState } from 'react';
import { Card, Select, Stat, StatusChip } from '../../components/ui';
import { Plot, fmtNum } from '../../components/Plot';
import { useProject } from '../../state/project';
import { BoardView, DEFAULT_LAYERS } from './BoardView';
import type { AnnealResult, Layout } from './types';
import { useWorkerCompute, workerCall } from './useWorker';
import { prefersReducedMotion } from './zip';

export function AnnealTab({ layout }: { layout: Layout }) {
  const project = useProject((s) => s.project);
  const [iters, setIters] = useState('20000');
  const [seed, setSeed] = useState('42');
  const run = useWorkerCompute<AnnealResult>('pcb.anneal', { project, iterations: Number(iters), seed: Number(seed) });
  const r = run.data;
  const [frame, setFrame] = useState(0);
  const [playing, setPlaying] = useState(true);
  const [routed, setRouted] = useState<Layout | null>(null);
  const [routing, setRouting] = useState(false);

  useEffect(() => {
    if (!r) return;
    setRouted(null);
    if (prefersReducedMotion()) { setFrame(r.frames.length - 1); setPlaying(false); return; }
    setFrame(0);
    setPlaying(true);
  }, [r]);
  useEffect(() => {
    if (!r || !playing) return;
    let raf = 0;
    let last = performance.now();
    const tick = (t: number) => {
      if (t - last > 110) {
        last = t;
        setFrame((f) => {
          if (f >= r.frames.length - 1) { setPlaying(false); return f; }
          return f + 1;
        });
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [r, playing]);

  const fr = r?.frames[Math.min(frame, (r?.frames.length ?? 1) - 1)];
  const series = useMemo(() => r ? [
    { name: 'Current HPWL', x: r.iterations, y: r.hpwl },
    { name: 'Best so far', x: r.iterations, y: r.best_hpwl, dash: '5 4' },
  ] : [], [r]);

  const routeIt = async () => {
    if (!r) return;
    setRouting(true);
    try {
      setRouted(await workerCall<Layout>('pcb.layout', { project, placement: r.final_parts }));
    } finally {
      setRouting(false);
    }
  };

  return (
    <div className="stack">
      <Card
        title="Simulated annealing on half-perimeter wire length"
        subtitle="Movable parts start scattered at random; the edge anchors (connectors, antenna, microphones, SoC) stay put"
        actions={<div className="row">
          <Select value={iters} onChange={setIters} options={[{ value: '5000', label: '5 000 moves' }, { value: '20000', label: '20 000 moves' }, { value: '60000', label: '60 000 moves' }]} />
          <Select value={seed} onChange={setSeed} options={['1', '7', '42', '2026'].map((s) => ({ value: s, label: `seed ${s}` }))} />
          <button className="btn small" onClick={() => setPlaying(!playing)} disabled={!r}>{playing ? 'Pause' : 'Play'}</button>
          <button className="btn small" onClick={() => { setFrame(0); setPlaying(true); }} disabled={!r}>Replay</button>
        </div>}
      >
        {run.busy && !r && <div className="muted small">Annealing…</div>}
        {run.error && <div className="small" style={{ color: 'var(--bad-text)' }}>{run.error}</div>}
        {r && fr && (
          <div className="stack">
            <BoardView layout={layout} layers={{ ...DEFAULT_LAYERS, fcu: false, bcu: false, pour: false, silk: false, drc: false, court: true, rats: true }} ghostParts={fr.parts} hideRouting height={160} />
            <input type="range" min={0} max={r.frames.length - 1} value={frame} onChange={(e) => { setFrame(Number(e.target.value)); setPlaying(false); }} aria-label="Annealing frame" />
            <div className="grid grid-4">
              <Stat label="Iteration" value={fr.iteration.toLocaleString()} />
              <Stat label="Temperature" value={fmtNum(fr.temperature)} note="accept uphill moves with p = e^(−ΔC/T)" />
              <Stat label="HPWL" value={fmtNum(fr.hpwl)} unit="mm" note={`start ${fmtNum(r.initial_hpwl)} mm`} />
              <Stat label="Courtyard overlap" value={fmtNum(fr.overlap)} unit="mm²" tone={fr.overlap < 1e-6 ? 'pass' : 'warn'} />
            </div>
          </div>
        )}
      </Card>
      {r && (
        <div className="grid grid-2">
          <Card title="Wire length vs iteration">
            <Plot series={series} height={230} xLabel="iteration" yLabel="HPWL (mm)" vLines={fr ? [{ value: fr.iteration, label: 'now' }] : []} hLines={[{ value: r.constructive_hpwl, label: `constructive ${fmtNum(r.constructive_hpwl)} mm` }]} />
          </Card>
          <Card title="Temperature schedule and acceptance">
            <Plot series={[{ name: 'Temperature', x: r.iterations, y: r.temperature }]} height={110} xLabel="iteration" yLabel="T" yLog />
            <Plot series={[{ name: 'Acceptance ratio', x: r.iterations, y: r.acceptance, color: 'var(--series-3)' }]} height={110} xLabel="iteration" yLabel="accepted" yDomain={[0, 1]} />
          </Card>
        </div>
      )}
      {r && (
        <Card title="Does shorter wire mean an easier board?" subtitle="HPWL ignores congestion, pin escape and the ground pour — route the annealed placement and compare">
          <div className="row">
            <button className="btn primary" disabled={routing} onClick={routeIt}>{routing ? 'Routing in the background…' : 'Route the annealed placement'}</button>
            <span className="small muted">Annealed HPWL {fmtNum(r.final_hpwl)} mm vs constructive {fmtNum(r.constructive_hpwl)} mm (current board: {layout.stats.routed}/{layout.stats.connections} routed, {layout.stats.drc_violations} DRC).</span>
          </div>
          {routed && (
            <div className="stack" style={{ marginTop: 12 }}>
              <div className="grid grid-4">
                <Stat label="Routed" value={`${routed.stats.routed}/${routed.stats.connections}`} tone={routed.stats.routed_pct >= 100 ? 'pass' : 'fail'} />
                <Stat label="DRC" value={routed.stats.drc_violations} tone={routed.stats.drc_violations === 0 ? 'pass' : 'fail'} />
                <Stat label="Vias" value={routed.stats.via_count} />
                <Stat label="Track" value={fmtNum(routed.stats.track_length_mm)} unit="mm" />
              </div>
              <BoardView layout={routed} layers={DEFAULT_LAYERS} height={160} />
              <div className="row small">{routed.stats.routed_pct >= 100 && routed.stats.drc_violations === 0 ? <StatusChip status="pass" /> : <StatusChip status="warn" />}
                <span className="muted">The project keeps the constructive placement (the robotics pick-and-place planner uses it); this comparison is for learning.</span>
              </div>
            </div>
          )}
        </Card>
      )}
    </div>
  );
}
