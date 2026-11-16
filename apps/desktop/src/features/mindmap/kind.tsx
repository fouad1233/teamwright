import { Network } from 'lucide-react';

import type { Kind } from '../views/viewKinds';
import { MindMapView } from './MindMapView';

/** The mind map: the project as a tree that grows to the right. */
const mindMap: Kind = {
  kind: 'mind_map',
  label: 'Mind map',
  purpose: 'The project as a tree',
  icon: Network,
  group: 'tasks',
  order: 80,
  component: MindMapView,
};

export default mindMap;
