// Transmission lines: microstrip, grounded coplanar waveguide and
// differential pairs on the Iris stack-up, with a cross-section field sketch.
import { useMemo, useState } from 'react';
import { Card, Chip, DownloadButton, ErrorBox, Explainer, Formula, Slider, Stat, Toggle } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { IrisOut, LineKind, LineOut } from './types';
import { fmt, toCsv, useIrisRf } from './util';
import { REFS } from './refs';

const KINDS: { id: LineKind; label: string; target: number }[] = [
  { id: 'cpwg', label: 'Coplanar waveguide with ground (CPWG)', target: 50 },
  { id: 'microstrip', label: 'Microstrip', target: 50 },
  { id: 'diff_microstrip', label: 'Differential pair (USB 90 Ω)', target: 90 },
];

export function LinesTab() {
  const project = useProject((s) => s.project);
  const [rf, patchRf] = useIrisRf();
  const iris = useCompute<IrisOut>('rf.iris', { project });
  const [kind, setKind] = useState<LineKind>('cpwg');
  const [width, setWidth] = useState(0.91);
  const [gap, setGap] = useState(0.2);
  const [irisStack, setIrisStack] = useState(true);
  const [h, setH] = useState(0.73);
  const [er, setEr] = useState(rf.er);
  const [tanD, setTanD] = useState(rf.tan_d);
  const [tUm, setTUm] = useState(rf.cu_um);
  const [f, setF] = useState(2.44);
  const [len, setLen] = useState(6);
  const [synth, setSynth] = useState(false);
  const target = KINDS.find((k) => k.id === kind)!.target;
  const input = useMemo(
    () => ({ project, iris_stack: irisStack, kind, width_mm: width, gap_mm: gap, h_mm: h, t_um: tUm, er, tan_d: tanD, f_ghz: f, length_mm: len, target_ohm: synth ? target : null }),
    [project, irisStack, kind, width, gap, h, tUm, er, tanD, f, len, synth, target],
  );
  const res = useCompute<LineOut>('rf.line', input);
  const o = res.data;
  const z = o ? (kind === 'diff_microstrip' ? o.zdiff_ohm : o.z0_ohm) : NaN;
  const ok = Math.abs(z - target) <= 0.1 * target;
  return (
    <div className="stack">
      <div className="rf-split">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card title="Cross-section" subtitle={o ? `${KINDS.find((k) => k.id === kind)!.label} · ${fmt(o.width_mm, 3)} mm on ${fmt(o.h_mm, 2)} mm, εr ${fmt(irisStack ? rf.er : er, 2)}` : ''}>
            <ErrorBox error={res.error} />
            {o && <CrossSection kind={kind} w={o.width_mm} g={o.gap_mm} h={o.h_mm} t={tUm / 1000} />}
          </Card>
          {o && (
            <div className="grid grid-4 rf-stats">
              <Stat label={kind === 'diff_microstrip' ? 'Differential Z' : 'Characteristic Z₀'} value={fmt(z, 1)} unit="Ω" tone={ok ? 'pass' : 'fail'} note={`${ok ? '✓' : '✕'} target ${target} Ω ±10 %${kind === 'diff_microstrip' ? ` · each line ${fmt(o.z0_ohm, 1)} Ω` : ''}`} />
              <Stat label="Effective permittivity" value={fmt(o.eeff, 3)} note={`λ in line ${fmt(o.wavelength_mm, 1)} mm`} />
              <Stat label={`${fmt(len, 1)} mm of line`} value={fmt(o.electrical_deg, 1)} unit="°" note={`delay ${fmt(o.delay_ps, 1)} ps`} />
              <Stat label="Loss" value={fmt(o.loss_db, 3)} unit="dB" note={`conductor ${fmt(o.alpha_c_db_m, 1)} + dielectric ${fmt(o.alpha_d_db_m, 1)} dB/m`} />
            </div>
          )}
          {o && (
            <Card title="Impedance vs width" subtitle="Same stack-up and gap; the dot is your trace">
              <Plot
                series={[
                  { name: kind === 'diff_microstrip' ? 'Z_diff' : 'Z₀', x: o.sweep_w, y: o.sweep_z },
                  { name: 'this trace', x: [o.width_mm], y: [z], scatter: true, color: 'var(--series-2)' },
                ]}
                height={230}
                xLog
                xLabel="trace width (mm)"
                yLabel="impedance (Ω)"
                hLines={[{ value: target, label: `${target} Ω` }]}
              />
              <DownloadButton small filename={`${kind}-z-vs-width.csv`} content={() => toCsv([['width_mm', 'z_ohm'], ...o.sweep_w.map((w, k) => [w, o.sweep_z[k]])])} label="CSV" />
            </Card>
          )}
        </div>
        <div className="stack rf-sticky">
          <Card title="Line">
            <div className="rf-presets" style={{ marginBottom: 10 }}>
              {KINDS.map((k) => (
                <button key={k.id} className={`rf-preset ${kind === k.id ? 'active' : ''}`} onClick={() => { setKind(k.id); setGap(k.id === 'diff_microstrip' ? 0.15 : 0.2); }} aria-pressed={kind === k.id}>
                  <b>{k.label}</b>
                </button>
              ))}
            </div>
            <Toggle label={`Synthesise the width for ${target} Ω`} value={synth} onChange={setSynth} />
            {!synth && <Slider label="Trace width w" value={width} min={0.05} max={4} step={0.01} unit="mm" onChange={setWidth} />}
            {kind !== 'microstrip' && <Slider label={kind === 'cpwg' ? 'Gap to the top ground s' : 'Pair spacing s'} value={gap} min={0.08} max={2} step={0.01} unit="mm" onChange={setGap} />}
            <Slider label="Line length" value={len} min={1} max={60} step={0.5} unit="mm" onChange={setLen} />
            <Slider label="Frequency" value={f} min={0.1} max={10} step={0.01} unit="GHz" onChange={setF} />
          </Card>
          <Card title="Stack-up">
            <Toggle label={`Iris stack-up (${project.design.pcb.layers}-layer, ${project.design.pcb.thickness_mm} mm → ${iris.data ? fmt(iris.data.h_mm, 2) : '…'} mm dielectric, εr ${rf.er})`} value={irisStack} onChange={setIrisStack} />
            {!irisStack && (
              <>
                <Slider label="Dielectric height h" value={h} min={0.05} max={3.2} step={0.01} unit="mm" onChange={setH} />
                <Slider label="Relative permittivity εr" value={er} min={1} max={12} step={0.05} onChange={setEr} />
              </>
            )}
            <Slider label="Loss tangent tan δ (FR-4 ≈ 0.015–0.02)" value={tanD} min={0} max={0.03} step={0.001} onChange={setTanD} />
            <Slider label="Copper thickness (1 oz ≈ 35 µm)" value={tUm} min={0} max={70} step={1} unit="µm" onChange={setTUm} />
            <div className="small muted">FR-4 permittivity is typically 4.3–4.5 at 2.4 GHz and varies by laminate and glass weave — ask the fab for its impedance-controlled stack-up.</div>
          </Card>
          {iris.data && (
            <Card title="Iris antenna feed" subtitle="U1 RF pin → matching network → AE1">
              <div className="stack" style={{ gap: 8 }}>
                <div className="small">
                  Now: {iris.data.current_feed.kind.toUpperCase()} {fmt(iris.data.current_feed.width_mm, 2)} mm / gap {fmt(iris.data.current_feed.gap_mm, 2)} mm <span className="muted">({iris.data.current_feed_source})</span>
                </div>
                <Chip tone={Math.abs(iris.data.current_feed_z0 - 50) <= 5 ? 'pass' : 'fail'}>
                  {Math.abs(iris.data.current_feed_z0 - 50) <= 5 ? '✓' : '✕'} rf.feed_z0_ohm = {fmt(iris.data.current_feed_z0, 1)} Ω
                </Chip>
                <div className="small">
                  Recommended: CPWG <b>{fmt(iris.data.recommended_feed.width_mm, 2)} mm</b> with <b>{fmt(iris.data.recommended_feed.gap_mm, 2)} mm</b> gaps → {fmt(iris.data.recommended_feed_z0, 1)} Ω, ground vias every ~1–2 mm along both sides.
                </div>
                <div className="row">
                  <button className="btn small primary" onClick={() => patchRf({ feed: iris.data!.recommended_feed })}>Use on Iris</button>
                  <button className="btn small" onClick={() => { setKind('cpwg'); setIrisStack(true); setSynth(false); setWidth(iris.data!.recommended_feed.width_mm); setGap(iris.data!.recommended_feed.gap_mm); }}>Show it here</button>
                  {rf.feed && <button className="btn small" onClick={() => patchRf({ feed: null })}>Back to as-routed</button>}
                </div>
                <div className="small muted">PCB Studio routes every signal at the minimum width; the RF net needs its own net class width and pour clearance for this to reach the copper.</div>
              </div>
            </Card>
          )}
        </div>
      </div>
      <div className="grid grid-2">
        <Explainer title="Microstrip: Hammerstad & Jensen" refs={[REFS.hj, REFS.pozar]}>
          A strip over a ground plane has part of its field in air, so it sees an effective permittivity between 1 and εr. With u = w/h:
          <Formula>{'Z₀₁(u) = η₀/(2π)·ln(f(u)/u + √(1 + 4/u²)),  f(u) = 6 + (2π − 6)·e^(−(30.666/u)^0.7528)'}</Formula>
          <Formula>{'εeff = (εr+1)/2 + (εr−1)/2·(1 + 10/u)^(−ab),   Z₀ = Z₀₁(u)/√εeff'}</Formula>
          Accurate to about 0.2 % for 0.01 ≤ w/h ≤ 100; strip thickness widens the effective strip. Loss: α_c ≈ R_s/(Z₀w), α_d = k₀εr(εeff − 1)tanδ / (2√εeff(εr − 1)).
        </Explainer>
        <Explainer title="CPWG and differential pairs" refs={[REFS.wadell]}>
          Grounded coplanar waveguide adds ground copper beside the strip; conformal mapping gives, with k = w/(w + 2s) and k₃ = tanh(πw/4h)/tanh(π(w + 2s)/4h):
          <Formula>{'Z₀ = 60π/√εeff · 1/(K(k)/K(k′) + K(k₃)/K(k₃′))'}</Formula>
          where K is the complete elliptic integral (computed by the arithmetic–geometric mean). CPWG lets a thin 2-layer board carry 50 Ω with a sub-millimetre trace. For a USB pair the IPC-2141 approximation Z_diff ≈ 2Z₀(1 − 0.48·e^(−0.96 s/h)) is a first cut (±10 %); Iris’s USB-C is power-only today, so this is for a future data-capable revision.
        </Explainer>
      </div>
    </div>
  );
}

