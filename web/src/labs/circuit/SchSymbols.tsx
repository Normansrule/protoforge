// Symbols for the schematic editor, drawn in grid units at rotation 0 with
// pin 1 at (0, −2) and pin 2 at (0, +2) — the same geometry as the Circuit
// Lab's animated preset schematics, so imported presets look identical.
import type { SchPart } from './schTypes';

const SW = 0.13;

function Lead({ y0, y1 }: { y0: number; y1: number }) {
  return <line x1={0} y1={y0} x2={0} y2={y1} />;
}

export function SymbolBody({ kind, props, glow = 0, on }: { kind: string; props?: Record<string, string>; glow?: number; on?: boolean }) {
  switch (kind) {
    case 'resistor': {
      const ys = [-1.3, -1.083, -0.65, -0.217, 0.217, 0.65, 1.083, 1.3];
      const xs = [0, 0.45, -0.45, 0.45, -0.45, 0.45, -0.45, 0];
      return (<g><Lead y0={-2} y1={-1.3} /><polyline points={ys.map((y, k) => `${xs[k]},${y}`).join(' ')} fill="none" strokeLinejoin="round" /><Lead y0={1.3} y1={2} /></g>);
    }
    case 'capacitor':
      return (<g><Lead y0={-2} y1={-0.28} /><line x1={-0.9} y1={-0.28} x2={0.9} y2={-0.28} strokeWidth={SW * 1.5} /><line x1={-0.9} y1={0.28} x2={0.9} y2={0.28} strokeWidth={SW * 1.5} /><Lead y0={0.28} y1={2} /></g>);
    case 'inductor':
      return (<g><Lead y0={-2} y1={-1.3} /><path d="M0,-1.3 a0.325,0.325 0 0 1 0,0.65 a0.325,0.325 0 0 1 0,0.65 a0.325,0.325 0 0 1 0,0.65 a0.325,0.325 0 0 1 0,0.65" fill="none" /><Lead y0={1.3} y1={2} /></g>);
    case 'vsource': {
      const w = props?.wave ?? 'dc';
      return (
        <g>
          <Lead y0={-2} y1={-1.1} /><circle r={1.1} fill="var(--surface-1)" /><Lead y0={1.1} y1={2} />
          {w === 'sin' ? <path d="M-0.55,0.05 q0.275,-0.6 0.55,0 t0.55,0" fill="none" />
            : w === 'pulse' ? <path d="M-0.6,0.3 h0.3 v-0.6 h0.5 v0.6 h0.4" fill="none" strokeWidth={SW * 0.9} />
              : (<><path d="M-0.25,-0.45 h0.5 M0,-0.7 v0.5" /><path d="M-0.25,0.5 h0.5" /></>)}
        </g>
      );
    }
    case 'isource':
      return (<g><Lead y0={-2} y1={-1.1} /><circle r={1.1} fill="var(--surface-1)" /><Lead y0={1.1} y1={2} /><path d="M0,-0.6 V0.55 M-0.3,0.2 L0,0.6 L0.3,0.2" fill="none" strokeLinejoin="round" /></g>);
    case 'battery':
      return (
        <g>
          <Lead y0={-2} y1={-0.55} />
          <line x1={-0.95} y1={-0.55} x2={0.95} y2={-0.55} /><line x1={-0.5} y1={-0.2} x2={0.5} y2={-0.2} strokeWidth={SW * 2.2} />
          <line x1={-0.95} y1={0.2} x2={0.95} y2={0.2} /><line x1={-0.5} y1={0.55} x2={0.5} y2={0.55} strokeWidth={SW * 2.2} />
          <Lead y0={0.55} y1={2} /><path d="M0.75,-1.25 h0.45 M0.975,-1.475 v0.45" strokeWidth={SW * 0.8} />
        </g>
      );
    case 'diode':
    case 'led':
      return (
        <g>
          <Lead y0={-2} y1={-0.6} />
          <path d="M-0.7,-0.6 H0.7 L0,0.55 Z" fill={kind === 'led' ? `color-mix(in srgb, var(--series-5) ${Math.round(15 + 85 * glow)}%, var(--surface-1))` : 'var(--surface-1)'} strokeLinejoin="round" />
          <line x1={-0.7} y1={0.55} x2={0.7} y2={0.55} /><Lead y0={0.55} y1={2} />
          {kind === 'led' && (
            <g strokeWidth={SW * 0.8} opacity={0.5 + 0.5 * glow}>
              <path d="M0.85,-0.35 L1.55,-0.95 M1.2,-0.95 H1.55 V-0.6" fill="none" /><path d="M0.85,0.15 L1.55,-0.45 M1.2,-0.45 H1.55 V-0.1" fill="none" />
            </g>
          )}
        </g>
      );
    case 'switch': {
      const closed = on ?? props?.state !== 'open';
      return (
        <g>
          <Lead y0={-2} y1={-0.9} />
          <circle cx={0} cy={-0.9} r={0.14} fill="var(--surface-1)" /><circle cx={0} cy={0.9} r={0.14} fill="var(--surface-1)" />
          <line x1={0} y1={0.9} x2={closed ? 0 : -0.8} y2={closed ? -0.9 : -0.65} strokeWidth={SW * 1.3} />
          <Lead y0={0.9} y1={2} />
          {props?.state === 'pwm' && <path d="M0.5,0.3 h0.25 v-0.5 h0.35 v0.5 h0.25" fill="none" strokeWidth={SW * 0.7} />}
        </g>
      );
    }
    case 'vcvs':
    case 'vccs':
      return (<g><Lead y0={-2} y1={-1.1} /><path d="M0,-1.1 L1.1,0 L0,1.1 L-1.1,0 Z" fill="var(--surface-1)" strokeLinejoin="round" /><Lead y0={1.1} y1={2} /></g>);
    case 'nmos':
    case 'pmos': {
      const pm = kind === 'pmos';
      return (
        <g>
          <line x1={-2} y1={0} x2={pm ? -1.15 : -0.8} y2={0} />
          {pm && <circle cx={-0.97} cy={0} r={0.18} fill="var(--surface-1)" />}
          <line x1={-0.78} y1={-0.85} x2={-0.78} y2={0.85} />
          <line x1={-0.45} y1={-1.05} x2={-0.45} y2={-0.45} strokeWidth={SW * 1.4} />
          <line x1={-0.45} y1={-0.25} x2={-0.45} y2={0.25} strokeWidth={SW * 1.4} />
          <line x1={-0.45} y1={0.45} x2={-0.45} y2={1.05} strokeWidth={SW * 1.4} />
          <path d="M-0.45,-0.7 H0 V-2" fill="none" /><path d="M-0.45,0.7 H0 V2" fill="none" /><path d="M-0.45,0 H0" fill="none" />
          {pm ? <path d="M-0.1,-0.92 L-0.42,-0.7 L-0.1,-0.48" fill="none" strokeLinejoin="round" /> : <path d="M-0.32,0.48 L0,0.7 L-0.32,0.92" fill="none" strokeLinejoin="round" />}
        </g>
      );
    }
    case 'opamp':
      return (
        <g>
          <path d="M-2,-2 L-2,2 L2.4,0 Z" fill="var(--surface-1)" strokeLinejoin="round" />
          <line x1={-3} y1={-1} x2={-2} y2={-1} /><line x1={-3} y1={1} x2={-2} y2={1} /><line x1={2.4} y1={0} x2={3} y2={0} />
          <path d="M-1.7,-1 h0.45 M-1.475,-1.225 v0.45 M-1.7,1 h0.45" strokeWidth={SW * 0.9} />
        </g>
      );
    case 'ground':
      return (<g><line x1={-0.65} y1={0} x2={0.65} y2={0} /><line x1={-0.4} y1={0.28} x2={0.4} y2={0.28} /><line x1={-0.16} y1={0.56} x2={0.16} y2={0.56} /></g>);
    case 'probe':
      return (<g><circle r={0.3} fill="var(--accent)" stroke="none" /><path d="M0.2,-0.2 L1,-1" strokeWidth={SW * 1.2} /><circle cx={1.15} cy={-1.15} r={0.22} fill="var(--surface-1)" /></g>);
    case 'label':
      return <circle r={0.16} fill="var(--text)" stroke="none" />;
    default:
      return <circle r={1} fill="none" strokeDasharray="0.2 0.2" />;
  }
}

