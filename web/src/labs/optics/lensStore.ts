// Shared state for the Optics Pro tabs (Lens designer, Image quality): the
// lens system being edited survives switching tabs.
import { create } from 'zustand';
import type { LensSystem, Variable } from './proTypes';

interface LensState {
  /** Preset the editor started from ('' until the presets load). */
  presetId: string;
  system: LensSystem | null;
  variables: Variable[];
  /** true once the user edited the table (the system no longer equals the preset). */
  edited: boolean;
  load: (presetId: string, system: LensSystem, variables: Variable[]) => void;
  setSystem: (system: LensSystem, edited?: boolean) => void;
  setVariables: (v: Variable[]) => void;
}

export const useLens = create<LensState>((set) => ({
  presetId: '',
  system: null,
  variables: [],
  edited: false,
  load: (presetId, system, variables) => set({ presetId, system, variables, edited: false }),
  setSystem: (system, edited = true) => set({ system, edited }),
  setVariables: (variables) => set({ variables }),
}));

/** Index of the first lens surface in the reversed-HUD systems (after the eye pupil and the unfolded combiner). */
export const LENS_START = 2;
