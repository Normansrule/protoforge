// Firmware architecture diagram: hardware, interrupts, threads and queues,
// with packets animated along the three data paths.
import { useMemo, useState } from 'react';
import { useFrame, useReducedMotion } from '../dsp/viz';
import { PlayControls } from './util';

type Kind = 'hw' | 'isr' | 'thread' | 'q';
interface Box { id: string; x: number; y: number; w: number; h: number; kind: Kind; title: string; sub: string }

const BOXES: Box[] = [
  { id: 'imu', x: 20, y: 30, w: 150, h: 54, kind: 'hw', title: 'IMU (I²C)', sub: '6-axis, 200 Hz' },
  { id: 'mic', x: 20, y: 150, w: 150, h: 54, kind: 'hw', title: 'PDM microphones', sub: '1.024 MHz bit stream' },
  { id: 'touch', x: 20, y: 270, w: 150, h: 54, kind: 'hw', title: 'Touch pad', sub: 'digital level' },
  { id: 'batt', x: 20, y: 380, w: 150, h: 54, kind: 'hw', title: 'Battery ADC', sub: 'VBAT divider' },
  { id: 'sensor', x: 230, y: 30, w: 170, h: 54, kind: 'thread', title: 'sensor thread', sub: 'complementary filter' },
  { id: 'dma', x: 230, y: 150, w: 170, h: 54, kind: 'isr', title: 'PDM EasyDMA IRQ', sub: 'CIC+FIR → 16 kHz blocks' },
  { id: 'timer', x: 230, y: 270, w: 170, h: 54, kind: 'isr', title: '1 kHz timer ISR', sub: 'gesture FSM step' },
  { id: 'power', x: 230, y: 380, w: 170, h: 54, kind: 'thread', title: 'power thread', sub: 'learned policy table' },
  { id: 'state', x: 460, y: 30, w: 150, h: 54, kind: 'q', title: 'HUD state', sub: 'mutex-protected' },
  { id: 'audio', x: 460, y: 150, w: 150, h: 54, kind: 'thread', title: 'audio thread', sub: 'high-pass → ADPCM' },
  { id: 'queue', x: 460, y: 270, w: 150, h: 54, kind: 'q', title: 'UI event queue', sub: 'k_msgq, 32 events' },
  { id: 'ui', x: 670, y: 90, w: 150, h: 54, kind: 'thread', title: 'ui thread', sub: 'render 2–10 fps' },
  { id: 'disp', x: 670, y: 20, w: 150, h: 46, kind: 'hw', title: 'Micro-OLED (SPI)', sub: '640 × 400' },
  { id: 'radio', x: 670, y: 200, w: 150, h: 54, kind: 'hw', title: 'BLE radio', sub: 'GATT notify / write' },
  { id: 'phone', x: 670, y: 330, w: 150, h: 54, kind: 'hw', title: 'Phone app', sub: 'speech → captions' },
];

const B = Object.fromEntries(BOXES.map((b) => [b.id, b]));
const R = (id: string) => [B[id].x + B[id].w, B[id].y + B[id].h / 2] as [number, number];
const L = (id: string) => [B[id].x, B[id].y + B[id].h / 2] as [number, number];

interface Flow { id: string; label: string; color: string; pts: [number, number][]; period: number }

const FLOWS: Flow[] = [
  { id: 'imu', label: 'IMU → filter → HUD', color: 'var(--series-1)', period: 2600, pts: [R('imu'), L('sensor'), R('sensor'), L('state'), R('state'), [640, 57], [640, 117], L('ui'), R('ui'), [832, 117], [832, 43], R('disp')] },
  { id: 'mic', label: 'mic → decimation → BLE', color: 'var(--series-2)', period: 2200, pts: [R('mic'), L('dma'), R('dma'), L('audio'), R('audio'), [640, 177], [640, 227], L('radio'), R('radio'), [832, 227], [832, 357], R('phone')] },
  { id: 'touch', label: 'touch → FSM → UI', color: 'var(--series-3)', period: 3000, pts: [R('touch'), L('timer'), R('timer'), L('queue'), R('queue'), [650, 297], [650, 130], [670, 130]] },
  { id: 'power', label: 'battery → policy → display + radio', color: 'var(--series-4)', period: 4200, pts: [R('batt'), L('power'), R('power'), [430, 407], [430, 360], [660, 360], [660, 240], [670, 240]] },
];

