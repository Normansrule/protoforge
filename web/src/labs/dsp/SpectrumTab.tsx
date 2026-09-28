import { useMemo, useRef, useState } from 'react';
import { useCompute } from '../../engine/hooks';
import { tryCall } from '../../engine/engine';
import { Plot, fmtNum } from '../../components/Plot';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Toggle } from '../../components/ui';
import type { AnalyzeOut, ComponentKind, LeakageTrace, SigComponent, Signal, SignalSpec, WindowInfo, WindowKind } from './types';
import { WINDOW_LABEL } from './types';
import { playSamples, stopAudio, usePlaying } from './audio';
import { Seg, csvOf, useFrame, useReducedMotion } from './viz';

export const REFS_FFT = [
  { label: 'Oppenheim & Schafer, Discrete-Time Signal Processing (3rd ed.), ch. 8–10' },
  { label: 'Cooley & Tukey, Math. Comp. 19 (1965)' },
  { label: 'F. J. Harris, "On the use of windows for harmonic analysis with the DFT", Proc. IEEE 66 (1978)' },
];

export function comp(kind: ComponentKind, freq: number, amp: number, extra: Partial<SigComponent> = {}): SigComponent {
  return { kind, freq, freq_end: 4000, amp, phase_deg: 0, bandlimited: false, vowel: 'a', seed: 1, ...extra };
}

export const PRESETS: { id: string; label: string; spec: SignalSpec }[] = [
  { id: 'voice', label: 'Iris voice + noise', spec: { fs: 16000, n: 4096, components: [comp('voice', 120, 0.5), comp('tone', 50, 0.08), comp('noise', 0, 0.01, { seed: 7 })] } },
  { id: 'tones', label: 'Two close tones', spec: { fs: 16000, n: 1024, components: [comp('tone', 1000, 0.5), comp('tone', 1080, 0.02)] } },
  { id: 'chirp', label: 'Chirp 100 Hz → 7 kHz', spec: { fs: 16000, n: 8192, components: [comp('chirp', 100, 0.5, { freq_end: 7000 })] } },
  { id: 'square', label: 'Square wave (aliasing)', spec: { fs: 16000, n: 4096, components: [comp('square', 1230, 0.4)] } },
  { id: 'hum', label: 'Tone in white noise', spec: { fs: 16000, n: 4096, components: [comp('tone', 1500, 0.3), comp('noise', 0, 0.1, { seed: 3 })] } },
];

const KIND_OPTS: { value: ComponentKind; label: string }[] = [
  { value: 'tone', label: 'Sine tone' },
  { value: 'noise', label: 'White noise' },
  { value: 'chirp', label: 'Linear chirp' },
  { value: 'square', label: 'Square' },
  { value: 'saw', label: 'Sawtooth' },
  { value: 'voice', label: 'Voice-like vowel' },
  { value: 'impulse', label: 'Impulse' },
];

const WINDOWS: WindowKind[] = ['rectangular', 'hann', 'hamming', 'blackman', 'kaiser'];

