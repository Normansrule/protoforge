import { useState } from 'react';
import { LabPage } from '../../components/LabPage';
import { Tabs } from '../../components/ui';
import { Gridworld } from './Gridworld';
import { CartPole } from './CartPole';
import { PowerPolicy } from './PowerPolicy';

type Tab = 'grid' | 'cartpole' | 'power';

export default function RlLab() {
  const [tab, setTab] = useState<Tab>(() => (sessionStorage.getItem('protoforge.rl.tab') as Tab) || 'power');
  const choose = (t: Tab) => {
    setTab(t);
    try { sessionStorage.setItem('protoforge.rl.tab', t); } catch { /* ignore */ }
  };
  return (
    <LabPage
      kicker="Motion & Intelligence · Reinforcement learning"
      title="RL Lab"
      subtitle="Watch value spread through a gridworld, teach a table to balance CartPole, then let Q-learning discover a power policy that stretches the Iris battery without shortchanging the wearer."
      metricPrefixes={['rl.']}
    >
      <Tabs
        value={tab}
        onChange={choose}
        tabs={[
          { id: 'power', label: 'Iris power policy' },
          { id: 'grid', label: 'Gridworld' },
          { id: 'cartpole', label: 'CartPole' },
        ]}
      />
      {tab === 'power' && <PowerPolicy />}
      {tab === 'grid' && <Gridworld />}
      {tab === 'cartpole' && <CartPole />}
    </LabPage>
  );
}
