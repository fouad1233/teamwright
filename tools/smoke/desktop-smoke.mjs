/**
 * Smoke test of the desktop application on Windows.
 *
 * It starts the built application with an empty data folder, drives the real
 * window through the debugging protocol of its web view, and checks that the
 * interface, the commands of the shell and the database work together. Then
 * it starts the application a second time on the same folder to prove that
 * what was stored is still there.
 *
 * Usage:
 *   node tools/smoke/desktop-smoke.mjs <path-to-teamwright-desktop.exe> [screenshot.png]
 *
 * It needs nothing but Node.js 22 or later. The user's real data is never
 * touched: the application is pointed at a temporary folder through
 * TEAMWRIGHT_DATA_DIR.
 */

import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  closeWindowTitled,
  DesktopApplication,
  removeDataDirectory,
  STAND_IN_AGENT_VERSION,
} from '../shared/desktop-application.mjs';

const [executable, screenshotPath] = process.argv.slice(2);
if (!executable) {
  console.error('usage: node desktop-smoke.mjs <path-to-exe> [screenshot.png]');
  process.exit(2);
}

/** The port on which the web view of the application offers its debugging protocol. */
const DEBUGGING_PORT = 9333;

/** The title the test gives the window for choosing a folder, to find it again. */
const FOLDER_DIALOG_TITLE = 'Teamwright smoke test: choose a folder';

// ------------------------------------------------------------------ checks

let failures = 0;

function check(description, passed, detail = '') {
  console.log(
    `${passed ? 'ok  ' : 'FAIL'}  ${description}${passed || !detail ? '' : `  (${detail})`}`,
  );
  if (!passed) {
    failures += 1;
  }
}

const dataDirectory = mkdtempSync(join(tmpdir(), 'teamwright-smoke-'));
let application = DesktopApplication.start(executable, dataDirectory, DEBUGGING_PORT);

