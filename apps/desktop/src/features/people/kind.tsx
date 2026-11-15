import { UsersRound } from 'lucide-react';

import type { Kind } from '../views/viewKinds';
import { PeopleView } from './PeopleView';

/**
 * The view of people: a column for every person and their work. On the wire
 * it is `team`; it is called People because the program already has a place
 * called Team for the members themselves.
 */
const people: Kind = {
  kind: 'team',
  label: 'People',
  purpose: 'Each person and their work',
  icon: UsersRound,
  group: 'tasks',
  order: 60,
  component: PeopleView,
};

export default people;
