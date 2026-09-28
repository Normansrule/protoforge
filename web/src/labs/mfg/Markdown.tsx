// Minimal Markdown renderer for the engine's order guides and reports
// (headings, paragraphs, ordered/unordered lists, bold, italics, inline code,
// links, fenced code, simple tables). No HTML injection: everything is text.
import type { ReactNode } from 'react';

function inline(s: string, key: string): ReactNode[] {
  const out: ReactNode[] = [];
  const re = /(\*\*[^*]+\*\*|`[^`]+`|\[[^\]]+\]\([^)]+\)|\*[^*]+\*|_[^_]+_)/g;
  let last = 0;
  let m: RegExpExecArray | null;
  let i = 0;
  while ((m = re.exec(s))) {
    if (m.index > last) out.push(s.slice(last, m.index));
    const t = m[0];
    const k = `${key}-${i++}`;
    if (t.startsWith('**')) out.push(<b key={k}>{t.slice(2, -2)}</b>);
    else if (t.startsWith('`')) out.push(<code key={k} className="mono">{t.slice(1, -1)}</code>);
    else if (t.startsWith('[')) {
      const mm = /\[([^\]]+)\]\(([^)]+)\)/.exec(t)!;
      out.push(<a key={k} href={mm[2]} target="_blank" rel="noreferrer">{mm[1]}</a>);
    } else out.push(<i key={k}>{t.slice(1, -1)}</i>);
    last = m.index + t.length;
  }
  if (last < s.length) out.push(s.slice(last));
  return out;
}

export function Markdown({ src }: { src: string }) {
  const lines = src.split('\n');
  const blocks: ReactNode[] = [];
  let i = 0;
  let key = 0;
  while (i < lines.length) {
    const l = lines[i];
    if (l.startsWith('```')) {
      const body: string[] = [];
      i++;
      while (i < lines.length && !lines[i].startsWith('```')) body.push(lines[i++]);
      i++;
      blocks.push(<pre key={key++} className="code">{body.join('\n')}</pre>);
      continue;
    }
    const h = /^(#{1,4})\s+(.*)$/.exec(l);
    if (h) {
      const lvl = h[1].length;
      const content = inline(h[2], `h${key}`);
      blocks.push(lvl <= 1 ? <h2 key={key++} style={{ margin: '4px 0 8px' }}>{content}</h2> : lvl === 2 ? <h3 key={key++} style={{ margin: '12px 0 6px' }}>{content}</h3> : <h4 key={key++} style={{ margin: '10px 0 4px' }}>{content}</h4>);
      i++;
      continue;
    }
    if (/^\s*(\d+\.|[*-])\s+/.test(l)) {
      const ordered = /^\s*\d+\./.test(l);
      const items: ReactNode[] = [];
      while (i < lines.length && /^\s*(\d+\.|[*-])\s+/.test(lines[i])) {
        items.push(<li key={i} style={{ marginBottom: 4 }}>{inline(lines[i].replace(/^\s*(\d+\.|[*-])\s+/, ''), `li${i}`)}</li>);
        i++;
      }
      blocks.push(ordered ? <ol key={key++} style={{ paddingLeft: 22 }}>{items}</ol> : <ul key={key++} style={{ paddingLeft: 20 }}>{items}</ul>);
      continue;
    }
    if (l.startsWith('|')) {
      const rows: string[][] = [];
      while (i < lines.length && lines[i].startsWith('|')) {
        const cells = lines[i].split('|').slice(1, -1).map((c) => c.trim());
        if (!cells.every((c) => /^-+$/.test(c))) rows.push(cells);
        i++;
      }
      blocks.push(
        <div key={key++} className="table-wrap"><table className="table"><thead><tr>{rows[0]?.map((c, j) => <th key={j}>{inline(c, `th${j}`)}</th>)}</tr></thead>
          <tbody>{rows.slice(1).map((r, ri) => <tr key={ri}>{r.map((c, j) => <td key={j}>{inline(c, `td${ri}-${j}`)}</td>)}</tr>)}</tbody></table></div>,
      );
      continue;
    }
    if (l.trim() === '') { i++; continue; }
    const para: string[] = [];
    while (i < lines.length && lines[i].trim() !== '' && !/^(#|\s*(\d+\.|[*-])\s|\||```)/.test(lines[i])) para.push(lines[i++]);
    blocks.push(<p key={key++}>{inline(para.join(' '), `p${key}`)}</p>);
  }
  return <div className="md" style={{ fontSize: 14, color: 'var(--text-2)' }}>{blocks}</div>;
}
