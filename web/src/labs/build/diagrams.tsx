// Small explanatory SVG diagrams for design rules and common failures,
// keyed by the `diagram` id the engine returns.
import type { ReactNode } from 'react';

const P = 'var(--series-3)';
const M = 'var(--muted)';
const T = 'var(--text-2)';
const BAD = 'var(--bad)';
const GOOD = 'var(--good)';
const C = 'var(--copper)';

function D({ children, label }: { children: ReactNode; label: string }) {
  return (
    <svg viewBox="0 0 120 72" className="bd-diagram" role="img" aria-label={label}>
      {children}
    </svg>
  );
}

const beads = (x: number, y: number, n: number, w = 10, h = 5, fill = P) =>
  Array.from({ length: n }, (_, k) => <ellipse key={`${x}-${y}-${k}`} cx={x + k * w} cy={y} rx={w / 2} ry={h / 2} fill={fill} opacity={0.8} />);

const DIAGRAMS: Record<string, () => ReactNode> = {
  wall: () => (
    <>
      {[0, 1, 2, 3].map((r) => <g key={r}>{beads(20, 60 - r * 6, 1, 10, 5, BAD)}</g>)}
      {[0, 1, 2, 3].map((r) => <g key={r}>{beads(70, 60 - r * 6, 2)}</g>)}
      <text x={20} y={16} textAnchor="middle" className="bd-svg-small">1 line ✕</text>
      <text x={75} y={16} textAnchor="middle" className="bd-svg-small">2 lines ✓</text>
    </>
  ),
  overhang: () => (
    <>
      <line x1={10} y1={64} x2={110} y2={64} stroke={M} />
      <polygon points="20,64 40,64 60,24 40,24" fill={P} opacity={0.8} />
      <polygon points="66,64 80,64 118,40 104,40" fill={BAD} opacity={0.6} />
      <text x={42} y={18} className="bd-svg-small">≤45° ✓</text>
      <text x={84} y={34} className="bd-svg-small">✕</text>
    </>
  ),
  bridge: () => (
    <>
      <rect x={14} y={24} width={14} height={40} fill={P} opacity={0.8} />
      <rect x={92} y={24} width={14} height={40} fill={P} opacity={0.8} />
      <path d="M28,26 Q60,38 92,26" fill="none" stroke={P} strokeWidth={4} />
      <text x={60} y={60} textAnchor="middle" className="bd-svg-small">span sags</text>
    </>
  ),
  hole: () => (
    <>
      <circle cx={34} cy={36} r={20} fill="none" stroke={T} strokeDasharray="3 2" />
      <polygon points={Array.from({ length: 8 }, (_, k) => `${34 + 16 * Math.cos((k * Math.PI) / 4)},${36 + 16 * Math.sin((k * Math.PI) / 4)}`).join(' ')} fill="none" stroke={BAD} />
      <path d="M86,16 L100,34 A14,14 0 1 1 72,34 Z" fill="none" stroke={GOOD} />
      <text x={34} y={68} textAnchor="middle" className="bd-svg-small">undersize</text>
      <text x={86} y={68} textAnchor="middle" className="bd-svg-small">teardrop</text>
    </>
  ),
  orient: () => (
    <>
      {[0, 1, 2, 3, 4].map((k) => <rect key={k} x={10} y={20 + k * 8} width={44} height={7} fill={P} opacity={0.7} />)}
      {[0, 1, 2, 3, 4, 5].map((k) => <rect key={k} x={70 + k * 7} y={16} width={6} height={44} fill={P} opacity={0.7} />)}
      <path d="M32,12 v-8 M28,8 l4,-4 l4,4" stroke={GOOD} fill="none" />
      <text x={32} y={70} textAnchor="middle" className="bd-svg-small">strong ✓</text>
      <text x={90} y={70} textAnchor="middle" className="bd-svg-small">splits ✕</text>
    </>
  ),
  layer: () => (
    <>
      <path d="M10,64 L10,52 L22,52 L22,40 L34,40 L34,28 L46,28 L46,16" fill="none" stroke={BAD} strokeWidth={2} />
      <path d="M64,64 L64,58 L70,58 L70,52 L76,52 L76,46 L82,46 L82,40 L88,40 L88,34 L94,34 L94,28 L100,28 L100,22 L106,22 L106,16" fill="none" stroke={GOOD} strokeWidth={2} />
      <text x={28} y={12} className="bd-svg-small">thick</text>
      <text x={80} y={12} className="bd-svg-small">thin</text>
    </>
  ),
  flow: () => (
    <>
      <rect x={10} y={30} width={60} height={12} rx={6} fill={P} opacity={0.8} />
      <text x={40} y={24} textAnchor="middle" className="bd-svg-small">h × w × v</text>
      <path d="M76,36 h30 M100,30 l6,6 l-6,6" stroke={T} fill="none" />
      <text x={90} y={58} textAnchor="middle" className="bd-svg-small">mm³/s</text>
    </>
  ),
  fit: () => (
    <>
      <rect x={20} y={14} width={34} height={44} fill="none" stroke={T} strokeWidth={3} />
      <rect x={27} y={21} width={20} height={30} fill={P} opacity={0.8} />
      <text x={70} y={32} className="bd-svg-small">clearance</text>
      <text x={70} y={46} className="bd-svg-small">per side</text>
    </>
  ),
  efoot: () => (
    <>
      <path d="M30,16 L30,56 Q30,62 24,64 L96,64 Q90,62 90,56 L90,16 Z" fill={P} opacity={0.8} />
      <text x={60} y={40} textAnchor="middle" className="bd-svg-small">bulge at the base</text>
    </>
  ),
  stringing: () => (
    <>
      <rect x={14} y={24} width={18} height={40} fill={P} opacity={0.8} />
      <rect x={88} y={24} width={18} height={40} fill={P} opacity={0.8} />
      {[0, 1, 2].map((k) => <path key={k} d={`M32,${30 + k * 8} Q60,${36 + k * 8} 88,${30 + k * 9}`} stroke={P} fill="none" strokeWidth={0.8} />)}
    </>
  ),
  warping: () => (
    <>
      <line x1={8} y1={62} x2={112} y2={62} stroke={M} />
      <path d="M16,50 Q60,62 104,50 L104,40 Q60,52 16,40 Z" fill={BAD} opacity={0.6} />
      <text x={60} y={24} textAnchor="middle" className="bd-svg-small">corners lift</text>
    </>
  ),
  delam: () => (
    <>
      {[0, 1, 2, 3, 4].map((k) => <rect key={k} x={30} y={16 + k * 9} width={60} height={7} fill={P} opacity={0.8} transform={k === 2 ? 'translate(4,0)' : ''} />)}
      <path d="M26,36 L94,34" stroke={BAD} strokeWidth={1.5} />
    </>
  ),
  underx: () => (
    <>
      {[0, 1, 2].map((r) => <g key={r}>{Array.from({ length: 5 }, (_, k) => <ellipse key={k} cx={22 + k * 19} cy={24 + r * 14} rx={6} ry={4} fill={P} opacity={0.8} />)}</g>)}
      <text x={60} y={68} textAnchor="middle" className="bd-svg-small">gaps between lines</text>
    </>
  ),
  adhesion: () => (
    <>
      <line x1={8} y1={60} x2={112} y2={60} stroke={M} strokeWidth={2} />
      <rect x={30} y={28} width={60} height={24} fill={P} opacity={0.7} transform="rotate(-8 60 40)" />
      <text x={60} y={20} textAnchor="middle" className="bd-svg-small">not stuck</text>
    </>
  ),
  ringing: () => (
    <>
      <rect x={14} y={14} width={20} height={48} fill={P} opacity={0.8} />
      <path d="M34,20 q6,4 12,0 t12,0 t12,0 t12,0 t12,0 t12,0" fill="none" stroke={T} />
      <path d="M34,34 q6,3 12,0 t12,0 t12,0 t12,0" fill="none" stroke={T} />
      <text x={70} y={58} textAnchor="middle" className="bd-svg-small">echoes</text>
    </>
  ),
  drain: () => (
    <>
      <rect x={24} y={12} width={72} height={48} rx={6} fill="none" stroke={P} strokeWidth={5} />
      <circle cx={30} cy={56} r={4} fill="var(--bg)" stroke={GOOD} />
      <circle cx={90} cy={16} r={4} fill="var(--bg)" stroke={GOOD} />
      <text x={60} y={40} textAnchor="middle" className="bd-svg-small">2 holes</text>
    </>
  ),
  cupping: () => (
    <>
      <path d="M24,20 L24,50 L96,50 L96,20" fill="none" stroke={BAD} strokeWidth={5} />
      <line x1={10} y1={58} x2={110} y2={58} stroke={T} strokeDasharray="3 2" />
      <text x={60} y={38} textAnchor="middle" className="bd-svg-small">suction</text>
      <text x={60} y={70} textAnchor="middle" className="bd-svg-small">film</text>
    </>
  ),
  tilt: () => (
    <>
      <line x1={8} y1={62} x2={112} y2={62} stroke={T} strokeDasharray="3 2" />
      <rect x={40} y={18} width={50} height={14} fill={P} opacity={0.8} transform="rotate(30 65 25)" />
      {[0, 1, 2].map((k) => <line key={k} x1={40 + k * 14} y1={62} x2={48 + k * 14} y2={40 - k * 6} stroke={M} />)}
    </>
  ),
  feature: () => (
    <>
      <rect x={10} y={50} width={100} height={12} fill={P} opacity={0.8} />
      {[0, 1, 2, 3].map((k) => <rect key={k} x={20 + k * 24} y={50 - (8 + k * 6)} width={2 + k * 2} height={8 + k * 6} fill={k === 0 ? BAD : P} opacity={0.8} />)}
      <text x={60} y={70} textAnchor="middle" className="bd-svg-small">smallest feature</text>
    </>
  ),
  kerf: () => (
    <>
      <rect x={10} y={24} width={44} height={30} fill="var(--series-4)" opacity={0.5} />
      <rect x={60} y={24} width={50} height={30} fill="var(--series-4)" opacity={0.5} />
      <line x1={57} y1={10} x2={57} y2={24} stroke="var(--series-8)" strokeWidth={4} />
      <text x={57} y={68} textAnchor="middle" className="bd-svg-small">kerf</text>
    </>
  ),
  corner: () => (
    <>
      <path d="M20,14 L20,58 L100,58" fill="none" stroke={T} strokeWidth={2} />
      <path d="M20,14 L20,44 A14,14 0 0 0 34,58 L100,58" fill="none" stroke={GOOD} strokeWidth={2} />
      <circle cx={34} cy={44} r={12} fill="none" stroke={M} strokeDasharray="2 2" />
      <text x={70} y={34} className="bd-svg-small">R ≥ tool R</text>
    </>
  ),
  pocket: () => (
    <>
      <rect x={10} y={10} width={100} height={56} fill="var(--surface-3)" />
      <rect x={46} y={10} width={28} height={50} fill="var(--bg)" />
      <rect x={52} y={4} width={16} height={50} fill={M} />
      <text x={88} y={40} className="bd-svg-small">≤ 4·D</text>
    </>
  ),
  chip: () => (
    <>
      <rect x={10} y={40} width={100} height={24} fill="var(--surface-3)" />
      <circle cx={60} cy={30} r={14} fill={M} />
      <path d="M44,40 q-6,-10 -2,-18 q6,-4 8,4" fill="none" stroke={C} strokeWidth={2} />
      <text x={86} y={24} className="bd-svg-small">chip</text>
    </>
  ),
  bend: () => (
    <>
      <path d="M10,56 L60,56 A10,10 0 0 0 70,46 L70,10" fill="none" stroke="var(--series-1)" strokeWidth={6} />
      <path d="M10,56 L60,56 A10,10 0 0 0 70,46 L70,10" fill="none" stroke="var(--warn)" strokeDasharray="3 2" />
      <text x={78} y={40} className="bd-svg-small">R, T, K</text>
    </>
  ),
  flange: () => (
    <>
      <path d="M10,56 L80,56 L80,30" fill="none" stroke="var(--series-1)" strokeWidth={5} />
      <line x1={88} y1={56} x2={88} y2={30} stroke={T} />
      <text x={92} y={46} className="bd-svg-small">≥ 4T</text>
    </>
  ),
  relief: () => (
    <>
      <rect x={10} y={14} width={100} height={46} fill="var(--series-1)" opacity={0.35} />
      <line x1={10} y1={36} x2={80} y2={36} stroke={T} strokeDasharray="4 2" />
      <rect x={78} y={30} width={8} height={30} fill="var(--bg)" />
      <text x={60} y={30} className="bd-svg-small">bend line</text>
      <text x={90} y={68} className="bd-svg-small">relief</text>
    </>
  ),
  stencil: () => (
    <>
      <rect x={10} y={20} width={100} height={10} fill={M} />
      <rect x={40} y={20} width={40} height={10} fill="var(--bg)" />
      <rect x={40} y={30} width={40} height={8} fill="var(--text-2)" opacity={0.6} />
      <rect x={34} y={38} width={52} height={6} fill={C} />
      <text x={60} y={60} textAnchor="middle" className="bd-svg-small">area / wall area ≥ 0.66</text>
    </>
  ),
  coat: () => (
    <>
      <rect x={10} y={44} width={100} height={10} fill="var(--mask)" />
      <rect x={24} y={34} width={20} height={10} fill={M} />
      <rect x={70} y={30} width={26} height={14} fill={M} />
      <path d="M8,42 L24,42 L24,32 L44,32 L44,42 L70,42 L70,28 L96,28 L96,42 L112,42" fill="none" stroke="var(--accent)" strokeWidth={1.5} />
      <text x={60} y={68} textAnchor="middle" className="bd-svg-small">thin conformal film</text>
    </>
  ),
  tombstone: () => (
    <>
      <rect x={20} y={52} width={20} height={6} fill={C} />
      <rect x={70} y={52} width={20} height={6} fill={C} />
      <rect x={26} y={16} width={12} height={36} fill={M} />
      <text x={80} y={36} textAnchor="middle" className="bd-svg-small">tombstone</text>
    </>
  ),
  cold: () => (
    <>
      <rect x={20} y={50} width={80} height={8} fill={C} />
      <rect x={44} y={26} width={32} height={24} fill={M} />
      <path d="M34,50 Q40,40 44,40 M86,50 Q80,40 76,40" stroke={GOOD} fill="none" strokeWidth={2} />
      <text x={60} y={18} textAnchor="middle" className="bd-svg-small">good fillet: concave, shiny</text>
    </>
  ),
  edge: () => (
    <>
      <rect x={10} y={10} width={100} height={52} fill="var(--mask)" opacity={0.8} />
      <line x1={110} y1={10} x2={110} y2={62} stroke={T} strokeWidth={2} />
      <rect x={20} y={30} width={70} height={6} fill={C} />
      <text x={100} y={50} textAnchor="end" className="bd-svg-small">≥ edge gap</text>
    </>
  ),
  annular: () => (
    <>
      <circle cx={40} cy={36} r={22} fill={C} />
      <circle cx={43} cy={34} r={11} fill="var(--bg)" />
      <text x={76} y={40} className="bd-svg-small">ring</text>
    </>
  ),
  molding: () => (
    <>
      <rect x={10} y={20} width={50} height={32} fill="var(--surface-3)" />
      <rect x={60} y={14} width={50} height={44} fill={M} opacity={0.5} />
      <rect x={70} y={26} width={30} height={20} fill="var(--series-2)" opacity={0.8} />
      <text x={60} y={68} textAnchor="middle" className="bd-svg-small">fill → pack → cool</text>
    </>
  ),
  sink: () => (
    <>
      <path d="M10,20 L50,20 Q60,28 70,20 L110,20 L110,40 L10,40 Z" fill={P} opacity={0.8} />
      <rect x={52} y={40} width={16} height={24} fill={P} opacity={0.8} />
      <text x={60} y={14} textAnchor="middle" className="bd-svg-small">sink over a thick rib</text>
    </>
  ),
  flash: () => (
    <>
      <rect x={20} y={16} width={80} height={40} fill={P} opacity={0.8} />
      <path d="M100,36 L114,34 L112,38 Z" fill={BAD} />
      <line x1={10} y1={36} x2={116} y2={36} stroke={T} strokeDasharray="3 2" />
      <text x={60} y={68} textAnchor="middle" className="bd-svg-small">parting line</text>
    </>
  ),
  splay: () => (
    <>
      <rect x={10} y={16} width={100} height={40} fill={P} opacity={0.8} />
      {[0, 1, 2, 3].map((k) => <path key={k} d={`M${20 + k * 20},24 l14,20`} stroke="var(--surface-1)" strokeWidth={1.5} />)}
    </>
  ),
};

export function Diagram({ id, label }: { id: string; label: string }) {
  const f = DIAGRAMS[id] ?? DIAGRAMS.fit;
  return <D label={label}>{f()}</D>;
}
