import { useMemo, useRef, useState } from 'react';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { Plot, fmtNum } from '../../components/Plot';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat, Toggle } from '../../components/ui';
import { useElementWidth } from '../../components/useElementWidth';
import type { ChainSim } from './types';
import { cHeader } from './FilterTab';
import { KV, Seg, useFrame, useReducedMotion } from './viz';

const ALIAS = 'color-mix(in srgb, var(--bad) 13%, transparent)';

function BitStrip(props: { bits: number[]; input: number[]; playing: boolean; period: number }) {
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const reduced = useReducedMotion();
  const [off, setOff] = useState(0);
  const offRef = useRef(0);
  const n = props.bits.length;
  const visible = Math.max(32, Math.min(n - 1, Math.round(props.period)));
  useFrame(props.playing && !reduced && n > visible, (dt) => {
    offRef.current = (offRef.current + dt * 0.12) % (n - visible);
    setOff(offRef.current);
  });
  const start = Math.floor(off);
  const W = Math.max(width, 100);
  const H = 140;
  const bw = W / visible;
  const mid = H / 2;
  const amp = H * 0.4;
  const win = Math.max(8, Math.round(visible / 24));
  const pts = Math.min(visible, 400);
  const dens: number[] = [];
  const inp: number[] = [];
  const xs: number[] = [];
  for (let q = 0; q < pts; q++) {
    const i = Math.floor((q / (pts - 1)) * (visible - 1));
    const k0 = Math.max(0, start + i - win);
    const k1 = Math.min(n, start + i + win);
    let s = 0;
    for (let k = k0; k < k1; k++) s += props.bits[k] ? 1 : -1;
    dens.push(s / (k1 - k0));
    inp.push(props.input[start + i] ?? 0);
    xs.push(i * bw + bw / 2);
  }
  const path = (v: number[]) => v.map((y, i) => `${i ? 'L' : 'M'}${xs[i].toFixed(1)},${(mid - y * amp).toFixed(1)}`).join('');
  // bars: one path per level keeps the DOM small even for ~1000 bits
  let ones = '';
  let zeros = '';
  for (let i = 0; i < visible; i++) {
    const x = (i * bw).toFixed(2);
    const w = Math.max(0.6, bw * 0.8).toFixed(2);
    if (props.bits[start + i]) ones += `M${x},${mid - amp}h${w}V${mid}h-${w}Z`;
    else zeros += `M${x},${mid}h${w}V${mid + amp}h-${w}Z`;
  }
  return (
    <div className="bitstrip" ref={ref} style={{ height: H }}>
      {width > 0 && (
        <svg width={W} height={H} role="img" aria-label="PDM bitstream over one period of the test tone">
          <path d={ones} fill="var(--series-1)" opacity={0.75} />
          <path d={zeros} fill="var(--series-2)" opacity={0.5} />
          <line x1={0} x2={W} y1={mid} y2={mid} stroke="var(--border-strong)" />
          <path d={path(dens)} fill="none" stroke="var(--text)" strokeWidth={2.2} strokeDasharray="5 3" />
          <path d={path(inp)} fill="none" stroke="var(--copper)" strokeWidth={2.6} />
          <text x={8} y={16} fontSize={11} fill="var(--text-2)" style={{ paintOrder: 'stroke', stroke: 'var(--surface-2)', strokeWidth: 3 }}>bit 1 → +1</text>
          <text x={8} y={H - 8} fontSize={11} fill="var(--text-2)" style={{ paintOrder: 'stroke', stroke: 'var(--surface-2)', strokeWidth: 3 }}>bit 0 → −1</text>
          <text x={W - 8} y={16} fontSize={11} fill="var(--copper)" textAnchor="end" style={{ paintOrder: 'stroke', stroke: 'var(--surface-2)', strokeWidth: 3 }}>analogue input</text>
          <text x={W - 8} y={30} fontSize={11} fill="var(--text)" textAnchor="end" style={{ paintOrder: 'stroke', stroke: 'var(--surface-2)', strokeWidth: 3 }}>density of ones (moving average)</text>
        </svg>
      )}
    </div>
  );
}

