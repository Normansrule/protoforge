// Global project store. The whole product lives in one JSON document
// (see examples/smart-glasses/project.json); every lab reads from it and the
// Systems Studio verifies requirements against it.
import { create } from 'zustand';
import { useEffect, useMemo } from 'react';
import iris from '../../../examples/smart-glasses/project.json';
import type { Design, Evaluation, Project } from '../engine/types';
import { useEngineReady, type Computed } from '../engine/hooks';
import { callInWorker } from '../engine/worker';

const STORAGE_KEY = 'protoforge.project.v1';

function loadSaved(): Project | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? (JSON.parse(raw) as Project) : null;
  } catch {
    return null;
  }
}

function save(p: Project) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(p));
  } catch {
    /* storage unavailable (private mode) – state still lives in memory */
  }
}

export const IRIS = iris as unknown as Project;

interface ProjectState {
  project: Project;
  setProject: (p: Project) => void;
  /** Immutably update the project with a producer function. */
  update: (fn: (draft: Project) => void) => void;
  /** Patch one design section, e.g. patchDesign('frame', { bridge_mm: 20 }). */
  patchDesign: <K extends keyof Design>(section: K, patch: Partial<Design[K]>) => void;
  reset: () => void;
}

export const useProject = create<ProjectState>((set, get) => ({
  project: loadSaved() ?? structuredClone(IRIS),
  setProject: (p) => {
    save(p);
    set({ project: p });
  },
  update: (fn) => {
    const next = structuredClone(get().project);
    fn(next);
    save(next);
    set({ project: next });
  },
  patchDesign: (section, patch) => {
    const next = structuredClone(get().project);
    next.design[section] = { ...(next.design[section] as object), ...(patch as object) } as Design[typeof section];
    save(next);
    set({ project: next });
  },
  reset: () => {
    const p = structuredClone(IRIS);
    save(p);
    set({ project: p });
  },
}));

// ---------------------------------------------------------------------------
// Shared live verification. One evaluation of every requirement (all lab
// analyses, in Rust) runs in a Web Worker whenever the project changes; every
// component reads the same result, and a newer edit supersedes older runs.
// ---------------------------------------------------------------------------

interface EvalState {
  data: Evaluation | null;
  error: string | null;
  ms: number;
  running: boolean;
}

const useEvalStore = create<EvalState>(() => ({ data: null, error: null, ms: 0, running: false }));

let evalTimer: ReturnType<typeof setTimeout> | null = null;
let inFlight = false;
let queued: Project | null = null;
let started = false;

/** At most one evaluation in flight; while it runs, only the latest edit is queued. */
function runEvaluation(project: Project) {
  if (inFlight) {
    queued = project;
    return;
  }
  inFlight = true;
  useEvalStore.setState({ running: true });
  callInWorker<Evaluation>('systems.evaluate', project).then((r) => {
    inFlight = false;
    useEvalStore.setState({ data: r.data ?? useEvalStore.getState().data, error: r.error, ms: r.ms, running: queued !== null });
    if (queued) {
      const next = queued;
      queued = null;
      runEvaluation(next);
    }
  });
}

function scheduleEvaluation(project: Project) {
  if (evalTimer) clearTimeout(evalTimer);
  evalTimer = setTimeout(() => runEvaluation(project), 30);
}

function startEvaluation() {
  if (started) return;
  started = true;
  scheduleEvaluation(useProject.getState().project);
  useProject.subscribe((s, prev) => {
    if (s.project !== prev.project) scheduleEvaluation(s.project);
  });
}

/** Live verification of every requirement (shared, computed off the main thread). */
export function useEvaluation(): Computed<Evaluation> & { running: boolean } {
  const ready = useEngineReady();
  useEffect(() => startEvaluation(), []);
  const st = useEvalStore();
  return { data: st.data, error: st.error, ms: st.ms, ready, running: st.running };
}

/** Metrics indexed by key. */
export function useMetricMap() {
  const ev = useEvaluation();
  return useMemo(() => {
    const m = new Map<string, Evaluation['metrics'][number]>();
    ev.data?.metrics.forEach((x) => m.set(x.key, x));
    return m;
  }, [ev.data]);
}
