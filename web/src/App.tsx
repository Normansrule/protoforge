import { Suspense, useEffect, useRef, useState } from 'react';
import { HashRouter, NavLink, Route, Routes, useLocation } from 'react-router-dom';
import { GROUPS, LABS } from './labs/registry';
import { Icon } from './components/Icon';
import { useEngineReady } from './engine/hooks';
import { looksLikeProject, useEvaluation, useProject } from './state/project';
import { call } from './engine/engine';
import { ErrorBoundary } from './components/ErrorBoundary';
import { downloadText } from './components/ui';
import Home from './pages/Home';
import Systems from './pages/Systems';
import Learn from './pages/Learn';
import Toolchain from './pages/Toolchain';
import Versions from './pages/Versions';
import Docs from './pages/Docs';
import type { Project } from './engine/types';

const REPO_URL = 'https://github.com/Normansrule/protoforge';

function useTheme() {
  const [theme, setTheme] = useState<'dark' | 'light'>(() => {
    try {
      return (localStorage.getItem('protoforge.theme') as 'dark' | 'light') || 'dark';
    } catch {
      return 'dark';
    }
  });
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem('protoforge.theme', theme);
    } catch {
      /* ignore */
    }
  }, [theme]);
  return [theme, setTheme] as const;
}

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

function Sidebar({ open, onNavigate }: { open: boolean; onNavigate: () => void }) {
  const ev = useEvaluation();
  const project = useProject((s) => s.project);
  const dot = (s: string | null) =>
    s ? (
      <span className="nav-status" title={s === 'pass' ? 'all requirements pass' : s === 'fail' ? 'a requirement fails' : 'some requirements unverified'} style={{ color: s === 'pass' ? 'var(--good)' : s === 'fail' ? 'var(--bad)' : 'var(--warn)' }}>
        {s === 'pass' ? '✓' : s === 'fail' ? '✕' : '!'}
        <span className="sr-only">{s === 'pass' ? 'all requirements pass' : s === 'fail' ? 'a requirement fails' : 'some requirements unverified'}</span>
      </span>
    ) : null;
  return (
    <nav className={`sidebar ${open ? 'open' : ''}`} aria-label="Main">
      <NavLink to="/" className="brand" onClick={onNavigate}>
        <img src="./favicon.svg" width={34} height={34} alt="" />
        <div>
          <div className="brand-name">Protoforge</div>
          <div className="brand-sub">idea → atoms</div>
        </div>
      </NavLink>
      <div className="nav-group">
        <NavLink to="/" end className="nav-link" onClick={onNavigate}><Icon name="home" className="nav-icon" /> Overview</NavLink>
        <NavLink to="/systems" className="nav-link" onClick={onNavigate}>
          <Icon name="systems" className="nav-icon" /> Systems Studio
          {ev.data && <span className="chip" style={{ marginLeft: 'auto', padding: '0 7px' }}>{ev.data.pass}/{ev.data.results.length}</span>}
        </NavLink>
        <NavLink to="/versions" className="nav-link" onClick={onNavigate}><Icon name="versions" className="nav-icon" /> Versions</NavLink>
        <NavLink to="/toolchain" className="nav-link" onClick={onNavigate}><Icon name="toolchain" className="nav-icon" /> Web toolchain</NavLink>
      </div>
      {GROUPS.map((g) => (
        <div className="nav-group" key={g}>
          <div className="nav-group-title">{g}</div>
          {LABS.filter((l) => l.group === g).map((l) => (
            <NavLink key={l.id} to={l.path} className="nav-link" onClick={onNavigate}>
              <Icon name={l.icon} className="nav-icon" /> {l.title}
              {dot(labStatus(l.metrics, ev, project))}
            </NavLink>
          ))}
        </div>
      ))}
      <div className="nav-group">
        <div className="nav-group-title">Learn</div>
        <NavLink to="/learn" className="nav-link" onClick={onNavigate}><Icon name="learn" className="nav-icon" /> Guide & references</NavLink>
        <NavLink to="/docs" className="nav-link" onClick={onNavigate}><Icon name="docs" className="nav-icon" /> Documentation</NavLink>
        <a className="nav-link" href={REPO_URL} target="_blank" rel="noreferrer"><Icon name="github" className="nav-icon" /> Source on GitHub</a>
      </div>
    </nav>
  );
}

