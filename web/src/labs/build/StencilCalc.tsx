// Stencil area-ratio calculator + every Iris pad checked at the chosen thickness.
import { useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat, Toggle } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { PlayButton, usePlaying, useTicker } from './anim';
import type { StencilCheck, StencilOut } from './types';

function Aperture({ l, w, th, circular, t, ar }: { l: number; w: number; th: number; circular: boolean; t: number; ar: number }) {
  // side view: squeegee pushes paste into the aperture, stencil lifts, brick stays (or sticks)
  const s = 160 / Math.max(l, w, 0.5);
  const aw = Math.max(8, (circular ? w : Math.min(l, w)) * s);
  const tt = Math.max(6, th * s * 1.5);
  const p = (t / 3) % 1;
  const sq = p < 0.4 ? p / 0.4 : 1;
  const lift = p > 0.5 ? Math.min(1, (p - 0.5) / 0.3) : 0;
  const release = Math.max(0, Math.min(1, (ar - 0.5) / 0.2));
  const cx = 180;
  const stencilY = 90 - lift * 30;
  const brickH = tt * (1 - lift * (1 - release) * 0.7);
  return (
    <svg viewBox="0 0 360 200" className="bd-anim" role="img" aria-label="Stencil printing: paste fills the aperture and must release when the stencil lifts">
      <rect x={20} y={120} width={320} height={12} fill="var(--mask)" />
      <rect x={cx - aw / 2 - 6} y={114} width={aw + 12} height={6} fill="var(--copper)" />
      <rect x={20} y={stencilY} width={cx - aw / 2 - 20} height={tt} fill="var(--muted)" />
      <rect x={cx + aw / 2} y={stencilY} width={340 - cx - aw / 2} height={tt} fill="var(--muted)" />
      {lift > 0 && <rect x={cx - aw / 2} y={stencilY} width={aw} height={(1 - release) * tt * lift * 0.7} fill="var(--text-2)" opacity={0.6} />}
      <rect x={cx - aw / 2} y={114 - brickH} width={aw} height={brickH} fill="var(--text-2)" opacity={p < 0.4 ? sq * 0.8 : 0.8} />
      <g transform={`translate(${30 + sq * 260},${stencilY - 30})`}>
        <rect x={-4} y={-10} width={8} height={40} fill="var(--accent)" transform="rotate(20)" />
      </g>
      <text x={20} y={24} className="bd-svg-small">stencil {th.toFixed(3)} mm · aperture {circular ? `Ø${w}` : `${l} × ${w}`} mm</text>
      <text x={20} y={40} className="bd-svg-small">{ar >= 0.66 ? 'paste releases cleanly (area ratio ≥ 0.66)' : 'paste tends to stay in the aperture (area ratio < 0.66)'}</text>
      <text x={20} y={180} className="bd-svg-small">pad on the PCB</text>
    </svg>
  );
}

