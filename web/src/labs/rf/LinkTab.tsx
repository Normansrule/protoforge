// Link budget: Bluetooth Low Energy (BLE) from the glasses to a phone —
// waterfall, range per PHY, a top-down range map and received power vs distance.
import { useEffect, useMemo, useRef, useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useElementWidth } from '../../components/useElementWidth';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { Direction, IrisOut, LinkDesign, LinkOut } from './types';
import { PHY_LABEL, RF_DEFAULTS, fmt, prefersReducedMotion, toCsv, useIrisRf, useRaf } from './util';
import { REFS } from './refs';

const ENVS = [
  { n: 2.0, label: 'Free space / open line of sight', note: 'n = 2 (Friis)' },
  { n: 2.5, label: 'Indoor, mostly line of sight', note: 'n ≈ 2–3 (assumed default)' },
  { n: 3.0, label: 'Indoor, through people and furniture', note: 'n ≈ 3' },
  { n: 3.5, label: 'Through walls', note: 'n ≈ 3.5–4' },
];
const PHY_COLORS: Record<string, string> = { '1m': 'var(--series-1)', '2m': 'var(--series-2)', s2: 'var(--series-3)', s8: 'var(--series-4)' };

export function LinkTab() {
  const project = useProject((s) => s.project);
  const [rf, patchRf] = useIrisRf();
  const iris = useCompute<IrisOut>('rf.iris', { project });
  const [l, setL] = useState<LinkDesign>(rf.link);
  const set = (p: Partial<LinkDesign>) => setL((x) => ({ ...x, ...p }));
  const gain = iris.data?.glasses_gain_dbi ?? 1.76 + 10 * Math.log10(rf.rad_efficiency);
  const matchLoss = iris.data?.match_loss_db ?? 0;
  const input = useMemo(() => ({ ...l, f_ghz: 2.44, glasses_gain_dbi: gain, match_loss_db: matchLoss }), [l, gain, matchLoss]);
  const res = useCompute<LinkOut>('rf.link', input);
  const o = res.data;
  const [dir, setDir] = useState<'limiting' | 'forward' | 'reverse'>('limiting');
  const shown: Direction | null = o ? (dir === 'forward' ? o.forward : dir === 'reverse' ? o.reverse : o.forward.margin_db <= o.reverse.margin_db ? o.forward : o.reverse) : null;
  const differs = JSON.stringify(l) !== JSON.stringify(rf.link);
  return (
    <div className="stack">
      {o && (
        <div className="grid grid-4 rf-stats">
          <Stat label={`Link margin at ${fmt(l.distance_m, 1)} m`} value={fmt(o.margin_db, 1)} unit="dB" tone={o.margin_db >= 0 ? 'pass' : 'fail'} note={`${o.margin_db >= 0 ? '✓' : '✕'} ${o.phy}, limited by ${o.limiting.toLowerCase()}`} />
          <Stat label="Range (0 dB margin)" value={fmt(o.range_m, 1)} unit="m" note={`with ${fmt(o.fade_db, 1)} dB fade margin (${fmt(100 * l.availability, 1)} %)`} />
          <Stat label="Path loss" value={fmt(o.path_loss_db, 1)} unit="dB" note={`Friis ${fmt(o.fspl_1m_db, 1)} dB at 1 m, then n = ${fmt(l.path_exponent, 1)}`} />
          <Stat label="Glasses antenna" value={fmt(gain, 1)} unit="dBi" note={`− ${fmt(matchLoss, 1)} dB match loss (from Matching)`} />
        </div>
      )}
      <div className="rf-split">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card
            title="Budget waterfall"
            subtitle={shown ? `${shown.name}: ${fmt(shown.rx_dbm, 1)} dBm received vs ${fmt(shown.sens_dbm, 1)} dBm sensitivity (typical) − ${fmt(shown.fade_db, 1)} dB fade margin` : ''}
            actions={
              <div className="rf-seg" role="group" aria-label="Direction">
                {(['limiting', 'forward', 'reverse'] as const).map((d) => (
                  <button key={d} className={`rf-seg-btn ${dir === d ? 'active' : ''}`} onClick={() => setDir(d)} aria-pressed={dir === d}>
                    {d === 'limiting' ? 'limiting' : d === 'forward' ? 'glasses → phone' : 'phone → glasses'}
                  </button>
                ))}
              </div>
            }
          >
            <ErrorBox error={res.error} />
            {shown && <Waterfall d={shown} />}
          </Card>
          {o && <RangeMap o={o} distance={l.distance_m} phy={l.phy} onDistance={(d) => set({ distance_m: d })} />}
          {o && (
            <Card title="Received power vs distance" subtitle="Both directions; dashed: sensitivity + fade margin for the selected PHY">
              <Plot
                series={[
                  { name: 'glasses → phone', x: o.dist_m, y: o.rx_fwd_dbm },
                  { name: 'phone → glasses', x: o.dist_m, y: o.rx_rev_dbm },
                ]}
                height={240}
                xLog
                xLabel="distance (m)"
                yLabel="P_rx (dBm)"
                hLines={
                  // one label when the two thresholds would print on top of each other
                  Math.abs(o.forward.sens_dbm - o.reverse.sens_dbm) < 6
                    ? [
                        { value: Math.max(o.forward.sens_dbm, o.reverse.sens_dbm) + o.fade_db, label: 'phone / glasses sens. + fade' },
                        { value: Math.min(o.forward.sens_dbm, o.reverse.sens_dbm) + o.fade_db, label: '' },
                      ]
                    : [
                        { value: o.forward.sens_dbm + o.fade_db, label: 'phone sens. + fade' },
                        { value: o.reverse.sens_dbm + o.fade_db, label: 'glasses sens. + fade' },
                      ]
                }
                vLines={[{ value: l.distance_m, label: `${fmt(l.distance_m, 1)} m` }]}
                legend
              />
              <DownloadButton small filename="ble-link-vs-distance.csv" content={() => toCsv([['distance_m', 'rx_glasses_to_phone_dbm', 'rx_phone_to_glasses_dbm'], ...o.dist_m.map((d, k) => [d, o.rx_fwd_dbm[k], o.rx_rev_dbm[k]])])} label="CSV" />
            </Card>
          )}
        </div>
        <div className="stack rf-sticky">
          <Card title="Radio">
            <div className="rf-seg" role="group" aria-label="PHY" style={{ marginBottom: 10 }}>
              {Object.keys(PHY_LABEL).map((p) => (
                <button key={p} className={`rf-seg-btn ${l.phy === p ? 'active' : ''}`} onClick={() => set({ phy: p })} aria-pressed={l.phy === p}>{PHY_LABEL[p]}</button>
              ))}
            </div>
            <Slider label="Phone distance" value={l.distance_m} min={0.5} max={100} log unit="m" onChange={(v) => set({ distance_m: Math.round(v * 10) / 10 })} />
            <Slider label="Glasses transmit power" value={l.tx_dbm} min={-20} max={8} step={1} unit="dBm" onChange={(v) => set({ tx_dbm: v })} />
            <Slider label="Phone transmit power (assumed)" value={l.phone_tx_dbm} min={-10} max={10} step={1} unit="dBm" onChange={(v) => set({ phone_tx_dbm: v })} />
            <Slider label="Phone antenna gain (assumed)" value={l.phone_gain_dbi} min={-8} max={3} step={0.5} unit="dBi" onChange={(v) => set({ phone_gain_dbi: v })} />
            <Slider label="Phone sensitivity on LE 1M (assumed)" value={l.phone_sens_1m_dbm} min={-100} max={-70} step={1} unit="dBm" onChange={(v) => set({ phone_sens_1m_dbm: v })} />
          </Card>
          <Card title="Channel">
            <div className="stack" style={{ gap: 6, marginBottom: 10 }}>
              {ENVS.map((e) => (
                <button key={e.n} className={`rf-preset ${Math.abs(l.path_exponent - e.n) < 1e-6 ? 'active' : ''}`} onClick={() => set({ path_exponent: e.n })} aria-pressed={Math.abs(l.path_exponent - e.n) < 1e-6}>
                  <b>{e.label}</b>
                  <span className="muted small">{e.note}</span>
                </button>
              ))}
            </div>
            <Slider label="Path-loss exponent n" value={l.path_exponent} min={1.6} max={4.5} step={0.1} onChange={(v) => set({ path_exponent: v })} />
            <Slider label="Head loss (glasses on the head)" value={l.head_loss_db} min={0} max={20} step={0.5} unit="dB" onChange={(v) => set({ head_loss_db: v })} hint="Typical body-worn 2.4 GHz losses are several dB up to 10+ dB in shadowed directions; see Fields → head for a 2-D estimate" />
            <Slider label="Body loss (phone in hand / pocket)" value={l.body_loss_db} min={0} max={20} step={0.5} unit="dB" onChange={(v) => set({ body_loss_db: v })} />
            <div className="rf-seg" role="group" aria-label="Availability" style={{ marginBottom: 6 }}>
              {[0.9, 0.99, 0.999].map((a) => (
                <button key={a} className={`rf-seg-btn ${l.availability === a ? 'active' : ''}`} onClick={() => set({ availability: a })} aria-pressed={l.availability === a}>{a * 100} %</button>
              ))}
            </div>
            <div className="small muted">Rayleigh fade margin for this availability (indoor multipath).</div>
          </Card>
          <Card title="On Iris" subtitle="These assumptions drive rf.link_margin_db and rf.range_m">
            <div className="stack" style={{ gap: 8 }}>
              {iris.data && (
                <>
                  <Chip tone={iris.data.link.margin_db >= 0 ? 'pass' : 'fail'}>{iris.data.link.margin_db >= 0 ? '✓' : '✕'} rf.link_margin_db = {fmt(iris.data.link.margin_db, 1)} dB</Chip>
                  <Chip>rf.range_m = {fmt(iris.data.link.range_m, 1)} m</Chip>
                </>
              )}
              <div className="row">
                <button className="btn small primary" disabled={!differs} onClick={() => patchRf({ link: l })}>Save these assumptions to Iris</button>
                <button className="btn small" disabled={!differs} onClick={() => setL(rf.link)}>Revert</button>
                <button className="btn small" onClick={() => setL(RF_DEFAULTS.link)}>Defaults</button>
              </div>
            </div>
          </Card>
        </div>
      </div>
      {o && (
        <Card title="Range by PHY" subtitle="Slower coded PHYs trade data rate for sensitivity (forward error correction)">
          <div className="rf-bars">
            {o.phys.map((p) => {
              const max = Math.max(...o.phys.map((x) => x.range_m));
              return (
                <div key={p.id} className="rf-bar-row">
                  <span className="small"><b>{p.name}</b> <span className="muted">{p.rate_kbps >= 1000 ? `${p.rate_kbps / 1000} Mb/s` : `${p.rate_kbps} kb/s`}</span></span>
                  <div className="rf-bar"><div style={{ width: `${(100 * p.range_m) / max}%`, background: PHY_COLORS[p.id] }} /></div>
                  <span className="mono small tnum">{fmt(p.range_m, 1)} m · {p.margin_db >= 0 ? '+' : ''}{fmt(p.margin_db, 1)} dB</span>
                </div>
              );
            })}
          </div>
          <ul className="small muted rf-list" style={{ marginTop: 10 }}>{o.notes.map((n, k) => <li key={k}>{n}</li>)}</ul>
        </Card>
      )}
      <div className="grid grid-2">
        <Explainer title="Friis and the log-distance model" refs={[REFS.friis, REFS.rappaport, REFS.pozar]}>
          In free space the power reaching an antenna falls with the square of distance:
          <Formula>{'P_rx = P_tx + G_tx + G_rx − 20·log₁₀(4πd/λ)    (dBm, dBi, dB)'}</Formula>
          At 2.44 GHz that is 40.2 dB at 1 m. Indoors, clutter steepens the slope: PL(d) = PL(1 m) + 10·n·log₁₀(d/1 m) with n ≈ 2–4. A head next to the antenna absorbs and shadows several dB; a phone in a pocket loses more.
        </Explainer>
        <Explainer title="Sensitivity, fading and the four BLE PHYs" refs={[REFS.bt, REFS.rappaport]}>
          Margin = P_rx − sensitivity − fade margin. Multipath makes the received power Rayleigh-distributed around its mean, so a link up 90 % of the time needs
          <Formula>{'F = −10·log₁₀(−ln(1 − p_out)) ≈ 9.8 dB (90 %), 20 dB (99 %)'}</Formula>
          LE 1M and 2M are uncoded; LE Coded adds forward error correction (S = 2 at 500 kb/s, S = 8 at 125 kb/s), buying roughly 4 and 8 dB of sensitivity — about 1.5× and 2× the range in free space. The specification only requires −70 dBm on the uncoded PHYs; real chips are ~25 dB better (values here are typical, check your data sheet).
        </Explainer>
      </div>
    </div>
  );
}

