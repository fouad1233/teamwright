import type {
  Assignee,
  CalendarDate,
  Id,
  Member,
  NewMember,
  NewProject,
  NewTask,
  Priority,
  Project,
  StatusProperties,
  Task,
} from './types';

/**
 * Everything the interface can ask of the application.
 *
 * The interface never talks to the desktop shell directly. It talks to a
 * `Backend`, which has two implementations: `TauriBackend` calls the real
 * application, and `DemoBackend` runs in the browser with sample data.
 */
export interface Backend {
  /** Lists the projects that are not archived, oldest first. */
  listProjects(): Promise<Project[]>;
  createProject(project: NewProject): Promise<Project>;
  renameProject(projectId: Id, name: string): Promise<Project>;
  describeProject(projectId: Id, description: string): Promise<Project>;
  /** Puts a project away. Its tasks and its team are kept, and it can be brought back. */
  archiveProject(projectId: Id): Promise<Project>;
  /** Lists the projects that were put away, the one that was put away last first. */
  listArchivedProjects(): Promise<Project[]>;
  /** Brings a project back that was put away. */
  restoreProject(projectId: Id): Promise<Project>;
  /** Removes a project for good, with its tasks and its team. */
  deleteProject(projectId: Id): Promise<void>;

  /**
   * Adds a status to the workflow of a project. It goes behind the last
   * status that is as far along in the course of the work, or less far.
   */
  addStatus(projectId: Id, status: StatusProperties): Promise<Project>;
  /**
   * Replaces the name, the category and the color of a status. If the
   * category changes between done and not done, the tasks in the status are
   * completed or reopened with it.
   */
  changeStatus(projectId: Id, statusId: Id, status: StatusProperties): Promise<Project>;
  /** Moves a status to another place in the workflow. Index 0 is the first place. */
  moveStatus(projectId: Id, statusId: Id, index: number): Promise<Project>;
  /**
   * Removes a status from the workflow. If tasks are in it, `moveTasksTo`
   * must name the status they go to; they are put at its bottom.
   */
  removeStatus(projectId: Id, statusId: Id, moveTasksTo?: Id): Promise<Project>;

  /** Lists the tasks of a project in the order the manager arranged them. */
  listTasks(projectId: Id): Promise<Task[]>;
  createTask(task: NewTask): Promise<Task>;
  renameTask(taskId: Id, title: string): Promise<Task>;
  describeTask(taskId: Id, description: string): Promise<Task>;
  prioritizeTask(taskId: Id, priority: Priority): Promise<Task>;
  assignTask(taskId: Id, assignee: Assignee | null): Promise<Task>;
  estimateTask(taskId: Id, minutes: number | null): Promise<Task>;
  rescheduleTask(
    taskId: Id,
    startOn: CalendarDate | null,
    dueOn: CalendarDate | null,
  ): Promise<Task>;
  /**
   * Moves a task to a status and to a position among the other tasks there.
   * Index 0 is the top; an index past the end means the bottom.
   */
  moveTask(taskId: Id, statusId: Id, index: number): Promise<Task>;
  /** Makes a task wait until another task is finished. */
  addTaskDependency(taskId: Id, blockerId: Id): Promise<Task>;
  removeTaskDependency(taskId: Id, blockerId: Id): Promise<Task>;
  /** Removes a task together with its subtasks. */
  deleteTask(taskId: Id): Promise<void>;

  /** Lists the team of a project. The member who joined first comes first. */
  listMembers(projectId: Id): Promise<Member[]>;
  addMember(member: NewMember): Promise<Member>;
  renameMember(memberId: Id, name: string): Promise<Member>;
  /** Replaces what a member is on the team for. */
  setMemberResponsibility(memberId: Id, responsibility: string): Promise<Member>;
  /** Takes a member off the team. The member's tasks are given to nobody. */
  removeMember(memberId: Id): Promise<void>;
}

/**
 * Why the backend refused or failed a request.
 *
 * `code` is stable and says what kind of problem it is. `message` is written
 * for the person using the program and can be shown as it is.
 */
export class BackendError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = 'BackendError';
    this.code = code;
  }
}
