// Gate-level schematic and digital waveform viewer (SVG), shared by the
// logic minimiser and the gate simulator.
import { useMemo, useRef, useState } from 'react';
import { useElementWidth } from '../../components/useElementWidth';
import type { Glitch, GateKind, PlacedGate, Wave } from './types';

const GW = 46; // gate body width
const COLW = 118;
const ROWH = 64;

function valColor(v: number | undefined) {
  if (v === undefined) return 'var(--text-2)';
  if (v === 1) return 'var(--accent)';
  if (v === 0) return 'var(--muted)';
  return 'var(--warn)';
}

/** Standard distinctive-shape gate symbol with its output at (GW, 0) and body centred on y = 0. */
function GateShape({ kind, h, stroke, fill }: { kind: GateKind; h: number; stroke: string; fill: string }) {
  const t = -h / 2;
  const b = h / 2;
  const bubble = (x: number) => <circle cx={x + 4} cy={0} r={4} fill="var(--surface-1)" stroke={stroke} strokeWidth={1.8} />;
  const common = { fill, stroke, strokeWidth: 1.8, strokeLinejoin: 'round' as const };
  switch (kind) {
    case 'AND':
    case 'NAND':
      return (
        <g>
          <path d={`M0,${t} H${GW * 0.5} A${GW * 0.5},${h / 2} 0 0 1 ${GW * 0.5},${b} H0 Z`} {...common} transform={kind === 'NAND' ? `scale(${(GW - 8) / GW} 1)` : undefined} />
          {kind === 'NAND' && bubble(GW - 8)}
        </g>
      );
    case 'OR':
    case 'NOR':
    case 'XOR':
    case 'XNOR': {
      const w = kind === 'NOR' || kind === 'XNOR' ? GW - 8 : GW;
      return (
        <g>
          <path d={`M0,${t} Q${w * 0.55},${t} ${w},0 Q${w * 0.55},${b} 0,${b} Q${w * 0.28},0 0,${t} Z`} {...common} />
          {(kind === 'XOR' || kind === 'XNOR') && <path d={`M-6,${t} Q${w * 0.28 - 6},0 -6,${b}`} fill="none" stroke={stroke} strokeWidth={1.8} />}
          {(kind === 'NOR' || kind === 'XNOR') && bubble(w)}
        </g>
      );
    }
    case 'NOT':
    case 'BUF':
      return (
        <g>
          <path d={`M4,${-14} L${GW - (kind === 'NOT' ? 12 : 4)},0 L4,14 Z`} {...common} />
          {kind === 'NOT' && bubble(GW - 12)}
        </g>
      );
    case 'DFF':
      return (
        <g>
          <rect x={0} y={t} width={GW} height={h} rx={4} {...common} />
          <path d={`M0,${b - 12} L8,${b - 6} L0,${b}`} fill="none" stroke={stroke} strokeWidth={1.5} />
          <text x={6} y={t + 13} fontSize={9.5} fill="var(--text-2)">D</text>
          <text x={GW - 12} y={4} fontSize={9.5} fill="var(--text-2)">Q</text>
        </g>
      );
    default:
      return <rect x={0} y={t} width={GW} height={h} rx={4} {...common} />;
  }
}

export interface DiagramProps {
  inputs: string[];
  outputs: string[];
  gates: PlacedGate[];
  levels: number;
  values?: Map<string, number>;
  critical?: { gates: string[]; nets: string[] } | null;
  showDelays?: boolean;
}

