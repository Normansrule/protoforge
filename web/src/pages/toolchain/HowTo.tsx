// Animated "how to" strip: the few physical actions it takes to hand a
// Protoforge file to a tool (download → open → drop → run …), each drawn as
// a small SVG pictogram that animates while its step is active. One timer per
// strip, running only while the strip is on screen and not paused; with
// prefers-reduced-motion every step is shown statically.
import { useEffect, useRef, useState } from 'react';
import { useNearViewport, useReducedMotion } from '../home/motion';

export type Action = 'download' | 'open' | 'drop' | 'paste' | 'click' | 'run' | 'view' | 'edit' | 'export' | 'commit' | 'link' | 'import' | 'unzip';

export interface HowStep { a: Action; t: string }

const STEP_MS = 1900;

/** 64 × 44 pictogram per action; `.ht-anim` parts move while the step is active. */
function Picto({ a }: { a: Action }) {
  const win = (
    <g>
      <rect x={6} y={6} width={52} height={34} rx={4} className="ht-win" />
      <line x1={6} y1={13} x2={58} y2={13} className="ht-line" />
      <circle cx={10.5} cy={9.5} r={1.4} className="ht-dot" />
      <circle cx={15} cy={9.5} r={1.4} className="ht-dot" />
    </g>
  );
  const file = (x: number, y: number, cls = '') => (
    <g transform={`translate(${x} ${y})`} className={cls}>
      <path d="M0 0 H9 L13 4 V17 H0 Z" className="ht-file" />
      <path d="M9 0 V4 H13" className="ht-line" fill="none" />
    </g>
  );
  switch (a) {
    case 'download':
      return (
        <g>
          <path d="M16 30 V38 H48 V30" className="ht-stroke" fill="none" />
          <g className="ht-anim ht-down"><line x1={32} y1={6} x2={32} y2={28} className="ht-accent" /><path d="M24 21 L32 29 L40 21" className="ht-accent" fill="none" /></g>
        </g>
      );
    case 'unzip':
      return (
        <g>
          <rect x={20} y={8} width={24} height={30} rx={3} className="ht-file" />
          {[12, 17, 22, 27].map((y) => <line key={y} x1={30} y1={y} x2={34} y2={y} className="ht-stroke" />)}
          <g className="ht-anim ht-fanL">{file(4, 18)}</g>
          <g className="ht-anim ht-fanR">{file(47, 18)}</g>
        </g>
      );
    case 'open':
      return (
        <g>
          <g className="ht-anim ht-pop">{win}</g>
          <path d="M40 22 L50 22 L50 32 M50 22 L38 34" className="ht-accent" fill="none" />
        </g>
      );
    case 'drop':
      return (
        <g>
          {win}
          <rect x={16} y={18} width={32} height={18} rx={3} className="ht-target" />
          {file(26, 0, 'ht-anim ht-drop')}
        </g>
      );
    case 'import':
      return (
        <g>
          {win}
          <path d="M14 26 H36" className="ht-accent" />
          <path d="M30 20 L37 26 L30 32" className="ht-accent" fill="none" />
          <g className="ht-anim ht-pulse"><path d="M42 18 L50 22 L50 32 L42 36 L34 32 L34 22 Z" className="ht-solid" /></g>
        </g>
      );
    case 'paste':
      return (
        <g>
          {win}
          {[19, 25, 31].map((y, i) => <line key={y} x1={12} y1={y} x2={12 + [34, 26, 30][i]} y2={y} className={`ht-accent ht-anim ht-type ht-d${i}`} />)}
        </g>
      );
    case 'click':
      return (
        <g>
          <rect x={10} y={14} width={34} height={14} rx={7} className="ht-btn" />
          <circle cx={27} cy={21} r={4} className="ht-anim ht-ripple" />
          <path d="M34 24 L34 40 L38 36 L41 43 L44 41.5 L41 35 L46 35 Z" className="ht-cursor ht-anim ht-tap" />
        </g>
      );
    case 'run':
      return (
        <g>
          <path d="M8 10 L18 16 L8 22 Z" className="ht-solid" />
          <path d="M22 30 L28 30 L31 16 L35 38 L39 22 L42 30 L58 30" className="ht-accent ht-anim ht-trace" fill="none" />
          <line x1={22} y1={40} x2={58} y2={40} className="ht-line" />
        </g>
      );
    case 'view':
      return (
        <g className="ht-anim ht-spin" style={{ transformOrigin: '32px 23px' }}>
          <path d="M32 8 L48 16 L48 32 L32 40 L16 32 L16 16 Z" className="ht-win" />
          <path d="M16 16 L32 24 L48 16 M32 24 L32 40" className="ht-stroke" fill="none" />
        </g>
      );
    case 'edit':
      return (
        <g>
          {win}
          <line x1={12} y1={22} x2={42} y2={22} className="ht-line" />
          <text x={12} y={33} className="ht-code">x = 20;</text>
          <g className="ht-anim ht-pencil"><path d="M44 34 L52 20 L56 22 L48 36 L43 38 Z" className="ht-solid" /></g>
        </g>
      );
    case 'export':
      return (
        <g>
          <rect x={8} y={12} width={26} height={24} rx={3} className="ht-win" />
          <g className="ht-anim ht-right"><path d="M24 24 H50" className="ht-accent" /><path d="M44 18 L51 24 L44 30" className="ht-accent" fill="none" /></g>
        </g>
      );
    case 'commit':
      return (
        <g>
          <line x1={10} y1={24} x2={54} y2={24} className="ht-stroke" />
          <circle cx={14} cy={24} r={4} className="ht-solid" />
          <circle cx={30} cy={24} r={4} className="ht-solid" />
          <circle cx={48} cy={24} r={5} className="ht-accent-fill ht-anim ht-pulse" />
        </g>
      );
    case 'link':
      return (
        <g>
          <rect x={6} y={16} width={52} height={14} rx={7} className="ht-win" />
          <text x={12} y={26} className="ht-code">?ctz=…</text>
          <path d="M44 23 h8" className="ht-accent ht-anim ht-right" />
        </g>
      );
  }
}

