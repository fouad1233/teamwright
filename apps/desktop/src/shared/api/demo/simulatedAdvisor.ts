import type { Area, InterviewQuestion, Project, Proposal, ProposedTask, Task } from '../types';
import { WHAT_THIS_IS, type Script } from './simulatedSpecialist';

/**
 * What the Advisor does in the demo.
 *
 * Like the specialists of the demo, the Advisor of the demo is played from
 * a script. Whatever the manager writes, it asks the same three questions
 * and then proposes the same team and plan: those of the project in the
 * product vision, a small connected device that measures the air. The
 * first step of every script says that this is so.
 */

/** A proposal that the manager has not seen yet. */
type NewProposal = Omit<Proposal, 'approvedAt'>;

const QUESTIONS: InterviewQuestion[] = [
  {
    question: 'What matters most for the first version?',
    reason: 'It decides where the team spends its care, and which tasks come first.',
    options: [
      {
        label: 'A working prototype soon',
        consequence:
          'The plan takes the shortest way to something you can hold and test. Cost and size are looked at afterwards.',
      },
      {
        label: 'The lowest cost for each unit',
        consequence:
          'Parts are chosen by their price first. Finding and testing them takes longer.',
      },
      {
        label: 'The smallest size',
        consequence:
          'The board and the enclosure are designed tightly. That takes more care and more rounds.',
      },
    ],
    recommended: 0,
  },
  {
    question: 'Who does the work that needs hands?',
    reason: 'A specialist writes and designs. It cannot order parts, solder or print.',
    options: [
      {
        label: 'I do it myself',
        consequence: 'Ordering, assembling and testing become tasks of yours in the plan.',
      },
      {
        label: 'Leave it out of the plan',
        consequence: 'The plan ends with files that are ready to build from.',
      },
    ],
    recommended: 0,
  },
  {
    question: 'Is anything already decided that must not change?',
    reason: 'The team builds around what is fixed instead of proposing it again.',
    options: [],
    recommended: null,
  },
];

const HARDWARE = 'Hardware engineer';
const FIRMWARE = 'Firmware engineer';
const MECHANICAL = 'Mechanical engineer';

