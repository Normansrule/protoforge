// PCB fab capability DFM check: the routed Iris layout vs capability tiers.
import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Card, Chip, DownloadButton, ErrorBox, Explainer } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { Dfm, IpcClass, Tier } from './types';

const IDS = ['basic', 'standard', 'advanced', 'hdi'] as const;

function tone(s: string) {
  return s === 'pass' ? 'pass' : s === 'fail' ? 'fail' : s === 'warn' ? 'warn' : 'info';
}
function word(s: string) {
  return s === 'pass' ? '✓ ok' : s === 'fail' ? '✕ exceeds' : s === 'warn' ? '! check' : 'i info';
}

export function DfmCalc() {
  const project = useProject((s) => s.project);
  const tiers = useCompute<Tier[]>('build.pcb_tiers', {});
  const classes = useCompute<IpcClass[]>('build.pcb_classes', {});
  const [tier, setTier] = useState<string>(() => String((project.design.extra as Record<string, unknown>)?.pcb_tier ?? 'standard'));
  const all = [
    useCompute<Dfm>('build.pcb_dfm', { project, tier: IDS[0] }),
    useCompute<Dfm>('build.pcb_dfm', { project, tier: IDS[1] }),
    useCompute<Dfm>('build.pcb_dfm', { project, tier: IDS[2] }),
    useCompute<Dfm>('build.pcb_dfm', { project, tier: IDS[3] }),
  ];
  const cur = all[Math.max(0, IDS.indexOf(tier as (typeof IDS)[number]))].data;
  const err = all.find((a) => a.error)?.error ?? null;
  const md = () =>
    cur
      ? `# ${project.meta.name} — PCB DFM vs "${cur.tier.name}" tier\n\nTypical tier values — check your vendor's capabilities page.\n\n| Rule | Design | Tier limit | Status | Note |\n|---|---|---|---|---|\n${cur.items.map((i) => `| ${i.rule} | ${i.design.toFixed(3)} ${i.unit} | ${i.limit.toFixed(3)} | ${i.status} | ${i.note} |`).join('\n')}\n`
      : '';
  return (
    <div className="stack">
      <ErrorBox error={err} />
      <div className="bd-tiers">
        {all.map((a, k) => {
          const t = tiers.data?.[k];
          const d = a.data;
          return (
            <button key={IDS[k]} className={`bd-tier ${tier === IDS[k] ? 'active' : ''}`} aria-pressed={tier === IDS[k]} onClick={() => setTier(IDS[k])}>
              <div className="bd-tier-name">{t?.name ?? IDS[k]}</div>
              <div className="small muted mono">{t ? `${t.trace_mm}/${t.space_mm} mm · drill ${t.drill_mm} · ${t.max_layers} layers` : ''}</div>
              {d && <Chip tone={d.violations === 0 ? 'pass' : 'fail'}>{d.violations === 0 ? '✓ fits' : `✕ ${d.violations} rule${d.violations > 1 ? 's' : ''}`}</Chip>}
              {d && d.warnings > 0 && <Chip tone="warn">! {d.warnings}</Chip>}
            </button>
          );
        })}
      </div>
      {cur && (
        <div className="grid grid-sidebar">
          <Card title={`${cur.tier.name} tier`} subtitle={cur.tier.notes}>
            <table className="table">
              <tbody>
                <tr><td className="muted">Finishes</td><td>{cur.tier.finishes.join(', ')}</td></tr>
                <tr><td className="muted">Thickness</td><td>{cur.tier.min_thickness_mm}–{cur.tier.max_thickness_mm} mm</td></tr>
                <tr><td className="muted">Via-in-pad</td><td>{cur.tier.via_in_pad ? 'filled + capped' : 'no'}</td></tr>
                <tr><td className="muted">Impedance</td><td>{cur.tier.impedance}</td></tr>
                <tr><td className="muted">Mask web</td><td>{cur.tier.mask_web_mm} mm</td></tr>
              </tbody>
            </table>
            <div className="small" style={{ marginTop: 8 }}>
              Cheapest tier with no violations: <b>{all.find((x) => x.data?.cheapest_ok)?.data?.cheapest_ok ?? '—'}</b>. {cur.has_layout ? 'Measured on the routed layout from the PCB Studio.' : 'No layout — design rules only.'}
            </div>
            <div className="row" style={{ marginTop: 8 }}>
              <DownloadButton filename={`pcb-dfm-${cur.tier.id}.md`} content={md} label="DFM report (.md)" small />
              <Link className="btn small" to="/pcb">→ PCB Studio</Link>
            </div>
          </Card>
          <Card title="Rule by rule" subtitle="Design value vs the tier's typical limit — check your vendor's capabilities page">
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>Rule</th><th style={{ textAlign: 'right' }}>Design</th><th style={{ textAlign: 'right' }}>Tier</th><th>Status</th><th>From</th></tr></thead>
                <tbody>
                  {cur.items.map((i) => (
                    <tr key={i.rule}>
                      <td>{i.rule}{i.note && <div className="small muted">{i.note}</div>}</td>
                      <td className="tnum mono" style={{ textAlign: 'right' }}>{i.status === 'info' ? '' : `${Number(i.design.toFixed(3))} ${i.unit}`}</td>
                      <td className="tnum mono" style={{ textAlign: 'right' }}>{i.status === 'info' || i.unit === 'vias' ? '' : Number(i.limit.toFixed(3))}</td>
                      <td><Chip tone={tone(i.status)}>{word(i.status)}</Chip></td>
                      <td className="small muted">{i.source}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
        </div>
      )}
      {classes.data && (
        <Card title="Capability tier ≠ quality class" subtitle="The IPC performance class is ordered separately: it sets how strictly boards and assemblies are inspected and accepted">
          <div className="grid grid-3">
            {classes.data.map((c) => (
              <div key={c.class} className={`bd-rule ${c.class === 2 ? 'bd-class-pick' : ''}`} style={{ flexDirection: 'column', gap: 4 }}>
                <div className="row" style={{ gap: 6 }}><span className="bd-rank-n">{c.class}</span><b>Class {c.class} — {c.name}</b>{c.class === 2 && <Chip tone="info">Iris</Chip>}</div>
                <div className="small">{c.typical}</div>
                <div className="small muted">{c.effect}</div>
              </div>
            ))}
          </div>
          <div className="small muted" style={{ marginTop: 8 }}>Classes come from IPC-6011 / IPC-6012 (bare boards) and IPC-A-600 / IPC-A-610 (acceptability). Class 2 is the usual default; ask the fab and the assembler explicitly if you need Class 3.</div>
        </Card>
      )}
      <Explainer title="Capability tiers" refs={[{ label: 'IPC-2221 — Generic standard on printed board design' }, { label: 'IPC-6012 — Qualification and performance specification for rigid printed boards' }, { label: 'IPC-A-600 — Acceptability of printed boards' }]}>
        Board houses price by capability: finer traces and smaller drills need better imaging, drilling and plating control. The checker reads the finest track, via and hole sizes from the routed layout (PCB Studio), the clearance enforced by its design-rule check, the layer count and thickness, and flags anything the chosen tier cannot make. Warnings (solder-mask webs, thermal vias in exposed pads, surface finish for fine pitch) do not count as violations but affect assembly yield. The metric <code>build.pcb_capability_violations</code> uses the tier stored in <code>design.extra.pcb_tier</code> (default "standard").
      </Explainer>
    </div>
  );
}
