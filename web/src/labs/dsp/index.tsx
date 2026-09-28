// DSP Lab: spectra, filter design, the Iris PDM microphone chain and the
// two-microphone beamformer. All maths runs in crates/forge-core/src/dsp.
import { useState } from 'react';
import { LabPage } from '../../components/LabPage';
import { Tabs } from '../../components/ui';
import { SpectrumTab, PRESETS } from './SpectrumTab';
import { FilterTab } from './FilterTab';
import { PdmTab } from './PdmTab';
import { BeamTab } from './BeamTab';
import { useStopOnUnmount } from './audio';
import type { SignalSpec } from './types';
import './lab.css';

type Tab = 'spectrum' | 'filter' | 'pdm' | 'beam';

function initialTab(): Tab {
  try {
    const t = localStorage.getItem('protoforge.dsp.tab');
    if (t === 'spectrum' || t === 'filter' || t === 'pdm' || t === 'beam') return t;
  } catch {
    /* ignore */
  }
  return 'spectrum';
}

export default function DspLab() {
  const [tab, setTabState] = useState<Tab>(initialTab);
  const setTab = (t: Tab) => {
    setTabState(t);
    try {
      localStorage.setItem('protoforge.dsp.tab', t);
    } catch {
      /* ignore */
    }
  };
  const [spec, setSpec] = useState<SignalSpec>(() => structuredClone(PRESETS[0].spec));
  useStopOnUnmount();
  return (
    <LabPage
      kicker="Signals & Silicon"
      title="DSP Lab"
      subtitle={
        <>
          Fourier analysis, FIR and IIR filter design, fixed-point effects — and the two audio subsystems of Iris: the PDM microphone
          decimation chain and the two-microphone differential beamformer that pulls the wearer's voice out of street noise.
        </>
      }
      metricPrefixes={['dsp.']}
    >
      <Tabs
        tabs={[
          { id: 'spectrum', label: 'Spectrum' },
          { id: 'filter', label: 'Filter designer' },
          { id: 'pdm', label: 'PDM mic chain' },
          { id: 'beam', label: 'Beamformer' },
        ]}
        value={tab}
        onChange={setTab}
      />
      {tab === 'spectrum' && <SpectrumTab spec={spec} setSpec={setSpec} />}
      {tab === 'filter' && <FilterTab spec={spec} />}
      {tab === 'pdm' && <PdmTab />}
      {tab === 'beam' && <BeamTab />}
    </LabPage>
  );
}
