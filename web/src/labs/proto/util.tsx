// Small helpers shared by the Prototype & Firmware Lab tabs.
import { useState, type ReactNode } from 'react';
import type { HudOut, HudState } from './types';

export function Seg<T extends string | number>(props: { value: T; options: { value: T; label: ReactNode }[]; onChange: (v: T) => void; ariaLabel?: string }) {
  return (
    <div className="pf-seg" role="group" aria-label={props.ariaLabel}>
      {props.options.map((o) => (
        <button key={String(o.value)} className={`pf-seg-btn ${o.value === props.value ? 'active' : ''}`} aria-pressed={o.value === props.value} onClick={() => props.onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}

/** useState persisted to localStorage (per-viewer convenience only). Re-reads when `key` changes. */
export function usePersistent<T>(key: string, initial: T): [T, (v: T) => void] {
  const read = (k: string): T => {
    try {
      const s = localStorage.getItem(k);
      return s ? (JSON.parse(s) as T) : initial;
    } catch {
      return initial;
    }
  };
  const [state, setState] = useState<{ key: string; v: T }>(() => ({ key, v: read(key) }));
  // a different key (e.g. another breadboard build) has its own saved value
  const current = state.key === key ? state.v : read(key);
  if (state.key !== key) setState({ key, v: current });
  const set = (nv: T) => {
    setState({ key, v: nv });
    try {
      localStorage.setItem(key, JSON.stringify(nv));
    } catch {
      /* storage unavailable */
    }
  };
  return [current, set];
}

export function wireColor(c: string): string {
  const known = ['red', 'black', 'yellow', 'blue', 'green', 'purple', 'orange', 'white'];
  return known.includes(c) ? `var(--pf-wire-${c})` : 'var(--accent)';
}

export function decodeB64(s: string): Uint8Array {
  const bin = atob(s);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

/** Grey level 0..15 of pixel (x, y) in an engine framebuffer (same layout as iris_gfx.c). */
export function fbLevel(bytes: Uint8Array, hud: HudOut, x: number, y: number): number {
  if (hud.bpp === 1) return (bytes[y * hud.stride + (x >> 3)] >> (x & 7)) & 1 ? 15 : 0;
  const b = bytes[y * hud.stride + (x >> 1)];
  return x & 1 ? b & 15 : b >> 4;
}

/** Paint a framebuffer into an ImageData (colour = lit pixel colour, alpha from grey level). */
export function fbToImage(hud: HudOut, rgb: [number, number, number], bg: [number, number, number, number] = [0, 0, 0, 255]): ImageData {
  const bytes = decodeB64(hud.data_b64);
  const img = new ImageData(hud.width, hud.height);
  for (let y = 0; y < hud.height; y++) {
    for (let x = 0; x < hud.width; x++) {
      const l = fbLevel(bytes, hud, x, y) / 15;
      const i = (y * hud.width + x) * 4;
      img.data[i] = Math.round(bg[0] * (1 - l) + rgb[0] * l);
      img.data[i + 1] = Math.round(bg[1] * (1 - l) + rgb[1] * l);
      img.data[i + 2] = Math.round(bg[2] * (1 - l) + rgb[2] * l);
      img.data[i + 3] = Math.max(bg[3], Math.round(255 * l));
    }
  }
  return img;
}

/** Framebuffer → PNG data URL (for SVG <image>). */
export function fbToDataUrl(hud: HudOut, rgb: [number, number, number]): string {
  if (typeof document === 'undefined') return '';
  const c = document.createElement('canvas');
  c.width = hud.width;
  c.height = hud.height;
  const ctx = c.getContext('2d');
  if (!ctx) return '';
  ctx.putImageData(fbToImage(hud, rgb), 0, 0);
  return c.toDataURL('image/png');
}

export const DEFAULT_HUD: HudState = {
  screen: 'navigation',
  hour: 9,
  minute: 41,
  battery_pct: 82,
  ble_connected: true,
  nav_arrow_deg: 45,
  nav_distance_m: 350,
  nav_street: 'Gaffey St',
  nav_eta_min: 4,
  captions: 'Welcome to the systems engineering seminar. Today we look at how a product goes from an idea to a factory.',
  caption_scroll_px: 0,
  notif_app: 'MESSAGES',
  notif_title: 'Lab meeting moved',
  notif_body: 'The prototype review is now at 3 pm in the fabrication lab. Bring the Iris breadboard.',
  prompter: 'Good afternoon. Iris is a monocular heads-up display that puts navigation, captions and notifications in the corner of your eye, for a full day on one charge. Everything you see here was designed, simulated and generated in the browser.',
  prompter_scroll_px: 0,
  prompter_progress: 0,
};

export const SCREENS: { value: HudState['screen']; label: string }[] = [
  { value: 'clock', label: 'Clock' },
  { value: 'navigation', label: 'Navigation' },
  { value: 'captions', label: 'Captions' },
  { value: 'notification', label: 'Notification' },
  { value: 'teleprompter', label: 'Teleprompter' },
];

/** Length of an orthogonal polyline. */
export function polyLen(pts: [number, number][]): number {
  let s = 0;
  for (let i = 1; i < pts.length; i++) s += Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]);
  return s;
}

/** Play/pause/replay control row. */
export function PlayControls(props: { playing: boolean; onToggle: () => void; onReplay?: () => void; reduced: boolean }) {
  return (
    <div className="row">
      <button className="btn small" onClick={props.onToggle} aria-label={props.playing ? 'Pause animation' : 'Play animation'}>
        {props.playing ? '❚❚ Pause' : '▶ Play'}
      </button>
      {props.onReplay && <button className="btn small" onClick={props.onReplay}>↺ Replay</button>}
      {props.reduced && <span className="muted small">Reduced motion: animations start paused</span>}
    </div>
  );
}

export const fmtKB = (b: number) => `${(b / 1024).toFixed(b < 10240 ? 2 : 1)} KB`;
