import { evenlySpacedRanks } from '@/shared/lib/rank';

import type {
  Area,
  Assignee,
  CalendarDate,
  Id,
  Member,
  MemberProfile,
  Priority,
  Project,
  Sprint,
  SprintState,
  SprintSummary,
  Status,
  Task,
} from '../types';
import type { DemoWorkspace } from './demoBackend';
import { startingViews } from './views';

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

/**
 * The team of the sample project, by a short name used in this file: the
 * three disciplines a small connected device needs.
 */
const TEAM = {
  hardware: {
    id: 'demo-member-hardware',
    name: 'Hardware engineer',
    responsibility: 'The parts, the schematic and the board.',
    color: 'teal',
    model: 'opus',
    effort: 'high',
  },
  firmware: {
    id: 'demo-member-firmware',
    name: 'Firmware engineer',
    responsibility: 'Everything that runs on the device.',
    color: 'blue',
    model: 'sonnet',
  },
  mechanical: {
    id: 'demo-member-mechanical',
    name: 'Mechanical engineer',
    responsibility: 'The enclosure, and how the board sits in it.',
    color: 'purple',
  },
} as const satisfies Record<
  string,
  Pick<Member, 'id' | 'name' | 'responsibility' | 'color'> & Partial<MemberProfile>
>;

type TeamMemberName = keyof typeof TEAM;

/** One sprint of the sample project, with its days counted from today. */
interface SampleSprint {
  id: Id;
  name: string;
  goal: string;
  state: SprintState;
  /** Days from today. Negative numbers lie in the past. */
  createdInDays: number;
  startsInDays?: number;
  endsInDays?: number;
  /** When the sprint was completed, if it was. */
  completedInDays?: number;
  summary?: SprintSummary;
}

/**
 * The sprints of the sample project: one that is over, one that runs with
 * some of its work done, and one that is planned to come next.
 */
const SPRINTS = {
  first: {
    id: 'demo-sprint-1',
    name: 'Sprint 1',
    goal: 'Agree on the device and choose its heart',
    state: 'completed',
    createdInDays: -10,
    startsInDays: -10,
    endsInDays: -5,
    completedInDays: -5,
    // The sensors were not chosen in time and went on into the next sprint.
    summary: { unit: 'points', committed: 10, completed: 5 },
  },
  current: {
    id: 'demo-sprint-2',
    name: 'Sprint 2',
    goal: 'A schematic that can be routed',
    state: 'active',
    createdInDays: -6,
    startsInDays: -4,
    endsInDays: 9,
  },
  next: {
    id: 'demo-sprint-3',
    name: 'Sprint 3',
    goal: 'Firmware that talks to the sensors',
    state: 'planned',
    createdInDays: -1,
  },
} as const satisfies Record<string, SampleSprint>;

type SprintName = keyof typeof SPRINTS;

/**
 * The areas of the sample project, in their order: the jobs a small
 * connected device is made of.
 */
const AREAS = {
  electronics: { id: 'demo-area-electronics', name: 'Electronics' },
  firmware: { id: 'demo-area-firmware', name: 'Firmware' },
  enclosure: { id: 'demo-area-enclosure', name: 'Enclosure' },
} as const satisfies Record<string, Pick<Area, 'id' | 'name'>>;

type AreaName = keyof typeof AREAS;

