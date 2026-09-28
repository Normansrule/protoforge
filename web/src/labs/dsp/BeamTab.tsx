import { useMemo, useRef, useState } from 'react';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { Plot, fmtNum } from '../../components/Plot';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat, StatusChip, Toggle } from '../../components/ui';
import { useElementWidth } from '../../components/useElementWidth';
import type { BeamResult, FieldResult, Pattern } from './types';
import { PATTERN_LABEL } from './types';
import { HeatCanvas, KV, PolarPlot, heatColor, useTween, csvOf } from './viz';

// Side view of a head, millimetres, facing +x, y up; the mic array centre sits at the origin on the right temple.
const XMIN = -170, XMAX = 150, YMIN = -150, YMAX = 110;
const MOUTH: [number, number] = [58, -80];
const HEAD =
  'M -150 20 C -158 92 -62 124 0 112 C 52 104 62 70 60 40 L 61 14 C 63 6 59 1 61 -6 L 82 -36 L 66 -46 L 68 -62 L 63 -74 L 66 -86 C 64 -102 60 -112 46 -122 C 20 -132 -20 -122 -40 -96 L -46 -150 L -112 -150 C -126 -96 -152 -40 -150 20 Z';

function mouthAngle(): number {
  return (Math.atan2(MOUTH[1], MOUTH[0]) * 180) / Math.PI;
}

