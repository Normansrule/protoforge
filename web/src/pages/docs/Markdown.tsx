// A small GitHub-flavoured Markdown renderer for the docs/ guides: headings
// with anchors, nested lists, tables with alignment, fenced code with copy,
// blockquotes, images, links (internal docs links rewritten to #/docs/…),
// inline code / bold / italic / strikethrough, autolinks and Mermaid fences
// shown as code with a note. Everything renders as React text — no HTML
// injection.
import { useState, type ReactNode } from 'react';
import { imageUrl, linkTarget, slugOf } from './sources';
import { FlowDiagram, parseFlow } from './Mermaid';

export interface Heading { level: number; text: string; id: string }

export function headingId(text: string, seen: Map<string, number>): string {
  const base = text
    .toLowerCase()
    .replace(/`/g, '')
    .replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/[^\p{L}\p{N}\s-]/gu, '')
    .trim()
    .replace(/\s/g, '-');
  const n = seen.get(base) ?? 0;
  seen.set(base, n + 1);
  return n ? `${base}-${n}` : base;
}

/** h2/h3 outline of a document (for the table of contents). */
export function outline(src: string): Heading[] {
  const seen = new Map<string, number>();
  const out: Heading[] = [];
  let fence = false;
  for (const l of src.split('\n')) {
    if (/^\s*(```|~~~)/.test(l)) fence = !fence;
    if (fence) continue;
    const m = /^(#{1,6})\s+(.*?)\s*#*\s*$/.exec(l);
    if (m) {
      const text = m[2];
      const id = headingId(text, seen);
      if (m[1].length >= 2 && m[1].length <= 3) out.push({ level: m[1].length, text: text.replace(/`/g, ''), id });
    }
  }
  return out;
}

interface Ctx { doc: string; hl: string }

function mark(s: string, hl: string, key: string): ReactNode[] {
  if (!hl || hl.length < 2) return [s];
  const out: ReactNode[] = [];
  const lower = s.toLowerCase();
  const q = hl.toLowerCase();
  let i = 0;
  let k = 0;
  for (let j = lower.indexOf(q); j >= 0; j = lower.indexOf(q, i)) {
    if (j > i) out.push(s.slice(i, j));
    out.push(<mark key={`${key}m${k++}`}>{s.slice(j, j + q.length)}</mark>);
    i = j + q.length;
  }
  if (i < s.length) out.push(s.slice(i));
  return out;
}

const INLINE = /(\\[\\`*_{}[\]()#+\-.!~|<>])|(`+)([\s\S]*?[^`])\2(?!`)|!\[([^\]]*)\]\(([^)\s]+)(?:\s+"[^"]*")?\)|\[((?:[^\]\\]|\\.)*)\]\(([^)\s]+)(?:\s+"[^"]*")?\)|<(https?:\/\/[^>\s]+)>|(\*\*|__)(?=\S)([\s\S]*?\S)\9|~~(?=\S)([\s\S]*?\S)~~|(\*|_)(?=\S)([\s\S]*?\S)\12(?![\w*])|(https?:\/\/[^\s<>()]+[^\s<>().,;:!?'"])/g;

export function inline(s: string, ctx: Ctx, key: string): ReactNode[] {
  const out: ReactNode[] = [];
  let last = 0;
  let i = 0;
  INLINE.lastIndex = 0;
  const re = new RegExp(INLINE.source, 'g');
  let m: RegExpExecArray | null;
  while ((m = re.exec(s))) {
    // italic with _ only at word boundaries (snake_case stays text)
    if (m[12] === '_' && m.index > 0 && /\w/.test(s[m.index - 1])) continue;
    if (m.index > last) out.push(...mark(s.slice(last, m.index), ctx.hl, `${key}t${i}`));
    const k = `${key}-${i++}`;
    if (m[1]) out.push(m[1].slice(1));
    else if (m[2]) out.push(<code key={k} className="mono">{m[3].replace(/^ (.*) $/, '$1')}</code>);
    else if (m[5] !== undefined) out.push(<img key={k} src={imageUrl(ctx.doc, m[5])} alt={m[4]} loading="lazy" className="md-img" />);
    else if (m[7] !== undefined) {
      const t = linkTarget(ctx.doc, m[7]);
      out.push(<a key={k} href={t.href} {...(t.external ? { target: '_blank', rel: 'noreferrer' } : {})}>{inline(m[6], ctx, k)}{t.external ? ' ↗' : ''}</a>);
    } else if (m[8]) out.push(<a key={k} href={m[8]} target="_blank" rel="noreferrer">{m[8]}</a>);
    else if (m[9]) out.push(<b key={k}>{inline(m[10], ctx, k)}</b>);
    else if (m[11] !== undefined) out.push(<s key={k}>{inline(m[11], ctx, k)}</s>);
    else if (m[12]) out.push(<i key={k}>{inline(m[13], ctx, k)}</i>);
    else if (m[14]) out.push(<a key={k} href={m[14]} target="_blank" rel="noreferrer">{m[14]}</a>);
    last = m.index + m[0].length;
  }
  if (last < s.length) out.push(...mark(s.slice(last), ctx.hl, `${key}end`));
  return out;
}

function CodeBlock({ code, lang }: { code: string; lang: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="md-code">
      <div className="md-code-bar">
        <span className="mono small muted">{lang || 'text'}</span>
        <button className="btn small" onClick={() => navigator.clipboard?.writeText(code).then(() => { setCopied(true); setTimeout(() => setCopied(false), 1200); }).catch(() => {})}>{copied ? 'Copied' : 'Copy'}</button>
      </div>
      <pre className="code">{code}</pre>
    </div>
  );
}

const LIST_RE = /^(\s*)([-*+]|\d+[.)])\s+(.*)$/;
const isBlockStart = (l: string) => /^\s*(```|~~~)/.test(l) || /^#{1,6}\s/.test(l) || /^\s*>/.test(l) || LIST_RE.test(l) || /^\s*\|/.test(l) || /^\s*([-*_])(\s*\1){2,}\s*$/.test(l);

function splitRow(l: string): string[] {
  const t = l.trim().replace(/^\|/, '').replace(/\|$/, '');
  const cells: string[] = [];
  let cur = '';
  let code = false;
  for (let i = 0; i < t.length; i++) {
    const c = t[i];
    if (c === '`') code = !code;
    if (c === '\\' && t[i + 1] === '|') {
      cur += '|';
      i++;
      continue;
    }
    if (c === '|' && !code) {
      cells.push(cur.trim());
      cur = '';
    } else cur += c;
  }
  cells.push(cur.trim());
  return cells;
}

interface ListItem { text: string[]; children: string[] }

function renderList(lines: string[], ctx: Ctx, key: string, seen: Map<string, number>): ReactNode {
  const first = LIST_RE.exec(lines[0])!;
  const indent = first[1].length;
  const ordered = /\d/.test(first[2]);
  const start = ordered ? parseInt(first[2], 10) : 1;
  const items: ListItem[] = [];
  for (const l of lines) {
    const m = LIST_RE.exec(l);
    if (m && m[1].length <= indent + 1) items.push({ text: [m[3]], children: [] });
    else if (items.length) {
      const it = items[items.length - 1];
      if (it.children.length || LIST_RE.test(l)) it.children.push(l);
      else it.text.push(l.trim());
    }
  }
  const lis = items.map((it, i) => {
    let text = it.text.join(' ');
    let check: ReactNode = null;
    const task = /^\[( |x|X)\]\s+(.*)$/.exec(text);
    if (task) {
      check = <span className="md-task" aria-label={task[1] === ' ' ? 'not done' : 'done'}>{task[1] === ' ' ? '☐' : '☑'} </span>;
      text = task[2];
    }
    return (
      <li key={i}>
        {check}
        {inline(text, ctx, `${key}li${i}`)}
        {it.children.length > 0 && <Blocks src={it.children.map((c) => c.slice(Math.min(c.length - c.trimStart().length, indent + 2))).join('\n')} ctx={ctx} seen={seen} k={`${key}c${i}`} />}
      </li>
    );
  });
  return ordered ? <ol key={key} start={start}>{lis}</ol> : <ul key={key}>{lis}</ul>;
}

function Blocks({ src, ctx, seen, k }: { src: string; ctx: Ctx; seen: Map<string, number>; k: string }) {
  const lines = src.split('\n');
  const out: ReactNode[] = [];
  let i = 0;
  let key = 0;
  while (i < lines.length) {
    const l = lines[i];
    const kk = `${k}b${key++}`;
    const fence = /^\s*(```+|~~~+)\s*([\w+-]*)/.exec(l);
    if (fence) {
      const body: string[] = [];
      i++;
      while (i < lines.length && !lines[i].trim().startsWith(fence[1])) body.push(lines[i++]);
      i++;
      const lang = fence[2].toLowerCase();
      if (lang === 'mermaid') {
        const flow = parseFlow(body.join('\n'));
        out.push(
          flow ? (
            <figure key={kk} className="md-mermaid">
              <FlowDiagram flow={flow} />
              <details>
                <summary className="small muted">Mermaid source (drawn here without the Mermaid library; GitHub renders the original)</summary>
                <CodeBlock code={body.join('\n')} lang="mermaid" />
              </details>
            </figure>
          ) : (
            <div key={kk} className="md-mermaid">
              <div className="small muted">Mermaid diagram — GitHub draws it; this viewer only draws flowcharts, so here is the source (paste it into the Mermaid Live Editor at https://mermaid.live to see it):</div>
              <CodeBlock code={body.join('\n')} lang="mermaid" />
            </div>
          ),
        );
      } else out.push(<CodeBlock key={kk} code={body.join('\n')} lang={lang} />);
      continue;
    }
    const h = /^(#{1,6})\s+(.*?)\s*#*\s*$/.exec(l);
    if (h) {
      const lvl = h[1].length;
      const id = headingId(h[2], seen);
      const content = inline(h[2], ctx, kk);
      const Tag = `h${lvl}` as 'h1';
      out.push(
        <Tag key={kk} id={`md-${id}`} className={`md-h md-h${lvl}`}>
          {content}
          {lvl > 1 && <a className="md-anchor" href={`#/docs/${slugOf(ctx.doc)}?s=${encodeURIComponent(id)}`} aria-label="Link to this section">#</a>}
        </Tag>,
      );
      i++;
      continue;
    }
    if (/^\s*([-*_])(\s*\1){2,}\s*$/.test(l)) {
      out.push(<hr key={kk} />);
      i++;
      continue;
    }
    if (/^\s*>/.test(l)) {
      const body: string[] = [];
      while (i < lines.length && /^\s*>/.test(lines[i])) body.push(lines[i++].replace(/^\s*>\s?/, ''));
      out.push(<blockquote key={kk} className="md-quote"><Blocks src={body.join('\n')} ctx={ctx} seen={seen} k={kk} /></blockquote>);
      continue;
    }
    if (/^\s*\|/.test(l) && i + 1 < lines.length && /^\s*\|?\s*:?-{2,}:?\s*(\|\s*:?-{2,}:?\s*)*\|?\s*$/.test(lines[i + 1])) {
      const head = splitRow(l);
      const align = splitRow(lines[i + 1]).map((c) => (c.startsWith(':') && c.endsWith(':') ? 'center' : c.endsWith(':') ? 'right' : 'left') as 'left');
      i += 2;
      const rows: string[][] = [];
      while (i < lines.length && /^\s*\|/.test(lines[i])) rows.push(splitRow(lines[i++]));
      out.push(
        <div key={kk} className="table-wrap md-table">
          <table className="table">
            <thead><tr>{head.map((c, j) => <th key={j} style={{ textAlign: align[j] }}>{inline(c, ctx, `${kk}h${j}`)}</th>)}</tr></thead>
            <tbody>{rows.map((r, ri) => <tr key={ri}>{r.map((c, j) => <td key={j} style={{ textAlign: align[j] }}>{inline(c, ctx, `${kk}r${ri}c${j}`)}</td>)}</tr>)}</tbody>
          </table>
        </div>,
      );
      continue;
    }
    if (LIST_RE.test(l)) {
      const body: string[] = [];
      while (i < lines.length) {
        const x = lines[i];
        if (x.trim() === '') {
          // a blank line ends the list unless the next line continues it (indented or another item)
          const nx = lines[i + 1];
          if (nx !== undefined && (LIST_RE.test(nx) || /^\s{2,}\S/.test(nx))) {
            i++;
            continue;
          }
          break;
        }
        if (!LIST_RE.test(x) && !/^\s/.test(x) && body.length && isBlockStart(x)) break;
        body.push(x);
        i++;
      }
      out.push(renderList(body, ctx, kk, seen));
      continue;
    }
    if (l.trim() === '') {
      i++;
      continue;
    }
    const para: string[] = [];
    while (i < lines.length && lines[i].trim() !== '' && !(para.length && isBlockStart(lines[i]))) para.push(lines[i++]);
    // Soft line breaks join into one run (so **bold across lines** works);
    // hard breaks (two trailing spaces or a backslash) split it.
    const parts: ReactNode[] = [];
    const runs: string[][] = [[]];
    para.forEach((p) => {
      const hard = / {2,}$/.test(p) || p.endsWith('\\');
      runs[runs.length - 1].push(p.replace(/\\$/, '').trim());
      if (hard) runs.push([]);
    });
    runs
      .filter((r) => r.length)
      .forEach((r, j) => {
        if (j > 0) parts.push(<br key={`${kk}br${j}`} />);
        parts.push(...inline(r.join(' '), ctx, `${kk}p${j}`));
      });
    out.push(<p key={kk}>{parts}</p>);
  }
  return <>{out}</>;
}

export function Markdown({ src, doc, highlight = '' }: { src: string; doc: string; highlight?: string }) {
  return (
    <div className="md-doc">
      <Blocks src={src.replace(/\r\n/g, '\n')} ctx={{ doc, hl: highlight }} seen={new Map()} k="m" />
    </div>
  );
}
