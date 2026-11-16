import { Activity } from 'lucide-react';

import type { Kind } from '../views/viewKinds';
import { ActivityView } from './ActivityView';

/** The activity: a log of everything that happened to the tasks, the newest first. */
const activity: Kind = {
  kind: 'activity',
  label: 'Activity',
  purpose: 'Everything that happened, newest first',
  icon: Activity,
  group: 'tasks',
  order: 110,
  component: ActivityView,
};

export default activity;
