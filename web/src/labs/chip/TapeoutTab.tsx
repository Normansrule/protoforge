import { useState } from 'react';
import JSZip from 'jszip';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { Card, Code, DownloadButton, ErrorBox, Explainer, downloadBlob } from '../../components/ui';
import type { TtOut } from './types';

const PINS_LEFT = ['ui_in[0] PDM data', 'ui_in[1] PDM clock', 'ui_in[2] touch', 'ui_in[3] byte select', 'ui_in[7:4] —'];
const PINS_RIGHT = ['uo_out[7:0] PCM byte'];
const PINS_BOTTOM = ['uio[0] strobe', 'uio[1] tap', 'uio[2] 2×tap', 'uio[3] long', 'uio[6:4] state'];

function Pinout() {
  const W = 620;
  const H = 330;
  const cx = 200;
  const cy = 40;
  const cw = 220;
  const ch = 210;
  return (
    <svg viewBox={`0 0 ${W} ${H}`} style={{ width: '100%', maxWidth: W, display: 'block', margin: '0 auto' }} role="img" aria-label="tt_um_iris_dsp pin-out">
      <defs>
        <linearGradient id="die" x1="0" x2="1" y1="0" y2="1">
          <stop offset="0%" stopColor="var(--surface-3)" />
          <stop offset="100%" stopColor="var(--surface-2)" />
        </linearGradient>
      </defs>
      <rect x={cx} y={cy} width={cw} height={ch} rx={10} fill="url(#die)" stroke="var(--copper)" strokeWidth={2} />
      {Array.from({ length: 8 }, (_, i) => <rect key={i} x={cx + 16 + i * 25} y={cy + 16} width={18} height={18} rx={3} fill="var(--mask)" opacity={0.35 + (i % 3) * 0.15} />)}
      <text x={cx + cw / 2} y={cy + 92} textAnchor="middle" fontWeight={800} fontSize={16} fill="var(--text)">tt_um_iris_dsp</text>
      <text x={cx + cw / 2} y={cy + 114} textAnchor="middle" fontSize={11.5} fill="var(--text-2)">CIC → FIR → 16 kHz PCM</text>
      <text x={cx + cw / 2} y={cy + 132} textAnchor="middle" fontSize={11.5} fill="var(--text-2)">touch-gesture FSM</text>
      <text x={cx + cw / 2} y={cy + 160} textAnchor="middle" fontSize={10.5} fill="var(--muted)" fontFamily="var(--mono)">SkyWater sky130 · Tiny Tapeout</text>
      <text x={cx + cw / 2} y={cy - 12} textAnchor="middle" fontSize={11} fill="var(--text-2)" fontFamily="var(--mono)">clk · rst_n · ena</text>
      <line x1={cx + cw / 2} x2={cx + cw / 2} y1={cy - 8} y2={cy} stroke="var(--text-2)" strokeWidth={2} />
      {PINS_LEFT.map((p, i) => {
        const y = cy + 30 + i * 38;
        return (
          <g key={p}>
            <line x1={cx - 30} x2={cx} y1={y} y2={y} stroke="var(--series-3)" strokeWidth={3} />
            <text x={cx - 36} y={y + 4} textAnchor="end" fontSize={11.5} fill="var(--text)" fontFamily="var(--mono)">{p}</text>
          </g>
        );
      })}
      {PINS_RIGHT.map((p, i) => {
        const y = cy + 105 + i * 40;
        return (
          <g key={p}>
            <line x1={cx + cw} x2={cx + cw + 30} y1={y} y2={y} stroke="var(--series-1)" strokeWidth={6} />
            <text x={cx + cw + 36} y={y + 4} fontSize={11.5} fill="var(--text)" fontFamily="var(--mono)">{p}</text>
          </g>
        );
      })}
      {PINS_BOTTOM.map((p, i) => {
        const x = cx + 22 + i * 44;
        return (
          <g key={p}>
            <line x1={x} x2={x} y1={cy + ch} y2={cy + ch + 22} stroke="var(--series-2)" strokeWidth={3} />
            <text transform={`translate(${x + 4} ${cy + ch + 28}) rotate(35)`} fontSize={11} fill="var(--text)" fontFamily="var(--mono)">{p}</text>
          </g>
        );
      })}
    </svg>
  );
}

const STEPS: { title: string; body: string; code?: string }[] = [
  { title: 'Simulate every block (Icarus Verilog)', body: 'Each testbench checks the RTL against Protoforge’s Rust reference model and prints PASS/FAIL.', code: 'iverilog -g2005 -o tb_fir iris_fir.v tb_iris_fir.v && vvp tb_fir\niverilog -g2005 -o tb_fsm iris_gesture_fsm.v tb_iris_gesture_fsm.v && vvp tb_fsm\niverilog -g2005 -o tb_top tt_um_iris_dsp.v iris_cic.v iris_fir.v iris_gesture_fsm.v tb_tt_um_iris_dsp.v && vvp tb_top\ngtkwave tb_tt_um_iris_dsp.vcd' },
  { title: 'Lint and fast simulation (Verilator)', body: 'Verilator catches width mismatches and unused signals; it is also the fastest simulator for long audio runs.', code: 'verilator --lint-only -Wall tt_um_iris_dsp.v iris_cic.v iris_fir.v iris_gesture_fsm.v --top-module tt_um_iris_dsp' },
  { title: 'Synthesise (Yosys)', body: 'Check the real cell count against the estimate on the FIR → Verilog tab.', code: 'yosys -p "read_verilog tt_um_iris_dsp.v iris_cic.v iris_fir.v iris_gesture_fsm.v; synth -top tt_um_iris_dsp; stat"' },
  { title: 'Python testbenches (cocotb, optional)', body: 'For richer tests — e.g. feeding real microphone recordings — drive the same RTL from Python with cocotb and compare against numpy/scipy.' },
  { title: 'Harden and submit (Tiny Tapeout)', body: 'Copy the .v files into the src/ folder of the current Tiny Tapeout Verilog template repository, fill in info.yaml from the template (top module tt_um_iris_dsp), push, and let the GitHub Actions run OpenLane/OpenROAD on sky130. For local hardening follow the Tiny Tapeout docs, which use tt-support-tools.' },
];

