// RAM / flash budget of the production firmware on the nRF52840.
import { useMemo, useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat, Toggle } from '../../components/ui';
import { fmtNum } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { ResItem, ResourceParams, Resources } from './types';
import { Seg, fmtKB } from './util';

const GROUP_COLOR: Record<string, string> = {
  display: 'var(--series-1)',
  audio: 'var(--series-2)',
  sensors: 'var(--series-3)',
  stacks: 'var(--series-4)',
  kernel: 'var(--series-7)',
  ble: 'var(--series-5)',
  app: 'var(--series-1)',
  const: 'var(--series-3)',
};

function StackedBar({ items, capacityKb, markKb, markLabel, title }: { items: ResItem[]; capacityKb: number; markKb?: number; markLabel?: string; title: string }) {
  const groups = useMemo(() => {
    const m = new Map<string, number>();
    for (const it of items) m.set(it.group, (m.get(it.group) ?? 0) + it.bytes);
    return [...m.entries()].sort((a, b) => b[1] - a[1]);
  }, [items]);
  const total = items.reduce((s, x) => s + x.bytes, 0) / 1024;
  const [hover, setHover] = useState<string | null>(null);
  return (
    <div>
      <div className="row" style={{ justifyContent: 'space-between', marginBottom: 8 }}>
        <b className="small">{title}</b>
        <span className="mono small">{fmtNum(total)} / {fmtNum(capacityKb)} KB ({fmtNum((total / capacityKb) * 100)} %)</span>
      </div>
      <div className="pf-bar-wrap">
        <div className="pf-bar" role="img" aria-label={`${title}: ${fmtNum(total)} of ${fmtNum(capacityKb)} KB`}>
          {groups.map(([g, b]) => (
            <div key={g} title={`${g}: ${fmtKB(b)}`} onMouseEnter={() => setHover(g)} onMouseLeave={() => setHover(null)} style={{ width: `${(b / 1024 / capacityKb) * 100}%`, background: GROUP_COLOR[g] ?? 'var(--series-8)', opacity: hover && hover !== g ? 0.45 : 1 }} />
          ))}
        </div>
        {markKb !== undefined && (
          <div className="pf-bar-mark" style={{ left: `${Math.min(100, (markKb / capacityKb) * 100)}%` }}>
            <span>{markLabel}</span>
          </div>
        )}
      </div>
      <div className="pf-bar-axis"><span>0</span><span>{fmtNum(capacityKb / 2)} KB</span><span>{fmtNum(capacityKb)} KB</span></div>
      <div style={{ marginTop: 6 }}>
        {groups.map(([g, b]) => (
          <span key={g} className="pf-key"><i style={{ background: GROUP_COLOR[g] ?? 'var(--series-8)' }} />{g} {fmtKB(b)}</span>
        ))}
      </div>
    </div>
  );
}

