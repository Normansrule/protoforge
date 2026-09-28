// Calculators: reflow, stencil, PCB DFM, feeds & speeds, sheet bend, resin exposure.
import { useState } from 'react';
import { BendCalc } from './BendCalc';
import { DfmCalc } from './DfmCalc';
import { FeedsCalc } from './FeedsCalc';
import { ReflowCalc } from './ReflowCalc';
import { ResinCalc } from './ResinCalc';
import { StencilCalc } from './StencilCalc';

type Calc = 'reflow' | 'feeds' | 'bend' | 'stencil' | 'dfm' | 'resin';
const CALCS: { id: Calc; label: string; hint: string }[] = [
  { id: 'reflow', label: 'Reflow profile', hint: 'oven zones → board temperature' },
  { id: 'stencil', label: 'Stencil area ratio', hint: 'IPC-7525 paste release' },
  { id: 'dfm', label: 'PCB fab DFM', hint: 'layout vs capability tier' },
  { id: 'feeds', label: 'Feeds & speeds', hint: 'CNC RPM, feed, power' },
  { id: 'bend', label: 'Sheet-metal bend', hint: 'K-factor flat pattern' },
  { id: 'resin', label: 'Resin exposure', hint: 'working curve, staircase fit' },
];

export function CalcTab({ initial }: { initial?: Calc }) {
  const [c, setC] = useState<Calc>(initial ?? 'reflow');
  return (
    <div className="stack">
      <div className="bd-calc-pick" role="tablist" aria-label="Calculator">
        {CALCS.map((k) => (
          <button key={k.id} role="tab" aria-selected={c === k.id} className={`bd-calc ${c === k.id ? 'active' : ''}`} onClick={() => setC(k.id)}>
            <b>{k.label}</b>
            <span className="small muted">{k.hint}</span>
          </button>
        ))}
      </div>
      {c === 'feeds' && <FeedsCalc />}
      {c === 'bend' && <BendCalc />}
      {c === 'stencil' && <StencilCalc />}
      {c === 'reflow' && <ReflowCalc />}
      {c === 'dfm' && <DfmCalc />}
      {c === 'resin' && <ResinCalc />}
    </div>
  );
}
