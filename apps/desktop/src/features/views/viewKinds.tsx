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

import type { Assignee, Project, Task, View, ViewKind } from '@/shared/api/types';

import { CalendarView } from '../calendar/CalendarView';
import { DashboardView } from '../dashboard/DashboardView';
import { GanttView } from '../gantt/GanttView';
import { BacklogView } from '../sprints/BacklogView';
import { SprintBoard } from '../sprints/SprintBoard';
import { TaskListView } from '../tasks/TaskListView';

/** What a view of any kind is drawn with. */
export interface KindProps {
  /**
   * The view being drawn: its id, its name, and the content it keeps of its
   * own, which a kind that is a page reads here and writes with
   * `useSetViewContent`.
   */
  view: View;
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
  /** Whether it is a look at the tasks, or a page that the view keeps. */
  group: 'tasks' | 'pages';
  /** Its place in the chooser: the kinds are offered from the smallest on. */
  order: number;
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

/** The six kinds the program began with. */
const FIRST_KINDS: readonly Kind[] = [
  {
    kind: 'board',
    label: 'Board',
    purpose: 'A lane for every status',
    icon: SquareKanban,
    group: 'tasks',
    order: 10,
    component: FilteredBoard,
  },
  {
    kind: 'backlog',
    label: 'Backlog',
    purpose: 'Sprints and the work that waits',
    icon: ListOrdered,
    group: 'tasks',
    order: 12,
    component: BacklogView,
  },
  {
    kind: 'list',
    label: 'List',
    purpose: 'Every task in one table',
    icon: Rows3,
    group: 'tasks',
    order: 15,
    component: TaskListView,
  },
  {
    kind: 'gantt',
    label: 'Gantt',
    purpose: 'The plan against the days',
    icon: ChartGantt,
    group: 'tasks',
    order: 40,
    component: GanttView,
  },
  {
    kind: 'calendar',
    label: 'Calendar',
    purpose: 'A month with the tasks on their days',
    icon: CalendarDays,
    group: 'tasks',
    order: 45,
    component: CalendarView,
  },
  {
    kind: 'dashboard',
    label: 'Dashboard',
    purpose: 'The state of the project at a glance',
    icon: LayoutDashboard,
    group: 'tasks',
    order: 115,
    component: DashboardView,
  },
];

/**
 * Every other kind lives in a folder of its own, `features/<kind>/kind.tsx`,
 * whose default export is its entry. They are found here by where they lie:
 * a new kind is one more folder, and no shared file changes for it.
 */
const FOUND_KINDS = Object.values(
  import.meta.glob<{ default: Kind }>('../*/kind.tsx', { eager: true }),
).map((found) => found.default);

/** The kinds of view that have an interface, in the order of the chooser. */
export const KINDS: readonly Kind[] = [...FIRST_KINDS, ...FOUND_KINDS].sort(
  (a, b) => a.order - b.order,
);

/** The entry of a kind in the table. */
export function kindOf(kind: ViewKind): Kind {
  // The board stands in for a kind that has no entry yet: one whose
  // interface is still to be built, or one a newer backend knows and this
  // interface does not.
  return KINDS.find((entry) => entry.kind === kind) ?? (FIRST_KINDS[0] as Kind);
}
