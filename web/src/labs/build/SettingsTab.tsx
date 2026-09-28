// Settings library: pick a process and material → animated illustration,
// recommended windows as range bars, design rules, failures, downloads.
import { useEffect, useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, NumberField, Select } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { call } from '../../engine/engine';
import { useProject } from '../../state/project';
import { PlayButton, pct, usePlaying, useTicker } from './anim';
import { Diagram } from './diagrams';
import { ProcessAnim } from './illustrations';
import type { FileOut, GridOut, LaserMaterial, Process, ProjectCheck, Range } from './types';

const GROUP_ORDER = ['Additive', 'Subtractive', 'Forming', 'Electronics'];

export function RangeBar({ r, marker, markerLabel }: { r: Range; marker?: number; markerLabel?: string }) {
  const a = pct(r.lo, r.scale);
  const b = pct(r.hi, r.scale);
  const single = Math.abs(r.hi - r.lo) < 1e-12;
  const val = single ? `${r.lo} ${r.unit}` : `${r.lo}–${r.hi} ${r.unit}`;
  const inside = marker !== undefined && marker >= r.lo - 1e-9 && marker <= r.hi + 1e-9;
  return (
    <div className="bd-range">
      <div className="bd-range-top">
        <span className="bd-range-label">{r.label}</span>
        <span className="mono tnum">{val}</span>
      </div>
      <div className="bd-range-track" aria-label={`${r.label}: ${val}`}>
        <div className="bd-range-fill" style={{ left: `${a}%`, width: single ? 3 : `${Math.max(1, b - a)}%` }} />
        {marker !== undefined && (
          <div className={`bd-range-marker ${inside ? 'in' : 'out'}`} style={{ left: `${Math.max(0, Math.min(100, pct(marker, r.scale)))}%` }} title={markerLabel}>
            <span>{inside ? '✓' : '!'} {markerLabel}</span>
          </div>
        )}
      </div>
      <div className="bd-range-scale small muted"><span>{r.scale[0]}</span>{r.note ? <span className="bd-range-note">{r.note}</span> : null}<span>{r.scale[1]}</span></div>
    </div>
  );
}

function CheckRow({ c }: { c: ProjectCheck }) {
  return (
    <div className="bd-check">
      <Chip tone={c.status === 'pass' ? 'pass' : 'warn'}>{c.status === 'pass' ? '✓ ok' : '! check'}</Chip>
      <div>
        <div>{c.label}: <b className="tnum">{Number(c.value.toFixed(2))} {c.unit}</b> <span className="muted small">(typical {c.lo}{c.hi != null ? `–${c.hi}` : '+'} {c.unit})</span></div>
        <div className="small muted">{c.note}</div>
      </div>
    </div>
  );
}

