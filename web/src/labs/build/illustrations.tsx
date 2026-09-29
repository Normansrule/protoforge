// Animated process illustrations for the settings cards. Each is a pure SVG
// function of time `t` (seconds) so play/pause and reduced motion are handled
// by the caller. Colours come from theme variables only.
import type { ReactNode } from 'react';

const W = 360;
const H = 210;

function Frame({ children, label }: { children: ReactNode; label: string }) {
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="bd-anim" role="img" aria-label={label}>
      {children}
    </svg>
  );
}

const frac = (x: number) => x - Math.floor(x);

// ---------------------------------------------------------------------------
// FDM: nozzle laying lines, layer height and line width annotated
// ---------------------------------------------------------------------------

export function FdmAnim({ t, layer = 0.2, width = 0.45, nozzle = 0.4 }: { t: number; layer?: number; width?: number; nozzle?: number }) {
  const s = 70; // px per mm
  const lh = layer * s;
  const lw = width * s;
  const bedY = 176;
  const x0 = 24, x1 = 220;
  const period = 3.2;
  const pass = Math.floor(t / period);
  const p = frac(t / period);
  const layers = 3 + (pass % 3);
  const nx = x0 + (x1 - x0) * p;
  const topY = bedY - layers * lh;
  const nozW = nozzle * s;
  return (
    <Frame label="FDM nozzle depositing a line on previous layers">
      <rect x={0} y={bedY} width={250} height={H - bedY} fill="var(--surface-3)" />
      <text x={8} y={H - 10} className="bd-svg-label">heated bed</text>
      {Array.from({ length: layers }, (_, k) => (
        <rect key={k} x={x0} y={bedY - (k + 1) * lh} width={x1 - x0} height={lh - 1} rx={lh / 2} fill="var(--series-3)" opacity={0.45 + 0.1 * (k % 2)} />
      ))}
      <rect x={x0} y={topY - lh} width={Math.max(0, nx - x0)} height={lh - 1} rx={lh / 2} fill="var(--series-3)" />
      {/* nozzle */}
      <g transform={`translate(${nx},${topY - lh})`}>
        <rect x={-16} y={-64} width={32} height={34} rx={3} fill="var(--surface-3)" stroke="var(--border-strong)" />
        <text x={0} y={-43} textAnchor="middle" className="bd-svg-small">heater</text>
        <path d={`M-12,-30 L12,-30 L${nozW / 2},-2 L${-nozW / 2},-2 Z`} fill="var(--copper)" />
        <rect x={-3} y={-84} width={6} height={20} fill="var(--series-3)" opacity={0.7} />
        <circle cx={-24} cy={-10} r={4} fill="var(--accent)" opacity={0.5 + 0.5 * Math.abs(Math.sin(t * 8))} />
        <text x={-30} y={-18} textAnchor="end" className="bd-svg-small">fan</text>
      </g>
      {/* dimension: layer height */}
      <line x1={x1 + 8} y1={topY} x2={x1 + 8} y2={topY + lh} stroke="var(--text-2)" />
      <text x={x1 + 12} y={topY + lh / 2 + 4} className="bd-svg-small">{layer.toFixed(2)} mm layer</text>
      {/* cross-section inset */}
      <g transform="translate(262,40)">
        <rect x={-8} y={-22} width={96} height={150} rx={8} fill="var(--surface-2)" stroke="var(--border)" />
        <text x={40} y={-8} textAnchor="middle" className="bd-svg-small">cross-section</text>
        {Array.from({ length: 3 }, (_, r) =>
          Array.from({ length: 2 }, (_, c) => <ellipse key={`${r}-${c}`} cx={22 + c * lw * 0.9} cy={100 - r * lh} rx={lw / 2} ry={lh / 2} fill="var(--series-3)" opacity={0.75} />),
        )}
        <line x1={22 - lw / 2} y1={100 + lh / 2 + 8} x2={22 + lw / 2} y2={100 + lh / 2 + 8} stroke="var(--text-2)" />
        <text x={40} y={100 + lh / 2 + 22} textAnchor="middle" className="bd-svg-small">{width.toFixed(2)} mm line</text>
        <text x={40} y={16} textAnchor="middle" className="bd-svg-small">2 lines = min wall</text>
      </g>
    </Frame>
  );
}

