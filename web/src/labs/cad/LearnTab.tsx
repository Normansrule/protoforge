// Teaching views: ear-clipping walkthrough of the real Iris front, a draft
// cross-section, and the background theory with references.
import { useEffect, useMemo, useRef, useState } from 'react';
import { Card, Explainer, Formula, Slider } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import { prefersReducedMotion } from './viewer';
import type { TriOut } from './api';

export function LearnTab() {
  return (
    <div className="stack">
      <TriangulationCard />
      <div className="grid grid-2">
        <DraftCard />
        <Explainer
          title="B-rep vs mesh"
          refs={[
            { label: 'Open CASCADE Technology (FreeCAD, CadQuery, build123d kernel)', href: 'https://dev.opencascade.org/' },
            { label: 'Onshape (Parasolid kernel)', href: 'https://www.onshape.com/' },
          ]}
        >
          Professional CAD keeps a <b>boundary representation</b>: faces lying on exact surfaces (planes, cylinders, NURBS),
          bounded by edges and vertices with full topology. Fillets, shells and booleans are computed on those exact surfaces
          and only tessellated into triangles for display or export. This studio skips straight to the tessellation: a
          <b> mesh</b> of triangles, which is exactly what slicers, the molding analysis and WebGL consume. Meshes are simple and
          fast; B-reps are precise (a hole stays a perfect cylinder at any zoom) and editable by feature history. When a part
          outgrows this generator, rebuild it in Onshape, FreeCAD, build123d/CadQuery (Python) or OpenSCAD using the numbers
          from the Mass & export tab.
        </Explainer>
      </div>
      <div className="grid grid-2">
        <Explainer title="What makes a mesh printable (2-manifold, outward normals)" refs={[{ label: 'M. Botsch et al., Polygon Mesh Processing, A K Peters (2010), ch. 1' }]}>
          A solid's surface must be <b>closed</b> (every edge shared by exactly two triangles), <b>consistently oriented</b>{' '}
          (the two triangles traverse their shared edge in opposite directions, so all normals point out) and have positive
          signed volume. The validator counts every edge with a hash map keyed by its two vertex ids; any edge seen once is a
          hole, three or more times is non-manifold, twice in the same direction is a flipped triangle. Slicers repair such
          meshes heuristically — generating them correctly is better.
        </Explainer>
        <Explainer title="Sweeps and the ear bend" refs={[{ label: 'W. Wang, B. Jüttler, D. Zheng, Y. Liu, "Computation of rotation minimizing frames", ACM TOG 27(1), 2008' }]}>
          A sweep moves a 2D profile along a path. At each path point the profile is placed in a frame (tangent, normal,
          binormal); rotation-minimising frames (the double-reflection method) stop the profile from twisting. The temple
          arm is a planar sweep of its section along a centre line — straight to the ear-bend point, then an arc of radius
          22 mm through 38°, then a tapering tail — built as a drafted extrusion of the swept outline so every face keeps its
          molding draft.
        </Explainer>
      </div>
    </div>
  );
}

