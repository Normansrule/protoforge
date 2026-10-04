// Start-here content per engineering discipline: what Protoforge does for
// you, an ordered path through real lab tabs tied to real Iris v0.6
// requirements, a five-minute first win, your labs, key terms, the
// professional tools you graduate to and the guides to read.

export interface PathStep {
  lab: string;
  tab?: string;
  /** What to do, in plain words. */
  action: string;
  /** Iris requirement this step shows (live status + value). */
  req?: string;
  /** Metric key, when no requirement covers the step. */
  metric?: string;
  /** Rough minutes. */
  min: number;
}

export interface QuestStep {
  id: string;
  text: string;
  lab?: string;
  tab?: string;
}

export interface Role {
  id: string;
  title: string;
  /** Short name for chips and the sidebar. */
  short: string;
  icon: string;
  /** One line: what Protoforge does for you. */
  pitch: string;
  /** Labs shown first in the Simple-mode sidebar. */
  labs: string[];
  path: PathStep[];
  quest: { title: string; goal: string; steps: QuestStep[] };
  terms: string[];
  /** Discipline groups from pages/learn/repos.ts. */
  repoGroups: string[];
  /** Doc slugs under #/docs. */
  docs: string[];
}

export const ROLES: Role[] = [
  {
    id: 'electrical',
    title: 'Electrical & electronics',
    short: 'Electrical',
    icon: 'circuit',
    pitch: 'Simulate the power and sensing circuits, budget every milliwatt, route the board and get factory files — each number checked live against the product’s requirements.',
    labs: ['circuit', 'power', 'pcb', 'rf', 'build', 'proto'],
    path: [
      { lab: 'power', tab: 'budget', action: 'Pick a usage scenario and read the battery life; follow the waterfall to see which part drains the most.', req: 'R-002', min: 5 },
      { lab: 'circuit', tab: 'sim', action: 'Open the “3.7 V → 1.8 V synchronous buck” preset, drag its sliders and watch the measured output ripple.', req: 'R-013', min: 6 },
      { lab: 'circuit', tab: 'sim', action: 'Switch to “1.8 V rail · display load step” and see how far the rail dips when the display turns on.', req: 'R-012', min: 4 },
      { lab: 'power', tab: 'thermal', action: 'Run the pod cross-section with the display on and compare board and skin temperature with the 41 °C limit.', req: 'R-008', min: 5 },
      { lab: 'pcb', tab: 'board', action: 'Toggle layers, click a pad to light its net and read the design-rule check (DRC) list.', req: 'R-018', min: 6 },
      { lab: 'pcb', tab: 'files', action: 'Preview the Gerber layers rendered from the generated files and download the zip, bill of materials (BOM) and pick-and-place file.', req: 'R-017', min: 4 },
      { lab: 'build', tab: 'calc', action: 'Check every pad’s solder-paste stencil area ratio and the board against a standard fab tier.', req: 'R-035', min: 5 },
      { lab: 'rf', tab: 'matching', action: 'See why the chip antenna needs a matching network and what return loss it reaches.', req: 'R-036', min: 6 },
    ],
    quest: {
      title: 'Find what drains the battery',
      goal: 'Learn the one lever that moves battery life most, and see the requirement react.',
      steps: [
        { id: 'open', text: 'Open the energy budget.', lab: 'power', tab: 'budget' },
        { id: 'scenario', text: 'Click the “Display always on” scenario and note the battery life.' },
        { id: 'waterfall', text: 'Find the biggest bar in the waterfall — that is the part to optimise first.' },
        { id: 'back', text: 'Switch back to “Reference mixed use” and check R-002 is still green in the strip under the title.' },
      ],
    },
    terms: ['Buck', 'Ripple', 'Droop', 'Duty cycle', 'LiPo', 'DRC', 'Gerber', 'BOM', 'CPL', 'Stencil area ratio'],
    repoGroups: ['Circuit simulation', 'PCB design', 'Web tools for hand-off'],
    docs: ['labs-circuit', 'labs-power', 'labs-pcb', 'labs-build', 'toolchain'],
  },
  {
    id: 'mechanical',
    title: 'Mechanical & structures',
    short: 'Mechanical',
    icon: 'structures',
    pitch: 'Shape a parametric frame, check it is strong and stiff enough with finite elements, design springs and hinges, stack up tolerances and keep the wearer cool.',
    labs: ['cad', 'structures', 'mechanisms', 'fluids', 'print', 'ergonomics'],
    path: [
      { lab: 'cad', tab: 'design', action: 'Drag the frame sliders (lens width, bridge, pods) and watch mass and printed circuit board (PCB) fit update.', req: 'R-001', min: 6 },
      { lab: 'structures', tab: 'strength', action: 'Run the 3-D finite element analysis (FEA) on the temple under a load case and find the critical point.', req: 'R-026', min: 6 },
      { lab: 'structures', tab: 'tolerances', action: 'Run the worst-case and Monte Carlo stack-up of the board in its pod, including cold shrink.', req: 'R-027', min: 5 },
      { lab: 'mechanisms', tab: 'hinge', action: 'Open the Iris spring hinge: torque against angle, spring stress and Goodman fatigue safety factor.', req: 'R-041', min: 6 },
      { lab: 'structures', tab: 'fatigue', action: 'Read hinge life from the stress-life (S–N) curve and the temple’s natural frequencies.', metric: 'structures.hinge_fatigue_cycles', min: 5 },
      { lab: 'fluids', tab: 'conduction', action: 'See where the heat goes on the real frame and how hot the hottest surface gets.', req: 'R-039', min: 6 },
      { lab: 'ergonomics', tab: 'balance', action: 'Check the load on the nose and the nose-pad pressure against the comfort limit.', req: 'R-045', min: 4 },
      { lab: 'print', tab: 'frame', action: 'Estimate the print time of the whole frame set for a hobby printer.', req: 'R-021', min: 3 },
    ],
    quest: {
      title: 'Will the temple break?',
      goal: 'Run a real finite element analysis and read the safety factor like a stress engineer.',
      steps: [
        { id: 'open', text: 'Open the strength analysis.', lab: 'structures', tab: 'strength' },
        { id: 'case', text: 'Pick a load case and wait for the stress heat map.' },
        { id: 'read', text: 'Find the pulsing critical point and read the safety factor tile (2 or more passes R-026).' },
        { id: 'material', text: 'Try another material in the picker and watch the safety factor change.' },
      ],
    },
    terms: ['FEA', 'Safety factor', 'Goodman', 'Stack-up', 'RSS', 'Topology optimisation', 'Spring rate', 'Draft', 'CAD', 'STL'],
    repoGroups: ['CAD & geometry', 'Structures & finite elements', 'Mechanisms', 'Heat transfer & fluids'],
    docs: ['labs-cad', 'labs-structures', 'labs-mechanisms', 'labs-fluids'],
  },
  {
    id: 'firmware',
    title: 'Firmware & software',
    short: 'Firmware',
    icon: 'proto',
    pitch: 'Breadboard the product from development boards, generate its firmware from the other labs’ models, simulate the display and size the Bluetooth link — with memory and latency budgets checked.',
    labs: ['proto', 'dsp', 'chip', 'control', 'rl'],
    path: [
      { lab: 'proto', tab: 'build', action: 'Pick a breadboard build and watch the wiring draw itself; tick off the build checklist.', min: 5 },
      { lab: 'proto', tab: 'firmware', action: 'Browse the generated firmware, choose a target and download the project as a zip.', req: 'R-031', min: 6 },
      { lab: 'proto', tab: 'firmware', action: 'Read the flash image estimate against the two-slot update budget.', req: 'R-032', min: 2 },
      { lab: 'proto', tab: 'ble', action: 'Set the Bluetooth Low Energy (BLE) connection interval and maximum transmission unit (MTU) and check caption latency end to end.', req: 'R-033', min: 6 },
      { lab: 'proto', tab: 'hud', action: 'Drive the heads-up display (HUD) simulator with tap, double tap and long press.', metric: 'firmware.hud_frame_ms', min: 4 },
      { lab: 'dsp', tab: 'pdm', action: 'Follow the microphone’s 1-bit stream through the decimation filter to audio samples.', req: 'R-010', min: 6 },
      { lab: 'control', tab: 'head', action: 'Tune the inertial measurement unit (IMU) fusion filter and watch the head-tracking error.', req: 'R-011', min: 5 },
      { lab: 'rl', tab: 'power', action: 'Train the learned power-management policy and compare its battery life with the fixed one.', req: 'R-024', min: 5 },
    ],
    quest: {
      title: 'Ship your first firmware build',
      goal: 'Generate real, compilable firmware for a development board in under five minutes.',
      steps: [
        { id: 'open', text: 'Open the firmware generator.', lab: 'proto', tab: 'firmware' },
        { id: 'target', text: 'Choose the target that matches a board you have (or the Wokwi simulator).' },
        { id: 'read', text: 'Find the table that shows which lab each constant in the code came from.' },
        { id: 'zip', text: 'Download the project zip and check random-access memory (RAM) and flash margins (R-031, R-032).' },
      ],
    },
    terms: ['MCU', 'BLE', 'HUD', 'IMU', 'PDM', 'FSM', 'Decimation', 'Complementary filter', 'Policy', 'RTL'],
    repoGroups: ['Firmware & prototyping', 'Signal processing', 'Digital design & chips'],
    docs: ['labs-proto', 'labs-dsp', 'labs-chip', 'labs-control', 'labs-rl'],
  },
  {
    id: 'optical',
    title: 'Optical',
    short: 'Optical',
    icon: 'optics',
    pitch: 'Ray-trace the heads-up display, design and optimise the lens, and check what a real wearer sees — field of view, sharpness, eyebox and sunlight legibility.',
    labs: ['optics', 'ergonomics', 'structures', 'cad'],
    path: [
      { lab: 'optics', tab: 'ray', action: 'Drag focal length and display position in the to-scale ray diagram and read field of view.', req: 'R-004', min: 6 },
      { lab: 'optics', tab: 'ray', action: 'Watch pixels per degree trade against field of view as you change the focal length.', req: 'R-005', min: 3 },
      { lab: 'optics', tab: 'eyebox', action: 'Slide the eye sideways and see when rays start missing the pupil.', req: 'R-007', min: 4 },
      { lab: 'optics', tab: 'quality', action: 'Read spot diagrams and the modulation transfer function (MTF) of the real lens.', req: 'R-029', min: 6 },
      { lab: 'optics', tab: 'quality', action: 'Check lateral colour at the edge of the field — an open goal for Iris.', req: 'R-030', min: 4 },
      { lab: 'optics', tab: 'view', action: 'Look through the glasses at a street scene from night to sunlit snow.', min: 3 },
      { lab: 'ergonomics', tab: 'display', action: 'Check eyebox coverage over a population and outdoor contrast in sunlight.', req: 'R-047', min: 5 },
      { lab: 'structures', tab: 'tolerances', action: 'See how manufacturing tolerances move the display focus.', req: 'R-028', min: 4 },
    ],
    quest: {
      title: 'Make the picture bigger',
      goal: 'Feel the field-of-view versus sharpness trade every display designer lives with.',
      steps: [
        { id: 'open', text: 'Open the ray diagram.', lab: 'optics', tab: 'ray' },
        { id: 'focal', text: 'Shorten the focal length a little and watch the field of view grow.' },
        { id: 'ppd', text: 'Notice pixels per degree fall — R-005 needs at least 25.' },
        { id: 'see', text: 'Open “What you see” to view the result as a wearer would.', lab: 'optics', tab: 'view' },
      ],
    },
    terms: ['HUD', 'FOV', 'PPD', 'Eyebox', 'MTF', 'RMS spot', 'Lateral colour', 'ACR', 'nit', 'IPD'],
    repoGroups: ['Optics', 'Open smart-glasses hardware', 'Human factors'],
    docs: ['labs-optics', 'labs-ergonomics'],
  },
  {
    id: 'rf',
    title: 'RF & wireless',
    short: 'RF',
    icon: 'rf',
    pitch: 'Watch fields leave the antenna, match it on a Smith chart, size a 50 Ω feed and close the Bluetooth link budget with the head in the way.',
    labs: ['rf', 'pcb', 'proto', 'power'],
    path: [
      { lab: 'rf', tab: 'fields', action: 'Run the finite-difference time-domain (FDTD) solver with the “antenna next to a head” preset.', min: 5 },
      { lab: 'rf', tab: 'antennas', action: 'Compare antenna types and read the efficiency chain from chip to air.', metric: 'rf.antenna_efficiency_pct', min: 5 },
      { lab: 'rf', tab: 'matching', action: 'Drag the load on the Smith chart and synthesise an L-network snapped to standard values.', req: 'R-036', min: 7 },
      { lab: 'rf', tab: 'lines', action: 'Size the coplanar waveguide feed on the real 0.8 mm board.', req: 'R-037', min: 4 },
      { lab: 'rf', tab: 'link', action: 'Close the link budget to a phone 5 m away with head and body loss.', req: 'R-038', min: 5 },
      { lab: 'proto', tab: 'ble', action: 'See what the link margin buys: throughput and caption latency over Bluetooth Low Energy (BLE).', req: 'R-033', min: 4 },
      { lab: 'pcb', tab: 'board', action: 'Find the antenna keep-out on the routed board.', req: 'R-018', min: 3 },
    ],
    quest: {
      title: 'Match an antenna',
      goal: 'Turn a reflective chip antenna into one that radiates — the classic RF first win.',
      steps: [
        { id: 'open', text: 'Open the Smith chart.', lab: 'rf', tab: 'matching' },
        { id: 'drag', text: 'Drag the load point and watch return loss change.' },
        { id: 'net', text: 'Pick one of the L-network solutions and read its component values.' },
        { id: 'check', text: 'Confirm return loss stays at 10 dB or better across the band (R-036).' },
      ],
    },
    terms: ['RF', 'BLE', 'FDTD', 'Return loss', 'Smith chart', 'Z0', 'Link margin', 'dBm', 'PCB'],
    repoGroups: ['RF & antennas', 'PCB design'],
    docs: ['labs-rf', 'labs-pcb', 'labs-proto'],
  },
  {
    id: 'controls',
    title: 'Controls & robotics',
    short: 'Controls',
    icon: 'control',
    pitch: 'Fuse sensors, tune proportional–integral–derivative (PID) loops and the linear–quadratic regulator (LQR), plan robot arm moves for assembly and learn policies with reinforcement learning (RL).',
    labs: ['control', 'robotics', 'rl', 'mechanisms', 'dsp'],
    path: [
      { lab: 'control', tab: 'head', action: 'Tune the complementary-filter blend and watch the estimated head pose follow the true one.', req: 'R-011', min: 6 },
      { lab: 'control', tab: 'pid', action: 'Pick a plant, autotune with Ziegler–Nichols or Skogestad’s internal model control (SIMC) and read the Bode margins.', min: 7 },
      { lab: 'control', tab: 'lqr', action: 'Balance the inverted pendulum with LQR, then push the pole.', min: 5 },
      { lab: 'robotics', tab: 'pnp', action: 'Watch a selective compliance assembly robot arm (SCARA) populate the Iris board and find where the cycle time goes.', req: 'R-023', min: 5 },
      { lab: 'robotics', tab: 'traj', action: 'Compare trapezoidal, cubic and quintic moves under the same limits.', min: 4 },
      { lab: 'rl', tab: 'power', action: 'Train the power policy and check the battery-life gain against the fixed policy.', req: 'R-024', min: 5 },
      { lab: 'rl', tab: 'grid', action: 'Paint a gridworld and watch Q-learning values spread.', min: 4 },
      { lab: 'mechanisms', tab: 'linkages', action: 'Drag a four-bar linkage and watch its coupler curve and transmission angle.', min: 4 },
    ],
    quest: {
      title: 'Tune a PID loop',
      goal: 'Get a fast, well-damped step response and know why it is stable.',
      steps: [
        { id: 'open', text: 'Open the PID designer.', lab: 'control', tab: 'pid' },
        { id: 'auto', text: 'Press an autotune button (Ziegler–Nichols or SIMC).' },
        { id: 'kd', text: 'Nudge Kp up until overshoot appears, then add a little Kd.' },
        { id: 'bode', text: 'Read the phase margin on the Bode plot — 45° or more is comfortable.' },
      ],
    },
    terms: ['PID', 'LQR', 'Bode plot', 'PM / GM', 'Complementary filter', 'IMU', 'IK / FK', 'SCARA', 'RL', 'Policy'],
    repoGroups: ['Control & estimation', 'Robotics & reinforcement learning'],
    docs: ['labs-control', 'labs-robotics', 'labs-rl'],
  },
  {
    id: 'manufacturing',
    title: 'Manufacturing & industrial',
    short: 'Manufacturing',
    icon: 'mfg',
    pitch: 'Pick the right process for each part, get starting settings, check moldability and printability, and see unit cost fall with quantity — then download one fabrication package.',
    labs: ['build', 'mfg', 'molding', 'print', 'pcb', 'cad'],
    path: [
      { lab: 'build', tab: 'wizard', action: 'Set quantity, tolerance and finish and see which process wins and where to send the file.', min: 5 },
      { lab: 'build', tab: 'settings', action: 'Pick a process and material and read its typical starting settings (tune on your machine).', min: 4 },
      { lab: 'print', tab: 'frame', action: 'Estimate the print time of the whole frame set.', req: 'R-021', min: 3 },
      { lab: 'molding', tab: 'draft', action: 'Colour the frame by draft angle and find faces that would stick in the mould.', req: 'R-020', min: 5 },
      { lab: 'mfg', tab: 'cost', action: 'Hover the cost-versus-quantity chart and find where molding beats printing.', req: 'R-022', min: 5 },
      { lab: 'build', tab: 'calc', action: 'Run the board against a standard fab tier and check the stencil.', req: 'R-034', min: 4 },
      { lab: 'robotics', tab: 'pnp', action: 'See how long automated pick-and-place takes per board.', req: 'R-023', min: 3 },
      { lab: 'mfg', tab: 'package', action: 'Preview and download the one-click fabrication package.', min: 3 },
    ],
    quest: {
      title: 'Price the product',
      goal: 'Find the quantity where tooling pays off and the unit cost at the target volume.',
      steps: [
        { id: 'open', text: 'Open cost versus quantity.', lab: 'mfg', tab: 'cost' },
        { id: 'hover', text: 'Hover the stacked chart at 1,000 units and read where the money goes.' },
        { id: 'cross', text: 'Find the crossover where molding beats printing.' },
        { id: 'req', text: 'Check unit cost is under 120 USD at 1,000 units (R-022).' },
      ],
    },
    terms: ['DFM', 'DFA', 'NRE', 'MOQ', 'FDM', 'Draft', 'Clamp force', 'Crossover quantity', 'Unit cost', 'Gerber'],
    repoGroups: ['3D printing', 'CAD & geometry', 'Web tools for hand-off'],
    docs: ['labs-build', 'labs-mfg', 'labs-molding', 'labs-print'],
  },
  {
    id: 'test',
    title: 'Test & reliability',
    short: 'Test',
    icon: 'reliability',
    pitch: 'Fit failure data, turn years of use into weeks of accelerated testing, size demonstration tests and generate the design verification test (DVT) plan from the product itself.',
    labs: ['reliability', 'structures', 'mechanisms', 'fluids', 'build'],
    path: [
      { lab: 'reliability', tab: 'weibull', action: 'Load a sample data set and fit a Weibull line on probability paper.', min: 6 },
      { lab: 'reliability', tab: 'alt', action: 'Pick an acceleration model and see how many weeks of chamber time equal two years of use.', min: 5 },
      { lab: 'reliability', tab: 'sample', action: 'Find how many units prove 90 % reliability at 90 % confidence with zero failures.', min: 4 },
      { lab: 'reliability', tab: 'system', action: 'Build the reliability block diagram from the bill of materials and read 2-year reliability.', req: 'R-048', min: 5 },
      { lab: 'reliability', tab: 'system', action: 'Read the parts-count mean time between failures (MTBF) of the electronics.', req: 'R-049', min: 2 },
      { lab: 'reliability', tab: 'dvt', action: 'Generate the DVT plan: legs, sample sizes, schedule and requirement matrix.', req: 'R-050', min: 6 },
      { lab: 'mechanisms', tab: 'hinge', action: 'Check the hinge spring survives 12 opens a day for 3 years.', req: 'R-042', min: 4 },
      { lab: 'build', tab: 'compliance', action: 'List the compliance items to plan before launch (check each standard yourself).', metric: 'build.compliance_items', min: 4 },
    ],
    quest: {
      title: 'Size a reliability test',
      goal: 'Answer the question every test lead gets: how many units, for how long?',
      steps: [
        { id: 'open', text: 'Open the sample-size calculator.', lab: 'reliability', tab: 'sample' },
        { id: 'run', text: 'Set reliability 90 % and confidence 90 % with zero failures and read the unit count.' },
        { id: 'fail', text: 'Allow one failure and see how many more units you need.' },
        { id: 'dvt', text: 'Open the DVT plan and check it stays within 40 units (R-050).', lab: 'reliability', tab: 'dvt' },
      ],
    },
    terms: ['Weibull', 'B10', 'MTBF', 'ALT', 'DVT', 'EVT', 'PVT', 'FMEA', 'Goodman', 'Monte Carlo'],
    repoGroups: ['Reliability & test', 'Structures & finite elements'],
    docs: ['labs-reliability', 'labs-mechanisms', 'labs-build'],
  },
  {
    id: 'systems',
    title: 'Systems & program',
    short: 'Systems',
    icon: 'systems',
    pitch: 'Hold the whole product in one model: needs, requirements bound to live metrics, budgets, trades and risks — then the schedule, its risk, the business case and readiness.',
    labs: ['program', 'reliability', 'mfg', 'power', 'ergonomics'],
    path: [
      { lab: 'systems', tab: 'requirements', action: 'Read every requirement with its live value, margin and the lab that verifies it.', min: 6 },
      { lab: 'systems', tab: 'traceability', action: 'Follow a stakeholder need down to requirements, subsystems and metrics.', min: 4 },
      { lab: 'systems', tab: 'budgets', action: 'Check the mass, power and cost roll-ups against their limits.', req: 'R-001', min: 4 },
      { lab: 'program', tab: 'schedule', action: 'Watch the critical path light up on the Gantt chart and network diagram.', metric: 'program.cpm_weeks', min: 5 },
      { lab: 'program', tab: 'risk', action: 'Run the Monte Carlo schedule and read the 80 % confidence date.', req: 'R-051', min: 5 },
      { lab: 'program', tab: 'economics', action: 'Check gross margin and break-even volume at the retail price.', req: 'R-052', min: 5 },
      { lab: 'program', tab: 'readiness', action: 'See which subsystem holds technology readiness back — an open goal for Iris.', req: 'R-053', min: 4 },
      { lab: 'systems', tab: 'risks', action: 'Rank failure modes by risk priority number in the failure mode and effects analysis (FMEA).', min: 4 },
    ],
    quest: {
      title: 'Read the product’s health in five minutes',
      goal: 'Know what passes, what fails and what to do about it.',
      steps: [
        { id: 'open', text: 'Open Mission control in the Systems Studio.', lab: 'systems', tab: 'overview' },
        { id: 'fail', text: 'Find the three open goals (lateral colour, outdoor contrast, readiness).' },
        { id: 'req', text: 'Click R-047 to open its drawer and see which lab verifies it.' },
        { id: 'risk', text: 'Open schedule risk and read the 80 % date (R-051).', lab: 'program', tab: 'risk' },
      ],
    },
    terms: ['V-model', 'Traceability', 'Trade study', 'FMEA', 'RPN', 'CPM', 'P80', 'EVM', 'TRL', 'NPV'],
    repoGroups: ['Program management', 'Protoforge’s own stack'],
    docs: ['systems-studio', 'labs-program', 'versions', 'getting-started'],
  },
  {
    id: 'quantum',
    title: 'Quantum',
    short: 'Quantum',
    icon: 'quantum',
    pitch: 'Compose and simulate quantum circuits, animate teleportation and Bell tests, design a fiber quantum key distribution (QKD) link and test a quantum random number generator (QRNG).',
    labs: ['quantum', 'chip', 'dsp', 'optics'],
    path: [
      { lab: 'quantum', tab: 'composer', action: 'Drag a Hadamard (H) and a controlled-NOT (CNOT) gate onto the wires to make a Bell pair; step through the Bloch spheres.', min: 5 },
      { lab: 'quantum', tab: 'protocols', action: 'Play teleportation and the Clauser–Horne–Shimony–Holt (CHSH) Bell test as animated stories.', min: 6 },
      { lab: 'quantum', tab: 'noise', action: 'Add noise and watch fidelity fall in the density-matrix simulator.', min: 5 },
      { lab: 'quantum', tab: 'qkd', action: 'Load the Q-Link template (it replaces the current project — save first) and read key rate against fiber length.', metric: 'quantum.qkd_key_rate_bps', min: 6 },
      { lab: 'quantum', tab: 'qrng', action: 'Run the National Institute of Standards and Technology (NIST) statistical tests on the conditioned diode-noise stream.', metric: 'quantum.qrng_nist_failures', min: 5 },
      { lab: 'quantum', tab: 'algorithms', action: 'Run Grover search and watch the marked item’s probability grow.', min: 4 },
      { lab: 'chip', tab: 'fir', action: 'Turn a filter into Verilog — the field-programmable gate array (FPGA) side of a QRNG board.', min: 5 },
    ],
    quest: {
      title: 'Entangle two qubits',
      goal: 'Build a Bell pair and export it to run on real quantum tools.',
      steps: [
        { id: 'open', text: 'Open the circuit composer.', lab: 'quantum', tab: 'composer' },
        { id: 'h', text: 'Put an H gate on qubit 0, then a CNOT from qubit 0 to qubit 1.' },
        { id: 'shots', text: 'Run shots: you should only ever see 00 and 11.' },
        { id: 'export', text: 'Export OpenQASM or a Qiskit script.' },
      ],
    },
    terms: ['Qubit', 'Fidelity', 'CHSH', 'QKD', 'QRNG', 'OpenQASM', 'FPGA'],
    repoGroups: ['Quantum information', 'Digital design & chips'],
    docs: ['labs-quantum', 'labs-chip'],
  },
  {
    id: 'student',
    title: 'New to engineering / student',
    short: 'Student',
    icon: 'learn',
    pitch: 'Learn by changing real designs: every lab animates what is happening, explains the maths in plain words and tells you whether the product still works.',
    labs: ['circuit', 'power', 'cad', 'print', 'structures', 'optics'],
    path: [
      { lab: 'systems', tab: 'overview', action: 'See the product as a whole: what it must do and which checks pass.', min: 4 },
      { lab: 'power', tab: 'budget', action: 'Find out how long the battery lasts and what uses the most energy.', req: 'R-002', min: 5 },
      { lab: 'circuit', tab: 'sim', action: 'Pick “RC anti-alias filter”, drag the sliders and watch current flow through the drawing.', min: 6 },
      { lab: 'cad', tab: 'design', action: 'Resize the glasses frame and watch the total mass change.', req: 'R-001', min: 5 },
      { lab: 'structures', tab: 'calculators', action: 'Use the beam calculator to see how thickness changes bending.', min: 5 },
      { lab: 'print', tab: 'layers', action: 'Watch the slicer lay down a part layer by layer.', min: 4 },
      { lab: 'optics', tab: 'view', action: 'See what the wearer sees through the heads-up display.', min: 3 },
      { lab: 'quantum', tab: 'composer', action: 'Make two qubits agree every time with an H and a CNOT gate.', min: 5 },
    ],
    quest: {
      title: 'Your first design change',
      goal: 'Change one number, see every check react, and undo it.',
      steps: [
        { id: 'open', text: 'Open the frame designer.', lab: 'cad', tab: 'design' },
        { id: 'slide', text: 'Drag “Lens width A” up a few millimetres.' },
        { id: 'see', text: 'Watch R-001 (mass ≤ 50 g) in the strip under the title.' },
        { id: 'undo', text: 'Use the reset button in the top bar to go back to Iris.' },
      ],
    },
    terms: ['CAD', 'FEA', 'PCB', 'BOM', 'FDM', 'HUD', 'Duty cycle', 'Safety factor', 'Qubit', 'V-model'],
    repoGroups: ['Circuit simulation', 'CAD & geometry', '3D printing'],
    docs: ['getting-started', 'start-here', 'labs-circuit', 'labs-cad'],
  },
];

export const roleById = (id: string | null | undefined) => ROLES.find((r) => r.id === id);
