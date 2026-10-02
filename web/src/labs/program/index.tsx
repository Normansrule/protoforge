// Program & Business Lab: turns the product into a program you could run
// and a business you could launch — critical-path schedule, Monte Carlo
// schedule risk, earned value, unit economics, readiness levels and review
// gates. Engine: crates/forge-core/src/program.
import { useSearchParams } from 'react-router-dom';
import { LabPage } from '../../components/LabPage';
import { Tabs } from '../../components/ui';
import { ScheduleTab } from './ScheduleTab';
import { RiskTab } from './RiskTab';
import { EvmTab } from './EvmTab';
import { EconTab } from './EconTab';
import { ReadinessTab } from './ReadinessTab';
import { LearnTab } from './LearnTab';
import './program.css';

type Tab = 'schedule' | 'risk' | 'evm' | 'economics' | 'readiness' | 'learn';
const TABS: { id: Tab; label: string }[] = [
  { id: 'schedule', label: 'Schedule' },
  { id: 'risk', label: 'Schedule risk' },
  { id: 'evm', label: 'Earned value' },
  { id: 'economics', label: 'Unit economics' },
  { id: 'readiness', label: 'Readiness & reviews' },
  { id: 'learn', label: 'How it works' },
];

export default function Lab() {
  const [params, setParams] = useSearchParams();
  const q = params.get('tab') as Tab | null;
  const tab: Tab = q && TABS.some((t) => t.id === q) ? q : 'schedule';
  const setTab = (t: Tab) => setParams((p) => { const n = new URLSearchParams(p); n.set('tab', t); return n; }, { replace: true });
  return (
    <LabPage
      kicker="Test & Launch · program and business"
      title="Program & Business Lab"
      subtitle={
        <>
          Iris as a program you could run and a business you could launch: a work breakdown structure (WBS) derived from the milestones, the Build Guide
          roadmap and the compliance plan; the critical path method (CPM) with animated Gantt and network diagrams; a seeded Monte Carlo schedule-risk
          analysis with P50 / P80 dates; earned value management (EVM); landed unit cost, gross margin, break-even and net present value (NPV); and
          technology / manufacturing readiness levels (TRL / MRL) with the systems-engineering review gates tied to live requirement status.
        </>
      }
      metricPrefixes={['program.']}
    >
      <Tabs tabs={TABS} value={tab} onChange={setTab} />
      {tab === 'schedule' && <ScheduleTab />}
      {tab === 'risk' && <RiskTab />}
      {tab === 'evm' && <EvmTab />}
      {tab === 'economics' && <EconTab />}
      {tab === 'readiness' && <ReadinessTab />}
      {tab === 'learn' && <LearnTab />}
    </LabPage>
  );
}
