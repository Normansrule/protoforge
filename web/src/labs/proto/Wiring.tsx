// Wiring: the breadboard diagram (shared with the Build tab) with bus
// lighting and animated signal flow, the pin map derived from the Iris
// interfaces, and the pin-to-pin wiring table.
import { useMemo, useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula } from '../../components/ui';
import { fmtNum } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { useFrame, useReducedMotion } from '../dsp/viz';
import type { Breadboard, Build, Bus, HudOut, Part, Wire } from './types';
import { DEFAULT_HUD, PlayControls, Seg, fbToDataUrl, polyLen, usePersistent, wireColor } from './util';

export const BUILDS: { value: Build; label: string }[] = [
  { value: 'xiao', label: 'XIAO nRF52840 Sense' },
  { value: 'nrf52840dk', label: 'nRF52840 DK (Zephyr)' },
  { value: 'esp32s3', label: 'ESP32-S3 (Wokwi)' },
];

export const BUS_INFO: Record<Bus, { label: string; color: string; what: string }> = {
  power: { label: 'Power', color: 'var(--pf-wire-red)', what: '3.3 V (or the DK\'s 3.0 V VDD) to every module' },
  ground: { label: 'Ground', color: 'var(--pf-wire-black)', what: 'one shared reference — every module must share it' },
  battery: { label: 'Battery', color: 'var(--pf-wire-orange)', what: 'LiPo into the charger, or the simulated battery-sense voltage' },
  i2c: { label: 'I²C', color: 'var(--pf-wire-blue)', what: 'two open-drain wires (SDA, SCL) shared by addressed devices' },
  spi: { label: 'SPI', color: 'var(--pf-wire-yellow)', what: 'clock + data out + chip select (+ data/command, reset) to the display' },
  pdm: { label: 'PDM', color: 'var(--pf-wire-purple)', what: '1-bit microphone stream: clock out, data back' },
  i2s: { label: 'I²S', color: 'var(--pf-wire-purple)', what: 'bit clock, word select and serial data from a digital microphone' },
  gpio: { label: 'GPIO', color: 'var(--pf-wire-green)', what: 'single digital signals: touch output, interrupt lines' },
};
const BUS_ORDER: Bus[] = ['power', 'ground', 'battery', 'i2c', 'spi', 'pdm', 'i2s', 'gpio'];
const BOARDS = ['xiao', 'dk', 'esp'];

/** Does energy / data on this wire flow towards the development board? */
function towardBoard(w: Wire): boolean {
  if (w.bus === 'battery') return true;
  return /TOUCH|INT|DATA|DAT\b|SD\b|SIG|VBAT/.test(w.signal);
}

