// Firmware generator: target selector, file tree + viewer, zip download,
// Wokwi and PlatformIO instructions, provenance and the architecture diagram.
import { useMemo, useState } from 'react';
import { Card, Code, ErrorBox, Explainer, Formula, StatusChip } from '../../components/ui';
import { fmtNum } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { downloadZip } from '../pcb/zip';
import { Architecture } from './Architecture';
import type { FwProject, Target } from './types';
import { Seg, usePersistent } from './util';

const TARGETS: { value: Target; label: string }[] = [
  { value: 'xiao_arduino', label: 'XIAO (Arduino)' },
  { value: 'esp32s3_wokwi', label: 'ESP32-S3 (Wokwi)' },
  { value: 'zephyr', label: 'Zephyr (custom board)' },
];

const CONTEXTS = ['idle', 'glance', 'navigate', 'caption'];
const DISPLAY = ['off', 'low', 'full'];
const RADIO = ['fast', 'slow'];

function PolicyTable({ fw }: { fw: FwProject }) {
  const nb = fw.soc_buckets;
  const tone = (d: number) => (d === 2 ? 'var(--series-1)' : d === 1 ? 'color-mix(in srgb, var(--series-1) 45%, var(--surface-2))' : 'var(--surface-3)');
  return (
    <div className="table-wrap">
      <table className="table">
        <thead>
          <tr><th>Battery</th>{CONTEXTS.map((c) => <th key={c}>{c}</th>)}</tr>
        </thead>
        <tbody>
          {Array.from({ length: nb }, (_, b) => nb - 1 - b).map((b) => (
            <tr key={b}>
              <td className="mono small">{Math.round((b / nb) * 100)}–{Math.round(((b + 1) / nb) * 100)} %</td>
              {CONTEXTS.map((_, c) => {
                const a = fw.policy[b * 4 + c] ?? 0;
                const d = a >> 1;
                const r = a & 1;
                return (
                  <td key={c}>
                    <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
                      <i style={{ width: 12, height: 12, borderRadius: 3, background: tone(d), display: 'inline-block', border: '1px solid var(--border)' }} />
                      <span className="small">{DISPLAY[d]} · {RADIO[r]}</span>
                    </span>
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

export function FirmwareTab() {
  const project = useProject((s) => s.project);
  const [target, setTarget] = usePersistent<Target>('pf.target', 'xiao_arduino');
  const res = useCompute<FwProject>('firmware.project', { project, target });
  const fw = res.data;
  const [sel, setSel] = useState<string | null>(null);
  const files = fw?.files ?? [];
  const current = files.find((f) => f.path === sel) ?? files.find((f) => /\.(ino|c)$/.test(f.path) && /main|sketch|iris_v0/.test(f.path)) ?? files[0];
  const tree = useMemo(() => {
    const groups = new Map<string, typeof files>();
    for (const f of files) {
      const i = f.path.lastIndexOf('/');
      const dir = i < 0 ? '' : f.path.slice(0, i);
      if (!groups.has(dir)) groups.set(dir, []);
      groups.get(dir)!.push(f);
    }
    return [...groups.entries()].sort(([a], [b]) => a.localeCompare(b));
  }, [files]);
  const totalLines = files.reduce((s, f) => s + f.content.split('\n').length, 0);
  const zipName = `iris-firmware-${target}.zip`;
  const diagram = files.find((f) => f.path === 'diagram.json');
  const sketch = files.find((f) => f.path === 'src/sketch.ino');
  return (
    <div className="stack">
      <div className="row" style={{ justifyContent: 'space-between' }}>
        <Seg<Target> ariaLabel="Firmware target" value={target} options={TARGETS} onChange={(t) => { setTarget(t); setSel(null); }} />
        {fw && (
          <button className="btn primary" onClick={() => downloadZip(zipName, files.map((f) => ({ path: `iris-${target}/${f.path}`, content: f.content })))}>
            ↓ {zipName}
          </button>
        )}
      </div>
      <ErrorBox error={res.error} />
      {!fw && !res.error && <Card title="Generating…"><div className="muted small">Training the power policy and emitting the project…</div></Card>}
      {fw && (
        <>
          <Card title={fw.title} subtitle={`${files.length} files · ${totalLines.toLocaleString()} lines, generated from the Iris project — you cannot compile here, so every API used is one its library documents; board specifics are marked "check".`}>
            <div className="pf-files">
              <div className="pf-tree" role="tree" aria-label="Generated files">
                {tree.map(([dir, fs]) => (
                  <div key={dir}>
                    <div className="pf-tree-dir">{dir || 'project root'}</div>
                    {fs.map((f) => (
                      <button key={f.path} role="treeitem" aria-selected={current?.path === f.path} className={`pf-tree-file ${current?.path === f.path ? 'active' : ''}`} onClick={() => setSel(f.path)}>
                        <span>{f.path.split('/').pop()}</span>
                        <small>{f.content.split('\n').length}</small>
                      </button>
                    ))}
                  </div>
                ))}
              </div>
              {current && <Code code={current.content} filename={current.path.split('/').pop()} maxHeight={520} />}
            </div>
          </Card>
          <div className="grid grid-2">
            <Card title="Where every number came from" subtitle="Regenerate after changing another lab — the firmware follows.">
              <div className="table-wrap">
                <table className="table">
                  <thead><tr><th>Lab</th><th>Item</th><th>Value</th><th>File</th></tr></thead>
                  <tbody>
                    {fw.provenance.map((p) => (
                      <tr key={p.item}><td className="small" style={{ fontWeight: 650 }}>{p.source}</td><td className="small">{p.item}</td><td className="mono small">{p.value}</td><td className="mono small muted">{p.file}</td></tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </Card>
            <Card title="Power manager lookup table" subtitle={`The RL Lab's Q-learning policy, exported as IRIS_POLICY[${fw.soc_buckets}][4] (predicted ${fmtNum(fw.policy_gain_pct)} % longer battery life than the fixed policy).`}>
              <PolicyTable fw={fw} />
              <div className="muted small" style={{ marginTop: 8 }}>
                Each cell is one byte, <span className="mono">display × 2 + radio</span>. The firmware reads the battery voltage, converts it to a state of charge with the Power Lab's open-circuit-voltage curve, picks the row, and uses the current screen as the column.
              </div>
            </Card>
          </div>
          {target === 'esp32s3_wokwi' && (
            <Card title="Open in Wokwi — run the firmware in your browser" subtitle="Wokwi simulates the ESP32-S3, the SSD1306, the MPU-6050 and the button; it does not simulate Bluetooth or the S3's I²S, so those are compiled out.">
              <div className="grid grid-2">
                <div>
                  <ol className="pf-ol">
                    <li>Open <a href="https://wokwi.com/projects/new/esp32" target="_blank" rel="noreferrer">wokwi.com → new ESP32 project</a> (Arduino template).</li>
                    <li>Replace <span className="mono">sketch.ino</span> with the generated <span className="mono">src/sketch.ino</span> (copy button on the right).</li>
                    <li>Open the <span className="mono">diagram.json</span> tab and paste the generated diagram.</li>
                    <li>Library Manager → add <b>U8g2</b> (or create <span className="mono">libraries.txt</span> with one line: <span className="mono">U8g2</span>).</li>
                    <li>Press ▶. Click the blue button or press <b>T</b>: tap, double-tap, hold. Click the MPU-6050 and drag <b>rotationZ</b>: the arrow stays world-locked.</li>
                  </ol>
                  <p className="muted small">Wokwi documentation: <a href="https://docs.wokwi.com" target="_blank" rel="noreferrer">docs.wokwi.com</a> (diagram.json format, ESP32 guide, libraries.txt).</p>
                </div>
                <div className="stack">
                  {diagram && <Code code={diagram.content} filename="diagram.json" maxHeight={220} />}
                  {sketch && <Code code={sketch.content.split('\n').slice(0, 30).join('\n') + '\n…'} fullCode={sketch.content} filename="sketch.ino" maxHeight={180} />}
                </div>
              </div>
            </Card>
          )}
          <div className="grid grid-2">
            <Card title="Build & flash" subtitle={target === 'zephyr' ? 'Zephyr west commands (Zephyr or nRF Connect SDK installed)' : 'PlatformIO commands (VS Code extension or the pio CLI)'}>
              <div className="pf-cmd">{fw.commands.join('\n')}</div>
              <div className="row" style={{ marginTop: 10 }}>
                <StatusChip status="warn" />
                <span className="small muted">Generated, not compiled here. First build: expect to fix a library version or two — the README in the zip lists exact board packages and libraries.</span>
              </div>
            </Card>
            <Explainer title="Fixed point: why the filters are integers" refs={[{ label: 'R. G. Lyons, Understanding Digital Signal Processing, 3rd ed., Prentice Hall 2010, ch. 12' }]}>
              A Q15 number stores x as the 16-bit integer round(x · 2¹⁵), so ±1 maps to ±32768 and one multiply-accumulate needs no floating-point unit. The biquad uses Q14 because its feedback coefficients reach ±2. Products are 32-bit, sums are kept in a 64-bit accumulator, then shifted back with rounding and saturated.
              <Formula>{'code = clamp(round(x · 2^15), −32768, 32767)\ny[n] = sat16((Σ h_k · x[n−k] + 2^14) >> 15)'}</Formula>
            </Explainer>
          </div>
          <Card title="Firmware architecture" subtitle="Interrupts do the least possible and hand work to threads through a queue; the three data paths from the brief are animated (click one to isolate it).">
            <Architecture />
          </Card>
        </>
      )}
    </div>
  );
}
