// QRNG design bench: simulate a diode-noise (or photon) entropy source,
// digitise it, condition it (von Neumann, XOR, Toeplitz) and run NIST
// SP 800-22 tests at every stage — or test your own board's bitstream.
import { useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat, Toggle } from '../../components/ui';
import { Plot, fmtNum } from '../../components/Plot';
import { tryCall } from '../../engine/engine';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { NistOut, QrngIn, QrngOut, Stage, TestResult } from './types';
import { BitCanvas, csv, Kv, PlayBar, Seg, usePlayer } from './viz';

const DEFAULTS: QrngIn = {
  source: 'diode', samples: 40000, seed: 42, split_ratio: 0.5, eff1: 0.62, eff0: 0.6, photon_rate_hz: 1e6,
  noise_rms_mv: 40, flicker_rms_mv: 4, offset_mv: 3, hum_mv: 1, hum_hz: 60, bandwidth_hz: 0, sample_rate_hz: 2e7, adc_bits: 8,
  adc_vpp_mv: 400, digitize: 'lsb', lsb_bits: 4, von_neumann: false, xor_fold: 1, toeplitz: true, toeplitz_block: 1024, security_bits: 64,
};

function hexToBytes(hex: string): Uint8Array {
  const out = new Uint8Array(Math.floor(hex.length / 2));
  for (let i = 0; i < out.length; i++) out[i] = parseInt(hex.substr(2 * i, 2), 16);
  return out;
}
function hexToAscii(hex: string): string {
  let s = '';
  for (const ch of hex) s += parseInt(ch, 16).toString(2).padStart(4, '0');
  return s;
}

