// Quantum key distribution: an animated BB84 photon stream with an optional
// eavesdropper, the decoy-state fiber link model against the PLOB bound, and
// entanglement-based E91 with a CHSH check.
import { useMemo, useState } from 'react';
import { Card, Chip, DownloadButton, Explainer, Formula, Slider, Stat, StatusChip, Toggle } from '../../components/ui';
import { Plot, fmtNum } from '../../components/Plot';
import { call } from '../../engine/engine';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { Bb84Out, E91Out, LinkOut, LinkParams, Pulse } from './types';
import { csv, Gauge, Kv, PlayBar, Seg, usePlayer } from './viz';

// ---------------------------------------------------------------------------
// BB84
// ---------------------------------------------------------------------------

/** Polarisation glyph: + basis 0 → ↔ (H), 1 → ↕ (V); × basis 0 → ⤢ (45°), 1 → ⤡ (135°). */
function Pol({ basis, bit, size = 18, color = 'var(--text)' }: { basis: number; bit: number; size?: number; color?: string }) {
  const ang = basis === 0 ? (bit === 0 ? 0 : 90) : bit === 0 ? 45 : 135;
  const r = size / 2 - 2;
  const rad = (ang * Math.PI) / 180;
  const dx = r * Math.cos(rad), dy = -r * Math.sin(rad);
  return (
    <svg width={size} height={size} viewBox={`${-size / 2} ${-size / 2} ${size} ${size}`} aria-label={`${basis === 0 ? '+' : '×'} basis, bit ${bit}`}>
      <line x1={-dx} y1={-dy} x2={dx} y2={dy} stroke={color} strokeWidth={2.2} strokeLinecap="round" />
      <circle cx={dx} cy={dy} r={1.8} fill={color} />
      <circle cx={-dx} cy={-dy} r={1.8} fill={color} />
    </svg>
  );
}

const B = (b: number | null) => (b == null ? '·' : b === 0 ? '+' : '×');

function Channel({ pulse, t, eve }: { pulse: Pulse | undefined; t: number; eve: boolean }) {
  const W = 640, H = 120, xa = 70, xb = 570, xe = 320, y = 62;
  if (!pulse) return null;
  const x = xa + (xb - xa) * t;
  const afterEve = eve && pulse.eve_basis != null && x > xe;
  const basis = afterEve ? pulse.eve_basis! : pulse.alice_basis;
  const bit = afterEve ? pulse.eve_bit! : pulse.alice_bit;
  const lost = !pulse.detected && t > 0.8;
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="qx-story" role="img" aria-label="photon on the fiber">
      <line x1={xa} x2={xb} y1={y} y2={y} stroke="var(--series-1)" strokeWidth={2} opacity={0.35} />
      {[{ x: xa, t: 'Alice', s: `bit ${pulse.alice_bit}, basis ${B(pulse.alice_basis)}` }, { x: xb, t: 'Bob', s: pulse.bob_bit != null && t > 0.95 ? `basis ${B(pulse.bob_basis)} → ${pulse.bob_bit}` : `basis ${B(pulse.bob_basis)}` }].map((n) => (
        <g key={n.t}>
          <rect x={n.x - 50} y={y - 30} width={100} height={60} rx={10} fill="var(--surface-1)" stroke="var(--border-strong)" />
          <text x={n.x} y={y - 8} textAnchor="middle" fontSize={12} fontWeight={700}>{n.t}</text>
          <text x={n.x} y={y + 10} textAnchor="middle" fontSize={10} style={{ fill: 'var(--text-2)' }}>{n.s}</text>
        </g>
      ))}
      {eve && (
        <g>
          <rect x={xe - 40} y={y - 44} width={80} height={26} rx={8} fill="color-mix(in srgb, var(--bad) 18%, var(--surface-1))" stroke="var(--bad)" />
          <text x={xe} y={y - 27} textAnchor="middle" fontSize={11} fontWeight={700} style={{ fill: 'var(--bad-text)' }}>Eve{pulse.eve_basis != null ? ` (${B(pulse.eve_basis)})` : ' idle'}</text>
        </g>
      )}
      {!lost && (
        <g transform={`translate(${x} ${y})`}>
          <circle r={13} fill="color-mix(in srgb, var(--series-4) 30%, transparent)" />
          <g transform="translate(-9 -9)"><Pol basis={basis} bit={bit} color="var(--series-4)" /></g>
        </g>
      )}
      {lost && <text x={xb - 70} y={y + 44} fontSize={10} style={{ fill: 'var(--muted)' }}>photon lost in the fiber</text>}
    </svg>
  );
}