/** One task of the sample project, written the way a person would list it. */
interface SampleTask {
  number: number;
  title: string;
  /** The area the task is in. A subtask is in the area of its task and leaves this out. */
  area?: AreaName;
  status: StatusName;
  /** The sprint the task is planned into. A subtask is in the sprint of its task. */
  sprint?: SprintName;
  points?: number;
  description?: string;
  priority?: Priority;
  /** Who the task is given to. A task without an owner is unassigned. */
  owner?: 'manager' | TeamMemberName;
  /** The number of the task this one is a part of, if it is a subtask. */
  partOf?: number;
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
    area: 'electronics',
    status: 'done',
    sprint: 'first',
    points: 2,
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
    area: 'electronics',
    status: 'done',
    sprint: 'first',
    points: 3,
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
    area: 'electronics',
    status: 'test',
    sprint: 'current',
    points: 5,
    owner: 'hardware',
    waitsFor: [1],
    estimateHours: 4,
    startsInDays: -6,
    dueInDays: -2,
  },
  {
    number: 4,
    title: 'Draw the schematic',
    area: 'electronics',
    status: 'progress',
    sprint: 'current',
    points: 8,
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
    // The pins are where the firmware meets the board; the firmware needs them first.
    area: 'firmware',
    status: 'revision',
    sprint: 'current',
    points: 2,
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
    area: 'electronics',
    status: 'todo',
    sprint: 'next',
    points: 8,
    owner: 'hardware',
    waitsFor: [4],
    estimateHours: 10,
    startsInDays: 2,
    dueInDays: 6,
  },
  {
    number: 7,
    title: 'Write the Wi-Fi setup',
    area: 'firmware',
    status: 'todo',
    sprint: 'next',
    points: 3,
    owner: 'firmware',
    waitsFor: [5],
    estimateHours: 6,
    startsInDays: 1,
    dueInDays: 5,
  },
  {
    number: 8,
    title: 'Write the sensor drivers',
    area: 'firmware',
    status: 'todo',
    sprint: 'next',
    points: 5,
    owner: 'firmware',
    waitsFor: [5],
    estimateHours: 8,
    startsInDays: 1,
    dueInDays: 7,
  },
  {
    number: 9,
    title: 'Design the screens of the display',
    area: 'firmware',
    status: 'todo',
    points: 2,
    owner: 'firmware',
    priority: 'low',
    estimateHours: 5,
    dueInDays: 9,
  },
  {
    number: 10,
    title: 'Design the enclosure',
    area: 'enclosure',
    status: 'todo',
    points: 5,
    owner: 'mechanical',
    waitsFor: [6],
    estimateHours: 8,
    startsInDays: 6,
    dueInDays: 11,
  },
  {
    number: 11,
    title: 'Order the parts for three prototypes',
    area: 'electronics',
    status: 'todo',
    sprint: 'current',
    points: 1,
    priority: 'urgent',
    owner: 'manager',
    waitsFor: [4],
    estimateHours: 1,
    dueInDays: 3,
  },
  {
    number: 12,
    title: 'Assemble and test the prototype',
    area: 'enclosure',
    status: 'todo',
    waitsFor: [6, 8, 10, 11],
    estimateHours: 6,
    startsInDays: 11,
    dueInDays: 14,
  },
  // The parts of two of the tasks above. Each part has a status of its own.
  {
    number: 13,
    title: 'Power supply and battery charger',
    status: 'done',
    owner: 'hardware',
    partOf: 4,
    estimateHours: 2,
    startsInDays: -2,
    dueInDays: -1,
  },
  {
    number: 14,
    title: 'Microcontroller and its clock',
    status: 'progress',
    owner: 'hardware',
    partOf: 4,
    estimateHours: 3,
    startsInDays: -1,
    dueInDays: 1,
  },
  {
    number: 15,
    title: 'Connectors for the sensors and the display',
    status: 'todo',
    owner: 'hardware',
    partOf: 4,
    estimateHours: 3,
    dueInDays: 2,
  },
  {
    number: 16,
    title: 'Driver for temperature and humidity',
    status: 'todo',
    owner: 'firmware',
    partOf: 8,
    estimateHours: 3,
    dueInDays: 4,
  },
  {
    number: 17,
    title: 'Driver for the oxygen sensor',
    status: 'todo',
    owner: 'firmware',
    partOf: 8,
    estimateHours: 5,
    dueInDays: 7,
  },
  // Work of the running sprint that is already finished, and more work for
  // the backlog.
  {
    number: 18,
    title: 'Check the power budget',
    area: 'electronics',
    status: 'done',
    sprint: 'current',
    points: 3,
    owner: 'hardware',
    estimateHours: 2,
    startsInDays: -2,
    dueInDays: -1,
  },
  {
    number: 19,
    title: 'Set up the firmware project',
    area: 'firmware',
    status: 'done',
    sprint: 'current',
    points: 3,
    owner: 'firmware',
    estimateHours: 2,
    startsInDays: -4,
    dueInDays: -3,
  },
  {
    number: 20,
    title: 'Write the guide for setting up the device',
    area: 'firmware',
    status: 'todo',
    points: 3,
    priority: 'low',
    owner: 'manager',
    estimateHours: 3,
  },
  {
    number: 21,
    title: 'Plan a second batch of boards',
    area: 'electronics',
    status: 'todo',
    estimateHours: 2,
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

  const members = Object.values(TEAM).map((member): Member => ({
    // The way a specialist runs is left to the program, unless the sample
    // says otherwise.
    model: null,
    effort: null,
    ...member,
    projectId: project.id,
    kind: 'agent',
    joinedAt: createdAt,
    updatedAt: createdAt,
  }));

  const tasks = SAMPLE_TASKS.map((sample): Task => {
    const status = STATUSES[sample.status];
    const isDone = status.category === 'done';
    return {
      id: taskId(sample.number),
      projectId: project.id,
      areaId: areaIdOf(sample),
      number: sample.number,
      title: sample.title,
      description: sample.description ?? '',
      statusId: status.id,
      priority: sample.priority ?? 'normal',
      assignee: assigneeOf(sample),
      parentId: sample.partOf === undefined ? null : taskId(sample.partOf),
      blockedBy: (sample.waitsFor ?? []).map(taskId),
      estimateMinutes: sample.estimateHours === undefined ? null : sample.estimateHours * 60,
      sprintId: sprintIdOf(sample),
      points: sample.points ?? null,
      startOn: dateInDays(today, sample.startsInDays),
      dueOn: dateInDays(today, sample.dueInDays),
      rank: rankWithinStatus(sample),
      createdAt,
      updatedAt: createdAt,
      completedAt: isDone ? shiftedByDays(today, sample.dueInDays ?? 0).toISOString() : null,
    };
  });

  const sprints = Object.values(SPRINTS).map((sample: SampleSprint): Sprint => {
    const momentInDays = (days: number | undefined) =>
      days === undefined ? null : shiftedByDays(today, days).toISOString();
    return {
      id: sample.id,
      projectId: project.id,
      name: sample.name,
      goal: sample.goal,
      startOn: dateInDays(today, sample.startsInDays),
      endOn: dateInDays(today, sample.endsInDays),
      state: sample.state,
      createdAt: shiftedByDays(today, sample.createdInDays).toISOString(),
      startedAt: sample.state === 'planned' ? null : momentInDays(sample.startsInDays),
      completedAt: momentInDays(sample.completedInDays),
      summary: sample.summary ?? null,
    };
  });

  // The sample project has the views every new project starts with.
  const views = startingViews(project.id, sampleViewId());

  // The sample project is cut into areas, so that they can be seen.
  const areas = Object.values(AREAS).map((area): Area => ({ ...area, projectId: project.id }));

  return { projects: [project], tasks, members, sprints, views, areas };
}

/** The area of a sample task: its own, or that of the task it is a part of. */
function areaIdOf(sample: SampleTask): Id {
  const parent = SAMPLE_TASKS.find((other) => other.number === sample.partOf);
  const area = (parent ?? sample).area ?? 'electronics';
  return AREAS[area].id;
}

/** The sprint of a sample task: its own, or that of the task it is a part of. */
function sprintIdOf(sample: SampleTask): Id | null {
  const parent = SAMPLE_TASKS.find((other) => other.number === sample.partOf);
  const sprint = parent ? parent.sprint : sample.sprint;
  return sprint === undefined ? null : SPRINTS[sprint].id;
}

function assigneeOf(sample: SampleTask): Assignee | null {
  if (sample.owner === undefined) {
    return null;
  }
  if (sample.owner === 'manager') {
    return { kind: 'manager' };
  }
  return { kind: 'member', memberId: TEAM[sample.owner].id };
}

/** Hands out the identifiers of the views of the sample project, in turn. */
function sampleViewId(): () => Id {
  let handedOut = 0;
  return () => {
    handedOut += 1;
    return `demo-view-${handedOut.toString()}`;
  };
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