function PartShape({ p, oledImg }: { p: Part; oledImg: string }) {
  const board = p.kind === 'board';
  const onboard = p.kind === 'onboard';
  const battery = p.kind === 'battery';
  const fill = board ? 'var(--pf-board)' : battery ? 'var(--surface-3)' : onboard ? 'var(--pf-chip)' : 'var(--pf-module)';
  const stroke = board ? 'var(--pf-board-edge)' : onboard ? 'rgba(255,255,255,0.25)' : 'var(--border-strong)';
  const isOled = p.id === 'oled';
  const pinsLeft = p.pins[0]?.side === 'l';
  const labelAbove = board && p.id !== 'esp';
  const pinText = battery ? 'var(--text-2)' : 'rgba(255,255,255,0.85)';
  const hasPins = p.kind === 'module' && p.pins.length > 0;
  const cx = hasPins && !isOled ? (pinsLeft ? p.x + 22 + (p.w - 22) / 2 : p.x + (p.w - 22) / 2) : p.x + p.w / 2;
  const scrW = Math.min(p.w - 48, 150);
  const scrX = pinsLeft ? p.x + p.w - scrW - 12 : p.x + 12;
  const scrH = scrW / 2;
  if (onboard) {
    return (
      <g>
        <rect x={p.x} y={p.y} width={p.w} height={p.h} rx={3} fill={fill} stroke={stroke} />
        <text className="pf-chip-label" x={p.x + p.w / 2} y={p.y + p.h / 2 + (p.sublabel && p.h > 30 ? -1 : 3)} textAnchor="middle">{p.label}</text>
        {p.h > 30 && <text className="pf-chip-sub" x={p.x + p.w / 2} y={p.y + p.h / 2 + 11} textAnchor="middle">{p.sublabel}</text>}
        {p.pins.map((q) => <circle key={q.name} cx={q.x} cy={q.y} r={2.5} fill="var(--copper)" />)}
      </g>
    );
  }
  return (
    <g>
      <rect x={p.x} y={p.y} width={p.w} height={p.h} rx={board ? 8 : 6} fill={fill} stroke={stroke} strokeWidth={1.5} />
      {p.id === 'xiao' && <rect x={p.x + p.w / 2 - 18} y={p.y - 8} width={36} height={14} rx={4} fill="var(--surface-3)" stroke="var(--border-strong)" />}
      {p.id === 'esp' && <rect x={p.x + p.w / 2 - 16} y={p.y + p.h - 6} width={32} height={14} rx={4} fill="var(--surface-3)" stroke="var(--border-strong)" />}
      {p.id === 'esp' && <rect x={p.x + p.w / 2 - 22} y={p.y + 150} width={44} height={44} rx={3} fill="rgba(0,0,0,0.35)" stroke="rgba(255,255,255,0.18)" />}
      {p.id === 'dk' && (
        <>
          <rect x={p.x + p.w / 2 - 28} y={p.y + 110} width={56} height={56} rx={4} fill="var(--pf-chip)" stroke="rgba(255,255,255,0.25)" />
          <text className="pf-chip-label" x={p.x + p.w / 2} y={p.y + 141} textAnchor="middle">nRF52840</text>
          <rect x={p.x + p.w / 2 - 30} y={p.y + p.h - 70} width={60} height={34} rx={3} fill="rgba(0,0,0,0.3)" stroke="rgba(255,255,255,0.18)" />
          <text className="pf-chip-sub" x={p.x + p.w / 2} y={p.y + p.h - 49} textAnchor="middle">debugger</text>
          <rect x={p.x - 8} y={p.y + p.h - 34} width={16} height={24} rx={3} fill="var(--surface-3)" stroke="var(--border-strong)" />
        </>
      )}
      {isOled && (
        <>
          <rect x={scrX} y={p.y + 14} width={scrW} height={scrH} rx={3} fill="var(--pf-screen)" stroke="rgba(255,255,255,0.2)" />
          {oledImg && <image href={oledImg} x={scrX + 2} y={p.y + 16} width={scrW - 4} height={scrH - 4} style={{ imageRendering: 'pixelated' }} preserveAspectRatio="none" />}
        </>
      )}
      {battery && <rect x={p.x + 6} y={p.y + 8} width={p.w - 12} height={p.h - 16} rx={4} fill="none" stroke="var(--border-strong)" strokeDasharray="3 3" />}
      {labelAbove ? (
        <>
          <text className="pf-part-label pf-on-bread" x={p.x + p.w / 2} y={p.y - 28} textAnchor="middle">{p.label}</text>
          <text className="pf-part-sub pf-on-bread" x={p.x + p.w / 2} y={p.y - 15} textAnchor="middle">{p.sublabel}</text>
        </>
      ) : (
        <>
          <text className="pf-part-label" x={isOled ? scrX + scrW / 2 : cx} y={isOled ? p.y + p.h - 26 : p.id === 'esp' ? p.y + 225 : p.y + p.h / 2 + (battery ? -2 : -8)} textAnchor="middle" style={{ fill: battery ? 'var(--text)' : undefined }}>
            {p.label}
          </text>
          <text className="pf-part-sub" x={isOled ? scrX + scrW / 2 : cx} y={isOled ? p.y + p.h - 12 : p.id === 'esp' ? p.y + 240 : p.y + p.h / 2 + (battery ? 14 : 8)} textAnchor="middle" style={{ fill: battery ? 'var(--text-2)' : undefined }}>
            {p.sublabel}
          </text>
        </>
      )}
      {p.pins.map((q) => {
        const inside = q.side === 'l' ? 1 : q.side === 'r' ? -1 : 0;
        const lx = q.x + inside * 7;
        const ly = q.side === 't' ? q.y + 13 : q.side === 'b' ? q.y - 7 : q.y + 3.5;
        const anchor = q.side === 'l' ? 'start' : q.side === 'r' ? 'end' : 'middle';
        return (
          <g key={q.name}>
            <rect x={q.x - 3.5} y={q.y - 3.5} width={7} height={7} rx={1.5} fill="var(--copper)" />
            <text className="pf-pin-label" x={lx} y={ly} textAnchor={anchor} style={{ fill: pinText }}>{q.name.replace(/\.\d$/, '')}</text>
          </g>
        );
      })}
    </g>
  );
}

