// PCB Studio — placement, negotiated routing, DRC and fab outputs for the
// Iris main board, all computed by the Rust engine.
import { useEffect, useState } from 'react';
import { LabPage } from '../../components/LabPage';
import { Card, ErrorBox, Explainer, Formula, Slider, Stat, Tabs } from '../../components/ui';
import { fmtNum } from '../../components/Plot';
import { call } from '../../engine/engine';
import { useEngineReady } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { AnnealTab } from './AnnealTab';
import { BoardTab } from './BoardTab';
import { FilesTab } from './FilesTab';
import { Board3D, FootprintsTab } from './FootprintsTab';
import { RoutingTab } from './RoutingTab';
import { Safe3D } from './Safe3D';
import type { Layout } from './types';
import { useWorkerCompute } from './useWorker';

type Tab = 'board' | 'place' | 'route' | 'files' | 'lib' | '3d' | 'learn';

const REFS = {
  ipc2221: { label: 'IPC-2221, Generic Standard on Printed Board Design' },
  ipc7351: { label: 'IPC-7351, Generic Requirements for Surface Mount Design and Land Pattern Standard' },
  lee: { label: 'C. Y. Lee, "An algorithm for path connections and its applications", IRE Trans. Electronic Computers, 1961' },
  pathfinder: { label: 'L. McMurchie, C. Ebeling, "PathFinder: a negotiation-based performance-driven router for FPGAs", FPGA 1995' },
  sa: { label: 'S. Kirkpatrick, C. D. Gelatt, M. P. Vecchi, "Optimization by simulated annealing", Science 220, 1983' },
  gerber: { label: 'Ucamco, The Gerber Layer Format Specification', href: 'https://www.ucamco.com/en/gerber' },
  jlc: { label: 'JLCPCB help: BOM & CPL files', href: 'https://jlcpcb.com/help' },
};

function ipcWidthMm(i: number, dT: number, oz: number) {
  const area = (i / (0.048 * dT ** 0.44)) ** (1 / 0.725);
  return (area / (1.378 * oz)) * 0.0254;
}

