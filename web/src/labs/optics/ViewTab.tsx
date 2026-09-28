// "What you see": a drawn street scene with the heads-up display overlaid at
// its true angular size and pixel resolution, washing out as the world brightens.
import { useEffect, useMemo, useRef, useState } from 'react';
import { Card, Chip, Explainer, Formula, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { Rng } from './rng';
import type { AnalyzeOut, ContrastSweep } from './types';
import { REFS, Seg } from './util';


type Content = 'nav' | 'captions' | 'notify';

const PRESETS: { label: string; nits: number }[] = [
  { label: 'Night street', nits: 10 },
  { label: 'Indoor', nits: 100 },
  { label: 'Overcast', nits: 2000 },
  { label: 'Sunny', nits: 5000 },
  { label: 'Snow', nits: 12000 },
];

/** Draw HUD content at the display's native resolution. */
function drawHud(c: HTMLCanvasElement, pxW: number, pxH: number, em: number, mode: Content) {
  const W = Math.max(16, Math.min(2048, pxW));
  const H = Math.max(16, Math.min(2048, pxH));
  c.width = W;
  c.height = H;
  const g = c.getContext('2d');
  if (!g) return;
  g.fillStyle = '#000';
  g.fillRect(0, 0, W, H);
  g.imageSmoothingEnabled = false;
  const pad = Math.round(em * 0.6);
  const font = (px: number, weight = 600) => `${weight} ${Math.max(4, Math.round(px))}px Inter, system-ui, sans-serif`;
  g.textBaseline = 'top';
  const wrap = (text: string, x: number, y: number, maxW: number, px: number, color: string, weight = 500) => {
    g.font = font(px, weight);
    g.fillStyle = color;
    const words = text.split(' ');
    let line = '';
    let yy = y;
    for (const w of words) {
      const test = line ? `${line} ${w}` : w;
      if (g.measureText(test).width > maxW && line) {
        g.fillText(line, x, yy);
        line = w;
        yy += px * 1.25;
      } else line = test;
    }
    if (line) g.fillText(line, x, yy);
    return yy + px * 1.25;
  };
  if (mode === 'nav') {
    // turn arrow
    const s = Math.min(H * 0.55, em * 3.2);
    const ax = pad, ay = pad;
    g.strokeStyle = '#7dffb2';
    g.lineWidth = Math.max(2, s * 0.14);
    g.lineCap = 'round';
    g.lineJoin = 'round';
    g.beginPath();
    g.moveTo(ax + s * 0.3, ay + s);
    g.lineTo(ax + s * 0.3, ay + s * 0.45);
    g.quadraticCurveTo(ax + s * 0.3, ay + s * 0.25, ax + s * 0.55, ay + s * 0.25);
    g.lineTo(ax + s * 0.9, ay + s * 0.25);
    g.stroke();
    g.beginPath();
    g.moveTo(ax + s * 0.72, ay + s * 0.07);
    g.lineTo(ax + s * 0.92, ay + s * 0.25);
    g.lineTo(ax + s * 0.72, ay + s * 0.43);
    g.stroke();
    g.font = font(em * 1.5, 750);
    g.fillStyle = '#ffffff';
    g.fillText('120 m', ax + s + pad, ay + s * 0.05);
    wrap('Turn right onto Harbor Blvd', ax + s + pad, ay + s * 0.05 + em * 1.7, W - (ax + s + 2 * pad), em, '#cfe9ff');
    g.font = font(em * 0.8, 500);
    g.fillStyle = '#9fb3c8';
    g.fillText('ETA 14:32 · 1.2 km · walk', pad, H - pad - em * 0.8);
  } else if (mode === 'captions') {
    g.font = font(em * 0.8, 700);
    g.fillStyle = '#7dffb2';
    g.fillText('MIRA', pad, pad);
    wrap('…so the tide pools near Point Fermin are best at low tide, around four this afternoon. Want to meet at the lighthouse?', pad, pad + em * 1.1, W - 2 * pad, em, '#ffffff');
  } else {
    g.fillStyle = '#3987e5';
    g.beginPath();
    g.arc(pad + em * 0.8, pad + em * 0.8, em * 0.8, 0, Math.PI * 2);
    g.fill();
    g.font = font(em * 0.9, 800);
    g.fillStyle = '#fff';
    g.fillText('A', pad + em * 0.5, pad + em * 0.35);
    g.font = font(em, 700);
    g.fillStyle = '#ffffff';
    g.fillText('Alex', pad + em * 2, pad);
    wrap('Running 5 min late — can you order me a latte?', pad + em * 2, pad + em * 1.3, W - pad * 2 - em * 2, em * 0.9, '#cfe9ff');
    g.font = font(em * 0.75, 500);
    g.fillStyle = '#9fb3c8';
    g.fillText('tap the temple to reply', pad + em * 2, H - pad - em * 0.8);
  }
}

/** The world: sky, harbour-town street in perspective, buildings, lamps, sun. */
function Scene({ nits, zoom }: { nits: number; zoom: number }) {
  const L = Math.log10(Math.max(1, nits));
  const dark = Math.max(0, Math.min(0.78, 0.98 - 0.3 * L));
  const wash = Math.max(0, Math.min(0.45, (L - 3.3) * 0.45));
  const buildings = useMemo(() => {
    const r = new Rng(7);
    const out: { x: number; w: number; h: number; hue: number; win: number }[] = [];
    let x = -20;
    while (x < 1020) {
      const w = 40 + r.next() * 90;
      if (x + w < 360 || x > 640) out.push({ x, w, h: 60 + r.next() * 170, hue: r.next(), win: Math.floor(r.next() * 4) });
      x += w + 4;
    }
    return out;
  }, []);
  const hy = 300; // horizon
  return (
    <svg viewBox="0 0 1000 600" preserveAspectRatio="xMidYMid slice" aria-hidden style={{ transform: `scale(${zoom})` }}>
      <defs>
        <linearGradient id="op-sky" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#4d86c9" />
          <stop offset="0.7" stopColor="#a9cbe8" />
          <stop offset="1" stopColor="#f2dcc0" />
        </linearGradient>
        <linearGradient id="op-road" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#5d6168" />
          <stop offset="1" stopColor="#3b3e44" />
        </linearGradient>
        <radialGradient id="op-sun" cx="0.5" cy="0.5" r="0.5">
          <stop offset="0" stopColor="#fffbe6" />
          <stop offset="0.35" stopColor="#fff2b3" stopOpacity={0.9} />
          <stop offset="1" stopColor="#fff2b3" stopOpacity={0} />
        </radialGradient>
        <linearGradient id="op-sea" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#6d9dc4" />
          <stop offset="1" stopColor="#3d6f97" />
        </linearGradient>
      </defs>
      <rect width="1000" height={hy} fill="url(#op-sky)" />
      <circle cx="820" cy="95" r="120" fill="url(#op-sun)" />
      {/* distant sea and hills */}
      <path d={`M0,${hy - 30} C200,${hy - 70} 300,${hy - 40} 420,${hy - 55} C560,${hy - 75} 700,${hy - 35} 1000,${hy - 50} L1000,${hy} L0,${hy} Z`} fill="#7c95a8" opacity={0.7} />
      <rect x="0" y={hy - 12} width="1000" height="12" fill="url(#op-sea)" />
      {buildings.map((b, i) => (
        <g key={i}>
          <rect x={b.x} y={hy - b.h} width={b.w} height={b.h + 2} fill={`hsl(${20 + b.hue * 30}, ${18 + b.hue * 20}%, ${58 + b.hue * 16}%)`} />
          {Array.from({ length: Math.floor(b.h / 26) }, (_, k) => (
            <rect key={k} x={b.x + 8} y={hy - b.h + 12 + k * 26} width={b.w - 16} height={10} fill="#2b3a4a" opacity={0.35 + (b.win % 2) * 0.1} />
          ))}
        </g>
      ))}
      {/* ground */}
      <rect x="0" y={hy} width="1000" height={600 - hy} fill="#8f8a80" />
      <polygon points={`470,${hy} 530,${hy} 900,600 100,600`} fill="url(#op-road)" />
      {[0, 1, 2, 3, 4, 5].map((k) => {
        const t0 = 0.08 + k * 0.17;
        const t1 = t0 + 0.08;
        const y0 = hy + (600 - hy) * t0 * t0;
        const y1 = hy + (600 - hy) * t1 * t1;
        const w0 = 2 + 12 * t0 * t0;
        const w1 = 2 + 12 * t1 * t1;
        return <polygon key={k} points={`${500 - w0},${y0} ${500 + w0},${y0} ${500 + w1},${y1} ${500 - w1},${y1}`} fill="#f1e6c2" opacity={0.85} />;
      })}
      {/* lamp posts */}
      {[0.25, 0.45, 0.7].map((t, k) => {
        const y = hy + (600 - hy) * t * t;
        const xl = 470 - (370 * (y - hy)) / (600 - hy) - 20;
        const xr = 1000 - xl;
        const h = 30 + 280 * t * t;
        return (
          <g key={k} stroke="#2e3238" strokeWidth={1 + 4 * t}>
            <line x1={xl} y1={y} x2={xl} y2={y - h} />
            <line x1={xr} y1={y} x2={xr} y2={y - h} />
          </g>
        );
      })}
      {/* palm */}
      <g transform="translate(150,300)">
        <path d="M0,0 C-4,-60 4,-120 -6,-170" stroke="#5a4632" strokeWidth={7} fill="none" />
        {[-150, -110, -60, -20, 20].map((a, i) => <path key={i} d={`M-6,-170 q${Math.cos((a * Math.PI) / 180) * 40},${Math.sin((a * Math.PI) / 180) * 30 - 10} ${Math.cos((a * Math.PI) / 180) * 70},${Math.sin((a * Math.PI) / 180) * 50 + 10}`} stroke="#3e6b3a" strokeWidth={6} fill="none" />)}
      </g>
      <rect width="1000" height="600" fill="#050810" opacity={dark} />
      <rect width="1000" height="600" fill="#ffffff" opacity={wash} />
    </svg>
  );
}

export function ViewTab() {
  const project = useProject((s) => s.project);
  const o = project.design.optics;
  const an = useCompute<AnalyzeOut>('optics.analyze', { project });
  const cs = useCompute<ContrastSweep>('optics.contrast', { project });
  const [nits, setNits] = useState(100);
  const [mode, setMode] = useState<Content>('nav');
  const [em, setEm] = useState(24);
  const [zoom, setZoom] = useState<1 | 2>(2);
  const SCENE_H_DEG = 100 / zoom;
  const SCENE_V_DEG = 60 / zoom;
  const hud = useRef<HTMLCanvasElement>(null);
  const loupe = useRef<HTMLCanvasElement>(null);
  const a = an.data;
  useEffect(() => {
    if (!hud.current) return;
    drawHud(hud.current, o.px_w, o.px_h, em, mode);
    const l = loupe.current;
    if (!l) return;
    const cw = Math.min(200, o.px_w), ch = Math.min(100, o.px_h);
    l.width = cw;
    l.height = ch;
    const g = l.getContext('2d');
    if (!g) return;
    g.imageSmoothingEnabled = false;
    g.drawImage(hud.current, 0, 0, cw, ch, 0, 0, cw, ch);
  }, [o.px_w, o.px_h, em, mode]);
  if (!a) return <Card title="What you see">Starting the engine…</Card>;
  const T = a.see_through_pct / 100;
  const cr = (a.image_nits + T * nits) / Math.max(1e-9, T * nits);
  const vis = Math.max(0.04, Math.min(1, 1 - Math.pow(cr, -1.3)));
  const verdict = cr >= 3 ? 'good' : cr >= 1.5 ? 'marginal' : 'washed out';
  const wPct = (a.hfov_deg / SCENE_H_DEG) * 100;
  const hPct = (a.vfov_deg / SCENE_V_DEG) * 100;
  // HUD centre sits 8° right and 6° below straight ahead (in front of the right eye); the zoom scales about that point
  const cx = 58, cy = 56;
  const left = cx - wPct / 2;
  const top = cy - hPct / 2;
  const needed3 = (2 * T * nits) / Math.max(1e-9, a.lens_transmission * o.combiner_reflectance);
  const apx = a.arcmin_per_px;
  return (
    <div className="stack">
      <div className="op-stats">
        <Stat label="Contrast ratio" value={`${cr.toFixed(cr < 10 ? 2 : 1)}:1`} tone={cr >= 3 ? 'pass' : cr >= 1.5 ? 'warn' : 'fail'} note={`${cr >= 3 ? '✓' : cr >= 1.5 ? '!' : '✕'} ${verdict} · (L_img + T·L_bg)/(T·L_bg)`} />
        <Stat label="Image luminance" value={a.image_nits.toFixed(0)} unit="nits" note={`${o.display_luminance_nits} × ${(a.lens_transmission * 100).toFixed(0)} % × R ${(o.combiner_reflectance * 100).toFixed(0)} %`} />
        <Stat label="World behind it" value={(T * nits).toFixed(0)} unit="nits" note={`${nits.toFixed(0)} nits × ${(T * 100).toFixed(0)} % see-through`} />
        <Stat label="Display needed for 3:1" value={needed3 >= 1e5 ? `${(needed3 / 1000).toFixed(0)}k` : needed3.toFixed(0)} unit="nits" note={needed3 > o.display_luminance_nits ? `${(needed3 / o.display_luminance_nits).toFixed(1)}× brighter than now` : '✓ already bright enough'} tone={needed3 > o.display_luminance_nits ? 'warn' : 'pass'} />
      </div>
      <div className="op-main">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card title="Through the right lens" subtitle={`A ${SCENE_H_DEG}° × ${SCENE_V_DEG}° slice of the view. The dashed frame is the ${a.hfov_deg.toFixed(1)}° × ${a.vfov_deg.toFixed(1)}° display field, drawn from the actual ${o.px_w} × ${o.px_h} frame buffer.`}>
            <div className="op-view" role="img" aria-label={`first-person view with heads-up display, contrast ${cr.toFixed(1)} to 1, ${verdict}`}>
              <Scene nits={nits} zoom={zoom} />
              <div className="op-combiner" style={{ left: `${left - 3}%`, top: `${top - 5}%`, width: `${wPct + 6}%`, height: `${hPct + 10}%`, background: `rgba(0,0,0,${(o.combiner_reflectance * 0.8).toFixed(2)})` }} />
              <canvas ref={hud} className="op-hud" style={{ left: `${left}%`, top: `${top}%`, width: `${wPct}%`, height: `${hPct}%`, opacity: vis }} />
              <div className="op-hud-frame" style={{ left: `${left}%`, top: `${top}%`, width: `${wPct}%`, height: `${hPct}%` }} />
              <div className="op-view-tag">{PRESETS.reduce((b, p) => (Math.abs(Math.log10(p.nits / nits)) < Math.abs(Math.log10(b.nits / nits)) ? p : b)).label} · {nits.toFixed(0)} nits</div>
              <div className="op-view-tag right">contrast {cr.toFixed(2)}:1 · {verdict}</div>
            </div>
            <div className="small muted" style={{ marginTop: 8 }}>
              Illustrative: an additive see-through display can only add light, so black pixels are transparent and contrast is set by how bright the world behind the combiner is.
              The darker patch is the combiner dimming the world by R = {(o.combiner_reflectance * 100).toFixed(0)} %.
            </div>
          </Card>
          {cs.data && (
            <Card title="Contrast vs background brightness" subtitle="Indoors the display is vivid; on a sunny street it all but disappears.">
              <Plot
                series={[{ name: 'contrast ratio', x: cs.data.background_nits, y: cs.data.contrast_ratio, color: 'var(--series-1)', width: 2.4 }, { name: 'now', x: [nits], y: [cr], scatter: true, color: 'var(--series-2)' }]}
                height={230}
                xLog
                yLog
                xLabel="background luminance (nits, cd/m²)"
                yLabel="contrast ratio"
                hLines={[{ value: 3, label: '3:1 comfortable', color: 'var(--good)' }, { value: 1.5, label: '1.5:1 barely visible', color: 'var(--bad)' }]}
                vLines={PRESETS.map((p) => ({ value: p.nits, label: p.label }))}
                legend={false}
              />
              <div className="small muted">Comfortable (3:1) up to ≈ {cs.data.bg_at_3.toFixed(0)} nits background; visible (1.5:1) up to ≈ {cs.data.bg_at_1_5.toFixed(0)} nits.</div>
            </Card>
          )}
        </div>
        <div className="stack">
          <Card title="World & content">
            <Slider label="Background brightness" value={nits} min={3} max={20000} log unit="nits" format={(v) => v.toFixed(0)} onChange={setNits} />
            <div className="row" style={{ gap: 6, marginBottom: 12 }}>
              {PRESETS.map((p) => <button key={p.label} className={`btn small ${Math.abs(p.nits - nits) < 1 ? 'primary' : ''}`} onClick={() => setNits(p.nits)}>{p.label}</button>)}
            </div>
            <div className="field"><span className="field-label small">Content</span>
              <Seg value={mode} onChange={setMode} options={[{ value: 'nav', label: 'Navigation' }, { value: 'captions', label: 'Captions' }, { value: 'notify', label: 'Message' }]} />
            </div>
            <div className="field"><span className="field-label small">View</span>
              <Seg value={zoom} onChange={setZoom} options={[{ value: 1, label: '100° wide' }, { value: 2, label: '50° (zoom ×2)' }]} />
            </div>
            <Slider label="Font size (em)" value={em} min={8} max={64} step={1} unit="px" onChange={setEm} format={(v) => v.toFixed(0)} />
            <div className="small muted">Cap height ≈ {(em * 0.7).toFixed(0)} px = {(em * 0.7 * apx).toFixed(0)} arcmin ({em * 0.7 * apx >= 16 ? '✓ comfortable' : '✕ small'}).</div>
          </Card>
          <Card title="Pixel loupe" subtitle={`Top-left of the frame buffer at 1 : 1 display pixels — each pixel spans ${apx.toFixed(2)} arcmin`}>
            <canvas ref={loupe} className="op-loupe" />
            <div className="small muted" style={{ marginTop: 6 }}>
              A 20/20 letter (5 arcmin) would be only {(5 / apx).toFixed(1)} pixels tall here — this display cannot draw text at the limit of vision; it needs ≈ {(16 / apx / 0.7).toFixed(0)} px fonts to be comfortable.
            </div>
          </Card>
        </div>
      </div>
      <div className="op-explain">
        <Explainer title="Luminance through the optics" refs={[REFS.hecht, REFS.kress]}>
          <p>An ideal magnifier conserves luminance (brightness per solid angle): the virtual image is as bright as the display, minus losses. The combiner reflects R of it to the eye and transmits 1 − R of the world:</p>
          <Formula>{`L_img = L_display · T_lens · R = ${o.display_luminance_nits} · ${a.lens_transmission} · ${o.combiner_reflectance} = ${a.image_nits.toFixed(0)} nits\nCR = (L_img + (1−R)·L_bg) / ((1−R)·L_bg)`}</Formula>
          <p>Raising R brightens the image but darkens the world — the see-through trade. Rules of thumb for see-through displays ask for roughly 1.5:1 to be visible and 3:1 or more for comfortable text.</p>
        </Explainer>
        <Explainer title="Iris tie-in" refs={[REFS.kress]}>
          <p>
            At {a.image_nits.toFixed(0)} nits Iris is excellent indoors (UC-2 captions in a meeting) but marginal on an overcast street and washed out in sun (UC-1 walking directions).
            Fixes: a brighter panel (the T-2 trade: micro-LED), a higher-R or polarising combiner, or photochromic / electrochromic dimming of the front lens. Dark text boxes are impossible on an additive display, so use bold, high-contrast glyphs.
            Brighter costs power: see the <a href="#/power">Power & Thermal Lab</a>.
          </p>
          <div className="row" style={{ marginTop: 6 }}><Chip>UC-1</Chip><Chip>UC-2</Chip><Chip>T-2</Chip></div>
        </Explainer>
      </div>
    </div>
  );
}
