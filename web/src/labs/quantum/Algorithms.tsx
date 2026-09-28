// Algorithms: Grover's search (amplitude amplification animated per
// iteration), the quantum Fourier transform and phase estimation,
// Deutsch–Jozsa and Bernstein–Vazirani.
import { useMemo, useState } from 'react';
import { Card, Chip, Explainer, Formula, Slider } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { circuitToGrid, fmtAngle } from './grid';
import { GateGlyph } from './Composer';
import type { Circuit, Entry, RunOut } from './types';
import { AmplitudeBars, Histogram, Kv, PhaseWheel, PlayBar, ProbBars, Seg, usePlayer } from './viz';

function MiniCircuit({ circuit, maxCols = 60 }: { circuit: Circuit; maxCols?: number }) {
  const g = useMemo(() => circuitToGrid(circuit), [circuit]);
  const cols = g.cols.slice(0, maxCols);
  return (
    <div className="qx-gridwrap">
      <div className="qx-grid" style={{ ['--cell' as string]: '30px' }}>
        <div className="qx-wirelabels">{Array.from({ length: g.qubits }, (_, q) => <div key={q} className="qx-wlabel mono small">q{q}</div>)}</div>
        <div className="qx-cols">
          {cols.map((col, ci) => {
            const filled = col.cells.map((c, q) => (c ? q : -1)).filter((q) => q >= 0);
            const multi = filled.length > 1 && col.cells.some((c) => c?.g === 'ctrl' || c?.g === 'swap');
            return (
              <div key={ci} className={`qx-col ${col.barrier ? 'barrier' : ''}`}>
                {multi && <div className="qx-vline" style={{ top: `calc(${Math.min(...filled) + 0.5} * var(--cell))`, height: `calc(${Math.max(...filled) - Math.min(...filled)} * var(--cell))` }} />}
                {Array.from({ length: g.qubits }, (_, q) => (
                  <div key={q} className="qx-cell" style={{ cursor: 'default' }}>
                    <div className="qx-wire" />
                    {col.cells[q] && <div className="qx-cellin" style={{ transform: 'scale(0.78)' }}><GateGlyph cell={col.cells[q]!} small /></div>}
                  </div>
                ))}
              </div>
            );
          })}
          {g.cols.length > maxCols && <div className="small muted" style={{ alignSelf: 'center', padding: 8 }}>… {g.cols.length - maxCols} more columns</div>}
        </div>
      </div>
    </div>
  );
}

const bits = (v: number, n: number) => v.toString(2).padStart(n, '0');

