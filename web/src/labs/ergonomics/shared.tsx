// Shared pieces of the Human Factors Lab: the project-bound settings hook,
// segmented control, play/pause clock that honours prefers-reduced-motion,
// verdict chips, CSV helpers and the reference list.
import { useEffect, useRef, useState, type ReactNode } from 'react';
import { Chip } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { usePrefersReducedMotion } from '../control/kit';
import type { ErgoConfig } from './types';

/** Settings live in design.extra.ergonomics; missing fields fall back to the engine defaults. */
export function useErgo() {
  const project = useProject((s) => s.project);
  const patchDesign = useProject((s) => s.patchDesign);
  const defaults = useCompute<ErgoConfig>('ergonomics.defaults', {});
  const stored = (project.design.extra?.ergonomics ?? {}) as Partial<ErgoConfig>;
  const cfg: ErgoConfig | null = defaults.data ? { ...defaults.data, ...stored } : null;
  const setCfg = (patch: Partial<ErgoConfig>) => {
    const cur = (useProject.getState().project.design.extra?.ergonomics ?? {}) as Partial<ErgoConfig>;
    patchDesign('extra', { ...useProject.getState().project.design.extra, ergonomics: { ...cur, ...patch } });
  };
  const resetCfg = () => {
    const extra = { ...useProject.getState().project.design.extra };
    delete extra.ergonomics;
    useProject.getState().update((d) => {
      d.design.extra = extra;
    });
  };
  return { project, cfg, setCfg, resetCfg, stored };
}

export function Seg<T extends string | number>(props: { value: T; options: { value: T; label: ReactNode; title?: string }[]; onChange: (v: T) => void; label?: string }) {
  return (
    <div className="hf-seg-wrap">
      {props.label && <span className="field-label small">{props.label}</span>}
      <div className="hf-seg" role="radiogroup" aria-label={props.label}>
        {props.options.map((o) => (
          <button key={String(o.value)} role="radio" aria-checked={props.value === o.value} title={o.title} className={props.value === o.value ? 'on' : ''} onClick={() => props.onChange(o.value)}>
            {o.label}
          </button>
        ))}
      </div>
    </div>
  );
}

/** Playing state that starts paused when the user prefers reduced motion. */
export function usePlaying(initial = true): [boolean, (p: boolean) => void] {
  const reduced = usePrefersReducedMotion();
  const [playing, setPlaying] = useState(initial && !reduced);
  useEffect(() => {
    if (reduced) setPlaying(false);
  }, [reduced]);
  return [playing, setPlaying];
}

/** Seconds of animation time; advances only while playing; `resetKey` changes restart it at 0. */
export function useClock(playing: boolean, resetKey: unknown = 0): number {
  const [t, setT] = useState(0);
  const last = useRef<number | null>(null);
  useEffect(() => {
    setT(0);
  }, [resetKey]);
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

export function PlayButton({ playing, onChange }: { playing: boolean; onChange: (p: boolean) => void }) {
  return (
    <button className="btn small" onClick={() => onChange(!playing)} aria-label={playing ? 'Pause animation' : 'Play animation'} style={{ minWidth: 76, justifyContent: 'center' }}>
      {playing ? '❚❚ Pause' : '▶ Play'}
    </button>
  );
}

/** Pass / fail / check chip with an icon and a word (never colour alone). */
export function OkChip({ ok, yes = 'fits', no = 'fails', warn }: { ok: boolean; yes?: string; no?: string; warn?: boolean }) {
  if (ok) return <Chip tone="pass">✓ {yes}</Chip>;
  if (warn) return <Chip tone="warn">! {no}</Chip>;
  return <Chip tone="fail">✕ {no}</Chip>;
}

export function VerdictChip({ v }: { v: string }) {
  const good = ['comfortable', 'good', 'fits'].includes(v);
  const warnish = ['marginal', 'small', 'threshold', 'large', 'too loose'].includes(v);
  if (good) return <Chip tone="pass">✓ {v}</Chip>;
  if (warnish) return <Chip tone="warn">! {v}</Chip>;
  return <Chip tone="fail">✕ {v}</Chip>;
}

export const f1 = (v: number | undefined | null, d = 1) => (v == null || !Number.isFinite(v) ? '—' : v.toFixed(d));

export function csv(header: string[], rows: (string | number | boolean)[][]) {
  const esc = (v: string | number | boolean) => {
    const s = typeof v === 'number' ? (Number.isFinite(v) ? String(Number(v.toPrecision(8))) : '') : String(v);
    return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  };
  return [header.join(','), ...rows.map((r) => r.map(esc).join(','))].join('\n');
}

export const GROUP_LABEL: Record<string, string> = { men: 'Men', women: 'Women', combined: 'Combined' };

/** Key/value list. */
export function Kv({ rows }: { rows: [ReactNode, ReactNode][] }) {
  return (
    <dl className="hf-kv">
      {rows.map(([k, v], i) => (
        <div key={i} style={{ display: 'contents' }}>
          <dt>{k}</dt>
          <dd>{v}</dd>
        </div>
      ))}
    </dl>
  );
}

export const REFS = {
  ansur: { label: 'C. C. Gordon et al., 2012 Anthropometric Survey of U.S. Army Personnel: Methods and Summary Statistics, NATICK/TR-15/007 (2014) — ANSUR II' },
  dodgson: { label: 'N. A. Dodgson, "Variation and extrema of human interpupillary distance", Proc. SPIE 5291 (2004)' },
  bodyspace: { label: 'S. Pheasant & C. M. Haslegrave, Bodyspace: Anthropometry, Ergonomics and the Design of Work, 3rd ed. (CRC Press, 2006)' },
  iso7250: { label: 'ISO 7250-1, Basic human body measurements for technological design — Part 1: Body measurement definitions and landmarks' },
  sanders: { label: 'M. S. Sanders & E. J. McCormick, Human Factors in Engineering and Design, 7th ed. (McGraw-Hill, 1993)' },
  wickens: { label: 'C. D. Wickens, J. G. Hollands, S. Banbury & R. Parasuraman, Engineering Psychology and Human Performance, 4th ed. (Pearson, 2013)' },
  kress2013: { label: 'B. Kress & T. Starner, "A review of head-mounted displays (HMD) technologies and applications for consumer electronics", Proc. SPIE 8720 (2013)' },
  kress2020: { label: 'B. C. Kress, Optical Architectures for Augmented-, Virtual-, and Mixed-Reality Headsets (SPIE Press, 2020)' },
  landis: { label: 'E. M. Landis, "Micro-injection studies of capillary blood pressure in human skin", Heart 15, 209–228 (1930)' },
  derler: { label: 'S. Derler & L.-C. Gerhardt, "Tribology of skin: review and analysis of experimental results for the friction coefficient of human skin", Tribology Letters 45, 1–27 (2012)' },
  hfes100: { label: 'ANSI/HFES 100-2007, Human Factors Engineering of Computer Workstations' },
  roark: { label: 'W. C. Young & R. G. Budynas, Roark\'s Formulas for Stress and Strain, 7th ed. (McGraw-Hill, 2002)' },
  iso8624: { label: 'ISO 8624, Ophthalmic optics — Spectacle frames — Measuring system and vocabulary (boxing system)' },
  ansurData: { label: 'ANSUR II public data files (as mirrored in AllenDowney/ProbablyOverthinkingIt, data/)', href: 'https://github.com/AllenDowney/ProbablyOverthinkingIt' },
};
