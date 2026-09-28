import { useState } from 'react';
import { useCompute } from '../../engine/hooks';
import { Card, Code, ErrorBox, Explainer, Formula, Stat } from '../../components/ui';
import { useElementWidth } from '../../components/useElementWidth';
import type { KMap, MinimizeResult, SimResult } from './types';
import { GateDiagram } from './gates';
import { Seg } from '../dsp/viz';

const EXAMPLES: { label: string; expr?: string; minterms?: number[]; dc?: number[]; n?: number }[] = [
  { label: 'Majority (a·b + a·c + b·c)', expr: 'a b + a c + b c' },
  { label: 'Σm(0,1,2,5,6,7) — cyclic', minterms: [0, 1, 2, 5, 6, 7], n: 3 },
  { label: 'Corners Σm(0,2,8,10)', minterms: [0, 2, 8, 10], n: 4 },
  { label: 'With don’t cares', minterms: [1, 3, 7, 11, 15], dc: [0, 2, 5], n: 4 },
  { label: '7-seg segment “e”', minterms: [0, 2, 6, 8], dc: [10, 11, 12, 13, 14, 15], n: 4 },
  { label: 'Parity a^b^c^d', expr: 'a ^ b ^ c ^ d' },
  { label: 'Messy expression', expr: "A'B'C'D + A B C + C D' + A'D + (A + B)'C" },
];

const GROUP_COLORS = ['var(--series-1)', 'var(--series-2)', 'var(--series-3)', 'var(--series-4)', 'var(--series-5)', 'var(--series-7)', 'var(--series-8)', 'var(--series-6)'];

function KMapView({ k, replay }: { k: KMap; replay: number }) {
  const [ref, width] = useElementWidth<HTMLDivElement>();
  const cell = Math.max(38, Math.min(64, ((width || 360) - 90) / k.cols));
  const L = 70;
  const T = 56;
  const W = L + k.cols * cell + 16;
  const H = T + k.rows * cell + 16;
  return (
    <div className="kmap-wrap" ref={ref}>
      <svg width={W} height={H} role="img" aria-label="Karnaugh map" key={replay}>
        <text x={L - 10} y={T - 30} fontSize={12} textAnchor="end" fill="var(--text-2)" fontFamily="var(--mono)">{`${k.row_vars || '·'} \\ ${k.col_vars}`}</text>
        {k.col_labels.map((c, i) => <text key={c} x={L + (i + 0.5) * cell} y={T - 10} fontSize={12} textAnchor="middle" fill="var(--text-2)" fontFamily="var(--mono)">{c}</text>)}
        {k.row_labels.map((r, i) => <text key={r + i} x={L - 10} y={T + (i + 0.5) * cell + 4} fontSize={12} textAnchor="end" fill="var(--text-2)" fontFamily="var(--mono)">{r || '—'}</text>)}
        {k.cells.map((c) => (
          <g key={c.minterm}>
            <rect x={L + c.col * cell} y={T + c.row * cell} width={cell} height={cell} fill={c.value === '1' ? 'var(--accent-soft)' : 'var(--surface-2)'} stroke="var(--border-strong)" />
            <text x={L + (c.col + 0.5) * cell} y={T + (c.row + 0.5) * cell + 6} fontSize={17} textAnchor="middle" fontWeight={700} fill={c.value === '1' ? 'var(--text)' : c.value === 'x' ? 'var(--warn)' : 'var(--muted)'} fontFamily="var(--mono)">{c.value === 'x' ? '×' : c.value}</text>
            <text x={L + c.col * cell + 4} y={T + c.row * cell + 11} fontSize={9} fill="var(--muted)" fontFamily="var(--mono)">{c.minterm}</text>
          </g>
        ))}
        {k.groups.map((g, gi) =>
          g.rects.map((r, ri) => {
            const inset = 4 + (gi % 3) * 3;
            const x = L + r.c0 * cell + inset;
            const y = T + r.r0 * cell + inset;
            const w = (r.c1 - r.c0 + 1) * cell - 2 * inset;
            const h = (r.r1 - r.r0 + 1) * cell - 2 * inset;
            return (
              <rect key={`${gi}-${ri}`} className="kgroup" style={{ animationDelay: `${0.35 + gi * 0.45}s` }} x={x} y={y} width={w} height={h} rx={Math.min(14, cell / 3)} fill={GROUP_COLORS[gi % 8]} fillOpacity={0.12} stroke={GROUP_COLORS[gi % 8]} strokeWidth={2.6} />
            );
          }),
        )}
      </svg>
    </div>
  );
}

