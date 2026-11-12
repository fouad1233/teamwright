/**
 * Takes the pictures of the application that the README and the user manual
 * show. Windows only for now.
 *
 * It starts the built application on an empty temporary data folder, takes a
 * picture of the first screen, creates a sample project through the commands
 * of the shell, and takes pictures of the board, of one task, of the list and
 * of the team. Every picture is taken twice: in the dark theme and in the
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
 * a task waits for. Days are counted from today; negative numbers lie in the
 * past. `owner` is 'manager' or the key of a member of the team; a task
 * without an owner is given to nobody.
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
];

/** The task that is opened for the picture of a task. */
const SHOWN_TASK = 'Draw the schematic';

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

/** Creates the sample project: its team, its tasks, what they wait for, and where they stand. */
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

/** JavaScript that finds the card of a task on the board by its title. */
function card(title) {
  return `[...document.querySelectorAll('li')]
    .find((entry) => entry.innerText.includes(${JSON.stringify(title)}))`;
}

/** Takes the pictures of the sample project in one theme: the board, a task, the list, the team. */
async function takeProjectPictures(window, theme) {
  await window.showInTheme(theme);

  await window.click(tab('Board'));
  await window.waitForText(SHOWN_TASK);
  await takePicture(window, `board-${theme}`);

  await window.click(card(SHOWN_TASK));
  await window.waitForText('WAITS FOR');
  await takePicture(window, `task-${theme}`);
  await window.pressEscape();

  await window.click(tab('List'));
  await window.waitForText('ASSIGNEE');
  await takePicture(window, `list-${theme}`);

  await window.click(tab('Team'));
  await window.waitForText('MEMBER');
  await takePicture(window, `team-${theme}`);
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

  await createSampleProject(window);
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
