import { Table2 } from 'lucide-react';

import type { Kind } from '../views/viewKinds';
import { TableView } from './TableView';

/** The table: every field of every task a column, changed in place. */
const table: Kind = {
  kind: 'table',
  label: 'Table',
  purpose: 'Every field a column, to change in place',
  icon: Table2,
  group: 'tasks',
  order: 20,
  component: TableView,
};

export default table;