export function GateDiagram(props: DiagramProps) {
  const { inputs, outputs, gates } = props;
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const layout = useMemo(() => {
    const maxLevel = Math.max(1, ...gates.map((g) => g.level));
    const perLevel = new Map<number, number>();
    gates.forEach((g) => perLevel.set(g.level, Math.max(perLevel.get(g.level) ?? 0, g.row + 1)));
    const maxRows = Math.max(inputs.length, ...Array.from(perLevel.values()), 1);
    const H = maxRows * ROWH + 30;
    const W = 70 + (maxLevel + 1) * COLW + 60;
    const inY = new Map<string, number>();
    inputs.forEach((n, i) => inY.set(n, 20 + ((i + 0.5) * (H - 30)) / inputs.length));
    const pos = new Map<string, { x: number; y: number; h: number }>();
    gates.forEach((g) => {
      const rows = perLevel.get(g.level) ?? 1;
      const y = 20 + ((g.row + 0.5) * (H - 30)) / rows;
      const h = Math.max(30, g.inputs.length * 13 + 6);
      pos.set(g.id, { x: 70 + g.level * COLW - GW, y, h });
    });
    const driver = new Map<string, { x: number; y: number }>();
    inputs.forEach((n) => driver.set(n, { x: 58, y: inY.get(n)! }));
    gates.forEach((g) => {
      const p = pos.get(g.id)!;
      driver.set(g.output, { x: p.x + GW, y: p.y });
    });
    return { H, W, inY, pos, driver };
  }, [gates, inputs]);
  const crit = new Set(props.critical?.gates ?? []);
  const critNets = new Set(props.critical?.nets ?? []);
  const scale = width > 0 ? Math.max(0.72, Math.min(1, width / layout.W)) : 1;
  return (
    <div ref={ref} style={{ width: '100%', overflowX: 'auto' }}>
      <svg width={layout.W * scale} height={layout.H * scale} viewBox={`0 0 ${layout.W} ${layout.H}`} role="img" aria-label="Gate-level schematic">
        {/* wires */}
        {gates.map((g) => {
          const p = layout.pos.get(g.id)!;
          return g.inputs.map((net, k) => {
            const src = layout.driver.get(net);
            if (!src) return null;
            const ty = p.y - p.h / 2 + ((k + 1) * p.h) / (g.inputs.length + 1);
            const tx = p.x + (g.kind === 'OR' || g.kind === 'NOR' || g.kind === 'XOR' || g.kind === 'XNOR' ? 6 : 0);
            const onCrit = crit.has(g.id) && critNets.has(net);
            const v = props.values?.get(net);
            let d: string;
            if (src.x < tx - 8) {
              const mx = tx - 14 - (k % 3) * 5;
              d = `M${src.x},${src.y} H${mx} V${ty} H${tx}`;
            } else {
              // feedback: route under the diagram
              const yb = layout.H - 6 - (k % 3) * 4;
              d = `M${src.x},${src.y} h10 V${yb} H${tx - 16} V${ty} H${tx}`;
            }
            return (
              <path key={`${g.id}-${k}`} d={d} fill="none" stroke={onCrit ? 'var(--copper)' : valColor(v)} strokeWidth={onCrit ? 3.2 : v === 1 ? 2.2 : 1.6} strokeLinejoin="round" opacity={props.values && v === undefined ? 0.6 : 1} />
            );
          });
        })}
        {/* input ports */}
        {inputs.map((n) => {
          const y = layout.inY.get(n)!;
          const v = props.values?.get(n);
          return (
            <g key={n}>
              <rect x={8} y={y - 10} width={50} height={20} rx={6} fill="var(--surface-2)" stroke={valColor(v)} strokeWidth={1.5} />
              <text x={33} y={y + 4} fontSize={11.5} textAnchor="middle" fill="var(--text)" fontFamily="var(--mono)">{n}{v !== undefined ? `=${v === 2 ? 'X' : v}` : ''}</text>
            </g>
          );
        })}
        {/* gates */}
        {gates.map((g) => {
          const p = layout.pos.get(g.id)!;
          const onCrit = crit.has(g.id);
          const v = props.values?.get(g.output);
          const isOut = outputs.includes(g.output);
          return (
            <g key={g.id} transform={`translate(${p.x} ${p.y})`} className={onCrit ? 'gate-glow' : undefined}>
              <GateShape kind={g.kind} h={p.h} stroke={onCrit ? 'var(--copper)' : 'var(--text-2)'} fill={v === 1 ? 'var(--accent-soft)' : 'var(--surface-2)'} />
              <text x={GW / 2 - 4} y={-p.h / 2 - 5} fontSize={9.5} textAnchor="middle" fill="var(--muted)" fontFamily="var(--mono)">
                {g.id}{props.showDelays ? ` ${g.delay_ns}ns` : ''}
              </text>
              {isOut && (
                <g>
                  <line x1={GW} y1={0} x2={GW + 22} y2={0} stroke={valColor(v)} strokeWidth={2} />
                  <rect x={GW + 22} y={-10} width={48} height={20} rx={6} fill="var(--surface-2)" stroke={valColor(v)} strokeWidth={1.5} />
                  <text x={GW + 46} y={4} fontSize={11.5} textAnchor="middle" fill="var(--text)" fontFamily="var(--mono)">{g.output}{v !== undefined ? `=${v === 2 ? 'X' : v}` : ''}</text>
                </g>
              )}
            </g>
          );
        })}
      </svg>
    </div>
  );
}

