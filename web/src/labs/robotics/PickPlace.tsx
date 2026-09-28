import { useMemo } from 'react';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat, StatusChip, Toggle } from '../../components/ui';
import { PlayBar, fmt, frameAt, json, toCSV, useClock } from '../control/kit';
import type { PnpConfig, PnpPlan } from './types';

export const DEFAULT_PNP: PnpConfig = {
  l1_mm: 250, l2_mm: 200, j1_vmax_dps: 360, j1_amax_dps2: 2400, j2_vmax_dps: 540, j2_amax_dps2: 3600, j4_vmax_dps: 1200, j4_amax_dps2: 12000,
  z_stroke_mm: 8, z_vmax_mm_s: 400, z_amax_mm_s2: 6000, pick_dwell_s: 0.05, place_dwell_s: 0.04, vision: true, vision_dwell_s: 0.12,
  feeder_pitch_mm: 12, feeder_y_mm: 200, camera_xy_mm: [0, 250], board_origin_mm: [-29, 290], board_handling_s: 3, fiducial_dwell_s: 0.25,
  flip_s: 8, optimize: true, fps: 30,
};

const KINDS = [
  { id: 'travel', label: 'XY travel', color: 'var(--series-1)' },
  { id: 'z', label: 'Z moves', color: 'var(--series-3)' },
  { id: 'vision', label: 'Vision', color: 'var(--series-7)' },
  { id: 'dwell', label: 'Vacuum dwell', color: 'var(--series-4)' },
  { id: 'handling', label: 'Board handling', color: 'var(--series-5)' },
] as const;

const partColor = (fp: string) => {
  if (/^(0402|0201|0603|0805|1206)/.test(fp)) return 'var(--copper)';
  if (/^(USB|FPC|JST)/.test(fp)) return 'var(--text-2)';
  return 'var(--surface-3)';
};

