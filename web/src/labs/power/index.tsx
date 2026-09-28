import { LabPage } from '../../components/LabPage';
import { Card } from '../../components/ui';

export default function Lab() {
  return (
    <LabPage kicker="Lab" title="Power & Thermal" subtitle="Coming online…" metricPrefixes={['power.']}>
      <Card title="Under construction">This lab is being built.</Card>
    </LabPage>
  );
}
