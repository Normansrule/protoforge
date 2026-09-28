import { useState } from 'react';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { Plot, fmtNum } from '../../components/Plot';
import { Card, Code, ErrorBox, Explainer, Formula, Slider, Stat, Tabs, Toggle } from '../../components/ui';
import type { AdderKind, Arch, BlockCost, DspChipCost, IrisDsp } from './types';
import type { FixedOut } from '../dsp/types';
import { KV, Seg, StemPlot } from '../dsp/viz';

const BLOCK_COLORS = ['var(--series-1)', 'var(--series-3)', 'var(--series-4)', 'var(--series-7)', 'var(--series-5)'];

export function CostBars(props: { cost: DspChipCost; title?: string }) {
  const c = props.cost;
  const max = Math.max(c.cell_budget * 1.25, c.total_cells_pessimistic * 1.04, c.total_cells * 1.08);
  const pct = (v: number) => `${(100 * v) / max}%`;
  let acc = 0;
  const tmax = Math.max(c.clock_period_ns * 1.3, ...c.blocks.map((b) => b.critical_ns * 1.05));
  return (
    <div>
      {props.title && <div className="small muted" style={{ marginBottom: 4 }}>{props.title}</div>}
      <div className="bar-track" aria-label="cell budget bar">
        {c.blocks.map((b, i) => {
          const left = acc;
          acc += b.cells;
          return (
            <div key={b.name} className="bar-seg" title={`${b.name}: ${b.cells.toFixed(0)} cells`} style={{ left: pct(left), width: pct(b.cells), background: BLOCK_COLORS[i % 5] }}>
              {b.cells / max > 0.09 ? b.cells.toFixed(0) : ''}
            </div>
          );
        })}
        <div className="bar-limit" style={{ left: pct(c.cell_budget) }} title="cell budget" />
        <div className="bar-limit" style={{ left: pct(c.total_cells_pessimistic), background: 'var(--muted)', width: 1, opacity: 0.8 }} title="pessimistic estimate" />
      </div>
      <div className="row small" style={{ justifyContent: 'space-between', marginTop: 4 }}>
        <span className="muted">0</span>
        <span>
          <b>{c.total_cells.toFixed(0)}</b> cells {c.area_ok ? '✓' : '✕'} budget {c.cell_budget.toFixed(0)} · pessimistic {c.total_cells_pessimistic.toFixed(0)}
        </span>
      </div>
      <div className="pill-row" style={{ margin: '6px 0 12px' }}>
        {c.blocks.map((b, i) => (
          <span key={b.name} className="chip"><span style={{ width: 9, height: 9, borderRadius: 3, background: BLOCK_COLORS[i % 5], display: 'inline-block' }} /> {b.name}</span>
        ))}
      </div>
      {c.blocks.map((b, i) => (
        <div key={b.name} style={{ marginBottom: 6 }}>
          <div className="row small" style={{ justifyContent: 'space-between' }}>
            <span>{b.name}</span>
            <span className="mono">{b.critical_ns.toFixed(2)} ns</span>
          </div>
          <div className="bar-track" style={{ height: 12 }}>
            <div className="bar-seg" style={{ left: 0, width: `${(100 * b.critical_ns) / tmax}%`, background: b.critical_ns > c.clock_period_ns ? 'var(--bad)' : BLOCK_COLORS[i % 5] }} />
            <div className="bar-limit" style={{ left: `${(100 * c.clock_period_ns) / tmax}%` }} title="clock period" />
          </div>
        </div>
      ))}
      <div className="muted small">Black line: {c.clock_period_ns.toFixed(1)} ns clock period</div>
    </div>
  );
}

