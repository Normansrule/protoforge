// Results of a schematic-editor simulation, plotted with the lab's shared
// Plot component: transient waveforms, Bode plot, DC sweep and the operating
// point, with probes defaulting to the nets the user marked on the drawing.
import { useEffect, useMemo, useState } from 'react';
import { Card, DownloadButton, Tabs } from '../../components/ui';
import { Plot, seriesColor, type Series } from '../../components/Plot';
import type { Extracted } from './schTypes';
import type { SimResult } from './types';
import { acCsv, dcCsv, opCsv, si, tranCsv } from './util';

function b64bytes(s: string): ArrayBuffer {
  const bin = atob(s);
  const u8 = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) u8[i] = bin.charCodeAt(i);
  return u8.buffer;
}
const f32 = (s: unknown): number[] => (typeof s === 'string' ? Array.from(new Float32Array(b64bytes(s))) : (s as number[]));
const f64 = (s: unknown): number[] => (typeof s === 'string' ? Array.from(new Float64Array(b64bytes(s))) : (s as number[]));

/** Decode the compact (base64) waveform arrays of `circuit.*` results. */
export function decodeSim(raw: SimResult): SimResult {
  const r: SimResult = { ...raw };
  if (raw.tran) r.tran = { ...raw.tran, time: f64(raw.tran.time), v: raw.tran.v.map(f32), i: raw.tran.i.map(f32) };
  if (raw.ac) r.ac = { freq: f64(raw.ac.freq), mag: raw.ac.mag.map(f32), phase: raw.ac.phase.map(f32) };
  if (raw.dc) r.dc = { ...raw.dc, sweep: f64(raw.dc.sweep), v: raw.dc.v.map(f32), i: raw.dc.i.map(f32) };
  return r;
}

type Tab = 'wave' | 'ac' | 'dc' | 'op';