function LaserTools() {
  const mats = useCompute<LaserMaterial[]>('build.laser_materials', {});
  const [g, setG] = useState({ rows: 5, cols: 5, power_min: 20, power_max: 100, speed_min: 5, speed_max: 25, speed_unit: 'mm/s', cell_mm: 10, gap_mm: 3, mode: 'cut', kerf_strip: true, title: 'LASER TEST' });
  const grid = useCompute<GridOut>('build.laser_grid', g);
  const [kerf, setKerf] = useState({ nominal_mm: 50, measured_mm: 48.5, slices: 10 });
  const k = useCompute<{ kerf_mm: number; offset_mm: number }>('build.kerf', kerf);
  const img = grid.data ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(grid.data.svg)}` : null;
  return (
    <>
      <Card title="Laser-safe materials" subtitle="If you cannot identify a plastic, do not put it in the laser">
        <div className="bd-laser-mats">
          {mats.data?.map((m) => (
            <div key={m.name} className={`bd-laser-mat ${m.verdict}`}>
              <Chip tone={m.verdict === 'safe' ? 'pass' : m.verdict === 'caution' ? 'warn' : 'fail'}>{m.verdict === 'safe' ? '✓ safe' : m.verdict === 'caution' ? '! caution' : '✕ never'}</Chip>
              <div><b>{m.name}</b><div className="small muted">{m.why}</div></div>
            </div>
          ))}
        </div>
      </Card>
      <div className="grid grid-sidebar">
        <Card title="Test-grid generator" subtitle="Rows = power, columns = speed; one colour per cell = one layer in your laser software">
          <div className="grid grid-2" style={{ gap: 8 }}>
            <NumberField label="Rows (power)" value={g.rows} step={1} onChange={(v) => setG({ ...g, rows: Math.max(1, Math.min(10, Math.round(v))) })} />
            <NumberField label="Columns (speed)" value={g.cols} step={1} onChange={(v) => setG({ ...g, cols: Math.max(1, Math.min(10, Math.round(v))) })} />
            <NumberField label="Power min" unit="%" value={g.power_min} onChange={(v) => setG({ ...g, power_min: v })} />
            <NumberField label="Power max" unit="%" value={g.power_max} onChange={(v) => setG({ ...g, power_max: v })} />
            <NumberField label="Speed min" unit={g.speed_unit} value={g.speed_min} onChange={(v) => setG({ ...g, speed_min: v })} />
            <NumberField label="Speed max" unit={g.speed_unit} value={g.speed_max} onChange={(v) => setG({ ...g, speed_max: v })} />
            <NumberField label="Cell" unit="mm" value={g.cell_mm} onChange={(v) => setG({ ...g, cell_mm: v })} />
            <Select label="Mode" value={g.mode} onChange={(v) => setG({ ...g, mode: v })} options={[{ value: 'cut', label: 'Cut (outlines)' }, { value: 'engrave', label: 'Engrave (fills)' }]} />
          </div>
          <ErrorBox error={grid.error} />
          {grid.data && (
            <div className="row" style={{ marginTop: 8 }}>
              <DownloadButton filename="laser-test-grid.svg" content={() => new Blob([grid.data!.svg], { type: 'image/svg+xml' })} label="laser-test-grid.svg" small primary />
              <DownloadButton filename="laser-test-grid-settings.csv" content={grid.data.csv} label="settings.csv" small />
            </div>
          )}
        </Card>
        <Card title="Preview" subtitle={grid.data ? `${grid.data.width_mm.toFixed(0)} × ${grid.data.height_mm.toFixed(0)} mm · black = label engraving, red = kerf strip` : ''}>
          {img && <img src={img} alt="Laser test grid preview" className="bd-grid-preview" />}
          <div className="grid grid-3" style={{ gap: 8, marginTop: 10 }}>
            <NumberField label="Strip length" unit="mm" value={kerf.nominal_mm} onChange={(v) => setKerf({ ...kerf, nominal_mm: v })} />
            <NumberField label="Measured slices" unit="mm" value={kerf.measured_mm} step={0.05} onChange={(v) => setKerf({ ...kerf, measured_mm: v })} />
            <NumberField label="Slices" value={kerf.slices} step={1} onChange={(v) => setKerf({ ...kerf, slices: Math.max(1, Math.round(v)) })} />
          </div>
          {k.data && <div className="small">Kerf = ({kerf.nominal_mm} − {kerf.measured_mm}) / {kerf.slices} = <b className="tnum">{k.data.kerf_mm.toFixed(3)} mm</b> → offset every path by <b className="tnum">{k.data.offset_mm.toFixed(3)} mm</b>.</div>}
        </Card>
      </div>
    </>
  );
}

export function SettingsTab() {
  const project = useProject((s) => s.project);
  const lib = useCompute<Process[]>('build.library', { project });
  const [pid, setPid] = useState('fdm');
  const [mid, setMid] = useState<string>('');
  const [playing, setPlaying] = usePlaying();
  const t = useTicker(playing);
  const [err, setErr] = useState<string | null>(null);
  const proc = lib.data?.find((p) => p.id === pid) ?? null;
  const frameMat = project.design.frame.material;
  useEffect(() => {
    if (!proc) return;
    const want = pid === 'fdm' ? proc.materials.find((m) => frameMat.toUpperCase().includes(m.id))?.id : pid === 'molding' ? project.design.manufacturing.molding_material : pid === 'pcb' ? 'standard' : undefined;
    setMid((cur) => (proc.materials.some((m) => m.id === cur) ? cur : want && proc.materials.some((m) => m.id === want) ? want : proc.materials[0]?.id ?? ''));
  }, [proc, pid, frameMat, project.design.manufacturing.molding_material]);
  const mat = proc?.materials.find((m) => m.id === mid) ?? proc?.materials[0];
  const groups = useMemo(() => GROUP_ORDER.map((g) => ({ g, items: (lib.data ?? []).filter((p) => p.group === g) })), [lib.data]);
  const pr = project.design.print;
  const markers: Record<string, number> = pid === 'fdm' && mat && frameMat.toUpperCase().includes(mat.id) ? { Nozzle: pr.nozzle_temp_c, Bed: pr.bed_temp_c } : {};

  const sheet = (format: 'md' | 'csv') => () => {
    try {
      const f = call<FileOut>('build.settings_sheet', { project, process: pid, material: mat?.id, format });
      return f.content;
    } catch (e) {
      setErr(String(e));
      return '';
    }
  };

  return (
    <div className="stack">
      <ErrorBox error={lib.error || err} />
      <div className="grid grid-sidebar">
        <Card title="Process">
          <div className="stack" style={{ gap: 10 }}>
            {groups.map(({ g, items }) => (
              <div key={g}>
                <div className="bd-gtitle">{g}</div>
                <div className="bd-proc-list">
                  {items.map((p) => (
                    <button key={p.id} className={`bd-proc ${pid === p.id ? 'active' : ''}`} aria-pressed={pid === p.id} onClick={() => setPid(p.id)}>{p.name}</button>
                  ))}
                </div>
              </div>
            ))}
          </div>
          {proc && proc.materials.length > 1 && (
            <div style={{ marginTop: 14 }}>
              <div className="bd-gtitle">{pid === 'pcb' ? 'Capability tier' : pid === 'electronics' ? 'Solder alloy' : 'Material'}</div>
              <div className="row" style={{ gap: 6 }}>
                {proc.materials.map((m) => (
                  <button key={m.id} className={`chip ${mat?.id === m.id ? 'info' : ''}`} style={{ cursor: 'pointer' }} aria-pressed={mat?.id === m.id} onClick={() => setMid(m.id)}>{m.name}</button>
                ))}
              </div>
            </div>
          )}
        </Card>
        {proc && mat && (
          <div className="stack">
            <Card
              title={`${proc.name} — ${mat.name}`}
              subtitle={proc.summary}
              actions={<PlayButton playing={playing} onChange={setPlaying} />}
            >
              <div className="bd-caveat">⚠ {proc.caveat}</div>
              <div className="bd-settings">
                <div className="bd-anim-wrap">
                  <ProcessAnim id={proc.anim} t={t} params={{ layer: pr.layer_height_mm, width: pr.line_width_mm, nozzle: pr.nozzle_mm, kerf: 0.2 }} />
                </div>
                <div>
                  {mat.ranges.map((r) => <RangeBar key={r.label} r={r} marker={markers[r.label]} markerLabel={markers[r.label] !== undefined ? `Iris ${markers[r.label]}` : undefined} />)}
                  <table className="table" style={{ marginTop: 6 }}>
                    <tbody>
                      {mat.facts.map((f) => <tr key={f.label}><td className="muted" style={{ width: '32%' }}>{f.label}</td><td>{f.value}</td></tr>)}
                    </tbody>
                  </table>
                  {mat.notes.map((n) => <div key={n} className="small" style={{ marginTop: 6 }}>{n}</div>)}
                </div>
              </div>
              <div className="row" style={{ marginTop: 10 }}>
                <DownloadButton filename={`${pid}-${(mat.id || 'all').toLowerCase()}-settings.md`} content={sheet('md')} label="Settings sheet (.md)" small primary />
                <DownloadButton filename={`${pid}-${(mat.id || 'all').toLowerCase()}-settings.csv`} content={sheet('csv')} label="Settings (.csv)" small />
                {proc.links.map((l) => <Link key={l.route + l.label} className="btn small" to={l.route}>→ {l.label}</Link>)}
              </div>
            </Card>
            {proc.project_checks.length > 0 && (
              <Card title={`Checked against ${project.meta.name}`} subtitle="Project values from the Print lab and CAD Studio vs the typical windows">
                {proc.project_checks.map((c) => <CheckRow key={c.label} c={c} />)}
              </Card>
            )}
          </div>
        )}
      </div>

      {proc && proc.rules.length > 0 && (
        <Card title="Design rules" subtitle="Typical — the machine or service's own guide wins">
          <div className="bd-rules">
            {proc.rules.map((r) => (
              <div key={r.title} className="bd-rule">
                <Diagram id={r.diagram} label={r.title} />
                <div>
                  <div className="bd-rule-title">{r.title}</div>
                  <div className="bd-rule-value">{r.value}</div>
                  <div className="small muted">{r.detail}</div>
                </div>
              </div>
            ))}
          </div>
        </Card>
      )}

      {proc && proc.failures.length > 0 && (
        <Card title="Common failures and fixes">
          <div className="bd-rules">
            {proc.failures.map((f) => (
              <div key={f.name} className="bd-rule">
                <Diagram id={f.diagram} label={f.name} />
                <div>
                  <div className="bd-rule-title">{f.name}</div>
                  <div className="small">{f.symptom}</div>
                  <div className="small muted" style={{ marginTop: 4 }}>Causes: {f.causes.join(' · ')}</div>
                  <ul className="bd-fixes">{f.fixes.map((x) => <li key={x}>{x}</li>)}</ul>
                </div>
              </div>
            ))}
          </div>
        </Card>
      )}

      {pid === 'laser' && <LaserTools />}

      {proc && (
        <div className="grid grid-2">
          {proc.tips.length > 0 && (
            <Card title="Notes">
              <ul className="bd-fixes">{proc.tips.map((x) => <li key={x}>{x}</li>)}</ul>
            </Card>
          )}
          <Explainer title={pid === 'fdm' ? 'Why these numbers?' : 'References'} refs={proc.refs.map((r) => ({ label: r, href: r.includes('help.prusa3d.com') ? 'https://help.prusa3d.com' : r.includes('wiki.bambulab.com') ? 'https://wiki.bambulab.com' : undefined }))}>
            {pid === 'fdm' ? (
              <>
                The nozzle must melt plastic as fast as the motion system asks for it. The volumetric flow is
                <Formula>{`Q = layer height × line width × speed = ${pr.layer_height_mm} × ${pr.line_width_mm} × ${pr.print_speed_mm_s} = ${(pr.layer_height_mm * pr.line_width_mm * pr.print_speed_mm_s).toFixed(1)} mm³/s`}</Formula>
                Temperature windows are where the polymer flows and bonds to the previous layer without degrading; bed temperature sits near the polymer's glass transition so the first layer stays soft enough to grip; cooling freezes overhangs but weakens layer bonds — which is why ABS, ASA and PC want little fan and an enclosure.
              </>
            ) : (
              <>Settings here are typical windows from the references below and makers' datasheets — they get you close; a calibration test on your machine gets you right.</>
            )}
          </Explainer>
        </div>
      )}
    </div>
  );
}
