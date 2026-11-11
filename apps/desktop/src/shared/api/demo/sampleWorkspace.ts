import { evenlySpacedRanks } from '@/shared/lib/rank';

import type { Assignee, CalendarDate, Id, Priority, Project, Status, Task } from '../types';
import type { DemoWorkspace } from './demoBackend';

/** The statuses of the sample project, by a short name used in this file. */
const STATUSES = {
  todo: { id: 'demo-status-todo', name: 'To do', category: 'todo', color: 'gray' },
  progress: { id: 'demo-status-progress', name: 'In progress', category: 'active', color: 'blue' },
  revision: {
    id: 'demo-status-revision',
    name: 'In revision',
    category: 'active',
    color: 'purple',
  },
  test: { id: 'demo-status-test', name: 'In test', category: 'review', color: 'amber' },
  done: { id: 'demo-status-done', name: 'Done', category: 'done', color: 'green' },
} as const satisfies Record<string, Status>;

type StatusName = keyof typeof STATUSES;

/** One task of the sample project, written the way a person would list it. */
interface SampleTask {
  number: number;
  title: string;
  status: StatusName;
  description?: string;
  priority?: Priority;
  /** Tasks given to the manager. All others are unassigned until a team exists. */
  forManager?: boolean;
  /** The numbers of the tasks this one waits for. */
  waitsFor?: number[];
  estimateHours?: number;
  /** Days from today. Negative numbers lie in the past. */
  startsInDays?: number;
  dueInDays?: number;
}

/**
 * The project from the product vision: a small connected sensor with a board,
 * firmware and an enclosure. It is partly done, so every status has
 * something in it.
 */
const SAMPLE_TASKS: SampleTask[] = [
  {
    number: 1,
    title: 'Agree on what the device must do',
    status: 'done',
    description:
      'Temperature, humidity and oxygen. A small screen. Wi-Fi. Low cost and a compact board.',
    forManager: true,
    estimateHours: 2,
    startsInDays: -9,
    dueInDays: -8,
  },
  {
    number: 2,
    title: 'Choose the microcontroller',
    status: 'done',
    priority: 'high',
    waitsFor: [1],
    estimateHours: 3,
    startsInDays: -8,
    dueInDays: -6,
  },
  {
    number: 3,
    title: 'Select the sensors and the display',
    status: 'test',
    waitsFor: [1],
    estimateHours: 4,
    startsInDays: -6,
    dueInDays: -2,
  },
  {
    number: 4,
    title: 'Draw the schematic',
    status: 'progress',
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
    status: 'revision',
    description: 'The firmware cannot start before the pins are fixed.',
    waitsFor: [2],
    estimateHours: 2,
    startsInDays: -3,
    dueInDays: 1,
  },
  {
    number: 6,
    title: 'Route the board',
    status: 'todo',
    waitsFor: [4],
    estimateHours: 10,
    startsInDays: 2,
    dueInDays: 6,
  },
  {
    number: 7,
    title: 'Write the Wi-Fi setup',
    status: 'todo',
    waitsFor: [5],
    estimateHours: 6,
    startsInDays: 1,
    dueInDays: 5,
  },
  {
    number: 8,
    title: 'Write the sensor drivers',
    status: 'todo',
    waitsFor: [5],
    estimateHours: 8,
    startsInDays: 1,
    dueInDays: 7,
  },
  {
    number: 9,
    title: 'Design the screens of the display',
    status: 'todo',
    priority: 'low',
    estimateHours: 5,
    dueInDays: 9,
  },
  {
    number: 10,
    title: 'Design the enclosure',
    status: 'todo',
    waitsFor: [6],
    estimateHours: 8,
    startsInDays: 6,
    dueInDays: 11,
  },
  {
    number: 11,
    title: 'Order the parts for three prototypes',
    status: 'todo',
    priority: 'urgent',
    forManager: true,
    waitsFor: [4],
    estimateHours: 1,
    dueInDays: 3,
  },
  {
    number: 12,
    title: 'Assemble and test the prototype',
    status: 'todo',
    waitsFor: [6, 8, 10, 11],
    estimateHours: 6,
    startsInDays: 11,
    dueInDays: 14,
  },
];

const SAMPLE_PROJECT_ID: Id = 'demo-project-sensor';
const MILLISECONDS_PER_DAY = 24 * 60 * 60 * 1000;

/**
 * Builds the workspace the demo starts with. Dates are counted from `today`,
 * so the sample project always looks like it is under way.
 */
export function sampleWorkspace(today: Date = new Date()): DemoWorkspace {
  const createdAt = shiftedByDays(today, -10).toISOString();
  const project: Project = {
    id: SAMPLE_PROJECT_ID,
    key: 'SSN',
    name: 'Smart sensor node',
    description:
      'A small connected device that measures air quality and shows it on a screen. ' +
      'Hardware, firmware and an enclosure.',
    statuses: Object.values(STATUSES),
    createdAt,
    archivedAt: null,
  };

  const tasks = SAMPLE_TASKS.map((sample): Task => {
    const status = STATUSES[sample.status];
    const assignee: Assignee | null = sample.forManager ? { kind: 'manager' } : null;
    const isDone = status.category === 'done';
    return {
      id: taskId(sample.number),
      projectId: project.id,
      number: sample.number,
      title: sample.title,
      description: sample.description ?? '',
      statusId: status.id,
      priority: sample.priority ?? 'normal',
      assignee,
      parentId: null,
      blockedBy: (sample.waitsFor ?? []).map(taskId),
      estimateMinutes: sample.estimateHours === undefined ? null : sample.estimateHours * 60,
      startOn: dateInDays(today, sample.startsInDays),
      dueOn: dateInDays(today, sample.dueInDays),
      rank: rankWithinStatus(sample),
      createdAt,
      updatedAt: createdAt,
      completedAt: isDone ? shiftedByDays(today, sample.dueInDays ?? 0).toISOString() : null,
    };
  });

  return { projects: [project], tasks };
}

function taskId(number: number): Id {
  return `demo-task-${number.toString()}`;
}

/** Tasks of the same status are ranked in the order they are listed above. */
function rankWithinStatus(sample: SampleTask): number {
  const sameStatus = SAMPLE_TASKS.filter((other) => other.status === sample.status);
  const ranks = evenlySpacedRanks(sameStatus.length);
  return ranks[sameStatus.indexOf(sample)] ?? 0;
}

function shiftedByDays(day: Date, days: number): Date {
  return new Date(day.getTime() + days * MILLISECONDS_PER_DAY);
}

/** The calendar date that lies `days` after `today`, in the form year-month-day. */
function dateInDays(today: Date, days: number | undefined): CalendarDate | null {
  if (days === undefined) {
    return null;
  }
  return shiftedByDays(today, days).toISOString().slice(0, 'YYYY-MM-DD'.length);
}
