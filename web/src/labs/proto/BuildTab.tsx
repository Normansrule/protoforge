// Breadboard build: animated wiring diagram, BOM, assembly checklist,
// differences from the custom board and a power comparison through the
// Power Lab's budget.
import { useMemo, useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat } from '../../components/ui';
import { fmtNum } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { useFrame, useReducedMotion } from '../dsp/viz';
import type { Breadboard, Build, HudOut, Part, Wire } from './types';
import { DEFAULT_HUD, PlayControls, Seg, fbToDataUrl, polyLen, usePersistent, wireColor } from './util';

const SCENARIOS = [
  { value: 'reference', label: 'Reference mixed use' },
  { value: 'navigation', label: 'Navigation walk (UC-1)' },
  { value: 'captions', label: 'All-day captions (UC-2)' },
  { value: 'notifications', label: 'Glanceable notifications (UC-3)' },
  { value: 'standby', label: 'Standby' },
  { value: 'display_on', label: 'Display always on' },
];

function PartShape({ p, oledImg }: { p: Part; oledImg: string }) {
  const fill = p.kind === 'board' ? 'var(--pf-board)' : p.kind === 'battery' ? 'var(--surface-3)' : 'var(--pf-module)';
  const stroke = p.kind === 'board' ? 'var(--pf-board-edge)' : 'var(--border-strong)';
  const isOled = p.id === 'oled';
  const isBattery = p.kind === 'battery';
  const textFill = isBattery ? 'var(--text)' : undefined;
  return (
    <g>
      <rect x={p.x} y={p.y} width={p.w} height={p.h} rx={p.kind === 'board' ? 8 : 6} fill={fill} stroke={stroke} strokeWidth={1.5} />
      {p.kind === 'board' && p.id === 'xiao' && <rect x={p.x + p.w / 2 - 18} y={p.y - 8} width={36} height={14} rx={4} fill="var(--surface-3)" stroke="var(--border-strong)" />}
      {p.kind === 'board' && p.id === 'esp' && <rect x={p.x + p.w / 2 - 16} y={p.y + p.h - 6} width={32} height={14} rx={4} fill="var(--surface-3)" stroke="var(--border-strong)" />}
      {p.kind === 'board' && <rect x={p.x + p.w / 2 - 22} y={p.y + (p.id === 'esp' ? 150 : 70)} width={44} height={44} rx={3} fill="rgba(0,0,0,0.35)" stroke="rgba(255,255,255,0.18)" />}
      {isOled && (
        <>
          <rect x={p.x + 34} y={p.y + 14} width={p.w - 48} height={(p.w - 48) / 2} rx={3} fill="var(--pf-screen)" stroke="rgba(255,255,255,0.2)" />
          {oledImg && <image href={oledImg} x={p.x + 36} y={p.y + 16} width={p.w - 52} height={(p.w - 52) / 2} style={{ imageRendering: 'pixelated' }} preserveAspectRatio="none" />}
        </>
      )}
      {isBattery && (
        <>
          <rect x={p.x + 6} y={p.y + 8} width={p.w - 12} height={p.h - 16} rx={4} fill="none" stroke="var(--border-strong)" strokeDasharray="3 3" />
        </>
      )}
      <text className="pf-part-label" x={p.x + p.w / 2} y={isOled ? p.y + p.h - 26 : p.id === 'esp' ? p.y + 225 : p.y + p.h / 2 + (p.kind === 'board' ? 60 : isBattery ? -2 : -8)} textAnchor="middle" style={{ fill: textFill }}>
        {p.label}
      </text>
      <text className="pf-part-sub" x={p.x + p.w / 2} y={isOled ? p.y + p.h - 12 : p.id === 'esp' ? p.y + 240 : p.y + p.h / 2 + (p.kind === 'board' ? 76 : isBattery ? 14 : 8)} textAnchor="middle" style={{ fill: isBattery ? 'var(--text-2)' : undefined }}>
        {p.sublabel}
      </text>
      {p.pins.map((q) => {
        const inside = q.side === 'l' ? 1 : q.side === 'r' ? -1 : 0;
        const lx = q.x + inside * 7;
        const ly = q.side === 't' ? q.y + 13 : q.side === 'b' ? q.y - 7 : q.y + 3.5;
        const anchor = q.side === 'l' ? 'start' : q.side === 'r' ? 'end' : 'middle';
        const short = q.name.replace(/\.\d$/, '');
        return (
          <g key={q.name}>
            <rect x={q.x - 3.5} y={q.y - 3.5} width={7} height={7} rx={1.5} fill="var(--copper)" />
            <text className="pf-pin-label" x={lx} y={ly} textAnchor={anchor} style={{ fill: p.kind === 'battery' ? 'var(--text-2)' : 'rgba(255,255,255,0.85)' }}>{short}</text>
          </g>
        );
      })}
    </g>
  );
}