function HeadStage(props: { spacing: number; pattern: Pattern; tau: number; freq: number; tilt: number }) {
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const [src, setSrc] = useState<[number, number]>([-120, 60]);
  const dragging = useRef(false);
  const svgRef = useRef<SVGSVGElement>(null);
  const common = { spacing_mm: props.spacing, pattern: props.pattern, tau_ratio: props.tau, freq: props.freq, tilt_deg: props.tilt, x_min: XMIN, x_max: XMAX, y_min: YMIN, y_max: YMAX };
  const grid = useCompute<FieldResult>('dsp.beam_field', { ...common, nx: 96, ny: 78, probes: [] });
  const probe = useCompute<FieldResult>('dsp.beam_field', { ...common, nx: 4, ny: 4, probes: [src, MOUTH] });
  const W = Math.max(width, 200);
  const H = (W * (YMAX - YMIN)) / (XMAX - XMIN);
  const X = (x: number) => ((x - XMIN) / (XMAX - XMIN)) * W;
  const Y = (y: number) => ((YMAX - y) / (YMAX - YMIN)) * H;
  const toMm = (e: React.PointerEvent): [number, number] => {
    const r = svgRef.current!.getBoundingClientRect();
    const x = XMIN + ((e.clientX - r.left) / r.width) * (XMAX - XMIN);
    const y = YMAX - ((e.clientY - r.top) / r.height) * (YMAX - YMIN);
    return [Math.max(XMIN, Math.min(XMAX, x)), Math.max(YMIN, Math.min(YMAX, y))];
  };
  const sc = W / (XMAX - XMIN);
  const mics = grid.data?.mics ?? probe.data?.mics ?? [];
  const lvl = probe.data?.probe_db ?? [];
  const tf = `translate(${X(0)} ${Y(0)}) scale(${sc} ${-sc})`;
  return (
    <div>
      <div className="beam-stage" ref={ref} style={{ aspectRatio: `${XMAX - XMIN} / ${YMAX - YMIN}` }}>
        {grid.data && <div className="heat"><HeatCanvas nx={grid.data.nx} ny={grid.data.ny} values={grid.data.level_db} min={-30} max={10} flipY /></div>}
        {width > 0 && (
          <svg
            ref={svgRef}
            viewBox={`0 0 ${W} ${H}`}
            onPointerDown={(e) => {
              dragging.current = true;
              (e.target as Element).setPointerCapture?.(e.pointerId);
              setSrc(toMm(e));
            }}
            onPointerMove={(e) => dragging.current && setSrc(toMm(e))}
            onPointerUp={() => (dragging.current = false)}
            role="img"
            aria-label="Head side view with the microphone array; drag the sound source"
          >
            <g transform={tf}>
              <path d={HEAD} fill="var(--surface-1)" fillOpacity={0.35} stroke="var(--text)" strokeOpacity={0.7} strokeWidth={1.6 / sc} />
              <ellipse cx={-96} cy={-6} rx={13} ry={24} fill="none" stroke="var(--text)" strokeOpacity={0.55} strokeWidth={1.4 / sc} />
              {/* glasses: front rim, temple, electronics pod */}
              <rect x={47} y={-16} width={9} height={34} rx={4} fill="var(--text)" fillOpacity={0.85} />
              <path d="M 52 9 L -92 8 C -104 8 -110 -4 -112 -18" fill="none" stroke="var(--text)" strokeWidth={4.5} strokeLinecap="round" opacity={0.9} />
              <rect x={-24} y={1} width={48} height={14} rx={5} fill="var(--copper)" opacity={0.9} />
              {mics.map(([x, y], i) => <circle key={i} cx={x} cy={y} r={3.2} fill={i === 0 ? 'var(--series-1)' : 'var(--series-2)'} stroke="var(--surface-1)" strokeWidth={0.8} />)}
              <line x1={0} y1={0} x2={120 * Math.cos((-props.tilt * Math.PI) / 180)} y2={120 * Math.sin((-props.tilt * Math.PI) / 180)} stroke="var(--series-1)" strokeDasharray="5 4" strokeWidth={1.4 / sc} />
              <circle cx={MOUTH[0]} cy={MOUTH[1]} r={5} fill="none" stroke="var(--good-text)" strokeWidth={2 / sc} />
            </g>
            {/* source */}
            <g className="drag-handle">
              <circle className="ripple" cx={X(src[0])} cy={Y(src[1])} r={4} fill="none" stroke="var(--warn)" strokeWidth={2} />
              <circle cx={X(src[0])} cy={Y(src[1])} r={11} fill="var(--warn)" stroke="var(--surface-1)" strokeWidth={2.5} />
              <text x={X(src[0])} y={Y(src[1]) + 4} fontSize={11} textAnchor="middle" fill="#000" fontWeight={800}>♪</text>
            </g>
            <text x={X(MOUTH[0]) + 10} y={Y(MOUTH[1]) + 4} fontSize={11.5} fill="var(--good-text)" fontWeight={700}>mouth</text>
            <text x={X(src[0]) + 15} y={Y(src[1]) - 12} fontSize={12} fill="var(--text)" fontWeight={700} style={{ paintOrder: 'stroke', stroke: 'var(--surface-1)', strokeWidth: 3 }}>
              {lvl[0] !== undefined ? `${lvl[0] >= 0 ? '+' : ''}${lvl[0].toFixed(1)} dB` : ''}
            </text>
          </svg>
        )}
      </div>
      <div className="beam-legend" style={{ marginTop: 8 }}>
        <span>−30 dB</span>
        <div className="ramp" style={{ background: `linear-gradient(90deg, ${[0, 0.25, 0.5, 0.75, 1].map(heatColor).join(',')})` }} />
        <span>+10 dB</span>
      </div>
      <div style={{ marginTop: 10 }}>
        <KV k={<>Your source ♪ ({probe.data ? Math.abs(probe.data.probe_angle_deg[0]).toFixed(0) : '—'}° off-axis)</>} v={<b style={{ fontSize: 15 }}>{lvl[0] !== undefined ? `${lvl[0].toFixed(1)} dB` : '—'}</b>} />
        <KV k="Mouth (near field — proximity boost)" v={<b style={{ fontSize: 15, color: 'var(--good-text)' }}>{lvl[1] !== undefined ? `${lvl[1].toFixed(1)} dB` : '—'}</b>} />
        <KV k="Voice-to-interferer advantage" v={<b style={{ fontSize: 15 }}>{lvl[0] !== undefined && lvl[1] !== undefined ? `${(lvl[1] - lvl[0]).toFixed(1)} dB` : '—'}</b>} />
      </div>
      <p className="muted small" style={{ marginTop: 6 }}>
        Levels are after the on-axis EQ, relative to a far-field plane wave arriving on-axis (0 dB) — which is also what a single omni microphone would read everywhere.
        Drag the ♪ anywhere; the map shows the level for a source at each point at {fmtNum(props.freq)} Hz.
      </p>
    </div>
  );
}