export function StencilCalc() {
  const project = useProject((s) => s.project);
  const [i, setI] = useState({ length_mm: 0.8, width_mm: 0.2, thickness_mm: 0.12, circular: false });
  const r = useCompute<StencilOut>('build.stencil', i);
  const chk = useCompute<StencilCheck>('build.stencil_check', { project, thickness_mm: i.thickness_mm });
  const [playing, setPlaying] = usePlaying();
  const t = useTicker(playing);
  const o = r.data;
  const set = (p: Partial<typeof i>) => setI({ ...i, ...p });
  return (
    <div className="stack">
      <div className="grid grid-sidebar">
        <Card title="Aperture">
          <Toggle label="Circular aperture (BGA / via pad)" value={i.circular} onChange={(v) => set({ circular: v })} />
          {!i.circular && <Slider label="Length L" unit="mm" value={i.length_mm} min={0.1} max={3} onChange={(v) => set({ length_mm: v })} format={(v) => v.toFixed(2)} />}
          <Slider label={i.circular ? 'Diameter' : 'Width W'} unit="mm" value={i.width_mm} min={0.1} max={2} onChange={(v) => set({ width_mm: v })} format={(v) => v.toFixed(2)} />
          <Slider label="Stencil thickness t" unit="mm" value={i.thickness_mm} min={0.08} max={0.2} step={0.005} onChange={(v) => set({ thickness_mm: v })} format={(v) => v.toFixed(3)} />
          <div className="row" style={{ gap: 6 }}>
            {[0.1, 0.12, 0.13, 0.15].map((v) => <button key={v} className={`chip ${Math.abs(i.thickness_mm - v) < 1e-6 ? 'info' : ''}`} style={{ cursor: 'pointer' }} onClick={() => set({ thickness_mm: v })}>{v.toFixed(2)} mm ({Math.round(v / 0.0254)} mil)</button>)}
          </div>
          <div className="small muted" style={{ marginTop: 10 }}>Preset: the Iris BLE system-on-chip's 0.4 mm-pitch QFN pad is 0.8 × 0.2 mm.</div>
        </Card>
        <div className="stack">
          <ErrorBox error={r.error || chk.error} />
          {o && (
            <>
              <div className="grid grid-3">
                <Stat label="Area ratio" value={o.area_ratio.toFixed(3)} tone={o.area_ok ? 'pass' : 'fail'} note={o.area_ok ? '✓ ≥ 0.66 (IPC-7525)' : '✕ below 0.66'} />
                <Stat label="Aspect ratio" value={o.aspect_ratio.toFixed(2)} tone={o.aspect_ok ? 'pass' : 'fail'} note={o.aspect_ok ? '✓ ≥ 1.5' : '✕ below 1.5'} />
                <Stat label="Thickest stencil for this pad" value={o.max_thickness_mm.toFixed(3)} unit="mm" note={`paste ${(o.paste_volume_mm3 * 1000).toFixed(1)} nL per pad`} />
              </div>
              <Card title="Print and release" actions={<PlayButton playing={playing} onChange={setPlaying} />}>
                <Aperture l={i.length_mm} w={i.width_mm} th={i.thickness_mm} circular={i.circular} t={t} ar={o.area_ratio} />
              </Card>
            </>
          )}
        </div>
      </div>
      {chk.data && (
        <Card title={`Every ${project.meta.name} pad at ${chk.data.thickness_mm.toFixed(3)} mm`} subtitle={`Smallest area ratio ${chk.data.min_area_ratio.toFixed(3)} (${chk.data.worst}) · ${chk.data.failing} pad type(s) below 0.66`}>
          <div className="table-wrap">
            <table className="table">
              <thead><tr><th>Footprint</th><th>Pad</th><th style={{ textAlign: 'right' }}>Count</th><th>Parts</th><th>Area ratio</th><th style={{ textAlign: 'right' }}>Aspect</th></tr></thead>
              <tbody>
                {chk.data.rows.map((r) => (
                  <tr key={`${r.footprint}-${r.pad_w_mm}-${r.pad_h_mm}`}>
                    <td className="mono">{r.footprint}</td>
                    <td className="mono tnum">{r.pad_w_mm.toFixed(2)} × {r.pad_h_mm.toFixed(2)}</td>
                    <td className="tnum" style={{ textAlign: 'right' }}>{r.count}</td>
                    <td className="small">{r.refdes.join(', ')}</td>
                    <td>
                      <div className="bd-arbar"><div style={{ width: `${Math.min(100, (r.area_ratio / 1.5) * 100)}%` }} className={r.pass ? 'ok' : 'bad'} /><span className="bd-arbar-mark" style={{ left: `${(0.66 / 1.5) * 100}%` }} /></div>
                      <span className="tnum small">{r.pass ? '✓' : '✕'} {r.area_ratio.toFixed(2)}</span>
                    </td>
                    <td className="tnum" style={{ textAlign: 'right' }}>{r.aspect_ratio.toFixed(2)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {chk.data.notes.map((n) => <div key={n} className="small muted" style={{ marginTop: 6 }}>{n}</div>)}
          <DownloadButton filename="stencil-area-ratios.csv" content={['footprint,pad_w_mm,pad_h_mm,count,parts,area_ratio,aspect_ratio,pass', ...chk.data.rows.map((r) => `${r.footprint},${r.pad_w_mm},${r.pad_h_mm},${r.count},"${r.refdes.join(' ')}",${r.area_ratio.toFixed(4)},${r.aspect_ratio.toFixed(3)},${r.pass}`)].join('\n')} label="stencil-area-ratios.csv" small />
        </Card>
      )}
      <Explainer title="Why the area ratio matters" refs={[{ label: 'IPC-7525 — Stencil design guidelines' }]}>
        When the stencil lifts, the paste brick is pulled down by the pad (area L·W) and held back by the aperture walls (area 2·(L+W)·t). If the walls win, paste stays in the stencil and the joint starves.
        <Formula>{'area ratio = L·W / (2·(L + W)·t) ≥ 0.66\naspect ratio = W / t ≥ 1.5\ncircle: d / (4·t)'}</Formula>
        Fine-pitch parts force a thinner stencil — which then starves the big pads (USB-C shell, connectors). Step stencils or enlarged apertures on large pads solve that.
      </Explainer>
    </div>
  );
}
