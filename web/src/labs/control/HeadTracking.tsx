import { useMemo, useState } from 'react';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat, StatusChip, Toggle } from '../../components/ui';
import { Plot, type Series } from '../../components/Plot';
import { HeadModel, MiniScene } from './HeadModel';
import { PlayBar, lerpAt, toCSV, useClock, useCssColors, useIdleCompute, fmt } from './kit';
import type { HeadResult } from './types';

const EST_COLORS: Record<string, string> = { gyro: 'var(--series-2)', accel: 'var(--series-3)', cf: 'var(--series-1)', kf: 'var(--series-4)' };
const EST_VARS: Record<string, string> = { gyro: '--series-2', accel: '--series-3', cf: '--series-1', kf: '--series-4' };

export function HeadTracking() {
  const imu = useProject((s) => s.project.design.imu);
  const patch = useProject((s) => s.patchDesign);
  const req = useProject((s) => s.project.requirements.find((r) => r.verification.metric === 'control.head_rms_err_deg'));
  const limit = req?.verification.target ?? 2;
  const [kfMotion, setKfMotion] = useState(2);
  const [ghost, setGhost] = useState<'cf' | 'kf' | 'gyro' | 'accel'>('cf');
  const [shown, setShown] = useState<Record<string, boolean>>({ gyro: true, accel: true, cf: true, kf: true });
  const res = useCompute<HeadResult>('control.head_tracking', { imu, kf_motion_deg: kfMotion, sweep: false, out_hz: 50 });
  const d = res.data;
  // the α sweep (16 full simulations) runs when the inputs settle, and ignores α itself
  const sweepRes = useIdleCompute<HeadResult>('control.head_tracking', { imu: { ...imu, fusion_alpha: 0.98 }, sweep: true, out_hz: 5 });
  const sweep = useMemo(() => sweepRes.data?.sweep ?? [], [sweepRes.data]);
  const clock = useClock(60, { speed: 1 });
  const colors = useCssColors(['--series-1', '--series-2', '--series-3', '--series-4', '--text-2', '--accent'] as const);

  const truthAt = (t: number): [number, number, number] =>
    d ? [lerpAt(d.t, d.true_roll, t), lerpAt(d.t, d.true_pitch, t), lerpAt(d.t, d.true_yaw, t)] : [0, 0, 0];
  const estAt = (t: number): [number, number, number] | undefined => {
    const e = d?.estimates.find((x) => x.id === ghost);
    if (!e || !d) return undefined;
    const yaw = e.has_yaw ? lerpAt(d.t, e.yaw.map((v) => v ?? 0), t) : lerpAt(d.t, d.true_yaw, t);
    return [lerpAt(d.t, e.roll, t), lerpAt(d.t, e.pitch, t), yaw];
  };

  const axisSeries = (axis: 'roll' | 'pitch' | 'yaw'): Series[] => {
    if (!d) return [];
    const truth = axis === 'roll' ? d.true_roll : axis === 'pitch' ? d.true_pitch : d.true_yaw;
    const s: Series[] = [{ name: 'Truth', x: d.t, y: truth, color: 'var(--text)', width: 2.4 }];
    for (const e of d.estimates) {
      if (!shown[e.id]) continue;
      if (axis === 'yaw' && !e.has_yaw) continue;
      const y = axis === 'roll' ? e.roll : axis === 'pitch' ? e.pitch : e.yaw.map((v) => v ?? NaN);
      s.push({ name: e.name, x: d.t, y, color: EST_COLORS[e.id], width: e.id === 'accel' ? 1 : 1.8 });
    }
    return s;
  };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const pitchS = useMemo(() => axisSeries('pitch'), [d, shown]);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const rollS = useMemo(() => axisSeries('roll'), [d, shown]);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const yawS = useMemo(() => axisSeries('yaw'), [d, shown]);
  const errS = useMemo<Series[]>(() => {
    if (!d) return [];
    return d.estimates.filter((e) => e.id === 'cf' || e.id === 'kf').map((e) => ({
      name: `${e.name} |pitch err|`,
      x: d.t,
      y: e.pitch.map((v, i) => Math.abs(v - d.true_pitch[i])),
      color: EST_COLORS[e.id],
      width: 1.5,
    }));
  }, [d]);
  const sweepS = useMemo<Series[]>(() => [{ name: 'CF tilt RMS', x: sweep.map((p) => p[0]), y: sweep.map((p) => p[1]), color: 'var(--series-1)' }, { name: 'sweep points', x: sweep.map((p) => p[0]), y: sweep.map((p) => p[1]), color: 'var(--series-1)', scatter: true }], [sweep]);
  const bands = useMemo(() => (d ? d.events.filter((e) => e.label === 'walking' || e.label === 'glance down' || e.label === 'tilt gesture').map((e) => ({ from: e.t0, to: e.t1, label: e.label })) : []), [d]);

  const cf = d?.estimates.find((e) => e.id === 'cf');
  const now = [{ value: clock.t, color: 'var(--accent)' }];
  const csv = () => {
    if (!d) return '';
    const cols: number[][] = [d.t, d.true_roll, d.true_pitch, d.true_yaw];
    const head = ['t_s', 'true_roll_deg', 'true_pitch_deg', 'true_yaw_deg'];
    for (const e of d.estimates) {
      head.push(`${e.id}_roll_deg`, `${e.id}_pitch_deg`, `${e.id}_yaw_deg`);
      cols.push(e.roll, e.pitch, e.yaw.map((v) => v ?? NaN));
    }
    return toCSV(head, cols);
  };
  const eventNow = d?.events.find((e) => clock.t >= e.t0 && clock.t <= e.t1)?.label ?? 'standing';
  const tr = truthAt(clock.t);
  const es = estAt(clock.t);

  return (
    <div className="stack">
      <div className="grid grid-4">
        <Stat label="CF tilt error (RMS)" value={fmt(cf?.rms_tilt, 2)} unit="°" tone={cf ? (cf.rms_tilt <= limit ? 'pass' : 'fail') : undefined} note={<span>{cf && <StatusChip status={cf.rms_tilt <= limit ? 'pass' : 'fail'} />} requirement ≤ {limit}° (R-011)</span>} />
        <Stat label="Kalman tilt error (RMS)" value={fmt(d?.estimates.find((e) => e.id === 'kf')?.rms_tilt, 2)} unit="°" note={`bias estimate ${fmt(d?.kf_bias_est[1], 2)} °/s (true ∓${imu.gyro_bias_dps})`} />
        <Stat label="Gyro-only yaw drift" value={fmt(d?.estimates.find((e) => e.id === 'gyro')?.final_yaw_err, 1)} unit="° / 60 s" note="yaw is unobservable from gravity" />
        <Stat label="CF crossover" value={fmt(d?.cf_crossover_hz, 2)} unit="Hz" note={`τ = α·Δt/(1−α) = ${fmt(d?.cf_tau_s, 3)} s`} />
      </div>

      <div className="grid grid-sidebar">
        <Card title="IMU & fusion (writes to the project)" subtitle="design.imu — verification re-runs live">
          <Slider label="IMU sample rate" value={imu.rate_hz} min={25} max={1600} log unit="Hz" onChange={(v) => patch('imu', { rate_hz: Math.round(v) })} />
          <Slider label="Gyro noise density" value={imu.gyro_noise_dps_rthz} min={0.001} max={0.1} log unit="°/s/√Hz" onChange={(v) => patch('imu', { gyro_noise_dps_rthz: v })} />
          <Slider label="Gyro bias" value={imu.gyro_bias_dps} min={0} max={5} step={0.05} unit="°/s" onChange={(v) => patch('imu', { gyro_bias_dps: v })} />
          <Slider label="Accel noise density" value={imu.accel_noise_mg_rthz} min={0.02} max={2} log unit="mg/√Hz" onChange={(v) => patch('imu', { accel_noise_mg_rthz: v })} />
          <Slider label="Complementary α (gyro weight)" value={imu.fusion_alpha} min={0.5} max={0.999} step={0.001} format={(v) => v.toFixed(3)} onChange={(v) => patch('imu', { fusion_alpha: v })} />
          <Slider label="Kalman motion allowance σ_m" value={kfMotion} min={0.1} max={10} log unit="°" hint="Extra accelerometer-angle noise assumed for walking/turning accelerations" onChange={setKfMotion} />
          <div className="muted small">Per-sample σ: gyro {fmt(d?.gyro_sigma_dps, 3)} °/s, accel {fmt(d?.accel_sigma_mg, 2)} mg · {d?.samples ?? 0} samples · {res.ms.toFixed(0)} ms</div>
          <ErrorBox error={res.error} />
        </Card>

        <Card
          title="Head pose — truth vs estimate"
          subtitle={<span>Solid head = true pose · ghost frame = {d?.estimates.find((e) => e.id === ghost)?.name ?? ''} · now: <b>{eventNow}</b></span>}
          actions={<Select value={ghost} onChange={(v) => setGhost(v)} options={[{ value: 'cf', label: 'CF' }, { value: 'kf', label: 'Kalman' }, { value: 'gyro', label: 'Gyro' }, { value: 'accel', label: 'Accel' }]} />}
        >
          <MiniScene height={330}>
            <HeadModel truth={tr} est={es} estColor={colors[EST_VARS[ghost] as '--series-1']} accent={colors['--accent']} />
          </MiniScene>
          <div style={{ marginTop: 10 }}>
            <PlayBar clock={clock} />
          </div>
          <div className="row small muted" style={{ marginTop: 6, gap: 16 }}>
            <span>roll {tr[0].toFixed(1)}° / {es?.[0].toFixed(1)}°</span>
            <span>pitch {tr[1].toFixed(1)}° / {es?.[1].toFixed(1)}°</span>
            <span>yaw {tr[2].toFixed(1)}° / {es?.[2].toFixed(1)}°</span>
          </div>
        </Card>
      </div>

      <Card
        title="Pitch: truth vs estimators"
        subtitle="60 s scenario — look around, glance down to read, walk at 2 steps/s, tilt gesture, nods"
        actions={
          <div className="row">
            {(['gyro', 'accel', 'cf', 'kf'] as const).map((id) => (
              <Toggle key={id} label={id.toUpperCase()} value={shown[id]} onChange={(v) => setShown({ ...shown, [id]: v })} />
            ))}
            <DownloadButton filename="iris-head-tracking.csv" content={csv} label="CSV" small />
          </div>
        }
      >
        <Plot series={pitchS} height={260} xLabel="time (s)" yLabel="pitch (°)" vLines={now} xBands={bands} />
      </Card>

      <div className="grid grid-2">
        <Card title="Roll">
          <Plot series={rollS} height={220} xLabel="time (s)" yLabel="roll (°)" vLines={now} legend={false} />
        </Card>
        <Card title="Yaw (no magnetometer)" subtitle="Every estimator integrates the gyro here, so bias makes it drift">
          <Plot series={yawS} height={220} xLabel="time (s)" yLabel="yaw (°)" vLines={now} legend={false} />
        </Card>
        <Card title="Pitch error: complementary vs Kalman" subtitle="Spikes line up with head accelerations (walking start, nods)">
          <Plot series={errS} height={220} xLabel="time (s)" yLabel="|error| (°)" vLines={now} hLines={[{ value: limit, label: `${limit}° requirement (RMS)`, color: 'var(--bad)' }]} />
        </Card>
        <Card title="Choosing α" subtitle={sweepRes.busy ? 'Re-running 16 simulations…' : 'Tilt RMS error of the complementary filter vs its gyro weight (same scenario)'}>
          <Plot series={sweepS} height={220} xLabel="α" yLabel="RMS tilt error (°)" legend={false} vLines={[{ value: imu.fusion_alpha, label: `α = ${imu.fusion_alpha}`, color: 'var(--accent)' }]} hLines={[{ value: limit, label: 'requirement', color: 'var(--bad)' }]} />
        </Card>
      </div>

      <Card title="Estimator scoreboard">
        <div className="table-wrap">
          <table className="table">
            <thead><tr><th>Estimator</th><th>Roll RMS</th><th>Pitch RMS</th><th>Tilt RMS</th><th>Max tilt err</th><th>Yaw after 60 s</th></tr></thead>
            <tbody>
              {d?.estimates.map((e) => (
                <tr key={e.id}>
                  <td><span style={{ display: 'inline-block', width: 10, height: 3, background: EST_COLORS[e.id], marginRight: 8, verticalAlign: 'middle' }} />{e.name}</td>
                  <td className="mono">{fmt(e.rms_roll, 2)}°</td>
                  <td className="mono">{fmt(e.rms_pitch, 2)}°</td>
                  <td className="mono"><b>{fmt(e.rms_tilt, 2)}°</b></td>
                  <td className="mono">{fmt(e.max_tilt_err, 1)}°</td>
                  <td className="mono">{e.has_yaw ? `${fmt(e.final_yaw_err, 1)}°` : 'n/a'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>

      <div className="grid grid-2">
        <Explainer
          title="Why fuse two bad sensors?"
          refs={[
            { label: 'Mahony, Hamel & Pflimlin, IEEE TAC 2008' },
            { label: 'Madgwick et al., IEEE ICORR 2011' },
            { label: 'Åström & Murray, Feedback Systems', href: 'https://fbswiki.org' },
          ]}
        >
          <p>A gyro measures angular <i>rate</i>: integrating it is smooth and fast but any bias b grows into an angle error b·t. An accelerometer measures <i>specific force</i> — gravity plus the head's own acceleration — so its tilt is drift-free but noisy and wrong whenever you walk or nod. The complementary filter trusts each where it is good:</p>
          <Formula>{`θ̂ₖ = α (θ̂ₖ₋₁ + ω Δt) + (1 − α) θ_acc
    = high-pass(gyro) + low-pass(accel),  f_c = (1 − α) / (2π α Δt)`}</Formula>
          <p>Accelerometer tilt: <span className="mono">φ = atan2(−f_y, −f_z)</span>, <span className="mono">θ = atan2(f_x, √(f_y² + f_z²))</span>. A constant gyro bias leaves a steady error of only b·τ (τ = α Δt/(1−α)), while walking accelerations are attenuated above f_c. The α sweep shows the trade-off: too low → accelerometer motion errors leak through; too high → bias and slow correction dominate.</p>
        </Explainer>
        <Explainer title="Kalman filter with bias state" refs={[{ label: 'R. E. Kalman, "A new approach to linear filtering and prediction problems", J. Basic Eng. 1960' }]}>
          <Formula>{`x = [angle, bias]ᵀ      x⁻ = F x + G ω_meas,   F = [[1, −Δt], [0, 1]]
P⁻ = F P Fᵀ + Q          Q = diag(N_g² Δt, N_b² Δt)
K = P⁻Hᵀ (H P⁻ Hᵀ + R)⁻¹,  H = [1 0],  R = σ_acc² + σ_m²
x = x⁻ + K (θ_acc − H x⁻)`}</Formula>
          <p>Because the gyro bias is a state, the filter learns it (see the bias estimate above) and then integrates a nearly unbiased gyro — so it can trust the accelerometer far less (σ_m) and reject walking accelerations. That is why it beats every fixed α. Yaw is <b>unobservable</b>: rotating about the vertical does not change the gravity vector, so without a magnetometer (or camera/GNSS) every heading estimate drifts — Iris only needs pitch/roll for its head-tilt gestures and horizon-locked text.</p>
        </Explainer>
      </div>
      <Explainer title="How this verifies Iris requirement R-011">
        The metric <span className="mono">control.head_rms_err_deg</span> is the complementary filter's RMS pitch/roll error over this deterministic 60 s scenario, using <span className="mono">design.imu</span> (rate, noise densities, bias, α). The IMU sits in the right temple pod, ~13 cm from the neck pivot, so head rotations add lever-arm accelerations too. Per-sample noise is density × √(rate) — raising the rate lowers per-step correction but adds samples. Move the sliders and watch the requirement chip at the top of the page update.
      </Explainer>
    </div>
  );
}