function Waterfall({ d }: { d: Direction }) {
  const [wrapRef, width] = useElementWidth<HTMLDivElement>();
  const W = Math.max(320, width);
  const rows = d.steps;
  const all = [...rows.map((s) => s.level_dbm), ...rows.map((s) => s.level_dbm - s.delta_db), d.sens_dbm, d.sens_dbm + d.fade_db, 10];
  const hi = Math.ceil(Math.max(...all) / 10) * 10;
  const lo = Math.floor(Math.min(...all, d.sens_dbm - 5) / 10) * 10;
  const H = 280;
  // wider left margin on phones so the first rotated label is not clipped
  const m = { l: W < 520 ? 66 : 44, r: 10, t: 14, b: 76 };
  const n = rows.length + 1;
  const bw = (W - m.l - m.r) / n;
  const y = (v: number) => m.t + ((hi - v) / (hi - lo)) * (H - m.t - m.b);
  const ticks: number[] = [];
  for (let v = lo; v <= hi; v += 10) ticks.push(v);
  const key = rows.map((r) => r.delta_db.toFixed(2)).join(',');
  const [anim, setAnim] = useState(0);
  const reduced = prefersReducedMotion();
  useEffect(() => setAnim((a) => a + 1), [key]);
  return (
    <div ref={wrapRef}>
      <svg viewBox={`0 0 ${W} ${H}`} width="100%" className="rf-wf" role="img" aria-label={`Link budget waterfall, received ${d.rx_dbm.toFixed(1)} dBm, margin ${d.margin_db.toFixed(1)} dB`}>
        {ticks.map((t) => (
          <g key={t}>
            <line x1={m.l} x2={W - m.r} y1={y(t)} y2={y(t)} className="rf-wf-grid" />
            <text x={m.l - 6} y={y(t) + 4} textAnchor="end" className="rf-wf-tick">{t}</text>
          </g>
        ))}
        <text x={10} y={m.t + 4} className="rf-wf-tick" transform={`rotate(-90 10 ${m.t + 60})`}>dBm</text>
        <rect x={m.l} y={y(d.sens_dbm + d.fade_db)} width={W - m.l - m.r} height={y(d.sens_dbm) - y(d.sens_dbm + d.fade_db)} className="rf-wf-fade" />
        <line x1={m.l} x2={W - m.r} y1={y(d.sens_dbm)} y2={y(d.sens_dbm)} className="rf-wf-sens" />
        <text x={W - m.r - 4} y={y(d.sens_dbm) + 13} textAnchor="end" className="rf-wf-tick">sensitivity {d.sens_dbm.toFixed(0)} dBm · fade band above</text>
        {rows.map((s, k) => {
          const from = k === 0 ? 0 : rows[k - 1].level_dbm;
          const to = s.level_dbm;
          const top = Math.min(from, to);
          const bot = Math.max(from, to);
          const isGain = k === 0 ? true : s.delta_db >= 0;
          const x = m.l + k * bw + bw * 0.14;
          return (
            <g key={`${anim}-${k}`} className={reduced ? '' : 'rf-wf-in'} style={{ animationDelay: `${k * 90}ms` }}>
              <rect x={x} y={y(bot)} width={bw * 0.72} height={Math.max(2, y(top) - y(bot))} rx={3} className={k === 0 ? 'rf-wf-tx' : isGain ? 'rf-wf-gain' : 'rf-wf-loss'} />
              {k > 0 && <line x1={x - bw * 0.28} x2={x} y1={y(from)} y2={y(from)} className="rf-wf-link" />}
              <text x={x + bw * 0.36} y={y(bot) - 4 < m.t + 8 ? y(top) + 14 : y(Math.max(from, to)) - 4} textAnchor="middle" className="rf-wf-val">
                {k === 0 ? `${s.delta_db.toFixed(0)}` : `${s.delta_db >= 0 ? '+' : ''}${s.delta_db.toFixed(1)}`}
              </text>
              <text x={x + bw * 0.36} y={H - m.b + 12} textAnchor="end" transform={`rotate(-35 ${x + bw * 0.36} ${H - m.b + 12})`} className="rf-wf-tick">{s.label.replace(/ \(.*\)/, '')}</text>
            </g>
          );
        })}
        {(() => {
          const x = m.l + rows.length * bw + bw * 0.14;
          const ok = d.margin_db >= 0;
          return (
            <g>
              <rect x={x} y={y(d.rx_dbm)} width={bw * 0.72} height={Math.max(2, y(d.sens_dbm) - y(d.rx_dbm))} rx={3} className={ok ? 'rf-wf-margin' : 'rf-wf-short'} />
              <text x={x + bw * 0.36} y={y(d.rx_dbm) - 4} textAnchor="middle" className="rf-wf-val">{d.rx_dbm.toFixed(1)}</text>
              <text x={x + bw * 0.36} y={H - m.b + 12} textAnchor="end" transform={`rotate(-35 ${x + bw * 0.36} ${H - m.b + 12})`} className="rf-wf-tick">
                {ok ? '✓' : '✕'} margin {d.margin_db.toFixed(1)} dB
              </text>
            </g>
          );
        })()}
      </svg>
      <div className="rf-legend-row small">
        <span><i className="rf-swatch rf-wf-tx" /> transmit power</span>
        <span><i className="rf-swatch rf-wf-gain" /> gain</span>
        <span><i className="rf-swatch rf-wf-loss" /> loss</span>
        <span><i className="rf-swatch rf-wf-margin" /> received − sensitivity</span>
      </div>
    </div>
  );
}

