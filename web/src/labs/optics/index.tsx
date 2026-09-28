// Optics Lab: first-order design of the Iris heads-up display — ray diagram,
// first-person view, eyebox, legibility and the FOV-vs-resolution design space.
import { useState } from 'react';
import { LabPage } from '../../components/LabPage';
import { Tabs } from '../../components/ui';
import { RayTab } from './RayTab';
import { ViewTab } from './ViewTab';
import { EyeboxTab } from './EyeboxTab';
import { LegibilityTab } from './LegibilityTab';
import { DesignTab } from './DesignTab';
import { LensTab } from './LensTab';
import { QualityTab } from './QualityTab';
import { MaterialsTab } from './MaterialsTab';
import './optics.css';

type Tab = 'ray' | 'view' | 'eyebox' | 'legibility' | 'design' | 'lens' | 'quality' | 'materials';
const TAB_KEY = 'protoforge.optics.tab';

export default function OpticsLab() {
  const [tab, setTab] = useState<Tab>(() => {
    try { return (sessionStorage.getItem(TAB_KEY) as Tab) || 'ray'; } catch { return 'ray'; }
  });
  const choose = (t: Tab) => {
    setTab(t);
    try { sessionStorage.setItem(TAB_KEY, t); } catch { /* ignore */ }
  };
  return (
    <LabPage
      kicker="Optics & Mechanics · Optics"
      title="Optics Lab"
      subtitle={
        <>
          How a 8 mm micro-display becomes a half-metre-wide screen floating in the street. Trace light through the collimating lens and the 45° combiner into
          the eye, then check what matters to a wearer: field of view (FOV), pixels per degree (PPD), where the image appears, how far the eye may move (the eyebox), and whether text survives sunlight.
        </>
      }
      metricPrefixes={['optics.']}
    >
      <Tabs<Tab>
        value={tab}
        onChange={choose}
        tabs={[
          { id: 'ray', label: 'Ray diagram' },
          { id: 'view', label: 'What you see' },
          { id: 'eyebox', label: 'Eyebox' },
          { id: 'legibility', label: 'Legibility' },
          { id: 'design', label: 'Design space' },
          { id: 'lens', label: 'Lens designer' },
          { id: 'quality', label: 'Image quality' },
          { id: 'materials', label: 'Materials' },
        ]}
      />
      <div className="op-root">
        {tab === 'ray' && <RayTab />}
        {tab === 'view' && <ViewTab />}
        {tab === 'eyebox' && <EyeboxTab />}
        {tab === 'legibility' && <LegibilityTab />}
        {tab === 'design' && <DesignTab />}
        {tab === 'lens' && <LensTab />}
        {tab === 'quality' && <QualityTab />}
        {tab === 'materials' && <MaterialsTab />}
      </div>
    </LabPage>
  );
}
