// Every lab, in V-model order. Adding a lab = one entry here + a folder in src/labs.
import { lazy, type ComponentType, type LazyExoticComponent } from 'react';

export interface LabInfo {
  id: string;
  path: string;
  title: string;
  group: string;
  icon: string;
  blurb: string;
  /** Metric key prefixes this lab produces (drives status dots and requirement chips). */
  metrics: string[];
  component: LazyExoticComponent<ComponentType>;
}

export const LABS: LabInfo[] = [
  { id: 'circuit', path: '/circuit', title: 'Circuit Lab', group: 'Electronics', icon: 'circuit', metrics: ['circuit.'], blurb: 'SPICE-style simulator: DC, transient and AC analysis of the Iris power and sensing circuits.', component: lazy(() => import('./circuit')) },
  { id: 'power', path: '/power', title: 'Power & Thermal', group: 'Electronics', icon: 'power', metrics: ['power.', 'thermal.'], blurb: 'Energy budget, battery discharge and a thermal network that predicts skin temperature.', component: lazy(() => import('./power')) },
  { id: 'pcb', path: '/pcb', title: 'PCB Studio', group: 'Electronics', icon: 'pcb', metrics: ['pcb.'], blurb: 'Placement, a Lee maze autorouter, DRC and Gerber / drill / BOM / CPL export.', component: lazy(() => import('./pcb')) },
  { id: 'dsp', path: '/dsp', title: 'DSP Lab', group: 'Signals & Silicon', icon: 'dsp', metrics: ['dsp.'], blurb: 'FFT, filter design, PDM decimation and the two-microphone beamformer.', component: lazy(() => import('./dsp')) },
  { id: 'chip', path: '/chip', title: 'Chip Lab', group: 'Signals & Silicon', icon: 'chip', metrics: ['chip.'], blurb: 'Logic minimisation, gate simulation, FSMs, FIR-to-Verilog and timing.', component: lazy(() => import('./chip')) },
  { id: 'control', path: '/control', title: 'Control Lab', group: 'Motion & Intelligence', icon: 'control', metrics: ['control.'], blurb: 'Head-tracking sensor fusion, PID tuning, Bode plots and LQR balancing.', component: lazy(() => import('./control')) },
  { id: 'robotics', path: '/robotics', title: 'Robotics Lab', group: 'Motion & Intelligence', icon: 'robotics', metrics: ['robotics.'], blurb: 'Arm kinematics, trajectory planning and pick-and-place of the Iris board.', component: lazy(() => import('./robotics')) },
  { id: 'rl', path: '/rl', title: 'RL Lab', group: 'Motion & Intelligence', icon: 'rl', metrics: ['rl.'], blurb: 'Q-learning from gridworlds to a learned power-management policy.', component: lazy(() => import('./rl')) },
  { id: 'optics', path: '/optics', title: 'Optics Lab', group: 'Optics & Mechanics', icon: 'optics', metrics: ['optics.'], blurb: 'Ray tracing the heads-up display: field of view, resolution, eyebox, image distance.', component: lazy(() => import('./optics')) },
  { id: 'cad', path: '/cad', title: 'CAD Studio', group: 'Optics & Mechanics', icon: 'cad', metrics: ['cad.', 'mass.', 'fit.'], blurb: 'Parametric frame generator with mass properties and STL export.', component: lazy(() => import('./cad')) },
  { id: 'print', path: '/print', title: 'Print Lab', group: 'Optics & Mechanics', icon: 'print', metrics: ['print.'], blurb: 'Slicer: layers, perimeters, infill, G-code, time and filament estimates.', component: lazy(() => import('./print')) },
  { id: 'molding', path: '/molding', title: 'Molding Lab', group: 'Production', icon: 'molding', metrics: ['molding.'], blurb: 'Injection-molding DFM: draft analysis, wall thickness, clamp force, cycle time.', component: lazy(() => import('./molding')) },
  { id: 'mfg', path: '/mfg', title: 'Manufacturing Hub', group: 'Production', icon: 'mfg', metrics: ['mfg.'], blurb: 'Cost vs quantity, vendor directory and a one-click fabrication package.', component: lazy(() => import('./mfg')) },
];

export const GROUPS = ['Electronics', 'Signals & Silicon', 'Motion & Intelligence', 'Optics & Mechanics', 'Production'];
