// The command palette dialog (lazy-loaded the first time it opens).
import { useEffect, useId, useMemo, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Icon } from '../../components/Icon';
import { downloadText } from '../../components/ui';
import { useEvaluation, useProject } from '../../state/project';
import { usePrefs, useTheme } from '../prefs';
import { buildItems, search, type PItem, type Scored, type Category } from './items';
import './palette.css';

export default function PaletteDialog({ initialQuery, onClose }: { initialQuery: string; onClose: () => void }) {
  const project = useProject((s) => s.project);
  const reset = useProject((s) => s.reset);
  const ev = useEvaluation();
  const mode = usePrefs((s) => s.mode);
  const toggleMode = usePrefs((s) => s.toggleMode);
  const recent = usePrefs((s) => s.recent);
  const pushRecent = usePrefs((s) => s.pushRecent);
  const theme = useTheme((s) => s.theme);
  const toggleTheme = useTheme((s) => s.toggle);
  const navigate = useNavigate();
  const [q, setQ] = useState(initialQuery);
  const [active, setActive] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const dialog = useRef<HTMLDivElement>(null);
  const uid = useId();
  const listId = `${uid}-list`;

  const items = useMemo(() => buildItems(project, ev.data, { mode, theme }), [project, ev.data, mode, theme]);

  const groups = useMemo<{ cat: Category; items: (Scored | PItem)[] }[]>(() => {
    if (q.trim()) return search(items, q);
    const byId = new Map(items.map((i) => [i.id, i]));
    const rec = recent.map((r) => byId.get(r.id)).filter((x): x is PItem => !!x);
    const suggested = ['page:/start', 'page:/new', 'page:/systems', 'action:mode', 'action:theme', 'action:role']
      .map((id) => byId.get(id))
      .filter((x): x is PItem => !!x && !rec.some((r) => r.id === x.id));
    const out: { cat: Category; items: PItem[] }[] = [];
    if (rec.length) out.push({ cat: 'Recent', items: rec.slice(0, 6) });
    out.push({ cat: rec.length ? 'Actions' : 'Pages', items: suggested });
    return out;
  }, [q, items, recent]);

  const flat = useMemo(() => groups.flatMap((g) => g.items), [groups]);
  useEffect(() => setActive(0), [q]);
  useEffect(() => {
    input.current?.focus();
    input.current?.select();
  }, []);
  useEffect(() => {
    list.current?.querySelector<HTMLElement>(`[data-idx="${active}"]`)?.scrollIntoView({ block: 'nearest' });
  }, [active]);

  const run = (it: PItem | undefined) => {
    if (!it) return;
    pushRecent({ id: it.id, label: it.title, hint: it.sub, href: it.href });
    onClose();
    if (it.href) {
      navigate(it.href);
      return;
    }
    switch (it.action) {
      case 'theme':
        toggleTheme();
        break;
      case 'mode':
        toggleMode();
        break;
      case 'role':
        navigate('/start');
        break;
      case 'new':
        navigate('/new');
        break;
      case 'save':
        downloadText(`${project.meta.id || 'project'}.json`, JSON.stringify(project, null, 2), 'application/json');
        break;
      case 'reset':
        if (confirm('Reset to the Iris reference project? Unsaved edits are lost.')) reset();
        break;
    }
  };

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setActive((a) => (flat.length ? (a + 1) % flat.length : 0));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setActive((a) => (flat.length ? (a - 1 + flat.length) % flat.length : 0));
    } else if (e.key === 'Home' && flat.length) {
      e.preventDefault();
      setActive(0);
    } else if (e.key === 'End' && flat.length) {
      e.preventDefault();
      setActive(flat.length - 1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      run(flat[active]);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    } else if (e.key === 'Tab') {
      // Focus trap: the input is the only tab stop; Shift+Tab / Tab cycle the close button.
      const focusables = dialog.current?.querySelectorAll<HTMLElement>('input, button.pl-close');
      if (!focusables || focusables.length === 0) return;
      const arr = Array.from(focusables);
      const i = arr.indexOf(document.activeElement as HTMLElement);
      e.preventDefault();
      arr[(i + (e.shiftKey ? -1 : 1) + arr.length) % arr.length].focus();
    }
  };

  let idx = -1;
  const activeId = flat[active] ? `${uid}-opt-${active}` : undefined;
  return (
    <div className="pl-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div ref={dialog} className="pl-dialog" role="dialog" aria-modal="true" aria-label="Command palette" onKeyDown={onKey}>
        <div className="pl-input-row">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" aria-hidden className="pl-glass">
            <circle cx="11" cy="11" r="7" />
            <path d="M20 20l-3.5-3.5" />
          </svg>
          <input
            ref={input}
            className="pl-input"
            role="combobox"
            aria-expanded="true"
            aria-controls={listId}
            aria-activedescendant={activeId}
            aria-autocomplete="list"
            aria-label="Search labs, tabs, requirements, metrics, terms, guides and actions"
            placeholder="Search labs, tabs, R-0xx, metrics, terms, “I want to…”"
            value={q}
            onChange={(e) => setQ(e.target.value)}
            spellCheck={false}
            autoComplete="off"
          />
          <button type="button" className="pl-close" onClick={onClose} aria-label="Close">Esc</button>
        </div>
        <div ref={list} id={listId} role="listbox" aria-label="Results" className="pl-list">
          {flat.length === 0 && <div className="pl-empty">No match for “{q}”. Try a lab name, a requirement such as R-002, or a task such as “size a battery”.</div>}
          {groups.map((g) => (
            <div key={g.cat} role="group" aria-labelledby={`${uid}-${g.cat}`} className="pl-group">
              <div id={`${uid}-${g.cat}`} className="pl-cat" role="presentation">{!q.trim() && g.cat !== 'Recent' ? 'Suggested' : g.cat}</div>
              {g.items.map((it) => {
                idx++;
                const i = idx;
                return (
                  <div
                    key={it.id}
                    id={`${uid}-opt-${i}`}
                    data-idx={i}
                    role="option"
                    aria-selected={i === active}
                    className={`pl-item ${i === active ? 'on' : ''}`}
                    onMouseMove={() => i !== active && setActive(i)}
                    onClick={() => run(it)}
                  >
                    <span className="pl-ico"><Icon name={it.icon ?? 'arrow'} size={16} /></span>
                    <span className="pl-main">
                      <span className="pl-title">
                        {it.cat === 'Labs & tabs' && it.id.startsWith('tab:') && <span className="pl-parent">{it.sub} › </span>}
                        {it.title}
                        {it.cat === 'Glossary' && it.sub && <span className="pl-full"> · {it.sub}</span>}
                      </span>
                      {it.detail ? <span className="pl-detail">{it.detail}</span> : it.sub && !it.id.startsWith('tab:') && it.cat !== 'Glossary' && <span className="pl-sub">{it.sub}</span>}
                    </span>
                    {it.badge && <span className={`chip ${it.badge.tone ?? ''} pl-badge`}>{it.badge.text}</span>}
                    {i === active && <span className="pl-enter" aria-hidden>↵</span>}
                  </div>
                );
              })}
            </div>
          ))}
        </div>
        <div className="pl-foot" aria-hidden>
          <span><kbd>↑</kbd><kbd>↓</kbd> move</span>
          <span><kbd>↵</kbd> open</span>
          <span><kbd>Esc</kbd> close</span>
          <span className="pl-foot-r">Press <kbd>/</kbd> or <kbd>Ctrl K</kbd> anywhere</span>
        </div>
      </div>
    </div>
  );
}
