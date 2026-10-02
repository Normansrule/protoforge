// Animated reliability block diagram: subsystems in series (left to right,
// wrapping), their parts inside, k-of-n groups as parallel branches. Each
// block shows its reliability at time t; one Monte Carlo unit is replayed so
// blocks fail (✕) at their sampled times and the signal path breaks.
import type { RbdNode } from './types';
import { fmt } from './util';

export function blockRel(n: RbdNode, tH: number): number {
  const q = Math.max(0, n.qty ?? 1);
  if (n.model === 'weibull') {
    if (!(n.eta > 0 && n.beta > 0)) return 1;
    const per = n.per_hour > 0 ? n.per_hour : 1;
    return Math.exp(-q * ((tH * per) / n.eta) ** n.beta);
  }
  return Math.exp(-q * n.fit * 1e-9 * tH);
}

export function nodeRel(n: RbdNode, tH: number): number {
  if (!n.children?.length) return blockRel(n, tH);
  const rs = n.children.map((c) => nodeRel(c, tH));
  if (n.kind === 'parallel') return 1 - rs.reduce((a, r) => a * (1 - r), 1);
  if (n.kind === 'kofn') {
    const k = Math.max(1, Math.min(n.k, rs.length));
    let dp = [1];
    rs.forEach((r) => {
      const nx = new Array(dp.length + 1).fill(0);
      dp.forEach((p, j) => {
        nx[j] += p * (1 - r);
        nx[j + 1] += p * r;
      });
      dp = nx;
    });
    return dp.slice(k).reduce((a, b) => a + b, 0);
  }
  return rs.reduce((a, r) => a * r, 1);
}

/** Is the node failed at time t for a simulated unit (leaf times to failure by block id)? */
export function nodeFailed(n: RbdNode, tH: number, ttf: Map<string, number>): boolean {
  if (!n.children?.length) return (ttf.get(n.id) ?? Infinity) <= tH;
  const f = n.children.map((c) => nodeFailed(c, tH, ttf));
  if (n.kind === 'parallel') return f.every(Boolean);
  if (n.kind === 'kofn') return f.filter((x) => !x).length < Math.max(1, n.k);
  return f.some(Boolean);
}

function shortLabel(s: string) {
  return s.replace(/\s*\(.*?\)\s*/g, ' ').replace(/\s+/g, ' ').trim();
}

function Block({ n, tH, ttf, selected, onSelect }: { n: RbdNode; tH: number; ttf: Map<string, number> | null; selected: string | null; onSelect: (id: string) => void }) {
  const r = blockRel(n, tH);
  const failed = ttf ? (ttf.get(n.id) ?? Infinity) <= tH : false;
  return (
    <button
      className={`rl-blk ${failed ? 'failed' : ''} ${n.model === 'weibull' ? 'wear' : ''} ${selected === n.id ? 'sel' : ''}`}
      onClick={() => onSelect(n.id)}
      title={`${n.label} — ${n.class}\n${n.model === 'weibull' ? `Weibull β ${fmt(n.beta)}, η ${fmt(n.eta)} ${n.life_unit}` : `${fmt(n.fit)} FIT`}${n.qty !== 1 ? ` × ${n.qty}` : ''}\nR(t) = ${(r * 100).toFixed(3)} %`}
    >
      <span className="rl-blk-top">
        <span className="rl-blk-ref">{n.refdes || n.id}</span>
        <span className="rl-blk-name">{shortLabel(n.label)}{n.qty !== 1 ? ` ×${n.qty}` : ''}</span>
        <span className="rl-blk-state" aria-label={failed ? 'failed' : 'working'}>{failed ? '✕' : n.model === 'weibull' ? '◷' : ''}</span>
      </span>
      <span className="rl-blk-bar"><span style={{ width: `${Math.max(0, Math.min(1, r)) * 100}%` }} /></span>
    </button>
  );
}

function Inner({ n, tH, ttf, selected, onSelect }: { n: RbdNode; tH: number; ttf: Map<string, number> | null; selected: string | null; onSelect: (id: string) => void }) {
  if (!n.children?.length) return <Block n={n} tH={tH} ttf={ttf} selected={selected} onSelect={onSelect} />;
  const failed = ttf ? nodeFailed(n, tH, ttf) : false;
  if (n.kind === 'kofn' || n.kind === 'parallel') {
    const k = n.kind === 'parallel' ? 1 : n.k;
    return (
      <div className={`rl-par ${failed ? 'failed' : ''}`}>
        <div className="rl-par-head small">{k} of {n.children.length} needed · R {(nodeRel(n, tH) * 100).toFixed(3)} %</div>
        <div className="rl-par-branches">
          {n.children.map((c) => (
            <Inner key={c.id} n={c} tH={tH} ttf={ttf} selected={selected} onSelect={onSelect} />
          ))}
        </div>
      </div>
    );
  }
  return (
    <div className="rl-ser">
      {n.children.map((c) => (
        <Inner key={c.id} n={c} tH={tH} ttf={ttf} selected={selected} onSelect={onSelect} />
      ))}
    </div>
  );
}

export function RbdDiagram(props: { root: RbdNode; tH: number; ttf: Map<string, number> | null; flowing: boolean; selected: string | null; onSelect: (id: string) => void }) {
  const { root, tH, ttf } = props;
  const sysFailed = ttf ? nodeFailed(root, tH, ttf) : false;
  return (
    <div className={`rl-rbd ${props.flowing && !sysFailed ? 'flow' : ''}`} role="group" aria-label="Reliability block diagram">
      <div className="rl-io">in</div>
      {root.children.map((g) => {
        const gf = ttf ? nodeFailed(g, tH, ttf) : false;
        const r = nodeRel(g, tH);
        return (
          <div key={g.id} className="rl-rbd-step">
            <span className={`rl-wire ${gf || sysFailed ? 'cut' : ''}`} aria-hidden />
            <div className={`rl-grp ${gf ? 'failed' : ''}`}>
              <div className="rl-grp-head">
                <b>{g.label}</b>
                <span className="small tnum">{gf ? '✕ failed' : `R ${(r * 100).toFixed(2)} %`}</span>
              </div>
              <Inner n={{ ...g, kind: g.kind || 'series' }} tH={tH} ttf={ttf} selected={props.selected} onSelect={props.onSelect} />
            </div>
          </div>
        );
      })}
      <span className={`rl-wire ${sysFailed ? 'cut' : ''}`} aria-hidden />
      <div className={`rl-io ${sysFailed ? 'bad' : 'good'}`}>{sysFailed ? '✕ out' : '✓ out'}</div>
    </div>
  );
}
