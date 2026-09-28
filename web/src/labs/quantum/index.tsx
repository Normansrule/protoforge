// Quantum Lab: circuits, protocols, algorithms, noise, quantum key
// distribution and quantum randomness — a Rust state-vector and
// density-matrix simulator compiled to WebAssembly.
import { useSearchParams } from 'react-router-dom';
import { LabPage } from '../../components/LabPage';
import { Tabs } from '../../components/ui';
import Composer from './Composer';
import Protocols from './Protocols';
import Algorithms from './Algorithms';
import Noise from './Noise';
import Qkd from './Qkd';
import Qrng from './Qrng';
import Learn from './Learn';
import './quantum.css';

type Tab = 'composer' | 'protocols' | 'algorithms' | 'noise' | 'qkd' | 'qrng' | 'learn';
const TABS: { id: Tab; label: string }[] = [
  { id: 'composer', label: 'Circuit composer' },
  { id: 'protocols', label: 'Protocols' },
  { id: 'algorithms', label: 'Algorithms' },
  { id: 'noise', label: 'Noise' },
  { id: 'qkd', label: 'QKD' },
  { id: 'qrng', label: 'QRNG' },
  { id: 'learn', label: 'How it works' },
];

export default function Lab() {
  const [params, setParams] = useSearchParams();
  const q = params.get('tab') as Tab | null;
  const tab: Tab = q && TABS.some((t) => t.id === q) ? q : 'composer';
  const setTab = (t: Tab) => setParams((p) => { const n = new URLSearchParams(p); n.set('tab', t); return n; }, { replace: true });
  return (
    <LabPage
      kicker="Quantum · circuits, links and randomness"
      title="Quantum Lab"
      subtitle={
        <>
          A 14-qubit state-vector and 6-qubit density-matrix simulator in your browser: compose circuits and step through the state on Bloch spheres, export Open Quantum
          Assembly Language (OpenQASM) or open them in Quirk, watch teleportation and entanglement swapping as stories, run a Clauser–Horne–Shimony–Holt (CHSH) Bell test,
          design a fiber quantum key distribution (QKD) link against the fundamental repeaterless bound, and build a quantum random number generator (QRNG) — diode noise,
          analog-to-digital converter (ADC), extractor and National Institute of Standards and Technology (NIST) tests — for a field-programmable gate array (FPGA) board.
        </>
      }
      metricPrefixes={['quantum.']}
    >
      <Tabs tabs={TABS} value={tab} onChange={setTab} />
      {tab === 'composer' && <Composer />}
      {tab === 'protocols' && <Protocols />}
      {tab === 'algorithms' && <Algorithms />}
      {tab === 'noise' && <Noise />}
      {tab === 'qkd' && <Qkd />}
      {tab === 'qrng' && <Qrng />}
      {tab === 'learn' && <Learn />}
    </LabPage>
  );
}