function Grover() {
  const [n, setN] = useState(3);
  const [marked, setMarked] = useState(5);
  const [iters, setIters] = useState<number | null>(null);
  const m = marked % (1 << n);
  const ent = useCompute<Entry>('quantum.algorithm', { id: 'grover', params: { n, value: m, iterations: iters ?? undefined, measure: false } });
  const x = ent.data?.extra;
  const kmax = x ? x.iterations.length : 1;
  const p = usePlayer(kmax, 0.9, { resetKey: `${n},${m}` });
  const k = Math.min(p.i, kmax - 1);
  const dist = x?.distributions[k] ?? [];
  const labels = dist.map((_, i) => bits(i, n));
  const theta = x?.theta ?? 0.3;
  const ang = (2 * k + 1) * theta;
  const angNext = (2 * Math.min(k + 1, kmax - 1) + 1) * theta;
  const angNow = ang + (angNext - ang) * (p.frac < 0.2 ? 0 : Math.min(1, (p.frac - 0.2) / 0.6));
  return (
    <div className="grid grid-sidebar">
      <Card title="Search problem">
        <Seg value={n} onChange={(v) => { setN(v); setIters(null); }} options={[2, 3, 4, 5, 6].map((q) => ({ value: q, label: `${q} qubits` }))} label="Search space N = 2ⁿ" />
        <Slider label="Marked item" value={m} min={0} max={(1 << n) - 1} step={1} format={(v) => `${bits(Math.round(v), n)} (${Math.round(v)})`} onChange={(v) => setMarked(Math.round(v))} />
        <Slider label="Iterations in the circuit" value={iters ?? x?.optimal ?? 1} min={0} max={Math.max(1, 2 * (x?.optimal ?? 1) + 2)} step={1} format={(v) => `${Math.round(v)}${Math.round(v) === x?.optimal ? ' (optimal)' : ''}`} onChange={(v) => setIters(Math.round(v))} />
        {ent.data && <Kv rows={ent.data.expected.map((e) => [e.label, e.value] as [string, string])} />}
        <div className="small muted" style={{ marginTop: 8 }}>{ent.data?.explanation}</div>
      </Card>
      <div className="stack">
        <Card title="Amplitude amplification, one iteration at a time" subtitle={`After ${k} iteration${k === 1 ? '' : 's'}: P(${bits(m, n)}) = ${((dist[m] ?? 0) * 100).toFixed(1)} %`}>
          <PlayBar p={p} label={(i) => `iteration ${i}`} />
          <div className="grid grid-2" style={{ marginTop: 10 }}>
            <div>
              <ProbBars labels={labels.length > 32 ? labels.slice(0, 32) : labels} values={dist.slice(0, 32)} highlight={m < 32 ? m : undefined} />
              {labels.length > 32 && <div className="small muted">Showing the first 32 of {labels.length} states.</div>}
            </div>
            <div>
              <GroverCircle angle={angNow} theta={theta} />
              <div className="small muted">Geometric picture: the state lives in the plane of |w⟩ (marked) and |s′⟩ (everything else). Each oracle + diffuser pair is two reflections = a rotation by 2θ toward |w⟩, with sin θ = 1/√N.</div>
            </div>
          </div>
        </Card>
        {x && (
          <Card title="Success probability vs iterations">
            <Plot
              series={[
                { name: 'theory sin²((2k+1)θ)', x: x.iterations.flatMap((i) => [i - 0.5, i, i + 0.5]).filter((v) => v >= 0), y: x.iterations.flatMap((i) => [i - 0.5, i, i + 0.5]).filter((v) => v >= 0).map((v) => Math.sin((2 * v + 1) * theta) ** 2), dash: '4 3' },
                { name: 'state-vector simulation', x: x.iterations, y: x.p_marked_sim, scatter: true },
              ]}
              vLines={[{ value: x.optimal, label: `optimal k = ${x.optimal}` }, { value: k, label: 'now', color: 'var(--accent)' }]}
              yDomain={[0, 1]} xLabel="iterations k" yLabel="P(marked)" height={220} legend
            />
          </Card>
        )}
        {ent.data && <Card title="The circuit" subtitle="H on every qubit, then k × (oracle, diffuser). Barriers separate iterations."><MiniCircuit circuit={ent.data.circuit} /></Card>}
      </div>
    </div>
  );
}

function GroverCircle({ angle, theta }: { angle: number; theta: number }) {
  const S = 200, c = 100, R = 80;
  const pt = (a: number, r = R) => [c + r * Math.cos(a), c - r * Math.sin(a)];
  const [vx, vy] = pt(angle);
  const [sx, sy] = pt(theta);
  return (
    <svg viewBox={`0 0 ${S} ${S}`} style={{ width: '100%', maxWidth: 240 }} role="img" aria-label="Grover rotation">
      <circle cx={c} cy={c} r={R} fill="none" stroke="var(--border-strong)" />
      <line x1={c - R - 8} x2={c + R + 8} y1={c} y2={c} stroke="var(--muted)" />
      <line y1={c - R - 8} y2={c + R + 8} x1={c} x2={c} stroke="var(--muted)" />
      <text x={c + R + 4} y={c - 4} fontSize={10} style={{ fill: 'var(--text-2)' }}>|s′⟩</text>
      <text x={c + 4} y={c - R - 2} fontSize={10} style={{ fill: 'var(--series-3)' }}>|w⟩ marked</text>
      <line x1={c} y1={c} x2={sx} y2={sy} stroke="var(--muted)" strokeDasharray="3 3" />
      <text x={sx + 4} y={sy} fontSize={9} style={{ fill: 'var(--muted)' }}>|s⟩ start</text>
      <line x1={c} y1={c} x2={vx} y2={vy} stroke="var(--accent)" strokeWidth={3} strokeLinecap="round" />
      <circle cx={vx} cy={vy} r={4} fill="var(--accent)" />
      <line x1={vx} y1={vy} x2={c} y2={vy} stroke="var(--series-3)" strokeDasharray="2 2" />
      <text x={10} y={S - 8} fontSize={9.5} style={{ fill: 'var(--text-2)' }}>angle = (2k+1)θ = {((angle * 180) / Math.PI).toFixed(1)}°</text>
    </svg>
  );
}

