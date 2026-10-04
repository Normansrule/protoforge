// Block diagram of the generated architecture: subsystems as boxes in three
// bands (what the user meets · compute and power · physical build), interfaces
// as lines coloured by kind. Data lines flow while animation is on.
import type { Interface, Subsystem } from '../../engine/types';

const TOP_FIRST = ['SYS-UI', 'SYS-AUD', 'SYS-SNS', 'SYS-OPT', 'SYS-ACT', 'SYS-QRNG', 'SYS-QLINK'];
const MIDDLE = ['SYS-PWR', 'SYS-CMP', 'SYS-FW'];
const BOTTOM = ['SYS-PCB', 'SYS-STR', 'SYS-MFG'];

const DOMAIN_COLOR: Record<string, string> = {
  electrical: 'var(--series-1)',
  mechanical: 'var(--series-3)',
  optical: 'var(--series-5)',
  software: 'var(--series-7)',
  silicon: 'var(--series-2)',
  acoustic: 'var(--series-4)',
  thermal: 'var(--series-8)',
  manufacturing: 'var(--muted)',
};

export function ifaceClass(kind: string): 'power' | 'physical' | 'data' {
  const k = kind.toUpperCase();
  if (k === 'POWER') return 'power';
  if (k === 'MECH' || k === 'OPTICAL' || k === 'THERMAL') return 'physical';
  return 'data';
}

const IF_COLOR = { power: 'var(--series-4)', physical: 'var(--muted)', data: 'var(--accent)' };

interface Box { id: string; x: number; y: number; w: number; h: number; s: Subsystem }

function wrap(text: string, maxChars: number): string[] {
  const words = text.split(/\s+/);
  const lines: string[] = [];
  let cur = '';
  for (const w of words) {
    if ((cur + ' ' + w).trim().length > maxChars && cur) {
      lines.push(cur);
      cur = w;
    } else cur = (cur + ' ' + w).trim();
  }
  if (cur) lines.push(cur);
  if (lines.length > 2) return [lines[0], lines.slice(1).join(' ').slice(0, maxChars - 1) + '…'];
  return lines;
}

/** Point where the segment from the box centre toward (tx, ty) leaves the box. */
function clip(b: Box, tx: number, ty: number, pad = 2): [number, number] {
  const cx = b.x + b.w / 2, cy = b.y + b.h / 2;
  const dx = tx - cx, dy = ty - cy;
  if (dx === 0 && dy === 0) return [cx, cy];
  const sx = dx !== 0 ? (b.w / 2 + pad) / Math.abs(dx) : Infinity;
  const sy = dy !== 0 ? (b.h / 2 + pad) / Math.abs(dy) : Infinity;
  const s = Math.min(sx, sy);
  return [cx + dx * s, cy + dy * s];
}