export function WiringDiagram(props: { bb: Breadboard; focus: number[]; oledImg: string }) {
  const { bb } = props;
  const reduced = useReducedMotion();
  const [playing, setPlaying] = useState(!reduced);
  const [t, setT] = useState(reduced ? bb.wires.length + 1 : 0);
  const [hover, setHover] = useState<Wire | null>(null);
  const lens = useMemo(() => bb.wires.map((w) => polyLen(w.route)), [bb]);
  useFrame(playing, (dt) => {
    setT((v) => {
      const nv = v + dt / 650;
      if (nv > bb.wires.length + 1.5) {
        setPlaying(false);
        return bb.wires.length + 1.5;
      }
      return nv;
    });
  });
  const hot = hover ? [hover.id] : props.focus;
  const endpoint = (part: string, pin: string) => bb.parts.find((p) => p.id === part)?.pins.find((q) => q.name === pin);
  return (
    <div>
      <div className="row" style={{ justifyContent: 'space-between', marginBottom: 8 }}>
        <PlayControls
          playing={playing}
          reduced={reduced}
          onToggle={() => {
            if (!playing && t >= bb.wires.length + 1) setT(0);
            setPlaying(!playing);
          }}
          onReplay={() => {
            setT(0);
            setPlaying(true);
          }}
        />
        <span className="muted small">{Math.min(bb.wires.length, Math.max(0, Math.floor(t)))} / {bb.wires.length} wires</span>
      </div>
      <div className="pf-diagram">
        <svg viewBox={`0 0 ${bb.width} ${bb.height}`} role="img" aria-label={`Wiring diagram: ${bb.title}`}>
          <defs>
            <pattern id="pf-holes" width="12" height="12" patternUnits="userSpaceOnUse">
              <circle cx="6" cy="6" r="1.6" fill="var(--pf-hole)" />
            </pattern>
          </defs>
          <rect x={0} y={0} width={bb.width} height={bb.height} fill="url(#pf-holes)" />
          {bb.parts.map((p) => <PartShape key={p.id} p={p} oledImg={props.oledImg} />)}
          {bb.wires.map((w, i) => {
            const f = Math.max(0, Math.min(1, t - i));
            if (f <= 0) return null;
            const L = lens[i];
            const on = hot.includes(w.id);
            const dim = hot.length > 0 && !on;
            const d = 'M' + w.route.map(([x, y]) => `${x},${y}`).join(' L');
            return (
              <g key={w.id} onMouseEnter={() => setHover(w)} onMouseLeave={() => setHover(null)} onFocus={() => setHover(w)} onBlur={() => setHover(null)} tabIndex={0} aria-label={`${w.from_part} ${w.from_pin} to ${w.to_part} ${w.to_pin}, ${w.signal}`}>
                <path className="pf-wire" d={d} stroke="rgba(0,0,0,0.35)" strokeWidth={on ? 7 : 5.5} strokeDasharray={`${L} ${L}`} strokeDashoffset={L * (1 - f)} style={{ opacity: dim ? 0.25 : 1 }} />
                <path className="pf-wire" d={d} stroke={wireColor(w.color)} strokeWidth={on ? 4.5 : 3} strokeDasharray={`${L} ${L}`} strokeDashoffset={L * (1 - f)} style={{ opacity: dim ? 0.3 : 1 }} />
                <path className="pf-wire-hit" d={d} />
                {f >= 1 && [endpoint(w.from_part, w.from_pin), endpoint(w.to_part, w.to_pin)].map((q, k) =>
                  q ? <circle key={k} cx={q.x} cy={q.y} r={on ? 7 : 4} fill={wireColor(w.color)} stroke={on ? 'var(--text)' : 'none'} strokeWidth={1.5} /> : null,
                )}
              </g>
            );
          })}
        </svg>
      </div>
      <div className="pf-tip" aria-live="polite">
        {hover ? (
          <>
            <b>{hover.from_part.toUpperCase()} {hover.from_pin}</b> → <b>{hover.to_part.toUpperCase()} {hover.to_pin}</b> · {hover.signal}
            {hover.note && <span className="muted"> — {hover.note}</span>}
          </>
        ) : (
          <span className="muted">Hover or tab to a wire to highlight both of its ends.</span>
        )}
      </div>
      <div className="pf-legend">
        {[['red', '3.3 V / battery +'], ['black', 'ground'], ['yellow', 'I²C clock'], ['blue', 'I²C data'], ['green', 'touch signal'], ['purple', 'I²S word select'], ['orange', 'I²S clock'], ['white', 'I²S data']]
          .filter(([c]) => bb.wires.some((w) => w.color === c))
          .map(([c, l]) => (
            <span key={c}><i className="pf-swatch" style={{ background: wireColor(c) }} />{l}</span>
          ))}
      </div>
    </div>
  );
}