export function TapeoutTab() {
  const project = useProject((s) => s.project);
  const r = useCompute<TtOut>('chip.tinytapeout', { project });
  const [sel, setSel] = useState('tt_um_iris_dsp.v');
  const d = r.data;
  const file = d?.files.find((f) => f.name === sel) ?? d?.files[0];
  const zipAll = async () => {
    if (!d) return;
    const zip = new JSZip();
    const dir = zip.folder('tt_iris_dsp')!;
    d.files.forEach((f) => dir.file(f.name, f.content));
    downloadBlob('tt_iris_dsp.zip', await zip.generateAsync({ type: 'blob' }));
  };
  return (
    <div className="stack">
      <ErrorBox error={r.error} />
      <div className="grid grid-2">
        <Card title="tt_um_iris_dsp" subtitle="Standard Tiny Tapeout user-module ports: ui_in, uo_out, uio_in/out/oe, ena, clk, rst_n">
          <Pinout />
          {d && (
            <p className="lab-note" style={{ marginTop: 8 }}>
              Estimated <b>{d.design.summary.total_cells.toFixed(0)}</b> cells and a <b>{d.design.summary.critical_ns.toFixed(1)} ns</b> critical path with the
              {d.design.fir.arch === 'mac' ? ' decimating MAC' : ' transposed-form'} FIR — change it on the FIR → Verilog tab and the package regenerates.
            </p>
          )}
        </Card>
        <Card
          title="Package"
          subtitle="RTL, self-checking testbenches and a README; the end-to-end testbench streams PDM bits and checks every PCM sample bit-exactly"
          actions={<button className="btn small primary" onClick={zipAll} disabled={!d}>↓ Download all (.zip)</button>}
        >
          <div className="file-list">
            {d?.files.map((f) => (
              <div key={f.name} className={`file-item ${file?.name === f.name ? 'on' : ''}`} onClick={() => setSel(f.name)} role="button" tabIndex={0} onKeyDown={(e) => e.key === 'Enter' && setSel(f.name)}>
                <span className="mono">{f.name}</span>
                <span className="muted small" style={{ marginLeft: 'auto' }}>{f.content.split('\n').length} lines</span>
                <DownloadButton small filename={f.name} label="" content={f.content} />
              </div>
            ))}
          </div>
        </Card>
      </div>
      {file && (
        <Card title={file.name}>
          <Code code={file.content} filename={file.name} maxHeight={480} />
        </Card>
      )}
      <div className="grid grid-2">
        <Card title="From Protoforge to silicon" subtitle="Run these locally with the open-source tools">
          <div className="steps">
            {STEPS.map((s) => (
              <div key={s.title}>
                <b>{s.title}</b>
                <p className="lab-note" style={{ margin: '2px 0 6px' }}>{s.body}</p>
                {s.code && <Code code={s.code} />}
              </div>
            ))}
          </div>
        </Card>
        <div className="stack">
          <Explainer title="Tiny Tapeout" refs={[{ label: 'Tiny Tapeout', href: 'https://tinytapeout.com' }, { label: 'tt-support-tools', href: 'https://github.com/TinyTapeout/tt-support-tools' }]}>
            Tiny Tapeout aggregates hundreds of small designs onto one shared shuttle chip, so a student project can reach real silicon. Each design is
            a user module with 8 inputs, 8 outputs and 8 bidirectional pins behind a multiplexer; the flow (OpenLane / OpenROAD on the SkyWater
            sky130 open PDK) runs in GitHub Actions from the template repository. Field names in <span className="mono">info.yaml</span> change between
            shuttles, so Protoforge points you to the template instead of guessing them.
          </Explainer>
          <Explainer title="Graduating to professional tools" refs={[{ label: 'Yosys', href: 'https://github.com/YosysHQ/yosys' }, { label: 'Icarus Verilog', href: 'https://github.com/steveicarus/iverilog' }, { label: 'Verilator', href: 'https://github.com/verilator/verilator' }, { label: 'OpenROAD', href: 'https://github.com/The-OpenROAD-Project/OpenROAD' }, { label: 'cocotb', href: 'https://github.com/cocotb/cocotb' }]}>
            Protoforge's estimates tell you which architecture to pick; Yosys (synthesis), OpenSTA (timing), OpenROAD (place & route) and
            cocotb/Verilator (verification) are what you sign off with. The generated RTL was checked with Icarus Verilog and synthesised with Yosys while
            building this lab.
          </Explainer>
        </div>
      </div>
    </div>
  );
}
