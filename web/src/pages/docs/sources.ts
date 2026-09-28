// Every Markdown file in docs/ (loaded lazily at build time by Vite) and the
// images in docs/images, plus titles, groups and "open the lab" targets.
import { LABS } from '../../labs/registry';

export const REPO_URL = 'https://github.com/Normansrule/protoforge';

const RAW = import.meta.glob('../../../../docs/**/*.md', { query: '?raw', import: 'default' }) as Record<string, () => Promise<string>>;
const IMAGES = import.meta.glob('../../../../docs/images/*.{webp,png,jpg,jpeg,gif,svg}', { query: '?url', import: 'default', eager: true }) as Record<string, string>;

export interface DocInfo {
  /** Path inside docs/, e.g. `labs/pcb.md`. */
  path: string;
  slug: string;
  title: string;
  group: string;
  order: number;
  /** In-app route of the page this guide describes. */
  open?: { href: string; label: string };
  load: () => Promise<string>;
}

const KNOWN: Record<string, { title: string; group: string; order: number; open?: { href: string; label: string } }> = {
  'getting-started.md': { title: 'Getting started', group: 'Start here', order: 0 },
  'architecture.md': { title: 'Architecture', group: 'Start here', order: 1 },
  'systems-studio.md': { title: 'Systems Studio', group: 'Guides', order: 10, open: { href: '#/systems', label: 'Open Systems Studio' } },
  'toolchain.md': { title: 'Web toolchain', group: 'Guides', order: 11, open: { href: '#/toolchain', label: 'Open the web toolchain' } },
  'versions.md': { title: 'Versions', group: 'Guides', order: 12, open: { href: '#/versions', label: 'Open Versions' } },
  'references.md': { title: 'References', group: 'Reference', order: 40 },
  'reference-repos.md': { title: 'Reference repositories', group: 'Reference', order: 41 },
  'CONTRIBUTING-LABS.md': { title: 'Building a lab', group: 'Contributing', order: 50 },
};

export const GROUP_ORDER = ['Start here', 'Guides', 'Lab guides', 'Reference', 'Contributing', 'More'];

function titleCase(s: string) {
  return s.replace(/[-_]+/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
}

export function slugOf(path: string): string {
  return path.replace(/\.md$/i, '').toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}

export const DOCS: DocInfo[] = Object.entries(RAW)
  .map(([key, load]) => {
    const path = key.replace(/^(\.\.\/)+docs\//, '');
    const k = KNOWN[path];
    let info: Omit<DocInfo, 'path' | 'slug' | 'load'>;
    const lab = /^labs\/([^/]+)\.md$/.exec(path);
    if (k) info = k;
    else if (lab) {
      const l = LABS.find((x) => x.id === lab[1]);
      info = { title: l ? l.title : titleCase(lab[1]), group: 'Lab guides', order: 20 + (l ? LABS.indexOf(l) : 50), open: l ? { href: `#${l.path}`, label: `Open ${l.title}` } : undefined };
    } else info = { title: titleCase(path.replace(/\.md$/, '').split('/').pop() ?? path), group: 'More', order: 90 };
    return { path, slug: slugOf(path), load, ...info };
  })
  .sort((a, b) => a.order - b.order || a.title.localeCompare(b.title));

export const docBySlug = (slug: string | undefined) => DOCS.find((d) => d.slug === slug);

/** Resolve a relative path against a doc's folder (`labs/pcb.md` + `../x.md` → `x.md`; may leave docs/ as `../…`). */
export function resolve(fromDoc: string, rel: string): string {
  const parts = fromDoc.split('/').slice(0, -1);
  for (const seg of rel.split('/')) {
    if (seg === '' || seg === '.') continue;
    if (seg === '..') {
      if (parts.length && parts[parts.length - 1] !== '..') parts.pop();
      else parts.push('..');
    } else parts.push(seg);
  }
  return parts.join('/');
}

/** Link target for an href inside a doc: in-app route, image URL or external URL. */
export function linkTarget(fromDoc: string, href: string): { href: string; external: boolean } {
  if (/^[a-z]+:/i.test(href)) return { href, external: true };
  if (href.startsWith('#/')) return { href, external: false };
  const [pathPart, anchor] = href.split('#');
  if (!pathPart) return { href: `#/docs/${slugOf(fromDoc)}${anchor ? `?s=${encodeURIComponent(anchor)}` : ''}`, external: false };
  const p = resolve(fromDoc, pathPart);
  if (!p.startsWith('..')) {
    const d = DOCS.find((x) => x.path === p || x.path === `${p.replace(/\/$/, '')}.md`);
    if (d) return { href: `#/docs/${d.slug}${anchor ? `?s=${encodeURIComponent(anchor)}` : ''}`, external: false };
    if (/\/$/.test(p) || !/\.[a-z0-9]+$/i.test(p)) {
      // a folder such as labs/ → the docs index
      return { href: '#/docs', external: false };
    }
    return { href: `${REPO_URL}/blob/main/docs/${p}`, external: true };
  }
  const outside = p.replace(/^(\.\.\/)+/, '');
  return { href: `${REPO_URL}/blob/main/${outside}${anchor ? `#${anchor}` : ''}`, external: true };
}

/** URL of an image referenced from a doc (bundled from docs/images, else GitHub raw). */
export function imageUrl(fromDoc: string, src: string): string {
  if (/^[a-z]+:/i.test(src)) return src;
  const p = resolve(fromDoc, src);
  const key = Object.keys(IMAGES).find((k) => k.endsWith(`/docs/${p}`));
  if (key) return IMAGES[key];
  return `https://raw.githubusercontent.com/Normansrule/protoforge/main/${p.startsWith('..') ? p.replace(/^(\.\.\/)+/, '') : `docs/${p}`}`;
}

let all: Promise<Map<string, string>> | null = null;
/** Load every doc (for search and the index). */
export function loadAll(): Promise<Map<string, string>> {
  if (!all) all = Promise.all(DOCS.map(async (d) => [d.slug, await d.load()] as const)).then((e) => new Map(e));
  return all;
}
