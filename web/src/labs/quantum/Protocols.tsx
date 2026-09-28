// Protocols told as animated stories: Alice / Charlie / Bob lanes, qubits
// flying down the quantum channel, classical bits as packets on the classical
// channel — with the simulated state (Bloch spheres, concurrence) at every scene.
import { useMemo, useState } from 'react';
import { Card, Chip, Explainer, Formula, Slider, StatusChip } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { circuitToGrid, fmtAngle } from './grid';
import { GateGlyph } from './Composer';
import type { ChshOut, Circuit, Entry, RunOut, Snapshot } from './types';
import { BlochSphere, Gauge, Histogram, PlayBar, Seg, usePlayer } from './viz';

type V3 = [number, number, number];

interface Tok { x: number; y: number; kind: 'q' | 'b'; label: string; tone?: string; faded?: boolean }
interface Scene { title: string; text: string; step: number; tokens: Record<string, Tok>; links: [string, string][]; gates?: { x: number; y: number; label: string }[] }

const W = 720, H = 250;
const QY = 112, CY = 208;

/** Index (in run.steps) after the last column containing an op matching `pred`. */
function after(c: Circuit, pred: (label: string, gate: string) => boolean): number {
  let k = 0;
  c.cols.forEach((col, i) => {
    if (col.some((o) => pred(o.label ?? '', o.gate))) k = i + 1;
  });
  return k;
}

function StoryStage({ lanes, scenes, p }: { lanes: { x: number; name: string; role: string }[]; scenes: Scene[]; p: { i: number; frac: number } }) {
  const a = scenes[Math.min(p.i, scenes.length - 1)];
  const b = scenes[Math.min(p.i + 1, scenes.length - 1)];
  const t = p.frac < 0.15 ? 0 : Math.min(1, (p.frac - 0.15) / 0.7);
  const e = t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2;
  const ids = new Set([...Object.keys(a.tokens), ...Object.keys(b.tokens)]);
  const pos: Record<string, { x: number; y: number; op: number; tok: Tok }> = {};
  ids.forEach((id) => {
    const ta = a.tokens[id], tb = b.tokens[id];
    if (ta && tb) pos[id] = { x: ta.x + (tb.x - ta.x) * e, y: ta.y + (tb.y - ta.y) * e, op: 1, tok: e < 0.5 ? ta : tb };
    else if (ta) pos[id] = { x: ta.x, y: ta.y, op: 1 - e, tok: ta };
    else if (tb) pos[id] = { x: tb.x, y: tb.y, op: e, tok: tb };
  });
  const links = e < 0.5 ? a.links : b.links;
  const gates = e < 0.5 ? a.gates : b.gates;
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="qx-story" role="img" aria-label={a.title}>
      {lanes.map((l) => (
        <g key={l.name}>
          <rect x={l.x - 70} y={14} width={140} height={H - 28} rx={12} fill="var(--surface-1)" stroke="var(--border)" />
          <text x={l.x} y={34} textAnchor="middle" className="qx-lane-label">{l.name}</text>
          <text x={l.x} y={49} textAnchor="middle" fontSize={10} fill="var(--muted)" style={{ fill: 'var(--muted)' }}>{l.role}</text>
        </g>
      ))}
      <line x1={40} x2={W - 40} y1={QY} y2={QY} stroke="var(--series-1)" strokeWidth={2} opacity={0.35} />
      <text x={480} y={QY + 16} fontSize={9.5} textAnchor="middle" style={{ fill: 'var(--series-1)' }}>quantum channel (fiber)</text>
      <line x1={40} x2={W - 40} y1={CY} y2={CY} stroke="var(--series-4)" strokeWidth={1.5} strokeDasharray="6 5" opacity={0.6} />
      <text x={W / 2} y={CY + 16} fontSize={9.5} textAnchor="middle" style={{ fill: 'var(--series-4)' }}>classical channel (≤ speed of light)</text>
      {links.map(([u, v]) => {
        const A = pos[u], B = pos[v];
        if (!A || !B) return null;
        const mx = (A.x + B.x) / 2, my = Math.min(A.y, B.y) - 26 - Math.abs(A.x - B.x) * 0.08;
        return <path key={u + v} d={`M${A.x} ${A.y} Q${mx} ${my} ${B.x} ${B.y}`} fill="none" stroke="var(--series-5)" strokeWidth={2} strokeDasharray="3 4" opacity={Math.min(A.op, B.op)} className="flow-line" />;
      })}
      {gates?.map((g, k) => (
        <g key={k}>
          <rect x={g.x - 26} y={g.y - 11} width={52} height={22} rx={6} fill="var(--surface-3)" stroke="var(--border-strong)" />
          <text x={g.x} y={g.y + 4} textAnchor="middle" fontSize={10.5} fontWeight={700}>{g.label}</text>
        </g>
      ))}
      {Object.entries(pos).map(([id, { x, y, op, tok }]) =>
        tok.kind === 'q' ? (
          <g key={id} opacity={op * (tok.faded ? 0.4 : 1)} transform={`translate(${x} ${y})`}>
            <circle r={14} fill={tok.tone ?? 'var(--series-1)'} opacity={0.9} />
            <circle r={14} fill="none" stroke="var(--surface-1)" strokeWidth={1.5} />
            <text textAnchor="middle" y={4} fontSize={10.5} fontWeight={700} style={{ fill: '#fff' }}>{tok.label}</text>
          </g>
        ) : (
          <g key={id} opacity={op} transform={`translate(${x} ${y})`}>
            <rect x={-13} y={-11} width={26} height={22} rx={4} fill="var(--series-4)" />
            <text textAnchor="middle" y={4.5} fontSize={12} fontWeight={800} style={{ fill: '#111' }}>{tok.label}</text>
          </g>
        ),
      )}
    </svg>
  );
}

