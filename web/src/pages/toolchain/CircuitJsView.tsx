// Draws CircuitJS text (the exact string packed into the Falstad link) as a
// small schematic, so you can see what will open before clicking.
import type { ReactNode } from 'react';

interface El { t: string; x1: number; y1: number; x2: number; y2: number; f: string[] }

export function parseCircuitJs(text: string): El[] {
  return text
    .split('\n')
    .slice(1)
    .map((l) => l.trim().split(/\s+/))
    .filter((t) => t.length >= 5)
    .map((t) => ({ t: t[0], x1: +t[1], y1: +t[2], x2: +t[3], y2: +t[4], f: t.slice(5) }));
}

function twoTerm(e: El, body: (len: number) => ReactNode, key: number, label?: string) {
  const dx = e.x2 - e.x1;
  const dy = e.y2 - e.y1;
  const len = Math.hypot(dx, dy) || 1;
  const ang = (Math.atan2(dy, dx) * 180) / Math.PI;
  const bl = Math.min(32, len * 0.7);
  return (
    <g key={key} transform={`translate(${e.x1} ${e.y1}) rotate(${ang})`}>
      <line x1={0} y1={0} x2={(len - bl) / 2} y2={0} />
      <line x1={(len + bl) / 2} y1={0} x2={len} y2={0} />
      <g transform={`translate(${(len - bl) / 2} 0)`}>{body(bl)}</g>
      {label && (
        <text x={len / 2} y={-10} textAnchor="middle" transform={Math.abs(ang) > 90 ? `rotate(180 ${len / 2} -10)` : undefined} className="cj-label">{label}</text>
      )}
    </g>
  );
}

function si(v: number, unit: string) {
  const a = Math.abs(v);
  const p: [number, string][] = [[1e9, 'G'], [1e6, 'M'], [1e3, 'k'], [1, ''], [1e-3, 'm'], [1e-6, 'µ'], [1e-9, 'n'], [1e-12, 'p'], [1e-15, 'f']];
  for (const [k, s] of p) if (a >= k * 0.9999) return `${+(v / k).toPrecision(3)}${s}${unit}`;
  return `${v}${unit}`;
}