function PowerCard({ bb, battery, setBattery, scenario, setScenario }: { bb: Breadboard; battery: number; setBattery: (v: number) => void; scenario: string; setScenario: (v: string) => void }) {
  const pw = bb.power;
  const max = Math.max(...pw.proto_rows.map((r) => r.avg_mw), ...pw.custom_rows.map((r) => r.avg_mw), 1);
  const Bars = ({ rows, title }: { rows: typeof pw.proto_rows; title: string }) => (
    <div>
      <div className="small" style={{ fontWeight: 650, marginBottom: 6 }}>{title}</div>
      {rows
        .filter((r) => r.avg_mw > 1e-3)
        .sort((a, b) => b.avg_mw - a.avg_mw)
        .map((r) => (
          <div key={r.name} style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1.4fr) minmax(0, 1fr) 64px', gap: 8, alignItems: 'center', fontSize: 12, marginBottom: 4 }}>
            <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', color: 'var(--text-2)' }} title={r.name}>{r.name}</span>
            <div style={{ height: 10, background: 'var(--surface-3)', borderRadius: 4 }}>
              <div style={{ width: `${(r.avg_mw / max) * 100}%`, height: '100%', background: 'var(--series-1)', borderRadius: 4, transition: 'width 0.4s' }} />
            </div>
            <span className="mono" style={{ textAlign: 'right' }}>{fmtNum(r.avg_mw)} mW</span>
          </div>
        ))}
    </div>
  );
  return (
    <Card title="Power: breadboard vs custom board" subtitle="Both run through the Power Lab's duty-cycle budget; module figures on the breadboard are typical values — measure yours.">
      <div className="grid grid-2" style={{ marginBottom: 10 }}>
        <Select label="Usage scenario" value={scenario} options={SCENARIOS} onChange={setScenario} />
        <Slider label="Battery (same cell for both)" value={battery} min={50} max={1000} step={10} unit="mAh" onChange={setBattery} />
      </div>
      <div className="grid grid-4">
        <Stat label="Breadboard average" value={fmtNum(pw.proto_avg_mw)} unit="mW" note={`regulator η ≈ ${(pw.proto_efficiency * 100).toFixed(0)} %`} />
        <Stat label="Breadboard life" value={fmtNum(pw.proto_life_h)} unit="h" tone={pw.proto_life_h >= 8 ? 'pass' : 'warn'} note={pw.proto_life_h >= 8 ? '✓ ≥ 8 h' : '! below the 8 h target'} />
        <Stat label="Custom board average" value={fmtNum(pw.custom_avg_mw)} unit="mW" note={`buck η = ${(pw.custom_efficiency * 100).toFixed(0)} %`} />
        <Stat label="Custom board life" value={fmtNum(pw.custom_life_h)} unit="h" tone={pw.custom_life_h >= 8 ? 'pass' : 'fail'} note={`breadboard draws ×${fmtNum(pw.ratio)}`} />
      </div>
      <div className="grid grid-2" style={{ marginTop: 14 }}>
        <Bars rows={pw.proto_rows} title="Breadboard (battery side of the regulator: ÷ η)" />
        <Bars rows={pw.custom_rows} title="Custom Iris board" />
      </div>
      <Formula>{`P_avg = (Σ qty·[P_active·d + P_sleep·(1 − d)]) / η      life = C·V_nom·usable / P_avg\nbreadboard: ${pw.proto_regulator}`}</Formula>
      <ul className="small muted" style={{ margin: '6px 0 0', paddingLeft: 18 }}>
        {bb.notes.map((n) => <li key={n}>{n}</li>)}
      </ul>
    </Card>
  );
}