function ItemTable({ items }: { items: ResItem[] }) {
  return (
    <div className="table-wrap">
      <table className="table">
        <thead><tr><th>Item</th><th style={{ textAlign: 'right' }}>Size</th><th>Basis</th><th>How</th></tr></thead>
        <tbody>
          {[...items].sort((a, b) => b.bytes - a.bytes).map((x) => (
            <tr key={x.name}>
              <td><span className="pf-key" style={{ margin: 0 }}><i style={{ background: GROUP_COLOR[x.group] ?? 'var(--series-8)' }} />{x.name}</span></td>
              <td className="mono" style={{ textAlign: 'right', whiteSpace: 'nowrap' }}>{fmtKB(x.bytes)}</td>
              <td>{x.basis === 'computed' ? <Chip tone="pass">✓ computed</Chip> : <Chip tone="warn">! assumption</Chip>}</td>
              <td className="small muted">{x.note}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function ResourcesTab() {
  const project = useProject((s) => s.project);
  const [over, setOver] = useState<Partial<ResourceParams>>({});
  const base = useCompute<Resources>('firmware.resources', { project });
  const params = base.data ? { ...base.data.params, ...over } : undefined;
  const res = useCompute<Resources>('firmware.resources', { project, params }, !!params);
  const r = res.data ?? base.data;
  const set = <K extends keyof ResourceParams>(k: K, v: ResourceParams[K]) => setOver((o) => ({ ...o, [k]: v }));
  const csv = () =>
    'section,item,group,bytes,basis,note\n' +
    [...(r?.ram ?? []).map((x) => ['RAM', x]), ...(r?.flash ?? []).map((x) => ['flash', x])]
      .map(([s, x]) => {
        const it = x as ResItem;
        return [s as string, it.name, it.group, it.bytes.toFixed(0), it.basis, it.note].map((c) => `"${String(c).replace(/"/g, '""')}"`).join(',');
      })
      .join('\n');
  return (
    <div className="stack">
      <ErrorBox error={res.error ?? base.error} />
      {r && params && (
        <>
          <div className="grid grid-4">
            <Stat label="RAM" value={fmtNum(r.ram_kb)} unit="KB" tone={r.ram_kb <= r.recommended_ram_kb ? 'pass' : r.ram_kb <= r.ram_capacity_kb ? 'warn' : 'fail'} note={`${fmtNum(r.ram_pct)} % of 256 KB · target ≤ 75 %`} />
            <Stat label="Flash image" value={fmtNum(r.flash_kb)} unit="KB" tone={r.flash_kb <= r.flash_budget_kb ? 'pass' : 'fail'} note={`${fmtNum(r.flash_pct_of_budget)} % of the ${fmtNum(r.flash_budget_kb)} KB slot`} />
            <Stat label="Framebuffer" value={fmtKB(r.ram.find((x) => x.name === 'HUD framebuffer')?.bytes ?? 0)} note={`${params.panel_w}×${params.panel_h} at ${params.fb_bpp} bpp${params.double_buffer ? ', ×2' : ''}`} />
            <Stat label="CPU load (busiest context)" value={fmtNum(r.cpu_load_pct)} unit="%" tone={r.cpu_load_pct <= 50 ? 'pass' : r.cpu_load_pct <= 80 ? 'warn' : 'fail'} note={`of a ${fmtNum(params.cpu_mhz)} MHz Cortex-M4F · live captions`} />
          </div>
          <div className="pf-main">
            <Card title="nRF52840 memory" subtitle="Bars fill against the chip's capacity; the dashed line is the recommended ceiling." actions={<DownloadButton small filename="iris-firmware-memory.csv" content={csv} label="CSV" />}>
              <StackedBar items={r.ram} capacityKb={r.ram_capacity_kb} markKb={r.recommended_ram_kb} markLabel="75 % ceiling" title="RAM (256 KB)" />
              <div style={{ height: 22 }} />
              <StackedBar items={r.flash} capacityKb={r.flash_capacity_kb} markKb={r.flash_budget_kb} markLabel={params.dfu ? 'one DFU slot' : 'flash minus storage'} title="Flash (1 MB)" />
              <ul className="small muted" style={{ margin: '14px 0 0', paddingLeft: 18 }}>{r.notes.map((n) => <li key={n}>{n}</li>)}</ul>
            </Card>
            <Card title="What if…">
              <div className="field">
                <span className="field-label small">Framebuffer bits per pixel</span>
                <Seg<number> ariaLabel="Bits per pixel" value={params.fb_bpp} onChange={(v) => set('fb_bpp', v)} options={[{ value: 1, label: '1 bpp (mono)' }, { value: 4, label: '4 bpp (16 greys)' }]} />
              </div>
              <Toggle label="Double-buffered display (tear-free)" value={params.double_buffer} onChange={(v) => set('double_buffer', v)} />
              <Toggle label="Reserve bootloader + second slot for over-the-air updates" value={params.dfu} onChange={(v) => set('dfu', v)} />
              <Slider label="Audio blocks in flight" value={params.audio_blocks} min={2} max={16} step={1} onChange={(v) => set('audio_blocks', Math.round(v))} />
              <Slider label="IMU batch period" value={params.imu_batch_ms} min={5} max={500} step={5} unit="ms" onChange={(v) => set('imu_batch_ms', v)} />
              <Slider label="Bluetooth stack RAM (assumption)" value={params.ble_ram_kb} min={8} max={64} step={1} unit="KB" onChange={(v) => set('ble_ram_kb', v)} />
              <Slider label="Bluetooth stack flash (assumption)" value={params.ble_flash_kb} min={60} max={300} step={5} unit="KB" onChange={(v) => set('ble_flash_kb', v)} />
              <Slider label="Code density (heuristic)" value={params.bytes_per_statement} min={4} max={20} step={1} unit="B/stmt" onChange={(v) => set('bytes_per_statement', v)} />
              <button className="btn small" onClick={() => setOver({})}>Reset</button>
            </Card>
          </div>
          <div className="grid grid-2">
            <Card title="RAM breakdown"><ItemTable items={r.ram} /></Card>
            <Card title="Flash breakdown"><ItemTable items={r.flash} /></Card>
          </div>
          <Card title="CPU load per task" subtitle="Worst context (live captions: audio streaming + full-rate display). load = rate × cycles / f_CPU; cycle counts are estimates to check with the DWT cycle counter on hardware.">
            <div className="pf-cpu" role="img" aria-label={`CPU load ${fmtNum(r.cpu_load_pct)} percent`}>
              {[...r.cpu].filter((t) => t.load_pct > 0).sort((a, b) => b.load_pct - a.load_pct).map((t, i) => (
                <div key={t.name} title={`${t.name}: ${fmtNum(t.load_pct)} %`} style={{ width: `${t.load_pct}%`, background: `var(--series-${(i % 8) + 1})` }} />
              ))}
            </div>
            <div className="pf-bar-axis"><span>0 %</span><span>25 %</span><span>50 %</span><span>75 %</span><span>100 % of {fmtNum(params.cpu_mhz)} MHz</span></div>
            <div className="table-wrap" style={{ marginTop: 10 }}>
              <table className="table">
                <thead><tr><th>Task</th><th style={{ textAlign: 'right' }}>Rate</th><th style={{ textAlign: 'right' }}>Cycles</th><th style={{ textAlign: 'right' }}>Load</th><th>Basis</th></tr></thead>
                <tbody>
                  {[...r.cpu].sort((a, b) => b.load_pct - a.load_pct).map((t) => {
                    const k = [...r.cpu].filter((x) => x.load_pct > 0).sort((a, b) => b.load_pct - a.load_pct).findIndex((x) => x.name === t.name);
                    return (
                      <tr key={t.name}>
                        <td><span className="pf-key" style={{ margin: 0 }}><i style={{ background: k >= 0 ? `var(--series-${(k % 8) + 1})` : 'var(--surface-3)' }} />{t.name}</span></td>
                        <td className="mono" style={{ textAlign: 'right' }}>{fmtNum(t.rate_hz)} Hz</td>
                        <td className="mono" style={{ textAlign: 'right' }}>{fmtNum(t.cycles)}</td>
                        <td className="mono" style={{ textAlign: 'right' }}>{fmtNum(t.load_pct)} %</td>
                        <td className="small muted">{t.basis}</td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
            <div className="grid grid-2" style={{ marginTop: 10 }}>
              <Slider label="Display refresh while rendering" value={params.ui_fps} min={1} max={30} step={1} unit="fps" onChange={(v) => set('ui_fps', v)} />
              <Slider label="BLE connection interval" value={params.conn_interval_ms} min={7.5} max={100} step={1.25} unit="ms" onChange={(v) => set('conn_interval_ms', v)} />
            </div>
            <Formula>{`load = Σ f_i · cycles_i / f_CPU = ${fmtNum(r.cpu.reduce((s, t) => s + t.mcycles_per_s, 0))} Mcycles/s / ${fmtNum(params.cpu_mhz)} MHz = ${fmtNum(r.cpu_load_pct)} %`}</Formula>
          </Card>
          <Card title="Threads and interrupts" subtitle="The Zephyr skeleton's scheduling plan (lower number = higher priority).">
            <div className="table-wrap">
              <table className="table">
                <thead><tr><th>Context</th><th>Priority</th><th>Stack</th><th>Rate</th><th>Work</th></tr></thead>
                <tbody>
                  {r.threads.map((t) => (
                    <tr key={t.name}><td style={{ fontWeight: 650 }}>{t.name}</td><td className="mono">{t.priority < 0 ? 'ISR' : t.priority}</td><td className="mono">{t.stack ? `${t.stack} B` : 'ISR stack'}</td><td className="small">{t.rate}</td><td className="small muted">{t.work}</td></tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
          <div className="grid grid-2">
            <Explainer title="Where the RAM goes" refs={[{ label: 'Nordic Semiconductor, nRF52840 Product Specification', href: 'https://docs.nordicsemi.com' }, { label: 'Zephyr Project: memory footprint tools (ram_report / rom_report)', href: 'https://docs.zephyrproject.org' }]}>
              Microcontrollers have no virtual memory: every buffer is carved out of 256 KB at link time. The framebuffer dominates — a full 640 × 400 frame is 32 000 bytes at 1 bit per pixel and 128 000 at 4 bits; double-buffering doubles that. Audio needs a few 20 ms blocks in flight so the direct-memory-access engine never waits for a thread. Keep a margin: stacks grow with features, and an overflow corrupts memory silently.
              <Formula>{`framebuffer = W·H·bpp/8 = ${params.panel_w}·${params.panel_h}·${params.fb_bpp}/8 = ${fmtNum((params.panel_w * params.panel_h * params.fb_bpp) / 8 / 1024)} KB\naudio = blocks · (f_s/50) · 2 B · mics`}</Formula>
            </Explainer>
            <Explainer title="Flash and over-the-air updates" refs={[{ label: 'MCUboot documentation (mcu-tools/mcuboot on GitHub)', href: 'https://github.com/mcu-tools/mcuboot' }]}>
              A product must be updatable in the field. The usual scheme keeps a bootloader plus two image slots: the new firmware downloads into the second slot over Bluetooth while the old one runs, and the bootloader swaps them after checking the signature. That halves the space one image may use — here {fmtNum(r.flash_budget_kb)} KB.
              <Formula>{'budget = (1024 − bootloader − storage) / 2'}</Formula>
            </Explainer>
          </div>
        </>
      )}
    </div>
  );
}
