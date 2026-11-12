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

/**
 * Where the specialists of a project work on this computer, and what else
 * they may read there. These are facts about one computer: the same project
 * on another computer has folders of its own.
 */
export interface ProjectPlaces {
  /**
   * The folder the specialists work in. They read, create and change files
   * there, and nowhere else.
   */
  workFolder: string;
  /**
   * Whether the manager chose the work folder. If not, it is the folder
   * Teamwright gives every project.
   */
  isWorkFolderChosen: boolean;
  /**
   * Folders with reference material, such as data sheets or an existing
   * design. The specialists may read them and cannot change them.
   */
  referenceFolders: string[];
}

/** Whether a member of a team is an AI agent or a person. */
export type MemberKind = 'agent' | 'human';

/** How hard the model of a specialist thinks. More effort uses more tokens. */
export type Effort = 'low' | 'medium' | 'high' | 'xhigh' | 'max';

/**
 * How an AI specialist runs. Nothing in a field means: the way the program
 * that runs the specialists does it by default.
 */
export interface MemberProfile {
  /**
   * The model, by a name that program understands: a short name for the
   * latest model of a family, such as `sonnet`, or the full name of a model.
   */
  model: string | null;
  effort: Effort | null;
}

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
  /** The model an AI specialist runs on. Always nothing for a person. */
  model: string | null;
  /** How hard the model of an AI specialist thinks. Always nothing for a person. */
  effort: Effort | null;
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

/** One of the answers a specialist offers the manager for a decision. */
export interface DecisionOption {
  /** The option in a few words. */
  label: string;
  /** What choosing it leads to. */
  consequence: string;
}

/** What the manager decided. */
export interface DecisionAnswer {
  /**
   * The option the manager chose, by its place among the options, counted
   * from 0. Nothing if the manager answered in their own words.
   */
  option: number | null;
  /**
   * What the manager added to the option, or the whole answer if no option
   * was chosen. Empty if an option was chosen and nothing was added.
   */
  note: string;
  at: Moment;
}

/**
 * A question that is the manager's to answer. A specialist asks it instead
 * of guessing, and ends its turn. The answer starts the next turn of the
 * same conversation.
 */
export interface DecisionRequest {
  question: string;
  /** Why it matters. May be empty. */
  reason: string;
  /** Two to four options. */
  options: DecisionOption[];
  /** The option the specialist recommends, by its place from 0, if it recommends one. */
  recommended: number | null;
  /** What the manager decided. Nothing while the question is open. */
  answer: DecisionAnswer | null;
}

/** What the manager answers to a decision request. */
export interface NewDecisionAnswer {
  /** The option that is chosen, by its place from 0, or nothing. */
  option: number | null;
  /** Words of the manager's own. Needed if no option is chosen. */
  note: string;
}

/**
 * One question of an interview: something the Advisor needs to know before
 * it can propose a team and a plan.
 */
export interface InterviewQuestion {
  question: string;
  /** Why the Advisor asks: what the answer changes. May be empty. */
  reason: string;
  /** Two to four options, or none for a question that is answered in words. */
  options: DecisionOption[];
  /** The option the Advisor recommends, by its place from 0, if it recommends one. */
  recommended: number | null;
}

/** The manager's answer to one question of an interview. */
export interface InterviewAnswer {
  /** The option the manager chose, by its place from 0, or nothing. */
  option: number | null;
  /**
   * What the manager added to the option, or the whole answer if no option
   * was chosen. Empty if an option was chosen and nothing was added.
   */
  note: string;
}

/**
 * The questions the Advisor asks the manager before it proposes. It asks
 * them all at once and ends its turn. The answers start the next turn of
 * the same conversation.
 */
export interface Interview {
  /** One to five questions. */
  questions: InterviewQuestion[];
  /** One answer for each question, in the same order. Nothing while the interview is open. */
  answers: InterviewAnswer[] | null;
  /** When the manager answered. Nothing while the interview is open. */
  answeredAt: Moment | null;
}

/** An AI specialist the Advisor proposes to add to the team. */
export interface ProposedMember {
  name: string;
  /** What the member is responsible for. */
  responsibility: string;
  /** Why the project needs this member. */
  reason: string;
  /** The model the member is to run on, or nothing for the default of the program. */
  model: string | null;
  /** How much the member is to think before it acts, or nothing for the default. */
  effort: Effort | null;
}

/**
 * Who a proposed task is given to: the manager, or a member by name. The
 * name is that of a proposed member, or of a member the project already
 * has.
 */
export type ProposedOwner = { kind: 'manager' } | { kind: 'member'; name: string };

