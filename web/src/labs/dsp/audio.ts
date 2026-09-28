// Tiny Web Audio helper: play a mono buffer (looped) and stop it again.
import { useEffect, useState } from 'react';

let ctx: AudioContext | null = null;
let current: AudioBufferSourceNode | null = null;
let currentId: string | null = null;
const listeners = new Set<(id: string | null) => void>();

function emit() {
  listeners.forEach((l) => l(currentId));
}

export function stopAudio() {
  if (current) {
    try {
      current.stop();
    } catch {
      /* already stopped */
    }
    current.disconnect();
  }
  current = null;
  currentId = null;
  emit();
}

/** Play samples at `fs` (normalised to −2 dBFS peak). Returns false if Web Audio is unavailable. */
export function playSamples(id: string, x: ArrayLike<number>, fs: number, loop = true): boolean {
  stopAudio();
  const AC = window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
  if (!AC || x.length === 0) return false;
  ctx ??= new AC();
  if (ctx.state === 'suspended') void ctx.resume();
  let peak = 1e-9;
  for (let i = 0; i < x.length; i++) peak = Math.max(peak, Math.abs(x[i]));
  const g = 0.8 / peak;
  const rate = Math.min(Math.max(fs, 3000), 384000);
  const buf = ctx.createBuffer(1, x.length, rate);
  const ch = buf.getChannelData(0);
  // short fade in/out to avoid clicks at the loop point
  const fade = Math.min(64, Math.floor(x.length / 4));
  for (let i = 0; i < x.length; i++) {
    let w = 1;
    if (i < fade) w = i / fade;
    else if (i > x.length - fade) w = (x.length - i) / fade;
    ch[i] = x[i] * g * w;
  }
  const src = ctx.createBufferSource();
  src.buffer = buf;
  src.loop = loop;
  src.connect(ctx.destination);
  src.onended = () => {
    if (current === src) {
      current = null;
      currentId = null;
      emit();
    }
  };
  src.start();
  current = src;
  currentId = id;
  emit();
  return true;
}

/** Which clip is playing (id) — re-renders when that changes; stops audio on unmount. */
export function usePlaying(): string | null {
  const [id, setId] = useState<string | null>(currentId);
  useEffect(() => {
    listeners.add(setId);
    return () => {
      listeners.delete(setId);
    };
  }, []);
  return id;
}

export function useStopOnUnmount() {
  useEffect(() => () => stopAudio(), []);
}
