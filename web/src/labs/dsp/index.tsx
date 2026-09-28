import { LabPage } from '../../components/LabPage';
import { Card } from '../../components/ui';

export default function Lab() {
  return (
    <LabPage kicker="Lab" title="DSP Lab" subtitle="Coming online…" metricPrefixes={['dsp.']}>
      <Card title="Under construction">This lab is being built.</Card>
    </LabPage>
  );
}