function TriangulationCard() {
  const project = useProject((s) => s.project);
  const { data } = useCompute<TriOut>('cad.triangulate', { project });
  const [step, setStep] = useState(0);
  const [playing, setPlaying] = useState(false);
  const n = data?.triangles.length ?? 0;
  const raf = useRef(0);
  useEffect(() => {
    setStep(n);
  }, [n]);
  useEffect(() => {
    if (!playing) return;
    if (prefersReducedMotion()) {
      setStep(n);
      setPlaying(false);
      return;
    }
    let last = performance.now();
    const tick = (now: number) => {
      const adv = Math.max(1, Math.floor((now - last) / 12));
      if (now - last >= 12) {
        last = now;
        setStep((s) => {
          const ns = Math.min(n, s + adv);
          if (ns >= n) setPlaying(false);
          return ns;
        });
      }
      raf.current = requestAnimationFrame(tick);
    };
    raf.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf.current);
  }, [playing, n]);
  const view = useMemo(() => {
    if (!data) return null;
    const xs = data.points.map((p) => p[0]);
    const ys = data.points.map((p) => p[1]);
    const minx = Math.min(...xs);
    const maxx = Math.max(...xs);
    const miny = Math.min(...ys);
    const maxy = Math.max(...ys);
    const pad = 4;
    return { vb: `${minx - pad} ${-maxy - pad} ${maxx - minx + 2 * pad} ${maxy - miny + 2 * pad}` };
  }, [data]);
  const loops = useMemo(() => {
    if (!data) return [];
    const out: [number, number][][] = [];
    let o = 0;
    for (const s of data.loop_sizes) {
      out.push(data.points.slice(o, o + s));
      o += s;
    }
    return out;
  }, [data]);
  const area = useMemo(() => {
    if (!data) return 0;
    let a = 0;
    for (const t of data.triangles.slice(0, step)) {
      const [p, q, r] = t.map((i) => data.points[i]);
      a += 0.5 * ((q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]));
    }
    return a;
  }, [data, step]);
  return (
    <Card
      title="Ear clipping, live on the Iris front"
      subtitle="The front's outline and both lens openings, triangulated the way the engine does it before extruding"
      actions={
        <div className="row">
          <button className="btn small" onClick={() => { if (step >= n) setStep(0); setPlaying((p) => !p); }}>{playing ? '❚❚ Pause' : '▶ Play'}</button>
          <button className="btn small" onClick={() => { setPlaying(false); setStep(0); }}>Reset</button>
        </div>
      }
    >
      {data && view && (
        <>
          <svg viewBox={view.vb} width="100%" style={{ maxHeight: 360, display: 'block' }} role="img" aria-label="Triangulation of the frame front">
            <g transform="scale(1,-1)">
              {data.triangles.slice(0, step).map((t, i) => {
                const pts = t.map((k) => data.points[k].join(',')).join(' ');
                const latest = i === step - 1;
                return (
                  <polygon
                    key={i}
                    points={pts}
                    fill={latest ? 'var(--series-2)' : `var(--series-${(i % 3) + 1})`}
                    fillOpacity={latest ? 0.9 : 0.22}
                    stroke="var(--text-2)"
                    strokeOpacity={0.35}
                    strokeWidth={0.12}
                  />
                );
              })}
              {loops.map((l, i) => (
                <polygon key={i} points={l.map((p) => p.join(',')).join(' ')} fill="none" stroke={i === 0 ? 'var(--accent)' : 'var(--series-5)'} strokeWidth={0.45} />
              ))}
            </g>
          </svg>
          <Slider label="Ears clipped" value={step} min={0} max={n} step={1} onChange={(v) => { setPlaying(false); setStep(Math.round(v)); }} format={(v) => `${Math.round(v)} / ${n}`} />
          <div className="small muted">
            Σ triangle area {area.toFixed(1)} mm² of {data.area_mm2.toFixed(1)} mm² · {data.loop_sizes[0]} outline vertices +{' '}
            {data.loop_sizes.slice(1).join(' + ')} hole vertices → {n} triangles (n + 2h − 2)
          </div>
        </>
      )}
      <div className="grid grid-2" style={{ marginTop: 12 }}>
        <ol className="cp-steps">
          <li>Bridge each hole into the outer loop through a mutually visible vertex pair (cast a ray to +x from the hole's rightmost vertex).</li>
          <li>Walk the merged loop; a convex vertex whose triangle with its two neighbours contains no reflex vertex is an <b>ear</b>.</li>
          <li>Clip the ear (emit the triangle, unlink the vertex) and repeat. A simple polygon with n vertices always has ≥ 2 ears (Meisters' two-ears theorem), so it ends with n − 2 triangles.</li>
        </ol>
        <Explainer
          title="Why triangulate?"
          refs={[
            { label: 'D. Eberly, "Triangulation by Ear Clipping", Geometric Tools', href: 'https://www.geometrictools.com/Documentation/TriangulationByEarClipping.pdf' },
            { label: 'de Berg et al., Computational Geometry (2008), ch. 3' },
          ]}
        >
          GPUs, STL files and the volume formula all speak triangles. The extruded front needs its top and bottom caps as
          triangles; the side walls are quads between offset rings. Ear clipping is O(n²) but simple and robust; production
          kernels use constrained Delaunay triangulation for better-shaped triangles.
        </Explainer>
      </div>
    </Card>
  );
}

function DraftCard() {
  const project = useProject((s) => s.project);
  const patch = useProject((s) => s.patchDesign);
  const f = project.design.frame;
  const [exaggerate, setExaggerate] = useState(true);
  const deg = f.draft_deg;
  const k = exaggerate ? 6 : 1;
  const H = f.pod_width_mm;
  const Wd = f.pod_height_mm;
  const w = f.wall_mm;
  const t = Math.tan((deg * Math.PI) / 180);
  const s = 12;
  const ob = Wd / 2; // outer half-width at the floor
  const ot = ob - H * t * k; // … at the rim
  const it = ot - w; // pocket half-width at the rim (wall = w there)
  const iff = it - (H - w) * t * k; // pocket half-width at the floor
  const ox = 30 + ob * s;
  const oy = 20 + H * s;
  const outer = [[-ob * s, 0], [ob * s, 0], [ot * s, -H * s], [-ot * s, -H * s]];
  const inner = [[-iff * s, -w * s], [iff * s, -w * s], [it * s, -H * s], [-it * s, -H * s]];
  const poly = (pts: number[][]) => pts.map(([x, y]) => `${x + ox},${y + oy}`).join(' ');
  return (
    <Card title="Extrusion with draft" subtitle="Cross-section through the electronics pod as molded (pull direction up)">
      <svg className="cp-svg" viewBox={`0 0 ${Wd * s + 60} ${H * s + 60}`} width="100%" style={{ maxHeight: 260, maxWidth: 420, display: "block", margin: "0 auto" }} role="img" aria-label="Draft cross-section">
        <polygon points={poly(outer)} fill="var(--accent-soft)" stroke="var(--accent)" strokeWidth={1.5} />
        <polygon points={poly(inner)} fill="var(--surface-1)" stroke="var(--series-2)" strokeWidth={1.5} />
        <line x1={ox} y1={oy - w * s - 6} x2={ox} y2={oy - H * s - 14} stroke="var(--text-2)" markerEnd="url(#pull)" />
        <text x={ox + 6} y={oy - H * s - 4}>pull</text>
        <line x1={ox - ob * s} y1={oy + 10} x2={ox + ob * s} y2={oy + 10} className="dim" />
        <text x={ox} y={oy + 24} textAnchor="middle">floor on the bed · parting plane</text>
        <text x={ox + ob * s + 4} y={oy - H * s / 2}>{deg.toFixed(1)}°</text>
        <defs>
          <marker id="pull" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
            <path d="M0 0L10 5L0 10z" fill="var(--text-2)" />
          </marker>
        </defs>
      </svg>
      <Slider label="Draft angle (writes to the project)" value={deg} min={0} max={5} step={0.1} unit="°" onChange={(v) => patch('frame', { draft_deg: v })} />
      <label className="cp-check"><input type="checkbox" checked={exaggerate} onChange={(e) => setExaggerate(e.target.checked)} /> exaggerate ×6</label>
      <Formula>{`offset(z) = −z · tan(α)     over ${H.toFixed(1)} mm: ${(H * t).toFixed(3)} mm per wall`}</Formula>
      <div className="small muted">
        Outer walls lean in going up and the pocket opens up, so the part releases when the mold opens along +Z. The pocket
        is sized so the wall is exactly {w} mm at the rim and thicker at the floor. The Molding Lab checks every face has at
        least 1° of draft (R-020).
      </div>
    </Card>
  );
}
