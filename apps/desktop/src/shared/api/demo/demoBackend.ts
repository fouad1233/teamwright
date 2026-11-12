import { evenlySpacedRanks, rankBetween, rankFitsBetween } from '@/shared/lib/rank';

import { BackendError, type Backend } from '../backend';
import type {
  Assignee,
  CalendarDate,
  Color,
  Id,
  Member,
  Moment,
  NewMember,
  NewProject,
  NewTask,
  Priority,
  Project,
  Status,
  Task,
  WorkflowTemplate,
} from '../types';

/** Where the demo backend gets the time and new identifiers from. */
export interface DemoEnvironment {
  now(): Date;
  newId(): Id;
}

/** The projects, tasks and team members a demo backend starts with. */
export interface DemoWorkspace {
  projects: Project[];
  tasks: Task[];
  members: Member[];
}

const browserEnvironment: DemoEnvironment = {
  now: () => new Date(),
  newId: () => crypto.randomUUID(),
};

const WORKFLOWS: Record<WorkflowTemplate, readonly Omit<Status, 'id'>[]> = {
  standard: [
    { name: 'To do', category: 'todo', color: 'gray' },
    { name: 'In progress', category: 'active', color: 'blue' },
    { name: 'In review', category: 'review', color: 'amber' },
    { name: 'Done', category: 'done', color: 'green' },
  ],
  engineering: [
    { name: 'To do', category: 'todo', color: 'gray' },
    { name: 'In progress', category: 'active', color: 'blue' },
    { name: 'In revision', category: 'active', color: 'purple' },
    { name: 'In test', category: 'review', color: 'amber' },
    { name: 'Done', category: 'done', color: 'green' },
  ],
};

/**
 * The colors that are handed out to new members of a team, in this order.
 * Gray, amber and red are left out: the interface uses them for nobody, for
 * the manager and for trouble.
 */
const MEMBER_COLORS: readonly Color[] = ['teal', 'blue', 'purple', 'pink', 'green'];

const PROJECT_NAME_MAX_LENGTH = 80;
const TASK_TITLE_MAX_LENGTH = 200;
const MEMBER_NAME_MAX_LENGTH = 60;
const PROJECT_KEY_MAX_LENGTH = 6;
const PROJECT_KEY_PATTERN = /^[A-Z][A-Z0-9]{1,5}$/;
const FALLBACK_PROJECT_KEY = 'PRJ';

/**
 * A backend that lives in the memory of the browser.
 *
 * It lets the interface run without the desktop shell: for interface
 * development, for tests, and for a public demo. It behaves like the real
 * application for everything the interface does, but it is a stand-in. The
 * rules themselves are defined, and tested, in the Rust crates.
 */
export class DemoBackend implements Backend {
  private readonly environment: DemoEnvironment;
  private readonly projects = new Map<Id, Project>();
  private readonly tasks = new Map<Id, Task>();
  private readonly members = new Map<Id, Member>();
  private readonly nextTaskNumbers = new Map<Id, number>();

  constructor(workspace?: DemoWorkspace, environment: DemoEnvironment = browserEnvironment) {
    this.environment = environment;
    for (const project of workspace?.projects ?? []) {
      this.projects.set(project.id, structuredClone(project));
    }
    for (const task of workspace?.tasks ?? []) {
      this.tasks.set(task.id, structuredClone(task));
      const next = Math.max(this.nextTaskNumbers.get(task.projectId) ?? 1, task.number + 1);
      this.nextTaskNumbers.set(task.projectId, next);
    }
    for (const member of workspace?.members ?? []) {
      this.members.set(member.id, structuredClone(member));
    }
  }

  listProjects(): Promise<Project[]> {
    return respond(() => this.allProjects().filter((project) => project.archivedAt === null));
  }

  createProject(newProject: NewProject): Promise<Project> {
    return respond(() => {
      const name = tidyName(newProject.name, 'project name', PROJECT_NAME_MAX_LENGTH);
      const requestedKey = newProject.key?.trim();
      const key = requestedKey ? this.claimKey(requestedKey) : this.unusedKeyFor(name);
      const project: Project = {
        id: this.environment.newId(),
        key,
        name,
        description: newProject.description?.trim() ?? '',
        statuses: WORKFLOWS[newProject.workflow].map((status) => ({
          ...status,
          id: this.environment.newId(),
        })),
        createdAt: this.nowAsText(),
        archivedAt: null,
      };
      this.projects.set(project.id, project);
      return project;
    });
  }

