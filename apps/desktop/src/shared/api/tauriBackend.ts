import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import { BackendError, type Backend } from './backend';
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
  StatusProperties,
  Task,
  TaskChange,
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
    const listening = listen<Conversation>(CONVERSATION_CHANGED, (event) => {
      listener(event.payload);
    });
    return () => {
      void listening.then((stopListening) => {
        stopListening();
      });
    };
  }
}

/** The name of the event the shell sends when a conversation has changed. */
const CONVERSATION_CHANGED = 'conversation-changed';

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