function along(pts: [number, number][], f: number): [number, number] {
  const segs: number[] = [];
  let total = 0;
  for (let i = 1; i < pts.length; i++) {
    const d = Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]);
    segs.push(d);
    total += d;
  }
  let s = f * total;
  for (let i = 0; i < segs.length; i++) {
    if (s <= segs[i]) {
      const k = segs[i] > 0 ? s / segs[i] : 0;
      return [pts[i][0] + (pts[i + 1][0] - pts[i][0]) * k, pts[i][1] + (pts[i + 1][1] - pts[i][1]) * k];
    }
    s -= segs[i];
  }
  return pts[pts.length - 1];
}

export function Architecture() {
  const reduced = useReducedMotion();
  const [playing, setPlaying] = useState(!reduced);
  const [t, setT] = useState(0);
  const [only, setOnly] = useState<string | null>(null);
  useFrame(playing, (dt) => setT((v) => v + dt));
  const cls = (k: Kind) => (k === 'hw' ? 'pf-arch-hw' : k === 'isr' ? 'pf-arch-isr' : k === 'thread' ? 'pf-arch-thread' : 'pf-arch-q');
  const dots = useMemo(
    () =>
      FLOWS.flatMap((fl) =>
        [0, 0.33, 0.66].map((ph) => {
          const f = ((t / fl.period + ph) % 1 + 1) % 1;
          return { fl, p: along(fl.pts, f) };
        }),
      ),
    [t],
  );
  return (
    <div className="pf-arch">
      <div className="row" style={{ justifyContent: 'space-between', marginBottom: 8 }}>
        <PlayControls playing={playing} reduced={reduced} onToggle={() => setPlaying(!playing)} />
        <div className="row">
          {FLOWS.map((f) => (
            <button key={f.id} className={`btn small ${only === f.id ? 'primary' : ''}`} onClick={() => setOnly(only === f.id ? null : f.id)} aria-pressed={only === f.id}>
              <span className="pf-swatch" style={{ background: f.color }} /> {f.label}
            </button>
          ))}
        </div>
      </div>
      <svg viewBox="0 0 840 450" role="img" aria-label="Firmware architecture: interrupts, threads, queues and data flows">
        {FLOWS.map((f) => (
          <polyline key={f.id} points={f.pts.map((p) => p.join(',')).join(' ')} className="pf-arch-link" style={{ stroke: only && only !== f.id ? 'var(--border)' : f.color, opacity: only && only !== f.id ? 0.4 : 0.75 }} />
        ))}
        {BOXES.map((b) => (
          <g key={b.id}>
            <rect x={b.x} y={b.y} width={b.w} height={b.h} rx={9} className={cls(b.kind)} strokeWidth={1.4} />
            <text className="pf-arch-title" x={b.x + 10} y={b.y + 21}>{b.title}</text>
            <text className="pf-arch-sub" x={b.x + 10} y={b.y + 38}>{b.sub}</text>
          </g>
        ))}
        {dots
          .filter((d) => !only || d.fl.id === only)
          .map((d, i) => (
            <circle key={i} cx={d.p[0]} cy={d.p[1]} r={5} fill={d.fl.color} stroke="var(--surface-1)" strokeWidth={1.5} />
          ))}
      </svg>
      <div className="pf-legend">
        <span><i className="pf-swatch" style={{ background: 'var(--series-3)', height: 10 }} />hardware</span>
        <span><i className="pf-swatch" style={{ background: 'var(--series-2)', height: 10 }} />interrupt</span>
        <span><i className="pf-swatch" style={{ background: 'var(--accent)', height: 10 }} />thread</span>
        <span><i className="pf-swatch" style={{ background: 'var(--series-4)', height: 10 }} />shared state / queue</span>
      </div>
    </div>
  );
}
