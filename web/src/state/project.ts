// Global project store. The whole product lives in one JSON document
// (see examples/smart-glasses/project.json); every lab reads from it and the
// Systems Studio verifies requirements against it.
import { create } from 'zustand';
import { useMemo } from 'react';
import iris from '../../../examples/smart-glasses/project.json';
import type { Design, Evaluation, Project } from '../engine/types';
import { useCompute } from '../engine/hooks';

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

/** Live verification of every requirement (runs all analyses in Rust). */
export function useEvaluation() {
  const project = useProject((s) => s.project);
  return useCompute<Evaluation>('systems.evaluate', project);
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
