import { FileText } from 'lucide-react';

import type { Kind } from '../views/viewKinds';
import { ReportView } from './ReportView';

/** The report: one page about the project, to print or save as a PDF. */
const report: Kind = {
  kind: 'report',
  label: 'Report',
  purpose: 'One page about the project, to hand to somebody',
  icon: FileText,
  group: 'pages',
  order: 170,
  component: ReportView,
};

export default report;
