// Thermal & Fluids Lab: the spatially resolved companion of the Power Lab's
// lumped thermal network — 3-D heat conduction by finite elements on the real
// Iris CAD, heat-spreader trades, convection correlations and a
// lattice-Boltzmann wind tunnel, all in the Rust engine (WebAssembly).
import { useSearchParams } from 'react-router-dom';
import { LabPage } from '../../components/LabPage';
import { Tabs } from '../../components/ui';
import { ConductionTab } from './ConductionTab';
import { SpreaderTab } from './SpreaderTab';
import { ConvectionTab } from './ConvectionTab';
import { WindTunnelTab } from './WindTunnelTab';
import { LearnTab } from './LearnTab';
import { usePodControls } from './controls';
import '../cad/cadprint.css';
import '../structures/structures.css';
import './fluids.css';

type Tab = 'conduction' | 'spreader' | 'convection' | 'tunnel' | 'learn';
const TABS: { id: Tab; label: string }[] = [
  { id: 'conduction', label: '3-D conduction (FEA)' },
  { id: 'spreader', label: 'Heat spreading trade' },
  { id: 'convection', label: 'Convection' },
  { id: 'tunnel', label: 'Wind tunnel (LBM)' },
  { id: 'learn', label: 'How it works' },
];

export default function Lab() {
  const [params, setParams] = useSearchParams();
  const q = params.get('tab') as Tab | null;
  const tab: Tab = q && TABS.some((t) => t.id === q) ? q : 'conduction';
  const setTab = (t: Tab) => setParams((p) => { const n = new URLSearchParams(p); n.set('tab', t); return n; }, { replace: true });
  const [ctl, setCtl] = usePodControls();
  return (
    <LabPage
      kicker="Optics & Mechanics · Lab"
      title="Thermal & Fluids Lab"
      subtitle={
        <>
          Where does the heat really go? A 3-D finite element analysis (FEA) of heat conduction runs on the real Iris CAD — pod, lid, optics housing and front — with every component's power placed
          where the printed circuit board (PCB) puts it, cooled by convection and radiation from textbook correlations, and compared honestly with the Power Lab's lumped model. Trade graphite and copper
          heat spreaders, size convection with Churchill–Chu and Churchill–Bernstein, and watch a Kármán vortex street form in a lattice-Boltzmann method (LBM) wind tunnel.
        </>
      }
      metricPrefixes={['fluids.', 'thermal.max_skin']}
    >
      <Tabs tabs={TABS} value={tab} onChange={setTab} />
      <div className="fl-lab">
        {tab === 'conduction' && <ConductionTab ctl={ctl} setCtl={setCtl} />}
        {tab === 'spreader' && <SpreaderTab ctl={ctl} setCtl={setCtl} />}
        {tab === 'convection' && <ConvectionTab />}
        {tab === 'tunnel' && <WindTunnelTab />}
        {tab === 'learn' && <LearnTab />}
      </div>
    </LabPage>
  );
}
