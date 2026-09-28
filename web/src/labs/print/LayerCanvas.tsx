// 2D preview of one sliced layer: beads drawn at their real width, travel
// dashed, previous layer ghosted; wheel to zoom, drag to pan, and a "trace"
// animation that replays the print order of the layer.
import { useCallback, useEffect, useRef, useState } from 'react';
import { KINDS, type LayerOut } from './api';
import { cssVar, prefersReducedMotion, useTheme } from '../cad/viewer';
import { useElementWidth } from '../../components/useElementWidth';

interface View {
  s: number;
  tx: number;
  ty: number;
}

export function LayerCanvas(props: {
  layer: LayerOut | null;
  prev: LayerOut | null;
  bbox: [[number, number, number], [number, number, number]];
  lineWidth: number;
  showTravel: boolean;
  /** 0..1 fraction of the layer's paths to draw (trace animation). */
  progress: number;
  height?: number;
}) {
  const ref = useRef<HTMLCanvasElement>(null);
  const [wrap, width] = useElementWidth<HTMLDivElement>();
  const theme = useTheme();
  // Height follows the part's aspect ratio so thin parts are not letterboxed.
  const [[bx0, by0], [bx1, by1]] = props.bbox;
  const aspectH = width > 0 ? (width - 48) * ((by1 - by0 + 8) / Math.max(bx1 - bx0 + 8, 1)) + 48 : 300;
  const height = Math.round(Math.min(props.height ?? 420, Math.max(220, aspectH)));
  const [view, setView] = useState<View | null>(null);
  const drag = useRef<{ x: number; y: number; v: View } | null>(null);

  const fit = useCallback((): View => {
    const [[x0, y0], [x1, y1]] = props.bbox;
    const w = Math.max(width, 100);
    const pad = 24;
    const s = Math.min((w - 2 * pad) / Math.max(x1 - x0 + 8, 1), (height - 2 * pad) / Math.max(y1 - y0 + 8, 1));
    return { s, tx: w / 2 - ((x0 + x1) / 2) * s, ty: height / 2 + ((y0 + y1) / 2) * s };
  }, [props.bbox, width, height]);

  useEffect(() => setView(null), [props.bbox, width]);
  const viewRef = useRef<View | null>(null);
  viewRef.current = view ?? (width > 0 ? fit() : null);
  // Non-passive wheel listener so zooming does not scroll the page.
  useEffect(() => {
    const c = ref.current;
    if (!c) return;
    const onWheel = (e: WheelEvent) => {
      const v = viewRef.current;
      if (!v) return;
      e.preventDefault();
      const r = c.getBoundingClientRect();
      const mx = e.clientX - r.left;
      const my = e.clientY - r.top;
      const k = Math.exp(-e.deltaY * 0.0015);
      setView({ s: v.s * k, tx: mx - (mx - v.tx) * k, ty: my - (my - v.ty) * k });
    };
    c.addEventListener('wheel', onWheel, { passive: false });
    return () => c.removeEventListener('wheel', onWheel);
  }, []);

  useEffect(() => {
    const c = ref.current;
    if (!c || width <= 0) return;
    const dpr = window.devicePixelRatio || 1;
    c.width = Math.round(width * dpr);
    c.height = Math.round(height * dpr);
    const ctx = c.getContext('2d');
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, width, height);
    const v = view ?? fit();
    const X = (x: number) => (x / 100) * v.s + v.tx;
    const Y = (y: number) => -(y / 100) * v.s + v.ty;
    // mm grid
    const grid = cssVar('--grid', '#333');
    ctx.strokeStyle = grid;
    ctx.lineWidth = 1;
    const step = v.s > 12 ? 1 : v.s > 3 ? 5 : 10;
    const x0 = Math.floor(-v.tx / v.s / step) * step;
    const x1 = Math.ceil((width - v.tx) / v.s / step) * step;
    const y0 = Math.floor((v.ty - height) / v.s / step) * step;
    const y1 = Math.ceil(v.ty / v.s / step) * step;
    ctx.globalAlpha = 0.5;
    ctx.beginPath();
    for (let x = x0; x <= x1; x += step) {
      ctx.moveTo(x * v.s + v.tx, 0);
      ctx.lineTo(x * v.s + v.tx, height);
    }
    for (let y = y0; y <= y1; y += step) {
      ctx.moveTo(0, -y * v.s + v.ty);
      ctx.lineTo(width, -y * v.s + v.ty);
    }
    ctx.stroke();
    ctx.globalAlpha = 1;
    const bead = Math.max(1, props.lineWidth * v.s);
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    // previous layer ghost
    if (props.prev) {
      ctx.strokeStyle = cssVar('--muted');
      ctx.globalAlpha = 0.18;
      ctx.lineWidth = bead;
      for (const p of props.prev.paths) {
        if (p.k === 0) continue;
        ctx.beginPath();
        for (let i = 0; i < p.p.length; i += 2) (i ? ctx.lineTo : ctx.moveTo).call(ctx, X(p.p[i]), Y(p.p[i + 1]));
        ctx.stroke();
      }
      ctx.globalAlpha = 1;
    }
    if (!props.layer) return;
    const colors = KINDS.map((k) => cssVar(k.color));
    const paths = props.layer.paths;
    const upto = Math.round(paths.length * Math.max(0, Math.min(1, props.progress)));
    for (let j = 0; j < upto; j++) {
      const p = paths[j];
      if (p.k === 0) {
        if (!props.showTravel) continue;
        ctx.setLineDash([4, 4]);
        ctx.lineWidth = 1;
        ctx.globalAlpha = 0.7;
        ctx.strokeStyle = p.r ? cssVar('--bad-text') : colors[0];
      } else {
        ctx.setLineDash([]);
        ctx.lineWidth = bead;
        ctx.globalAlpha = 0.92;
        ctx.strokeStyle = colors[p.k] ?? colors[2];
      }
      ctx.beginPath();
      for (let i = 0; i < p.p.length; i += 2) (i ? ctx.lineTo : ctx.moveTo).call(ctx, X(p.p[i]), Y(p.p[i + 1]));
      ctx.stroke();
    }
    ctx.setLineDash([]);
    ctx.globalAlpha = 1;
    // nozzle marker at the last drawn point
    if (upto > 0 && upto < paths.length) {
      const last = paths[upto - 1].p;
      const nx = X(last[last.length - 2]);
      const ny = Y(last[last.length - 1]);
      ctx.fillStyle = cssVar('--text');
      ctx.beginPath();
      ctx.arc(nx, ny, 4, 0, Math.PI * 2);
      ctx.fill();
    }
    // scale bar
    const bar = step * 2;
    ctx.fillStyle = cssVar('--text-2');
    ctx.fillRect(12, height - 18, bar * v.s, 2);
    ctx.font = '11px ' + cssVar('--mono', 'monospace');
    ctx.fillText(`${bar} mm`, 12, height - 24);
  }, [props.layer, props.prev, props.progress, props.showTravel, props.lineWidth, view, fit, width, height, theme]);

  return (
    <div ref={wrap} style={{ position: 'relative' }}>
      <canvas
        ref={ref}
        className="cp-canvas2d"
        style={{ height }}
        onPointerDown={(e) => {
          (e.target as HTMLCanvasElement).setPointerCapture(e.pointerId);
          drag.current = { x: e.clientX, y: e.clientY, v: view ?? fit() };
        }}
        onPointerMove={(e) => {
          const d = drag.current;
          if (!d) return;
          setView({ s: d.v.s, tx: d.v.tx + e.clientX - d.x, ty: d.v.ty + e.clientY - d.y });
        }}
        onPointerUp={() => (drag.current = null)}
        onDoubleClick={() => setView(null)}
        role="img"
        aria-label={`Layer ${props.layer?.i ?? ''} toolpaths`}
      />
      <div className="cp-badge">wheel: zoom · drag: pan · double-click: fit</div>
    </div>
  );
}

/** Drives the 0..1 trace progress with requestAnimationFrame. */
export function useTrace(seconds: number): [number, boolean, () => void, () => void] {
  const [p, setP] = useState(1);
  const [on, setOn] = useState(false);
  useEffect(() => {
    if (!on) return;
    if (prefersReducedMotion()) {
      setP(1);
      setOn(false);
      return;
    }
    let raf = 0;
    const t0 = performance.now();
    const tick = (now: number) => {
      const x = (now - t0) / 1000 / seconds;
      if (x >= 1) {
        setP(1);
        setOn(false);
        return;
      }
      setP(x);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [on, seconds]);
  return [p, on, () => { setP(0); setOn(true); }, () => { setOn(false); setP(1); }];
}
