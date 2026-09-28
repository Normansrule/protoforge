// Legibility: font size in pixels → angular size → Snellen equivalent, and how much text fits.
import { useEffect, useRef, useState } from 'react';
import { Card, Chip, DownloadButton, Explainer, Formula, Slider, Stat } from '../../components/ui';
import { Plot } from '../../components/Plot';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { toCSV } from '../control/kit';
import type { Legibility } from './types';
import { REFS } from './util';

const SAMPLE = 'Turn right onto Harbor Blvd in 120 m. Meeting with Priya at 3 pm, room 4B. Low tide 16:05 at Point Fermin. The quick brown fox jumps over the lazy dog 0123456789. Battery 64 % - about 6 h left. Next bus 246 in 7 min. Captions: so the tide pools are best at low tide, around four this afternoon.';

const SHORT: Record<string, string> = { 'below acuity': '✕ unreadable', threshold: '✕ tiny', small: '! small', comfortable: '✓ good', large: '✓ large' };

function verdictTone(v: string) {
  return v === 'comfortable' || v === 'large' ? 'pass' : v === 'small' ? 'warn' : 'fail';
}

/** A letter "E" rasterised at `px` pixels tall, magnified to `scale` screen px per display px. */
function PixelE({ px, scale, color = '#ffffff' }: { px: number; scale: number; color?: string }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const c = ref.current;
    if (!c) return;
    const h = Math.max(1, Math.round(px));
    const w = Math.max(1, Math.round(h * 0.8));
    c.width = w + 2;
    c.height = h + 2;
    const g = c.getContext('2d');
    if (!g) return;
    g.fillStyle = '#000';
    g.fillRect(0, 0, c.width, c.height);
    g.fillStyle = color;
    // Snellen-style E: 5 strokes high, bars one stroke thick
    const s = h / 5;
    const r = (x: number, y: number, ww: number, hh: number) => g.fillRect(Math.round(1 + x), Math.round(1 + y), Math.max(1, Math.round(ww)), Math.max(1, Math.round(hh)));
    r(0, 0, s, h);
    r(0, 0, w, s);
    r(0, 2 * s, w * 0.85, s);
    r(0, 4 * s, w, s);
  }, [px, color]);
  return <canvas ref={ref} style={{ width: (Math.max(1, Math.round(px * 0.8)) + 2) * scale, height: (Math.max(1, Math.round(px)) + 2) * scale }} />;
}