// ---------------------------------------------------------------------------
// Resin (MSLA): UV mask cures a layer, plate peels and lowers
// ---------------------------------------------------------------------------

export function ResinAnim({ t }: { t: number }) {
  const period = 2.4;
  const p = frac(t / period);
  const n = 4 + (Math.floor(t / period) % 6);
  const cure = p < 0.45;
  const lift = p < 0.45 ? 0 : p < 0.7 ? (p - 0.45) / 0.25 : 1 - (p - 0.7) / 0.3;
  const plateY = 60 - lift * 30;
  const layerH = 6;
  return (
    <Frame label="Resin printer: UV light cures a layer through an LCD mask, then the plate lifts">
      <rect x={60} y={120} width={240} height={40} fill="var(--accent-soft)" stroke="var(--border-strong)" />
      <text x={66} y={134} className="bd-svg-small">resin vat</text>
      <line x1={60} y1={160} x2={300} y2={160} stroke="var(--text-2)" strokeDasharray="4 3" />
      <text x={304} y={163} className="bd-svg-small">film</text>
      <rect x={60} y={166} width={240} height={14} fill="var(--surface-3)" />
      <text x={66} y={177} className="bd-svg-small">LCD mask</text>
      <rect x={140} y={166} width={80} height={14} fill={cure ? 'var(--series-7)' : 'var(--surface-3)'} opacity={cure ? 0.9 : 1} />
      {cure && Array.from({ length: 6 }, (_, k) => <line key={k} x1={146 + k * 14} y1={196} x2={146 + k * 14} y2={180} stroke="var(--series-7)" strokeWidth={2} opacity={0.4 + 0.6 * Math.abs(Math.sin(t * 10 + k))} />)}
      <text x={180} y={206} textAnchor="middle" className="bd-svg-small">405 nm UV</text>
      {/* plate + hanging part */}
      <rect x={120} y={plateY - 12} width={120} height={12} fill="var(--surface-3)" stroke="var(--border-strong)" />
      <rect x={176} y={20} width={8} height={Math.max(0, plateY - 32)} fill="var(--muted)" />
      {Array.from({ length: n }, (_, k) => <rect key={k} x={140} y={plateY + k * layerH} width={80} height={layerH - 0.8} fill="var(--series-5)" opacity={0.85} />)}
      <text x={250} y={plateY - 2} className="bd-svg-small">{cure ? 'curing layer' : lift > 0.05 ? 'peel / lift' : ''}</text>
    </Frame>
  );
}

// ---------------------------------------------------------------------------
// Powder bed: recoater spreads powder, laser fuses the cross-section
// ---------------------------------------------------------------------------

export function PowderAnim({ t }: { t: number }) {
  const period = 3;
  const p = frac(t / period);
  const n = 3 + (Math.floor(t / period) % 5);
  const bedTop = 110;
  const recX = p < 0.35 ? 40 + (p / 0.35) * 280 : 320;
  const scanning = p >= 0.35 && p < 0.95;
  const sp = (p - 0.35) / 0.6;
  const row = Math.floor(sp * 6);
  const lx = 120 + (row % 2 === 0 ? frac(sp * 6) : 1 - frac(sp * 6)) * 120;
  const dots: ReactNode[] = [];
  for (let i = 0; i < 90; i++) {
    const x = 42 + ((i * 37) % 276);
    const y = bedTop + 6 + ((i * 53) % 76);
    dots.push(<circle key={i} cx={x} cy={y} r={1.6} fill="var(--muted)" opacity={0.5} />);
  }
  return (
    <Frame label="Powder bed fusion: recoater spreads powder, laser fuses each layer">
      <rect x={40} y={bedTop} width={280} height={90} fill="var(--surface-3)" />
      {dots}
      {Array.from({ length: n }, (_, k) => <rect key={k} x={120} y={bedTop + 60 - k * 8} width={120} height={7} fill="var(--series-4)" opacity={0.85} />)}
      <rect x={recX - 4} y={bedTop - 30} width={8} height={30} fill="var(--text-2)" />
      <text x={recX} y={bedTop - 34} textAnchor="middle" className="bd-svg-small">recoater</text>
      {scanning && (
        <>
          <polygon points={`170,14 190,14 ${lx + 2},${bedTop + 60 - n * 8 + 8} ${lx - 2},${bedTop + 60 - n * 8 + 8}`} fill="var(--series-2)" opacity={0.35} />
          <circle cx={lx} cy={bedTop + 60 - n * 8 + 8} r={4} fill="var(--series-2)" />
        </>
      )}
      <text x={46} y={bedTop + 84} className="bd-svg-small">unfused powder supports the part</text>
    </Frame>
  );
}

