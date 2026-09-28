import { LabPage } from '../../components/LabPage';
import { Card } from '../../components/ui';

export default function Lab() {
  return (
    <LabPage kicker="Lab" title="Manufacturing Hub" subtitle="Coming online…" metricPrefixes={['mfg.']}>
      <Card title="Under construction">This lab is being built.</Card>
    </LabPage>
  );
}