function CircuitStrip({ circuit, active }: { circuit: Circuit; active: number }) {
  const g = useMemo(() => circuitToGrid(circuit), [circuit]);
  return (
    <div className="qx-gridwrap" style={{ marginTop: 8 }}>
      <div className="qx-grid" style={{ ['--cell' as string]: '34px' }}>
        <div className="qx-wirelabels">
          {Array.from({ length: g.qubits }, (_, q) => <div key={q} className="qx-wlabel mono">q{q}</div>)}
        </div>
        <div className="qx-cols">
          {g.cols.map((col, ci) => {
            const filled = col.cells.map((c, q) => (c ? q : -1)).filter((q) => q >= 0);
            const multi = filled.length > 1 && col.cells.some((c) => c?.g === 'ctrl' || c?.g === 'swap');
            return (
              <div key={ci} className={`qx-col ${ci < active ? '' : ''} ${ci === active - 1 ? 'active' : ''} ${col.barrier ? 'barrier' : ''}`} style={{ opacity: ci < active ? 1 : 0.45 }}>
                {multi && <div className="qx-vline" style={{ top: `calc(${Math.min(...filled) + 0.5} * var(--cell))`, height: `calc(${Math.max(...filled) - Math.min(...filled)} * var(--cell))` }} />}
                {Array.from({ length: g.qubits }, (_, q) => (
                  <div key={q} className="qx-cell" style={{ cursor: 'default' }}>
                    <div className="qx-wire" />
                    {col.cells[q] && <div className="qx-cellin" style={{ transform: 'scale(0.85)' }}><GateGlyph cell={col.cells[q]!} small /></div>}
                  </div>
                ))}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

const bl = (s: Snapshot | undefined, q: number): V3 => (s ? (s.qubits[q].bloch as V3) : [0, 0, 1]);
const fid = (u: V3, v: V3) => (1 + u[0] * v[0] + u[1] * v[1] + u[2] * v[2]) / 2;

// ---------------------------------------------------------------------------

function Teleport() {
  const [theta, setTheta] = useState(1.1);
  const [phi, setPhi] = useState(0.7);
  const [seed, setSeed] = useState(3);
  const ent = useCompute<Entry>('quantum.algorithm', { id: 'teleport', params: { theta, phi } });
  const run = useCompute<RunOut>('quantum.run', { circuit: ent.data?.circuit, shots: 400, seed, steps: true }, !!ent.data);
  const c = ent.data?.circuit;
  const steps = run.data?.steps ?? [];
  const sPrep = c ? after(c, (l) => l.startsWith('prepare')) : 0;
  const sPair = c ? after(c, (l) => l === 'Bell pair') : 0;
  const sBm = c ? after(c, (l) => l === 'Bell measurement') : 0;
  const sMeas = c ? after(c, (_, g) => g === 'measure') : 0;
  const sEnd = c ? c.cols.length : 0;
  const bits = steps[sMeas]?.clbits ?? [0, 0];
  const m0 = String(bits[0] ?? 0), m1 = String(bits[1] ?? 0);
  const A = 120, S = 360, B = 600;
  const psiTone = 'var(--series-2)';
  const scenes: Scene[] = [
    { title: 'Alice holds an unknown state |ψ⟩', text: `Alice's qubit q0 is prepared as U(θ = ${fmtAngle(theta)}, φ = ${fmtAngle(phi)}). She does not know it, and measuring it would destroy it; copying it is impossible (no-cloning).`, step: sPrep,
      tokens: { q0: { x: A, y: 80, kind: 'q', label: 'ψ', tone: psiTone }, q1: { x: S - 18, y: QY, kind: 'q', label: 'q1' }, q2: { x: S + 18, y: QY, kind: 'q', label: 'q2' } }, links: [] },
    { title: 'A source shares a Bell pair', text: 'H and CNOT make |Φ+⟩ = (|00⟩ + |11⟩)/√2 from q1, q2. One photon goes to Alice, the other to Bob over the fiber. Each alone is completely random: Bob\'s Bloch vector is at the centre.', step: sPair,
      tokens: { q0: { x: A, y: 80, kind: 'q', label: 'ψ', tone: psiTone }, q1: { x: A, y: QY + 34, kind: 'q', label: 'q1' }, q2: { x: B, y: QY, kind: 'q', label: 'q2' } }, links: [['q1', 'q2']] },
    { title: 'Alice\'s Bell-basis measurement', text: 'CNOT(q0 → q1) then H(q0) rotate the Bell basis onto the computational basis. Now all three qubits are entangled; Bob\'s qubit already holds |ψ⟩ up to one of four Pauli errors.', step: sBm,
      tokens: { q0: { x: A - 18, y: 92, kind: 'q', label: 'ψ', tone: psiTone }, q1: { x: A + 18, y: 92, kind: 'q', label: 'q1' }, q2: { x: B, y: QY, kind: 'q', label: 'q2' } }, links: [['q0', 'q2'], ['q1', 'q2']], gates: [{ x: A, y: 64, label: 'CX · H' }] },
    { title: 'Alice measures two classical bits', text: `Outcomes m0 = ${m0}, m1 = ${m1} (each of the four pairs has probability ¼, whatever |ψ⟩ is — so they reveal nothing about it). Alice's qubits are now just classical records.`, step: sMeas,
      tokens: { q0: { x: A - 18, y: 92, kind: 'q', label: m0, faded: true }, q1: { x: A + 18, y: 92, kind: 'q', label: m1, faded: true }, q2: { x: B, y: QY, kind: 'q', label: 'q2' }, b0: { x: A - 16, y: CY, kind: 'b', label: m0 }, b1: { x: A + 16, y: CY, kind: 'b', label: m1 } }, links: [] },
    { title: 'Two bits travel to Bob', text: 'Teleportation cannot beat light: until the bits arrive, Bob\'s qubit (averaged over the outcomes he does not know yet) is maximally mixed.', step: sMeas,
      tokens: { q0: { x: A - 18, y: 92, kind: 'q', label: m0, faded: true }, q1: { x: A + 18, y: 92, kind: 'q', label: m1, faded: true }, q2: { x: B, y: QY, kind: 'q', label: 'q2' }, b0: { x: B - 16, y: CY, kind: 'b', label: m0 }, b1: { x: B + 16, y: CY, kind: 'b', label: m1 } }, links: [] },
    { title: 'Bob corrects: X^m1 then Z^m0', text: 'Bob applies X if m1 = 1 and Z if m0 = 1. His qubit is now exactly |ψ⟩ — fidelity 1. The state moved; it was never copied (Alice\'s original is gone).', step: sEnd,
      tokens: { q0: { x: A - 18, y: 92, kind: 'q', label: m0, faded: true }, q1: { x: A + 18, y: 92, kind: 'q', label: m1, faded: true }, q2: { x: B, y: QY, kind: 'q', label: 'ψ', tone: psiTone }, b0: { x: B - 16, y: CY, kind: 'b', label: m0 }, b1: { x: B + 16, y: CY, kind: 'b', label: m1 } }, links: [], gates: [{ x: B, y: 70, label: `${m1 === '1' ? 'X' : ''}${m0 === '1' ? 'Z' : ''}` || 'I' }] },
  ];
  const p = usePlayer(scenes.length, 0.45, { resetKey: `${theta},${phi},${seed}` });
  const sc = scenes[p.i];
  const psi = bl(steps[sPrep], 0);
  const bob = bl(steps[sc.step], 2);
  // Before the bits arrive Bob's best description is the average over outcomes: the maximally mixed state.
  const bobKnown: V3 = p.i === 4 || (p.i < 5 && p.i >= 1) ? [0, 0, 0] : bob;
  return (
    <div className="grid grid-sidebar">
      <div className="stack">
        <Card title="Your state |ψ⟩">
          <Slider label="θ (polar angle)" value={theta} min={0} max={Math.PI} step={Math.PI / 48} format={fmtAngle} onChange={setTheta} />
          <Slider label="φ (azimuth)" value={phi} min={-Math.PI} max={Math.PI} step={Math.PI / 48} format={fmtAngle} onChange={setPhi} />
          <button className="btn small" onClick={() => setSeed((s) => s + 1)}>New measurement outcomes</button>
          <div className="row" style={{ justifyContent: 'space-around', marginTop: 10 }}>
            <BlochSphere v={psi} size={130} label="Alice's |ψ⟩" color="var(--series-2)" />
            <BlochSphere v={bobKnown} ghost={psi} size={130} label="Bob's q2" sub={`fidelity ${fid(psi, bobKnown).toFixed(3)}`} color="var(--series-1)" />
          </div>
          <div className="small muted">Dashed arrow: the target |ψ⟩. Bob's arrow is what Bob can know at this scene.</div>
        </Card>
      </div>
      <Card title="Quantum teleportation" subtitle="Bennett, Brassard, Crépeau, Jozsa, Peres & Wootters (1993)">
        <StoryStage lanes={[{ x: A, name: 'Alice', role: 'sender' }, { x: S, name: 'Source', role: 'entangled-pair source' }, { x: B, name: 'Bob', role: 'receiver' }]} scenes={scenes} p={p} />
        <PlayBar p={p} label={(i) => `scene ${i + 1} / ${scenes.length}`} />
        <div className="qx-caption"><b>{sc.title}.</b> <span className="small">{sc.text}</span></div>
        {c && <CircuitStrip circuit={c} active={sc.step} />}
        {run.data && (
          <div className="row small" style={{ marginTop: 8 }}>
            <Chip tone="pass">✓ fidelity after correction: {fid(psi, bl(steps[sEnd], 2)).toFixed(6)}</Chip>
            <span className="muted">Try every outcome with “New measurement outcomes” — the fidelity stays 1.</span>
          </div>
        )}
      </Card>
    </div>
  );
}

// ---------------------------------------------------------------------------

function Superdense() {
  const [msg, setMsg] = useState(2);
  const ent = useCompute<Entry>('quantum.algorithm', { id: 'superdense', params: { message: msg } });
  const run = useCompute<RunOut>('quantum.run', { circuit: ent.data?.circuit, shots: 512, seed: 1, steps: true }, !!ent.data);
  const c = ent.data?.circuit;
  const A = 120, S = 360, B = 600;
  const bitsTxt = msg.toString(2).padStart(2, '0');
  const enc = `${msg & 2 ? 'X' : ''}${msg & 1 ? 'Z' : ''}` || 'I';
  const sEnc = c ? after(c, (l) => l === 'encode') || after(c, (l) => l === 'Bell pair') + 1 : 0;
  const scenes: Scene[] = [
    { title: 'Share a Bell pair in advance', text: 'Alice and Bob each hold half of |Φ+⟩ — distributed earlier, before Alice knows her message.', step: c ? after(c, (l) => l === 'Bell pair') : 0,
      tokens: { q0: { x: A, y: QY, kind: 'q', label: 'q0' }, q1: { x: B, y: QY, kind: 'q', label: 'q1' } }, links: [['q0', 'q1']] },
    { title: `Alice encodes two bits: ${bitsTxt}`, text: `One of four operations on her half — I, X, Z or XZ — turns |Φ+⟩ into one of the four orthogonal Bell states. For ${bitsTxt} she applies ${enc}.`, step: sEnc,
      tokens: { q0: { x: A, y: QY, kind: 'q', label: 'q0', tone: 'var(--series-2)' }, q1: { x: B, y: QY, kind: 'q', label: 'q1' }, m: { x: A, y: 70, kind: 'b', label: bitsTxt } }, links: [['q0', 'q1']], gates: [{ x: A, y: 150, label: enc }] },
    { title: 'She sends one qubit', text: 'Only one qubit crosses the channel.', step: sEnc,
      tokens: { q0: { x: B - 30, y: QY, kind: 'q', label: 'q0', tone: 'var(--series-2)' }, q1: { x: B + 16, y: QY, kind: 'q', label: 'q1' } }, links: [['q0', 'q1']] },
    { title: 'Bob decodes both bits', text: 'CNOT then H map the four Bell states to 00, 01, 10, 11: Bob reads two bits from one transmitted qubit (Bennett & Wiesner 1992).', step: c ? c.cols.length : 0,
      tokens: { q0: { x: B - 30, y: QY, kind: 'q', label: 'q0', faded: true }, q1: { x: B + 16, y: QY, kind: 'q', label: 'q1', faded: true }, r: { x: B, y: 160, kind: 'b', label: bitsTxt } }, links: [], gates: [{ x: B, y: 70, label: 'CX · H' }] },
  ];
  const p = usePlayer(scenes.length, 0.45, { resetKey: msg });
  const sc = scenes[p.i];
  return (
    <div className="grid grid-sidebar">
      <Card title="Message">
        <Seg value={msg} onChange={setMsg} options={[0, 1, 2, 3].map((m) => ({ value: m, label: m.toString(2).padStart(2, '0') }))} label="Two classical bits c1 c0" />
        {run.data && <Histogram entries={run.data.histogram} exact={run.data.exact} height={140} />}
        <div className="small muted">Bob's measured c1 c0 over 512 shots: always the message.</div>
      </Card>
      <Card title="Superdense coding" subtitle="Two bits per qubit sent, paid for with one pre-shared Bell pair">
        <StoryStage lanes={[{ x: A, name: 'Alice', role: 'sender' }, { x: S, name: '', role: '' }, { x: B, name: 'Bob', role: 'receiver' }]} scenes={scenes} p={p} />
        <PlayBar p={p} label={(i) => `scene ${i + 1} / ${scenes.length}`} />
        <div className="qx-caption"><b>{sc.title}.</b> <span className="small">{sc.text}</span></div>
        {c && <CircuitStrip circuit={c} active={sc.step} />}
      </Card>
    </div>
  );
}

// ---------------------------------------------------------------------------

function Swapping() {
  const [seed, setSeed] = useState(2);
  const ent = useCompute<Entry>('quantum.algorithm', { id: 'swap_ent', params: {} });
  const run = useCompute<RunOut>('quantum.run', { circuit: ent.data?.circuit, shots: 256, seed, steps: true }, !!ent.data);
  const c = ent.data?.circuit;
  const steps = run.data?.steps ?? [];
  const A = 110, C = 360, B = 610;
  const sPairs = c ? after(c, (l) => l.startsWith('pair')) : 0;
  const sBm = c ? after(c, (l) => l.startsWith("Charlie's")) : 0;
  const sMeas = c ? after(c, (_, g) => g === 'measure') : 0;
  const sEnd = c ? c.cols.length : 0;
  const bits = steps[sMeas]?.clbits ?? [0, 0];
  const m0 = String(bits[0] ?? 0), m1 = String(bits[1] ?? 0);
  const conc = (s: Snapshot | undefined, i: number, j: number) => (s?.concurrence ? s.concurrence[i * 4 + j] : 0);
  const scenes: Scene[] = [
    { title: 'Two independent Bell pairs', text: 'Source 1 entangles q0–q1, source 2 entangles q2–q3. Alice gets q0, Bob gets q3, the relay node Charlie gets q1 and q2.', step: sPairs,
      tokens: { q0: { x: A, y: QY, kind: 'q', label: 'q0' }, q1: { x: C - 26, y: QY, kind: 'q', label: 'q1' }, q2: { x: C + 26, y: QY, kind: 'q', label: 'q2' }, q3: { x: B, y: QY, kind: 'q', label: 'q3' } }, links: [['q0', 'q1'], ['q2', 'q3']] },
    { title: "Charlie's Bell measurement", text: 'CNOT(q1 → q2), H(q1): Charlie measures his two qubits in the Bell basis. He never touches q0 or q3.', step: sBm,
      tokens: { q0: { x: A, y: QY, kind: 'q', label: 'q0' }, q1: { x: C - 18, y: QY + 30, kind: 'q', label: 'q1' }, q2: { x: C + 18, y: QY + 30, kind: 'q', label: 'q2' }, q3: { x: B, y: QY, kind: 'q', label: 'q3' } }, links: [['q0', 'q1'], ['q1', 'q2'], ['q2', 'q3']], gates: [{ x: C, y: 72, label: 'CX · H' }] },
    { title: 'Outcomes to Bob', text: `Charlie gets m0 = ${m0}, m1 = ${m1} and sends them over the classical channel. q0 and q3 are now entangled — up to a known Pauli.`, step: sMeas,
      tokens: { q0: { x: A, y: QY, kind: 'q', label: 'q0' }, q1: { x: C - 18, y: QY + 30, kind: 'q', label: m0, faded: true }, q2: { x: C + 18, y: QY + 30, kind: 'q', label: m1, faded: true }, q3: { x: B, y: QY, kind: 'q', label: 'q3' }, b0: { x: B - 16, y: CY, kind: 'b', label: m0 }, b1: { x: B + 16, y: CY, kind: 'b', label: m1 } }, links: [['q0', 'q3']] },
    { title: 'Bob corrects: Alice and Bob share |Φ+⟩', text: 'Two particles that never met are maximally entangled. Chain this over many short links and you have a quantum repeater: the loss of each hop, not of the whole distance, sets the rate.', step: sEnd,
      tokens: { q0: { x: A, y: QY, kind: 'q', label: 'q0', tone: 'var(--series-5)' }, q1: { x: C - 18, y: QY + 30, kind: 'q', label: m0, faded: true }, q2: { x: C + 18, y: QY + 30, kind: 'q', label: m1, faded: true }, q3: { x: B, y: QY, kind: 'q', label: 'q3', tone: 'var(--series-5)' } }, links: [['q0', 'q3']] },
  ];
  const p = usePlayer(scenes.length, 0.45, { resetKey: seed });
  const sc = scenes[p.i];
  const s = steps[sc.step];
  return (
    <div className="grid grid-sidebar">
      <Card title="Who is entangled with whom?" subtitle="Wootters concurrence (1 = maximally entangled, 0 = not)">
        <table className="table qx-table-tight">
          <thead><tr><th>Pair</th><th>Concurrence</th></tr></thead>
          <tbody>
            {[[0, 1, 'Alice–Charlie (q0, q1)'], [2, 3, 'Charlie–Bob (q2, q3)'], [0, 3, 'Alice–Bob (q0, q3)']].map(([i, j, l]) => (
              <tr key={String(l)}><td>{l}</td><td className="tnum">{conc(s, i as number, j as number).toFixed(3)}</td></tr>
            ))}
          </tbody>
        </table>
        <button className="btn small" style={{ marginTop: 8 }} onClick={() => setSeed((x) => x + 1)}>New measurement outcomes</button>
        <div className="small muted" style={{ marginTop: 8 }}>Relevance to a two-node link: a repeater node is two of these links plus Charlie in the middle — the upgrade path of a point-to-point QKD link beyond the PLOB bound (see the QKD tab).</div>
      </Card>
      <Card title="Entanglement swapping" subtitle="Żukowski, Zeilinger, Horne & Ekert (1993) — the core of a quantum repeater">
        <StoryStage lanes={[{ x: A, name: 'Alice', role: 'end node' }, { x: C, name: 'Charlie', role: 'relay / repeater' }, { x: B, name: 'Bob', role: 'end node' }]} scenes={scenes} p={p} />
        <PlayBar p={p} label={(i) => `scene ${i + 1} / ${scenes.length}`} />
        <div className="qx-caption"><b>{sc.title}.</b> <span className="small">{sc.text}</span></div>
        {c && <CircuitStrip circuit={c} active={sc.step} />}
      </Card>
    </div>
  );
}

// ---------------------------------------------------------------------------

function Chsh() {
  const [vis, setVis] = useState(1);
  const [a1, setA1] = useState(Math.PI / 2);
  const [b0, setB0] = useState(Math.PI / 4);
  const [shots, setShots] = useState(2000);
  const [seed, setSeed] = useState(1);
  const r = useCompute<ChshOut>('quantum.chsh', { a0: 0, a1, b0, b1: b0 + Math.PI / 2, visibility: vis, shots, seed });
  const o = r.data;
  const p = usePlayer(40, 6, { resetKey: `${vis},${a1},${b0},${shots},${seed}` });
  const prog = (p.i + 1) / 40;
  return (
    <div className="grid grid-sidebar">
      <Card title="Settings">
        <Slider label="Alice's second angle a₁ (a₀ = 0)" value={a1} min={0} max={Math.PI} step={Math.PI / 48} format={fmtAngle} onChange={setA1} />
        <Slider label="Bob's angle b₀ (b₁ = b₀ + π/2)" value={b0} min={0} max={Math.PI} step={Math.PI / 48} format={fmtAngle} onChange={setB0} />
        <Slider label="Source visibility V" value={vis} min={0} max={1} step={0.01} onChange={setVis} hint="Werner state V|Φ+⟩⟨Φ+| + (1 − V) I/4: mixes in white noise" />
        <Seg value={shots} onChange={setShots} options={[200, 2000, 20000].map((s) => ({ value: s, label: String(s) }))} label="Pairs per setting" />
        <button className="btn small" onClick={() => setSeed((s) => s + 1)}>Re-run the experiment</button>
        <div className="small muted" style={{ marginTop: 8 }}>Optimal: a₀ = 0, a₁ = π/2, b₀ = π/4, b₁ = 3π/4 → S = 2√2. Violation needs V &gt; 1/√2 ≈ 0.707 — the same visibility an entanglement-based QKD link (E91) needs.</div>
      </Card>
      <Card title="CHSH Bell test" subtitle="Clauser, Horne, Shimony & Holt (1969): no local hidden-variable model gives |S| > 2">
        {o && (
          <div className="grid grid-2">
            <div>
              <Gauge value={Math.abs(o.s_sampled)} max={3} threshold={2} unit="" label="|S| measured" marks={[{ v: 2, t: '2 local' }, { v: 2 * Math.SQRT2, t: '2√2' }]} />
              <PlayBar p={p} label={() => `${Math.round(prog * shots)} pairs / setting`} />
              <div className="row small" style={{ marginTop: 6 }}>
                <StatusChip status={o.violated ? 'pass' : 'fail'} />
                <span>S exact = {o.s_exact.toFixed(3)}, sampled = {o.s_sampled.toFixed(3)} ± {o.s_std.toFixed(3)}</span>
              </div>
            </div>
            <div className="table-wrap">
              <table className="table qx-table-tight">
                <thead><tr><th>Setting</th><th>same</th><th>diff</th><th>E</th><th>cos(a − b)·V</th></tr></thead>
                <tbody>
                  {o.settings.map((st, k) => (
                    <tr key={k}>
                      <td className="mono small">{['a₀b₀', 'a₀b₁ (−)', 'a₁b₀', 'a₁b₁'][k]}</td>
                      <td className="tnum">{Math.round(st.same * prog)}</td>
                      <td className="tnum">{Math.round(st.diff * prog)}</td>
                      <td className="tnum">{st.e_sampled.toFixed(3)}</td>
                      <td className="tnum">{st.e_exact.toFixed(3)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}
        {o && (
          <Plot
            series={[{ name: 'S when Bob\'s pair of angles is rotated', x: o.sweep_x.map((x) => (x * 180) / Math.PI), y: o.sweep_s }]}
            hLines={[{ value: 2, label: 'local bound 2', color: 'var(--bad)' }, { value: -2, label: '−2', color: 'var(--bad)' }, { value: 2 * Math.SQRT2, label: '2√2 Tsirelson', color: 'var(--good)' }]}
            vLines={[{ value: ((b0 % Math.PI) * 180) / Math.PI, label: 'now' }]}
            xLabel="Bob's b₀ (degrees)" yLabel="S" height={200} legend={false}
          />
        )}
        <Explainer title="Why this matters for a quantum link" refs={[{ label: 'Clauser et al., PRL 23, 880 (1969)' }, { label: 'Ekert, PRL 67, 661 (1991)' }]}>
          E(a, b) = P(same) − P(different) = V cos(a − b) for this source.
          <Formula>S = E(a₀,b₀) − E(a₀,b₁) + E(a₁,b₀) + E(a₁,b₁)  ≤ 2 (local)  ≤ 2√2 (quantum)</Formula>
          In entanglement-based QKD the measured S certifies that the pairs are really entangled — an eavesdropper who measured them would push S down to ≤ 2.
        </Explainer>
      </Card>
    </div>
  );
}

export default function Protocols() {
  const [which, setWhich] = useState<'teleport' | 'superdense' | 'swap' | 'chsh'>('teleport');
  return (
    <div className="stack">
      <Seg
        value={which}
        onChange={setWhich}
        options={[
          { value: 'teleport', label: 'Teleportation' },
          { value: 'superdense', label: 'Superdense coding' },
          { value: 'swap', label: 'Entanglement swapping' },
          { value: 'chsh', label: 'Bell / CHSH test' },
        ]}
      />
      {which === 'teleport' && <Teleport />}
      {which === 'superdense' && <Superdense />}
      {which === 'swap' && <Swapping />}
      {which === 'chsh' && <Chsh />}
    </div>
  );
}
