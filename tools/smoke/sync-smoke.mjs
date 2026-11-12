// Checks that two copies of the built application keep each other in step
// through one folder, the way two computers of one manager would.
//
// Usage:
//   node tools/smoke/sync-smoke.mjs <path-to-teamwright-desktop.exe>
//
// It starts the application twice, each on a temporary data folder of its
// own, and gives both the same temporary folder to sync through. One copy
// is the computer "Desk", the other "Laptop". Then it works on both the way
// a manager would and looks whether each sees what the other did: a project
// that is taken in, changes in both directions, two changes to one task
// that must both survive, and a deletion that wins over a change.
//
// It needs nothing but Node.js 22 or later, and it does not use Claude Code:
// both copies are pointed at the stand-in. The user's real data and real
// sync folder are never touched.
//
// What it cannot show: two real computers, a real cloud drive between them
// with its delays and half-copied files, and conversations with specialists,
// which need the real Claude Code.
import { mkdirSync, mkdtempSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';

import { DesktopApplication, removeDataDirectory } from '../shared/desktop-application.mjs';

const [executable] = process.argv.slice(2);
if (!executable) {
  console.error('usage: node sync-smoke.mjs <path-to-exe>');
  process.exit(2);
}

const DESK_PORT = 9351;
const LAPTOP_PORT = 9352;

/** How long a change may take to arrive when nobody asks for a round. */
const BACKGROUND_TIMEOUT_MILLISECONDS = 90_000;

let failures = 0;
function check(description, passed, detail = '') {
  if (passed) {
    console.log(`ok    ${description}`);
  } else {
    failures += 1;
    console.log(`FAIL  ${description}${detail ? `  (${detail})` : ''}`);
  }
}

const deskData = mkdtempSync(join(tmpdir(), 'teamwright-sync-desk-'));
const laptopData = mkdtempSync(join(tmpdir(), 'teamwright-sync-laptop-'));
const shared = mkdtempSync(join(tmpdir(), 'teamwright-sync-folder-'));
const folder = join(shared, 'Teamwright sync');
mkdirSync(folder);

let desk = DesktopApplication.start(executable, deskData, DESK_PORT);
const laptop = DesktopApplication.start(executable, laptopData, LAPTOP_PORT);

/** Calls a command that is expected to work, and returns its answer. */
function commandsOf(window) {
  return async (name, args) => {
    const answer = await window.command(name, args);
    if (answer.refused) {
      throw new Error(`${name} was refused: ${JSON.stringify(answer.refused)}`);
    }
    return answer.ok;
  };
}

try {
  let deskWindow = await desk.openWindow();
  const laptopWindow = await laptop.openWindow();
  await deskWindow.waitForText('No projects yet');
  await laptopWindow.waitForText('No projects yet');
  let onDesk = commandsOf(deskWindow);
  const onLaptop = commandsOf(laptopWindow);

  // The desk has a project before syncing is ever set up.
  const project = await onDesk('create_project', {
    input: { name: 'Smart sensor node', workflow: 'engineering' },
  });
  const engineer = await onDesk('add_member', {
    input: {
      projectId: project.id,
      name: 'Hardware engineer',
      kind: 'agent',
      responsibility: 'The board.',
    },
  });
  const board = await onDesk('create_task', {
    input: { projectId: project.id, title: 'Draw the schematic' },
  });
  const firmware = await onDesk('create_task', {
    input: { projectId: project.id, title: 'Write the firmware' },
  });
  await onDesk('assign_task', {
    taskId: board.id,
    assignee: { kind: 'member', memberId: engineer.id },
  });
  const sprint = await onDesk('create_sprint', { input: { projectId: project.id } });
  await onDesk('plan_task', { taskId: board.id, sprintId: sprint.id });

  // Before a folder is chosen.
  const before = await onDesk('sync_state');
  check(
    'a new installation does not sync, and knows which computer it is',
    before.phase === 'off' && before.folder === null && before.thisComputer.length > 0,
    JSON.stringify(before),
  );
  const roundWhileOff = await deskWindow.command('sync_now');
  const missingFolder = await deskWindow.command('choose_sync_folder', {
    folder: join(shared, 'not there'),
    computerName: 'Desk',
  });
  const insideData = await deskWindow.command('choose_sync_folder', {
    folder: deskData,
    computerName: 'Desk',
  });
  const withoutName = await deskWindow.command('choose_sync_folder', {
    folder,
    computerName: '  ',
  });
  check(
    'a round without a folder, a folder that is not there, one inside the data, and a computer without a name are refused',
    roundWhileOff.refused?.code === 'sync_off' &&
      missingFolder.refused?.code === 'folder_not_found' &&
      insideData.refused?.code === 'sync_folder_in_data' &&
      withoutName.refused?.code === 'rule_broken' &&
      withoutName.refused.message === 'the name of the computer must not be empty',
    JSON.stringify([roundWhileOff, missingFolder, insideData, withoutName]),
  );

  // The desk chooses the folder.
  const deskState = await onDesk('choose_sync_folder', { folder, computerName: 'Desk' });
  check(
    'choosing a folder and a name puts the desk in step and writes into the folder',
    deskState.phase === 'in_step' &&
      deskState.computerName === 'Desk' &&
      deskState.problem === null &&
      readdirSync(folder).length > 0,
    JSON.stringify(deskState),
  );

  // The laptop joins. It sees the desk, and takes nothing in before the
  // manager says so.
  const joined = await onLaptop('choose_sync_folder', { folder, computerName: 'Laptop' });
  const deskSeen = joined.computers.find((computer) => computer.name === 'Desk');
  check(
    'a second computer sees the first by its name, as one it has not decided about',
    deskSeen?.acceptance === 'undecided' && deskSeen.id === deskState.thisComputer,
    JSON.stringify(joined.computers),
  );
  check(
    'and takes nothing in before the manager accepts that computer',
    (await onLaptop('list_projects')).length === 0,
  );
  await onLaptop('accept_computer', { computerId: deskSeen.id });
  const taken = await onLaptop('list_projects');
  const takenTasks = taken[0] ? await onLaptop('list_tasks', { projectId: taken[0].id }) : [];
  const takenTeam = taken[0] ? await onLaptop('list_members', { projectId: taken[0].id }) : [];
  const takenSprints = taken[0] ? await onLaptop('list_sprints', { projectId: taken[0].id }) : [];
  check(
    'once accepted, the project arrives with its tasks, its team and its sprint',
    taken.length === 1 &&
      taken[0].id === project.id &&
      taken[0].key === 'SSN' &&
      takenTasks.length === 2 &&
      takenTeam.length === 1 &&
      takenSprints.length === 1,
    JSON.stringify([taken.length, takenTasks.length, takenTeam.length, takenSprints.length]),
  );
  const takenBoard = takenTasks.find((task) => task.id === board.id);
  check(
    'a task arrives as it was: given to the same member and planned into the same sprint',
    takenBoard?.assignee?.memberId === engineer.id && takenBoard.sprintId === sprint.id,
    JSON.stringify(takenBoard),
  );

  // The desk accepts the laptop, so that changes travel both ways.
  const deskAgain = await onDesk('sync_now');
  const laptopSeen = deskAgain.computers.find((computer) => computer.name === 'Laptop');
  check(
    'the first computer sees the second after its next round',
    laptopSeen?.acceptance === 'undecided',
    JSON.stringify(deskAgain.computers),
  );
  await onDesk('accept_computer', { computerId: laptopSeen.id });

  // A change on the laptop reaches the desk.
  await onLaptop('rename_task', { taskId: firmware.id, title: 'Write the firmware for the board' });
  const enclosure = await onLaptop('create_task', {
    input: { projectId: project.id, title: 'Design the enclosure' },
  });
  await onLaptop('sync_now');
  await onDesk('sync_now');
  let deskTasks = await onDesk('list_tasks', { projectId: project.id });
  check(
    'what the laptop changed and created reaches the desk',
    deskTasks.find((task) => task.id === firmware.id)?.title ===
      'Write the firmware for the board' &&
      deskTasks.find((task) => task.id === enclosure.id)?.title === 'Design the enclosure',
    JSON.stringify(deskTasks.map((task) => task.title)),
  );
  check(
    'the task the laptop created has a number of its own on both computers',
    new Set(deskTasks.map((task) => task.number)).size === deskTasks.length,
    JSON.stringify(deskTasks.map((task) => task.number)),
  );

  // A view the manager adds on one computer is there on the other, and the
  // six a project starts with are not doubled by the taking in.
  const laptopView = await onLaptop('add_view', {
    input: { projectId: project.id, kind: 'list', name: 'Mine' },
  });
  await onLaptop('set_view_filter', {
    viewId: laptopView.id,
    filter: { assignee: { kind: 'manager' }, hidesFinished: true },
  });
  await onLaptop('sync_now');
  await onDesk('sync_now');
  const deskViews = await onDesk('list_views', { projectId: project.id });
  check(
    'a view added and filtered on the laptop reaches the desk, after the six of the project',
    deskViews.map((view) => view.name).join(', ') ===
      'Board, Backlog, List, Gantt, Calendar, Dashboard, Mine' &&
      deskViews.at(-1).filter.assignee.kind === 'manager' &&
      deskViews.at(-1).filter.hidesFinished === true,
    JSON.stringify(deskViews.map((view) => [view.name, view.filter])),
  );

  // Two changes to one task, made apart, must both survive.
  await onDesk('rename_task', { taskId: board.id, title: 'Draw the schematic of the board' });
  await onLaptop('prioritize_task', { taskId: board.id, priority: 'urgent' });
  await onLaptop('sync_now');
  await onDesk('sync_now');
  await onLaptop('sync_now');
  const onBothDesk = (await onDesk('list_tasks', { projectId: project.id })).find(
    (task) => task.id === board.id,
  );
  const onBothLaptop = (await onLaptop('list_tasks', { projectId: project.id })).find(
    (task) => task.id === board.id,
  );
  check(
    'a new title from one computer and a new priority from the other both hold, on both',
    [onBothDesk, onBothLaptop].every(
      (task) => task?.title === 'Draw the schematic of the board' && task.priority === 'urgent',
    ),
    JSON.stringify([
      onBothDesk?.title,
      onBothDesk?.priority,
      onBothLaptop?.title,
      onBothLaptop?.priority,
    ]),
  );

  // A deletion is final: the change made to the task elsewhere is dropped,
  // and the computer that made it is told.
  await onDesk('delete_task', { taskId: enclosure.id });
  await onLaptop('rename_task', { taskId: enclosure.id, title: 'Design the printed enclosure' });
  await onDesk('sync_now');
  await onLaptop('sync_now');
  await onDesk('sync_now');
  const goneOnLaptop = !(await onLaptop('list_tasks', { projectId: project.id })).some(
    (task) => task.id === enclosure.id,
  );
  const goneOnDesk = !(await onDesk('list_tasks', { projectId: project.id })).some(
    (task) => task.id === enclosure.id,
  );
  const notices = await onLaptop('sync_notices');
  check('a task deleted on one computer is gone on both', goneOnLaptop && goneOnDesk);
  check(
    'the computer whose change was dropped is told so',
    notices.some((notice) => notice.kind === 'change_dropped'),
    JSON.stringify(notices),
  );
  if (notices[0]) {
    await onLaptop('dismiss_sync_notice', { noticeId: notices[0].id });
    check(
      'a notice that is dismissed is gone',
      (await onLaptop('sync_notices')).every((notice) => notice.id !== notices[0].id),
    );
  }

  // Nobody asks for a round: the change must travel by itself.
  await onDesk('rename_task', { taskId: firmware.id, title: 'Write and test the firmware' });
  const deadline = Date.now() + BACKGROUND_TIMEOUT_MILLISECONDS;
  let arrived = false;
  while (Date.now() < deadline && !arrived) {
    await sleep(2_000);
    arrived =
      (await onLaptop('list_tasks', { projectId: project.id })).find(
        (task) => task.id === firmware.id,
      )?.title === 'Write and test the firmware';
  }
  check('a change travels by itself, without anybody asking for a round', arrived);
  await sleep(1_500);
  const shown = await laptopWindow.evaluate('document.body.innerText');
  check(
    'and the window of the other computer shows it without being loaded again',
    shown.includes('Write and test the firmware'),
    shown.slice(0, 200),
  );
  check(
    'the foot of its navigation says how syncing stands',
    /Computers/.test(shown) && /In step/i.test(shown),
    shown.slice(-200),
  );

  // The laptop stops syncing: its data stays, and it hears nothing more.
  const stopped = await onLaptop('stop_syncing');
  await onDesk('rename_task', { taskId: firmware.id, title: 'Firmware, second pass' });
  await onDesk('sync_now');
  const afterStopping = await laptopWindow.command('sync_now');
  check(
    'a computer that stops syncing keeps its data and takes nothing more in',
    stopped.phase === 'off' &&
      afterStopping.refused?.code === 'sync_off' &&
      (await onLaptop('list_projects')).length === 1 &&
      (await onLaptop('list_tasks', { projectId: project.id })).find(
        (task) => task.id === firmware.id,
      )?.title === 'Write and test the firmware',
    JSON.stringify([stopped.phase, afterStopping]),
  );

  // The desk is closed and opened again: it still knows its folder, its
  // name and the other computer.
  deskWindow.close();
  await desk.stop();
  desk = DesktopApplication.start(executable, deskData, DESK_PORT);
  deskWindow = await desk.openWindow();
  await deskWindow.waitForText('Smart sensor node');
  onDesk = commandsOf(deskWindow);
  const restarted = await onDesk('sync_now');
  check(
    'after a restart the desk still syncs with its folder, under its name',
    restarted.phase === 'in_step' &&
      restarted.computerName === 'Desk' &&
      restarted.folder !== null &&
      restarted.computers.some(
        (computer) => computer.name === 'Laptop' && computer.acceptance === 'accepted',
      ),
    JSON.stringify(restarted),
  );
  deskWindow.close();
  laptopWindow.close();
} catch (failure) {
  check('the test ran to its end', false, failure.message);
} finally {
  await desk.stop();
  await laptop.stop();
  removeDataDirectory(deskData);
  removeDataDirectory(laptopData);
  rmSync(shared, { recursive: true, force: true });
}

console.log(failures === 0 ? '\nAll checks passed.' : `\n${failures} check(s) failed.`);
process.exit(failures === 0 ? 0 : 1);
