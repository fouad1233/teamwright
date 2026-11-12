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

import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
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

// ------------------------------------------------------------------ checks

let failures = 0;

function check(description, passed, detail = '') {
  console.log(`${passed ? 'ok  ' : 'FAIL'}  ${description}${passed || !detail ? '' : `  (${detail})`}`);
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
  check('tasks are numbered from one', [board, firmware, enclosure].map((t) => t?.number).join() === '1,2,3');

  await window.command('assign_task', { taskId: board.id, assignee: { kind: 'manager' } });
  await window.command('prioritize_task', { taskId: board.id, priority: 'high' });
  await window.command('reschedule_task', { taskId: board.id, startOn: '2026-11-12', dueOn: '2026-11-20' });
  await window.command('add_task_dependency', { taskId: firmware.id, blockerId: board.id });
  await window.command('add_task_dependency', { taskId: enclosure.id, blockerId: firmware.id });

  const cycle = await window.command('add_task_dependency', {
    taskId: board.id,
    blockerId: enclosure.id,
  });
  check(
    'a dependency that would close a loop is refused by name',
    cycle.refused?.code === 'dependency_cycle' &&
      cycle.refused?.message === 'SSN-1 cannot wait for SSN-3, because SSN-3 already waits for SSN-1',
    JSON.stringify(cycle),
  );

  const done = project.statuses.find((status) => status.category === 'done');
  const moved = await window.command('move_task', { taskId: board.id, statusId: done.id, index: 0 });
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
    part?.parentId === enclosure.id && mainTasksToDo?.join(', ') === 'Design the enclosure, Write the firmware',
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
    (await window.command('list_tasks', { projectId: project.id })).ok?.find((stored) => stored.id === id);

  const sameName = await window.command('add_status', {
    projectId: project.id,
    status: { name: 'in TEST', category: 'review', color: 'amber' },
  });
  check('a second status with the same name is refused', sameName.refused?.code === 'rule_broken', JSON.stringify(sameName));

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
    names(accepted.ok)?.includes('Accepted') && typeof (await storedTask(firmware.id))?.completedAt === 'string',
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
    await window.command('create_project', { input: { name: 'Display driver', workflow: 'standard' } })
  ).ok;
  await window.command('create_task', { input: { projectId: spare.id, title: 'Write the driver' } });
  await window.command('rename_project', { projectId: spare.id, name: 'Display driver, second try' });
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
  check('the board shows every task', ['Design the board', 'Design the enclosure'].every((title) => shown.includes(title)));
  check('the header counts the tasks', shown.includes('4 tasks · 1 done'), shown.slice(0, 200));
  check(
    'the board shows a subtask behind the fold of its parent, not as a card of its own',
    shown.includes('1 subtask') && !shown.includes('Choose the material'),
    shown.slice(0, 400),
  );
  check(
    'the views of a project are offered as tabs',
    ['Board', 'List', 'Gantt', 'Calendar', 'Team'].every((view) => shown.includes(view)),
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
  window.close();
} catch (failure) {
  check('the smoke test ran to its end', false, failure.message);
} finally {
  await application.stop();
  removeDataDirectory(dataDirectory);
}

console.log(failures === 0 ? '\nAll checks passed.' : `\n${failures} check(s) failed.`);
process.exit(failures === 0 ? 0 : 1);