const PROPOSAL: NewProposal = {
  summary: [
    'A small connected device that measures the air and shows it on a screen.',
    '',
    '- **What is built:** a board of your own with a low-cost microcontroller, sensors for temperature, humidity and air quality, a small screen and Wi-Fi; the firmware for it; a printed enclosure.',
    '- **Done means:** files to order the board from, firmware that builds, an enclosure that prints, and a prototype that you have assembled and tested.',
    '- **Left out:** a phone app, a battery, certification.',
    '- **Yours:** ordering the parts, assembling and testing. A specialist cannot do work that needs hands.',
  ].join('\n'),
  members: [
    {
      name: HARDWARE,
      responsibility: 'The schematic, the parts and the board.',
      reason: 'The device is a board of its own, and every other part builds on it.',
      model: null,
      effort: 'high',
    },
    {
      name: FIRMWARE,
      responsibility: 'The software on the device: sensors, screen and Wi-Fi.',
      reason: 'The device does nothing without its software.',
      model: 'sonnet',
      effort: null,
    },
    {
      name: MECHANICAL,
      responsibility: 'The enclosure, and how the board sits in it.',
      reason: 'The enclosure has to fit the board, the screen and the openings for the sensors.',
      model: 'sonnet',
      effort: null,
    },
  ],
  tasks: [
    {
      title: 'Agree on what the device must do',
      description:
        'A short list of what the device measures, shows and sends, and what it may cost. Done when you have written it down.',
      owner: { kind: 'manager' },
      estimateMinutes: 120,
      priority: 'high',
      partOfTask: null,
      waitsFor: [],
      partOf: null,
    },
    {
      title: 'Choose the microcontroller and the sensors',
      description:
        'A list of the main parts, with the reason for each and what it costs. Done when you have approved it.',
      owner: { kind: 'member', name: HARDWARE },
      estimateMinutes: 240,
      priority: 'high',
      partOfTask: null,
      waitsFor: [0],
      partOf: null,
    },
    {
      title: 'Define the pin map',
      description:
        'Which pin goes to which part, with the addresses on the bus. Done when the firmware and the board can both work from it.',
      owner: { kind: 'member', name: HARDWARE },
      estimateMinutes: 120,
      priority: 'normal',
      partOfTask: null,
      waitsFor: [1],
      partOf: null,
    },
    {
      title: 'Draw the schematic',
      description: 'The schematic of the whole board. Done when it passes its electrical check.',
      owner: { kind: 'member', name: HARDWARE },
      estimateMinutes: 480,
      priority: 'normal',
      partOfTask: null,
      waitsFor: [2],
      partOf: null,
    },
    {
      title: 'Route the board',
      description:
        'The layout of the board, with the files a manufacturer needs. Done when it passes its design check.',
      owner: { kind: 'member', name: HARDWARE },
      estimateMinutes: 600,
      priority: 'normal',
      partOfTask: null,
      waitsFor: [3],
      partOf: null,
    },
    {
      title: 'Define the outline of the board',
      description:
        'The size of the board, its holes, and where the connectors and the screen sit. Done when the enclosure can be designed from it.',
      owner: { kind: 'member', name: HARDWARE },
      estimateMinutes: 60,
      priority: 'normal',
      partOfTask: null,
      waitsFor: [4],
      partOf: null,
    },
    {
      title: 'Write the sensor drivers',
      description: 'Code that reads every sensor. Done when it builds and its tests pass.',
      owner: { kind: 'member', name: FIRMWARE },
      estimateMinutes: 480,
      priority: 'normal',
      partOfTask: null,
      waitsFor: [2],
      partOf: null,
    },
    {
      title: 'Write the screens of the display',
      description: 'What the screen shows, and how one gets from one view to the next.',
      owner: { kind: 'member', name: FIRMWARE },
      estimateMinutes: 300,
      priority: 'normal',
      partOfTask: null,
      waitsFor: [6],
      partOf: null,
    },
    {
      title: 'Write the Wi-Fi setup',
      description: 'How the device joins a network and sends what it measures.',
      owner: { kind: 'member', name: FIRMWARE },
      estimateMinutes: 360,
      priority: 'normal',
      partOfTask: null,
      waitsFor: [6],
      partOf: null,
    },
    {
      title: 'Design the enclosure',
      description:
        'The enclosure as files that can be printed. Done when the board, the screen and the sensors fit.',
      owner: { kind: 'member', name: MECHANICAL },
      estimateMinutes: 480,
      priority: 'normal',
      partOfTask: null,
      waitsFor: [5],
      partOf: null,
    },
    {
      title: 'Order the parts for three prototypes',
      description: 'The boards and the parts for three devices, ordered.',
      owner: { kind: 'manager' },
      estimateMinutes: 60,
      priority: 'high',
      partOfTask: null,
      waitsFor: [4],
      partOf: null,
    },
    {
      title: 'Assemble and test the prototype',
      description:
        'One device, assembled, running the firmware, in its enclosure. Its parts come first; what is left is fitting the board into the enclosure.',
      owner: { kind: 'manager' },
      estimateMinutes: 60,
      priority: 'normal',
      partOfTask: null,
      waitsFor: [7, 8, 9, 10],
      partOf: null,
    },
    {
      title: 'Solder the first board',
      description: 'One board with every part on it, checked for shorts before it is powered.',
      owner: { kind: 'manager' },
      estimateMinutes: 150,
      priority: 'normal',
      partOfTask: null,
      waitsFor: [10],
      partOf: 11,
    },
    {
      title: 'Load the firmware',
      description: 'The firmware runs on the board, and the screen shows the first readings.',
      owner: { kind: 'manager' },
      estimateMinutes: 30,
      priority: 'normal',
      partOfTask: null,
      waitsFor: [12],
      partOf: 11,
    },
    {
      title: 'Compare the readings with a reference meter',
      description:
        'An hour side by side with a meter you trust. Done when the readings agree within the tolerance of the sensors.',
      owner: { kind: 'manager' },
      estimateMinutes: 120,
      priority: 'normal',
      partOfTask: null,
      waitsFor: [13],
      partOf: 11,
    },
  ],
  moves: [],
};

/**
 * Whether the manager asks the Advisor to regroup the tasks of a project:
 * the demo reads only this of what the manager writes.
 */
