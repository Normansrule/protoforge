import { useState } from 'react';
import { LabPage } from '../../components/LabPage';
import { Tabs } from '../../components/ui';
import { ArmPlayground } from './ArmPlayground';
import { Trajectories } from './Trajectories';
import { PickPlace } from './PickPlace';
import { MobileRobot } from './MobileRobot';

type Tab = 'arm' | 'traj' | 'pnp' | 'mobile';

export default function RoboticsLab() {
  const [tab, setTab] = useState<Tab>(() => (sessionStorage.getItem('protoforge.robotics.tab') as Tab) || 'pnp');
  const choose = (t: Tab) => {
    setTab(t);
    try { sessionStorage.setItem('protoforge.robotics.tab', t); } catch { /* ignore */ }
  };
  return (
    <LabPage
      kicker="Motion & Intelligence · Robotics"
      title="Robotics Lab"
      subtitle="Kinematics you can drag, trajectories you can compare, a SCARA cell that populates the Iris main board part by part — and a mobile robot chasing a spline with pure pursuit."
      metricPrefixes={['robotics.']}
    >
      <Tabs
        value={tab}
        onChange={choose}
        tabs={[
          { id: 'pnp', label: 'Pick & place' },
          { id: 'arm', label: 'Arm playground' },
          { id: 'traj', label: 'Trajectories' },
          { id: 'mobile', label: 'Mobile robot' },
        ]}
      />
      {tab === 'pnp' && <PickPlace />}
      {tab === 'arm' && <ArmPlayground />}
      {tab === 'traj' && <Trajectories />}
      {tab === 'mobile' && <MobileRobot />}
    </LabPage>
  );
}
