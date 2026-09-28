# Reference repositories

Mature open-source projects that do what each Protoforge lab teaches, at professional scale. Study their code, cross-check Protoforge's numbers against them, and graduate your design to them. Contributors: when you extend a lab, read the matching project first and cite it.

The same list appears on the **Guide & references** page (`#/learn`), generated from `web/src/pages/learn/repos.ts` (regenerate with `node --experimental-strip-types web/src/pages/learn/gen-docs.mjs`). GitHub `owner/name` is given where the project lives on GitHub; otherwise the official site.

**Abbreviations:** API — application programming interface; BSIM — Berkeley short-channel insulated-gate field-effect transistor model; CAD — computer-aided design; DQN — deep Q-network; DRC — design rule check; DSN — Specctra design file; DSP — digital signal processing; EDA — electronic design automation; FIR — finite impulse response (filter); FPGA — field-programmable gate array; GDSII — Graphic Design System II; HDL — hardware description language; LQR — linear–quadratic regulator; LU — lower–upper (decomposition); MDP — Markov decision process; MIT — Massachusetts Institute of Technology; MNA — modified nodal analysis; PCB — printed circuit board; PDK — process design kit; PID — proportional–integral–derivative (controller); PPO — proximal policy optimisation; RL — reinforcement learning; ROS — Robot Operating System; RTL — register-transfer level; SAC — soft actor–critic; SKY130 — SkyWater 130 nm process; SPICE — Simulation Program with Integrated Circuit Emphasis; STEP — Standard for the Exchange of Product model data (ISO 10303).

## Circuit simulation