export function SchResults({ res, extracted, frame, fileBase }: { res: SimResult; extracted: Extracted; frame: number | null; fileBase: string }) {
  const visibleNodes = res.nodes.filter((n) => !n.startsWith('_'));
  const labelled = extracted.nets.filter((n) => n.labels.length > 0 && n.name !== '0').map((n) => n.name);
  const defaults = useMemo(() => {
    const d = [...extracted.probes, ...labelled].filter((n, i, a) => a.indexOf(n) === i && visibleNodes.includes(n));
    return (d.length ? d : visibleNodes.slice(0, 4)).map((n) => `v(${n})`);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [extracted, res]);
  const [probes, setProbes] = useState<string[]>(defaults);
  useEffect(() => setProbes(defaults), [defaults]);
  const toggle = (p: string) => setProbes((s) => (s.includes(p) ? s.filter((x) => x !== p) : [...s, p]));
  const tabs: { id: Tab; label: string }[] = [];
  if (res.tran) tabs.push({ id: 'wave', label: 'Waveforms' });
  if (res.ac) tabs.push({ id: 'ac', label: 'Bode (AC)' });
  if (res.dc) tabs.push({ id: 'dc', label: 'DC sweep' });
  if (res.op) tabs.push({ id: 'op', label: 'Operating point' });
  const [tab, setTab] = useState<Tab>('wave');
  const tabNow = tabs.some((t) => t.id === tab) ? tab : tabs[0]?.id;
  const tr = res.tran;
  const now = tr && frame !== null ? [{ value: tr.time[Math.min(frame, tr.time.length - 1)], color: 'var(--accent)' }] : [];
  const series = (kind: 'v' | 'i'): Series[] => {
    if (!tr) return [];
    return probes.filter((p) => p.startsWith(kind + '(')).map((p) => {
      const name = p.slice(2, -1).toLowerCase();
      const arr = kind === 'v' ? tr.v[res.nodes.indexOf(name)] : tr.i[res.elements.findIndex((e) => e.name.toLowerCase() === name)];
      return arr ? { name: p, x: tr.time, y: arr, color: seriesColor(probes.indexOf(p)) } : null;
    }).filter(Boolean) as Series[];
  };
  const acSeries = (kind: 'mag' | 'phase'): Series[] => {
    if (!res.ac) return [];
    return probes.filter((p) => p.startsWith('v(')).map((p) => {
      const j = res.nodes.indexOf(p.slice(2, -1).toLowerCase());
      if (j < 0) return null;
      const y = kind === 'mag' ? res.ac!.mag[j].map((m) => 20 * Math.log10(Math.max(m, 1e-30))) : res.ac!.phase[j];
      return { name: p, x: res.ac!.freq, y, color: seriesColor(probes.indexOf(p)) };
    }).filter(Boolean) as Series[];
  };
  const dcSeries: Series[] = res.dc ? probes.filter((p) => p.startsWith('v(')).map((p) => {
    const j = res.nodes.indexOf(p.slice(2, -1).toLowerCase());
    return j >= 0 ? { name: p, x: res.dc!.sweep, y: res.dc!.v[j], color: seriesColor(probes.indexOf(p)) } : null;
  }).filter(Boolean) as Series[] : [];
  return (
    <Card
      title="Results"
      subtitle="Probes default to the nets you marked with a probe or a net label."
      actions={
        <div className="row" style={{ gap: 6 }}>
          {tabNow === 'wave' && tr && <DownloadButton small filename={`${fileBase}-tran.csv`} content={() => tranCsv(res)} label="tran.csv" />}
          {tabNow === 'ac' && res.ac && <DownloadButton small filename={`${fileBase}-ac.csv`} content={() => acCsv(res)} label="ac.csv" />}
          {tabNow === 'dc' && res.dc && <DownloadButton small filename={`${fileBase}-dc.csv`} content={() => dcCsv(res)} label="dc.csv" />}
          {tabNow === 'op' && res.op && <DownloadButton small filename={`${fileBase}-op.csv`} content={() => opCsv(res)} label="op.csv" />}
        </div>
      }
    >
      {tabs.length > 0 && <Tabs tabs={tabs} value={tabNow!} onChange={setTab} />}
      {tabNow !== 'op' && (
        <div className="cl-probes">
          <span className="small muted">Probes</span>
          {visibleNodes.map((n) => { const p = `v(${n})`; const on = probes.includes(p); return <button key={p} className={`cl-probe ${on ? 'on' : ''}`} style={on ? { borderColor: seriesColor(probes.indexOf(p)) } : undefined} onClick={() => toggle(p)} aria-pressed={on}>{p}</button>; })}
          {tabNow === 'wave' && res.elements.filter((e) => !e.name.toUpperCase().endsWith('CTL')).map((e) => { const p = `i(${e.name})`; const on = probes.includes(p); return <button key={p} className={`cl-probe cur ${on ? 'on' : ''}`} style={on ? { borderColor: seriesColor(probes.indexOf(p)) } : undefined} onClick={() => toggle(p)} aria-pressed={on}>{p}</button>; })}
        </div>
      )}
      {tabNow === 'wave' && tr && (
        <div className="stack" style={{ gap: 8 }}>
          {series('v').length > 0 && <Plot series={series('v')} height={260} xLabel="time (s)" yLabel="voltage (V)" vLines={now} legend />}
          {series('i').length > 0 && <Plot series={series('i')} height={200} xLabel="time (s)" yLabel="current (A)" vLines={now} legend />}
          {series('v').length === 0 && series('i').length === 0 && <div className="muted small">Pick a probe above.</div>}
        </div>
      )}
      {tabNow === 'ac' && res.ac && (
        <div className="stack" style={{ gap: 8 }}>
          <Plot series={acSeries('mag')} height={240} xLog xLabel="frequency (Hz)" yLabel="magnitude (dB)" hLines={[{ value: -3, label: '−3 dB', color: 'var(--muted)' }]} legend />
          <Plot series={acSeries('phase')} height={180} xLog xLabel="frequency (Hz)" yLabel="phase (°)" legend={false} />
        </div>
      )}
      {tabNow === 'dc' && res.dc && <Plot series={dcSeries} height={280} xLabel={`${res.dc.source} (V)`} yLabel="voltage (V)" legend />}
      {tabNow === 'op' && res.op && (
        <div className="grid grid-2">
          <div className="table-wrap">
            <table className="table">
              <thead><tr><th>Node</th><th>Voltage</th></tr></thead>
              <tbody>{res.nodes.map((n, k) => <tr key={n}><td className="mono">{n}</td><td className="tnum">{si(res.op!.node_v[k], 'V', 5)}</td></tr>)}</tbody>
            </table>
          </div>
          <div className="table-wrap">
            <table className="table">
              <thead><tr><th>Element</th><th>V</th><th>I</th><th>P</th></tr></thead>
              <tbody>{res.elements.map((e, k) => <tr key={e.name}><td><span className="mono">{e.name}</span> <span className="muted small">{e.value}</span></td><td className="tnum">{si(res.op!.elem_v[k], 'V', 4)}</td><td className="tnum">{si(res.op!.elem_i[k], 'A', 4)}</td><td className="tnum">{si(res.op!.elem_p[k], 'W', 3)}</td></tr>)}</tbody>
            </table>
          </div>
        </div>
      )}
      {res.warnings.length > 0 && <details className="small" style={{ marginTop: 8 }}><summary className="muted">{res.warnings.length} solver note{res.warnings.length > 1 ? 's' : ''}</summary><ul className="cl-warn">{res.warnings.map((w, k) => <li key={k}>{w}</li>)}</ul></details>}
    </Card>
  );
}
