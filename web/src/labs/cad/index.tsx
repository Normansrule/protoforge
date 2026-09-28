import { LabPage } from '../../components/LabPage';
import { Card } from '../../components/ui';

export default function Lab() {
  return (
    <LabPage kicker="Lab" title="CAD Studio" subtitle="Coming online…" metricPrefixes={['cad.']}>
      <Card title="Under construction">This lab is being built.</Card>
    </LabPage>
  );
}