  renameProject(projectId: Id, name: string): Promise<Project> {
    return respond(() => {
      const project = this.requireProject(projectId);
      project.name = tidyName(name, 'project name', PROJECT_NAME_MAX_LENGTH);
      return project;
    });
  }

  describeProject(projectId: Id, description: string): Promise<Project> {
    return respond(() => {
      const project = this.requireProject(projectId);
      project.description = description.trim();
      return project;
    });
  }

  archiveProject(projectId: Id): Promise<Project> {
    return respond(() => {
      const project = this.requireProject(projectId);
      project.archivedAt ??= this.nowAsText();
      return project;
    });
  }

  listTasks(projectId: Id): Promise<Task[]> {
    return respond(() => {
      this.requireProject(projectId);
      return this.tasksOfProject(projectId).sort(asArranged);
    });
  }

  createTask(newTask: NewTask): Promise<Task> {
    return respond(() => {
      const project = this.requireProject(newTask.projectId);
      const title = tidyName(newTask.title, 'task title', TASK_TITLE_MAX_LENGTH);
      requireStartNotAfterDue(newTask.startOn ?? null, newTask.dueOn ?? null);
      const status = newTask.statusId
        ? requireStatus(project, newTask.statusId)
        : initialStatus(project);
      if (newTask.parentId) {
        const parent = this.requireTask(newTask.parentId);
        if (parent.projectId !== project.id) {
          throw new BackendError(
            'different_projects',
            'tasks of different projects cannot be connected',
          );
        }
      }
      this.checkAssignee(newTask.assignee ?? null, project.id);

      const lastInStatus = this.tasksOfStatus(project.id, status.id).at(-1);
      const number = this.nextTaskNumbers.get(project.id) ?? 1;
      this.nextTaskNumbers.set(project.id, number + 1);
      const now = this.nowAsText();
      const task: Task = {
        id: this.environment.newId(),
        projectId: project.id,
        number,
        title,
        description: newTask.description ?? '',
        statusId: status.id,
        priority: newTask.priority ?? 'normal',
        assignee: newTask.assignee ?? null,
        parentId: newTask.parentId ?? null,
        blockedBy: [],
        estimateMinutes: newTask.estimateMinutes ?? null,
        startOn: newTask.startOn ?? null,
        dueOn: newTask.dueOn ?? null,
        rank: rankBetween(lastInStatus?.rank, undefined),
        createdAt: now,
        updatedAt: now,
        completedAt: status.category === 'done' ? now : null,
      };
      this.tasks.set(task.id, task);
      return task;
    });
  }

  renameTask(taskId: Id, title: string): Promise<Task> {
    return respond(() => {
      const tidyTitle = tidyName(title, 'task title', TASK_TITLE_MAX_LENGTH);
      return this.change(taskId, (task) => {
        task.title = tidyTitle;
      });
    });
  }

  describeTask(taskId: Id, description: string): Promise<Task> {
    return respond(() =>
      this.change(taskId, (task) => {
        task.description = description;
      }),
    );
  }

  prioritizeTask(taskId: Id, priority: Priority): Promise<Task> {
    return respond(() =>
      this.change(taskId, (task) => {
        task.priority = priority;
      }),
    );
  }

  assignTask(taskId: Id, assignee: Assignee | null): Promise<Task> {
    return respond(() =>
      this.change(taskId, (task) => {
        this.checkAssignee(assignee, task.projectId);
        task.assignee = assignee;
      }),
    );
  }

  estimateTask(taskId: Id, minutes: number | null): Promise<Task> {
    return respond(() =>
      this.change(taskId, (task) => {
        task.estimateMinutes = minutes;
      }),
    );
  }

  rescheduleTask(
    taskId: Id,
    startOn: CalendarDate | null,
    dueOn: CalendarDate | null,
  ): Promise<Task> {
    return respond(() => {
      requireStartNotAfterDue(startOn, dueOn);
      return this.change(taskId, (task) => {
        task.startOn = startOn;
        task.dueOn = dueOn;
      });
    });
  }