export function asksToRegroup(text: string): boolean {
  return /\b(regroup|group|module)/i.test(text);
}

/** The most tasks the demo puts into one module when a project has one area. */
const TASKS_IN_A_MODULE = 4;

/**
 * The script of the Advisor that is asked to regroup the tasks of a
 * project into modules, as the founder asked: one new task for each area
 * of the project, named after it, with the tasks of their own of that area
 * moved under it, each with its parts. A project with one area gets a
 * module for every few tasks, in the order of their numbers. It proposes
 * no member and asks nothing first.
 */
export function scriptForRegrouping(
  project: Project,
  areas: readonly Area[],
  tasks: readonly Task[],
): Script {
  const tops = tasks
    .filter((task) => task.parentId === null)
    .sort((first, second) => first.number - second.number);
  const groups: { name: string; tasks: Task[] }[] =
    areas.length > 1
      ? areas.map((area) => ({
          name: area.name,
          tasks: tops.filter((task) => task.areaId === area.id),
        }))
      : Array.from({ length: Math.ceil(tops.length / TASKS_IN_A_MODULE) }, (_, index) => ({
          name: `Part ${(index + 1).toString()}`,
          tasks: tops.slice(index * TASKS_IN_A_MODULE, (index + 1) * TASKS_IN_A_MODULE),
        }));
  const modules = groups.filter((group) => group.tasks.length > 0);
  const proposal: NewProposal = {
    summary: [
      `The tasks of ${project.name} grouped into ${modules.length.toString()} modules, as you asked.`,
      '',
      '- **New:** one task for each module. It starts once its parts are finished.',
      '- **Moved:** every task of its own goes under its module, with its parts. Nothing else of it changes, and a task that waits for another still does.',
      '- **Where they go:** a moved task goes into the area and the sprint of its module.',
    ].join('\n'),
    members: [],
    tasks: modules.map((module, place): ProposedTask => ({
      title: `Module ${place.toString()}: ${module.name}`,
      description: `Holds the tasks of ${module.name}. Done when they are.`,
      owner: { kind: 'manager' },
      estimateMinutes: null,
      priority: 'normal',
      waitsFor: [],
      partOf: null,
      partOfTask: null,
    })),
    moves: modules.flatMap((module, place) =>
      module.tasks.map((task) => ({ taskId: task.id, partOf: place, partOfTask: null })),
    ),
  };
  return {
    acts: [
      { after: 0, act: { kind: 'note', message: WHAT_THIS_IS } },
      { after: 700, act: { kind: 'say', text: 'I am reading the tasks of the project.' } },
      {
        after: 900,
        act: {
          kind: 'say',
          text: 'I group them by the area each one is in, and move each with its parts.',
        },
      },
    ],
    endsAfter: 800,
    answer: `I propose ${modules.length.toString()} modules and move ${proposal.moves.length.toString()} tasks under them. Change where a task goes, or take a move out, and approve it when it fits.`,
    tokens: { input: 20, output: 1400, cacheRead: 15_200, cacheWritten: 3100 },
    contextAdded: 18_000,
    proposes: proposal,
  };
}

/**
 * The script of the Advisor that reads what the manager wants: it asks. It
 * says that it read the digests of earlier conversations, if `digests` were
 * kept with the project.
 */
export function scriptForBrief(digests = 0): Script {
  const imported =
    digests === 0
      ? []
      : [
          {
            after: 600,
            act: {
              kind: 'say' as const,
              text:
                digests === 1
                  ? 'I have read the account of the earlier conversation you imported. I will ask where it may be out of date.'
                  : `I have read the accounts of the ${digests.toString()} earlier conversations you imported. I will ask where they may be out of date.`,
            },
          },
        ];
  return {
    acts: [
      { after: 0, act: { kind: 'note', message: WHAT_THIS_IS } },
      { after: 700, act: { kind: 'say', text: 'I am reading what you wrote.' } },
      ...imported,
      { after: 600, act: { kind: 'use', id: 'look', tool: 'Glob', summary: '*' } },
      { after: 500, act: { kind: 'finish', id: 'look' } },
      {
        after: 800,
        act: {
          kind: 'say',
          text: 'Before I propose a team, there are three things only you can tell me.',
        },
      },
    ],
    endsAfter: 600,
    answer: 'I have asked three questions and wait for your answers.',
    tokens: { input: 24, output: 1180, cacheRead: 14_000, cacheWritten: 8200 },
    contextAdded: 22_900,
    interviews: structuredClone(QUESTIONS),
  };
}

