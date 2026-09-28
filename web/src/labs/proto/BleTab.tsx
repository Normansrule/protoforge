// Bluetooth Low Energy protocol: GATT table, connection-event timing
// animation, throughput/latency results and a phone ↔ glasses sequence chart.
import { useState } from 'react';
import { Card, Chip, Code, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat, Toggle } from '../../components/ui';
import { Plot, fmtNum } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { useFrame, useReducedMotion } from '../dsp/viz';
import type { BleOut, LinkParams, LinkResult, Sweep } from './types';
import { PlayControls, Seg } from './util';

const DEFAULT_LINK: LinkParams = {
  conn_interval_ms: 30, slow_interval_ms: 300, mtu: 247, phy: '2M', dle: true, encrypted: true, max_packets_per_event: 6,
  peripheral_latency: 0, asr_ms: 300, audio_frame_ms: 20, caption_chars: 40, audio_rate_hz: 16000, audio_bits: 4,
};

function Timing({ l }: { l: LinkResult }) {
  const reduced = useReducedMotion();
  const [playing, setPlaying] = useState(!reduced);
  const [t, setT] = useState(0); // µs of simulated time
  const ci = l.params.conn_interval_ms * 1000;
  const span = 2 * ci;
  useFrame(playing, (dt) => setT((v) => (v + dt * (span / 4000)) % span)); // two intervals every 4 s
  const W = 860;
  const X = (us: number) => 40 + (us / span) * (W - 60);
  const evEnd = l.event.length ? l.event[l.event.length - 1].t_us + l.event[l.event.length - 1].dur_us : 0;
  // zoom on the first connection event
  const ZX = (us: number) => 40 + (us / Math.max(evEnd * 1.05, 1)) * (W - 60);
  const delivered = [0, 1].reduce((s, k) => s + l.event.filter((e) => e.data && k * ci + e.t_us + e.dur_us <= t).length * (l.att_data_per_notification / l.ll_packets_per_notification), 0);
  return (
    <div className="pf-timing">
      <div className="row" style={{ justifyContent: 'space-between', marginBottom: 6 }}>
        <PlayControls playing={playing} reduced={reduced} onToggle={() => setPlaying(!playing)} />
        <span className="mono small">t = {(t / 1000).toFixed(1)} ms · delivered {Math.round(delivered)} B</span>
      </div>
      <svg viewBox={`0 0 ${W} 250`} role="img" aria-label="Connection event timing">
        <text x={40} y={16} fontSize={12} fill="var(--text-2)">Two connection intervals ({fmtNum(l.params.conn_interval_ms)} ms each) — the radio sleeps between events</text>
        <line x1={40} x2={W - 20} y1={70} y2={70} stroke="var(--axis)" />
        {[0, 1, 2].map((k) => (
          <g key={k}>
            <line x1={X(k * ci)} x2={X(k * ci)} y1={30} y2={78} stroke="var(--border-strong)" strokeDasharray="3 3" />
            <text x={X(k * ci)} y={92} fontSize={10.5} fill="var(--muted)" textAnchor="middle" fontFamily="var(--mono)">{fmtNum((k * ci) / 1000)} ms</text>
          </g>
        ))}
        {[0, 1].map((k) =>
          l.event.map((e, i) => {
            const x = X(k * ci + e.t_us);
            const w = Math.max(1.2, X(k * ci + e.t_us + e.dur_us) - x);
            const past = k * ci + e.t_us <= t;
            return <rect key={`${k}-${i}`} x={x} y={e.from === 'central' ? 40 : 55} width={w} height={12} fill={e.from === 'central' ? 'var(--series-7)' : 'var(--series-1)'} opacity={past ? 1 : 0.3} />;
          }),
        )}
        <line x1={X(t)} x2={X(t)} y1={30} y2={78} stroke="var(--accent)" strokeWidth={2} />
        <text x={40} y={122} fontSize={12} fill="var(--text-2)">Zoom: one connection event ({l.packets_per_event} exchanges, {fmtNum(evEnd / 1000)} ms)</text>
        <text x={36} y={152} fontSize={10.5} fill="var(--muted)" textAnchor="end">phone</text>
        <text x={36} y={190} fontSize={10.5} fill="var(--muted)" textAnchor="end">glasses</text>
        {l.event.map((e, i) => {
          const x = ZX(e.t_us);
          const w = Math.max(2, ZX(e.t_us + e.dur_us) - x);
          const y = e.from === 'central' ? 140 : 178;
          const tIn = t % ci;
          const on = tIn >= e.t_us;
          return (
            <g key={i}>
              <rect x={x} y={y} width={w} height={18} rx={3} fill={e.from === 'central' ? 'var(--series-7)' : 'var(--series-1)'} opacity={on ? 1 : 0.3} />
              {e.data && w > 40 && <text x={x + w / 2} y={y + 13} fontSize={10} fill="#fff" textAnchor="middle">{e.bytes} B</text>}
              {i + 1 < l.event.length && <text x={ZX(e.t_us + e.dur_us + 75)} y={e.from === 'central' ? 170 : 172} fontSize={9} fill="var(--muted)" textAnchor="middle">150 µs</text>}
            </g>
          );
        })}
        <g fontSize={11} fill="var(--text-2)">
          <rect x={40} y={222} width={12} height={10} fill="var(--series-7)" /><text x={58} y={231}>central (phone) — empty poll, {fmtNum(l.empty_packet_us)} µs</text>
          <rect x={360} y={222} width={12} height={10} fill="var(--series-1)" /><text x={378} y={231}>peripheral (glasses) — data, {fmtNum(l.data_packet_us)} µs</text>
        </g>
      </svg>
    </div>
  );
}

