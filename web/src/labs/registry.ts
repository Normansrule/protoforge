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
  { id: 'rf', path: '/rf', title: 'RF & Antenna Lab', group: 'Electronics', icon: 'rf', metrics: ['rf.'], blurb: 'Finite-difference time-domain fields, antenna patterns, Smith-chart matching, controlled-impedance traces and a Bluetooth link budget.', component: lazy(() => import('./rf')) },
  { id: 'dsp', path: '/dsp', title: 'DSP Lab', group: 'Signals & Silicon', icon: 'dsp', metrics: ['dsp.'], blurb: 'FFT, filter design, PDM decimation and the two-microphone beamformer.', component: lazy(() => import('./dsp')) },
  { id: 'chip', path: '/chip', title: 'Chip Lab', group: 'Signals & Silicon', icon: 'chip', metrics: ['chip.'], blurb: 'Logic minimisation, gate simulation, FSMs, FIR-to-Verilog and timing.', component: lazy(() => import('./chip')) },
  { id: 'control', path: '/control', title: 'Control Lab', group: 'Motion & Intelligence', icon: 'control', metrics: ['control.'], blurb: 'Head-tracking sensor fusion, PID tuning, Bode plots and LQR balancing.', component: lazy(() => import('./control')) },
  { id: 'robotics', path: '/robotics', title: 'Robotics Lab', group: 'Motion & Intelligence', icon: 'robotics', metrics: ['robotics.'], blurb: 'Arm kinematics, trajectory planning and pick-and-place of the Iris board.', component: lazy(() => import('./robotics')) },
  { id: 'rl', path: '/rl', title: 'RL Lab', group: 'Motion & Intelligence', icon: 'rl', metrics: ['rl.'], blurb: 'Q-learning from gridworlds to a learned power-management policy.', component: lazy(() => import('./rl')) },
  { id: 'optics', path: '/optics', title: 'Optics Lab', group: 'Optics & Mechanics', icon: 'optics', metrics: ['optics.'], blurb: 'Ray tracing the heads-up display: field of view, resolution, eyebox, image distance.', component: lazy(() => import('./optics')) },
  { id: 'cad', path: '/cad', title: 'CAD Studio', group: 'Optics & Mechanics', icon: 'cad', metrics: ['cad.', 'mass.total_g', 'fit.'], blurb: 'Parametric frame generator with mass properties and STL export.', component: lazy(() => import('./cad')) },
  { id: 'print', path: '/print', title: 'Print Lab', group: 'Optics & Mechanics', icon: 'print', metrics: ['print.'], blurb: 'Slicer: layers, perimeters, infill, G-code, time and filament estimates.', component: lazy(() => import('./print')) },
  { id: 'structures', path: '/structures', title: 'Structures Lab', group: 'Optics & Mechanics', icon: 'structures', metrics: ['structures.'], blurb: 'Finite-element strength analysis, topology optimisation, fasteners, snap fits, fatigue and tolerance stack-ups.', component: lazy(() => import('./structures')) },
  { id: 'mechanisms', path: '/mechanisms', title: 'Mechanisms Lab', group: 'Optics & Mechanics', icon: 'mechanisms', metrics: ['mechanisms.'], blurb: 'Four-bar linkages, springs, gears and cams, and the Iris spring hinge — animated, with stress and life checks.', component: lazy(() => import('./mechanisms')) },
  { id: 'fluids', path: '/fluids', title: 'Thermal & Fluids Lab', group: 'Optics & Mechanics', icon: 'fluids', metrics: ['fluids.'], blurb: '3-D heat-conduction finite elements on the real frame, convection correlations, heat-spreader trades and a lattice-Boltzmann wind tunnel.', component: lazy(() => import('./fluids')) },
  { id: 'ergonomics', path: '/ergonomics', title: 'Human Factors Lab', group: 'Optics & Mechanics', icon: 'ergonomics', metrics: ['ergonomics.'], blurb: 'Head anthropometry and fit coverage, weight and nose-pad load, eyebox coverage and display legibility.', component: lazy(() => import('./ergonomics')) },
  { id: 'molding', path: '/molding', title: 'Molding Lab', group: 'Production', icon: 'molding', metrics: ['molding.'], blurb: 'Injection-molding DFM: draft analysis, wall thickness, clamp force, cycle time.', component: lazy(() => import('./molding')) },
  { id: 'build', path: '/build', title: 'Build Guide', group: 'Production', icon: 'build', metrics: ['build.'], blurb: 'Ideal process settings, feeds and speeds, reflow profiles, where to send each part, and the compliance path to a product.', component: lazy(() => import('./build')) },
  { id: 'mfg', path: '/mfg', title: 'Manufacturing Hub', group: 'Production', icon: 'mfg', metrics: ['mfg.'], blurb: 'Cost vs quantity, vendor directory and a one-click fabrication package.', component: lazy(() => import('./mfg')) },
  { id: 'reliability', path: '/reliability', title: 'Reliability & Test Lab', group: 'Test & Launch', icon: 'reliability', metrics: ['reliability.'], blurb: 'Weibull analysis, accelerated life tests, test sample sizes, reliability block diagrams and a design-verification test plan.', component: lazy(() => import('./reliability')) },
  { id: 'program', path: '/program', title: 'Program & Business Lab', group: 'Test & Launch', icon: 'program', metrics: ['program.'], blurb: 'Critical-path schedule, Monte Carlo schedule risk, earned value, unit economics and readiness levels.', component: lazy(() => import('./program')) },
  { id: 'proto', path: '/proto', title: 'Prototype & Firmware', group: 'Prototype & Software', icon: 'proto', metrics: ['firmware.'], blurb: 'Breadboard Iris from development boards, wiring, generated firmware, a heads-up display simulator and Bluetooth protocol design.', component: lazy(() => import('./proto')) },
  { id: 'quantum', path: '/quantum', title: 'Quantum Lab', group: 'Quantum', icon: 'quantum', metrics: ['quantum.'], blurb: 'State-vector quantum circuits, Bloch spheres, teleportation, Bell tests, quantum key distribution and quantum randomness.', component: lazy(() => import('./quantum')) },
];

export const GROUPS = ['Electronics', 'Signals & Silicon', 'Motion & Intelligence', 'Optics & Mechanics', 'Production', 'Test & Launch', 'Prototype & Software', 'Quantum'];