/**
 * The script of the Advisor that has the answers: it proposes a team and a
 * plan. A specialist the team already has is not proposed again. Its tasks
 * are given to the one that is there. `team` holds the names of the members
 * the project has.
 */
export function scriptForAnswers(team: readonly string[] = []): Script {
  const there = new Set(team.map((name) => name.toLowerCase()));
  const proposal = structuredClone(PROPOSAL);
  proposal.members = proposal.members.filter((member) => !there.has(member.name.toLowerCase()));
  const whom = [
    'The team you have covers this. I propose',
    'I propose one more specialist and',
    'I propose two more specialists and',
    'I propose a team of three and',
  ][proposal.members.length];
  return {
    acts: [
      { after: 0, act: { kind: 'note', message: WHAT_THIS_IS } },
      { after: 700, act: { kind: 'say', text: 'Thank you. That is what I needed.' } },
      {
        after: 1200,
        act: {
          kind: 'say',
          text: 'Three disciplines: the board, its software and the enclosure. The work that needs hands is yours.',
        },
      },
    ],
    endsAfter: 900,
    answer: `${whom ?? 'I propose'} twelve tasks, the last of them in three parts. Ordering the parts, assembling and testing are yours. Change what does not fit, and approve it when it does.`,
    tokens: { input: 30, output: 2240, cacheRead: 16_700, cacheWritten: 143_000 },
    // It reads the documents of the project before it proposes, which fills
    // most of its window.
    contextAdded: 141_000,
    proposes: proposal,
  };
}

/**
 * The script of the Advisor that is asked for a change in words. The demo
 * does not read the words. To show how a new proposal takes the place of
 * the one before, it proposes a smaller team: without its last member and
 * without that member's tasks. With nothing to make smaller, it only says
 * that this is the demo.
 */
export function scriptForChange(previous: NewProposal | undefined): Script {
  const smaller = previous && previous.members.length > 1 ? withoutLastMember(previous) : undefined;
  return {
    acts: [
      { after: 0, act: { kind: 'note', message: WHAT_THIS_IS } },
      { after: 800, act: { kind: 'say', text: 'I am reading your message.' } },
    ],
    endsAfter: 900,
    answer: smaller
      ? 'This is the demo, so your words were not read. To show a changed proposal, here is one with a smaller team.'
      : 'This is the demo, so there is nobody to answer. In the desktop program, the Advisor would go on from your message here.',
    tokens: { input: 18, output: 900, cacheRead: 19_300, cacheWritten: 700 },
    contextAdded: 9800,
    proposes: smaller,
  };
}

/**
 * A proposal without its last member. The tasks that were given to that
 * member go with it, and no task waits for a task that is gone.
 */
export function withoutLastMember(proposal: NewProposal): NewProposal {
  const gone = proposal.members.at(-1)?.name.toLowerCase();
  const stays = proposal.tasks.map(
    (task) => !(task.owner?.kind === 'member' && task.owner.name.toLowerCase() === gone),
  );
  // Where each task that stays is in the new list.
  const newPlace = new Map<number, number>();
  for (const [place, keeps] of stays.entries()) {
    if (keeps) {
      newPlace.set(place, newPlace.size);
    }
  }
  return {
    summary: proposal.summary,
    members: structuredClone(proposal.members.slice(0, -1)),
    tasks: proposal.tasks
      .filter((_, place) => stays[place])
      .map((task) => ({
        ...structuredClone(task),
        waitsFor: task.waitsFor.flatMap((place) => {
          const moved = newPlace.get(place);
          return moved === undefined ? [] : [moved];
        }),
        partOf: task.partOf === null ? null : (newPlace.get(task.partOf) ?? null),
      })),
    moves: proposal.moves.map((move) => ({
      ...move,
      partOf: move.partOf === null ? null : (newPlace.get(move.partOf) ?? null),
    })),
  };
}
