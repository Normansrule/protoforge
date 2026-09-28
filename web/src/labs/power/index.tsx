// Power & Thermal Lab: energy budget, battery discharge, a two-node thermal
// network with throttling, and the battery capacity trade.
import { useState } from 'react';
import { LabPage } from '../../components/LabPage';
import { Chip, ErrorBox, Tabs } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { useProject } from '../../state/project';
import type { Budget, ScenarioResult } from './types';
import { BudgetTab } from './BudgetTab';
import { DischargeTab } from './DischargeTab';
import { ThermalTab } from './ThermalTab';
import { TradeTab } from './TradeTab';
import './power.css';

type Tab = 'budget' | 'discharge' | 'thermal' | 'trade';
const TAB_KEY = 'protoforge.power.tab';

export default function PowerLab() {
  const [tab, setTab] = useState<Tab>(() => {
    try { return (sessionStorage.getItem(TAB_KEY) as Tab) || 'budget'; } catch { return 'budget'; }
  });
  const choose = (t: Tab) => {
    setTab(t);
    try { sessionStorage.setItem(TAB_KEY, t); } catch { /* ignore */ }
  };
  const project = useProject((s) => s.project);
  const [scenario, setScenario] = useState('reference');
  const budget = useCompute<Budget>('power.budget', { project, scenario });
  const scen = useCompute<ScenarioResult[]>('power.scenarios', { project });
  const scenarios = scen.data ?? [];
  const scenarioName = scenarios.find((s) => s.id === scenario)?.name ?? 'Reference mixed use';
  return (
    <LabPage
      kicker="Electronics · Power & Thermal"
      title="Power & Thermal"
      subtitle={
        <>
          Where every milliwatt goes and where it ends up. A duty-cycle energy budget built from the bill of materials, a lithium-polymer cell
          discharged through a simulated day, and a two-node thermal network that predicts how warm the temple pod gets against the wearer's skin.
        </>
      }
      metricPrefixes={['power.', 'thermal.']}
      actions={scenario !== 'reference' ? <Chip tone="info">scenario: {scenarioName}</Chip> : undefined}
    >
      <Tabs<Tab>
        value={tab}
        onChange={choose}
        tabs={[
          { id: 'budget', label: 'Energy budget' },
          { id: 'discharge', label: 'Discharge' },
          { id: 'thermal', label: 'Thermal' },
          { id: 'trade', label: 'Battery trade' },
        ]}
      />
      <ErrorBox error={budget.error ?? scen.error} />
      <div className="pw-root">
        {tab === 'budget' && <BudgetTab budget={budget.data} scenarios={scenarios} scenario={scenario} setScenario={setScenario} />}
        {tab === 'discharge' && <DischargeTab scenario={scenario} scenarioName={scenarioName} />}
        {tab === 'thermal' && <ThermalTab scenario={scenario} />}
        {tab === 'trade' && <TradeTab scenario={scenario} />}
      </div>
    </LabPage>
  );
}