function Bb84() {
  const [eve, setEve] = useState(0);
  const [noise, setNoise] = useState(0.02);
  const [tr, setTr] = useState(0.7);
  const [pulses, setPulses] = useState(20000);
  const [f, setF] = useState(1.16);
  const [seed, setSeed] = useState(7);
  const r = useCompute<Bb84Out>('quantum.bb84', { pulses, noise, eve, transmittance: tr, ec_efficiency: f, sample_fraction: 0.2, show: 32, seed });
  const o = r.data;
  const n = o?.pulses.length ?? 1;
  const p = usePlayer(n, 1.6, { resetKey: `${eve},${noise},${tr},${seed}` });
  const cur = o?.pulses[p.i];
  return (
    <div className="grid grid-sidebar">
      <Card title="BB84 settings" subtitle="Bennett & Brassard (1984)">
        <Slider label="Eve intercepts (fraction of photons)" value={eve} min={0} max={1} step={0.05} format={(v) => `${Math.round(v * 100)} %`} onChange={setEve} />
        <div className="row" style={{ gap: 6, marginTop: -4, marginBottom: 10 }}>
          <button className={`btn small ${eve === 0 ? 'primary' : ''}`} onClick={() => setEve(0)}>No Eve</button>
          <button className={`btn small ${eve === 1 ? 'primary' : ''}`} onClick={() => setEve(1)}>Eve on every photon</button>
        </div>
        <Slider label="Channel error (misalignment, noise)" value={noise} min={0} max={0.15} step={0.0025} format={(v) => `${(v * 100).toFixed(1)} %`} onChange={setNoise} />
        <Slider label="Transmittance (animation)" value={tr} min={0.05} max={1} step={0.05} format={(v) => `${Math.round(v * 100)} %`} onChange={setTr} />
        <Slider label="Error-correction inefficiency f" value={f} min={1} max={1.5} step={0.01} onChange={setF} hint="1 = Shannon limit; LDPC codes reach ~1.1" />
        <Seg value={pulses} onChange={setPulses} options={[2000, 20000, 200000].map((v) => ({ value: v, label: fmtNum(v) }))} label="Pulses for the statistics" />
        <button className="btn small" onClick={() => setSeed((s) => s + 1)}>New random run</button>
      </Card>
      <div className="stack">
        <Card title="One photon at a time" subtitle="Alice picks a random bit and basis; Bob picks a random basis. Wrong basis → random result.">
          {o && <Channel pulse={cur} t={p.frac} eve={eve > 0} />}
          {o && <PlayBar p={p} label={(i) => `photon ${i + 1} / ${n}`} />}
          {o && <PulseTable pulses={o.pulses} cur={p.i} eve={eve > 0} />}
          <div className="small muted" style={{ marginTop: 4 }}>Green columns survive sifting (same basis, photon detected). Red outline: Bob's bit differs from Alice's — an error the QBER estimate will catch.</div>
        </Card>
        {o && (
          <div className="grid grid-2">
            <Card title="Quantum bit error rate">
              <Gauge value={o.qber * 100} max={30} threshold={o.threshold * 100} label="QBER on sifted bits" marks={[{ v: o.threshold * 100, t: '11 %' }, { v: 25, t: '25 % (Eve)' }]} />
              <div className="row small" style={{ justifyContent: 'center' }}>
                <StatusChip status={o.abort ? 'fail' : 'pass'} />
                <span>{o.abort ? 'abort: no secret key possible' : 'key distillation proceeds'}</span>
              </div>
              <Kv rows={[
                ['Expected QBER (1−e)p + e(p/2 + ¼)', `${(o.qber_expected * 100).toFixed(2)} %`],
                ['Estimated from disclosed sample', `${(o.qber_estimate * 100).toFixed(2)} % (${fmtNum(o.sampled)} bits)`],
              ]} />
            </Card>
            <Card title="From photons to a secret key">
              <div className="qx-stats">
                <Stat label="Sent" value={fmtNum(o.sent)} />
                <Stat label="Detected" value={fmtNum(o.detected)} />
                <Stat label="Sifted" value={fmtNum(o.sifted)} note="≈ ½ of detected" />
                <Stat label="Secret key" value={fmtNum(o.final_key_bits)} unit="bits" tone={o.abort ? 'fail' : 'pass'} />
              </div>
              <Kv rows={[
                ['Leaked by error correction f·h(Q)', `${fmtNum(o.leak_ec_bits)} bits`],
                ['Removed by privacy amplification h(Q)', `${fmtNum(o.pa_removed_bits)} bits`],
                ['Secret fraction r = 1 − f·h(Q) − h(Q)', o.secret_fraction.toFixed(4)],
              ]} />
            </Card>
          </div>
        )}
        {o && (
          <Card title="Secret fraction vs QBER" subtitle="Shor–Preskill asymptotic bound for BB84 with one-way post-processing">
            <Plot
              series={[{ name: `r(Q), f = ${f.toFixed(2)}`, x: o.curve_q.map((q) => q * 100), y: o.curve_r }]}
              vLines={[{ value: o.threshold * 100, label: '11 % (f = 1)', color: 'var(--bad)' }, { value: o.qber * 100, label: 'this run', color: 'var(--accent)' }]}
              xLabel="QBER (%)" yLabel="secret bits per sifted bit" height={200} legend={false} yDomain={[0, 1]}
            />
          </Card>
        )}
      </div>
    </div>
  );
}

