import { LabPage } from '../../components/LabPage';
import { Card } from '../../components/ui';

export default function Lab() {
  return (
    <LabPage kicker="Lab" title="Build Guide" subtitle="Coming online…" metricPrefixes={['build.']}>
      <Card title="Under construction">This lab is being built.</Card>
    </LabPage>
  );
}
