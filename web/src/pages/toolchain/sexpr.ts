// Minimal S-expression reader for KiCad files, used to re-read the board
// Protoforge exports and draw it — proof that the file is complete.
export type Sx = string | Sx[];

export function parseSexpr(src: string): Sx {
  const stack: Sx[][] = [[]];
  let i = 0;
  const n = src.length;
  while (i < n) {
    const c = src[i];
    if (c === '(') {
      stack.push([]);
      i++;
    } else if (c === ')') {
      const top = stack.pop();
      if (!top || stack.length === 0) throw new Error(`unbalanced ')' at ${i}`);
      stack[stack.length - 1].push(top);
      i++;
    } else if (c === '"') {
      let s = '';
      i++;
      while (i < n && src[i] !== '"') {
        if (src[i] === '\\' && i + 1 < n) {
          s += src[i + 1] === 'n' ? '\n' : src[i + 1];
          i += 2;
        } else s += src[i++];
      }
      if (i >= n) throw new Error('unterminated string');
      i++;
      stack[stack.length - 1].push(s);
    } else if (c === ' ' || c === '\n' || c === '\t' || c === '\r') {
      i++;
    } else {
      const st = i;
      while (i < n && !' \n\t\r()'.includes(src[i])) i++;
      stack[stack.length - 1].push(src.slice(st, i));
    }
  }
  if (stack.length !== 1) throw new Error(`${stack.length - 1} unclosed '('`);
  if (stack[0].length !== 1) throw new Error('expected one top-level expression');
  return stack[0][0];
}

export const head = (x: Sx): string | undefined => (Array.isArray(x) && typeof x[0] === 'string' ? x[0] : undefined);
export const kids = (x: Sx, name: string): Sx[][] => (Array.isArray(x) ? (x.filter((k) => head(k) === name) as Sx[][]) : []);
export const kid = (x: Sx, name: string): Sx[] | undefined => kids(x, name)[0];
export const num = (x: Sx | undefined, i: number): number => (Array.isArray(x) && typeof x[i] === 'string' ? Number(x[i]) : NaN);
export const str = (x: Sx | undefined, i: number): string => (Array.isArray(x) && typeof x[i] === 'string' ? (x[i] as string) : '');

export interface KPad { x: number; y: number; w: number; h: number; angle: number; shape: string; drill: number; layer: 'F' | 'B' | 'TH'; net: string; num: string }
export interface KFootprint { ref: string; value: string; x: number; y: number; angle: number; bottom: boolean; pads: KPad[]; crtyd: [number, number, number, number] | null }
export interface KBoard {
  version: string; generator: string; outline: [number, number, number, number] | null;
  footprints: KFootprint[];
  segments: { x0: number; y0: number; x1: number; y1: number; w: number; layer: string; net: number }[];
  vias: { x: number; y: number; size: number; drill: number; net: number }[];
  zones: { net: string; layer: string; keepout: boolean; pts: [number, number][]; name: string }[];
  nets: Map<number, string>;
}

/** Read a .kicad_pcb into drawable geometry (absolute KiCad coordinates, y down). */
export function readBoard(src: string): KBoard {
  const t = parseSexpr(src);
  if (head(t) !== 'kicad_pcb') throw new Error('not a kicad_pcb file');
  const nets = new Map<number, string>();
  for (const n of kids(t, 'net')) nets.set(num(n, 1), str(n, 2));
  const rect = kids(t, 'gr_rect').find((g) => str(kid(g, 'layer'), 1) === 'Edge.Cuts');
  const outline: KBoard['outline'] = rect ? [num(kid(rect, 'start'), 1), num(kid(rect, 'start'), 2), num(kid(rect, 'end'), 1), num(kid(rect, 'end'), 2)] : null;
  const footprints: KFootprint[] = kids(t, 'footprint').map((f) => {
    const at = kid(f, 'at');
    const fx = num(at, 1);
    const fy = num(at, 2);
    const ang = num(at, 3) || 0;
    const r = (ang * Math.PI) / 180;
    const bottom = str(kid(f, 'layer'), 1) === 'B.Cu';
    const ref = str(kids(f, 'fp_text').find((x) => str(x, 1) === 'reference'), 2);
    const value = str(kids(f, 'fp_text').find((x) => str(x, 1) === 'value'), 2);
    const pads: KPad[] = kids(f, 'pad').map((p) => {
      const pa = kid(p, 'at');
      const lx = num(pa, 1);
      const ly = num(pa, 2);
      // KiCad: absolute = anchor + R(angle) · local (angle counter-clockwise on screen, y down)
      const x = fx + lx * Math.cos(r) + ly * Math.sin(r);
      const y = fy - lx * Math.sin(r) + ly * Math.cos(r);
      const layers = (kid(p, 'layers') ?? []).slice(1) as string[];
      const layer: KPad['layer'] = layers.includes('*.Cu') ? 'TH' : layers.includes('B.Cu') ? 'B' : 'F';
      return {
        x, y, w: num(kid(p, 'size'), 1), h: num(kid(p, 'size'), 2), angle: num(pa, 3) || 0, shape: str(p, 3),
        drill: num(kid(p, 'drill'), 1) || 0, layer, net: str(kid(p, 'net'), 2), num: str(p, 1),
      };
    });
    const cy = kids(f, 'fp_rect').find((g) => str(kid(g, 'layer'), 1).endsWith('CrtYd'));
    let crtyd: KFootprint['crtyd'] = null;
    if (cy) {
      const pts = [[num(kid(cy, 'start'), 1), num(kid(cy, 'start'), 2)], [num(kid(cy, 'end'), 1), num(kid(cy, 'start'), 2)], [num(kid(cy, 'end'), 1), num(kid(cy, 'end'), 2)], [num(kid(cy, 'start'), 1), num(kid(cy, 'end'), 2)]].map(([lx, ly]) => [fx + lx * Math.cos(r) + ly * Math.sin(r), fy - lx * Math.sin(r) + ly * Math.cos(r)]);
      const xs = pts.map((p) => p[0]);
      const ys = pts.map((p) => p[1]);
      crtyd = [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)];
    }
    return { ref, value, x: fx, y: fy, angle: ang, bottom, pads, crtyd };
  });
  const segments = kids(t, 'segment').map((s) => ({
    x0: num(kid(s, 'start'), 1), y0: num(kid(s, 'start'), 2), x1: num(kid(s, 'end'), 1), y1: num(kid(s, 'end'), 2),
    w: num(kid(s, 'width'), 1), layer: str(kid(s, 'layer'), 1), net: num(kid(s, 'net'), 1),
  }));
  const vias = kids(t, 'via').map((v) => ({ x: num(kid(v, 'at'), 1), y: num(kid(v, 'at'), 2), size: num(kid(v, 'size'), 1), drill: num(kid(v, 'drill'), 1), net: num(kid(v, 'net'), 1) }));
  const zones = kids(t, 'zone').map((z) => {
    const pts = (kid(kid(z, 'polygon') ?? [], 'pts') ?? []).slice(1).map((p) => [num(p as Sx[], 1), num(p as Sx[], 2)] as [number, number]);
    return { net: str(kid(z, 'net_name'), 1), layer: str(kid(z, 'layer'), 1), keepout: !!kid(z, 'keepout'), pts, name: str(kid(z, 'name'), 1) };
  });
  return { version: str(kid(t, 'version'), 1), generator: str(kid(t, 'generator'), 1), outline, footprints, segments, vias, zones, nets };
}
