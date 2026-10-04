// Small helpers for deep links into labs and their tabs (`#/<lab>?tab=<id>`).
import { LABS } from '../../labs/registry';
import { LAB_TABS } from './labTabs';

const PAGES: Record<string, { title: string; icon: string }> = {
  systems: { title: 'Systems Studio', icon: 'systems' },
  toolchain: { title: 'Web toolchain', icon: 'toolchain' },
  versions: { title: 'Versions', icon: 'versions' },
  learn: { title: 'Guide & references', icon: 'learn' },
  docs: { title: 'Documentation', icon: 'docs' },
  new: { title: 'New product', icon: 'arrow' },
  start: { title: 'Start here', icon: 'home' },
};

/** Route for a lab (or page) and optional tab, e.g. `/rf?tab=matching`. */
export function tabRoute(lab: string, tab?: string): string {
  const base = LABS.find((l) => l.id === lab)?.path ?? `/${lab}`;
  return tab ? `${base}?tab=${encodeURIComponent(tab)}` : base;
}

export function labTitle(lab: string): string {
  return LABS.find((l) => l.id === lab)?.title ?? PAGES[lab]?.title ?? lab;
}

export function labIcon(lab: string): string {
  return LABS.find((l) => l.id === lab)?.icon ?? PAGES[lab]?.icon ?? 'systems';
}

export function tabLabel(lab: string, tab?: string): string | undefined {
  if (!tab) return undefined;
  return LAB_TABS[lab]?.tabs.find((t) => t.id === tab)?.label;
}

/** "RF & Antenna Lab › Matching" */
export function placeLabel(lab: string, tab?: string): string {
  const t = tabLabel(lab, tab);
  return t ? `${labTitle(lab)} › ${t}` : labTitle(lab);
}

/** Lab id from a pathname such as `/rf` (undefined for non-lab pages). */
export function labFromPath(pathname: string): string | undefined {
  return LABS.find((l) => l.path === pathname)?.id;
}
