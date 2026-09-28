// The real tools Protoforge hands files to, grouped by discipline.
// Facts only: names, what they are for, public URLs and the steps to open our
// files. No endorsement, no prices.

import type { HowStep } from './HowTo';

export type Discipline = 'Circuits' | 'PCB' | 'CAD / Mechanical' | 'Printing' | 'Chips / HDL' | 'Optics' | 'Quantum' | 'Firmware';

/** A downloadable slice of the toolchain bundle: files whose path matches. */
export interface DownloadSpec { label: string; filename: string; match: (path: string) => boolean; zip: boolean }

export interface Tool {
  id: string;
  name: string;
  discipline: Discipline;
  url: string;
  kind: 'web' | 'desktop' | 'web + desktop' | 'in Protoforge';
  purpose: string;
  /** Files from Protoforge it opens. */
  formats: string[];
  steps: string[];
  /** In-app page that builds its input (e.g. '#/quantum'). */
  lab?: { href: string; label: string };
  downloads?: DownloadSpec[];
  /** Tab of the Toolchain page with a live preview / generated links. */
  tab?: 'kicad' | 'cad' | 'circuits' | 'more';
  /** The animated how-to strip (short captions). */
  howto: HowStep[];
}

const starts = (p: string) => (path: string) => path.startsWith(p);

export const DISCIPLINES: Discipline[] = ['Circuits', 'PCB', 'CAD / Mechanical', 'Printing', 'Chips / HDL', 'Optics', 'Quantum', 'Firmware'];

