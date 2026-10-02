// Light-weight syntax highlighting (OpenSCAD, STEP, CircuitJS text) with copy
// and download — text only, no HTML injection.
import { useMemo, useState, type ReactNode } from 'react';
import { DownloadButton } from '../../components/ui';

const SCAD_KW = new Set(['module', 'function', 'let', 'for', 'if', 'else', 'true', 'false', 'each', 'include', 'use']);
const SCAD_BUILTIN = new Set([
  'cube', 'sphere', 'cylinder', 'polyhedron', 'polygon', 'circle', 'square', 'text', 'linear_extrude', 'rotate_extrude', 'translate', 'rotate', 'scale',
  'mirror', 'difference', 'union', 'intersection', 'hull', 'minkowski', 'offset', 'color', 'pow', 'sign', 'abs', 'cos', 'sin', 'tan', 'min', 'max',
  'sqrt', 'len', 'children', 'echo', 'assert',
]);

type Lang = 'scad' | 'step' | 'plain';

function tokens(src: string, lang: Lang): ReactNode[] {
  const out: ReactNode[] = [];
  const re =
    lang === 'scad'
      ? /(\/\/[^\n]*|\/\*[\s\S]*?\*\/|"(?:\\.|[^"\\])*"|\$?[A-Za-z_][A-Za-z0-9_]*|\d+\.?\d*(?:[eE][-+]?\d+)?)/g
      : /('(?:''|[^'])*'|#\d+|\b[A-Z][A-Z0-9_]{2,}\b|-?\d+\.\d*(?:E[-+]?\d+)?)/g;
  let last = 0;
  let m: RegExpExecArray | null;
  let k = 0;
  while ((m = re.exec(src))) {
    if (m.index > last) out.push(src.slice(last, m.index));
    const t = m[0];
    let cls = '';
    if (lang === 'scad') {
      if (t.startsWith('//') || t.startsWith('/*')) cls = 'tk-com';
      else if (t.startsWith('"')) cls = 'tk-str';
      else if (/^\d/.test(t)) cls = 'tk-num';
      else if (SCAD_KW.has(t)) cls = 'tk-kw';
      else if (SCAD_BUILTIN.has(t)) cls = 'tk-fn';
      else if (t.startsWith('$')) cls = 'tk-kw';
    } else {
      if (t.startsWith("'")) cls = 'tk-str';
      else if (t.startsWith('#')) cls = 'tk-ref';
      else if (/^-?\d/.test(t)) cls = 'tk-num';
      else cls = 'tk-fn';
    }
    out.push(cls ? <span key={k++} className={cls}>{t}</span> : t);
    last = m.index + t.length;
  }
  if (last < src.length) out.push(src.slice(last));
  return out;
}

export function CodeView(props: { code: string; filename: string; lang: Lang; maxLines?: number; maxHeight?: number; mime?: string }) {
  const [copied, setCopied] = useState(false);
  const shown = useMemo(() => {
    const lines = props.code.split('\n');
    const max = props.maxLines ?? 400;
    return lines.length > max ? lines.slice(0, max).join('\n') + `\n… ${lines.length - max} more lines (download for the full file)` : props.code;
  }, [props.code, props.maxLines]);
  const html = useMemo(() => tokens(shown, props.lang), [shown, props.lang]);
  return (
    <div>
      <div className="row" style={{ justifyContent: 'space-between', marginBottom: 6 }}>
        <span className="muted small mono">{props.filename}</span>
        <div className="row">
          <button className="btn small" onClick={() => navigator.clipboard?.writeText(props.code).then(() => { setCopied(true); setTimeout(() => setCopied(false), 1200); }).catch(() => {})}>{copied ? 'Copied' : 'Copy'}</button>
          <DownloadButton filename={props.filename} content={() => new Blob([props.code], { type: props.mime ?? 'text/plain' })} small />
        </div>
      </div>
      <pre className="code tc-code" style={{ maxHeight: props.maxHeight ?? 420 }}>{html}</pre>
    </div>
  );
}