// ---------------------------------------------------------------------------
// Laser: focused beam, kerf behind the head, compensation in top view
// ---------------------------------------------------------------------------

export function LaserAnim({ t, kerf = 0.2 }: { t: number; kerf?: number }) {
  const p = frac(t / 4);
  const hx = 30 + p * 190;
  const kw = Math.max(3, kerf * 30);
  return (
    <Frame label="Laser cutting: the beam removes a kerf; the path is offset by half the kerf">
      <rect x={20} y={130} width={210} height={26} fill="var(--series-4)" opacity={0.5} />
      <rect x={30} y={130} width={Math.max(0, hx - 30)} height={26} fill="var(--bg)" opacity={0} />
      {/* kerf slot behind head */}
      <rect x={30} y={130} width={Math.max(0, hx - 30)} height={26} fill="none" />
      <line x1={30} y1={143} x2={hx} y2={143} stroke="var(--bg)" strokeWidth={kw} />
      <g transform={`translate(${hx},0)`}>
        <rect x={-14} y={40} width={28} height={40} rx={4} fill="var(--surface-3)" stroke="var(--border-strong)" />
        <ellipse cx={0} cy={80} rx={12} ry={4} fill="var(--accent-soft)" stroke="var(--accent)" />
        <polygon points="-9,84 9,84 0,130" fill="var(--series-8)" opacity={0.55} />
        {Array.from({ length: 5 }, (_, k) => <circle key={k} cx={-4 - k * 4 * frac(t * 3 + k * 0.2)} cy={158 + k * 3 * frac(t * 3 + k * 0.3)} r={1.4} fill="var(--warn)" />)}
      </g>
      <text x={24} y={124} className="bd-svg-small">sheet — beam focused at the surface</text>
      <text x={24} y={176} className="bd-svg-small">kerf ≈ {kerf.toFixed(2)} mm</text>
      {/* top view */}
      <g transform="translate(250,30)">
        <rect x={0} y={0} width={100} height={150} rx={8} fill="var(--surface-2)" stroke="var(--border)" />
        <text x={50} y={16} textAnchor="middle" className="bd-svg-small">top view</text>
        <rect x={25} y={40} width={50} height={70} fill="none" stroke="var(--text-2)" strokeDasharray="3 3" />
        <rect x={25 - kw / 4} y={40 - kw / 4} width={50 + kw / 2} height={70 + kw / 2} fill="none" stroke="var(--series-8)" strokeWidth={1.5} />
        <text x={50} y={128} textAnchor="middle" className="bd-svg-small">part (dashed)</text>
        <text x={50} y={141} textAnchor="middle" className="bd-svg-small">path +kerf/2</text>
      </g>
    </Frame>
  );
}

// ---------------------------------------------------------------------------
// CNC: end mill with moving flutes and flying chips
// ---------------------------------------------------------------------------

