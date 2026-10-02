// Human Factors Lab: anthropometry and fit coverage of the Iris frame, weight
// and balance on the nose and ears, temple clamp force, and the legibility and
// eyebox coverage of the heads-up display — tied to the Iris CAD, components
// and optics through the ergonomics.* engine methods.
import { useSearchParams } from 'react-router-dom';
import { LabPage } from '../../components/LabPage';
import { Tabs } from '../../components/ui';
import { FitTab } from './FitTab';
import { BalanceTab } from './BalanceTab';
import { ClampTab } from './ClampTab';
import { DisplayTab } from './DisplayTab';
import { LearnTab } from './LearnTab';
import '../cad/cadprint.css';
import './ergonomics.css';

type Tab = 'fit' | 'balance' | 'clamp' | 'display' | 'learn';
const TABS: { id: Tab; label: string }[] = [
  { id: 'fit', label: 'Anthropometry & fit' },
  { id: 'balance', label: 'Weight & balance' },
  { id: 'clamp', label: 'Temple clamp' },
  { id: 'display', label: 'Legibility & eyebox' },
  { id: 'learn', label: 'How it works' },
];

export default function Lab() {
  const [params, setParams] = useSearchParams();
  const q = params.get('tab') as Tab | null;
  const tab: Tab = q && TABS.some((t) => t.id === q) ? q : 'fit';
  const setTab = (t: Tab) => setParams((p) => { const n = new URLSearchParams(p); n.set('tab', t); return n; }, { replace: true });
  return (
    <LabPage
      kicker="Optics & Mechanics · people"
      title="Human Factors Lab"
      subtitle={
        <>
          Smart glasses live or die by comfort and fit. Published head and face statistics (ANSUR II) become a seeded Monte Carlo population that tries on the Iris frame; the
          computer-aided design (CAD) masses load the nose and ears; the temples become springs that squeeze the head; and the heads-up display is checked for angular text size,
          ambient contrast ratio (ACR) in sunlight and whether each wearer&apos;s pupil lands inside the eyebox — with interpupillary distance (IPD) percentiles, S/M/L sizing and a
          parametric 3-D head.
        </>
      }
      metricPrefixes={['ergonomics.']}
    >
      <Tabs tabs={TABS} value={tab} onChange={setTab} />
      {tab === 'fit' && <FitTab />}
      {tab === 'balance' && <BalanceTab />}
      {tab === 'clamp' && <ClampTab />}
      {tab === 'display' && <DisplayTab />}
      {tab === 'learn' && <LearnTab />}
    </LabPage>
  );
}
