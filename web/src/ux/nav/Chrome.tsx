// Navigation chrome: sidebar and top bar.
// Simple mode shows "Your labs" for the chosen discipline first and folds the
// rest under "All labs"; Expert mode shows the full grouped list. The top bar
// carries the Simple/Expert switch, the command-palette button (Ctrl/⌘ K) and
// Open / Save / Reset / theme (folded into a menu on phones).
import { useEffect, useRef, useState } from 'react';
import { Link, NavLink, useLocation } from 'react-router-dom';
import { GROUPS, LABS } from '../../labs/registry';
import { Icon } from '../../components/Icon';
import { useEngineReady } from '../../engine/hooks';
import { looksLikeProject, useEvaluation, useProject } from '../../state/project';
import { call } from '../../engine/engine';
import { downloadText } from '../../components/ui';
import type { Project } from '../../engine/types';
import { openPalette, usePrefs, useTheme } from '../prefs';
import { roleById } from '../data/roles';
import { LAB_TABS } from '../data/labTabs';
import './nav.css';

const REPO_URL = 'https://github.com/Normansrule/protoforge';

function labStatus(prefixes: string[], ev: ReturnType<typeof useEvaluation>, project: Project) {
  if (!ev.data) return null;
  const ids = project.requirements.filter((r) => r.verification.metric && prefixes.some((p) => r.verification.metric!.startsWith(p))).map((r) => r.id);
  if (ids.length === 0) return null;
  const rs = ev.data.results.filter((r) => ids.includes(r.id));
  if (rs.some((r) => r.status === 'fail')) return 'fail';
  if (rs.every((r) => r.status === 'pass')) return 'pass';
  if (rs.every((r) => r.status === 'unverified')) return null;
  return 'partial';
}

function StatusDot({ s }: { s: string | null }) {
  if (!s) return null;
  const text = s === 'pass' ? 'all requirements pass' : s === 'fail' ? 'a requirement fails' : 'some requirements unverified';
  return (
    <span className="nav-status" title={text} style={{ color: s === 'pass' ? 'var(--good)' : s === 'fail' ? 'var(--bad)' : 'var(--warn)' }}>
      {s === 'pass' ? '✓' : s === 'fail' ? '✕' : '!'}
      <span className="sr-only">{text}</span>
    </span>
  );
}

export function Sidebar({ open, onNavigate }: { open: boolean; onNavigate: () => void }) {
  const ev = useEvaluation();
  const project = useProject((s) => s.project);
  const mode = usePrefs((s) => s.mode);
  const role = roleById(usePrefs((s) => s.role));
  const allOpen = usePrefs((s) => s.allLabsOpen);
  const setAllOpen = usePrefs((s) => s.setAllLabsOpen);
  const { pathname } = useLocation();

  const labLink = (id: string, key?: string) => {
    const l = LABS.find((x) => x.id === id);
    if (!l) return null;
    return (
      <NavLink key={key ?? l.id} to={l.path} className="nav-link" onClick={onNavigate}>
        <Icon name={l.icon} className="nav-icon" /> {l.title}
        <StatusDot s={labStatus(l.metrics, ev, project)} />
      </NavLink>
    );
  };
  const groups = (
    <>
      {GROUPS.map((g) => (
        <div className="nav-group" key={g}>
          <div className="nav-group-title">{g}</div>
          {LABS.filter((l) => l.group === g).map((l) => labLink(l.id))}
        </div>
      ))}
    </>
  );
  const simple = mode === 'simple';
  const yourLabs = role?.labs ?? [];
  // Keep the active lab visible: unfold "All labs" when it is not one of yours.
  const activeElsewhere = simple && LABS.some((l) => l.path === pathname) && !yourLabs.some((id) => LABS.find((l) => l.id === id)?.path === pathname);
  const unfolded = allOpen || activeElsewhere;

  return (
    <nav className={`sidebar ${open ? 'open' : ''}`} aria-label="Main">
      <NavLink to="/" className="brand" onClick={onNavigate}>
        <img src="./favicon.svg" width={34} height={34} alt="" />
        <div>
          <div className="brand-name">Protoforge</div>
          <div className="brand-sub">idea → atoms</div>
        </div>
      </NavLink>
      <div className="ux-nav-cta">
        <NavLink to="/start" className="ux-nav-start" onClick={onNavigate}>
          <span className="ux-nav-start-icon"><Icon name={role?.icon ?? 'arrow'} size={16} /></span>
          <span className="ux-nav-start-text">
            <b>Start here</b>
            <span>{role ? `Your path · ${role.short}` : 'Pick your discipline'}</span>
          </span>
        </NavLink>
        <NavLink to="/new" className="ux-nav-new" onClick={onNavigate}>
          <span aria-hidden>＋</span> New product
        </NavLink>
      </div>
      <div className="nav-group">
        <NavLink to="/" end className="nav-link" onClick={onNavigate}><Icon name="home" className="nav-icon" /> Overview</NavLink>
        <NavLink to="/systems" className="nav-link" onClick={onNavigate}>
          <Icon name="systems" className="nav-icon" /> Systems Studio
          {ev.data && <span className="chip" style={{ marginLeft: 'auto', padding: '0 7px' }}>{ev.data.pass}/{ev.data.results.length}</span>}
        </NavLink>
        <NavLink to="/versions" className="nav-link" onClick={onNavigate}><Icon name="versions" className="nav-icon" /> Versions</NavLink>
        <NavLink to="/toolchain" className="nav-link" onClick={onNavigate}><Icon name="toolchain" className="nav-icon" /> Web toolchain</NavLink>
      </div>
      {simple ? (
        <>
          <div className="nav-group">
            <div className="nav-group-title">{role ? `Your labs · ${role.short}` : 'Your labs'}</div>
            {role ? (
              yourLabs.map((id) => labLink(id, `mine-${id}`))
            ) : (
              <Link to="/start" className="ux-nav-hint" onClick={onNavigate}>
                Choose your discipline and the labs you need most appear here.
              </Link>
            )}
          </div>
          <div className="nav-group">
            <button type="button" className="ux-nav-fold" aria-expanded={unfolded} onClick={() => setAllOpen(!unfolded)}>
              <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden className="ux-nav-chev"><path d="M4 2.5L7.5 6 4 9.5" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" /></svg>
              All labs ({LABS.length})
            </button>
            {unfolded && <div className="ux-nav-all">{groups}</div>}
          </div>
        </>
      ) : (
        groups
      )}
      <div className="nav-group">
        <div className="nav-group-title">Learn</div>
        <NavLink to="/learn" className="nav-link" onClick={onNavigate}><Icon name="learn" className="nav-icon" /> Guide & references</NavLink>
        <NavLink to="/docs" className="nav-link" onClick={onNavigate}><Icon name="docs" className="nav-icon" /> Documentation</NavLink>
        <a className="nav-link" href={REPO_URL} target="_blank" rel="noreferrer"><Icon name="github" className="nav-icon" /> Source on GitHub</a>
      </div>
    </nav>
  );
}

