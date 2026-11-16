import { ClipboardList } from 'lucide-react';

import type { Kind } from '../views/viewKinds';
import { FormView } from './FormView';

/** The form: a page that makes a task of this project when it is sent. */
const form: Kind = {
  kind: 'form',
  label: 'Form',
  purpose: 'A form that makes a task',
  icon: ClipboardList,
  group: 'pages',
  order: 140,
  component: FormView,
};

export default form;
