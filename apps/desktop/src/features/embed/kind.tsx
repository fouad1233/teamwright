import { PanelTop } from 'lucide-react';

import type { Kind } from '../views/viewKinds';
import { EmbedView } from './EmbedView';

/** The embed: a page from elsewhere, shown inside the project. */
const embed: Kind = {
  kind: 'embed',
  label: 'Embed',
  purpose: 'A page from elsewhere, inside the project',
  icon: PanelTop,
  group: 'pages',
  order: 150,
  component: EmbedView,
};

export default embed;
