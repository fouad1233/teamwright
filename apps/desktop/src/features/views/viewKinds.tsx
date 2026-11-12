import {
  CalendarDays,
  ChartGantt,
  LayoutDashboard,
  ListOrdered,
  Rows3,
  SquareKanban,
  type LucideIcon,
} from 'lucide-react';
import type { ComponentType } from 'react';

import type { Assignee, Project, Task, ViewKind } from '@/shared/api/types';

import { CalendarView } from '../calendar/CalendarView';
import { DashboardView } from '../dashboard/DashboardView';
import { GanttView } from '../gantt/GanttView';
import { BacklogView } from '../sprints/BacklogView';
import { SprintBoard } from '../sprints/SprintBoard';
import { TaskListView } from '../tasks/TaskListView';

/** What a view of any kind is drawn with. */
export interface KindProps {
  project: Project;
  /** The tasks the filter of the view lets through, subtasks with their task. */
  tasks: Task[];
  /** Every task of the project, for a view that must know the ones it does not show. */
  everyTask: Task[];
  /** Whether a task is let through by the filter. Nothing when every task is. */
  shows: ((task: Task) => boolean) | undefined;
  /** Who a task opened in the view is given to, so that the filter keeps showing it. */
  assigneeOfNewTasks: Assignee | undefined;
}

/** One kind of view: what it is called, what it is for, its mark, and what draws it. */
export interface Kind {
  kind: ViewKind;
  label: string;
  /** What the kind is for, in a few words, for the chooser. */
  purpose: string;
  icon: LucideIcon;
  component: ComponentType<KindProps>;
}

/**
 * The board keeps every task and is told which ones get a card: a card
 * dropped between the shown ones then lands where it was dropped, and a card
 * still counts a task it waits for that the filter hides.
 */
function FilteredBoard({ project, everyTask, shows, assigneeOfNewTasks }: KindProps) {
  return (
    <SprintBoard
      project={project}
      tasks={everyTask}
      shows={shows}
      assigneeOfNewTasks={assigneeOfNewTasks}
    />
  );
}

/** The kinds of view, in the order of the chooser. A new kind is one more entry. */
export const KINDS: readonly Kind[] = [
  {
    kind: 'board',
    label: 'Board',
    purpose: 'A lane for every status',
    icon: SquareKanban,
    component: FilteredBoard,
  },
  {
    kind: 'backlog',
    label: 'Backlog',
    purpose: 'Sprints and the work that waits',
    icon: ListOrdered,
    component: BacklogView,
  },
  {
    kind: 'list',
    label: 'List',
    purpose: 'Every task in one table',
    icon: Rows3,
    component: TaskListView,
  },
  {
    kind: 'gantt',
    label: 'Gantt',
    purpose: 'The plan against the days',
    icon: ChartGantt,
    component: GanttView,
  },
  {
    kind: 'calendar',
    label: 'Calendar',
    purpose: 'A month with the tasks on their days',
    icon: CalendarDays,
    component: CalendarView,
  },
  {
    kind: 'dashboard',
    label: 'Dashboard',
    purpose: 'The state of the project at a glance',
    icon: LayoutDashboard,
    component: DashboardView,
  },
];

/** The entry of a kind in the table. */
export function kindOf(kind: ViewKind): Kind {
  // The table has an entry for every kind; the board stands in for one
  // that a newer backend might know and this interface does not.
  return KINDS.find((entry) => entry.kind === kind) ?? (KINDS[0] as Kind);
}
