import { Gauge } from 'lucide-react';

import type { Kind } from '../views/viewKinds';
import { WorkloadView } from './WorkloadView';

/** The workload: how many hours lie on each person, week by week. */
const workload: Kind = {
  kind: 'workload',
  label: 'Workload',
  purpose: 'How much lies on each person, week by week',
  icon: Gauge,
  group: 'tasks',
  order: 70,
  component: WorkloadView,
};

export default workload;
