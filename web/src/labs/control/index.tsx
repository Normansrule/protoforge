import { useState } from 'react';
import { LabPage } from '../../components/LabPage';
import { Tabs } from '../../components/ui';
import { HeadTracking } from './HeadTracking';
import { PidDesigner } from './PidDesigner';
import { Pendulum } from './Pendulum';

type Tab = 'head' | 'pid' | 'lqr';

export default function ControlLab() {
  const [tab, setTab] = useState<Tab>(() => (sessionStorage.getItem('protoforge.control.tab') as Tab) || 'head');
  const choose = (t: Tab) => {
    setTab(t);
    try { sessionStorage.setItem('protoforge.control.tab', t); } catch { /* ignore */ }
  };
  return (
    <LabPage
      kicker="Motion & Intelligence · Control"
      title="Control Lab"
      subtitle="Estimate where Iris is looking by fusing a noisy gyro with a motion-corrupted accelerometer, design PID loops with Bode, Nyquist and root-locus views, and balance an inverted pendulum with an LQR on the full nonlinear model."
      metricPrefixes={['control.']}
    >
      <Tabs
        value={tab}
        onChange={choose}
        tabs={[
          { id: 'head', label: 'Head tracking' },
          { id: 'pid', label: 'PID designer' },
          { id: 'lqr', label: 'Inverted pendulum (LQR)' },
        ]}
      />
      {tab === 'head' && <HeadTracking />}
      {tab === 'pid' && <PidDesigner />}
      {tab === 'lqr' && <Pendulum />}
    </LabPage>
  );
}