  moveTask(taskId: Id, statusId: Id, index: number): Promise<Task> {
    return respond(() => {
      const task = this.requireTask(taskId);
      const project = this.requireProject(task.projectId);
      const status = requireStatus(project, statusId);
      const now = this.nowAsText();

      // A position counts the tasks that are listed next to this one: the
      // ones with the same parent. Subtasks are shown under their parent.
      const others = this.tasksOfStatus(project.id, statusId).filter(
        (other) => other.id !== taskId && other.parentId === task.parentId,
      );
      const position = Math.min(Math.max(index, 0), others.length);
      const before = others[position - 1]?.rank;
      const after = others[position]?.rank;
      const rank = rankBetween(before, after);

      task.statusId = statusId;
      task.rank = rank;
      task.completedAt = status.category === 'done' ? (task.completedAt ?? now) : null;
      task.updatedAt = now;

      if (!rankFitsBetween(rank, before, after)) {
        // The neighbours are too close together for another rank between
        // them. Space the whole list out again, with the task in place.
        const wholeList = [...others.slice(0, position), task, ...others.slice(position)];
        const ranks = evenlySpacedRanks(wholeList.length);
        wholeList.forEach((member, memberPosition) => {
          member.rank = ranks[memberPosition] ?? member.rank;
          member.updatedAt = now;
        });
      }
      return task;
    });
  }

  addTaskDependency(taskId: Id, blockerId: Id): Promise<Task> {
    return respond(() => {
      const task = this.requireTask(taskId);
      const blocker = this.requireTask(blockerId);
      if (task.projectId !== blocker.projectId) {
        throw new BackendError(
          'different_projects',
          'tasks of different projects cannot be connected',
        );
      }
      if (taskId === blockerId) {
        throw new BackendError('rule_broken', 'a task cannot depend on itself');
      }
      if (this.waitsFor(blockerId, taskId)) {
        const project = this.requireProject(task.projectId);
        const taskName = `${project.key}-${task.number.toString()}`;
        const blockerName = `${project.key}-${blocker.number.toString()}`;
        throw new BackendError(
          'dependency_cycle',
          `${taskName} cannot wait for ${blockerName}, because ${blockerName} already waits for ${taskName}`,
        );
      }
      return this.change(taskId, (changed) => {
        if (!changed.blockedBy.includes(blockerId)) {
          changed.blockedBy.push(blockerId);
        }
      });
    });
  }

  removeTaskDependency(taskId: Id, blockerId: Id): Promise<Task> {
    return respond(() =>
      this.change(taskId, (task) => {
        task.blockedBy = task.blockedBy.filter((id) => id !== blockerId);
      }),
    );
  }

  deleteTask(taskId: Id): Promise<void> {
    return respond(() => {
      const task = this.requireTask(taskId);
      const removed = this.subtreeOf(task);
      const now = this.nowAsText();
      for (const other of this.tasksOfProject(task.projectId)) {
        if (removed.has(other.id)) {
          this.tasks.delete(other.id);
        } else if (other.blockedBy.some((id) => removed.has(id))) {
          other.blockedBy = other.blockedBy.filter((id) => !removed.has(id));
          other.updatedAt = now;
        }
      }
    });
  }

  listMembers(projectId: Id): Promise<Member[]> {
    return respond(() => {
      this.requireProject(projectId);
      return this.teamOf(projectId);
    });
  }

  addMember(newMember: NewMember): Promise<Member> {
    return respond(() => {
      const project = this.requireProject(newMember.projectId);
      const name = tidyName(newMember.name, 'name of the team member', MEMBER_NAME_MAX_LENGTH);
      const team = this.teamOf(project.id);
      refuseNameInUse(team, name);
      const now = this.nowAsText();
      const member: Member = {
        id: this.environment.newId(),
        projectId: project.id,
        name,
        kind: newMember.kind,
        responsibility: newMember.responsibility?.trim() ?? '',
        color: colorForNewcomer(team.map((other) => other.color)),
        joinedAt: now,
        updatedAt: now,
      };
      this.members.set(member.id, member);
      return member;
    });
  }

  renameMember(memberId: Id, name: string): Promise<Member> {
    return respond(() => {
      const member = this.requireMember(memberId);
      const tidy = tidyName(name, 'name of the team member', MEMBER_NAME_MAX_LENGTH);
      refuseNameInUse(
        this.teamOf(member.projectId).filter((other) => other.id !== member.id),
        tidy,
      );
      member.name = tidy;
      member.updatedAt = this.nowAsText();
      return member;
    });
  }

  setMemberResponsibility(memberId: Id, responsibility: string): Promise<Member> {
    return respond(() => {
      const member = this.requireMember(memberId);
      member.responsibility = responsibility.trim();
      member.updatedAt = this.nowAsText();
      return member;
    });
  }

