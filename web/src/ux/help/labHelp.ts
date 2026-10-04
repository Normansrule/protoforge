// Plain-words help for every lab: the question it answers, what you change,
// what you read (with good/bad ranges), three steps to try, who it is for and
// — honestly — whether it analyses your own product or the Iris example.

export type Scope = 'project' | 'iris' | 'general' | 'mixed';

export interface HelpOutput {
  name: string;
  /** Good / bad range in plain words. */
  good: string;
  /** Metric key for a live value. */
  metric?: string;
}

export interface LabHelpData {
  question: string;
  inputs: string[];
  outputs: HelpOutput[];
  try3: { text: string; tab?: string }[];
  roles: string[];
  scope: Scope;
  scopeNote: string;
  terms: string[];
}

export const SCOPE_LABEL: Record<Scope, string> = {
  project: 'Works on your product',
  iris: 'Analyses the Iris example geometry',
  general: 'General-purpose tools',
  mixed: 'General tools + Iris example',
};

export const LAB_HELP: Record<string, LabHelpData> = {
  circuit: {
    question: 'Does this circuit do what I think it does — voltages, currents, ripple, frequency response?',
    inputs: ['A preset circuit or your own Simulation Program with Integrated Circuit Emphasis (SPICE) netlist', 'Sliders that rewrite the component values', 'Probes: which nodes to plot'],
    outputs: [
      { name: '1.8 V rail droop on a load step', good: 'Iris needs ≤ 50 mV; smaller is better', metric: 'circuit.rail_droop_mv' },
      { name: 'Buck output ripple', good: 'Iris needs ≤ 30 mV peak to peak', metric: 'circuit.buck_ripple_mv' },
      { name: 'Battery-sense divider current', good: '≤ 2 µA so it does not drain the cell', metric: 'circuit.vbat_divider_ua' },
    ],
    try3: [
      { text: 'Pick “RC anti-alias filter” (resistor–capacitor) in the gallery and press Play — dots show current flowing.', tab: 'sim' },
      { text: 'Drag a slider and watch the waveform and the measurements re-simulate.', tab: 'sim' },
      { text: 'Open the Schematic editor and draw your own circuit.', tab: 'editor' },
    ],
    roles: ['electrical', 'student', 'firmware'],
    scope: 'general',
    scopeNote: 'A general SPICE-style simulator: any netlist you paste runs. The Iris presets show how the product’s own circuits verify its requirements.',
    terms: ['SPICE', 'MNA', 'Ripple', 'Droop', 'Buck'],
  },
  power: {
    question: 'How long does the battery last, where does the energy go, and how warm does it get against the skin?',
    inputs: ['Usage scenario (reference, navigation, captions, display always on…)', 'Per-part duty cycles', 'Battery cell and regulator', 'Thermal network values'],
    outputs: [
      { name: 'Battery life', good: 'Iris needs ≥ 8 h in the reference day', metric: 'power.battery_life_h' },
      { name: 'Average power', good: '≤ 80 mW for Iris', metric: 'power.avg_power_mw' },
      { name: 'Pod skin temperature', good: '≤ 41 °C — above that is uncomfortable to wear', metric: 'thermal.max_skin_c' },
    ],
    try3: [
      { text: 'Click each scenario and watch battery life change.', tab: 'budget' },
      { text: 'Play the simulated day and try a cold (0 °C) cell.', tab: 'discharge' },
      { text: 'Turn the display on continuously and see the skin temperature curve.', tab: 'thermal' },
    ],
    roles: ['electrical', 'systems', 'student', 'rf'],
    scope: 'project',
    scopeNote: 'Reads your project’s bill of materials (each part’s active and sleep power), battery and rails — so it works for your own product, not only Iris.',
    terms: ['Duty cycle', 'LiPo', 'Buck', 'SoC (battery)'],
  },
  pcb: {
    question: 'Can this board be placed, routed and checked so a board house will make it?',
    inputs: ['Board size, layers and thickness', 'Design rules: trace width, clearance, via size, grid'],
    outputs: [
      { name: 'Connections routed', good: '100 % — anything less is an unfinished board', metric: 'pcb.routed_pct' },
      { name: 'Design-rule violations', good: '0 — every violation is a possible short or open', metric: 'pcb.drc_violations' },
      { name: 'Vias', good: 'Fewer is cheaper and more reliable', metric: 'pcb.via_count' },
    ],
    try3: [
      { text: 'Click a pad to highlight its net and read the design-rule check (DRC) list.', tab: 'board' },
      { text: 'Watch the annealer shuffle parts to shorten wires.', tab: 'place' },
      { text: 'Download the Gerber zip, bill of materials (BOM) and pick-and-place file.', tab: 'files' },
    ],
    roles: ['electrical', 'rf', 'manufacturing'],
    scope: 'iris',
    scopeNote: 'Routes the Iris main board: its schematic (pin-to-net table) is built in. Board size and design rules are yours to change; parts removed from the BOM drop out.',
    terms: ['PCB', 'DRC', 'HPWL', 'Gerber', 'CPL'],
  },
  rf: {
    question: 'Will the radio link to the phone work — antenna, match, feed line and link margin?',
    inputs: ['Antenna type and size', 'Chip-antenna impedance (assumed — measure it)', 'Matching network', 'Trace geometry and board stack-up', 'Link budget: distance, head and body loss'],
    outputs: [
      { name: 'Return loss in the band', good: '≥ 10 dB (≤ 10 % of power reflected)', metric: 'rf.match_return_loss_db' },
      { name: 'Feed impedance', good: '50 Ω ± 5 Ω', metric: 'rf.feed_z0_ohm' },
      { name: 'Link margin', good: '≥ 0 dB closes the link; more covers fading', metric: 'rf.link_margin_db' },
    ],
    try3: [
      { text: 'Run the “antenna next to a head” field preset.', tab: 'fields' },
      { text: 'Drag the load on the Smith chart and pick a matching network.', tab: 'matching' },
      { text: 'Change the distance in the link budget and watch the margin.', tab: 'link' },
    ],
    roles: ['rf', 'electrical'],
    scope: 'mixed',
    scopeNote: 'The field solver, antenna patterns, Smith chart and line calculators are general. Matching, feed and link-budget metrics use the Iris board and its assumed chip-antenna model.',
    terms: ['RF', 'FDTD', 'Smith chart', 'Return loss', 'Z0', 'Link margin', 'dBm'],
  },
  dsp: {
    question: 'How do I get clean audio from the microphones — spectra, filters, decimation and beamforming?',
    inputs: ['Test signals and windows', 'Filter specs: finite or infinite impulse response (FIR / IIR)', 'Pulse-density modulation (PDM) clock and decimation', 'Mic spacing'],
    outputs: [
      { name: 'Decimation stopband', good: '≥ 60 dB keeps aliases out of the voice band', metric: 'dsp.decim_stopband_db' },
      { name: 'Mic array directivity index', good: '≥ 4 dB at 1 kHz', metric: 'dsp.mic_di_db' },
    ],
    try3: [
      { text: 'Build a two-tone signal and switch windows to see leakage.', tab: 'spectrum' },
      { text: 'Design a low-pass FIR from specs.', tab: 'filter' },
      { text: 'Follow the 1-bit microphone stream to audio.', tab: 'pdm' },
    ],
    roles: ['firmware', 'controls', 'quantum'],
    scope: 'mixed',
    scopeNote: 'Spectrum and filter tools are general; the decimation chain and beamformer use your project’s audio settings (Iris: two microphones).',
    terms: ['DSP', 'FFT', 'FIR', 'IIR', 'PDM', 'Decimation', 'Beamformer'],
  },
  chip: {
    question: 'How does logic become a chip — minimise, simulate, describe in Verilog and check timing?',
    inputs: ['Boolean expressions or minterms', 'Gate circuits', 'Finite impulse response (FIR) filter coefficients and bit widths', 'Clock and gate delay'],
    outputs: [
      { name: 'Critical path', good: '≤ 20 ns to run at 50 MHz', metric: 'chip.critical_path_ns' },
      { name: 'Standard cells', good: '≤ 3000 for the Iris co-processor budget', metric: 'chip.est_cells' },
    ],
    try3: [
      { text: 'Type an expression and watch the K-map group.', tab: 'logic' },
      { text: 'Simulate a 4-bit adder.', tab: 'sim' },
      { text: 'Generate Verilog for the FIR filter.', tab: 'fir' },
    ],
    roles: ['firmware', 'quantum', 'student'],
    scope: 'general',
    scopeNote: 'General digital-design tools; the FIR → Verilog and tape-out tabs use the Iris co-processor (IRIS-DSP) as the worked example.',
    terms: ['RTL', 'HDL', 'FSM', 'Critical path (chip)', 'Standard cell', 'ASIC', 'FPGA'],
  },
  control: {
    question: 'How do I estimate where things point and make loops fast but stable?',
    inputs: ['Inertial measurement unit (IMU) noise and rate', 'Filter blend α', 'Plant model and proportional–integral–derivative (PID) gains', 'Linear–quadratic regulator (LQR) weights'],
    outputs: [
      { name: 'Head-tracking error', good: '≤ 2° RMS during normal head motion', metric: 'control.head_rms_err_deg' },
      { name: 'Phase margin', good: '≥ 45° is comfortable; < 30° rings' },
    ],
    try3: [
      { text: 'Move the fusion blend slider and watch the ghost head.', tab: 'head' },
      { text: 'Autotune a PID loop and read the Bode plot.', tab: 'pid' },
      { text: 'Push the inverted pendulum while LQR balances it.', tab: 'lqr' },
    ],
    roles: ['controls', 'firmware'],
    scope: 'mixed',
    scopeNote: 'Head tracking uses your project’s IMU settings; the PID designer and pendulum are general.',
    terms: ['IMU', 'Complementary filter', 'PID', 'Bode plot', 'PM / GM', 'LQR'],
  },
  robotics: {
    question: 'How do robot arms move, and how long does automated assembly take?',
    inputs: ['Arm speed, Z stroke, vision and dwell times', 'Link lengths and joint limits', 'Trajectory limits', 'Waypoints'],
    outputs: [{ name: 'Pick-and-place cycle', good: 'Iris needs ≤ 60 s per board', metric: 'robotics.pnp_cycle_s' }],
    try3: [
      { text: 'Watch the arm populate the board and find the slowest move.', tab: 'pnp' },
      { text: 'Drag the target and flip elbow up / down.', tab: 'arm' },
      { text: 'Compare trapezoidal and quintic moves.', tab: 'traj' },
    ],
    roles: ['controls', 'manufacturing'],
    scope: 'mixed',
    scopeNote: 'Pick-and-place assembles the Iris board; the arm, trajectory and mobile-robot tabs are general.',
    terms: ['SCARA', 'IK / FK', 'PnP', 'CPH'],
  },
  rl: {
    question: 'Can a learned policy beat a hand-written rule — and how does reinforcement learning work?',
    inputs: ['Energy price and battery awareness', 'Episodes and seed', 'Gridworld walls, rewards, slip'],
    outputs: [{ name: 'Battery-life gain of the learned policy', good: '≥ 5 % over the fixed policy', metric: 'rl.policy_life_gain_pct' }],
    try3: [
      { text: 'Train the power policy and compare simulated days.', tab: 'power' },
      { text: 'Paint a gridworld and train Q-learning.', tab: 'grid' },
      { text: 'Train CartPole and watch the replay.', tab: 'cartpole' },
    ],
    roles: ['controls', 'firmware'],
    scope: 'mixed',
    scopeNote: 'The power policy uses the Iris energy model from the Power lab; gridworld and CartPole are general.',
    terms: ['RL', 'MDP', 'Q-learning', 'Policy'],
  },
  optics: {
    question: 'How big, sharp and bright is the floating picture, and where can the eye be?',
    inputs: ['Focal length, display position and size', 'Lens aperture, eye relief, combiner', 'Lens surfaces and glasses (Lens designer)'],
    outputs: [
      { name: 'Horizontal field of view', good: '≥ 20° for Iris', metric: 'optics.hfov_deg' },
      { name: 'Pixels per degree', good: '≥ 25; about 60 matches 20/20 vision', metric: 'optics.ppd' },
      { name: 'Eyebox', good: '≥ 5 mm wide', metric: 'optics.eyebox_mm' },
      { name: 'Lateral colour', good: '≤ 4.5 arcmin (two pixels) — an open goal', metric: 'optics.lateral_color_arcmin' },
    ],
    try3: [
      { text: 'Drag the focal length and watch field of view and pixels per degree trade.', tab: 'ray' },
      { text: 'Slide the eye sideways to find the eyebox edge.', tab: 'eyebox' },
      { text: 'Read the spot diagram and modulation transfer function (MTF) of the real lens.', tab: 'quality' },
    ],
    roles: ['optical', 'student'],
    scope: 'iris',
    scopeNote: 'Analyses the Iris display layout (micro-display, collimating lens, 45° combiner) from your project’s optics settings; the lens designer and materials tables are general.',
    terms: ['HUD', 'FOV', 'PPD', 'Eyebox', 'MTF', 'RMS spot', 'Lateral colour'],
  },
  cad: {
    question: 'What shape is the product, what does it weigh, and does the electronics fit?',
    inputs: ['Frame sizes: lens, bridge, temples, pods, wall, draft', 'Face width and pupil distance (Face fit)', 'Enclosure box, bosses and cut-outs'],
    outputs: [
      { name: 'Total wearable mass', good: '≤ 50 g for Iris', metric: 'mass.total_g' },
      { name: 'PCB-to-pod margin', good: '≥ 1 mm', metric: 'fit.pcb_pod_margin_mm' },
    ],
    try3: [
      { text: 'Drag “Lens width A” and watch mass change.', tab: 'design' },
      { text: 'Enter a face width and apply the recommended size.', tab: 'fit' },
      { text: 'Build a box for your own board and download the stereolithography (STL) file.', tab: 'enclosure' },
    ],
    roles: ['mechanical', 'student', 'optical', 'manufacturing'],
    scope: 'iris',
    scopeNote: 'Generates the Iris glasses frame from your sizes. The enclosure generator is general — use it for any board.',
    terms: ['CAD', 'STL', '3MF', 'Draft'],
  },
  print: {
    question: 'How long will it take to 3D-print, how much filament, and will it need supports?',
    inputs: ['Layer height, line width, walls, infill', 'Speeds and temperatures', 'Part to slice'],
    outputs: [
      { name: 'Frame-set print time', good: '≤ 8 h on a hobby printer for Iris', metric: 'print.total_time_h' },
      { name: 'Filament', good: 'Lower is cheaper', metric: 'print.filament_g' },
    ],
    try3: [
      { text: 'Play the layer trace animation.', tab: 'layers' },
      { text: 'Check overhangs beyond 45° that need support.', tab: 'overhang' },
      { text: 'Download the G-code.', tab: 'gcode' },
    ],
    roles: ['manufacturing', 'mechanical', 'student'],
    scope: 'iris',
    scopeNote: 'Slices the Iris frame parts (or an enclosure from the CAD Studio). Settings are typical starting points — tune on your printer.',
    terms: ['FDM', 'G-code', 'PLA', 'PETG'],
  },
  structures: {
    question: 'Will it break, bend, wear out or fail to fit?',
    inputs: ['Load case and force', 'Material', 'Mesh (voxel) size', 'Calculator inputs', 'Tolerances'],
    outputs: [
      { name: 'Minimum safety factor', good: '≥ 2 under the wear load cases', metric: 'structures.min_safety_factor' },
      { name: 'Worst-case PCB-in-pod clearance', good: '≥ 0 mm (it fits every unit)', metric: 'structures.pod_fit_worst_mm' },
      { name: 'Display focus error', good: '≤ 0.25 dioptre', metric: 'structures.focus_error_worst_d' },
      { name: 'Drop safety factor', good: '≥ 1 survives; below 1 expect damage (informational)', metric: 'structures.drop_safety_factor' },
    ],
    try3: [
      { text: 'Run the temple under a load case and find the critical point.', tab: 'strength' },
      { text: 'Size a snap fit or a screw boss.', tab: 'calculators' },
      { text: 'Run the Monte Carlo tolerance stack-up.', tab: 'tolerances' },
    ],
    roles: ['mechanical', 'test', 'optical', 'student'],
    scope: 'mixed',
    scopeNote: 'The 3-D finite element analysis runs on the Iris frame parts; the calculators (snap fits, bolts, beams, fits) are general.',
    terms: ['FEA', 'Safety factor', 'Stack-up', 'RSS', 'Topology optimisation'],
  },
  mechanisms: {
    question: 'How do I design things that move — and keep moving?',
    inputs: ['Linkage pivots and lengths', 'Spring wire, diameter, coils', 'Gear teeth and module', 'Cam motion program', 'Iris hinge spring'],
    outputs: [
      { name: 'Hinge spring fatigue safety factor', good: '≥ 1.5 (Goodman)', metric: 'mechanisms.hinge_spring_sf' },
      { name: 'Hinge life', good: '≥ 13,140 uses (12 a day for 3 years)', metric: 'mechanisms.hinge_life_cycles' },
      { name: 'Opening torque', good: '3–30 N·mm: not floppy, not stiff', metric: 'mechanisms.hinge_open_torque_nmm' },
    ],
    try3: [
      { text: 'Drag a four-bar pivot and watch the coupler curve.', tab: 'linkages' },
      { text: 'Design a compression spring and read its fatigue safety factor.', tab: 'springs' },
      { text: 'Open the Iris hinge and change the spring.', tab: 'hinge' },
    ],
    roles: ['mechanical', 'controls', 'test'],
    scope: 'mixed',
    scopeNote: 'Linkages, springs, gears and cams are general; the hinge tab designs the Iris spring hinge.',
    terms: ['Four-bar', 'Spring rate', 'Goodman'],
  },
  fluids: {
    question: 'Where does the heat really go on the frame, and how hot is the hottest spot?',
    inputs: ['Usage scenario', 'Heat spreader type and location', 'Frame material and emissivity', 'Air speed'],
    outputs: [
      { name: 'Peak frame surface temperature', good: '≤ 41 °C with the display always on', metric: 'fluids.peak_skin_c' },
      { name: 'Hotspot above mean', good: '≤ 10 °C — spread the heat', metric: 'fluids.hotspot_over_mean_c' },
    ],
    try3: [
      { text: 'Run the 3-D conduction and x-ray the lid.', tab: 'conduction' },
      { text: 'Compare graphite and copper spreaders.', tab: 'spreader' },
      { text: 'Grow a vortex street in the wind tunnel.', tab: 'tunnel' },
    ],
    roles: ['mechanical', 'test', 'electrical'],
    scope: 'iris',
    scopeNote: 'The 3-D heat conduction runs on the Iris frame geometry; the convection calculator and wind tunnel are general.',
    terms: ['FEA', 'Convection', 'LBM'],
  },
  ergonomics: {
    question: 'Does it fit real people, sit comfortably, and can they read it?',
    inputs: ['Fit rules (splay, decentration, ear window)', 'Population mix', 'Pad area and masses', 'Display brightness and background'],
    outputs: [
      { name: 'Fit coverage (one size)', good: '≥ 30 % of adults', metric: 'ergonomics.fit_coverage_pct' },
      { name: 'Nose-pad pressure', good: '≤ 4.3 kPa', metric: 'ergonomics.nose_pad_pressure_kpa' },
      { name: 'Eyebox coverage', good: '≥ 85 % see the whole image', metric: 'ergonomics.eyebox_coverage_pct' },
      { name: 'Outdoor contrast', good: '≥ 1.5 : 1 in sunlight — an open goal', metric: 'ergonomics.outdoor_contrast' },
    ],
    try3: [
      { text: 'Move the head-size sliders and watch the temples meet the head.', tab: 'fit' },
      { text: 'Move the battery towards the ear and watch nose load fall.', tab: 'balance' },
      { text: 'Check contrast against a sunlit street.', tab: 'display' },
    ],
    roles: ['mechanical', 'optical', 'systems'],
    scope: 'iris',
    scopeNote: 'Puts the Iris frame, masses and display on a simulated population (published head statistics).',
    terms: ['Anthropometry', 'IPD', 'ACR', 'Eyebox'],
  },
  molding: {
    question: 'Could this part be injection-molded, and what would that take?',
    inputs: ['Part and pull direction', 'Resin, tool steel, cavities'],
    outputs: [
      { name: 'Minimum draft', good: '≥ 1° so the part releases', metric: 'molding.min_draft_deg' },
      { name: 'Clamp force', good: 'Sets the press size — lower is cheaper', metric: 'molding.clamp_tonnage_t' },
      { name: 'Cycle time', good: 'Shorter is cheaper per part', metric: 'molding.cycle_time_s' },
    ],
    try3: [
      { text: 'Read the draft colours: red faces would stick.', tab: 'draft' },
      { text: 'Switch to the thickness map and find thick sections.', tab: 'thickness' },
      { text: 'Change cavities and watch tool and piece cost.' },
    ],
    roles: ['manufacturing', 'mechanical'],
    scope: 'iris',
    scopeNote: 'Checks the Iris frame parts from the CAD Studio. Costs are indicative — get real quotes.',
    terms: ['Draft', 'DFM', 'Clamp force', 'NRE'],
  },
  build: {
    question: 'What settings do I start from, which process do I use, and where do I send it?',
    inputs: ['Process and material', 'Calculator inputs (feeds, bends, stencil, reflow)', 'Quantity, tolerance, finish'],
    outputs: [
      { name: 'PCB rules beyond the fab tier', good: '0 — the board is standard-tier', metric: 'build.pcb_capability_violations' },
      { name: 'Smallest stencil area ratio', good: '≥ 0.66 for clean paste release', metric: 'build.stencil_min_area_ratio' },
    ],
    try3: [
      { text: 'Pick fused deposition modelling (FDM) and a filament to see typical settings.', tab: 'settings' },
      { text: 'Run the reflow profile animation.', tab: 'calc' },
      { text: 'Set quantity and tolerance and see where to send it.', tab: 'wizard' },
    ],
    roles: ['manufacturing', 'electrical', 'test'],
    scope: 'general',
    scopeNote: 'Settings, calculators and the process wizard work for anything; a few checks read your project’s board. Settings are typical starting points.',
    terms: ['FDM', 'Stencil area ratio', 'Reflow', 'DFM', 'IPC'],
  },
  mfg: {
    question: 'What does one unit cost at each quantity, and how do I order it?',
    inputs: ['Quantities and target quantity', 'Frame process', 'Assembly time and labour rate'],
    outputs: [
      { name: 'Unit cost at target quantity', good: '≤ 120 USD at 1,000 for Iris', metric: 'mfg.unit_cost_usd' },
      { name: 'Molding beats printing above', good: 'Tells you when tooling pays off', metric: 'mfg.crossover_qty' },
    ],
    try3: [
      { text: 'Hover the cost chart at 1,000 units.', tab: 'cost' },
      { text: 'Read an order guide.', tab: 'guides' },
      { text: 'Download the fabrication package.', tab: 'package' },
    ],
    roles: ['manufacturing', 'systems'],
    scope: 'project',
    scopeNote: 'Costs your project’s bill of materials and frame process. Indicative parametric estimates — get real quotes.',
    terms: ['Unit cost', 'BOM', 'NRE', 'MOQ', 'Crossover quantity'],
  },
  reliability: {
    question: 'Will it last, and how do I prove it with a test?',
    inputs: ['Failure data', 'Acceleration model and stresses', 'Reliability and confidence targets', 'Use profile'],
    outputs: [
      { name: 'Reliability over 2 years', good: '≥ 90 %', metric: 'reliability.r_2yr_pct' },
      { name: 'Electronics MTBF', good: '≥ 1,000,000 h (typical failure rates)', metric: 'reliability.mtbf_h' },
      { name: 'Units for the DVT plan', good: '≤ 40', metric: 'reliability.dvt_units' },
    ],
    try3: [
      { text: 'Load a sample data set and fit a Weibull.', tab: 'weibull' },
      { text: 'Find units for 90 % reliability at 90 % confidence.', tab: 'sample' },
      { text: 'Generate the design verification test (DVT) plan.', tab: 'dvt' },
    ],
    roles: ['test', 'systems', 'mechanical'],
    scope: 'mixed',
    scopeNote: 'Weibull, accelerated-life and sample-size tools are general; the System and Test-plan tabs read your project.',
    terms: ['Weibull', 'B10', 'MTBF', 'ALT', 'DVT'],
  },
  program: {
    question: 'When will it be ready, how sure am I, and is it a business?',
    inputs: ['Task estimates and dependencies', 'Monte Carlo iterations', 'Earned-value scenario', 'Price, volume, costs'],
    outputs: [
      { name: '80 % confidence finish', good: '≤ 52 weeks for Iris', metric: 'program.p80_weeks' },
      { name: 'Gross margin', good: '≥ 50 %', metric: 'program.gross_margin_pct' },
      { name: 'Lowest readiness level', good: 'TRL 6 by design review — an open goal', metric: 'program.min_trl' },
    ],
    try3: [
      { text: 'Play the Gantt chart and find the critical path.', tab: 'schedule' },
      { text: 'Run schedule risk and read the 80th-percentile (P80) finish.', tab: 'risk' },
      { text: 'Change the retail price and watch margin.', tab: 'economics' },
    ],
    roles: ['systems', 'test', 'manufacturing'],
    scope: 'project',
    scopeNote: 'Derives the plan from your project’s milestones, builds and compliance items, so it follows your product.',
    terms: ['CPM', 'P80', 'EVM', 'TRL', 'NPV', 'Gross margin'],
  },
  proto: {
    question: 'How do I build a working prototype and its firmware today?',
    inputs: ['Development-board build', 'Firmware target', 'Heads-up display (HUD) screen and gestures', 'Bluetooth Low Energy (BLE) connection settings'],
    outputs: [
      { name: 'RAM margin', good: '≥ 25 % free', metric: 'firmware.ram_margin_pct' },
      { name: 'Flash margin', good: '≥ 20 % (room for two update slots)', metric: 'firmware.flash_margin_pct' },
      { name: 'Caption latency', good: '≤ 100 ms feels live', metric: 'firmware.caption_latency_ms' },
    ],
    try3: [
      { text: 'Pick a build and watch the wiring draw.', tab: 'build' },
      { text: 'Download the generated firmware.', tab: 'firmware' },
      { text: 'Tap the touch pad in the HUD simulator.', tab: 'hud' },
    ],
    roles: ['firmware', 'electrical', 'rf'],
    scope: 'iris',
    scopeNote: 'Breadboards Iris from development boards; the firmware is generated from your project’s lab settings for the Iris architecture.',
    terms: ['MCU', 'BLE', 'HUD', 'OLED'],
  },
  quantum: {
    question: 'How do qubits, entanglement and quantum links really behave?',
    inputs: ['Gates on wires', 'Noise channels', 'Fiber length and detector specs', 'Random-number noise source and analog-to-digital converter (ADC)'],
    outputs: [
      { name: 'Bell test S', good: '> 2 shows entanglement; 2√2 ≈ 2.83 is the maximum' },
      { name: 'QKD key rate', good: '> 0 at your distance (Q-Link template)', metric: 'quantum.qkd_key_rate_bps' },
      { name: 'NIST test failures', good: '0 on the conditioned stream (Q-Link template)', metric: 'quantum.qrng_nist_failures' },
    ],
    try3: [
      { text: 'Place a Hadamard (H) then a controlled-NOT (CNOT) gate to make a Bell pair.', tab: 'composer' },
      { text: 'Play teleportation.', tab: 'protocols' },
      { text: 'Design a fiber key-distribution link.', tab: 'qkd' },
    ],
    roles: ['quantum', 'student'],
    scope: 'general',
    scopeNote: 'General quantum simulators. Iris has no quantum requirements; the Q-Link template (load it from the quantum key distribution (QKD) tab) is a quantum product verified by this lab.',
    terms: ['Qubit', 'Fidelity', 'CHSH', 'QKD', 'QRNG', 'OpenQASM'],
  },
};