const MSC: { from: 'phone' | 'glasses'; label: string; note?: string }[] = [
  { from: 'glasses', label: 'ADV_IND  (HUD service UUID)', note: 'advertising every 20 ms, then 152.5 ms' },
  { from: 'phone', label: 'CONNECT_IND', note: 'interval, latency, timeout' },
  { from: 'phone', label: 'Exchange MTU (247)' },
  { from: 'glasses', label: 'Exchange MTU response' },
  { from: 'phone', label: 'Write CCCD 0x2902 on Audio, Gesture, Status', note: 'enable notifications' },
  { from: 'phone', label: 'Write w/o resp: Navigation {arrow 45°, 350 m, "Gaffey St"}' },
  { from: 'glasses', label: 'Notify: Gesture {tap, screen 2}', note: 'user switches to captions' },
  { from: 'glasses', label: 'Notify: Status {caption, full, fast}', note: 'policy keeps the fast interval' },
  { from: 'glasses', label: 'Notify: Audio {seq, ADPCM 160 B} × 50/s' },
  { from: 'phone', label: 'Write w/o resp: Captions {seq, final, "hello…"}', note: 'after speech recognition' },
  { from: 'glasses', label: 'L2CAP connection-parameter update request (300 ms)', note: 'context → idle: policy picks slow' },
  { from: 'phone', label: 'Connection update (the phone decides)' },
];

function SequenceChart() {
  const reduced = useReducedMotion();
  const [playing, setPlaying] = useState(!reduced);
  const [k, setK] = useState(reduced ? MSC.length : 0);
  useFrame(playing, (dt) => setK((v) => {
    const nv = v + dt / 900;
    if (nv > MSC.length + 2) return 0;
    return nv;
  }));
  const W = 860;
  const xp = 150;
  const xg = W - 150;
  const H = 70 + MSC.length * 34;
  return (
    <div className="pf-msc">
      <PlayControls playing={playing} reduced={reduced} onToggle={() => setPlaying(!playing)} onReplay={() => { setK(0); setPlaying(true); }} />
      <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Phone app and glasses message sequence chart" style={{ marginTop: 8 }}>
        <defs>
          <marker id="pf-msc-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
            <path d="M0,0 L10,5 L0,10 Z" fill="var(--text-2)" />
          </marker>
        </defs>
        {[[xp, 'Phone app (central)'], [xg, 'Iris glasses (peripheral)']].map(([x, l]) => (
          <g key={l as string}>
            <rect x={(x as number) - 90} y={8} width={180} height={30} rx={8} fill="var(--surface-2)" stroke="var(--border-strong)" />
            <text x={x as number} y={28} fontSize={12.5} fontWeight={700} fill="var(--text)" textAnchor="middle">{l as string}</text>
            <line x1={x as number} x2={x as number} y1={38} y2={H - 6} stroke="var(--border-strong)" strokeDasharray="4 4" />
          </g>
        ))}
        {MSC.map((m, i) => {
          const y = 62 + i * 34;
          const f = Math.max(0, Math.min(1, k - i));
          if (f <= 0) return null;
          const [a, b] = m.from === 'phone' ? [xp, xg] : [xg, xp];
          const xe = a + (b - a) * f;
          return (
            <g key={i}>
              <line x1={a} x2={xe} y1={y} y2={y} stroke={m.from === 'phone' ? 'var(--series-7)' : 'var(--series-1)'} strokeWidth={1.8} markerEnd="url(#pf-msc-arrow)" />
              <text x={(xp + xg) / 2} y={y - 5} fontSize={11.5} fill="var(--text)" textAnchor="middle" opacity={f}>{m.label}</text>
              {m.note && <text x={m.from === 'phone' ? xp - 12 : xg + 12} y={y + 4} fontSize={10} fill="var(--muted)" textAnchor={m.from === 'phone' ? 'end' : 'start'} opacity={f}>{m.note}</text>}
            </g>
          );
        })}
      </svg>
    </div>
  );
}