const PATTERNS: Pattern[] = ['omni', 'dipole', 'cardioid', 'hypercardioid', 'supercardioid', 'delay_sum', 'custom'];

export function BeamTab() {
  const project = useProject((s) => s.project);
  const patchDesign = useProject((s) => s.patchDesign);
  const spacing = project.design.audio.mic_spacing_mm;
  const [pattern, setPattern] = useState<Pattern>('hypercardioid');
  const [tau, setTau] = useState(0.5);
  const [freq, setFreq] = useState(1000);
  const [tilt, setTilt] = useState(-mouthAngle());
  const [multi, setMulti] = useState(true);
  const r = useCompute<BeamResult>('dsp.beamformer', { spacing_mm: spacing, pattern, tau_ratio: tau, polar_freqs: [freq, 250, 4000, 8000], f_min: 50, f_max: 12000, n_freq: 160, eq_low_hz: project.design.audio.voice_band_lo_hz });
  const d = r.data;
  const target = useMemo(() => (d ? d.polar.map((p) => p.gain_db) : [[]]), [d]);
  const tween = useTween(target);
  const traces = d
    ? (multi ? d.polar : d.polar.slice(0, 1)).map((p, i) => ({ label: `${fmtNum(p.f)} Hz`, angle_deg: p.angle_deg, gain_db: tween[i] ?? p.gain_db, color: i === 0 ? 'var(--series-1)' : `var(--series-${i + 2})` }))
    : [];
  const di = d?.di_1k_db ?? 0;
  const mouthOff = -mouthAngle() - tilt;
  return (
    <div className="stack">
      <div className="grid grid-sidebar">
        <Card title="Two-microphone array" subtitle="Endfire pair on the right temple (MK1 front, MK2 rear)">
          <label className="field">
            <span className="field-top"><span className="field-label">Pattern</span></span>
            <div className="seg" role="radiogroup">
              {PATTERNS.map((p) => (
                <button key={p} role="radio" aria-checked={p === pattern} className={`seg-btn ${p === pattern ? 'on' : ''}`} onClick={() => setPattern(p)}>{PATTERN_LABEL[p]}</button>
              ))}
            </div>
          </label>
          {pattern === 'custom' && <Slider label="Internal delay τ·c/d" value={tau} min={0} max={1.5} step={0.01} onChange={setTau} format={(v) => v.toFixed(2)} />}
          <Slider label="Mic spacing d (project)" value={spacing} min={4} max={40} step={0.5} unit="mm" onChange={(v) => patchDesign('audio', { mic_spacing_mm: v })} />
          <Slider label="Polar-plot frequency" value={freq} min={100} max={10000} log unit="Hz" onChange={setFreq} format={(v) => v.toFixed(0)} />
          <Toggle label="Overlay 250 Hz, 4 kHz, 8 kHz" value={multi} onChange={setMulti} />
          {d && (
            <div style={{ marginTop: 4 }}>
              <KV k="Internal delay τ" v={`${d.tau_us.toFixed(2)} µs (${d.tau_ratio.toFixed(3)}·d/c)`} />
              <KV k="Pattern α + (1−α)cosθ" v={`α = ${d.alpha.toFixed(3)}`} />
              <KV k="Null" v={d.null_deg != null ? `${d.null_deg.toFixed(1)}°` : 'none'} />
              <KV k="DI @ 1 kHz" v={`${d.di_1k_db.toFixed(2)} dB`} />
              <KV k="White-noise gain @ 1 kHz" v={`${d.wng_1k_db.toFixed(1)} dB`} />
              <KV k="Spatial aliasing above" v={`${fmtNum(d.spatial_alias_hz)} Hz (c/2d)`} />
            </div>
          )}
          {d && <div style={{ marginTop: 10 }}><DownloadButton small filename="iris_beamformer.csv" label="DI/WNG CSV" content={() => csvOf({ f_hz: d.freq, di_db: d.di_db, di_das_db: d.di_das_db, wng_db: d.wng_db, onaxis_db: d.onaxis_db, eq_db: d.eq_db })} /></div>}
        </Card>
        <div className="stack">
          <ErrorBox error={r.error} />
          {d && (
            <div className="lab-hero">
              <Stat label="Directivity index @ 1 kHz (R-009)" value={di.toFixed(2)} unit="dB" tone={di >= 4 ? 'pass' : 'fail'} note={<>{di >= 4 ? '✓' : '✕'} requirement ≥ 4 dB</>} />
              <Stat label="Diffuse-noise rejection" value={`${(100 * (1 - 10 ** (-di / 10))).toFixed(0)}`} unit="%" note="of diffuse noise power removed" />
              <Stat label="Mic-noise boost after EQ" value={(-d.wng_1k_db).toFixed(1)} unit="dB" tone={-d.wng_1k_db > 15 ? 'warn' : undefined} note="@ 1 kHz — the price of a small array" />
            </div>
          )}
          <div className="grid grid-2">
            <Card title="Polar pattern" subtitle={`${PATTERN_LABEL[pattern]} · d = ${spacing} mm · normalised to 0 dB on-axis`}>
              <PolarPlot traces={traces} marker={{ angle: Math.abs(mouthOff), label: 'mouth' }} />
              <div className="pill-row" style={{ justifyContent: 'center' }}>
                {traces.map((t) => <span key={t.label} className="chip"><span style={{ width: 10, height: 3, background: t.color, borderRadius: 2, display: 'inline-block' }} /> {t.label}</span>)}
              </div>
            </Card>
            <Card title="Where is the mouth?" subtitle="Drag the sound source around the head">
              <Slider label="Array axis tilt (down from the temple)" value={tilt} min={-10} max={90} step={0.5} unit="°" onChange={setTilt} format={(v) => v.toFixed(1)} />
              <div className="pill-row" style={{ marginBottom: 8 }}>
                <button className="btn small" onClick={() => setTilt(-mouthAngle())}>Aim at the mouth</button>
                <button className="btn small" onClick={() => setTilt(0)}>Along the temple</button>
              </div>
              <HeadStage spacing={spacing} pattern={pattern} tau={tau} freq={freq} tilt={tilt} />
            </Card>
          </div>
        </div>
      </div>
      <div className="grid grid-3">
        <Card title="Directivity index vs frequency" subtitle="Numerical integration over the sphere">
          {d && (
            <Plot height={250} series={[{ name: PATTERN_LABEL[pattern], x: d.freq, y: d.di_db, width: 2.6 }, { name: 'delay-and-sum', x: d.freq, y: d.di_das_db, dash: '5 3' }]} xLog xLabel="frequency (Hz)" yLabel="DI (dB)" yDomain={[-1, 8]}
              hLines={[{ value: 4, label: 'R-009 ≥ 4 dB', color: 'var(--bad)' }]} vLines={[{ value: 1000, label: '1 kHz' }]} />
          )}
        </Card>
        <Card title="White-noise gain" subtitle="SNR gain against uncorrelated mic self-noise (0 dB = one mic)">
          {d && (
            <Plot height={250} series={[{ name: PATTERN_LABEL[pattern], x: d.freq, y: d.wng_db, width: 2.6 }, { name: 'delay-and-sum', x: d.freq, y: d.wng_das_db, dash: '5 3' }]} xLog xLabel="frequency (Hz)" yLabel="WNG (dB)" hLines={[{ value: 0 }]} />
          )}
        </Card>
        <Card title="On-axis response & EQ" subtitle="The differential array rises 6 dB/octave; the EQ flattens it above the voice band edge">
          {d && (
            <Plot height={250} series={[{ name: 'raw on-axis', x: d.freq, y: d.onaxis_db }, { name: 'EQ', x: d.freq, y: d.eq_db, dash: '5 3' }, { name: 'equalised', x: d.freq, y: d.equalized_db, width: 2.6 }]} xLog xLabel="frequency (Hz)" yLabel="dB re one mic" yDomain={[-45, 45]} />
          )}
        </Card>
      </div>
      <div className="grid grid-2">
        <Card title="First-order patterns compared" subtitle={`d = ${spacing} mm, 1 kHz, c = 343 m/s`}>
          {d && (
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>Pattern</th><th style={{ textTransform: "none" }}>τ (µs)</th><th>DI ideal (dB)</th><th>DI 1 kHz (dB)</th><th>Null</th><th>F/B (dB)</th><th>WNG (dB)</th><th>R-009</th></tr></thead>
                <tbody>
                  {d.table.map((row) => (
                    <tr key={row.pattern} style={row.pattern === pattern ? { background: 'var(--accent-soft)' } : undefined}>
                      <td>{PATTERN_LABEL[row.pattern]}{row.pattern === 'hypercardioid' ? ' ★' : ''}</td>
                      <td className="mono">{row.tau_us.toFixed(1)}</td>
                      <td className="mono">{row.di_ideal_db.toFixed(2)}</td>
                      <td className="mono">{row.di_1k_db.toFixed(2)}</td>
                      <td className="mono">{row.null_deg != null ? `${row.null_deg.toFixed(1)}°` : '—'}</td>
                      <td className="mono">{row.front_back_db > 90 ? '∞' : row.front_back_db.toFixed(1)}</td>
                      <td className="mono">{row.wng_1k_db.toFixed(1)}</td>
                      <td><StatusChip status={row.di_1k_db >= 4 ? 'pass' : 'fail'} /></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
          <p className="lab-note" style={{ marginTop: 8 }}>
            ★ Iris uses the <b>hypercardioid</b>: among first-order differential arrays it maximises the directivity index — the rejection of diffuse
            (reverberant, street) noise that R-009 asks for. The supercardioid has the best front-to-back ratio instead; the cardioid the deepest rear null.
          </p>
        </Card>
        <div className="stack">
          <Explainer title="Delay-and-subtract" refs={[{ label: 'G. W. Elko, "Superdirectional microphone arrays", in Acoustic Signal Processing for Telecommunication, Kluwer 2000' }, { label: 'J. Benesty & J. Chen, Study and Design of Differential Microphone Arrays, Springer 2013' }]}>
            <Formula>{'H(ω,θ) = 1 − e^(−jω(τ + d·cosθ/c))\n|H| ≈ ω(τ + d/c) · [α + (1 − α)·cosθ],   α = τ / (τ + d/c)\nDF = 3 / (3α² + (1 − α)²)  →  hypercardioid α = ¼: DF = 4 (6.02 dB)'}</Formula>
            Subtracting a delayed copy of the rear microphone from the front one cancels sound from the rear; the internal delay τ steers the
            null. Because the two signals are nearly identical at low frequency, the output shrinks 6 dB/octave — the EQ must boost it back, and
            with it the microphones' self-noise (the white-noise gain).
          </Explainer>
          <Explainer title="Directivity index & white-noise gain">
            <Formula>{'DI = 10·log10( |H(θ₀)|² / ((1/4π)∮|H(θ,φ)|² dΩ) )\nWNG = |wᴴd(θ₀)|² / (wᴴw)'}</Formula>
            DI is how much diffuse noise the array rejects relative to the look direction. Delay-and-sum on a 14 mm array has almost no
            directivity (kd ≪ π) but +3 dB WNG; the differential array buys ~6 dB of DI at the cost of noise gain — a classic trade-off. The
            {' '}{fmtNum(spacing)} mm spacing sets both: widen it and WNG improves, but spatial aliasing (c/2d) moves down.
          </Explainer>
        </div>
      </div>
    </div>
  );
}
