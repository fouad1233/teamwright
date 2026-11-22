/**
 * Smoke test of the desktop application on Linux.
 *
 * It starts the built application on an empty data folder, on a virtual
 * display, drives the real window through WebDriver (tauri-driver in front
 * of WebKitWebDriver), and checks that the interface, the commands of the
 * shell and the database work together for the features that the Windows
 * smoke test does not reach: templates, tags, checklists, custom fields,
 * grouped views, the Gantt chart against what happened, the dashboard, the
 * report of the whole project, ready-made specialists and their library,
 * memory, the coordinator, the list of earlier conversations, themes and
 * the notice of newer versions. Then it starts the application a second
 * time on the same folder to prove that what was stored is still there.
 *
 * Usage:
 *   node tools/smoke/desktop-smoke-linux.mjs <path-to-teamwright-desktop> [picture-folder]
 *
 * With a picture folder, it takes a picture of the window at every station.
 * It needs Xvfb, tauri-driver and WebKitWebDriver; see
 * docs/development/linux-without-admin.md. The user's real data is never
 * touched: the application is pointed at a temporary folder through
 * TEAMWRIGHT_DATA_DIR, and at a stand-in for Claude Code.
 *
 * Two of its stations reach outside: "Look now" asks GitHub's public list
 * of releases once, and the list of earlier conversations reads the
 * conversations of Claude Code on this computer. The test prints only how
 * many it found, and its pictures stay in the folder it is given.
 */