function TopBar({ onMenu }: { onMenu: () => void }) {
  const [theme, setTheme] = useTheme();
  const project = useProject((s) => s.project);
  const setProject = useProject((s) => s.setProject);
  const reset = useProject((s) => s.reset);
  const ev = useEvaluation();
  const ready = useEngineReady();
  const fileRef = useRef<HTMLInputElement>(null);
  return (
    <header className="topbar">
      <button className="btn ghost menu-btn" onClick={onMenu} aria-label="Menu"><Icon name="menu" /></button>
      <div style={{ minWidth: 0 }}>
        <div style={{ fontWeight: 700, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{project.meta.name}</div>
        <div className="muted small hide-sm">v{project.meta.version} · {ready ? 'Rust engine loaded (WebAssembly)' : 'loading engine…'}</div>
      </div>
      {ev.data && (
        <div className="row hide-sm" style={{ marginLeft: 8 }}>
          <span className="chip pass">✓ {ev.data.pass}</span>
          {ev.data.fail - (ev.data.open_goals ?? 0) > 0 && <span className="chip fail">✕ {ev.data.fail - (ev.data.open_goals ?? 0)}</span>}
          {(ev.data.open_goals ?? 0) > 0 && <span className="chip warn" title="stretch goals not yet met">! {ev.data.open_goals} open goal{ev.data.open_goals === 1 ? '' : 's'}</span>}
          {ev.data.unverified > 0 && <span className="chip">○ {ev.data.unverified}</span>}
        </div>
      )}
      <div className="spacer" />
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
      <button className="btn small" onClick={() => fileRef.current?.click()} title="Open a project.json"><Icon name="upload" size={15} /> <span className="hide-sm">Open</span></button>
      <button className="btn small" onClick={() => downloadText(`${project.meta.id || 'project'}.json`, JSON.stringify(project, null, 2), 'application/json')} title="Save project.json"><Icon name="download" size={15} /> <span className="hide-sm">Save</span></button>
      <button className="btn small" onClick={() => confirm('Reset to the Iris reference project? Unsaved edits are lost.') && reset()} title="Reset to Iris"><Icon name="reset" size={15} /></button>
      <button className="btn small" onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')} aria-label="Toggle theme"><Icon name={theme === 'dark' ? 'sun' : 'moon'} size={15} /></button>
    </header>
  );
}

function ScrollTop() {
  const { pathname } = useLocation();
  useEffect(() => window.scrollTo(0, 0), [pathname]);
  return null;
}

/** Error boundary that resets when you navigate to another page. */
function RouteBoundary({ children }: { children: React.ReactNode }) {
  const { pathname } = useLocation();
  return <ErrorBoundary key={pathname}>{children}</ErrorBoundary>;
}

function Loading() {
  return <div className="muted" style={{ padding: 40 }}>Loading lab…</div>;
}

export default function App() {
  const [menu, setMenu] = useState(false);
  const ready = useEngineReady();
  return (
    <HashRouter>
      <ScrollTop />
      <div className="app">
        <Sidebar open={menu} onNavigate={() => setMenu(false)} />
        <div className="main">
          <TopBar onMenu={() => setMenu((m) => !m)} />
          <main className="content">
            {!ready ? (
              <Loading />
            ) : (
              <RouteBoundary>
              <Suspense fallback={<Loading />}>
                <Routes>
                  <Route path="/" element={<Home />} />
                  <Route path="/systems" element={<Systems />} />
                  <Route path="/learn" element={<Learn />} />
                  <Route path="/toolchain" element={<Toolchain />} />
                  <Route path="/versions" element={<Versions />} />
                  <Route path="/docs" element={<Docs />} />
                  <Route path="/docs/:slug" element={<Docs />} />
                  {LABS.map((l) => (
                    <Route key={l.id} path={l.path} element={<l.component />} />
                  ))}
                </Routes>
              </Suspense>
              </RouteBoundary>
            )}
          </main>
        </div>
      </div>
    </HashRouter>
  );
}
