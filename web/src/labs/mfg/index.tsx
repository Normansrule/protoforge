// Manufacturing Hub — cost vs quantity, vendors, order guides, the
// fabrication package and the road from prototype to mass production.
import { useEffect, useMemo, useState } from 'react';
import { LabPage } from '../../components/LabPage';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, NumberField, Select, Stat, Tabs, Toggle } from '../../components/ui';
import { Plot, fmtNum } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useMetricMap, useProject } from '../../state/project';
import { workerCall } from '../pcb/useWorker';
import { downloadZip } from '../pcb/zip';
import { Markdown } from './Markdown';
import { StackedCost } from './StackedCost';

interface Assumptions { [k: string]: number | string }
interface CostCurve {
  quantities: number[]; categories: string[]; per_unit: number[][]; total: number[]; frame_choice: string[];
  frame_options: { process: string; unit_usd: number[] }[]; crossover_qty: number | null; target_qty: number; target_unit_usd: number;
  target_breakdown: [string, number][]; bom_usd_at_target: number; assumptions: Assumptions; frame_volume_cm3: number; frame_mass_g: number;
  pcb_area_cm2: number; smd_joints: number; tht_joints: number; unique_parts: number; tooling_usd: number; notes: string[];
}
interface Vendor { name: string; url: string; categories: string[]; services: string; consumes: string[]; region: string }
interface Guide { id: string; title: string; markdown: string }
interface Stage { id: string; title: string; goal: string; checklist: string[]; milestones: string[] }
interface PackageFile { path: string; content: string; encoding: string }

const FIELDS: { group: string; items: [string, string, string?, number?][] }[] = [
  { group: 'Components', items: [['bom_floor_fraction', 'Price floor f', '× p₁', 0.05], ['bom_exponent', 'Learning exponent b', '', 0.01]] },
  { group: 'PCB & assembly', items: [['pcb_setup_usd', 'PCB setup', '$'], ['pcb_usd_per_cm2', 'PCB area price', '$/cm²', 0.005], ['panel_efficiency', 'Panel efficiency', '', 0.05], ['pcba_setup_usd', 'PCBA setup', '$'], ['stencil_usd', 'Stencil', '$'], ['usd_per_smd_joint', 'Per SMD joint', '$', 0.0005], ['usd_per_tht_joint', 'Per THT joint', '$', 0.005], ['usd_per_unique_part', 'Per unique part (loading)', '$']] },
  { group: 'Frame processes', items: [['fdm_filament_usd_kg', 'Filament', '$/kg'], ['fdm_machine_usd_h', 'Printer time', '$/h', 0.1], ['fdm_post_min', 'FDM post-processing', 'min'], ['fdm_rate_cm3_h', 'FDM deposition rate', 'cm³/h'], ['mjf_usd_cm3', 'MJF/SLS service', '$/cm³', 0.05], ['mjf_order_usd', 'MJF order overhead', '$'], ['urethane_master_usd', 'Urethane master + mould', '$', 100], ['urethane_mold_life', 'Castings per mould', ''], ['urethane_usd_cm3', 'Urethane resin', '$/cm³', 0.05], ['mold_setup_usd', 'Molding setup per order', '$', 50]] },
  { group: 'Assembly & yield', items: [['test_minutes', 'Test time', 'min', 0.5], ['packaging_usd', 'Packaging', '$', 0.5], ['yield_pct', 'Yield', '%', 1]] },
];

const CATS = [{ id: 'all', label: 'All' }, { id: 'pcb', label: 'PCB & assembly' }, { id: 'parts', label: 'Parts' }, { id: 'print', label: '3-D printing' }, { id: 'molding', label: 'Injection molding' }, { id: 'cnc', label: 'CNC & sheet' }];

function useChecklist(key: string) {
  const [done, setDone] = useState<Record<string, boolean>>(() => {
    try { return JSON.parse(localStorage.getItem(key) ?? '{}'); } catch { return {}; }
  });
  useEffect(() => { try { localStorage.setItem(key, JSON.stringify(done)); } catch { /* ignore */ } }, [key, done]);
  return [done, setDone] as const;
}

type Tab = 'cost' | 'vendors' | 'guides' | 'package' | 'road';

function frameDom(v: number[]): [number, number] {
  const pos = v.filter((x) => x > 0 && isFinite(x));
  return pos.length ? [Math.min(...pos) / 2, Math.max(...pos) * 1.5] : [1, 10];
}