export function CncAnim({ t, rpmScale = 1 }: { t: number; rpmScale?: number }) {
  const p = frac(t / 5);
  const tx = 50 + p * 170;
  const spin = t * 6 * rpmScale;
  const flutes = 3;
  const chips: ReactNode[] = [];
  for (let k = 0; k < 6; k++) {
    const a = frac(t * 1.5 * rpmScale + k / 6);
    chips.push(<path key={k} d="M0,0 q3,-4 6,0 q3,4 6,0" fill="none" stroke="var(--copper)" strokeWidth={1.6} transform={`translate(${tx - 18 - a * 40},${110 - a * 50 + a * a * 60}) rotate(${a * 400})`} opacity={1 - a} />);
  }
  return (
    <Frame label="CNC end mill cutting a step, chips flying off">
      <rect x={20} y={120} width={250} height={60} fill="var(--surface-3)" stroke="var(--border-strong)" />
      <rect x={20} y={104} width={Math.max(0, tx - 12 - 20)} height={16} fill="var(--surface-3)" opacity={0} />
      <rect x={tx + 12} y={104} width={Math.max(0, 270 - tx - 12)} height={16} fill="var(--surface-3)" stroke="var(--border-strong)" />
      <text x={24} y={196} className="bd-svg-small">workpiece — the tool removes the top step</text>
      <g transform={`translate(${tx},0)`}>
        <rect x={-20} y={10} width={40} height={40} rx={4} fill="var(--surface-2)" stroke="var(--border-strong)" />
        <text x={0} y={34} textAnchor="middle" className="bd-svg-small">spindle</text>
        <rect x={-12} y={50} width={24} height={70} fill="var(--muted)" />
        <clipPath id="bd-flute"><rect x={-12} y={62} width={24} height={58} /></clipPath>
        <g clipPath="url(#bd-flute)">
          {Array.from({ length: 10 }, (_, k) => {
            const y = 62 + ((k * 12 + spin * 14) % 120) - 12;
            return <line key={k} x1={-12} y1={y} x2={12} y2={y + 10} stroke="var(--text)" strokeWidth={2.5} opacity={0.55} />;
          })}
        </g>
      </g>
      {chips}
      {/* top view spinning cutter */}
      <g transform="translate(318,60)">
        <circle r={30} fill="var(--surface-2)" stroke="var(--border-strong)" />
        {Array.from({ length: flutes }, (_, k) => {
          const a = spin + (k * 2 * Math.PI) / flutes;
          return <path key={k} d={`M0,0 L${26 * Math.cos(a)},${26 * Math.sin(a)} A26,26 0 0 1 ${26 * Math.cos(a + 0.6)},${26 * Math.sin(a + 0.6)} Z`} fill="var(--muted)" />;
        })}
        <text y={48} textAnchor="middle" className="bd-svg-small">top view</text>
      </g>
    </Frame>
  );
}

// ---------------------------------------------------------------------------
// Sheet metal: press brake bending
// ---------------------------------------------------------------------------

export function SheetAnim({ t, angle = 90 }: { t: number; angle?: number }) {
  const p = frac(t / 3.5);
  const prog = p < 0.5 ? p / 0.5 : p < 0.8 ? 1 : 1 - (p - 0.8) / 0.2;
  const a = (angle * prog * Math.PI) / 180 / 2;
  const cx = 180, cy = 130;
  const L = 110;
  const lx = cx - L * Math.cos(a), ly = cy - L * Math.sin(a);
  const rx = cx + L * Math.cos(a), ry = cy - L * Math.sin(a);
  return (
    <Frame label="Press brake: the punch pushes the sheet into a V-die">
      <polygon points={`${cx - 60},190 ${cx - 14},190 ${cx},150 ${cx + 14},190 ${cx + 60},190 ${cx + 60},206 ${cx - 60},206`} fill="var(--surface-3)" stroke="var(--border-strong)" />
      <polygon points={`${cx - 40},196 ${cx},140 ${cx + 40},196`} fill="none" />
      <path d={`M${cx - 44},186 L${cx},142 L${cx + 44},186`} fill="none" stroke="var(--border-strong)" />
      <text x={cx + 66} y={200} className="bd-svg-small">V-die</text>
      <polyline points={`${lx},${ly} ${cx},${cy} ${rx},${ry}`} fill="none" stroke="var(--series-1)" strokeWidth={6} strokeLinejoin="round" />
      <polyline points={`${lx},${ly} ${cx},${cy} ${rx},${ry}`} fill="none" stroke="var(--warn)" strokeWidth={1} strokeDasharray="4 3" />
      <polygon points={`${cx - 16},${cy - 70} ${cx + 16},${cy - 70} ${cx},${cy - 4}`} transform={`translate(0,${0})`} fill="var(--muted)" />
      <rect x={cx - 26} y={cy - 110} width={52} height={40} fill="var(--surface-2)" stroke="var(--border-strong)" />
      <text x={cx} y={cy - 86} textAnchor="middle" className="bd-svg-small">punch</text>
      <text x={16} y={24} className="bd-svg-small">dashed = neutral axis (K · T from the inside)</text>
      <text x={16} y={40} className="bd-svg-small">bend {Math.round(angle * prog)}° of {angle}°</text>
    </Frame>
  );
}

