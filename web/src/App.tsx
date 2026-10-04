import { Suspense, lazy, useEffect, useState } from 'react';
import { HashRouter, Route, Routes, useLocation } from 'react-router-dom';
import { LABS } from './labs/registry';
import { useEngineReady } from './engine/hooks';
import { ErrorBoundary } from './components/ErrorBoundary';
import { Sidebar, TopBar } from './ux/nav/Chrome';
import { CommandPalette } from './ux/palette/CommandPalette';
import Home from './pages/Home';
import Systems from './pages/Systems';
import Learn from './pages/Learn';
import Toolchain from './pages/Toolchain';
import Versions from './pages/Versions';
import Docs from './pages/Docs';

const StartPage = lazy(() => import('./ux/start/StartPage'));
const NewProductPage = lazy(() => import('./ux/wizard/NewProductPage'));

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
      <CommandPalette />
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
                  <Route path="/start" element={<StartPage />} />
                  <Route path="/new" element={<NewProductPage />} />
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