try {
  // First run: an empty application, used the way the interface uses it.
  let window = await application.openWindow();
  await window.waitForText('No projects yet');
  check('a new installation shows the first-run screen', true);

  const noProjects = await window.command('list_projects');
  check('it starts without projects', noProjects.ok?.length === 0, JSON.stringify(noProjects));

  const created = await window.command('create_project', {
    input: { name: 'Smart sensor node', workflow: 'engineering' },
  });
  const project = created.ok;
  check('a project is created with a key derived from its name', project?.key === 'SSN');
  check(
    'it has the five statuses of the engineering workflow',
    project?.statuses.map((status) => status.name).join(', ') ===
      'To do, In progress, In revision, In test, Done',
  );

  const task = async (title) =>
    (await window.command('create_task', { input: { projectId: project.id, title } })).ok;
  const board = await task('Design the board');
  const firmware = await task('Write the firmware');
  const enclosure = await task('Design the enclosure');
  check(
    'tasks are numbered from one',
    [board, firmware, enclosure].map((t) => t?.number).join() === '1,2,3',
  );

  await window.command('assign_task', { taskId: board.id, assignee: { kind: 'manager' } });
  await window.command('prioritize_task', { taskId: board.id, priority: 'high' });
  await window.command('reschedule_task', {
    taskId: board.id,
    startOn: '2026-11-12',
    dueOn: '2026-11-20',
  });
  await window.command('add_task_dependency', { taskId: firmware.id, blockerId: board.id });
  await window.command('add_task_dependency', { taskId: enclosure.id, blockerId: firmware.id });

  const cycle = await window.command('add_task_dependency', {
    taskId: board.id,
    blockerId: enclosure.id,
  });
  check(
    'a dependency that would close a loop is refused by name',
    cycle.refused?.code === 'dependency_cycle' &&
      cycle.refused?.message ===
        'SSN-1 cannot wait for SSN-3, because SSN-3 already waits for SSN-1',
    JSON.stringify(cycle),
  );

  const done = project.statuses.find((status) => status.category === 'done');
  const moved = await window.command('move_task', {
    taskId: board.id,
    statusId: done.id,
    index: 0,
  });
  check('moving a task to a done status completes it', typeof moved.ok?.completedAt === 'string');

  const history = (await window.command('list_task_history', { taskId: board.id })).ok;
  check(
    'the history of a task tells what happened to it, the oldest first',
    history?.map((change) => change.kind).join(', ') ===
      'created, assigned, prioritized, rescheduled, moved',
    JSON.stringify(history),
  );
  check(
    'a change in the history says from what to what',
    history?.[1]?.to?.kind === 'manager' &&
      history[2].from === 'normal' &&
      history[2].to === 'high' &&
      history[3].to.dueOn === '2026-11-20' &&
      history[4].fromStatusId === board.statusId &&
      history[4].toStatusId === done.id,
    JSON.stringify(history),
  );

  const badDate = await window.command('reschedule_task', {
    taskId: firmware.id,
    startOn: 'tomorrow',
    dueOn: null,
  });
  check('a date that is not a date is refused', badDate.refused?.code === 'invalid_request');

  // A team: members join, are given tasks, and one of them leaves.
  const engineer = (
    await window.command('add_member', {
      input: {
        projectId: project.id,
        name: 'Hardware engineer',
        kind: 'agent',
        responsibility: 'The parts, the schematic and the board.',
      },
    })
  ).ok;
  const person = (
    await window.command('add_member', {
      input: { projectId: project.id, name: 'Dana', kind: 'human' },
    })
  ).ok;
  check(
    'members join the team, each with a color of its own',
    engineer?.color === 'teal' && person?.color === 'blue' && person?.kind === 'human',
    JSON.stringify([engineer, person]),
  );

  const namesake = await window.command('add_member', {
    input: { projectId: project.id, name: 'hardware ENGINEER', kind: 'agent' },
  });
  check(
    'a second member with the same name is refused',
    namesake.refused?.code === 'member_name_taken',
    JSON.stringify(namesake),
  );

  const profiled = await window.command('set_member_profile', {
    memberId: engineer.id,
    profile: { model: ' sonnet ', effort: 'high' },
  });
  check(
    'a specialist is given a model and an effort',
    engineer?.model === null && profiled.ok?.model === 'sonnet' && profiled.ok.effort === 'high',
    JSON.stringify(profiled),
  );
  const personWithModel = await window.command('set_member_profile', {
    memberId: person.id,
    profile: { model: 'sonnet', effort: null },
  });
  const twoWords = await window.command('set_member_profile', {
    memberId: engineer.id,
    profile: { model: 'the best one', effort: null },
  });
  check(
    'a person gets no model, and the name of a model is one word',
    personWithModel.refused?.code === 'no_specialist' &&
      personWithModel.refused.message === 'Dana is not an AI specialist' &&
      twoWords.refused?.code === 'rule_broken',
    JSON.stringify([personWithModel, twoWords]),
  );

  const given = await window.command('assign_task', {
    taskId: firmware.id,
    assignee: { kind: 'member', memberId: engineer.id },
  });
  check('a task is given to a member of the team', given.ok?.assignee?.memberId === engineer.id);

  const stranger = await window.command('assign_task', {
    taskId: firmware.id,
    assignee: { kind: 'member', memberId: '0196f3a2-7c1e-7d53-9b6f-2f1a4c8e9d10' },
  });
  check(
    'a task is not given to a member that does not exist',
    stranger.refused?.code === 'not_found',
    JSON.stringify(stranger),
  );

  await window.command('assign_task', {
    taskId: enclosure.id,
    assignee: { kind: 'member', memberId: person.id },
  });
  await window.command('remove_member', { memberId: person.id });
  const afterLeaving = (await window.command('list_tasks', { projectId: project.id })).ok;
  check(
    'the tasks of a member who leaves are given to nobody',
    afterLeaving?.find((stored) => stored.id === enclosure.id)?.assignee === null,
  );

  // A subtask: it belongs to its parent, and a place on the board does not count it.
  const part = (
    await window.command('create_task', {
      input: { projectId: project.id, title: 'Choose the material', parentId: enclosure.id },
    })
  ).ok;
  const todo = project.statuses.find((status) => status.category === 'todo');
  await window.command('move_task', { taskId: enclosure.id, statusId: todo.id, index: 0 });
  const arranged = (await window.command('list_tasks', { projectId: project.id })).ok;
  const mainTasksToDo = arranged
    ?.filter((stored) => stored.statusId === todo.id && stored.parentId === null)
    .map((stored) => stored.title);
  check(
    'a subtask is stored with its parent and is not counted among the cards of a status',
    part?.parentId === enclosure.id &&
      mainTasksToDo?.join(', ') === 'Design the enclosure, Write the firmware',
    JSON.stringify(mainTasksToDo),
  );

  // The statuses of the project: one is added, used, changed, and removed.
  const names = (changed) => changed?.statuses.map((status) => status.name).join(', ');
  const withWaiting = await window.command('add_status', {
    projectId: project.id,
    status: { name: 'Waiting for parts', category: 'active', color: 'pink' },
  });
  check(
    'a new step of the work is placed with the other steps, in front of review and done',
    names(withWaiting.ok) === 'To do, In progress, In revision, Waiting for parts, In test, Done',
    JSON.stringify(withWaiting),
  );
  const waiting = withWaiting.ok?.statuses.find((status) => status.name === 'Waiting for parts');
  const inProgress = project.statuses.find((status) => status.category === 'active');
  await window.command('move_task', { taskId: firmware.id, statusId: waiting.id, index: 0 });
  const storedTask = async (id) =>
    (await window.command('list_tasks', { projectId: project.id })).ok?.find(
      (stored) => stored.id === id,
    );

  const sameName = await window.command('add_status', {
    projectId: project.id,
    status: { name: 'in TEST', category: 'review', color: 'amber' },
  });
  check(
    'a second status with the same name is refused',
    sameName.refused?.code === 'rule_broken',
    JSON.stringify(sameName),
  );

  const noTodo = await window.command('remove_status', {
    projectId: project.id,
    statusId: todo.id,
    moveTasksTo: inProgress.id,
  });
  check(
    'the only status in which work has not started cannot be removed',
    noTodo.refused?.code === 'rule_broken',
    JSON.stringify(noTodo),
  );

  const inUse = await window.command('remove_status', {
    projectId: project.id,
    statusId: waiting.id,
    moveTasksTo: null,
  });
  check(
    'a status with tasks is not removed while they have nowhere to go',
    inUse.refused?.code === 'status_in_use' &&
      inUse.refused?.message ===
        'the status "Waiting for parts" still has 1 task, which need another status to go to',
    JSON.stringify(inUse),
  );

  const accepted = await window.command('change_status', {
    projectId: project.id,
    statusId: waiting.id,
    status: { name: 'Accepted', category: 'done', color: 'green' },
  });
  check(
    'a status that becomes a done status completes its tasks',
    names(accepted.ok)?.includes('Accepted') &&
      typeof (await storedTask(firmware.id))?.completedAt === 'string',
    JSON.stringify(accepted),
  );

  const reordered = await window.command('move_status', {
    projectId: project.id,
    statusId: waiting.id,
    index: 1,
  });
  check(
    'a status moves to another place of the workflow',
    names(reordered.ok) === 'To do, Accepted, In progress, In revision, In test, Done',
    JSON.stringify(reordered),
  );

  const removed = await window.command('remove_status', {
    projectId: project.id,
    statusId: waiting.id,
    moveTasksTo: inProgress.id,
  });
  const firmwareAfter = await storedTask(firmware.id);
  check(
    'removing a status sends its tasks to another one, which reopens them',
    names(removed.ok) === 'To do, In progress, In revision, In test, Done' &&
      firmwareAfter?.statusId === inProgress.id &&
      firmwareAfter?.completedAt === null,
    JSON.stringify([removed, firmwareAfter]),
  );

  // A second project is put away, brought back, and deleted for good.
  const spare = (
    await window.command('create_project', {
      input: { name: 'Display driver', workflow: 'standard' },
    })
  ).ok;
  await window.command('create_task', {
    input: { projectId: spare.id, title: 'Write the driver' },
  });
  await window.command('rename_project', {
    projectId: spare.id,
    name: 'Display driver, second try',
  });
  await window.command('archive_project', { projectId: spare.id });
  const openProjects = (await window.command('list_projects')).ok;
  const archived = (await window.command('list_archived_projects')).ok;
  check(
    'a project that is put away is listed with the archived ones only',
    openProjects?.length === 1 &&
      archived?.length === 1 &&
      archived[0].name === 'Display driver, second try',
    JSON.stringify(archived),
  );
  const restored = await window.command('restore_project', { projectId: spare.id });
  check(
    'a project that was put away is brought back',
    restored.ok?.archivedAt === null && (await window.command('list_projects')).ok?.length === 2,
    JSON.stringify(restored),
  );
  const deleted = await window.command('delete_project', { projectId: spare.id });
  const tasksOfDeleted = await window.command('list_tasks', { projectId: spare.id });
  check(
    'a project is deleted for good, with its tasks',
    deleted.refused === undefined &&
      tasksOfDeleted.refused?.code === 'not_found' &&
      (await window.command('list_projects')).ok?.length === 1,
    JSON.stringify([deleted, tasksOfDeleted]),
  );

  // The interface shows what the commands stored.
  await window.reload();
  const shown = await window.waitForText('Write the firmware');
  check('the board shows the project', shown.includes('Smart sensor node'));
  check(
    'the board shows every task',
    ['Design the board', 'Design the enclosure'].every((title) => shown.includes(title)),
  );
  check('the header counts the tasks', shown.includes('4 tasks · 1 done'), shown.slice(0, 200));
  check(
    'the board shows a subtask behind the fold of its parent, not as a card of its own',
    shown.includes('1 subtask') && !shown.includes('Choose the material'),
    shown.slice(0, 400),
  );
  check(
    'the views of a project are offered as tabs',
    ['Board', 'List', 'Gantt', 'Calendar', 'Team', 'Advisor'].every((view) => shown.includes(view)),
  );
  check(
    'the board shows who a task is given to',
    await window.evaluate(
      `document.querySelector('li [role="img"][aria-label="Hardware engineer"]') !== null`,
    ),
  );
  if (screenshotPath) {
    await window.screenshot(screenshotPath);
    console.log(`      screenshot written to ${screenshotPath}`);
  }

  // The window of the operating system in which the manager chooses a
  // folder. Nobody is there to choose, so the test closes that window, and
  // closing it must choose nothing.
  const choosing = window.command('plugin:dialog|open', {
    options: { directory: true, multiple: false, title: FOLDER_DIALOG_TITLE },
  });
  const [wasOpen, chosen] = await Promise.all([closeWindowTitled(FOLDER_DIALOG_TITLE), choosing]);
  check(
    'the interface may open the window for choosing a folder, and closing it chooses nothing',
    wasOpen && 'ok' in chosen && chosen.ok === null,
    JSON.stringify({ wasOpen, chosen }),
  );

  // The folders of a project: where its specialists work, and what else
  // they may read. The folders are made inside the data folder of this run.
  const places = (await window.command('project_places', { projectId: project.id })).ok;
  check(
    'a project has a work folder of its own, named by its key',
    places?.isWorkFolderChosen === false &&
      places.referenceFolders.length === 0 &&
      places.workFolder.replaceAll('\\', '/').toLowerCase().endsWith('/projects/ssn'),
    JSON.stringify(places),
  );
  const chosenFolder = join(dataDirectory, 'chosen work');
  const shelf = join(dataDirectory, 'shelf (data sheets)');
  mkdirSync(chosenFolder);
  mkdirSync(shelf);
  writeFileSync(join(shelf, 'sheet.txt'), 'The supply is 3.3 V.');
  const withChosenFolder = await window.command('set_work_folder', {
    projectId: project.id,
    folder: chosenFolder,
  });
  check(
    'the manager chooses the folder the specialists work in',
    withChosenFolder.ok?.isWorkFolderChosen === true,
    JSON.stringify(withChosenFolder),
  );
  const nowhere = await window.command('set_work_folder', {
    projectId: project.id,
    folder: join(dataDirectory, 'nowhere'),
  });
  check(
    'a work folder that does not exist is refused',
    nowhere.refused?.code === 'folder_not_found',
    JSON.stringify(nowhere),
  );
  const withShelf = await window.command('add_reference_folder', {
    projectId: project.id,
    folder: shelf,
  });
  check(
    'a folder is added that the specialists may read',
    withShelf.ok?.referenceFolders.length === 1,
    JSON.stringify(withShelf),
  );
  const around = await window.command('add_reference_folder', {
    projectId: project.id,
    folder: dataDirectory,
  });
  check(
    'a reference folder that contains the work folder is refused',
    around.refused?.code === 'rule_broken',
    JSON.stringify(around),
  );
  const copied = await window.command('add_reference_files', {
    projectId: project.id,
    files: [join(shelf, 'sheet.txt'), join(shelf, 'sheet.txt')],
  });
  check(
    'single files are copied into the work folder, and a name that is taken gets a number',
    JSON.stringify(copied.ok) === '["sheet.txt","sheet (2).txt"]' &&
      existsSync(join(chosenFolder, 'references', 'sheet (2).txt')) &&
      readFileSync(join(chosenFolder, 'references', 'sheet.txt'), 'utf8') ===
        'The supply is 3.3 V.',
    JSON.stringify(copied),
  );

  // The work of a specialist, as far as it goes without a real one. The
  // application is pointed at a stand-in for Claude Code, which says that it
  // is installed and signed in. The stand-in is a script, and Teamwright
  // does not start a turn on a script, so this is also the way to see what
  // a start leaves behind when the program refuses it: nothing.
  const program = (await window.command('agent_program')).ok;
  check(
    'the program for the specialists is asked for its version and its sign-in',
    program?.state === 'ready' &&
      program.version === STAND_IN_AGENT_VERSION &&
      program.subscription === 'max',
    JSON.stringify(program),
  );
  const notForASpecialist = await window.command('start_work', { taskId: board.id });
  check(
    'work starts only on a task that is given to an AI specialist',
    notForASpecialist.refused?.code === 'no_specialist',
    JSON.stringify(notForASpecialist),
  );
  const statusBefore = (await window.command('list_tasks', { projectId: project.id })).ok?.find(
    (stored) => stored.id === firmware.id,
  )?.statusId;
  const refusedStart = await window.command('start_work', { taskId: firmware.id });
  check(
    'a start that the program refuses says why in words',
    refusedStart.refused?.code === 'agent_could_not_run' &&
      refusedStart.refused.message.includes('installed as a script'),
    JSON.stringify(refusedStart),
  );
  const afterRefusal = (await window.command('list_tasks', { projectId: project.id })).ok;
  check(
    'a start that was refused leaves nothing behind: no conversation, and the task where it was',
    (await window.command('list_conversations', { projectId: project.id })).ok?.length === 0 &&
      afterRefusal?.find((stored) => stored.id === firmware.id)?.statusId === statusBefore,
  );

  // Questions to the manager. Without a real specialist nobody asks, so
  // nothing waits and there is nothing to answer.
  const questions = await window.command('list_conversations_waiting');
  check('nothing waits for the manager', questions.ok?.length === 0, JSON.stringify(questions));
  const answerToNothing = await window.command('answer_decision', {
    conversationId: project.id,
    answer: { option: 0, note: '' },
  });
  check(
    'an answer in a conversation that does not exist is refused',
    answerToNothing.refused?.code === 'not_found',
    JSON.stringify(answerToNothing),
  );

  // The Advisor, as far as it goes without a real program. Planning starts
  // a turn, and the stand-in cannot run one, so a start is refused and
  // leaves nothing behind. There are no questions to answer and nothing to
  // approve.
  const noPlanning = await window.command('advise', {
    projectId: project.id,
    advice: { brief: 'A small device that measures the air.' },
  });
  check(
    'planning that the program refuses says why, and leaves no conversation behind',
    noPlanning.refused?.code === 'agent_could_not_run' &&
      (await window.command('list_conversations', { projectId: project.id })).ok?.length === 0,
    JSON.stringify(noPlanning),
  );
  const emptyBrief = await window.command('advise', {
    projectId: project.id,
    advice: { brief: '   ' },
  });
  check(
    'the Advisor is not started without a word of what is wanted',
    emptyBrief.refused?.code === 'rule_broken',
    JSON.stringify(emptyBrief),
  );
  const answersToNothing = await window.command('answer_interview', {
    conversationId: project.id,
    answers: [{ option: 0, note: '' }],
  });
  const approvalOfNothing = await window.command('approve_proposal', {
    conversationId: project.id,
    proposal: { members: [], tasks: [] },
  });
  check(
    'answers and an approval in a conversation that does not exist are refused',
    answersToNothing.refused?.code === 'not_found' &&
      approvalOfNothing.refused?.code === 'not_found',
    JSON.stringify([answersToNothing, approvalOfNothing]),
  );

  // The views of a project: six to start with, and more that the manager
  // adds, names, filters, orders and removes.
  const viewsOf = async () => (await window.command('list_views', { projectId: project.id })).ok;
  const startingViews = await viewsOf();
  check(
    'a project starts with six views, one of each kind, named after its kind',
    startingViews?.map((view) => `${view.kind}: ${view.name}`).join(', ') ===
      'board: Board, backlog: Backlog, list: List, gantt: Gantt, calendar: Calendar, dashboard: Dashboard',
    JSON.stringify(startingViews),
  );
  const secondBoard = await window.command('add_view', {
    input: { projectId: project.id, kind: 'board' },
  });
  check(
    'a second board gets a number, comes last and shows every task',
    secondBoard.ok?.name === 'Board 2' &&
      secondBoard.ok.filter.assignee.kind === 'anyone' &&
      secondBoard.ok.filter.hidesFinished === false &&
      (await viewsOf())?.at(-1)?.id === secondBoard.ok.id,
    JSON.stringify(secondBoard),
  );
  const secondBacklog = await window.command('add_view', {
    input: { projectId: project.id, kind: 'backlog' },
  });
  const nameTaken = await window.command('rename_view', {
    viewId: secondBoard.ok.id,
    name: 'gantt',
  });
  check(
    'a second backlog and a name that is taken, whatever the capitals, are refused',
    secondBacklog.refused?.code === 'view_kind_taken' &&
      secondBacklog.refused.message ===
        'the project already has a backlog, "Backlog", and can have only one' &&
      nameTaken.refused?.code === 'view_name_taken' &&
      nameTaken.refused.message === 'the project already has a view called "gantt"',
    JSON.stringify([secondBacklog, nameTaken]),
  );
  const renamedView = await window.command('rename_view', {
    viewId: secondBoard.ok.id,
    name: '  Hardware only ',
  });
  const filteredView = await window.command('set_view_filter', {
    viewId: secondBoard.ok.id,
    filter: { assignee: { kind: 'member', memberId: engineer.id }, hidesFinished: true },
  });
  const filteredBacklog = await window.command('set_view_filter', {
    viewId: startingViews[1].id,
    filter: { assignee: { kind: 'manager' }, hidesFinished: false },
  });
  check(
    'a view is renamed and given a filter of its own, and a backlog cannot be filtered',
    renamedView.ok?.name === 'Hardware only' &&
      filteredView.ok?.filter.assignee.memberId === engineer.id &&
      filteredView.ok.filter.hidesFinished === true &&
      filteredBacklog.refused?.code === 'rule_broken' &&
      filteredBacklog.refused.message ===
        '"Backlog" is a backlog, which shows every task and cannot be filtered',
    JSON.stringify([renamedView, filteredView, filteredBacklog]),
  );
  const movedViews = await window.command('move_view', { viewId: secondBoard.ok.id, index: 0 });
  check(
    'a view is moved to the front, and the answer is the whole row in its new order',
    movedViews.ok?.map((view) => view.name).join(', ') ===
      'Hardware only, Board, Backlog, List, Gantt, Calendar, Dashboard',
    JSON.stringify(movedViews),
  );
  await window.command('remove_view', { viewId: startingViews[4].id });
  check(
    'a view is removed',
    (await viewsOf())?.map((view) => view.name).join(', ') ===
      'Hardware only, Board, Backlog, List, Gantt, Dashboard',
  );
  await window.reload();
  await window.waitForText('Hardware only');
  const row = await window.evaluate(
    `[...document.querySelectorAll('[role="tab"]')].map((tab) => tab.innerText.trim()).join(', ')`,
  );
  check(
    'the window shows the views in their order, and then the places',
    row === 'Hardware only, Board, Backlog, List, Gantt, Dashboard, Team, Office, Advisor',
    row,
  );

  // Sprints and the backlog: a sprint is planned, filled, started and
  // completed, and what it must not do is refused by name.
  const tasksOf = async () => (await window.command('list_tasks', { projectId: project.id })).ok;
  const sprintsOf = async () =>
    (await window.command('list_sprints', { projectId: project.id })).ok;
  check(
    'a project starts without sprints: every task is in the backlog, without points',
    (await sprintsOf())?.length === 0 &&
      (await tasksOf())?.every((stored) => stored.sprintId === null && stored.points === null),
  );
  const sprint = (await window.command('create_sprint', { input: { projectId: project.id } })).ok;
  check(
    'a sprint without a name is called by its number, and is planned without dates',
    sprint?.name === 'Sprint 1' &&
      sprint.state === 'planned' &&
      sprint.startOn === null &&
      sprint.endOn === null,
    JSON.stringify(sprint),
  );
  const firstSprint = {
    name: 'First board',
    goal: 'A board that can be ordered',
    startOn: '2026-11-12',
    endOn: '2026-11-25',
  };
  const startedEmpty = await window.command('start_sprint', {
    sprintId: sprint.id,
    input: firstSprint,
  });
  check(
    'a sprint without tasks is not started',
    startedEmpty.refused?.code === 'sprint_empty' &&
      startedEmpty.refused.message ===
        '"Sprint 1" has no tasks; plan some into it before you start it',
    JSON.stringify(startedEmpty),
  );
  const plannedInto = await window.command('plan_task', {
    taskId: enclosure.id,
    sprintId: sprint.id,
  });
  await window.command('plan_task', { taskId: firmware.id, sprintId: sprint.id });
  const subtaskAlone = await window.command('plan_task', { taskId: part.id, sprintId: null });
  check(
    'a task goes into a sprint with its subtasks, and a subtask is not planned by itself',
    plannedInto.ok?.sprintId === sprint.id &&
      (await tasksOf())?.find((stored) => stored.id === part.id)?.sprintId === sprint.id &&
      subtaskAlone.refused?.code === 'subtask_planned_alone' &&
      subtaskAlone.refused.message === 'SSN-4 is a part of SSN-3 and goes where it goes',
    JSON.stringify([plannedInto, subtaskAlone]),
  );
  const withPoints = await window.command('set_task_points', { taskId: firmware.id, points: 5 });
  const withHalfAPoint = await window.command('set_task_points', {
    taskId: firmware.id,
    points: 2.5,
  });
  check(
    'a task is given story points, and half a point is refused',
    withPoints.ok?.points === 5 &&
      withHalfAPoint.refused?.code === 'rule_broken' &&
      withHalfAPoint.refused.message === 'story points are a whole number from 0 to 999',
    JSON.stringify([withPoints, withHalfAPoint]),
  );
  const started = await window.command('start_sprint', { sprintId: sprint.id, input: firstSprint });
  check(
    'a sprint starts with its name, its goal and its dates',
    started.ok?.state === 'active' &&
      started.ok.name === 'First board' &&
      started.ok.goal === 'A board that can be ordered' &&
      started.ok.startOn === '2026-11-12' &&
      started.ok.endOn === '2026-11-25' &&
      started.ok.startedAt !== null,
    JSON.stringify(started),
  );
  const next = (await window.command('create_sprint', { input: { projectId: project.id } })).ok;
  const startedBeside = await window.command('start_sprint', {
    sprintId: next.id,
    input: { name: 'Sprint 2', goal: '', startOn: '2026-11-26', endOn: '2026-12-09' },
  });
  const deletedWhileRunning = await window.command('delete_sprint', { sprintId: sprint.id });
  check(
    'one sprint runs at a time, and a running sprint is not deleted',
    next?.name === 'Sprint 2' &&
      startedBeside.refused?.code === 'another_sprint_active' &&
      startedBeside.refused.message ===
        '"First board" is still running; complete it before you start another sprint' &&
      deletedWhileRunning.refused?.code === 'sprint_active',
    JSON.stringify([startedBeside, deletedWhileRunning]),
  );
  check(
    'the sprint that runs is listed first',
    (await sprintsOf())?.map((stored) => stored.name).join(', ') === 'First board, Sprint 2',
  );
  await window.command('move_task', { taskId: firmware.id, statusId: done.id, index: 0 });
  const course = (await window.command('sprint_report', { sprintId: sprint.id })).ok;
  check(
    'the report of a sprint has a day for each of its dates and counts in points',
    course?.unit === 'points' && course.total === 5 && course.days.length === 14,
    JSON.stringify(course)?.slice(0, 300),
  );
  const completed = await window.command('complete_sprint', {
    sprintId: sprint.id,
    input: { moveUnfinishedTo: next.id },
  });
  const afterSprint = await tasksOf();
  check(
    'a sprint is completed with what it took on and what it finished',
    completed.ok?.state === 'completed' &&
      completed.ok.completedAt !== null &&
      completed.ok.summary?.unit === 'points' &&
      completed.ok.summary.committed === 5 &&
      completed.ok.summary.completed === 5,
    JSON.stringify(completed),
  );
  check(
    'its finished task stays in it, and the unfinished one moves on with its subtask',
    afterSprint?.find((stored) => stored.id === firmware.id)?.sprintId === sprint.id &&
      afterSprint.find((stored) => stored.id === enclosure.id)?.sprintId === next.id &&
      afterSprint.find((stored) => stored.id === part.id)?.sprintId === next.id,
    JSON.stringify(afterSprint?.map((stored) => [stored.number, stored.sprintId])),
  );
  const deletedRecord = await window.command('delete_sprint', { sprintId: sprint.id });
  const plannedIntoRecord = await window.command('plan_task', {
    taskId: board.id,
    sprintId: sprint.id,
  });
  check(
    'a completed sprint is kept as a record and takes no more tasks',
    deletedRecord.refused?.code === 'sprint_completed' &&
      deletedRecord.refused.message === '"First board" is completed and is kept as a record' &&
      plannedIntoRecord.refused?.code === 'sprint_completed',
    JSON.stringify([deletedRecord, plannedIntoRecord]),
  );
  await window.reload();
  await window.waitForText('Smart sensor node');
  await window.click(
    `[...document.querySelectorAll('[role="tab"]')].find((entry) => entry.innerText.trim() === 'Backlog')`,
  );
  const backlogShown = await window.waitForText('COMPLETED SPRINTS');
  check(
    'the Backlog view shows the planned sprint, its task and the completed sprint',
    backlogShown.includes('Sprint 2') &&
      backlogShown.includes('First board') &&
      backlogShown.includes('Design the enclosure'),
  );
  window.close();

  // Second run: what was stored is still there.
  await application.stop();
  application = DesktopApplication.start(executable, dataDirectory, DEBUGGING_PORT);
  window = await application.openWindow();
  await window.waitForText('Smart sensor node');
  const projects = (await window.command('list_projects')).ok;
  const tasks = (await window.command('list_tasks', { projectId: projects[0]?.id })).ok;
  const storedBoard = tasks?.find((stored) => stored.number === 1);
  check('the project survives a restart', projects?.length === 1 && projects[0].key === 'SSN');
  check('the tasks survive a restart', tasks?.length === 4);
  check(
    'the workflow is as it was left, and the deleted project stays deleted',
    projects[0].statuses.map((status) => status.name).join(', ') ===
      'To do, In progress, In revision, In test, Done' &&
      (await window.command('list_archived_projects')).ok?.length === 0,
    JSON.stringify(projects[0].statuses),
  );
  const team = (await window.command('list_members', { projectId: projects[0]?.id })).ok;
  check(
    'the team survives a restart',
    team?.length === 1 &&
      team[0].name === 'Hardware engineer' &&
      team[0].color === 'teal' &&
      team[0].model === 'sonnet' &&
      team[0].effort === 'high',
    JSON.stringify(team),
  );
  const storedPlaces = (await window.command('project_places', { projectId: projects[0]?.id })).ok;
  check(
    'the folders of the project survive a restart',
    storedPlaces?.isWorkFolderChosen === true && storedPlaces.referenceFolders.length === 1,
    JSON.stringify(storedPlaces),
  );
  check(
    'a task is still given to the same member',
    tasks?.find((stored) => stored.number === 2)?.assignee?.memberId === team?.[0]?.id,
  );
  check(
    'a task keeps everything that was set on it',
    storedBoard?.assignee?.kind === 'manager' &&
      storedBoard.priority === 'high' &&
      storedBoard.startOn === '2026-11-12' &&
      storedBoard.dueOn === '2026-11-20' &&
      storedBoard.completedAt !== null,
    JSON.stringify(storedBoard),
  );
  check(
    'dependencies survive a restart',
    tasks?.find((stored) => stored.number === 2)?.blockedBy.length === 1,
  );
  const storedViews = (await window.command('list_views', { projectId: projects[0]?.id })).ok;
  check(
    'the views survive a restart, in their order and with the filter of the one that has one',
    storedViews?.map((view) => view.name).join(', ') ===
      'Hardware only, Board, Backlog, List, Gantt, Dashboard' &&
      storedViews[0].filter.assignee.memberId === team?.[0]?.id &&
      storedViews[0].filter.hidesFinished === true,
    JSON.stringify(storedViews),
  );
  const storedSprints = (await window.command('list_sprints', { projectId: projects[0]?.id })).ok;
  const record = storedSprints?.find((stored) => stored.state === 'completed');
  check(
    'the sprints survive a restart, the planned one before the completed one',
    storedSprints?.map((stored) => `${stored.name}: ${stored.state}`).join(', ') ===
      'Sprint 2: planned, First board: completed' &&
      record?.summary?.committed === 5 &&
      record.summary.completed === 5,
    JSON.stringify(storedSprints),
  );
  check(
    'the tasks are still in their sprints, with their points',
    tasks?.find((stored) => stored.number === 2)?.sprintId === record?.id &&
      tasks.find((stored) => stored.number === 2)?.points === 5 &&
      tasks.find((stored) => stored.number === 3)?.sprintId === storedSprints?.[0]?.id &&
      tasks.find((stored) => stored.number === 1)?.sprintId === null,
    JSON.stringify(tasks?.map((stored) => [stored.number, stored.sprintId, stored.points])),
  );
  window.close();
} catch (failure) {
  check('the smoke test ran to its end', false, failure.message);
} finally {
  await application.stop();
  removeDataDirectory(dataDirectory);
}

console.log(failures === 0 ? '\nAll checks passed.' : `\n${failures} check(s) failed.`);
process.exit(failures === 0 ? 0 : 1);