function Qft() {
  const [n, setN] = useState(3);
  const [x, setX] = useState(3);
  const xv = x % (1 << n);
  const ent = useCompute<Entry>('quantum.algorithm', { id: 'qft', params: { n, value: xv } });
  const run = useCompute<RunOut>('quantum.run', { circuit: ent.data?.circuit, shots: 16, seed: 1, steps: false }, !!ent.data);
  const [t, setT] = useState(4);
  const [phi, setPhi] = useState(0.3125);
  const qpe = useCompute<Entry>('quantum.algorithm', { id: 'qpe', params: { n: t, theta: phi } });
  const qrun = useCompute<RunOut>('quantum.run', { circuit: qpe.data?.circuit, shots: 2048, seed: 2, steps: false }, !!qpe.data);
  const st = run.data?.state;
  return (
    <div className="stack">
      <div className="grid grid-sidebar">
        <Card title="Quantum Fourier transform">
          <Seg value={n} onChange={setN} options={[2, 3, 4, 5].map((q) => ({ value: q, label: `${q} qubits` }))} label="Register size" />
          <Slider label="Input basis state x" value={xv} min={0} max={(1 << n) - 1} step={1} format={(v) => `|${bits(Math.round(v), n)}⟩ = ${Math.round(v)}`} onChange={(v) => setX(Math.round(v))} />
          <Explainer title="The transform">
            <Formula>{'QFT |x⟩ = 2^(−n/2) Σ_y exp(2πi·x·y / 2ⁿ) |y⟩'}</Formula>
            It is the discrete Fourier transform of the 2ⁿ amplitudes, built from n Hadamards and n(n−1)/2 controlled phases — exponentially fewer operations than the classical FFT's n·2ⁿ. The unit test checks it against a direct DFT to 10⁻⁹.
          </Explainer>
        </Card>
        <Card title={`QFT|${bits(xv, n)}⟩: equal magnitudes, phase winding x = ${xv} times`} actions={<PhaseWheel size={40} />}>
          {st && <AmplitudeBars labels={st.labels} re={st.re} im={st.im} />}
          <div className="small muted" style={{ marginTop: 6 }}>Every bar has height 1/2ⁿ; the colour (phase 2π·x·y/2ⁿ) advances by x steps of the wheel from one y to the next.</div>
          {ent.data && <div style={{ marginTop: 10 }}><MiniCircuit circuit={ent.data.circuit} /></div>}
        </Card>
      </div>
      <div className="grid grid-sidebar">
        <Card title="Phase estimation">
          <Seg value={t} onChange={setT} options={[2, 3, 4, 5, 6].map((q) => ({ value: q, label: `${q}` }))} label="Counting qubits t" />
          <Slider label="Eigenphase φ of U = P(2πφ)" value={phi} min={0} max={0.999} step={0.001} format={(v) => v.toFixed(4)} onChange={setPhi} />
          {qpe.data && <Kv rows={qpe.data.expected.map((e) => [e.label, e.value] as [string, string])} />}
          <div className="small muted" style={{ marginTop: 6 }}>{qpe.data?.explanation}</div>
        </Card>
        <Card title="Readout distribution" subtitle="Exact when φ·2ᵗ is an integer; otherwise it peaks at the nearest integer with ≥ 4/π² ≈ 40.5 % probability">
          {qrun.data && <Histogram entries={qrun.data.histogram} exact={qrun.data.exact} height={170} />}
          {qpe.data && <div style={{ marginTop: 8 }}><MiniCircuit circuit={qpe.data.circuit} maxCols={40} /></div>}
        </Card>
      </div>
    </div>
  );
}