export function WiringDiagram(props: { bb: Breadboard; focus: number[]; oledImg: string; bus?: Bus | 'all'; flow?: boolean }) {
  const { bb } = props;
  const reduced = useReducedMotion();
  const [playing, setPlaying] = useState(!reduced);
  const [t, setT] = useState(reduced ? bb.wires.length + 2 : 0);
  const [phase, setPhase] = useState(0);
  const [hover, setHover] = useState<Wire | null>(null);
  const lens = useMemo(() => bb.wires.map((w) => polyLen(w.route)), [bb]);
  const building = t < bb.wires.length + 1.5;
  useFrame(playing, (dt) => {
    if (building) {
      setT((v) => Math.min(bb.wires.length + 1.5, v + dt / 420));
    } else if (props.flow) {
      setPhase((v) => (v + dt * 0.05) % 1000);
    } else {
      setPlaying(false);
    }
  });
  const bus = props.bus ?? 'all';
  const hot = hover ? [hover.id] : props.focus;
  const lit = (w: Wire) => (hot.length ? hot.includes(w.id) : bus === 'all' || w.bus === bus);
  const endpoint = (part: string, pin: string) => bb.parts.find((p) => p.id === part)?.pins.find((q) => q.name === pin);
  const boards = bb.parts.filter((p) => p.kind !== 'onboard');
  const chips = bb.parts.filter((p) => p.kind === 'onboard');
  const renderWire = (w: Wire, i: number) => {
    const f = Math.max(0, Math.min(1, t - i));
    if (f <= 0) return null;
    const L = lens[i];
    const on = lit(w);
    const strong = hot.includes(w.id) || (bus !== 'all' && w.bus === bus);
    const d = 'M' + w.route.map(([x, y]) => `${x},${y}`).join(' L');
    const color = wireColor(w.color);
    const reveal = f < 1 ? { strokeDasharray: `${L} ${L}`, strokeDashoffset: L * (1 - f) } : w.onboard ? { strokeDasharray: '5 3' } : {};
    const dir = towardBoard(w) === BOARDS.includes(w.to_part) ? -1 : 1;
    const flowing = props.flow && f >= 1 && strong && !['ground'].includes(w.bus);
    return (
      <g key={w.id} onMouseEnter={() => setHover(w)} onMouseLeave={() => setHover(null)} onFocus={() => setHover(w)} onBlur={() => setHover(null)} tabIndex={0} aria-label={`${w.from_part} ${w.from_pin} to ${w.to_part} ${w.to_pin}, ${w.signal}`} style={{ opacity: on ? 1 : 0.14, transition: 'opacity 0.25s' }}>
        {strong && <path className="pf-wire" d={d} stroke={color} strokeWidth={12} opacity={0.22} />}
        {!w.onboard && <path className="pf-wire" d={d} stroke="rgba(0,0,0,0.35)" strokeWidth={strong ? 7 : 5.5} {...reveal} />}
        <path className="pf-wire" d={d} stroke={color} strokeWidth={w.onboard ? 2.2 : strong ? 4.5 : 3} {...reveal} />
        {flowing && <path className="pf-wire pf-flow" d={d} strokeDasharray="3 15" strokeDashoffset={dir * -phase} />}
        <path className="pf-wire-hit" d={d} />
        {f >= 1 && !w.onboard && [endpoint(w.from_part, w.from_pin), endpoint(w.to_part, w.to_pin)].map((q, k) =>
          q ? <circle key={k} cx={q.x} cy={q.y} r={strong ? 6 : 4} fill={color} stroke={strong ? 'var(--text)' : 'none'} strokeWidth={1.5} /> : null,
        )}
      </g>
    );
  };
  const nWires = bb.wires.filter((w) => !w.onboard).length;
  const shown = Math.min(nWires, bb.wires.filter((w, i) => !w.onboard && t - i >= 1).length);
  return (
    <div>
      <div className="row" style={{ justifyContent: 'space-between', marginBottom: 8 }}>
        <PlayControls
          playing={playing}
          reduced={reduced}
          onToggle={() => {
            if (!playing && !building && !props.flow) setT(0);
            setPlaying(!playing);
          }}
          onReplay={() => {
            setT(0);
            setPlaying(true);
          }}
        />
        <span className="muted small">{shown} / {nWires} jumper wires{building && playing ? ' · wiring…' : props.flow && playing ? ' · signals flowing' : ''}</span>
      </div>
      <div className="pf-diagram">
        <svg viewBox={`0 0 ${bb.width} ${bb.height}`} role="img" aria-label={`Wiring diagram: ${bb.title}`}>
          <defs>
            <pattern id="pf-holes" width="12" height="12" patternUnits="userSpaceOnUse">
              <circle cx="6" cy="6" r="1.6" fill="var(--pf-hole)" />
            </pattern>
          </defs>
          <rect x={0} y={0} width={bb.width} height={bb.height} fill="url(#pf-holes)" />
          {boards.map((p) => <PartShape key={p.id} p={p} oledImg={props.oledImg} />)}
          {bb.wires.map((w, i) => (w.onboard ? renderWire(w, i) : null))}
          {chips.map((p) => <PartShape key={p.id} p={p} oledImg={props.oledImg} />)}
          {bb.wires.map((w, i) => (w.onboard ? null : renderWire(w, i)))}
        </svg>
      </div>
      <div className="pf-tip" aria-live="polite">
        {hover ? (
          <>
            <b>{hover.from_part.toUpperCase()} {hover.from_pin.replace(/\.\d$/, '')}</b> → <b>{hover.to_part.toUpperCase()} {hover.to_pin.replace(/\.\d$/, '')}</b> · {hover.signal} · <span className="mono">{BUS_INFO[hover.bus]?.label ?? hover.bus}</span>
            {hover.onboard && <span className="muted"> (trace on the board)</span>}
            {hover.note && <span className="muted"> — {hover.note}</span>}
          </>
        ) : (
          <span className="muted">Hover or tab to a wire to see both ends; dashed lines are traces already on the development board.</span>
        )}
      </div>
    </div>
  );
}

