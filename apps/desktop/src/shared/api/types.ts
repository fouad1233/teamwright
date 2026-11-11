/**
 * The data that travels between the interface and the backend.
 *
 * These types mirror the ones in `src-tauri/src/dto.rs`. The two files are a
 * contract and must be changed together.
 */

/** A UUID as text. */
export type Id = string;

/** A moment in UTC, in RFC 3339 form: `2026-11-11T09:00:00.000Z`. */
export type Moment = string;

/** A calendar date: `2026-11-11`. */
export type CalendarDate = string;

/** What a status means for the progress of a task, whatever it is called. */
export type StatusCategory = 'todo' | 'active' | 'review' | 'done';

/** A color, by name. A status has one. The theme decides the exact shade. */
export type Color = 'gray' | 'blue' | 'purple' | 'amber' | 'green' | 'red' | 'pink' | 'teal';

export type Priority = 'low' | 'normal' | 'high' | 'urgent';

/** The workflows a new project can start with. */
export type WorkflowTemplate = 'standard' | 'engineering';

export interface Status {
  id: Id;
  name: string;
  category: StatusCategory;
  color: Color;
}

export interface Project {
  id: Id;
  /** The prefix of the project's task numbers, as in `IOT-12`. */
  key: string;
  name: string;
  description: string;
  /** The statuses of the project's workflow, in board order. */
  statuses: Status[];
  createdAt: Moment;
  archivedAt: Moment | null;
}

/** Who is responsible for a task: the manager, or a member of the team. */
export type Assignee = { kind: 'manager' } | { kind: 'member'; memberId: Id };

export interface Task {
  id: Id;
  projectId: Id;
  /** The number of the task within its project. */
  number: number;
  title: string;
  description: string;
  statusId: Id;
  priority: Priority;
  assignee: Assignee | null;
  /** The task this one is a part of, if it is a subtask. */
  parentId: Id | null;
  /** The tasks that must be finished before this one can start. */
  blockedBy: Id[];
  estimateMinutes: number | null;
  startOn: CalendarDate | null;
  dueOn: CalendarDate | null;
  /** The position among the tasks of the same status. Lower comes first. */
  rank: number;
  createdAt: Moment;
  updatedAt: Moment;
  completedAt: Moment | null;
}

export interface NewProject {
  name: string;
  /** If it is left out, the backend derives a key from the name. */
  key?: string;
  description?: string;
  workflow: WorkflowTemplate;
}

export interface NewTask {
  projectId: Id;
  title: string;
  description?: string;
  priority?: Priority;
  assignee?: Assignee;
  /** If it is left out, the task starts in the first "todo" status. */
  statusId?: Id;
  parentId?: Id;
  estimateMinutes?: number;
  startOn?: CalendarDate;
  dueOn?: CalendarDate;
}