export default function Lab() {
  const project = useProject((s) => s.project);
  const metrics = useMetricMap();
  const defaults = useCompute<Assumptions>('mfg.assumptions', {});
  const [edits, setEdits] = useState<Assumptions>({});
  const [usePrint, setUsePrint] = useState(true);
  const printH = metrics.get('print.total_time_h')?.value;
  const printG = metrics.get('print.filament_g')?.value;
  const assumptions = useMemo(() => {
    if (!defaults.data) return null;
    const a: Assumptions = { ...defaults.data, ...edits };
    if (usePrint && printH) a.print_time_h = printH;
    if (usePrint && printG) a.filament_g = printG;
    return a;
  }, [defaults.data, edits, usePrint, printH, printG]);
  const cost = useCompute<CostCurve>('mfg.cost', { project, assumptions }, !!assumptions);
  const vendors = useCompute<Vendor[]>('mfg.vendors', {});
  const guides = useCompute<Guide[]>('mfg.guides', project);
  const stages = useCompute<Stage[]>('mfg.stages', project);
  const [tab, setTab] = useState<Tab>('cost');
  const [cat, setCat] = useState('all');
  const [guide, setGuide] = useState('pcba');
  const [pkg, setPkg] = useState<PackageFile[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [done, setDone] = useChecklist(`protoforge.mfg.checklist.${project.meta.id}`);
  const c = cost.data;
  const target = project.requirements.find((r) => r.verification.metric === 'mfg.unit_cost_usd');

  const buildPackage = async (download: boolean) => {
    setBusy(true);
    setErr(null);
    try {
      const files = await workerCall<PackageFile[]>('mfg.package', project);
      setPkg(files);
      if (download) await downloadZip(`${project.meta.codename || project.meta.id || 'product'}-fab-package.zip`, files);
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <LabPage
      kicker="Production · Factory"
      title="Manufacturing Hub"
      subtitle={<>What will {project.meta.name} cost at 1, 1 000 or 10 000 units, which process wins when, who can make it, how to order — and one button that zips every file the factories need.</>}
      metricPrefixes={['mfg.']}
      actions={<button className="btn primary" disabled={busy} onClick={() => buildPackage(true)}>{busy ? 'Building package…' : '↓ Download fabrication package'}</button>}
    >
      <ErrorBox error={cost.error || err} />
      {c && (
        <div className="grid grid-4">
          <Stat label={`Unit cost at ${fmtNum(c.target_qty)}`} value={`$${c.target_unit_usd.toFixed(2)}`} tone={target ? (c.target_unit_usd <= target.verification.target ? 'pass' : 'fail') : undefined} note={target ? `requirement ≤ $${target.verification.target} · indicative` : 'indicative'} />
          <Stat label="Molding beats printing" value={c.crossover_qty ? fmtNum(c.crossover_qty) : '—'} unit="units" note={`frame tooling ≈ $${fmtNum(c.tooling_usd)}`} />
          <Stat label="Parts at target qty" value={`$${c.bom_usd_at_target.toFixed(2)}`} note="price-break model on qty-1 prices" />
          <Stat label="Board" value={`${c.smd_joints} joints`} note={`${c.unique_parts} unique parts · ${fmtNum(c.pcb_area_cm2)} cm²`} />
        </div>
      )}
      <Tabs<Tab> value={tab} onChange={setTab} tabs={[{ id: 'cost', label: 'Cost vs quantity' }, { id: 'vendors', label: 'Vendors' }, { id: 'guides', label: 'Order guides' }, { id: 'package', label: 'Fab package' }, { id: 'road', label: 'Prototype → production' }]} />

      {tab === 'cost' && c && assumptions && (
        <>
          <div className="grid grid-sidebar">
            <Card title="Assumptions" subtitle="Indicative defaults — edit to match your quotes" actions={<button className="btn small" onClick={() => setEdits({})}>Reset</button>}>
              <Select label="Frame process" value={String(assumptions.frame_process)} onChange={(v) => setEdits({ ...edits, frame_process: v })} options={[{ value: 'auto', label: 'Cheapest at each quantity' }, { value: 'fdm', label: 'FDM (in-house)' }, { value: 'mjf', label: 'MJF / SLS service' }, { value: 'urethane', label: 'Urethane casting' }, { value: 'molding', label: 'Injection molding' }]} />
              {printH ? <Toggle label={`FDM time from Print lab (${fmtNum(printH)} h)`} value={usePrint} onChange={setUsePrint} /> : <div className="small muted" style={{ marginBottom: 10 }}>FDM time estimated from part volume (Print lab estimate not available).</div>}
              <div style={{ maxHeight: 520, overflowY: 'auto', paddingRight: 4 }}>
                {FIELDS.map((g) => (
                  <div key={g.group}>
                    <div className="field-label" style={{ margin: '8px 0 6px', fontWeight: 700 }}>{g.group}</div>
                    <div className="grid grid-2" style={{ gap: 8 }}>
                      {g.items.map(([k, label, unit, step]) => (
                        <NumberField key={k} label={label} unit={unit} step={step ?? 1} value={Number(assumptions[k])} onChange={(v) => setEdits({ ...edits, [k]: v })} />
                      ))}
                    </div>
                  </div>
                ))}
                <div className="small muted" style={{ marginTop: 8 }}>Assembly time ({project.design.manufacturing.assembly_minutes} min) and labour rate (${project.design.manufacturing.labor_rate_usd_h}/h) come from the project; molding tool steel, material and cavities from the Molding Lab.</div>
              </div>
            </Card>
            <div className="stack">
              <Card title="Cost per unit vs quantity" subtitle="Stacked by category · hover for the breakdown · indicative estimates, get real quotes">
                <StackedCost quantities={c.quantities} categories={c.categories} perUnit={c.per_unit} target={c.target_qty} crossover={c.crossover_qty} frameChoice={c.frame_choice} height={330} />
              </Card>
              <Card title="Frame: which process when?" subtitle="Per frame set; fixed costs (moulds, tools, setups) amortised over the quantity">
                <Plot
                  series={c.frame_options.map((o) => ({ name: o.process, x: c.quantities.filter((_, i) => o.unit_usd[i] > 0), y: o.unit_usd.filter((v) => v > 0) }))}
                  height={240} xLog yLog yDomain={frameDom(c.frame_options.flatMap((o) => o.unit_usd))} xLabel="quantity" yLabel="USD per frame set"
                  vLines={c.crossover_qty ? [{ value: c.crossover_qty, label: `crossover ${fmtNum(c.crossover_qty)}` }] : []}
                />
              </Card>
            </div>
          </div>
          <div className="grid grid-2">
            <Card title={`Breakdown at ${fmtNum(c.target_qty)} units`} subtitle="Table view of the chart">
              <table className="table">
                <thead><tr><th>Category</th><th style={{ textAlign: 'right' }}>USD / unit</th><th style={{ textAlign: 'right' }}>Share</th></tr></thead>
                <tbody>
                  {c.target_breakdown.map(([k, v], i) => (
                    <tr key={k}><td><span style={{ display: 'inline-block', width: 10, height: 10, borderRadius: 3, background: `var(--series-${(i % 8) + 1})`, marginRight: 8 }} />{k}</td><td className="tnum" style={{ textAlign: 'right' }}>{v.toFixed(2)}</td><td className="tnum" style={{ textAlign: 'right' }}>{((100 * v) / c.target_unit_usd).toFixed(1)} %</td></tr>
                  ))}
                  <tr><td><b>Total</b></td><td className="tnum" style={{ textAlign: 'right' }}><b>{c.target_unit_usd.toFixed(2)}</b></td><td /></tr>
                </tbody>
              </table>
            </Card>
            <div className="stack">
              <Explainer title="The cost model" refs={[{ label: 'G. Boothroyd, P. Dewhurst, W. Knight, Product Design for Manufacture and Assembly' }, { label: 'T. P. Wright, "Factors affecting the cost of airplanes", 1936' }]}>
                Every category splits into a fixed cost (setup, stencil, tooling, mould) spread over the quantity and a variable cost per unit:
                <Formula>{'unit(q) = Σ fixed / q + Σ variable(q)\npart price(q) = p₁ · (f + (1 − f) · q^−b)\nmolding(q) = piece + tool · ⌈q / shot life⌉ / q'}</Formula>
                The crossover is where the molding curve (high fixed, low variable) dips under printing (no fixed, high variable).
              </Explainer>
              <div className="callout small">{c.notes.map((n, i) => <div key={i}>• {n}</div>)}</div>
              <DownloadButton filename="cost-vs-quantity.csv" content={() => ['Quantity,' + c.categories.join(',') + ',Total,Frame process', ...c.quantities.map((q, i) => [q, ...c.per_unit.map((col) => col[i].toFixed(4)), c.total[i].toFixed(4), c.frame_choice[i]].join(','))].join('\n')} label="cost-vs-quantity.csv" />
            </div>
          </div>
        </>
      )}

      {tab === 'vendors' && vendors.data && (
        <>
          <div className="row">{CATS.map((k) => <button key={k.id} className={`chip ${cat === k.id ? 'info' : ''}`} style={{ cursor: 'pointer' }} onClick={() => setCat(k.id)} aria-pressed={cat === k.id}>{k.label}</button>)}</div>
          <div className="grid grid-3">
            {vendors.data.filter((v) => cat === 'all' || v.categories.includes(cat)).map((v) => (
              <Card key={v.name} title={<a href={v.url} target="_blank" rel="noreferrer">{v.name} ↗</a>} subtitle={v.region}>
                <div className="small">{v.services}</div>
                <div className="small muted" style={{ margin: '8px 0 4px' }}>Upload from Protoforge:</div>
                <div className="row" style={{ gap: 6 }}>{v.consumes.map((f) => <Chip key={f}>{f}</Chip>)}</div>
              </Card>
            ))}
          </div>
          <div className="callout small">Facts only: names, websites and service categories. No prices or lead times are stored here — use each vendor's own quote page. Listing is not an endorsement.</div>
        </>
      )}

      {tab === 'guides' && guides.data && (
        <Card title="How to order" actions={<Select value={guide} onChange={setGuide} options={guides.data.map((g) => ({ value: g.id, label: g.title }))} />}>
          {guides.data.filter((g) => g.id === guide).map((g) => (
            <div key={g.id}>
              <Markdown src={g.markdown} />
              <DownloadButton filename={`order-${g.id}.md`} content={g.markdown} label={`order-${g.id}.md`} small />
            </div>
          ))}
        </Card>
      )}

      {tab === 'package' && (
        <Card title="Fabrication package" subtitle="Gerbers, drill, BOM/CPL, assembly drawing, molding DFM report, cost report, order guides, verification report and project.json — built by the engine's producer list (other labs plug in their STL / G-code)">
          <div className="row">
            <button className="btn primary" disabled={busy} onClick={() => buildPackage(true)}>{busy ? 'Building…' : '↓ Download fabrication package (.zip)'}</button>
            <button className="btn" disabled={busy} onClick={() => buildPackage(false)}>Preview contents</button>
          </div>
          {pkg && (
            <div className="table-wrap" style={{ marginTop: 12 }}>
              <table className="table">
                <thead><tr><th>File</th><th style={{ textAlign: 'right' }}>Size</th></tr></thead>
                <tbody>{pkg.map((f) => <tr key={f.path}><td className="mono">{f.path}</td><td className="tnum" style={{ textAlign: 'right' }}>{(f.content.length / 1024).toFixed(1)} kB</td></tr>)}</tbody>
              </table>
            </div>
          )}
        </Card>
      )}

      {tab === 'road' && stages.data && (
        <div className="stack">
          <div className="grid" style={{ gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))' }}>
            {stages.data.map((s, i) => {
              const n = s.checklist.filter((x) => done[`${s.id}:${x}`]).length;
              return (
                <Card key={s.id} title={<span><span className="muted mono">{i + 1}</span> {s.title}</span>} subtitle={s.goal}>
                  <div style={{ height: 6, background: 'var(--surface-3)', borderRadius: 3, marginBottom: 10 }}><div style={{ width: `${(100 * n) / s.checklist.length}%`, height: 6, background: 'var(--accent)', borderRadius: 3 }} /></div>
                  {s.checklist.map((x) => (
                    <label key={x} className="row small" style={{ alignItems: 'flex-start', marginBottom: 6, cursor: 'pointer' }}>
                      <input type="checkbox" checked={!!done[`${s.id}:${x}`]} onChange={(e) => setDone({ ...done, [`${s.id}:${x}`]: e.target.checked })} style={{ accentColor: 'var(--accent)', marginTop: 3 }} />
                      <span>{x}</span>
                    </label>
                  ))}
                  {s.milestones.length > 0 && <div className="row" style={{ gap: 4, marginTop: 6 }}>{s.milestones.map((m) => <Chip key={m} tone="info">{m}</Chip>)}</div>}
                </Card>
              );
            })}
          </div>
          <Explainer title="EVT, DVT, PVT">
            Hardware teams gate production in validation builds. <b>EVT</b> (engineering validation) asks whether the design works — a handful of assembled boards in printed frames. <b>DVT</b> (design validation) builds parts in production materials from soft tools and tortures them (drop, sweat, thermal, radio pre-scan). <b>PVT</b> (production validation) runs the real line with hard tools, fixtures and test jigs to prove yield. Only then does mass production start. Iris maps its project milestones onto these stages; checkboxes are saved in this browser.
          </Explainer>
        </div>
      )}
    </LabPage>
  );
}
