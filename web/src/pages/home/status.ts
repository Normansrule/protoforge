// Live requirement status helpers shared by the Overview page sections.
import type { Evaluation, Metric, Project, ReqResult } from '../../engine/types';
import { goalIds, shownStatus, type Shown } from '../../components/reqStatus';
import { GROUPS, LABS, type LabInfo } from '../../labs/registry';

export interface ReqStatus {
  id: string;
  text: string;
  metric: string;
  result: ReqResult | undefined;
  /** What to show: a failing goal is an open goal (amber), not a fail. */
  shown: Shown;
}

export interface Tally {
  pass: number;
  /** binding failures only */
  fail: number;
  /** open stretch goals (failing requirements of kind "goal") */
  goal: number;
  unverified: number;
  total: number;
  reqs: ReqStatus[];
}

const EMPTY: Tally = { pass: 0, fail: 0, goal: 0, unverified: 0, total: 0, reqs: [] };

/** Requirements whose bound metric starts with one of `prefixes`, with live results. */
export function tallyFor(prefixes: string[], project: Project, ev: Evaluation | null): Tally {
  const goals = goalIds(project);
  const reqs: ReqStatus[] = project.requirements
    .filter((r) => r.verification.metric && prefixes.some((p) => r.verification.metric!.startsWith(p)))
    .map((r) => {
      const result = ev?.results.find((x) => x.id === r.id);
      return { id: r.id, text: r.text, metric: r.verification.metric!, result, shown: shownStatus(result, goals) };
    });
  if (reqs.length === 0) return EMPTY;
  const pass = reqs.filter((r) => r.shown === 'pass').length;
  const fail = reqs.filter((r) => r.shown === 'fail').length;
  const goal = reqs.filter((r) => r.shown === 'goal').length;
  return { pass, fail, goal, unverified: reqs.length - pass - fail - goal, total: reqs.length, reqs };
}

export type Health = 'pass' | 'fail' | 'goal' | 'partial' | 'none';

export function health(t: Tally): Health {
  if (t.total === 0) return 'none';
  if (t.fail > 0) return 'fail';
  if (t.goal > 0) return 'goal';
  if (t.pass === t.total) return 'pass';
  return 'partial';
}

export const healthIcon = (h: Health) => (h === 'pass' ? '✓' : h === 'fail' ? '✕' : h === 'goal' ? '!' : h === 'partial' ? '…' : '○');
export const healthWord = (h: Health) => (h === 'pass' ? 'verified' : h === 'fail' ? 'failing' : h === 'goal' ? 'open goal' : h === 'partial' ? 'in progress' : 'no requirements');

/** Metric key prefixes of every lab in a registry group. */
export function groupPrefixes(group: string): string[] {
  return LABS.filter((l) => l.group === group).flatMap((l) => l.metrics);
}

/** Prefixes covered by no lab (system-level metrics such as mass.total_g, fit.*, bom.*). */
export function systemPrefixes(project: Project): string[] {
  const labPrefixes = LABS.flatMap((l) => l.metrics);
  const out = new Set<string>();
  for (const r of project.requirements) {
    const m = r.verification.metric;
    if (m && !labPrefixes.some((p) => m.startsWith(p))) out.add(m.split('.')[0] + '.');
  }
  return [...out];
}

/** The headline number shown on each lab card (falls back to the lab's first metric). */
export const HEADLINE: Record<string, { key: string; label: string }> = {
  circuit: { key: 'circuit.rail_droop_mv', label: 'Rail droop' },
  power: { key: 'power.battery_life_h', label: 'Battery life' },
  pcb: { key: 'pcb.routed_pct', label: 'Nets routed' },
  dsp: { key: 'dsp.decim_stopband_db', label: 'Decimation stopband' },
  chip: { key: 'chip.critical_path_ns', label: 'Critical path' },
  control: { key: 'control.head_rms_err_deg', label: 'Head-tracking error' },
  robotics: { key: 'robotics.pnp_cycle_s', label: 'Pick-and-place cycle' },
  rl: { key: 'rl.policy_life_gain_pct', label: 'Learned battery gain' },
  optics: { key: 'optics.hfov_deg', label: 'Field of view' },
  cad: { key: 'cad.frame_mass_g', label: 'Frame mass' },
  print: { key: 'print.total_time_h', label: 'Print time' },
  molding: { key: 'molding.min_draft_deg', label: 'Minimum draft' },
  mfg: { key: 'mfg.unit_cost_usd', label: 'Unit cost @ target qty' },
};

export function headline(lab: LabInfo, metrics: Map<string, Metric>): { label: string; metric: Metric } | null {
  const h = HEADLINE[lab.id];
  if (h) {
    const m = metrics.get(h.key);
    if (m) return { label: h.label, metric: m };
  }
  for (const m of metrics.values()) {
    if (lab.metrics.some((p) => m.key.startsWith(p))) return { label: m.label, metric: m };
  }
  return null;
}

/** Compact number formatting for dashboards. */
export function fmt(v: number): string {
  const a = Math.abs(v);
  if (a >= 10000) return v.toLocaleString('en-US', { maximumFractionDigits: 0 });
  if (a >= 100) return v.toFixed(0);
  if (a >= 10) return v.toFixed(1);
  if (a >= 1) return v.toFixed(2);
  if (a === 0) return '0';
  return v.toPrecision(2);
}

export function unitText(u: string): string {
  return u === 'uA' ? 'µA' : u === 'C' ? '°C' : u === 'deg' ? '°' : u;
}

export { GROUPS, LABS };
