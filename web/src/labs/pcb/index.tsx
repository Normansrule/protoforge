import { LabPage } from '../../components/LabPage';
import { Card } from '../../components/ui';

export default function Lab() {
  return (
    <LabPage kicker="Lab" title="PCB Studio" subtitle="Coming online…" metricPrefixes={['pcb.']}>
      <Card title="Under construction">This lab is being built.</Card>
    </LabPage>
  );
}
