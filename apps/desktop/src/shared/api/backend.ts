import type {
  Assignee,
  CalendarDate,
  Id,
  NewProject,
  NewTask,
  Priority,
  Project,
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
  archiveProject(projectId: Id): Promise<Project>;

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
