// Canvas animation of Lee's wavefront expansion for one net.
import { useEffect, useMemo, useRef, useState } from 'react';
import type { Layout, WaveTrace } from './types';
import { useElementWidth } from '../../components/useElementWidth';
import { prefersReducedMotion } from './zip';

function cssVar(name: string, fallback: string) {
  const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return v || fallback;
}

function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace('#', '');
  const n = parseInt(h.length === 3 ? h.split('').map((c) => c + c).join('') : h, 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

export function Wavefront(props: { layout: Layout; trace: WaveTrace; waveIndex: number; playing: boolean; speed: number; onDone?: () => void }) {
  const { layout: L, trace } = props;
  const [wrapRef, width] = useElementWidth<HTMLDivElement>();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [shown, setShown] = useState(0);
  const wave = trace.waves[props.waveIndex];
  const total = wave ? wave.expanded.length / 4 : 0;
  const maxCost = useMemo(() => {
    if (!wave) return 1;
    let m = 1;
    for (let i = 3; i < wave.expanded.length; i += 4) m = Math.max(m, wave.expanded[i]);
    return m;
  }, [wave]);

  useEffect(() => {
    setShown(prefersReducedMotion() ? total : 0);
  }, [wave, total]);

  useEffect(() => {
    if (!props.playing || prefersReducedMotion()) return;
    let raf = 0;
    const perFrame = Math.max(20, Math.round((total / 240) * props.speed));
    const tick = () => {
      setShown((s) => {
        const n = Math.min(total, s + perFrame);
        if (n >= total) props.onDone?.();
        return n;
      });
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.playing, total, props.speed]);

  const W = L.board.width, H = L.board.height;
  const scale = width > 0 ? width / (W + 2) : 10;
  const cw = Math.max(1, Math.round(width));
  const ch = Math.round((H + 2) * scale);

  useEffect(() => {
    const cv = canvasRef.current;
    if (!cv || width <= 0) return;
    const dpr = window.devicePixelRatio || 1;
    cv.width = cw * dpr;
    cv.height = ch * dpr;
    const ctx = cv.getContext('2d')!;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    const X = (x: number) => (x + 1) * scale;
    const Y = (y: number) => (H - y + 1) * scale;
    ctx.fillStyle = cssVar('--surface-2', '#1d1d20');
    ctx.fillRect(0, 0, cw, ch);
    ctx.fillStyle = cssVar('--mask', '#1f6f3f');
    ctx.fillRect(X(0), Y(H), W * scale, H * scale);
    // pads as obstacles
    ctx.fillStyle = cssVar('--copper', '#d9a55a');
    ctx.globalAlpha = 0.55;
    for (const p of L.pads) {
      if (!p.top) continue;
      ctx.fillRect(X(p.x - p.w / 2), Y(p.y + p.h / 2), p.w * scale, p.h * scale);
    }
    ctx.globalAlpha = 1;
    if (!wave) return;
    const g = trace.grid_mm;
    const cell = Math.max(1, g * scale);
    const [r1, g1, b1] = hexToRgb(cssVar('--series-1', '#3987e5'));
    const [r2, g2, b2] = hexToRgb(cssVar('--series-7', '#9085e9'));
    for (let k = 0; k < shown; k++) {
      const layer = wave.expanded[4 * k], i = wave.expanded[4 * k + 1], j = wave.expanded[4 * k + 2], c = wave.expanded[4 * k + 3];
      const f = c / maxCost; // sequential: light near the source, dark far away
      const a = 0.85 - 0.6 * f;
      ctx.fillStyle = layer === 0 ? `rgba(${r1},${g1},${b1},${a})` : `rgba(${r2},${g2},${b2},${a * 0.8})`;
      ctx.fillRect(X(i * g) - cell / 2, Y(j * g) - cell / 2, cell, cell);
    }
    // frontier highlight
    if (shown < total) {
      ctx.fillStyle = cssVar('--text', '#fff');
      for (let k = Math.max(0, shown - 60); k < shown; k++) {
        const i = wave.expanded[4 * k + 1], j = wave.expanded[4 * k + 2];
        ctx.fillRect(X(i * g) - cell / 2, Y(j * g) - cell / 2, cell, cell);
      }
    }
    if (shown >= total && wave.found) {
      ctx.strokeStyle = cssVar('--copper', '#d9a55a');
      ctx.lineWidth = Math.max(2, 0.15 * scale * 2);
      ctx.lineCap = 'round';
      ctx.beginPath();
      for (let k = 0; k < wave.path.length / 3; k++) {
        const i = wave.path[3 * k + 1], j = wave.path[3 * k + 2];
        if (k === 0) ctx.moveTo(X(i * g), Y(j * g));
        else ctx.lineTo(X(i * g), Y(j * g));
      }
      ctx.stroke();
    }
  }, [shown, wave, width, cw, ch, scale, L, H, W, trace.grid_mm, maxCost, total]);

  return (
    <div ref={wrapRef}>
      <canvas ref={canvasRef} style={{ width: cw, height: ch, display: 'block', borderRadius: 12, border: '1px solid var(--border)' }} aria-label={`Lee wavefront for ${trace.net}`} />
      <div className="row small muted" style={{ marginTop: 6, justifyContent: 'space-between' }}>
        <span>
          <span style={{ display: 'inline-block', width: 10, height: 10, background: 'var(--series-1)', borderRadius: 2, marginRight: 4 }} />top-layer front
          <span style={{ display: 'inline-block', width: 10, height: 10, background: 'var(--series-7)', borderRadius: 2, margin: '0 4px 0 12px' }} />bottom-layer front (after a via)
          <span style={{ display: 'inline-block', width: 14, height: 3, background: 'var(--copper)', borderRadius: 2, margin: '0 4px 2px 12px' }} />back-traced path
        </span>
        <span className="mono">{Math.min(shown, total).toLocaleString()} / {total.toLocaleString()} cells{wave && !wave.found ? ' · no path' : ''}</span>
      </div>
    </div>
  );
}