export function CircuitJsView({ text, height = 220 }: { text: string; height?: number }) {
  const els = parseCircuitJs(text);
  if (els.length === 0) return null;
  const xs = els.flatMap((e) => [e.x1, e.x2]);
  const ys = els.flatMap((e) => [e.y1, e.y2]);
  const pad = 24;
  const vb = [Math.min(...xs) - pad, Math.min(...ys) - pad, Math.max(...xs) - Math.min(...xs) + 2 * pad, Math.max(...ys) - Math.min(...ys) + 2 * pad];
  const nodes: ReactNode[] = [];
  els.forEach((e, k) => {
    const v = Number(e.f[1]);
    switch (e.t) {
      case 'w':
        nodes.push(<line key={k} x1={e.x1} y1={e.y1} x2={e.x2} y2={e.y2} />);
        break;
      case 'r':
        nodes.push(twoTerm(e, (bl) => <polyline fill="none" points={Array.from({ length: 7 }, (_, i) => `${(i * bl) / 6},${i === 0 || i === 6 ? 0 : i % 2 ? -5 : 5}`).join(' ')} />, k, si(v, 'Ω')));
        break;
      case 'c':
        nodes.push(twoTerm(e, (bl) => <g><line x1={0} y1={0} x2={bl / 2 - 3} y2={0} /><line x1={bl / 2 + 3} y1={0} x2={bl} y2={0} /><line x1={bl / 2 - 3} y1={-8} x2={bl / 2 - 3} y2={8} strokeWidth={2} /><line x1={bl / 2 + 3} y1={-8} x2={bl / 2 + 3} y2={8} strokeWidth={2} /></g>, k, si(v, 'F')));
        break;
      case 'l':
        nodes.push(twoTerm(e, (bl) => <path fill="none" d={Array.from({ length: 4 }, (_, i) => `${i === 0 ? 'M0 0' : ''} a${bl / 8} ${bl / 8} 0 0 1 ${bl / 4} 0`).join(' ')} />, k, si(v, 'H')));
        break;
      case 'v': {
        const wf = Number(e.f[1]);
        const amp = Number(e.f[3]);
        const lab = wf === 0 ? `${+amp.toPrecision(3)} V` : wf === 1 ? `~${+amp.toPrecision(3)} V` : `⎍ ${+amp.toPrecision(3)} V`;
        nodes.push(twoTerm(e, (bl) => <g><circle cx={bl / 2} cy={0} r={10} fill="var(--surface-1)" /><text x={bl / 2 + 4} y={-3} className="cj-sym">+</text></g>, k, lab));
        break;
      }
      case 'i':
        nodes.push(twoTerm(e, (bl) => <g><circle cx={bl / 2} cy={0} r={10} fill="var(--surface-1)" /><path d={`M${bl / 2 - 5} 0 L${bl / 2 + 5} 0 M${bl / 2 + 1} -4 L${bl / 2 + 5} 0 L${bl / 2 + 1} 4`} fill="none" /></g>, k, si(v, 'A')));
        break;
      case 'd':
      case '162':
        nodes.push(twoTerm(e, (bl) => <g><path d={`M${bl / 2 - 6} -7 L${bl / 2 + 6} 0 L${bl / 2 - 6} 7 Z`} fill={e.t === '162' ? 'var(--bad)' : 'var(--surface-1)'} /><line x1={bl / 2 + 6} y1={-7} x2={bl / 2 + 6} y2={7} strokeWidth={2} /><line x1={0} y1={0} x2={bl / 2 - 6} y2={0} /><line x1={bl / 2 + 6} y1={0} x2={bl} y2={0} /></g>, k, e.t === '162' ? 'LED' : undefined));
        break;
      case 's':
        nodes.push(twoTerm(e, (bl) => <g><circle cx={3} cy={0} r={2} /><circle cx={bl - 3} cy={0} r={2} /><line x1={3} y1={0} x2={bl - 5} y2={e.f[1] === '0' ? -1 : -10} strokeWidth={1.6} /></g>, k));
        break;
      case 'f': {
        const pnp = e.f[0] === '1';
        nodes.push(
          <g key={k}>
            <line x1={e.x1} y1={e.y1} x2={e.x2 - 10} y2={e.y2} />
            <line x1={e.x2 - 10} y1={e.y2 - 10} x2={e.x2 - 10} y2={e.y2 + 10} strokeWidth={2} />
            <line x1={e.x2 - 5} y1={e.y2 - 16} x2={e.x2 - 5} y2={e.y2 + 16} strokeWidth={2} />
            <line x1={e.x2 - 5} y1={e.y2 - 16} x2={e.x2} y2={e.y2 - 16} />
            <line x1={e.x2 - 5} y1={e.y2 + 16} x2={e.x2} y2={e.y2 + 16} />
            {pnp && <circle cx={e.x2 - 13} cy={e.y2} r={3} fill="var(--surface-1)" />}
            <text x={e.x2 + 4} y={e.y2 + 4} className="cj-label">{pnp ? 'P' : 'N'}</text>
          </g>,
        );
        break;
      }
      case 'a': {
        const swap = (Number(e.f[0]) & 1) === 1;
        nodes.push(
          <g key={k}>
            <path d={`M${e.x1 + 8} ${e.y1 - 22} L${e.x2 - 8} ${e.y2} L${e.x1 + 8} ${e.y1 + 22} Z`} fill="var(--surface-1)" />
            <line x1={e.x1} y1={e.y1 - 16} x2={e.x1 + 8} y2={e.y1 - 16} />
            <line x1={e.x1} y1={e.y1 + 16} x2={e.x1 + 8} y2={e.y1 + 16} />
            <line x1={e.x2 - 8} y1={e.y2} x2={e.x2} y2={e.y2} />
            <text x={e.x1 + 11} y={e.y1 - 12} className="cj-sym">{swap ? '+' : '−'}</text>
            <text x={e.x1 + 11} y={e.y1 + 20} className="cj-sym">{swap ? '−' : '+'}</text>
          </g>,
        );
        break;
      }
      case 'g': {
        const d = Math.sign(e.y2 - e.y1) || 1;
        const y = e.y1 + 8 * d;
        nodes.push(<g key={k}><line x1={e.x1} y1={e.y1} x2={e.x1} y2={y} /><line x1={e.x1 - 9} y1={y} x2={e.x1 + 9} y2={y} /><line x1={e.x1 - 5} y1={y + 3 * d} x2={e.x1 + 5} y2={y + 3 * d} /><line x1={e.x1 - 2} y1={y + 6 * d} x2={e.x1 + 2} y2={y + 6 * d} /></g>);
        break;
      }
      case '207':
        nodes.push(<g key={k}><line x1={e.x1} y1={e.y1} x2={e.x2} y2={e.y2} /><text x={e.x2 + 2} y={e.y2 + (e.y2 < e.y1 ? -3 : 11)} className="cj-label">{e.f.slice(1).join(' ')}</text></g>);
        break;
      default:
        nodes.push(<line key={k} x1={e.x1} y1={e.y1} x2={e.x2} y2={e.y2} strokeDasharray="3 3" />);
    }
  });
  return (
    <svg className="cj-view" viewBox={vb.join(' ')} style={{ width: '100%', height, display: 'block' }} role="img" aria-label="CircuitJS circuit preview">
      <g stroke="var(--text-2)" strokeWidth={1.4} fill="none">{nodes}</g>
    </svg>
  );
}