function PulseTable({ pulses, cur, eve }: { pulses: Pulse[]; cur: number; eve: boolean }) {
  const rows = ['Alice bit', 'Alice basis', 'photon', ...(eve ? ['Eve basis'] : []), 'Bob basis', 'Bob bit', 'kept'];
  return (
    <div className="row" style={{ alignItems: 'stretch', gap: 6, flexWrap: 'nowrap', marginTop: 8 }}>
      <div className="qx-rowlabels muted">{rows.map((r) => <div key={r} style={{ height: 20, display: 'flex', alignItems: 'center', whiteSpace: 'nowrap' }}>{r}</div>)}</div>
      <div className="qx-pulses" style={{ flex: 1 }}>
        {pulses.map((pl, k) => (
          <div key={k} className={`qx-pulse ${pl.sifted ? 'sifted' : ''} ${pl.error ? 'err' : ''} ${k === cur ? 'cur' : ''}`} style={{ opacity: k <= cur ? 1 : 0.35 }}>
            <div style={{ height: 20 }} className="mono">{pl.alice_bit}</div>
            <div style={{ height: 20 }} className="qx-basis">{B(pl.alice_basis)}</div>
            <div style={{ height: 20 }}><Pol basis={pl.alice_basis} bit={pl.alice_bit} size={16} /></div>
            {eve && <div style={{ height: 20, color: 'var(--bad-text)' }} className="qx-basis">{B(pl.eve_basis)}</div>}
            <div style={{ height: 20 }} className="qx-basis">{B(pl.bob_basis)}</div>
            <div style={{ height: 20 }} className="mono">{pl.bob_bit ?? '–'}</div>
            <div style={{ height: 20 }}>{pl.sifted ? (pl.error ? '✕' : '✓') : ''}</div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Fiber link model
// ---------------------------------------------------------------------------

const DEFAULTS: LinkParams = {
  distance_km: 25, fiber_loss_db_km: 0.2, extra_loss_db: 3, detector_efficiency: 0.2, dark_count_prob: 1e-6, misalignment: 0.015,
  ec_efficiency: 1.16, source_rate_hz: 1e7, mu: -1, sifting: 0.5, dead_time_s: 1e-6,
};
const PRESETS: Record<string, { label: string; p: LinkParams; note: string }> = {
  ingaas: { label: 'Gated InGaAs APD', p: DEFAULTS, note: 'Typical published ranges for thermoelectrically cooled InGaAs/InP single-photon avalanche diodes.' },
  snspd: { label: 'Superconducting nanowire', p: { ...DEFAULTS, detector_efficiency: 0.8, dark_count_prob: 1e-8, dead_time_s: 5e-8, extra_loss_db: 2 }, note: 'Typical published ranges for SNSPDs (cryostat at ~2–3 K).' },
  gys: { label: 'GYS (Lo, Ma & Chen 2005)', p: { distance_km: 50, fiber_loss_db_km: 0.21, extra_loss_db: 0, detector_efficiency: 0.045, dark_count_prob: 8.5e-7, misalignment: 0.033, ec_efficiency: 1.22, source_rate_hz: 2e6, mu: -1, sifting: 0.5, dead_time_s: 0 }, note: 'The Gobby–Yuan–Shields experiment parameters used in the decoy-state paper: max distance ≈ 140 km.' },
};

function FiberLink() {
  const project = useProject((s) => s.project);
  const patchDesign = useProject((s) => s.patchDesign);
  const setProject = useProject((s) => s.setProject);
  const bound = project.design.extra?.qkd as (Partial<LinkParams> & { random_bits_per_pulse?: number }) | undefined;
  const [local, setLocal] = useState<LinkParams>(DEFAULTS);
  const [useProjectParams, setUseProjectParams] = useState(true);
  const linked = !!bound && useProjectParams;
  const params: LinkParams = linked ? { ...DEFAULTS, ...bound } : local;
  const set = (k: keyof LinkParams, v: number) => {
    if (linked) patchDesign('extra', { qkd: { ...bound, [k]: v } });
    else setLocal((p) => ({ ...p, [k]: v }));
  };
  const [maxKm, setMaxKm] = useState(250);
  const r = useCompute<LinkOut>('quantum.qkd_link', { params, max_km: maxKm, points: 126 });
  const o = r.data;
  const pos = (ys: number[]) => ys.map((v) => (v > 0 ? v : NaN));
  const loadTemplate = () => {
    if (!window.confirm('Replace the current project with the Qlink fiber QKD template? (Save your project first from the top bar if you need it.)')) return;
    const ts = call<{ id: string; project: typeof project }[]>('systems.templates', {});
    const t = ts.find((x) => x.id === 'qlink');
    if (t) setProject(structuredClone(t.project));
  };
  const curveCsv = () => (o ? csv(['distance_km', 'decoy_bps', 'gllp_bps', 'plob_bps', 'mu_opt', 'qber_pct'], o.distance_km.map((d, k) => [d.toFixed(2), o.decoy_bps[k], o.gllp_bps[k], o.plob_bps[k], o.mu_opt[k].toFixed(4), o.qber_pct[k].toFixed(3)])) : '');
  return (
    <div className="grid grid-sidebar">
      <Card title="Link parameters" subtitle={linked ? `Bound to ${project.meta.name}: design.extra.qkd — edits re-verify the requirements.` : 'Local what-if values.'}>
        {bound ? (
          <Toggle label={`Edit the project's link (${project.meta.name})`} value={useProjectParams} onChange={setUseProjectParams} />
        ) : (
          <div className="callout small" style={{ marginBottom: 10 }}>
            This project has no quantum link. <button className="btn small" onClick={loadTemplate}>Load the Qlink template</button> — a two-node fiber QKD link whose requirements are verified by this model.
          </div>
        )}
        <div className="row" style={{ gap: 4, marginBottom: 8 }}>
          {Object.entries(PRESETS).map(([k, v]) => (
            <button key={k} className="btn small" title={v.note} onClick={() => { if (linked) patchDesign('extra', { qkd: { ...bound, ...v.p, distance_km: params.distance_km } }); else setLocal(v.p); }}>{v.label}</button>
          ))}
        </div>
        <Slider label="Design distance" value={params.distance_km} min={1} max={200} step={1} unit="km" onChange={(v) => set('distance_km', v)} />
        <Slider label="Fiber loss α" value={params.fiber_loss_db_km} min={0.15} max={0.4} step={0.005} unit="dB/km" onChange={(v) => set('fiber_loss_db_km', v)} hint="≈ 0.2 dB/km for standard single-mode fiber at 1550 nm; ultra-low-loss fiber ≈ 0.16" />
        <Slider label="Fixed loss (connectors, Bob's optics)" value={params.extra_loss_db} min={0} max={10} step={0.1} unit="dB" onChange={(v) => set('extra_loss_db', v)} />
        <Slider label="Detector efficiency η_det" value={params.detector_efficiency} min={0.01} max={0.95} step={0.005} onChange={(v) => set('detector_efficiency', v)} />
        <Slider label="Dark-count probability per gate" value={params.dark_count_prob} min={1e-9} max={1e-4} log onChange={(v) => set('dark_count_prob', v)} format={(v) => v.toExponential(1)} />
        <Slider label="Misalignment error e_d" value={params.misalignment} min={0} max={0.08} step={0.001} format={(v) => `${(v * 100).toFixed(1)} %`} onChange={(v) => set('misalignment', v)} />
        <Slider label="Error-correction inefficiency f" value={params.ec_efficiency} min={1} max={1.5} step={0.01} onChange={(v) => set('ec_efficiency', v)} />
        <Slider label="Source repetition rate" value={params.source_rate_hz} min={1e5} max={2.5e9} log unit="Hz" format={(v) => fmtNum(v)} onChange={(v) => set('source_rate_hz', v)} />
        <Slider label="Detector dead time" value={Math.max(params.dead_time_s, 1e-9)} min={1e-9} max={2e-5} log format={(v) => `${(v * 1e6).toPrecision(2)} µs`} onChange={(v) => set('dead_time_s', v)} />
        <Slider label="Signal μ (0 = optimise)" value={Math.max(0, params.mu)} min={0} max={1.2} step={0.01} format={(v) => (v <= 0 ? 'optimal' : v.toFixed(2))} onChange={(v) => set('mu', v <= 0 ? -1 : v)} />
        <Slider label="Plot range" value={maxKm} min={50} max={400} step={10} unit="km" onChange={setMaxKm} />
      </Card>
      <div className="stack">
        {o && (
          <div className="qx-stats">
            <Stat label="Key rate @ design" value={fmtNum(o.design.key_rate_bps)} unit="bit/s" note={`${o.params.distance_km.toFixed(0)} km, μ = ${o.design.mu.toFixed(2)}`} />
            <Stat label="QBER @ design" value={(o.design.qber * 100).toFixed(2)} unit="%" tone={o.design.qber < 0.11 ? 'pass' : 'fail'} />
            <Stat label="Max distance" value={o.max_distance_km.toFixed(0)} unit="km" note={`no decoys: ${o.max_distance_gllp_km.toFixed(0)} km`} />
            <Stat label="Channel loss" value={o.loss_db.toFixed(1)} unit="dB" note={`η = ${o.design.eta.toExponential(2)}`} />
            <Stat label="Share of PLOB" value={(o.plob_ratio * 100).toFixed(2)} unit="%" note={`bound ${fmtNum(o.design_plob_bps)} bit/s`} />
          </div>
        )}
        {o && (
          <Card title="Secret key rate vs fiber length" actions={<DownloadButton small filename="qkd-key-rate.csv" content={curveCsv} label="curve.csv" />}>
            <Plot
              series={[
                { name: 'decoy-state BB84 (Lo–Ma–Chen 2005)', x: o.distance_km, y: pos(o.decoy_bps), width: 2.4 },
                { name: 'BB84 without decoys (GLLP)', x: o.distance_km, y: pos(o.gllp_bps), dash: '5 3' },
                { name: 'PLOB repeaterless bound', x: o.distance_km, y: pos(o.plob_bps), dash: '2 3' },
              ]}
              yLog
              vLines={[{ value: o.params.distance_km, label: 'design', color: 'var(--accent)' }, { value: o.max_distance_km, label: `max ${o.max_distance_km.toFixed(0)} km`, color: 'var(--bad)' }]}
              xLabel="distance (km)" yLabel="secret key rate (bit/s)" height={300} legend
            />
            <div className="small muted">Every curve falls ~0.2 dB/km × distance on this log axis until dark counts dominate and the decoy rate crashes to zero. The PLOB line is the most any point-to-point protocol can do; beating it needs quantum repeaters (entanglement swapping in the Protocols tab).</div>
          </Card>
        )}
        {o && (
          <div className="grid grid-2">
            <Card title="QBER vs distance">
              <Plot series={[{ name: 'E_μ', x: o.distance_km, y: o.qber_pct.map((q, k) => (o.decoy_bps[k] > 0 ? q : NaN)) }]} hLines={[{ value: 11, label: '11 %', color: 'var(--bad)' }]} xLabel="km" yLabel="QBER (%)" height={190} legend={false} />
            </Card>
            <Card title="Optimal signal intensity μ">
              <Plot series={[{ name: 'μ', x: o.distance_km, y: o.mu_opt.map((m, k) => (o.decoy_bps[k] > 0 ? m : NaN)) }]} xLabel="km" yLabel="mean photons / pulse" height={190} legend={false} />
            </Card>
          </div>
        )}
        <Explainer title="The decoy-state key rate" refs={[{ label: 'Lo, Ma & Chen, PRL 94, 230504 (2005)' }, { label: 'Gottesman, Lo, Lütkenhaus & Preskill, QIC 4, 325 (2004)' }, { label: 'Pirandola et al., Nat. Commun. 8, 15043 (2017)' }]}>
          A laser pulse attenuated to μ photons on average sometimes carries two; Eve could split one off (photon-number splitting). Decoy intensities let Alice and Bob measure the single-photon yield Y₁ and error e₁ and keep only the provably secret part:
          <Formula>{'η = η_det·10^(−(αL + L_fixed)/10)     Q_μ = Y₀ + 1 − e^(−ημ)\nE_μ Q_μ = e₀Y₀ + e_d(1 − e^(−ημ))    Q₁ = (Y₀ + η) μ e^(−μ)    e₁ = (e₀Y₀ + e_d η)/(Y₀ + η)\nR = q { −Q_μ f h(E_μ) + Q₁ [1 − h(e₁)] }   ×  pulse rate\nPLOB:  R ≤ −log₂(1 − η_fiber)  per channel use'}</Formula>
          Here Y₀ = 2 × dark-count probability (two detectors), q = ½ for standard sifting, and h is the binary entropy. Asymptotic (infinitely long keys, infinitely many decoys); finite-key corrections lower the rate, especially for short blocks. Detector dead time τ caps detections at 1/τ (non-paralysable approximation).
        </Explainer>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// E91
// ---------------------------------------------------------------------------

function E91() {
  const [vis, setVis] = useState(0.97);
  const [eve, setEve] = useState(0);
  const [pairs, setPairs] = useState(20000);
  const [seed, setSeed] = useState(5);
  const r = useCompute<E91Out>('quantum.e91', { pairs, visibility: vis, eve, show: 40, seed });
  const o = r.data;
  const p = usePlayer(o?.pairs.length ?? 1, 2.5, { resetKey: `${vis},${eve},${seed}` });
  const angs = useMemo(() => ({ a: [0, 45, 90], b: [45, 90, 135] }), []);
  return (
    <div className="grid grid-sidebar">
      <Card title="E91 settings" subtitle="Ekert (1991): security certified by a Bell test">
        <Slider label="Source visibility V" value={vis} min={0.5} max={1} step={0.005} onChange={setVis} hint="Werner state; QBER on key bits ≈ (1 − V)/2" />
        <Slider label="Eve intercepts Bob's photons" value={eve} min={0} max={1} step={0.05} format={(v) => `${Math.round(v * 100)} %`} onChange={setEve} />
        <Seg value={pairs} onChange={setPairs} options={[2000, 20000, 200000].map((v) => ({ value: v, label: fmtNum(v) }))} label="Pairs" />
        <button className="btn small" onClick={() => setSeed((s) => s + 1)}>New random run</button>
        <svg viewBox="0 0 220 120" style={{ width: '100%', maxWidth: 260, marginTop: 10 }} role="img" aria-label="measurement angles">
          {(['a', 'b'] as const).map((side, si) => (
            <g key={side} transform={`translate(${si ? 165 : 55} 70)`}>
              <circle r={42} fill="none" stroke="var(--border-strong)" />
              {angs[side].map((d, k) => {
                const rad = (d * Math.PI) / 180;
                return (
                  <g key={d}>
                    <line x1={0} y1={0} x2={40 * Math.sin(rad)} y2={-40 * Math.cos(rad)} stroke={`var(--series-${k + 1})`} strokeWidth={2} />
                    <text x={50 * Math.sin(rad)} y={-50 * Math.cos(rad) + 3} fontSize={9} textAnchor="middle" style={{ fill: 'var(--text-2)' }}>{d}°</text>
                  </g>
                );
              })}
              <text y={58} fontSize={11} fontWeight={700} textAnchor="middle">{si ? 'Bob' : 'Alice'}</text>
            </g>
          ))}
        </svg>
      </Card>
      {o && (
        <div className="stack">
          <div className="grid grid-2">
            <Card title="Bell test on the discarded settings">
              <Gauge value={Math.abs(o.s)} max={3} threshold={2} unit="" label="|S| (CHSH)" marks={[{ v: 2, t: '2' }, { v: 2 * Math.SQRT2, t: '2√2' }]} />
              <div className="row small" style={{ justifyContent: 'center' }}>
                <StatusChip status={o.secure ? 'pass' : 'fail'} />
                <span>{o.secure ? 'entanglement certified, key usable' : 'no Bell violation or QBER too high: abort'}</span>
              </div>
              <Kv rows={[['Expected S', o.s_expected.toFixed(3)], ['Key bits (matching angles)', fmtNum(o.key_bits)], ['QBER on key bits', `${(o.qber * 100).toFixed(2)} %`], ['Secret fraction', o.secret_fraction.toFixed(3)]]} />
            </Card>
            <Card title="Correlations E(a, b)" subtitle="Rows: Alice 0°, 45°, 90°. Columns: Bob 45°, 90°, 135°. Key from 45°/45° and 90°/90° (E ≈ 1).">
              <div className="qx-cmat" style={{ gridTemplateColumns: '44px repeat(3, 1fr)' }}>
                <span />
                {o.bob_deg.map((b) => <span key={b} className="small muted" style={{ textAlign: 'center' }}>{b}°</span>)}
                {o.alice_deg.map((a, i) => (
                  <div key={a} style={{ display: 'contents' }}>
                    <span className="small muted">{a}°</span>
                    {o.bob_deg.map((b, j) => {
                      const v = o.correlations[i * 3 + j];
                      const key = (i === 1 && j === 0) || (i === 2 && j === 1);
                      return (
                        <span key={b} className="qx-cm" style={{ height: 34, fontSize: 12, fontWeight: key ? 800 : 500, background: `color-mix(in srgb, ${v >= 0 ? 'var(--series-1)' : 'var(--series-2)'} ${Math.round(Math.abs(v) * 70)}%, var(--surface-2))` }}>
                          {v.toFixed(2)}{key ? ' · key' : ''}
                        </span>
                      );
                    })}
                  </div>
                ))}
              </div>
            </Card>
          </div>
          <Card title="Pairs as they arrive">
            <PlayBar p={p} label={(i) => `pair ${i + 1}`} />
            <div className="qx-pulses" style={{ marginTop: 8 }}>
              {o.pairs.map((pr, k) => (
                <div key={k} className={`qx-pulse ${pr.key ? 'sifted' : ''} ${pr.key && pr.x !== pr.y ? 'err' : ''} ${k === p.i ? 'cur' : ''}`} style={{ opacity: k <= p.i ? 1 : 0.3 }}>
                  <span className="muted">{o.alice_deg[pr.a]}°</span>
                  <span className="mono">{pr.x > 0 ? '+' : '−'}</span>
                  <span className="mono">{pr.y > 0 ? '+' : '−'}</span>
                  <span className="muted">{o.bob_deg[pr.b]}°</span>
                  {pr.eve && <span style={{ color: 'var(--bad-text)', fontSize: 10 }}>E</span>}
                </div>
              ))}
            </div>
            <div className="small muted">Top to bottom: Alice's angle, her ±1 result, Bob's result, his angle. Green: matching angles → key bit. After the run both announce their angles (never the results) for the discarded pairs to compute S.</div>
          </Card>
        </div>
      )}
    </div>
  );
}

export default function Qkd() {
  const [which, setWhich] = useState<'bb84' | 'link' | 'e91'>('bb84');
  return (
    <div className="stack">
      <Seg value={which} onChange={setWhich} options={[{ value: 'bb84', label: 'BB84 photon stream' }, { value: 'link', label: 'Fiber link: key rate vs distance' }, { value: 'e91', label: 'E91 (entanglement-based)' }]} />
      {which === 'bb84' && <Bb84 />}
      {which === 'link' && <FiberLink />}
      {which === 'e91' && <E91 />}
      <div className="row small">
        <Chip>Graduate to: NetSquid and SeQUeNCe (network simulators), QuTiP (open systems), Qiskit/Cirq (circuits)</Chip>
      </div>
    </div>
  );
}