export function NistTable({ tests }: { tests: TestResult[] }) {
  return (
    <div className="table-wrap">
      <table className="table qx-table-tight">
        <thead><tr><th>Test (SP 800-22 §)</th><th>p-value</th><th>Result</th><th>Statistic</th></tr></thead>
        <tbody>
          {tests.map((t) => (
            <tr key={t.name} title={t.note}>
              <td>{t.name} <span className="muted small">§{t.section}</span><div className="small muted">{t.note}</div></td>
              <td className="tnum">{t.applicable ? `${t.p_value.toFixed(4)}${t.p_value2 != null ? ` / ${t.p_value2.toFixed(4)}` : ''}` : '—'}</td>
              <td>{!t.applicable ? <Chip>○ n/a</Chip> : t.pass ? <Chip tone="pass">✓ pass</Chip> : <Chip tone="fail">✕ fail</Chip>}</td>
              <td className="small mono">{t.statistic}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function Pipeline({ o, sel, onSel }: { o: QrngOut; sel: string; onSel: (id: string) => void }) {
  const first: { id: string; name: string; sub: string }[] = o.source === 'diode'
    ? [
      { id: '_src', name: 'Diode noise', sub: `${o.total_rms_mv.toFixed(1)} mV rms` },
      { id: '_adc', name: 'ADC', sub: `${fmtNum(o.sample_rate_hz)} S/s, LSB ${o.lsb_mv.toFixed(2)} mV` },
    ]
    : [{ id: '_src', name: 'Photon + beam splitter', sub: `P(1) = ${(o.photon_p1 ?? 0.5).toFixed(3)}` }];
  const boxes = [...first, ...o.stages.map((s) => ({ id: s.id, name: s.name, sub: `${fmtNum(s.rate_bps)} bit/s · ${(s.efficiency * 100).toFixed(0)} %` }))];
  return (
    <div className="qx-pipe" role="list">
      {boxes.map((b, k) => {
        const st = o.stages.find((s) => s.id === b.id);
        return (
          <div key={b.id} style={{ display: 'contents' }}>
            {k > 0 && (
              <svg className="qx-pipe-arrow" viewBox="0 0 40 20" aria-hidden>
                <line x1={2} y1={10} x2={36} y2={10} className="flow-line" stroke="var(--series-4)" strokeWidth={2.5} />
                <path d="M31 5 L38 10 L31 15" fill="none" stroke="var(--series-4)" strokeWidth={2} />
              </svg>
            )}
            <button role="listitem" className={`qx-pipe-box ${sel === b.id ? 'on' : ''} ${st ? '' : 'phys'}`} onClick={() => st && onSel(b.id)} disabled={!st}>
              <b>{b.name}</b>
              <span className="small muted">{b.sub}</span>
              {st && <span className="small">{st.passed}/{st.applicable} NIST {st.passed === st.applicable ? '✓' : '✕'}</span>}
            </button>
          </div>
        );
      })}
    </div>
  );
}

export default function Qrng() {
  const project = useProject((s) => s.project);
  const patchDesign = useProject((s) => s.patchDesign);
  const bound = project.design.extra?.qrng as Partial<QrngIn> | undefined;
  const [useProj, setUseProj] = useState(true);
  const linked = !!bound && useProj;
  const [local, setLocal] = useState<QrngIn>(DEFAULTS);
  const cfg: QrngIn = linked ? { ...DEFAULTS, ...bound } : local;
  const set = <K extends keyof QrngIn>(k: K, v: QrngIn[K]) => {
    if (linked) patchDesign('extra', { qrng: { ...bound, [k]: v } });
    else setLocal((c) => ({ ...c, [k]: v }));
  };
  const r = useCompute<QrngOut>('quantum.qrng', { ...cfg, samples: Math.max(cfg.samples, 20000) });
  const o = r.data;
  const [sel, setSel] = useState('toeplitz');
  const stage: Stage | undefined = o?.stages.find((s) => s.id === sel) ?? o?.stages[o.stages.length - 1];
  const raw = o?.stages[0];
  const scope = usePlayer(40, 6, { resetKey: JSON.stringify(cfg) });
  const win = 250;
  const off = o ? Math.floor(((o.analog_mv.length || o.codes.length) - win) * (scope.i / 39)) : 0;

  const [userBits, setUserBits] = useState('');
  const [userRes, setUserRes] = useState<NistOut | null>(null);
  const [userErr, setUserErr] = useState<string | null>(null);
  const testUser = (bits: string) => {
    const res = tryCall<NistOut>('quantum.nist', { bits });
    setUserRes(res.data);
    setUserErr(res.error?.replace(/^quantum\.nist:\s*/, '') ?? null);
  };
  const onFile = (f: File | undefined) => {
    if (!f) return;
    f.arrayBuffer().then((buf) => {
      const u8 = new Uint8Array(buf.slice(0, 262144));
      const txt = new TextDecoder().decode(u8.slice(0, 64));
      if (/^[01\s]+$/.test(txt)) {
        testUser(new TextDecoder().decode(u8));
      } else {
        let hex = 'hex:';
        for (const b of u8) hex += b.toString(16).padStart(2, '0');
        testUser(hex);
      }
    });
  };

  return (
    <div className="stack">
      <div className="grid grid-sidebar">
        <Card title="Entropy source" subtitle={linked ? `Bound to ${project.meta.name}: design.extra.qrng` : 'Design your QRNG board: diode, amplifier, ADC, FPGA conditioning.'}>
          {bound && <Toggle label={`Edit the project's QRNG (${project.meta.name})`} value={useProj} onChange={setUseProj} />}
          <Seg value={cfg.source} onChange={(v) => set('source', v)} options={[{ value: 'diode', label: 'Reverse-biased diode' }, { value: 'photon', label: 'Photon at a beam splitter' }]} />
          {cfg.source === 'diode' ? (
            <>
              <Slider label="Avalanche noise at the ADC (rms)" value={cfg.noise_rms_mv} min={1} max={200} step={0.5} unit="mV" onChange={(v) => set('noise_rms_mv', v)} hint="diode noise × amplifier gain" />
              <Slider label="1/f drift (rms)" value={cfg.flicker_rms_mv} min={0} max={50} step={0.5} unit="mV" onChange={(v) => set('flicker_rms_mv', v)} />
              <Slider label="DC offset from mid-scale" value={cfg.offset_mv} min={-50} max={50} step={0.5} unit="mV" onChange={(v) => set('offset_mv', v)} />
              <Slider label="Mains pick-up amplitude" value={cfg.hum_mv} min={0} max={50} step={0.5} unit="mV" onChange={(v) => set('hum_mv', v)} />
              <Slider label="Amplifier bandwidth (0 = wide)" value={cfg.bandwidth_hz} min={0} max={5e7} step={1e5} format={(v) => (v <= 0 ? 'wide' : fmtNum(v))} unit={cfg.bandwidth_hz > 0 ? 'Hz' : ''} onChange={(v) => set('bandwidth_hz', v)} />
              <Slider label="ADC sample rate" value={cfg.sample_rate_hz} min={1e5} max={2e8} log unit="S/s" format={(v) => fmtNum(v)} onChange={(v) => set('sample_rate_hz', v)} />
              <Seg value={cfg.adc_bits} onChange={(v) => set('adc_bits', v)} options={[6, 8, 10, 12].map((b) => ({ value: b, label: `${b}-bit` }))} label="ADC resolution" />
              <Slider label="ADC full scale (peak-to-peak)" value={cfg.adc_vpp_mv} min={20} max={2000} step={10} unit="mV" onChange={(v) => set('adc_vpp_mv', v)} />
              <Seg value={cfg.digitize} onChange={(v) => set('digitize', v)} options={[{ value: 'lsb', label: 'k LSBs' }, { value: 'comparator', label: 'Comparator (1 bit)' }]} label="Digitisation" />
              {cfg.digitize === 'lsb' && <Slider label="Bits kept per sample" value={cfg.lsb_bits} min={1} max={cfg.adc_bits} step={1} onChange={(v) => set('lsb_bits', Math.round(v))} />}
            </>
          ) : (
            <>
              <Slider label="Beam-splitter transmission t" value={cfg.split_ratio} min={0.3} max={0.7} step={0.005} onChange={(v) => set('split_ratio', v)} />
              <Slider label="Detector 1 efficiency" value={cfg.eff1} min={0.05} max={1} step={0.01} onChange={(v) => set('eff1', v)} />
              <Slider label="Detector 0 efficiency" value={cfg.eff0} min={0.05} max={1} step={0.01} onChange={(v) => set('eff0', v)} />
              <Slider label="Photon rate" value={cfg.photon_rate_hz} min={1e4} max={1e9} log unit="Hz" format={(v) => fmtNum(v)} onChange={(v) => set('photon_rate_hz', v)} />
            </>
          )}
          <h4 style={{ margin: '10px 0 6px' }}>Conditioning</h4>
          <Toggle label="von Neumann debiasing" value={cfg.von_neumann} onChange={(v) => set('von_neumann', v)} />
          <Slider label="XOR folding (k bits → 1)" value={cfg.xor_fold} min={1} max={8} step={1} onChange={(v) => set('xor_fold', Math.round(v))} />
          <Toggle label="Toeplitz-hashing extractor" value={cfg.toeplitz} onChange={(v) => set('toeplitz', v)} />
          {cfg.toeplitz && (
            <>
              <Seg value={cfg.toeplitz_block} onChange={(v) => set('toeplitz_block', v)} options={[256, 512, 1024, 2048, 4096].map((b) => ({ value: b, label: String(b) }))} label="Extractor input block m" />
              <Slider label="Security parameter −log₂ ε" value={cfg.security_bits} min={0} max={128} step={1} unit="bits" onChange={(v) => set('security_bits', Math.round(v))} />
            </>
          )}
          <button className="btn small" onClick={() => set('seed', cfg.seed + 1)}>New noise realisation</button>
        </Card>
        <div className="stack">
          {o && (
            <div className="qx-stats">
              <Stat label="Min-entropy / sample" value={o.min_entropy_per_sample.toFixed(2)} unit="bit" note={`MCV (SP 800-90B); ideal ${o.min_entropy_theory.toFixed(2)}${o.source === 'diode' ? ' for the full code' : ''}`} />
              <Stat label="Raw bias P(1) − ½" value={(o.raw_bias * 100).toFixed(2)} unit="%" tone={Math.abs(o.raw_bias) > 0.01 ? 'warn' : undefined} />
              {o.source === 'diode' && <Stat label="Clipping" value={(o.clipped_fraction * 100).toFixed(2)} unit="%" tone={o.clipped_fraction > 0.001 ? 'fail' : 'pass'} />}
              <Stat label="Output rate" value={fmtNum(o.output_rate_bps)} unit="bit/s" note={`${o.stages[o.stages.length - 1].name}`} />
            </div>
          )}
          {o?.notes.map((n) => <div key={n} className="callout small" style={{ borderLeftColor: 'var(--warn)' }}>! {n}</div>)}
          {o && o.source === 'diode' && (
            <div className="grid grid-2">
              <Card title="Oscilloscope: ADC input" subtitle="Analog noise (mV) sampled by the ADC">
                <PlayBar p={scope} label={() => `samples ${off}–${off + win}`} />
                <Plot
                  series={[{ name: 'v (mV)', x: Array.from({ length: win }, (_, k) => off + k), y: o.analog_mv.slice(off, off + win), step: true, width: 1.2 }]}
                  hLines={[{ value: o.lsb_mv * 2 ** (cfg.adc_bits - 1), label: '+full scale', color: 'var(--bad)' }, { value: -o.lsb_mv * 2 ** (cfg.adc_bits - 1), label: '−full scale', color: 'var(--bad)' }, { value: 0, label: 'mid-scale' }]}
                  xLabel="sample" yLabel="mV" height={200} legend={false}
                />
              </Card>
              <Card title="Histogram of ADC codes" subtitle="Bars: simulated. Line: Gaussian of the same rms — wider is more entropy.">
                <Plot
                  series={[
                    { name: 'simulated', x: o.hist_x, y: o.hist_counts, step: true, area: true },
                    { name: 'Gaussian', x: o.hist_x, y: o.hist_expected, dash: '4 3' },
                  ]}
                  xLabel="input (mV)" yLabel="count" height={200} legend
                />
              </Card>
            </div>
          )}
          {o && o.source === 'photon' && (
            <Card title="Detector clicks">
              <BitCanvas bits={raw?.preview ?? ''} cols={64} px={5} title="raw clicks: black = detector 1" />
              <div className="small muted">P(1) = t η₁ / (t η₁ + (1 − t) η₀) = {(o.photon_p1 ?? 0.5).toFixed(4)} — even a perfect 50:50 splitter is biased by unequal detectors.</div>
            </Card>
          )}
          {o && (
            <Card
              title="Extraction pipeline"
              subtitle="Click a stage to inspect its bits and NIST results"
              actions={stage && (
                <div className="row" style={{ gap: 6 }}>
                  <DownloadButton small filename={`qrng-${stage.id}.bin`} content={() => new Blob([hexToBytes(stage.hex) as BlobPart], { type: 'application/octet-stream' })} label=".bin" />
                  <DownloadButton small filename={`qrng-${stage.id}.txt`} content={() => hexToAscii(stage.hex)} label="ASCII 0/1" />
                  {o.source === 'diode' && <DownloadButton small filename="qrng-samples.csv" content={() => csv(['sample', 'mV', 'code'], o.analog_mv.map((v, k) => [k, v, o.codes[k]]))} label="samples.csv" />}
                </div>
              )}
            >
              <Pipeline o={o} sel={stage?.id ?? ''} onSel={setSel} />
              {stage && (
                <div className="grid grid-2" style={{ marginTop: 12 }}>
                  <div>
                    <div className="row" style={{ gap: 12, alignItems: 'flex-start' }}>
                      <div>
                        <div className="small muted">{stage.name}: first {Math.min(4096, stage.bits)} bits</div>
                        <BitCanvas bits={stage.preview} cols={64} px={4} />
                      </div>
                      <div style={{ flex: 1, minWidth: 160 }}>
                        <Kv rows={[
                          ['Bits', fmtNum(stage.bits)],
                          ['Ones', `${(stage.ones_fraction * 100).toFixed(2)} %`],
                          ['Min-entropy / bit (MCV)', stage.min_entropy_per_bit.toFixed(4)],
                          ['Output / raw bit', stage.efficiency.toFixed(3)],
                          ['Rate', `${fmtNum(stage.rate_bps)} bit/s`],
                        ]} />
                      </div>
                    </div>
                    <Plot
                      series={[
                        { name: 'raw', x: raw!.autocorr.map((_, k) => k + 1), y: raw!.autocorr, scatter: true },
                        ...(stage.id !== 'raw' ? [{ name: stage.name, x: stage.autocorr.map((_, k) => k + 1), y: stage.autocorr, scatter: true }] : []),
                      ]}
                      hLines={[{ value: 3 / Math.sqrt(Math.max(1, stage.bits)), label: '±3/√n', color: 'var(--muted)' }, { value: -3 / Math.sqrt(Math.max(1, stage.bits)), label: '', color: 'var(--muted)' }]}
                      xLabel="lag (bits)" yLabel="autocorrelation" height={170} legend
                    />
                  </div>
                  <NistTable tests={stage.tests} />
                </div>
              )}
            </Card>
          )}
          {r.error && <ErrorBox error={r.error} />}
        </div>
      </div>

      <div className="grid grid-2">
        <Card title="Test your own board's bits" subtitle="Paste 0/1 text or hex (prefix hex:), or open a binary or ASCII-0/1 file from your FPGA. Runs in your browser; nothing is uploaded.">
          <textarea rows={4} value={userBits} onChange={(e) => setUserBits(e.target.value)} placeholder="hex:C90FDAA22168C234C4C6628B80DC1CD1…  or  0110100111…" aria-label="bitstream" />
          <div className="row" style={{ marginTop: 6 }}>
            <button className="btn small primary" onClick={() => testUser(userBits)} disabled={!userBits.trim()}>Run the tests</button>
            <label className="btn small" style={{ cursor: 'pointer' }}>
              Open a file…
              <input type="file" style={{ display: 'none' }} onChange={(e) => onFile(e.target.files?.[0])} />
            </label>
            <span className="small muted">First 256 kB are used.</span>
          </div>
          {userErr && <ErrorBox error={userErr} />}
          {userRes && (
            <div style={{ marginTop: 8 }}>
              <Kv rows={[['Bits', fmtNum(userRes.n)], ['Ones', `${(userRes.ones_fraction * 100).toFixed(3)} %`], ['Min-entropy / bit (MCV)', userRes.min_entropy_per_bit.toFixed(4)], ...(userRes.min_entropy_per_byte != null ? [['Min-entropy / byte (MCV)', userRes.min_entropy_per_byte.toFixed(3)] as [string, string]] : [])]} />
              <NistTable tests={userRes.tests} />
            </div>
          )}
        </Card>
        <Explainer title="Designing a diode-noise QRNG board" refs={[{ label: 'Herrero-Collantes & Garcia-Escartin, Rev. Mod. Phys. 89, 015004 (2017)' }, { label: 'NIST SP 800-90B (2018)' }, { label: 'NIST SP 800-22 Rev. 1a (2010)' }, { label: 'Ma et al., Phys. Rev. A 87, 062327 (2013)' }]}>
          <ul className="small" style={{ margin: '4px 0', paddingLeft: 18 }}>
            <li><b>Source.</b> A junction biased past breakdown through a resistor makes avalanche noise of a few mV. Keep the bias stable; the noise level drifts with temperature and current.</li>
            <li><b>Gain.</b> AC-couple and amplify until the noise rms is a sizeable fraction of the ADC range — about 1/10 of full scale keeps clipping negligible (the clipping readout above).</li>
            <li><b>Bandwidth.</b> Samples are independent only if the analog bandwidth exceeds half the sample rate; otherwise consecutive samples correlate (try the bandwidth slider and watch the autocorrelation).</li>
            <li><b>Digitise and credit honestly.</b> Keep only the low bits and credit the measured min-entropy, not the ideal; mains hum and 1/f drift mostly affect the high bits.</li>
            <li><b>Condition in the FPGA.</b> A Toeplitz extractor is a shift register and an XOR tree; its output length follows the leftover hash lemma. Run continuous health tests (SP 800-90B §4.4) and stop on failure.</li>
          </ul>
          <Formula>{'H∞ = −log₂ max P(x)     ideal quantised Gaussian: H∞ ≈ log₂(σ√(2π) / LSB)\nToeplitz output: l = ⌊m·h − 2 log₂(1/ε)⌋ bits per m-bit block'}</Formula>
          Passing SP 800-22 does not prove randomness — a counter through a hash passes too. The security argument is the physical source plus a conservative min-entropy estimate plus the extractor.
        </Explainer>
      </div>
    </div>
  );
}
