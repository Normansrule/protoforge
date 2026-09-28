// Noise: density-matrix simulation with Kraus channels, the Bloch vector
// shrinking column by column, fidelity vs depth and vs noise strength, and a
// single-qubit channel explorer that shows the Bloch sphere deforming.
import { useMemo, useState } from 'react';
import { Card, Chip, Explainer, Formula, Select, Slider, Toggle } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { gridToCircuit, type Grid } from './grid';
import type { ChannelOut, Circuit, DmOut, Entry, Noise as NoiseT, RunOut, SweepOut } from './types';
import { BlochSphere, Kv, PlayBar, ProbBars, Seg, usePlayer } from './viz';

type V3 = [number, number, number];

const CHANNELS: { key: keyof NoiseT; label: string; max: number; hint: string }[] = [
  { key: 'depolarizing', label: 'Depolarising p', max: 0.3, hint: 'ρ → (1−p)ρ + p·I/2 after every gate, on each qubit it touched' },
  { key: 'bit_flip', label: 'Bit flip p', max: 0.3, hint: 'X error with probability p' },
  { key: 'phase_flip', label: 'Phase flip p', max: 0.3, hint: 'Z error with probability p' },
  { key: 'amplitude_damping', label: 'Amplitude damping γ (T₁)', max: 0.3, hint: 'energy relaxation |1⟩ → |0⟩' },
  { key: 'phase_damping', label: 'Phase damping λ (T₂)', max: 0.3, hint: 'loss of phase coherence without energy loss' },
  { key: 'readout', label: 'Readout error', max: 0.2, hint: 'each measured bit read wrong with this probability' },
];

const ZERO: NoiseT = { depolarizing: 0, bit_flip: 0, phase_flip: 0, amplitude_damping: 0, phase_damping: 0, readout: 0, idle: false };

function composerCircuit(): Circuit | null {
  try {
    const s = localStorage.getItem('protoforge.quantum.grid.v1');
    if (!s) return null;
    const { circuit, errors } = gridToCircuit(JSON.parse(s) as Grid);
    return errors.length ? null : circuit;
  } catch {
    return null;
  }
}

