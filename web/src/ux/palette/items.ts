// Everything the command palette can find, built once per project/evaluation
// change: pages, labs and their tabs, requirements, metrics, glossary terms,
// docs, "I want to…" tasks and actions.
import { LABS } from '../../labs/registry';
import { DOCS } from '../../pages/docs/sources';
import { fmtNum } from '../../components/Plot';
import type { Evaluation, Project } from '../../engine/types';
import { LAB_TABS } from '../data/labTabs';
import { TASKS } from '../data/tasks';
import { ALL_TERMS } from '../data/terms';
import { ROLES } from '../data/roles';
import { tabRoute } from '../data/links';
import { scoreFields } from './fuzzy';

export type Category = 'Labs & tabs' | 'Pages' | 'Requirements' | 'Metrics' | 'I want to…' | 'Glossary' | 'Docs' | 'Actions' | 'Recent';

export const CATEGORY_ORDER: Category[] = ['Recent', 'Labs & tabs', 'Pages', 'Requirements', 'Metrics', 'I want to…', 'Glossary', 'Docs', 'Actions'];

const CATEGORY_WEIGHT: Record<Category, number> = {
  Recent: 1,
  'Labs & tabs': 1,
  Pages: 0.97,
  Requirements: 0.95,
  Metrics: 0.86,
  'I want to…': 0.9,
  Glossary: 0.8,
  Docs: 0.78,
  Actions: 0.9,
};

export type ActionId = 'theme' | 'mode' | 'role' | 'save' | 'new' | 'reset';

export interface PItem {
  id: string;
  cat: Category;
  title: string;
  sub?: string;
  /** Extra searchable words (not shown). */
  keys?: string;
  icon?: string;
  href?: string;
  action?: ActionId;
  badge?: { text: string; tone?: 'pass' | 'fail' | 'warn' | 'info' };
  /** Shown under the title (glossary definitions). */
  detail?: string;
  /** Parent-lab ordering for tabs (keeps a lab's tabs right after it). */
  parent?: string;
  order?: number;
}

export interface Scored extends PItem {
  score: number;
}

const LAB_KEYWORDS: Record<string, string> = {
  circuit: 'spice electronics schematic simulator ripple',
  power: 'battery energy budget thermal heat skin',
  pcb: 'printed circuit board layout routing gerber kicad',
  rf: 'radio frequency antenna bluetooth wireless smith link budget',
  dsp: 'digital signal processing audio filter fft microphone',
  chip: 'verilog logic asic fpga tapeout',
  control: 'pid lqr kalman imu feedback',
  robotics: 'arm scara pick and place kinematics',
  rl: 'reinforcement learning q-learning policy',
  optics: 'lens display hud ray trace mtf',
  cad: 'frame model stl 3d geometry enclosure',
  print: '3d printing slicer gcode fdm bambu',
  structures: 'fea stress strength fatigue tolerance',
  mechanisms: 'linkage spring gear cam hinge',
  fluids: 'heat conduction convection cfd wind tunnel',
  ergonomics: 'human factors fit comfort anthropometry',
  molding: 'injection mould draft',
  build: 'settings process vendor compliance reflow feeds speeds',
  mfg: 'manufacturing cost quantity vendors package',
  reliability: 'weibull mtbf life test dvt',
  program: 'schedule gantt risk business margin trl',
  proto: 'firmware breadboard prototype ble hud',
  quantum: 'qubit qkd qrng entanglement qiskit',
};

/** Lab (or page) whose route shows a metric. */
export function metricHome(key: string): string {
  const lab = LABS.find((l) => l.metrics.some((p) => key.startsWith(p)));
  if (lab) return lab.path;
  if (key.startsWith('firmware.')) return '/proto';
  if (key.startsWith('budget.') || key.startsWith('bom.')) return '/systems?tab=budgets';
  return '/systems?tab=requirements';
}