function SearchGlyph() {
  return (
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" aria-hidden>
      <circle cx="11" cy="11" r="7" />
      <path d="M20 20l-3.5-3.5" />
    </svg>
  );
}

const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

/** Opens `?tab=` deep links on pages that do not read the parameter themselves, by clicking the matching tab. */
function TabBridge() {
  const { pathname, search } = useLocation();
  useEffect(() => {
    const tab = new URLSearchParams(search).get('tab');
    const page = Object.values(LAB_TABS).find((p) => p.path === pathname);
    if (!tab || !page || page.native) return;
    const label = page.tabs.find((t) => t.id === tab)?.label;
    if (!label) return;
    let tries = 0;
    let timer: ReturnType<typeof setTimeout>;
    const attempt = () => {
      const btn = Array.from(document.querySelectorAll<HTMLElement>('main [role="tab"]')).find((b) => b.textContent?.trim() === label);
      if (btn) {
        if (btn.getAttribute('aria-selected') !== 'true') btn.click();
        return;
      }
      if (++tries < 60) timer = setTimeout(attempt, 100);
    };
    timer = setTimeout(attempt, 0);
    return () => clearTimeout(timer);
  }, [pathname, search]);
  return null;
}

export function ModeSwitch({ compact }: { compact?: boolean }) {
  const mode = usePrefs((s) => s.mode);
  const setMode = usePrefs((s) => s.setMode);
  return (
    <div className={`ux-mode ${compact ? 'compact' : ''}`} role="group" aria-label="Interface mode">
      {(['simple', 'expert'] as const).map((m) => (
        <button
          key={m}
          type="button"
          aria-pressed={mode === m}
          className={mode === m ? 'on' : ''}
          onClick={() => setMode(m)}
          title={m === 'simple' ? 'Simple: your labs first, help opens by itself' : 'Expert: every lab in the sidebar'}
        >
          {m === 'simple' ? 'Simple' : 'Expert'}
        </button>
      ))}
    </div>
  );
}

