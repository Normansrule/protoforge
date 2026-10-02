// Reliability & Test Lab: the verification-and-reliability workbench used
// between design and production — Weibull life-data analysis, accelerated
// life testing, demonstration sample sizes, the Iris reliability block
// diagram and a generated design verification test (DVT) plan. All maths in
// the Rust engine (crates/forge-core/src/reliability).
import { useSearchParams } from 'react-router-dom';
import { LabPage } from '../../components/LabPage';
import { Stat, Tabs } from '../../components/ui';
import { useMetricMap } from '../../state/project';
import { WeibullTab } from './WeibullTab';
import { AltTab } from './AltTab';
import { SampleTab } from './SampleTab';
import { SystemTab } from './SystemTab';
import { DvtTab } from './DvtTab';
import { LearnTab } from './LearnTab';
import { fmt } from './util';
import './reliability.css';

type Tab = 'weibull' | 'alt' | 'sample' | 'system' | 'dvt' | 'learn';
const TABS: { id: Tab; label: string }[] = [
  { id: 'weibull', label: 'Weibull analysis' },
  { id: 'alt', label: 'Accelerated life' },
  { id: 'sample', label: 'Sample size' },
  { id: 'system', label: 'System reliability' },
  { id: 'dvt', label: 'Test plan (DVT)' },
  { id: 'learn', label: 'How it works' },
];

export default function Lab() {
  const [params, setParams] = useSearchParams();
  const q = params.get('tab') as Tab | null;
  const tab: Tab = q && TABS.some((t) => t.id === q) ? q : 'weibull';
  const setTab = (t: Tab) => setParams((p) => { const n = new URLSearchParams(p); n.set('tab', t); return n; }, { replace: true });
  const metrics = useMetricMap();
  const m = (k: string) => metrics.get(k);
  return (
    <LabPage
      kicker="Test & Launch · Lab"
      title="Reliability & Test Lab"
      subtitle={
        <>
          Will it last, and how do you prove it? Fit failure data with suspensions on Weibull paper by rank regression and maximum likelihood (MLE) with
          confidence bounds; turn years of use into weeks of accelerated life testing (ALT); size demonstration tests with the success-run theorem and the
          Lipson extension; build the Iris reliability block diagram (RBD) from its bill of materials and find what dominates; and generate the design
          verification test (DVT) plan — legs, sample sizes, environmental test families, schedule and requirement matrix — ready to export.
        </>
      }
      metricPrefixes={['reliability.']}
    >
      <div className="grid grid-4 rl-metrics">
        <Stat label="Reliability over 2 years" value={m('reliability.r_2yr_pct') ? fmt(m('reliability.r_2yr_pct')!.value, 3) : '…'} unit="%" note="Iris, use profile, all modes" />
        <Stat label="Parts-count MTBF" value={m('reliability.mtbf_h') ? fmt(m('reliability.mtbf_h')!.value, 3) : '…'} unit="h" note="random failures, typical FITs" />
        <Stat label="DVT units" value={m('reliability.dvt_units') ? fmt(m('reliability.dvt_units')!.value) : '…'} unit="units" note="at the plan's R and confidence" />
        <Stat label="System B10 life" value={m('reliability.b10_years') ? fmt(m('reliability.b10_years')!.value, 3) : '…'} unit="years" note="10 % of units failed" />
      </div>
      <Tabs tabs={TABS} value={tab} onChange={setTab} />
      {tab === 'weibull' && <WeibullTab />}
      {tab === 'alt' && <AltTab />}
      {tab === 'sample' && <SampleTab />}
      {tab === 'system' && <SystemTab />}
      {tab === 'dvt' && <DvtTab />}
      {tab === 'learn' && <LearnTab />}
    </LabPage>
  );
}
