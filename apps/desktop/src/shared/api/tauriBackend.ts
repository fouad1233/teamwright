import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';

import { BackendError, type Backend } from './backend';
import type {
  AgentProgram,
  ApprovedProposal,
  Area,
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
 * The backend of the desktop application. Every method calls one command of
 * the Tauri shell (`src-tauri/src/commands`).
 */
export class TauriBackend implements Backend {
  listProjects(): Promise<Project[]> {
    return call('list_projects');
  }

  createProject(project: NewProject): Promise<Project> {
    return call('create_project', { input: project });
  }

  renameProject(projectId: Id, name: string): Promise<Project> {
    return call('rename_project', { projectId, name });
  }

  describeProject(projectId: Id, description: string): Promise<Project> {
    return call('describe_project', { projectId, description });
  }

  archiveProject(projectId: Id): Promise<Project> {
    return call('archive_project', { projectId });
  }

  listArchivedProjects(): Promise<Project[]> {
    return call('list_archived_projects');
  }

  restoreProject(projectId: Id): Promise<Project> {
    return call('restore_project', { projectId });
  }

  deleteProject(projectId: Id): Promise<void> {
    return call('delete_project', { projectId });
  }

  addStatus(projectId: Id, status: StatusProperties): Promise<Project> {
    return call('add_status', { projectId, status });
  }

  changeStatus(projectId: Id, statusId: Id, status: StatusProperties): Promise<Project> {
    return call('change_status', { projectId, statusId, status });
  }

  moveStatus(projectId: Id, statusId: Id, index: number): Promise<Project> {
    return call('move_status', { projectId, statusId, index });
  }

  removeStatus(projectId: Id, statusId: Id, moveTasksTo?: Id): Promise<Project> {
    return call('remove_status', { projectId, statusId, moveTasksTo: moveTasksTo ?? null });
  }

  listTasks(projectId: Id): Promise<Task[]> {
    return call('list_tasks', { projectId });
  }

  createTask(task: NewTask): Promise<Task> {
    return call('create_task', { input: task });
  }

  renameTask(taskId: Id, title: string): Promise<Task> {
    return call('rename_task', { taskId, title });
  }

  describeTask(taskId: Id, description: string): Promise<Task> {
    return call('describe_task', { taskId, description });
  }

  prioritizeTask(taskId: Id, priority: Priority): Promise<Task> {
    return call('prioritize_task', { taskId, priority });
  }

  assignTask(taskId: Id, assignee: Assignee | null): Promise<Task> {
    return call('assign_task', { taskId, assignee });
  }

  estimateTask(taskId: Id, minutes: number | null): Promise<Task> {
    return call('estimate_task', { taskId, minutes });
  }

  rescheduleTask(
    taskId: Id,
    startOn: CalendarDate | null,
    dueOn: CalendarDate | null,
  ): Promise<Task> {
    return call('reschedule_task', { taskId, startOn, dueOn });
  }

  moveTask(taskId: Id, statusId: Id, index: number): Promise<Task> {
    return call('move_task', { taskId, statusId, index });
  }

  addTaskDependency(taskId: Id, blockerId: Id): Promise<Task> {
    return call('add_task_dependency', { taskId, blockerId });
  }

  removeTaskDependency(taskId: Id, blockerId: Id): Promise<Task> {
    return call('remove_task_dependency', { taskId, blockerId });
  }

  deleteTask(taskId: Id): Promise<void> {
    return call('delete_task', { taskId });
  }

  listTaskHistory(taskId: Id): Promise<TaskChange[]> {
    return call('list_task_history', { taskId });
  }

  listSprints(projectId: Id): Promise<Sprint[]> {
    return call('list_sprints', { projectId });
  }

  createSprint(sprint: NewSprint): Promise<Sprint> {
    return call('create_sprint', { input: sprint });
  }

  changeSprint(sprintId: Id, changes: SprintChanges): Promise<Sprint> {
    return call('change_sprint', { sprintId, changes });
  }

  deleteSprint(sprintId: Id): Promise<void> {
    return call('delete_sprint', { sprintId });
  }

  startSprint(sprintId: Id, start: SprintStart): Promise<Sprint> {
    return call('start_sprint', { sprintId, input: start });
  }

  completeSprint(sprintId: Id, completion: SprintCompletion): Promise<Sprint> {
    return call('complete_sprint', { sprintId, input: completion });
  }

  planTask(taskId: Id, sprintId: Id | null): Promise<Task> {
    return call('plan_task', { taskId, sprintId });
  }

  setTaskPoints(taskId: Id, points: number | null): Promise<Task> {
    return call('set_task_points', { taskId, points });
  }

  sprintReport(sprintId: Id): Promise<SprintReport> {
    return call('sprint_report', { sprintId });
  }

  listViews(projectId: Id): Promise<View[]> {
    return call('list_views', { projectId });
  }

  addView(view: NewView): Promise<View> {
    return call('add_view', { input: view });
  }

  renameView(viewId: Id, name: string): Promise<View> {
    return call('rename_view', { viewId, name });
  }

  setViewFilter(viewId: Id, filter: ViewFilter): Promise<View> {
    return call('set_view_filter', { viewId, filter });
  }

  moveView(viewId: Id, index: number): Promise<View[]> {
    return call('move_view', { viewId, index });
  }

  removeView(viewId: Id): Promise<void> {
    return call('remove_view', { viewId });
  }

  listAreas(projectId: Id): Promise<Area[]> {
    return call('list_areas', { projectId });
  }

  addArea(projectId: Id, name: string): Promise<Area> {
    return call('add_area', { projectId, name });
  }

  renameArea(areaId: Id, name: string): Promise<Area> {
    return call('rename_area', { areaId, name });
  }

  moveArea(areaId: Id, index: number): Promise<Area[]> {
    return call('move_area', { areaId, index });
  }

  removeArea(areaId: Id, moveTasksTo?: Id): Promise<void> {
    return call('remove_area', { areaId, moveTasksTo: moveTasksTo ?? null });
  }

  moveTaskToArea(taskId: Id, areaId: Id): Promise<Task> {
    return call('move_task_to_area', { taskId, areaId });
  }

  projectPlaces(projectId: Id): Promise<ProjectPlaces> {
    return call('project_places', { projectId });
  }

  setWorkFolder(projectId: Id, folder: string | null): Promise<ProjectPlaces> {
    return call('set_work_folder', { projectId, folder });
  }

  addReferenceFolder(projectId: Id, folder: string): Promise<ProjectPlaces> {
    return call('add_reference_folder', { projectId, folder });
  }

  removeReferenceFolder(projectId: Id, folder: string): Promise<ProjectPlaces> {
    return call('remove_reference_folder', { projectId, folder });
  }

  addReferenceFiles(projectId: Id, files: string[]): Promise<string[]> {
    return call('add_reference_files', { projectId, files });
  }

  async chooseFolder(): Promise<string | null> {
    // The window of the operating system for picking a folder. It answers
    // with nothing when the manager closes it without picking.
    return open({ directory: true, multiple: false, title: 'Choose a folder' });
  }

  async chooseFiles(): Promise<string[]> {
    const picked = await open({ directory: false, multiple: true, title: 'Choose files' });
    return picked ?? [];
  }

  showWorkFolder(projectId: Id): Promise<void> {
    return call('show_work_folder', { projectId });
  }

  listMembers(projectId: Id): Promise<Member[]> {
    return call('list_members', { projectId });
  }

  addMember(member: NewMember): Promise<Member> {
    return call('add_member', { input: member });
  }

  renameMember(memberId: Id, name: string): Promise<Member> {
    return call('rename_member', { memberId, name });
  }

  setMemberResponsibility(memberId: Id, responsibility: string): Promise<Member> {
    return call('set_member_responsibility', { memberId, responsibility });
  }

  setMemberProfile(memberId: Id, profile: MemberProfile): Promise<Member> {
    return call('set_member_profile', { memberId, profile });
  }

  removeMember(memberId: Id): Promise<void> {
    return call('remove_member', { memberId });
  }

  agentProgram(): Promise<AgentProgram> {
    return call('agent_program');
  }

  listConversations(projectId: Id): Promise<Conversation[]> {
    return call('list_conversations', { projectId });
  }

  startWork(taskId: Id): Promise<Conversation> {
    return call('start_work', { taskId });
  }

  reply(conversationId: Id, message: string): Promise<Conversation> {
    return call('reply', { conversationId, message });
  }

  ask(memberId: Id, message: string): Promise<Conversation> {
    return call('ask', { memberId, message });
  }

  answerDecision(conversationId: Id, answer: NewDecisionAnswer): Promise<Conversation> {
    return call('answer_decision', { conversationId, answer });
  }

  advise(projectId: Id, advice: NewAdvice): Promise<Conversation> {
    return call('advise', { projectId, advice });
  }

  answerInterview(conversationId: Id, answers: InterviewAnswer[]): Promise<Conversation> {
    return call('answer_interview', { conversationId, answers });
  }

  approveProposal(conversationId: Id, proposal: ApprovedProposal): Promise<Conversation> {
    return call('approve_proposal', { conversationId, proposal });
  }

  listConversationsWaiting(): Promise<Conversation[]> {
    return call('list_conversations_waiting');
  }

  stopWork(conversationId: Id): Promise<Conversation> {
    return call('stop_work', { conversationId });
  }

  openInClaude(conversationId: Id): Promise<void> {
    return call('open_in_claude', { conversationId });
  }

  onConversationChanged(listener: (conversation: Conversation) => void): () => void {
    // The shell sends this event with the whole conversation every time one changes.
    return listenTo(CONVERSATION_CHANGED, listener);
  }

  syncState(): Promise<SyncState> {
    return call('sync_state');
  }

  chooseSyncFolder(folder: string, computerName: string): Promise<SyncState> {
    return call('choose_sync_folder', { folder, computerName });
  }

  stopSyncing(): Promise<SyncState> {
    return call('stop_syncing');
  }

  syncNow(): Promise<SyncState> {
    return call('sync_now');
  }

  renameThisComputer(name: string): Promise<SyncState> {
    return call('rename_this_computer', { name });
  }

  acceptComputer(computerId: Id): Promise<SyncState> {
    return call('accept_computer', { computerId });
  }

  ignoreComputer(computerId: Id): Promise<SyncState> {
    return call('ignore_computer', { computerId });
  }

  syncNotices(): Promise<SyncNotice[]> {
    return call('sync_notices');
  }

  dismissSyncNotice(noticeId: number): Promise<void> {
    return call('dismiss_sync_notice', { noticeId });
  }

  giveThisComputerANewIdentity(): Promise<SyncState> {
    return call('give_this_computer_a_new_identity');
  }

  onSyncChanged(listener: (state: SyncState) => void): () => void {
    // The shell sends the whole state after every round, also one that failed.
    return listenTo(SYNC_CHANGED, listener);
  }

  onDataChanged(listener: (change: DataChanged) => void): () => void {
    // The shell sends the projects in which a round took in changes.
    return listenTo(DATA_CHANGED, listener);
  }
}

/** The name of the event the shell sends when a conversation has changed. */
const CONVERSATION_CHANGED = 'conversation-changed';

/** The name of the event the shell sends with the state of syncing after a round. */
const SYNC_CHANGED = 'sync-changed';

/** The name of the event the shell sends when a round changed projects. */
const DATA_CHANGED = 'data-changed';

/**
 * Calls a listener with the payload of every event of a name the shell
 * sends. Returns the function that stops the calls.
 */
function listenTo<Payload>(event: string, listener: (payload: Payload) => void): () => void {
  const listening = listen<Payload>(event, (received) => {
    listener(received.payload);
  });
  return () => {
    void listening.then((stopListening) => {
      stopListening();
    });
  };
}

/** Calls a command of the shell and turns its failure into a `BackendError`. */
async function call<Result>(command: string, args?: Record<string, unknown>): Promise<Result> {
  try {
    return await invoke<Result>(command, args);
  } catch (failure) {
    throw toBackendError(failure);
  }
}

/** The shape of the error a command reports (`src-tauri/src/error.rs`). */
interface CommandError {
  code: string;
  message: string;
}

function isCommandError(value: unknown): value is CommandError {
  if (typeof value !== 'object' || value === null) {
    return false;
  }
  const candidate = value as Partial<CommandError>;
  return typeof candidate.code === 'string' && typeof candidate.message === 'string';
}

function toBackendError(failure: unknown): BackendError {
  if (isCommandError(failure)) {
    return new BackendError(failure.code, failure.message);
  }
  // The shell itself failed, for example because a command does not exist.
  const detail = typeof failure === 'string' ? failure : 'The request failed unexpectedly.';
  return new BackendError('unexpected', detail);
}