export function buildItems(project: Project, ev: Evaluation | null, opts: { mode: 'simple' | 'expert'; theme: 'dark' | 'light' }): PItem[] {
  const items: PItem[] = [];
  const pages: [string, string, string, string?][] = [
    ['Overview', '/', 'home', 'home dashboard'],
    ['Start here', '/start', 'arrow', 'discipline role onboarding path'],
    ['New product', '/new', 'arrow', 'wizard describe idea create'],
    ['Systems Studio', '/systems', 'systems', 'requirements needs trades risks'],
    ['Versions', '/versions', 'versions', 'history compare merge'],
    ['Web toolchain', '/toolchain', 'toolchain', 'kicad onshape step export falstad'],
    ['Guide & references', '/learn', 'learn', 'glossary references repositories resources'],
    ['Documentation', '/docs', 'docs', 'docs guides'],
  ];
  for (const [title, href, icon, keys] of pages) items.push({ id: `page:${href}`, cat: 'Pages', title, href, icon, keys });

  // Labs, then each lab's tabs (also the Systems Studio and toolchain tabs).
  const labLike = [...LABS.map((l) => ({ id: l.id, title: l.title, icon: l.icon, sub: l.blurb })), { id: 'systems', title: 'Systems Studio', icon: 'systems', sub: '' }, { id: 'toolchain', title: 'Web toolchain', icon: 'toolchain', sub: '' }];
  for (const l of labLike) {
    const isLab = LABS.some((x) => x.id === l.id);
    if (isLab) items.push({ id: `lab:${l.id}`, cat: 'Labs & tabs', title: l.title, sub: l.sub, keys: LAB_KEYWORDS[l.id], icon: l.icon, href: tabRoute(l.id), parent: l.id, order: 0 });
    (LAB_TABS[l.id]?.tabs ?? []).forEach((t, i) =>
      items.push({ id: `tab:${l.id}:${t.id}`, cat: 'Labs & tabs', title: t.label, sub: l.title, keys: isLab ? LAB_KEYWORDS[l.id] : undefined, icon: l.icon, href: tabRoute(l.id, t.id), parent: l.id, order: i + 1 }),
    );
  }

  // Requirements with live status.
  for (const r of project.requirements) {
    const res = ev?.results.find((x) => x.id === r.id);
    const badge = res
      ? { text: `${res.status === 'pass' ? '✓' : res.status === 'fail' ? (r.kind === 'goal' ? '!' : '✕') : '○'} ${res.value != null ? `${fmtNum(res.value)} ${res.unit}` : res.status}`, tone: res.status === 'pass' ? ('pass' as const) : res.status === 'fail' ? (r.kind === 'goal' ? ('warn' as const) : ('fail' as const)) : undefined }
      : undefined;
    items.push({ id: `req:${r.id}`, cat: 'Requirements', title: r.id, sub: r.text, keys: `${r.verification.metric ?? ''} ${r.kind}`, icon: 'check', href: `/systems?req=${r.id}`, badge });
  }

  // Metrics with live values.
  for (const m of ev?.metrics ?? []) {
    items.push({ id: `metric:${m.key}`, cat: 'Metrics', title: m.label, sub: m.key, icon: 'rl', href: metricHome(m.key), badge: { text: `${fmtNum(m.value)} ${m.unit}`.trim(), tone: 'info' } });
  }

  // Glossary (shared + supplement).
  const seen = new Set<string>();
  for (const t of ALL_TERMS) {
    if (seen.has(t.term)) continue;
    seen.add(t.term);
    items.push({ id: `term:${t.term}`, cat: 'Glossary', title: t.term, sub: t.full, detail: t.def, icon: 'learn', href: t.lab ?? '/learn' });
  }

  for (const d of DOCS) items.push({ id: `doc:${d.slug}`, cat: 'Docs', title: d.title, sub: d.group, icon: 'docs', href: `/docs/${d.slug}` });

  for (const t of TASKS) {
    const role = ROLES.find((r) => r.id === t.role);
    items.push({ id: `task:${t.id}`, cat: 'I want to…', title: `I want to ${t.title}`, sub: t.how, keys: `${t.keywords ?? ''} ${role?.short ?? ''}`, icon: 'arrow', href: tabRoute(t.lab, t.tab) });
  }

  const actions: [ActionId, string, string, string][] = [
    ['theme', opts.theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme', opts.theme === 'dark' ? 'sun' : 'moon', 'theme dark light toggle colour'],
    ['mode', opts.mode === 'simple' ? 'Switch to Expert mode' : 'Switch to Simple mode', 'systems', 'simple expert mode sidebar toggle'],
    ['role', 'Choose your discipline', 'home', 'role discipline engineer start path'],
    ['save', 'Save project.json', 'download', 'save download export json file'],
    ['new', 'New product', 'arrow', 'new create wizard describe'],
    ['reset', 'Reset to the Iris example', 'reset', 'reset iris default clear'],
  ];
  for (const [action, title, icon, keys] of actions) items.push({ id: `action:${action}`, cat: 'Actions', title, keys, icon, action });
  return items;
}

/** Score, filter and group. Tabs of a matching lab follow it directly. */
export function search(items: PItem[], query: string, perCategory = 7): { cat: Category; items: Scored[] }[] {
  const scored: Scored[] = [];
  const labScore = new Map<string, number>();
  for (const it of items) {
    if (it.id.startsWith('lab:')) {
      const s = scoreFields(query, [{ text: it.title, w: 1 }, { text: it.keys, w: 0.62 }]);
      labScore.set(it.parent!, s);
    }
  }
  for (const it of items) {
    let s: number;
    if (it.cat === 'Labs & tabs') {
      const own = scoreFields(query, [{ text: it.title, w: 1 }, { text: `${it.sub ?? ''} ${it.title}`, w: 0.9 }]);
      const lab = labScore.get(it.parent ?? '') ?? 0;
      s = it.id.startsWith('lab:') ? lab : Math.max(own * 0.98, lab * 0.97 - (it.order ?? 0) * 0.2);
    } else if (it.cat === 'Requirements') {
      s = scoreFields(query, [{ text: it.title, w: 1 }, { text: it.sub, w: 0.7 }, { text: it.keys, w: 0.6 }]);
    } else {
      s = scoreFields(query, [{ text: it.title, w: 1 }, { text: it.sub, w: it.cat === 'Glossary' ? 0.85 : 0.55 }, { text: it.keys, w: 0.6 }]);
    }
    s *= CATEGORY_WEIGHT[it.cat];
    if (s > 0) scored.push({ ...it, score: s });
  }
  const groups = new Map<Category, Scored[]>();
  for (const s of scored) {
    const g = groups.get(s.cat) ?? [];
    g.push(s);
    groups.set(s.cat, g);
  }
  const out = [...groups.entries()].map(([cat, list]) => ({
    cat,
    items: list.sort((a, b) => b.score - a.score || (a.order ?? 0) - (b.order ?? 0)).slice(0, perCategory),
  }));
  out.sort((a, b) => b.items[0].score - a.items[0].score || CATEGORY_ORDER.indexOf(a.cat) - CATEGORY_ORDER.indexOf(b.cat));
  return out;
}
