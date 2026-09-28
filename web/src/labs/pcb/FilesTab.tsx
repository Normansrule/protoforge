import { useMemo, useState } from 'react';
import { Card, Code, DownloadButton, Stat } from '../../components/ui';
import { useProject } from '../../state/project';
import { GerberSvg, gerberStats } from './GerberView';
import type { FabFile, Layout } from './types';
import { useWorkerCompute } from './useWorker';
import { downloadZip } from './zip';

const LAYER_COLOR: Record<string, string> = {
  F_Cu: 'var(--copper)',
  B_Cu: 'var(--series-1)',
  F_Mask: 'var(--mask)',
  B_Mask: 'var(--mask)',
  F_Paste: 'var(--series-4)',
  F_Silkscreen: 'var(--text)',
  Edge_Cuts: 'var(--series-5)',
  Drill_PTH: 'var(--text-2)',
  Drill_NPTH: 'var(--series-8)',
};

export function FilesTab({ layout }: { layout: Layout }) {
  const project = useProject((s) => s.project);
  const res = useWorkerCompute<FabFile[]>('pcb.fab_files', { project });
  const files = res.data ?? [];
  const layers = files.filter((f) => LAYER_COLOR[f.kind]);
  const [sel, setSel] = useState('F_Cu');
  const cur = files.find((f) => f.kind === sel) ?? layers[0];
  const stats = useMemo(() => (cur ? gerberStats(cur.content, cur.kind) : null), [cur]);
  const byKind = (k: string) => files.find((f) => f.kind === k);
  const base = files.find((f) => f.kind === 'README')?.path.replace('-FAB-README.md', '') ?? 'board';
  const board: [number, number] = [layout.board.width, layout.board.height];
  return (
    <div className="stack">
      <Card
        title="Fabrication files"
        subtitle="Gerber RS-274X with X2 attributes, Excellon drill, JLCPCB-style BOM/CPL — rendered below by parsing the generated files themselves"
        actions={<div className="row">
          <button className="btn primary" disabled={!files.length} onClick={() => downloadZip(`${base}-gerbers.zip`, files.filter((f) => f.path.startsWith('gerbers/')).map((f) => ({ path: f.path.replace('gerbers/', ''), content: f.content })))}>↓ Gerber + drill zip</button>
          {byKind('BOM') && <DownloadButton filename={byKind('BOM')!.path.split('/').pop()!} content={byKind('BOM')!.content} label="BOM.csv" />}
          {byKind('CPL') && <DownloadButton filename={byKind('CPL')!.path.split('/').pop()!} content={byKind('CPL')!.content} label="CPL.csv" />}
          {byKind('README') && <DownloadButton filename={byKind('README')!.path} content={byKind('README')!.content} label="Fab README" />}
          {byKind('Assembly') && <DownloadButton filename={byKind('Assembly')!.path.split('/').pop()!} content={byKind('Assembly')!.content} label="Assembly drawing" />}
        </div>}
      >
        {res.busy && !files.length && <div className="muted small">Generating files…</div>}
        {res.error && <div className="small" style={{ color: 'var(--bad-text)' }}>{res.error}</div>}
        <div className="grid grid-3" style={{ gap: 10 }}>
          {layers.map((f) => (
            <button key={f.kind} onClick={() => setSel(f.kind)} className="card" style={{ padding: 8, cursor: 'pointer', textAlign: 'left', borderColor: sel === f.kind ? 'var(--accent)' : undefined, background: 'var(--surface-1)' }} aria-pressed={sel === f.kind}>
              <div className="row small" style={{ justifyContent: 'space-between', marginBottom: 4 }}><b>{f.kind.replace('_', '.')}</b><span className="muted mono">{(f.content.length / 1024).toFixed(0)} kB</span></div>
              <GerberSvg src={f.content} kind={f.kind} height={70} width={200} color={LAYER_COLOR[f.kind]} board={board} negative={f.kind.includes('Mask')} />
            </button>
          ))}
        </div>
      </Card>
      {cur && stats && (
        <div className="grid grid-sidebar" style={{ alignItems: 'start' }}>
          <Card title={cur.path.split('/').pop()} subtitle={stats.attributes.find((a) => a.startsWith('FileFunction'))?.replace('FileFunction,', 'Function: ') ?? ''}>
            <div className="grid grid-2" style={{ gap: 8 }}>
              <Stat label="Flashes (D03)" value={stats.flashes} />
              <Stat label="Draws (D01)" value={stats.draws} />
              <Stat label="Regions (G36/37)" value={stats.regions} />
              <Stat label="Drill hits" value={stats.holes} />
            </div>
            <div className="small muted" style={{ marginTop: 8 }}>{stats.attributes.map((a) => <div key={a} className="mono">%TF.{a}*%</div>)}</div>
          </Card>
          <Card title="Layer preview">
            <GerberSvg src={cur.content} kind={cur.kind} height={260} width={800} color={LAYER_COLOR[cur.kind] ?? 'var(--text)'} board={board} negative={cur.kind.includes('Mask')} />
          </Card>
        </div>
      )}
      {cur && <Card title="Source"><Code code={cur.content.length > 60000 ? cur.content.slice(0, 60000) + '\n… (truncated in the viewer; the download is complete)' : cur.content} filename={cur.path.split('/').pop()} maxHeight={320} /></Card>}
      <div className="grid grid-2">
        {byKind('BOM') && <Card title="BOM (JLCPCB columns)" subtitle="LCSC Part # left blank on purpose — choose real parts yourself"><Code code={byKind('BOM')!.content} maxHeight={260} /></Card>}
        {byKind('CPL') && <Card title="Pick-and-place (CPL)" subtitle="Check rotations in the vendor's preview"><Code code={byKind('CPL')!.content} maxHeight={260} /></Card>}
      </div>
    </div>
  );
}
