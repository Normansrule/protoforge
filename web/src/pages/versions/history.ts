// Onshape-style version history for the product model. Stored in
// localStorage under its own key — never inside project.json — so the
// project file stays a single configuration and the history is the list of
// baselines around it.
import { create } from 'zustand';
import type { Project } from '../../engine/types';
import { call } from '../../engine/engine';
import { workerCall } from '../../labs/pcb/useWorker';
import { useProject } from '../../state/project';

export const HISTORY_KEY = 'protoforge.versions.v1';
const MAX_AUTO = 25;
const AUTO_MIN_INTERVAL_MS = 60_000;
const AUTO_DEBOUNCE_MS = 4_000;

export interface Summary {
  hash: string; short: string; name: string; version: string; needs: number; requirements: number; subsystems: number;
  interfaces: number; components: number; risks: number; trades: number; bom_cost_usd: number; component_mass_g: number;
  evaluation: { pass: number; fail: number; unverified: number } | null;
}

export interface Version {
  id: string;
  name: string;
  description: string;
  created: number;
  branch: string;
  parent: string | null;
  /** Second parent of a merge version (the tip of the branch merged in). */
  merged?: string | null;
  /** Configuration-management baseline label (e.g. "Allocated baseline"), if this version is one. */
  baseline?: string | null;
  kind: 'manual' | 'auto' | 'example' | 'merge';
  summary: Summary;
  project: Project;
}

export interface Branch { name: string; from: string | null; created: number }

interface Persisted { versions: Version[]; branches: Branch[]; branch: string; base: string | null; auto: boolean; lastAuto: number; seeded: boolean }

interface HistoryState extends Persisted {
  storageError: string | null;
  create: (name: string, description: string, kind?: Version['kind'], merged?: string | null) => Version;
  setBaseline: (id: string, label: string | null) => void;
  commitMerge: (merged: Project, theirs: string, name: string, description: string) => Version;
  restore: (id: string) => void;
  branchFrom: (id: string, name: string) => void;
  switchBranch: (name: string) => void;
  remove: (id: string) => void;
  rename: (id: string, name: string, description: string) => void;
  setAuto: (v: boolean) => void;
  importJson: (text: string) => void;
  exportJson: () => string;
  reset: () => void;
}

const EMPTY: Persisted = { versions: [], branches: [{ name: 'main', from: null, created: 0 }], branch: 'main', base: null, auto: false, lastAuto: 0, seeded: false };

function load(): Persisted {
  try {
    const raw = localStorage.getItem(HISTORY_KEY);
    if (!raw) return EMPTY;
    const p = JSON.parse(raw) as Persisted;
    if (!Array.isArray(p.versions) || !Array.isArray(p.branches)) return EMPTY;
    return { ...EMPTY, ...p };
  } catch {
    return EMPTY;
  }
}

function persisted(s: HistoryState): Persisted {
  return { versions: s.versions, branches: s.branches, branch: s.branch, base: s.base, auto: s.auto, lastAuto: s.lastAuto, seeded: s.seeded };
}

const uid = () => `v${Date.now().toString(36)}${Math.random().toString(36).slice(2, 7)}`;

/** Fingerprint + counts of a project (no evaluation; synchronous, ~1 ms). */
export function quickSummary(p: Project): Summary {
  return call<Summary>('bridge.summary', { project: p, evaluate: false });
}

/** Summary with every requirement evaluated, in the engine worker. */
export function fullSummary(p: Project): Promise<Summary> {
  return workerCall<Summary>('bridge.summary', { project: p, evaluate: true });
}