export default function Noise() {
  const [src, setSrc] = useState('bell');
  const [noise, setNoise] = useState<NoiseT>({ ...ZERO, depolarizing: 0.05, amplitude_damping: 0.02 });
  const params = src === 'ghz' ? { n: 4, measure: false } : src === 'grover' ? { n: 3, value: 5, measure: false } : src === 'qft' ? { n: 3, value: 5 } : { measure: false };
  const lib = useCompute<Entry>('quantum.algorithm', { id: src, params }, src !== 'composer');
  const circuit = useMemo(() => (src === 'composer' ? composerCircuit() : lib.data?.circuit ?? null), [src, lib.data]);
  const tooBig = circuit ? circuit.qubits > 6 : false;
  const dm = useCompute<DmOut>('quantum.noise', { circuit, noise }, !!circuit && !tooBig);
  const ideal = useCompute<RunOut>('quantum.run', { circuit, shots: 16, seed: 1, steps: true }, !!circuit && !tooBig);
  const [sweepCh, setSweepCh] = useState<keyof NoiseT>('depolarizing');
  const sweep = useCompute<SweepOut>('quantum.noise_sweep', { circuit, channel: sweepCh, pmax: 0.3, points: 16 }, !!circuit && !tooBig);
  const d = dm.data;
  const nSteps = d?.steps.length ?? 1;
  const p = usePlayer(nSteps, 1.3, { resetKey: `${src}` });
  const st = d?.steps[Math.min(p.i, nSteps - 1)];
  const idealSt = ideal.data?.steps[Math.min(p.i, (ideal.data?.steps.length ?? 1) - 1)];

  // single-qubit channel explorer
  const [chKey, setChKey] = useState<keyof NoiseT>('amplitude_damping');
  const [chP, setChP] = useState(0.15);
  const [start, setStart] = useState<'plus' | 'one' | 'tilt'>('plus');
  const startV: V3 = start === 'plus' ? [1, 0, 0] : start === 'one' ? [0, 0, -1] : [0.6, 0.48, 0.64];
  const ch = useCompute<ChannelOut>('quantum.channel', { noise: { ...ZERO, [chKey]: chP }, start: startV, steps: 20 });
  const cp = usePlayer(21, 3, { resetKey: `${chKey},${chP},${start}` });

  const set = (k: keyof NoiseT, v: number | boolean) => setNoise((n) => ({ ...n, [k]: v }));
  return (
    <div className="stack">
      <div className="grid grid-sidebar">
        <Card title="Noise model" subtitle="Applied after every column to each qubit a gate touched (Kraus operators, Nielsen & Chuang §8.3)">
          <Select
            label="Circuit"
            value={src}
            onChange={setSrc}
            options={[
              { value: 'bell', label: 'Bell pair (2 qubits)' },
              { value: 'ghz', label: 'GHZ (4 qubits)' },
              { value: 'teleport_deferred', label: 'Teleportation, deferred form (3)' },
              { value: 'grover', label: 'Grover, N = 8 (3)' },
              { value: 'qft', label: 'QFT (3)' },
              { value: 'composer', label: 'The circuit in the composer' },
            ]}
          />
          {CHANNELS.map((c) => (
            <Slider key={c.key} label={c.label} value={noise[c.key] as number} min={0} max={c.max} step={0.005} hint={c.hint} onChange={(v) => set(c.key, v)} />
          ))}
          <Toggle label="Idle qubits also decay (T₁/T₂ every column)" value={noise.idle} onChange={(v) => set('idle', v)} />
          <div className="row">
            <button className="btn small" onClick={() => setNoise(ZERO)}>Noiseless</button>
            <button className="btn small" onClick={() => setNoise({ ...ZERO, depolarizing: 0.01, amplitude_damping: 0.005, phase_damping: 0.01, readout: 0.02, idle: true })}>Typical NISQ-ish</button>
          </div>
          {tooBig && <div className="small" style={{ color: 'var(--warn)' }}>! The density-matrix simulator handles at most 6 qubits.</div>}
          {!circuit && src === 'composer' && <div className="small muted">Build a circuit in the composer first.</div>}
        </Card>
        <div className="stack">
          <Card title="Bloch vectors shrink as coherence leaks away" subtitle="Solid: noisy qubit. Dashed: the ideal (noise-free) qubit at the same column.">
            {d && <PlayBar p={p} label={(i) => (i === 0 ? 'start' : `column ${i}`)} />}
            <div className="qx-blochs" style={{ marginTop: 8 }}>
              {st?.qubits.map((q, k) => (
                <BlochSphere key={k} v={q.bloch as V3} ghost={(idealSt?.qubits[k]?.bloch as V3) ?? null} size={d && d.qubits > 3 ? 120 : 145} label={`q${k}`} sub={`|r| = ${q.radius.toFixed(3)}`} />
              ))}
            </div>
            {d && st && (
              <div className="row small" style={{ marginTop: 6 }}>
                <Chip tone={st.fidelity != null && st.fidelity > 0.9 ? 'pass' : 'warn'}>fidelity {st.fidelity != null ? st.fidelity.toFixed(4) : '—'}</Chip>
                <Chip>purity Tr ρ² = {st.purity.toFixed(4)}</Chip>
              </div>
            )}
          </Card>
          {d && (
            <div className="grid grid-2">
              <Card title="Fidelity and purity vs depth">
                <Plot
                  series={[
                    { name: 'fidelity with ideal', x: d.steps.map((s) => s.column), y: d.steps.map((s) => s.fidelity ?? NaN) },
                    { name: 'purity Tr ρ²', x: d.steps.map((s) => s.column), y: d.steps.map((s) => s.purity), dash: '4 3' },
                  ]}
                  vLines={[{ value: p.i, label: '', color: 'var(--accent)' }]}
                  yDomain={[0, 1.02]} xLabel="column" height={210} legend
                />
              </Card>
              <Card title="Outcome probabilities" subtitle="Bars: noisy (with readout error). Ticks: ideal.">
                <ProbBars labels={d.labels} values={d.probs_measured} compare={d.probs_ideal} />
                <Kv rows={[
                  ['Final fidelity', d.fidelity != null ? d.fidelity.toFixed(4) : '—'],
                  ['Total-variation distance', d.total_variation.toFixed(4)],
                  ['von Neumann entropy S(ρ)', `${d.entropy.toFixed(3)} bit`],
                  ...(d.concurrence01 != null ? [['Concurrence q0–q1 (ideal → noisy)', `${(d.concurrence01_ideal ?? 0).toFixed(3)} → ${d.concurrence01.toFixed(3)}`] as [string, string]] : []),
                ]} />
              </Card>
            </div>
          )}
          {d?.rho_re && d.rho_im && <DensityMatrix re={d.rho_re} im={d.rho_im} labels={d.labels} />}
        </div>
      </div>

      <div className="grid grid-2">
        <Card title="Fidelity vs noise strength" subtitle={sweep.data ? `Success = probability of the most likely ideal outcome |${sweep.data.target}⟩` : ''}>
          <Select value={sweepCh} onChange={(v) => setSweepCh(v as keyof NoiseT)} options={CHANNELS.map((c) => ({ value: c.key, label: c.label }))} />
          {sweep.data && (
            <Plot
              series={[
                { name: 'fidelity', x: sweep.data.p, y: sweep.data.fidelity },
                { name: 'success probability', x: sweep.data.p, y: sweep.data.success, dash: '4 3' },
                { name: 'purity', x: sweep.data.p, y: sweep.data.purity, dash: '1 3' },
              ]}
              yDomain={[0, 1.02]} xLabel="error per gate" height={220} legend
            />
          )}
        </Card>
        <Card title="One channel on one qubit" subtitle="Orange wireframe: where the whole Bloch sphere goes after one use. Arrow: a state after repeated uses.">
          <div className="grid grid-2">
            <div>
              <Select value={chKey} onChange={(v) => setChKey(v as keyof NoiseT)} options={CHANNELS.filter((c) => c.key !== 'readout').map((c) => ({ value: c.key, label: c.label }))} />
              <Slider label="Strength" value={chP} min={0} max={0.6} step={0.01} onChange={setChP} />
              <Seg value={start} onChange={setStart} options={[{ value: 'plus', label: '|+⟩' }, { value: 'one', label: '|1⟩' }, { value: 'tilt', label: 'tilted' }]} label="Start" />
              {ch.data && <PlayBar p={cp} label={(i) => `${i} uses`} />}
            </div>
            {ch.data && (
              <BlochSphere
                v={ch.data.trajectory[Math.min(cp.i, ch.data.trajectory.length - 1)]}
                trail={ch.data.trajectory.slice(0, cp.i + 1)}
                ellipsoid={{ m: ch.data.m, t: ch.data.t }}
                size={200}
                sub={`purity ${ch.data.purity[Math.min(cp.i, ch.data.purity.length - 1)].toFixed(3)}`}
              />
            )}
          </div>
          {ch.data && <div className="small muted mono">r → M r + t, M = diag({ch.data.m.map((r, i) => r[i].toFixed(3)).join(', ')}), t = ({ch.data.t.map((x) => x.toFixed(3)).join(', ')})</div>}
        </Card>
      </div>

      <div className="grid grid-2">
        <Explainer title="Density matrices and Kraus operators" refs={[{ label: 'Nielsen & Chuang (2010) §2.4, §8.3' }, { label: 'Preskill Ph219 ch. 3', href: 'http://theory.caltech.edu/~preskill/ph219/' }]}>
          A noisy state is a density matrix ρ = Σ pᵢ|ψᵢ⟩⟨ψᵢ|. Any physical noise process is a set of Kraus operators with Σ K†K = I:
          <Formula>{'ρ → Σₖ Kₖ ρ Kₖ†\namplitude damping: K₀ = [[1,0],[0,√(1−γ)]], K₁ = [[0,√γ],[0,0]]\nphase damping:     K₀ = [[1,0],[0,√(1−λ)]], K₁ = [[0,0],[0,√λ]]'}</Formula>
          On the Bloch ball every single-qubit channel is an affine map r → M r + t: depolarising shrinks it uniformly, dephasing squashes it onto the Z axis, amplitude damping squashes and pulls it toward |0⟩.
        </Explainer>
        <Explainer title="Fidelity" refs={[{ label: 'Jozsa, J. Mod. Opt. 41, 2315 (1994)' }]}>
          For a pure ideal state F = ⟨ψ|ρ|ψ⟩; in general F(ρ, σ) = (Tr √(√ρ σ √ρ))², computed here by Hermitian eigen-decomposition. In a quantum link the same number decides everything: teleportation and entanglement swapping only beat classical strategies when the shared pairs have F &gt; 1/2 (Bell-pair fidelity), and QKD's error rate is roughly (1 − F)·(2/3) for depolarising noise.
        </Explainer>
      </div>
    </div>
  );
}

