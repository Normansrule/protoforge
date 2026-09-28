// Chip Lab: logic minimisation, gate-level simulation, the Iris touch-gesture
// FSM, FIR-to-Verilog for the IRIS-DSP co-processor and a Tiny Tapeout
// package. All maths runs in crates/forge-core/src/chip.
import { useState } from 'react';
import { LabPage } from '../../components/LabPage';
import { Tabs } from '../../components/ui';
import { LogicTab } from './LogicTab';
import { SimTab, type LogicInput } from './SimTab';
import { FsmTab } from './FsmTab';
import { FirTab } from './FirTab';
import { TapeoutTab } from './TapeoutTab';
import '../dsp/lab.css';

type Tab = 'logic' | 'sim' | 'fsm' | 'fir' | 'tt';
const TABS: Tab[] = ['logic', 'sim', 'fsm', 'fir', 'tt'];

function initialTab(): Tab {
  try {
    const t = localStorage.getItem('protoforge.chip.tab') as Tab | null;
    if (t && TABS.includes(t)) return t;
  } catch {
    /* ignore */
  }
  return 'logic';
}

export default function ChipLab() {
  const [tab, setTabState] = useState<Tab>(initialTab);
  const setTab = (t: Tab) => {
    setTabState(t);
    try {
      localStorage.setItem('protoforge.chip.tab', t);
    } catch {
      /* ignore */
    }
  };
  const [logic, setLogic] = useState<LogicInput | null>(null);
  return (
    <LabPage
      kicker="Signals & Silicon"
      title="Chip Lab"
      subtitle={
        <>
          From Boolean algebra to silicon: minimise logic, watch gates switch (and glitch), drive the Iris touch-gesture state machine with
          your finger, turn the DSP Lab's decimation filter into Verilog, and package the IRIS-DSP co-processor for a Tiny Tapeout shuttle.
        </>
      }
      metricPrefixes={['chip.']}
    >
      <Tabs
        tabs={[
          { id: 'logic', label: 'Logic minimiser' },
          { id: 'sim', label: 'Gate simulator' },
          { id: 'fsm', label: 'Gesture FSM' },
          { id: 'fir', label: 'FIR → Verilog' },
          { id: 'tt', label: 'Tape-out' },
        ]}
        value={tab}
        onChange={setTab}
      />
      {tab === 'logic' && (
        <LogicTab
          onSimulate={(l) => {
            setLogic(l);
            setTab('sim');
          }}
        />
      )}
      {tab === 'sim' && <SimTab logic={logic} />}
      {tab === 'fsm' && <FsmTab />}
      {tab === 'fir' && <FirTab />}
      {tab === 'tt' && <TapeoutTab />}
    </LabPage>
  );
}