function RangeMap({ o, distance, phy, onDistance }: { o: LinkOut; distance: number; phy: string; onDistance: (d: number) => void }) {
  const [wrapRef, width] = useElementWidth<HTMLDivElement>();
  const W = Math.max(300, width);
  const H = Math.min(420, Math.max(280, W * 0.62));
  const maxR = Math.max(distance * 1.15, ...o.phys.map((p) => p.range_m)) * 1.08;
  const pxPerM = Math.min(W, H) / 2 / maxR;
  const cx = W / 2;
  const cy = H / 2;
  const reduced = prefersReducedMotion();
  const [t, setT] = useState(0);
  const [play, setPlay] = useState(!reduced);
  useRaf(play, (dt) => setT((x) => (x + dt) % 2));
  const grid = maxR > 60 ? 10 : maxR > 25 ? 5 : 1;
  const svgRef = useRef<SVGSVGElement>(null);
  const [drag, setDrag] = useState(false);
  const pick = (e: React.PointerEvent) => {
    const rect = svgRef.current!.getBoundingClientRect();
    const x = ((e.clientX - rect.left) / rect.width) * W - cx;
    const y = ((e.clientY - rect.top) / rect.height) * H - cy;
    const d = Math.hypot(x, y) / pxPerM;
    onDistance(Math.max(0.5, Math.min(100, Math.round(d * 10) / 10)));
  };
  const angle = -0.6;
  const phX = cx + distance * pxPerM * Math.cos(angle);
  const phY = cy + distance * pxPerM * Math.sin(angle);
  const cur = o.phys.find((p) => p.id === phy);
  return (
    <Card title="Range map" subtitle={`Top view, ${grid} m grid. Rings: 0 dB-margin range per PHY. Drag the phone.`} actions={<button className="btn small" onClick={() => setPlay((p) => !p)} aria-pressed={play}>{play ? '❚❚ Pause' : '▶ Packets'}</button>}>
      <div ref={wrapRef}>
        <svg
          ref={svgRef}
          viewBox={`0 0 ${W} ${H}`}
          width="100%"
          className="rf-map"
          onPointerDown={(e) => { setDrag(true); e.currentTarget.setPointerCapture?.(e.pointerId); pick(e); }}
          onPointerMove={(e) => drag && pick(e)}
          onPointerUp={() => setDrag(false)}
          role="img"
          aria-label={`Range map: phone at ${distance} m, ${cur?.name} range ${cur?.range_m.toFixed(1)} m`}
        >
          {Array.from({ length: Math.ceil(W / 2 / (grid * pxPerM)) + 1 }, (_, k) => k).flatMap((k) => [cx + k * grid * pxPerM, cx - k * grid * pxPerM]).map((x, k) => (
            <line key={`v${k}`} x1={x} x2={x} y1={0} y2={H} className="rf-map-grid" />
          ))}
          {Array.from({ length: Math.ceil(H / 2 / (grid * pxPerM)) + 1 }, (_, k) => k).flatMap((k) => [cy + k * grid * pxPerM, cy - k * grid * pxPerM]).map((y, k) => (
            <line key={`h${k}`} x1={0} x2={W} y1={y} y2={y} className="rf-map-grid" />
          ))}
          {o.phys.map((p) => (
            <g key={p.id}>
              <circle cx={cx} cy={cy} r={p.range_m * pxPerM} fill="none" stroke={PHY_COLORS[p.id]} strokeWidth={p.id === phy ? 3 : 1.5} strokeDasharray={p.id === phy ? undefined : '6 4'} />
              {(() => {
                // label at the ring's lower-right; flip to the left of the point when it would leave the map
                const label = `${p.name} ${p.range_m.toFixed(0)} m`;
                const lx = cx + p.range_m * pxPerM * 0.7071;
                const ly = Math.min(H - 6, cy + p.range_m * pxPerM * 0.7071 + 4);
                const flip = lx + 4 + label.length * 6.2 > W - 4;
                return (
                  <text x={flip ? lx - 4 : lx + 4} y={ly} textAnchor={flip ? 'end' : 'start'} className="rf-map-t">
                    {label}
                  </text>
                );
              })()}
            </g>
          ))}
          {play && [0, 1].map((k) => {
            const ph = ((t + k) % 2) / 2;
            return <circle key={k} cx={cx} cy={cy} r={ph * (cur?.range_m ?? 10) * pxPerM} fill="none" stroke="var(--accent)" strokeWidth={2} opacity={1 - ph} />;
          })}
          <line x1={cx} y1={cy} x2={phX} y2={phY} className="rf-map-ray" />
          <text x={phX + 14 + 150 > W ? phX - 14 : phX + 14} y={phY - 14} textAnchor={phX + 14 + 150 > W ? 'end' : 'start'} className="rf-map-t">{distance.toFixed(1)} m · margin {o.margin_db >= 0 ? '+' : ''}{o.margin_db.toFixed(1)} dB</text>
          <g transform={`translate(${cx},${cy})`}>
            <circle r={13} className="rf-map-head" />
            <path d="M-10,-2 h6 a4,4 0 1 0 0,0.1 M4,-2 h6" className="rf-map-glasses" />
            <text y={28} textAnchor="middle" className="rf-map-t">Iris</text>
          </g>
          <g transform={`translate(${phX},${phY})`} style={{ cursor: 'grab' }}>
            <rect x={-7} y={-12} width={14} height={24} rx={3} className={o.margin_db >= 0 ? 'rf-map-phone ok' : 'rf-map-phone bad'} />
            <text y={26} textAnchor="middle" className="rf-map-t">{o.margin_db >= 0 ? '✓ phone' : '✕ phone'}</text>
          </g>
        </svg>
      </div>
    </Card>
  );
}