function TradePlots({ sw, l }: { sw: Sweep; l: LinkResult }) {
  const x = sw.interval_ms;
  const streamX: number[] = [];
  const streamY: number[] = [];
  sw.stream_mw.forEach((v, i) => {
    if (v !== null) {
      streamX.push(x[i]);
      streamY.push(v);
    }
  });
  const marks = [
    { value: l.params.conn_interval_ms, label: 'fast' },
    { value: l.params.slow_interval_ms, label: 'slow' },
  ];
  return (
    <div className="grid grid-3">
      <div>
        <div className="pf-plot-title">Throughput vs connection interval</div>
        <Plot height={220} xLog xLabel="interval (ms)" yLabel="kbit/s" yLog series={[{ name: 'application throughput', x, y: sw.throughput_kbps }]} hLines={[{ value: l.audio_demand_kbps, label: `audio ${fmtNum(l.audio_demand_kbps)} kbit/s` }]} vLines={marks} />
      </div>
      <div>
        <div className="pf-plot-title">Radio power vs connection interval</div>
        <Plot
          height={220}
          xLog
          yLog
          xLabel="interval (ms)"
          yLabel="mW (radio, average)"
          series={[
            { name: 'idle link (empty packets)', x, y: sw.idle_mw },
            { name: 'streaming audio', x: streamX, y: streamY },
          ]}
          vLines={[...marks, { value: sw.max_stream_interval_ms, label: `audio ≤ ${fmtNum(sw.max_stream_interval_ms)} ms` }]}
          legend
        />
      </div>
      <div>
        <div className="pf-plot-title">Caption link latency vs interval</div>
        <Plot height={220} xLog xLabel="interval (ms)" yLabel="ms" series={[{ name: 'frame + up + down (average)', x, y: sw.caption_latency_ms }]} hLines={[{ value: 150, label: '150 ms' }]} vLines={marks} />
      </div>
    </div>
  );
}

function AudioBudget({ l }: { l: LinkResult }) {
  const rows = [
    { label: '16-bit PCM', kbps: l.pcm_kbps, note: `${fmtNum(l.params.audio_rate_hz / 1000)} kHz × 16 bit` },
    { label: `IMA ADPCM (${fmtNum(l.compression_ratio)}:1)`, kbps: (l.params.audio_rate_hz * l.params.audio_bits) / 1000, note: `${l.params.audio_bits} bit/sample` },
    { label: '+ 6 B header per frame', kbps: l.audio_demand_kbps, note: `${l.audio_bytes_per_frame} B per ${fmtNum(l.params.audio_frame_ms)} ms, ${l.audio_notifications_per_frame} notification(s)` },
    { label: 'link capacity (fast interval)', kbps: l.throughput_kbps, note: `${l.packets_per_event} packets/event` },
  ];
  const max = Math.max(...rows.map((r) => r.kbps));
  return (
    <div>
      {rows.map((r, i) => (
        <div key={r.label} className="pf-abar">
          <span>{r.label}</span>
          <div><div style={{ width: `${(r.kbps / max) * 100}%`, background: i === 3 ? 'var(--series-2)' : i === 0 ? 'var(--surface-3)' : 'var(--series-1)' }} /></div>
          <span className="mono">{fmtNum(r.kbps)} kbit/s</span>
          <small className="muted">{r.note}</small>
        </div>
      ))}
      <div className="row" style={{ marginTop: 8 }}>
        <Chip tone={l.audio_util_pct <= 50 ? 'pass' : l.audio_util_pct <= 100 ? 'warn' : 'fail'}>
          {l.audio_util_pct <= 50 ? '✓' : l.audio_util_pct <= 100 ? '!' : '✕'} audio uses {fmtNum(l.audio_util_pct)} % of the link
        </Chip>
        <span className="muted small">{l.audio_frame_samples} samples → {l.audio_frame_payload_bytes} B of ADPCM per frame</span>
      </div>
    </div>
  );
}