export default function Lab() {
  const project = useProject((s) => s.project);
  const ready = useEngineReady();
  const [tab, setTab] = useState<Tab>('board');
  const res = useWorkerCompute<Layout>('pcb.layout', ready ? { project } : null);
  const L = res.data;
  // hand the worker's result to the main-thread engine so requirement checks don't re-route
  useEffect(() => {
    if (L && ready) {
      try { call('pcb.adopt', { project, layout: L }); } catch { /* ignore */ }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [L]);
  const [amps, setAmps] = useState(0.5);
  const [rise, setRise] = useState(10);
  const s = L?.stats;
  return (
    <LabPage
      kicker="Electronics · PCB"
      title="PCB Studio"
      subtitle={<>Placement, a negotiated-congestion maze router, design-rule check and factory files for the Iris main board — {project.design.pcb.width_mm} × {project.design.pcb.height_mm} mm, {project.design.pcb.layers} layers, living in the right temple pod.</>}
      metricPrefixes={['pcb.']}
    >
      <ErrorBox error={res.error} />
      {!L && <Card title="Routing the board…"><div className="muted small">The engine is placing parts, negotiating {project.components.filter((c) => c.on_pcb).length} components' connections and pouring ground in a background worker.</div></Card>}
      {L && s && (
        <>
          <div className="grid grid-4">
            <Stat label="Routed" value={fmtNum(s.routed_pct)} unit="%" tone={s.routed_pct >= 100 ? 'pass' : 'fail'} note={`${s.routed}/${s.connections} connections`} />
            <Stat label="DRC violations" value={s.drc_violations} tone={s.drc_violations === 0 ? 'pass' : 'fail'} note={s.drc_violations === 0 ? '✓ clean' : '✕ see the DRC list'} />
            <Stat label="Vias · track" value={s.via_count} note={`${fmtNum(s.track_length_mm)} mm (${fmtNum(s.track_length_top_mm)} top / ${fmtNum(s.track_length_bottom_mm)} bottom)`} />
            <Stat label="HPWL · ground pour" value={fmtNum(s.hpwl_mm)} unit="mm" note={`${fmtNum(s.pour_area_mm2)} mm² of stitched GND copper`} />
          </div>
          <Tabs<Tab>
            value={tab}
            onChange={setTab}
            tabs={[
              { id: 'board', label: 'Board & DRC' },
              { id: 'place', label: 'Placement annealing' },
              { id: 'route', label: 'Lee wavefront' },
              { id: 'files', label: 'Gerbers & files' },
              { id: 'lib', label: 'Footprints' },
              { id: '3d', label: '3-D' },
              { id: 'learn', label: 'Learn' },
            ]}
          />
          {tab === 'board' && <BoardTab layout={L} />}
          {tab === 'place' && <AnnealTab layout={L} />}
          {tab === 'route' && <RoutingTab layout={L} />}
          {tab === 'files' && <FilesTab layout={L} />}
          {tab === 'lib' && <FootprintsTab />}
          {tab === '3d' && <Card title="Assembled board" subtitle="Package bodies from the footprint library, on a 0.8 mm board — drag to orbit"><Safe3D height={360}><Board3D layout={L} /></Safe3D></Card>}
        </>
      )}
      {(tab === 'learn' || !L) && (
        <div className="grid grid-2">
          <Explainer title="Stack-up: what a 2-layer board is" refs={[REFS.ipc2221]}>
            Two 35 µm (1 oz) copper foils on a {project.design.pcb.thickness_mm} mm FR-4 core, green solder mask on both sides and white legend on top. Iris keeps parts on the top, routes signals on both layers and pours ground on both, stitched together with vias — on a board this thin, a solid return path under every signal matters more than routing space.
          </Explainer>
          <Explainer title="How wide must a trace be? (IPC-2221)" refs={[REFS.ipc2221]}>
            <Formula>{'I = k · ΔT^0.44 · A^0.725      (A in mil², k = 0.048 outer layers)\nwidth = A / (1.378 mil × oz of copper)'}</Formula>
            <Slider label="Current" value={amps} min={0.02} max={3} step={0.01} unit="A" onChange={setAmps} />
            <Slider label="Allowed temperature rise" value={rise} min={2} max={40} step={1} unit="°C" onChange={setRise} />
            <div>Minimum width on 1 oz outer copper: <b className="mono">{fmtNum(ipcWidthMm(amps, rise, 1))} mm</b> (Iris uses {project.design.pcb.min_trace_mm} mm everywhere; its busiest net, the buck switch node, carries ≈ 0.2 A → {fmtNum(ipcWidthMm(0.2, 10, 1))} mm).</div>
          </Explainer>
          <Explainer title="Lee's algorithm and why real routers negotiate" refs={[REFS.lee, REFS.pathfinder]}>
            Lee's maze router floods a grid breadth-first from the source until it touches the target, then walks back — it always finds a path if one exists, and the shortest one. Routing nets one after another is order-dependent: an early net can wall off a later one. PathFinder-style <i>negotiated congestion</i> lets every net route while sharing is allowed but priced; the price rises each iteration and contested cells remember it, until nets settle into a legal solution.
            <Formula>{'cost(cell) = base + p_pres · (other nets here) + h_history'}</Formula>
          </Explainer>
          <Explainer title="Placement by simulated annealing" refs={[REFS.sa]}>
            Half-perimeter wire length (HPWL) estimates routing length: for every net, the half perimeter of the box around its pins. The annealer proposes random moves, rotations and swaps, always accepting improvements and accepting a worse layout with probability e^(−ΔC/T), cooling T slowly — the metallurgy analogy that lets it escape local minima.
            <Formula>{'HPWL = Σ_nets (x_max − x_min) + (y_max − y_min)\nP(accept) = min(1, e^(−ΔC / T)),  T ← α·T'}</Formula>
          </Explainer>
          <Explainer title="Gerber X2: the files the factory reads" refs={[REFS.gerber, REFS.jlc]}>
            Each layer is one plain-text file of drawing commands: <span className="mono">%FSLAX46Y46*%</span> (coordinates in mm × 10⁶), <span className="mono">%ADD10C,0.15*%</span> defines an aperture, <span className="mono">D01</span> draws, <span className="mono">D02</span> moves, <span className="mono">D03</span> flashes a pad, <span className="mono">G36…G37</span> fills a region (the ground pour). X2 attributes (<span className="mono">%TF.FileFunction,Copper,L1,Top*%</span>, <span className="mono">%TO.N,GND*%</span>) tell the fab what each file and object is. Drills go in Excellon files; assembly houses want a BOM and a centroid (CPL) file.
          </Explainer>
          <Explainer title="Design for manufacture (DFM) on this board" refs={[REFS.ipc7351]}>
            0.15 mm trace/space and 0.3 mm vias are standard 2-layer capability at most prototype fabs; 0.4 mm-pitch QFN pads want ENIG finish and a windowed paste stencil on the exposed pad; keep copper 0.3 mm from the board edge; keep ground away from the chip antenna; put decoupling capacitors next to the pins they serve; give every component a courtyard and never let them overlap.
          </Explainer>
        </div>
      )}
      {tab !== 'learn' && L && (
        <Explainer title="Tie-back to Iris" refs={[REFS.lee, REFS.gerber]}>
          This board drives R-017 (all nets routed) and R-018 (zero DRC violations). The robotics lab's pick-and-place planner reads this placement, the Manufacturing Hub zips these Gerbers with the BOM/CPL, and the pod length in CAD must leave ≥ 1 mm around the {project.design.pcb.width_mm} mm board (R-019). Graduate to KiCad + FreeRouting for production layout, and check the Gerbers in gerbv or tracespace before ordering.
        </Explainer>
      )}
    </LabPage>
  );
}
