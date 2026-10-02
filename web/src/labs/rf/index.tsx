// RF & Antenna Lab: a 2-D FDTD field solver, antenna patterns, Smith-chart
// matching, controlled-impedance traces and a Bluetooth Low Energy link
// budget for the Iris smart glasses — Rust engine in WebAssembly.
import { useSearchParams } from 'react-router-dom';
import { LabPage } from '../../components/LabPage';
import { Tabs } from '../../components/ui';
import { FieldsTab } from './FieldsTab';
import { AntennasTab } from './AntennasTab';
import { MatchingTab } from './MatchingTab';
import { LinesTab } from './LinesTab';
import { LinkTab } from './LinkTab';
import { LearnTab } from './LearnTab';
import './rf.css';

type Tab = 'fields' | 'antennas' | 'matching' | 'lines' | 'link' | 'learn';
const TABS: { id: Tab; label: string }[] = [
  { id: 'fields', label: 'Fields (FDTD)' },
  { id: 'antennas', label: 'Antennas' },
  { id: 'matching', label: 'Matching' },
  { id: 'lines', label: 'Transmission lines' },
  { id: 'link', label: 'Link budget' },
  { id: 'learn', label: 'How it works' },
];

export default function Lab() {
  const [params, setParams] = useSearchParams();
  const q = params.get('tab') as Tab | null;
  const tab: Tab = q && TABS.some((t) => t.id === q) ? q : 'fields';
  const setTab = (t: Tab) => setParams((p) => { const n = new URLSearchParams(p); n.set('tab', t); return n; }, { replace: true });
  return (
    <LabPage
      kicker="Electronics · radio"
      title="RF & Antenna Lab"
      subtitle={
        <>
          The radio-frequency (RF) side of the Iris Bluetooth Low Energy (BLE) link at 2.4 GHz: watch waves leave an antenna and soak into a head in a
          finite-difference time-domain (FDTD) solver with a perfectly matched layer (PML), size and plot antennas, match the chip antenna to 50 Ω on an
          interactive Smith chart, design the feed as a grounded coplanar waveguide (CPWG) on the real 0.8 mm board, and check the link budget to a phone.
        </>
      }
      metricPrefixes={['rf.']}
    >
      <Tabs tabs={TABS} value={tab} onChange={setTab} />
      {tab === 'fields' && <FieldsTab />}
      {tab === 'antennas' && <AntennasTab />}
      {tab === 'matching' && <MatchingTab />}
      {tab === 'lines' && <LinesTab />}
      {tab === 'link' && <LinkTab />}
      {tab === 'learn' && <LearnTab />}
    </LabPage>
  );
}
