// Animation helpers for the Build Guide: a requestAnimationFrame clock that
// starts paused when the user prefers reduced motion, and a play/pause button.
import { useEffect, useRef, useState } from 'react';
import { usePrefersReducedMotion } from '../control/kit';

/** Seconds since start while `playing`; frozen otherwise. */
export function useTicker(playing: boolean): number {
  const [t, setT] = useState(0);
  const last = useRef<number | null>(null);
  useEffect(() => {
    if (!playing) {
      last.current = null;
      return;
    }
    let raf = 0;
    const tick = (now: number) => {
      if (last.current !== null) {
        const dt = Math.min(0.1, (now - last.current) / 1000);
        setT((v) => v + dt);
      }
      last.current = now;
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [playing]);
  return t;
}

/** Playing state that defaults to paused under prefers-reduced-motion. */
export function usePlaying(): [boolean, (p: boolean) => void] {
  const reduced = usePrefersReducedMotion();
  const [playing, setPlaying] = useState(!reduced);
  useEffect(() => {
    if (reduced) setPlaying(false);
  }, [reduced]);
  return [playing, setPlaying];
}

export function PlayButton({ playing, onChange }: { playing: boolean; onChange: (p: boolean) => void }) {
  return (
    <button className="btn small" onClick={() => onChange(!playing)} aria-label={playing ? 'Pause animation' : 'Play animation'} style={{ minWidth: 72, justifyContent: 'center' }}>
      {playing ? '❚❚ Pause' : '▶ Play'}
    </button>
  );
}

export const clamp = (v: number, a: number, b: number) => Math.max(a, Math.min(b, v));
export const fmt = (v: number, d = 1) => (Number.isFinite(v) ? v.toFixed(d) : '—');

/** Map a value inside [lo, hi] of scale → percentage. */
export const pct = (v: number, s: [number, number]) => (100 * (v - s[0])) / (s[1] - s[0] || 1);