export function TopBar({ onMenu }: { onMenu: () => void }) {
  const theme = useTheme((s) => s.theme);
  const toggleTheme = useTheme((s) => s.toggle);
  const project = useProject((s) => s.project);
  const setProject = useProject((s) => s.setProject);
  const reset = useProject((s) => s.reset);
  const ev = useEvaluation();
  const ready = useEngineReady();
  const fileRef = useRef<HTMLInputElement>(null);
  const [more, setMore] = useState(false);
  const moreRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!more) return;
    const close = (e: Event) => {
      if (e instanceof KeyboardEvent && e.key !== 'Escape') return;
      if (e instanceof PointerEvent && moreRef.current?.contains(e.target as Node)) return;
      setMore(false);
    };
    document.addEventListener('pointerdown', close);
    document.addEventListener('keydown', close);
    return () => {
      document.removeEventListener('pointerdown', close);
      document.removeEventListener('keydown', close);
    };
  }, [more]);

  const save = () => downloadText(`${project.meta.id || 'project'}.json`, JSON.stringify(project, null, 2), 'application/json');
  const doReset = () => confirm('Reset to the Iris reference project? Unsaved edits are lost.') && reset();
  const fileActions = (
    <>
      <button className="btn small" onClick={() => fileRef.current?.click()} title="Open a project.json"><Icon name="upload" size={15} /> Open</button>
      <button className="btn small" onClick={save} title="Save project.json"><Icon name="download" size={15} /> Save</button>
      <button className="btn small" onClick={doReset} title="Reset to Iris" aria-label="Reset to Iris"><Icon name="reset" size={15} /></button>
    </>
  );

  return (
    <header className="topbar">
      <TabBridge />
      <button className="btn ghost menu-btn" onClick={onMenu} aria-label="Menu"><Icon name="menu" /></button>
      <div style={{ minWidth: 0 }}>
        <div style={{ fontWeight: 700, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{project.meta.name}</div>
        <div className="muted small hide-sm" style={{ whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>v{project.meta.version} · {ready ? 'Rust engine loaded (WebAssembly)' : 'loading engine…'}</div>
      </div>
      {ev.data && (
        <div className="row hide-md" style={{ marginLeft: 8, flexWrap: 'nowrap' }}>
          <span className="chip pass">✓ {ev.data.pass}</span>
          {ev.data.fail - (ev.data.open_goals ?? 0) > 0 && <span className="chip fail">✕ {ev.data.fail - (ev.data.open_goals ?? 0)}</span>}
          {(ev.data.open_goals ?? 0) > 0 && <span className="chip warn" title="stretch goals not yet met">! {ev.data.open_goals} open goal{ev.data.open_goals === 1 ? '' : 's'}</span>}
          {ev.data.unverified > 0 && <span className="chip">○ {ev.data.unverified}</span>}
        </div>
      )}
      <div className="spacer" />
      <button type="button" className="ux-search" onClick={() => openPalette()} aria-label="Search (Ctrl K)" aria-keyshortcuts={isMac ? 'Meta+K' : 'Control+K'}>
        <SearchGlyph />
        <span className="ux-search-text">Search…</span>
        <kbd className="ux-kbd">{isMac ? '⌘' : 'Ctrl'} K</kbd>
      </button>
      <ModeSwitch />
      <input
        ref={fileRef}
        type="file"
        accept="application/json,.json"
        style={{ display: 'none' }}
        onChange={async (e) => {
          const f = e.target.files?.[0];
          if (!f) return;
          try {
            const parsed = JSON.parse(await f.text()) as Project;
            // the Rust engine deserialises the whole product model: any missing
            // or mistyped field is reported instead of breaking the app
            call('systems.lint', parsed);
            if (!looksLikeProject(parsed)) throw new Error('missing meta, concept, design, needs, requirements, subsystems or components');
            setProject(parsed);
          } catch (err) {
            alert(`Could not open this file as a Protoforge project:\n${err instanceof Error ? err.message : err}`);
          }
          e.target.value = '';
        }}
      />
      <div className="ux-file-actions">{fileActions}</div>
      <div className="ux-more" ref={moreRef}>
        <button type="button" className="btn small" aria-label="Project file actions" aria-haspopup="menu" aria-expanded={more} onClick={() => setMore((m) => !m)}>
          <svg width="15" height="15" viewBox="0 0 24 24" fill="currentColor" aria-hidden><circle cx="5" cy="12" r="1.8" /><circle cx="12" cy="12" r="1.8" /><circle cx="19" cy="12" r="1.8" /></svg>
        </button>
        {more && (
          <div className="ux-more-menu" role="menu">
            <button role="menuitem" onClick={() => { setMore(false); fileRef.current?.click(); }}><Icon name="upload" size={15} /> Open project.json</button>
            <button role="menuitem" onClick={() => { setMore(false); save(); }}><Icon name="download" size={15} /> Save project.json</button>
            <button role="menuitem" onClick={() => { setMore(false); doReset(); }}><Icon name="reset" size={15} /> Reset to Iris</button>
          </div>
        )}
      </div>
      <button className="btn small" onClick={toggleTheme} aria-label={theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}><Icon name={theme === 'dark' ? 'sun' : 'moon'} size={15} /></button>
    </header>
  );
}
