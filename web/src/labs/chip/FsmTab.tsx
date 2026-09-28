import { useMemo, useRef, useState } from 'react';
import { useCompute } from '../../engine/hooks';
import { tryCall } from '../../engine/engine';
import { Plot } from '../../components/Plot';
import { Card, Code, Explainer, Formula, Slider, Tabs } from '../../components/ui';
import { useElementWidth } from '../../components/useElementWidth';
import type { Encoding, Fsm, FsmOut, GestureEvent } from './types';
import { KV, Seg, useFrame, useReducedMotion } from '../dsp/viz';

const HIST = 3000; // ms of history shown

function StateDiagram(props: { fsm: Fsm; active: number; lastFrom: number | null }) {
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const W = 860;
  const H = 420;
  const R = 46;
  const P = (i: number) => ({ x: 60 + props.fsm.states[i].x * (W - 120), y: 50 + props.fsm.states[i].y * (H - 100) });
  const idx = (n: string) => props.fsm.states.findIndex((s) => s.name === n);
  const scale = width > 0 ? Math.max(0.62, Math.min(1, width / W)) : 1;
  const label = (g: Fsm['transitions'][number]['guard']) => {
    const p: string[] = [];
    if (g.touch !== null && g.touch !== undefined) p.push(g.touch ? 'touch' : '!touch');
    if (g.timer_ge != null) p.push(`t≥${g.timer_ge}`);
    if (g.timer_lt != null) p.push(`t<${g.timer_lt}`);
    return p.length ? p.join(' · ') : 'next tick';
  };
  return (
    <div ref={ref} style={{ width: '100%', overflowX: 'auto' }}>
      <svg width={W * scale} height={H * scale} viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Gesture FSM state diagram">
        <defs>
          <marker id="fsm-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
            <path d="M0,0 L10,5 L0,10 Z" fill="var(--text-2)" />
          </marker>
          <marker id="fsm-arrow-hot" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
            <path d="M0,0 L10,5 L0,10 Z" fill="var(--accent)" />
          </marker>
          <radialGradient id="state-hot">
            <stop offset="0%" stopColor="var(--accent)" stopOpacity={0.55} />
            <stop offset="100%" stopColor="var(--accent)" stopOpacity={0} />
          </radialGradient>
        </defs>
        {props.fsm.transitions.map((t, k) => {
          const a = idx(t.from);
          const b = idx(t.to);
          if (a < 0 || b < 0 || a === b) return null;
          const pa = P(a);
          const pb = P(b);
          const reverse = props.fsm.transitions.some((u) => u.from === t.to && u.to === t.from);
          const dx = pb.x - pa.x;
          const dy = pb.y - pa.y;
          const len = Math.hypot(dx, dy) || 1;
          const nx = -dy / len;
          const ny = dx / len;
          const bend = reverse ? 38 : 16;
          const cx = (pa.x + pb.x) / 2 + nx * bend;
          const cy = (pa.y + pb.y) / 2 + ny * bend;
          const ang0 = Math.atan2(cy - pa.y, cx - pa.x);
          const ang1 = Math.atan2(cy - pb.y, cx - pb.x);
          const sx = pa.x + R * Math.cos(ang0);
          const sy = pa.y + R * Math.sin(ang0);
          const ex = pb.x + (R + 3) * Math.cos(ang1);
          const ey = pb.y + (R + 3) * Math.sin(ang1);
          const hot = props.lastFrom === a && props.active === b;
          const lx = 0.25 * sx + 0.5 * cx + 0.25 * ex;
          const ly = 0.25 * sy + 0.5 * cy + 0.25 * ey;
          return (
            <g key={k}>
              <path d={`M${sx},${sy} Q${cx},${cy} ${ex},${ey}`} fill="none" stroke={hot ? 'var(--accent)' : 'var(--border-strong)'} strokeWidth={hot ? 3 : 1.6} markerEnd={`url(#${hot ? 'fsm-arrow-hot' : 'fsm-arrow'})`} />
              <text x={lx} y={ly} fontSize={12.5} textAnchor="middle" fill={hot ? 'var(--text)' : 'var(--text-2)'} fontFamily="var(--mono)" style={{ paintOrder: 'stroke', stroke: 'var(--surface-1)', strokeWidth: 4 }}>{label(t.guard)}</text>
            </g>
          );
        })}
        {props.fsm.states.map((s, i) => {
          const p = P(i);
          const on = i === props.active;
          const out = s.outputs.length > 0;
          return (
            <g key={s.name}>
              {on && <circle className="state-glow" cx={p.x} cy={p.y} r={R + 22} fill="url(#state-hot)" />}
              <circle cx={p.x} cy={p.y} r={R} fill={on ? 'var(--accent)' : 'var(--surface-2)'} stroke={on ? 'var(--accent)' : out ? 'var(--copper)' : 'var(--border-strong)'} strokeWidth={out ? 2.5 : 1.6} />
              {s.name === props.fsm.reset && <circle cx={p.x} cy={p.y} r={R - 5} fill="none" stroke={on ? '#fff' : 'var(--border-strong)'} strokeWidth={1.2} />}
              {s.name.split('_').map((w, li, arr) => (
                <text key={w} x={p.x} y={p.y + (li - (arr.length - 1) / 2) * 13 + (out ? -4 : 4)} fontSize={13} fontWeight={800} textAnchor="middle" fill={on ? "#fff" : "var(--text)"}>{w}</text>
              ))}
              {out && <text x={p.x} y={p.y + (s.name.includes('_') ? 22 : 14)} fontSize={9.5} textAnchor="middle" fill={on ? '#fff' : 'var(--copper)'} fontFamily="var(--mono)">{s.outputs.join(',')}</text>}
            </g>
          );
        })}
      </svg>
    </div>
  );
}

