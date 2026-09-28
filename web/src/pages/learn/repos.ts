// Open-source projects worth studying (and pointing Protoforge contributors at),
// grouped by discipline. `repo` is the GitHub owner/name when it lives there;
// `url` otherwise. Mirrored in docs/reference-repos.md.

export interface Repo {
  name: string;
  repo?: string;
  url?: string;
  learn: string;
  lab?: string;
}

export interface RepoGroup {
  discipline: string;
  items: Repo[];
}

export const REPOS: RepoGroup[] = [
  {
    discipline: 'Circuit simulation',
    items: [
      { name: 'ngspice', url: 'https://ngspice.sourceforge.io', lab: '/circuit', learn: 'The reference open SPICE: device models (BSIM), sparse LU, timestep control — every Circuit Lab deck runs in it unchanged.' },
      { name: 'Xyce', repo: 'Xyce/Xyce', lab: '/circuit', learn: 'Sandia’s parallel SPICE; a clean C++ study of MNA assembly, Newton solvers and large-circuit scaling.' },
      { name: 'Qucs-S', repo: 'ra3xdh/qucs_s', lab: '/circuit', learn: 'Schematic front end driving ngspice/Xyce — how simulator back ends are abstracted.' },
    ],
  },
  {
    discipline: 'PCB design',
    items: [
      { name: 'KiCad', url: 'https://www.kicad.org', lab: '/pcb', learn: 'The open EDA suite: schematic, layout, DRC, Gerber/drill export and built-in ngspice. Where Iris’s board should go next.' },
      { name: 'FreeRouting', repo: 'freerouting/freerouting', lab: '/pcb', learn: 'Push-and-shove autorouter for KiCad via Specctra DSN — compare with the Lee maze router in the PCB Studio.' },
      { name: 'gerbv', repo: 'gerbv/gerbv', lab: '/pcb', learn: 'Independent Gerber viewer; check exported layers before ordering.' },
      { name: 'tracespace', repo: 'tracespace/tracespace', lab: '/pcb', learn: 'Gerber/drill parsing and rendering in JavaScript — useful for web viewers.' },
      { name: 'OpenPnP', repo: 'openpnp/openpnp', lab: '/robotics', learn: 'Open pick-and-place machine control with feeders and vision; the real-world version of the Robotics Lab’s PnP task.' },
    ],
  },
  {
    discipline: 'Digital design & chips',
    items: [
      { name: 'Yosys', repo: 'YosysHQ/yosys', lab: '/chip', learn: 'RTL synthesis; `synth; abc -liberty` gives real cell counts for the Chip Lab’s generated Verilog.' },
      { name: 'Verilator', repo: 'verilator/verilator', lab: '/chip', learn: 'Fast cycle-based Verilog simulator and linter (`--lint-only -Wall`).' },
      { name: 'Icarus Verilog', repo: 'steveicarus/iverilog', lab: '/chip', learn: 'Event-driven Verilog simulator, closest in spirit to the Chip Lab’s gate simulator.' },
      { name: 'cocotb', repo: 'cocotb/cocotb', lab: '/chip', learn: 'Python testbenches for HDL — feed real audio through the IRIS-DSP FIR and compare with SciPy.' },
      { name: 'OpenROAD', repo: 'The-OpenROAD-Project/OpenROAD', lab: '/chip', learn: 'Placement, clock-tree synthesis, routing and static timing (OpenSTA) from netlist to layout.' },
      { name: 'OpenLane 2', repo: 'efabless/openlane2', lab: '/chip', learn: 'Scripted RTL-to-GDSII flow around Yosys and OpenROAD for the SkyWater SKY130 process (continued by the community as LibreLane).' },
      { name: 'SkyWater SKY130 PDK', repo: 'google/skywater-pdk', lab: '/chip', learn: 'An open 130 nm process design kit — real standard cells, rules and models.' },
      { name: 'Tiny Tapeout', repo: 'TinyTapeout/tt-support-tools', url: 'https://tinytapeout.com', lab: '/chip', learn: 'Shared-shuttle tape-outs of small designs; the Chip Lab exports a Tiny Tapeout-style project.' },
      { name: 'nextpnr', repo: 'YosysHQ/nextpnr', lab: '/chip', learn: 'Open FPGA place-and-route — try the RTL on real hardware before tape-out.' },
      { name: 'KLayout', repo: 'KLayout/klayout', lab: '/chip', learn: 'Layout viewer/editor for GDSII with scriptable design-rule checks.' },
    ],
  },
  {
    discipline: 'Signal processing',
    items: [
      { name: 'SciPy (scipy.signal)', repo: 'scipy/scipy', lab: '/dsp', learn: '`firwin`, `remez`, `butter(output=\'sos\')`, `welch`, `resample_poly` — cross-check every DSP Lab number.' },
      { name: 'GNU Radio', repo: 'gnuradio/gnuradio', lab: '/dsp', learn: 'Real-time streaming flowgraphs, polyphase decimators and filter-design tools for software radio.' },
      { name: 'Faust', repo: 'grame-cncm/faust', lab: '/dsp', learn: 'Functional audio DSP language compiling to C++ and WebAssembly — prototype the beamformer at audio rate.' },
      { name: 'Pyroomacoustics', repo: 'LCAV/pyroomacoustics', lab: '/dsp', learn: 'Room acoustics simulation and microphone-array beamforming in Python.' },
    ],
  },
  {
    discipline: 'Control & estimation',
    items: [
      { name: 'python-control', repo: 'python-control/python-control', lab: '/control', learn: 'Transfer functions, margins, root locus and LQR with MATLAB-style names.' },
      { name: 'Drake', repo: 'RobotLocomotion/drake', lab: '/control', learn: 'Rigid-body dynamics, LQR and trajectory optimisation from MIT and Toyota Research Institute.' },
      { name: 'x-io Fusion', repo: 'xioTechnologies/Fusion', lab: '/control', learn: 'Production attitude-and-heading reference system based on the Madgwick filter, for embedded IMUs.' },
      { name: 'PX4 Autopilot', repo: 'PX4/PX4-Autopilot', lab: '/control', learn: 'Full extended-Kalman-filter state estimation and cascaded PID flight control.' },
      { name: 'ArduPilot', repo: 'ArduPilot/ardupilot', lab: '/control', learn: 'Another mature autopilot — compare its estimator and controller structure with PX4.' },
    ],
  },
  {
    discipline: 'Robotics & reinforcement learning',
    items: [
      { name: 'MuJoCo', repo: 'google-deepmind/mujoco', lab: '/rl', learn: 'Fast contact-rich physics for controllers and RL beyond the cart–pole.' },
      { name: 'Gymnasium', repo: 'Farama-Foundation/Gymnasium', lab: '/rl', learn: 'The standard RL environment API — wrap the Iris power-management MDP as an Env.' },
      { name: 'Stable-Baselines3', repo: 'DLR-RM/stable-baselines3', lab: '/rl', learn: 'Reliable DQN, PPO and SAC implementations to compare against the RL Lab’s tabular agents.' },
      { name: 'MoveIt 2', repo: 'moveit/moveit2', lab: '/robotics', learn: 'ROS 2 motion planning, inverse kinematics and time parameterisation.' },
      { name: 'Robotics Toolbox for Python', repo: 'petercorke/robotics-toolbox-python', lab: '/robotics', learn: 'Kinematics, Jacobians and trajectories with textbook-style code (Corke).' },
    ],
  },
  {
    discipline: 'Optics',
    items: [
      { name: 'ray-optics', repo: 'mjhoptics/ray-optics', lab: '/optics', learn: 'Python geometric optics: paraxial and real ray tracing, lens layouts and aberrations.' },
    ],
  },
  {
    discipline: 'CAD & geometry',
    items: [
      { name: 'OpenSCAD', repo: 'openscad/openscad', lab: '/cad', learn: 'Code-first constructive solid geometry — the simplest route to parametric parts.' },
      { name: 'CadQuery', repo: 'CadQuery/cadquery', lab: '/cad', learn: 'Python CAD on the Open CASCADE B-rep kernel; exports STEP for tooling quotes.' },
      { name: 'build123d', repo: 'gumyr/build123d', lab: '/cad', learn: 'Modern Python B-rep CAD (also Open CASCADE) with a clean builder API.' },
      { name: 'FreeCAD', repo: 'FreeCAD/FreeCAD', lab: '/cad', learn: 'Full parametric CAD with draft analysis, FEM workbench and STEP export.' },
      { name: 'Elmer FEM', repo: 'ElmerCSC/elmerfem', learn: 'Multiphysics finite-element solver — thermal and structural analysis beyond lumped models.' },
    ],
  },
  {
    discipline: '3D printing',
    items: [
      { name: 'PrusaSlicer', repo: 'prusa3d/PrusaSlicer', lab: '/print', learn: 'Mature open slicer (Slic3r lineage): perimeter generation, infill, flow maths and G-code.' },
      { name: 'OrcaSlicer', repo: 'SoftFever/OrcaSlicer', lab: '/print', learn: 'Community slicer with calibration tools and Bambu Lab printer support.' },
      { name: 'Bambu Studio', repo: 'bambulab/BambuStudio', lab: '/print', learn: 'Bambu Lab’s open-source slicer; open the CAD Studio’s 3MF plate here.' },
      { name: 'CuraEngine', repo: 'Ultimaker/CuraEngine', lab: '/print', learn: 'Cura’s C++ slicing core — a readable comparison for the Print Lab’s slicer.' },
      { name: 'Klipper', repo: 'Klipper3d/klipper', lab: '/print', learn: 'Printer firmware with host-side kinematics and the trapezoid motion planner the Print Lab’s time estimate follows.' },
      { name: 'Marlin', repo: 'MarlinFirmware/Marlin', lab: '/print', learn: 'The most widely used printer firmware; its G-code index is the reference for every command.' },
    ],
  },
  {
    discipline: 'Open smart-glasses hardware',
    items: [
      { name: 'Brilliant Labs (Frame, Monocle)', url: 'https://github.com/brilliantlabsAR', lab: '/optics', learn: 'Open-source firmware and hardware files for real heads-up-display glasses — compare with Iris’s architecture.' },
      { name: 'OpenGlass', repo: 'BasedHardware/OpenGlass', learn: 'Turn ordinary glasses into AI glasses with a small camera module; a minimal, hackable reference.' },
    ],
  },
  {
    discipline: 'Protoforge’s own stack',
    items: [
      { name: 'three.js', repo: 'mrdoob/three.js', learn: 'WebGL 3D engine behind every viewer.' },
      { name: 'React Three Fiber', repo: 'pmndrs/react-three-fiber', learn: 'React renderer for three.js used by the CAD, PCB and Print viewers.' },
      { name: 'axum', repo: 'tokio-rs/axum', learn: 'The Rust web framework behind forge-server.' },
      { name: 'wasm-bindgen', learn: 'Generates the JavaScript glue between the browser and the Rust engine.' },
    ],
  },
];