export function PdmTab() {
  const project = useProject((s) => s.project);
  const patchDesign = useProject((s) => s.patchDesign);
  const audio = project.design.audio;
  const [order, setOrder] = useState(2);
  const [tone, setTone] = useState(1000);
  const [ampl, setAmpl] = useState(0.5);
  const [interf, setInterf] = useState(false);
  const [interfHz, setInterfHz] = useState(28500);
  const [cic, setCic] = useState(0);
  const [hb, setHb] = useState(false);
  const [logF, setLogF] = useState(true);
  const [playing, setPlaying] = useState(true);
  const r = useCompute<ChainSim>('dsp.pdm_chain', {
    project,
    chain: { cic_order: cic, halfband_taps: hb ? 11 : 0 },
    sim: { modulator_order: order, tone_hz: tone, amplitude: ampl, n_out: 512, interferer_hz: interf ? interfHz : 0, interferer_amp: 0.2, strip_bits: Math.min(4096, Math.round((2.2 * audio.pdm_clock_hz) / tone)) },
  });
  const d = r.data;
  const nyqOut = d ? d.design.fs_out / 2 : 8000;
  const bands = useMemo(() => (d ? d.alias_bands.map(([a, b], i) => ({ from: a, to: b, color: ALIAS, label: i === 0 ? 'aliases into 0–4 kHz' : undefined })) : []), [d]);
  const pass = d && d.stopband_db >= 60;
  return (
    <div className="stack">
      <div className="grid grid-sidebar">
        <Card title="Iris microphone chain" subtitle={`PDM ${fmtNum(audio.pdm_clock_hz)} Hz → ↓${audio.decimation} → ${fmtNum(audio.sample_rate_hz)} Hz PCM`}>
          <Seg label="Sigma-delta modulator order" value={order} options={[{ value: 1, label: '1st order' }, { value: 2, label: '2nd order' }]} onChange={setOrder} />
          <Slider label="Test tone" value={tone} min={100} max={4000} log unit="Hz" onChange={setTone} format={(v) => v.toFixed(0)} />
          <Slider label="Amplitude (re full scale)" value={ampl} min={0.02} max={0.7} onChange={setAmpl} format={(v) => `${v.toFixed(2)} (${(20 * Math.log10(v)).toFixed(1)} dBFS)`} />
          <Toggle label="Add an ultrasonic interferer (−14 dBFS)" value={interf} onChange={setInterf} />
          {interf && <Slider label="Interferer frequency" value={interfHz} min={9000} max={200000} log unit="Hz" onChange={setInterfHz} format={(v) => fmtNum(v)} />}
          <Seg label="CIC order N" value={cic} options={[{ value: 0, label: 'auto' }, ...[2, 3, 4, 5, 6].map((n) => ({ value: n, label: String(n) }))]} onChange={setCic} />
          <Toggle label="Insert an 11-tap half-band stage" value={hb} onChange={setHb} />
          <Slider label="Final FIR taps (project fir_taps)" value={audio.fir_taps} min={3} max={63} step={2} onChange={(v) => patchDesign('audio', { fir_taps: Math.round(v) | 1 })} format={(v) => v.toFixed(0)} />
          {d && (
            <div style={{ marginTop: 4 }}>
              <KV k="CIC" v={`N=${d.design.cic_order}, R=${d.design.cic_r}, M=${d.design.cic_m}`} />
              <KV k="CIC register width" v={`${d.design.cic_register_bits} bits`} hint="B_in + N·log2(RM) (Hogenauer)" />
              <KV k="CIC DC gain (RM)^N" v={fmtNum(d.design.cic_dc_gain)} />
              {d.design.halfband.length > 0 && <KV k="Half-band" v={`${d.design.halfband.length} taps ↓2`} />}
              <KV k="Final FIR" v={`${d.design.fir.length} taps at ${fmtNum(d.design.fir_in_hz)} Hz ↓${d.design.fir_decim}`} />
              <KV k="Pass / stop edges" v={`${fmtNum(d.design.pass_hz)} / ${fmtNum(d.design.stop_hz)} Hz`} />
              {d.design.notes.map((n) => <p key={n} className="muted small">{n}</p>)}
            </div>
          )}
          <div className="pill-row" style={{ marginTop: 12 }}>
            {d && <DownloadButton small filename="iris_decim_fir.h" label="FIR C header" content={() => cHeader('iris_decim_fir', d.design.fir, null, project.design.chip.coeff_bits)} />}
            {d && <DownloadButton small filename="iris_pdm_chain.json" label="Chain JSON" content={() => JSON.stringify({ design: d.design, stopband_db: d.stopband_db, alias_bands: d.alias_bands }, null, 2)} />}
          </div>
        </Card>
        <div className="stack">
          <ErrorBox error={r.error} />
          {d && (
            <div className="lab-hero">
              <Stat label="Alias attenuation (R-010)" value={d.stopband_db.toFixed(1)} unit="dB" tone={pass ? 'pass' : 'fail'} note={<>{pass ? '✓ meets' : '✕ misses'} ≥ 60 dB · worst at {fmtNum(d.stopband_worst_hz)} Hz</>} />
              <Stat label={`With ${d.coeff_bits}-bit coefficients`} value={d.stopband_quantized_db.toFixed(1)} unit="dB" tone={d.stopband_quantized_db >= 60 ? 'pass' : 'fail'} note="as built on the IRIS-DSP chip" />
              <Stat label="Minimum taps for 60 dB" value={d.min_taps_60db ?? '∞'} note={d.min_taps_60db ? `with this CIC (N=${d.design.cic_order})` : `CIC floor ${d.cic_floor_db.toFixed(1)} dB < 60 — raise N`} tone={d.min_taps_60db ? 'info' : 'fail'} />
              <Stat label="Output SNDR" value={d.sndr_db.toFixed(1)} unit="dB" note={`ENOB ${d.enob.toFixed(1)} bits · ripple ${d.passband_ripple_db.toFixed(3)} dB`} />
            </div>
          )}
          <Card
            title="The 1-bit stream"
            subtitle={`One period of the ${fmtNum(tone)} Hz tone = ${fmtNum(audio.pdm_clock_hz / tone)} bits. Each bit is ±1; the density of ones follows the waveform — decimation filters are, at heart, very good averagers.`}
            actions={<button className="btn small" onClick={() => setPlaying((p) => !p)}>{playing ? '❚❚ Pause' : '▶ Scroll'}</button>}
          >
            {d && <BitStrip bits={d.strip_bits} input={d.strip_input} playing={playing} period={audio.pdm_clock_hz / tone} />}
          </Card>
          <Card title="Noise shaping" subtitle={`PDM spectrum (Welch, Blackman) with the ideal NTF |1 − z⁻¹|^${order} — quantisation noise is pushed far above the voice band`} actions={<Toggle label="Log f" value={logF} onChange={setLogF} />}>
            {d && (
              <Plot
                height={260}
                series={[{ name: 'PDM bitstream', x: d.pdm_spectrum.freq, y: d.pdm_spectrum.db }, { name: `ideal NTF (order ${order})`, x: d.pdm_spectrum.freq, y: d.ntf_db, dash: '6 4', color: 'var(--copper)' }]}
                xLog={logF}
                xDomain={logF ? [500, d.design.pdm_hz / 2] : undefined}
                yDomain={[-160, 5]}
                xLabel="frequency (Hz)"
                yLabel="dBFS per bin"
                xBands={[{ from: audio.voice_band_lo_hz, to: audio.voice_band_hi_hz, label: 'voice' }]}
              />
            )}
          </Card>
        </div>
      </div>
      <div className="grid grid-2">
        <Card title="Decimation filter response" subtitle="Every shaded band folds into 0–4 kHz after decimation to 16 kHz — the chain must crush all of them (R-010)">
          {d && (
            <Plot
              height={300}
              series={[
                { name: `CIC (N=${d.design.cic_order})`, x: d.resp_freq, y: d.resp_cic_db, dash: '5 3', width: 1.3, color: 'var(--series-4)' },
                ...(d.resp_hb_db.length ? [{ name: 'half-band', x: d.resp_freq, y: d.resp_hb_db, dash: '2 3', width: 1.3, color: 'var(--series-5)' }] : []),
                { name: `FIR (${d.design.fir.length} taps)`, x: d.resp_freq, y: d.resp_fir_db, dash: '5 3', width: 1.2, color: 'var(--series-3)' },
                { name: 'complete chain', x: d.resp_freq, y: d.resp_total_db, width: 2.6, color: 'var(--series-1)' },
              ]}
              xBands={bands}
              xLog
              xDomain={[200, d.design.pdm_hz / 2]}
              yDomain={[-150, 5]}
              hLines={[{ value: -60, label: '−60 dB requirement', color: 'var(--bad)' }]}
              xLabel="frequency (Hz, at the PDM rate)"
              yLabel="dB"
            />
          )}
        </Card>
        <Card title="Pass band (0 – 8 kHz)" subtitle="The FIR is designed by weighted least squares with D(f) = 1/|H_CIC(f)|, cancelling the CIC's sinc droop">
          {d && (
            <Plot
              height={300}
              series={[{ name: 'CIC alone (droop)', x: d.pass_freq, y: d.pass_cic_db, dash: '5 3' }, { name: 'CIC + compensating FIR', x: d.pass_freq, y: d.pass_total_db, width: 2.6 }]}
              xDomain={[0, nyqOut]}
              yDomain={[-12, 2]}
              xBands={[{ from: 0, to: audio.voice_band_hi_hz, label: 'voice band', color: 'var(--accent-soft)' }]}
              xLabel="frequency (Hz)"
              yLabel="dB"
            />
          )}
        </Card>
      </div>
      <div className="grid grid-2">
        <Card title="Decimated output (16 kHz PCM)" subtitle={d ? `${d.out_x.length} samples after the filters settle · SNDR ${d.sndr_db.toFixed(1)} dB by least-squares sine fit` : ''}>
          {d && <Plot height={200} series={[{ name: 'y[n]', x: d.out_t.slice(0, 160).map((t) => t * 1000), y: d.out_x.slice(0, 160) }]} xLabel="time (ms)" yLabel="amplitude" />}
          {d && <Plot height={200} series={[{ name: 'output spectrum', x: d.out_spectrum.freq, y: d.out_spectrum.db }]} xLabel="frequency (Hz)" yLabel="dBFS" yDomain={[-160, 5]} />}
        </Card>
        <div className="stack">
          <Explainer title="Sigma-delta modulation" refs={[{ label: 'R. Schreier & G. C. Temes, Understanding Delta-Sigma Data Converters, Wiley 2005' }]}>
            <Formula>{'Y(z) = X(z) + (1 − z⁻¹)^L · E(z)\nSQNR gain ≈ (2L+1)·10·log10(OSR) − const'}</Formula>
            A MEMS microphone's ΣΔ modulator samples at 1.024 MHz with a 1-bit quantiser. The feedback loop high-passes the quantisation error
            (+20·L dB/decade), so at OSR = 64 the audio band is nearly clean — provided the decimator removes the enormous noise above it.
          </Explainer>
          <Explainer title="CIC decimator" refs={[{ label: 'E. B. Hogenauer, "An economical class of digital filters for decimation and interpolation", IEEE Trans. ASSP 29(2), 1981' }]}>
            <Formula>{'H(z) = [(1 − z^(−RM)) / (1 − z⁻¹)]^N\n|H(f)| = |sin(πfRM/fs) / sin(πf/fs)|^N,  DC gain (RM)^N\nregister bits = B_in + N·log2(RM)'}</Formula>
            N integrators and N combs, no multipliers — perfect for silicon. Its nulls sit exactly on the multiples of fs/R that alias to DC, but the
            edges of each alias band (k·32 kHz ± 4 kHz) get only ~17 dB per stage: that is why Iris needs <b>N = 4</b> (68 dB) and why the
            FIR cannot help there (its response repeats every 32 kHz).
          </Explainer>
          <Explainer title="Iris design notes">
            With the default 31 taps the chain reaches <b>{d ? d.stopband_db.toFixed(1) : '…'} dB</b>, limited by the CIC, not the FIR:
            only <b>{d?.min_taps_60db ?? '…'}</b> taps are needed for 60 dB. Fewer taps shrink the IRIS-DSP chip (each tap costs a 16-bit register in the
            MAC's sample buffer) — try it here and watch <i>chip.est_cells</i> in the Chip Lab.
          </Explainer>
        </div>
      </div>
    </div>
  );
}