export function HowToStrip({ steps, label }: { steps: HowStep[]; label: string }) {
  const ref = useRef<HTMLDivElement>(null);
  const near = useNearViewport(ref, '0px');
  const reduced = useReducedMotion();
  const [playing, setPlaying] = useState(true);
  const [active, setActive] = useState(0);
  const run = playing && !reduced && near;
  useEffect(() => {
    if (!run) return;
    const id = setInterval(() => setActive((a) => (a + 1) % steps.length), STEP_MS);
    return () => clearInterval(id);
  }, [run, steps.length]);
  return (
    <div ref={ref} className={`ht ${reduced ? 'ht-static' : ''}`} role="group" aria-label={`How to: ${label}`}>
      <ol className="ht-steps">
        {steps.map((s, i) => (
          <li key={i} className={`ht-step ${!reduced && i === active ? 'on' : ''} ${!reduced && i < active ? 'done' : ''}`}>
            <button type="button" className="ht-hit" onClick={() => { setActive(i); setPlaying(false); }} aria-label={`Step ${i + 1}: ${s.t}`} aria-current={i === active ? 'step' : undefined}>
              <svg viewBox="0 0 64 44" width="64" height="44" aria-hidden>
                <Picto a={s.a} />
              </svg>
              <span className="ht-num">{i + 1}</span>
            </button>
            <span className="ht-text">{s.t}</span>
          </li>
        ))}
      </ol>
      {!reduced && (
        <button type="button" className="ht-play" onClick={() => setPlaying((p) => !p)} aria-label={playing ? 'Pause the how-to animation' : 'Play the how-to animation'}>
          {playing ? '❚❚' : '▶'}
        </button>
      )}
    </div>
  );
}
