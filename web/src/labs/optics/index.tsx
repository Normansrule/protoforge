import { LabPage } from '../../components/LabPage';
import { Card } from '../../components/ui';

export default function Lab() {
  return (
    <LabPage kicker="Lab" title="Optics Lab" subtitle="Coming online…" metricPrefixes={['optics.']}>
      <Card title="Under construction">This lab is being built.</Card>
    </LabPage>
  );
}
