import { useEffect, useMemo, useState } from 'react';
import { useCompute } from '../../engine/hooks';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat, Toggle } from '../../components/ui';
import type { CircuitInfo, SimResult, Stimulus } from './types';
import { GateDiagram, WaveView, valuesAt } from './gates';
import { Seg, useFrame, useReducedMotion } from '../dsp/viz';

export type LogicInput = { expr?: string; minterms?: number[]; dont_cares?: number[]; num_vars?: number };

function vcd(r: SimResult): string {
  const ids = r.waves.map((_, i) => String.fromCharCode(33 + (i % 90)) + (i >= 90 ? String(i) : ''));
  const lines = ['$date Protoforge Chip Lab $end', '$timescale 1ps $end', `$scope module ${r.name} $end`];
  r.waves.forEach((w, i) => lines.push(`$var wire 1 ${ids[i]} ${w.net} $end`));
  lines.push('$upscope $end', '$enddefinitions $end');
  const events: [number, string][] = [];
  r.waves.forEach((w, i) => w.points.forEach(([t, v]) => events.push([Math.round(t * 1000), `${v === 2 ? 'x' : v}${ids[i]}`])));
  events.sort((a, b) => a[0] - b[0]);
  let last = -1;
  for (const [t, s] of events) {
    if (t !== last) {
      lines.push(`#${t}`);
      last = t;
    }
    lines.push(s);
  }
  return lines.join('\n') + '\n';
}

