// How a requirement result is shown. A failing requirement of kind "goal" is an
// open (stretch) goal, shown in amber with "!" — only a binding requirement that
// fails is a red "✕ fail". Every status view uses these so the words, icons and
// colours agree across the top bar, sidebar, Overview, Systems Studio and labs.
import type { Project, ReqResult, Status } from '../engine/types';

export type Shown = Status | 'goal';

export const SHOWN: Record<Shown, { icon: string; word: string; tone: 'pass' | 'fail' | 'warn' | undefined; color: string }> = {
  pass: { icon: '✓', word: 'pass', tone: 'pass', color: 'var(--good)' },
  fail: { icon: '✕', word: 'fail', tone: 'fail', color: 'var(--bad)' },
  goal: { icon: '!', word: 'open goal', tone: 'warn', color: 'var(--warn)' },
  unverified: { icon: '○', word: 'unverified', tone: undefined, color: 'var(--muted)' },
};

/** Ids of the project's requirements of kind "goal". */
export function goalIds(project: Project): Set<string> {
  return new Set(project.requirements.filter((r) => r.kind === 'goal').map((r) => r.id));
}

/** The status to show for a result: a failing goal reads as an open goal. */
export function shownStatus(res: Pick<ReqResult, 'id' | 'status'> | undefined | null, goals: Set<string>): Shown {
  if (!res) return 'unverified';
  return res.status === 'fail' && goals.has(res.id) ? 'goal' : res.status;
}

/** Counts of pass / binding fail / open goal / unverified. */
export function countShown(results: Pick<ReqResult, 'id' | 'status'>[], goals: Set<string>) {
  const c = { pass: 0, fail: 0, goal: 0, unverified: 0 };
  for (const r of results) c[shownStatus(r, goals)]++;
  return c;
}

/** "1 open goal", "3 open goals". */
export function plural(n: number, one: string, many = `${one}s`): string {
  return `${n.toLocaleString('en-US')} ${n === 1 ? one : many}`;
}