export const TOOLS: Tool[] = [
  {
    id: 'falstad', name: 'Falstad CircuitJS', discipline: 'Circuits', url: 'https://www.falstad.com/circuit/', kind: 'web',
    purpose: 'Animated, interactive circuit simulator: watch current flow and tweak parts while it runs.',
    formats: ['CircuitJS text (in the link)'],
    steps: ['Click “Open in Falstad” on a circuit — the whole circuit is packed into the URL (lz-string, ?ctz=).', 'Or: File → Import From Text and paste the .circuitjs.txt file.', 'Right-click a part → Edit to change values; the simulation keeps running.'],
    downloads: [{ label: 'CircuitJS texts', filename: 'circuitjs.zip', match: (p) => p.endsWith('.circuitjs.txt') || p === 'circuits/README.md', zip: true }],
    tab: 'circuits',
    howto: [{ a: 'click', t: 'Click “Open in Falstad”' }, { a: 'link', t: 'Circuit rides in the ?ctz= link' }, { a: 'run', t: 'It simulates at once' }, { a: 'edit', t: 'Right-click a part → Edit' }],
  },
  {
    id: 'ngspice', name: 'ngspice', discipline: 'Circuits', url: 'https://ngspice.sourceforge.io/', kind: 'desktop',
    purpose: 'Open-source SPICE engine: batch transient/AC/DC runs of the exact decks the Circuit Lab simulates.',
    formats: ['.cir SPICE decks'],
    steps: ['Install ngspice (packages exist for Linux, macOS and Windows).', 'Run ngspice -b rail_load_step.cir (batch mode) or open it interactively and type run, then plot v(vload).'],
    downloads: [{ label: 'SPICE decks', filename: 'spice-decks.zip', match: (p) => p.endsWith('.cir'), zip: true }],
    lab: { href: '#/circuit', label: 'Circuit Lab' },
    howto: [{ a: 'download', t: 'Download the .cir decks' }, { a: 'paste', t: 'ngspice -b deck.cir' }, { a: 'run', t: 'Transient / AC runs' }, { a: 'view', t: 'Compare with the Circuit Lab' }],
  },
  {
    id: 'kicad', name: 'KiCad', discipline: 'PCB', url: 'https://www.kicad.org/', kind: 'desktop',
    purpose: 'The open-source PCB suite: edit the routed Iris board, re-run DRC, refill zones, plot Gerbers.',
    formats: ['.kicad_pcb', '.kicad_pro', '.net', 'Protoforge.pretty'],
    steps: ['Download the KiCad files and unzip them into one folder.', 'Open iris.kicad_pro (KiCad 7 or newer).', 'Press B to refill the ground zones, then Inspect → Design Rules Checker.', 'Swap Protoforge’s typical footprints for the manufacturers’ land patterns before ordering.'],
    downloads: [{ label: 'KiCad project', filename: 'iris-kicad.zip', match: starts('kicad/'), zip: true }],
    tab: 'kicad',
    howto: [{ a: 'download', t: 'Download KiCad project' }, { a: 'unzip', t: 'Unzip into one folder' }, { a: 'open', t: 'Open iris.kicad_pro' }, { a: 'click', t: 'B to fill zones, run DRC' }],
  },
  {
    id: 'kicanvas', name: 'KiCanvas', discipline: 'PCB', url: 'https://kicanvas.org/', kind: 'web',
    purpose: 'Views KiCad boards and schematics in the browser — perfect for a GitHub Pages project page.',
    formats: ['.kicad_pcb (drag-and-drop or GitHub URL)'],
    steps: ['Commit iris.kicad_pcb to a public GitHub repository.', 'Open https://kicanvas.org/?github=<URL of the file on github.com> (the link builder on the KiCad tab does this).', 'Or drag the file onto kicanvas.org.'],
    tab: 'kicad',
    howto: [{ a: 'download', t: 'Download iris.kicad_pcb' }, { a: 'open', t: 'Open kicanvas.org' }, { a: 'drop', t: 'Drag the file onto the page' }, { a: 'view', t: 'Pan, zoom, inspect nets' }],
  },
  {
    id: 'tracespace', name: 'tracespace viewer', discipline: 'PCB', url: 'https://tracespace.io/view/', kind: 'web',
    purpose: 'Renders Gerber and drill files in the browser — check what the fab will see.',
    formats: ['Gerber RS-274X', 'Excellon drill'],
    steps: ['Download the Gerber zip.', 'Drop it onto the viewer page; toggle layers and check the board from both sides.', 'Fabs such as JLCPCB render the uploaded zip on their online quote page too (their own viewer) — a second opinion before you pay; see the Manufacturing Hub’s vendor list.'],
    downloads: [{ label: 'Gerbers + drill', filename: 'iris-gerbers.zip', match: (p) => p.startsWith('pcb/') && !p.startsWith('pcb/assembly'), zip: true }],
    lab: { href: '#/mfg', label: 'Manufacturing Hub' },
    howto: [{ a: 'download', t: 'Download Gerbers + drill' }, { a: 'open', t: 'Open the viewer' }, { a: 'drop', t: 'Drop the zip' }, { a: 'view', t: 'Check both sides' }],
  },
  {
    id: 'onshape', name: 'Onshape', discipline: 'CAD / Mechanical', url: 'https://www.onshape.com/', kind: 'web',
    purpose: 'Browser CAD with real versions and branches: import the frame, then rebuild it as native parametric features.',
    formats: ['STEP (faceted)', 'STL'],
    steps: ['In a document, click + (Insert new tab) → Import, and pick iris-frame.step (units: millimetre).', 'The parts arrive as faceted bodies — use them as reference geometry and re-model with Sketch/Extrude for production.', 'Create a Version (Onshape’s version history) for each baseline, just like the Versions page here.'],
    downloads: [{ label: 'STEP assembly', filename: 'iris-frame.step', match: (p) => p === 'cad/iris-frame.step', zip: false }],
    tab: 'cad',
    howto: [{ a: 'download', t: 'Download iris-frame.step' }, { a: 'open', t: 'Onshape document' }, { a: 'import', t: '+ → Import (millimetre)' }, { a: 'commit', t: 'Create a Version' }],
  },
  {
    id: 'freecad', name: 'FreeCAD', discipline: 'CAD / Mechanical', url: 'https://www.freecad.org/', kind: 'desktop',
    purpose: 'Open-source parametric CAD on OpenCASCADE: open the STEP, convert to solids, re-model, export for a molder.',
    formats: ['STEP (faceted)', 'STL'],
    steps: ['File → Open → iris-frame.step.', 'Part workbench → select a body → Part → Convert to solid if you need boolean operations.'],
    downloads: [{ label: 'STEP (print orientation)', filename: 'iris-frame-print.step', match: (p) => p === 'cad/iris-frame-print.step', zip: false }],
    tab: 'cad',
    howto: [{ a: 'download', t: 'Download the STEP' }, { a: 'open', t: 'File → Open' }, { a: 'import', t: 'Part → Convert to solid' }, { a: 'export', t: 'Re-model, export for the shop' }],
  },
  {
    id: 'openscad', name: 'OpenSCAD Playground', discipline: 'CAD / Mechanical', url: 'https://ochafik.com/openscad2', kind: 'web + desktop',
    purpose: 'Code-CAD in the browser (OpenSCAD compiled to WebAssembly): the frame as an editable parametric program.',
    formats: ['.scad'],
    steps: ['Open the playground and replace the editor contents with iris-frame.scad (or use desktop OpenSCAD, https://openscad.org).', 'Change a parameter at the top (e.g. bridge = 20;) and press F5 to preview, F6 to render.', 'Export STL or 3MF for your slicer.'],
    downloads: [{ label: 'OpenSCAD program', filename: 'iris-frame.scad', match: (p) => p.endsWith('.scad'), zip: false }],
    tab: 'cad',
    howto: [{ a: 'download', t: 'Download iris-frame.scad' }, { a: 'paste', t: 'Paste into the Playground' }, { a: 'edit', t: 'Change bridge = 20;' }, { a: 'run', t: 'Render (F6)' }, { a: 'export', t: 'Export STL / 3MF' }],
  },
  {
    id: '3dviewer', name: 'Online 3D Viewer', discipline: 'Printing', url: 'https://3dviewer.net/', kind: 'web',
    purpose: 'Open-source browser viewer for STL, OBJ, 3MF and STEP — measure and inspect parts before printing.',
    formats: ['STL', 'STEP'],
    steps: ['Drag the STL files (or iris-frame.step) onto the page — Online 3D Viewer (open source, by Viktor Kovacs) reads both in the browser.', 'Use the measure tool to check wall thickness and hinge clearances.'],
    downloads: [{ label: 'STL parts', filename: 'iris-stl.zip', match: starts('cad/stl/'), zip: true }],
    howto: [{ a: 'download', t: 'Download STL or STEP' }, { a: 'open', t: 'Open 3dviewer.net' }, { a: 'drop', t: 'Drag the files in' }, { a: 'view', t: 'Measure and inspect' }],
  },
  {
    id: 'bambu', name: 'Bambu Studio', discipline: 'Printing', url: 'https://bambulab.com/en/download/studio', kind: 'desktop',
    purpose: 'Slicer for Bambu Lab printers (PrusaSlicer, OrcaSlicer and Cura work the same way).',
    formats: ['STL', '3MF'],
    steps: ['File → Import → select the STL parts (already in print orientation).', 'Choose the filament (the project uses PETG) and a 0.2 mm profile, then Slice.', 'Compare the time and filament with the Print Lab’s estimate.'],
    lab: { href: '#/print', label: 'Print Lab' },
    downloads: [{ label: 'STL parts', filename: 'iris-stl.zip', match: starts('cad/stl/'), zip: true }],
    howto: [{ a: 'download', t: 'Download STL parts' }, { a: 'import', t: 'File → Import' }, { a: 'click', t: 'PETG, 0.2 mm → Slice' }, { a: 'view', t: 'Compare with Print Lab' }],
  },
  {
    id: 'ncviewer', name: 'NC Viewer', discipline: 'Printing', url: 'https://ncviewer.com/', kind: 'web',
    purpose: 'Web G-code viewer: step through the toolpath of Protoforge’s generic Marlin G-code.',
    formats: ['G-code'],
    steps: ['Open the page, use its file open button and choose a .gcode file.', 'Scrub the layer slider; compare with the Print Lab’s layer view.', 'Re-slice with your printer’s own slicer for real prints.'],
    downloads: [{ label: 'G-code', filename: 'iris-gcode.zip', match: starts('print/'), zip: true }],
    howto: [{ a: 'download', t: 'Download G-code' }, { a: 'open', t: 'Open NC Viewer' }, { a: 'drop', t: 'Open the .gcode file' }, { a: 'view', t: 'Scrub the toolpath' }],
  },
  {
    id: 'edaplayground', name: 'EDA Playground', discipline: 'Chips / HDL', url: 'https://www.edaplayground.com/', kind: 'web',
    purpose: 'Run Verilog testbenches in the browser with Icarus Verilog and other simulators (free account).',
    formats: ['Verilog (.v) + testbench'],
    steps: ['Paste tb_iris_fir.v into the left (testbench) pane and iris_fir.v into the right (design) pane.', 'Tools & Simulators → Icarus Verilog, tick “Open EPWave after run”, press Run.'],
    downloads: [{ label: 'Verilog + testbenches', filename: 'iris-verilog.zip', match: starts('chip/'), zip: true }],
    lab: { href: '#/chip', label: 'Chip Lab' },
    howto: [{ a: 'download', t: 'Download Verilog' }, { a: 'paste', t: 'Testbench left, design right' }, { a: 'click', t: 'Icarus Verilog + EPWave' }, { a: 'run', t: 'Run, read the waveform' }],
  },
  {
    id: 'digitaljs', name: 'DigitalJS', discipline: 'Chips / HDL', url: 'https://digitaljs.tilk.eu/', kind: 'web',
    purpose: 'Synthesises Verilog with Yosys in the browser and animates the resulting gate diagram.',
    formats: ['Verilog (.v)'],
    steps: ['Paste iris_gesture_fsm.v, press Synthesize and simulate.', 'Toggle inputs and watch the state register change.'],
    lab: { href: '#/chip', label: 'Chip Lab' },
    howto: [{ a: 'open', t: 'Open DigitalJS' }, { a: 'paste', t: 'Paste the FSM module' }, { a: 'click', t: 'Synthesize and simulate' }, { a: 'run', t: 'Toggle inputs' }],
  },
  {
    id: '8bitworkshop', name: '8bitworkshop', discipline: 'Chips / HDL', url: 'https://8bitworkshop.com/', kind: 'web',
    purpose: 'Browser IDE with a Verilog platform: edit and simulate small designs with a live waveform view.',
    formats: ['Verilog (.v)'],
    steps: ['Choose the Verilog platform, create a file and paste a module.', 'Watch the waveform/scope view update as you edit.'],
    howto: [{ a: 'open', t: 'Open the IDE' }, { a: 'click', t: 'Pick the Verilog platform' }, { a: 'paste', t: 'Paste a module' }, { a: 'run', t: 'Watch the scope' }],
  },
  {
    id: 'rayoptics', name: 'Ray Optics Simulation', discipline: 'Optics', url: 'https://phydemo.app/ray-optics/', kind: 'web',
    purpose: '2-D ray tracer in the browser: lenses, mirrors, beam splitters and detectors you can drag.',
    formats: ['scene JSON (format version 5)'],
    steps: ['Download iris-hud.ray-optics.json.', 'In the simulator use the Open (file) button and choose it.', 'Drag the combiner towards the lens: past the clearance limit its upper end hits the lens aperture — exactly what requirement R-025 now guards.'],
    downloads: [{ label: 'Ray Optics scene', filename: 'iris-hud.ray-optics.json', match: (p) => p.endsWith('.ray-optics.json'), zip: false }],
    lab: { href: '#/optics', label: 'Optics Lab' },
    tab: 'more',
    howto: [{ a: 'download', t: 'Download the scene JSON' }, { a: 'open', t: 'Open Ray Optics' }, { a: 'import', t: 'Open → the .json file' }, { a: 'edit', t: 'Drag the combiner' }],
  },
  {
    id: 'ibmcomposer', name: 'IBM Quantum Composer', discipline: 'Quantum', url: 'https://quantum.ibm.com/composer', kind: 'web',
    purpose: 'Drag-and-drop quantum circuits with an OpenQASM editor; run on simulators or real hardware (IBM account).',
    formats: ['OpenQASM 2.0'],
    steps: ['Build or open a circuit in the Quantum Lab and copy its OpenQASM.', 'In the Composer open the OpenQASM editor, paste, and inspect the probabilities / Q-sphere.'],
    lab: { href: '#/quantum', label: 'Quantum Lab' },
    howto: [{ a: 'click', t: 'Quantum Lab → OpenQASM' }, { a: 'open', t: 'Open the Composer' }, { a: 'paste', t: 'Paste into the code editor' }, { a: 'run', t: 'Simulate or run' }],
  },
  {
    id: 'quirk', name: 'Quirk', discipline: 'Quantum', url: 'https://algassert.com/quirk', kind: 'web',
    purpose: 'Instant drag-and-drop quantum circuit simulator with live state displays; circuits live in the URL.',
    formats: ['Quirk circuit URL'],
    steps: ['The Quantum Lab generates Quirk links for its circuits — open one and drag gates to explore.'],
    lab: { href: '#/quantum', label: 'Quantum Lab' },
    howto: [{ a: 'click', t: 'Quantum Lab → Quirk link' }, { a: 'link', t: 'Circuit rides in the URL' }, { a: 'edit', t: 'Drag gates' }, { a: 'view', t: 'Read the state displays' }],
  },
  {
    id: 'wokwi', name: 'Wokwi', discipline: 'Firmware', url: 'https://wokwi.com/', kind: 'web',
    purpose: 'Simulates Arduino, ESP32 and Raspberry Pi Pico boards with sensors and displays in the browser.',
    formats: ['Arduino / ESP32 sketches', 'diagram.json'],
    steps: ['The Prototype & Firmware page builds the breadboard version of Iris and its firmware.', 'Start a new ESP32 project in Wokwi and paste the sketch; add the parts from the wiring table.'],
    lab: { href: '#/proto', label: 'Prototype & Firmware' },
    howto: [{ a: 'click', t: 'Prototype page → Wokwi files' }, { a: 'open', t: 'New ESP32 project' }, { a: 'paste', t: 'Paste sketch + diagram.json' }, { a: 'run', t: 'Start the simulation' }],
  },
];

/** Where files go after the tools: the vendors (categories only). */
export const VENDOR_STAGES = [
  { id: 'fab', label: 'PCB fab + assembly', tools: ['kicad', 'tracespace'] },
  { id: 'molder', label: 'Molder / CNC shop', tools: ['onshape', 'freecad'] },
  { id: 'printer', label: 'Your printer / print service', tools: ['bambu', '3dviewer', 'ncviewer', 'openscad'] },
  { id: 'shuttle', label: 'Tiny Tapeout shuttle / FPGA', tools: ['edaplayground', 'digitaljs', '8bitworkshop'] },
  { id: 'bench', label: 'Bench prototype', tools: ['falstad', 'ngspice', 'wokwi', 'rayoptics', 'kicanvas'] },
  { id: 'cloud', label: 'Quantum hardware (cloud)', tools: ['ibmcomposer', 'quirk'] },
];