export function SimTab(props: { logic: LogicInput | null }) {
  const reduced = useReducedMotion();
  const list = useCompute<CircuitInfo[]>('chip.circuits', {});
  const [circuit, setCircuit] = useState(props.logic ? 'custom' : 'rca4');
  useEffect(() => {
    if (props.logic) setCircuit('custom');
  }, [props.logic]);
  const [mode, setMode] = useState<'script' | 'live'>('script');
  const [scale, setScale] = useState(1);
  const [showCrit, setShowCrit] = useState(true);
  const [showInternal, setShowInternal] = useState(false);
  const [live, setLive] = useState<Stimulus | null>(null);
  const base = useCompute<SimResult>('chip.simulate', circuit === 'custom' && props.logic ? { logic: props.logic, delay_scale: scale } : { circuit, delay_scale: scale });
  const inputs = base.data?.inputs ?? [];
  const stim = mode === 'live' && live ? live : null;
  const r = useCompute<SimResult>('chip.simulate', circuit === 'custom' && props.logic ? { logic: props.logic, delay_scale: scale, stimulus: stim ?? undefined } : { circuit, delay_scale: scale, stimulus: stim ?? undefined });
  const d = r.data ?? base.data;
  // reset the live stimulus whenever the circuit changes
  useEffect(() => {
    setLive(null);
  }, [circuit, props.logic]);
  const [playing, setPlaying] = useState(!reduced);
  const [ph, setPh] = useState(reduced ? 1e9 : 0);
  const tEnd = d?.t_end_ns ?? 100;
  useEffect(() => {
    setPh(reduced ? 1e9 : mode === 'live' ? tEnd : 0);
  }, [d?.name, d?.t_end_ns, reduced, mode]);
  useFrame(playing && !reduced && mode === 'script', (dt) => setPh((p) => (p >= tEnd * 1.15 ? 0 : p + (dt / 4500) * tEnd)));
  const values = useMemo(() => (d ? valuesAt(d.waves, Math.min(ph, tEnd)) : undefined), [d, ph, tEnd]);
  const liveState = useMemo(() => {
    const m = new Map<string, number>();
    const s = live;
    inputs.forEach((n) => m.set(n, 0));
    if (s) for (const [, n, v] of s.changes) m.set(n, v);
    return m;
  }, [live, inputs]);
  const toggle = (net: string) => {
    const s: Stimulus = live ?? { changes: inputs.map((n) => [0, n, 0] as [number, string, number]), clocks: base.data?.stimulus.clocks ?? [], t_end_ns: 20 };
    const lastT = Math.max(0, ...s.changes.map((c) => c[0]));
    const t = lastT + 20;
    const nv = liveState.get(net) === 1 ? 0 : 1;
    setLive({ ...s, changes: [...s.changes, [t, net, nv]], t_end_ns: t + 25 });
  };
  const crit = d && showCrit ? { gates: d.critical.gates, nets: d.critical.nets } : null;
  return (
    <div className="stack">
      <div className="grid grid-sidebar">
        <Card title="Circuit" subtitle="Event-driven simulation with per-gate transport delays">
          <Select
            label="Built-in circuit"
            value={circuit}
            options={[...(list.data ?? []).map((c) => ({ value: c.name, label: c.title })), ...(props.logic ? [{ value: 'custom', label: 'Minimised function (from the logic tab)' }] : [])]}
            onChange={setCircuit}
          />
          {d && <p className="lab-note">{d.description}</p>}
          <Seg label="Stimulus" value={mode} options={[{ value: 'script', label: 'Scripted' }, { value: 'live', label: 'Click inputs' }]} onChange={setMode} />
          {mode === 'live' && (
            <div>
              <div className="pill-row" style={{ marginBottom: 8 }}>
                {inputs.filter((n) => !(base.data?.stimulus.clocks ?? []).some((c) => c.net === n)).map((n) => (
                  <button key={n} className={`btn small input-toggle ${liveState.get(n) === 1 ? 'on' : ''}`} onClick={() => toggle(n)} aria-pressed={liveState.get(n) === 1}>
                    {n} = {liveState.get(n) ?? 0}
                  </button>
                ))}
              </div>
              <p className="muted small">Each click toggles an input 20 ns after the previous change and re-simulates.</p>
              <button className="btn small" onClick={() => setLive(null)}>Reset</button>
            </div>
          )}
          <Slider label="Gate-delay scale" value={scale} min={0.25} max={3} step={0.05} unit="×" onChange={setScale} />
          <Toggle label="Highlight the critical path" value={showCrit} onChange={setShowCrit} />
          <Toggle label="Show internal nets" value={showInternal} onChange={setShowInternal} />
          <table className="table" style={{ marginTop: 6 }}>
            <thead><tr><th>Gate</th><th>Delay</th></tr></thead>
            <tbody>
              {[['NOT / BUF', 1], ['NAND / NOR', 1.5], ['AND / OR', 2], ['XOR / XNOR', 3], ['DFF clk→Q', 2]].map(([k, v]) => (
                <tr key={k as string}><td>{k}</td><td className="mono">{((v as number) * scale).toFixed(2)} ns</td></tr>
              ))}
            </tbody>
          </table>
        </Card>
        <div className="stack">
          <ErrorBox error={r.error ?? base.error} />
          {d && (
            <div className="lab-hero">
              <Stat label="Critical path" value={d.critical.delay_ns.toFixed(2)} unit="ns" note={`${d.critical.from} → ${d.critical.to} (${d.critical.gates.length} gates)`} />
              {d.critical.fmax_mhz != null && <Stat label="Max clock" value={d.critical.fmax_mhz.toFixed(0)} unit="MHz" note="1 / (path + 1 ns setup)" />}
              <Stat label="Glitches" value={d.glitches.length} tone={d.glitches.length ? 'warn' : 'pass'} note={d.glitches.length ? 'hazards shorter than 4 ns' : 'clean transitions'} />
              <Stat label="Events" value={d.events} note={d.has_loop ? 'feedback loop (latch)' : 'discrete-event queue'} />
            </div>
          )}
          <Card
            title="Schematic"
            subtitle={`Wire colour = logic value at t = ${Math.min(ph, tEnd).toFixed(1)} ns (blue 1, grey 0, amber X)${showCrit ? '; copper = critical path' : ''}`}
          >
            {d && <GateDiagram inputs={d.inputs} outputs={d.outputs} gates={d.gates} levels={d.levels} values={values} critical={crit} showDelays />}
          </Card>
          <Card
            title="Waveforms"
            subtitle="Red boxes mark glitches: unequal path delays make outputs flicker before they settle"
            actions={
              <>
                {mode === 'script' && !reduced && <button className="btn small" onClick={() => setPlaying((p) => !p)}>{playing ? '❚❚ Pause' : '▶ Play'}</button>}
                {mode === 'script' && <button className="btn small" onClick={() => { setPlaying(false); setPh(tEnd); }}>Show all</button>}
                {d && <DownloadButton small filename={`${d.name}.vcd`} label="VCD" content={() => vcd(d)} />}
              </>
            }
          >
            {d && <WaveView waves={d.waves.filter((w) => showInternal || w.role !== 'internal' || (showCrit && d.critical.nets.includes(w.net)))} tEnd={tEnd} playhead={ph} glitches={d.glitches} highlight={showCrit ? new Set(d.critical.nets) : undefined} />}
          </Card>
        </div>
      </div>
      <div className="grid grid-3">
        <Explainer title="Discrete-event simulation" refs={[{ label: 'IEEE Std 1364-2005 (Verilog), §11 scheduling semantics' }]}>
          <Formula>{'on change(net, t):\n  for gate in fanout(net):\n    schedule(gate.out, eval(gate), t + delay)'}</Formula>
          Only gates whose inputs change are evaluated — the same idea as Icarus Verilog and Verilator's event mode. Transport delays let
          pulses shorter than a gate delay through, which is how glitches appear.
        </Explainer>
        <Explainer title="Ripple carry and static timing" refs={[{ label: 'N. Weste & D. Harris, CMOS VLSI Design, 4th ed., ch. 4 & 11' }]}>
          <Formula>{'t_RCA ≈ t_XOR + N·(t_AND + t_OR)\nf_max = 1 / (t_clk→Q + t_logic + t_setup)'}</Formula>
          The carry must ripple through every full adder, so an N-bit adder is O(N) deep. Static timing analysis takes the longest path
          without simulating vectors — the IRIS-DSP timing estimate is built the same way.
        </Explainer>
        <Explainer title="Hazards">
          The 2:1 mux Y = A·S′ + B·S glitches low when S falls with A = B = 1, because S′ arrives one inverter later. Adding the consensus
          term A·B (a redundant prime implicant!) removes the hazard — try <span className="mono">a s' + b s + a b</span> in the logic tab.
        </Explainer>
      </div>
    </div>
  );
}
