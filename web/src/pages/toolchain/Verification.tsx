// Honest table: which export was checked with which independent parser, what
// that proves, and what was not checked. Numbers are from the validation run
// described in docs/toolchain.md (Iris v0.4 defaults).
import { Card, Chip, Explainer } from '../../components/ui';

interface Row { format: string; tool: string; result: string; proves: string; notChecked: string; ok: boolean }

export const ROWS: Row[] = [
  {
    format: '.kicad_pcb (KiCad 7 board)',
    tool: 'kiutils 1.4.8 (Python KiCad parser) · Protoforge’s TypeScript reader (this page) · Rust unit test',
    result: '28 footprints, 181 pads, 427 segments, 87 vias, 2 GND zones + 3 keep-outs, 31 nets; parse → write → parse round trip; every pad at the engine’s position (±0.1 µm) on the same net',
    proves: 'The file is well-formed KiCad S-expression syntax with the expected objects, coordinates and nets.',
    notChecked: 'Opening in KiCad itself (KiCad cannot be installed in the build sandbox) — please report anything KiCad rejects.',
    ok: true,
  },
  {
    format: 'Protoforge.pretty (.kicad_mod) · .kicad_pro · .net',
    tool: 'kiutils 1.4.8 · JSON parser · Rust S-expression parser',
    result: '15 library footprints parsed; project JSON with 4 net classes; netlist with every net and pin',
    proves: 'Library and netlist files are syntactically valid and complete.',
    notChecked: 'KiCad’s own netlist importer.',
    ok: true,
  },
  {
    format: 'STEP AP214 faceted B-rep',
    tool: 'gmsh 4.15.2 (OpenCASCADE STEP reader)',
    result: '20 solids, 5 148 faces; total volume 24.741 cm³ = CAD Studio’s 24.741 cm³; every shell closed (χ checked per shell)',
    proves: 'An industrial STEP kernel reads the parts as closed solids with the right size and volume.',
    notChecked: 'The Onshape and FreeCAD import dialogs themselves (FreeCAD reads STEP with OpenCASCADE, the same kernel gmsh uses).',
    ok: true,
  },
  {
    format: 'OpenSCAD .scad',
    tool: 'OpenSCAD compiled to WebAssembly (openscad-wasm-prebuilt 1.2.0, the engine family the Playground uses)',
    result: 'Renders without warnings to a closed polyhedron (3 770 facets) in ≈ 17 s',
    proves: 'The program is valid OpenSCAD and produces the frame.',
    notChecked: 'That it equals CAD Studio’s mesh — it is a deliberately simplified re-statement.',
    ok: true,
  },
  {
    format: 'Falstad CircuitJS links (?ctz=)',
    tool: 'Reference lz-string 1.5.0 (JavaScript) · CircuitJS dump() field layouts from sharpie7/circuitjs1',
    result: 'All 10 links decode back to the exact circuit text; the Rust port matches 7 reference vectors bit for bit; connectivity re-extracted with CircuitJS’s post rules matches every SPICE netlist',
    proves: 'The URL payload is exactly what CircuitJS decompresses, and the drawing connects the same nodes as the netlist.',
    notChecked: 'Loading in CircuitJS itself (falstad.com is not reachable from the build sandbox).',
    ok: true,
  },
  {
    format: 'Ray Optics scene JSON',
    tool: 'Key check against the app’s source (ricktu288/ray-optics, serializableDefaults, DATA_VERSION 5)',
    result: 'All 13 objects use only keys the app accepts (it rejects unknown keys)',
    proves: 'The scene matches the documented format.',
    notChecked: 'Loading in the app itself.',
    ok: true,
  },
  {
    format: 'Gerber X2 + Excellon (PCB Studio)',
    tool: 'PCB Studio’s own Gerber re-parser (pcb::fab::parse_gerber)',
    result: 'Re-read and rendered on the PCB Studio Files tab',
    proves: 'Syntax and apertures are consistent.',
    notChecked: 'A third-party Gerber viewer in this sandbox.',
    ok: false,
  },
];

export const VERIFIED_COUNT = ROWS.filter((r) => r.ok).length;

export function Verification() {
  return (
    <div className="stack">
      <Card title="Which export is verified by which parser" subtitle="Run on the Iris v0.4 defaults while building this page; re-run the scripts in docs/toolchain.md after changing an exporter">
        <div className="table-wrap">
          <table className="table tc-verify">
            <thead><tr><th>Export</th><th>Checked with</th><th>Result</th><th>What that proves</th><th>Not checked</th></tr></thead>
            <tbody>
              {ROWS.map((r) => (
                <tr key={r.format}>
                  <td><b>{r.format}</b><div style={{ marginTop: 4 }}><Chip tone={r.ok ? 'pass' : undefined}>{r.ok ? '✓ outside parser' : '○ internal only'}</Chip></div></td>
                  <td className="small">{r.tool}</td>
                  <td className="small">{r.result}</td>
                  <td className="small">{r.proves}</td>
                  <td className="small muted">{r.notChecked}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>
      <Explainer title="Why an independent parser?" refs={[{ label: 'kiutils', href: 'https://github.com/mvnmgrx/kiutils' }, { label: 'gmsh', href: 'https://gmsh.info' }, { label: 'OpenSCAD', href: 'https://openscad.org/documentation.html' }, { label: 'lz-string', href: 'https://github.com/pieroxy/lz-string' }]}>
        A writer and a reader built by the same person share the same misunderstandings. Checking each export with software written by someone else — and comparing numbers that must agree, like the STEP volume against CAD Studio's mass properties — catches the mistakes a round trip through our own code would hide.
      </Explainer>
    </div>
  );
}
