// Every acronym and term of art used across Protoforge, with its full form and
// a one-line definition. Keep alphabetical within the list; the page sorts anyway.

export interface Term {
  term: string;
  full: string;
  def: string;
  area: 'Systems' | 'Electronics' | 'Signals' | 'Silicon' | 'Control' | 'Robotics' | 'Learning' | 'Optics' | 'Mechanical' | 'Manufacturing' | 'Software';
  lab?: string; // route of the lab where it appears
}

export const GLOSSARY: Term[] = [
  // Systems engineering
  { term: 'V-model', full: 'V-model (verification and validation model)', area: 'Systems', lab: '/systems', def: 'Lifecycle diagram: decompose needs into requirements and designs down the left side, then integrate and verify back up the right side, each level checked against its partner.' },
  { term: 'N²', full: 'N-squared (N²) diagram', area: 'Systems', lab: '/systems', def: 'Square matrix with subsystems on the diagonal and the interfaces between them in the off-diagonal cells; exposes coupling at a glance.' },
  { term: 'FMEA', full: 'failure mode and effects analysis', area: 'Systems', lab: '/systems', def: 'Systematic table of how each item can fail, what the effect is, why it happens and how it is detected, used to rank and mitigate risks.' },
  { term: 'RPN', full: 'risk priority number', area: 'Systems', lab: '/systems', def: 'FMEA ranking score: severity × occurrence × detection, each rated 1–10, so 1 … 1000.' },
  { term: 'EVT', full: 'engineering validation test', area: 'Manufacturing', lab: '/mfg', def: 'First hardware build stage: prove the design works (functions, power, fit) using prototype processes.' },
  { term: 'DVT', full: 'design validation test', area: 'Manufacturing', lab: '/mfg', def: 'Second build stage on production-intent parts: certification, reliability and environmental testing.' },
  { term: 'PVT', full: 'production validation test', area: 'Manufacturing', lab: '/mfg', def: 'Final pilot run on the real production line to prove yield, cycle time and quality before mass production.' },
  { term: 'Trade study', full: 'trade study (decision matrix)', area: 'Systems', lab: '/systems', def: 'Weighted scoring of design options against criteria; Protoforge also shows how sensitive the winner is to the weights.' },
  { term: 'Traceability', full: 'requirements traceability', area: 'Systems', lab: '/systems', def: 'Links from each need to the requirements it derives, the subsystems they are allocated to and the metric that verifies them.' },
  { term: 'INCOSE', full: 'International Council on Systems Engineering', area: 'Systems', def: 'Professional society that publishes the Systems Engineering Handbook cited by the Systems Studio.' },
  { term: 'NASA', full: 'National Aeronautics and Space Administration', area: 'Systems', def: 'United States space agency; its Systems Engineering Handbook (SP-2016-6105) is a free standard reference.' },
  { term: 'BOM', full: 'bill of materials', area: 'Manufacturing', lab: '/mfg', def: 'List of every part, quantity, reference designator and (for assembly) manufacturer part number that makes up a product or board.' },
  { term: 'CPL', full: 'component placement list', area: 'Manufacturing', lab: '/pcb', def: 'Also called a centroid or pick-and-place file: X/Y position, rotation and side of every part, used by the assembly machine.' },
  { term: 'DFM', full: 'design for manufacturing', area: 'Manufacturing', lab: '/molding', def: 'Shaping a design so it is easy and cheap to make with the chosen process (draft angles, uniform walls, trace clearances…).' },
  { term: 'DFA', full: 'design for assembly', area: 'Manufacturing', lab: '/mfg', def: 'Reducing part count and assembly steps; with DFM forms DFMA (Boothroyd & Dewhurst).' },
  { term: 'NRE', full: 'non-recurring engineering', area: 'Manufacturing', lab: '/mfg', def: 'One-off costs such as mold tooling or stencils, amortised over the production quantity.' },
  { term: 'MOQ', full: 'minimum order quantity', area: 'Manufacturing', lab: '/mfg', def: 'Smallest quantity a supplier will sell or build.' },
  { term: 'USD', full: 'United States dollar', area: 'Manufacturing', def: 'Currency used for all indicative cost estimates.' },
  { term: 'CPH', full: 'components per hour', area: 'Manufacturing', lab: '/robotics', def: 'Throughput rating of a pick-and-place machine.' },

  // Electronics
  { term: 'SPICE', full: 'Simulation Program with Integrated Circuit Emphasis', area: 'Electronics', lab: '/circuit', def: 'The Berkeley circuit simulator (Nagel, 1975) whose netlist format and analyses (.op, .dc, .tran, .ac) the Circuit Lab follows.' },
  { term: 'MNA', full: 'modified nodal analysis', area: 'Electronics', lab: '/circuit', def: 'Circuit formulation: Kirchhoff current law at every node plus one extra equation per voltage source, solved as G·x = b.' },
  { term: 'KCL', full: 'Kirchhoff’s current law', area: 'Electronics', lab: '/circuit', def: 'The currents into any node sum to zero; the backbone of nodal analysis.' },
  { term: 'DC', full: 'direct current', area: 'Electronics', lab: '/circuit', def: 'Constant voltages and currents; a DC operating point (.op) is the circuit’s steady state with capacitors open and inductors shorted.' },
  { term: 'AC', full: 'alternating current (small-signal analysis)', area: 'Electronics', lab: '/circuit', def: 'In SPICE, a linearised frequency sweep around the operating point that gives gain and phase (Bode plots).' },
  { term: 'ESR', full: 'equivalent series resistance', area: 'Electronics', lab: '/circuit', def: 'The parasitic resistance of a capacitor; sets output ripple and damping in a buck converter.' },
  { term: 'MOSFET', full: 'metal–oxide–semiconductor field-effect transistor', area: 'Electronics', lab: '/circuit', def: 'Voltage-controlled switch/amplifier; modelled with the Shichman–Hodges (level 1) equations.' },
  { term: 'Buck', full: 'buck (step-down) converter', area: 'Electronics', lab: '/circuit', def: 'Switching regulator that chops a higher voltage and filters it with an inductor and capacitor to a lower one.' },
  { term: 'SoC (battery)', full: 'state of charge', area: 'Electronics', lab: '/power', def: 'Fraction of a battery’s usable capacity remaining, 0–100 %.' },
  { term: 'SoC (chip)', full: 'system on chip', area: 'Silicon', lab: '/chip', def: 'An integrated circuit that combines processor cores, memory and peripherals on one die.' },
  { term: 'LiPo', full: 'lithium-ion polymer battery', area: 'Electronics', lab: '/power', def: 'Flat pouch lithium-ion cell (≈ 3.7 V nominal, 4.2 V full) used in wearables.' },
  { term: 'BLE', full: 'Bluetooth Low Energy', area: 'Electronics', lab: '/power', def: 'Low-power radio link used by Iris to talk to a phone.' },
  { term: 'MCU', full: 'microcontroller unit', area: 'Electronics', lab: '/pcb', def: 'Single-chip computer with processor, memory and peripherals.' },
  { term: 'IMU', full: 'inertial measurement unit', area: 'Control', lab: '/control', def: 'Accelerometer + gyroscope (sometimes magnetometer) package; Iris fuses it to track head orientation.' },
  { term: 'OLED', full: 'organic light-emitting diode', area: 'Optics', lab: '/optics', def: 'Self-emissive display technology; Iris uses a 640 × 400 micro-OLED panel.' },
  { term: 'USB', full: 'Universal Serial Bus', area: 'Electronics', lab: '/pcb', def: 'Connector and protocol used for charging and data.' },
  { term: 'PCB', full: 'printed circuit board', area: 'Electronics', lab: '/pcb', def: 'Laminated copper and fibreglass board that mechanically holds and electrically connects components.' },
  { term: 'DRC', full: 'design rule check', area: 'Electronics', lab: '/pcb', def: 'Automatic check of clearances, trace widths, drill sizes and connectivity against the fab’s capabilities.' },
  { term: 'HPWL', full: 'half-perimeter wirelength', area: 'Electronics', lab: '/pcb', def: 'Fast placement quality estimate: for each net, half the perimeter of the bounding box of its pins, summed.' },
  { term: 'IPC', full: 'IPC (formerly Institute for Printed Circuits)', area: 'Electronics', lab: '/pcb', def: 'Electronics-industry association whose standards (IPC-2221 trace sizing, IPC-7351 land patterns) the PCB Studio uses.' },
  { term: 'SMD / SMT', full: 'surface-mount device / technology', area: 'Electronics', lab: '/pcb', def: 'Components soldered onto pads on the board surface rather than through holes.' },
  { term: 'QFN', full: 'quad flat no-lead package', area: 'Electronics', lab: '/pcb', def: 'Compact IC package with pads on the underside edges and a thermal pad in the middle.' },
  { term: 'ENIG', full: 'electroless nickel immersion gold', area: 'Electronics', lab: '/pcb', def: 'Flat, solderable PCB surface finish suited to fine-pitch parts.' },
  { term: 'GND', full: 'ground', area: 'Electronics', lab: '/pcb', def: 'The 0 V reference net; on Iris a stitched copper pour on both layers.' },
  { term: 'Gerber', full: 'Gerber format (RS-274X / X2)', area: 'Electronics', lab: '/pcb', def: 'Ucamco’s vector file format, one file per copper, mask, silkscreen and outline layer — the universal PCB fab input.' },

  // Signals
  { term: 'DSP', full: 'digital signal processing', area: 'Signals', lab: '/dsp', def: 'Processing sampled signals with arithmetic: filtering, transforms, estimation.' },
  { term: 'DFT', full: 'discrete Fourier transform', area: 'Signals', lab: '/dsp', def: 'Decomposes N samples into N complex frequency components.' },
  { term: 'FFT', full: 'fast Fourier transform', area: 'Signals', lab: '/dsp', def: 'O(N log N) algorithm for the DFT (Cooley & Tukey, 1965).' },
  { term: 'FIR', full: 'finite impulse response (filter)', area: 'Signals', lab: '/dsp', def: 'Filter whose output is a weighted sum of recent inputs; always stable, can have exactly linear phase.' },
  { term: 'IIR', full: 'infinite impulse response (filter)', area: 'Signals', lab: '/dsp', def: 'Recursive filter using past outputs; very efficient, built from second-order “biquad” sections.' },
  { term: 'CIC', full: 'cascaded integrator–comb (filter)', area: 'Signals', lab: '/dsp', def: 'Multiplier-free decimation filter (Hogenauer, 1981) of integrators and combs; needs a compensation FIR.' },
  { term: 'PDM', full: 'pulse-density modulation', area: 'Signals', lab: '/dsp', def: '1-bit high-rate stream from a digital microphone; the density of 1s encodes the waveform.' },
  { term: 'PCM', full: 'pulse-code modulation', area: 'Signals', lab: '/dsp', def: 'Ordinary multi-bit samples at a fixed rate, the output of the decimation chain.' },
  { term: 'PSD', full: 'power spectral density', area: 'Signals', lab: '/dsp', def: 'Power per unit frequency; estimated with Welch’s averaged periodogram.' },
  { term: 'SNR', full: 'signal-to-noise ratio', area: 'Signals', lab: '/dsp', def: 'Ratio of signal power to noise power, usually in decibels.' },
  { term: 'SNDR', full: 'signal-to-noise-and-distortion ratio', area: 'Signals', lab: '/dsp', def: 'Like SNR but counting harmonic distortion as noise too.' },
  { term: 'ENBW', full: 'equivalent noise bandwidth', area: 'Signals', lab: '/dsp', def: 'Width of the ideal rectangular filter that passes the same noise power as a window or filter.' },
  { term: 'OSR', full: 'oversampling ratio', area: 'Signals', lab: '/dsp', def: 'Modulator clock rate divided by twice the signal bandwidth.' },
  { term: 'NTF', full: 'noise transfer function', area: 'Signals', lab: '/dsp', def: 'How a delta-sigma modulator shapes its quantisation noise out of the signal band.' },
  { term: 'DI', full: 'directivity index', area: 'Signals', lab: '/dsp', def: 'How much a microphone array favours its look direction over diffuse noise, in dB.' },
  { term: 'WNG', full: 'white noise gain', area: 'Signals', lab: '/dsp', def: 'Array gain against uncorrelated sensor noise; negative values mean the beamformer amplifies self-noise.' },
  { term: 'EQ', full: 'equaliser', area: 'Signals', lab: '/dsp', def: 'Filter that shapes the frequency response (the audio-EQ biquad cookbook).' },
  { term: 'dB', full: 'decibel', area: 'Signals', lab: '/dsp', def: 'Logarithmic ratio: 10·log₁₀ of a power ratio or 20·log₁₀ of an amplitude ratio.' },
  { term: 'RMS', full: 'root mean square', area: 'Signals', lab: '/control', def: 'Square root of the mean of the squares — the “effective” size of a varying signal or error.' },

  // Silicon
  { term: 'RTL', full: 'register-transfer level', area: 'Silicon', lab: '/chip', def: 'Hardware description in terms of registers and the logic between them — what you write in Verilog.' },
  { term: 'HDL', full: 'hardware description language', area: 'Silicon', lab: '/chip', def: 'Language such as Verilog or VHDL that describes digital circuits.' },
  { term: 'FSM', full: 'finite-state machine', area: 'Silicon', lab: '/chip', def: 'A circuit that moves between a finite set of states on inputs (Moore or Mealy); Iris decodes tap gestures with one.' },
  { term: 'MAC', full: 'multiply–accumulate', area: 'Silicon', lab: '/chip', def: 'The a ← a + x·h operation at the heart of FIR filters and neural networks.' },
  { term: 'CSD', full: 'canonical signed digit', area: 'Silicon', lab: '/chip', def: 'Number representation with digits −1, 0, +1 and no two adjacent non-zeros, minimising adders in constant multipliers.' },
  { term: 'ASIC', full: 'application-specific integrated circuit', area: 'Silicon', lab: '/chip', def: 'Custom chip made for one purpose, e.g. the IRIS-DSP co-processor tile.' },
  { term: 'FPGA', full: 'field-programmable gate array', area: 'Silicon', lab: '/chip', def: 'Reconfigurable chip of lookup tables and flip-flops; a common way to test RTL before tape-out.' },
  { term: 'CMOS', full: 'complementary metal–oxide–semiconductor', area: 'Silicon', lab: '/chip', def: 'Logic family built from paired n- and p-channel transistors; the basis of standard cells.' },
  { term: 'VLSI', full: 'very-large-scale integration', area: 'Silicon', lab: '/chip', def: 'Designing chips with millions to billions of transistors.' },
  { term: 'PDK', full: 'process design kit', area: 'Silicon', lab: '/chip', def: 'Foundry data (models, rules, cell libraries) for a process such as the open SkyWater SKY130.' },
  { term: 'GDS', full: 'GDSII (Graphic Design System II) stream format', area: 'Silicon', lab: '/chip', def: 'The layout file sent to the foundry at tape-out.' },
  { term: 'STA', full: 'static timing analysis', area: 'Silicon', lab: '/chip', def: 'Checks every register-to-register path meets the clock period without simulation (OpenSTA in OpenROAD).' },
  { term: 'Tape-out', full: 'tape-out', area: 'Silicon', lab: '/chip', def: 'Sending the final layout to fabrication; Tiny Tapeout shares one shuttle among hundreds of small designs.' },

  // Control
  { term: 'PID', full: 'proportional–integral–derivative (controller)', area: 'Control', lab: '/control', def: 'The workhorse feedback law u = Kp·e + Ki∫e + Kd·de/dt.' },
  { term: 'LQR', full: 'linear–quadratic regulator', area: 'Control', lab: '/control', def: 'Optimal state feedback u = −Kx minimising ∫(xᵀQx + uᵀRu) dt, found from the algebraic Riccati equation.' },
  { term: 'LTI', full: 'linear time-invariant (system)', area: 'Control', lab: '/control', def: 'System described by constant-coefficient linear differential equations; transfer functions and Bode plots apply.' },
  { term: 'TF', full: 'transfer function', area: 'Control', lab: '/control', def: 'Ratio of output to input Laplace transforms, G(s) = N(s)/D(s).' },
  { term: 'PM / GM', full: 'phase margin / gain margin', area: 'Control', lab: '/control', def: 'How much extra phase lag or gain a loop tolerates before instability; robustness measures read from a Bode plot.' },
  { term: 'ZOH', full: 'zero-order hold', area: 'Control', lab: '/control', def: 'Discretisation that holds each control output constant for one sample period.' },
  { term: 'SIMC', full: 'Skogestad internal model control (tuning rules)', area: 'Control', lab: '/control', def: 'Simple analytic PID tuning from a first-order-plus-delay model (Skogestad, 2003).' },
  { term: 'EKF / UKF', full: 'extended / unscented Kalman filter', area: 'Control', lab: '/control', def: 'Nonlinear state estimators; the professional next step beyond the complementary filter used for Iris.' },

  // Robotics
  { term: 'IK / FK', full: 'inverse / forward kinematics', area: 'Robotics', lab: '/robotics', def: 'FK: joint angles → tool pose. IK: desired tool pose → joint angles (possibly several solutions).' },
  { term: 'SCARA', full: 'selective compliance assembly robot arm', area: 'Robotics', lab: '/robotics', def: 'Two-link planar arm with a vertical axis — stiff vertically, compliant sideways; ideal for pick-and-place.' },
  { term: 'PnP', full: 'pick-and-place', area: 'Robotics', lab: '/robotics', def: 'Machine or task that picks components from feeders and places them on a board.' },
  { term: 'ROS', full: 'Robot Operating System', area: 'Robotics', def: 'Open-source robotics middleware (ROS 2) with MoveIt 2 for manipulation and Nav2 for navigation.' },

  // Learning
  { term: 'RL', full: 'reinforcement learning', area: 'Learning', lab: '/rl', def: 'Learning a policy by trial and error from rewards rather than labelled examples.' },
  { term: 'MDP', full: 'Markov decision process', area: 'Learning', lab: '/rl', def: 'States, actions, transition probabilities and rewards where the future depends only on the present state.' },
  { term: 'Q-learning', full: 'Q-learning', area: 'Learning', lab: '/rl', def: 'Off-policy temporal-difference method that learns action values Q(s, a) (Watkins & Dayan, 1992).' },
  { term: 'SARSA', full: 'state–action–reward–state–action', area: 'Learning', lab: '/rl', def: 'On-policy counterpart of Q-learning that updates towards the action actually taken next.' },
  { term: 'DQN', full: 'deep Q-network', area: 'Learning', lab: '/rl', def: 'Q-learning with a neural network approximating Q; the next step beyond tables.' },
  { term: 'PPO', full: 'proximal policy optimisation', area: 'Learning', lab: '/rl', def: 'Popular policy-gradient algorithm with a clipped objective (in Stable-Baselines3).' },

  // Optics
  { term: 'HUD', full: 'heads-up display', area: 'Optics', lab: '/optics', def: 'See-through display that overlays information on the wearer’s view.' },
  { term: 'FOV', full: 'field of view', area: 'Optics', lab: '/optics', def: 'Angular size of the virtual image; HFOV is the horizontal field of view.' },
  { term: 'PPD', full: 'pixels per degree', area: 'Optics', lab: '/optics', def: 'Angular resolution: pixels across the image divided by its field of view (≈ 60 matches 20/20 acuity).' },
  { term: 'Eyebox', full: 'eyebox', area: 'Optics', lab: '/optics', def: 'Region where the eye can move and still see the whole image.' },
  { term: 'PD', full: 'pupillary distance', area: 'Optics', lab: '/cad', def: 'Distance between the pupil centres, used to size the frame.' },
  { term: 'nit', full: 'nit (candela per square metre)', area: 'Optics', lab: '/optics', def: 'Unit of luminance, i.e. display brightness.' },

  // Mechanical / printing / molding
  { term: 'CAD', full: 'computer-aided design', area: 'Mechanical', lab: '/cad', def: 'Software for defining geometry; the CAD Studio generates the Iris frame parametrically.' },
  { term: 'CSG', full: 'constructive solid geometry', area: 'Mechanical', lab: '/cad', def: 'Building solids from Boolean union, difference and intersection of primitives (OpenSCAD’s model).' },
  { term: 'B-rep', full: 'boundary representation', area: 'Mechanical', lab: '/cad', def: 'Exact solid model of faces, edges and vertices used by professional CAD kernels; exchanged as STEP.' },
  { term: 'STL', full: 'stereolithography (file format)', area: 'Mechanical', lab: '/cad', def: 'List of triangles describing a surface; the lowest-common-denominator 3D printing format.' },
  { term: '3MF', full: '3D Manufacturing Format', area: 'Mechanical', lab: '/cad', def: 'Zip-based print format carrying meshes, units, colours and build plates.' },
  { term: 'OBJ', full: 'Wavefront OBJ', area: 'Mechanical', lab: '/cad', def: 'Plain-text mesh format common in graphics tools.' },
  { term: 'STEP', full: 'Standard for the Exchange of Product model data (ISO 10303)', area: 'Mechanical', lab: '/molding', def: 'Neutral B-rep CAD format that tooling shops quote from.' },
  { term: 'FDM', full: 'fused deposition modelling', area: 'Mechanical', lab: '/print', def: 'Extruding molten filament layer by layer; the hobby 3D-printing process (also called FFF).' },
  { term: 'G-code', full: 'G-code (RS-274)', area: 'Mechanical', lab: '/print', def: 'Line-by-line machine instructions (G1 X… Y… E…) that drive printers and CNC machines.' },
  { term: 'MJF', full: 'Multi Jet Fusion', area: 'Manufacturing', lab: '/mfg', def: 'HP’s powder-bed nylon printing process, good for small production runs.' },
  { term: 'SLS', full: 'selective laser sintering', area: 'Manufacturing', lab: '/mfg', def: 'Powder-bed printing where a laser fuses nylon powder; no supports needed.' },
  { term: 'PETG', full: 'polyethylene terephthalate glycol-modified', area: 'Mechanical', lab: '/print', def: 'Tough, slightly flexible printing filament used for the Iris prototype frame.' },
  { term: 'PLA', full: 'polylactic acid', area: 'Mechanical', lab: '/print', def: 'Easy-to-print, stiff but brittle filament.' },
  { term: 'PC', full: 'polycarbonate', area: 'Manufacturing', lab: '/molding', def: 'Tough, clear engineering thermoplastic; the Iris molding material.' },
  { term: 'ABS', full: 'acrylonitrile butadiene styrene', area: 'Manufacturing', lab: '/molding', def: 'Common, easily molded engineering plastic.' },
  { term: 'Draft', full: 'draft angle', area: 'Manufacturing', lab: '/molding', def: 'Taper on walls parallel to the mold’s pull direction so the part releases; typically ≥ 1°.' },
  { term: 'FEA', full: 'finite element analysis', area: 'Mechanical', def: 'Numerical stress, thermal or vibration analysis on a mesh — the professional step beyond Protoforge’s lumped models.' },
  { term: 'CFD', full: 'computational fluid dynamics', area: 'Mechanical', def: 'Numerical simulation of fluid flow and heat transfer.' },

  // Software
  { term: 'JSON', full: 'JavaScript Object Notation', area: 'Software', def: 'Text data format; every engine call takes and returns JSON.' },
  { term: 'API', full: 'application programming interface', area: 'Software', def: 'The set of engine methods such as dsp.fir_design, callable from the browser, CLI or REST server.' },
  { term: 'CLI', full: 'command-line interface', area: 'Software', def: 'The protoforge binary: evaluate, report, package, call …' },
  { term: 'REST', full: 'representational state transfer', area: 'Software', def: 'HTTP API style; forge-server exposes POST /api/{method}.' },
  { term: 'WASM', full: 'WebAssembly', area: 'Software', def: 'Portable binary format browsers run at near-native speed; the Rust engine is compiled to it.' },
  { term: 'UI', full: 'user interface', area: 'Software', def: 'The React front end.' },
  { term: 'CI', full: 'continuous integration', area: 'Software', def: 'Automated build and test on every push (GitHub Actions), which also re-verifies Iris.' },
  { term: 'CORS', full: 'cross-origin resource sharing', area: 'Software', def: 'Browser rule for calling an API on another origin; the local server allows it for development.' },
  { term: 'CSV', full: 'comma-separated values', area: 'Software', def: 'Plain-text table format used for every data download.' },
  { term: 'SVG', full: 'Scalable Vector Graphics', area: 'Software', def: 'Vector image format used for plots, schematics and diagrams.' },
  { term: 'WSL2', full: 'Windows Subsystem for Linux 2', area: 'Software', def: 'Runs a real Linux kernel on Windows; the recommended development setup on Windows.' },
];

export const AREAS: Term['area'][] = ['Systems', 'Electronics', 'Signals', 'Silicon', 'Control', 'Robotics', 'Learning', 'Optics', 'Mechanical', 'Manufacturing', 'Software'];
