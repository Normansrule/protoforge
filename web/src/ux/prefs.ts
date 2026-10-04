// Per-viewer ease-of-use preferences: chosen discipline, Simple/Expert mode,
// welcome card, first-win quest progress, recent palette items and which lab
// help panels were already shown. Persisted in localStorage when available;
// every access is wrapped so the app works in private windows and previews.
import { create } from 'zustand';

export type Mode = 'simple' | 'expert';

export interface RecentItem {
  id: string;
  label: string;
  hint?: string;
  href?: string;
  at: number;
}

export interface Prefs {
  role: string | null;
  mode: Mode;
  welcomeDismissed: boolean;
  /** Quest step ids ticked, per role. */
  quests: Record<string, string[]>;
  recent: RecentItem[];
  /** Lab ids whose help panel was already opened automatically. */
  helpSeen: string[];
  /** "All labs" group expanded in the Simple-mode sidebar. */
  allLabsOpen: boolean;
}

interface PrefsState extends Prefs {
  setRole: (role: string | null) => void;
  setMode: (mode: Mode) => void;
  toggleMode: () => void;
  dismissWelcome: () => void;
  toggleQuest: (role: string, step: string) => void;
  resetQuest: (role: string) => void;
  pushRecent: (item: Omit<RecentItem, 'at'>) => void;
  markHelpSeen: (lab: string) => void;
  setAllLabsOpen: (open: boolean) => void;
}

const KEY = 'protoforge.prefs.v1';
const PROJECT_KEY = 'protoforge.project.v1';

function storageGet(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function storageSet(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* storage blocked: preferences live in memory for this visit */
  }
}

function defaults(): Prefs {
  // A first visit starts in Simple mode; someone who already has a saved
  // project from before this feature existed keeps the full sidebar they know.
  const returning = storageGet(PROJECT_KEY) !== null;
  return { role: null, mode: returning ? 'expert' : 'simple', welcomeDismissed: false, quests: {}, recent: [], helpSeen: [], allLabsOpen: false };
}

function load(): Prefs {
  const base = defaults();
  const raw = storageGet(KEY);
  if (!raw) return base;
  try {
    const p = JSON.parse(raw) as Partial<Prefs>;
    return {
      role: typeof p.role === 'string' ? p.role : null,
      mode: p.mode === 'expert' || p.mode === 'simple' ? p.mode : base.mode,
      welcomeDismissed: !!p.welcomeDismissed,
      quests: p.quests && typeof p.quests === 'object' ? p.quests : {},
      recent: Array.isArray(p.recent) ? p.recent.filter((r) => r && typeof r.id === 'string').slice(0, 8) : [],
      helpSeen: Array.isArray(p.helpSeen) ? p.helpSeen.filter((x) => typeof x === 'string') : [],
      allLabsOpen: !!p.allLabsOpen,
    };
  } catch {
    return base;
  }
}

function persist(s: Prefs) {
  const { role, mode, welcomeDismissed, quests, recent, helpSeen, allLabsOpen } = s;
  storageSet(KEY, JSON.stringify({ role, mode, welcomeDismissed, quests, recent, helpSeen, allLabsOpen }));
}

export const usePrefs = create<PrefsState>((set, get) => {
  const apply = (patch: Partial<Prefs>) => {
    set(patch);
    persist(get());
  };
  return {
    ...load(),
    setRole: (role) => apply({ role }),
    setMode: (mode) => apply({ mode }),
    toggleMode: () => apply({ mode: get().mode === 'simple' ? 'expert' : 'simple' }),
    dismissWelcome: () => apply({ welcomeDismissed: true }),
    toggleQuest: (role, step) => {
      const done = new Set(get().quests[role] ?? []);
      if (done.has(step)) done.delete(step);
      else done.add(step);
      apply({ quests: { ...get().quests, [role]: [...done] } });
    },
    resetQuest: (role) => {
      const q = { ...get().quests };
      delete q[role];
      apply({ quests: q });
    },
    pushRecent: (item) => apply({ recent: [{ ...item, at: Date.now() }, ...get().recent.filter((r) => r.id !== item.id)].slice(0, 8) }),
    markHelpSeen: (lab) => {
      if (!get().helpSeen.includes(lab)) apply({ helpSeen: [...get().helpSeen, lab] });
    },
    setAllLabsOpen: (allLabsOpen) => apply({ allLabsOpen }),
  };
});

// ---------------------------------------------------------------------------
// Theme (shared by the top bar and the command palette). Same storage key as
// before so main.tsx applies it before first paint.
// ---------------------------------------------------------------------------

export type Theme = 'dark' | 'light';
const THEME_KEY = 'protoforge.theme';

function loadTheme(): Theme {
  return storageGet(THEME_KEY) === 'light' ? 'light' : 'dark';
}

export const useTheme = create<{ theme: Theme; setTheme: (t: Theme) => void; toggle: () => void }>((set, get) => ({
  theme: loadTheme(),
  setTheme: (theme) => {
    document.documentElement.dataset.theme = theme;
    storageSet(THEME_KEY, theme);
    set({ theme });
  },
  toggle: () => get().setTheme(get().theme === 'dark' ? 'light' : 'dark'),
}));

/** Custom window event that opens the command palette (top-bar button, shortcuts, other pages). */
export const PALETTE_EVENT = 'protoforge:palette';

export function openPalette(query?: string) {
  window.dispatchEvent(new CustomEvent(PALETTE_EVENT, { detail: { query } }));
}