function Oracles() {
  const [n, setN] = useState(3);
  const [kind, setKind] = useState<'constant0' | 'constant1' | 'balanced'>('balanced');
  const [mask, setMask] = useState(3);
  const [secret, setSecret] = useState(5);
  const dj = useCompute<Entry>('quantum.algorithm', { id: 'deutsch_jozsa', params: { n, kind, value: mask % (1 << n) || 1 } });
  const djr = useCompute<RunOut>('quantum.run', { circuit: dj.data?.circuit, shots: 256, seed: 3 }, !!dj.data);
  const bv = useCompute<Entry>('quantum.algorithm', { id: 'bernstein_vazirani', params: { n, value: secret % (1 << n) } });
  const bvr = useCompute<RunOut>('quantum.run', { circuit: bv.data?.circuit, shots: 256, seed: 3 }, !!bv.data);
  const toggle = (v: number, k: number) => v ^ (1 << k);
  return (
    <div className="stack">
      <Seg value={n} onChange={setN} options={[2, 3, 4, 5, 6].map((q) => ({ value: q, label: `${q} input qubits` }))} />
      <div className="grid grid-2">
        <Card title="Deutsch–Jozsa" subtitle="Is f constant or balanced? One query decides.">
          <Seg value={kind} onChange={setKind} options={[{ value: 'constant0', label: 'f = 0' }, { value: 'constant1', label: 'f = 1' }, { value: 'balanced', label: 'balanced f(x) = s·x' }]} />
          {kind === 'balanced' && (
            <div className="row small" style={{ marginBottom: 8 }}>
              <span className="muted">oracle mask s:</span>
              {Array.from({ length: n }, (_, k) => n - 1 - k).map((k) => (
                <button key={k} className={`btn small ${(mask >> k) & 1 ? 'primary' : ''}`} onClick={() => setMask((v) => toggle(v, k) || v)}>{(mask >> k) & 1}</button>
              ))}
            </div>
          )}
          {djr.data && <Histogram entries={djr.data.histogram} exact={djr.data.exact} height={140} />}
          {dj.data && (
            <div className="row small">
              <Chip tone="info">{dj.data.expected[0]?.value}</Chip>
            </div>
          )}
          <div className="small muted" style={{ marginTop: 6 }}>{dj.data?.explanation}</div>
          {dj.data && <div style={{ marginTop: 8 }}><MiniCircuit circuit={dj.data.circuit} /></div>}
        </Card>
        <Card title="Bernstein–Vazirani" subtitle="Read a hidden string s from f(x) = s·x mod 2 in one query">
          <div className="row small" style={{ marginBottom: 8 }}>
            <span className="muted">secret s:</span>
            {Array.from({ length: n }, (_, k) => n - 1 - k).map((k) => (
              <button key={k} className={`btn small ${(secret >> k) & 1 ? 'primary' : ''}`} onClick={() => setSecret((v) => toggle(v, k))}>{(secret >> k) & 1}</button>
            ))}
          </div>
          {bvr.data && <Histogram entries={bvr.data.histogram} exact={bvr.data.exact} height={140} />}
          {bv.data && <div className="row small"><Chip tone="pass">✓ {bv.data.expected[0]?.value}</Chip></div>}
          <div className="small muted" style={{ marginTop: 6 }}>{bv.data?.explanation}</div>
          {bv.data && <div style={{ marginTop: 8 }}><MiniCircuit circuit={bv.data.circuit} /></div>}
        </Card>
      </div>
      <Explainer title="Phase kickback, the trick behind both" refs={[{ label: 'Deutsch & Jozsa, Proc. R. Soc. A 439, 553 (1992)' }, { label: 'Bernstein & Vazirani, SIAM J. Comput. 26, 1411 (1997)' }, { label: 'Nielsen & Chuang §1.4' }]}>
          The ancilla starts in |−⟩ = (|0⟩ − |1⟩)/√2. XOR-ing f(x) into it multiplies the amplitude of |x⟩ by (−1)^f(x) and leaves the ancilla unchanged, so the oracle becomes a phase pattern on the input register:
          <Formula>{'|x⟩|−⟩  →  (−1)^f(x) |x⟩|−⟩'}</Formula>
          A final layer of Hadamards is a Fourier transform over bit strings, which turns the pattern (−1)^(s·x) into the single basis state |s⟩.
        </Explainer>
    </div>
  );
}

export default function Algorithms() {
  const [which, setWhich] = useState<'grover' | 'qft' | 'oracle'>('grover');
  return (
    <div className="stack">
      <Seg value={which} onChange={setWhich} options={[{ value: 'grover', label: 'Grover search' }, { value: 'qft', label: 'QFT & phase estimation' }, { value: 'oracle', label: 'Deutsch–Jozsa & Bernstein–Vazirani' }]} />
      {which === 'grover' && <Grover />}
      {which === 'qft' && <Qft />}
      {which === 'oracle' && <Oracles />}
      <div className="small muted">Angles in sliders: {fmtAngle(Math.PI / 4)} = 45°. All circuits open in the composer through its preset menu.</div>
    </div>
  );
}
