import { FolderOpen } from 'lucide-react';

import type { Kind } from '../views/viewKinds';
import { FilesView } from './FilesView';

/** The files: what lies in the folders of the project on this computer. */
const files: Kind = {
  kind: 'files',
  label: 'Files',
  purpose: 'What lies in the folders of the project',
  icon: FolderOpen,
  group: 'pages',
  order: 125,
  component: FilesView,
};

export default files;