function CsdGrid(props: { d: IrisDsp }) {
  const f = props.d.fir;
  const nd = f.coeff_bits + 1;
  const cols = f.coefs.length;
  const firstRow = Math.max(0, Math.min(nd - 1, ...f.coefs.map((c) => { const k = c.csd.findIndex((x) => x !== 0); return k < 0 ? nd : k; })));
  return (
    <div style={{ overflowX: 'auto' }}>
      <div className="csd-grid" style={{ gridTemplateColumns: `52px repeat(${cols}, minmax(16px, 1fr))`, minWidth: 52 + cols * 18 }}>
        <div className="csd-cell lbl">tap</div>
        {f.coefs.map((c) => <div key={c.index} className="csd-cell lbl" style={{ justifyContent: 'center', padding: 0 }}>{c.index}</div>)}
        {Array.from({ length: nd }, (_, row) => row).filter((row) => row >= firstRow).map((row) => {
          const weight = nd - 1 - row - f.frac_bits; // power of two of this digit
          return [
            <div key={`l${row}`} className="csd-cell lbl">2^{weight}</div>,
            ...f.coefs.map((c) => {
              const dgt = c.csd[row];
              return <div key={`${row}-${c.index}`} className={`csd-cell ${dgt > 0 ? 'p' : dgt < 0 ? 'n' : ''}`} title={`tap ${c.index}: code ${c.code}`}>{dgt > 0 ? '+' : dgt < 0 ? '−' : ''}</div>;
            }),
          ];
        })}
        <div className="csd-cell lbl">adds</div>
        {f.coefs.map((c) => <div key={`a${c.index}`} className="csd-cell lbl" style={{ justifyContent: 'center', padding: 0, color: c.shared_with != null ? 'var(--muted)' : 'var(--text)' }}>{c.shared_with != null ? '=' : c.adders}</div>)}
      </div>
    </div>
  );
}