  removeMember(memberId: Id): Promise<void> {
    return respond(() => {
      const member = this.requireMember(memberId);
      const now = this.nowAsText();
      for (const task of this.tasksOfProject(member.projectId)) {
        if (task.assignee?.kind === 'member' && task.assignee.memberId === member.id) {
          task.assignee = null;
          task.updatedAt = now;
        }
      }
      this.members.delete(member.id);
    });
  }

  private allProjects(): Project[] {
    return [...this.projects.values()].sort((first, second) =>
      first.createdAt.localeCompare(second.createdAt),
    );
  }

  private tasksOfProject(projectId: Id): Task[] {
    return [...this.tasks.values()].filter((task) => task.projectId === projectId);
  }

  /** The tasks of one status, in the order the manager arranged them. */
  private tasksOfStatus(projectId: Id, statusId: Id): Task[] {
    return this.tasksOfProject(projectId)
      .filter((task) => task.statusId === statusId)
      .sort(asArranged);
  }

  private requireProject(projectId: Id): Project {
    const project = this.projects.get(projectId);
    if (!project) {
      throw new BackendError('not_found', `there is no project with the identifier ${projectId}`);
    }
    return project;
  }

  /** The team of a project. The member who joined first comes first. */
  private teamOf(projectId: Id): Member[] {
    return [...this.members.values()]
      .filter((member) => member.projectId === projectId)
      .sort((first, second) => first.joinedAt.localeCompare(second.joinedAt));
  }

  private requireMember(memberId: Id): Member {
    const member = this.members.get(memberId);
    if (!member) {
      throw new BackendError(
        'not_found',
        `there is no team member with the identifier ${memberId}`,
      );
    }
    return member;
  }

  /**
   * Makes sure that a task of a project can be given to an assignee: a team
   * member must exist and be on the team of that project.
   */
  private checkAssignee(assignee: Assignee | null, projectId: Id): void {
    if (assignee?.kind !== 'member') {
      return;
    }
    const member = this.requireMember(assignee.memberId);
    if (member.projectId !== projectId) {
      throw new BackendError(
        'assignee_from_another_team',
        'a task can only be given to a member of the team of its own project',
      );
    }
  }

  private requireTask(taskId: Id): Task {
    const task = this.tasks.get(taskId);
    if (!task) {
      throw new BackendError('not_found', `there is no task with the identifier ${taskId}`);
    }
    return task;
  }

  /** Changes a task and records when it was changed. */
  private change(taskId: Id, apply: (task: Task) => void): Task {
    const task = this.requireTask(taskId);
    apply(task);
    task.updatedAt = this.nowAsText();
    return task;
  }

  /** Checks a key the manager asked for and makes sure nobody else has it. */
  private claimKey(requested: string): string {
    const key = requested.toUpperCase();
    if (!PROJECT_KEY_PATTERN.test(key)) {
      throw new BackendError(
        'rule_broken',
        `a project key has 2 to 6 characters, starts with a letter and contains only letters and digits, which '${requested}' does not`,
      );
    }
    if (this.isKeyInUse(key)) {
      throw new BackendError(
        'project_key_taken',
        `the key ${key} is already used by another project`,
      );
    }
    return key;
  }

  /** Derives a key from a project name that no other project uses. */
  private unusedKeyFor(name: string): string {
    const suggested = suggestKey(name);
    if (!this.isKeyInUse(suggested)) {
      return suggested;
    }
    for (let number = 2; number <= 9999; number += 1) {
      const suffix = number.toString();
      const candidate = suggested.slice(0, PROJECT_KEY_MAX_LENGTH - suffix.length) + suffix;
      if (!this.isKeyInUse(candidate)) {
        return candidate;
      }
    }
    return suggested;
  }

  private isKeyInUse(key: string): boolean {
    return [...this.projects.values()].some((project) => project.key === key);
  }

  /** Tells whether one task waits for another, directly or through others. */
  private waitsFor(waitingId: Id, targetId: Id): boolean {
    const toVisit = [waitingId];
    const visited = new Set<Id>();
    for (let current = toVisit.pop(); current !== undefined; current = toVisit.pop()) {
      if (visited.has(current)) {
        continue;
      }
      visited.add(current);
      for (const blockerId of this.tasks.get(current)?.blockedBy ?? []) {
        if (blockerId === targetId) {
          return true;
        }
        toVisit.push(blockerId);
      }
    }
    return false;
  }

