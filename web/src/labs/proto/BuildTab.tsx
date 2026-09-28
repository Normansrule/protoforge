// Breadboard build: animated wiring diagram, BOM, assembly checklist,
// differences from the custom board and a power comparison through the
// Power Lab's budget.
import { useState } from 'react';
import { Card, DownloadButton, ErrorBox, Explainer, Formula, Select, Slider, Stat } from '../../components/ui';
import { fmtNum } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { Breadboard, Build } from './types';
import { Seg, usePersistent } from './util';
import { BUILDS, WiringDiagram, useOledImage } from './Wiring';

const SCENARIOS = [
  { value: 'reference', label: 'Reference mixed use' },
  { value: 'navigation', label: 'Navigation walk (UC-1)' },
  { value: 'captions', label: 'All-day captions (UC-2)' },
  { value: 'notifications', label: 'Glanceable notifications (UC-3)' },
  { value: 'standby', label: 'Standby' },
  { value: 'display_on', label: 'Display always on' },
];

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

export function BuildTab(props: { onWiring?: () => void }) {
  const project = useProject((s) => s.project);
  const [build, setBuild] = usePersistent<Build>('pf.build', 'xiao');
  const [battery, setBattery] = useState(project.design.battery.capacity_mah);
  const [scenario, setScenario] = useState('reference');
  const res = useCompute<Breadboard>('firmware.breadboard', { project, build, battery_mah: battery, scenario });
  const oledImg = useOledImage();
  const [done, setDone] = usePersistent<number[]>(`pf.steps.${build}`, []);
  const [focus, setFocus] = useState<number | null>(null);
  const bb = res.data;
  const q = (c: string | number) => `"${String(c).replace(/"/g, '""')}"`;
  const bomCsv = () =>
    'item,qty,vendor category,stands in for,notes,search\n' +
    (bb?.bom ?? []).map((b) => [b.item, b.qty, b.category, b.stands_in_for, b.notes, b.search.map((l) => l.url).join(' ')].map(q).join(',')).join('\n');
  const checklist = () =>
    `# ${bb?.title ?? 'Iris v0'} — build checklist\n\n` + (bb?.steps ?? []).map((s, i) => `- [${done.includes(i) ? 'x' : ' '}] **${i + 1}. ${s.title}** — ${s.detail}`).join('\n') + '\n';
  const nDone = bb ? done.filter((d) => d < bb.steps.length).length : 0;
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
          options={BUILDS}
        />
        {bb && (
          <div className="row">
            <DownloadButton small filename={`iris-v0-${build}-bom.csv`} content={bomCsv} label="BOM CSV" />
            <DownloadButton small filename={`iris-v0-${build}-checklist.md`} content={checklist} label="Checklist" />
          </div>
        )}
      </div>
      <ErrorBox error={res.error} />
      {bb && (
        <>
          <div className="pf-main">
            <Card title={bb.title} subtitle={bb.summary}>
              <WiringDiagram key={build} bb={bb} focus={focus !== null ? bb.steps[focus]?.wires ?? [] : []} oledImg={oledImg} />
              {props.onWiring && (
                <button className="btn small" style={{ marginTop: 8 }} onClick={props.onWiring}>
                  Light up each bus and see the pin map →
                </button>
              )}
            </Card>
            <Card title="Build checklist" subtitle="Click a step to highlight its wires; tick it off when done (saved in this browser).">
              <div className="pf-progress" role="progressbar" aria-valuemin={0} aria-valuemax={bb.steps.length} aria-valuenow={nDone} aria-label="Steps done">
                <div style={{ width: `${(nDone / bb.steps.length) * 100}%` }} />
              </div>
              <div className="muted small" style={{ marginBottom: 8 }}>{nDone} of {bb.steps.length} steps done</div>
              <div className="pf-steps">
                {bb.steps.map((s, i) => (
                  <div
                    key={s.title}
                    className={`pf-step ${focus === i ? 'focus' : ''} ${done.includes(i) ? 'done' : ''}`}
                    onClick={() => setFocus(focus === i ? null : i)}
                    role="button"
                    tabIndex={0}
                    aria-pressed={focus === i}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') setFocus(focus === i ? null : i);
                    }}
                  >
                    <input
                      type="checkbox"
                      checked={done.includes(i)}
                      aria-label={`Mark step ${i + 1} done`}
                      onClick={(e) => e.stopPropagation()}
                      onChange={(e) => setDone(e.target.checked ? [...done, i] : done.filter((d) => d !== i))}
                    />
                    <div>
                      <div className="pf-step-title">{i + 1}. {s.title}{s.wires.length > 0 && <span className="pf-step-n">{s.wires.length} wire{s.wires.length > 1 ? 's' : ''}</span>}</div>
                      <div className="pf-step-detail">{s.detail}</div>
                    </div>
                  </div>
                ))}
              </div>
              {nDone === bb.steps.length && <div className="pf-done">✓ Iris v0 is built — now measure it and feed the numbers back into the Power Lab.</div>}
            </Card>
          </div>
          <Card title="Bill of materials (development boards and modules)" subtitle="Module names you can search for; links are distributor searches, not quotes — check availability and the exact variant.">
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>Item</th><th>Qty</th><th>Where (category)</th><th>Stands in for</th><th>Notes</th><th>Search</th></tr></thead>
                <tbody>
                  {bb.bom.map((b) => (
                    <tr key={b.item}>
                      <td style={{ fontWeight: 600 }}>{b.item}</td>
                      <td className="mono">{b.qty}</td>
                      <td className="small">{b.category}</td>
                      <td className="small mono">{b.stands_in_for}</td>
                      <td className="small muted">{b.notes}</td>
                      <td className="small" style={{ whiteSpace: 'nowrap' }}>
                        {b.search.length ? b.search.map((l) => <a key={l.url} href={l.url} target="_blank" rel="noreferrer" style={{ marginRight: 8 }}>{l.label} ↗</a>) : <span className="muted">—</span>}
                      </td>
                    </tr>
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
            <Explainer title="Why a development board first" refs={[{ label: 'Seeed Studio wiki: XIAO nRF52840 (Sense)', href: 'https://wiki.seeedstudio.com' }, { label: 'Nordic Semiconductor, nRF52840 DK user guide', href: 'https://docs.nordicsemi.com' }, { label: 'E. White, Making Embedded Systems, 2nd ed., O\'Reilly 2024' }]}>
              A breadboard with a development board lets the firmware, the phone app and the user interface be exercised weeks before a custom board exists — and every bug found here is one that does not cost a board re-spin. The XIAO nRF52840 Sense is unusually close to Iris: the same Bluetooth system-on-chip family, an inertial measurement unit (IMU), a pulse-density-modulation (PDM) microphone and a LiPo charger on a 21 × 17.5 mm module. The nRF52840 DK build uses the exact pins of the custom board, so the production (Zephyr) firmware moves over unchanged.
            </Explainer>
            <Explainer title="From breadboard to product" refs={[{ label: 'E. White, Making Embedded Systems, 2nd ed., O\'Reilly 2024' }]}>
              The usual path: (1) a breadboard on development boards proves the functions and the firmware architecture; (2) the DK with breakouts on the final pin map runs the production firmware; (3) the custom board from the PCB Lab replaces the DK one-to-one; (4) the enclosure from the CAD and Molding labs makes it wearable. Each step keeps the firmware and changes as little hardware as possible.
              <Formula>{'breadboard → DK on the final pin map → custom PCB → enclosure → pilot run'}</Formula>
            </Explainer>
          </div>
        </>
      )}
    </div>
  );
}
