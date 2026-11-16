import { FileText } from 'lucide-react';

import type { Kind } from '../views/viewKinds';
import { DocView } from './DocView';

/** The doc: a page of writing that the view keeps, read as a set page and written in Markdown. */
const doc: Kind = {
  kind: 'doc',
  label: 'Doc',
  purpose: 'A page of writing kept with the project',
  icon: FileText,
  group: 'pages',
  order: 120,
  component: DocView,
};

export default doc;