export function ArchDiagram({ subsystems, interfaces, width, animate }: { subsystems: Subsystem[]; interfaces: Interface[]; width: number; animate: boolean }) {
  const W = Math.max(300, Math.round(width || 600));
  const narrow = W < 520;
  const showLabels = W >= 430;
  const pad = 10, gap = narrow ? 8 : 16, bh = narrow ? 48 : 54, vgap = narrow ? 40 : 48;
  const ids = subsystems.map((s) => s.id);
  const middle = MIDDLE.filter((i) => ids.includes(i));
  const bottom = BOTTOM.filter((i) => ids.includes(i));
  const top = [...TOP_FIRST.filter((i) => ids.includes(i)), ...ids.filter((i) => !TOP_FIRST.includes(i) && !MIDDLE.includes(i) && !BOTTOM.includes(i))];
  const perRow = narrow ? 2 : 4;
  const chunk = (arr: string[], n: number) => {
    const out: string[][] = [];
    for (let i = 0; i < arr.length; i += n) out.push(arr.slice(i, i + n));
    return out;
  };
  const rows = [...chunk(top, perRow), ...(middle.length ? [middle] : []), ...chunk(bottom, narrow ? 3 : 4)];
  const boxes: Box[] = [];
  let y = pad + 4;
  for (const row of rows) {
    const n = row.length;
    const bw = Math.min(narrow ? 140 : 150, (W - 2 * pad - (n - 1) * gap) / n);
    const total = n * bw + (n - 1) * gap;
    let x = (W - total) / 2;
    for (const id of row) {
      boxes.push({ id, x, y, w: bw, h: bh, s: subsystems.find((s) => s.id === id)! });
      x += bw + gap;
    }
    y += bh + vgap;
  }
  const H = y - vgap + pad + 4;
  const byId = new Map(boxes.map((b) => [b.id, b]));
  // parallel edges between the same pair get offsets
  const pairCount = new Map<string, number>();
  const pairIndex = new Map<string, number>();
  for (const i of interfaces) {
    const k = [i.from, i.to].sort().join('|');
    pairCount.set(k, (pairCount.get(k) ?? 0) + 1);
  }
  const minW = Math.min(...boxes.map((b) => b.w), 150);
  const maxChars = Math.max(8, Math.floor((minW - 14) / (narrow ? 6.6 : 7.2)));
  const label = subsystems.map((s) => s.name).join(', ');
  return (
    <svg className="wz-arch" viewBox={`0 0 ${W} ${H}`} width="100%" height={H} role="img" aria-label={`Architecture: ${subsystems.length} subsystems (${label}) joined by ${interfaces.length} interfaces`}>
      <g>
        {interfaces.map((i) => {
          const a = byId.get(i.from), b = byId.get(i.to);
          if (!a || !b) return null;
          const k = [i.from, i.to].sort().join('|');
          const n = pairCount.get(k) ?? 1;
          const idx = pairIndex.get(k) ?? 0;
          pairIndex.set(k, idx + 1);
          const off = (idx - (n - 1) / 2) * 9;
          const acx = a.x + a.w / 2, acy = a.y + a.h / 2, bcx = b.x + b.w / 2, bcy = b.y + b.h / 2;
          const len = Math.hypot(bcx - acx, bcy - acy) || 1;
          const nx = -(bcy - acy) / len * off, ny = (bcx - acx) / len * off;
          const [x1, y1] = clip({ ...a, x: a.x + nx, y: a.y + ny }, bcx + nx, bcy + ny);
          const [x2, y2] = clip({ ...b, x: b.x + nx, y: b.y + ny }, acx + nx, acy + ny);
          const cls = ifaceClass(i.kind);
          const mx = (x1 + x2) / 2, my = (y1 + y2) / 2;
          const tw = i.kind.length * 6 + 8;
          return (
            <g key={i.id}>
              <title>{`${i.id} ${i.kind}: ${i.description}`}</title>
              <line x1={x1} y1={y1} x2={x2} y2={y2} stroke={IF_COLOR[cls]} strokeWidth={cls === 'power' ? 2.2 : 1.6}
                strokeDasharray={cls === 'physical' ? '3 4' : cls === 'data' ? '6 6' : undefined}
                className={cls === 'data' && animate ? 'flow-line' : undefined} opacity={0.9} />
              {showLabels && Math.hypot(x2 - x1, y2 - y1) > tw + 6 && (
                <g transform={`translate(${mx},${my})`}>
                  <rect x={-tw / 2} y={-8} width={tw} height={16} rx={8} fill="var(--surface-1)" stroke={IF_COLOR[cls]} strokeOpacity={0.5} />
                  <text textAnchor="middle" y={4} fontSize={9.5} fill="var(--text-2)" fontFamily="var(--mono)">{i.kind}</text>
                </g>
              )}
            </g>
          );
        })}
      </g>
      {boxes.map((b) => {
        const lines = wrap(b.s.name, maxChars);
        const c = DOMAIN_COLOR[b.s.domain] ?? 'var(--accent)';
        return (
          <g key={b.id} className="wz-box">
            <title>{`${b.s.id} — ${b.s.name}: ${b.s.description}`}</title>
            <rect x={b.x} y={b.y} width={b.w} height={b.h} rx={10} fill="var(--surface-2)" stroke={c} strokeWidth={1.4} />
            <rect x={b.x} y={b.y + 8} width={3} height={b.h - 16} rx={1.5} fill={c} />
            <text x={b.x + 10} y={b.y + 15} fontSize={8.5} fill="var(--muted)" style={{ textTransform: 'uppercase', letterSpacing: 0.6 }}>{b.s.domain}</text>
            {lines.map((l, k) => (
              <text key={k} x={b.x + 10} y={b.y + (lines.length === 1 ? 34 : 30 + k * 13)} fontSize={narrow ? 10.5 : 11.5} fontWeight={600} fill="var(--text)">{l}</text>
            ))}
          </g>
        );
      })}
    </svg>
  );
}
