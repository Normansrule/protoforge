// Every Circuit Lab preset as a Falstad CircuitJS link, plus a converter for
// your own SPICE netlist.
import { useState } from 'react';
import { Card, Chip, DownloadButton, Explainer, Formula } from '../../components/ui';
import { useCompute } from '../../engine/hooks';
import { CircuitJsView } from './CircuitJsView';
import type { FalstadCircuit } from './types';

const SUPPORT: Record<string, { tone: 'pass' | 'warn' | 'fail'; text: string }> = {
  full: { tone: 'pass', text: '✓ exact' },
  approximate: { tone: 'warn', text: '! simplified' },
  partial: { tone: 'fail', text: '✕ partial' },
  none: { tone: 'fail', text: '✕ not converted' },
};

function CircuitCard({ c }: { c: FalstadCircuit }) {
  const ctz = c.url.split('ctz=')[1] ?? '';
  const s = SUPPORT[c.support] ?? SUPPORT.none;
  return (
    <Card
      title={c.title}
      subtitle={`${c.group} · ${c.elements} elements · ${c.text.length} characters → ${ctz.length}-character link`}
      actions={<a className="btn small primary" href={c.url} target="_blank" rel="noreferrer">Open in Falstad ↗</a>}
    >
      <div className="tc-cj"><CircuitJsView text={c.text} height={190} /></div>
      <div className="row" style={{ gap: 6, margin: '8px 0' }}>
        <Chip tone={s.tone}>{s.text}</Chip>
        <Chip tone={c.connectivity_ok ? 'pass' : 'fail'} title="Connectivity re-extracted from the CircuitJS post coordinates matches the SPICE netlist">{c.connectivity_ok ? '✓' : '✕'} connectivity</Chip>
        {c.unsupported.map((u) => <Chip key={u} tone="fail">✕ {u}</Chip>)}
      </div>
      {c.notes.length > 0 && <div className="small muted">{c.notes.map((n, i) => <div key={i}>• {n}</div>)}</div>}
      <div className="row" style={{ marginTop: 8 }}>
        <DownloadButton filename={`${c.id}.circuitjs.txt`} content={c.text} label="CircuitJS text" small />
        <DownloadButton filename={`${c.id}.cir`} content={c.netlist} label="ngspice deck" small />
      </div>
    </Card>
  );
}

const EXAMPLE = `* RC low-pass
V1 in 0 SIN(0 1 1k)
R1 in out 1.6k
C1 out 0 100n
.tran 10u 5m
.end`;

export function CircuitsTab({ circuits }: { circuits: FalstadCircuit[] | null }) {
  const [deck, setDeck] = useState(EXAMPLE);
  const conv = useCompute<FalstadCircuit>('bridge.falstad_netlist', { netlist: deck, title: 'my netlist' });
  const total = circuits?.reduce((s, c) => s + c.text.length, 0) ?? 0;
  const packed = circuits?.reduce((s, c) => s + (c.url.split('ctz=')[1]?.length ?? 0), 0) ?? 0;
  return (
    <div className="stack">
      <div className="grid grid-2">
        <Explainer title="How a circuit fits in a URL" refs={[{ label: 'pieroxy, lz-string — LZ-based compression for JavaScript', href: 'https://github.com/pieroxy/lz-string' }, { label: 'I. Sharp, CircuitJS1 (JavaScript port of the Falstad simulator)', href: 'https://github.com/sharpie7/circuitjs1' }, { label: 'P. Falstad, Circuit Simulator applet', href: 'https://www.falstad.com/circuit/' }, { label: 'T. A. Welch, “A technique for high-performance data compression”, IEEE Computer 17(6), 1984 (LZW)' }]}>
          CircuitJS saves a circuit as text — one line per element, <span className="mono">type x1 y1 x2 y2 flags values…</span> in pixels, and parts connect only where their end points coincide. Protoforge places each part where the Circuit Lab schematic draws it (16 px per grid unit), splits wires at every junction, then packs the text with lz-string's LZW coder into the <span className="mono">?ctz=</span> parameter CircuitJS decompresses on load.
          <Formula>{`LZW: code(w + c) = next free code; width grows 1 bit per power of two\n6 bits → one URL-safe character (A–Z a–z 0–9 + - $)\nall ${circuits?.length ?? 0} circuits: ${total.toLocaleString()} characters → ${packed.toLocaleString()} in links (${total ? ((100 * packed) / total).toFixed(0) : '—'} %)`}</Formula>
          Before a link is offered, its connectivity is re-extracted from the post coordinates the way CircuitJS does it (coincident posts, wires, grounds, same-name labels) and compared with the SPICE netlist.
        </Explainer>
        <Card title="Convert your own netlist" subtitle="Any SPICE deck the Circuit Lab parses (R C L V I D M S) — auto-placed on a grid, nets joined by named nodes">
          <textarea value={deck} onChange={(e) => setDeck(e.target.value)} rows={7} spellCheck={false} className="mono" style={{ width: '100%', fontSize: 12.5 }} aria-label="SPICE netlist" />
          {conv.data && (
            <div className="stack" style={{ gap: 8, marginTop: 8 }}>
              <div className="row">
                <Chip tone={(SUPPORT[conv.data.support] ?? SUPPORT.none).tone}>{(SUPPORT[conv.data.support] ?? SUPPORT.none).text}</Chip>
                <Chip tone={conv.data.connectivity_ok ? 'pass' : 'fail'}>{conv.data.connectivity_ok ? '✓' : '✕'} connectivity</Chip>
                {conv.data.support !== 'none' && <a className="btn small primary" href={conv.data.url} target="_blank" rel="noreferrer">Open in Falstad ↗</a>}
              </div>
              {conv.data.support !== 'none' && <div className="tc-cj"><CircuitJsView text={conv.data.text} height={150} /></div>}
              {conv.data.notes.length > 0 && <div className="small muted">{conv.data.notes.map((n, i) => <div key={i}>• {n}</div>)}</div>}
            </div>
          )}
        </Card>
      </div>
      <div className="grid grid-2">{circuits?.map((c) => <CircuitCard key={c.id} c={c} />)}</div>
    </div>
  );
}