export function BuildTab() {
  const project = useProject((s) => s.project);
  const [build, setBuild] = usePersistent<Build>('pf.build', 'xiao');
  const [battery, setBattery] = useState(project.design.battery.capacity_mah);
  const [scenario, setScenario] = useState('reference');
  const res = useCompute<Breadboard>('firmware.breadboard', { project, build, battery_mah: battery, scenario });
  const hud = useCompute<HudOut>('firmware.hud_render', { state: DEFAULT_HUD, width: 128, height: 64, bpp: 1 });
  const oledImg = useMemo(() => (hud.data ? fbToDataUrl(hud.data, [127, 209, 255]) : ''), [hud.data]);
  const [done, setDone] = usePersistent<number[]>(`pf.steps.${build}`, []);
  const [focus, setFocus] = useState<number | null>(null);
  const bb = res.data;
  const bomCsv = () =>
    'item,qty,vendor category,stands in for,notes\n' +
    (bb?.bom ?? []).map((b) => [b.item, b.qty, b.category, b.stands_in_for, b.notes].map((c) => `"${String(c).replace(/"/g, '""')}"`).join(',')).join('\n');
  const wiringCsv = () =>
    'from,pin,to,pin,colour,signal,note\n' +
    (bb?.wires ?? []).map((w) => [w.from_part, w.from_pin, w.to_part, w.to_pin, w.color, w.signal, w.note].map((c) => `"${String(c).replace(/"/g, '""')}"`).join(',')).join('\n');
  return (
    <div className="stack">
      <div className="row" style={{ justifyContent: 'space-between' }}>
        <Seg<Build>
          ariaLabel="Build"
          value={build}
          onChange={(b) => {
            setBuild(b);
            setFocus(null);
          }}
          options={[
            { value: 'xiao', label: 'XIAO nRF52840 Sense (primary)' },
            { value: 'esp32s3', label: 'ESP32-S3 (Wokwi-simulatable)' },
          ]}
        />
        {bb && (
          <div className="row">
            <DownloadButton small filename={`iris-v0-${build}-bom.csv`} content={bomCsv} label="BOM CSV" />
            <DownloadButton small filename={`iris-v0-${build}-wiring.csv`} content={wiringCsv} label="Wiring CSV" />
          </div>
        )}
      </div>
      <ErrorBox error={res.error} />
      {bb && (
        <>
          <div className="pf-main">
            <Card title={bb.title} subtitle={bb.summary}>
              <WiringDiagram key={build} bb={bb} focus={focus !== null ? bb.steps[focus]?.wires ?? [] : []} oledImg={oledImg} />
            </Card>
            <Card title="Assembly" subtitle="Click a step to highlight its wires; tick it off when done (saved in this browser).">
              <div className="pf-progress" aria-label={`${done.length} of ${bb.steps.length} steps done`}><div style={{ width: `${(done.length / bb.steps.length) * 100}%` }} /></div>
              <div className="pf-steps">
                {bb.steps.map((s, i) => (
                  <div key={s.title} className={`pf-step ${focus === i ? 'focus' : ''} ${done.includes(i) ? 'done' : ''}`} onClick={() => setFocus(focus === i ? null : i)} role="button" tabIndex={0} onKeyDown={(e) => { if (e.key === 'Enter') setFocus(focus === i ? null : i); }}>
                    <input
                      type="checkbox"
                      checked={done.includes(i)}
                      aria-label={`Mark step ${i + 1} done`}
                      onClick={(e) => e.stopPropagation()}
                      onChange={(e) => setDone(e.target.checked ? [...done, i] : done.filter((d) => d !== i))}
                    />
                    <div>
                      <div className="pf-step-title">{i + 1}. {s.title}</div>
                      <div className="pf-step-detail">{s.detail}</div>
                    </div>
                  </div>
                ))}
              </div>
            </Card>
          </div>
          <Card title="Wiring table" subtitle="Pin ↔ pin with the wire colour; names follow the board vendor's published pinout — check it against your board revision.">
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>#</th><th>From</th><th>To</th><th>Wire</th><th>Signal</th><th>Note</th></tr></thead>
                <tbody>
                  {bb.wires.map((w) => (
                    <tr key={w.id}>
                      <td className="mono">{w.id + 1}</td>
                      <td className="mono">{w.from_part} · {w.from_pin}</td>
                      <td className="mono">{w.to_part} · {w.to_pin}</td>
                      <td><span className="pf-key"><i style={{ background: wireColor(w.color) }} />{w.color}</span></td>
                      <td>{w.signal}</td>
                      <td className="muted small">{w.note}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
          <Card title="Bill of materials (development boards)" subtitle="Vendor categories, not quotes — check current availability.">
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>Item</th><th>Qty</th><th>Where (category)</th><th>Stands in for</th><th>Notes</th></tr></thead>
                <tbody>
                  {bb.bom.map((b) => (
                    <tr key={b.item}><td>{b.item}</td><td className="mono">{b.qty}</td><td className="small">{b.category}</td><td className="small mono">{b.stands_in_for}</td><td className="small muted">{b.notes}</td></tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
          <PowerCard bb={bb} battery={battery} setBattery={setBattery} scenario={scenario} setScenario={setScenario} />
          <Card title="What differs from the custom board" subtitle="The breadboard proves function; these gaps are what the other labs cover.">
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>Aspect</th><th>Breadboard</th><th>Custom Iris board</th><th>Consequence</th></tr></thead>
                <tbody>
                  {bb.differences.map((d) => (
                    <tr key={d.aspect}><td style={{ fontWeight: 650 }}>{d.aspect}</td><td className="small">{d.prototype}</td><td className="small">{d.custom}</td><td className="small muted">{d.consequence}</td></tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
          <div className="grid grid-2">
            <Explainer title="Why a development board first" refs={[{ label: 'Seeed Studio wiki: XIAO nRF52840 (Sense)', href: 'https://wiki.seeedstudio.com' }, { label: 'E. White, Making Embedded Systems, 2nd ed., O\'Reilly 2024' }]}>
              A breadboard with a development board lets the firmware, the phone app and the user interface be exercised weeks before a custom board exists — and every bug found here is one that does not cost a board re-spin. The XIAO nRF52840 Sense is unusually close to Iris: the same Bluetooth system-on-chip family, an inertial measurement unit (IMU), a pulse-density-modulation (PDM) microphone and a LiPo charger on a 21 × 17.5 mm module.
            </Explainer>
            <Explainer title="I²C in two wires" refs={[{ label: 'NXP, UM10204 I²C-bus specification and user manual' }]}>
              Inter-Integrated Circuit (I²C) shares one clock (SCL) and one data line (SDA) among many chips, each with a 7-bit address (OLED 0x3C, MPU-6050 0x68). Lines are open-drain with pull-up resistors, so any device can pull low and none can short the bus. At 400 kHz a full 128 × 64 frame (1024 bytes ≈ 9 bits each) takes ≈ 23 ms — why the prototype redraws at 10 frames per second at most.
              <Formula>{'t_frame ≈ 1024 B × 9 bit / 400 kbit/s ≈ 23 ms'}</Formula>
            </Explainer>
          </div>
        </>
      )}
    </div>
  );
}
