import { AlignStartVertical } from 'lucide-react';

import type { Kind } from '../views/viewKinds';
import { TimelineView } from './TimelineView';

/** The timeline: a row for every person, with their tasks along the days. */
const timeline: Kind = {
  kind: 'timeline',
  label: 'Timeline',
  purpose: 'Who works on what, along the days',
  icon: AlignStartVertical,
  group: 'tasks',
  order: 50,
  component: TimelineView,
};

export default timeline;
