// Tab 1 — Anthropometry & fit: published head/face dimensions as normal
// approximations, percentile tables, a seeded Monte Carlo population scored
// against the Iris frame, an S/M/L size search and a 3-D parametric head.
import { useEffect, useMemo, useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, NumberField, Slider, Stat, Toggle } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useElementWidth } from '../../components/useElementWidth';
import type { Project } from '../../engine/types';
import { useProject } from '../../state/project';
import type { PartsOut } from '../cad/api';
import { Head3D } from './Head3D';
import { csv, f1, GROUP_LABEL, Kv, OkChip, PlayButton, REFS, Seg, useClock, useErgo, usePlaying } from './shared';
import type { DimTable, ErgoConfig, FitOut, HeadOut, PopulationOut, SizeOut, SizesOut } from './types';

type DimId = 'head_breadth' | 'ipd' | 'ear_distance' | 'nose_bridge';

export function FitTab() {
  const { project, cfg, setCfg, resetCfg } = useErgo();
  const fit = useCompute<FitOut>('ergonomics.fit', { project, scatter: 900 }, !!cfg);
  const pop = useCompute<PopulationOut>('ergonomics.population', { project }, !!cfg);
  const sizes = useCompute<SizesOut>('ergonomics.sizes', { project, samples: 1500 }, !!cfg);
  const f = fit.data;
  if (!cfg) return <div className="muted small">Loading the engine…</div>;
  const tone = (v: number, good: number, ok: number) => (v >= good ? 'pass' : v >= ok ? 'warn' : 'fail');
  return (
    <div className="stack">
      <ErrorBox error={fit.error ?? pop.error ?? sizes.error} />
      {f && (
        <div className="grid grid-4 hf-stats">
          <Stat label="One-size fit coverage" value={f1(f.combined.fit_pct)} unit="%" tone={tone(f.combined.fit_pct, 90, 60)} note={`${f.frame.label} · ergonomics.fit_coverage_pct`} />
          <Stat label="Pupils centred (PD rule)" value={f1(f.combined.pd_pct)} unit="%" note={`|frame PD − IPD|/2 ≤ ${cfg.pd_tolerance_mm} mm`} />
          <Stat label="Temple reaches the ear" value={f1(f.combined.ear_pct)} unit="%" note={`bend ${f1(f.frame.geom.bend_from_pin_mm)} mm behind the hinge`} />
          <Stat
            label="Clamp inside comfort band"
            value={f1(f.combined.width_pct)}
            unit="%"
            tone={tone(f.combined.width_pct, 80, 50)}
            note={`${f1(f.combined.too_tight_pct, 0)} % too tight · ${f1(f.combined.too_loose_pct, 0)} % too loose`}
          />
        </div>
      )}
      <div className="hf-split">
        <HeadCard project={project} cfg={cfg} sizes={sizes.data?.sizes ?? []} />
        <RulesCard cfg={cfg} setCfg={setCfg} resetCfg={resetCfg} fit={f} />
      </div>
      {f && <MonteCarloCard fit={f} />}
      {pop.data && f && <PopulationCard pop={pop.data} fit={f} cfg={cfg} setCfg={setCfg} />}
      {sizes.data && <SizesCard out={sizes.data} />}
      <div className="grid grid-2">
        <Explainer title="From published statistics to a fit percentage" refs={[REFS.ansur, REFS.bodyspace, REFS.iso7250]}>
          Each dimension is approximated by a normal distribution per sex, x_p = μ + z_p σ, with z = ±1.645 at the 5th/95th percentiles. A population is a mix of women and men,
          so its percentiles come from the mixture F(x) = w Φ((x − μ_w)/σ_w) + (1 − w) Φ((x − μ_m)/σ_m). Designing to the "average person" fits almost nobody: this tab draws
          thousands of correlated people (Cholesky factor of the ANSUR II correlation matrix) and asks the frame about each one.
          <Formula>{'x = μ + σ ⊙ (L z),   L Lᵀ = R,   z ~ N(0, I)     coverage = (1/N) Σ 1[person fits]'}</Formula>
          With N = {cfg.samples} the Monte Carlo standard error is at most √(0.25/N) ≈ {f1(100 * Math.sqrt(0.25 / cfg.samples), 1)} percentage points.
        </Explainer>
        <Explainer title="The three fit rules" refs={[REFS.iso8624, REFS.sanders]}>
          <b>Pupils centred:</b> the frame PD is A + DBL (boxing system); decentration per eye |A + DBL − IPD|/2 must stay within {cfg.pd_tolerance_mm} mm (a typical dispensing
          tolerance — check with an optician). <b>Temple length:</b> the ear bend must sit between {-cfg.ear_window_mm[0]} mm in front of and {cfg.ear_window_mm[1]} mm behind the
          top of the ear root. <b>Frame width:</b> the temples must spread to the head breadth with a clamp force between the "does not slide when you look down" and the "sustained
          pressure" limits (Temple clamp tab). The nose-bridge rule is optional because its statistics are assumptions.
        </Explainer>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------- 3-D head

function HeadCard({ project, cfg, sizes }: { project: Project; cfg: ErgoConfig; sizes: SizeOut[] }) {
  const [sex, setSex] = useState<'men' | 'women'>('men');
  const [hb, setHb] = useState(50);
  const [ipd, setIpd] = useState(50);
  const [ear, setEar] = useState(50);
  const [nose, setNose] = useState(50);
  const [size, setSize] = useState<string>('iris');
  const chosen = sizes.find((s) => s.name === size);
  const headIn = { project, sex, head_breadth_pct: hb, ipd_pct: ipd, ear_pct: ear, nose_pct: nose, size: chosen?.size };
  const head = useCompute<HeadOut>('ergonomics.head', headIn);
  const partsProject = useMemo(() => {
    if (!chosen) return project;
    return { ...project, design: { ...project.design, frame: { ...project.design.frame, lens_width_mm: chosen.size.lens_width_mm, bridge_mm: chosen.size.bridge_mm, temple_length_mm: chosen.size.temple_length_mm } } };
  }, [project, chosen]);
  const parts = useCompute<PartsOut>('cad.parts', { project: partsProject, orientation: 'assembly', display: true });
  const h = head.data;
  return (
    <Card title="3-D fit: a parametric head wearing Iris" subtitle="Drag the percentile sliders; the temples swing open until they meet the head. Markers: temple contacts and ear-root tops (green = rule met).">
      <ErrorBox error={head.error ?? parts.error} />
      <div className="hf-head">
        <div style={{ minWidth: 0 }}>{h ? <Head3D head={h} parts={parts.data} /> : <div className="muted small">Building the head…</div>}</div>
        <div className="hf-head-controls">
         <div className="stack" style={{ gap: 2 }}>
          <Seg value={sex} onChange={setSex} options={[{ value: 'men', label: 'Man' }, { value: 'women', label: 'Woman' }]} label="Wearer" />
          <Slider label="Head breadth percentile" value={hb} min={1} max={99} step={1} unit={h ? `→ ${f1(h.person.head_breadth)} mm` : ''} format={(v) => `P${v.toFixed(0)}`} onChange={setHb} />
          <Slider label="IPD percentile" value={ipd} min={1} max={99} step={1} unit={h ? `→ ${f1(h.person.ipd)} mm` : ''} format={(v) => `P${v.toFixed(0)}`} onChange={setIpd} />
          <Slider label="Hinge-to-ear percentile" value={ear} min={1} max={99} step={1} unit={h ? `→ ${f1(h.person.ear)} mm` : ''} format={(v) => `P${v.toFixed(0)}`} onChange={setEar} />
          <Slider label="Nose bridge percentile" value={nose} min={1} max={99} step={1} unit={h ? `→ ${f1(h.person.nose)} mm` : ''} format={(v) => `P${v.toFixed(0)}`} onChange={setNose} />
          <Seg
            value={size}
            onChange={setSize}
            label="Frame"
            options={[{ value: 'iris', label: 'Iris' }, ...sizes.map((s) => ({ value: s.name, label: s.name, title: s.label }))]}
          />
         </div>
         <div className="stack" style={{ gap: 8 }}>
          {h && (
            <>
              <div className="small muted">{h.frame.label}</div>
              <div className="row" style={{ gap: 6 }}>
                <OkChip ok={h.check.pd_ok} yes="pupils centred" no={`decentred ${f1(h.check.decentration_mm)} mm`} />
                <OkChip ok={h.check.ear_ok} yes="temple length" no={h.check.bend_minus_ear_mm < 0 ? 'temple short' : 'temple long'} />
                <OkChip ok={h.check.width_ok} yes={`clamp ${f1(h.check.clamp_n, 2)} N`} no={`${h.check.clamp_n > h.band.max_n ? 'tight' : 'loose'} ${f1(h.check.clamp_n, 2)} N`} />
                <OkChip ok={h.in_eyebox} yes="sees the display" no="pupil outside eyebox" />
              </div>
              <Kv
                rows={[
                  ['Temple opening', `${f1(h.temple_angle_deg[0])}° per side`],
                  ['Outward deflection R / L', `${f1(h.deflection_mm[0])} / ${f1(h.deflection_mm[1])} mm`],
                  ['Pupil vs display axis', `${h.pupil_offset_mm >= 0 ? '+' : ''}${f1(h.pupil_offset_mm)} mm (eyebox ±${f1(h.eyebox_mm / 2)})`],
                  ['Bend − ear', `${f1(h.check.bend_minus_ear_mm)} mm`],
                ]}
              />
            </>
          )}
          <p className="small muted" style={{ margin: 0 }}>A procedural head (ellipsoid cranium, tapered face, wedge nose) sized by the sliders — a teaching aid, not a scanned head form. Splay: {f1(cfg.temple_splay_deg)}°.</p>
         </div>
        </div>
      </div>
    </Card>
  );
}

// ---------------------------------------------------------------- rules

function RulesCard({ cfg, setCfg, resetCfg, fit }: { cfg: ErgoConfig; setCfg: (p: Partial<ErgoConfig>) => void; resetCfg: () => void; fit: FitOut | null }) {
  return (
    <Card title="Fit rules & population" subtitle="Saved in the project (design.extra.ergonomics) — the metrics use them." actions={<button className="btn small" onClick={resetCfg}>Reset</button>}>
      <Slider label="Temple splay at the hinge stop" value={cfg.temple_splay_deg} min={-2} max={16} step={0.5} unit="°" onChange={(v) => setCfg({ temple_splay_deg: v })} hint="0° = straight temples, as the Iris CAD builds them" />
      <Slider label="Max decentration per eye" value={cfg.pd_tolerance_mm} min={1} max={8} step={0.5} unit="mm" onChange={(v) => setCfg({ pd_tolerance_mm: v })} />
      <Slider label="Bend may sit in front of the ear by" value={-cfg.ear_window_mm[0]} min={0} max={10} step={0.5} unit="mm" onChange={(v) => setCfg({ ear_window_mm: [-v, cfg.ear_window_mm[1]] })} />
      <Slider label="…or behind it by" value={cfg.ear_window_mm[1]} min={2} max={25} step={0.5} unit="mm" onChange={(v) => setCfg({ ear_window_mm: [cfg.ear_window_mm[0], v] })} />
      <Slider label="Share of women" value={cfg.female_fraction * 100} min={0} max={100} step={5} unit="%" onChange={(v) => setCfg({ female_fraction: v / 100 })} />
      <Toggle label="Also require the bridge to suit the nose (assumed statistics)" value={cfg.nose_check} onChange={(v) => setCfg({ nose_check: v })} />
      <div className="grid grid-2" style={{ gap: 8 }}>
        <NumberField label="People simulated" value={cfg.samples} step={500} onChange={(v) => setCfg({ samples: Math.max(200, Math.min(50000, Math.round(v))) })} />
        <NumberField label="Seed" value={cfg.seed} step={1} onChange={(v) => setCfg({ seed: Math.max(0, Math.round(v)) })} />
      </div>
      {fit && (
        <div className="small muted">
          Comfort band {f1(fit.band.min_n, 2)}–{f1(fit.band.max_n, 2)} N per side ↔ head breadth {f1(fit.band_heads_mm[0], 0)}–{f1(fit.band_heads_mm[1], 0)} mm with this frame.
        </div>
      )}
    </Card>
  );
}

// ---------------------------------------------------------------- Monte Carlo scatter

type Cls = 'fits' | 'tight' | 'loose' | 'pd' | 'ear' | 'nose';
const CLS: Record<Cls, { label: string; color: string }> = {
  fits: { label: 'fits', color: 'var(--good)' },
  tight: { label: 'too tight', color: 'var(--bad)' },
  loose: { label: 'too loose', color: 'var(--series-4)' },
  pd: { label: 'pupils off-centre', color: 'var(--series-7)' },
  ear: { label: 'temple length', color: 'var(--series-2)' },
  nose: { label: 'bridge vs nose', color: 'var(--series-5)' },
};
function classify(p: FitOut['people'][number], band: FitOut['band']): Cls {
  if (p.fits) return 'fits';
  if (!p.width_ok) return p.clamp_n > band.max_n ? 'tight' : 'loose';
  if (!p.pd_ok) return 'pd';
  if (!p.ear_ok) return 'ear';
  return 'nose';
}
function Marker({ c, x, y, r = 3.2 }: { c: Cls; x: number; y: number; r?: number }) {
  const col = CLS[c].color;
  switch (c) {
    case 'fits':
      return <circle cx={x} cy={y} r={r} fill={col} opacity={0.85} />;
    case 'tight':
      return <path d={`M${x - r},${y - r}L${x + r},${y + r}M${x - r},${y + r}L${x + r},${y - r}`} stroke={col} strokeWidth={1.6} />;
    case 'loose':
      return <circle cx={x} cy={y} r={r} fill="none" stroke={col} strokeWidth={1.4} />;
    case 'pd':
      return <path d={`M${x},${y - r * 1.2}L${x + r},${y + r * 0.8}L${x - r},${y + r * 0.8}Z`} fill={col} opacity={0.85} />;
    case 'ear':
      return <rect x={x - r * 0.85} y={y - r * 0.85} width={r * 1.7} height={r * 1.7} fill={col} opacity={0.85} />;
    default:
      return <path d={`M${x},${y - r}L${x + r},${y}L${x},${y + r}L${x - r},${y}Z`} fill={col} opacity={0.85} />;
  }
}

function MonteCarloCard({ fit }: { fit: FitOut }) {
  const [playing, setPlaying] = usePlaying();
  const [nonce, setNonce] = useState(0);
  const t = useClock(playing, `${nonce}|${fit.n}|${fit.frame.label}|${fit.config.seed}`);
  const people = fit.people;
  const shown = playing || t > 0 ? Math.min(people.length, Math.floor(t * 320)) : people.length;
  useEffect(() => {
    if (playing && shown >= people.length) setPlaying(false);
  }, [playing, shown, people.length, setPlaying]);
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const W = Math.max(width, 300);
  const H = Math.min(380, Math.max(260, W * 0.55));
  const m = { l: 46, r: 12, t: 12, b: 36 };
  const x0 = 128, x1 = 178, y0 = 50, y1 = 78;
  const sx = (v: number) => m.l + ((v - x0) / (x1 - x0)) * (W - m.l - m.r);
  const sy = (v: number) => H - m.b - ((v - y0) / (y1 - y0)) * (H - m.t - m.b);
  const counts: Record<Cls, number> = { fits: 0, tight: 0, loose: 0, pd: 0, ear: 0, nose: 0 };
  const cls = people.map((p) => classify(p, fit.band));
  for (let i = 0; i < shown; i++) counts[cls[i]]++;
  const bh = fit.band_heads_mm;
  const pw = fit.pd_window_mm;
  const clip = (v: number, a: number, b: number) => Math.max(a, Math.min(b, v));
  const covRows: [string, FitOut['combined']][] = [['Women', fit.women], ['Men', fit.men], ['Combined', fit.combined]];
  return (
    <Card
      title="Monte Carlo population vs the Iris frame"
      subtitle={`${fit.n} simulated adults (seed ${fit.config.seed}); the first ${people.length} are drawn. Each marker is one person: head breadth vs interpupillary distance (IPD).`}
      actions={
        <>
          <PlayButton playing={playing} onChange={(p) => { if (p && shown >= people.length) setNonce((n) => n + 1); setPlaying(p); }} />
          <DownloadButton
            small
            filename="iris-fit-population.csv"
            label="CSV"
            content={() => csv(['female', 'head_breadth_mm', 'ipd_mm', 'hinge_to_ear_mm', 'nose_bridge_mm', 'clamp_n', 'decentration_mm', 'bend_minus_ear_mm', 'pd_ok', 'ear_ok', 'width_ok', 'nose_ok', 'fits'],
              fit.people.map((p) => [p.female, p.head_breadth, p.ipd, p.ear, p.nose, p.clamp_n, p.decentration_mm, p.bend_minus_ear_mm, p.pd_ok, p.ear_ok, p.width_ok, p.nose_ok, p.fits]))}
          />
        </>
      }
    >
      <div className="hf-mc">
        <div ref={ref} style={{ minWidth: 0 }}>
          <svg width={W} height={H} className="hf-svg" role="img" aria-label="Scatter of simulated people coloured by fit">
            {/* comfort window in head breadth and PD window */}
            <rect x={sx(clip(bh[0], x0, x1))} y={m.t} width={Math.max(0, sx(clip(bh[1], x0, x1)) - sx(clip(bh[0], x0, x1)))} height={H - m.t - m.b} fill="var(--good)" opacity={0.08} />
            <rect x={m.l} y={sy(clip(pw[1], y0, y1))} width={W - m.l - m.r} height={Math.max(0, sy(clip(pw[0], y0, y1)) - sy(clip(pw[1], y0, y1)))} fill="var(--accent)" opacity={0.07} />
            {[130, 140, 150, 160, 170].map((v) => (
              <g key={`x${v}`}>
                <line x1={sx(v)} x2={sx(v)} y1={m.t} y2={H - m.b} className="hf-grid" />
                <text x={sx(v)} y={H - m.b + 14} textAnchor="middle" className="hf-tick">{v}</text>
              </g>
            ))}
            {[55, 60, 65, 70, 75].map((v) => (
              <g key={`y${v}`}>
                <line x1={m.l} x2={W - m.r} y1={sy(v)} y2={sy(v)} className="hf-grid" />
                <text x={m.l - 6} y={sy(v) + 4} textAnchor="end" className="hf-tick">{v}</text>
              </g>
            ))}
            <text x={(m.l + W - m.r) / 2} y={H - 4} textAnchor="middle" className="hf-axis">head breadth (mm)</text>
            <text transform={`translate(12 ${(m.t + H - m.b) / 2}) rotate(-90)`} textAnchor="middle" className="hf-axis">IPD (mm)</text>
            {bh[1] > x0 && bh[0] < x1 && <text x={sx(clip(bh[0], x0, x1)) + 4} y={m.t + 12} className="hf-tick">comfort band</text>}
            {pw[0] < y1 && <text x={W - m.r - 4} y={sy(clip(pw[1], y0, y1)) + 12} textAnchor="end" className="hf-tick">pupils centred</text>}
            {people.slice(0, shown).map((p, i) => (
              <Marker key={i} c={cls[i]} x={sx(clip(p.head_breadth, x0, x1))} y={sy(clip(p.ipd, y0, y1))} r={p.female ? 2.8 : 3.3} />
            ))}
          </svg>
          <div className="hf-legend">
            {(Object.keys(CLS) as Cls[]).filter((c) => c !== 'nose' || fit.config.nose_check).map((c) => (
              <span key={c}>
                <svg width={12} height={12}><Marker c={c} x={6} y={6} r={4} /></svg> {CLS[c].label} <b className="tnum">{shown ? f1((100 * counts[c]) / shown, 0) : '0'} %</b>
              </span>
            ))}
          </div>
        </div>
        <div className="stack" style={{ gap: 10 }}>
          {covRows.map(([name, c]) => (
            <div key={name}>
              <div className="row" style={{ justifyContent: 'space-between' }}>
                <b className="small">{name}</b>
                <span className="small tnum">{f1(c.fit_pct)} % fit</span>
              </div>
              {[
                ['PD rule', c.pd_pct],
                ['Temple length', c.ear_pct],
                ['Clamp band', c.width_pct],
              ].map(([k, v]) => (
                <div className="hf-bar" key={k as string}>
                  <span className="small muted">{k}</span>
                  <div className="hf-bar-track"><div style={{ width: `${v}%` }} /></div>
                  <span className="small tnum">{f1(v as number, 0)} %</span>
                </div>
              ))}
            </div>
          ))}
          <p className="small muted" style={{ margin: 0 }}>
            Iris clamp force at the 5th / 50th / 95th percentile head: {fit.clamp_p.map((v) => f1(v, 2)).join(' / ')} N per side (band {f1(fit.band.min_n, 2)}–{f1(fit.band.max_n, 2)} N).
          </p>
        </div>
      </div>
    </Card>
  );
}

// ---------------------------------------------------------------- population

function PopulationCard({ pop, fit, cfg, setCfg }: { pop: PopulationOut; fit: FitOut; cfg: ErgoConfig; setCfg: (p: Partial<ErgoConfig>) => void }) {
  const [dim, setDim] = useState<DimId>('ipd');
  const t = pop.tables.find((x) => x.dim.id === dim) as DimTable;
  const g = fit.frame.geom;
  const refs =
    dim === 'head_breadth'
      ? [{ value: fit.band_heads_mm[0], label: 'band lo' }, { value: fit.band_heads_mm[1], label: 'band hi' }]
      : dim === 'ipd'
        ? [{ value: g.frame_pd_mm, label: `frame PD ${f1(g.frame_pd_mm, 0)}` }, { value: fit.pd_window_mm[0], label: '−tol' }, { value: fit.pd_window_mm[1], label: '+tol' }]
        : dim === 'ear_distance'
          ? [{ value: g.bend_from_pin_mm, label: `Iris bend ${f1(g.bend_from_pin_mm, 0)}` }, { value: fit.ear_window_abs_mm[0], label: 'window' }, { value: fit.ear_window_abs_mm[1], label: '' }]
          : [{ value: g.bridge_mm, label: `DBL ${f1(g.bridge_mm, 0)}` }];
  const ov = cfg.overrides?.[dim] ?? {};
  const setOv = (sex: 'men' | 'women', k: 0 | 1, v: number) => {
    const cur = sex === 'men' ? ov.men ?? [t.dim.men.mean, t.dim.men.sd] : ov.women ?? [t.dim.women.mean, t.dim.women.sd];
    const next: [number, number] = [cur[0], cur[1]];
    next[k] = v;
    setCfg({ overrides: { ...(cfg.overrides ?? {}), [dim]: { ...ov, [sex]: next } } });
  };
  const clearOv = () => {
    const o = { ...(cfg.overrides ?? {}) };
    delete o[dim];
    setCfg({ overrides: o });
  };
  const table = () =>
    csv(
      ['dimension', 'group', 'mean_mm', 'sd_mm', ...pop.percentiles.map((p) => `P${p}`), 'source'],
      pop.tables.flatMap((tb) => tb.groups.map((gs) => [tb.dim.label, gs.group, gs.mean, gs.sd, ...gs.percentiles, tb.dim.source])),
    );
  return (
    <Card
      title="Anthropometry: head and face dimensions"
      subtitle="Normal approximations per sex; the combined population mixes them with the share of women set above."
      actions={<DownloadButton small filename="anthropometry-percentiles.csv" label="Percentiles CSV" content={table} />}
    >
      <Seg
        value={dim}
        onChange={setDim}
        options={pop.tables.map((tb) => ({ value: tb.dim.id as DimId, label: tb.dim.label }))}
      />
      <div className="hf-split">
        <div style={{ minWidth: 0 }}>
          <Plot
            height={250}
            xLabel={`${t.dim.label} (${t.dim.unit})`}
            yLabel="density"
            series={[
              { name: 'Men', x: t.x, y: t.pdf[0], color: 'var(--series-1)' },
              { name: 'Women', x: t.x, y: t.pdf[1], color: 'var(--series-2)' },
              { name: 'Combined', x: t.x, y: t.pdf[2], color: 'var(--text-2)', dash: '5 4', area: true },
            ]}
            vLines={refs}
            formatY={(v) => v.toFixed(3)}
          />
          <div className="table-wrap">
            <table className="table hf-table">
              <thead>
                <tr>
                  <th>Group</th>
                  <th>Mean</th>
                  <th>SD</th>
                  {pop.percentiles.map((p) => <th key={p}>P{p}</th>)}
                </tr>
              </thead>
              <tbody>
                {t.groups.map((gs, gi) => (
                  <tr key={gs.group}>
                    <td>{GROUP_LABEL[gs.group]}</td>
                    <td className="tnum">{f1(gs.mean)}</td>
                    <td className="tnum">{f1(gs.sd, 2)}</td>
                    {gs.percentiles.map((v, i) => {
                      const ai = pop.ansur_percentiles.indexOf(pop.percentiles[i]);
                      const emp = t.dim.ansur && gi < 2 && ai >= 0 ? t.dim.ansur[gi][ai] : null;
                      return (
                        <td key={i} className="tnum" title={emp != null ? `ANSUR II empirical: ${emp} mm` : undefined}>
                          {f1(v)}
                          {emp != null && <div className="hf-emp">{f1(emp)}</div>}
                        </td>
                      );
                    })}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {t.dim.ansur && <div className="small muted">Small grey numbers: ANSUR II empirical percentiles for comparison with the normal approximation.</div>}
        </div>
        <div className="stack" style={{ gap: 8 }}>
          <div className="row" style={{ gap: 6 }}>
            {t.dim.measured ? <Chip tone="pass">✓ measured data · normal approx.</Chip> : <Chip tone="warn">! engineering assumption</Chip>}
            {t.dim.overridden && <Chip tone="info">edited</Chip>}
          </div>
          <div className="small"><b>{t.dim.symbol}:</b> {t.dim.definition}</div>
          <div className="small muted">Source: {t.dim.source}</div>
          <div className="grid grid-2" style={{ gap: 8 }}>
            <NumberField label="Men mean" unit="mm" value={Number(t.dim.men.mean.toFixed(2))} step={0.5} onChange={(v) => setOv('men', 0, v)} />
            <NumberField label="Men SD" unit="mm" value={Number(t.dim.men.sd.toFixed(2))} step={0.1} onChange={(v) => setOv('men', 1, v)} />
            <NumberField label="Women mean" unit="mm" value={Number(t.dim.women.mean.toFixed(2))} step={0.5} onChange={(v) => setOv('women', 0, v)} />
            <NumberField label="Women SD" unit="mm" value={Number(t.dim.women.sd.toFixed(2))} step={0.1} onChange={(v) => setOv('women', 1, v)} />
          </div>
          {t.dim.overridden && <button className="btn small" onClick={clearOv}>Restore published values</button>}
          <div className="small muted">
            Correlations used for sampling (men / women): head breadth–IPD {pop.correlations[0][0][1].toFixed(2)} / {pop.correlations[1][0][1].toFixed(2)}; hinge-to-ear borrows ANSUR II head
            length&apos;s correlations; the nose is independent.
          </div>
        </div>
      </div>
    </Card>
  );
}

// ---------------------------------------------------------------- sizes

function SizesCard({ out }: { out: SizesOut }) {
  const { setCfg } = useErgo();
  const patchDesign = useProject((st) => st.patchDesign);
  const apply = (s: SizeOut) => {
    patchDesign('frame', { lens_width_mm: s.size.lens_width_mm, bridge_mm: s.size.bridge_mm, temple_length_mm: s.size.temple_length_mm });
    if (s.size.splay_deg != null) setCfg({ temple_splay_deg: s.size.splay_deg });
  };
  return (
    <Card
      title="Size range: which S / M / L cover the most people?"
      subtitle={`Each person takes the smallest size that fits them. Three sizes cover ${f1(out.union_pct)} % vs ${f1(out.iris_pct)} % for the one Iris size.`}
      actions={
        <DownloadButton
          small
          filename="iris-size-range.csv"
          label="CSV"
          content={() => csv(['size', 'lens_width_mm', 'bridge_mm', 'temple_mm', 'splay_deg', 'frame_pd_mm', 'front_width_mm', 'head_breadth_lo_mm', 'head_breadth_hi_mm', 'alone_pct', 'served_pct'],
            out.sizes.map((s) => [s.name, s.size.lens_width_mm, s.size.bridge_mm, s.size.temple_length_mm, s.size.splay_deg ?? '', s.frame_pd_mm, s.total_width_mm, s.head_breadth_range_mm[0], s.head_breadth_range_mm[1], s.alone_pct, s.served_pct]))}
        />
      }
    >
      <div className="table-wrap">
        <table className="table">
          <thead>
            <tr><th>Size</th><th>A □ DBL – temple · splay</th><th>Frame PD</th><th>Comfortable heads</th><th>Fits alone</th><th>Served</th><th /></tr>
          </thead>
          <tbody>
            {out.sizes.map((s) => (
              <tr key={s.name}>
                <td><b>{s.name}</b></td>
                <td className="mono small">{s.label}</td>
                <td className="tnum">{f1(s.frame_pd_mm, 0)} mm</td>
                <td className="tnum">{f1(s.head_breadth_range_mm[0], 0)}–{f1(s.head_breadth_range_mm[1], 0)} mm</td>
                <td className="tnum">{f1(s.alone_pct)} %</td>
                <td>
                  <div className="hf-bar" style={{ gridTemplateColumns: 'minmax(60px, 1fr) 44px' }}>
                    <div className="hf-bar-track"><div style={{ width: `${s.served_pct}%` }} /></div>
                    <span className="small tnum">{f1(s.served_pct, 0)} %</span>
                  </div>
                </td>
                <td><button className="btn small" onClick={() => apply(s)} title="Set the Iris frame and splay to this size">Use for Iris</button></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="small muted">{out.notes.join(' ')}</div>
    </Card>
  );
}