import { existsSync, mkdirSync, mkdtempSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';

import {
  DesktopApplication,
  removeDataDirectory,
  STAND_IN_AGENT_VERSION,
} from '../shared/linux-desktop-application.mjs';

const [executable, pictureFolder] = process.argv.slice(2);
if (!executable) {
  console.error(
    'usage: node desktop-smoke-linux.mjs <path-to-teamwright-desktop> [picture-folder]',
  );
  process.exit(2);
}
if (pictureFolder) {
  mkdirSync(pictureFolder, { recursive: true });
}

/** The port of tauri-driver; WebKitWebDriver gets the next one. */
const DRIVER_PORT = Number(process.env.TEAMWRIGHT_SMOKE_PORT ?? 4744);

/** How long the coordinator is watched with a specialist that cannot work. */
const COORDINATOR_WATCH_MILLISECONDS = Number(process.env.TEAMWRIGHT_SMOKE_WATCH ?? 60_000);

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

let window;
let pictureNumber = 0;

/** Takes a picture of the window, if the test was given a folder for them. */
async function picture(name) {
  if (!pictureFolder) {
    return;
  }
  pictureNumber += 1;
  const path = join(pictureFolder, `${String(pictureNumber).padStart(2, '0')}-${name}.png`);
  await window.screenshot(path);
  console.log(`      picture: ${path}`);
}

/**
 * Runs one station of the test. A station that throws is one failure; the
 * test goes on with the next station, so that one defect does not hide the
 * others.
 */
async function station(name, run) {
  console.log(`\n# ${name}`);
  try {
    await run();
  } catch (failure) {
    check(`the station "${name}" ran to its end`, false, failure.message);
    await picture(`${name.replaceAll(/[^a-z0-9]+/gi, '-').toLowerCase()}-failed`).catch(() => {});
  }
}

/** The value of a command, or a failure that names the command and its refusal. */
async function must(name, args) {
  const answer = await window.command(name, args);
  if (!('ok' in answer)) {
    throw new Error(`${name} was refused: ${JSON.stringify(answer.refused)}`);
  }
  return answer.ok;
}

/** Opens a project from the navigation. */
async function openProject(name) {
  await window.waitFor(
    `[...document.querySelectorAll('nav[aria-label="Projects"] button')].some((b) => b.innerText.includes(${JSON.stringify(name)}))`,
  );
  await window.click(
    `[...document.querySelectorAll('nav[aria-label="Projects"] button')].find((b) => b.innerText.includes(${JSON.stringify(name)}))`,
  );
  await sleep(500);
}

/**
 * Clicks the element a JavaScript expression finds the way a person would,
 * through WebDriver. A menu that opens when the pointer goes down needs
 * this; a click made by JavaScript does not open it.
 */
async function pointerClick(expression) {
  await window.evaluate(`(() => {
    document.querySelectorAll('[data-smoke-target]').forEach((element) => element.removeAttribute('data-smoke-target'));
    (${expression}).setAttribute('data-smoke-target', '');
  })()`);
  await window.clickElement('[data-smoke-target]');
}

/**
 * Opens a view or a place of the open project by the name on its tab. A
 * view that does not fit the window is behind "N more".
 */
async function openTab(name) {
  const tabs = `[...document.querySelectorAll('[role="tab"]')]`;
  const isThere = await window.evaluate(
    `${tabs}.some((element) => element.innerText.trim() === ${JSON.stringify(name)})`,
  );
  if (isThere) {
    await window.clickText(name, '[role="tab"]');
  } else {
    await pointerClick(
      `[...document.querySelectorAll('button')].find((element) => / more$/.test(element.innerText.trim()))`,
    );
    await window.waitFor(
      `[...document.querySelectorAll('[role="menuitem"]')].some((element) => element.innerText.trim() === ${JSON.stringify(name)})`,
    );
    await pointerClick(
      `[...document.querySelectorAll('[role="menuitem"]')].find((element) => element.innerText.trim() === ${JSON.stringify(name)})`,
    );
  }
  await sleep(800);
}

/** Whether two values of a field are the same, whatever the order of their keys. */
function sameValue(one, other) {
  const sorted = (value) =>
    JSON.stringify(
      Object.fromEntries(Object.entries(value ?? {}).sort(([a], [b]) => a.localeCompare(b))),
    );
  return sorted(one) === sorted(other);
}

/** What the window shows, as a person reads it. */
function shownText() {
  return window.evaluate("document.body?.innerText ?? ''");
}

/**
 * What the window shows, with what its fields hold: the text of a field is
 * not part of the text of the page.
 */
function shownTextWithFields() {
  return window.evaluate(
    `(document.body?.innerText ?? '') + '\\n' + [...document.querySelectorAll('input, textarea')].map((field) => field.value).join('\\n')`,
  );
}

// ------------------------------------------------------------------ the run

const dataDirectory = mkdtempSync(join(tmpdir(), 'teamwright-smoke-'));
let application = await DesktopApplication.start(executable, dataDirectory, DRIVER_PORT);

/** What the first run made, for the second run to find again. */
const made = {};

try {
  window = await application.openWindow();
  await window.showAtSize(1440, 900);

  await station('first screen', async () => {
    await window.waitForText('No projects yet');
    check('a new installation shows the first-run screen', true);
    check('it starts without projects', (await must('list_projects')).length === 0);
    await picture('first-screen');
  });

  await station('templates', async () => {
    const templates = await must('list_project_templates');
    check(
      'five templates are offered',
      templates.map((template) => template.key).join(', ') ===
        'standard, kanban, scrum, hardware, engineering',
      templates.map((template) => template.key).join(', '),
    );
    const names = {
      standard: 'Plain list',
      kanban: 'Support flow',
      scrum: 'Mobile app',
      hardware: 'Smart sensor node',
      engineering: 'Test rig',
    };
    made.projects = {};
    // The Kanban project is made the way a person makes one: in the dialog,
    // typed key by key, with the template chosen by a click. The others are
    // made with the command the dialog calls.
    await window.clickText('Create a project', 'button');
    await window.waitForText('Kanban');
    await window.type('input[placeholder="Smart sensor node"]', names.kanban);
    await pointerClick(
      `[...document.querySelectorAll('label')].find((label) => label.querySelector('input[type="radio"]') && label.innerText.trim().startsWith('Kanban'))`,
    );
    await sleep(300);
    await picture('new-project-dialog');
    await window.clickText('Create project', 'button[type="submit"]');
    await window.waitFor(
      `[...document.querySelectorAll('nav[aria-label="Projects"] button')].some((b) => b.innerText.includes(${JSON.stringify(names.kanban)}))`,
    );
    for (const template of templates) {
      const project =
        template.key === 'kanban'
          ? (await must('list_projects')).find((candidate) => candidate.name === names.kanban)
          : await must('create_project', {
              input: { name: names[template.key] ?? template.name, workflow: template.key },
            });
      if (!project) {
        throw new Error(`the project from the template ${template.name} was not made`);
      }
      made.projects[template.key] = project;
      const views = await must('list_views', { projectId: project.id });
      const areas = await must('list_areas', { projectId: project.id });
      const sprints = await must('list_sprints', { projectId: project.id });
      check(
        `a project from the template ${template.name} has its statuses, limits, views, areas and sprints`,
        project.statuses.map((s) => `${s.name}/${s.category}/${s.limit ?? '-'}`).join() ===
          template.statuses.map((s) => `${s.name}/${s.category}/${s.limit ?? '-'}`).join() &&
          views.map((view) => view.kind).join() === template.views.join() &&
          areas.map((area) => area.name).join() === template.areas.join() &&
          sprints.length === (template.plansFirstSprint ? 1 : 0),
        JSON.stringify({
          statuses: project.statuses.map((s) => [s.name, s.category, s.limit]),
          views: views.map((view) => view.kind),
          areas: areas.map((area) => area.name),
          sprints: sprints.length,
          template,
        }),
      );
    }
    const kanban = made.projects.kanban;
    const limited = kanban.statuses.filter((status) => status.limit !== null);
    const loads = await must('list_status_loads', { projectId: kanban.id });
    check(
      'the statuses of a Kanban project have limits, and their loads say so',
      limited.length > 0 &&
        limited.every(
          (status) => loads.find((load) => load.statusId === status.id)?.limit === status.limit,
        ),
      JSON.stringify(loads),
    );
    // Fill a limited lane beyond its limit, so that the board marks it.
    const lane = limited[0];
    for (let index = 0; index <= lane.limit; index += 1) {
      const task = await must('create_task', {
        input: { projectId: kanban.id, title: `Ticket ${index + 1}` },
      });
      await must('move_task', { taskId: task.id, statusId: lane.id, index });
    }
    const overLoad = (await must('list_status_loads', { projectId: kanban.id })).find(
      (load) => load.statusId === lane.id,
    );
    check(
      'a lane past its limit is counted as over it',
      overLoad?.count === lane.limit + 1 && overLoad.fill === 'over',
      JSON.stringify(overLoad),
    );
    await window.reload();
    await openProject('Support flow');
    await openTab('Board');
    const shown = await window.waitForText(lane.name);
    check(
      'the board of the Kanban project shows the limit at the head of its lane, and "over"',
      shown.includes(`${lane.limit + 1} / ${lane.limit}`) || /over/i.test(shown),
      shown.slice(0, 600),
    );
    await picture('kanban-board-limits');
  });

  await station('a task with a tag, a checklist, custom fields and a dependency', async () => {
    const project = made.projects.hardware;
    const board = await must('create_task', {
      input: { projectId: project.id, title: 'Design the board' },
    });
    const firmware = await must('create_task', {
      input: { projectId: project.id, title: 'Write the firmware' },
    });
    made.board = board;
    made.firmware = firmware;

    const tagged = await must('put_named_tag_on_task', { taskId: board.id, name: 'Power' });
    made.tag = tagged.tag;
    check(
      'a tag made in place is put on the task',
      tagged.tag.name === 'Power' && tagged.task.tagIds.includes(tagged.tag.id),
      JSON.stringify(tagged),
    );
    const again = await must('put_named_tag_on_task', { taskId: firmware.id, name: 'power' });
    check(
      'the same name, whatever the capitals, finds the tag that is there',
      again.tag.id === tagged.tag.id &&
        (await must('list_tags', { projectId: project.id })).length === 1,
      JSON.stringify(again),
    );

    let task;
    for (const text of ['Choose the regulator', 'Draw the schematic', 'Order the parts']) {
      task = await must('add_checklist_item', { taskId: board.id, text });
    }
    task = await must('tick_checklist_item', {
      taskId: board.id,
      itemId: task.checklist[0].id,
      done: true,
    });
    task = await must('tick_checklist_item', {
      taskId: board.id,
      itemId: task.checklist[1].id,
      done: true,
    });
    check(
      'a checklist of three items has two ticked',
      task.checklist.map((item) => `${item.text}:${item.done}`).join() ===
        'Choose the regulator:true,Draw the schematic:true,Order the parts:false',
      JSON.stringify(task.checklist),
    );

    const fieldOf = (name, kind, unit = null) =>
      must('add_field', { projectId: project.id, name, kind, unit });
    const supplier = await fieldOf('Supplier', 'text');
    const cost = await fieldOf('Unit cost', 'number', 'EUR');
    const sample = await fieldOf('Sample due', 'date');
    const certified = await fieldOf('Certified', 'yes_no');
    let stage = await fieldOf('Stage', 'choice');
    stage = await must('add_field_choice', { fieldId: stage.id, name: 'Prototype', color: null });
    stage = await must('add_field_choice', { fieldId: stage.id, name: 'Series', color: null });
    const sheet = await fieldOf('Data sheet', 'link');
    made.fields = { supplier, cost, sample, certified, stage, sheet };
    const values = [
      [supplier, { kind: 'text', text: 'Acme Parts' }],
      [cost, { kind: 'number', number: 12.5 }],
      [sample, { kind: 'date', date: '2026-12-01' }],
      [certified, { kind: 'yes_no', yes: true }],
      [stage, { kind: 'choice', choiceId: stage.choices[0].id }],
      [sheet, { kind: 'link', address: 'https://example.com/regulator.pdf' }],
    ];
    for (const [field, value] of values) {
      task = await must('set_field_value', { taskId: board.id, fieldId: field.id, value });
    }
    check(
      'a value of each of the six kinds of field is set on the task',
      values.every(([field, value]) => sameValue(task.fieldValues[field.id], value)),
      JSON.stringify(task.fieldValues),
    );
    const wrongKind = await window.command('set_field_value', {
      taskId: board.id,
      fieldId: cost.id,
      value: { kind: 'text', text: 'cheap' },
    });
    check(
      'a value of the wrong kind for a field is refused',
      wrongKind.refused !== undefined,
      JSON.stringify(wrongKind),
    );

    await must('add_task_dependency', { taskId: firmware.id, blockerId: board.id });
    const history = await must('list_task_history', { taskId: board.id });
    const kinds = history.map((change) => change.kind);
    check(
      'the history of the task tells the tag, the two ticks and the six values',
      kinds.filter((kind) => kind === 'tagged').length === 1 &&
        kinds.filter((kind) => kind === 'ticked').length === 2 &&
        kinds.filter((kind) => kind === 'field_value_changed').length === 6,
      kinds.join(', '),
    );
    const firmwareHistory = await must('list_task_history', { taskId: firmware.id });
    check(
      'the history of the waiting task says what it waits for',
      firmwareHistory.some(
        (change) => change.kind === 'now_waits_for' && change.taskId === board.id,
      ),
      JSON.stringify(firmwareHistory.map((change) => change.kind)),
    );

    await window.reload();
    await openProject('Smart sensor node');
    await openTab('Board');
    const card = await window.waitForText('Design the board');
    check(
      'the card shows its tag and the progress of its checklist',
      card.includes('Power') && card.includes('2/3'),
      card.slice(0, 600),
    );
    await window.click(
      `[...document.querySelectorAll('[role="button"]')].find((element) => element.innerText.trim().startsWith('Design the board'))`,
    );
    await window.waitForText('CHECKLIST');
    await sleep(500);
    const panel = await shownTextWithFields();
    check(
      'the panel of the task shows its tag, its checklist and the values of its fields',
      ['Power', '2 of 3', 'Order the parts', 'Acme Parts', '12.5', 'Prototype', 'Yes'].every(
        (text) => panel.includes(text),
      ),
      panel.slice(-800),
    );
    await picture('task-panel');
    await window.pressEscape();
  });

  await station('views', async () => {
    const project = made.projects.hardware;
    const table = await must('add_view', { input: { projectId: project.id, kind: 'table' } });
    const byAssignee = await must('add_view', {
      input: { projectId: project.id, kind: 'board', name: 'By owner' },
    });
    await must('set_view_grouping', { viewId: byAssignee.id, grouping: 'assignee' });
    const byTag = await must('add_view', {
      input: { projectId: project.id, kind: 'board', name: 'By tag' },
    });
    await must('set_view_grouping', { viewId: byTag.id, grouping: 'tag' });
    const report = await must('add_view', { input: { projectId: project.id, kind: 'report' } });
    made.views = { table, byAssignee, byTag, report };
    await must('assign_task', { taskId: made.board.id, assignee: { kind: 'manager' } });

    const ownerGroups = await must('list_task_groups', { viewId: byAssignee.id });
    check(
      'a board grouped by assignee has a lane for the manager and one for nobody',
      ownerGroups.some((group) => group.key.kind === 'manager') &&
        ownerGroups.some((group) => group.key.kind === 'nobody'),
      JSON.stringify(ownerGroups),
    );
    const tagGroups = await must('list_task_groups', { viewId: byTag.id });
    check(
      'a board grouped by tag has a lane for the tag',
      tagGroups.some((group) => group.key.kind === 'tag' && group.key.id === made.tag.id),
      JSON.stringify(tagGroups),
    );

    await window.reload();
    await openProject('Smart sensor node');
    await openTab(table.name);
    // The heads of the columns are shown in capitals, and read so.
    await window.waitForText('UNIT COST');
    const tableShown = await shownTextWithFields();
    check(
      'the Table view has a column for each custom field, with the values',
      ['SUPPLIER', 'STAGE', 'CERTIFIED', 'DATA SHEET', 'Acme Parts'].every((text) =>
        tableShown.includes(text),
      ),
      tableShown.slice(0, 600),
    );
    // The columns of the fields come after the fixed ones: show them.
    await window.evaluate(
      `[...document.querySelectorAll('*')].filter((element) => element.scrollWidth > element.clientWidth + 20 && getComputedStyle(element).overflowX !== 'visible').forEach((element) => { element.scrollLeft = element.scrollWidth; })`,
    );
    await sleep(300);
    await picture('table-view');

    await openTab('By owner');
    const ownerShown = await shownText();
    check(
      'the board grouped by assignee shows its lanes',
      /manager|you/i.test(ownerShown) && /nobody|unassigned/i.test(ownerShown),
      ownerShown.slice(0, 600),
    );
    await picture('board-by-assignee');
    await openTab('By tag');
    const tagShown = await shownText();
    check('the board grouped by tag shows a lane for the tag', tagShown.includes('Power'));
    await picture('board-by-tag');

    // The Gantt chart with planned against actual.
    await must('reschedule_task', {
      taskId: made.board.id,
      startOn: '2026-11-16',
      dueOn: '2026-11-20',
    });
    const active = project.statuses.find((status) => status.category === 'active');
    await must('move_task', { taskId: made.board.id, statusId: active.id, index: 0 });
    await window.reload();
    await openProject('Smart sensor node');
    await openTab('Gantt');
    await window.waitFor(
      `[...document.querySelectorAll('button, [role="switch"]')].some((element) => element.innerText.trim() === 'Actual' || element.getAttribute('aria-label') === 'Actual')`,
    );
    await window.click(
      `[...document.querySelectorAll('button, [role="switch"]')].find((element) => element.innerText.trim() === 'Actual' || element.getAttribute('aria-label') === 'Actual')`,
    );
    await sleep(1000);
    const gantt = (await must('list_views', { projectId: project.id })).find(
      (view) => view.kind === 'gantt',
    );
    const actuals = await must('list_task_actuals', { projectId: project.id });
    check(
      'the Actual switch is kept by the view, and what happened is read from the log',
      (gantt.content ?? '').includes('true') &&
        actuals.some((actual) => actual.taskId === made.board.id),
      JSON.stringify({ content: gantt.content, actuals }),
    );
    await picture('gantt-actual');

    await openTab('Dashboard');
    const dashboard = await window.waitForText('LATEST ACTIVITY').catch(async () => shownText());
    check(
      'the Dashboard shows the latest activity',
      /latest activity/i.test(dashboard) && dashboard.includes('Design the board'),
      dashboard.slice(0, 800),
    );
    await picture('dashboard');

    // The report of the whole project, saved as Markdown into the work folder.
    await openTab(report.name);
    await window.clickText('Whole project', 'button[role="radio"]');
    const sheet = await window.waitForText('REPORT OF THE PROJECT SO FAR');
    check(
      'the Report view shows the sheet of the whole project',
      sheet.includes('Smart sensor node'),
    );
    await picture('report-whole-project');
    const whole = await must('project_report', { projectId: project.id });
    check(
      'the report of a project that is not finished says so',
      whole.isComplete === false && whole.title === 'Report of the project so far',
      JSON.stringify({ title: whole.title, isComplete: whole.isComplete }),
    );
    // The save window of the operating system cannot be driven here; the
    // interface calls these two commands around it, and the test does too.
    const suggested = await must('suggested_report_path', { projectId: project.id });
    const saved = await must('save_project_report', { projectId: project.id, path: suggested });
    const places = await must('project_places', { projectId: project.id });
    const markdown = existsSync(saved) ? readFileSync(saved, 'utf8') : '';
    check(
      'the report is saved as Markdown into the work folder of the project',
      saved.startsWith(places.workFolder) &&
        saved.endsWith('.md') &&
        markdown.includes('Smart sensor node') &&
        markdown.includes('Design the board'),
      JSON.stringify({
        suggested,
        saved,
        workFolder: places.workFolder,
        start: markdown.slice(0, 120),
      }),
    );
    made.reportPath = saved;
  });

  await station('the team', async () => {
    const project = made.projects.hardware;
    const readyMade = await must('add_ready_made_member', {
      projectId: project.id,
      key: 'firmware-engineer',
    });
    made.specialist = readyMade;
    check(
      'a ready-made specialist joins with instructions of its own',
      readyMade.kind === 'agent' && readyMade.instructions.length > 0,
      JSON.stringify(readyMade).slice(0, 300),
    );
    const changed = await must('set_member_instructions', {
      memberId: readyMade.id,
      instructions: 'Write C for the STM32. Keep every interrupt handler short.',
    });
    check(
      'its instructions are changed',
      changed.instructions === 'Write C for the STM32. Keep every interrupt handler short.',
      JSON.stringify(changed).slice(0, 300),
    );
    const bare = await must('add_connector', {
      projectId: project.id,
      input: {
        name: 'parts-search',
        start: {
          kind: 'program',
          program: 'uvx',
          arguments: ['parts-search-server'],
          variables: [],
        },
        tools: ['search_parts'],
      },
    });
    check(
      'a connector whose program is a bare name is added to the library',
      bare.connectors.some((connector) => connector.name === 'parts-search'),
      JSON.stringify(bare),
    );
    const relative = await window.command('add_connector', {
      projectId: project.id,
      input: {
        name: 'local-server',
        start: { kind: 'program', program: './server', arguments: [], variables: [] },
        tools: ['run'],
      },
    });
    check(
      'a connector whose program is a relative path is refused',
      relative.refused !== undefined,
      JSON.stringify(relative),
    );
    await window.reload();
    await openProject('Smart sensor node');
    await openTab('Team');
    const team = await window.waitForText(readyMade.name);
    check('the Team place shows the ready-made specialist', team.includes(readyMade.name));
    await picture('team');
    await window.clickText('Skills and connectors', 'button');
    const library = await window.waitForText('parts-search');
    check(
      'the library of skills and connectors opens and lists the connector',
      library.includes('Skills and connectors of Smart sensor node'),
      library.slice(0, 400),
    );
    // The dialog fades in.
    await sleep(600);
    await picture('library');
    await window.pressEscape();
  });

  await station('memory', async () => {
    const project = made.projects.hardware;
    const fact = await must('add_memory_fact', {
      input: {
        projectId: project.id,
        title: 'The supply is 3.3 V',
        text: 'Every part on the board runs from one 3.3 V rail.',
        taskId: null,
      },
    });
    made.fact = fact;
    const found = await must('search_memory', { projectId: project.id, query: 'rail' });
    check(
      'a fact added by hand is found by a word of its text',
      found.some((entry) => entry.id === fact.id),
      JSON.stringify(found),
    );
    await window.reload();
    await openProject('Smart sensor node');
    await openTab('Memory');
    const shown = await window.waitForText('The supply is 3.3 V');
    check('the Memory place shows the fact', shown.includes('The supply is 3.3 V'));
    await picture('memory');
  });

  await station('the coordinator', async () => {
    const project = made.projects.hardware;
    const program = await must('agent_program');
    check(
      'the stand-in says it is installed and signed in',
      program.state === 'ready' && program.version === STAND_IN_AGENT_VERSION,
      JSON.stringify(program),
    );
    // A task for the specialist that waits for nothing.
    const job = await must('create_task', {
      input: { projectId: project.id, title: 'Write the start-up code' },
    });
    await must('assign_task', {
      taskId: job.id,
      assignee: { kind: 'member', memberId: made.specialist.id },
    });
    made.job = job;
    await must('set_budget', {
      projectId: project.id,
      scope: { scope: 'project', targetId: project.id },
      tokens: 200000,
    });
    const switched = await must('switch_coordinator', { projectId: project.id, on: true });
    check(
      'the coordinator is switched on, with the budget of the project',
      switched.runs === 'here' && switched.budgets.some((budget) => budget.tokens === 200000),
      JSON.stringify(switched).slice(0, 400),
    );
    // Watch what it does with a specialist that cannot work.
    const samples = [];
    const deadline = Date.now() + COORDINATOR_WATCH_MILLISECONDS;
    while (Date.now() < deadline) {
      const conversations = await must('list_conversations', { projectId: project.id });
      const coordinator = await must('coordinator', { projectId: project.id });
      samples.push({
        conversations: conversations.length,
        turns: conversations.reduce((sum, c) => sum + (c.turns?.length ?? 0), 0),
        entries: coordinator.entries.length,
        atWork: coordinator.atWork,
      });
      await sleep(5000);
    }
    const last = samples.at(-1);
    console.log(`      coordinator over the minute: ${JSON.stringify(samples)}`);
    const coordinator = await must('coordinator', { projectId: project.id });
    console.log(
      `      its record: ${JSON.stringify(coordinator.entries.map((entry) => entry.kind ?? entry.text ?? entry))}`,
    );
    const conversations = await must('list_conversations', { projectId: project.id });
    const turns = conversations.flatMap((conversation) => conversation.turns);
    console.log(`      turns: ${JSON.stringify(turns.map((turn) => [turn.state, turn.failure]))}`);
    const secondHalf = samples.slice(Math.floor(samples.length / 2));
    check(
      'the coordinator started the task once, and not again while the minute lasted',
      last.conversations === 1 &&
        last.turns === 1 &&
        secondHalf.every((sample) => sample.entries === last.entries && sample.turns === 1),
      JSON.stringify(samples),
    );
    check('nobody is at work at the end of the minute', last.atWork === 0, JSON.stringify(last));
    // On Linux Teamwright does start a turn on the stand-in, which fails at
    // once; on Windows the start is refused before that.
    check(
      'the failed turn says why, in the words of the program',
      turns[0]?.state === 'failed' && (turns[0].failure ?? '').includes('stand-in'),
      JSON.stringify(turns[0]?.failure),
    );
    check(
      'the task waits for the manager and the coordinator says so',
      coordinator.waiting.some((waiting) => waiting.taskId === job.id && waiting.words.length > 0),
      JSON.stringify(coordinator.waiting),
    );
    await window.reload();
    await openProject('Smart sensor node');
    await openTab('Team');
    await window.waitForText(made.specialist.name);
    await picture('coordinator');
  });

  await station('earlier conversations', async () => {
    const project = made.projects.hardware;
    const folders = await must('find_earlier_conversations', { projectId: project.id });
    const count = folders.reduce((sum, folder) => sum + folder.conversations.length, 0);
    console.log(`      found ${count} earlier conversation(s) in ${folders.length} folder(s)`);
    check('the earlier conversations of this computer are listed', Array.isArray(folders));
    await window.reload();
    await openProject('Smart sensor node');
    await openTab('Advisor');
    await window.clickText('Import earlier conversations', 'button');
    await sleep(1500);
    const shown = await shownText();
    check(
      'the list of earlier conversations opens in the Advisor',
      /earlier conversations/i.test(shown),
      'the dialog did not open',
    );
    // This picture shows the conversations of this computer: keep it out of
    // the repository.
    await picture('earlier-conversations-private');
    await window.pressEscape();
  });

  await station('a digest of an earlier conversation', async () => {
    const project = made.projects.hardware;
    const folders = await must('find_earlier_conversations', { projectId: project.id });
    const smallest = folders
      .flatMap((folder) => folder.conversations)
      .sort((one, other) => one.sizeBytes - other.sizeBytes)[0];
    if (!smallest) {
      console.log('      no earlier conversation on this computer: the digest was not tried');
      return;
    }
    // The text of the digest stays in the temporary database of this run
    // and is never printed.
    const draft = await must('draft_digest', { projectId: project.id, source: smallest.source });
    const kept = await must('keep_digest', { projectId: project.id, digest: draft });
    const digests = await must('list_digests', { projectId: project.id });
    check(
      'a digest is drafted from an earlier conversation and kept with the project',
      typeof draft.text === 'string' && digests.length === 1 && digests[0].id === kept.id,
      `${digests.length} digest(s)`,
    );
    made.digest = kept.id;
  });

  await station('the rest of the commands of tonight', async () => {
    const project = made.projects.hardware;
    const views = await must('list_views', { projectId: project.id });
    const board = views.find((view) => view.kind === 'board');
    const byTag = await must('set_view_filter', {
      viewId: board.id,
      filter: { assignee: { kind: 'anyone' }, hidesFinished: false, tagId: made.tag.id },
    });
    const byChoice = await must('set_view_filter', {
      viewId: made.views.byAssignee.id,
      filter: {
        assignee: { kind: 'anyone' },
        hidesFinished: false,
        fieldFilter: {
          kind: 'choice',
          fieldId: made.fields.stage.id,
          choiceId: made.fields.stage.choices[0].id,
        },
      },
    });
    check(
      'a view is filtered by a tag, and another by a choice of a field',
      byTag.filter.tagId === made.tag.id &&
        byChoice.filter.fieldFilter?.fieldId === made.fields.stage.id,
      JSON.stringify([byTag.filter, byChoice.filter]),
    );
    await must('set_view_filter', {
      viewId: board.id,
      filter: { assignee: { kind: 'anyone' }, hidesFinished: false, tagId: null },
    });
    const gantt = views.find((view) => view.kind === 'gantt');
    const ganttGrouped = await window.command('set_view_grouping', {
      viewId: gantt.id,
      grouping: 'tag',
    });
    check(
      'a Gantt chart is not grouped by anything but status',
      ganttGrouped.refused !== undefined,
      JSON.stringify(ganttGrouped),
    );
    const noLimit = await window.command('set_status_limit', {
      projectId: project.id,
      statusId: project.statuses[1].id,
      limit: 0,
    });
    check('a limit of no tasks is refused', noLimit.refused?.code === 'rule_broken');
    const tooMany = await window.command('set_coordinator_limit', {
      projectId: project.id,
      limit: 99,
    });
    check(
      'the coordinator does not let 99 specialists work at once',
      tooMany.refused !== undefined,
    );
    const workbook = join(dataDirectory, 'project.xlsx');
    await must('export_workbook', { projectId: project.id, path: workbook });
    check('the workbook, with tags and fields, is written', existsSync(workbook));
    for (const [name, args] of [
      ['plan_limit', {}],
      ['project_usage', { projectId: project.id }],
      ['team_talk', { projectId: project.id }],
      ['project_memory', { projectId: project.id }],
      ['permission_requests', { projectId: project.id }],
      ['open_permission_requests', {}],
      ['budget_questions', {}],
      ['sprint_forecast', { projectId: made.projects.scrum.id }],
      ['list_project_history', { projectId: project.id, limit: 10, before: null }],
    ]) {
      const answer = await window.command(name, args);
      check(`${name} answers`, 'ok' in answer, JSON.stringify(answer));
    }
  });

  await station('computers: theme and version', async () => {
    const chosen = await must('choose_theme', { name: 'paper' });
    check('a theme is chosen', chosen === 'paper' && (await must('theme')) === 'paper');
    const before = await must('update_status');
    const looked = await must('look_for_update_now');
    console.log(
      `      version ${looked.thisVersion}; last look ${looked.lastLook}; failed ${looked.lastLookFailed}; newer ${JSON.stringify(looked.newer)}`,
    );
    check(
      'Look now asks GitHub once and remembers when',
      before.lastLook === null && looked.lastLook !== null,
      JSON.stringify(looked),
    );
    await window.reload();
    await window.clickText('Computers', 'button');
    const shown = await window.waitForText('Look now');
    check(
      'the page Computers shows the part Version and the theme',
      shown.includes(looked.thisVersion) && /paper/i.test(shown),
      shown.slice(0, 800),
    );
    await picture('computers');
  });

  // Second run: what was stored is still there.
  await application.stop();
  application = await DesktopApplication.start(executable, dataDirectory, DRIVER_PORT);
  window = await application.openWindow();
  await window.showAtSize(1440, 900);

  await station('after a restart', async () => {
    await window.waitForText('Smart sensor node');
    const projects = await must('list_projects');
    check('the five projects survive a restart', projects.length === 5, String(projects.length));
    const project = projects.find((p) => p.key === made.projects.hardware.key);
    const tasks = await must('list_tasks', { projectId: project.id });
    const board = tasks.find((task) => task.id === made.board.id);
    check(
      'the tag, the checklist and the six values survive',
      board.tagIds.includes(made.tag.id) &&
        board.checklist.filter((item) => item.done).length === 2 &&
        Object.keys(board.fieldValues).length === 6,
      JSON.stringify(board),
    );
    check(
      'the dependency survives',
      tasks.find((task) => task.id === made.firmware.id)?.blockedBy.includes(made.board.id),
    );
    const views = await must('list_views', { projectId: project.id });
    check(
      'the views survive with their grouping and the Actual switch',
      views.find((view) => view.id === made.views.byAssignee.id)?.grouping === 'assignee' &&
        views.find((view) => view.id === made.views.byTag.id)?.grouping === 'tag' &&
        (views.find((view) => view.kind === 'gantt')?.content ?? '').includes('true'),
      JSON.stringify(views.map((view) => [view.name, view.grouping, view.content])),
    );
    const members = await must('list_members', { projectId: project.id });
    check(
      'the ready-made specialist keeps its changed instructions',
      members.find((member) => member.id === made.specialist.id)?.instructions ===
        'Write C for the STM32. Keep every interrupt handler short.',
    );
    const library = await must('project_library', { projectId: project.id });
    check(
      'the library keeps its connector',
      library.connectors.some((connector) => connector.name === 'parts-search'),
    );
    const memory = await must('search_memory', { projectId: project.id, query: 'rail' });
    check(
      'the fact is still in memory',
      memory.some((entry) => entry.id === made.fact.id),
    );
    const coordinator = await must('coordinator', { projectId: project.id });
    check(
      'the coordinator is still on, with its budget',
      coordinator.runs === 'here' && coordinator.budgets.some((budget) => budget.tokens === 200000),
      JSON.stringify(coordinator).slice(0, 300),
    );
    if (made.digest) {
      check(
        'the kept digest survives',
        (await must('list_digests', { projectId: project.id })).some(
          (digest) => digest.id === made.digest,
        ),
      );
    }
    check('the theme is still the one chosen', (await must('theme')) === 'paper');
    check(
      'the last look for a newer version is remembered',
      (await must('update_status')).lastLook !== null,
    );
    check(
      'the saved report is still in the work folder',
      typeof made.reportPath === 'string' && existsSync(made.reportPath),
    );
    await openProject('Smart sensor node');
    await openTab('Board');
    await window.waitForText('Design the board');
    await picture('after-restart');
  });
} catch (failure) {
  check('the smoke test ran to its end', false, failure.message);
} finally {
  await application.stop();
  removeDataDirectory(dataDirectory);
}

console.log(failures === 0 ? '\nAll checks passed.' : `\n${failures} check(s) failed.`);
process.exit(failures === 0 ? 0 : 1);
