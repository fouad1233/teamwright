import type {
  AgentProgram,
  Assignee,
  CalendarDate,
  Conversation,
  Id,
  Member,
  MemberProfile,
  NewDecisionAnswer,
  NewMember,
  NewProject,
  NewTask,
  Priority,
  Project,
  ProjectPlaces,
  StatusProperties,
  Task,
  TaskChange,
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

  /** Says where the specialists of a project work and what else they may read. */
  projectPlaces(projectId: Id): Promise<ProjectPlaces>;
  /**
   * Lets the specialists of a project work in a folder the manager chose,
   * for example one that already holds the project. With nothing, they work
   * in the folder Teamwright gives the project. It holds from the next turn
   * on, and cannot change while a specialist works in the project.
   */
  setWorkFolder(projectId: Id, folder: string | null): Promise<ProjectPlaces>;
  /**
   * Lets the specialists of a project read a folder, and not change it. The
   * folder cannot be the work folder, lie inside it, or contain it.
   */
  addReferenceFolder(projectId: Id, folder: string): Promise<ProjectPlaces>;
  /** Takes a folder away from what the specialists of a project may read. */
  removeReferenceFolder(projectId: Id, folder: string): Promise<ProjectPlaces>;
  /**
   * Copies files into the folder `references` inside the work folder of a
   * project, where its specialists can read them. Nothing else of the
   * folders the files come from is shown to them. Answers with the names
   * the files have there, which differ from their own when a name was
   * taken.
   */
  addReferenceFiles(projectId: Id, files: string[]): Promise<string[]>;
  /**
   * Lets the manager pick a folder of this computer. Resolves to its path,
   * or to nothing if the manager changed their mind.
   */
  chooseFolder(): Promise<string | null>;
  /** Lets the manager pick files of this computer. Resolves to their paths. */
  chooseFiles(): Promise<string[]>;
  /**
   * Shows the work folder of a project in the file manager of this
   * computer. The interface names the project and not a folder, so that it
   * can never make the program open a place the manager did not choose.
   */
  showWorkFolder(projectId: Id): Promise<void>;

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
  /** Lists what has happened to a task since it was opened, the oldest change first. */
  listTaskHistory(taskId: Id): Promise<TaskChange[]>;

  /** Lists the team of a project. The member who joined first comes first. */
  listMembers(projectId: Id): Promise<Member[]>;
  addMember(member: NewMember): Promise<Member>;
  renameMember(memberId: Id, name: string): Promise<Member>;
  /** Replaces what a member is on the team for. */
  setMemberResponsibility(memberId: Id, responsibility: string): Promise<Member>;
  /**
   * Says which model an AI specialist runs on and how hard it thinks. It
   * holds from the next turn on. A person has no profile.
   */
  setMemberProfile(memberId: Id, profile: MemberProfile): Promise<Member>;
  /** Takes a member off the team. The member's tasks are given to nobody. */
  removeMember(memberId: Id): Promise<void>;

  /** Says whether the program that runs the AI specialists is installed and signed in. */
  agentProgram(): Promise<AgentProgram>;
  /** Lists the conversations of a project with their turns, the oldest first. */
  listConversations(projectId: Id): Promise<Conversation[]>;
  /**
   * Lets the AI specialist a task is given to start on it. The specialist
   * gets a brief that the program writes from the task, and the task moves
   * to the first status in which work is under way. When the specialist has
   * finished, the task moves on to the first status for work that waits to
   * be checked, if the workflow has one.
   */
  startWork(taskId: Id): Promise<Conversation>;
  /**
   * Sends the specialist of a conversation a message of the manager's. It
   * is a new turn of the same session, so the specialist knows what went
   * before.
   */
  reply(conversationId: Id, message: string): Promise<Conversation>;
  /** Asks an AI specialist something that is not about a task. It starts a new conversation. */
  ask(memberId: Id, message: string): Promise<Conversation>;
  /**
   * Answers the question a specialist asked the manager in a conversation:
   * with one of its options, with words of the manager's own, or with both.
   * The specialist goes on in the same session with the answer. While a
   * question is open, this is the only way to go on with the conversation.
   */
  answerDecision(conversationId: Id, answer: NewDecisionAnswer): Promise<Conversation>;
  /**
   * Lists the conversations in which a specialist waits for a decision of
   * the manager, in every project that is not put away. The one that has
   * waited longest comes first.
   */
  listConversationsWaiting(): Promise<Conversation[]>;
  /** Stops the turn that is running in a conversation. */
  stopWork(conversationId: Id): Promise<Conversation>;
  /** Opens a conversation in the Claude desktop app, to look at all of it there. */
  openInClaude(conversationId: Id): Promise<void>;
  /**
   * Calls the listener whenever a conversation changes: a turn starts,
   * takes a step, or ends. Returns the function that stops the calls.
   */
  onConversationChanged(listener: (conversation: Conversation) => void): () => void;
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
