// Documentation viewer: every Markdown guide in docs/ rendered inside the
// app (so the GitHub Pages site carries all of them), with a table of
// contents, full-text search and "open the lab" buttons.
import { useEffect, useMemo, useState } from 'react';
import { Link, useParams, useSearchParams } from 'react-router-dom';
import { Card, Chip, DownloadButton, EmptyState } from '../components/ui';
import { Markdown, outline } from './docs/Markdown';
import { DOCS, GROUP_ORDER, REPO_URL, docBySlug, loadAll, type DocInfo } from './docs/sources';
import './docs/docs.css';

function useDocText(doc: DocInfo | undefined) {
  const [state, setState] = useState<{ slug: string; text: string | null; error: string | null }>({ slug: '', text: null, error: null });
  useEffect(() => {
    if (!doc) return;
    let alive = true;
    setState({ slug: doc.slug, text: null, error: null });
    doc.load().then(
      (text) => alive && setState({ slug: doc.slug, text, error: null }),
      (e: Error) => alive && setState({ slug: doc.slug, text: null, error: e.message }),
    );
    return () => {
      alive = false;
    };
  }, [doc]);
  return state.slug === doc?.slug ? state : { slug: '', text: null, error: null };
}

function useAllDocs(enabled: boolean) {
  const [all, setAll] = useState<Map<string, string> | null>(null);
  useEffect(() => {
    if (enabled && !all) loadAll().then(setAll);
  }, [enabled, all]);
  return all;
}

interface Hit { doc: DocInfo; count: number; snippet: string; section: string | null }

