// Glossary look-up for <Term>, the start page and the command palette: the
// shared glossary (pages/learn/glossary.ts, read-only here) plus a local
// supplement of terms the start page and lab help need that the glossary does
// not have yet. Definitions are plain-language and textbook-standard.
import { GLOSSARY } from '../../pages/learn/glossary';

export interface TermDef {
  term: string;
  full: string;
  def: string;
  /** Route where the idea is used. */
  lab?: string;
  /** true when it comes from this supplement rather than the shared glossary. */
  local?: boolean;
}

export const SUPPLEMENT: TermDef[] = [
  { term: 'RF', full: 'radio frequency', lab: '/rf', def: 'Electromagnetic signals from roughly 3 kHz to 300 GHz; Iris talks to the phone at 2.4 GHz over Bluetooth Low Energy (BLE).' },
  { term: 'MTF', full: 'modulation transfer function', lab: '/optics?tab=quality', def: 'How much of a pattern’s contrast an optical system keeps at each spatial frequency: 1 is perfect, near 0 the detail is washed out.' },
  { term: 'FDTD', full: 'finite-difference time-domain', lab: '/rf?tab=fields', def: 'Solves Maxwell’s equations on a grid, step by step in time, so you can watch waves leave an antenna and reflect.' },
  { term: 'Return loss', full: 'return loss (S11 in decibels, sign flipped)', lab: '/rf?tab=matching', def: 'How much of the power sent to an antenna bounces back. 10 dB means 10 % reflected; bigger is better.' },
  { term: 'Smith chart', full: 'Smith chart', lab: '/rf?tab=matching', def: 'A circular map of every complex impedance; matching an antenna means walking its point to the centre (50 Ω) with inductors and capacitors.' },
  { term: 'Z0', full: 'characteristic impedance (Z₀)', lab: '/rf?tab=lines', def: 'The impedance a signal sees travelling along a transmission line, set by trace width, gap and board material; RF feeds are usually 50 Ω.' },
  { term: 'Link margin', full: 'link margin', lab: '/rf?tab=link', def: 'Received power minus the receiver’s sensitivity, in decibels. Above 0 dB the radio link closes; extra margin covers fading and bodies in the way.' },
  { term: 'dBm', full: 'decibels relative to one milliwatt', lab: '/rf?tab=link', def: 'Absolute power on a log scale: 0 dBm = 1 mW, −90 dBm = 1 pW. Adding dB multiplies power.' },
  { term: 'Safety factor', full: 'safety factor (factor of safety)', lab: '/structures', def: 'Strength divided by the stress the part actually sees. 2 means the part could take twice the load before yielding.' },
  { term: 'Goodman', full: 'Goodman fatigue criterion', lab: '/mechanisms?tab=springs', def: 'A straight-line rule combining steady (mean) and alternating stress to decide whether a part survives millions of load cycles.' },
  { term: 'Stack-up', full: 'tolerance stack-up', lab: '/structures?tab=tolerances', def: 'Adding up every part’s manufacturing tolerance to see whether an assembly still fits in the worst (or a statistically likely) case.' },
  { term: 'RSS', full: 'root sum square', lab: '/structures?tab=tolerances', def: 'Statistical stack-up: √(t₁² + t₂² + …). Less pessimistic than worst case because tolerances rarely all go the wrong way together.' },
  { term: 'Monte Carlo', full: 'Monte Carlo simulation', def: 'Answering “how likely?” by running a model thousands of times with randomly drawn inputs and counting the outcomes.' },
  { term: 'Weibull', full: 'Weibull distribution', lab: '/reliability?tab=weibull', def: 'The workhorse life distribution: shape β < 1 means early failures, β ≈ 1 random failures, β > 1 wear-out.' },
  { term: 'MTBF', full: 'mean time between failures', lab: '/reliability?tab=system', def: 'Average operating hours between random failures of a repairable item. A 1,000,000 h MTBF does not mean one unit lasts a million hours.' },
  { term: 'B10', full: 'B10 life', lab: '/reliability?tab=weibull', def: 'The time by which 10 % of units are expected to have failed.' },
  { term: 'ALT', full: 'accelerated life test', lab: '/reliability?tab=alt', def: 'Testing at higher temperature, humidity or load so years of use happen in weeks, then converting back with an acceleration factor.' },
  { term: 'CPM', full: 'critical path method', lab: '/program?tab=schedule', def: 'Scheduling by finding the longest chain of dependent tasks; any delay on it delays the whole project.' },
  { term: 'P80', full: '80th percentile (P80)', lab: '/program?tab=risk', def: 'The value you meet or beat in 80 % of simulated outcomes — a common “confident” schedule or cost commitment.' },
  { term: 'EVM', full: 'earned value management', lab: '/program?tab=evm', def: 'Tracking a project by comparing planned value, the value of work actually done (earned value) and actual cost.' },
  { term: 'TRL', full: 'technology readiness level', lab: '/program?tab=readiness', def: 'A 1–9 scale of maturity, from a principle observed (1) through a prototype in a relevant environment (6) to proven in operation (9).' },
  { term: 'MRL', full: 'manufacturing readiness level', lab: '/program?tab=readiness', def: 'The 1–10 companion scale to TRL for how ready the factory processes are.' },
  { term: 'NPV', full: 'net present value', lab: '/program?tab=economics', def: 'All future cash flows discounted back to today and summed; positive means the product is worth more than it costs.' },
  { term: 'Gross margin', full: 'gross margin', lab: '/program?tab=economics', def: '(Price − cost to make one unit) ÷ price. Hardware companies often aim for about 50 % or more to pay for development, support and sales.' },
  { term: 'Unit cost', full: 'unit cost', lab: '/mfg', def: 'What one finished product costs to make at a given quantity: parts, fabrication, assembly, test, and tooling spread over the run.' },
  { term: 'Lateral colour', full: 'lateral chromatic aberration (lateral colour)', lab: '/optics?tab=quality', def: 'Red and blue images landing at slightly different sizes, so edges near the corner of the view get coloured fringes.' },
  { term: 'RMS spot', full: 'root-mean-square (RMS) spot size', lab: '/optics?tab=quality', def: 'How big a single point of light is smeared by the lens; smaller than about two pixels looks sharp.' },
  { term: 'ACR', full: 'ambient contrast ratio', lab: '/ergonomics?tab=display', def: '(Display + background brightness) ÷ background brightness for a see-through display; Iris aims for 1.5 : 1 against a sunlit street.' },
  { term: 'IPD', full: 'interpupillary distance', lab: '/ergonomics', def: 'The distance between the centres of the two pupils — about 56–71 mm for 95 % of adults (mean about 63 mm).' },
  { term: 'Anthropometry', full: 'anthropometry', lab: '/ergonomics', def: 'Measurements of the human body (head breadth, nose width…) used to size products so most people fit.' },
  { term: 'Qubit', full: 'quantum bit', lab: '/quantum', def: 'A two-level quantum system that can be in a superposition of 0 and 1; measuring it gives 0 or 1 with probabilities set by its amplitudes.' },
  { term: 'QKD', full: 'quantum key distribution', lab: '/quantum?tab=qkd', def: 'Sharing a secret key using single photons, so any eavesdropper shows up as extra errors (e.g. the BB84 protocol).' },
  { term: 'QRNG', full: 'quantum random number generator', lab: '/quantum?tab=qrng', def: 'A random-number source whose unpredictability comes from a quantum process such as diode noise or photon arrival.' },
  { term: 'Fidelity', full: 'state fidelity', lab: '/quantum?tab=noise', def: 'How close a quantum state is to the one intended: 1 is identical, 0.5 is a coin flip for a qubit.' },
  { term: 'CHSH', full: 'Clauser–Horne–Shimony–Holt (CHSH) inequality', lab: '/quantum?tab=protocols', def: 'A Bell test: local hidden-variable theories give S ≤ 2, quantum entanglement can reach 2√2 ≈ 2.83.' },
  { term: 'OpenQASM', full: 'Open Quantum Assembly Language', lab: '/quantum', def: 'A text format for quantum circuits that IBM Qiskit and many simulators read and write.' },
  { term: 'Topology optimisation', full: 'topology optimisation', lab: '/structures?tab=topology', def: 'Letting an algorithm decide where material is needed under the loads, so a part gets lighter for the same stiffness.' },
  { term: 'LBM', full: 'lattice-Boltzmann method', lab: '/fluids?tab=tunnel', def: 'A fluid simulation that streams and collides particle populations on a grid; good at showing flow and vortices.' },
  { term: 'Convection', full: 'convection heat transfer', lab: '/fluids?tab=convection', def: 'Heat carried away by moving air or liquid; the coefficient h (W/m²·K) says how strong it is — roughly 2–10 in still air, more when air moves.' },
  { term: 'Spring rate', full: 'spring rate (stiffness)', lab: '/mechanisms?tab=springs', def: 'Force per unit deflection (N/mm), or torque per turn for a torsion spring.' },
  { term: 'Four-bar', full: 'four-bar linkage', lab: '/mechanisms', def: 'Four rigid links joined by four pins; the simplest mechanism that turns one rotation into a useful other motion.' },
  { term: 'Duty cycle', full: 'duty cycle', lab: '/power', def: 'The fraction of time a part is on. Average power = on-power × duty + sleep-power × (1 − duty).' },
  { term: 'Ripple', full: 'output ripple', lab: '/circuit', def: 'The small sawtooth left on a switching regulator’s output, measured peak to peak.' },
  { term: 'Droop', full: 'voltage droop', lab: '/circuit', def: 'How far a supply rail dips when a load suddenly switches on, before the regulator catches up.' },
  { term: 'Bode plot', full: 'Bode plot', lab: '/control?tab=pid', def: 'Gain and phase of a system against frequency on log axes; margins read from it tell you how close a loop is to oscillating.' },
  { term: 'Complementary filter', full: 'complementary filter', lab: '/control', def: 'Blends a gyro (good short-term) with an accelerometer (good long-term) using a high-pass and a low-pass that add up to one.' },
  { term: 'Stencil area ratio', full: 'stencil area ratio', lab: '/build?tab=calc', def: 'Aperture opening area ÷ aperture wall area. Above about 0.66 the solder paste releases cleanly from the stencil.' },
  { term: 'Reflow', full: 'reflow soldering', lab: '/build?tab=calc', def: 'Melting printed solder paste in an oven along a controlled temperature profile to attach surface-mount parts.' },
  { term: 'Clamp force', full: 'clamp force (injection molding)', lab: '/molding', def: 'The force the press needs to hold the mould shut against the injection pressure; sets which press size you need.' },
  { term: 'Crossover quantity', full: 'crossover quantity', lab: '/mfg', def: 'The volume above which an expensive-to-start process (a mould) becomes cheaper per part than a flexible one (printing).' },
  { term: 'Critical path (chip)', full: 'critical path (digital timing)', lab: '/chip', def: 'The slowest chain of logic between two flip-flops; it sets the highest clock frequency the chip can run.' },
  { term: 'Standard cell', full: 'standard cell', lab: '/chip', def: 'A pre-designed logic gate layout (NAND, flip-flop…) from a chip process library; cell count measures design size.' },
  { term: 'Decimation', full: 'decimation', lab: '/dsp?tab=pdm', def: 'Low-pass filtering then keeping every Nth sample, turning a fast 1-bit microphone stream into normal audio samples.' },
  { term: 'Beamformer', full: 'beamformer', lab: '/dsp?tab=beam', def: 'Combining several microphones with delays so sound from one direction adds up and the rest partly cancels.' },
  { term: 'Policy', full: 'policy (reinforcement learning)', lab: '/rl', def: 'The rule an agent uses to pick an action in each state; RL learns it from rewards.' },
  { term: 'Gerber X2', full: 'Gerber X2 format', lab: '/pcb?tab=files', def: 'The file format board houses use for each copper, mask, paste and silkscreen layer; X2 adds attributes such as layer function.' },
];

const norm = (s: string) => s.toLowerCase().replace(/[^a-z0-9²]+/g, '');

const ALL: TermDef[] = [
  ...GLOSSARY.map((g) => ({ term: g.term, full: g.full, def: g.def, lab: g.lab })),
  ...SUPPLEMENT.map((s) => ({ ...s, local: true })),
];

const INDEX = new Map<string, TermDef>();
for (const t of ALL) {
  const keys = [t.term, t.term.replace(/\s*\(.*\)\s*/, ''), ...t.term.split(/\s*\/\s*/)];
  for (const k of keys) {
    const n = norm(k);
    if (n && !INDEX.has(n)) INDEX.set(n, t);
  }
}

/** Find a term by its short form, e.g. `lookupTerm('MTF')`, `lookupTerm('SoC')`. */
export function lookupTerm(k: string): TermDef | undefined {
  return INDEX.get(norm(k));
}

/** Every term (shared glossary first, then the supplement). */
export const ALL_TERMS = ALL;
