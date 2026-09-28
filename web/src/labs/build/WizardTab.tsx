// Process-selection wizard: sliders → ranked processes with a transparent
// weighted score, reasons, vendor links and which Protoforge file to upload.
import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider } from '../../components/ui';
import { fmtNum, seriesColor } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import type { Category, Vendor, Weights, WizardOut } from './types';

const DEFAULT_W: Weights = { quantity: 3, cost: 3, tolerance: 2, finish: 1, strength: 2, lead_time: 2, size: 1, geometry: 2 };
const CRITERIA: (keyof Weights)[] = ['quantity', 'cost', 'tolerance', 'finish', 'strength', 'lead_time', 'size', 'geometry'];

interface Inputs { quantity: number; material: string; geometry: string; size_mm: number; tolerance_mm: number; finish: number; strength: number; lead_days: number }

const PRESETS: { label: string; v: Inputs }[] = [
  { label: 'Iris frame · EVT (10)', v: { quantity: 10, material: 'plastic', geometry: 'enclosure', size_mm: 150, tolerance_mm: 0.2, finish: 2, strength: 2, lead_days: 10 } },
  { label: 'Iris frame · 1 000 units', v: { quantity: 1000, material: 'plastic', geometry: 'enclosure', size_mm: 150, tolerance_mm: 0.1, finish: 3, strength: 3, lead_days: 60 } },
  { label: 'Iris frame · 100 000', v: { quantity: 100000, material: 'plastic', geometry: 'enclosure', size_mm: 150, tolerance_mm: 0.1, finish: 3, strength: 3, lead_days: 90 } },
  { label: 'Aluminium hinge block (5)', v: { quantity: 5, material: 'metal', geometry: 'solid', size_mm: 20, tolerance_mm: 0.05, finish: 2, strength: 3, lead_days: 14 } },
  { label: 'Acrylic test fixture (2)', v: { quantity: 2, material: 'plastic', geometry: 'flat', size_mm: 120, tolerance_mm: 0.2, finish: 2, strength: 1, lead_days: 3 } },
  { label: 'Steel shield bracket (200)', v: { quantity: 200, material: 'metal', geometry: 'bent', size_mm: 40, tolerance_mm: 0.2, finish: 2, strength: 3, lead_days: 20 } },
];