/** Digital waveform viewer with a playhead that reveals the simulation, glitch highlights and a hover cursor. */
export function WaveView(props: { waves: Wave[]; tEnd: number; playhead: number; glitches?: Glitch[]; highlight?: Set<string>; critical?: { from: string; to: string; delay: number } | null }) {
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const [hover, setHover] = useState<number | null>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const L = 74;
  const R = 12;
  const RH = 28;
  const W = Math.max(width, 320);
  const H = props.waves.length * RH + 30;
  const X = (t: number) => L + (t / props.tEnd) * (W - L - R);
  const ticks = useMemo(() => {
    const step = [1, 2, 5, 10, 20, 25, 50, 100, 200, 500].find((s) => props.tEnd / s <= 12) ?? 1000;
    const out: number[] = [];
    for (let t = 0; t <= props.tEnd + 1e-9; t += step) out.push(t);
    return out;
  }, [props.tEnd]);
  const ph = Math.min(props.playhead, props.tEnd);
  const valueAt = (w: Wave, t: number) => {
    let v = 2;
    for (const [tt, vv] of w.points) {
      if (tt <= t) v = vv;
      else break;
    }
    return v;
  };
  return (
    <div className="wave-wrap" ref={ref}>
      {width > 0 && (
        <svg
          ref={svgRef}
          width={W}
          height={H}
          role="img"
          aria-label="Digital waveforms"
          onMouseMove={(e) => {
            const r = svgRef.current!.getBoundingClientRect();
            const x = e.clientX - r.left;
            setHover(x >= L && x <= W - R ? ((x - L) / (W - L - R)) * props.tEnd : null);
          }}
          onMouseLeave={() => setHover(null)}
        >
          <defs>
            <pattern id="xhatch" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
              <line x1="0" y1="0" x2="0" y2="6" stroke="var(--warn)" strokeWidth="1.5" opacity="0.6" />
            </pattern>
            <clipPath id="wave-reveal"><rect x={0} y={0} width={X(ph)} height={H} /></clipPath>
          </defs>
          {ticks.map((t) => (
            <g key={t}>
              <line x1={X(t)} x2={X(t)} y1={4} y2={H - 22} stroke="var(--grid)" />
              <text x={X(t)} y={H - 8} fontSize={10.5} textAnchor="middle" fill="var(--muted)" fontFamily="var(--mono)">{t}</text>
            </g>
          ))}
          <text x={W - R} y={H - 8} fontSize={10.5} textAnchor="end" fill="var(--muted)">ns</text>
          {props.waves.map((w, i) => {
            const y0 = 8 + i * RH;
            const hi = y0 + 4;
            const lo = y0 + RH - 8;
            const hl = props.highlight?.has(w.net);
            return (
              <g key={w.net}>
                <text x={L - 8} y={(hi + lo) / 2 + 4} textAnchor="end" className="wave-name" style={hl ? { fill: 'var(--copper)', fontWeight: 700 } : w.role === 'output' ? { fill: 'var(--text)', fontWeight: 700 } : undefined}>{w.net}</text>
                <line x1={L} x2={W - R} y1={lo + 4} y2={lo + 4} stroke="var(--border)" />
              </g>
            );
          })}
          <g clipPath="url(#wave-reveal)">
            {(props.glitches ?? []).map((gl, k) => {
              const i = props.waves.findIndex((w) => w.net === gl.net);
              if (i < 0) return null;
              const y0 = 8 + i * RH;
              return <rect key={k} x={X(gl.t0) - 2} y={y0} width={Math.max(4, X(gl.t1) - X(gl.t0) + 4)} height={RH - 4} rx={4} fill="color-mix(in srgb, var(--bad) 30%, transparent)" stroke="var(--bad)" strokeWidth={1} />;
            })}
            {props.waves.map((w, i) => {
              const y0 = 8 + i * RH;
              const hi = y0 + 4;
              const lo = y0 + RH - 8;
              let d = '';
              const xs: string[] = [];
              const pts = w.points;
              for (let k = 0; k < pts.length; k++) {
                const [t, v] = pts[k];
                const tn = k + 1 < pts.length ? pts[k + 1][0] : props.tEnd;
                const x0 = X(t);
                const x1 = X(Math.max(t, tn));
                if (v === 2) {
                  xs.push(`M${x0},${hi}H${x1}V${lo}H${x0}Z`);
                  continue;
                }
                const y = v === 1 ? hi : lo;
                d += `${d ? 'L' : 'M'}${x0},${y}H${x1}`;
              }
              const col = props.highlight?.has(w.net) ? 'var(--copper)' : w.role === 'input' ? 'var(--series-3)' : w.role === 'output' ? 'var(--series-1)' : 'var(--series-7)';
              return (
                <g key={w.net}>
                  {xs.length > 0 && <path d={xs.join('')} fill="url(#xhatch)" stroke="var(--warn)" strokeWidth={1} />}
                  <path d={d} fill="none" stroke={col} strokeWidth={2} strokeLinejoin="round" />
                </g>
              );
            })}
          </g>
          <line x1={X(ph)} x2={X(ph)} y1={2} y2={H - 22} stroke="var(--accent)" strokeWidth={1.5} />
          {hover !== null && (
            <g pointerEvents="none">
              <line x1={X(hover)} x2={X(hover)} y1={2} y2={H - 22} stroke="var(--border-strong)" strokeDasharray="3 3" />
              {props.waves.map((w, i) => {
                const v = valueAt(w, hover);
                return (
                  <text key={w.net} x={X(hover) + 5} y={8 + i * RH + 14} fontSize={10.5} fill="var(--text)" fontFamily="var(--mono)" style={{ paintOrder: 'stroke', stroke: 'var(--surface-1)', strokeWidth: 3 }}>{v === 2 ? 'X' : v}</text>
                );
              })}
              <text x={X(hover)} y={H - 20} fontSize={10.5} textAnchor="middle" fill="var(--text)" fontFamily="var(--mono)">{hover.toFixed(1)} ns</text>
            </g>
          )}
        </svg>
      )}
    </div>
  );
}

/** Net values at time t from waveforms (for colouring the schematic). */
export function valuesAt(waves: Wave[], t: number): Map<string, number> {
  const m = new Map<string, number>();
  for (const w of waves) {
    let v = 2;
    for (const [tt, vv] of w.points) {
      if (tt <= t) v = vv;
      else break;
    }
    m.set(w.net, v);
  }
  return m;
}
