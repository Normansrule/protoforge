// Structures Lab: 3D finite element analysis (FEA) of the real Iris CAD parts,
// SIMP topology optimisation, design calculators, fatigue and vibration,
// tolerance stack-ups and the theory behind them. Heavy solves run in the
// lab's own Web Worker (engineWorker.ts) so the page never blocks.
import { useMemo, useState } from 'react';
import { LabPage } from '../../components/LabPage';
import { Tabs } from '../../components/ui';
import { useProject } from '../../state/project';
import { StrengthTab } from './StrengthTab';
import { TopologyTab } from './TopologyTab';
import { CalculatorsTab } from './CalculatorsTab';
import { FatigueTab } from './FatigueTab';
import { TolerancesTab } from './TolerancesTab';
import { LearnTab } from './LearnTab';
import type { IrisOut } from './types';
import { useWorkerCompute } from './util';
import '../cad/cadprint.css';
import './structures.css';

type TabId = 'strength' | 'topology' | 'calculators' | 'fatigue' | 'tolerances' | 'learn';

export default function Lab() {
  const [tab, setTab] = useState<TabId>('strength');
  const project = useProject((s) => s.project);
  const [fidelity, setFidelity] = useState<'metric' | 'fine'>('metric');
  // The Iris verification (every load case, drop, stack-ups, modes, fatigue)
  // runs in the worker; the metric pass is the one the requirements use.
  const irisInput = useMemo(() => ({ project, fidelity }), [project, fidelity]);
  const iris = useWorkerCompute<IrisOut>('structures.iris', irisInput);
  return (
    <LabPage
      kicker="Optics & Mechanics · Lab"
      title="Structures Lab"
      subtitle={
        <>
          Will it break, bend, wear out or not fit? A voxel finite element analysis (FEA) solver runs the real Iris CAD
          parts under named load cases in WebAssembly; SIMP (Solid Isotropic Material with Penalisation) topology
          optimisation grows lighter parts; closed-form calculators size snap fits, screws, beams, fits and hinges; and
          worst-case, root-sum-square (RSS) and Monte Carlo stack-ups check that the PCB (printed circuit board) and the
          display optics fit every time.
        </>
      }
      metricPrefixes={['structures.']}
    >
      <Tabs<TabId>
        tabs={[
          { id: 'strength', label: 'Strength (3D FEA)' },
          { id: 'topology', label: 'Topology optimisation' },
          { id: 'calculators', label: 'Calculators' },
          { id: 'fatigue', label: 'Fatigue & vibration' },
          { id: 'tolerances', label: 'Tolerances' },
          { id: 'learn', label: 'How it works' },
        ]}
        value={tab}
        onChange={setTab}
      />
      {tab === 'strength' && <StrengthTab iris={iris} fidelity={fidelity} setFidelity={setFidelity} />}
      {tab === 'topology' && <TopologyTab />}
      {tab === 'calculators' && <CalculatorsTab />}
      {tab === 'fatigue' && <FatigueTab iris={iris.data} />}
      {tab === 'tolerances' && <TolerancesTab />}
      {tab === 'learn' && <LearnTab />}
    </LabPage>
  );
}