export function LogicTab(props: { onSimulate: (expr: { expr?: string; minterms?: number[]; dont_cares?: number[]; num_vars?: number }) => void }) {
  const [mode, setMode] = useState<'expr' | 'minterms'>('expr');
  const [expr, setExpr] = useState('a b + a c + b c');
  const [mts, setMts] = useState('0,1,2,5,6,7');
  const [dcs, setDcs] = useState('');
  const [nv, setNv] = useState(3);
  const [replay, setReplay] = useState(0);
  const parseList = (s: string) => s.split(/[\s,]+/).filter(Boolean).map(Number).filter((v) => Number.isInteger(v) && v >= 0 && v < 256);
  const input = mode === 'expr' ? { expr, dont_cares: parseList(dcs) } : { expr: '', minterms: parseList(mts), dont_cares: parseList(dcs), num_vars: nv };
  const r = useCompute<MinimizeResult>('chip.minimize', input);
  const d = r.data;
  const sim = useCompute<SimResult>('chip.simulate', { logic: input }, !!d);
  const n = d?.vars.length ?? 0;
  return (
    <div className="stack">
      <div className="grid grid-sidebar">
        <Card title="Boolean function" subtitle="Expression or minterm list, up to 8 variables">
          <Seg value={mode} options={[{ value: 'expr', label: 'Expression' }, { value: 'minterms', label: 'Minterms Σm' }]} onChange={setMode} />
          {mode === 'expr' ? (
            <label className="field">
              <span className="field-top"><span className="field-label">F =</span><span className="muted small">&amp; * · | + ^ ! ~ ' ( )</span></span>
              <input type="text" value={expr} onChange={(e) => setExpr(e.target.value)} spellCheck={false} style={{ fontFamily: 'var(--mono)' }} aria-label="Boolean expression" />
            </label>
          ) : (
            <>
              <label className="field">
                <span className="field-top"><span className="field-label">Minterms</span></span>
                <input type="text" value={mts} onChange={(e) => setMts(e.target.value)} style={{ fontFamily: 'var(--mono)' }} aria-label="minterms" />
              </label>
              <Seg label="Variables" value={nv} options={[2, 3, 4, 5, 6].map((v) => ({ value: v, label: String(v) }))} onChange={setNv} />
            </>
          )}
          <label className="field">
            <span className="field-top"><span className="field-label">Don’t cares d(…)</span></span>
            <input type="text" value={dcs} onChange={(e) => setDcs(e.target.value)} placeholder="e.g. 10,11" style={{ fontFamily: 'var(--mono)' }} aria-label="don't cares" />
          </label>
          <div className="pill-row">
            {EXAMPLES.map((ex) => (
              <button
                key={ex.label}
                className="btn small"
                onClick={() => {
                  if (ex.expr) {
                    setMode('expr');
                    setExpr(ex.expr);
                    setDcs('');
                  } else {
                    setMode('minterms');
                    setMts(ex.minterms!.join(','));
                    setDcs((ex.dc ?? []).join(','));
                    setNv(ex.n ?? 4);
                  }
                  setReplay((x) => x + 1);
                }}
              >
                {ex.label}
              </button>
            ))}
          </div>
          <ErrorBox error={r.error} />
          {d && (
            <div className="tt-scroll" style={{ marginTop: 12 }}>
              <table className="tt-table" style={{ width: '100%' }}>
                <thead>
                  <tr><th>m</th>{d.vars.map((v) => <th key={v}>{v}</th>)}<th>F</th></tr>
                </thead>
                <tbody>
                  {d.table.map((f, m) => (
                    <tr key={m}>
                      <td className="muted">{m}</td>
                      {d.vars.map((v, i) => <td key={v}>{(m >> (n - 1 - i)) & 1}</td>)}
                      <td className={f === '1' ? 'one' : f === 'x' ? 'x' : ''}>{f}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </Card>
        <div className="stack">
          {d && (
            <div className="lab-hero">
              <Stat label="Minimum SOP" value={<span className="mono" style={{ fontSize: 17 }}>F = {d.sop}</span>} note={`${d.sop_literals} literals · ${d.method}`} />
              <Stat label="Minimum POS" value={<span className="mono" style={{ fontSize: 17 }}>F = {d.pos}</span>} note={`${d.pos_literals} literals`} />
            </div>
          )}
          <div className="grid grid-2">
            <Card title="Karnaugh map" subtitle="Gray-code order: neighbours differ in one bit, and the edges wrap" actions={<button className="btn small" onClick={() => setReplay((x) => x + 1)}>↻ Replay</button>}>
              {d?.kmap ? <KMapView k={d.kmap} replay={replay} /> : <p className="muted small">K-maps are drawn for up to 4 variables; this function has {n}. The tabular Quine–McCluskey method on the right scales further.</p>}
              {d?.kmap && (
                <div className="pill-row" style={{ marginTop: 8 }}>
                  {d.kmap.groups.map((g, i) => (
                    <span key={g.pattern} className="chip" style={{ borderColor: GROUP_COLORS[i % 8] }}>
                      <span style={{ width: 9, height: 9, borderRadius: 3, background: GROUP_COLORS[i % 8], display: 'inline-block' }} /> {g.term} {g.essential ? '· essential' : ''}
                    </span>
                  ))}
                </div>
              )}
            </Card>
            <Card title="Quine–McCluskey" subtitle={d ? `${d.primes.length} prime implicants; ${d.sigma}` : ''}>
              {d && (
                <div className="table-wrap">
                  <table className="table">
                    <thead><tr><th>Prime</th><th>Term</th><th>Covers</th><th>Status</th></tr></thead>
                    <tbody>
                      {d.primes.slice(0, 40).map((p) => (
                        <tr key={p.pattern} style={p.chosen ? { background: 'var(--accent-soft)' } : undefined}>
                          <td className="mono">{p.pattern}</td>
                          <td className="mono">{p.term}</td>
                          <td className="mono small">{p.minterms.join(',')}</td>
                          <td className="small">{p.essential ? '★ essential' : p.chosen ? '✓ chosen' : '—'}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                  {d.primes.length > 40 && <p className="muted small">… {d.primes.length - 40} more</p>}
                  <p className="muted small" style={{ marginTop: 6 }}>Merge rounds: {d.qm_columns.map((c) => c.length).join(' → ')} implicants</p>
                </div>
              )}
            </Card>
          </div>
          <Card
            title="Gate-level realisation"
            subtitle="Two-level AND–OR network from the minimum SOP (inverters on complemented inputs)"
            actions={<button className="btn small primary" onClick={() => props.onSimulate(input)}>Simulate in the gate simulator →</button>}
          >
            {sim.data && <GateDiagram inputs={sim.data.inputs} outputs={sim.data.outputs} gates={sim.data.gates} levels={sim.data.levels} />}
          </Card>
        </div>
      </div>
      <div className="grid grid-2">
        {d && <Card title="Verilog"><Code code={d.verilog} filename="logic_fn.v" maxHeight={260} /></Card>}
        <div className="stack">
          <Explainer title="Quine–McCluskey and Petrick's method" refs={[{ label: 'W. V. Quine, Amer. Math. Monthly 59 (1952)' }, { label: 'E. J. McCluskey, Bell Syst. Tech. J. 35 (1956)' }, { label: 'S. R. Petrick, "A direct determination of the irredundant forms of a Boolean function…", 1956' }]}>
            <Formula>{"x·y + x·y' = x          (combine terms differing in one bit)\n1-01 + 1-11 → 1--1      (don't-care bits written as -)"}</Formula>
            Group minterms by their number of 1s, merge neighbours until nothing merges: what's left are the prime implicants. Essential primes
            cover a minterm nobody else does. For the rest, Petrick writes "cover every minterm" as a product of sums and multiplies it out to
            find the cheapest cover — exactly, when it fits.
          </Explainer>
          <Explainer title="Karnaugh maps" refs={[{ label: 'M. Karnaugh, Trans. AIEE 72 (1953)' }]}>
            A K-map lays the truth table out in Gray-code order so that algebraic neighbours are geometric neighbours; each rectangle of 2ᵏ ones is
            a product term with k variables eliminated. Iris's touch FSM next-state logic is exactly this kind of function — the Chip Lab
            generates it in Verilog and lets Yosys do the minimisation at scale.
          </Explainer>
        </div>
      </div>
    </div>
  );
}