export const useHistory = create<HistoryState>((set, get) => {
  const save = () => {
    try {
      localStorage.setItem(HISTORY_KEY, JSON.stringify(persisted(get())));
      if (get().storageError) set({ storageError: null });
    } catch (e) {
      set({ storageError: `Could not save the history in this browser (${e instanceof Error ? e.message : e}). Export it as JSON to keep it.` });
    }
  };
  const fill = (id: string, p: Project) => {
    fullSummary(p)
      .then((s) => {
        set({ versions: get().versions.map((v) => (v.id === id ? { ...v, summary: s } : v)) });
        save();
      })
      .catch(() => undefined);
  };
  return {
    ...load(),
    storageError: null,
    create: (name, description, kind = 'manual', merged = null) => {
      const project = structuredClone(useProject.getState().project);
      const v: Version = { id: uid(), name, description, created: Date.now(), branch: get().branch, parent: get().base, merged, kind, summary: quickSummary(project), project };
      let versions = [...get().versions, v];
      if (kind === 'auto') {
        const autos = versions.filter((x) => x.kind === 'auto');
        if (autos.length > MAX_AUTO) {
          const drop = autos[0];
          versions = versions.filter((x) => x.id !== drop.id).map((x) => (x.parent === drop.id ? { ...x, parent: drop.parent } : x));
        }
      }
      set({ versions, base: v.id, lastAuto: kind === 'auto' ? Date.now() : get().lastAuto });
      save();
      fill(v.id, project);
      return v;
    },
    setBaseline: (id, label) => {
      set({ versions: get().versions.map((v) => (v.id === id ? { ...v, baseline: label } : v)) });
      save();
    },
    commitMerge: (merged, theirs, name, description) => {
      useProject.getState().setProject(structuredClone(merged));
      return get().create(name, description, 'merge', theirs);
    },
    restore: (id) => {
      const v = get().versions.find((x) => x.id === id);
      if (!v) return;
      useProject.getState().setProject(structuredClone(v.project));
      set({ base: v.id, branch: v.branch });
      save();
    },
    branchFrom: (id, name) => {
      const v = get().versions.find((x) => x.id === id);
      const clean = name.trim();
      if (!v || !clean) return;
      if (get().branches.some((b) => b.name === clean)) {
        alert(`A branch called “${clean}” already exists — pick another name or switch to it.`);
        return;
      }
      useProject.getState().setProject(structuredClone(v.project));
      set({ branches: [...get().branches, { name: clean, from: id, created: Date.now() }], branch: clean, base: v.id });
      save();
    },
    switchBranch: (name) => {
      const latest = [...get().versions].filter((v) => v.branch === name).sort((a, b) => b.created - a.created)[0];
      const br = get().branches.find((b) => b.name === name);
      const target = latest ?? get().versions.find((v) => v.id === br?.from);
      if (target) useProject.getState().setProject(structuredClone(target.project));
      set({ branch: name, base: target?.id ?? get().base });
      save();
    },
    remove: (id) => {
      const v = get().versions.find((x) => x.id === id);
      if (!v) return;
      set({
        versions: get()
          .versions.filter((x) => x.id !== id)
          .map((x) => (x.parent === id ? { ...x, parent: v.parent } : x))
          .map((x) => (x.merged === id ? { ...x, merged: v.parent } : x)),
        branches: get().branches.map((b) => (b.from === id ? { ...b, from: v.parent } : b)),
        base: get().base === id ? v.parent : get().base,
      });
      save();
    },
    rename: (id, name, description) => {
      set({ versions: get().versions.map((v) => (v.id === id ? { ...v, name, description } : v)) });
      save();
    },
    setAuto: (auto) => {
      set({ auto });
      save();
    },
    exportJson: () => JSON.stringify({ format: 'protoforge-versions', version: 1, exported: new Date().toISOString(), history: persisted(get()) }, null, 1),
    importJson: (text) => {
      const o = JSON.parse(text) as { format?: string; history?: Persisted };
      if (o.format !== 'protoforge-versions' || !o.history || !Array.isArray(o.history.versions)) throw new Error('not a Protoforge version history (format "protoforge-versions")');
      const h = { ...EMPTY, ...o.history, seeded: true };
      if (!h.branches.some((b) => b.name === 'main')) h.branches = [{ name: 'main', from: null, created: 0 }, ...h.branches];
      set(h);
      save();
    },
    reset: () => {
      set({ ...EMPTY });
      save();
    },
  };
});

interface ExampleVersion { name: string; description: string; branch: string; parent: number | null; baseline: string | null; project: Project }