// ---------------------------------------------------------------------------
// Injection molding cycle
// ---------------------------------------------------------------------------

export function MoldingAnim({ t }: { t: number }) {
  const period = 4;
  const p = frac(t / period);
  const phase = p < 0.35 ? 'fill' : p < 0.55 ? 'pack / hold' : p < 0.8 ? 'cool' : 'open + eject';
  const fill = Math.min(1, p / 0.35);
  const open = p >= 0.8 ? Math.min(1, (p - 0.8) / 0.1) : 0;
  const screwX = 40 + fill * 50;
  return (
    <Frame label="Injection molding cycle: fill, pack, cool, eject">
      <rect x={20} y={90} width={150} height={36} rx={4} fill="var(--surface-3)" stroke="var(--border-strong)" />
      {Array.from({ length: 8 }, (_, k) => <line key={k} x1={screwX + k * 12} y1={94} x2={screwX + k * 12 + 8} y2={122} stroke="var(--muted)" strokeWidth={3} />)}
      <rect x={screwX + 96} y={96} width={Math.max(0, 170 - screwX - 96)} height={24} fill="var(--series-2)" opacity={0.7} />
      <rect x={24} y={60} width={40} height={30} fill="var(--surface-2)" stroke="var(--border-strong)" />
      <text x={44} y={80} textAnchor="middle" className="bd-svg-small">hopper</text>
      {Array.from({ length: 6 }, (_, k) => <rect key={k} x={30 + k * 22} y={126} width={14} height={6} fill="var(--series-8)" opacity={0.6} />)}
      <text x={30} y={148} className="bd-svg-small">heater bands</text>
      {/* mold halves */}
      <g transform={`translate(${-open * 20},0)`}>
        <rect x={180} y={50} width={60} height={116} fill="var(--surface-2)" stroke="var(--border-strong)" />
        <rect x={228} y={80} width={12} height={56} fill="var(--bg)" />
        <rect x={228} y={80 + 56 * (1 - fill)} width={12} height={56 * fill} fill={phase === 'cool' || phase === 'open + eject' ? 'var(--series-1)' : 'var(--series-2)'} opacity={0.8} />
      </g>
      <g transform={`translate(${open * 40},0)`}>
        <rect x={240} y={50} width={60} height={116} fill="var(--surface-2)" stroke="var(--border-strong)" />
        <rect x={240} y={80} width={12} height={56} fill="var(--bg)" />
        <rect x={240} y={80 + 56 * (1 - fill)} width={12} height={56 * fill} fill={phase === 'cool' || phase === 'open + eject' ? 'var(--series-1)' : 'var(--series-2)'} opacity={0.8} />
        {open > 0 && <rect x={300} y={100} width={20 * open} height={8} fill="var(--muted)" />}
      </g>
      <text x={180} y={188} className="bd-svg-label">{phase}</text>
      <text x={180} y={40} className="bd-svg-small">mold (cavity + core)</text>
    </Frame>
  );
}

// ---------------------------------------------------------------------------
// PCB: trace/space, drill wander vs annular ring
// ---------------------------------------------------------------------------