  /** The task and every task below it in the hierarchy of subtasks. */
  private subtreeOf(root: Task): Set<Id> {
    const members = new Set<Id>([root.id]);
    const candidates = this.tasksOfProject(root.projectId);
    for (let sizeBefore = 0; sizeBefore !== members.size;) {
      sizeBefore = members.size;
      for (const candidate of candidates) {
        if (candidate.parentId !== null && members.has(candidate.parentId)) {
          members.add(candidate.id);
        }
      }
    }
    return members;
  }

  private nowAsText(): Moment {
    return this.environment.now().toISOString();
  }
}

/**
 * Answers a request the way a real backend does: later rather than at once,
 * with a copy of the data so the caller cannot reach into the backend's
 * state, and with a refusal reported as a rejected promise.
 */
function respond<Result>(work: () => Result): Promise<Result> {
  return new Promise((resolve, reject) => {
    try {
      resolve(structuredClone(work()));
    } catch (failure) {
      reject(failure instanceof Error ? failure : new Error(String(failure)));
    }
  });
}

/**
 * Refuses a name that a member of the team already has. Capital and small
 * letters count as the same.
 */
function refuseNameInUse(team: Member[], name: string): void {
  const wanted = name.toLowerCase();
  if (team.some((member) => member.name.toLowerCase() === wanted)) {
    throw new BackendError('member_name_taken', `the team already has a member called "${name}"`);
  }
}

/**
 * The color for a newcomer to a team: the first one that nobody has, or the
 * one that the fewest members have when all are taken.
 */
function colorForNewcomer(colorsInUse: Color[]): Color {
  const timesUsed = (color: Color) => colorsInUse.filter((used) => used === color).length;
  return MEMBER_COLORS.reduce((best, color) => (timesUsed(color) < timesUsed(best) ? color : best));
}

/** Sorts tasks by rank, and by number where ranks are equal. */
function asArranged(first: Task, second: Task): number {
  return first.rank - second.rank || first.number - second.number;
}

/** Trims a name, puts it on one line, and checks that it is not empty or too long. */
function tidyName(raw: string, field: string, maxLength: number): string {
  const name = raw.trim().split(/\s+/).join(' ');
  if (name === '') {
    throw new BackendError('rule_broken', `the ${field} must not be empty`);
  }
  const length = [...name].length;
  if (length > maxLength) {
    throw new BackendError(
      'rule_broken',
      `the ${field} can have at most ${maxLength.toString()} characters, but this one has ${length.toString()}`,
    );
  }
  return name;
}

/** Initials of the words if there are at least two, otherwise the first three letters. */
function suggestKey(name: string): string {
  const fromFirstLetter = name.replace(/^[^A-Za-z]+/, '');
  const initials = fromFirstLetter
    .split(/\s+/)
    .map((word) => /[A-Za-z0-9]/.exec(word)?.[0] ?? '')
    .join('')
    .slice(0, PROJECT_KEY_MAX_LENGTH)
    .toUpperCase();
  const firstLetters = fromFirstLetter
    .replace(/[^A-Za-z0-9]/g, '')
    .slice(0, 3)
    .toUpperCase();
  return (
    [initials, firstLetters].find((candidate) => PROJECT_KEY_PATTERN.test(candidate)) ??
    FALLBACK_PROJECT_KEY
  );
}

function requireStatus(project: Project, statusId: Id): Status {
  const status = project.statuses.find((candidate) => candidate.id === statusId);
  if (!status) {
    throw new BackendError(
      'rule_broken',
      `the workflow has no status with the identifier ${statusId}`,
    );
  }
  return status;
}

/** The status a new task starts in: the first one in which work has not started. */
function initialStatus(project: Project): Status {
  const status =
    project.statuses.find((candidate) => candidate.category === 'todo') ?? project.statuses[0];
  if (!status) {
    throw new BackendError('rule_broken', 'the project has no statuses');
  }
  return status;
}

function requireStartNotAfterDue(startOn: CalendarDate | null, dueOn: CalendarDate | null): void {
  // Dates in the form year-month-day compare correctly as text.
  if (startOn !== null && dueOn !== null && startOn > dueOn) {
    throw new BackendError(
      'rule_broken',
      `the start date ${startOn} is after the due date ${dueOn}`,
    );
  }
}