export function PickPlace() {
  const project = useProject((s) => s.project);
  const patch = useProject((s) => s.patchDesign);
  const saved = (project.design.extra?.pnp ?? {}) as Partial<PnpConfig>;
  const cfg: PnpConfig = { ...DEFAULT_PNP, ...saved };
  const setCfg = (p: Partial<PnpConfig>) => patch('extra', { pnp: { ...saved, ...p } } as never);
  const req = project.requirements.find((r) => r.verification.metric === 'robotics.pnp_cycle_s');
  const limit = req?.verification.target ?? 60;
  const res = useCompute<PnpPlan>('robotics.pnp', { project, config: cfg });
  const d = res.data;
  const clock = useClock(d?.cycle_s ?? 1, { speed: 2, resetKey: d?.cycle_s });
  const i = d ? frameAt(d.frames.t, clock.t) : 0;
  const placed = d ? d.placements.filter((p) => p.t_placed <= clock.t).length : 0;
  const holding = d?.frames.holding[i] ?? -1;
  const ev = d?.events.find((e) => clock.t >= e.t0 && clock.t < e.t1);
  const speedScale = cfg.j1_vmax_dps / DEFAULT_PNP.j1_vmax_dps;

  // cell geometry in mm (y up); viewBox fixed around the reachable work area
  const vb = { x: -165, y: -30, w: 330, h: 350 };
  const bx = cfg.board_origin_mm[0], by = cfg.board_origin_mm[1];

  const tipX = d?.frames.x[i] ?? 0, tipY = d?.frames.y[i] ?? 0, z = d?.frames.z[i] ?? 0;
  const q1 = ((d?.frames.q1[i] ?? 0) * Math.PI) / 180;
  const elbow = [cfg.l1_mm * Math.cos(q1), cfg.l1_mm * Math.sin(q1)];

  const orderCsv = () => (d ? toCSV(['order', 'refdes', 'feeder_slot', 'part', 'board_x_mm', 'board_y_mm', 'rot_deg', 'side', 't_placed_s'],
    ((): (number[] | string[])[] => {
      const ps = [...d.placements].sort((a, b) => a.t_placed - b.t_placed);
      return [ps.map((_, k) => k + 1), ps.map((p) => p.refdes), ps.map((p) => p.feeder), ps.map((p) => d.feeders[p.feeder].key.replace(/,/g, ' ')), ps.map((p) => p.board_xy[0]), ps.map((p) => p.board_xy[1]), ps.map((p) => p.rot_deg), ps.map((p) => p.side), ps.map((p) => p.t_placed)];
    })()) : '');
  const eventsCsv = () => (d ? toCSV(['t0_s', 't1_s', 'kind', 'label', 'refdes'], [d.events.map((e) => e.t0), d.events.map((e) => e.t1), d.events.map((e) => e.kind), d.events.map((e) => e.label), d.events.map((e) => e.refdes ?? '')]) : '');

  const total = d?.cycle_s ?? 1;
  const bdRows = useMemo(() => (d ? KINDS.map((k) => ({ ...k, v: d.breakdown[k.id] })) : []), [d]);

  return (
    <div className="stack">
      <div className="grid grid-4">
        <Stat label="Board cycle time" value={fmt(d?.cycle_s, 1)} unit="s" tone={d ? (d.cycle_s <= limit ? 'pass' : 'fail') : undefined} note={<span>{d && <StatusChip status={d.cycle_s <= limit ? 'pass' : 'fail'} />} R-023 ≤ {limit} s</span>} />
        <Stat label="Throughput" value={fmt(d?.cph, 0)} unit="CPH" note={`${fmt(d?.per_part_s, 2)} s per placement`} />
        <Stat label="Sequencing gain" value={d ? fmt(d.naive_cycle_s - d.cycle_s, 2) : '—'} unit="s" note={`naïve BOM order ${fmt(d?.naive_cycle_s, 1)} s`} />
        <Stat label="Parts / feeders" value={d ? `${d.placements.length} / ${d.feeders.length}` : '—'} note={`XY travel ${fmt((d?.travel_mm ?? 0) / 1000, 2)} m per board`} />
      </div>

      <div className="grid grid-sidebar">
        <Card title="Cell settings" subtitle="Saved in design.extra.pnp — the R-023 metric uses them">
          <Slider label="Arm speed & acceleration" value={speedScale} min={0.25} max={2} step={0.05} format={(v) => `${(v * 100).toFixed(0)} %`} onChange={(v) => setCfg({ j1_vmax_dps: DEFAULT_PNP.j1_vmax_dps * v, j2_vmax_dps: DEFAULT_PNP.j2_vmax_dps * v, j1_amax_dps2: DEFAULT_PNP.j1_amax_dps2 * v, j2_amax_dps2: DEFAULT_PNP.j2_amax_dps2 * v })} />
          <Slider label="Z stroke" value={cfg.z_stroke_mm} min={2} max={30} step={0.5} unit="mm" onChange={(v) => setCfg({ z_stroke_mm: v })} />
          <Slider label="Pick dwell (vacuum on)" value={cfg.pick_dwell_s * 1000} min={10} max={300} step={5} unit="ms" onChange={(v) => setCfg({ pick_dwell_s: v / 1000 })} />
          <Slider label="Place dwell (vacuum off)" value={cfg.place_dwell_s * 1000} min={10} max={300} step={5} unit="ms" onChange={(v) => setCfg({ place_dwell_s: v / 1000 })} />
          <Toggle label="Up-camera alignment for every part" value={cfg.vision} onChange={(v) => setCfg({ vision: v })} />
          {cfg.vision && <Slider label="Vision dwell" value={cfg.vision_dwell_s * 1000} min={20} max={500} step={10} unit="ms" onChange={(v) => setCfg({ vision_dwell_s: v / 1000 })} />}
          <Slider label="Feeder pitch" value={cfg.feeder_pitch_mm} min={8} max={24} step={1} unit="mm" onChange={(v) => setCfg({ feeder_pitch_mm: v })} />
          <Slider label="Board load + unload" value={cfg.board_handling_s} min={0} max={10} step={0.1} unit="s" onChange={(v) => setCfg({ board_handling_s: v })} />
          <Toggle label="Optimise feeder order & sequence" value={cfg.optimize} onChange={(v) => setCfg({ optimize: v })} />
          <button className="btn small ghost" onClick={() => patch('extra', { pnp: undefined } as never)}>Reset cell to defaults</button>
          {d?.warnings.map((w) => <div key={w} className="callout small" style={{ borderLeftColor: 'var(--warn)', marginTop: 8 }}>! {w}</div>)}
          <ErrorBox error={res.error} />
        </Card>

        <Card
          title="SCARA populating the Iris main board"
          subtitle={<span>{ev ? <><b>{ev.label}</b>{ev.refdes ? ` · ${ev.refdes}` : ''}</> : 'idle'} · nozzle z {z.toFixed(1)} mm</span>}
          actions={<><DownloadButton filename="iris-pnp-sequence.csv" content={orderCsv} label="Sequence CSV" small /><DownloadButton filename="iris-pnp-timeline.csv" content={eventsCsv} label="Timeline CSV" small /><DownloadButton filename="iris-pnp-plan.json" content={() => json(d)} label="Plan JSON" small /></>}
        >
          {d && (
            <svg viewBox={`${vb.x} ${-(vb.y + vb.h)} ${vb.w} ${vb.h}`} style={{ width: '100%', maxHeight: 520, display: 'block' }} role="img" aria-label="Pick and place cell animation">
              <defs>
                <pattern id="pp-grid" width="20" height="20" patternUnits="userSpaceOnUse">
                  <path d="M20 0H0V20" fill="none" stroke="var(--grid)" strokeWidth={0.6} />
                </pattern>
              </defs>
              <rect x={vb.x} y={-(vb.y + vb.h)} width={vb.w} height={vb.h} fill="url(#pp-grid)" />
              <g transform="scale(1,-1)">
                {/* reach */}
                <circle r={d.reach_mm[1]} fill="var(--accent-soft)" stroke="var(--border-strong)" strokeWidth={0.8} />
                <circle r={d.reach_mm[0]} fill="var(--surface-1)" stroke="var(--border-strong)" strokeWidth={0.8} />
                {/* feeder rail */}
                <rect x={d.feeders[0]?.xy[0] - 10} y={cfg.feeder_y_mm - 16} width={(d.feeders.length - 1) * cfg.feeder_pitch_mm + 20} height={22} rx={3} fill="var(--surface-3)" stroke="var(--border-strong)" strokeWidth={0.6} />
                {d.feeders.map((f) => {
                  const active = holding >= 0 && d.placements[holding]?.feeder === f.slot;
                  return (
                    <g key={f.slot}>
                      <rect x={f.xy[0] - cfg.feeder_pitch_mm * 0.4} y={f.xy[1] - 14} width={cfg.feeder_pitch_mm * 0.8} height={18} rx={1.5} fill={active ? 'var(--accent)' : 'var(--surface-2)'} stroke="var(--border-strong)" strokeWidth={0.5} />
                      <rect x={f.xy[0] - 1.5} y={f.xy[1] - 1.5} width={3} height={3} fill={partColor(f.footprint)} />
                    </g>
                  );
                })}
                {/* up camera */}
                <circle cx={cfg.camera_xy_mm[0]} cy={cfg.camera_xy_mm[1]} r={7} fill="var(--surface-2)" stroke="var(--series-7)" strokeWidth={1.2} />
                <circle cx={cfg.camera_xy_mm[0]} cy={cfg.camera_xy_mm[1]} r={3} fill="var(--series-7)" opacity={ev?.kind === 'vision' ? 1 : 0.4} />
                {/* conveyor + board */}
                <rect x={vb.x} y={by - 6} width={vb.w} height={d.board_h_mm + 12} fill="var(--surface-2)" opacity={0.8} />
                <rect x={bx} y={by} width={d.board_w_mm} height={d.board_h_mm} rx={1.2} fill="var(--mask)" stroke="var(--copper)" strokeWidth={0.5} />
                {d.placements.filter((p) => p.t_placed <= clock.t).map((p) => (
                  <rect key={p.refdes} x={p.xy[0] - p.size_mm[0] / 2} y={p.xy[1] - p.size_mm[1] / 2} width={p.size_mm[0]} height={p.size_mm[1]} fill={partColor(p.footprint)} stroke="var(--text)" strokeWidth={0.2} transform={`rotate(${p.rot_deg} ${p.xy[0]} ${p.xy[1]})`} />
                ))}
                {/* arm */}
                <line x1={0} y1={0} x2={elbow[0]} y2={elbow[1]} stroke="var(--text-2)" strokeWidth={14} strokeLinecap="round" opacity={0.85} />
                <line x1={elbow[0]} y1={elbow[1]} x2={tipX} y2={tipY} stroke="var(--text-2)" strokeWidth={10} strokeLinecap="round" opacity={0.85} />
                <circle cx={elbow[0]} cy={elbow[1]} r={8} fill="var(--surface-1)" stroke="var(--text-2)" strokeWidth={2} />
                <rect x={-18} y={-18} width={36} height={36} rx={6} fill="var(--surface-3)" stroke="var(--text-2)" strokeWidth={2} />
                {/* nozzle: grows as it rises (z = 0 travel height) */}
                <circle cx={tipX} cy={tipY} r={6 + z * 0.35} fill="var(--surface-1)" stroke="var(--accent)" strokeWidth={2} />
                {holding >= 0 && d.placements[holding] && (
                  <rect x={tipX - 2.5} y={tipY - 2.5} width={5} height={5} fill={partColor(d.placements[holding].footprint)} stroke="var(--text)" strokeWidth={0.4} />
                )}
              </g>
              <text x={vb.x + 6} y={-(vb.y + vb.h) + 14} fontSize={10} fill="var(--muted)">SCARA base at origin · 20 mm grid</text>
            </svg>
          )}
          {d && (
            <div style={{ marginTop: 6 }}>
              <div className="small muted" style={{ marginBottom: 4 }}>Board close-up ({d.board_w_mm} × {d.board_h_mm} mm) — {placed}/{d.placements.length} placed</div>
              <svg viewBox={`-2 ${-(d.board_h_mm + 2)} ${d.board_w_mm + 4} ${d.board_h_mm + 4}`} style={{ width: '100%', display: 'block', maxHeight: 140 }} role="img" aria-label="Board close-up">
                <g transform="scale(1,-1)">
                  <rect x={0} y={0} width={d.board_w_mm} height={d.board_h_mm} rx={0.8} fill="var(--mask)" stroke="var(--copper)" strokeWidth={0.15} />
                  {d.placements.map((p) => {
                    const done = p.t_placed <= clock.t;
                    return (
                      <g key={p.refdes} transform={`rotate(${p.rot_deg} ${p.board_xy[0]} ${p.board_xy[1]})`}>
                        <rect x={p.board_xy[0] - p.size_mm[0] / 2} y={p.board_xy[1] - p.size_mm[1] / 2} width={p.size_mm[0]} height={p.size_mm[1]} fill={done ? partColor(p.footprint) : 'none'} stroke={done ? 'var(--text)' : 'var(--copper)'} strokeWidth={0.08} strokeDasharray={done ? undefined : '0.3 0.2'} />
                      </g>
                    );
                  })}
                </g>
                {d.placements.map((p) => (
                  <text key={p.refdes} x={p.board_xy[0]} y={-p.board_xy[1] - Math.max(p.size_mm[1] / 2, 0.4) - 0.3} fontSize={1.1} textAnchor="middle" fill={p.t_placed <= clock.t ? 'var(--text)' : 'var(--muted)'}>{p.refdes}</text>
                ))}
              </svg>
            </div>
          )}
          <div className="stack" style={{ gap: 6, marginTop: 8 }}>
            <div style={{ height: 8, borderRadius: 6, background: 'var(--surface-3)', overflow: 'hidden' }} role="progressbar" aria-valuenow={placed} aria-valuemax={d?.placements.length ?? 0}>
              <div style={{ width: `${d ? (100 * placed) / d.placements.length : 0}%`, height: '100%', background: 'var(--accent)', transition: 'width 0.2s' }} />
            </div>
            <PlayBar clock={clock} speeds={[0.25, 0.5, 1, 2, 4, 8]} label={(t) => `${t.toFixed(1)} / ${total.toFixed(1)} s`} />
          </div>
        </Card>
      </div>

      {d && (
        <Card title="Where the cycle time goes" subtitle="Every event of the board cycle; the marker follows the animation">
          <div style={{ display: 'flex', height: 26, borderRadius: 7, overflow: 'hidden', gap: 2, marginBottom: 6 }}>
            {bdRows.map((r) => (
              <div key={r.id} title={`${r.label}: ${r.v.toFixed(2)} s`} style={{ width: `${(100 * r.v) / total}%`, background: r.color, minWidth: r.v > 0 ? 2 : 0 }} />
            ))}
          </div>
          <div className="plot-legend" style={{ paddingLeft: 0, marginBottom: 10 }}>
            {bdRows.map((r) => <span className="lg" key={r.id}><span className="lg-swatch" style={{ background: r.color }} />{r.label} <b className="tnum">{r.v.toFixed(1)} s</b> ({((100 * r.v) / total).toFixed(0)} %)</span>)}
          </div>
          <svg viewBox={`0 0 1000 ${KINDS.length * 16 + 22}`} preserveAspectRatio="none" style={{ width: '100%', height: KINDS.length * 16 + 22, display: 'block' }} role="img" aria-label="Event timeline">
            {KINDS.map((k, row) => (
              <g key={k.id}>
                <rect x={0} y={row * 16} width={1000} height={14} fill="var(--surface-2)" />
                {d.events.filter((e) => e.kind === k.id).map((e, j) => (
                  <rect key={j} x={(1000 * e.t0) / total} y={row * 16 + 1} width={Math.max(0.8, (1000 * (e.t1 - e.t0)) / total)} height={12} fill={k.color} />
                ))}
              </g>
            ))}
            <line x1={(1000 * clock.t) / total} x2={(1000 * clock.t) / total} y1={0} y2={KINDS.length * 16} stroke="var(--text)" strokeWidth={2} vectorEffect="non-scaling-stroke" />
            <line x1={(1000 * limit) / total} x2={(1000 * limit) / total} y1={0} y2={KINDS.length * 16 + 22} stroke="var(--bad)" strokeWidth={1.5} strokeDasharray="6 4" vectorEffect="non-scaling-stroke" />
          </svg>
          <div className="row small muted" style={{ justifyContent: 'space-between' }}><span>0 s</span><span>rows: {KINDS.map((k) => k.label).join(' · ')}</span><span>{total.toFixed(1)} s</span></div>
        </Card>
      )}

      <div className="grid grid-2">
        <Card title="Feeder bank">
          <div className="table-wrap" style={{ maxHeight: 300 }}>
            <table className="table">
              <thead><tr><th>Slot</th><th>Part</th><th>Refdes</th></tr></thead>
              <tbody>
                {d?.feeders.map((f) => (
                  <tr key={f.slot}><td className="mono">{f.slot}</td><td>{f.key}</td><td className="mono">{f.refdes.join(', ')}</td></tr>
                ))}
              </tbody>
            </table>
          </div>
        </Card>
        <Explainer title="How the cycle is planned and timed" refs={[{ label: 'Biagiotti & Melchiorri, Trajectory Planning (2008)' }, { label: 'Siciliano et al., Robotics, ch. 4' }]}>
          <p>One feeder per unique part (same value + footprint). Feeders are ordered along the rail by where their parts land on the board, then placements are sequenced greedily (cheapest next pick-and-place in <i>time</i>, not distance) and improved by pairwise swaps. Every XY move is a synchronised joint-space trapezoid of the two SCARA arms with the nozzle rotation overlapped; Z moves are separate short trapezoids.</p>
          <Formula>{`per part = travel(prev→feeder) + 2·Z + t_pick + travel(feeder→camera) + t_vision
         + travel(camera→pad) + 2·Z + t_place
cycle    = Σ parts + load/unload + fiducials (+ flip if any bottom-side parts)`}</Formula>
          <p>The placement comes from <span className="mono">pcb::placement()</span>, so this cycle time updates when the PCB Studio moves parts. Tiny 0402 passives dominate the count — which is why real lines use multi-nozzle heads and why the per-part fixed costs (Z, vacuum, vision) matter as much as travel.</p>
        </Explainer>
      </div>
    </div>
  );
}