function search(all: Map<string, string>, q: string): Hit[] {
  const needle = q.trim().toLowerCase();
  if (needle.length < 2) return [];
  const hits: Hit[] = [];
  for (const d of DOCS) {
    const text = all.get(d.slug) ?? '';
    const lower = text.toLowerCase();
    let count = 0;
    for (let i = lower.indexOf(needle); i >= 0; i = lower.indexOf(needle, i + needle.length)) count++;
    const inTitle = d.title.toLowerCase().includes(needle);
    if (!count && !inTitle) continue;
    const at = lower.indexOf(needle);
    const lineStart = at >= 0 ? text.lastIndexOf('\n', at) + 1 : 0;
    const lineEnd = at >= 0 ? text.indexOf('\n', at) : -1;
    const line = at >= 0 ? text.slice(lineStart, lineEnd < 0 ? undefined : lineEnd) : '';
    const off = at - lineStart;
    const snippet = line.length > 140 ? `${off > 60 ? '…' : ''}${line.slice(Math.max(0, off - 60), off + 80)}…` : line;
    // nearest heading above the first hit
    const before = at >= 0 ? text.slice(0, at).split('\n').reverse().find((l) => /^#{2,3}\s/.test(l)) : undefined;
    const section = before ? outline(before).map((h) => h.id)[0] ?? null : null;
    hits.push({ doc: d, count: count + (inTitle ? 5 : 0), snippet: snippet.replace(/[#*`|>]/g, '').trim(), section });
  }
  return hits.sort((a, b) => b.count - a.count);
}

function DocList({ current }: { current?: string }) {
  return (
    <nav aria-label="Documents">
      {GROUP_ORDER.map((g) => {
        const ds = DOCS.filter((d) => d.group === g);
        if (!ds.length) return null;
        return (
          <div key={g} className="dc-group">
            <div className="nav-group-title" style={{ padding: '0 0 4px' }}>{g}</div>
            {ds.map((d) => (
              <Link key={d.slug} to={`/docs/${d.slug}`} className={`dc-link ${d.slug === current ? 'active' : ''}`} aria-current={d.slug === current ? 'page' : undefined}>{d.title}</Link>
            ))}
          </div>
        );
      })}
    </nav>
  );
}

function Index() {
  const all = useAllDocs(true);
  const intro = (d: DocInfo) => {
    const t = all?.get(d.slug);
    if (!t) return '';
    const para = t.split(/\n\s*\n/).map((p) => p.trim()).find((p) => p && !p.startsWith('#') && !p.startsWith('|') && !p.startsWith('```') && !/^Route:/.test(p));
    return (para ?? '').replace(/\s+/g, ' ').replace(/[*`]/g, '').replace(/\[([^\]]+)\]\([^)]+\)/g, '$1').slice(0, 220);
  };
  return (
    <div className="stack">
      {GROUP_ORDER.map((g) => {
        const ds = DOCS.filter((d) => d.group === g);
        if (!ds.length) return null;
        return (
          <div key={g} className="stack" style={{ gap: 10 }}>
            <h2 className="dc-group-title">{g}</h2>
            <div className="grid grid-3">
              {ds.map((d) => (
                <Card key={d.slug} title={<Link to={`/docs/${d.slug}`}>{d.title}</Link>} subtitle={<span className="mono">docs/{d.path}</span>}>
                  <div className="small" style={{ color: 'var(--text-2)', minHeight: 36 }}>{intro(d) || <span className="muted">…</span>}{intro(d).length >= 220 ? '…' : ''}</div>
                  <div className="row" style={{ marginTop: 10 }}>
                    <Link className="btn small primary" to={`/docs/${d.slug}`}>Read</Link>
                    {d.open && <a className="btn small" href={d.open.href}>{d.open.label} →</a>}
                  </div>
                </Card>
              ))}
            </div>
          </div>
        );
      })}
    </div>
  );
}

export default function Docs() {
  const { slug } = useParams();
  const [params] = useSearchParams();
  const doc = docBySlug(slug);
  const { text, error } = useDocText(doc);
  const [q, setQ] = useState(params.get('q') ?? '');
  const all = useAllDocs(q.trim().length >= 2);
  const hits = useMemo(() => (all ? search(all, q) : []), [all, q]);
  const toc = useMemo(() => (text ? outline(text) : []), [text]);
  const section = params.get('s');
  const hl = params.get('q') ?? '';
  const idx = doc ? DOCS.indexOf(doc) : -1;

  useEffect(() => {
    if (!text) return;
    const t = setTimeout(() => {
      const el = section ? document.getElementById(`md-${section}`) : hl ? document.querySelector('.md-doc mark') : null;
      if (el) el.scrollIntoView({ block: 'start', behavior: 'smooth' });
    }, 60);
    return () => clearTimeout(t);
  }, [text, section, hl]);

  const h1 = text ? /^\s*#\s+(.+)$/m.exec(text.split('\n').find((l) => l.trim() !== '') ?? '') : null;
  const title = doc ? (h1 ? h1[1].replace(/[`*]/g, '') : doc.title) : slug ? 'Not found' : 'Documentation';
  const body = text && h1 ? text.replace(/^\s*#\s+[^\n]*\n/, '') : text;
  const [narrow] = useState(() => typeof window !== 'undefined' && window.matchMedia?.('(max-width: 860px)').matches);
  return (
    <div className="stack">
      <div className="page-head">
        <div style={{ flex: 1, minWidth: 260 }}>
          <div className="page-kicker">Documentation{doc ? ` · ${doc.group}` : ''}</div>
          <h1 style={{ marginTop: 4 }}>{title}</h1>
          <div className="page-sub">
            {doc ? <>From <span className="mono">docs/{doc.path}</span> — the same Markdown that is on GitHub, rendered here so every guide lives on the Pages site.</> : <>Every guide in the repository's <span className="mono">docs/</span> folder: getting started, the architecture, one guide per lab, the Systems Studio, the web toolchain, versions, references and how to build a lab. {DOCS.length} documents.</>}
          </div>
        </div>
        {doc && (
          <div className="row">
            {doc.open && <a className="btn primary" href={doc.open.href}>{doc.open.label} →</a>}
            <a className="btn" href={`${REPO_URL}/blob/main/docs/${doc.path}`} target="_blank" rel="noreferrer">View on GitHub ↗</a>
            {text && <DownloadButton filename={doc.path.split('/').pop()!} content={text} label=".md" />}
          </div>
        )}
      </div>
      <div className="dc-layout">
        <aside className="dc-side">
          <label className="field" style={{ marginBottom: 10 }}>
            <span className="field-top"><span className="field-label">Search all docs</span></span>
            <input type="text" value={q} onChange={(e) => setQ(e.target.value)} placeholder="e.g. Gerber, eyebox, PathFinder" aria-label="Search all docs" />
          </label>
          {q.trim().length >= 2 ? (
            <div className="dc-hits">
              {!all && <div className="small muted">Loading documents…</div>}
              {all && hits.length === 0 && <div className="small muted">No match for “{q}”.</div>}
              {hits.map((h) => (
                <Link key={h.doc.slug} className="dc-hit" to={`/docs/${h.doc.slug}?q=${encodeURIComponent(q.trim())}${h.section ? `&s=${encodeURIComponent(h.section)}` : ''}`}>
                  <div className="row" style={{ justifyContent: 'space-between' }}><b>{h.doc.title}</b><Chip>{h.count}</Chip></div>
                  <div className="small muted">{h.snippet}</div>
                </Link>
              ))}
            </div>
          ) : (
            <details className="dc-toc" open={!narrow || !doc}>
              <summary className="small muted">All documents</summary>
              <DocList current={doc?.slug} />
            </details>
          )}
          {toc.length > 0 && (
            <div className="dc-outline">
              <div className="nav-group-title" style={{ padding: '10px 0 4px' }}>On this page</div>
              {toc.map((h) => <Link key={h.id} to={`/docs/${doc!.slug}?s=${encodeURIComponent(h.id)}`} className={`dc-link dc-l${h.level} ${section === h.id ? 'active' : ''}`}>{h.text}</Link>)}
            </div>
          )}
        </aside>
        <main className="dc-main" aria-live="polite">
          {!slug && <Index />}
          {slug && !doc && <EmptyState>No document “{slug}”. <Link to="/docs">All documents</Link></EmptyState>}
          {error && <div className="callout">{error}</div>}
          {doc && !text && !error && <div className="muted small">Loading…</div>}
          {doc && text && (
            <Card className="dc-card">
              <Markdown src={body ?? ''} doc={doc.path} highlight={hl} />
              <div className="row dc-nav">
                {idx > 0 && <Link className="btn small" to={`/docs/${DOCS[idx - 1].slug}`}>← {DOCS[idx - 1].title}</Link>}
                <span style={{ flex: 1 }} />
                {idx >= 0 && idx < DOCS.length - 1 && <Link className="btn small" to={`/docs/${DOCS[idx + 1].slug}`}>{DOCS[idx + 1].title} →</Link>}
              </div>
            </Card>
          )}
        </main>
      </div>
    </div>
  );
}
