import { PenLine } from 'lucide-react';

import type { Kind } from '../views/viewKinds';
import { WhiteboardView } from './WhiteboardView';

/** The whiteboard: a free ground for notes and sketches, kept in the view. */
const whiteboard: Kind = {
  kind: 'whiteboard',
  label: 'Whiteboard',
  purpose: 'A free ground for notes and sketches',
  icon: PenLine,
  group: 'pages',
  order: 130,
  component: WhiteboardView,
};

export default whiteboard;