/** A task the Advisor proposes to create. */
export interface ProposedTask {
  title: string;
  /** What the result is, and how to tell that it is done. May be empty. */
  description: string;
  /** Who does it, or nothing if nobody is named. */
  owner: ProposedOwner | null;
  /**
   * The size of the task: the minutes a skilled person would need for it,
   * if the Advisor said. It is an estimate of size, not of how long a
   * specialist takes.
   */
  estimateMinutes: number | null;
  priority: Priority;
  /** The tasks of the same proposal that must be finished first, by their places from 0. */
  waitsFor: number[];
}

/**
 * What the Advisor proposes: who to add to the team, and which tasks to
 * create. Nothing of it exists before the manager approves it.
 */
export interface Proposal {
  /** The project in the Advisor's words, for the manager to check. */
  summary: string;
  members: ProposedMember[];
  tasks: ProposedTask[];
  /**
   * When the manager approved it. Nothing while it is not approved. Once it
   * is, the members and the tasks are what the manager approved, which may
   * be less than the Advisor proposed.
   */
  approvedAt: Moment | null;
}

/**
 * A proposal as the manager approves it: the members and the tasks that
 * are left after the manager's changes.
 */
export interface ApprovedProposal {
  members: ProposedMember[];
  tasks: ProposedTask[];
}

/** One request to a specialist or to the Advisor, and what came of it. */
export interface Turn {
  id: Id;
  /** What the specialist was asked to do. */
  request: string;
  /**
   * `brief` when the program wrote the request from a task, `message` when
   * the manager wrote it in their own words, `decision` when it is the
   * manager's answer to a decision request, and `interview` when it is the
   * manager's answers to an interview.
   */
  origin: 'brief' | 'message' | 'decision' | 'interview';
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
  /** What the specialist asked the manager to decide in this turn, if anything. */
  decision: DecisionRequest | null;
  /** What the Advisor asked the manager in this turn, if anything. */
  interview: Interview | null;
  /** What the Advisor proposed in this turn, if anything. */
  proposal: Proposal | null;
}

/**
 * A thread of work with one AI specialist: about a task, or a question the
 * manager asked. Or a thread with the Advisor, in which a project is
 * planned. It is one session of the program that runs them, so the earlier
 * turns are remembered.
 */
export interface Conversation {
  id: Id;
  projectId: Id;
  /**
   * Whether the conversation is with the Advisor. The Advisor is not a
   * member of the team, so such a conversation has no member and no task.
   */
  isWithAdvisor: boolean;
  /**
   * The specialist, or nothing if it has left the team since, or if the
   * conversation is with the Advisor.
   */
  memberId: Id | null;
  /** The task the conversation is about, if it is about one. */
  taskId: Id | null;
  startedAt: Moment;
  /** The turns, the oldest first. */
  turns: Turn[];
}

/** The dates of a task: when it starts and when it is due. Either may be missing. */
export interface TaskDates {
  startOn: CalendarDate | null;
  dueOn: CalendarDate | null;
}

/**
 * One change in the life of a task, for its history. A move up or down
 * within a status is not a change of the task and is not listed.
 */
export type TaskChange = { at: Moment } & (
  | { kind: 'created' }
  | { kind: 'renamed'; from: string; to: string }
  | { kind: 'described' }
  /** The task went to another status. A status may have been removed since. */
  | { kind: 'moved'; fromStatusId: Id; toStatusId: Id }
  | { kind: 'assigned'; from: Assignee | null; to: Assignee | null }
  | { kind: 'prioritized'; from: Priority; to: Priority }
  /** The estimate changed. The numbers are minutes. */
  | { kind: 'estimated'; from: number | null; to: number | null }
  | { kind: 'rescheduled'; from: TaskDates; to: TaskDates }
  /** The task began to wait for another task, which may have been removed since. */
  | { kind: 'now_waits_for'; taskId: Id }
  | { kind: 'no_longer_waits_for'; taskId: Id }
);

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
  /** The model of an AI specialist. Left out, it is the default of the program. */
  model?: string | null;
  /** The effort of an AI specialist. Left out, it is the default of the program. */
  effort?: Effort | null;
}

/** What a conversation with the Advisor starts with. */
export interface NewAdvice {
  /** What the manager wants, in their own words. */
  brief: string;
  /**
   * The model the Advisor runs on in this conversation. Left out, it is the
   * default of the program.
   */
  model?: string | null;
  /** The effort of the Advisor in this conversation. Left out, it is the default. */
  effort?: Effort | null;
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