export function useOledImage(): string {
  const hud = useCompute<HudOut>('firmware.hud_render', { state: DEFAULT_HUD, width: 128, height: 64, bpp: 1 });
  return useMemo(() => (hud.data ? fbToDataUrl(hud.data, [127, 209, 255]) : ''), [hud.data]);
}

const csv = (rows: (string | number)[][]) => rows.map((r) => r.map((c) => `"${String(c).replace(/"/g, '""')}"`).join(',')).join('\n');

export function WiringTab() {
  const project = useProject((s) => s.project);
  const [build, setBuild] = usePersistent<Build>('pf.build', 'xiao');
  const [bus, setBus] = usePersistent<Bus | 'all'>('pf.bus', 'spi');
  const res = useCompute<Breadboard>('firmware.breadboard', { project, build });
  const oledImg = useOledImage();
  const bb = res.data;
  const present = BUS_ORDER.filter((b) => bb?.wires.some((w) => w.bus === b));
  const active: Bus | 'all' = bus !== 'all' && !present.includes(bus) ? 'all' : bus;
  const busWires = bb?.wires.filter((w) => active === 'all' || w.bus === active) ?? [];
  const audio = project.design.audio;
  const speed: Partial<Record<Bus, string>> = {
    i2c: '400 kHz (fast mode) — 9 bit times per byte incl. ACK',
    spi: `${build === 'esp32s3' ? '—' : '8 MHz'} clock, one bit per clock, no addressing: chip select picks the device`,
    pdm: `${fmtNum(audio.pdm_clock_hz / 1e6)} MHz clock, decimated ÷${audio.decimation} to ${fmtNum(audio.sample_rate_hz / 1000)} kHz PCM`,
    i2s: 'bit clock 32 × 2 × f_s; the microphone decimates internally',
    power: build === 'nrf52840dk' ? '3.0 V VDD from the DK regulator' : '3.3 V from the board regulator',
  };
  return (
    <div className="stack">
      <div className="row" style={{ justifyContent: 'space-between' }}>
        <Seg<Build> ariaLabel="Build" value={build} onChange={setBuild} options={BUILDS} />
        {bb && (
          <div className="row">
            <DownloadButton small filename={`iris-v0-${build}-wiring.csv`} label="Wiring CSV" content={() => csv([['from', 'pin', 'to', 'pin', 'bus', 'colour', 'signal', 'on board', 'note'], ...bb.wires.map((w) => [w.from_part, w.from_pin, w.to_part, w.to_pin, w.bus, w.color, w.signal, w.onboard ? 'yes' : 'no', w.note])])} />
            <DownloadButton small filename={`iris-v0-${build}-pinmap.csv`} label="Pin map CSV" content={() => csv([['interface', 'kind', 'from', 'to', 'signal', 'custom board', 'prototype', 'note'], ...bb.pinmap.flatMap((r) => r.signals.map((s) => [r.interface, r.kind, r.from, r.to, s.signal, s.custom, s.prototype, s.note]))])} />
          </div>
        )}
      </div>
      <ErrorBox error={res.error} />
      {bb && (
        <>
          <div className="pf-main">
            <Card title={`Light a bus — ${bb.title}`} subtitle="Pick a bus to light its wires and watch the signals flow (data towards the display, sensor data back to the processor).">
              <div className="pf-buses" role="group" aria-label="Bus">
                <button className={`pf-bus ${active === 'all' ? 'active' : ''}`} aria-pressed={active === 'all'} onClick={() => setBus('all')}>All</button>
                {present.map((b) => (
                  <button key={b} className={`pf-bus ${active === b ? 'active' : ''}`} aria-pressed={active === b} onClick={() => setBus(b)}>
                    <i style={{ background: BUS_INFO[b].color }} />{BUS_INFO[b].label}<small>{bb.wires.filter((w) => w.bus === b).length}</small>
                  </button>
                ))}
              </div>
              <WiringDiagram key={build} bb={bb} focus={[]} oledImg={oledImg} bus={active} flow={active !== 'all'} />
            </Card>
            <Card title={active === 'all' ? 'All connections' : `${BUS_INFO[active].label} bus`} subtitle={active === 'all' ? 'Pick a bus to see what it carries.' : BUS_INFO[active].what}>
              {active !== 'all' && speed[active] && <div className="pf-cmd" style={{ marginBottom: 10 }}>{speed[active]}</div>}
              <div className="pf-buslist">
                {busWires.map((w) => (
                  <div key={w.id}>
                    <i style={{ background: wireColor(w.color) }} />
                    <span><b>{w.signal}</b> <span className="mono small">{w.from_part}:{w.from_pin.replace(/\.\d$/, '')} → {w.to_part}:{w.to_pin.replace(/\.\d$/, '')}</span>{w.onboard && <span className="muted small"> · on board</span>}</span>
                  </div>
                ))}
              </div>
            </Card>
          </div>
          <Card title="Pin map from the Iris architecture" subtitle="Each electrical interface of the project (Systems Studio → interfaces) becomes the signals it needs: the proposed custom-board pin (same as the Zephyr overlay) and the pin on this build. Click a row to light its bus.">
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>Interface</th><th>Signal</th><th>Custom board (nRF52840)</th><th>This build</th><th>Note</th></tr></thead>
                <tbody>
                  {bb.pinmap.flatMap((r) =>
                    r.signals.map((s, k) => (
                      <tr key={r.interface + s.signal} className={r.bus && r.bus === active ? 'pf-row-on' : ''} onClick={() => r.bus && setBus(r.bus as Bus)} style={{ cursor: r.bus ? 'pointer' : undefined }}>
                        {k === 0 && (
                          <td rowSpan={r.signals.length} style={{ verticalAlign: 'top' }}>
                            <b className="mono">{r.interface}</b> <span className="pf-kind">{r.kind}</span>
                            <div className="small muted">{r.from} → {r.to}</div>
                            <div className="small muted" style={{ maxWidth: 260 }}>{r.description}</div>
                          </td>
                        )}
                        <td className="mono small">{s.signal}</td>
                        <td className="mono small">{s.custom}</td>
                        <td className="mono small">{s.prototype}</td>
                        <td className="small muted">{s.note}</td>
                      </tr>
                    )),
                  )}
                </tbody>
              </table>
            </div>
          </Card>
          <Card title="Wiring table" subtitle="Pin ↔ pin with the wire colour; names follow the board vendor's published pinout — check them against your board revision.">
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>#</th><th>From</th><th>To</th><th>Bus</th><th>Signal</th><th>Note</th></tr></thead>
                <tbody>
                  {bb.wires.map((w) => (
                    <tr key={w.id} className={active !== 'all' && w.bus === active ? 'pf-row-on' : ''}>
                      <td className="mono">{w.id + 1}</td>
                      <td className="mono small">{w.from_part} · {w.from_pin.replace(/\.\d$/, '')}</td>
                      <td className="mono small">{w.to_part} · {w.to_pin.replace(/\.\d$/, '')}</td>
                      <td><span className="pf-key"><i style={{ background: wireColor(w.color) }} />{BUS_INFO[w.bus]?.label}{w.onboard ? ' (board)' : ''}</span></td>
                      <td className="small">{w.signal}</td>
                      <td className="muted small">{w.note}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
          <div className="grid grid-3">
            <Explainer title="Serial Peripheral Interface (SPI)" refs={[{ label: 'Nordic Semiconductor, nRF52840 Product Specification (SPIM)' }, { label: 'Solomon Systech, SSD1306 datasheet (4-wire SPI)' }]}>
              A controller drives a clock (SCK) and shifts one bit per clock out on MOSI; chip select (CS) picks the device and a data/command (DC) line tells a display whether the bytes are pixels or instructions. No addresses and no acknowledgements, so it is fast — which is why Iris sends its 32 000-byte frames over SPI.
              <Formula>{'t_frame = 8 · W·H·b/8 / f_SCK   (640×400×1 bit at 8 MHz = 32 ms)'}</Formula>
            </Explainer>
            <Explainer title="Inter-Integrated Circuit (I²C)" refs={[{ label: 'NXP, UM10204 I²C-bus specification and user manual' }]}>
              Two open-drain wires shared by many chips, each with a 7-bit address (IMU 0x68, OLED 0x3C). Pull-up resistors pull the lines high; any device can pull them low, so nothing can short the bus. Every byte costs 9 clocks (8 data + acknowledge).
              <Formula>{'t_frame ≈ 1024 B × 9 bit / 400 kbit/s ≈ 23 ms  (128×64 OLED)'}</Formula>
            </Explainer>
            <Explainer title="Pulse-density modulation (PDM)" refs={[{ label: 'Nordic Semiconductor, nRF52840 Product Specification (PDM)' }, { label: 'R. G. Lyons, Understanding Digital Signal Processing, 3rd ed., Prentice Hall 2010' }]}>
              A PDM microphone outputs one bit per clock whose density follows the sound pressure. The processor supplies the clock and a decimation filter turns the bit stream into 16-bit samples: the DSP Lab designs that filter, and the nRF52840's PDM peripheral runs one in hardware. Two microphones share a clock and data line, one on each clock edge.
              <Formula>{`f_s = f_PDM / R = ${fmtNum(audio.pdm_clock_hz / 1e6)} MHz / ${audio.decimation} = ${fmtNum(audio.pdm_clock_hz / audio.decimation / 1000)} kHz`}</Formula>
            </Explainer>
          </div>
        </>
      )}
    </div>
  );
}
