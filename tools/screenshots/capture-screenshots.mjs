/**
 * Takes the pictures of the application that the README and the user manual
 * show. Windows only for now.
 *
 * It starts the built application on an empty temporary data folder, takes a
 * picture of the first screen, creates a sample project through the commands
 * of the shell, and takes pictures of the board, of one task, of the list,
 * of the Gantt chart, of the calendar, of the team and of the dialog for
 * the statuses. Every picture is taken twice: in the dark theme and in the
 * light theme.
 *
 * Usage:
 *   node tools/screenshots/capture-screenshots.mjs <path-to-teamwright-desktop.exe> [output-folder]
 *
 * The pictures go to docs/screenshots unless another folder is named.
 */

import { mkdirSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';

import { DesktopApplication, removeDataDirectory } from '../shared/desktop-application.mjs';

const [executable, outputDirectory = 'docs/screenshots'] = process.argv.slice(2);
if (!executable) {
  console.error('usage: node capture-screenshots.mjs <path-to-exe> [output-folder]');
  process.exit(2);
}

/** A port of its own, so that this tool can run next to the smoke test. */
const DEBUGGING_PORT = 9334;

/**
 * The size of every picture: wide enough to show the five statuses of the
 * sample project side by side, with one and a half pixels for each pixel of
 * the style sheet, so that text stays sharp when a picture is enlarged.
 */
const PICTURE = { width: 1712, height: 1000, scale: 1.5 };

const THEMES = ['dark', 'light'];

/** How long the interface gets to finish drawing before a picture is taken. */
const SETTLE_MILLISECONDS = 700;

/** An index beyond the last task of a status puts a task at its end. */
const END_OF_STATUS = 1000;

// -------------------------------------------------------- the sample project

/**
 * The project of the product vision: a small connected sensor with a board,
 * firmware and an enclosure. It is partly done, so every status has something
 * in it. The interface shows the same project on its demo data; see
 * apps/desktop/src/shared/api/demo/sampleWorkspace.ts.
 */
const SAMPLE_PROJECT = {
  name: 'Smart sensor node',
  description:
    'A small connected device that measures air quality and shows it on a screen. ' +
    'Hardware, firmware and an enclosure.',
  workflow: 'engineering',
};

/**
 * The team of the sample project, in the order its members join: the three
 * disciplines a small connected device needs.
 */
const SAMPLE_TEAM = [
  {
    key: 'hardware',
    name: 'Hardware engineer',
    responsibility: 'The parts, the schematic and the board.',
  },
  {
    key: 'firmware',
    name: 'Firmware engineer',
    responsibility: 'Everything that runs on the device.',
  },
  {
    key: 'mechanical',
    name: 'Mechanical engineer',
    responsibility: 'The enclosure, and how the board sits in it.',
  },
];

/**
 * The tasks of the sample project. `waitsFor` holds the numbers of the tasks
 * a task waits for, and `partOf` the number of the task a subtask is a part
 * of. Days are counted from today; negative numbers lie in the past. `owner`
 * is 'manager' or the key of a member of the team; a task without an owner
 * is given to nobody.
 */
const SAMPLE_TASKS = [
  {
    number: 1,
    title: 'Agree on what the device must do',
    status: 'Done',
    description:
      'Temperature, humidity and oxygen. A small screen. Wi-Fi. Low cost and a compact board.',
    owner: 'manager',
    estimateHours: 2,
    startsInDays: -9,
    dueInDays: -8,
  },
  {
    number: 2,
    title: 'Choose the microcontroller',
    status: 'Done',
    owner: 'hardware',
    priority: 'high',
    waitsFor: [1],
    estimateHours: 3,
    startsInDays: -8,
    dueInDays: -6,
  },
  {
    number: 3,
    title: 'Select the sensors and the display',
    status: 'In test',
    owner: 'hardware',
    waitsFor: [1],
    estimateHours: 4,
    startsInDays: -6,
    dueInDays: -2,
  },
  {
    number: 4,
    title: 'Draw the schematic',
    status: 'In progress',
    owner: 'hardware',
    description:
      'Power, microcontroller, sensors on I2C, display on SPI. Keep the board under 40 by 60 mm.',
    priority: 'high',
    waitsFor: [2, 3],
    estimateHours: 8,
    startsInDays: -2,
    dueInDays: 2,
  },
  {
    number: 5,
    title: 'Define the pin map',
    status: 'In revision',
    owner: 'hardware',
    description: 'The firmware cannot start before the pins are fixed.',
    waitsFor: [2],
    estimateHours: 2,
    startsInDays: -3,
    dueInDays: 1,
  },
  {
    number: 6,
    title: 'Route the board',
    status: 'To do',
    owner: 'hardware',
    waitsFor: [4],
    estimateHours: 10,
    startsInDays: 2,
    dueInDays: 6,
  },
  {
    number: 7,
    title: 'Write the Wi-Fi setup',
    status: 'To do',
    owner: 'firmware',
    waitsFor: [5],
    estimateHours: 6,
    startsInDays: 1,
    dueInDays: 5,
  },
  {
    number: 8,
    title: 'Write the sensor drivers',
    status: 'To do',
    owner: 'firmware',
    waitsFor: [5],
    estimateHours: 8,
    startsInDays: 1,
    dueInDays: 7,
  },
  {
    number: 9,
    title: 'Design the screens of the display',
    status: 'To do',
    owner: 'firmware',
    priority: 'low',
    estimateHours: 5,
    dueInDays: 9,
  },
  {
    number: 10,
    title: 'Design the enclosure',
    status: 'To do',
    owner: 'mechanical',
    waitsFor: [6],
    estimateHours: 8,
    startsInDays: 6,
    dueInDays: 11,
  },
  {
    number: 11,
    title: 'Order the parts for three prototypes',
    status: 'To do',
    priority: 'urgent',
    owner: 'manager',
    waitsFor: [4],
    estimateHours: 1,
    dueInDays: 3,
  },
  {
    number: 12,
    title: 'Assemble and test the prototype',
    status: 'To do',
    waitsFor: [6, 8, 10, 11],
    estimateHours: 6,
    startsInDays: 11,
    dueInDays: 14,
  },
  // The parts of two of the tasks above. Each part has a status of its own.
  {
    number: 13,
    title: 'Power supply and battery charger',
    status: 'Done',
    owner: 'hardware',
    partOf: 4,
    estimateHours: 2,
    startsInDays: -2,
    dueInDays: -1,
  },
  {
    number: 14,
    title: 'Microcontroller and its clock',
    status: 'In progress',
    owner: 'hardware',
    partOf: 4,
    estimateHours: 3,
    startsInDays: -1,
    dueInDays: 1,
  },
  {
    number: 15,
    title: 'Connectors for the sensors and the display',
    status: 'To do',
    owner: 'hardware',
    partOf: 4,
    estimateHours: 3,
    dueInDays: 2,
  },
  {
    number: 16,
    title: 'Driver for temperature and humidity',
    status: 'To do',
    owner: 'firmware',
    partOf: 8,
    estimateHours: 3,
    dueInDays: 4,
  },
  {
    number: 17,
    title: 'Driver for the oxygen sensor',
    status: 'To do',
    owner: 'firmware',
    partOf: 8,
    estimateHours: 5,
    dueInDays: 7,
  },
];

/** The task that is opened for the picture of a task. Its subtasks are shown unfolded. */
const SHOWN_TASK = 'Draw the schematic';

/**
 * The areas of the sample project and the tasks in them, by their numbers.
 * The first is the area every project starts with, under another name; a
 * subtask is in the area of its task. Every task that is not listed is in
 * the first area.
 */
const SAMPLE_AREAS = [
  { name: 'Electronics', tasks: [] },
  { name: 'Firmware', tasks: [7, 8, 9] },
  { name: 'Enclosure', tasks: [10] },
];

/** The area that is opened for its picture. */
const SHOWN_AREA = 'Firmware';

/**
 * The story points of the sample tasks, by their numbers. A task that is
 * not listed has none, as a task has until somebody sizes it.
 */
const SAMPLE_POINTS = [
  [1, 2],
  [2, 3],
  [3, 5],
  [4, 8],
  [5, 2],
  [6, 8],
  [7, 3],
  [8, 5],
  [9, 2],
  [10, 5],
  [11, 1],
];

/**
 * The sprints of the sample project, in the order they are planned: one
 * that is over, one that runs, and one that is to come. The tasks are named
 * by their numbers. Every other task stays in the backlog.
 */
const SAMPLE_SPRINTS = {
  over: {
    tasks: [1, 2, 3],
    goal: 'Know what to build, and with which parts',
    startsInDays: -11,
    endsInDays: -5,
  },
  running: {
    tasks: [4, 5, 11],
    goal: 'A schematic that can be routed',
    startsInDays: -4,
    endsInDays: 9,
  },
  toCome: { tasks: [6, 7, 8], goal: 'Firmware that talks to the sensors' },
};

/** The calendar date that lies `days` after today, in the form year-month-day. */
function dateInDays(days) {
  if (days === undefined) {
    return null;
  }
  const day = new Date();
  day.setDate(day.getDate() + days);
  const month = String(day.getMonth() + 1).padStart(2, '0');
  const dayOfMonth = String(day.getDate()).padStart(2, '0');
  return `${day.getFullYear()}-${month}-${dayOfMonth}`;
}

/** Calls a command of the shell and returns its value. A refusal ends the tool. */
async function run(window, name, args) {
  const answer = await window.command(name, args);
  if (answer.refused) {
    throw new Error(`${name} was refused: ${JSON.stringify(answer.refused)}`);
  }
  return answer.ok;
}

/**
 * Creates the sample project: its team, its tasks, what they wait for, and
 * where they stand. Returns the project and the identifier of the task that
 * is shown with its subtasks.
 */
async function createSampleProject(window) {
  const project = await run(window, 'create_project', { input: SAMPLE_PROJECT });

  const memberIds = new Map();
  for (const sample of SAMPLE_TEAM) {
    const member = await run(window, 'add_member', {
      input: {
        projectId: project.id,
        name: sample.name,
        kind: 'agent',
        responsibility: sample.responsibility,
      },
    });
    memberIds.set(sample.key, member.id);
  }
  /** Who a sample task is given to, in the form the commands expect. */
  const assigneeOf = (sample) => {
    if (sample.owner === undefined) {
      return null;
    }
    return sample.owner === 'manager'
      ? { kind: 'manager' }
      : { kind: 'member', memberId: memberIds.get(sample.owner) };
  };

  const taskIds = new Map();
  for (const sample of SAMPLE_TASKS) {
    const task = await run(window, 'create_task', {
      input: {
        projectId: project.id,
        title: sample.title,
        description: sample.description ?? '',
        priority: sample.priority ?? 'normal',
        assignee: assigneeOf(sample),
        estimateMinutes: sample.estimateHours * 60,
        startOn: dateInDays(sample.startsInDays),
        dueOn: dateInDays(sample.dueInDays),
        // A task is listed after the task it is a part of, so that one exists.
        ...(sample.partOf === undefined ? {} : { parentId: taskIds.get(sample.partOf) }),
      },
    });
    taskIds.set(sample.number, task.id);
  }

  for (const sample of SAMPLE_TASKS) {
    for (const blocker of sample.waitsFor ?? []) {
      await run(window, 'add_task_dependency', {
        taskId: taskIds.get(sample.number),
        blockerId: taskIds.get(blocker),
      });
    }
  }

  // A new task starts in the first status. The others are moved to theirs.
  const firstStatus = project.statuses[0].name;
  for (const sample of SAMPLE_TASKS.filter((task) => task.status !== firstStatus)) {
    const status = project.statuses.find((entry) => entry.name === sample.status);
    await run(window, 'move_task', {
      taskId: taskIds.get(sample.number),
      statusId: status.id,
      index: END_OF_STATUS,
    });
  }

  await planSampleSprints(window, project.id, taskIds);
  await cutIntoSampleAreas(window, project.id, taskIds);

  const shownTask = SAMPLE_TASKS.find((sample) => sample.title === SHOWN_TASK);
  return { project, shownTaskId: taskIds.get(shownTask.number) };
}

/**
 * Cuts the sample project into its areas: the one it started with gets its
 * name, the others are added, and the tasks are moved into theirs.
 */
async function cutIntoSampleAreas(window, projectId, taskIds) {
  const [first, ...others] = SAMPLE_AREAS;
  const [starting] = await run(window, 'list_areas', { projectId });
  await run(window, 'rename_area', { areaId: starting.id, name: first.name });
  for (const sample of others) {
    const area = await run(window, 'add_area', { projectId, name: sample.name });
    for (const number of sample.tasks) {
      await run(window, 'move_task_to_area', { taskId: taskIds.get(number), areaId: area.id });
    }
  }
}

/**
 * Plans the sample project in sprints, the way a manager would have: the
 * first sprint was started and completed, and what it did not finish went
 * on into the second, which runs. A third is planned.
 */
async function planSampleSprints(window, projectId, taskIds) {
  for (const [number, points] of SAMPLE_POINTS) {
    await run(window, 'set_task_points', { taskId: taskIds.get(number), points });
  }
  /** Plans a sprint with the tasks of a sample sprint in it. */
  const plan = async (sample) => {
    const sprint = await run(window, 'create_sprint', { input: { projectId } });
    for (const number of sample.tasks) {
      await run(window, 'plan_task', { taskId: taskIds.get(number), sprintId: sprint.id });
    }
    return sprint;
  };
  /** Starts a sprint with the goal and the dates of a sample sprint. */
  const start = (sprint, sample) =>
    run(window, 'start_sprint', {
      sprintId: sprint.id,
      input: {
        name: sprint.name,
        goal: sample.goal,
        startOn: dateInDays(sample.startsInDays),
        endOn: dateInDays(sample.endsInDays),
      },
    });

  const over = await plan(SAMPLE_SPRINTS.over);
  await start(over, SAMPLE_SPRINTS.over);
  const running = await plan(SAMPLE_SPRINTS.running);
  await run(window, 'complete_sprint', {
    sprintId: over.id,
    input: { moveUnfinishedTo: running.id },
  });
  await start(running, SAMPLE_SPRINTS.running);
  const toCome = await plan(SAMPLE_SPRINTS.toCome);
  await run(window, 'change_sprint', {
    sprintId: toCome.id,
    changes: { name: toCome.name, goal: SAMPLE_SPRINTS.toCome.goal, startOn: null, endOn: null },
  });
}

/**
 * Unfolds the subtasks of a task, the way a click on its fold does. The
 * interface remembers the folds of a project in the storage of the web view
 * and reads them when it loads.
 */
async function unfoldSubtasks(window, projectId, taskId) {
  await window.evaluate(
    `localStorage.setItem(${JSON.stringify(`teamwright.openSubtasks.${projectId}`)},
       ${JSON.stringify(JSON.stringify([taskId]))})`,
  );
}

// -------------------------------------------------------------- the pictures

/** Takes one picture, once the interface has had time to finish drawing. */
async function takePicture(window, name) {
  await sleep(SETTLE_MILLISECONDS);
  await window.screenshot(join(outputDirectory, `${name}.png`));
  console.log(`wrote ${name}.png`);
}

/** JavaScript that finds the tab of a view by its name. */
function tab(name) {
  return `[...document.querySelectorAll('[role="tab"]')]
    .find((entry) => entry.innerText.trim() === ${JSON.stringify(name)})`;
}

/**
 * JavaScript that presses a button or an entry of a menu, found by its
 * label. A menu opens when the pointer goes down on its button, so a plain
 * click is not enough.
 */
function press(label) {
  return `(() => {
    const target = [...document.querySelectorAll('button, [role="menuitem"]')].find(
      (entry) =>
        entry.getAttribute('aria-label') === ${JSON.stringify(label)} ||
        entry.innerText.trim() === ${JSON.stringify(label)},
    );
    for (const type of ['pointerdown', 'pointerup']) {
      target.dispatchEvent(new PointerEvent(type, { bubbles: true, button: 0, pointerType: 'mouse' }));
    }
    target.click();
  })()`;
}

/**
 * JavaScript that finds the line of a project in the navigation at the
 * side. Pressing it shows every area of the project again.
 */
function projectInNavigation(name) {
  return `[...document.querySelectorAll('nav button, aside button')]
    .find((entry) => entry.innerText.trim().startsWith(${JSON.stringify(name)}))`;
}

/**
 * JavaScript that finds the card of a task on the board by its title: the
 * part of the card that opens the task when it is pressed.
 */
function card(title) {
  return `[...document.querySelectorAll('li [role="button"]')]
    .find((entry) => entry.innerText.includes(${JSON.stringify(title)}))`;
}

/**
 * Takes the pictures of the sample project in one theme: the board of the
 * sprint and of all tasks, one area, a task, the backlog, the list, the
 * Gantt chart, the calendar, the dashboard, the team, the office, and the
 * statuses.
 */
async function takeProjectPictures(window, theme) {
  await window.showInTheme(theme);

  // While a sprint runs, the board opens on the tasks of that sprint.
  await window.click(tab('Board'));
  await window.waitForText(SHOWN_TASK);
  await takePicture(window, `sprint-board-${theme}`);
  await window.evaluate(press('All tasks'));
  await window.waitForText(SAMPLE_TASKS[0].title);
  await takePicture(window, `board-${theme}`);

  // One area of the project, and back to all of them: the name of the
  // project in the header leads back.
  await window.evaluate(press(SHOWN_AREA));
  await window.waitForText(`tasks in ${SHOWN_AREA}`);
  await window.evaluate(press('All tasks'));
  await takePicture(window, `area-${theme}`);
  await window.click(projectInNavigation(SAMPLE_PROJECT.name));
  await window.waitForText(SHOWN_TASK);
  await window.evaluate(press('All tasks'));
  await window.waitForText(SAMPLE_TASKS[0].title);

  await window.click(card(SHOWN_TASK));
  await window.waitForText('WAITS FOR');
  await takePicture(window, `task-${theme}`);
  await window.pressEscape();

  await window.click(tab('Backlog'));
  await window.waitForText('COMPLETED SPRINTS');
  await takePicture(window, `backlog-${theme}`);

  await window.click(tab('List'));
  await window.waitForText('ASSIGNEE');
  await takePicture(window, `list-${theme}`);

  await window.click(tab('Gantt'));
  await window.waitForText('Today');
  await takePicture(window, `gantt-${theme}`);

  await window.click(tab('Calendar'));
  await window.waitForText('MON');
  await takePicture(window, `calendar-${theme}`);

  await window.click(tab('Dashboard'));
  await window.waitForText('WHERE THE PROJECT STANDS');
  await takePicture(window, `dashboard-${theme}`);

  await window.click(tab('Team'));
  await window.waitForText('MEMBER');
  await takePicture(window, `team-${theme}`);

  // Nobody works in the sample project: the stand-in for Claude Code runs
  // no turn. The office with a lamp lit is a picture of the test that lets
  // a real specialist work.
  await window.click(tab('Office'));
  await window.waitForText('PLAN OF THE OFFICE');
  await takePicture(window, `office-${theme}`);

  await window.click(tab('Board'));
  await window.evaluate(press('Project'));
  await window.waitForText('Statuses');
  await window.evaluate(press('Statuses'));
  await window.waitForText('MEANS');
  await takePicture(window, `statuses-${theme}`);
  await window.pressEscape();
}

const dataDirectory = mkdtempSync(join(tmpdir(), 'teamwright-screenshots-'));
const application = DesktopApplication.start(executable, dataDirectory, DEBUGGING_PORT);

try {
  mkdirSync(outputDirectory, { recursive: true });
  const window = await application.openWindow();
  await window.showAtSize(PICTURE.width, PICTURE.height, PICTURE.scale);

  await window.waitForText('No projects yet');
  for (const theme of THEMES) {
    await window.showInTheme(theme);
    await takePicture(window, `first-run-${theme}`);
  }

  const { project, shownTaskId } = await createSampleProject(window);
  await unfoldSubtasks(window, project.id, shownTaskId);
  await window.reload();
  await window.waitForText(SHOWN_TASK);
  for (const theme of THEMES) {
    await takeProjectPictures(window, theme);
  }
  window.close();
} catch (failure) {
  console.error(`The pictures were not taken: ${failure.message}`);
  process.exitCode = 1;
} finally {
  await application.stop();
  removeDataDirectory(dataDirectory);
}
