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

/**
 * A color, by name. A status has one, and so has every member of a team. The
 * theme decides the exact shade.
 */
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

/** What a manager chooses about a status: its name, what it means, and its color. */
export interface StatusProperties {
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

/** Whether a member of a team is an AI agent or a person. */
export type MemberKind = 'agent' | 'human';

/**
 * Someone on the team of a project who can be given tasks. The manager is not
 * a member.
 */
export interface Member {
  id: Id;
  projectId: Id;
  /** What the member is called on the team. */
  name: string;
  kind: MemberKind;
  /** What the member is on the team for. May be empty. */
  responsibility: string;
  /** The color the member is shown in. The backend chooses it. */
  color: Color;
  joinedAt: Moment;
  updatedAt: Moment;
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

/**
 * Whether the program that runs the AI specialists can be used on this
 * computer. Today that program is the user's own Claude Code.
 *
 * - `ready`: it is installed and the user is signed in to it.
 * - `not_installed`: it was not found.
 * - `signed_out`: it is installed, and the user has to sign in to it.
 * - `unavailable`: it is there but could not be asked; `problem` says why.
 */
export interface AgentProgram {
  state: 'ready' | 'not_installed' | 'signed_out' | 'unavailable';
  /** Its version, if it is installed: `2.1.322`. */
  version: string | null;
  /** How the user is signed in, as the program names it: `claude.ai` for a subscription. */
  signInMethod: string | null;
  /** The kind of subscription, if the program says: `max`. */
  subscription: string | null;
  /** What went wrong, in words, when the state is `unavailable`. */
  problem: string | null;
}

/** How one request to a specialist stands. */
export type TurnState = 'running' | 'finished' | 'failed' | 'stopped';

/** One thing that happened while a specialist worked on a request. */
export type Step = { at: Moment } & (
  | {
      /** The specialist wrote something. */
      kind: 'text';
      text: string;
    }
  | {
      /** The specialist used a tool, for example to read or to write a file. */
      kind: 'tool';
      /** The identifier of this use, which stays the same while its outcome changes. */
      id: string;
      /** The name of the tool: `Read`, `Write`. */
      tool: string;
      /** What the tool was used on, short: usually a file. May be empty. */
      summary: string;
      outcome: 'running' | 'done' | 'failed';
    }
  | {
      /** A tool was refused, because the specialist is not allowed to use it that way. */
      kind: 'denied';
      tool: string;
    }
  | {
      /** The program said something about the run itself: a retry, a warning. */
      kind: 'notice';
      message: string;
    }
);

/** Counts of tokens, by kind. */
export interface TokenCounts {
  /** Tokens the model read that were not in its cache. */
  input: number;
  /** Tokens the model wrote, thinking included. */
  output: number;
  /** Tokens read from the cache. */
  cacheRead: number;
  /** Tokens written to the cache. */
  cacheWritten: number;
}

/** One request to a specialist, and what came of it. */
export interface Turn {
  id: Id;
  /** What the specialist was asked to do. */
  request: string;
  /**
   * `brief` when the program wrote the request from a task, and `message`
   * when the manager wrote it in their own words.
   */
  origin: 'brief' | 'message';
  state: TurnState;
  startedAt: Moment;
  endedAt: Moment | null;
  /** What happened, in order. */
  steps: Step[];
  /** The specialist's last answer: its report. Empty until it has one. */
  answer: string;
  /** Why the turn failed or was stopped, in words. */
  failure: string | null;
  /** The model the turn ran on, once that is known. */
  model: string | null;
  /** The tokens the turn used, all models together, once it has ended. */
  tokens: TokenCounts | null;
}

/**
 * A thread of work with one AI specialist: about a task, or a question the
 * manager asked. It is one session of the program that runs the specialist,
 * so the specialist remembers the earlier turns.
 */
export interface Conversation {
  id: Id;
  projectId: Id;
  /** The specialist, or nothing if it has left the team since. */
  memberId: Id | null;
  /** The task the conversation is about, if it is about one. */
  taskId: Id | null;
  startedAt: Moment;
  /** The turns, the oldest first. */
  turns: Turn[];
}

export interface NewProject {
  name: string;
  /** If it is left out, the backend derives a key from the name. */
  key?: string;
  description?: string;
  workflow: WorkflowTemplate;
}

export interface NewMember {
  projectId: Id;
  name: string;
  kind: MemberKind;
  responsibility?: string;
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