/** First visit: seed Iris v0.3 → v0.4 (the story the simulations tell) and an exploratory branch. */
export function seedIfEmpty() {
  const h = useHistory.getState();
  if (h.seeded || h.versions.length > 0) return;
  const ex = call<ExampleVersion[]>('bridge.example_history');
  const now = Date.now();
  const versions: Version[] = [];
  ex.forEach((e, i) => {
    versions.push({
      id: `example-${i}`, name: e.name, description: e.description, created: now - (ex.length - i) * 60_000, branch: e.branch || 'main',
      parent: e.parent === null ? null : `example-${e.parent}`, baseline: e.baseline, kind: 'example', summary: quickSummary(e.project), project: e.project,
    });
  });
  const branches: Branch[] = [{ name: 'main', from: null, created: 0 }];
  versions.forEach((v) => {
    if (!branches.some((b) => b.name === v.branch)) branches.push({ name: v.branch, from: v.parent, created: v.created });
  });
  // the working state starts on main at the newest main version (the shipped project)
  const mainTip = [...versions].filter((v) => v.branch === 'main').pop();
  useHistory.setState({ versions, branches, branch: 'main', base: mainTip?.id ?? null, seeded: true });
  try {
    localStorage.setItem(HISTORY_KEY, JSON.stringify(persisted(useHistory.getState())));
  } catch {
    /* storage unavailable: history lives in memory */
  }
  versions.forEach((v) =>
    fullSummary(v.project).then((s) => {
      useHistory.setState({ versions: useHistory.getState().versions.map((x) => (x.id === v.id ? { ...x, summary: s } : x)) });
      try {
        localStorage.setItem(HISTORY_KEY, JSON.stringify(persisted(useHistory.getState())));
      } catch {
        /* ignore */
      }
    }),
  );
}

// ---------------------------------------------------------------------------
// Auto-versioning: snapshot when a change is *significant* — a requirement
// verdict count, the requirement/BOM/subsystem structure, BOM cost or mass
// changes — at most once a minute, 4 s after the last edit.
// ---------------------------------------------------------------------------

let autoStarted = false;
let timer: ReturnType<typeof setTimeout> | null = null;

export function significant(a: Summary, b: Summary): string | null {
  const ea = a.evaluation;
  const eb = b.evaluation;
  if (ea && eb && (ea.pass !== eb.pass || ea.fail !== eb.fail || ea.unverified !== eb.unverified)) return `verdicts ✓${eb.pass} ✕${eb.fail}`;
  if (a.requirements !== b.requirements) return `${b.requirements} requirements`;
  if (a.components !== b.components) return `${b.components} BOM lines`;
  if (a.subsystems !== b.subsystems || a.interfaces !== b.interfaces) return 'architecture changed';
  if (Math.abs(a.bom_cost_usd - b.bom_cost_usd) > 0.01 * Math.max(1, a.bom_cost_usd)) return `BOM $${b.bom_cost_usd.toFixed(2)}`;
  if (Math.abs(a.component_mass_g - b.component_mass_g) > 0.01 * Math.max(1, a.component_mass_g)) return `mass ${b.component_mass_g.toFixed(1)} g`;
  return null;
}

async function maybeAutoVersion(project: Project) {
  const h = useHistory.getState();
  if (!h.auto || Date.now() - h.lastAuto < AUTO_MIN_INTERVAL_MS) return;
  const base = h.versions.find((v) => v.id === h.base);
  const now = await fullSummary(project);
  if (useProject.getState().project !== project) return; // superseded by a newer edit
  if (base && base.summary.hash === now.hash) return;
  const why = base ? significant(base.summary, now) : 'first snapshot';
  if (!why) return;
  const t = new Date();
  useHistory.getState().create(`Auto · ${t.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`, `Automatic snapshot: ${why}.`, 'auto');
}

export function startAutoVersioning() {
  if (autoStarted) return;
  autoStarted = true;
  useProject.subscribe((s, prev) => {
    if (s.project === prev.project || !useHistory.getState().auto) return;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => void maybeAutoVersion(s.project), AUTO_DEBOUNCE_MS);
  });
}


// ---------------------------------------------------------------------------
// Graph helpers
// ---------------------------------------------------------------------------

/** Every ancestor of a version (itself included), following both parents of merges. */
export function ancestors(versions: Version[], id: string | null): Set<string> {
  const out = new Set<string>();
  const stack = id ? [id] : [];
  while (stack.length) {
    const cur = stack.pop()!;
    if (out.has(cur)) continue;
    out.add(cur);
    const v = versions.find((x) => x.id === cur);
    if (v?.parent) stack.push(v.parent);
    if (v?.merged) stack.push(v.merged);
  }
  return out;
}

/** Newest common ancestor of two versions (the merge base), or null. */
export function mergeBase(versions: Version[], a: string | null, b: string | null): Version | null {
  const A = ancestors(versions, a);
  const common = [...ancestors(versions, b)].filter((id) => A.has(id));
  const vs = common.map((id) => versions.find((v) => v.id === id)!).filter(Boolean);
  return vs.sort((x, y) => y.created - x.created)[0] ?? null;
}

/** Latest version saved on a branch. */
export function branchTip(versions: Version[], branch: string): Version | null {
  return [...versions].filter((v) => v.branch === branch).sort((a, b) => b.created - a.created)[0] ?? null;
}
