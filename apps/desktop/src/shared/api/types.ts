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
/**
 * The template a new project starts from, by its key (decision 0033). The
 * name is older than templates: a template decides more than the workflow.
 */
export type WorkflowTemplate = 'standard' | 'engineering' | 'kanban' | 'scrum' | 'hardware';

export interface Status {
  id: Id;
  name: string;
  category: StatusCategory;
  color: Color;
  /**
   * The work-in-progress limit: the largest number of tasks that should
   * stand in the status at once. Left out or `null`, the status has none. A
   * limit is a signal, never a wall (decision 0033).
   */
  limit?: number | null;
}

/** How full a status with a limit is. */
export type Fill = 'under' | 'at' | 'over';

/** How many tasks stand in a status, against its limit. */
export interface StatusLoad {
  statusId: Id;
  /**
   * The tasks in the status, in the whole project: every area and sprint,
   * whatever a view shows. Subtasks are not counted apart from their task.
   */
  count: number;
  limit: number | null;
  /** `null` when the status has no limit. */
  fill: Fill | null;
}

/** A status a template gives the workflow of a new project. */
export interface TemplateStatus {
  name: string;
  category: StatusCategory;
  color: Color;
  limit: number | null;
}

/** A template a new project can start from (decision 0033). */
export interface ProjectTemplate {
  /** What `NewProject.workflow` names to start from it. */
  key: WorkflowTemplate;
  name: string;
  /** What it is for, in two sentences for the manager. */
  description: string;
  /** The statuses of its workflow, in board order. */
  statuses: TemplateStatus[];
  /** The views the project starts with, in their order. */
  views: ViewKind[];
  /** The areas the project starts with, in their order. */
  areas: string[];
  /** Whether a first sprint is planned, empty and without dates. */
  plansFirstSprint: boolean;
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

/**
 * What an entry of a folder is. A link is one that leads outside the folder
 * of the project, or nowhere: it is listed and not followed.
 */
export type FileEntryKind = 'folder' | 'file' | 'link';

/** One entry of a folder of a project. */
export interface FileEntry {
  name: string;
  kind: FileEntryKind;
  /** The size of a file in bytes; nothing for a folder or a link. */
  size: number | null;
  /** When it was last changed, if the system says. */
  changedAt: Moment | null;
}

/**
 * What lies in a folder of a project: folders first, then by name, without
 * the noise of the tools (`.git`, `node_modules`, `target`). At most 2000
 * entries; `hasMore` says when there were more.
 */
export interface FolderListing {
  entries: FileEntry[];
  hasMore: boolean;
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
  /**
   * The instructions of an AI specialist, in the manager's words: how it
   * works, what it cares about, what it must never do. Lines are separated
   * by `\n`. Empty if it has none; always empty for a person.
   */
  instructions: string;
  /** The color the member is shown in. The backend chooses it. */
  color: Color;
  joinedAt: Moment;
  updatedAt: Moment;
}

/**
 * A role that ships with Teamwright and can be put on a team in one step.
 * The member it becomes is an ordinary member that can be changed.
 */
export interface ReadyMadeSpecialist {
  /** What names the role in the program. Never shown. */
  key: string;
  /** What the member is called on the team. */
  name: string;
  /** What it does, in one sentence. It becomes what the member is on the team for. */
  description: string;
  /** The instructions written for the role. */
  instructions: string;
  /** The model it runs on, by a short name. */
  model: string;
  effort: Effort;
}

/** Who is responsible for a task: the manager, or a member of the team. */
export type Assignee = { kind: 'manager' } | { kind: 'member'; memberId: Id };

/**
 * A part of a project with a job of its own, such as the board, the
 * firmware or the enclosure of a device. A project has one or more areas,
 * in the order the manager gave them, and starts with one called "General".
 * Every task is in exactly one area.
 */
export interface Area {
  id: Id;
  projectId: Id;
  /** Unique within the project, whatever the capitals. At most 40 characters. */
  name: string;
}

/**
 * A short colored word the manager puts on tasks of a project, such as "EMC"
 * or "Waiting for parts". Tags belong to one project.
 */
/** What a custom field holds. It is chosen when the field is made and never changes. */
export type FieldKind = 'text' | 'number' | 'date' | 'yes_no' | 'choice' | 'link';

/** One choice of a choice field: a name and a color, like a tag. */
export interface FieldChoice {
  id: Id;
  /** Unique within the field, whatever the capitals. At most 30 characters. */
  name: string;
  color: Color;
}

/**
 * A property of the tasks of a project that the manager defines, such as
 * "Supplier" or "Unit cost". A task holds at most one value for each.
 */
export interface CustomField {
  id: Id;
  projectId: Id;
  /** Unique within the project, whatever the capitals. At most 40 characters. */
  name: string;
  kind: FieldKind;
  /** The unit shown after the numbers of a number field, at most 8 characters. */
  unit: string | null;
  /** The place among the fields of the project. Lower comes first. */
  position: number;
  /** The choices of a choice field, in their order; empty for the other kinds. */
  choices: FieldChoice[];
}

/** The value a task holds for a custom field, of the kind of the field. */
export type FieldValue =
  /** One line of at most 200 characters. */
  | { kind: 'text'; text: string }
  /** An ordinary number, at most a thousand billion either side of zero. */
  | { kind: 'number'; number: number }
  | { kind: 'date'; date: CalendarDate }
  | { kind: 'yes_no'; yes: boolean }
  | { kind: 'choice'; choiceId: Id }
  /** A web address that starts with https:// and names a host. */
  | { kind: 'link'; address: string };

/**
 * The value of a custom field the tasks a view shows hold: one choice of a
 * choice field, or yes or no. A task that holds nothing counts as no.
 */
export type FieldFilter =
  { kind: 'choice'; fieldId: Id; choiceId: Id } | { kind: 'yes_no'; fieldId: Id; yes: boolean };

export interface Tag {
  id: Id;
  projectId: Id;
  /** Unique within the project, whatever the capitals. At most 30 characters. */
  name: string;
  color: Color;
}

/**
 * An item of the checklist of a task: a short text, done or not done. An
 * item has no owner, no status and no dates; it is lighter than a subtask.
 */
export interface ChecklistItem {
  id: Id;
  /** At most 200 characters, on one line. */
  text: string;
  done: boolean;
}

export interface Task {
  id: Id;
  projectId: Id;
  /**
   * The area of the project the task is in. A subtask is always in the area
   * of the task it is a part of.
   */
  areaId: Id;
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
  /**
   * The sprint the task is planned into, or nothing if it is in the
   * backlog. A subtask is always in the sprint of the task it is a part of.
   */
  sprintId: Id | null;
  /**
   * The story points of the task: how big it is compared with other work,
   * a whole number from 0 to 999. Nothing if it has none.
   */
  points: number | null;
  startOn: CalendarDate | null;
  dueOn: CalendarDate | null;
  /** The position among the tasks of the same status. Lower comes first. */
  rank: number;
  /** The tags of its project the task carries, in no particular order. */
  tagIds: Id[];
  /** The items of its checklist, in their order. Empty when it has none. */
  checklist: ChecklistItem[];
  /** The values it holds for the custom fields of its project, by the identifier of the field. */
  fieldValues: Record<Id, FieldValue>;
  createdAt: Moment;
  updatedAt: Moment;
  completedAt: Moment | null;
}

/**
 * Where a sprint stands. A sprint is planned until it is started, active
 * while it runs, and completed for good. At most one sprint of a project is
 * active.
 */
export type SprintState = 'planned' | 'active' | 'completed';

/**
 * What a sprint counts in: story points if any of its tasks had points, and
 * tasks otherwise. A task without points then counts as no points.
 */
export type SprintUnit = 'points' | 'tasks';

/** What a completed sprint took on and what it finished. */
export interface SprintSummary {
  unit: SprintUnit;
  /** The work of every task that was in the sprint when it was completed. */
  committed: number;
  /** The work of the tasks among them that were finished. */
  completed: number;
}

/**
 * A stretch of time in which the team works on a chosen part of the
 * backlog. A task is in at most one sprint.
 */
export interface Sprint {
  id: Id;
  projectId: Id;
  name: string;
  /** What the sprint is for, in a sentence. May be empty. */
  goal: string;
  /** The first day of the sprint. May be missing while it is planned. */
  startOn: CalendarDate | null;
  /** The last day of the sprint. May be missing while it is planned. */
  endOn: CalendarDate | null;
  state: SprintState;
  createdAt: Moment;
  /** When the sprint was started. Nothing while it is planned. */
  startedAt: Moment | null;
  /** When the sprint was completed. Nothing before. */
  completedAt: Moment | null;
  /** What the sprint achieved. Nothing until it is completed. */
  summary: SprintSummary | null;
}

/** One day of a sprint in its report. */
export interface SprintDay {
  date: CalendarDate;
  /**
   * The work that remained at the end of the day. Nothing for a day that
   * has not come yet, or that came after the sprint was completed.
   */
  remaining: number | null;
  /** All the work the sprint held at the end of the day, finished or not. Nothing for a day that has not come. */
  scope: number | null;
}

/** A task as it stood in a sprint, in the report of the sprint. */
export interface ScopeTask {
  taskId: Id;
  number: number;
  /** Its title at the time. */
  title: string;
  /** Its work in the unit of the sprint. */
  work: number;
  /** Whether it has no points while the sprint counts points: it weighs nothing. */
  hasNoPoints: boolean;
}

/** What changed in what a sprint held, on one of its days. */
export interface ScopeChange {
  date: CalendarDate;
  /** The tasks that came into the sprint that day, by their number. */
  joined: ScopeTask[];
  /** The tasks that went out of it: planned elsewhere, back to the backlog, made a subtask, or deleted. */
  left: ScopeTask[];
  /** How much the work of the tasks that stayed changed, because their points changed. */
  reestimated: number;
}

/** A task that was not finished when its sprint was completed, and where it went. */
export interface MovedOn {
  task: ScopeTask;
  /** The sprint it went to, or nothing for the backlog. */
  toSprintId: Id | null;
}

/**
 * What really happened to a task against its plan, read from the log: when
 * work on it began (it first went into a status of a started category) and
 * when it was finished (it reached the category done), and how late, in
 * days. A late count is null when it does not apply.
 */
export interface TaskActual {
  taskId: Id;
  startedAt: Moment | null;
  /** Null while it is not done; a task opened again counts from its last finish. */
  finishedAt: Moment | null;
  startedOn: CalendarDate | null;
  finishedOn: CalendarDate | null;
  /** Whether it was finished once and opened again. */
  reopened: boolean;
  /** Began this many days after its planned start, or has not begun so many days after it. */
  startedLateDays: number | null;
  /** Whether it has not begun while it is late to begin. */
  notStarted: boolean;
  finishedLateDays: number | null;
  /** Not finished, and its due day was this many days ago. */
  runningLateDays: number | null;
}

/**
 * The pace of a team, read from its completed sprints: the least and the
 * most it finished in each of its last sprints, never one number. With
 * fewer than three completed sprints it is too early to tell.
 */
export type Velocity =
  | { state: 'no_sprints'; finished: []; low: null; high: null; sprints: null }
  /** `finished`: what each completed sprint finished, the latest first. */
  | { state: 'too_early'; finished: number[]; low: null; high: null; sprints: null }
  /** `sprints`: how many of the latest completed sprints the range is read from. */
  | { state: 'known'; finished: []; low: number; high: number; sprints: number };

/** A task a forecast cannot count: it has no points while the work is counted in points. */
export interface NamedTask {
  taskId: Id;
  number: number;
  title: string;
}

/** How the work put into a planned sprint compares with the pace of the team. */
export interface PlannedLoad {
  sprintId: Id;
  /** The work in it, in the unit of the forecast. */
  work: number;
  /** Below, within or above the range the team has finished; null while no pace can be told. */
  load: 'below' | 'within' | 'above' | null;
  withoutPoints: NamedTask[];
}

/** What the completed sprints of a project say about the work ahead. */
export interface SprintForecast {
  velocity: Velocity;
  /** What the work is counted in. */
  unit: SprintUnit;
  /** Each planned sprint against the pace, in the order they are listed. */
  planned: PlannedLoad[];
  /** The unfinished work of the project: backlog, planned sprints and what remains of the running one. */
  ahead: {
    work: number;
    /** How many unfinished tasks there are. */
    tasks: number;
    /** How many sprints it would take at the best and at the worst pace; null while no pace can be told. */
    fewestSprints: number | null;
    /** Null as well when the team once finished nothing in a sprint: at that pace it never ends. */
    mostSprints: number | null;
    withoutPoints: NamedTask[];
  };
}

/**
 * The course of a sprint, for its burndown chart. It is read off every
 * version the tasks of the project have had, so it follows the tasks that
 * joined or left the sprint while it ran.
 */
export interface SprintReport {
  sprintId: Id;
  unit: SprintUnit;
  /** The work the sprint holds now, or held just before it was completed. */
  total: number;
  /** The work the sprint held when it was started; for a planned sprint, what it holds now. */
  committed: number;
  /** Every day from the start date to the end date, in order. Empty without both dates. */
  days: SprintDay[];
  /** The days on which tasks joined or left the sprint, or changed their points, in order. */
  changes: ScopeChange[];
  /** The tasks that moved on when the sprint was completed, by their number. */
  movedOn: MovedOn[];
}

/**
 * The ways the tasks of a project can be shown. A project can have at most
 * one backlog and one dashboard, and several views of every other kind. A
 * new project starts with one view of each of the first six.
 */
export type ViewKind =
  | 'board'
  | 'backlog'
  | 'list'
  | 'gantt'
  | 'calendar'
  | 'dashboard'
  | 'timeline'
  | 'table'
  | 'workload'
  | 'team'
  | 'activity'
  | 'mind_map'
  | 'map'
  | 'whiteboard'
  | 'chat'
  | 'doc'
  | 'embed'
  | 'form'
  | 'gallery'
  | 'report'
  | 'files';

/**
 * Whose tasks a view shows: everybody's, the manager's, those of one member
 * of the team, or those given to nobody.
 */
export type ViewAssignee =
  { kind: 'anyone' } | { kind: 'manager' } | { kind: 'member'; memberId: Id } | { kind: 'nobody' };

/**
 * Which tasks a view shows. A task is shown if it passes the filter or one
 * of its subtasks does; its subtasks are shown with it. Only views of the
 * kinds that show some of the tasks can be filtered: board, list, gantt,
 * calendar, timeline, table, workload, team, mind_map, map and gallery.
 */
export interface ViewFilter {
  assignee: ViewAssignee;
  /** Whether tasks in a status of the category done are left out. */
  hidesFinished: boolean;
  /**
   * The tag the tasks carry, if only the tasks with one tag are shown.
   * Nothing, or left out, shows the tasks with any tag.
   */
  tagId?: Id | null;
  /**
   * The value of a custom field the tasks hold, if only those are shown.
   * Nothing, or left out, shows the tasks whatever they hold.
   */
  fieldFilter?: FieldFilter | null;
}

/**
 * One way the manager looks at the tasks of a project: a kind with a name
 * and a filter of its own. The views of a project are kept in the order the
 * manager gave them, and travel with the project.
 */
export interface View {
  id: Id;
  projectId: Id;
  kind: ViewKind;
  /** Unique within the project, whatever the capitals. */
  name: string;
  filter: ViewFilter;
  /**
   * What the view keeps of its own, such as the text of a doc or the notes
   * of a whiteboard: JSON the interface of its kind writes and reads back.
   * The backend does not read it. At most 2 MiB. Null for a view that has
   * none.
   */
  content: string | null;
  /**
   * What the view groups its tasks by. Only a board and a list can be
   * grouped by anything but `status`.
   */
  grouping: Grouping;
}

/** What a view groups its tasks by. */
export type Grouping = 'status' | 'assignee' | 'priority' | 'area' | 'tag' | 'sprint';

/** What a group of tasks stands for. */
export type GroupKey =
  | { kind: 'status'; id: Id }
  | { kind: 'manager' }
  | { kind: 'member'; id: Id }
  | { kind: 'nobody' }
  | { kind: 'priority'; priority: Priority }
  | { kind: 'area'; id: Id }
  /** The tasks of an area that is gone: there only if data was damaged. */
  | { kind: 'noArea' }
  | { kind: 'tag'; id: Id }
  | { kind: 'noTag' }
  | { kind: 'sprint'; id: Id }
  | { kind: 'backlog' };

/**
 * One group of the tasks of a view: a lane of a board or a section of a
 * list. The tasks are the ones that are not subtasks, in the order of their
 * statuses and then by rank.
 */
export interface TaskGroup {
  key: GroupKey;
  taskIds: Id[];
}

export interface NewView {
  projectId: Id;
  kind: ViewKind;
  /**
   * Left out or empty, the view is named after its kind: "Board", or
   * "Board 2" if the project has a view called "Board" already.
   */
  name?: string;
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

/**
 * One change the manager makes to an open proposal before approving it.
 * Tasks and members are named by their places from 0. Taking a task out
 * takes out what waited for it, and the places after it move up by one.
 * Taking a member out leaves its tasks to nobody.
 */
export type ProposalChange =
  | { kind: 'retitle'; task: number; title: string }
  | { kind: 'describe'; task: number; description: string }
  | { kind: 'estimate'; task: number; estimateMinutes: number | null }
  | { kind: 'prioritize'; task: number; priority: Priority }
  | { kind: 'giveTo'; task: number; owner: ProposedOwner | null }
  | { kind: 'addWait'; task: number; waitsFor: number }
  | { kind: 'removeWait'; task: number; waitsFor: number }
  | { kind: 'addTask'; title: string }
  | { kind: 'removeTask'; task: number }
  | { kind: 'removeMember'; member: number }
  | { kind: 'setProfile'; member: number; model: string | null; effort: Effort | null };

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
  /** How full the context of the session was after the turn, if the program said. */
  context: ContextUse | null;
  /** What the specialist asked the manager to decide in this turn, if anything. */
  decision: DecisionRequest | null;
  /** What the Advisor asked the manager in this turn, if anything. */
  interview: Interview | null;
  /** What the Advisor proposed in this turn, if anything. */
  proposal: Proposal | null;
  /**
   * Whether the turn ended because the limit of the Claude plan was
   * reached. Its state is `failed`, but by no fault of the specialist: the
   * plan refused the work until its limit resets.
   */
  planLimitReached: boolean;
}

/**
 * A window of the Claude plan: `five_hour`, `seven_day`, or another name
 * Claude Code gives, such as `seven_day_opus`.
 */
export type PlanWindowKind = 'five_hour' | 'seven_day' | (string & {});

/** A limit of the Claude plan that is reached. */
export interface ReachedLimit {
  /** When a turn ended because of it. */
  at: Moment;
  /** Which limit it is, or null if Claude Code did not say. */
  window: PlanWindowKind | null;
  /** When it resets, or null when that is not known. */
  resetsAt: Moment | null;
}

/** One window of the plan, as Claude Code last reported it. */
export interface PlanWindow {
  kind: PlanWindowKind;
  /** How much of it is used, from 0 to 1 (sometimes above), or null if not known. */
  used: number | null;
  /** When it starts again from nothing, or null if not known. */
  resetsAt: Moment | null;
  /** Whether the plan allows work in it, in Claude Code's words, such as `allowed`. */
  status: string | null;
}

/**
 * What this computer knows about the limits of its Claude plan. It is a
 * fact about the Claude account of this computer, never synced. Nothing is
 * guessed: a window Claude Code did not report is not listed.
 */
export interface PlanLimit {
  /** The limit, while it is reached; null otherwise. */
  reached: ReachedLimit | null;
  /** The windows as Claude Code last reported them. */
  windows: PlanWindow[];
  /** When Claude Code last reported them, or null if it never did. */
  reportedAt: Moment | null;
}

/**
 * The result of a task, handed over by the specialist who did it to whoever
 * builds on it. A task may have several, one for each turn that published
 * one; the latest counts. It never changes once published.
 */
export interface Handoff {
  id: Id;
  taskId: Id;
  /** The specialist who published it. It may have left the team since. */
  authorId: Id;
  conversationId: Id;
  /** The turn it was published in. */
  turnId: Id;
  summary: string;
  /** The interface contract others may rely on. Empty if there is none. */
  contract: string;
  /**
   * The registered artifacts: paths inside the work folder of the project,
   * with `/` between their parts.
   */
  files: string[];
  publishedAt: Moment;
}

/** What a message between teammates is: a question that wants an answer, or a note. */
export type MessageKind = 'question' | 'note';

/**
 * A question or a note from one specialist to another. It waits until the
 * receiver starts its next turn, and comes with that turn.
 */
export interface TeamMessage {
  id: Id;
  fromId: Id;
  toId: Id;
  conversationId: Id;
  /** The turn it was sent in. */
  turnId: Id;
  kind: MessageKind;
  text: string;
  sentAt: Moment;
  /** The turn of the receiver it came with; null while it waits. */
  deliveredTurnId: Id | null;
  deliveredAt: Moment | null;
}

/** Everything the specialists of a project handed each other, each list the oldest first. */
export interface TeamTalk {
  handoffs: Handoff[];
  messages: TeamMessage[];
}

/** What an entry of project memory is. */
export type MemoryKind = 'decision' | 'fact' | 'contract';

/**
 * Where an entry of project memory came from: the manager wrote it, a
 * specialist remembered it, the manager answered a specialist's decision
 * request, or a specialist published a handoff with an interface contract.
 */
export type MemorySource = 'manager' | 'specialist' | 'decision_request' | 'handoff';

/**
 * An entry of the memory of a project: what the team should not have to ask
 * twice. It never changes in place: a newer entry replaces it, and then it
 * says which (decision 0036).
 */
export interface MemoryEntry {
  id: Id;
  kind: MemoryKind;
  /** A short title of one line. */
  title: string;
  text: string;
  source: MemorySource;
  /**
   * The specialist, conversation and turn it came from; all null when the
   * manager wrote it. The specialist may have left the team since.
   */
  memberId: Id | null;
  conversationId: Id | null;
  turnId: Id | null;
  /** The task it belongs to, or null for the whole project. The task may be gone. */
  taskId: Id | null;
  recordedAt: Moment;
  /** The newer entry that replaced it, or null while it is current. */
  replacedBy: Id | null;
}

/**
 * What a permission is about (decision 0038): one exact command, the
 * commands that start with a program and its subcommand, or one web domain.
 */
export type PermissionKind = 'command' | 'command_prefix' | 'web_domain';

/**
 * How long an allowed permission holds: for the turn the answer starts, for
 * every turn of the specialist on the task, or for every turn of the
 * specialist in the project until the manager takes it back.
 */
export type PermissionScope = 'once' | 'task' | 'always';

/** The manager's answer to a permission request. */
export interface PermissionAnswer {
  decision: 'allowed' | 'refused';
  /** How long it holds, when allowed. */
  scope: PermissionScope | null;
  /** What was allowed, which may be the manager's edit of what was asked. */
  kind: PermissionKind | null;
  what: string | null;
  /** Words of the manager's. May be empty. */
  note: string;
  answeredAt: Moment;
  /**
   * Whether it was answered on this computer. An allowed permission holds
   * only on the computer it was allowed on.
   */
  onThisComputer: boolean;
  /** When an allowed permission was taken back, or null. */
  withdrawnAt: Moment | null;
}

/**
 * A specialist's request to do something it may not do on its own: run a
 * command or fetch from a web domain, with why (decision 0038).
 */
export interface PermissionRequest {
  id: Id;
  projectId: Id;
  /** The specialist who asked. It may have left the team since. */
  memberId: Id;
  conversationId: Id;
  turnId: Id;
  /** The task the conversation is about, or null. */
  taskId: Id | null;
  kind: PermissionKind;
  /** The command, the prefix or the domain, exactly as it would be allowed. */
  what: string;
  /** Why, in the specialist's words. */
  reason: string;
  askedAt: Moment;
  /** The answer, or null while the request waits for the manager. */
  answer: PermissionAnswer | null;
}

/** The manager's answer to a permission request, as the interface sends it. */
export type NewPermissionAnswer =
  | {
      decision: 'allow';
      scope: PermissionScope;
      /** The rule as the manager saw it, perhaps narrowed or edited. */
      kind: PermissionKind;
      what: string;
      note: string;
    }
  | { decision: 'refuse'; note: string };

/** A fact the manager adds to the memory of a project. */
export interface NewMemoryFact {
  projectId: Id;
  title: string;
  text: string;
  /** The task it belongs to, or null for the whole project. */
  taskId: Id | null;
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
  /**
   * The identity of the computer the conversation runs on: the one it was
   * started on, where the session of its specialist lives. It never
   * changes.
   */
  homeComputer: Id;
  /**
   * Whether that computer is this one. Only there can the manager reply,
   * answer, approve, start or stop. Elsewhere the conversation can be read.
   */
  isHere: boolean;
  /** The turns, the oldest first. */
  turns: Turn[];
  /**
   * How full the context of the conversation's session is: what its last
   * turn that reported one says. Nothing while no turn has.
   */
  context: ContextUse | null;
}

/**
 * How full the context of a session is: what its last request held,
 * against the window of the model. The session makes room when it is
 * full, so this can shrink from one turn to the next.
 */
export interface ContextUse {
  /** The tokens the last request held: read, read from the cache and written to it. */
  held: number;
  /** The tokens the window of the model has, if the program said. */
  window: number | null;
  /** Whether it is nearly full (from 80 % of the window): the session will soon make room. */
  isNearlyFull: boolean;
}

/** What one agent, or a group of them, has spent. */
export interface AgentUsage {
  /** The tokens of every turn that has ended, by kind. */
  tokens: TokenCounts;
  /** Every kind of token together. */
  allTokens: number;
  /** How many turns have ended. */
  turns: number;
  /** The model the latest turn ran on, once one named it. */
  lastModel: string | null;
  /**
   * How full the context of the agent is: that of its latest conversation.
   * Nothing for a group, and nothing while that conversation has not said.
   */
  context: ContextUse | null;
}

/** What a member of the team has spent. */
export interface MemberUsage {
  memberId: Id;
  usage: AgentUsage;
}

/** What a project has spent, and who spent it. */
export interface ProjectUsage {
  /** Every member that has had a conversation, the one that spent the most first. */
  members: MemberUsage[];
  /** The Advisor, if the project has spoken with it. */
  advisor: AgentUsage | null;
  /** The members who have left the team since, together, if they had conversations. */
  former: AgentUsage | null;
  /** The whole project. Its context is always nothing. */
  total: AgentUsage;
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
export type TaskChange = {
  at: Moment;
  /**
   * The name of the computer that made the change, when it was another
   * computer whose name is known. Left out for a change made here.
   */
  computer?: string;
} & (
  | { kind: 'created' }
  | { kind: 'renamed'; from: string; to: string }
  | { kind: 'described' }
  /** The task went to another status. A status may have been removed since. */
  | { kind: 'moved'; fromStatusId: Id; toStatusId: Id }
  | { kind: 'assigned'; from: Assignee | null; to: Assignee | null }
  | { kind: 'prioritized'; from: Priority; to: Priority }
  /** The estimate changed. The numbers are minutes. */
  | { kind: 'estimated'; from: number | null; to: number | null }
  /** The task went into another sprint, or back to the backlog (null). A sprint may have been deleted since. */
  | { kind: 'planned'; fromSprintId: Id | null; toSprintId: Id | null }
  /** Its story points changed, or it got or lost them. */
  | { kind: 'pointed'; from: number | null; to: number | null }
  | { kind: 'rescheduled'; from: TaskDates; to: TaskDates }
  /** The task began to wait for another task, which may have been removed since. */
  | { kind: 'now_waits_for'; taskId: Id }
  | { kind: 'no_longer_waits_for'; taskId: Id }
  /** A tag was put on the task or taken off. The tag may have been removed since. */
  | { kind: 'tagged'; tagId: Id }
  | { kind: 'untagged'; tagId: Id }
  /** An item of the checklist was ticked, or is not done any more. */
  | { kind: 'ticked'; item: string }
  | { kind: 'unticked'; item: string }
  /** The value of a custom field was set (from nothing), changed, or cleared (to nothing). The field may have been removed since. */
  | { kind: 'field_value_changed'; fieldId: Id; from: FieldValue | null; to: FieldValue | null }
);

/** A task with a tag put on it by name, and that tag, which may be new. */
export interface TaggedTask {
  task: Task;
  tag: Tag;
}

/**
 * One change in the history of a project: a change of one of its tasks, as
 * the history of the task tells it, with the task it is about. A task
 * deleted since keeps the number and the title it had, and is `gone`.
 */
export type ProjectChange = TaskChange & {
  task: { id: Id; number: number; title: string; gone: boolean };
};

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
  /**
   * The area the task goes into. Left out, the first area of the project.
   * A subtask is in the area of the task it is a part of: left out, it goes
   * there, and another area is refused.
   */
  areaId?: Id;
  estimateMinutes?: number;
  startOn?: CalendarDate;
  dueOn?: CalendarDate;
}

export interface NewSprint {
  projectId: Id;
  /** Left out or empty, the sprint is called "Sprint N". */
  name?: string;
  goal?: string;
  startOn?: CalendarDate | null;
  endOn?: CalendarDate | null;
}

/** What can be changed about a sprint that is planned or active. Every field is replaced. */
export interface SprintChanges {
  name: string;
  goal: string;
  startOn: CalendarDate | null;
  endOn: CalendarDate | null;
}

/** What a sprint is started with: its name and goal as they are to be, and its dates. */
export interface SprintStart {
  name: string;
  goal: string;
  startOn: CalendarDate;
  endOn: CalendarDate;
}

/** What happens when a sprint is completed. */
export interface SprintCompletion {
  /**
   * The planned sprint of the same project that the unfinished tasks go
   * to, or nothing for the backlog.
   */
  moveUnfinishedTo: Id | null;
}

/**
 * Where keeping computers in step through a folder stands, in one word.
 *
 * - `off`: no folder is chosen.
 * - `in_step`: the last round went well.
 * - `writing`: a round is writing or reading right now.
 * - `unreachable`: the folder could not be reached in the last round.
 * - `stopped`: another computer uses this computer's identity, or a new
 *   identity waits for Teamwright to start again.
 */
export type SyncPhase = 'off' | 'in_step' | 'writing' | 'unreachable' | 'stopped';

/**
 * What went wrong in syncing, as a code to decide by:
 *
 * - `sync_folder_unreachable`: the folder could not be reached.
 * - `identity_in_use`: another computer writes under this one's identity.
 * - `restart_needed`: this computer got a new identity, which waits for
 *   Teamwright to start again.
 * - `storage_failed`: the data could not be read or written.
 * - `sync_off`: a round was asked for while no folder is chosen.
 */
export type SyncProblemCode =
  'sync_folder_unreachable' | 'identity_in_use' | 'restart_needed' | 'storage_failed' | 'sync_off';

/**
 * Whether this computer takes in what another computer writes: not decided
 * yet, taken in, or ignored.
 */
export type ComputerAcceptance = 'undecided' | 'accepted' | 'ignored';

/** Another computer that writes into the sync folder. */
export interface Computer {
  id: Id;
  /** The name it gave itself, once its card was read. */
  name: string | null;
  /** The version of Teamwright it runs, once its card was read. */
  version: string | null;
  acceptance: ComputerAcceptance;
  /** When it last wrote into the folder, if it ever did. */
  lastHeard: Moment | null;
  /** Whether its clock is ahead of this computer's by more than five minutes. */
  clockAhead: boolean;
  /** Whether it uses a newer Teamwright, whose files this one cannot read. */
  newerFormat: boolean;
}

/**
 * Where syncing through a folder stands, as the manager sees it. It is also
 * what the event `sync-changed` carries.
 */
export interface SyncState {
  /** The chosen folder, or nothing while syncing is off. */
  folder: string | null;
  /**
   * The identity of this computer. After a new identity it is the new one,
   * already before Teamwright starts again.
   */
  thisComputer: Id;
  /** The name of this computer, once the manager gave one. */
  computerName: string | null;
  /**
   * A name to offer when the manager is asked for one: the name the
   * operating system gives the computer. Only a suggestion, because it
   * often holds a person's name.
   */
  suggestedName: string | null;
  phase: SyncPhase;
  /** When the last round ended, in this run of the program. */
  lastRound: Moment | null;
  /** What went wrong in the last round, in words. */
  problem: string | null;
  /** What went wrong, as a code to decide by; nothing when nothing went wrong. */
  problemCode: SyncProblemCode | null;
  /** The other computers. */
  computers: Computer[];
}

/** A folder of the manager's Google Drive that Google Drive for desktop keeps on this computer. */
export interface DriveFolder {
  path: string;
  /** The address of the account, when the path names it (on macOS). */
  account: string | null;
}

/**
 * What was found of Google Drive on this computer: folders, or no Google
 * Drive for desktop, or a system it is not made for (Linux).
 */
export type DriveFinding = 'found' | 'not_installed' | 'not_for_this_system';

/** What there is of Google Drive on this computer, and whether syncing runs through it. */
export interface GoogleDrive {
  finding: DriveFinding;
  /** The folders found; with several accounts the manager chooses. Empty unless found. */
  folders: DriveFolder[];
  /** The folder of Drive that holds the sync folder, when syncing runs through Drive. */
  syncsThrough: DriveFolder | null;
}

/**
 * What a notice of syncing is about:
 *
 * - `task_renumbered`: a task got a new number, because another computer
 *   gave its number to another task at the same time.
 * - `project_rekeyed`: a project got a new key, for the same reason.
 * - `change_dropped`: a change made here was dropped, because the thing was
 *   deleted on another computer.
 * - `task_moved`: a task moved, because its status was removed on another
 *   computer.
 * - `clock_ahead`: another computer's clock is ahead of this one's.
 */
export type SyncNoticeKind =
  'task_renumbered' | 'project_rekeyed' | 'change_dropped' | 'task_moved' | 'clock_ahead';

/** Something syncing tells the manager. It is kept on this computer until dismissed. */
export interface SyncNotice {
  /** A number of this computer's own; notices never leave it. */
  id: number;
  at: Moment;
  kind: SyncNoticeKind;
  /** The project it is about, if it is about one. */
  projectId: Id | null;
  /** The task it is about, if it is about one. */
  taskId: Id | null;
  /** What happened, in words written for the manager. */
  message: string;
}

/** What the event `data-changed` carries: the projects in which syncing changed something. */
export interface DataChanged {
  projectIds: Id[];
}

/**
 * The name of a theme of the window (decision 0028). It is kept on this
 * computer only: not synced, not logged. `lamplight` is the default.
 */
export type ThemeName =
  | 'lamplight'
  | 'paper'
  | 'dusk'
  | 'slate'
  | 'clear'
  | 'even'
  | 'blossom'
  | 'wing'
  | 'fern'
  | 'tide';

/** A newer version of the program that is out (decision 0043). */
export interface NewerVersion {
  /** The version without a `v`, such as `0.2.0`. */
  version: string;
  /** Whether it is published as a pre-release. */
  isPreRelease: boolean;
  /** The page of the release, an https address. */
  address: string;
}

/**
 * What is known about versions of the program (decision 0043). The choice
 * to look and the time of the last look are kept on this computer only:
 * not synced, not logged. Looking is off until switched on.
 */
export interface UpdateStatus {
  /** The version of this program. */
  thisVersion: string;
  /** Whether the program looks by itself, at most once a day. */
  isOn: boolean;
  /** When it last looked, or null if it never did. */
  lastLook: Moment | null;
  /** Whether the last look got no answer. */
  lastLookFailed: boolean;
  /** The newer version that is known, or null. */
  newer: NewerVersion | null;
}

/** Whether the manager has allowed notifications of the operating system on this computer. */
export type NotificationPermission = 'not_asked' | 'allowed' | 'declined';

/** The kinds of notification, each of which can be turned off. */
export type NotificationKind = 'waiting' | 'finished' | 'stopped';

/**
 * What the manager chose about notifications on this computer. It is kept
 * there only: not synced, not logged.
 */
export interface NotificationChoice {
  permission: NotificationPermission;
  /** A specialist or the Advisor waits for the manager. */
  waiting: boolean;
  /** A specialist finished its work on a task. */
  finished: boolean;
  /** A specialist or the Advisor could not go on. */
  stopped: boolean;
}

/**
 * What the event `notification-pressed` carries: what the notification
 * the manager pressed was about. A field it did not carry is null.
 */
export interface NotificationPressed {
  kind: NotificationKind | null;
  projectId: Id | null;
  taskId: Id | null;
  memberId: Id | null;
  isWithAdvisor: boolean;
}

/**
 * The kinds of events of a project that can be passed on to a Slack
 * channel or a web address. The names are the stable names of the events
 * a web address receives.
 */
export type EventKind =
  | 'task.created'
  | 'task.status_changed'
  | 'task.finished'
  | 'agent.waiting'
  | 'agent.stopped'
  | 'sprint.started'
  | 'sprint.completed';

/**
 * The integrations of a project on this computer: not synced, not logged.
 * An address is a secret, so only its host comes back.
 */
export interface ProjectIntegrations {
  /** The GitHub repository the manager named, `owner/name`, or null to read it from the work folder. */
  repository: string | null;
  slack: SlackChannel | null;
  webAddresses: ListeningAddress[];
}

/** The Slack channel of a project. */
export interface SlackChannel {
  /** The host of its address, the rest hidden. */
  address: string;
  events: EventKind[];
}

/** A web address that listens for the events of a project. */
export interface ListeningAddress {
  /** The host of the address, the rest hidden. */
  address: string;
  /** Whether the body is signed with a secret. */
  isSigned: boolean;
  events: EventKind[];
}

/** Whether the manager's GitHub command line program (`gh`) is there and signed in. */
export interface GitHubSignIn {
  state: 'not_installed' | 'not_signed_in' | 'signed_in';
  /** Where the program is, if it was found. */
  program: string | null;
  /** The account it is signed in as, if it said. */
  account: string | null;
  /** What is missing, in words, or null when nothing is. */
  problem: string | null;
}

/** A pull request or an issue on GitHub. */
export interface GitHubItem {
  kind: 'pull_request' | 'issue';
  number: number;
  title: string;
  state: 'open' | 'draft' | 'merged' | 'closed';
  /** Its page on GitHub. */
  url: string;
  /** The branch of a pull request; null for an issue. */
  branch: string | null;
}

/** GitHub for a project, from what was last read. */
export interface GitHubOverview {
  signIn: GitHubSignIn;
  /** `owner/name`, or null if the project has none. */
  repository: string | null;
  /** Whether the repository was read from the work folder rather than named. */
  isFromWorkFolder: boolean;
  /** When the repository was last read, or null if it was not read since the program started. */
  readAt: string | null;
  linkedPullRequests: number;
  linkedIssues: number;
  /** The pull requests and issues about no task, at most ten. */
  unlinked: GitHubItem[];
  /** How many are about no task in all. */
  unlinkedCount: number;
}

/** The pull requests and issues about one task. */
export interface TaskOnGitHub {
  repository: string | null;
  readAt: string | null;
  items: GitHubItem[];
}

/** What an issue opened for a task would say, and where it would go. Nothing is sent yet. */
export interface IssueDraft {
  repository: string;
  title: string;
  body: string;
}

/** One delivery to a Slack channel or a web address. */
export interface Delivery {
  at: Moment;
  /** The name of the event, or `test`. */
  event: string;
  /** The status of the answer, if there was one. */
  status: number | null;
  succeeded: boolean;
  /** Why it was not taken, in words, if it was not. */
  failure: string | null;
}

/** What was delivered for a project since the program started. Kept in memory only. */
export interface Deliveries {
  /** The last message to the channel, if it could not be delivered. */
  channelFailure: Delivery | null;
  /** For each web address, in their order, its last ten deliveries, newest first. */
  addresses: Delivery[][];
}

/**
 * An earlier conversation the manager held with Claude Code on this
 * computer, as the list of them shows it (decision 0041). It is only read:
 * nothing of it is kept until its digest is.
 */
export interface EarlierConversation {
  /** How the conversation is found again, to make its digest. */
  source: string;
  /** The folder it was held in. */
  heldIn: string;
  /** The title Claude Code or the manager gave it, if any. */
  title: string | null;
  /** The beginning of its first request, on one line, with what looks like a secret hidden. */
  firstRequest: string | null;
  startedAt: Moment | null;
  /** When it last changed. */
  endedAt: Moment | null;
  /** The size of its file: how long it is. */
  sizeBytes: number;
  /** Whether a digest of it is kept with the project already. */
  isKept: boolean;
}

/** The earlier conversations held in one folder, the newest first. */
export interface ConversationsOfFolder {
  folder: string;
  /** Whether the folder is a folder of the project, or lies inside one. */
  isOfProject: boolean;
  conversations: EarlierConversation[];
}

/**
 * A digest made by rules from an earlier conversation, for the manager to
 * read and correct. Sent back, corrected, to keep it.
 */
export interface DigestDraft {
  source: string;
  heldIn: string;
  /** At most 120 characters, on one line. */
  title: string;
  /** Plain lines, at most 8,000 characters. */
  text: string;
  startedAt: Moment | null;
  endedAt: Moment | null;
}

/** A digest kept with a project. The Advisor starts from these. It never changes. */
export interface Digest {
  id: Id;
  projectId: Id;
  source: string;
  heldIn: string;
  title: string;
  text: string;
  startedAt: Moment | null;
  endedAt: Moment | null;
  keptAt: Moment;
}

/**
 * The report of a whole project (decision 0042): one document from its
 * start to the day it is made, worked out by rules from what is stored.
 * There is no state "completed": a project is complete when it has tasks
 * and every one is in a status of the category done, and the report is
 * called "Report of the project so far" until then. Every figure and
 * sentence is the backend's; the interface only lays them out.
 */
export interface ProjectReport {
  projectId: Id;
  name: string;
  key: string;
  /** The manager's description of the project. May be empty. */
  description: string;
  madeOn: CalendarDate;
  begunOn: CalendarDate;
  isComplete: boolean;
  /** "Report of the project", or "Report of the project so far". */
  title: string;
  /** A few sentences that say where the project ended, or stands. */
  summary: string[];
  /** What was to be built, in the Advisor's words, if the manager approved a plan of it. */
  brief: ReportBrief | null;
  /** The work of every area that has tasks. Empty when the project has only one area. */
  byArea: WorkShare[];
  /** The work of the members in the order they joined, then the manager's, then nobody's. */
  byHolder: WorkShare[];
  /** The finished tasks, the one finished first first. */
  finished: ReportTask[];
  plan: PlanAgainstActual;
  /** The sprints, in the order they began. */
  sprints: SprintOutcome[];
  usage: ReportUsage;
  /** The questions the manager answered, the one answered first first. */
  decisions: ReportDecision[];
  /** The latest handoff of every task that has one, in the order of the tasks. */
  handoffs: ReportHandoff[];
  /** The tasks not finished, by number. */
  open: ReportTask[];
  /** The questions that still wait for the manager's answer. */
  waiting: number;
}

export interface ReportBrief {
  summary: string;
  approvedOn: CalendarDate;
}

/** What one area, or one holder of tasks, has finished and has open. */
export interface WorkShare {
  name: string;
  finished: number;
  open: number;
}

/** How a task stands against its due date. */
export type DueOutcome =
  | { kind: 'none' }
  | { kind: 'met' }
  | { kind: 'missed'; days: number }
  | { kind: 'coming' }
  | { kind: 'late'; days: number };

/** A task as the report lists it. */
export interface ReportTask {
  taskId: Id;
  reference: string;
  title: string;
  /** Its area, when the project has more than one. */
  area: string | null;
  /** Who holds it, in words: a member, "The manager" or "Nobody". */
  holder: string;
  status: string | null;
  dueOn: CalendarDate | null;
  finishedOn: CalendarDate | null;
  estimateMinutes: number | null;
  /** How long the turns of the specialists on it ran, in minutes. Nothing without a turn. */
  workedMinutes: number | null;
  due: DueOutcome;
}

/** The plan against what happened: only what the stored data can say. */
export interface PlanAgainstActual {
  finished: number;
  estimated: number;
  estimatedMinutes: number;
  worked: number;
  workedMinutes: number;
  compared: number;
  comparedEstimateMinutes: number;
  comparedWorkedMinutes: number;
  withDueDate: number;
  met: number;
  missed: number;
  late: number;
  /** What the figures say, with what is missing said plainly. */
  sentences: string[];
}

/** How a sprint went. A completed one keeps what it was completed with. */
export interface SprintOutcome {
  name: string;
  goal: string;
  state: SprintState;
  startOn: CalendarDate | null;
  endOn: CalendarDate | null;
  unit: SprintUnit;
  committed: number;
  completed: number;
}

/** What the team used, by who and by model. */
export interface ReportUsage {
  byWho: UsageLine[];
  byModel: UsageLine[];
  total: UsageLine;
}

export interface UsageLine {
  name: string;
  /** Every kind of token together. */
  tokens: number;
  turns: number;
}

/** A question the manager answered. */
export interface ReportDecision {
  askedBy: string;
  /** The reference of the task it was about, if any. */
  task: string | null;
  question: string;
  /** The label of the option chosen, if one was. */
  chosen: string | null;
  /** What the manager wrote. May be empty when an option was chosen. */
  note: string;
  answeredOn: CalendarDate;
}

/** What a specialist handed over for a task. */
export interface ReportHandoff {
  reference: string;
  title: string;
  author: string;
  publishedOn: CalendarDate;
  summary: string;
  contract: string;
  files: string[];
}

/** What a budget in tokens is for. */
export type BudgetScopeKind = 'project' | 'member' | 'task';

/** A budget, named by what it is for. For the project, `targetId` is the project. */
export interface BudgetScope {
  scope: BudgetScopeKind;
  targetId: Id;
}

/**
 * A budget in tokens (decision 0008). It counts every token in all, as the
 * usage of a project does, once a turn has ended.
 */
export interface Budget extends BudgetScope {
  tokens: number;
  used: number;
  /** When the manager was asked about it, while that question is open. */
  askedAt: Moment | null;
  /** The two raises offered when it is used up: half of it, and all of it again. */
  raises: [number, number];
}

/** What one member, or one task, has spent, in tokens in all. */
export interface Spent {
  id: Id;
  tokens: number;
}

/** What was spent in a project, against which the budgets count. */
export interface CoordinatorSpending {
  project: number;
  members: Spent[];
  tasks: Spent[];
}

/** Why something the coordinator could start waits. */
export type WaitReason =
  | 'budget'
  | 'limit'
  | 'busy'
  | 'waits_for_manager'
  | 'waits_for_check'
  | 'failed'
  /** The limit of the Claude plan is reached. */
  | 'plan_limit'
  /** The turn the coordinator gave to go on after the limit ended at it too. */
  | 'plan_limit_again'
  | 'message_limit';

/** Something the coordinator could start, and why it waits. */
export interface CoordinatorWaiting {
  /** The task, or null for a turn to answer what teammates asked. */
  taskId: Id | null;
  memberId: Id;
  reason: WaitReason;
  /** The task that waits to be accepted, for `waits_for_check`. */
  checkTaskId: Id | null;
  /** Why, in words, to follow a colon. */
  words: string;
}

export type CoordinatorEntryKind =
  'started' | 'woke' | 'held' | 'asked' | 'switched_on' | 'switched_off';

/** One entry of the coordinator's record: what it did and why, in one sentence. */
export interface CoordinatorEntry {
  id: Id;
  at: Moment;
  kind: CoordinatorEntryKind;
  text: string;
  taskId: Id | null;
  memberId: Id | null;
}

/**
 * The coordinator of a project (decision 0037): the code that starts work
 * when it is ready. It is off until the manager switches it on, and then
 * runs on the computer where it was switched on.
 */
export interface Coordinator {
  projectId: Id;
  runs: 'here' | 'elsewhere' | 'off';
  /** The name of the other computer it runs on, if known. */
  runsOnName: string | null;
  switchedAt: Moment | null;
  /** How many specialists may work at once. */
  limit: number;
  leastLimit: number;
  mostLimit: number;
  /** How many specialists are at work now, on every computer. */
  atWork: number;
  /** The budgets that are set. */
  budgets: Budget[];
  spent: CoordinatorSpending;
  /** What could start but waits, in the order it would start. */
  waiting: CoordinatorWaiting[];
  /** The latest entries of its record, the newest first. */
  entries: CoordinatorEntry[];
}

/** A question about a used-up budget, as the inbox shows it. */
export interface BudgetQuestion extends BudgetScope {
  projectId: Id;
  projectName: string;
  /** What the budget is called: "the budget of the project". */
  name: string;
  tokens: number;
  used: number;
  askedAt: Moment;
  raises: [number, number];
}

/** The payload of the event that says the coordinator of a project acted. */
export interface CoordinatorChanged {
  projectId: Id;
}

/**
 * How a connector is started: a program of the computer that runs the turn,
 * or a tool server at a web address. A secret is named by the environment
 * variable that holds it, never given (decision 0039).
 */
export type ConnectorStart =
  | {
      kind: 'program';
      program: string;
      arguments: string[];
      /** The environment variables the program is handed, by name. */
      variables: string[];
    }
  | {
      kind: 'address';
      url: string;
      headers: ConnectorHeader[];
    };

/** A header of a connector at a web address, whose value is the secret in a variable. */
export interface ConnectorHeader {
  name: string;
  variable: string;
  /** Whether the value is `Bearer ` and the secret. */
  bearer: boolean;
}

/** A skill of the library of a project. */
export interface LibrarySkill {
  id: Id;
  /** Lower-case letters, digits and hyphens. */
  name: string;
  /** What it is for and when it is used, from its SKILL.md. */
  description: string;
  addedAt: Moment;
  /** Whether this computer keeps its folder. Without it, no turn here is given the skill. */
  here: boolean;
}

/** A connector of the library of a project. */
export interface LibraryConnector {
  id: Id;
  /** Lower-case letters, digits and hyphens. */
  name: string;
  start: ConnectorStart;
  /** The names of its tools that the manager may give. */
  tools: string[];
  addedAt: Moment;
  /** The variables it needs that are not set on this computer. */
  missingVariables: string[];
  /**
   * Why no turn is given it any more, in words, if it breaks a rule newer than it, such as a
   * program named by a relative path.
   */
  problem: string | null;
}

/** A connector as a specialist is given it: only the tools named. */
export interface GivenConnector {
  connectorId: Id;
  tools: string[];
}

/** What a specialist was given of the library of its project. */
export interface MemberKit {
  memberId: Id;
  skills: Id[];
  connectors: GivenConnector[];
}

/** What the manager gives a specialist. */
export interface KitChoice {
  skills: Id[];
  connectors: GivenConnector[];
}

/** The library of a project, and what each specialist was given of it. */
export interface Library {
  /** By name. */
  skills: LibrarySkill[];
  /** By name. */
  connectors: LibraryConnector[];
  /** A specialist that was given nothing may have none. */
  kits: MemberKit[];
}

/** A connector the manager describes, to add to a library. */
export interface NewConnector {
  name: string;
  start: ConnectorStart;
  tools: string[];
}