export function PcbAnim({ t }: { t: number }) {
  const wx = 3 * Math.sin(t * 2.1), wy = 3 * Math.cos(t * 1.7);
  return (
    <Frame label="PCB features: trace width, spacing and annular ring with drill wander">
      <rect x={10} y={20} width={340} height={170} rx={10} fill="var(--mask)" opacity={0.85} />
      <rect x={30} y={60} width={150} height={10} fill="var(--copper)" />
      <rect x={30} y={84} width={150} height={10} fill="var(--copper)" />
      <line x1={188} y1={60} x2={188} y2={70} stroke="#fff" />
      <text x={192} y={69} className="bd-svg-small" fill="#fff">trace</text>
      <line x1={188} y1={70} x2={188} y2={84} stroke="#fff" strokeDasharray="2 2" />
      <text x={192} y={82} className="bd-svg-small" fill="#fff">space</text>
      <circle cx={100} cy={140} r={26} fill="var(--copper)" />
      <circle cx={100 + wx} cy={140 + wy} r={13} fill="var(--bg)" />
      <text x={134} y={136} className="bd-svg-small" fill="#fff">annular ring = (pad − drill) / 2</text>
      <text x={134} y={150} className="bd-svg-small" fill="#fff">drill wander eats into it</text>
      <g transform="translate(270,110)">
        {Array.from({ length: 4 }, (_, k) => <rect key={k} x={-40} y={-30 + k * 16} width={80} height={k % 3 === 0 ? 4 : 10} fill={k % 3 === 0 ? 'var(--copper)' : 'var(--surface-3)'} opacity={0.9} />)}
        <text x={0} y={46} textAnchor="middle" className="bd-svg-small" fill="#fff">2-layer stack</text>
      </g>
    </Frame>
  );
}

// ---------------------------------------------------------------------------
// Reflow (mini): board through heated zones
// ---------------------------------------------------------------------------

export function ReflowMini({ t, zones = [110, 175, 190, 240, 250, 90] }: { t: number; zones?: number[] }) {
  const n = zones.length;
  const zw = 300 / n;
  const bx = 20 + frac(t / 8) * 340 - 20;
  return (
    <Frame label="Reflow oven: the board rides a conveyor through heated zones">
      {zones.map((z, k) => (
        <g key={k}>
          <rect x={30 + k * zw} y={60} width={zw - 2} height={90} fill={heat(z)} opacity={0.85} />
          <text x={30 + k * zw + zw / 2} y={52} textAnchor="middle" className="bd-svg-small">{Math.round(z)} °C</text>
        </g>
      ))}
      <line x1={10} y1={130} x2={350} y2={130} stroke="var(--text-2)" strokeWidth={2} />
      <rect x={bx} y={122} width={30} height={6} fill="var(--mask)" />
      <rect x={bx + 10} y={118} width={8} height={4} fill="var(--text)" />
      <text x={30} y={180} className="bd-svg-small">preheat → soak → reflow → cool</text>
    </Frame>
  );
}

/** Temperature → colour (blue at 25 °C, amber, red at 260 °C) via color-mix. */
export function heat(tc: number): string {
  const f = Math.max(0, Math.min(1, (tc - 25) / 235));
  if (f < 0.5) return `color-mix(in srgb, var(--series-4) ${Math.round(f * 200)}%, var(--series-1))`;
  return `color-mix(in srgb, var(--series-8) ${Math.round((f - 0.5) * 200)}%, var(--series-4))`;
}

export function ProcessAnim({ id, t, params }: { id: string; t: number; params: Record<string, number> }) {
  switch (id) {
    case 'fdm':
      return <FdmAnim t={t} layer={params.layer} width={params.width} nozzle={params.nozzle} />;
    case 'resin':
      return <ResinAnim t={t} />;
    case 'powder':
      return <PowderAnim t={t} />;
    case 'laser':
      return <LaserAnim t={t} kerf={params.kerf} />;
    case 'cnc':
      return <CncAnim t={t} />;
    case 'sheet':
      return <SheetAnim t={t} />;
    case 'molding':
      return <MoldingAnim t={t} />;
    case 'pcb':
      return <PcbAnim t={t} />;
    default:
      return <ReflowMini t={t} />;
  }
}