function BlockTable({ blocks }: { blocks: BlockCost[] }) {
  return (
    <div className="table-wrap">
      <table className="table">
        <thead><tr><th>Block</th><th>Cells</th><th>Flops</th><th>Adder</th><th>Other</th><th>Path</th><th>Detail</th></tr></thead>
        <tbody>
          {blocks.map((b) => (
            <tr key={b.name}>
              <td>{b.name}</td>
              <td className="mono">{b.cells.toFixed(0)}</td>
              <td className="mono">{b.ffs.toFixed(0)}</td>
              <td className="mono">{b.adder_cells.toFixed(0)}</td>
              <td className="mono">{b.other_cells.toFixed(0)}</td>
              <td className="mono">{b.critical_ns.toFixed(2)} ns<div className="muted small">{b.critical_desc}</div></td>
              <td className="small muted">{b.detail}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function FirTab() {
  const project = useProject((s) => s.project);
  const patchDesign = useProject((s) => s.patchDesign);
  const chip = project.design.chip;
  const extra = project.design.extra as Record<string, unknown>;
  const arch = ((extra.chip_fir_arch as string) === 'transposed' ? 'transposed' : 'mac') as Arch;
  const adder = ((extra.chip_fir_adder as string) === 'cla' ? 'cla' : 'ripple') as AdderKind;
  const pipeline = extra.chip_fir_pipeline === true;
  const r = useCompute<IrisDsp>('chip.iris_dsp', { project });
  const d = r.data;
  const [codeTab, setCodeTab] = useState<'v' | 'tb'>('v');
  const q = useCompute<FixedOut>(
    'dsp.fixed_point',
    d ? { b: d.fir.coefs.map((c) => c.value), a: [1], fs: 32000, coeff_bits: chip.coeff_bits, data_bits: chip.data_bits, x: [], n_points: 500 } : null,
    !!d,
  );
  const s = d?.summary;
  return (
    <div className="stack">
      <div className="grid grid-sidebar">
        <Card title="IRIS-DSP implementation" subtitle="These choices are saved in the project and drive R-015 / R-016">
          <Seg label="FIR architecture" value={arch} options={[{ value: 'mac', label: 'Decimating MAC' }, { value: 'transposed', label: 'Transposed form' }]} onChange={(v) => patchDesign('extra', { chip_fir_arch: v })} />
          <Seg label="Adders" value={adder} options={[{ value: 'ripple', label: 'Ripple carry' }, { value: 'cla', label: 'Carry lookahead' }]} onChange={(v) => patchDesign('extra', { chip_fir_adder: v })} />
          {arch === 'transposed' && <Toggle label="Pipeline the multiplier block" value={pipeline} onChange={(v) => patchDesign('extra', { chip_fir_pipeline: v })} />}
          <Slider label="Coefficient bits" value={chip.coeff_bits} min={6} max={20} step={1} onChange={(v) => patchDesign('chip', { coeff_bits: Math.round(v) })} format={(v) => v.toFixed(0)} />
          <Slider label="Data bits" value={chip.data_bits} min={8} max={24} step={1} onChange={(v) => patchDesign('chip', { data_bits: Math.round(v) })} format={(v) => v.toFixed(0)} />
          <Slider label="Clock" value={chip.clock_mhz} min={10} max={100} step={1} unit="MHz" onChange={(v) => patchDesign('chip', { clock_mhz: Math.round(v) })} format={(v) => v.toFixed(0)} />
          <Slider label="Gate delay (model)" value={chip.gate_delay_ns} min={0.1} max={0.6} step={0.01} unit="ns" onChange={(v) => patchDesign('chip', { gate_delay_ns: v })} format={(v) => v.toFixed(2)} />
          {d && (
            <div style={{ marginTop: 6 }}>
              <KV k="Coefficients" v={d.coeff_source.split('(')[0]} />
              <KV k="Taps / format" v={`${d.fir.taps} × Q0.${d.fir.frac_bits}`} />
              <KV k="Accumulator" v={`${d.fir.acc_bits} bits`} hint="D + ⌈log2 Σ|c|⌉ + 1" />
              <KV k="CSD vs binary non-zeros" v={`${d.fir.total_csd_digits} vs ${d.fir.total_binary_digits}`} />
              <KV k="Unique multipliers" v={d.fir.unique_multipliers} hint="identical (symmetric) coefficients share one shift-add tree" />
              <KV k="Cycles per output" v={d.fir.cycles_per_output} />
              <KV k="CIC in the chip" v={`N=${d.cic.order}, R=${d.cic.rate}, ${d.cic.width} bits`} />
            </div>
          )}
        </Card>
        <div className="stack">
          <ErrorBox error={r.error} />
          {s && d && (
            <div className="lab-hero">
              <Stat label="Critical path (R-015)" value={s.critical_ns.toFixed(2)} unit="ns" tone={s.timing_ok ? 'pass' : 'fail'} note={<>{s.timing_ok ? '✓' : '✕'} ≤ {s.clock_period_ns.toFixed(0)} ns · slack {s.slack_ns.toFixed(2)} ns</>} />
              <Stat label="Standard cells (R-016)" value={s.total_cells.toFixed(0)} tone={s.area_ok ? 'pass' : 'fail'} note={<>{s.area_ok ? '✓' : '✕'} ≤ {s.cell_budget.toFixed(0)} · pessimistic {s.total_cells_pessimistic.toFixed(0)}</>} />
              <Stat label={`If ${arch === 'mac' ? 'transposed form' : 'decimating MAC'}`} value={`${d.alternative.total_cells.toFixed(0)}`} unit="cells" note={`${d.alternative.critical_ns.toFixed(1)} ns critical path`} tone={d.alternative.timing_ok && d.alternative.area_ok ? undefined : 'warn'} />
              <Stat label="Quantised FIR alone (> 12 kHz)" value={q.data ? Math.max(...q.data.fixed_db.filter((_, i) => q.data!.freq[i] >= 12000)).toFixed(1) : '—'} unit="dB" note={`${chip.coeff_bits}-bit; the CIC adds ≥ 8 dB here — full chain in the DSP Lab`} />
            </div>
          )}
          {s && (
            <Card title="Area and timing budget" subtitle={`CIC + FIR + gesture FSM + Tiny Tapeout glue at ${chip.clock_mhz} MHz`}>
              <CostBars cost={s} />
              {s.suggestions.length > 0 && (
                <div className="callout" style={{ marginTop: 12, borderLeftColor: 'var(--warn)' }}>
                  <h4>How to close it</h4>
                  <ul style={{ margin: 0, paddingLeft: 18 }}>{s.suggestions.map((x) => <li key={x}>{x}</li>)}</ul>
                </div>
              )}
            </Card>
          )}
        </div>
      </div>
      {d && s && (
        <Card title="Block breakdown" subtitle="Cell model: 1 cell per full-adder bit (ripple), ≈4 per bit for carry lookahead, 1 per flip-flop or mux bit; array multiplier D·B AND + D·(B−1) FA">
          <BlockTable blocks={s.blocks} />
        </Card>
      )}
      <div className="grid grid-2">
        <Card title="Canonical signed digits" subtitle="Each coefficient as Σ ±2^k with no two adjacent non-zeros; each extra non-zero digit costs one adder in a constant multiplier">
          {d && <CsdGrid d={d} />}
          {d && <div style={{ marginTop: 10 }}><StemPlot y={d.fir.coefs.map((c) => c.quantized)} height={150} label="quantised coefficients" xLabel="tap" /></div>}
        </Card>
        <Card title="Quantised response" subtitle={`Float vs ${chip.coeff_bits}-bit coefficients at the FIR's 32 kHz input rate (shaded: stop band that must reach 60 dB)`}>
          {q.data && (
            <Plot height={300} series={[{ name: 'double', x: q.data.freq, y: q.data.float_db, color: 'var(--muted)', width: 3 }, { name: `${chip.coeff_bits}-bit`, x: q.data.freq, y: q.data.fixed_db, dash: '5 3' }]}
              xBands={[{ from: 12000, to: 16000, color: 'color-mix(in srgb, var(--bad) 12%, transparent)', label: 'stop band' }]} hLines={[{ value: -60, label: '−60 dB', color: 'var(--bad)' }]} yDomain={[-140, 5]} xLabel="frequency (Hz)" yLabel="dB" />
          )}
        </Card>
      </div>
      {d && (
        <Card
          title="Generated RTL"
          subtitle={`${d.fir.arch === 'mac' ? 'Decimating multiply-accumulate' : 'Transposed form'} · testbench checks ${d.fir.y_expected.length} outputs against the exact integer convolution computed in Rust (verified with Icarus Verilog)`}
        >
          <Tabs tabs={[{ id: 'v', label: 'iris_fir.v' }, { id: 'tb', label: 'tb_iris_fir.v' }]} value={codeTab} onChange={setCodeTab} />
          {codeTab === 'v' ? <Code code={d.fir.verilog} filename="iris_fir.v" maxHeight={440} /> : <Code code={d.fir.testbench} filename="tb_iris_fir.v" maxHeight={440} />}
        </Card>
      )}
      <div className="grid grid-3">
        <Explainer title="Transposed form vs time-shared MAC" refs={[{ label: 'K. K. Parhi, VLSI Digital Signal Processing Systems, Wiley 1999, ch. 3 & 13' }]}>
          <Formula>{'transposed:  y = c0·x + z0,  z_k ← c_{k+1}·x + z_{k+1}\nMAC:         acc ← acc + c_j · x[n−j],  j = 0 … N−1'}</Formula>
          The transposed form computes a whole output every clock with N constant multipliers — overkill when Iris needs one output
          every {fmtNum((chip.clock_mhz * 1e6) / 16000)} clocks. The MAC reuses one multiplier N times per output.
        </Explainer>
        <Explainer title="Canonical signed digits" refs={[{ label: 'G. W. Reitwiesner, "Binary arithmetic", Advances in Computers 1 (1960)' }]}>
          <Formula>{'119 = 0111 0111₂ (6 ones) = 2⁷ − 2³ − 2⁰ (3 digits)\nx·119 = (x≪7) − (x≪3) − x   → 2 adders'}</Formula>
          CSD digits are the non-adjacent form: the minimum number of non-zero signed digits, about B/3 on average instead of B/2.
        </Explainer>
        <Explainer title="Static timing model" refs={[{ label: 'N. Weste & D. Harris, CMOS VLSI Design, 4th ed., ch. 11' }]}>
          <Formula>{'ripple adder ≈ bits × 2 × t_gate\ncritical path = max over stages + 3·t_gate (clk→Q, setup)'}</Formula>
          These are first-order estimates for architecture decisions; Yosys + OpenSTA on the real sky130 library give the sign-off numbers
          (see the Tape-out tab).
        </Explainer>
      </div>
    </div>
  );
}