export function BleTab() {
  const project = useProject((s) => s.project);
  const [link, setLink] = useState<LinkParams>(DEFAULT_LINK);
  const set = <K extends keyof LinkParams>(k: K, v: LinkParams[K]) => setLink((l) => ({ ...l, [k]: v }));
  const res = useCompute<BleOut>('firmware.ble', { project, link });
  const b = res.data;
  const l = b?.link;
  return (
    <div className="stack">
      <ErrorBox error={res.error} />
      <div className="pf-main">
        <Card title="Link timing" subtitle="Air-time arithmetic from the Bluetooth Core Specification; packets per event is set by the phone (typical values, not a guarantee).">
          {l && <Timing l={l} />}
        </Card>
        <Card title="Link parameters">
          <Slider label="Connection interval" value={link.conn_interval_ms} min={7.5} max={400} step={1.25} unit="ms" onChange={(v) => set('conn_interval_ms', v)} />
          <Slider label="ATT MTU" value={link.mtu} min={23} max={517} step={1} unit="B" onChange={(v) => set('mtu', Math.round(v))} />
          <Slider label="Packets per event (phone)" value={link.max_packets_per_event} min={1} max={20} step={1} onChange={(v) => set('max_packets_per_event', Math.round(v))} />
          <Slider label="Peripheral latency" value={link.peripheral_latency} min={0} max={10} step={1} unit="events" onChange={(v) => set('peripheral_latency', Math.round(v))} />
          <Slider label="Speech recogniser on the phone (assumption)" value={link.asr_ms} min={50} max={1500} step={10} unit="ms" onChange={(v) => set('asr_ms', v)} />
          <div className="row" style={{ marginBottom: 10 }}>
            <Seg<'1M' | '2M'> ariaLabel="PHY" value={link.phy} onChange={(v) => set('phy', v)} options={[{ value: '1M', label: 'LE 1M' }, { value: '2M', label: 'LE 2M' }]} />
          </div>
          <Toggle label="Data Length Extension (251-byte packets)" value={link.dle} onChange={(v) => set('dle', v)} />
          <Toggle label="Encrypted link (+4-byte MIC per data packet)" value={link.encrypted} onChange={(v) => set('encrypted', v)} />
          <button className="btn small" onClick={() => setLink(DEFAULT_LINK)}>Reset to Iris defaults</button>
        </Card>
      </div>
      {l && (
        <>
          <div className="grid grid-4">
            <Stat label="Application throughput" value={fmtNum(l.throughput_kbps)} unit="kbit/s" note={`${l.packets_per_event} × ${fmtNum(l.att_data_per_notification / l.ll_packets_per_notification)} B per ${fmtNum(l.params.conn_interval_ms)} ms · ${fmtNum(l.efficiency * 100)} % of the PHY rate`} />
            <Stat label="Audio uplink need" value={fmtNum(l.audio_demand_kbps)} unit="kbit/s" tone={l.audio_margin >= 1.5 ? 'pass' : l.audio_margin >= 1 ? 'warn' : 'fail'} note={l.audio_margin >= 1 ? `✓ ${fmtNum(l.audio_util_pct)} % of the link (×${fmtNum(l.audio_margin)} margin)` : `✕ link too slow (×${fmtNum(l.audio_margin)})`} />
            <Stat label="Caption link latency" value={fmtNum(l.caption_link_latency_ms)} unit="ms" note={`frame 20 + up ${fmtNum(l.audio_uplink_avg_ms)} + down ${fmtNum(l.caption_downlink_avg_ms)} (worst down ${fmtNum(l.caption_downlink_worst_ms)})`} />
            <Stat label="Speech → caption" value={fmtNum(l.caption_end_to_end_ms)} unit="ms" note={`incl. assumed ${fmtNum(l.params.asr_ms)} ms recogniser`} />
          </div>
          <Card title="Throughput maths">
            <Formula>{`T_packet = (preamble ${l.bit_us === 0.5 ? 2 : 1} + access addr 4 + header 2 + payload ≤ ${l.ll_payload_max}${l.params.encrypted ? ' + MIC 4' : ''} + CRC 3) B × 8 × ${l.bit_us} µs = ${fmtNum(l.data_packet_us)} µs
exchange = T_data + 150 µs + T_empty (${fmtNum(l.empty_packet_us)} µs) + 150 µs = ${fmtNum(l.exchange_us)} µs → ${l.packets_time_limited} fit in the interval, phone allows ${l.params.max_packets_per_event}
throughput = ${l.packets_per_event} × ${fmtNum(l.att_data_per_notification / l.ll_packets_per_notification)} B × 8 / ${fmtNum(l.params.conn_interval_ms)} ms = ${fmtNum(l.throughput_kbps)} kbit/s   (slow interval ${fmtNum(l.params.slow_interval_ms)} ms: ${fmtNum(l.slow_throughput_kbps)} kbit/s)`}</Formula>
            <ul className="small muted" style={{ margin: 0, paddingLeft: 18 }}>{l.notes.map((n) => <li key={n}>{n}</li>)}</ul>
          </Card>
          {b && (
            <Card title="Connection interval trade-off" subtitle={`Radio power model: nRF52840 typical currents (TX ${fmtNum(b.sweep.model.tx_ma)} mA at 0 dBm, RX ${fmtNum(b.sweep.model.rx_ma)} mA, DC/DC at ${fmtNum(b.sweep.model.supply_v)} V) plus ${fmtNum(b.sweep.model.pre_event_us)} µs wake-up and ${fmtNum(b.sweep.model.cpu_us)} µs stack time per event — assumptions to replace with a power-profiler measurement.`}>
              <TradePlots sw={b.sweep} l={l} />
              <div className="grid grid-3" style={{ marginTop: 10 }}>
                <Stat label="Idle link, fast interval" value={fmtNum(b.sweep.idle_fast_mw * 1000)} unit="µW" note={`${fmtNum(l.params.conn_interval_ms)} ms`} />
                <Stat label="Idle link, slow interval" value={fmtNum(b.sweep.idle_slow_mw * 1000)} unit="µW" note={`${fmtNum(l.params.slow_interval_ms)} ms — what the policy picks when idle`} />
                <Stat label="Streaming audio" value={b.sweep.stream_fast_mw !== null ? fmtNum(b.sweep.stream_fast_mw) : '—'} unit="mW" tone={b.sweep.stream_fast_mw !== null ? 'pass' : 'fail'} note={b.sweep.stream_fast_mw !== null ? `radio average at ${fmtNum(l.params.conn_interval_ms)} ms` : '✕ this interval cannot carry the audio'} />
              </div>
              <Formula>{`E_event = V·[I_rx·(t_wake + n·(t_poll + 150 µs)) + I_tx·n·(t_packet + 150 µs) + I_cpu·t_stack]
P_avg = V·I_sleep + E_event / T_interval        (n = packets the event must carry)`}</Formula>
            </Card>
          )}
          <div className="grid grid-2">
            <Card title="Audio streaming budget" subtitle="Microphone audio to the phone's speech recogniser, compressed 4:1 with IMA ADPCM.">
              <AudioBudget l={l} />
            </Card>
            {b && (
              <Card title="Custom UUIDs — project-specific" subtitle="Generated deterministically, so the firmware and a phone app derive the same values from the project.">
                <div className="pf-cmd">{`service base  ${b.uuid_base}\nname          "${b.uuid_name}"\nmethod        UUID version 5 (SHA-1, RFC 4122 URL name space),\n              hex digits 5–8 cleared for 16-bit characteristic aliases`}</div>
                <div className="row" style={{ marginTop: 8 }}>
                  <Chip tone="warn">! Not registered with the Bluetooth SIG</Chip>
                  <span className="muted small">Rename the project and every custom UUID changes; standard services keep their assigned 16-bit numbers.</span>
                </div>
              </Card>
            )}
          </div>
        </>
      )}
      {b && (
        <Card title="GATT table" subtitle="One custom service for the HUD plus two Bluetooth SIG standard services (assigned numbers)." actions={
          <div className="row">
            <DownloadButton small filename="iris_ble_protocol.h" content={b.c_header} label="C header" />
            <DownloadButton small filename="iris-gatt.md" content={b.markdown} label="Markdown spec" />
            <DownloadButton small filename="iris-gatt.json" content={b.json_spec} label="JSON spec" />
          </div>
        }>
          <div className="table-wrap">
            <table className="table">
              <thead><tr><th>Service / characteristic</th><th>UUID</th><th>Properties</th><th>Direction</th><th>Payload</th></tr></thead>
              <tbody>
                {b.services.flatMap((s) => [
                  <tr key={s.uuid}><td colSpan={5} style={{ background: 'var(--surface-2)', fontWeight: 700 }}>{s.name} <span className="pf-uuid">{s.uuid}</span> {s.standard && <Chip tone="info">Bluetooth SIG</Chip>}</td></tr>,
                  ...s.characteristics.map((c) => (
                    <tr key={s.uuid + c.uuid}>
                      <td><b>{c.name}</b><div className="muted small">{c.description}</div></td>
                      <td className="pf-uuid">{c.uuid}</td>
                      <td><div className="pf-props">{c.properties.map((p) => <Chip key={p}>{p}</Chip>)}</div></td>
                      <td className="small">{c.direction}</td>
                      <td className="small">
                        {c.fields.length > 0 ? (
                          <>
                            <span className="mono">{c.header_bytes} B:</span> {c.fields.map((f) => <span key={f.name} className="mono" title={f.desc}>{f.ctype.replace('_t', '')} {f.name}; </span>)}
                            {c.trailing && <div className="muted">+ {c.trailing}</div>}
                          </>
                        ) : (
                          <span className="muted">{c.trailing}</span>
                        )}
                      </td>
                    </tr>
                  )),
                ])}
              </tbody>
            </table>
          </div>
        </Card>
      )}
      <Card title="Phone app ↔ glasses sequence" subtitle="What goes over the air in a typical session (the phone is the central, the glasses the peripheral).">
        <SequenceChart />
      </Card>
      {b && (
        <div className="grid grid-2">
          <Code code={b.c_header} filename="iris_ble_protocol.h" maxHeight={320} />
          <div className="stack">
            <Explainer title="GATT in one paragraph" refs={[{ label: 'Bluetooth SIG, Bluetooth Core Specification v5.x, Vol 3 Part G (GATT)' }, { label: 'Bluetooth SIG Assigned Numbers', href: 'https://www.bluetooth.com/specifications/assigned-numbers/' }]}>
              A peripheral exposes <b>services</b> (groups) of <b>characteristics</b> (values with properties: read, write, write without response, notify). The phone writes small packed structs to characteristics; the glasses <b>notify</b> the phone after it enables notifications by writing 0x0001 to the Client Characteristic Configuration descriptor (0x2902). Standard services use 16-bit assigned numbers (Battery 0x180F, Device Information 0x180A); custom ones need a 128-bit UUID — here a name-based (version 5) UUID derived from the project name, so it is reproducible and clearly project-specific.
            </Explainer>
            <Explainer title="Latency for live captions" refs={[{ label: 'C. Gomez, J. Oller, J. Paradells, "Overview and evaluation of Bluetooth Low Energy", Sensors 12(9), 2012' }]}>
              A notification waits for the next connection event — on average half an interval — and a caption makes the round trip twice (audio up, text down) plus the recogniser's time. Short intervals cut latency but keep the radio busy; the RL Lab's policy switches to the slow interval whenever captions are off.
              <Formula>{'latency ≈ t_frame + T_ci/2 (up) + t_ASR + T_ci/2 (down) + air time'}</Formula>
            </Explainer>
          </div>
        </div>
      )}
    </div>
  );
}