export function SignalBuilder(props: { spec: SignalSpec; setSpec: (s: SignalSpec) => void }) {
  const { spec, setSpec } = props;
  const set = (i: number, patch: Partial<SigComponent>) => setSpec({ ...spec, components: spec.components.map((c, k) => (k === i ? { ...c, ...patch } : c)) });
  const nyq = spec.fs / 2;
  return (
    <div>
      <div className="pill-row" style={{ marginBottom: 12 }}>
        {PRESETS.map((p) => (
          <button key={p.id} className="btn small" onClick={() => setSpec(structuredClone(p.spec))}>{p.label}</button>
        ))}
      </div>
      <div className="grid grid-2" style={{ gap: 10 }}>
        <Select label="Sample rate" value={String(spec.fs)} options={['8000', '16000', '44100', '48000'].map((v) => ({ value: v, label: `${Number(v) / 1000} kHz` }))} onChange={(v) => setSpec({ ...spec, fs: Number(v) })} />
        <Select label="Samples N" value={String(spec.n)} options={['256', '512', '1024', '2048', '4096', '8192', '16384'].map((v) => ({ value: v, label: v }))} onChange={(v) => setSpec({ ...spec, n: Number(v) })} />
      </div>
      {spec.components.map((c, i) => (
        <div className="comp-row" key={i}>
          <div className="row" style={{ marginBottom: 6 }}>
            <select value={c.kind} onChange={(e) => set(i, { kind: e.target.value as ComponentKind })} style={{ width: 'auto', flex: 1 }} aria-label="component type">
              {KIND_OPTS.map((o) => <option key={o.value} value={o.value}>{o.label}</option>)}
            </select>
            <button className="icon-btn" title="Remove" aria-label="Remove component" onClick={() => setSpec({ ...spec, components: spec.components.filter((_, k) => k !== i) })}>✕</button>
          </div>
          {(c.kind === 'tone' || c.kind === 'square' || c.kind === 'saw' || c.kind === 'chirp') && (
            <Slider label={c.kind === 'chirp' ? 'Start frequency' : 'Frequency'} value={Math.min(c.freq, nyq)} min={20} max={nyq} log unit="Hz" onChange={(v) => set(i, { freq: v })} format={(v) => v.toFixed(0)} />
          )}
          {c.kind === 'chirp' && <Slider label="End frequency" value={Math.min(c.freq_end, nyq)} min={20} max={nyq} log unit="Hz" onChange={(v) => set(i, { freq_end: v })} format={(v) => v.toFixed(0)} />}
          {c.kind === 'voice' && (
            <>
              <Slider label="Pitch f0" value={c.freq} min={70} max={300} unit="Hz" onChange={(v) => set(i, { freq: v })} format={(v) => v.toFixed(0)} />
              <Seg label="Vowel" value={c.vowel} options={['a', 'e', 'i', 'o', 'u'].map((v) => ({ value: v, label: `/${v}/` }))} onChange={(v) => set(i, { vowel: v })} />
            </>
          )}
          {c.kind === 'impulse' && <Slider label="Position" value={c.freq} min={0} max={spec.n - 1} step={1} unit="samples" onChange={(v) => set(i, { freq: Math.round(v) })} format={(v) => v.toFixed(0)} />}
          <Slider label={c.kind === 'noise' ? 'RMS' : 'Amplitude'} value={c.amp} min={0.001} max={1} log onChange={(v) => set(i, { amp: v })} format={(v) => `${v.toFixed(3)} (${(20 * Math.log10(v)).toFixed(1)} dB)`} />
          {(c.kind === 'square' || c.kind === 'saw') && <Toggle label="Band-limited (no aliasing)" value={c.bandlimited} onChange={(v) => set(i, { bandlimited: v })} />}
        </div>
      ))}
      <div className="pill-row">
        {(['tone', 'noise', 'chirp', 'voice', 'square'] as ComponentKind[]).map((k) => (
          <button key={k} className="btn small" onClick={() => setSpec({ ...spec, components: [...spec.components, comp(k, k === 'voice' ? 140 : 2000, k === 'noise' ? 0.02 : 0.2, { seed: spec.components.length + 1 })] })}>
            + {k}
          </button>
        ))}
      </div>
    </div>
  );
}

/** Generate a ~1.5 s version of the signal for listening. */
export function longSignal(spec: SignalSpec, seconds = 1.5): number[] | null {
  const r = tryCall<Signal>('dsp.signal', { ...spec, n: Math.round(spec.fs * seconds) });
  return r.data?.x ?? null;
}

export function PlayButton(props: { id: string; label: string; get: () => { x: number[]; fs: number } | null }) {
  const playing = usePlaying();
  const on = playing === props.id;
  return (
    <button
      className={`btn small ${on ? 'playing' : ''}`}
      onClick={() => {
        if (on) return stopAudio();
        const r = props.get();
        if (r) playSamples(props.id, r.x, r.fs, true);
      }}
      aria-pressed={on}
    >
      {on ? <><span className="pulse-dot" /> Stop</> : <>▶ {props.label}</>}
    </button>
  );
}

