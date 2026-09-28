// A tiny Gerber RS-274X / Excellon reader + SVG renderer — it draws the
// *generated files*, so what you see is exactly what the fab will read.
import { useMemo } from 'react';

interface Aperture { kind: 'C' | 'R' | 'O'; a: number; b: number }
export interface GerberImage {
  flashes: { x: number; y: number; ap: Aperture }[];
  draws: { x0: number; y0: number; x1: number; y1: number; w: number }[];
  regions: [number, number][][];
  holes: { x: number; y: number; d: number }[];
  attributes: string[];
  bbox: [number, number, number, number];
}

export function parseGerber(src: string): GerberImage {
  const img: GerberImage = { flashes: [], draws: [], regions: [], holes: [], attributes: [], bbox: [Infinity, Infinity, -Infinity, -Infinity] };
  const aps = new Map<number, Aperture>();
  let cur: Aperture | null = null;
  let x = 0, y = 0;
  let region: [number, number][] | null = null;
  const grow = (px: number, py: number, r = 0) => {
    img.bbox = [Math.min(img.bbox[0], px - r), Math.min(img.bbox[1], py - r), Math.max(img.bbox[2], px + r), Math.max(img.bbox[3], py + r)];
  };
  const re = /%([^%]*)%|([^*%]+)\*/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(src))) {
    if (m[1] !== undefined) {
      const body = m[1].replace(/\*$/, '');
      if (body.startsWith('TF.')) img.attributes.push(body.slice(3));
      const ad = /^ADD(\d+)([CRO]),([\d.]+)(?:X([\d.]+))?/.exec(body);
      if (ad) aps.set(Number(ad[1]), { kind: ad[2] as 'C' | 'R' | 'O', a: Number(ad[3]), b: Number(ad[4] ?? ad[3]) });
      continue;
    }
    const w = m[2].trim();
    if (w === 'G36') { region = []; continue; }
    if (w === 'G37') { if (region && region.length > 2) img.regions.push(region); region = null; continue; }
    const sel = /^D(\d+)$/.exec(w);
    if (sel && Number(sel[1]) >= 10) { cur = aps.get(Number(sel[1])) ?? null; continue; }
    const c = /^X(-?\d+)Y(-?\d+)D0([123])$/.exec(w);
    if (!c) continue;
    const nx = Number(c[1]) / 1e6, ny = Number(c[2]) / 1e6, op = c[3];
    if (region) {
      region.push([nx, ny]);
    } else if (op === '1' && cur) {
      img.draws.push({ x0: x, y0: y, x1: nx, y1: ny, w: cur.a });
      grow(nx, ny, cur.a / 2);
      grow(x, y, cur.a / 2);
    } else if (op === '3' && cur) {
      img.flashes.push({ x: nx, y: ny, ap: cur });
      grow(nx, ny, Math.max(cur.a, cur.b) / 2);
    }
    x = nx;
    y = ny;
  }
  for (const r of img.regions) for (const [px, py] of r) grow(px, py);
  return img;
}

export function parseExcellon(src: string): GerberImage {
  const img: GerberImage = { flashes: [], draws: [], regions: [], holes: [], attributes: [], bbox: [Infinity, Infinity, -Infinity, -Infinity] };
  const tools = new Map<number, number>();
  let d = 0;
  for (const line of src.split('\n')) {
    const t = /^T(\d+)C([\d.]+)/.exec(line);
    if (t) { tools.set(Number(t[1]), Number(t[2])); continue; }
    const s = /^T(\d+)$/.exec(line.trim());
    if (s) { d = tools.get(Number(s[1])) ?? 0; continue; }
    const c = /^X(-?[\d.]+)Y(-?[\d.]+)/.exec(line);
    if (c && d > 0) {
      const x = Number(c[1]), y = Number(c[2]);
      img.holes.push({ x, y, d });
      img.bbox = [Math.min(img.bbox[0], x - d), Math.min(img.bbox[1], y - d), Math.max(img.bbox[2], x + d), Math.max(img.bbox[3], y + d)];
    }
  }
  const fn = /TF\.FileFunction,([^\n]*)/.exec(src);
  if (fn) img.attributes.push('FileFunction,' + fn[1]);
  return img;
}

export function GerberSvg(props: { src: string; kind: string; width: number; height: number; color: string; board: [number, number]; negative?: boolean }) {
  const img = useMemo(() => (props.kind.startsWith('Drill') ? parseExcellon(props.src) : parseGerber(props.src)), [props.src, props.kind]);
  const [W, H] = props.board;
  const Y = (y: number) => H - y;
  const pad = 1;
  return (
    <svg viewBox={`${-pad} ${-pad} ${W + 2 * pad} ${H + 2 * pad}`} width="100%" height={props.height} style={{ display: 'block', background: 'var(--surface-2)', borderRadius: 10, border: '1px solid var(--border)' }} role="img" aria-label={`${props.kind} layer`}>
      <rect x={0} y={0} width={W} height={H} fill={props.negative ? props.color : 'none'} opacity={props.negative ? 0.35 : 1} stroke="var(--border-strong)" strokeWidth={0.06} strokeDasharray="0.4 0.3" />
      <g fill={props.negative ? 'var(--surface-2)' : props.color} stroke="none">
        {img.regions.map((r, i) => <polygon key={`r${i}`} points={r.map(([x, y]) => `${x},${Y(y)}`).join(' ')} />)}
        {img.flashes.map((f, i) =>
          f.ap.kind === 'C'
            ? <circle key={`f${i}`} cx={f.x} cy={Y(f.y)} r={f.ap.a / 2} />
            : <rect key={`f${i}`} x={f.x - f.ap.a / 2} y={Y(f.y) - f.ap.b / 2} width={f.ap.a} height={f.ap.b} />,
        )}
      </g>
      <g stroke={props.negative ? 'var(--surface-2)' : props.color} strokeLinecap="round" fill="none">
        {img.draws.map((d, i) => <line key={`d${i}`} x1={d.x0} y1={Y(d.y0)} x2={d.x1} y2={Y(d.y1)} strokeWidth={d.w} />)}
      </g>
      {img.holes.map((h, i) => <circle key={`h${i}`} cx={h.x} cy={Y(h.y)} r={h.d / 2} fill={props.color} />)}
    </svg>
  );
}

export function gerberStats(src: string, kind: string) {
  const img = kind.startsWith('Drill') ? parseExcellon(src) : parseGerber(src);
  return { flashes: img.flashes.length, draws: img.draws.length, regions: img.regions.length, holes: img.holes.length, attributes: img.attributes };
}
