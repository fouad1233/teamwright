import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open, save } from '@tauri-apps/plugin-dialog';

import { BackendError, type Backend } from './backend';
import type { KitChoice, Library, NewConnector } from './types';
import type {
  AgentProgram,
  ApprovedProposal,
  Area,
  Assignee,
  BudgetQuestion,
  BudgetScope,
  CalendarDate,
  Color,
  CommandPermissions,
  Conversation,
  ConversationsOfFolder,
  Coordinator,
  CoordinatorChanged,
  CustomField,
  FieldKind,
  FieldValue,
  FolderListing,
  DataChanged,
  Deliveries,
  Delivery,
  Digest,
  DigestDraft,
  EventKind,
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
  PlanLimit,
  Priority,
  Project,
  ProjectChange,
  ProjectIntegrations,
  ProjectPlaces,
  ProjectReport,
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
  TaskActual,
  TaskChange,
  TaskOnGitHub,
  TeamTalk,
  MemoryEntry,
  NewMemoryFact,
  NewPermissionAnswer,
  PermissionRequest,
  ThemeName,
  UpdateInstall,
  UpdateStatus,
  View,
  ViewFilter,
  Grouping,
  TaskGroup,
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

  setStatusLimit(projectId: Id, statusId: Id, limit: number | null): Promise<Project> {
    return call('set_status_limit', { projectId, statusId, limit });
  }

  listStatusLoads(projectId: Id): Promise<StatusLoad[]> {
    return call('list_status_loads', { projectId });
  }

  listProjectTemplates(): Promise<ProjectTemplate[]> {
    return call('list_project_templates');
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

  listTaskActuals(projectId: Id): Promise<TaskActual[]> {
    return call('list_task_actuals', { projectId });
  }

  listProjectHistory(projectId: Id, limit: number, before?: Moment): Promise<ProjectChange[]> {
    return call('list_project_history', { projectId, limit, before: before ?? null });
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

  sprintForecast(projectId: Id): Promise<SprintForecast> {
    return call('sprint_forecast', { projectId });
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

  setViewGrouping(viewId: Id, grouping: Grouping): Promise<View> {
    return call('set_view_grouping', { viewId, grouping });
  }

  listTaskGroups(viewId: Id): Promise<TaskGroup[]> {
    return call('list_task_groups', { viewId });
  }

  setViewContent(viewId: Id, content: string | null): Promise<View> {
    return call('set_view_content', { viewId, content });
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

  listTags(projectId: Id): Promise<Tag[]> {
    return call('list_tags', { projectId });
  }

  addTag(projectId: Id, name: string, color?: Color): Promise<Tag> {
    return call('add_tag', { projectId, name, color: color ?? null });
  }

  renameTag(tagId: Id, name: string): Promise<Tag> {
    return call('rename_tag', { tagId, name });
  }

  recolorTag(tagId: Id, color: Color): Promise<Tag> {
    return call('recolor_tag', { tagId, color });
  }

  removeTag(tagId: Id): Promise<void> {
    return call('remove_tag', { tagId });
  }

  putTagOnTask(taskId: Id, tagId: Id): Promise<Task> {
    return call('put_tag_on_task', { taskId, tagId });
  }

  putNamedTagOnTask(taskId: Id, name: string): Promise<TaggedTask> {
    return call('put_named_tag_on_task', { taskId, name });
  }

  takeTagOffTask(taskId: Id, tagId: Id): Promise<Task> {
    return call('take_tag_off_task', { taskId, tagId });
  }

  addChecklistItem(taskId: Id, text: string): Promise<Task> {
    return call('add_checklist_item', { taskId, text });
  }

  renameChecklistItem(taskId: Id, itemId: Id, text: string): Promise<Task> {
    return call('rename_checklist_item', { taskId, itemId, text });
  }

  tickChecklistItem(taskId: Id, itemId: Id, done: boolean): Promise<Task> {
    return call('tick_checklist_item', { taskId, itemId, done });
  }

  moveChecklistItem(taskId: Id, itemId: Id, index: number): Promise<Task> {
    return call('move_checklist_item', { taskId, itemId, index });
  }

  listFields(projectId: Id): Promise<CustomField[]> {
    return call('list_fields', { projectId });
  }

  addField(
    projectId: Id,
    name: string,
    kind: FieldKind,
    unit?: string | null,
  ): Promise<CustomField> {
    return call('add_field', { projectId, name, kind, unit: unit ?? null });
  }

  renameField(fieldId: Id, name: string): Promise<CustomField> {
    return call('rename_field', { fieldId, name });
  }

  setFieldUnit(fieldId: Id, unit: string | null): Promise<CustomField> {
    return call('set_field_unit', { fieldId, unit });
  }

  moveField(fieldId: Id, index: number): Promise<CustomField[]> {
    return call('move_field', { fieldId, index });
  }

  removeField(fieldId: Id): Promise<void> {
    return call('remove_field', { fieldId });
  }

  addFieldChoice(fieldId: Id, name: string, color?: Color): Promise<CustomField> {
    return call('add_field_choice', { fieldId, name, color: color ?? null });
  }

  renameFieldChoice(fieldId: Id, choiceId: Id, name: string): Promise<CustomField> {
    return call('rename_field_choice', { fieldId, choiceId, name });
  }

  recolorFieldChoice(fieldId: Id, choiceId: Id, color: Color): Promise<CustomField> {
    return call('recolor_field_choice', { fieldId, choiceId, color });
  }

  removeFieldChoice(fieldId: Id, choiceId: Id): Promise<CustomField> {
    return call('remove_field_choice', { fieldId, choiceId });
  }

  setFieldValue(taskId: Id, fieldId: Id, value: FieldValue): Promise<Task> {
    return call('set_field_value', { taskId, fieldId, value });
  }

  clearFieldValue(taskId: Id, fieldId: Id): Promise<Task> {
    return call('clear_field_value', { taskId, fieldId });
  }

  removeChecklistItem(taskId: Id, itemId: Id): Promise<Task> {
    return call('remove_checklist_item', { taskId, itemId });
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

  addWrittenFolder(projectId: Id, folder: string): Promise<ProjectPlaces> {
    return call('add_written_folder', { projectId, folder });
  }

  removeWrittenFolder(projectId: Id, folder: string): Promise<ProjectPlaces> {
    return call('remove_written_folder', { projectId, folder });
  }

  setFullAccess(projectId: Id, on: boolean): Promise<ProjectPlaces> {
    return call('set_full_access', { projectId, on });
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

  listProjectFiles(
    projectId: Id,
    folder: string,
    path: string,
    showsHidden: boolean,
  ): Promise<FolderListing> {
    return call('list_project_files', { projectId, folder, path, showsHidden });
  }

  openProjectFile(projectId: Id, folder: string, path: string): Promise<void> {
    return call('open_project_file', { projectId, folder, path });
  }

  showProjectFile(projectId: Id, folder: string, path: string): Promise<void> {
    return call('show_project_file', { projectId, folder, path });
  }

  async chooseWorkbookFile(suggestedName: string): Promise<string | null> {
    // The window of the operating system for saving a file. It answers
    // with nothing when the manager closes it without choosing.
    return save({
      title: 'Export to Excel',
      defaultPath: suggestedName,
      filters: [{ name: 'Excel workbook', extensions: ['xlsx'] }],
    });
  }

  exportWorkbook(projectId: Id, path: string): Promise<string> {
    return call('export_workbook', { projectId, path });
  }

  showExportedWorkbook(path: string): Promise<void> {
    return call('show_exported_workbook', { path });
  }

  projectReport(projectId: Id): Promise<ProjectReport> {
    return call('project_report', { projectId });
  }

  async chooseReportFile(projectId: Id): Promise<string | null> {
    // The save window opens in the work folder of the project, so that the
    // report stays with the work, unless the manager chooses elsewhere.
    const suggested = await call<string>('suggested_report_path', { projectId });
    return save({
      title: 'Save the report as Markdown',
      defaultPath: suggested,
      filters: [{ name: 'Markdown', extensions: ['md'] }],
    });
  }

  saveProjectReport(projectId: Id, path: string): Promise<string> {
    return call('save_project_report', { projectId, path });
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

  setMemberInstructions(memberId: Id, instructions: string): Promise<Member> {
    return call('set_member_instructions', { memberId, instructions });
  }

  listReadyMadeSpecialists(): Promise<ReadyMadeSpecialist[]> {
    return call('list_ready_made_specialists');
  }

  addReadyMadeMember(projectId: Id, key: string): Promise<Member> {
    return call('add_ready_made_member', { projectId, key });
  }

  projectLibrary(projectId: Id): Promise<Library> {
    return call('project_library', { projectId });
  }

  async chooseSkillFolder(): Promise<string | null> {
    return open({ directory: true, multiple: false, title: 'Choose the folder of a skill' });
  }

  addSkill(projectId: Id, folder: string): Promise<Library> {
    return call('add_skill', { projectId, folder });
  }

  removeSkill(projectId: Id, skillId: Id): Promise<Library> {
    return call('remove_skill', { projectId, skillId });
  }

  addConnector(projectId: Id, connector: NewConnector): Promise<Library> {
    return call('add_connector', { projectId, input: connector });
  }

  removeConnector(projectId: Id, connectorId: Id): Promise<Library> {
    return call('remove_connector', { projectId, connectorId });
  }

  setMemberKit(projectId: Id, memberId: Id, kit: KitChoice): Promise<Library> {
    return call('set_member_kit', { projectId, memberId, kit });
  }

  confirmConnector(projectId: Id, connectorId: Id, fingerprint: string): Promise<Library> {
    return call('confirm_connector', { projectId, connectorId, fingerprint });
  }

  confirmMemberKit(projectId: Id, memberId: Id, kit: KitChoice): Promise<Library> {
    return call('confirm_member_kit', { projectId, memberId, kit });
  }

  agentProgram(): Promise<AgentProgram> {
    return call('agent_program');
  }

  planLimit(): Promise<PlanLimit> {
    return call('plan_limit');
  }

  onPlanLimitChanged(listener: (planLimit: PlanLimit) => void): () => void {
    // The shell sends all that is known after every change.
    return listenTo(PLAN_LIMIT_CHANGED, listener);
  }

  listConversations(projectId: Id): Promise<Conversation[]> {
    return call('list_conversations', { projectId });
  }

  projectUsage(projectId: Id): Promise<ProjectUsage> {
    return call('project_usage', { projectId });
  }

  teamTalk(projectId: Id): Promise<TeamTalk> {
    return call('team_talk', { projectId });
  }

  projectMemory(projectId: Id): Promise<MemoryEntry[]> {
    return call('project_memory', { projectId });
  }

  searchMemory(projectId: Id, query: string): Promise<MemoryEntry[]> {
    return call('search_memory', { projectId, query });
  }

  addMemoryFact(fact: NewMemoryFact): Promise<MemoryEntry> {
    return call('add_memory_fact', { input: fact });
  }

  changeMemoryEntry(entryId: Id, title: string, text: string): Promise<MemoryEntry> {
    return call('change_memory_entry', { entryId, title, text });
  }

  removeMemoryEntry(entryId: Id): Promise<void> {
    return call('remove_memory_entry', { entryId });
  }

  permissionRequests(projectId: Id): Promise<PermissionRequest[]> {
    return call('permission_requests', { projectId });
  }

  openPermissionRequests(): Promise<PermissionRequest[]> {
    return call('open_permission_requests');
  }

  answerPermission(requestId: Id, answer: NewPermissionAnswer): Promise<Conversation> {
    return call('answer_permission', { requestId, answer });
  }

  withdrawPermission(requestId: Id): Promise<PermissionRequest> {
    return call('withdraw_permission', { requestId });
  }

  commandPermissions(): Promise<CommandPermissions> {
    return call('command_permissions');
  }

  switchCommandPermissions(on: boolean): Promise<CommandPermissions> {
    return call('switch_command_permissions', { on });
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

  changeProposal(conversationId: Id, change: ProposalChange): Promise<Conversation> {
    return call('change_proposal', { conversationId, change });
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

  googleDrive(): Promise<GoogleDrive> {
    return call('google_drive');
  }

  prepareGoogleDriveFolder(folder: string): Promise<string> {
    return call('prepare_google_drive_folder', { folder });
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

  theme(): Promise<ThemeName> {
    return call('theme');
  }

  chooseTheme(name: ThemeName): Promise<ThemeName> {
    return call('choose_theme', { name });
  }

  updateStatus(): Promise<UpdateStatus> {
    return call('update_status');
  }

  lookForUpdateIfDue(): Promise<UpdateStatus> {
    return call('look_for_update_if_due');
  }

  lookForUpdateNow(): Promise<UpdateStatus> {
    return call('look_for_update_now');
  }

  switchUpdateLook(on: boolean): Promise<UpdateStatus> {
    return call('switch_update_look', { on });
  }

  updateInstall(): Promise<UpdateInstall> {
    return call('update_install');
  }

  installUpdate(): Promise<UpdateInstall> {
    return call('install_update');
  }

  restartForUpdate(evenWhileWorking: boolean): Promise<UpdateInstall> {
    return call('restart_for_update', { evenWhileWorking });
  }

  notificationChoice(): Promise<NotificationChoice> {
    return call('notification_choice');
  }

  allowNotifications(): Promise<NotificationChoice> {
    return call('allow_notifications');
  }

  declineNotifications(): Promise<NotificationChoice> {
    return call('decline_notifications');
  }

  setNotificationKind(kind: NotificationKind, on: boolean): Promise<NotificationChoice> {
    return call('set_notification_kind', { kind, on });
  }

  sendTestNotification(): Promise<void> {
    return call('send_test_notification');
  }

  onNotificationPressed(listener: (pressed: NotificationPressed) => void): () => void {
    // The shell sends this after it brought the window to the front.
    return listenTo(NOTIFICATION_PRESSED, listener);
  }

  openWebAddress(address: string): Promise<void> {
    return call('open_web_address', { address });
  }

  projectIntegrations(projectId: Id): Promise<ProjectIntegrations> {
    return call('project_integrations', { projectId });
  }

  setRepository(projectId: Id, name: string | null): Promise<ProjectIntegrations> {
    return call('set_repository', { projectId, name });
  }

  setSlackAddress(projectId: Id, address: string | null): Promise<ProjectIntegrations> {
    return call('set_slack_address', { projectId, address });
  }

  setSlackEvent(projectId: Id, kind: EventKind, on: boolean): Promise<ProjectIntegrations> {
    return call('set_slack_event', { projectId, kind, on });
  }

  addWebAddress(
    projectId: Id,
    address: string,
    secret: string | null,
  ): Promise<ProjectIntegrations> {
    return call('add_web_address', { projectId, address, secret });
  }

  removeWebAddress(projectId: Id, index: number): Promise<ProjectIntegrations> {
    return call('remove_web_address', { projectId, index });
  }

  setWebAddressEvent(
    projectId: Id,
    index: number,
    kind: EventKind,
    on: boolean,
  ): Promise<ProjectIntegrations> {
    return call('set_web_address_event', { projectId, index, kind, on });
  }

  gitHubSignIn(): Promise<GitHubSignIn> {
    return call('github_sign_in');
  }

  setGitHubProgram(program: string | null): Promise<GitHubSignIn> {
    return call('set_github_program', { program });
  }

  gitHubOverview(projectId: Id): Promise<GitHubOverview> {
    return call('github_overview', { projectId });
  }

  refreshGitHub(projectId: Id): Promise<GitHubOverview> {
    return call('refresh_github', { projectId });
  }

  taskOnGitHub(taskId: Id, refresh: boolean): Promise<TaskOnGitHub> {
    return call('task_on_github', { taskId, refresh });
  }

  issueDraft(taskId: Id): Promise<IssueDraft> {
    return call('issue_draft', { taskId });
  }

  openIssue(taskId: Id): Promise<GitHubItem> {
    return call('open_issue', { taskId });
  }

  integrationDeliveries(projectId: Id): Promise<Deliveries> {
    return call('integration_deliveries', { projectId });
  }

  testChannel(projectId: Id): Promise<Delivery> {
    return call('test_channel', { projectId });
  }

  testWebAddress(projectId: Id, index: number): Promise<Delivery> {
    return call('test_web_address', { projectId, index });
  }

  findEarlierConversations(projectId: Id): Promise<ConversationsOfFolder[]> {
    return call('find_earlier_conversations', { projectId });
  }

  draftDigest(projectId: Id, source: string): Promise<DigestDraft> {
    return call('draft_digest', { projectId, source });
  }

  keepDigest(projectId: Id, digest: DigestDraft): Promise<Digest> {
    return call('keep_digest', { projectId, digest });
  }

  listDigests(projectId: Id): Promise<Digest[]> {
    return call('list_digests', { projectId });
  }

  removeDigest(digestId: Id): Promise<void> {
    return call('remove_digest', { digestId });
  }

  coordinator(projectId: Id): Promise<Coordinator> {
    return call('coordinator', { projectId });
  }

  switchCoordinator(projectId: Id, on: boolean): Promise<Coordinator> {
    return call('switch_coordinator', { projectId, on });
  }

  setCoordinatorLimit(projectId: Id, limit: number): Promise<Coordinator> {
    return call('set_coordinator_limit', { projectId, limit });
  }

  setBudget(projectId: Id, scope: BudgetScope, tokens: number | null): Promise<Coordinator> {
    return call('set_budget', { projectId, scope, tokens });
  }

  answerBudget(projectId: Id, scope: BudgetScope, raiseBy: number | null): Promise<Coordinator> {
    return call('answer_budget', { projectId, scope, raiseBy });
  }

  listBudgetQuestions(): Promise<BudgetQuestion[]> {
    return call('budget_questions');
  }

  onCoordinatorChanged(listener: (change: CoordinatorChanged) => void): () => void {
    // The shell sends the project after its coordinator acted.
    return listenTo(COORDINATOR_CHANGED, listener);
  }
}

/** The name of the event the shell sends after the coordinator of a project acted. */
const COORDINATOR_CHANGED = 'coordinator-changed';

/** The name of the event the shell sends when the manager pressed a notification. */
const NOTIFICATION_PRESSED = 'notification-pressed';

/** The name of the event the shell sends when a conversation has changed. */
const CONVERSATION_CHANGED = 'conversation-changed';

/** The name of the event the shell sends with the state of syncing after a round. */
const SYNC_CHANGED = 'sync-changed';

/** The name of the event the shell sends when a round changed projects. */
const DATA_CHANGED = 'data-changed';

/** The name of the event the shell sends when what is known about the plan changed. */
const PLAN_LIMIT_CHANGED = 'plan-limit-changed';

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
