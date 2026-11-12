import { evenlySpacedRanks, rankBetween, rankFitsBetween } from '@/shared/lib/rank';

import { BackendError, type Backend } from '../backend';
import type {
  AgentProgram,
  ApprovedProposal,
  Assignee,
  CalendarDate,
  Color,
  Conversation,
  DecisionRequest,
  Effort,
  Id,
  InterviewAnswer,
  Member,
  MemberKind,
  MemberProfile,
  Moment,
  NewAdvice,
  NewDecisionAnswer,
  NewMember,
  NewProject,
  NewSprint,
  NewTask,
  Priority,
  Project,
  ProjectPlaces,
  ProposedMember,
  ProposedTask,
  Sprint,
  SprintChanges,
  SprintCompletion,
  SprintReport,
  SprintStart,
  Status,
  StatusCategory,
  StatusProperties,
  Task,
  TaskChange,
  Turn,
  WorkflowTemplate,
} from '../types';
import { briefFor } from './brief';
import { freeName, isInside, isSameFolder, nameOf } from './places';
import { answersInWords, openInterviewOf, openProposalOf, waitInCircle } from './planning';
import { scriptForAnswers, scriptForBrief, scriptForChange } from './simulatedAdvisor';
import {
  asListed,
  dayOfMoment,
  reportOf,
  requirePoints,
  requireStartNotAfterEnd,
  summaryOf,
  tidySprintName,
  type CountedTask,
} from './sprints';
import { historyOf } from './taskHistory';
import {
  SIMULATED_MODEL,
  scriptAfterDecision,
  scriptForMessage,
  scriptForTask,
  type Act,
  type Script,
} from './simulatedSpecialist';

/** Where the demo backend gets the time and new identifiers from. */
export interface DemoEnvironment {
  now(): Date;
  newId(): Id;
  /**
   * Does something after a pause, given in milliseconds. The specialists of
   * the demo act through it. Left out, a timer of the browser is used.
   */
  later?: (work: () => void, milliseconds: number) => void;
  /**
   * The folders that exist on the computer the demo pretends to run on.
   * Left out, every folder is taken to exist: a browser cannot look.
   */
  folders?: readonly string[];
  /**
   * What the manager picks when asked for a folder. Left out, the demo
   * hands out sample folders in turn: it has no window of the computer to
   * ask with.
   */
  pickFolder?: () => string | null;
  /** What the manager picks when asked for files. Left out, a sample file. */
  pickFiles?: () => string[];
}

/** The folders of a project that the manager has chosen. */
interface ChosenPlaces {
  /** The work folder, or nothing while Teamwright's own folder is used. */
  workFolder: string | null;
  referenceFolders: string[];
  /** The names of the files in the folder `references`. */
  referenceFiles: string[];
}

/** The folder in which the pretended computer keeps the folders of the projects. */
const PROJECTS_FOLDER = 'C:\\Users\\you\\Documents\\Teamwright';

/** The folders the browser demo hands out when the manager is asked to pick one. */
const SAMPLE_FOLDERS = [
  'C:\\Users\\you\\Projects\\sensor-node',
  'C:\\Users\\you\\Documents\\Datasheets',
];

/** The file the browser demo hands out when the manager is asked to pick files. */
const SAMPLE_FILE = 'C:\\Users\\you\\Downloads\\BME680 datasheet.pdf';

/** The projects, tasks and team members a demo backend starts with. */
export interface DemoWorkspace {
  projects: Project[];
  tasks: Task[];
  members: Member[];
  /** The sprints of the projects. Left out, there are none. */
  sprints?: Sprint[];
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

/**
 * How far along in the course of the work a status is, by its category. A
 * new status is placed by it.
 */
const PROGRESS_OF_CATEGORY: Record<StatusCategory, number> = {
  todo: 0,
  active: 1,
  review: 2,
  done: 3,
};

/** The categories no workflow can do without. */
const REQUIRED_CATEGORIES: readonly StatusCategory[] = ['todo', 'done'];

const PROJECT_NAME_MAX_LENGTH = 80;
const STATUS_NAME_MAX_LENGTH = 40;
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
  private readonly conversations = new Map<Id, Conversation>();
  /**
   * Every version a task has had, the oldest first. The real application
   * has its log of operations for this; the demo keeps the versions.
   */
  private readonly versions = new Map<Id, Task[]>();
  private readonly nextTaskNumbers = new Map<Id, number>();
  private readonly sprints = new Map<Id, Sprint>();
  /**
   * How many sprints each project has had, the deleted ones included. A new
   * sprint without a name is named after the next number.
   */
  private readonly sprintsCreated = new Map<Id, number>();
  private readonly places = new Map<Id, ChosenPlaces>();
  private sampleFoldersHandedOut = 0;
  /** The turns whose script is still being played. A turn that is stopped leaves this set. */
  private readonly playing = new Set<Id>();
  private readonly conversationListeners = new Set<(conversation: Conversation) => void>();

  constructor(workspace?: DemoWorkspace, environment: DemoEnvironment = browserEnvironment) {
    this.environment = environment;
    for (const project of workspace?.projects ?? []) {
      this.projects.set(project.id, structuredClone(project));
    }
    for (const task of workspace?.tasks ?? []) {
      this.tasks.set(task.id, structuredClone(task));
      this.versions.set(task.id, [structuredClone(task)]);
      const next = Math.max(this.nextTaskNumbers.get(task.projectId) ?? 1, task.number + 1);
      this.nextTaskNumbers.set(task.projectId, next);
    }
    for (const member of workspace?.members ?? []) {
      this.members.set(member.id, structuredClone(member));
    }
    for (const sprint of workspace?.sprints ?? []) {
      this.sprints.set(sprint.id, structuredClone(sprint));
      this.sprintsCreated.set(
        sprint.projectId,
        (this.sprintsCreated.get(sprint.projectId) ?? 0) + 1,
      );
    }
  }

  listProjects(): Promise<Project[]> {
    return this.answer(() => this.allProjects().filter((project) => project.archivedAt === null));
  }