function DensityMatrix({ re, im, labels }: { re: number[]; im: number[]; labels: string[] }) {
  const d = labels.length;
  return (
    <Card title="Density matrix ρ" subtitle="Cell brightness |ρᵢⱼ|, colour = phase. Diagonal: populations. Off-diagonal: coherences — the part noise destroys first.">
      <div className="table-wrap">
        <div className="qx-cmat" style={{ gridTemplateColumns: `42px repeat(${d}, minmax(34px, 1fr))`, minWidth: 42 + d * 36 }}>
          <span />
          {labels.map((l) => <span key={l} className="mono small muted" style={{ textAlign: 'center' }}>{l}</span>)}
          {labels.map((l, i) => (
            <div key={l} style={{ display: 'contents' }}>
              <span className="mono small muted">{l}</span>
              {labels.map((_, j) => {
                const a = re[i * d + j], b = im[i * d + j];
                const m = Math.hypot(a, b);
                const ph = Math.atan2(b, a);
                return (
                  <span key={j} className="qx-cm" title={`ρ[${i}][${j}] = ${a.toFixed(3)} ${b < 0 ? '−' : '+'} ${Math.abs(b).toFixed(3)}i`} style={{ background: m < 1e-4 ? 'var(--surface-2)' : `color-mix(in srgb, hsl(${((ph * 180) / Math.PI + 360) % 360} 70% 50%) ${Math.round(Math.min(1, m * 2) * 85)}%, var(--surface-2))` }}>
                    {m >= 0.005 ? m.toFixed(2) : ''}
                  </span>
                );
              })}
            </div>
          ))}
        </div>
      </div>
    </Card>
  );
}
