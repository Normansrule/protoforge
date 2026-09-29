// Build Guide — ideal process settings, calculators, a process wizard, where
// to send each part, the compliance path and the prototype → product roadmap.
import { useSearchParams } from 'react-router-dom';
import { LabPage } from '../../components/LabPage';
import { Explainer, Stat, Tabs } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { useMetricMap, useProject } from '../../state/project';
import { CalcTab } from './CalcTab';
import { ComplianceTab } from './ComplianceTab';
import { RoadmapTab } from './RoadmapTab';
import { SettingsTab } from './SettingsTab';
import { VendorsTab } from './VendorsTab';
import { WizardTab } from './WizardTab';
import './build.css';

type Tab = 'settings' | 'calc' | 'wizard' | 'vendors' | 'compliance' | 'roadmap';
const TABS: { id: Tab; label: string }[] = [
  { id: 'settings', label: 'Settings' },
  { id: 'calc', label: 'Calculators' },
  { id: 'wizard', label: 'Where to send it' },
  { id: 'vendors', label: 'Vendors' },
  { id: 'compliance', label: 'Compliance' },
  { id: 'roadmap', label: 'Roadmap' },
];

export default function Lab() {
  const project = useProject((s) => s.project);
  const metrics = useMetricMap();
  const [params, setParams] = useSearchParams();
  const tab = (TABS.some((t) => t.id === params.get('tab')) ? params.get('tab') : 'settings') as Tab;
  const setTab = (t: Tab) => setParams((p) => { const n = new URLSearchParams(p); n.set('tab', t); return n; }, { replace: true });
  const refs = useCompute<{ label: string; href: string }[]>('build.references', {});
  const ci = metrics.get('build.compliance_items');
  const pv = metrics.get('build.pcb_capability_violations');
  const ar = metrics.get('build.stencil_min_area_ratio');
  return (
    <LabPage
      kicker="Production · Build"
      title="Build Guide"
      subtitle={<>How to actually make {project.meta.name}: typical starting settings for every process (tune on your machine), calculators for machining, sheet metal, stencils and reflow, which process to pick, where to send each file, and the certification path from prototype to a product you can sell.</>}
      metricPrefixes={['build.']}
    >
      <div className="grid grid-4">
        <Stat label="Compliance items" value={ci ? ci.value.toFixed(0) : '…'} note={ci?.note ?? 'for the project markets'} />
        <Stat label="PCB rules beyond fab tier" value={pv ? pv.value.toFixed(0) : '…'} tone={pv ? (pv.value === 0 ? 'pass' : 'fail') : undefined} note={pv?.note ?? ''} />
        <Stat label="Smallest stencil area ratio" value={ar ? ar.value.toFixed(3) : metrics.size ? '—' : '…'} tone={ar ? (ar.value >= 0.66 ? 'pass' : 'warn') : undefined} note={ar ? `${ar.value >= 0.66 ? '✓' : '!'} IPC-7525 wants ≥ 0.66` : metrics.size ? 'no PCB in this project' : ''} />
        <Stat label="Frame material" value={project.design.frame.material} note={`prototype · ${project.design.manufacturing.molding_material} molded at volume`} />
      </div>
      <Tabs<Tab> tabs={TABS} value={tab} onChange={setTab} />
      {tab === 'settings' && <SettingsTab />}
      {tab === 'calc' && <CalcTab />}
      {tab === 'wizard' && <WizardTab />}
      {tab === 'vendors' && <VendorsTab />}
      {tab === 'compliance' && <ComplianceTab />}
      {tab === 'roadmap' && <RoadmapTab />}
      {refs.data && (
        <Explainer title="References" refs={refs.data.map((r) => ({ label: r.label, href: r.href || undefined }))}>
          Settings are typical starting points compiled from these general references and makers' published guidance; standards are cited by number — buy or borrow the current edition before relying on a clause. Nothing here is a vendor quote or a legal opinion.
        </Explainer>
      )}
    </LabPage>
  );
}