export function LegibilityTab() {
  const project = useProject((s) => s.project);
  const o = project.design.optics;
  const [em, setEm] = useState(20);
  const sizes = [6, 8, 10, 12, 14, 16, 18, 20, 24, 28, 32, 40, 48, 56, 64];
  const res = useCompute<Legibility>('optics.legibility', { project, config: { sizes_px: sizes, cap_ratio: 0.7, advance_ratio: 0.6, line_ratio: 1.25 } });
  const preview = useRef<HTMLCanvasElement>(null);
  const l = res.data;
  const apx = l?.arcmin_per_px ?? 2;
  const cap = em * 0.7;
  const capArc = cap * apx;
  const cpl = Math.floor(o.px_w / Math.max(1, em * 0.6));
  const lines = Math.floor(o.px_h / Math.max(1, em * 1.25));
  const verdict = capArc < 5 ? 'below acuity' : capArc < 10 ? 'threshold' : capArc < 16 ? 'small' : capArc <= 40 ? 'comfortable' : 'large';
  useEffect(() => {
    const c = preview.current;
    if (!c) return;
    c.width = Math.min(2048, o.px_w);
    c.height = Math.min(2048, o.px_h);
    const g = c.getContext('2d');
    if (!g) return;
    g.fillStyle = '#000';
    g.fillRect(0, 0, c.width, c.height);
    g.font = `500 ${Math.round(em)}px ui-monospace, Menlo, monospace`;
    g.fillStyle = '#e6fff0';
    g.textBaseline = 'top';
    const words = SAMPLE.split(' ');
    let line = '';
    let y = Math.round(em * 0.3);
    const maxChars = cpl;
    for (const w of words) {
      const test = line ? `${line} ${w}` : w;
      if (test.length > maxChars && line) {
        g.fillText(line, 4, y);
        y += Math.round(em * 1.25);
        line = w;
        if (y + em > c.height) break;
      } else line = test;
    }
    if (line && y + em <= c.height) g.fillText(line, 4, y);
  }, [em, o.px_w, o.px_h, cpl]);
  if (!l) return <Card title="Legibility">Computing…</Card>;
  // screen scale: draw 20/20 E and the font's E at the same arcmin scale (3 screen px per arcmin)
  const S = 3;
  const csv = () => toCSV(['em_px', 'cap_px', 'cap_arcmin', 'snellen_20_over', 'chars_per_line', 'lines', 'verdict'], [l.rows.map((r) => r.em_px), l.rows.map((r) => r.cap_px), l.rows.map((r) => r.cap_arcmin), l.rows.map((r) => r.snellen_20_over), l.rows.map((r) => r.chars_per_line), l.rows.map((r) => r.lines), l.rows.map((r) => r.verdict)]);
  return (
    <div className="stack">
      <div className="op-stats">
        <Stat label="Cap height" value={capArc.toFixed(1)} unit="arcmin" tone={verdictTone(verdict)} note={`${cap.toFixed(1)} px × ${apx.toFixed(2)}′/px · ${verdict}`} />
        <Stat label="Snellen equivalent" value={`20/${(20 * capArc / 5).toFixed(0)}`} note="acuity needed to just read it" />
        <Stat label="Characters per line" value={cpl} tone={cpl >= 20 ? 'pass' : 'fail'} note={`${cpl >= 20 ? '✓' : '✕'} 20 for R-004's rationale · ${lines} lines`} />
        <Stat label="Smallest comfortable" value={l.min_em_16} unit="px em" note={`≥ 16′ cap height; 20′ needs ${l.min_em_20} px`} />
      </div>
      <div className="op-main">
        <div className="stack" style={{ minWidth: 0 }}>
          <Card title="Letters at the same angular scale" subtitle={`Every letter is drawn at ${S} screen pixels per arcminute, rasterised on the display's own pixel grid (${apx.toFixed(2)}′ per pixel).`}>
            <div className="op-letters">
              <div className="op-letter"><PixelE px={5 / apx} scale={S * apx} /><span>20/20 optotype<br />5′ → {(5 / apx).toFixed(1)} px</span></div>
              <div className="op-letter"><PixelE px={16 / apx} scale={S * apx} color="#b8f5cf" /><span>16′ minimum<br />{(16 / apx).toFixed(1)} px</span></div>
              <div className="op-letter"><PixelE px={cap} scale={S * apx} color="#7dffb2" /><span><b>your font</b><br />{cap.toFixed(1)} px = {capArc.toFixed(0)}′</span></div>
              <div className="op-letter"><PixelE px={60 / apx} scale={S * apx} color="#cfe9ff" /><span>1° letter<br />{(60 / apx).toFixed(0)} px</span></div>
            </div>
            <div className="small muted" style={{ marginTop: 8 }}>The 20/20 letter needs only {(5 / apx).toFixed(1)} display pixels — too few to draw an E. Pixel-limited, not eye-limited: this is what {l.ppd_center.toFixed(0)} pixels per degree means.</div>
          </Card>
          <Card title="Frame buffer preview" subtitle={`The full ${o.px_w} × ${o.px_h} display at 1 : 1 pixels, monospace at ${em} px`}>
            <canvas ref={preview} className="op-preview" />
          </Card>
          <Card title="Size vs angle vs text capacity" actions={<DownloadButton small filename="iris-legibility.csv" content={csv} label="legibility.csv" />}>
            <div className="grid grid-2">
              <Plot series={[{ name: 'cap height', x: l.rows.map((r) => r.em_px), y: l.rows.map((r) => r.cap_arcmin), color: 'var(--series-1)', width: 2.4 }]} height={220} xLabel="font em (px)" yLabel="cap height (arcmin)" hLines={[{ value: 5, label: '20/20 (5′)', color: 'var(--bad)' }, { value: 16, label: '16′ minimum', color: 'var(--warn)' }]} vLines={[{ value: em }]} legend={false} />
              <Plot series={[{ name: 'characters / line', x: l.rows.map((r) => r.em_px), y: l.rows.map((r) => r.chars_per_line), color: 'var(--series-3)', width: 2.4 }, { name: 'lines', x: l.rows.map((r) => r.em_px), y: l.rows.map((r) => r.lines), color: 'var(--series-2)' }]} height={220} xLabel="font em (px)" yLabel="count" yLog hLines={[{ value: 20, label: '20 chars', color: 'var(--muted)' }, { value: 2, label: '2 lines', color: 'var(--muted)' }]} vLines={[{ value: em }]} legend />
            </div>
          </Card>
        </div>
        <div className="stack">
          <Card title="Font">
            <Slider label="Font size (em)" value={em} min={6} max={64} step={1} unit="px" format={(v) => v.toFixed(0)} onChange={setEm} />
            <div className="table-wrap">
              <table className="table" style={{ fontSize: 12 }}>
                <thead><tr><th>em</th><th>cap</th><th>20/x</th><th>chars</th><th /></tr></thead>
                <tbody>
                  {l.rows.map((r) => (
                    <tr key={r.em_px} onClick={() => setEm(r.em_px)} style={{ cursor: 'pointer', background: r.em_px === em ? 'var(--accent-soft)' : undefined }}>
                      <td className="mono">{r.em_px}</td>
                      <td className="tnum">{r.cap_arcmin.toFixed(0)}′</td>
                      <td className="tnum">{r.snellen_20_over.toFixed(0)}</td>
                      <td className="tnum">{r.chars_per_line}×{r.lines}</td>
                      <td><Chip tone={verdictTone(r.verdict)} title={r.verdict}>{SHORT[r.verdict] ?? r.verdict}</Chip></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
        </div>
      </div>
      <div className="op-explain">
        <Explainer title="Angles, not points" refs={[REFS.hfes, REFS.smith]}>
          <Formula>{`cap height (arcmin) = cap (px) × 60 / PPD = ${cap.toFixed(1)} × ${apx.toFixed(2)} = ${capArc.toFixed(1)}′\nSnellen 20/20 letter = 5′ tall (1′ strokes) → your font ≈ 20/${(20 * capArc / 5).toFixed(0)}\ncharacters per line = ${o.px_w} / (0.6·em) = ${cpl}`}</Formula>
          <p>Text size on a head-mounted display is an angle: the same 20 px letter looks larger on a lower-PPD display. Reading comfortably needs letters well above the acuity limit — display ergonomics guidance (e.g. ANSI/HFES 100-2007) asks for roughly 16 arcmin minimum and 20–22 arcmin preferred.</p>
        </Explainer>
        <Explainer title="Iris tie-in">
          <p>
            R-004's rationale is “two lines of 20 characters”. At a comfortable {l.min_em_16}–{l.min_em_20} px em Iris fits {Math.floor(o.px_w / (l.min_em_20 * 0.6))} characters × {Math.floor(o.px_h / (l.min_em_20 * 1.25))} lines — plenty of margin,
            which is why the design could trade some field of view for pixels per degree (see <i>Design space</i>).
          </p>
        </Explainer>
      </div>
    </div>
  );
}