function LeakageDemo() {
  const reduced = useReducedMotion();
  const [bin, setBin] = useState(10.5);
  const [anim, setAnim] = useState(false);
  const phase = useRef(0);
  useFrame(anim && !reduced, (dt) => {
    phase.current = (phase.current + dt / 4000) % 1;
    setBin(10.5 - 0.5 * Math.cos(phase.current * 2 * Math.PI));
  });
  const leak = useCompute<LeakageTrace[]>('dsp.leakage', { n: 64, bin: Number(bin.toFixed(3)), windows: ['rectangular', 'hann', 'blackman'] });
  const infos = useMemo(() => (leak.ready ? WINDOWS.map((w) => tryCall<WindowInfo>('dsp.window', { kind: w, n: 1024, beta: 8.6 }).data) : []), [leak.ready]);
  return (
    <Card
      title="Spectral leakage & scalloping"
      subtitle="A 64-point DFT of a cosine that sits between bins. Slide (or animate) the tone from a bin centre to half-way between bins."
      actions={<button className={`btn small ${anim ? 'playing' : ''}`} onClick={() => setAnim((a) => !a)} disabled={reduced}>{anim ? '❚❚ Pause' : '▶ Sweep'}</button>}
    >
      <Slider label="Tone position" value={bin} min={10} max={11} step={0.01} unit="bins" onChange={(v) => { setAnim(false); setBin(v); }} format={(v) => v.toFixed(2)} />
      {leak.data && (
        <Plot
          height={250}
          series={[
            ...leak.data.map((t) => ({ name: `${WINDOW_LABEL[t.window]} (reads ${t.peak_db.toFixed(2)} dB)`, x: t.bins, y: t.db, width: 1.4 })),
            ...leak.data.map((t, i) => ({ name: `${WINDOW_LABEL[t.window]} DFT bins`, x: t.bins.filter((_, k) => k % 16 === 0), y: t.db.filter((_, k) => k % 16 === 0), scatter: true, color: `var(--series-${i + 1})` })),
          ]}
          legend={false}
          xDomain={[0, 32]}
          yDomain={[-100, 4]}
          xLabel="frequency (bins)"
          yLabel="dB re true amplitude"
          vLines={[{ value: bin, label: 'true tone', color: 'var(--copper)' }]}
        />
      )}
      {leak.data && (
        <div className="pill-row" style={{ marginTop: 4 }}>
          {leak.data.map((t, i) => (
            <span key={t.window} className="chip"><span style={{ width: 10, height: 3, borderRadius: 2, background: `var(--series-${i + 1})`, display: 'inline-block' }} /> {WINDOW_LABEL[t.window]}: reads {t.peak_db.toFixed(2)} dB · leakage {t.leakage_db.toFixed(0)} dB</span>
          ))}
          <span className="muted small">dots = the 64 DFT bins; lines = the underlying window transform</span>
        </div>
      )}
      <div className="table-wrap" style={{ marginTop: 10 }}>
        <table className="table">
          <thead>
            <tr><th>Window</th><th>Coherent gain</th><th>ENBW (bins)</th><th>Scalloping (dB)</th><th>Side lobe (dB)</th><th>Main lobe (bins)</th></tr>
          </thead>
          <tbody>
            {infos.map((w) => w && (
              <tr key={w.kind}>
                <td>{WINDOW_LABEL[w.kind]}{w.kind === 'kaiser' ? ' β=8.6' : ''}</td>
                <td className="mono">{w.coherent_gain.toFixed(3)}</td>
                <td className="mono">{w.enbw_bins.toFixed(2)}</td>
                <td className="mono">{w.scalloping_loss_db.toFixed(2)}</td>
                <td className="mono">{w.peak_sidelobe_db.toFixed(1)}</td>
                <td className="mono">{w.mainlobe_width_bins.toFixed(1)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <p className="lab-note" style={{ marginTop: 8 }}>
        The rectangular window loses up to <b>3.9 dB</b> half-way between bins and its side lobes fall only 6 dB/octave, so a weak tone near a strong one is buried.
        Tapered windows trade a wider main lobe (poorer resolution) for far lower leakage — Harris' Table 1, reproduced numerically above.
      </p>
    </Card>
  );
}

export function SpectrumTab(props: { spec: SignalSpec; setSpec: (s: SignalSpec) => void }) {
  const { spec, setSpec } = props;
  const [win, setWin] = useState<WindowKind>('hann');
  const [beta, setBeta] = useState(8.6);
  const [pad, setPad] = useState(1);
  const [logF, setLogF] = useState(false);
  const r = useCompute<AnalyzeOut>('dsp.analyze', { spec, window: win, beta, nfft: spec.n * pad });
  const t = useMemo(() => (r.data ? r.data.signal.x.map((_, i) => (i / spec.fs) * 1000) : []), [r.data, spec.fs]);
  const showN = Math.min(t.length, Math.max(64, Math.round(spec.fs * 0.03)));
  return (
    <div className="stack">
      <div className="grid grid-sidebar">
        <Card title="Signal builder" subtitle="Sum of components, generated deterministically in Rust">
          <SignalBuilder spec={spec} setSpec={setSpec} />
        </Card>
        <div className="stack">
          <Card
            title="Time domain"
            subtitle={r.data ? `${spec.n} samples at ${spec.fs / 1000} kHz = ${((spec.n / spec.fs) * 1000).toFixed(1)} ms · RMS ${fmtNum(r.data.signal.rms)} · peak ${fmtNum(r.data.signal.peak)}` : '…'}
            actions={
              <>
                <PlayButton id="spectrum" label="Play" get={() => { const x = longSignal(spec); return x ? { x, fs: spec.fs } : null; }} />
                {r.data && <DownloadButton small filename="signal.csv" label="CSV" content={() => csvOf({ t_s: r.data!.signal.x.map((_, i) => i / spec.fs), x: r.data!.signal.x })} />}
              </>
            }
          >
            <ErrorBox error={r.error} />
            {r.data && <Plot height={190} series={[{ name: 'x[n]', x: t.slice(0, showN), y: r.data.signal.x.slice(0, showN) }]} xLabel="time (ms), first 30 ms" yLabel="amplitude" />}
          </Card>
          <Card
            title="Amplitude spectrum"
            subtitle={r.data ? `FFT ${r.data.spectrum.nfft} points · bin ${r.data.spectrum.bin_hz.toFixed(2)} Hz · coherent gain ${r.data.spectrum.coherent_gain.toFixed(3)} (corrected) · ENBW ${r.data.spectrum.enbw_bins.toFixed(2)} bins · ${r.ms.toFixed(1)} ms in Rust` : ''}
          >
            <div className="row" style={{ alignItems: 'flex-end' }}>
              <div style={{ minWidth: 200 }}><Select label="Window" value={win} options={WINDOWS.map((w) => ({ value: w, label: WINDOW_LABEL[w] }))} onChange={setWin} /></div>
              {win === 'kaiser' && <div style={{ minWidth: 200 }}><Slider label="Kaiser β" value={beta} min={0} max={14} step={0.1} onChange={setBeta} /></div>}
              <Seg label="Zero padding" value={pad} options={[1, 2, 4, 8].map((p) => ({ value: p, label: `×${p}` }))} onChange={setPad} />
              <Toggle label="Log frequency" value={logF} onChange={setLogF} />
            </div>
            {r.data && (
              <Plot
                height={280}
                series={[{ name: 'dBFS', x: r.data.spectrum.freq, y: r.data.spectrum.db }]}
                xLabel="frequency (Hz)"
                yLabel="dB (0 dB = amplitude 1)"
                yDomain={[-140, 6]}
                xLog={logF}
                xDomain={logF ? [Math.max(10, r.data.spectrum.bin_hz), spec.fs / 2] : undefined}
              />
            )}
            {r.data && r.data.spectrum.peaks.length > 0 && (
              <div className="pill-row" style={{ marginTop: 6 }}>
                <span className="muted small">Peaks:</span>
                {r.data.spectrum.peaks.slice(0, 6).map(([f, db]) => <span key={f} className="chip">{fmtNum(f)} Hz · {db.toFixed(1)} dB</span>)}
              </div>
            )}
          </Card>
        </div>
      </div>
      <div className="grid grid-2">
        <LeakageDemo />
        <div className="stack">
          <Explainer title="The DFT and the FFT" refs={REFS_FFT}>
            <Formula>{'X[k] = Σₙ w[n]·x[n]·e^(−j2πkn/N),   k = 0 … N−1\nbin spacing Δf = fs / N,   amplitude = 2|X[k]| / Σw[n]'}</Formula>
            The radix-2 Cooley–Tukey FFT splits an N-point DFT into two N/2-point DFTs of the even and odd samples, recursively, costing
            N·log₂N instead of N² operations — 4096 points is 49 k instead of 16.8 M multiply-adds. Protoforge's FFT is iterative (bit-reversal
            permutation, then log₂N butterfly stages) and runs in the Rust engine compiled to WebAssembly.
          </Explainer>
          <Explainer title="Windows trade resolution for leakage">
            <Formula>{'Hann: w[n] = 0.5 − 0.5 cos(2πn/N)       coherent gain 0.5, ENBW 1.5 bins\nKaiser: w[n] = I₀(β√(1 − (2n/N − 1)²)) / I₀(β)'}</Formula>
            A finite record is an infinite signal multiplied by a window, so its spectrum is convolved with the window's transform. The
            coherent gain Σw/N is divided out so a bin-centred sine reads its true amplitude; noise is scaled by the equivalent noise
            bandwidth instead. Kaiser's β dials side-lobe level continuously (β ≈ 8.6 ≈ Blackman-like).
          </Explainer>
          <Explainer title="Why this matters for Iris">
            Iris samples its voice microphones at <b>16 kHz</b>: everything above 8 kHz must be removed before sampling (see the PDM tab),
            and the wake-word engine looks at 100 Hz – 4 kHz where the vowel formants live. Try the <i>Iris voice</i> preset: the harmonics of
            the 120 Hz pitch are shaped by formant peaks near 730 Hz and 1.1 kHz — the features speech recognisers use.
          </Explainer>
        </div>
      </div>
    </div>
  );
}