  createProject(newProject: NewProject): Promise<Project> {
    return this.answer(() => {
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
    return this.answer(() => {
      const project = this.requireProject(projectId);
      project.name = tidyName(name, 'project name', PROJECT_NAME_MAX_LENGTH);
      return project;
    });
  }

  describeProject(projectId: Id, description: string): Promise<Project> {
    return this.answer(() => {
      const project = this.requireProject(projectId);
      project.description = description.trim();
      return project;
    });
  }

  archiveProject(projectId: Id): Promise<Project> {
    return this.answer(() => {
      const project = this.requireProject(projectId);
      this.refuseWhileWorkedOn(this.conversationsOf(project.id));
      project.archivedAt ??= this.nowAsText();
      return project;
    });
  }

  listArchivedProjects(): Promise<Project[]> {
    return this.answer(() =>
      this.allProjects()
        .filter((project) => project.archivedAt !== null)
        .sort((first, second) => (second.archivedAt ?? '').localeCompare(first.archivedAt ?? '')),
    );
  }

  restoreProject(projectId: Id): Promise<Project> {
    return this.answer(() => {
      const project = this.requireProject(projectId);
      project.archivedAt = null;
      return project;
    });
  }

  deleteProject(projectId: Id): Promise<void> {
    return this.answer(() => {
      const project = this.requireProject(projectId);
      this.refuseWhileWorkedOn(this.conversationsOf(project.id));
      for (const task of this.tasksOfProject(project.id)) {
        this.tasks.delete(task.id);
      }
      for (const member of this.teamOf(project.id)) {
        this.members.delete(member.id);
      }
      for (const sprint of this.sprintsOf(project.id)) {
        this.sprints.delete(sprint.id);
      }
      this.sprintsCreated.delete(project.id);
      for (const conversation of this.conversationsOf(project.id)) {
        this.forget(conversation);
      }
      this.nextTaskNumbers.delete(project.id);
      // The folders stay on the computer. Only what the project knew about
      // them goes with it: Teamwright never removes a file of the manager.
      this.places.delete(project.id);
      this.projects.delete(project.id);
    });
  }

  projectPlaces(projectId: Id): Promise<ProjectPlaces> {
    return this.answer(() => this.placesOf(this.requireProject(projectId)));
  }

  setWorkFolder(projectId: Id, folder: string | null): Promise<ProjectPlaces> {
    return this.answer(() => {
      const project = this.requireProject(projectId);
      this.refuseWhileWorkedOn(this.conversationsOf(project.id));
      const chosen = this.chosenPlacesOf(project.id);
      if (folder === null) {
        chosen.workFolder = null;
        return this.placesOf(project);
      }
      const tidy = this.requireFolder(folder);
      for (const reference of chosen.referenceFolders) {
        refuseFoldersThatOverlap(tidy, reference);
      }
      chosen.workFolder = tidy;
      return this.placesOf(project);
    });
  }

  addReferenceFolder(projectId: Id, folder: string): Promise<ProjectPlaces> {
    return this.answer(() => {
      const project = this.requireProject(projectId);
      const chosen = this.chosenPlacesOf(project.id);
      const tidy = this.requireFolder(folder);
      refuseFoldersThatOverlap(this.placesOf(project).workFolder, tidy);
      if (!chosen.referenceFolders.some((other) => isSameFolder(other, tidy))) {
        chosen.referenceFolders.push(tidy);
      }
      return this.placesOf(project);
    });
  }

  removeReferenceFolder(projectId: Id, folder: string): Promise<ProjectPlaces> {
    return this.answer(() => {
      const project = this.requireProject(projectId);
      const chosen = this.chosenPlacesOf(project.id);
      chosen.referenceFolders = chosen.referenceFolders.filter(
        (other) => !isSameFolder(other, folder),
      );
      return this.placesOf(project);
    });
  }

  addReferenceFiles(projectId: Id, files: string[]): Promise<string[]> {
    return this.answer(() => {
      const project = this.requireProject(projectId);
      const chosen = this.chosenPlacesOf(project.id);
      return files.map((file) => {
        const name = freeName(nameOf(file), chosen.referenceFiles);
        if (name === '') {
          throw new BackendError('rule_broken', 'a file to copy has no name');
        }
        chosen.referenceFiles.push(name);
        return name;
      });
    });
  }

  chooseFolder(): Promise<string | null> {
    return this.answer(() => {
      if (this.environment.pickFolder) {
        return this.environment.pickFolder();
      }
      const folder = SAMPLE_FOLDERS[this.sampleFoldersHandedOut % SAMPLE_FOLDERS.length];
      this.sampleFoldersHandedOut += 1;
      return folder ?? null;
    });
  }

  chooseFiles(): Promise<string[]> {
    return this.answer(() => this.environment.pickFiles?.() ?? [SAMPLE_FILE]);
  }

  showWorkFolder(projectId: Id): Promise<void> {
    return this.answer(() => {
      this.requireProject(projectId);
      throw new BackendError('not_available', 'the demo has no file manager to show a folder in');
    });
  }

  /** What the manager has chosen for a project, kept from the first time it is asked for. */
  private chosenPlacesOf(projectId: Id): ChosenPlaces {
    let chosen = this.places.get(projectId);
    if (!chosen) {
      chosen = { workFolder: null, referenceFolders: [], referenceFiles: [] };
      this.places.set(projectId, chosen);
    }
    return chosen;
  }

  /** The folders of a project as the interface sees them. */
  private placesOf(project: Project): ProjectPlaces {
    const chosen = this.chosenPlacesOf(project.id);
    return {
      workFolder: chosen.workFolder ?? `${PROJECTS_FOLDER}\\${project.key}`,
      isWorkFolderChosen: chosen.workFolder !== null,
      referenceFolders: [...chosen.referenceFolders],
    };
  }

  /** Checks that a folder is named and exists, and answers with its tidy path. */
  private requireFolder(folder: string): string {
    const tidy = folder.trim();
    if (tidy === '') {
      throw new BackendError('rule_broken', 'the folder must be named');
    }
    const known = this.environment.folders;
    if (known && !known.some((other) => isSameFolder(other, tidy))) {
      throw new BackendError('folder_not_found', `the folder ${tidy} does not exist`);
    }
    return tidy;
  }

  addStatus(projectId: Id, properties: StatusProperties): Promise<Project> {
    return this.answer(() => {
      const project = this.requireProject(projectId);
      const name = tidyName(properties.name, 'status name', STATUS_NAME_MAX_LENGTH);
      refuseStatusNameInUse(project.statuses, name);
      const status: Status = {
        id: this.environment.newId(),
        name,
        category: properties.category,
        color: properties.color,
      };
      project.statuses.splice(placeForNewStatus(project.statuses, status.category), 0, status);
      return project;
    });
  }

  changeStatus(projectId: Id, statusId: Id, properties: StatusProperties): Promise<Project> {
    return this.answer(() => {
      const project = this.requireProject(projectId);
      const status = requireStatus(project, statusId);
      const name = tidyName(properties.name, 'status name', STATUS_NAME_MAX_LENGTH);
      refuseStatusNameInUse(
        project.statuses.filter((other) => other.id !== statusId),
        name,
      );
      requireCategories(
        project.statuses.map((other) =>
          other.id === statusId ? properties.category : other.category,
        ),
      );

      const wasDone = status.category === 'done';
      const isDone = properties.category === 'done';
      status.name = name;
      status.category = properties.category;
      status.color = properties.color;

      // Whether a task is complete follows from the category of its status.
      if (wasDone !== isDone) {
        const now = this.nowAsText();
        for (const task of this.tasksOfStatus(project.id, statusId)) {
          task.completedAt = isDone ? (task.completedAt ?? now) : null;
          task.updatedAt = now;
        }
      }
      return project;
    });
  }

  moveStatus(projectId: Id, statusId: Id, index: number): Promise<Project> {
    return this.answer(() => {
      const project = this.requireProject(projectId);
      const status = requireStatus(project, statusId);
      const others = project.statuses.filter((other) => other.id !== statusId);
      others.splice(Math.min(Math.max(index, 0), others.length), 0, status);
      project.statuses = others;
      return project;
    });
  }

  removeStatus(projectId: Id, statusId: Id, moveTasksTo?: Id): Promise<Project> {
    return this.answer(() => {
      const project = this.requireProject(projectId);
      const status = requireStatus(project, statusId);
      const remaining = project.statuses.filter((other) => other.id !== statusId);
      requireCategories(remaining.map((other) => other.category));

      const tasks = this.tasksOfStatus(project.id, statusId);
      if (tasks.length > 0) {
        if (moveTasksTo === undefined || moveTasksTo === statusId) {
          throw new BackendError(
            'status_in_use',
            `the status "${status.name}" still has ${countOfTasks(tasks.length)}, which need another status to go to`,
          );
        }
        const target = requireStatus(project, moveTasksTo);
        const now = this.nowAsText();
        // The tasks go to the bottom of the other status, in their order.
        let lastRank = this.tasksOfStatus(project.id, target.id).at(-1)?.rank;
        for (const task of tasks) {
          lastRank = rankBetween(lastRank, undefined);
          task.statusId = target.id;
          task.rank = lastRank;
          task.completedAt = target.category === 'done' ? (task.completedAt ?? now) : null;
          task.updatedAt = now;
        }
      }

      project.statuses = remaining;
      return project;
    });
  }

  listTasks(projectId: Id): Promise<Task[]> {
    return this.answer(() => {
      this.requireProject(projectId);
      return this.tasksOfProject(projectId).sort(asArranged);
    });
  }

  createTask(newTask: NewTask): Promise<Task> {
    return this.answer(() => {
      const project = this.requireProject(newTask.projectId);
      const title = tidyName(newTask.title, 'task title', TASK_TITLE_MAX_LENGTH);
      requireStartNotAfterDue(newTask.startOn ?? null, newTask.dueOn ?? null);
      const status = newTask.statusId
        ? requireStatus(project, newTask.statusId)
        : initialStatus(project);
      const parent = newTask.parentId ? this.requireTask(newTask.parentId) : undefined;
      if (parent && parent.projectId !== project.id) {
        throw new BackendError(
          'different_projects',
          'tasks of different projects cannot be connected',
        );
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
        // A subtask goes where the task it is a part of goes.
        sprintId: parent?.sprintId ?? null,
        points: null,
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
    return this.answer(() => {
      const tidyTitle = tidyName(title, 'task title', TASK_TITLE_MAX_LENGTH);
      return this.change(taskId, (task) => {
        task.title = tidyTitle;
      });
    });
  }

  describeTask(taskId: Id, description: string): Promise<Task> {
    return this.answer(() =>
      this.change(taskId, (task) => {
        task.description = description;
      }),
    );
  }

  prioritizeTask(taskId: Id, priority: Priority): Promise<Task> {
    return this.answer(() =>
      this.change(taskId, (task) => {
        task.priority = priority;
      }),
    );
  }

  assignTask(taskId: Id, assignee: Assignee | null): Promise<Task> {
    return this.answer(() =>
      this.change(taskId, (task) => {
        this.checkAssignee(assignee, task.projectId);
        task.assignee = assignee;
      }),
    );
  }

  estimateTask(taskId: Id, minutes: number | null): Promise<Task> {
    return this.answer(() =>
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
    return this.answer(() => {
      requireStartNotAfterDue(startOn, dueOn);
      return this.change(taskId, (task) => {
        task.startOn = startOn;
        task.dueOn = dueOn;
      });
    });
  }

  moveTask(taskId: Id, statusId: Id, index: number): Promise<Task> {
    return this.answer(() => {
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
    return this.answer(() => {
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
    return this.answer(() =>
      this.change(taskId, (task) => {
        task.blockedBy = task.blockedBy.filter((id) => id !== blockerId);
      }),
    );
  }

  deleteTask(taskId: Id): Promise<void> {
    return this.answer(() => {
      const task = this.requireTask(taskId);
      const removed = this.subtreeOf(task);
      const now = this.nowAsText();
      // The record of the work on a task goes with the task. A task that
      // somebody is working on stays until the work has been stopped.
      const aboutRemoved = this.conversationsOf(task.projectId).filter(
        (conversation) => conversation.taskId !== null && removed.has(conversation.taskId),
      );
      this.refuseWhileWorkedOn(aboutRemoved);
      for (const conversation of aboutRemoved) {
        this.forget(conversation);
      }
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
    return this.answer(() => {
      this.requireProject(projectId);
      return this.teamOf(projectId);
    });
  }

  addMember(newMember: NewMember): Promise<Member> {
    return this.answer(() => {
      const project = this.requireProject(newMember.projectId);
      const name = tidyName(newMember.name, 'name of the team member', MEMBER_NAME_MAX_LENGTH);
      const team = this.teamOf(project.id);
      refuseNameInUse(team, name);
      const profile = tidyProfile(newMember.kind, name, {
        model: newMember.model ?? null,
        effort: newMember.effort ?? null,
      });
      const now = this.nowAsText();
      const member: Member = {
        id: this.environment.newId(),
        projectId: project.id,
        name,
        kind: newMember.kind,
        responsibility: newMember.responsibility?.trim() ?? '',
        ...profile,
        color: colorForNewcomer(team.map((other) => other.color)),
        joinedAt: now,
        updatedAt: now,
      };
      this.members.set(member.id, member);
      return member;
    });
  }

  renameMember(memberId: Id, name: string): Promise<Member> {
    return this.answer(() => {
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
    return this.answer(() => {
      const member = this.requireMember(memberId);
      member.responsibility = responsibility.trim();
      member.updatedAt = this.nowAsText();
      return member;
    });
  }

  setMemberProfile(memberId: Id, profile: MemberProfile): Promise<Member> {
    return this.answer(() => {
      const member = this.requireMember(memberId);
      Object.assign(member, tidyProfile(member.kind, member.name, profile));
      member.updatedAt = this.nowAsText();
      return member;
    });
  }

  removeMember(memberId: Id): Promise<void> {
    return this.answer(() => {
      const member = this.requireMember(memberId);
      this.refuseWhileBusy(member);
      const now = this.nowAsText();
      // What the member did stays on record, without the member.
      for (const conversation of this.conversationsOf(member.projectId)) {
        if (conversation.memberId === member.id) {
          conversation.memberId = null;
        }
      }
      for (const task of this.tasksOfProject(member.projectId)) {
        if (task.assignee?.kind === 'member' && task.assignee.memberId === member.id) {
          task.assignee = null;
          task.updatedAt = now;
        }
      }
      this.members.delete(member.id);
    });
  }

  agentProgram(): Promise<AgentProgram> {
    // The demo has no program to run specialists on. It plays their work.
    return this.answer(() => ({
      state: 'ready' as const,
      version: null,
      signInMethod: null,
      subscription: null,
      problem: null,
    }));
  }

  listConversations(projectId: Id): Promise<Conversation[]> {
    return this.answer(() => {
      this.requireProject(projectId);
      return this.conversationsOf(projectId);
    });
  }

  startWork(taskId: Id): Promise<Conversation> {
    return this.answer(() => {
      const task = this.requireTask(taskId);
      const project = this.requireProject(task.projectId);
      const member =
        task.assignee?.kind === 'member' ? this.members.get(task.assignee.memberId) : undefined;
      if (member?.kind !== 'agent') {
        throw new BackendError('no_specialist', 'the task is not given to an AI specialist');
      }
      if (this.conversationsOf(project.id).some((other) => other.taskId === task.id)) {
        throw new BackendError(
          'work_already_started',
          `work on ${referenceOf(project, task)} has already started`,
        );
      }
      this.refuseWhileBusy(member);

      const conversation: Conversation = {
        id: this.environment.newId(),
        projectId: project.id,
        isWithAdvisor: false,
        memberId: member.id,
        taskId: task.id,
        startedAt: this.nowAsText(),
        turns: [],
      };
      this.conversations.set(conversation.id, conversation);
      this.beginTurn(
        conversation,
        briefFor(project, task, this.tasksOfProject(project.id)),
        'brief',
        scriptForTask(task),
      );
      return conversation;
    });
  }

  reply(conversationId: Id, message: string): Promise<Conversation> {
    return this.answer(() => {
      const conversation = this.requireConversation(conversationId);
      if (conversation.isWithAdvisor) {
        return this.replyToAdvisor(conversation, message);
      }
      const member =
        conversation.memberId === null ? undefined : this.members.get(conversation.memberId);
      if (!member) {
        throw new BackendError(
          'no_specialist',
          'the specialist of this conversation has left the team',
        );
      }
      const text = tidyMessage(message);
      this.refuseWhileBusy(member);
      if (openDecisionOf(conversation)) {
        throw new BackendError(
          'decision_open',
          `${member.name} waits for a decision: answer its question first`,
        );
      }
      this.beginTurn(conversation, text, 'message', scriptForMessage(member));
      return conversation;
    });
  }

  answerDecision(conversationId: Id, answer: NewDecisionAnswer): Promise<Conversation> {
    return this.answer(() => {
      const conversation = this.requireConversation(conversationId);
      const decision = openDecisionOf(conversation);
      if (!decision) {
        throw new BackendError('no_decision_open', 'no question is open in this conversation');
      }
      const member =
        conversation.memberId === null ? undefined : this.members.get(conversation.memberId);
      if (!member) {
        throw new BackendError(
          'no_specialist',
          'the specialist of this conversation has left the team',
        );
      }
      const note = answer.note.trim();
      const chosen = answer.option === null ? undefined : decision.options[answer.option];
      if (answer.option !== null && !chosen) {
        throw new BackendError('rule_broken', 'the question has no such option');
      }
      if (!chosen && note === '') {
        throw new BackendError(
          'rule_broken',
          'an answer names one of the options or says something in words',
        );
      }
      this.refuseWhileBusy(member);

      decision.answer = { option: answer.option, note, at: this.nowAsText() };
      const words = answerInWords(chosen?.label, note);
      this.beginTurn(
        conversation,
        words,
        'decision',
        scriptAfterDecision(chosen?.label ?? 'in your own words'),
      );
      return conversation;
    });
  }

  listConversationsWaiting(): Promise<Conversation[]> {
    return this.answer(() =>
      this.allProjects()
        .filter((project) => project.archivedAt === null)
        .flatMap((project) => this.conversationsOf(project.id))
        .filter(
          (conversation) =>
            openDecisionOf(conversation) !== undefined ||
            openInterviewOf(conversation) !== undefined ||
            openProposalOf(conversation) !== undefined,
        )
        .sort((first, second) => askedAt(first).localeCompare(askedAt(second))),
    );
  }

  advise(projectId: Id, advice: NewAdvice): Promise<Conversation> {
    return this.answer(() => {
      const project = this.requireProject(projectId);
      const brief = tidyMessage(advice.brief);
      // The demo plays the Advisor, so the profile changes nothing here.
      // It is checked all the same, as the real application checks it.
      tidyProfile('agent', ADVISOR_NAME, {
        model: advice.model ?? null,
        effort: advice.effort ?? null,
      });
      this.refuseWhileAdvisorWorks(project.id);
      const conversation: Conversation = {
        id: this.environment.newId(),
        projectId: project.id,
        isWithAdvisor: true,
        memberId: null,
        taskId: null,
        startedAt: this.nowAsText(),
        turns: [],
      };
      this.conversations.set(conversation.id, conversation);
      this.beginTurn(conversation, brief, 'message', scriptForBrief());
      return conversation;
    });
  }

  answerInterview(conversationId: Id, answers: InterviewAnswer[]): Promise<Conversation> {
    return this.answer(() => {
      const conversation = this.requireConversation(conversationId);
      const interview = openInterviewOf(conversation);
      if (!interview) {
        throw new BackendError('no_interview_open', 'no questions are open in this conversation');
      }
      if (answers.length !== interview.questions.length) {
        throw new BackendError('rule_broken', 'every question needs an answer');
      }
      const tidy = answers.map((answer, place) => {
        const number = (place + 1).toString();
        const note = answer.note.trim();
        const options = interview.questions[place]?.options ?? [];
        if (answer.option !== null && !options[answer.option]) {
          throw new BackendError('rule_broken', `question ${number} has no such option`);
        }
        if (answer.option === null && note === '') {
          throw new BackendError(
            'rule_broken',
            `the answer to question ${number} names one of the options or says something in words`,
          );
        }
        return { option: answer.option, note };
      });
      this.refuseWhileAdvisorWorks(conversation.projectId);

      interview.answers = tidy;
      interview.answeredAt = this.nowAsText();
      this.beginTurn(
        conversation,
        answersInWords(interview.questions, tidy),
        'interview',
        scriptForAnswers(this.teamOf(conversation.projectId).map((member) => member.name)),
      );
      return conversation;
    });
  }

  approveProposal(conversationId: Id, approved: ApprovedProposal): Promise<Conversation> {
    return this.answer(() => {
      const conversation = this.requireConversation(conversationId);
      const proposal = openProposalOf(conversation);
      if (!proposal) {
        throw new BackendError('no_proposal_open', 'no proposal is open in this conversation');
      }
      const project = this.requireProject(conversation.projectId);
      const team = this.teamOf(project.id);
      const members = tidyProposedMembers(approved.members);
      const tasks = tidyProposedTasks(approved.tasks, [
        ...team.map((member) => member.name),
        ...members.map((member) => member.name),
      ]);
      if (members.length === 0 && tasks.length === 0) {
        throw new BackendError('rule_broken', 'the proposal has no members and no tasks');
      }

      // Everything was checked. From here on nothing is refused, so the
      // members and the tasks are created together or not at all.
      const now = this.nowAsText();
      const byName = new Map(team.map((member) => [member.name.toLowerCase(), member]));
      for (const proposed of members) {
        if (byName.has(proposed.name.toLowerCase())) {
          // The team has this member already. It is not added again.
          continue;
        }
        const member: Member = {
          id: this.environment.newId(),
          projectId: project.id,
          name: proposed.name,
          kind: 'agent',
          responsibility: proposed.responsibility,
          model: proposed.model,
          effort: proposed.effort,
          color: colorForNewcomer([...byName.values()].map((other) => other.color)),
          joinedAt: now,
          updatedAt: now,
        };
        this.members.set(member.id, member);
        byName.set(member.name.toLowerCase(), member);
      }

      const status = initialStatus(project);
      const created: Task[] = [];
      for (const proposed of tasks) {
        const owner = proposed.owner;
        const member = owner?.kind === 'member' ? byName.get(owner.name.toLowerCase()) : undefined;
        const lastInStatus = this.tasksOfStatus(project.id, status.id).at(-1);
        const number = this.nextTaskNumbers.get(project.id) ?? 1;
        this.nextTaskNumbers.set(project.id, number + 1);
        const task: Task = {
          id: this.environment.newId(),
          projectId: project.id,
          number,
          title: proposed.title,
          description: proposed.description,
          statusId: status.id,
          priority: proposed.priority,
          assignee:
            owner?.kind === 'manager'
              ? { kind: 'manager' }
              : member
                ? { kind: 'member', memberId: member.id }
                : null,
          parentId: null,
          blockedBy: [],
          estimateMinutes: proposed.estimateMinutes,
          sprintId: null,
          points: null,
          startOn: null,
          dueOn: null,
          rank: rankBetween(lastInStatus?.rank, undefined),
          createdAt: now,
          updatedAt: now,
          completedAt: status.category === 'done' ? now : null,
        };
        this.tasks.set(task.id, task);
        created.push(task);
      }
      for (const [place, proposed] of tasks.entries()) {
        const task = created[place];
        if (task) {
          task.blockedBy = proposed.waitsFor.flatMap((earlier) => created[earlier]?.id ?? []);
        }
      }

      // The record keeps what was approved, which may be less than what
      // was proposed.
      proposal.members = members;
      proposal.tasks = tasks;
      proposal.approvedAt = now;
      this.noteVersions();
      this.tellListeners(conversation);
      return conversation;
    });
  }

  /**
   * Sends the Advisor a message of the manager's: a wish for a change to
   * its proposal, or the start of another round.
   */
  private replyToAdvisor(conversation: Conversation, message: string): Conversation {
    const text = tidyMessage(message);
    if (openInterviewOf(conversation)) {
      throw new BackendError(
        'interview_open',
        'the Advisor waits for your answers: answer its questions first',
      );
    }
    this.refuseWhileAdvisorWorks(conversation.projectId);
    const previous = [...conversation.turns]
      .reverse()
      .find((turn) => turn.proposal !== null)?.proposal;
    this.beginTurn(conversation, text, 'message', scriptForChange(previous ?? undefined));
    return conversation;
  }

  /** One conversation with the Advisor runs in a project at a time. */
  private refuseWhileAdvisorWorks(projectId: Id): void {
    const working = this.conversationsOf(projectId).some(
      (conversation) =>
        conversation.isWithAdvisor && conversation.turns.some((turn) => turn.state === 'running'),
    );
    if (working) {
      throw new BackendError('advisor_busy', 'the Advisor is still working on this project');
    }
  }

  ask(memberId: Id, message: string): Promise<Conversation> {
    return this.answer(() => {
      const member = this.requireMember(memberId);
      if (member.kind !== 'agent') {
        throw new BackendError('no_specialist', `${member.name} is not an AI specialist`);
      }
      const text = tidyMessage(message);
      this.refuseWhileBusy(member);
      const conversation: Conversation = {
        id: this.environment.newId(),
        projectId: member.projectId,
        isWithAdvisor: false,
        memberId: member.id,
        taskId: null,
        startedAt: this.nowAsText(),
        turns: [],
      };
      this.conversations.set(conversation.id, conversation);
      this.beginTurn(conversation, text, 'message', scriptForMessage(member));
      return conversation;
    });
  }

  stopWork(conversationId: Id): Promise<Conversation> {
    return this.answer(() => {
      const conversation = this.requireConversation(conversationId);
      const turn = conversation.turns.find((candidate) => candidate.state === 'running');
      if (turn) {
        this.playing.delete(turn.id);
        turn.state = 'stopped';
        turn.endedAt = this.nowAsText();
        turn.failure = 'Stopped by the manager.';
        this.tellListeners(conversation);
      }
      return conversation;
    });
  }

  openInClaude(conversationId: Id): Promise<void> {
    return this.answer(() => {
      this.requireConversation(conversationId);
      throw new BackendError(
        'not_in_the_demo',
        'the demo has no Claude desktop app to open the conversation in',
      );
    });
  }

  onConversationChanged(listener: (conversation: Conversation) => void): () => void {
    this.conversationListeners.add(listener);
    return () => {
      this.conversationListeners.delete(listener);
    };
  }

  /** The conversations of a project, the oldest first. */
  private conversationsOf(projectId: Id): Conversation[] {
    return [...this.conversations.values()]
      .filter((conversation) => conversation.projectId === projectId)
      .sort((first, second) => first.startedAt.localeCompare(second.startedAt));
  }

  private requireConversation(conversationId: Id): Conversation {
    const conversation = this.conversations.get(conversationId);
    if (!conversation) {
      throw new BackendError(
        'not_found',
        `there is no conversation with the identifier ${conversationId}`,
      );
    }
    return conversation;
  }

  /** Removes a conversation and stops whatever is still being played in it. */
  private forget(conversation: Conversation): void {
    for (const turn of conversation.turns) {
      this.playing.delete(turn.id);
    }
    this.conversations.delete(conversation.id);
  }

  /**
   * Refuses to go on while a turn is running in one of some conversations:
   * what a specialist is working on is not taken away from under it.
   */
  private refuseWhileWorkedOn(conversations: readonly Conversation[]): void {
    const busy = conversations.find((conversation) =>
      conversation.turns.some((turn) => turn.state === 'running'),
    );
    if (!busy) {
      return;
    }
    const member = busy.memberId === null ? undefined : this.members.get(busy.memberId);
    const task = busy.taskId === null ? undefined : this.tasks.get(busy.taskId);
    const project = this.projects.get(busy.projectId);
    const who = busy.isWithAdvisor ? ADVISOR_NAME : (member?.name ?? 'A specialist');
    const what = busy.isWithAdvisor
      ? 'a plan'
      : task && project
        ? referenceOf(project, task)
        : 'a question';
    throw new BackendError(
      'work_in_progress',
      `${who} is still working on ${what}; stop the work first`,
    );
  }

  /** A specialist works on one thing at a time. */
  private refuseWhileBusy(member: Member): void {
    const busyWith = this.conversationsOf(member.projectId).find(
      (conversation) =>
        conversation.memberId === member.id &&
        conversation.turns.some((turn) => turn.state === 'running'),
    );
    if (!busyWith) {
      return;
    }
    const task = busyWith.taskId === null ? undefined : this.tasks.get(busyWith.taskId);
    const project = this.projects.get(member.projectId);
    const what = task && project ? referenceOf(project, task) : 'a question';
    throw new BackendError('member_busy', `${member.name} is still working on ${what}`);
  }

  /**
   * Starts a turn of a conversation and plays its script. A task that work
   * starts on, or goes on with, moves to the first status in which work is
   * under way.
   */
  private beginTurn(
    conversation: Conversation,
    request: string,
    origin: Turn['origin'],
    script: Script,
  ): void {
    const turn: Turn = {
      id: this.environment.newId(),
      request,
      origin,
      state: 'running',
      startedAt: this.nowAsText(),
      endedAt: null,
      steps: [],
      answer: '',
      failure: null,
      model: SIMULATED_MODEL,
      tokens: null,
      decision: null,
      interview: null,
      proposal: null,
    };
    conversation.turns.push(turn);

    const task = conversation.taskId === null ? undefined : this.tasks.get(conversation.taskId);
    if (task && ['todo', 'review'].includes(this.categoryOf(task))) {
      this.moveToFirst(task, 'active');
    }

    this.playing.add(turn.id);
    const later = this.environment.later ?? ((work, pause) => void setTimeout(work, pause));
    let pause = 0;
    for (const { after, act } of script.acts) {
      pause += after;
      later(() => {
        this.perform(conversation.id, turn.id, act);
      }, pause);
    }
    later(() => {
      this.endTurn(conversation.id, turn.id, script);
    }, pause + script.endsAfter);
  }

  /** The turn that is being played, or nothing if it was stopped or removed meanwhile. */
  private turnBeingPlayed(conversationId: Id, turnId: Id): [Conversation, Turn] | undefined {
    const conversation = this.conversations.get(conversationId);
    const turn = conversation?.turns.find((candidate) => candidate.id === turnId);
    return conversation && turn && this.playing.has(turnId) ? [conversation, turn] : undefined;
  }

  /** Lets a simulated specialist take one step. */
  private perform(conversationId: Id, turnId: Id, act: Act): void {
    const found = this.turnBeingPlayed(conversationId, turnId);
    if (!found) {
      return;
    }
    const [conversation, turn] = found;
    const at = this.nowAsText();
    if (act.kind === 'say') {
      turn.steps.push({ kind: 'text', at, text: act.text });
    } else if (act.kind === 'note') {
      turn.steps.push({ kind: 'notice', at, message: act.message });
    } else if (act.kind === 'use') {
      turn.steps.push({
        kind: 'tool',
        at,
        id: act.id,
        tool: act.tool,
        summary: act.summary,
        outcome: 'running',
      });
    } else {
      for (const step of turn.steps) {
        if (step.kind === 'tool' && step.id === act.id) {
          step.outcome = 'done';
        }
      }
    }
    this.tellListeners(conversation);
  }

  /**
   * Ends a simulated turn with its report. A task whose work is under way
   * moves on to the first status for work that waits to be checked.
   */
  private endTurn(conversationId: Id, turnId: Id, script: Script): void {
    const found = this.turnBeingPlayed(conversationId, turnId);
    if (!found) {
      return;
    }
    const [conversation, turn] = found;
    this.playing.delete(turnId);
    turn.state = 'finished';
    turn.endedAt = this.nowAsText();
    turn.answer = script.answer;
    turn.tokens = script.tokens;
    turn.decision = script.asks ? { ...structuredClone(script.asks), answer: null } : null;
    turn.interview = script.interviews
      ? { questions: structuredClone(script.interviews), answers: null, answeredAt: null }
      : null;
    turn.proposal = script.proposes
      ? { ...structuredClone(script.proposes), approvedAt: null }
      : null;

    // Work that ends in a question is not ready to be checked: the task
    // stays in work until the manager has decided and the specialist has
    // gone on.
    const task = conversation.taskId === null ? undefined : this.tasks.get(conversation.taskId);
    if (task && turn.decision === null && this.categoryOf(task) === 'active') {
      this.moveToFirst(task, 'review');
    }
    this.noteVersions();
    this.tellListeners(conversation);
  }

  private tellListeners(conversation: Conversation): void {
    for (const listener of this.conversationListeners) {
      listener(structuredClone(conversation));
    }
  }

  /** What the status a task is in means for its progress. */
  private categoryOf(task: Task): StatusCategory {
    const project = this.requireProject(task.projectId);
    return requireStatus(project, task.statusId).category;
  }

  /**
   * Moves a task to the bottom of the first status of a category. A workflow
   * that has no such status leaves the task where it is.
   */
  private moveToFirst(task: Task, category: StatusCategory): void {
    const project = this.requireProject(task.projectId);
    const status = project.statuses.find((candidate) => candidate.category === category);
    if (!status || status.id === task.statusId) {
      return;
    }
    const now = this.nowAsText();
    task.rank = rankBetween(this.tasksOfStatus(project.id, status.id).at(-1)?.rank, undefined);
    task.statusId = status.id;
    task.completedAt = status.category === 'done' ? (task.completedAt ?? now) : null;
    task.updatedAt = now;
  }

  listTaskHistory(taskId: Id): Promise<TaskChange[]> {
    return this.answer(() => {
      this.requireTask(taskId);
      return historyOf(this.versions.get(taskId) ?? []);
    });
  }

  listSprints(projectId: Id): Promise<Sprint[]> {
    return this.answer(() => {
      this.requireProject(projectId);
      return this.sprintsOf(projectId).sort(asListed);
    });
  }

  createSprint(newSprint: NewSprint): Promise<Sprint> {
    return this.answer(() => {
      const project = this.requireProject(newSprint.projectId);
      const number = (this.sprintsCreated.get(project.id) ?? 0) + 1;
      const name = tidySprintName(newSprint.name ?? '', number);
      const startOn = newSprint.startOn ?? null;
      const endOn = newSprint.endOn ?? null;
      requireStartNotAfterEnd(startOn, endOn);

      this.sprintsCreated.set(project.id, number);
      const sprint: Sprint = {
        id: this.environment.newId(),
        projectId: project.id,
        name,
        goal: newSprint.goal?.trim() ?? '',
        startOn,
        endOn,
        state: 'planned',
        createdAt: this.nowAsText(),
        startedAt: null,
        completedAt: null,
        summary: null,
      };
      this.sprints.set(sprint.id, sprint);
      return sprint;
    });
  }

  changeSprint(sprintId: Id, changes: SprintChanges): Promise<Sprint> {
    return this.answer(() => {
      const sprint = this.requireSprint(sprintId);
      refuseCompleted(sprint, 'can no longer be changed');
      const name = tidySprintName(changes.name);
      requireStartNotAfterEnd(changes.startOn, changes.endOn);
      if (sprint.state === 'active' && (changes.startOn === null || changes.endOn === null)) {
        throw new BackendError(
          'rule_broken',
          'a sprint that has started has a start and an end date',
        );
      }
      sprint.name = name;
      sprint.goal = changes.goal.trim();
      sprint.startOn = changes.startOn;
      sprint.endOn = changes.endOn;
      return sprint;
    });
  }

  deleteSprint(sprintId: Id): Promise<void> {
    return this.answer(() => {
      const sprint = this.requireSprint(sprintId);
      refuseCompleted(sprint, 'is kept as a record');
      if (sprint.state === 'active') {
        throw new BackendError(
          'sprint_active',
          `"${sprint.name}" has started and cannot be deleted; complete it instead`,
        );
      }
      const now = this.nowAsText();
      for (const task of this.tasksOfSprint(sprint.id)) {
        task.sprintId = null;
        task.updatedAt = now;
      }
      this.sprints.delete(sprint.id);
    });
  }

  startSprint(sprintId: Id, start: SprintStart): Promise<Sprint> {
    return this.answer(() => {
      const sprint = this.requireSprint(sprintId);
      refuseCompleted(sprint, 'cannot be started again');
      if (sprint.state === 'active') {
        throw new BackendError('sprint_active', `"${sprint.name}" has already started`);
      }
      const active = this.sprintsOf(sprint.projectId).find((other) => other.state === 'active');
      if (active) {
        throw new BackendError(
          'another_sprint_active',
          `"${active.name}" is still running; complete it before you start another sprint`,
        );
      }
      if (this.tasksOfSprint(sprint.id).length === 0) {
        throw new BackendError(
          'sprint_empty',
          `"${sprint.name}" has no tasks; plan some into it before you start it`,
        );
      }
      const name = tidySprintName(start.name);
      requireStartNotAfterEnd(start.startOn, start.endOn);

      sprint.name = name;
      sprint.goal = start.goal.trim();
      sprint.startOn = start.startOn;
      sprint.endOn = start.endOn;
      sprint.state = 'active';
      sprint.startedAt = this.nowAsText();
      return sprint;
    });
  }

  completeSprint(sprintId: Id, completion: SprintCompletion): Promise<Sprint> {
    return this.answer(() => {
      const sprint = this.requireSprint(sprintId);
      refuseCompleted(sprint, 'cannot be completed again');
      if (sprint.state !== 'active') {
        throw new BackendError(
          'sprint_not_active',
          `"${sprint.name}" has not started, so it cannot be completed`,
        );
      }
      const target =
        completion.moveUnfinishedTo === null
          ? null
          : this.requireSprint(completion.moveUnfinishedTo);
      if (target && target.projectId !== sprint.projectId) {
        throw new BackendError(
          'different_projects',
          'the unfinished tasks can only go to a sprint of the same project',
        );
      }
      if (target?.id === sprint.id) {
        throw new BackendError(
          'rule_broken',
          'the unfinished tasks cannot stay in the sprint that is completed',
        );
      }
      if (target) {
        refuseCompleted(target, 'cannot take the unfinished tasks');
      }

      // The tasks are counted as the manager sees them: a subtask is part
      // of its task and goes where its task goes.
      const mainTasks = this.tasksOfSprint(sprint.id).filter((task) => task.parentId === null);
      const now = this.nowAsText();
      for (const task of mainTasks) {
        if (this.categoryOf(task) === 'done') {
          continue;
        }
        for (const id of this.subtreeOf(task)) {
          const moving = this.tasks.get(id);
          if (moving) {
            moving.sprintId = target?.id ?? null;
            moving.updatedAt = now;
          }
        }
      }
      sprint.summary = summaryOf(mainTasks.map((task) => this.countedTask(task)));
      sprint.state = 'completed';
      sprint.completedAt = now;
      return sprint;
    });
  }

  planTask(taskId: Id, sprintId: Id | null): Promise<Task> {
    return this.answer(() => {
      const task = this.requireTask(taskId);
      const project = this.requireProject(task.projectId);
      const parent = task.parentId === null ? undefined : this.tasks.get(task.parentId);
      if (parent) {
        throw new BackendError(
          'subtask_planned_alone',
          `${referenceOf(project, task)} is a part of ${referenceOf(project, parent)} and goes where it goes`,
        );
      }
      if (sprintId !== null) {
        const sprint = this.requireSprint(sprintId);
        if (sprint.projectId !== task.projectId) {
          throw new BackendError(
            'different_projects',
            'a task can only be planned into a sprint of its own project',
          );
        }
        refuseCompleted(sprint, 'takes no more tasks');
      }
      const now = this.nowAsText();
      for (const id of this.subtreeOf(task)) {
        const planned = this.tasks.get(id);
        if (planned && planned.sprintId !== sprintId) {
          planned.sprintId = sprintId;
          planned.updatedAt = now;
        }
      }
      return task;
    });
  }

  setTaskPoints(taskId: Id, points: number | null): Promise<Task> {
    return this.answer(() => {
      requirePoints(points);
      return this.change(taskId, (task) => {
        task.points = points;
      });
    });
  }

  sprintReport(sprintId: Id): Promise<SprintReport> {
    return this.answer(() => {
      const sprint = this.requireSprint(sprintId);
      const mainTasks = this.tasksOfSprint(sprint.id).filter((task) => task.parentId === null);
      return reportOf(
        sprint,
        mainTasks.map((task) => this.countedTask(task)),
        dayOfMoment(this.nowAsText()),
      );
    });
  }

  /** The sprints of a project, in no particular order. */
  private sprintsOf(projectId: Id): Sprint[] {
    return [...this.sprints.values()].filter((sprint) => sprint.projectId === projectId);
  }

  /** The tasks of a sprint, its subtasks included. */
  private tasksOfSprint(sprintId: Id): Task[] {
    return [...this.tasks.values()].filter((task) => task.sprintId === sprintId);
  }

  private requireSprint(sprintId: Id): Sprint {
    const sprint = this.sprints.get(sprintId);
    if (!sprint) {
      throw new BackendError('not_found', `there is no sprint with the identifier ${sprintId}`);
    }
    return sprint;
  }

  /** A task as a sprint counts it. Finished means: in a status of the category done. */
  private countedTask(task: Task): CountedTask {
    return {
      points: task.points,
      completedAt: task.completedAt,
      isFinished: this.categoryOf(task) === 'done',
    };
  }

  /**
   * Answers a request the way `respond` does, and notes afterwards which
   * tasks the request has changed.
   */
  private answer<Result>(work: () => Result): Promise<Result> {
    return respond(() => {
      try {
        return work();
      } finally {
        this.noteVersions();
      }
    });
  }

  /**
   * Notes the versions of the tasks that have changed since they were last
   * noted, and forgets the versions of tasks that are gone. It runs after
   * everything that can change a task.
   */
  private noteVersions(): void {
    for (const task of this.tasks.values()) {
      const noted = this.versions.get(task.id) ?? [];
      const last = noted.at(-1);
      if (!last || JSON.stringify(last) !== JSON.stringify(task)) {
        this.versions.set(task.id, [...noted, structuredClone(task)]);
      }
    }
    for (const taskId of this.versions.keys()) {
      if (!this.tasks.has(taskId)) {
        this.versions.delete(taskId);
      }
    }
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
/** The efforts a model can be asked for. */
const EFFORTS: readonly Effort[] = ['low', 'medium', 'high', 'xhigh', 'max'];

/** The most characters the name of a model can have. */
const MODEL_NAME_MAX_LENGTH = 80;

/**
 * Checks the profile a member is to get. Only an AI specialist runs on a
 * model, so a person can only be given an empty profile. The name of a
 * model is one word; an empty one means the default.
 */
function tidyProfile(kind: MemberKind, name: string, profile: MemberProfile): MemberProfile {
  const model = profile.model?.trim() ?? '';
  const tidy: MemberProfile = { model: model === '' ? null : model, effort: profile.effort };
  if (kind !== 'agent' && (tidy.model !== null || tidy.effort !== null)) {
    throw new BackendError('no_specialist', `${name} is not an AI specialist`);
  }
  if (tidy.model !== null && (/\s/.test(tidy.model) || tidy.model.length > MODEL_NAME_MAX_LENGTH)) {
    throw new BackendError(
      'rule_broken',
      `the name of a model is one word of at most ${MODEL_NAME_MAX_LENGTH.toString()} characters`,
    );
  }
  if (tidy.effort !== null && !EFFORTS.includes(tidy.effort)) {
    throw new BackendError('invalid_request', 'the effort is not one of the known ones');
  }
  return tidy;
}

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

/** Refuses a name that a status of the workflow already has. Capital and small letters count as the same. */
function refuseStatusNameInUse(statuses: readonly Status[], name: string): void {
  const wanted = name.toLowerCase();
  if (statuses.some((status) => status.name.toLowerCase() === wanted)) {
    throw new BackendError('rule_broken', `the workflow already has a status named "${name}"`);
  }
}

/** Makes sure that a workflow keeps a status of every category it cannot do without. */
function requireCategories(categories: readonly StatusCategory[]): void {
  for (const required of REQUIRED_CATEGORIES) {
    if (!categories.includes(required)) {
      throw new BackendError(
        'rule_broken',
        `a workflow needs at least one status in the "${required}" category`,
      );
    }
  }
}

/**
 * Where a new status goes: behind the last status that is as far along in
 * the course of the work, or less far. A new step of the work then stands
 * with the other steps, and not behind the statuses in which work is done.
 */
function placeForNewStatus(statuses: readonly Status[], category: StatusCategory): number {
  let place = 0;
  statuses.forEach((status, position) => {
    if (PROGRESS_OF_CATEGORY[status.category] <= PROGRESS_OF_CATEGORY[category]) {
      place = position + 1;
    }
  });
  return place;
}

/** The name a manager knows a task by, such as `IOT-12`. */
function referenceOf(project: Project, task: Task): string {
  return `${project.key}-${task.number.toString()}`;
}

/** Trims a message of the manager's and refuses an empty one. */
/**
 * The question that is open in a conversation: the one its latest turn
 * ended with, if the manager has not answered it yet.
 */
function openDecisionOf(conversation: Conversation): DecisionRequest | undefined {
  const latest = conversation.turns.at(-1);
  return latest?.state !== 'running' && latest?.decision && latest.decision.answer === null
    ? latest.decision
    : undefined;
}

/** When the open question of a conversation was asked: when its latest turn ended. */
function askedAt(conversation: Conversation): Moment {
  return conversation.turns.at(-1)?.endedAt ?? conversation.startedAt;
}

/**
 * The answer of the manager as the specialist reads it: the option that was
 * chosen, and what the manager added in words.
 */
function answerInWords(option: string | undefined, note: string): string {
  if (option === undefined) {
    return note;
  }
  return note === '' ? `Decision: ${option}` : `Decision: ${option}\n\n${note}`;
}

/** What the Advisor is called where a member would be named. */
const ADVISOR_NAME = 'The Advisor';

/**
 * Checks the members of a proposal that the manager approves, and returns
 * them tidied. No two of them may have the same name.
 */
function tidyProposedMembers(members: readonly ProposedMember[]): ProposedMember[] {
  const names = new Set<string>();
  return members.map((member) => {
    const name = tidyName(member.name, 'name of the team member', MEMBER_NAME_MAX_LENGTH);
    if (names.has(name.toLowerCase())) {
      throw new BackendError('rule_broken', `two members of the proposal are called "${name}"`);
    }
    names.add(name.toLowerCase());
    return {
      name,
      responsibility: member.responsibility.trim(),
      reason: member.reason.trim(),
      ...tidyProfile('agent', name, { model: member.model, effort: member.effort }),
    };
  });
}

/**
 * Checks the tasks of a proposal that the manager approves, and returns
 * them tidied. A task that is given to a member names one of `known`: the
 * members of the team and of the proposal. A task waits only for other
 * tasks of the proposal, and the tasks do not wait in a circle.
 */
function tidyProposedTasks(
  tasks: readonly ProposedTask[],
  known: readonly string[],
): ProposedTask[] {
  const knownNames = new Set(known.map((name) => name.toLowerCase()));
  const tidy = tasks.map((task, place) => {
    const title = tidyName(task.title, 'task title', TASK_TITLE_MAX_LENGTH);
    const owner = task.owner;
    if (owner?.kind === 'member' && !knownNames.has(owner.name.trim().toLowerCase())) {
      throw new BackendError(
        'rule_broken',
        `"${title}" is given to "${owner.name.trim()}", who is neither on the team nor in the proposal`,
      );
    }
    const waitsFor = [...new Set(task.waitsFor)];
    if (
      waitsFor.some(
        (earlier) =>
          !Number.isInteger(earlier) || earlier < 0 || earlier >= tasks.length || earlier === place,
      )
    ) {
      throw new BackendError(
        'rule_broken',
        `"${title}" waits for a task that is not another task of the proposal`,
      );
    }
    return {
      title,
      description: task.description.trim(),
      owner:
        owner?.kind === 'member' ? { kind: 'member' as const, name: owner.name.trim() } : owner,
      estimateMinutes: task.estimateMinutes,
      priority: task.priority,
      waitsFor,
    };
  });
  if (waitInCircle(tidy)) {
    throw new BackendError(
      'rule_broken',
      'the tasks of the proposal wait for each other in a circle',
    );
  }
  return tidy;
}

/**
 * Refuses a work folder and a reference folder that get in each other's
 * way. The specialists write in the work folder and only read a reference
 * folder, so the one cannot be the other, lie inside it, or contain it.
 */
function refuseFoldersThatOverlap(workFolder: string, referenceFolder: string): void {
  if (isSameFolder(referenceFolder, workFolder)) {
    throw new BackendError(
      'rule_broken',
      `the work folder and a reference folder cannot be the same folder: ${workFolder}`,
    );
  }
  if (isInside(referenceFolder, workFolder)) {
    throw new BackendError(
      'rule_broken',
      `${referenceFolder} lies inside the work folder ${workFolder}, where the specialists read everything already`,
    );
  }
  if (isInside(workFolder, referenceFolder)) {
    throw new BackendError(
      'rule_broken',
      `the work folder ${workFolder} lies inside the reference folder ${referenceFolder}, where the specialists may not write`,
    );
  }
}

function tidyMessage(raw: string): string {
  const message = raw.trim();
  if (message === '') {
    throw new BackendError('rule_broken', 'the message must not be empty');
  }
  return message;
}

/**
 * Refuses to touch a completed sprint: it is the record of what was done.
 * `what` says what cannot be done to it, after its name.
 */
function refuseCompleted(sprint: Sprint, what: string): void {
  if (sprint.state === 'completed') {
    throw new BackendError('sprint_completed', `"${sprint.name}" is completed and ${what}`);
  }
}

/** Writes a number of tasks: "1 task", "3 tasks". */
function countOfTasks(count: number): string {
  return `${count.toString()} ${count === 1 ? 'task' : 'tasks'}`;
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
