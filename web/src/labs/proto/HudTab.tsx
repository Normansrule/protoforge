// Heads-up display simulator: the engine renders the same framebuffer the
// firmware draws; it is shown over a street scene at the Optics Lab's field
// of view, animated, and driven live by the gesture state machine.
import { memo, useEffect, useMemo, useRef, useState } from 'react';
import { Card, Explainer, Formula, Slider, Stat, Toggle } from '../../components/ui';
import { Plot, fmtNum } from '../../components/Plot';
import { tryCall } from '../../engine/engine';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { useFrame, useReducedMotion } from '../dsp/viz';
import type { DisplayBudget, DisplayParams, GestureOut, HudOut, HudState, Screen, ScreenDesc } from './types';
import { DEFAULT_HUD, PlayControls, SCREENS, Seg, decodeB64, fbToImage } from './util';

interface OpticsA { hfov_deg: number; vfov_deg: number; ppd: number; arcmin_per_px: number; virtual_image_m: number; eyebox_mm: number }

const CAPTION_SCRIPT =
  'So the question for today is how a heads up display can stay readable in daylight. The combiner reflects about thirty percent of the display light, so the micro OLED has to be bright, and the power policy decides when that brightness is worth the battery.'.split(' ');

/** Mirror of hud::wrap line counting (for the scroll-in animation only). */
function wrapCount(t: string, cpl: number): number {
  let i = 0;
  let n = 0;
  const L = t.length;
  cpl = Math.max(1, cpl);
  while (i < L) {
    while (i < L && t[i] === ' ') i++;
    if (i >= L) break;
    const start = i;
    let j = i;
    let col = 0;
    let last = -1;
    while (j < L && t[j] !== '\n' && col < cpl) {
      if (t[j] === ' ') last = j;
      j++;
      col++;
    }
    let end = j;
    if (j < L && t[j] !== '\n' && t[j] !== ' ' && last > start) end = last;
    n++;
    i = end;
    if (i < L && t[i] === '\n') i++;
  }
  return n;
}

function drawScene(ctx: CanvasRenderingContext2D, W: number, H: number, yawDeg: number, camFov: number) {
  const horizon = H * 0.55;
  const sky = ctx.createLinearGradient(0, 0, 0, horizon);
  sky.addColorStop(0, '#6fa6d6');
  sky.addColorStop(1, '#cfe3ef');
  ctx.fillStyle = sky;
  ctx.fillRect(0, 0, W, horizon);
  const ground = ctx.createLinearGradient(0, horizon, 0, H);
  ground.addColorStop(0, '#8a8f86');
  ground.addColorStop(1, '#4d5048');
  ctx.fillStyle = ground;
  ctx.fillRect(0, horizon, W, H - horizon);
  const pxPerDeg = W / camFov;
  const shift = -yawDeg * pxPerDeg;
  // buildings (parallax with head yaw)
  const blocks = [[-40, 0.2, 0.34], [-28, 0.12, 0.22], [-18, 0.1, 0.4], [18, 0.14, 0.3], [30, 0.1, 0.46], [44, 0.16, 0.26], [60, 0.12, 0.35], [-60, 0.14, 0.3]];
  for (const [deg, wf, hf] of blocks) {
    const x = W / 2 + deg * pxPerDeg + shift;
    const w = wf * W;
    const h = hf * H;
    ctx.fillStyle = deg < 0 ? '#7b8794' : '#8e8a80';
    ctx.fillRect(x - w / 2, horizon - h, w, h);
    ctx.fillStyle = 'rgba(255,255,255,0.35)';
    for (let r = 0; r < 5; r++) for (let c = 0; c < 3; c++) ctx.fillRect(x - w / 2 + 8 + c * (w / 3.2), horizon - h + 10 + r * (h / 5.5), w / 8, h / 12);
  }
  // road with a right turn ahead
  const vx = W / 2 + shift;
  ctx.fillStyle = '#3a3c3a';
  ctx.beginPath();
  ctx.moveTo(vx - 8, horizon);
  ctx.lineTo(vx + 8, horizon);
  ctx.lineTo(W * 0.5 + W * 0.38 + shift * 0.2, H);
  ctx.lineTo(W * 0.5 - W * 0.38 + shift * 0.2, H);
  ctx.closePath();
  ctx.fill();
  ctx.strokeStyle = '#e8e2c8';
  ctx.setLineDash([18, 16]);
  ctx.lineWidth = 3;
  ctx.beginPath();
  ctx.moveTo(vx, horizon);
  ctx.lineTo(W * 0.5 + shift * 0.2, H);
  ctx.stroke();
  ctx.setLineDash([]);
}

