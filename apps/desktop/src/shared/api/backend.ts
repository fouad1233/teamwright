import type {
  AgentProgram,
  ApprovedProposal,
  Assignee,
  CalendarDate,
  Conversation,
  DataChanged,
  Id,
  InterviewAnswer,
  Member,
  MemberProfile,
  NewAdvice,
  NewDecisionAnswer,
  NewMember,
  NewProject,
  NewSprint,
  NewTask,
  NewView,
  Priority,
  Project,
  ProjectPlaces,
  Sprint,
  SprintChanges,
  SprintCompletion,
  SprintReport,
  SprintStart,
  StatusProperties,
  SyncNotice,
  SyncState,
  Task,
  TaskChange,
  View,
  ViewFilter,
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
  /** Creates a project with the workflow it asks for and the six views every project starts with. */
  createProject(project: NewProject): Promise<Project>;
  renameProject(projectId: Id, name: string): Promise<Project>;
  describeProject(projectId: Id, description: string): Promise<Project>;
  /** Puts a project away. Its tasks and its team are kept, and it can be brought back. */
  archiveProject(projectId: Id): Promise<Project>;
  /** Lists the projects that were put away, the one that was put away last first. */
  listArchivedProjects(): Promise<Project[]>;
  /** Brings a project back that was put away. */
  restoreProject(projectId: Id): Promise<Project>;
  /** Removes a project for good, with its tasks, its sprints, its views and its team. */
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
  /**
   * Opens a task. It starts in the backlog without points, or, as a
   * subtask, in the sprint of the task it is a part of.
   */
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
  /** Removes a task together with its subtasks. They leave their sprint with it. */
  deleteTask(taskId: Id): Promise<void>;
  /** Lists what has happened to a task since it was opened, the oldest change first. */
  listTaskHistory(taskId: Id): Promise<TaskChange[]>;

  /**
   * Lists the sprints of a project: the active one first, then the planned
   * ones in the order they were created, then the completed ones, the one
   * completed last first.
   */
  listSprints(projectId: Id): Promise<Sprint[]>;
  /**
   * Plans a new sprint. Its name is trimmed and has at most 60 characters;
   * left empty, it is "Sprint N", N being one more than the number of
   * sprints the project has had. The start date is not after the end date.
   */
  createSprint(sprint: NewSprint): Promise<Sprint>;
  /**
   * Replaces the name, the goal and the dates of a sprint that is planned
   * or active. A completed sprint is not changed.
   */
  changeSprint(sprintId: Id, changes: SprintChanges): Promise<Sprint>;
  /** Removes a planned sprint. Its tasks go back to the backlog. */
  deleteSprint(sprintId: Id): Promise<void>;
  /**
   * Starts a planned sprint with its name, goal and dates. It needs at
   * least one task, and no other sprint of the project may be active.
   */
  startSprint(sprintId: Id, start: SprintStart): Promise<Sprint>;
  /**
   * Completes the active sprint. Its unfinished tasks, with their subtasks,
   * go to the backlog or to a planned sprint of the same project. The
   * finished ones stay in it as its record, and the sprint keeps a summary
   * of what was committed and what was completed.
   */
  completeSprint(sprintId: Id, completion: SprintCompletion): Promise<Sprint>;
  /**
   * Puts a task into a planned or an active sprint, or with nothing back
   * into the backlog. Its subtasks go with it. A subtask is not planned by
   * itself: it goes where the task it is a part of goes.
   */
  planTask(taskId: Id, sprintId: Id | null): Promise<Task>;
  /** Gives a task story points, a whole number from 0 to 999, or takes them away. */
  setTaskPoints(taskId: Id, points: number | null): Promise<Task>;
  /** Tells how much work remained in a sprint at the end of each of its days. */
  sprintReport(sprintId: Id): Promise<SprintReport>;

  /**
   * Lists the views of a project in the order the manager gave them. A new
   * project starts with one view of each kind: board, backlog, list, gantt,
   * calendar and dashboard, in that order, each named after its kind.
   */
  listViews(projectId: Id): Promise<View[]>;
  /**
   * Adds a view at the end of the views of a project, showing every task.
   * Its name is trimmed, not empty, at most 40 characters long, and not the
   * name of another view of the project, whatever the capitals. Left out or
   * empty, it is the name of its kind, numbered if that is taken: "Board 2".
   * A project has at most one backlog and one dashboard.
   */
  addView(view: NewView): Promise<View>;
  /** Renames a view, by the same rules as a name given when it is added. */
  renameView(viewId: Id, name: string): Promise<View>;
  /**
   * Replaces the filter of a view. Only a board, a list, a Gantt chart and a
   * calendar can be filtered, and a member named by the filter is on the
   * team of the project.
   */
  setViewFilter(viewId: Id, filter: ViewFilter): Promise<View>;
  /**
   * Moves a view to another place among the views of its project. Index 0
   * is the first place; an index past the end means the last. Answers with
   * all the views of the project in their new order.
   */
  moveView(viewId: Id, index: number): Promise<View[]>;
  /** Removes a view. The last view of a project cannot be removed. */
  removeView(viewId: Id): Promise<void>;

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
  /**
   * Takes a member off the team. The member's tasks are given to nobody, and
   * a view that showed only the member's tasks shows everybody's again.
   */
  removeMember(memberId: Id): Promise<void>;

  /** Says whether the program that runs the AI specialists is installed and signed in. */
  agentProgram(): Promise<AgentProgram>;
  /**
   * Lists the conversations of a project with their turns, the oldest
   * first, also those that run on another computer. A conversation can only
   * be continued on the computer it runs on: replying, answering,
   * approving and stopping are refused elsewhere (`runs_elsewhere`).
   */
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
   * before. In a conversation with the Advisor it is how the manager asks
   * for a change to a proposal, or for another round of planning.
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
   * Starts planning a project with the Advisor: a new conversation that
   * begins with what the manager wants. The Advisor can read the folders of
   * the project and change nothing. It asks what it needs to know and then
   * proposes whom to add to the team and which tasks to create. One
   * conversation with the Advisor runs in a project at a time.
   */
  advise(projectId: Id, advice: NewAdvice): Promise<Conversation>;
  /**
   * Answers the questions the Advisor asked in a conversation, one answer
   * for each question, in their order. The Advisor goes on in the same
   * session with the answers. While its questions are open, this is the
   * only way to go on with the conversation.
   */
  answerInterview(conversationId: Id, answers: InterviewAnswer[]): Promise<Conversation>;
  /**
   * Approves the proposal the Advisor made in the latest turn of a
   * conversation, as the manager changed it. The members are added to the
   * team and the tasks are created, all of them or nothing. A proposed
   * member whose name the team already has is not added again, and a task
   * given to that name goes to the member the team has. The tasks start in
   * the first status of the workflow, and no work starts by itself.
   */
  approveProposal(conversationId: Id, proposal: ApprovedProposal): Promise<Conversation>;
  /**
   * Lists the conversations that wait for the manager, in every project
   * that is not put away: a specialist waits for a decision, or the Advisor
   * waits for the answers to its questions or for a word about its
   * proposal. The one that has waited longest comes first.
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

  /**
   * Says where keeping this computer in step with the manager's other
   * computers stands: the folder, the name of this computer, the other
   * computers, and how the last round went.
   */
  syncState(): Promise<SyncState>;
  /**
   * Chooses the folder to keep computers in step through, a folder the
   * manager owns (for example one that a cloud drive keeps in step), and
   * names this computer. The name is written into the folder, where the
   * other computers read it. Answers after a first round, which writes the
   * whole log of this computer into the folder; what went wrong in it is in
   * the state. Refused if the folder is not named or the name is empty or
   * longer than 60 characters (`rule_broken`), if the folder does not exist
   * (`folder_not_found`), or if it lies inside Teamwright's own data folder
   * (`sync_folder_in_data`).
   */
  chooseSyncFolder(folder: string, computerName: string): Promise<SyncState>;
  /** Stops syncing. Nothing in the folder is removed; choosing it again goes on where it stopped. */
  stopSyncing(): Promise<SyncState>;
  /**
   * Runs a round now instead of waiting for the next one, and answers after
   * it. Refused while no folder is chosen (`sync_off`), when the folder
   * cannot be reached (`sync_folder_unreachable`), and when another
   * computer uses the identity of this one or a new identity waits for
   * Teamwright to start again (`identity_in_use`).
   */
  syncNow(): Promise<SyncState>;
  /**
   * Gives this computer another name, which the other computers see after
   * the next round. Refused if the name is empty or longer than 60
   * characters (`rule_broken`).
   */
  renameThisComputer(name: string): Promise<SyncState>;
  /**
   * Takes in what another computer writes into the folder, from now on and
   * all it wrote before. Answers after a round. Refused for a computer that
   * is not known (`not_found`).
   */
  acceptComputer(computerId: Id): Promise<SyncState>;
  /**
   * Takes in nothing more of another computer. What was taken in stays.
   * Refused for a computer that is not known (`not_found`).
   */
  ignoreComputer(computerId: Id): Promise<SyncState>;
  /**
   * Lists what syncing tells the manager, the newest first: a task or a
   * project that got a new number or key, a change that was dropped, a task
   * that moved, a clock that is ahead.
   */
  syncNotices(): Promise<SyncNotice[]>;
  /** Forgets a notice. A notice that is not kept is ignored. */
  dismissSyncNotice(noticeId: number): Promise<void>;
  /**
   * Gives this computer a new identity, for when its data folder was copied
   * to another computer, which now writes under the same identity. Syncing
   * stays stopped until Teamwright starts again. The old identity then
   * counts as another computer, so nothing is lost; the conversations with
   * the specialists stay with it.
   */
  giveThisComputerANewIdentity(): Promise<SyncState>;
  /**
   * Calls the listener with the whole state of syncing whenever it changes:
   * after every round, also one that failed. Returns the function that
   * stops the calls.
   */
  onSyncChanged(listener: (state: SyncState) => void): () => void;
  /**
   * Calls the listener when syncing took in changes from another computer,
   * with the projects they changed. Whatever is shown of those projects is
   * to be asked for again. Returns the function that stops the calls.
   */
  onDataChanged(listener: (change: DataChanged) => void): () => void;
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