interface Hist {
  touch: number[];
  state: number[];
}

export function FsmTab() {
  const reduced = useReducedMotion();
  const [tapMs, setTapMs] = useState(250);
  const [longMs, setLongMs] = useState(600);
  const [gapMs, setGapMs] = useState(250);
  const [enc, setEnc] = useState<Encoding>('binary');
  const [script, setScript] = useState('mixed');
  const [codeTab, setCodeTab] = useState<'v' | 'tb'>('v');
  const params = { tap_ms: tapMs, long_ms: longMs, gap_ms: gapMs };
  const full = useCompute<FsmOut>('chip.fsm', { ...params, encoding: enc, trace: script, verilog: true });
  const fsm = full.data?.fsm;
  // live machine state
  const [down, setDown] = useState(false);
  const downRef = useRef(false);
  const st = useRef({ s: 0, t: 0, acc: 0, from: null as number | null });
  const [view, setView] = useState({ s: 0, t: 0, from: null as number | null });
  const hist = useRef<Hist>({ touch: [], state: [] });
  const [events, setEvents] = useState<(GestureEvent & { id: number })[]>([]);
  const evId = useRef(0);
  const [mode, setMode] = useState<'live' | 'script'>('live');
  const play = useRef({ k: 0 });
  const push = (touch: number[], states: number[]) => {
    const h = hist.current;
    h.touch.push(...touch);
    h.state.push(...states);
    if (h.touch.length > HIST) {
      h.touch.splice(0, h.touch.length - HIST);
      h.state.splice(0, h.state.length - HIST);
    }
  };
  const addEvents = (evs: GestureEvent[]) => {
    if (!evs.length) return;
    setEvents((old) => [...evs.map((e) => ({ ...e, id: evId.current++ })).reverse(), ...old].slice(0, 12));
  };
  useFrame(!!fsm, (dt) => {
    if (mode === 'live') {
      const s = st.current;
      s.acc += dt;
      const ticks = Math.floor(s.acc);
      if (ticks <= 0) return;
      s.acc -= ticks;
      const touch = new Array(ticks).fill(downRef.current ? 1 : 0);
      const r = tryCall<FsmOut>('chip.fsm', { ...params, touch, state: s.s, timer: s.t, verilog: false }).data;
      if (!r) return;
      push(touch, r.trace.states);
      if (r.trace.final_state !== s.s) s.from = s.s;
      s.s = r.trace.final_state;
      s.t = r.trace.final_timer;
      addEvents(r.trace.events);
      setView({ s: s.s, t: s.t, from: s.from });
    } else if (full.data) {
      const tr = full.data.trace;
      const n = tr.states.length;
      const k0 = play.current.k;
      const k1 = reduced ? n : Math.min(n, k0 + Math.max(1, Math.round(dt)));
      if (k0 >= n) return;
      push(full.data.touch.slice(k0, k1), tr.states.slice(k0, k1));
      addEvents(tr.events.filter((e) => e.tick >= k0 && e.tick < k1));
      play.current.k = k1;
      const s = tr.states[k1 - 1];
      const prev = k0 > 0 ? tr.states[k0 - 1] : 0;
      setView({ s, t: tr.timers[k1 - 1], from: s !== prev ? prev : view.from });
      setDown(full.data.touch[k1 - 1] === 1);
    }
  });
  const runScript = (name: string) => {
    setScript(name);
    setMode('script');
    play.current.k = 0;
    hist.current = { touch: [], state: [] };
    setEvents([]);
  };
  const goLive = () => {
    setMode('live');
    st.current = { s: 0, t: 0, acc: 0, from: null };
    hist.current = { touch: [], state: [] };
    setDown(false);
    downRef.current = false;
  };
  const press = (v: boolean) => {
    if (mode !== 'live') goLive();
    downRef.current = v;
    setDown(v);
  };
  const h = hist.current;
  const xs = useMemo(() => h.touch.map((_, i) => i - h.touch.length), [h.touch.length, view]);
  const stateName = (v: number) => (Math.abs(v - Math.round(v)) < 1e-6 ? (fsm?.states[Math.round(v)]?.name ?? '').replace('DOUBLE_TAP', '2×TAP').replace('WAIT_RELEASE', 'RELEASE').replace('LONG_PRESS', 'LONG').replace('TAP_WAIT', 'TAP WAIT') : '');
  const pressMs = view.s === 1 ? view.t : null;
  return (
    <div className="stack">
      <div className="grid grid-sidebar">
        <Card title="Touch pad" subtitle="Press and hold with mouse, finger or the space bar — the FSM runs at 1 kHz in Rust">
          <div
            className={`touchpad ${down ? 'down' : ''}`}
            role="button"
            tabIndex={0}
            aria-pressed={down}
            onPointerDown={(e) => {
              (e.target as Element).setPointerCapture?.(e.pointerId);
              press(true);
            }}
            onPointerUp={() => press(false)}
            onPointerCancel={() => press(false)}
            onKeyDown={(e) => {
              if ((e.key === ' ' || e.key === 'Enter') && !e.repeat) {
                e.preventDefault();
                press(true);
              }
            }}
            onKeyUp={(e) => (e.key === ' ' || e.key === 'Enter') && press(false)}
          >
            <span>{down ? 'touching…' : 'TOUCH'}</span>
            <span className="hint">tap · double tap · hold</span>
            {pressMs !== null && <span className="mono small">{pressMs} ms {pressMs >= longMs ? '(long!)' : pressMs < tapMs ? '(tap window)' : ''}</span>}
          </div>
          <div className="pill-row" style={{ marginTop: 12 }}>
            <span className="muted small">Scripted:</span>
            {['tap', 'double_tap', 'long_press', 'slow_press', 'mixed'].map((s) => (
              <button key={s} className={`btn small ${mode === 'script' && script === s ? 'playing' : ''}`} onClick={() => runScript(s)}>{s.replace('_', ' ')}</button>
            ))}
            {mode === 'script' && <button className="btn small" onClick={goLive}>● live</button>}
          </div>
          <div className="event-log" style={{ marginTop: 12 }} aria-live="polite">
            {events.length === 0 && <span className="muted small">Gestures appear here.</span>}
            {events.map((e) => (
              <div key={e.id} className="event-item gesture-pop">
                <b style={{ color: e.output === 'tap' ? 'var(--series-1)' : e.output === 'double_tap' ? 'var(--series-3)' : 'var(--series-2)' }}>{e.output.replace('_', ' ').toUpperCase()}</b>
                <span className="muted mono">{mode === 'script' ? `t = ${e.tick} ms` : 'live'}</span>
              </div>
            ))}
          </div>
          <div style={{ marginTop: 12 }}>
            <Slider label="Tap: release before" value={tapMs} min={80} max={500} step={10} unit="ms" onChange={setTapMs} format={(v) => v.toFixed(0)} />
            <Slider label="Long press after" value={longMs} min={300} max={1500} step={10} unit="ms" onChange={setLongMs} format={(v) => v.toFixed(0)} />
            <Slider label="Double-tap gap" value={gapMs} min={80} max={600} step={10} unit="ms" onChange={setGapMs} format={(v) => v.toFixed(0)} />
          </div>
        </Card>
        <div className="stack">
          <Card title="State diagram" subtitle={`Moore machine · current state ${fsm?.states[view.s]?.name ?? ''} · timer ${view.t} ms (${full.data?.timer_bits ?? 10}-bit, saturating, cleared on every transition)`}>
            {fsm && <StateDiagram fsm={fsm} active={view.s} lastFrom={view.from} />}
          </Card>
          <Card title="Timeline" subtitle={`Last ${HIST / 1000} s of touch input and state`}>
            <Plot height={80} series={[{ name: 'touch', x: xs, y: h.touch.slice(), step: true, color: 'var(--copper)', area: true }]} yDomain={[-0.1, 1.2]} xDomain={[-HIST, 0]} formatY={(v) => (Math.abs(v - 1) < 1e-6 ? '1' : Math.abs(v) < 1e-6 ? '0' : '')} legend={false} />
            <Plot height={230} series={[{ name: 'state', x: xs, y: h.state.slice(), step: true }]} yDomain={[-0.3, (fsm?.states.length ?? 7) - 0.7]} xDomain={[-HIST, 0]} formatY={stateName} xLabel="time (ms, now = 0)" legend={false} />
          </Card>
        </div>
      </div>
      <div className="grid grid-2">
        <Card
          title="Generated Verilog"
          subtitle={full.data ? `${full.data.fsm.states.length} states, ${enc === 'binary' ? `${full.data.state_bits} state flops` : `${full.data.fsm.states.length} one-hot flops`} + ${full.data.timer_bits}-bit timer · testbench replays the “${script.replace('_', ' ')}” trace` : ''}
          actions={<Seg value={enc} options={[{ value: 'binary', label: 'Binary' }, { value: 'one_hot', label: 'One-hot' }]} onChange={setEnc} />}
        >
          <Tabs tabs={[{ id: 'v', label: 'iris_gesture_fsm.v' }, { id: 'tb', label: 'Self-checking testbench' }]} value={codeTab} onChange={setCodeTab} />
          {full.data?.verilog && codeTab === 'v' && <Code code={full.data.verilog} filename="iris_gesture_fsm.v" maxHeight={420} />}
          {full.data?.testbench && codeTab === 'tb' && <Code code={full.data.testbench} filename="tb_iris_gesture_fsm.v" maxHeight={420} />}
        </Card>
        <div className="stack">
          {full.data && (
            <Card title="Cost estimate" subtitle="Standard cells (see the FIR → Verilog tab for the model)">
              <KV k="State flip-flops" v={full.data.cost.state_ffs} />
              <KV k="Timer flip-flops" v={full.data.cost.timer_ffs} />
              <KV k="Timer incrementer" v={full.data.cost.timer_incr_cells} />
              <KV k="Threshold comparators" v={full.data.cost.comparator_cells} />
              <KV k="Next-state logic" v={full.data.cost.next_state_cells} />
              <KV k="Output decode" v={full.data.cost.output_cells} />
              <KV k="Total" v={<b>{full.data.cost.total_cells} cells</b>} />
              <KV k="Critical path" v={`${full.data.cost.critical_ns.toFixed(2)} ns`} />
            </Card>
          )}
          <Explainer title="Moore machines, timers and encodings" refs={[{ label: 'E. F. Moore, "Gedanken-experiments on sequential machines", 1956' }, { label: 'G. H. Mealy, Bell Syst. Tech. J. 34 (1955)' }, { label: 'N. Weste & D. Harris, CMOS VLSI Design, ch. 10' }]}>
            <Formula>{'state[k+1] = δ(state[k], touch[k], timer[k])\noutputs[k] = λ(state[k])            (Moore)\ntimer[k+1] = state changes ? 0 : min(timer+1, 2^B−1)'}</Formula>
            Moore outputs depend only on the state register, so the tap/double-tap/long-press pulses are glitch-free and last exactly one tick.
            Binary encoding needs ⌈log₂7⌉ = 3 flops; one-hot needs 7 but its next-state logic is shallower — the usual FPGA choice. The Iris
            touch controller (U7) gives a clean digital touch line; the chip synchronises it with two flip-flops before the FSM sees it.
          </Explainer>
        </div>
      </div>
    </div>
  );
}