| Project | Where | What to learn from it |
|---|---|---|
| **ngspice** | <https://ngspice.sourceforge.io> | The reference open SPICE: device models (BSIM), sparse LU, timestep control — every Circuit Lab deck runs in it unchanged. |
| **Xyce** | [`Xyce/Xyce`](https://github.com/Xyce/Xyce) | Sandia’s parallel SPICE; a clean C++ study of MNA assembly, Newton solvers and large-circuit scaling. |
| **Qucs-S** | [`ra3xdh/qucs_s`](https://github.com/ra3xdh/qucs_s) | Schematic front end driving ngspice/Xyce — how simulator back ends are abstracted. |

## PCB design

| Project | Where | What to learn from it |
|---|---|---|
| **KiCad** | <https://www.kicad.org> | The open EDA suite: schematic, layout, DRC, Gerber/drill export and built-in ngspice. Where Iris’s board should go next. |
| **FreeRouting** | [`freerouting/freerouting`](https://github.com/freerouting/freerouting) | Push-and-shove autorouter for KiCad via Specctra DSN — compare with the Lee maze router in the PCB Studio. |
| **gerbv** | [`gerbv/gerbv`](https://github.com/gerbv/gerbv) | Independent Gerber viewer; check exported layers before ordering. |
| **tracespace** | [`tracespace/tracespace`](https://github.com/tracespace/tracespace) | Gerber/drill parsing and rendering in JavaScript — useful for web viewers. |
| **OpenPnP** | [`openpnp/openpnp`](https://github.com/openpnp/openpnp) | Open pick-and-place machine control with feeders and vision; the real-world version of the Robotics Lab’s PnP task. |

## Digital design & chips

| Project | Where | What to learn from it |
|---|---|---|
| **Yosys** | [`YosysHQ/yosys`](https://github.com/YosysHQ/yosys) | RTL synthesis; `synth; abc -liberty` gives real cell counts for the Chip Lab’s generated Verilog. |
| **Verilator** | [`verilator/verilator`](https://github.com/verilator/verilator) | Fast cycle-based Verilog simulator and linter (`--lint-only -Wall`). |
| **Icarus Verilog** | [`steveicarus/iverilog`](https://github.com/steveicarus/iverilog) | Event-driven Verilog simulator, closest in spirit to the Chip Lab’s gate simulator. |
| **cocotb** | [`cocotb/cocotb`](https://github.com/cocotb/cocotb) | Python testbenches for HDL — feed real audio through the IRIS-DSP FIR and compare with SciPy. |
| **OpenROAD** | [`The-OpenROAD-Project/OpenROAD`](https://github.com/The-OpenROAD-Project/OpenROAD) | Placement, clock-tree synthesis, routing and static timing (OpenSTA) from netlist to layout. |
| **OpenLane 2** | [`efabless/openlane2`](https://github.com/efabless/openlane2) | Scripted RTL-to-GDSII flow around Yosys and OpenROAD for the SkyWater SKY130 process (continued by the community as LibreLane). |
| **SkyWater SKY130 PDK** | [`google/skywater-pdk`](https://github.com/google/skywater-pdk) | An open 130 nm process design kit — real standard cells, rules and models. |
| **Tiny Tapeout** | [`TinyTapeout/tt-support-tools`](https://github.com/TinyTapeout/tt-support-tools) · <https://tinytapeout.com> | Shared-shuttle tape-outs of small designs; the Chip Lab exports a Tiny Tapeout-style project. |
| **nextpnr** | [`YosysHQ/nextpnr`](https://github.com/YosysHQ/nextpnr) | Open FPGA place-and-route — try the RTL on real hardware before tape-out. |
| **KLayout** | [`KLayout/klayout`](https://github.com/KLayout/klayout) | Layout viewer/editor for GDSII with scriptable design-rule checks. |

## Signal processing

| Project | Where | What to learn from it |
|---|---|---|
| **SciPy (scipy.signal)** | [`scipy/scipy`](https://github.com/scipy/scipy) | `firwin`, `remez`, `butter(output='sos')`, `welch`, `resample_poly` — cross-check every DSP Lab number. |
| **GNU Radio** | [`gnuradio/gnuradio`](https://github.com/gnuradio/gnuradio) | Real-time streaming flowgraphs, polyphase decimators and filter-design tools for software radio. |
| **Faust** | [`grame-cncm/faust`](https://github.com/grame-cncm/faust) | Functional audio DSP language compiling to C++ and WebAssembly — prototype the beamformer at audio rate. |
| **Pyroomacoustics** | [`LCAV/pyroomacoustics`](https://github.com/LCAV/pyroomacoustics) | Room acoustics simulation and microphone-array beamforming in Python. |

## Control & estimation

| Project | Where | What to learn from it |
|---|---|---|
| **python-control** | [`python-control/python-control`](https://github.com/python-control/python-control) | Transfer functions, margins, root locus and LQR with MATLAB-style names. |
| **Drake** | [`RobotLocomotion/drake`](https://github.com/RobotLocomotion/drake) | Rigid-body dynamics, LQR and trajectory optimisation from MIT and Toyota Research Institute. |
| **x-io Fusion** | [`xioTechnologies/Fusion`](https://github.com/xioTechnologies/Fusion) | Production attitude-and-heading reference system based on the Madgwick filter, for embedded IMUs. |
| **PX4 Autopilot** | [`PX4/PX4-Autopilot`](https://github.com/PX4/PX4-Autopilot) | Full extended-Kalman-filter state estimation and cascaded PID flight control. |
| **ArduPilot** | [`ArduPilot/ardupilot`](https://github.com/ArduPilot/ardupilot) | Another mature autopilot — compare its estimator and controller structure with PX4. |

## Robotics & reinforcement learning

| Project | Where | What to learn from it |
|---|---|---|
| **MuJoCo** | [`google-deepmind/mujoco`](https://github.com/google-deepmind/mujoco) | Fast contact-rich physics for controllers and RL beyond the cart–pole. |
| **Gymnasium** | [`Farama-Foundation/Gymnasium`](https://github.com/Farama-Foundation/Gymnasium) | The standard RL environment API — wrap the Iris power-management MDP as an Env. |
| **Stable-Baselines3** | [`DLR-RM/stable-baselines3`](https://github.com/DLR-RM/stable-baselines3) | Reliable DQN, PPO and SAC implementations to compare against the RL Lab’s tabular agents. |
| **MoveIt 2** | [`moveit/moveit2`](https://github.com/moveit/moveit2) | ROS 2 motion planning, inverse kinematics and time parameterisation. |
| **Robotics Toolbox for Python** | [`petercorke/robotics-toolbox-python`](https://github.com/petercorke/robotics-toolbox-python) | Kinematics, Jacobians and trajectories with textbook-style code (Corke). |

## Optics

| Project | Where | What to learn from it |
|---|---|---|
| **ray-optics** | [`mjhoptics/ray-optics`](https://github.com/mjhoptics/ray-optics) | Python geometric optics: paraxial and real ray tracing, lens layouts and aberrations. |

## CAD & geometry

| Project | Where | What to learn from it |
|---|---|---|
| **OpenSCAD** | [`openscad/openscad`](https://github.com/openscad/openscad) | Code-first constructive solid geometry — the simplest route to parametric parts. |
| **CadQuery** | [`CadQuery/cadquery`](https://github.com/CadQuery/cadquery) | Python CAD on the Open CASCADE B-rep kernel; exports STEP for tooling quotes. |
| **build123d** | [`gumyr/build123d`](https://github.com/gumyr/build123d) | Modern Python B-rep CAD (also Open CASCADE) with a clean builder API. |
| **FreeCAD** | [`FreeCAD/FreeCAD`](https://github.com/FreeCAD/FreeCAD) | Full parametric CAD with draft analysis, FEM workbench and STEP export. |
| **Elmer FEM** | [`ElmerCSC/elmerfem`](https://github.com/ElmerCSC/elmerfem) | Multiphysics finite-element solver — thermal and structural analysis beyond lumped models. |

## 3D printing

| Project | Where | What to learn from it |
|---|---|---|
| **PrusaSlicer** | [`prusa3d/PrusaSlicer`](https://github.com/prusa3d/PrusaSlicer) | Mature open slicer (Slic3r lineage): perimeter generation, infill, flow maths and G-code. |
| **OrcaSlicer** | [`SoftFever/OrcaSlicer`](https://github.com/SoftFever/OrcaSlicer) | Community slicer with calibration tools and Bambu Lab printer support. |
| **Bambu Studio** | [`bambulab/BambuStudio`](https://github.com/bambulab/BambuStudio) | Bambu Lab’s open-source slicer; open the CAD Studio’s 3MF plate here. |
| **CuraEngine** | [`Ultimaker/CuraEngine`](https://github.com/Ultimaker/CuraEngine) | Cura’s C++ slicing core — a readable comparison for the Print Lab’s slicer. |
| **Klipper** | [`Klipper3d/klipper`](https://github.com/Klipper3d/klipper) | Printer firmware with host-side kinematics and the trapezoid motion planner the Print Lab’s time estimate follows. |
| **Marlin** | [`MarlinFirmware/Marlin`](https://github.com/MarlinFirmware/Marlin) | The most widely used printer firmware; its G-code index is the reference for every command. |

## Open smart-glasses hardware

| Project | Where | What to learn from it |
|---|---|---|
| **Brilliant Labs (Frame, Monocle)** | <https://github.com/brilliantlabsAR> | Open-source firmware and hardware files for real heads-up-display glasses — compare with Iris’s architecture. |
| **OpenGlass** | [`BasedHardware/OpenGlass`](https://github.com/BasedHardware/OpenGlass) | Turn ordinary glasses into AI glasses with a small camera module; a minimal, hackable reference. |

## Protoforge’s own stack

| Project | Where | What to learn from it |
|---|---|---|
| **three.js** | [`mrdoob/three.js`](https://github.com/mrdoob/three.js) | WebGL 3D engine behind every viewer. |
| **React Three Fiber** | [`pmndrs/react-three-fiber`](https://github.com/pmndrs/react-three-fiber) | React renderer for three.js used by the CAD, PCB and Print viewers. |
| **axum** | [`tokio-rs/axum`](https://github.com/tokio-rs/axum) | The Rust web framework behind forge-server. |
| **wasm-bindgen** | — | Generates the JavaScript glue between the browser and the Rust engine. |
