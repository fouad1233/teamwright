/**
 * Checks that a database made by an older build opens in a newer one, on
 * Linux.
 *
 * It starts the older program on an empty temporary data folder and makes a
 * project there with what every version since the first release has: tasks,
 * a dependency, a member, a running sprint, a filtered view, a second area
 * and a finished task. Then it starts the newer program on the same folder
 * and checks that the database was brought to the newer format, that the
 * project is there as it was left, that the commands of the newer features
 * answer on it, and that it can be changed with them.
 *
 * Usage:
 *   node tools/smoke/old-database-linux.mjs <older-program> <newer-program> [picture-folder]
 *
 * The older program is usually the built program of the main branch, from a
 * worktree of its own (see docs/development/README.md). It needs what
 * desktop-smoke-linux.mjs needs, and Node.js 22.13 or later for `node:sqlite`,
 * with which it reads the format version of the database.
 */

import { mkdirSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { setTimeout as sleep } from 'node:timers/promises';

import { DesktopApplication, removeDataDirectory } from '../shared/linux-desktop-application.mjs';

const [olderProgram, newerProgram, pictureFolder] = process.argv.slice(2);
if (!olderProgram || !newerProgram) {
  console.error(
    'usage: node old-database-linux.mjs <older-program> <newer-program> [picture-folder]',
  );
  process.exit(2);
}
if (pictureFolder) {
  mkdirSync(pictureFolder, { recursive: true });
}

const DRIVER_PORT = Number(process.env.TEAMWRIGHT_SMOKE_PORT ?? 4764);

let failures = 0;

function check(description, passed, detail = '') {
  console.log(
    `${passed ? 'ok  ' : 'FAIL'}  ${description}${passed || !detail ? '' : `  (${detail})`}`,
  );
  if (!passed) {
    failures += 1;
  }
}

/** The format version of the database in a data folder: the number of migrations it has seen. */
function formatVersion(dataDirectory) {
  const database = new DatabaseSync(join(dataDirectory, 'teamwright.db'), { readOnly: true });
  try {
    return database.prepare('PRAGMA user_version').get().user_version;
  } finally {
    database.close();
  }
}

function commandsOf(window) {
  return async (name, args) => {
    const answer = await window.command(name, args);
    if (!('ok' in answer)) {
      throw new Error(`${name} was refused: ${JSON.stringify(answer.refused)}`);
    }
    return answer.ok;
  };
}

const dataDirectory = mkdtempSync(join(tmpdir(), 'teamwright-old-database-'));
let application = null;

try {
  // The older program makes the database.
  application = await DesktopApplication.start(olderProgram, dataDirectory, DRIVER_PORT);
  let window = await application.openWindow();
  let must = commandsOf(window);
  await window.waitForText('No projects yet');
  const project = await must('create_project', {
    input: { name: 'Old project', workflow: 'engineering' },
  });
  const board = await must('create_task', { input: { projectId: project.id, title: 'Old board' } });
  const firmware = await must('create_task', {
    input: { projectId: project.id, title: 'Old firmware' },
  });
  const enclosure = await must('create_task', {
    input: { projectId: project.id, title: 'Old enclosure' },
  });
  await must('add_task_dependency', { taskId: firmware.id, blockerId: board.id });
  await must('assign_task', { taskId: board.id, assignee: { kind: 'manager' } });
  const member = await must('add_member', {
    input: {
      projectId: project.id,
      name: 'Old engineer',
      kind: 'agent',
      responsibility: 'The board.',
    },
  });
  await must('assign_task', {
    taskId: firmware.id,
    assignee: { kind: 'member', memberId: member.id },
  });
  const sprint = await must('create_sprint', { input: { projectId: project.id } });
  await must('plan_task', { taskId: board.id, sprintId: sprint.id });
  await must('set_task_points', { taskId: board.id, points: 3 });
  await must('start_sprint', {
    sprintId: sprint.id,
    input: { name: 'Old sprint', goal: 'A board', startOn: '2026-11-16', endOn: '2026-11-29' },
  });
  const view = await must('add_view', {
    input: { projectId: project.id, kind: 'list', name: 'Old list' },
  });
  await must('set_view_filter', {
    viewId: view.id,
    filter: { assignee: { kind: 'manager' }, hidesFinished: true },
  });
  const area = await must('add_area', { projectId: project.id, name: 'Old area' });
  await must('move_task_to_area', { taskId: enclosure.id, areaId: area.id });
  const done = project.statuses.find((status) => status.category === 'done');
  await must('move_task', { taskId: enclosure.id, statusId: done.id, index: 0 });
  await application.stop();
  application = null;
  const olderVersion = formatVersion(dataDirectory);
  console.log(`      the older program left a database of format ${olderVersion}`);

  // The newer program opens it.
  application = await DesktopApplication.start(newerProgram, dataDirectory, DRIVER_PORT);
  window = await application.openWindow();
  must = commandsOf(window);
  await window.waitForText('Old project');
  const projects = await must('list_projects');
  check(
    'the old project is listed, its statuses without limits',
    projects.length === 1 &&
      projects[0].id === project.id &&
      projects[0].statuses.every((status) => status.limit === null),
    JSON.stringify(projects),
  );
  const tasks = await must('list_tasks', { projectId: project.id });
  const storedBoard = tasks.find((task) => task.id === board.id);
  check(
    'the old tasks are there, without tags, checklists or values of fields',
    tasks.length === 3 &&
      tasks.every(
        (task) =>
          task.tagIds.length === 0 &&
          task.checklist.length === 0 &&
          Object.keys(task.fieldValues).length === 0,
      ),
    JSON.stringify(tasks),
  );
  check(
    'they keep their assignee, sprint, points, dependency, area and completion',
    storedBoard?.assignee?.kind === 'manager' &&
      storedBoard.sprintId === sprint.id &&
      storedBoard.points === 3 &&
      tasks.find((task) => task.id === firmware.id)?.blockedBy.includes(board.id) &&
      tasks.find((task) => task.id === enclosure.id)?.areaId === area.id &&
      tasks.find((task) => task.id === enclosure.id)?.completedAt !== null,
    JSON.stringify(tasks),
  );
  const sprints = await must('list_sprints', { projectId: project.id });
  check(
    'the old sprint still runs',
    sprints.length === 1 && sprints[0].state === 'active' && sprints[0].name === 'Old sprint',
    JSON.stringify(sprints),
  );
  const views = await must('list_views', { projectId: project.id });
  const oldView = views.find((candidate) => candidate.id === view.id);
  check(
    'the old view keeps its filter and is grouped by status',
    oldView?.filter.assignee.kind === 'manager' &&
      oldView.filter.hidesFinished === true &&
      oldView.grouping === 'status' &&
      (oldView.filter.tagId ?? null) === null,
    JSON.stringify(views),
  );
  const areas = await must('list_areas', { projectId: project.id });
  check(
    'the old areas are there',
    areas.map((stored) => stored.name).join(', ') === 'General, Old area',
    JSON.stringify(areas),
  );
  const members = await must('list_members', { projectId: project.id });
  check(
    'the old member is there, without instructions of its own',
    members.length === 1 && members[0].instructions === '',
    JSON.stringify(members),
  );
  const reads = [
    ['list_task_history', { taskId: board.id }],
    ['list_project_history', { projectId: project.id, limit: 20, before: null }],
    ['list_task_actuals', { projectId: project.id }],
    ['sprint_report', { sprintId: sprint.id }],
    ['sprint_forecast', { projectId: project.id }],
    ['list_status_loads', { projectId: project.id }],
    ['list_tags', { projectId: project.id }],
    ['list_fields', { projectId: project.id }],
    ['list_task_groups', { viewId: view.id }],
    ['project_memory', { projectId: project.id }],
    ['coordinator', { projectId: project.id }],
    ['project_library', { projectId: project.id }],
    ['team_talk', { projectId: project.id }],
    ['permission_requests', { projectId: project.id }],
    ['list_digests', { projectId: project.id }],
    ['project_report', { projectId: project.id }],
    ['project_usage', { projectId: project.id }],
    ['list_conversations', { projectId: project.id }],
  ];
  const refused = [];
  for (const [name, args] of reads) {
    const answer = await window.command(name, args);
    if (!('ok' in answer)) {
      refused.push(`${name}: ${JSON.stringify(answer.refused)}`);
    }
  }
  check(
    `the ${reads.length} commands that read the newer features answer on the old project`,
    refused.length === 0,
    refused.join('; '),
  );

  const tagged = await must('put_named_tag_on_task', { taskId: board.id, name: 'Legacy' });
  const listed = await must('add_checklist_item', {
    taskId: board.id,
    text: 'Check the old board',
  });
  const field = await must('add_field', {
    projectId: project.id,
    name: 'Revision',
    kind: 'text',
    unit: null,
  });
  const valued = await must('set_field_value', {
    taskId: board.id,
    fieldId: field.id,
    value: { kind: 'text', text: 'B' },
  });
  const grouped = await must('set_view_grouping', { viewId: view.id, grouping: 'tag' });
  const limited = await must('set_status_limit', {
    projectId: project.id,
    statusId: project.statuses[1].id,
    limit: 2,
  });
  const instructed = await must('set_member_instructions', {
    memberId: member.id,
    instructions: 'Keep it simple.',
  });
  check(
    'the old project is changed with the newer features',
    tagged.task.tagIds.length === 1 &&
      listed.checklist.length === 1 &&
      valued.fieldValues[field.id]?.text === 'B' &&
      grouped.grouping === 'tag' &&
      limited.statuses[1].limit === 2 &&
      instructed.instructions === 'Keep it simple.',
  );
  const history = await must('list_task_history', { taskId: board.id });
  check(
    'the history of an old task goes on where it was',
    history[0]?.kind === 'created' && history.some((change) => change.kind === 'tagged'),
    JSON.stringify(history.map((change) => change.kind)),
  );
  await window.reload();
  await window.click(
    `[...document.querySelectorAll('nav[aria-label="Projects"] button')].find((button) => button.innerText.includes('Old project'))`,
  );
  await sleep(800);
  const shown = await window.waitForText('Old board');
  check('the board of the old project shows the new tag', shown.includes('Legacy'));
  if (pictureFolder) {
    await window.screenshot(join(pictureFolder, 'old-database.png'));
  }
  await application.stop();
  application = null;
  const newerVersion = formatVersion(dataDirectory);
  console.log(`      the newer program brought it to format ${newerVersion}`);
  check(
    'the database was brought to a newer format',
    newerVersion > olderVersion,
    `${olderVersion} -> ${newerVersion}`,
  );
} catch (failure) {
  check('the test ran to its end', false, failure.message);
} finally {
  await application?.stop();
  removeDataDirectory(dataDirectory);
}

console.log(failures === 0 ? '\nAll checks passed.' : `\n${failures} check(s) failed.`);
process.exit(failures === 0 ? 0 : 1);