function CrossSection({ kind, w, g, h, t }: { kind: LineKind; w: number; g: number; h: number; t: number }) {
  // drawing scale: fit ~ (w + 2g + 2·side) mm into 560 px
  const side = Math.max(1.2, 1.5 * h);
  const pair = kind === 'diff_microstrip';
  const total = pair ? 2 * w + g + 2 * side : w + 2 * g + 2 * side;
  const W = 560;
  const sc = W / total;
  const H0 = 60; // air above
  const hp = Math.max(40, h * sc);
  const H = H0 + hp + 40;
  const yTop = H0;
  const yGnd = H0 + hp;
  const tp = Math.max(3, t * sc);
  const cx = W / 2;
  const strips: [number, number][] = pair
    ? [[cx - g * sc / 2 - w * sc, cx - g * sc / 2], [cx + g * sc / 2, cx + g * sc / 2 + w * sc]]
    : [[cx - (w * sc) / 2, cx + (w * sc) / 2]];
  const lines: string[] = [];
  // E-field sketch: arcs from the strip(s) down to ground (and sideways to coplanar grounds)
  for (const [a, b] of strips) {
    const n = 5;
    for (let k = 0; k <= n; k++) {
      const x = a + ((b - a) * k) / n;
      lines.push(`M${x},${yTop} L${x},${yGnd}`);
    }
    lines.push(`M${a},${yTop - tp / 2} C${a - 30},${yTop - 10} ${a - 40},${yGnd - 10} ${a - 46},${yGnd}`);
    lines.push(`M${b},${yTop - tp / 2} C${b + 30},${yTop - 10} ${b + 40},${yGnd - 10} ${b + 46},${yGnd}`);
    lines.push(`M${a + 4},${yTop - tp} C${a - 10},${yTop - 50} ${a - 70},${yTop - 30} ${a - 90},${yGnd}`);
    lines.push(`M${b - 4},${yTop - tp} C${b + 10},${yTop - 50} ${b + 70},${yTop - 30} ${b + 90},${yGnd}`);
  }
  if (kind === 'cpwg') {
    const [a, b] = strips[0];
    lines.push(`M${a},${yTop - tp / 2} Q${a - (g * sc) / 2},${yTop - 14} ${a - g * sc},${yTop - tp / 2}`);
    lines.push(`M${b},${yTop - tp / 2} Q${b + (g * sc) / 2},${yTop - 14} ${b + g * sc},${yTop - tp / 2}`);
  }
  return (
    <svg viewBox={`0 0 ${W} ${H}`} width="100%" className="rf-xsec" role="img" aria-label={`cross-section of a ${kind} trace ${w.toFixed(2)} mm wide on ${h.toFixed(2)} mm dielectric`}>
      <rect x={0} y={yTop} width={W} height={hp} className="rf-xsec-diel" />
      <text x={8} y={yTop + 16} className="rf-xsec-t">FR-4  h = {h.toFixed(2)} mm</text>
      <rect x={0} y={yGnd} width={W} height={tp} className="rf-xsec-cu" />
      <text x={8} y={yGnd + tp + 14} className="rf-xsec-t">bottom ground plane</text>
      {lines.map((d, k) => <path key={k} d={d} className="rf-xsec-e flow-line" />)}
      {strips.map(([a, b], k) => (
        <g key={k}>
          <rect x={a} y={yTop - tp} width={b - a} height={tp} className="rf-xsec-cu" />
          <ellipse cx={(a + b) / 2} cy={yTop - tp / 2} rx={(b - a) / 2 + 14} ry={tp / 2 + 12} className="rf-xsec-h" />
          <text x={(a + b) / 2} y={yTop - tp - 16} textAnchor="middle" className="rf-xsec-t">{pair ? (k ? 'D−' : 'D+') : 'RF'} w = {w.toFixed(2)}</text>
        </g>
      ))}
      {kind === 'cpwg' && (
        <>
          <rect x={0} y={yTop - tp} width={strips[0][0] - g * sc} height={tp} className="rf-xsec-cu" />
          <rect x={strips[0][1] + g * sc} y={yTop - tp} width={W - strips[0][1] - g * sc} height={tp} className="rf-xsec-cu" />
          <rect x={strips[0][0] - g * sc - 16} y={yTop - tp} width={6} height={hp + tp} className="rf-xsec-via" />
          <rect x={strips[0][1] + g * sc + 10} y={yTop - tp} width={6} height={hp + tp} className="rf-xsec-via" />
          <text x={strips[0][0] - (g * sc) / 2} y={yTop + 30} textAnchor="middle" className="rf-xsec-t">s</text>
          <text x={strips[0][0] - g * sc - 20} y={yTop - tp - 6} textAnchor="end" className="rf-xsec-t">top ground + vias</text>
        </>
      )}
      {pair && <text x={cx} y={yTop - tp - 34} textAnchor="middle" className="rf-xsec-t">s = {g.toFixed(2)} mm</text>}
      <g className="rf-xsec-key">
        <line x1={W - 170} y1={14} x2={W - 150} y2={14} className="rf-xsec-e" />
        <text x={W - 145} y={18} className="rf-xsec-t">E field</text>
        <ellipse cx={W - 70} cy={14} rx={10} ry={6} className="rf-xsec-h" />
        <text x={W - 55} y={18} className="rf-xsec-t">H field</text>
      </g>
    </svg>
  );
}