/** Where to write a part's name/value (not rotated with the symbol). */
export function labelPos(p: SchPart): { x: number; y: number; anchor: 'start' | 'middle' | 'end' } {
  const r = ((p.rot % 4) + 4) % 4;
  if (p.kind === 'opamp') return { x: p.x + 0.2, y: p.y - 2.5, anchor: 'middle' };
  if (p.kind === 'nmos' || p.kind === 'pmos') return { x: p.x + (p.flip ? -0.7 : 0.7), y: p.y - 0.15, anchor: p.flip ? 'end' : 'start' };
  if (p.kind === 'probe') return { x: p.x + 1.55, y: p.y - 1.05, anchor: 'start' };
  if (p.kind === 'label') return { x: p.x + 0.35, y: p.y - 0.35, anchor: 'start' };
  if (r === 1 || r === 3) return { x: p.x, y: p.y - 1.25, anchor: 'middle' };
  return { x: p.x + 1.45, y: p.y - 0.15, anchor: 'start' };
}

/** Short value text shown next to a symbol. */
export function valueText(p: SchPart): string {
  if (p.template) return p.value.length > 22 ? p.value.slice(0, 21) + '…' : p.value;
  const pr = p.props ?? {};
  switch (p.kind) {
    case 'vsource': case 'isource': {
      const u = p.kind === 'vsource' ? 'V' : 'A';
      if (pr.wave === 'pulse') return `⎍ ${pr.v1 ?? 0}→${pr.v2 ?? p.value} ${u}`;
      if (pr.wave === 'sin') return `∿ ${pr.va ?? p.value} ${u} ${pr.freq ?? ''}Hz`;
      return `${p.value} ${u}`;
    }
    case 'battery': return `${p.value} V`;
    case 'resistor': return `${p.value}Ω`;
    case 'capacitor': return `${p.value}F`;
    case 'inductor': return `${p.value}H`;
    case 'led': return 'LED';
    case 'diode': return '';
    case 'nmos': case 'pmos': return `W/L ${pr.w ?? '1m'}/${pr.l ?? '1u'}`;
    case 'switch': return pr.state === 'pwm' ? `PWM ${pr.freq ?? ''}Hz ${Math.round(Number(pr.duty ?? 0.5) * 100)}%` : pr.state ?? 'closed';
    case 'opamp': return `GBW ${pr.gbw ?? '1meg'}`;
    default: return '';
  }
}
