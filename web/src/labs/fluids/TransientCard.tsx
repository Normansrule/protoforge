// Transient heating for a use scenario: implicit-Euler time stepping of the
// same 3-D model, animated on the CAD surfaces with a time scrubber, next to
// the Power Lab's two-node network driven by the same schedule.
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { TempLegend, ThermalView, decodeSurfaces, type ViewOpts, type VolumeData } from './ThermalView';
import { configOf, type PodControls } from './controls';
import type { ScenariosOut, TransientOut } from './types';
import { b64Bytes, cancelLane, fmt, fmtTime, prefersReducedMotion, toCsv, useRaf, workerCall } from './util';

const LANE = 'transient';

interface Job { key: string; started: number }
interface Done { key: string; data: TransientOut | null; error: string | null; ms: number }

export function TransientCard({ ctl }: { ctl: PodControls }) {
  const project = useProject((s) => s.project);
  const scen = useCompute<ScenariosOut>('fluids.scenarios', {}).data;
  const [preset, setPreset] = useState('r008');
  // The transient runs only when asked (button or scenario change), in its own worker lane:
  // design edits mark the result out of date instead of silently queueing more multi-second runs.
  const input = useMemo(() => ({ project, config: { ...configOf(ctl), voxel_mm: Math.max(1.5, ctl.voxel_mm) }, preset, frames: 60 }), [project, ctl, preset]);
  const inputKey = useMemo(() => JSON.stringify(input), [input]);
  const [job, setJob] = useState<Job | null>(null);
  const [done, setDone] = useState<Done | null>(null);
  const [now, setNow] = useState(0);
  const jobRef = useRef<Job | null>(null);
  const start = useCallback((inp: unknown, key: string) => {
    cancelLane(LANE);
    const j = { key, started: performance.now() };
    jobRef.current = j;
    setJob(j);
    setNow(j.started);
    workerCall<TransientOut>('fluids.pod_transient', inp, LANE)
      .then(({ data, ms }) => {
        if (jobRef.current !== j) return;
        jobRef.current = null;
        setJob(null);
        setDone({ key, data, error: null, ms });
      })
      .catch((err: Error) => {
        if (jobRef.current !== j) return;
        jobRef.current = null;
        setJob(null);
        setDone((d) => ({ key, data: d?.data ?? null, error: err.message, ms: 0 }));
      });
  }, []);
  const cancel = () => {
    jobRef.current = null;
    cancelLane(LANE);
    setJob(null);
  };
  useEffect(() => () => cancelLane(LANE), []); // leaving the page stops the run
  useEffect(() => {
    if (!job) return;
    const id = window.setInterval(() => setNow(performance.now()), 250);
    return () => window.clearInterval(id);
  }, [job]);
  const elapsed = job ? Math.max(0, (now - job.started) / 1000) : 0;
  const res = { data: done?.data ?? null, error: done?.error ?? null, busy: !!job };
  const stale = !!done?.data && !job && done.key !== inputKey;
  const r = res.data;
  const surfaces = useMemo(() => (r ? decodeSurfaces(r.surfaces) : []), [r]);
  const inner = useMemo(() => (r ? decodeSurfaces(r.inner) : []), [r]);
  const [frame, setFrame] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [acc, setAcc] = useState(0);
  useEffect(() => {
    if (r) {
      setFrame(0);
      setPlaying(!prefersReducedMotion());
    }
  }, [r]);
  const nF = r?.frames.length ?? 0;
  useRaf(playing && nF > 1, (dt) => {
    // whole scenario in ~8 s
    const a = acc + (dt * nF) / 8;
    if (a >= 1) {
      setFrame((f) => (f + Math.floor(a)) % nF);
      setAcc(a - Math.floor(a));
    } else setAcc(a);
  });
  const volume = useMemo<VolumeData | null>(() => {
    if (!r || nF === 0) return null;
    const [nx, ny, nz] = r.volume_dims;
    return { nx, ny, nz, origin: r.volume_origin, h_mm: r.volume_h_mm, t_lo: r.t_lo, t_hi: r.t_hi, bytes: b64Bytes(r.frames[Math.min(frame, nF - 1)]) };
  }, [r, frame, nF]);
  const opts: ViewOpts = { xrayLid: false, xrayShell: false, inner: true, markers: false, wire: false };
  const limit = project.design.thermal.skin_limit_c;
  const tNow = r ? r.frame_t_s[Math.min(frame, nF - 1)] : 0;
  const mins = (v: number[]) => v.map((x) => x / 60);
  let segStart = 0;
  const bands = (r?.segments ?? []).map((s, i) => {
    const b = { from: segStart / 60, to: (segStart + s.duration_s) / 60, label: s.label, color: i % 2 ? 'var(--surface-3)' : 'var(--surface-2)' };
    segStart += s.duration_s;
    return b;
  });
  const idxNow = r ? Math.min(r.t_s.length - 1, r.t_s.findIndex((t) => t >= tNow - 1e-6)) : 0;
  return (
    <Card
      title="Transient: putting the glasses on"
      subtitle="Implicit Euler on the same 3-D model; the Power Lab's two-node network runs the same schedule for comparison"
      actions={res.busy ? <span className="cp-spin" aria-label="computing" /> : undefined}
    >
      <div className="cp-toolbar" style={{ marginTop: 0, marginBottom: 12 }}>
        <Select
          label="Use scenario"
          value={preset}
          options={(scen?.transient ?? [['r008', 'Put on, display always on (R-008)', []]]).map(([id, label]) => ({ value: id, label }))}
          onChange={(v) => {
            setPreset(v);
            // a new scenario re-runs straight away once there is a result to replace
            if (done?.data || job) {
              const inp = { ...input, preset: v };
              start(inp, JSON.stringify(inp));
            }
          }}
        />
        {!job && !done?.data && (
          <button className="btn primary" onClick={() => start(input, inputKey)}>▶ Run the transient</button>
        )}
        {job && (
          <span className="row" style={{ gap: 8 }} role="status" aria-live="polite">
            <span className="muted small tnum">Time-stepping… {elapsed.toFixed(0)} s</span>
            <button className="btn small" onClick={cancel}>Cancel</button>
          </span>
        )}
        {stale && (
          <span className="row" style={{ gap: 8 }}>
            <span className="small" style={{ color: 'var(--warn-text)' }}>The design changed since this run.</span>
            <button className="btn small primary" onClick={() => start(input, inputKey)}>▶ Re-run</button>
          </span>
        )}
      </div>
      {!done?.data && (
        <p className="small muted">
          Steps the full 3-D model through 240 implicit time steps (Δt = scenario length ÷ 240) in its own background worker — typically 2–6 s, measured on a
          2-core machine; longer while the machine is busy. The timer shows how long it has been running, and you can cancel it.
        </p>
      )}
      {done?.error && done.error !== 'cancelled' && <ErrorBox error={done.error} />}
      {r && volume && (
        <>
          <div className="grid grid-2">
            <ThermalView
              surfaces={surfaces}
              inner={inner}
              volume={volume}
              lo={r.t_lo}
              hi={r.t_hi}
              bbox={{ min: r.bbox_min, max: r.bbox_max }}
              opts={opts}
              height={360}
              preset="iso"
              nonce={0}
              overlay={
                <>
                  <TempLegend lo={r.t_lo} hi={r.t_hi} limit={limit} />
                  <div className="fl-clock mono">t = {fmtTime(tNow)}</div>
                </>
              }
            />
            <div className="stack" style={{ gap: 10 }}>
              <Plot
                series={[
                  { name: 'hottest frame surface', x: mins(r.t_s), y: r.peak_skin_c },
                  { name: 'mean frame surface', x: mins(r.t_s), y: r.mean_skin_c },
                  { name: 'board max', x: mins(r.t_s), y: r.board_max_c },
                  { name: 'lumped shell (Power Lab)', x: mins(r.t_s), y: r.lumped_shell_c, dash: '6 4' },
                ]}
                xLabel="time, min"
                yLabel="°C"
                height={250}
                legend
                hLines={[{ value: limit, label: `${limit} °C limit`, color: 'var(--bad)' }]}
                vLines={[{ value: tNow / 60, label: '' }]}
                xBands={bands}
              />
              <div className="grid grid-3">
                <Stat label="Now: hottest" value={fmt(r.peak_skin_c[idxNow], 1)} unit="°C" />
                <Stat label="Steady peak" value={fmt(r.steady_peak_c, 1)} unit="°C" />
                <Stat label="Reaches limit" value={r.time_to_limit_s != null ? fmtTime(r.time_to_limit_s) : 'never'} note={`${limit} °C`} tone={r.time_to_limit_s != null ? 'fail' : 'pass'} />
              </div>
            </div>
          </div>
          <div className="cp-toolbar">
            <button className="btn small" onClick={() => setPlaying((p) => !p)} aria-pressed={playing}>{playing ? '❚❚ Pause' : '▶ Play'}</button>
            <button className="btn small" onClick={() => { setPlaying(false); setFrame((f) => Math.min(nF - 1, f + 1)); }}>Step ▸</button>
            <Slider label="Time" value={frame} min={0} max={Math.max(1, nF - 1)} step={1} format={(v) => fmtTime(r.frame_t_s[Math.round(v)] ?? 0)} onChange={(v) => { setPlaying(false); setFrame(Math.round(v)); }} />
            <DownloadButton
              filename="iris-transient.csv"
              small
              content={() => toCsv([['t_s', 'display_duty', 'power_mw', 'peak_skin_c', 'mean_skin_c', 'board_max_c', 'display_max_c', 'lumped_shell_c', 'lumped_board_c'], ...r.t_s.map((t, i) => [t, r.duty[i], +r.power_mw[i].toFixed(3), +r.peak_skin_c[i].toFixed(3), +r.mean_skin_c[i].toFixed(3), +r.board_max_c[i].toFixed(3), +r.display_max_c[i].toFixed(3), +r.lumped_shell_c[i].toFixed(3), +r.lumped_board_c[i].toFixed(3)])])}
            />
          </div>
          <p className="small muted">
            {r.note} {fmt(r.iterations_per_step, 1)} conjugate-gradient iterations per step (warm-started). {Math.round((r.frame_t_s[nF - 1] ?? 0) / r.dt_s)} time steps, {nF} stored frames
            {done && done.ms > 0 ? `, computed in ${(done.ms / 1000).toFixed(1)} s` : ''}.
          </p>
          <Explainer title="Implicit (backward) Euler" refs={[{ label: 'Hughes, The Finite Element Method (2000), ch. 7' }, { label: 'Patankar (1980), §4.3' }]}>
            <Formula>(C/Δt + K + H) θ<sup>n+1</sup> = C/Δt θ<sup>n</sup> + Q(t<sub>n+1</sub>)</Formula>
            Unconditionally stable, so Δt = {fmt(r.dt_s, 0)} s is fine even though an explicit scheme would need Δt ≲ ρc h²/(6k) — tens of milliseconds in the copper-rich board voxels. C is the lumped heat capacity (ρc V/8 per node, plus each
            component's mass × 750 J/kg K). The slow rise is the frame's thermal mass; the lumped model's two time constants come from the same physics with hand-picked R and C.
          </Explainer>
        </>
      )}
    </Card>
  );
}