export function WizardTab() {
  const [i, setI] = useState<Inputs>(PRESETS[0].v);
  const [w, setW] = useState<Weights>(DEFAULT_W);
  const r = useCompute<WizardOut>('build.wizard', { ...i, weights: w });
  const vd = useCompute<{ categories: Category[]; vendors: Vendor[] }>('build.vendors', {});
  const set = (p: Partial<Inputs>) => setI({ ...i, ...p });
  const vendorsFor = useMemo(() => (cats: string[]) => (vd.data?.vendors ?? []).filter((v) => !v.category_card && v.categories.some((c) => cats.includes(c))).slice(0, 5), [vd.data]);
  const o = r.data;
  const wsum = CRITERIA.reduce((s, k) => s + Math.max(0, w[k]), 0) || 1;
  const csv = () => ['rank,process,score,feasible,cost_index,cost_band,lead_time,' + CRITERIA.join(','), ...(o?.ranked ?? []).map((x, k) => [k + 1, x.process.name, x.score.toFixed(1), x.feasible, x.cost_index.toFixed(2), x.cost_band, x.lead_band, ...x.subs.map((s) => s.score.toFixed(2))].join(','))].join('\n');
  return (
    <div className="stack">
      <div className="row" style={{ gap: 6 }}>
        {PRESETS.map((p) => <button key={p.label} className="chip" style={{ cursor: 'pointer' }} onClick={() => setI(p.v)}>{p.label}</button>)}
      </div>
      <div className="grid grid-sidebar">
        <div className="stack">
          <Card title="The part">
            <Slider label="Quantity" value={i.quantity} min={1} max={1_000_000} log onChange={(v) => set({ quantity: Math.round(v) })} format={(v) => fmtNum(Math.round(v))} />
            <Select label="Material family" value={i.material} onChange={(v) => set({ material: v })} options={[{ value: 'plastic', label: 'Plastic' }, { value: 'elastomer', label: 'Elastomer (rubber-like)' }, { value: 'metal', label: 'Metal' }, { value: 'wood', label: 'Wood' }]} />
            <Select label="Geometry" value={i.geometry} onChange={(v) => set({ geometry: v })} options={[{ value: 'solid', label: 'Solid 3D part' }, { value: 'enclosure', label: 'Thin-wall housing / frame' }, { value: 'flat', label: 'Flat profile (2D)' }, { value: 'bent', label: 'Bent sheet (bracket, shield)' }]} />
            <Slider label="Largest dimension" unit="mm" value={i.size_mm} min={5} max={2000} log onChange={(v) => set({ size_mm: Math.round(v) })} format={(v) => v.toFixed(0)} />
            <Select label="Tolerance needed" value={String(i.tolerance_mm)} onChange={(v) => set({ tolerance_mm: Number(v) })} options={[0.5, 0.3, 0.2, 0.1, 0.05, 0.025].map((x) => ({ value: String(x), label: `± ${x} mm` }))} />
            <Slider label="Surface finish" value={i.finish} min={1} max={3} step={1} onChange={(v) => set({ finish: v })} format={(v) => ['', 'rough OK', 'standard', 'cosmetic'][Math.round(v)]} />
            <Slider label="Strength" value={i.strength} min={1} max={3} step={1} onChange={(v) => set({ strength: v })} format={(v) => ['', 'low', 'medium', 'high'][Math.round(v)]} />
            <Slider label="Lead time available" unit="days" value={i.lead_days} min={1} max={120} step={1} onChange={(v) => set({ lead_days: v })} format={(v) => v.toFixed(0)} />
          </Card>
          <Card title="Weights" subtitle="The score is a weighted mean — change what matters" actions={<button className="btn small" onClick={() => setW(DEFAULT_W)}>Reset</button>}>
            {CRITERIA.map((k, n) => (
              <div key={k} className="bd-weight">
                <span className="bd-swatch" style={{ background: seriesColor(n) }} />
                <Slider label={k.replace('_', ' ')} value={w[k]} min={0} max={5} step={0.5} onChange={(v) => setW({ ...w, [k]: v })} format={(v) => `${v} (${((100 * v) / wsum).toFixed(0)} %)`} />
              </div>
            ))}
          </Card>
        </div>
        <div className="stack">
          <ErrorBox error={r.error} />
          {o && (
            <Card title="Ranked processes" subtitle="Bar segments = weight × sub-score per criterion (colours match the weights)" actions={<DownloadButton filename="process-ranking.csv" content={csv} label="ranking.csv" small />}>
              <div className="stack" style={{ gap: 10 }}>
                {o.ranked.map((x, k) => (
                  <div key={x.process.id} className={`bd-rank ${x.feasible ? '' : 'off'} ${k === 0 ? 'top' : ''}`}>
                    <div className="bd-rank-head">
                      <span className="bd-rank-n">{k + 1}</span>
                      <b>{x.process.name}</b>
                      <span className="bd-rank-score tnum">{x.score.toFixed(0)}</span>
                      <Chip title="relative cost band at this quantity (indicative)">{x.cost_band}</Chip>
                      <Chip>{x.lead_band}</Chip>
                      {!x.feasible && <Chip tone="fail">✕ not feasible</Chip>}
                    </div>
                    <div className="bd-rank-bar" aria-label={`score ${x.score.toFixed(0)} of 100`}>
                      {x.subs.map((s, n) => <div key={s.criterion} title={`${s.criterion}: ${(s.score * 100).toFixed(0)} % × weight ${s.weight}`} style={{ width: `${x.feasible ? (100 * Math.max(0, s.weight) * s.score) / wsum : 0}%`, background: seriesColor(n) }} />)}
                    </div>
                    {x.blockers.length > 0 && <div className="small" style={{ color: 'var(--bad-text)' }}>{x.blockers.join('; ')}</div>}
                    {x.feasible && (
                      <>
                        <div className="small">{x.reasons.join(' · ')}</div>
                        {k < 3 && (
                          <div className="bd-send">
                            <span className="small muted">Upload: {x.process.upload} — made in <Link to={x.process.lab_route}>{x.process.lab_label}</Link>. Send to:</span>
                            {vendorsFor(x.process.vendor_categories).map((v) => <a key={v.name} className="btn small" href={v.url} target="_blank" rel="noreferrer">{v.name} ↗</a>)}
                          </div>
                        )}
                      </>
                    )}
                  </div>
                ))}
              </div>
              <div className="small muted" style={{ marginTop: 10 }}>{o.notes.join(' ')}</div>
            </Card>
          )}
        </div>
      </div>
      <Explainer title="How the ranking works" refs={[{ label: 'G. Boothroyd, P. Dewhurst, W. Knight, Product Design for Manufacture and Assembly (2010)' }, { label: 'M. F. Ashby, Materials Selection in Mechanical Design — process selection' }]}>
        Each process has a capability profile (quantity sweet spot, achievable tolerance, finish, strength, lead time, relative setup and unit cost, size and geometry fit). Each criterion gives a sub-score between 0 and 1; the score is their weighted mean. A process that cannot make the material or the geometry scores 0 whatever its other merits.
        <Formula>{'score = 100 · Σ wᵢ·sᵢ / Σ wᵢ\ncost(q) = setup / q + unit           (relative units, palm-sized part)\ns_cost = clamp(1 − log₁₀(cost / cheapest) / 2, 0, 1)\ns_qty = 1 inside the sweet spot, falling to 0 over 1.5 decades outside'}</Formula>
        The cost bands ($ … $$$$$) are indicative relative bands, not prices — use the Manufacturing Hub for the Iris cost model and real quotes for decisions.
      </Explainer>
    </div>
  );
}
