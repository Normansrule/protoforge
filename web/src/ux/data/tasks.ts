// "I want to…" — everyday engineering jobs mapped to the lab tab that does
// them, with a one-line how. Grouped by discipline (role ids from roles.ts).

export interface Task {
  id: string;
  /** Finishes the sentence "I want to …". */
  title: string;
  role: string;
  lab: string;
  tab?: string;
  how: string;
  /** Extra words people might search with. */
  keywords?: string;
}

export const TASKS: Task[] = [
  // Electrical
  { id: 'battery', title: 'size a battery', role: 'electrical', lab: 'power', tab: 'trade', how: 'Sweep capacity and watch battery life rise against total mass; apply the capacity you pick.', keywords: 'lipo cell mah capacity runtime' },
  { id: 'battery-life', title: 'estimate battery life', role: 'electrical', lab: 'power', tab: 'budget', how: 'Choose a usage scenario; the duty-cycle budget gives hours of life and average power.', keywords: 'runtime hours power budget' },
  { id: 'discharge', title: 'see a battery drain over a day', role: 'electrical', lab: 'power', tab: 'discharge', how: 'Play the simulated day; change temperature, ageing and cutoff voltage.', keywords: 'ocv voltage cold' },
  { id: 'ripple', title: 'check regulator ripple', role: 'electrical', lab: 'circuit', tab: 'sim', how: 'Open the buck converter preset and read the ripple measurement as you drag the sliders.', keywords: 'buck converter switching smps' },
  { id: 'simulate', title: 'simulate a circuit', role: 'electrical', lab: 'circuit', tab: 'sim', how: 'Pick a preset or paste a Simulation Program with Integrated Circuit Emphasis (SPICE) netlist; transient, AC and DC results animate on the drawing.', keywords: 'spice netlist transient ac' },
  { id: 'schematic', title: 'draw a schematic and simulate it', role: 'electrical', lab: 'circuit', tab: 'editor', how: 'Place parts and wires in the schematic editor; it extracts a netlist and simulates it.', keywords: 'draw parts wires' },
  { id: 'route', title: 'route a board', role: 'electrical', lab: 'pcb', tab: 'board', how: 'Edit design rules and press Re-route; the design-rule check (DRC) re-reads the copper.', keywords: 'pcb layout autorouter traces drc' },
  { id: 'gerbers', title: 'get Gerbers for a board house', role: 'electrical', lab: 'pcb', tab: 'files', how: 'Preview each layer parsed from the files, then download the zip with drill, bill of materials (BOM) and pick-and-place.', keywords: 'gerber drill bom cpl jlcpcb fab' },
  { id: 'thermal', title: 'check skin temperature', role: 'electrical', lab: 'power', tab: 'thermal', how: 'Run the pod thermal network with the display on; add firmware throttling if it gets warm.', keywords: 'heat hot temperature skin' },
  { id: 'stencil', title: 'check a solder-paste stencil', role: 'electrical', lab: 'build', tab: 'calc', how: 'The stencil card checks every pad’s area ratio against the 0.66 guideline.', keywords: 'paste smt aperture' },
  { id: 'reflow', title: 'plan a reflow profile', role: 'electrical', lab: 'build', tab: 'calc', how: 'Animate a board through the oven and download the profile as CSV (typical — check your paste datasheet).', keywords: 'oven solder profile' },

  // Mechanical
  { id: 'strong', title: 'check a part is strong enough', role: 'mechanical', lab: 'structures', tab: 'strength', how: 'Run the 3-D finite element analysis under a load case and read the safety factor.', keywords: 'stress fea break yield strength' },
  { id: 'lighter', title: 'make a part lighter', role: 'mechanical', lab: 'structures', tab: 'topology', how: 'Run topology optimisation and export the outline as SVG or OpenSCAD.', keywords: 'topology weight simp' },
  { id: 'snap', title: 'design a snap fit or screw boss', role: 'mechanical', lab: 'structures', tab: 'calculators', how: 'Use the snap-fit, bolted-joint and boss calculators with their diagrams.', keywords: 'snap fit screw boss insert bolt' },
  { id: 'tolerance', title: 'check parts fit with tolerances', role: 'mechanical', lab: 'structures', tab: 'tolerances', how: 'Run worst-case, root-sum-square and Monte Carlo stack-ups of the board in its pod.', keywords: 'stack-up tolerance fit' },
  { id: 'spring', title: 'pick a spring', role: 'mechanical', lab: 'mechanisms', tab: 'springs', how: 'Set wire, diameter and coils; read rate, stress, buckling and fatigue safety factor.', keywords: 'compression torsion extension spring' },
  { id: 'gear', title: 'design a gear pair', role: 'mechanical', lab: 'mechanisms', tab: 'gears', how: 'Generate involute gears, watch them mesh and read contact ratio and undercut.', keywords: 'gears involute ratio' },
  { id: 'linkage', title: 'design a linkage', role: 'mechanical', lab: 'mechanisms', tab: 'linkages', how: 'Drag the pivots of a four-bar and read Grashof type, coupler curve and transmission angle.', keywords: 'four-bar mechanism' },
  { id: 'hinge', title: 'check a hinge lasts', role: 'mechanical', lab: 'mechanisms', tab: 'hinge', how: 'The Iris spring hinge shows torque, stress, Goodman safety factor and life in uses.', keywords: 'hinge fatigue life' },
  { id: 'heat', title: 'find hot spots on a part', role: 'mechanical', lab: 'fluids', tab: 'conduction', how: 'Run the 3-D heat conduction on the frame and try a graphite or copper spreader.', keywords: 'thermal fea hot spreader' },
  { id: 'enclosure', title: 'make an enclosure for a board', role: 'mechanical', lab: 'cad', tab: 'enclosure', how: 'Set box size, bosses and cut-outs and download a stereolithography (STL) mesh for printing.', keywords: 'box case stl enclosure' },
  { id: 'fit-heads', title: 'check fit for different head sizes', role: 'mechanical', lab: 'ergonomics', tab: 'fit', how: 'Try the frame on a simulated population and read fit coverage; search S/M/L sizes.', keywords: 'anthropometry head size population fit' },
  { id: 'frame', title: 'change the frame shape', role: 'mechanical', lab: 'cad', tab: 'design', how: 'Drag the frame sliders; mass, fit and every downstream lab update live.', keywords: 'cad parametric glasses' },

  // Firmware
  { id: 'firmware', title: 'generate firmware', role: 'firmware', lab: 'proto', tab: 'firmware', how: 'Choose a target board and download a project whose constants come from the labs.', keywords: 'code arduino zephyr esp32 nrf52' },
  { id: 'breadboard', title: 'breadboard a prototype', role: 'firmware', lab: 'proto', tab: 'build', how: 'Pick a development-board build, follow the wiring animation and tick the checklist.', keywords: 'dev board wiring xiao' },
  { id: 'ble', title: 'design a Bluetooth protocol', role: 'firmware', lab: 'proto', tab: 'ble', how: 'Set connection interval, maximum transmission unit (MTU) and physical layer (PHY); read throughput and caption latency and download the GATT table.', keywords: 'ble gatt bluetooth' },
  { id: 'filter', title: 'design a filter', role: 'firmware', lab: 'dsp', tab: 'filter', how: 'Design finite impulse response (FIR) or infinite impulse response (IIR) filters from specs and see magnitude, phase and fixed-point effects.', keywords: 'fir iir lowpass dsp' },
  { id: 'fft', title: 'look at a spectrum', role: 'firmware', lab: 'dsp', tab: 'spectrum', how: 'Build a signal, pick a window and read the spectrum; play it as sound.', keywords: 'fft window frequency' },
  { id: 'verilog', title: 'turn logic into Verilog', role: 'firmware', lab: 'chip', tab: 'fir', how: 'Generate a FIR filter in Verilog with timing and cell estimates.', keywords: 'hdl rtl fpga asic' },
  { id: 'fsm', title: 'design a state machine', role: 'firmware', lab: 'chip', tab: 'fsm', how: 'Edit the touch-gesture finite state machine and simulate taps.', keywords: 'fsm gestures states' },

  // Optical
  { id: 'fov', title: 'set a display’s field of view', role: 'optical', lab: 'optics', tab: 'ray', how: 'Change focal length and display size in the ray diagram; read FOV and pixels per degree.', keywords: 'fov hud magnifier' },
  { id: 'lens', title: 'design a lens', role: 'optical', lab: 'optics', tab: 'lens', how: 'Edit surfaces and glasses, then let the optimiser reduce spot size.', keywords: 'lens zemax optimise aspheric' },
  { id: 'sharp', title: 'check image sharpness', role: 'optical', lab: 'optics', tab: 'quality', how: 'Read spot diagrams, ray fans and the modulation transfer function (MTF) across the field.', keywords: 'mtf spot aberration' },
  { id: 'sunlight', title: 'check readability in sunlight', role: 'optical', lab: 'ergonomics', tab: 'display', how: 'Read ambient contrast ratio against a bright street and eyebox coverage.', keywords: 'contrast outdoor legibility nits' },

  // RF
  { id: 'match', title: 'match an antenna', role: 'rf', lab: 'rf', tab: 'matching', how: 'Drag the load on the Smith chart and pick an L-network snapped to standard values.', keywords: 'smith chart matching return loss' },
  { id: 'range', title: 'estimate radio range', role: 'rf', lab: 'rf', tab: 'link', how: 'Fill in the link budget with head and body loss and read margin and range.', keywords: 'link budget range bluetooth' },
  { id: 'trace50', title: 'size a 50 Ω trace', role: 'rf', lab: 'rf', tab: 'lines', how: 'Use the coplanar-waveguide calculator on your board stack-up.', keywords: 'impedance microstrip cpwg z0' },
  { id: 'fields', title: 'see an antenna radiate', role: 'rf', lab: 'rf', tab: 'fields', how: 'Run the finite-difference time-domain (FDTD) field solver with a dipole, an array or an antenna beside a head.', keywords: 'fdtd waves maxwell' },

  // Controls & robotics
  { id: 'pid', title: 'tune a PID loop', role: 'controls', lab: 'control', tab: 'pid', how: 'Pick a plant, autotune the proportional–integral–derivative (PID) gains, then read step response and Bode margins.', keywords: 'pid gains kp ki kd' },
  { id: 'fusion', title: 'fuse gyro and accelerometer', role: 'controls', lab: 'control', tab: 'head', how: 'Tune the complementary filter and compare with a Kalman filter on simulated head motion.', keywords: 'imu kalman complementary' },
  { id: 'lqr', title: 'balance an inverted pendulum', role: 'controls', lab: 'control', tab: 'lqr', how: 'Set linear–quadratic regulator (LQR) weights and push the pole while it balances.', keywords: 'lqr state feedback cart pole' },
  { id: 'arm', title: 'solve robot arm kinematics', role: 'controls', lab: 'robotics', tab: 'arm', how: 'Drag a target and see both inverse-kinematics solutions and singularities.', keywords: 'ik fk scara jacobian' },
  { id: 'rl', title: 'train a reinforcement-learning policy', role: 'controls', lab: 'rl', tab: 'grid', how: 'Paint a gridworld and train Q-learning or SARSA; compare with value iteration.', keywords: 'q-learning sarsa mdp' },

  // Manufacturing
  { id: 'print', title: 'pick 3D-print settings', role: 'manufacturing', lab: 'build', tab: 'settings', how: 'Choose fused deposition modelling (FDM) and a filament for typical temperatures, speeds and bed surface (tune on your printer).', keywords: 'bambu fdm pla petg filament' },
  { id: 'slice', title: 'slice a part and get G-code', role: 'manufacturing', lab: 'print', tab: 'gcode', how: 'Slice any frame part and download generic Marlin/Klipper G-code.', keywords: 'slicer gcode layers' },
  { id: 'process', title: 'choose a manufacturing process', role: 'manufacturing', lab: 'build', tab: 'wizard', how: 'Set quantity, material and tolerance; processes are ranked with where to send each part.', keywords: 'process selection vendor where' },
  { id: 'cost', title: 'estimate unit cost', role: 'manufacturing', lab: 'mfg', tab: 'cost', how: 'Read the stacked cost chart at any quantity (indicative — get real quotes).', keywords: 'price cost quantity' },
  { id: 'mold', title: 'check a part can be molded', role: 'manufacturing', lab: 'molding', tab: 'draft', how: 'Colour faces by draft angle and wall thickness; read clamp force and cycle time.', keywords: 'injection molding draft undercut' },
  { id: 'fab', title: 'download a fabrication package', role: 'manufacturing', lab: 'mfg', tab: 'package', how: 'One zip with Gerbers, bill of materials, component placement list (CPL), molding report, cost reports and order guides.', keywords: 'zip package order' },
  { id: 'export', title: 'export to KiCad or Onshape', role: 'manufacturing', lab: 'toolchain', tab: 'kicad', how: 'Download the KiCad project, or STEP (Standard for the Exchange of Product model data) and OpenSCAD files from the CAD tab to import into Onshape.', keywords: 'kicad onshape step export' },

  // Test & reliability
  { id: 'reliability-test', title: 'plan a reliability test', role: 'test', lab: 'reliability', tab: 'dvt', how: 'Generate the design verification test plan with legs, sample sizes and schedule.', keywords: 'dvt test plan' },
  { id: 'weibull', title: 'fit failure data', role: 'test', lab: 'reliability', tab: 'weibull', how: 'Paste failure times with suspensions and fit a Weibull with confidence bounds.', keywords: 'weibull b10 life data' },
  { id: 'alt', title: 'accelerate a life test', role: 'test', lab: 'reliability', tab: 'alt', how: 'Pick Arrhenius, Peck or Coffin–Manson and get the acceleration factor and test length.', keywords: 'arrhenius chamber accelerated' },
  { id: 'samples', title: 'decide how many units to test', role: 'test', lab: 'reliability', tab: 'sample', how: 'Success-run and binomial plans for a reliability and confidence target.', keywords: 'sample size confidence' },
  { id: 'compliance', title: 'plan certification', role: 'test', lab: 'build', tab: 'compliance', how: 'List the compliance families to plan for — always confirm with the standard and a test lab.', keywords: 'fcc ce certification compliance' },

  // Systems & program
  { id: 'requirements', title: 'see which requirements fail', role: 'systems', lab: 'systems', tab: 'requirements', how: 'Every requirement with its live value, margin and verifying lab.', keywords: 'requirements verification status' },
  { id: 'schedule', title: 'estimate schedule risk', role: 'systems', lab: 'program', tab: 'risk', how: 'Monte Carlo the whole plan and read the 50 % and 80 % finish dates.', keywords: 'schedule monte carlo p80' },
  { id: 'gantt', title: 'build a project schedule', role: 'systems', lab: 'program', tab: 'schedule', how: 'Edit three-point estimates and watch the critical path on the Gantt chart.', keywords: 'gantt critical path cpm' },
  { id: 'business', title: 'check the business case', role: 'systems', lab: 'program', tab: 'economics', how: 'Read gross margin, break-even volume and net present value at your price.', keywords: 'margin npv price breakeven' },
  { id: 'trade', title: 'compare design options', role: 'systems', lab: 'systems', tab: 'trades', how: 'Score options against weighted criteria and see how sensitive the winner is.', keywords: 'trade study decision matrix' },
  { id: 'risks', title: 'rank risks', role: 'systems', lab: 'systems', tab: 'risks', how: 'Fill in the failure mode and effects analysis and sort by risk priority number.', keywords: 'fmea rpn risk' },
  { id: 'new', title: 'start my own product', role: 'systems', lab: 'new', how: 'Describe it in plain words and get a starter project with requirements bound to metrics.', keywords: 'new wizard describe idea' },

  // Quantum
  { id: 'bell', title: 'entangle two qubits', role: 'quantum', lab: 'quantum', tab: 'composer', how: 'Place a Hadamard then a controlled-NOT gate and step through the Bloch spheres; export Open Quantum Assembly Language (OpenQASM).', keywords: 'bell pair cnot qiskit' },
  { id: 'qkd', title: 'design a quantum key distribution link', role: 'quantum', lab: 'quantum', tab: 'qkd', how: 'Set fiber loss and detector specs and read secret key rate against distance.', keywords: 'qkd bb84 fiber key rate' },
  { id: 'qrng', title: 'test a random number generator', role: 'quantum', lab: 'quantum', tab: 'qrng', how: 'Model diode noise, extract entropy and run the National Institute of Standards and Technology (NIST) statistical tests.', keywords: 'random nist entropy' },
  { id: 'teleport', title: 'see quantum teleportation', role: 'quantum', lab: 'quantum', tab: 'protocols', how: 'Play the animated teleportation story with qubits and classical bits in flight.', keywords: 'teleportation protocol' },
];
