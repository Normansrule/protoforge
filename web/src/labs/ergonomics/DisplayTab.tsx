// Tab 4 — Display legibility & eyebox: angular text size vs acuity, ambient
// contrast ratio of the see-through display in real scenes, and the share of
// wearers whose pupil lands inside the eyebox.
import { useMemo, useState } from 'react';
import { Card, ErrorBox, Explainer, Formula, Slider, Stat, DownloadButton } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useElementWidth } from '../../components/useElementWidth';
import { csv, f1, GROUP_LABEL, Kv, PlayButton, REFS, Seg, useClock, useErgo, usePlaying, VerdictChip } from './shared';
import type { DisplayOut, ErgoConfig, SceneRow } from './types';

export function DisplayTab() {
  const { project, cfg, setCfg } = useErgo();
  const r = useCompute<DisplayOut>('ergonomics.display', { project }, !!cfg);
  const d = r.data;
  if (!cfg) return <div className="muted small">Loading the engine…</div>;
  const eb = d?.eyebox;
  const comb = eb?.groups.find((g) => g.group === 'combined');
  return (
    <div className="stack">
      <ErrorBox error={r.error} />
      {d && eb && (
        <div className="grid grid-4 hf-stats">
          <Stat label="Caption cap height" value={f1(d.caption_arcmin)} unit="arcmin" tone={d.caption_arcmin >= 16 ? 'pass' : d.caption_arcmin >= d.acuity_arcmin ? 'warn' : 'fail'} note={`${d.caption_px} px × ${f1(d.arcmin_per_px, 2)}′/px · ${d.caption_verdict}`} />
          <Stat label="Pixel density" value={f1(d.ppd_center)} unit="px/°" note={`centre; 60 px/° ≈ 1′ per pixel (20/20)`} />
          <Stat label="Contrast in the sun" value={f1(d.bright.contrast_ratio, 2)} unit=": 1" tone={d.bright.contrast_ratio >= 3 ? 'pass' : d.bright.contrast_ratio >= 1.5 ? 'warn' : 'fail'} note={`${f1(d.bright.background_nits, 0)} cd/m² · ergonomics.outdoor_contrast`} />
          <Stat label="Eyebox coverage" value={f1(comb?.coverage_pct)} unit="%" tone={(comb?.coverage_pct ?? 0) >= 90 ? 'pass' : 'warn'} note={`${f1(eb.eyebox_mm)} mm eyebox · ergonomics.eyebox_coverage_pct`} />
        </div>
      )}
      {d && <SceneCard d={d} cfg={cfg} setCfg={setCfg} />}
      {d && (
        <div className="hf-split">
          <LegibilityCard d={d} cfg={cfg} setCfg={setCfg} />
          <ContrastTable scenes={d.scenes} d={d} />
        </div>
      )}
      {d && <EyeboxCard d={d} cfg={cfg} setCfg={setCfg} />}
      <div className="grid grid-3">
        <Explainer title="Angular size, not pixels" refs={[REFS.sanders, REFS.hfes100, REFS.wickens]}>
          What matters is the angle a glyph subtends: h_px × (arcmin per pixel). A 20/20 Snellen letter is 5 arcmin tall with 1 arcmin strokes; 20/x vision needs x/20 times more.
          Reading comfortably needs far more than threshold — about 16 arcmin minimum and 20–22 arcmin preferred for display characters (ANSI/HFES 100-2007).
          <Formula>{'θ_text = h_px · θ_px,   θ_px ≈ HFOV / pixels,   threshold = 5′ × (x / 20)'}</Formula>
        </Explainer>
        <Explainer title="Ambient contrast ratio of a see-through display" refs={[REFS.kress2013, REFS.kress2020]}>
          The display adds light to the world behind it. With image luminance L_img = L_display · T_lens · R_combiner and see-through T = (1 − R) · T_tint:
          <Formula>{'ACR = (L_img + T L_bg) / (T L_bg)'}</Formula>
          About 3 : 1 is the usual floor for legible display text; below ≈ 1.5 : 1 it washes out. Raising display luminance, reflectance or adding a tint all help — each at a cost
          (power, see-through brightness, looking "like sunglasses").
        </Explainer>
        <Explainer title="Eyebox coverage" refs={[REFS.dodgson, REFS.ansur, REFS.kress2020]}>
          A monocular display puts its exit pupil around the display axis x_d = (A + DBL)/2 − offset. A wearer sees the whole image while the pupil centre, at IPD/2, stays within ±e/2:
          <Formula>{'coverage = Σ_g w_g [Φ((2(x_d + e/2) − μ_g)/σ_g) − Φ((2(x_d − e/2) − μ_g)/σ_g)]'}</Formula>
          Without an adjustment, the eyebox must be centred on the population — not on the lens box.
        </Explainer>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------- animated scene

interface SceneDef { name: string; nits: number; sky: number; mid: number; ground: number; snow?: boolean }

function tone(l: number, adapt: number) {
  // Reinhard-style global tone map around the adaptation luminance, then gamma.
  const x = l / (l + adapt);
  return Math.round(255 * Math.pow(Math.min(1, Math.max(0, x)), 1 / 1.6));
}

function SceneCard({ d, cfg, setCfg }: { d: DisplayOut; cfg: ErgoConfig; setCfg: (p: Partial<ErgoConfig>) => void }) {
  const scenes: SceneDef[] = useMemo(
    () => d.scenes.filter((s) => s.name !== 'Custom').map((s) => ({ name: s.name, nits: s.background_nits, sky: 1.5, mid: 0.55, ground: s.name.startsWith('Snow') ? 1.25 : 0.75, snow: s.name.startsWith('Snow') })),
    [d.scenes],
  );
  const [playing, setPlaying] = usePlaying();
  const [sel, setSel] = useState(3);
  const [nits, setNits] = useState<number | null>(null);
  const t = useClock(playing);
  const idx = playing ? Math.floor(t / 4) % scenes.length : Math.min(sel, scenes.length - 1);
  const sc = scenes[idx];
  const ph = d.photometry;
  const dispNits = nits ?? ph.image_nits / Math.max(1e-9, ph.lens_transmission * ph.combiner_reflectance);
  const img = dispNits * ph.lens_transmission * ph.combiner_reflectance;
  const T = ph.see_through;
  const acr = (img + T * sc.nits) / (T * sc.nits);
  const verdict = acr >= 3 ? 'good' : acr >= 1.5 ? 'marginal' : 'washed out';
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const W = Math.max(300, width);
  const H = Math.min(300, Math.max(200, W * 0.42));
  const adapt = T * sc.nits;
  const zones = [
    { y: 0, h: H * 0.45, k: sc.sky },
    { y: H * 0.45, h: H * 0.3, k: sc.mid },
    { y: H * 0.75, h: H * 0.25, k: sc.ground },
  ];
  const caption = 'Turn left in 50 m · Meeting with Dana at 3 pm · Battery 82 %';
  const fontPx = Math.max(12, Math.min(34, W / 24));
  const scroll = playing ? (t * 60) % (caption.length * fontPx * 0.6 + W) : 0;
  const textY = H * 0.47;
  return (
    <Card
      title="Heads-up text against the world"
      subtitle="The display's light adds to whatever is behind it. Background luminance ×  see-through transmission, plus the image — tone-mapped as your eye adapts to the scene."
      actions={<PlayButton playing={playing} onChange={setPlaying} />}
    >
      <div className="row" style={{ marginBottom: 6 }}>
        <Seg value={idx} onChange={(v) => { setPlaying(false); setSel(v); }} options={scenes.map((s, i) => ({ value: i, label: s.name.split(' /')[0] }))} />
      </div>
      <div ref={ref}>
        <svg width={W} height={H} className="hf-scene" role="img" aria-label={`${sc.name}: contrast ${acr.toFixed(2)} to 1, ${verdict}`}>
          <defs>
            {zones.map((z, i) => (
              <clipPath id={`hfz${i}`} key={i}><rect x={0} y={z.y} width={W} height={z.h} /></clipPath>
            ))}
          </defs>
          {zones.map((z, i) => {
            const g = tone(T * sc.nits * z.k, adapt);
            return <rect key={i} x={0} y={z.y} width={W} height={z.h} fill={`rgb(${g},${g},${Math.min(255, g + (i === 0 && !sc.snow ? 12 : 0))})`} />;
          })}
          {/* buildings */}
          {[0.05, 0.2, 0.38, 0.6, 0.78].map((x, i) => {
            const g = tone(T * sc.nits * (0.35 + 0.1 * (i % 3)), adapt);
            const bh = H * (0.25 + 0.12 * ((i * 7) % 3));
            return <rect key={i} x={W * x} y={H * 0.75 - bh} width={W * 0.14} height={bh} fill={`rgb(${g},${g},${g})`} />;
          })}
          {zones.map((z, i) => {
            const bg = tone(T * sc.nits * z.k, adapt);
            const tx = tone(T * sc.nits * z.k + img, adapt);
            return (
              <g key={`t${i}`} clipPath={`url(#hfz${i})`}>
                <text x={W - scroll} y={textY} fontSize={fontPx} fontWeight={700} fill={`rgb(${bg},${tx},${bg})`} fontFamily="var(--mono)">{caption}</text>
                <text x={W * 0.06} y={H * 0.88} fontSize={fontPx * 0.8} fontWeight={700} fill={`rgb(${bg},${tx},${bg})`} fontFamily="var(--mono)">→ 50 m</text>
              </g>
            );
          })}
          <rect x={8} y={8} width={Math.min(W - 16, 250)} height={44} rx={8} fill="rgba(0,0,0,0.55)" />
          <text x={16} y={26} fill="#fff" fontSize={12.5} fontWeight={600}>{sc.name} · {sc.nits.toLocaleString()} cd/m²</text>
          <text x={16} y={44} fill="#fff" fontSize={12.5}>ACR {acr.toFixed(2)} : 1 — {verdict === 'good' ? '✓' : verdict === 'marginal' ? '!' : '✕'} {verdict}</text>
        </svg>
      </div>
      <div className="grid grid-2" style={{ marginTop: 8 }}>
        <Slider label="Sunglass tint transmission (saved)" value={cfg.tint_transmission * 100} min={5} max={100} step={1} unit="%" onChange={(v) => setCfg({ tint_transmission: v / 100 })} />
        <Slider label="What-if display luminance (not saved)" value={dispNits} min={200} max={30000} log unit="nits" format={(v) => v.toFixed(0)} onChange={setNits} />
      </div>
      <div className="small muted">
        Image {f1(img, 0)} cd/m² (display × lens {f1(ph.lens_transmission * 100, 0)} % × combiner {f1(ph.combiner_reflectance * 100, 0)} %) over {f1(T * 100, 0)} % see-through. The rendering
        is illustrative; the numbers are the model.
      </div>
    </Card>
  );
}

// ---------------------------------------------------------------- legibility

function LegibilityCard({ d, cfg, setCfg }: { d: DisplayOut; cfg: ErgoConfig; setCfg: (p: Partial<ErgoConfig>) => void }) {
  return (
    <Card title="Text size vs visual acuity" subtitle={`${f1(d.arcmin_per_px, 2)} arcmin per pixel at the centre of a ${d.px_w} × ${d.px_h} display spanning ${f1(d.hfov_deg)}° × ${f1(d.vfov_deg)}°.`}>
      <Slider label="Caption cap height (saved)" value={cfg.caption_cap_px} min={3} max={40} step={1} unit="px" onChange={(v) => setCfg({ caption_cap_px: v })} />
      <Seg
        label="Wearer's acuity (uncorrected)"
        value={cfg.acuity_20_over}
        onChange={(v) => setCfg({ acuity_20_over: v })}
        options={[20, 30, 40, 60].map((v) => ({ value: v, label: `20/${v}` }))}
      />
      <LetterLadder d={d} />
      <div className="table-wrap">
        <table className="table">
          <thead><tr><th>Cap height</th><th>Angle</th><th>Snellen</th><th>Verdict</th></tr></thead>
          <tbody>
            {d.rows.map((r) => (
              <tr key={r.cap_px}>
                <td className="tnum">{r.cap_px} px</td>
                <td className="tnum">{f1(r.arcmin)}′</td>
                <td className="tnum">20/{f1(r.snellen_20_over, 0)}</td>
                <td><VerdictChip v={r.verdict} /></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="small muted">Smallest cap height for 16′: {d.min_px_16} px · for 20′: {d.min_px_20} px.</div>
    </Card>
  );
}

function LetterLadder({ d }: { d: DisplayOut }) {
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const W = Math.max(280, width);
  const H = 150;
  const maxAm = Math.max(...d.rows.map((r) => r.arcmin), 40) * 1.08;
  const sx = (a: number) => 14 + (a / maxAm) * (W - 28);
  const base = 104;
  const px = (a: number) => Math.max(3, (a / maxAm) * Math.min(90, W * 0.2));
  return (
    <div ref={ref} style={{ marginBottom: 8 }}>
      <svg width={W} height={H} className="hf-svg" role="img" aria-label="Letter sizes compared with the acuity threshold and the comfortable band">
        <rect x={0} y={0} width={W} height={H} rx={8} className="hf-panel" />
        <rect x={sx(16)} y={8} width={sx(40) - sx(16)} height={base - 4} fill="var(--good)" opacity={0.1} />
        <text x={sx(16) + 4} y={20} className="hf-lbl">comfortable 16–40′</text>
        <line x1={sx(d.acuity_arcmin)} x2={sx(d.acuity_arcmin)} y1={8} y2={base + 4} stroke="var(--bad)" strokeDasharray="4 3" />
        <text x={sx(d.acuity_arcmin) + 4} y={36} className="hf-lbl">acuity {f1(d.acuity_arcmin)}′</text>
        <line x1={14} x2={W - 14} y1={base + 4} y2={base + 4} stroke="var(--axis)" />
        {d.rows.map((r, i) => (
          <g key={r.cap_px}>
            <text x={sx(r.arcmin)} y={base} fontSize={px(r.arcmin) / 0.72} textAnchor="middle" className="hf-glyph">E</text>
            <line x1={sx(r.arcmin)} x2={sx(r.arcmin)} y1={base + 4} y2={base + 9} stroke="var(--axis)" />
            <text x={sx(r.arcmin)} y={base + 22 + (i % 2) * 13} textAnchor="middle" className="hf-tick">{W < 520 ? `${r.cap_px}` : `${r.cap_px}px · ${f1(r.arcmin, 0)}′`}</text>
          </g>
        ))}
      </svg>
    </div>
  );
}

function ContrastTable({ scenes, d }: { scenes: SceneRow[]; d: DisplayOut }) {
  return (
    <Card
      title="Ambient contrast ratio by scene"
      subtitle={`Image ${f1(d.photometry.image_nits, 0)} cd/m²; see-through ${f1(d.photometry.see_through * 100, 0)} %.`}
      actions={<DownloadButton small filename="iris-contrast.csv" label="CSV" content={() => csv(['scene', 'background_cd_m2', 'see_through_cd_m2', 'image_cd_m2', 'acr', 'verdict', 'display_nits_for_3to1'], scenes.map((s) => [s.name, s.background_nits, s.see_through_nits, s.image_nits, s.contrast_ratio, s.verdict, s.nits_for_3to1]))} />}
    >
      <Plot
        height={200}
        xLog
        xLabel="background luminance (cd/m²)"
        yLabel="ACR"
        series={[{ name: 'ACR', x: d.bg_axis, y: d.acr_vs_bg }]}
        hLines={[{ value: 3, label: '3 : 1' }, { value: 1.5, label: '1.5 : 1', color: 'var(--bad)' }]}
        vLines={scenes.slice(1).map((s) => ({ value: s.background_nits, label: '' }))}
        yDomain={[1, 12]}
      />
      <div className="table-wrap">
        <table className="table">
          <thead><tr><th>Scene</th><th>Background</th><th>ACR</th><th>Verdict</th><th>Display for 3 : 1</th></tr></thead>
          <tbody>
            {scenes.map((s) => (
              <tr key={s.name}>
                <td title={s.description}>{s.name}</td>
                <td className="tnum">{s.background_nits.toLocaleString()} cd/m²</td>
                <td className="tnum">{f1(s.contrast_ratio, 2)}</td>
                <td><VerdictChip v={s.verdict} /></td>
                <td className="tnum">{f1(s.nits_for_3to1, 0)} nits</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Card>
  );
}

// ---------------------------------------------------------------- eyebox

function seeded(n: number, seed: number) {
  let s = seed >>> 0;
  const u = () => {
    s = (s * 1664525 + 1013904223) >>> 0;
    return (s + 0.5) / 4294967296;
  };
  return Array.from({ length: n }, () => {
    const a = u(), b = u();
    return { z: Math.sqrt(-2 * Math.log(a)) * Math.cos(2 * Math.PI * b), female: u() };
  });
}

function EyeboxCard({ d, cfg, setCfg }: { d: DisplayOut; cfg: ErgoConfig; setCfg: (p: Partial<ErgoConfig>) => void }) {
  const eb = d.eyebox;
  const [playing, setPlaying] = usePlaying();
  const t = useClock(playing);
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const W = Math.max(300, width);
  const H = 150;
  // monocular PD strip from 24 to 40 mm
  const x0 = 24, x1 = 40;
  const sx = (v: number) => 16 + ((v - x0) / (x1 - x0)) * (W - 32);
  const sample = useMemo(() => seeded(160, 11), []);
  const ff = cfg.female_fraction;
  // same IPD statistics as the engine (ANSUR II unless edited)
  const [[mm, sm], [mw, sw]] = eb.ipd_stats;
  const pupils = sample.map((p) => {
    const f = p.female < ff;
    const ipd = f ? mw + sw * p.z : mm + sm * p.z;
    return { x: ipd / 2, f };
  });
  const lo = eb.display_x_mm - eb.eyebox_mm / 2, hi = eb.display_x_mm + eb.eyebox_mm / 2;
  const nShown = playing || t > 0 ? Math.min(pupils.length, Math.floor(t * 40) % (pupils.length + 40)) : pupils.length;
  return (
    <Card
      title="Eyebox coverage: whose pupil lands in the display?"
      subtitle={`Display axis ${f1(eb.display_x_mm)} mm from the face mid-line (lens-box centre ${f1(d.eyebox.display_x_mm + cfg.display_offset_mm)} mm − offset ${f1(cfg.display_offset_mm)} mm); eyebox ${f1(eb.eyebox_mm)} mm accepts IPD ${f1(eb.ipd_window_mm[0])}–${f1(eb.ipd_window_mm[1])} mm.`}
      actions={<PlayButton playing={playing} onChange={setPlaying} />}
    >
      <div ref={ref}>
        <svg width={W} height={H} className="hf-svg" role="img" aria-label="Right-eye pupils of simulated wearers against the eyebox">
          <rect x={sx(lo)} y={18} width={sx(hi) - sx(lo)} height={H - 52} rx={6} fill="var(--good)" opacity={0.14} stroke="var(--good)" />
          <line x1={sx(eb.display_x_mm)} x2={sx(eb.display_x_mm)} y1={14} y2={H - 30} stroke="var(--accent)" strokeDasharray="4 3" />
          <text x={sx(eb.display_x_mm) + 4} y={14} className="hf-lbl">display axis</text>
          {[24, 28, 32, 36, 40].map((v) => (
            <g key={v}>
              <line x1={sx(v)} x2={sx(v)} y1={H - 30} y2={H - 25} stroke="var(--axis)" />
              <text x={sx(v)} y={H - 12} textAnchor="middle" className="hf-tick">{v}</text>
            </g>
          ))}
          <text x={W - 16} y={H - 2} textAnchor="end" className="hf-tick">right-eye monocular PD = IPD/2 (mm)</text>
          {pupils.slice(0, nShown).map((p, i) => {
            const inside = p.x >= lo && p.x <= hi;
            const y = 24 + ((i * 37) % (H - 62));
            return inside ? (
              <circle key={i} cx={sx(p.x)} cy={y} r={3.2} fill="var(--good)" />
            ) : (
              <path key={i} d={`M${sx(p.x) - 3},${y - 3}L${sx(p.x) + 3},${y + 3}M${sx(p.x) - 3},${y + 3}L${sx(p.x) + 3},${y - 3}`} stroke="var(--bad)" strokeWidth={1.6} />
            );
          })}
        </svg>
      </div>
      <div className="hf-legend">
        <span><svg width={10} height={10}><circle cx={5} cy={5} r={4} fill="var(--good)" /></svg> sees the whole image</span>
        <span><svg width={10} height={10}><path d="M1,1L9,9M1,9L9,1" stroke="var(--bad)" strokeWidth={1.6} /></svg> pupil outside the eyebox</span>
      </div>
      <div className="hf-split" style={{ marginTop: 8 }}>
        <div className="grid grid-2" style={{ minWidth: 0 }}>
          <Plot
            height={210}
            xLabel="IPD (mm)"
            yLabel="density"
            series={[{ name: 'Combined IPD', x: eb.ipd_x, y: eb.ipd_pdf, area: true }]}
            xBands={[{ from: eb.ipd_window_mm[0], to: eb.ipd_window_mm[1], color: 'color-mix(in srgb, var(--good) 18%, transparent)', label: 'in eyebox' }]}
            formatY={(v) => v.toFixed(2)}
          />
          <Plot
            height={210}
            xLabel="display offset towards the nose (mm)"
            yLabel="coverage (%)"
            series={[{ name: 'coverage', x: eb.offsets, y: eb.coverage_vs_offset }]}
            vLines={[{ value: cfg.display_offset_mm, label: 'now' }, { value: eb.best_offset_mm, label: 'best' }]}
            yDomain={[0, 100]}
          />
        </div>
        <div className="stack" style={{ gap: 6 }}>
          <Slider label="Display axis offset towards the nose (saved)" value={cfg.display_offset_mm} min={-4} max={8} step={0.1} unit="mm" onChange={(v) => setCfg({ display_offset_mm: v })} />
          <Slider label="Face asymmetry (SD of monocular PD difference)" value={cfg.asymmetry_sd_mm} min={0} max={2} step={0.1} unit="mm" onChange={(v) => setCfg({ asymmetry_sd_mm: v })} />
          <button className="btn small" disabled={Math.abs(eb.best_offset_mm - cfg.display_offset_mm) < 0.05} onClick={() => setCfg({ display_offset_mm: Number(eb.best_offset_mm.toFixed(1)) })}>
            Centre on the population ({f1(eb.best_offset_mm)} mm → {f1(eb.best_coverage_pct, 0)} %)
          </button>
          <Kv
            rows={[
              ...eb.groups.map((g) => [GROUP_LABEL[g.group], `${f1(g.coverage_pct)} %`] as [string, string]),
              ['Eyebox for 95 % (centred)', `${f1(eb.eyebox_for_95_mm)} mm`],
              ['Eyebox with a filled 4 mm pupil', `${f1(eb.eyebox_full_pupil_mm)} mm`],
            ]}
          />
        </div>
      </div>
    </Card>
  );
}