const DEFAULT_DISPLAY: DisplayParams = { panel_w: 0, panel_h: 0, link_bpp: 1, spi_mhz: 8, overhead_us: 50, fps_full: 10, partial: true, apl_ref: 0.25, static_frac: 0.3 };

/** Bounding box of changed pixels between two engine framebuffers (same layout). */
function diffRect(a: HudOut, b: HudOut): [number, number, number, number] | null {
  if (a.width !== b.width || a.height !== b.height || a.bpp !== b.bpp) return null;
  const x = decodeB64(a.data_b64);
  const y = decodeB64(b.data_b64);
  let x0 = Infinity, y0 = Infinity, x1 = -1, y1 = -1;
  for (let r = 0; r < a.height; r++) {
    for (let c = 0; c < a.stride; c++) {
      const i = r * a.stride + c;
      if (x[i] !== y[i]) {
        const px = a.bpp === 1 ? 8 : 2;
        x0 = Math.min(x0, c * px);
        x1 = Math.max(x1, c * px + px - 1);
        y0 = Math.min(y0, r);
        y1 = Math.max(y1, r);
      }
    }
  }
  return x1 < 0 ? null : [x0, y0, Math.min(a.width - 1, x1) - x0 + 1, y1 - y0 + 1];
}

const FrameBudget = memo(function FrameBudget() {
  const project = useProject((s) => s.project);
  const [dp, setDp] = useState<DisplayParams>(DEFAULT_DISPLAY);
  const set = <K extends keyof DisplayParams>(k: K, v: DisplayParams[K]) => setDp((d) => ({ ...d, [k]: v }));
  const res = useCompute<DisplayBudget>('firmware.hud_budget', { project, params: dp, state: DEFAULT_HUD });
  const b = res.data;
  if (!b) return null;
  const maxMw = Math.max(...b.screens.map((s) => s.total_full_mw), 1);
  return (
    <Card title="Frame budget: SPI bandwidth, refresh and power" subtitle={`Every number below comes from rendering each screen with the firmware's renderer at ${b.panel_w} × ${b.panel_h} and diffing consecutive frames.`}>
      <div className="grid grid-sidebar">
        <div>
          <Slider label="SPI clock" value={dp.spi_mhz} min={1} max={32} step={1} unit="MHz" onChange={(v) => set('spi_mhz', v)} hint="nRF52840: SPIM3 up to 32 MHz, other SPIM instances up to 8 MHz" />
          <Slider label="Full-rate refresh" value={dp.fps_full} min={1} max={60} step={1} unit="fps" onChange={(v) => set('fps_full', v)} />
          <div className="row" style={{ marginBottom: 10 }}>
            <Seg<number> ariaLabel="Bits per pixel on the link" value={dp.link_bpp} onChange={(v) => set('link_bpp', v)} options={[{ value: 1, label: '1 bpp' }, { value: 4, label: '4 bpp' }, { value: 8, label: '8 bpp' }, { value: 16, label: '16 bpp' }]} />
          </div>
          <Toggle label="Partial updates (send only the dirty rectangle)" value={dp.partial} onChange={(v) => set('partial', v)} />
          <Slider label="BOM display power refers to … lit" value={dp.apl_ref * 100} min={5} max={100} step={5} unit="%" onChange={(v) => set('apl_ref', v / 100)} hint="calibration assumption — replace with the panel datasheet" />
        </div>
        <div className="stack">
          <div className="grid grid-3">
            <Stat label="Full frame" value={fmtNum(b.frame_ms)} unit="ms" note={`${fmtNum(b.frame_bytes)} B at ${fmtNum(b.params.spi_mhz)} MHz`} />
            <Stat label="Max full-frame rate" value={fmtNum(b.max_fps)} unit="fps" tone={b.max_fps >= b.params.fps_full ? 'pass' : 'warn'} note={b.max_fps >= b.params.fps_full ? `✓ ≥ ${fmtNum(b.params.fps_full)} fps target` : `! below ${fmtNum(b.params.fps_full)} fps — use partial updates`} />
            <Stat label="Body text" value={fmtNum(b.text[0]?.cap_arcmin ?? 0)} unit="arcmin" note={`${b.text[0]?.cap_px ?? 0} px cap height · ${b.text[0]?.verdict ?? ''}`} />
          </div>
          <Plot
            height={180}
            xLabel="SPI clock (MHz)"
            yLabel="max full frames / s"
            series={[{ name: 'full-frame rate', x: b.sweep_mhz, y: b.sweep_max_fps }]}
            hLines={[{ value: b.params.fps_full, label: `${fmtNum(b.params.fps_full)} fps target` }]}
            vLines={[{ value: b.params.spi_mhz, label: 'now' }]}
            yZero
          />
        </div>
      </div>
      <div className="table-wrap" style={{ marginTop: 12 }}>
        <table className="table">
          <thead><tr><th>Screen</th><th>What changes</th><th>Lit</th><th>Dirty rectangle</th><th>SPI bytes/s (full → partial)</th><th>Link busy</th><th>Panel + link power</th></tr></thead>
          <tbody>
            {b.screens.map((s) => (
              <tr key={s.screen}>
                <td style={{ fontWeight: 650 }}>{s.title}</td>
                <td className="small muted">{s.update}</td>
                <td className="mono small">{fmtNum(s.apl_pct)} %</td>
                <td className="mono small">{s.dirty[2] ? `${s.dirty[2]}×${s.dirty[3]} px · ${fmtNum(s.dirty_bytes)} B · ${fmtNum(s.dirty_ms)} ms` : 'none'}</td>
                <td className="mono small">{fmtNum(s.full_bps / 1000)} k → {fmtNum(s.partial_bps / 1000)} k</td>
                <td className="mono small">{fmtNum(s.link_duty_pct)} %</td>
                <td style={{ minWidth: 170 }}>
                  <div className="pf-pwr" title={`panel ${fmtNum(s.panel_mw)} mW + link ${fmtNum(s.link_mw)} mW; full frames every period: ${fmtNum(s.total_full_mw)} mW`}>
                    <div style={{ width: `${(s.panel_mw / maxMw) * 100}%`, background: 'var(--series-1)' }} />
                    <div style={{ width: `${(s.link_mw / maxMw) * 100}%`, background: 'var(--series-3)' }} />
                    <span className="pf-pwr-ghost" style={{ left: `${(s.total_full_mw / maxMw) * 100}%` }} />
                  </div>
                  <div className="mono small">{fmtNum(s.total_mw)} mW <span className="muted">(full frames {fmtNum(s.total_full_mw)})</span></div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="row small muted" style={{ marginTop: 6 }}>
        <span className="pf-key"><i style={{ background: 'var(--series-1)' }} />panel (static + emission × lit fraction)</span>
        <span className="pf-key"><i style={{ background: 'var(--series-3)' }} />SPI link (SoC active while sending)</span>
        <span className="pf-key"><i style={{ background: 'transparent', borderLeft: '2px dashed var(--text-2)', borderRadius: 0 }} />full frames every period</span>
      </div>
      <div className="grid grid-2" style={{ marginTop: 10 }}>
        <div className="table-wrap">
          <table className="table">
            <thead><tr><th>Font size</th><th>Cap height</th><th>Angular size</th><th>Verdict</th><th>Used by</th></tr></thead>
            <tbody>
              {b.text.map((t) => (
                <tr key={t.scale}><td className="mono">×{t.scale}</td><td className="mono">{t.cap_px} px</td><td className="mono">{fmtNum(t.cap_arcmin)}′</td><td className="small">{t.verdict}</td><td className="small muted">{t.used_by.join(', ')}</td></tr>
              ))}
            </tbody>
          </table>
          <div className="muted small" style={{ marginTop: 4 }}>{fmtNum(b.arcmin_per_px)} arcmin per pixel ({fmtNum(b.ppd)} px/°) from the Optics Lab; a 20/20 letter is 5′ tall, comfortable reading ≈ 16–40′.</div>
        </div>
        <ul className="small muted" style={{ margin: 0, paddingLeft: 18 }}>{b.notes.map((n) => <li key={n}>{n}</li>)}</ul>
      </div>
      <Formula>{`t_frame = W·H·b / f_SCK + t_overhead = ${b.panel_w}·${b.panel_h}·${b.params.link_bpp} / ${fmtNum(b.params.spi_mhz)} MHz + ${fmtNum(b.params.overhead_us)} µs = ${fmtNum(b.frame_ms)} ms
P_panel = P_static + P_emit · APL        P_link = P_SoC,active · (bytes/s · 8 / f_SCK)`}</Formula>
    </Card>
  );
});

export function HudTab() {
  const project = useProject((s) => s.project);
  const optics = useCompute<OpticsA>('optics.analyze', { project });
  const screens = useCompute<ScreenDesc[]>('firmware.hud_screens', {});
  const reduced = useReducedMotion();
  const [playing, setPlaying] = useState(!reduced);
  const [res, setRes] = useState<'proto' | 'prod'>('prod');
  const [bpp, setBpp] = useState<1 | 4>(1);
  const [yaw, setYaw] = useState(0);
  const [worldLock, setWorldLock] = useState(true);
  const [st, setSt] = useState<HudState>({ ...DEFAULT_HUD, screen: 'navigation', captions: '' });
  const [displayOn, setDisplayOn] = useState(true);
  const [hud, setHud] = useState<HudOut | null>(null);
  const [fsm, setFsm] = useState({ state: 0, timer: 0 });
  const [log, setLog] = useState<string[]>([]);
  const [pressed, setPressed] = useState(false);
  const script = useRef<boolean[]>([]);
  const acc = useRef({ ms: 0, word: 0, pending: 0, frameMs: 0 });
  const W = res === 'prod' ? project.design.optics.px_w || 640 : 128;
  const H = res === 'prod' ? project.design.optics.px_h || 400 : 64;
  const fs = Math.max(1, Math.min(Math.floor(W / 128), Math.floor(H / 64)));
  const gestureInfo = useCompute<GestureOut>('firmware.gesture', { ticks: 0 });
  const states = gestureInfo.data?.states ?? [];

  const bearing = 45;
  const shown: HudState = useMemo(() => ({ ...st, nav_arrow_deg: Math.round(worldLock ? bearing - yaw : bearing) }), [st, yaw, worldLock]);

  // render through the engine whenever the state changes (throttled by the animation loop)
  const prevHud = useRef<HudOut | null>(null);
  const [dirty, setDirty] = useState<[number, number, number, number] | null>(null);
  useEffect(() => {
    const t0 = performance.now();
    const r = tryCall<HudOut>('firmware.hud_render', { state: shown, width: W, height: H, bpp });
    acc.current.frameMs = performance.now() - t0;
    if (r.data) {
      if (prevHud.current && prevHud.current.hash !== r.data.hash) setDirty(diffRect(prevHud.current, r.data));
      prevHud.current = r.data;
      setHud(r.data);
    }
  }, [shown, W, H, bpp]);

  const applyGesture = (g: string) => {
    setLog((l) => [`${new Date().toLocaleTimeString()}  ${g.replace('_', ' ')}`, ...l].slice(0, 12));
    if (g === 'tap') setSt((s) => ({ ...s, screen: SCREENS[(SCREENS.findIndex((x) => x.value === s.screen) + 1) % SCREENS.length].value, prompter_scroll_px: 0 }));
    if (g === 'double_tap') setSt((s) => ({ ...s, screen: 'clock' }));
    if (g === 'long_press') setDisplayOn((d) => !d);
  };

  // the gesture FSM runs every frame (interaction, not decoration, so it ignores reduced motion)
  const fsmRef = useRef(fsm);
  fsmRef.current = fsm;
  const pressedRef = useRef(pressed);
  pressedRef.current = pressed;
  useFrame(true, (dt) => {
    const ticks = Math.max(1, Math.round(dt));
    const trace: boolean[] = [];
    for (let k = 0; k < ticks; k++) trace.push(script.current.length ? script.current.shift()! : pressedRef.current);
    // no project in the per-frame call: the gesture thresholds are the Chip Lab defaults
    const r = tryCall<GestureOut>('firmware.gesture', { state: fsmRef.current.state, timer: fsmRef.current.timer, trace });
    if (r.data) {
      if (r.data.state !== fsmRef.current.state || r.data.timer !== fsmRef.current.timer) setFsm({ state: r.data.state, timer: r.data.timer });
      for (const e of r.data.events) applyGesture(e.output);
    }
  });

  // content animation
  useFrame(playing, (frameDt) => {
    const a = acc.current;
    a.pending += frameDt;
    if (a.pending < 50) return; // ~20 state updates per second
    const dt = a.pending;
    a.pending = 0;
    a.ms += dt;
    setSt((s) => {
      const n = { ...s };
      n.nav_distance_m = s.nav_distance_m > 12 ? s.nav_distance_m - dt * 0.012 : 1400;
      n.nav_distance_m = Math.round(n.nav_distance_m * 10) / 10;
      n.nav_eta_min = Math.max(1, Math.round(n.nav_distance_m / 80));
      if (a.ms - a.word * 450 > 450) {
        a.word++;
        const w = CAPTION_SCRIPT[a.word % CAPTION_SCRIPT.length];
        let cap = (s.captions ? s.captions + ' ' : '') + w;
        if (cap.length > 240) cap = cap.slice(cap.indexOf(' ', cap.length - 180) + 1);
        const cpl = Math.floor(Math.floor((124 * W) / 128) / (6 * fs));
        const lh = 8 * fs;
        const grew = wrapCount(cap, cpl) > wrapCount(s.captions, cpl);
        n.captions = cap;
        n.caption_scroll_px = grew ? lh : s.caption_scroll_px;
        n.minute = (s.minute + 1) % 60;
        if (n.minute === 0) n.hour = (s.hour + 1) % 24;
      }
      n.caption_scroll_px = Math.max(0, n.caption_scroll_px - dt * 0.06 * fs);
      n.caption_scroll_px = Math.round(n.caption_scroll_px);
      if (s.screen === 'teleprompter') {
        const cpl = Math.floor(Math.floor((124 * W) / 128) / (6 * fs));
        const total = wrapCount(s.prompter, cpl) * 8 * fs;
        const sp = (s.prompter_scroll_px + dt * 0.012 * fs) % (total + 1);
        n.prompter_scroll_px = Math.round(sp);
        n.prompter_progress = Math.round((sp / Math.max(1, total)) * 100);
      }
      n.nav_distance_m = Math.round(n.nav_distance_m);
      return n;
    });
  });

  // composite canvas: scene + HUD virtual image
  const sceneRef = useRef<HTMLCanvasElement>(null);
  const pixRef = useRef<HTMLCanvasElement>(null);
  const hfov = optics.data?.hfov_deg ?? 20;
  const vfov = optics.data?.vfov_deg ?? 12.5;
  const camFov = 70;
  useEffect(() => {
    const c = sceneRef.current;
    if (!c || !hud) return;
    const ctx = c.getContext('2d');
    if (!ctx) return;
    const CW = c.width;
    const CH = c.height;
    drawScene(ctx, CW, CH, yaw, camFov);
    const t = (d: number) => Math.tan((d * Math.PI) / 360);
    const f = CW / 2 / t(camFov);
    const hw = 2 * f * t(hfov);
    const hh = res === 'prod' ? 2 * f * t(vfov) : (hw * hud.height) / hud.width;
    const x0 = CW * 0.56 - hw / 2;
    const y0 = CH * 0.36 - hh / 2;
    if (displayOn) {
      const img = fbToImage(hud, [110, 235, 160], [0, 0, 0, 0]);
      const off = document.createElement('canvas');
      off.width = hud.width;
      off.height = hud.height;
      off.getContext('2d')?.putImageData(img, 0, 0);
      ctx.save();
      ctx.globalCompositeOperation = 'lighter';
      ctx.globalAlpha = Math.min(1, 0.4 + (project.design.optics.combiner_reflectance ?? 0.3));
      ctx.imageSmoothingEnabled = hud.width > 200;
      ctx.drawImage(off, x0, y0, hw, hh);
      ctx.restore();
    }
    ctx.strokeStyle = 'rgba(255,255,255,0.35)';
    ctx.setLineDash([5, 5]);
    ctx.strokeRect(x0, y0, hw, hh);
    ctx.setLineDash([]);
    ctx.fillStyle = 'rgba(255,255,255,0.8)';
    ctx.font = '12px system-ui, sans-serif';
    ctx.fillText(`virtual image ${fmtNum(hfov)}° × ${fmtNum(vfov)}° at ${fmtNum(optics.data?.virtual_image_m ?? 2)} m`, x0, y0 + hh + 16);
    // raw framebuffer
    const p = pixRef.current;
    if (p) {
      p.width = hud.width;
      p.height = hud.height;
      const pc = p.getContext('2d');
      if (pc) {
        pc.putImageData(fbToImage(hud, [255, 255, 255]), 0, 0);
        if (dirty) {
          pc.strokeStyle = 'rgba(255, 170, 40, 0.95)';
          pc.lineWidth = Math.max(1, Math.round(hud.width / 320));
          pc.strokeRect(dirty[0] + 0.5, dirty[1] + 0.5, dirty[2] - 1, dirty[3] - 1);
        }
      }
    }
  }, [hud, yaw, hfov, vfov, displayOn, optics.data, project.design.optics.combiner_reflectance, res, dirty]);

  const inject = (segs: [boolean, number][]) => {
    for (const [v, n] of segs) for (let k = 0; k < n; k++) script.current.push(v);
  };
  const g = gestureInfo.data;
  const ppd = optics.data?.ppd ?? 30;
  const charArcmin = res === 'prod' ? 6 * fs * (optics.data?.arcmin_per_px ?? 60 / ppd) : NaN;
  const screenInfo = screens.data?.find((s) => s.id === st.screen);

  return (
    <div className="stack">
      <div className="row" style={{ justifyContent: 'space-between' }}>
        <Seg<Screen> ariaLabel="Screen" value={st.screen} options={SCREENS} onChange={(v) => setSt((s) => ({ ...s, screen: v, prompter_scroll_px: 0 }))} />
        <PlayControls playing={playing} reduced={reduced} onToggle={() => setPlaying(!playing)} />
      </div>
      <div className="pf-main">
        <Card title="View through the glasses" subtitle={`Engine-rendered ${W} × ${H} framebuffer (${bpp} bpp, font scale ${fs}) composited at the Optics Lab's field of view over a street scene (camera ${camFov}° wide).`}>
          <div className="pf-hud-wrap">
            <canvas ref={sceneRef} width={960} height={540} aria-label="Scene with the heads-up display overlaid" />
          </div>
          <div className="grid grid-2" style={{ marginTop: 10 }}>
            <Slider label="Head yaw (turn your head)" value={yaw} min={-40} max={40} step={1} unit="°" onChange={setYaw} hint="The IMU's gyro integrates this; the firmware subtracts it from the navigation bearing." />
            <div>
              <Toggle label="World-locked arrow (IMU yaw compensation)" value={worldLock} onChange={setWorldLock} />
              <div className="row">
                <Seg<'proto' | 'prod'> ariaLabel="Panel" value={res} onChange={setRes} options={[{ value: 'prod', label: `Micro-OLED ${project.design.optics.px_w}×${project.design.optics.px_h}` }, { value: 'proto', label: 'Prototype OLED 128×64' }]} />
                <Seg<1 | 4> ariaLabel="Bits per pixel" value={bpp} onChange={setBpp} options={[{ value: 1, label: '1 bpp' }, { value: 4, label: '4 bpp' }]} />
              </div>
            </div>
          </div>
        </Card>
        <div className="stack">
          <Card title="Touch strip → gesture FSM" subtitle={g ? `Chip Lab thresholds: tap < ${g.tap_ms} ms, long press ≥ ${g.long_ms} ms, double-tap gap ${g.gap_ms} ms · 1 kHz ticks` : ''}>
            <div
              className={`pf-pad ${pressed ? 'on' : ''}`}
              role="button"
              tabIndex={0}
              aria-pressed={pressed}
              onPointerDown={(e) => { (e.target as HTMLElement).setPointerCapture?.(e.pointerId); setPressed(true); }}
              onPointerUp={() => setPressed(false)}
              onPointerCancel={() => setPressed(false)}
              onKeyDown={(e) => { if (e.key === ' ' || e.key === 'Enter') { e.preventDefault(); setPressed(true); } }}
              onKeyUp={(e) => { if (e.key === ' ' || e.key === 'Enter') setPressed(false); }}
            >
              {pressed ? 'Touching…' : 'Press & hold here (or Space)'}
            </div>
            <div className="row" style={{ marginTop: 10 }}>
              <button className="btn small" onClick={() => inject([[true, 110], [false, 20]])}>Tap</button>
              <button className="btn small" onClick={() => inject([[true, 100], [false, 120], [true, 100], [false, 20]])}>Double tap</button>
              <button className="btn small" onClick={() => inject([[true, 800], [false, 20]])}>Long press</button>
            </div>
            <div className="pf-fsm" style={{ marginTop: 10 }} aria-live="polite">
              {states.map((s, i) => <span key={s} className={i === fsm.state ? 'on' : ''}>{s}</span>)}
            </div>
            <div className="muted small" style={{ marginTop: 6 }}>timer {fsm.timer} ms · tap → next screen · double tap → clock · long press → display {displayOn ? 'off' : 'on'}</div>
            <div className="pf-log" style={{ marginTop: 8 }}>{log.length ? log.map((l, i) => <div key={i}>{l}</div>) : <span>No gestures yet.</span>}</div>
          </Card>
          <Card title="Exact firmware pixels" subtitle={hud ? `${hud.width} × ${hud.height}, ${hud.lit.toLocaleString()} lit · hash ${hud.hash.slice(0, 8)} · ${fmtNum(acc.current.frameMs)} ms per engine render` : ''}>
            <div className="pf-pixels"><canvas ref={pixRef} aria-label="Raw framebuffer" /></div>
            <div className="muted small" style={{ marginTop: 6 }}>
              The same bytes <span className="mono">iris_hud_render()</span> writes on the device — integer-only code, so the C and the Rust renderer agree bit for bit.
              {dirty && hud && <> The orange box is the last update's dirty rectangle: {dirty[2]} × {dirty[3]} px = {fmtNum(((dirty[2] * dirty[3] * hud.bpp) / 8 / ((hud.width * hud.height * hud.bpp) / 8)) * 100)} % of a full frame.</>}
            </div>
          </Card>
        </div>
      </div>
      <div className="grid grid-4">
        <Stat label="Field of view" value={`${fmtNum(hfov)}° × ${fmtNum(vfov)}°`} note="Optics Lab" />
        <Stat label="Angular resolution" value={fmtNum(ppd)} unit="px/°" note="R-005 ≥ 25" />
        <Stat label="Text height" value={res === 'prod' ? fmtNum(charArcmin) : '—'} unit={res === 'prod' ? 'arcmin' : ''} note={res === 'prod' ? `5×7 font × ${fs}: ${6 * fs} px cap height (a 20/20 letter is 5′)` : 'switch to the micro-OLED'} />
        <Stat label="Screen context" value={screenInfo?.context ?? '—'} note="column of the power-policy table" />
      </div>
      <FrameBudget />
      <div className="grid grid-2">
        <Explainer title="One UI description, two renderers" refs={[{ label: 'J. E. Bresenham, "Algorithm for computer control of a digital plotter", IBM Systems Journal 4(1), 1965' }, { label: 'U8g2 (olikraus/u8g2 on GitHub): drawXBM', href: 'https://github.com/olikraus/u8g2' }]}>
          Each screen is a table of widgets on a 128 × 64 design grid (text, word-wrapped text, arrow, bar, battery). The engine interprets the table into a framebuffer for this page; the generator emits the same table and a line-for-line C interpreter into <span className="mono">iris_hud.c</span>. Coordinates scale as <span className="mono">x·W/128</span>, strokes by an integer font scale, and rotation uses a Q14 sine table — no floating point, so both sides produce identical pixels. The prototype pushes the buffer to the OLED with U8g2's <span className="mono">drawXBM</span>.
          <Formula>{`fs = max(1, min(⌊W/128⌋, ⌊H/64⌋)) = ${fs}     (x', y') = (x·cos θ − y·sin θ, x·sin θ + y·cos θ) in Q14`}</Formula>
        </Explainer>
        <Explainer title="World-locked vs head-locked" refs={[{ label: 'R. Mahony, T. Hamel, J.-M. Pflimlin, "Nonlinear complementary filters on the special orthogonal group", IEEE Trans. Automatic Control 53(5), 2008' }]}>
          The phone sends a bearing to the next turn. If the arrow were simply drawn, it would turn with your head (head-locked). The firmware subtracts the yaw the gyroscope has integrated since the bearing arrived, so the arrow keeps pointing at the street (world-locked). Yaw has no gravity reference, so it drifts at the gyro bias rate — fine for the seconds between phone updates. Drag the head-yaw slider with the toggle on and off.
        </Explainer>
      </div>
    </div>
  );
}
