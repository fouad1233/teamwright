import type {
  AgentProgram,
  ApprovedProposal,
  Area,
  Assignee,
  CalendarDate,
  Color,
  Conversation,
  DataChanged,
  Deliveries,
  Delivery,
  EventKind,
  FolderListing,
  GitHubItem,
  GitHubOverview,
  GitHubSignIn,
  GoogleDrive,
  Id,
  InterviewAnswer,
  IssueDraft,
  Member,
  MemberProfile,
  Moment,
  NewAdvice,
  NewDecisionAnswer,
  NewMember,
  NewProject,
  NewSprint,
  NewTask,
  NewView,
  NotificationChoice,
  NotificationKind,
  NotificationPressed,
  Priority,
  Project,
  ProjectChange,
  ProjectIntegrations,
  ProjectPlaces,
  ProjectTemplate,
  ProjectUsage,
  ProposalChange,
  ReadyMadeSpecialist,
  Sprint,
  SprintChanges,
  SprintCompletion,
  SprintForecast,
  SprintReport,
  SprintStart,
  StatusLoad,
  StatusProperties,
  SyncNotice,
  SyncState,
  Tag,
  TaggedTask,
  Task,
  TaskChange,
  TaskOnGitHub,
  TeamTalk,
  MemoryEntry,
  NewMemoryFact,
  ThemeName,
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
  /**
   * Creates a project with the workflow it asks for, the six views every
   * project starts with, and one area called "General".
   */
  createProject(project: NewProject): Promise<Project>;
  renameProject(projectId: Id, name: string): Promise<Project>;
  describeProject(projectId: Id, description: string): Promise<Project>;
  /** Puts a project away. Its tasks and its team are kept, and it can be brought back. */
  archiveProject(projectId: Id): Promise<Project>;
  /** Lists the projects that were put away, the one that was put away last first. */
  listArchivedProjects(): Promise<Project[]>;
  /** Brings a project back that was put away. */
  restoreProject(projectId: Id): Promise<Project>;
  /** Removes a project for good, with its tasks, its sprints, its views, its areas and its team. */
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
   * What lies in a folder of a project (its work folder or a reference
   * folder, as `projectPlaces` names it) at a path inside it, written with
   * `/`; empty for the folder itself. A path that leaves the folder, by
   * `..` or through a link, is refused (`rule_broken`); nothing at the
   * path is `not_found`. Names that begin with a dot are left out unless
   * `showsHidden`.
   */
  listProjectFiles(
    projectId: Id,
    folder: string,
    path: string,
    showsHidden: boolean,
  ): Promise<FolderListing>;
  /**
   * Opens a file of a project with the program this computer has for it.
   * A folder, or a file that would run as a program, is refused
   * (`invalid_request`).
   */
  openProjectFile(projectId: Id, folder: string, path: string): Promise<void>;
  /** Shows a file or folder of a project in the file manager of this computer. */
  showProjectFile(projectId: Id, folder: string, path: string): Promise<void>;

  /**
   * Lets the manager choose where to save a workbook of this computer,
   * suggesting a name. Resolves to the path, or to nothing if the manager
   * changed their mind.
   */
  chooseWorkbookFile(suggestedName: string): Promise<string | null>;
  /**
   * Writes a project as an Excel workbook to a file (decision 0025): a
   * summary, the tasks, the sprints and the team. Resolves to the path of
   * the file. A file that is open in another program, a full disk or a
   * folder that is gone is refused with the reason in words.
   */
  exportWorkbook(projectId: Id, path: string): Promise<string>;
  /**
   * Shows a workbook in its folder in the file manager of this computer.
   * Only a file the program wrote since it started can be shown.
   */
  showExportedWorkbook(path: string): Promise<void>;

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
  /**
   * Gives a status a work-in-progress limit from 1 to 999, another one, or
   * none (`null`). A status of the category done takes none. Setting a
   * limit below the tasks already in the status is allowed.
   */
  setStatusLimit(projectId: Id, statusId: Id, limit: number | null): Promise<Project>;
  /**
   * How many tasks stand in each status of a project, against its limit,
   * in board order. Every task of the project counts; a subtask does not
   * count apart from its task.
   */
  listStatusLoads(projectId: Id): Promise<StatusLoad[]>;
  /** Lists the templates a new project can start from, in the order the manager is shown them. */
  listProjectTemplates(): Promise<ProjectTemplate[]>;

  /** Lists the tasks of a project in the order the manager arranged them. */
  listTasks(projectId: Id): Promise<Task[]>;
  /**
   * Opens a task. It starts in the backlog without points, or, as a
   * subtask, in the sprint of the task it is a part of. It goes into the
   * area it names, or the first area of the project; a subtask goes into
   * the area of its task. An area of another project is refused
   * (`different_projects`), and so is an area for a subtask that is not the
   * area of its task (`rule_broken`).
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
   * Tells what happened to the tasks of a project, the newest change first:
   * at most `limit` changes (never more than 200) older than the moment
   * `before`, or the newest of all without it. Moments count to the
   * millisecond, and a page never splits the changes of one millisecond: the
   * next page asks with the `at` of the last change of this one. A page
   * shorter than `limit` is the last.
   */
  listProjectHistory(projectId: Id, limit: number, before?: Moment): Promise<ProjectChange[]>;

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
   * Tells what the completed sprints of a project say about the work ahead:
   * the pace of the team as a range, each planned sprint against it, and
   * how many sprints the unfinished work would take.
   */
  sprintForecast(projectId: Id): Promise<SprintForecast>;

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
   * Replaces the filter of a view. Only the kinds that show some of the tasks
   * can be filtered (see `ViewFilter`), and a member named by the filter is
   * on the team of the project.
   */
  setViewFilter(viewId: Id, filter: ViewFilter): Promise<View>;
  /**
   * Replaces what a view keeps of its own, or clears it with null. Any kind
   * of view may keep content; it has at most 2 MiB, counted in UTF-8 bytes.
   */
  setViewContent(viewId: Id, content: string | null): Promise<View>;
  /**
   * Moves a view to another place among the views of its project. Index 0
   * is the first place; an index past the end means the last. Answers with
   * all the views of the project in their new order.
   */
  moveView(viewId: Id, index: number): Promise<View[]>;
  /** Removes a view. The last view of a project cannot be removed. */
  removeView(viewId: Id): Promise<void>;

  /**
   * Lists the areas of a project in the order the manager gave them. A
   * project has at least one: a new project starts with one called
   * "General". Refused for a project that does not exist (`not_found`).
   */
  listAreas(projectId: Id): Promise<Area[]>;
  /**
   * Adds an area at the end of the areas of a project. Its name is trimmed
   * and put on one line; it is not empty and has at most 40 characters
   * (`rule_broken`), and no other area of the project has it, whatever the
   * capitals (`area_name_taken`). Refused for a project that does not exist
   * (`not_found`).
   */
  addArea(projectId: Id, name: string): Promise<Area>;
  /** Renames an area, by the same rules as a name given when it is added. */
  renameArea(areaId: Id, name: string): Promise<Area>;
  /**
   * Moves an area to another place among the areas of its project. Index 0
   * is the first place; an index past the end means the last, one before
   * the start the first. Answers with all the areas of the project in their
   * new order.
   */
  moveArea(areaId: Id, index: number): Promise<Area[]>;
  /**
   * Removes an area. The last area of a project cannot be removed
   * (`last_area`). If tasks are in it, `moveTasksTo` names the area of the
   * same project they go to (`area_in_use` without it, `different_projects`
   * for an area of another project); they keep their status and their
   * place in it. Without tasks, `moveTasksTo` is not looked at.
   */
  removeArea(areaId: Id, moveTasksTo?: Id): Promise<void>;
  /**
   * Moves a task, with its subtasks, into another area of its project
   * (`different_projects` for an area of another project). A subtask is not
   * moved by itself: it goes where the task it is a part of goes
   * (`subtask_moved_alone`). Answers with the task.
   */
  moveTaskToArea(taskId: Id, areaId: Id): Promise<Task>;

  /**
   * Lists the tags of a project by name, whatever the capitals. Refused for
   * a project that does not exist (`not_found`).
   */
  listTags(projectId: Id): Promise<Tag[]>;
  /**
   * Adds a tag to a project. Its name is trimmed and put on one line; it is
   * not empty and has at most 30 characters (`rule_broken`), and no other
   * tag of the project has it, whatever the capitals (`tag_name_taken`). A
   * project has at most 100 tags (`rule_broken`). Without a color, the tag
   * gets the next color in turn: blue, green, amber, purple, teal, pink,
   * red, gray.
   */
  addTag(projectId: Id, name: string, color?: Color): Promise<Tag>;
  /** Renames a tag, by the same rules as a name given when it is added. */
  renameTag(tagId: Id, name: string): Promise<Tag>;
  /** Gives a tag another color. */
  recolorTag(tagId: Id, color: Color): Promise<Tag>;
  /**
   * Removes a tag. It comes off every task that carries it, which their
   * history says, and a view that showed only the tasks with it shows the
   * tasks with any tag again.
   */
  removeTag(tagId: Id): Promise<void>;
  /**
   * Puts a tag of its project on a task (`different_projects` for a tag of
   * another project), and answers with the task. A tag the task carries
   * already changes nothing.
   */
  putTagOnTask(taskId: Id, tagId: Id): Promise<Task>;
  /**
   * Puts a tag on a task by its name, as the manager types it: the tag of
   * the project with that name, whatever the capitals, or a new tag by the
   * rules of `addTag`. Answers with the task and the tag.
   */
  putNamedTagOnTask(taskId: Id, name: string): Promise<TaggedTask>;
  /** Takes a tag off a task, and answers with the task. */
  takeTagOffTask(taskId: Id, tagId: Id): Promise<Task>;

  /**
   * Adds an item, not done, at the end of the checklist of a task. Its text
   * is trimmed and put on one line; it is not empty and has at most 200
   * characters, and a checklist has at most 100 items (`rule_broken`).
   * Every change to a checklist answers with the task.
   */
  addChecklistItem(taskId: Id, text: string): Promise<Task>;
  /** Gives an item another text, by the same rules (`not_found` for an item the task does not have). */
  renameChecklistItem(taskId: Id, itemId: Id, text: string): Promise<Task>;
  /** Marks an item done or not done. Ticking it shows in the history of the task. */
  tickChecklistItem(taskId: Id, itemId: Id, done: boolean): Promise<Task>;
  /**
   * Moves an item to another place in the checklist. Index 0 is the first
   * place; an index past the end means the last, one before the start the
   * first.
   */
  moveChecklistItem(taskId: Id, itemId: Id, index: number): Promise<Task>;
  /** Removes an item from the checklist. */
  removeChecklistItem(taskId: Id, itemId: Id): Promise<Task>;

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
  /**
   * Replaces the instructions of an AI specialist. They reach its turns from
   * the next one on, after Teamwright's own instructions. Text that is only
   * whitespace takes them away. A person has none (`no_specialist`); longer
   * than the limit is refused (`rule_broken`).
   */
  setMemberInstructions(memberId: Id, instructions: string): Promise<Member>;
  /** Lists the ready-made specialists that ship with Teamwright. */
  listReadyMadeSpecialists(): Promise<ReadyMadeSpecialist[]>;
  /**
   * Puts a ready-made specialist on the team, with its name, description,
   * instructions, model and effort. A name the team has already is numbered:
   * "Firmware engineer 2". An unknown key is `not_found`.
   */
  addReadyMadeMember(projectId: Id, key: string): Promise<Member>;

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
   * Every handoff and every message the specialists of a project handed
   * each other, the oldest first. They arrive while specialists work, so
   * they are asked for again when a conversation changes.
   */
  teamTalk(projectId: Id): Promise<TeamTalk>;
  /**
   * Every entry of the memory of a project, current and replaced, the
   * newest first. Entries arrive while specialists work and when the
   * manager answers a question, so they are asked for again when a
   * conversation changes.
   */
  projectMemory(projectId: Id): Promise<MemoryEntry[]>;
  /**
   * The current entries of a project that fit a few words, the newest
   * first: each word begins a word of the title or the text, whatever the
   * capitals. Blank words give every current entry. The specialists search
   * by the same rule. Too many words are refused (`rule_broken`).
   */
  searchMemory(projectId: Id, query: string): Promise<MemoryEntry[]>;
  /** Adds a fact the manager writes. Returns the new entry. */
  addMemoryFact(fact: NewMemoryFact): Promise<MemoryEntry>;
  /**
   * Changes an entry: the words become a newer entry of the same kind and
   * task that replaces it, and the older one stays and says so. An entry
   * replaced already is refused (`rule_broken`). Returns the newer entry.
   */
  changeMemoryEntry(entryId: Id, title: string, text: string): Promise<MemoryEntry>;
  /** Removes an entry from memory. */
  removeMemoryEntry(entryId: Id): Promise<void>;
  /**
   * What the specialists and the Advisor of a project have spent, in tokens
   * of every kind, and how full the context of each is. Worked out from the
   * turns; it changes when a turn ends.
   */
  projectUsage(projectId: Id): Promise<ProjectUsage>;
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
   * Makes one change to the proposal the Advisor made in the latest turn
   * of a conversation, before it is approved. The change is stored at
   * once, so it is kept over a restart and reaches the other computers. A
   * change that breaks a rule is refused (`rule_broken`) with the reason
   * in words, and changes nothing.
   */
  changeProposal(conversationId: Id, change: ProposalChange): Promise<Conversation>;
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
   * What there is of Google Drive on this computer: the folders that Google
   * Drive for desktop keeps (on Windows its drive and the mirrored folder in
   * the home, on macOS one folder for each account), and whether the sync
   * folder lies inside one of them. On Linux nothing is looked for.
   */
  googleDrive(): Promise<GoogleDrive>;
  /**
   * Makes the folder `Teamwright` inside a folder of Drive that was found,
   * if it is not there, and answers with its path: the folder to sync
   * through, chosen afterwards with `chooseSyncFolder`. Refused if the
   * folder is not one found on this computer (`folder_not_found`).
   */
  prepareGoogleDriveFolder(folder: string): Promise<string>;
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

  /** The theme of the window on this computer: `lamplight` until the manager chooses another. */
  theme(): Promise<ThemeName>;
  /** Chooses the theme of the window on this computer, and returns it. */
  chooseTheme(name: ThemeName): Promise<ThemeName>;

  /**
   * What the manager chose about notifications of the operating system on
   * this computer. They are off until the manager allows them.
   */
  notificationChoice(): Promise<NotificationChoice>;
  /** Allows notifications on this computer, with the kinds as they were. */
  allowNotifications(): Promise<NotificationChoice>;
  /**
   * Turns notifications off on this computer, or says no to the question
   * whether to allow them. The program does not ask again.
   */
  declineNotifications(): Promise<NotificationChoice>;
  /** Turns one kind of notification on or off. */
  setNotificationKind(kind: NotificationKind, on: boolean): Promise<NotificationChoice>;
  /**
   * Sends a notification to try them. Refused while they are not allowed
   * (`notifications_off`), and where there are none (`not_available`).
   */
  sendTestNotification(): Promise<void>;
  /**
   * Calls the listener when the manager pressed a notification, with what
   * it was about. The window is in front by then. Returns the function
   * that stops the calls.
   */
  onNotificationPressed(listener: (pressed: NotificationPressed) => void): () => void;

  /**
   * Opens a web address in the manager's own browser. Only an address that
   * starts with `https://` and names a host is opened (`invalid_request`
   * otherwise).
   */
  openWebAddress(address: string): Promise<void>;

  /** The integrations of a project on this computer. */
  projectIntegrations(projectId: Id): Promise<ProjectIntegrations>;
  /**
   * Names the GitHub repository of a project (`owner/name`), or, with null,
   * reads it from the work folder again.
   */
  setRepository(projectId: Id, name: string | null): Promise<ProjectIntegrations>;
  /** Sets the address of the project's Slack channel (https), or takes it away with null. */
  setSlackAddress(projectId: Id, address: string | null): Promise<ProjectIntegrations>;
  /** Tells, or stops telling, a kind of event in the Slack channel (`no_channel` without one). */
  setSlackEvent(projectId: Id, kind: EventKind, on: boolean): Promise<ProjectIntegrations>;
  /** Adds a web address (https) that hears every kind of event. At most five. */
  addWebAddress(
    projectId: Id,
    address: string,
    secret: string | null,
  ): Promise<ProjectIntegrations>;
  /** Takes away the web address at a position. */
  removeWebAddress(projectId: Id, index: number): Promise<ProjectIntegrations>;
  /** Sends, or stops sending, a kind of event to the web address at a position. */
  setWebAddressEvent(
    projectId: Id,
    index: number,
    kind: EventKind,
    on: boolean,
  ): Promise<ProjectIntegrations>;

  /** Whether the manager's `gh` is there and signed in. Asks it, which may take a moment. */
  gitHubSignIn(): Promise<GitHubSignIn>;
  /** Names the `gh` of this computer, or, with null, looks for it again. */
  setGitHubProgram(program: string | null): Promise<GitHubSignIn>;
  /** GitHub for a project, from what was last read. Nothing is read. */
  gitHubOverview(projectId: Id): Promise<GitHubOverview>;
  /**
   * Reads the pull requests and issues of the project's repository again
   * (`no_repository` without one, `github_unavailable` when `gh` cannot).
   */
  refreshGitHub(projectId: Id): Promise<GitHubOverview>;
  /**
   * The pull requests and issues about a task. The repository is read once
   * if it was not read since the program started, and again with `refresh`.
   */
  taskOnGitHub(taskId: Id, refresh: boolean): Promise<TaskOnGitHub>;
  /** What an issue opened for a task would say, and where it would go. Nothing is sent. */
  issueDraft(taskId: Id): Promise<IssueDraft>;
  /** Opens the issue of `issueDraft` on GitHub. Only after the manager confirmed it. */
  openIssue(taskId: Id): Promise<GitHubItem>;

  /** What was delivered to the channel and the web addresses of a project since the program started. */
  integrationDeliveries(projectId: Id): Promise<Deliveries>;
  /** Sends a line to the project's Slack channel to try it (`no_channel` without one). */
  testChannel(projectId: Id): Promise<Delivery>;
  /** Sends the event `test` to the web address at a position. */
  testWebAddress(projectId: Id, index: number): Promise<Delivery>;
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
