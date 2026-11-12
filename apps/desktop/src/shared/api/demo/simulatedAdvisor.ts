import type { InterviewQuestion, Proposal } from '../types';
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
      waitsFor: [],
    },
    {
      title: 'Choose the microcontroller and the sensors',
      description:
        'A list of the main parts, with the reason for each and what it costs. Done when you have approved it.',
      owner: { kind: 'member', name: HARDWARE },
      estimateMinutes: 240,
      priority: 'high',
      waitsFor: [0],
    },
    {
      title: 'Define the pin map',
      description:
        'Which pin goes to which part, with the addresses on the bus. Done when the firmware and the board can both work from it.',
      owner: { kind: 'member', name: HARDWARE },
      estimateMinutes: 120,
      priority: 'normal',
      waitsFor: [1],
    },
    {
      title: 'Draw the schematic',
      description: 'The schematic of the whole board. Done when it passes its electrical check.',
      owner: { kind: 'member', name: HARDWARE },
      estimateMinutes: 480,
      priority: 'normal',
      waitsFor: [2],
    },
    {
      title: 'Route the board',
      description:
        'The layout of the board, with the files a manufacturer needs. Done when it passes its design check.',
      owner: { kind: 'member', name: HARDWARE },
      estimateMinutes: 600,
      priority: 'normal',
      waitsFor: [3],
    },
    {
      title: 'Define the outline of the board',
      description:
        'The size of the board, its holes, and where the connectors and the screen sit. Done when the enclosure can be designed from it.',
      owner: { kind: 'member', name: HARDWARE },
      estimateMinutes: 60,
      priority: 'normal',
      waitsFor: [4],
    },
    {
      title: 'Write the sensor drivers',
      description: 'Code that reads every sensor. Done when it builds and its tests pass.',
      owner: { kind: 'member', name: FIRMWARE },
      estimateMinutes: 480,
      priority: 'normal',
      waitsFor: [2],
    },
    {
      title: 'Write the screens of the display',
      description: 'What the screen shows, and how one gets from one view to the next.',
      owner: { kind: 'member', name: FIRMWARE },
      estimateMinutes: 300,
      priority: 'normal',
      waitsFor: [6],
    },
    {
      title: 'Write the Wi-Fi setup',
      description: 'How the device joins a network and sends what it measures.',
      owner: { kind: 'member', name: FIRMWARE },
      estimateMinutes: 360,
      priority: 'normal',
      waitsFor: [6],
    },
    {
      title: 'Design the enclosure',
      description:
        'The enclosure as files that can be printed. Done when the board, the screen and the sensors fit.',
      owner: { kind: 'member', name: MECHANICAL },
      estimateMinutes: 480,
      priority: 'normal',
      waitsFor: [5],
    },
    {
      title: 'Order the parts for three prototypes',
      description: 'The boards and the parts for three devices, ordered.',
      owner: { kind: 'manager' },
      estimateMinutes: 60,
      priority: 'high',
      waitsFor: [4],
    },
    {
      title: 'Assemble and test the prototype',
      description: 'One device, assembled, running the firmware, in its enclosure.',
      owner: { kind: 'manager' },
      estimateMinutes: 360,
      priority: 'normal',
      waitsFor: [7, 8, 9, 10],
    },
  ],
};

/** The script of the Advisor that reads what the manager wants: it asks. */
export function scriptForBrief(): Script {
  return {
    acts: [
      { after: 0, act: { kind: 'note', message: WHAT_THIS_IS } },
      { after: 700, act: { kind: 'say', text: 'I am reading what you wrote.' } },
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
    interviews: structuredClone(QUESTIONS),
  };
}

/** The script of the Advisor that has the answers: it proposes a team and a plan. */
export function scriptForAnswers(): Script {
  return {
    acts: [
      { after: 0, act: { kind: 'note', message: WHAT_THIS_IS } },
      { after: 700, act: { kind: 'say', text: 'Thank you. That is what I needed.' } },
      {
        after: 1200,
        act: {
          kind: 'say',
          text: 'Three disciplines, so three specialists. The work that needs hands is yours.',
        },
      },
    ],
    endsAfter: 900,
    answer:
      'I propose a team of three and twelve tasks. Ordering the parts, assembling and testing are yours. Change what does not fit, and approve it when it does.',
    tokens: { input: 30, output: 2240, cacheRead: 16_700, cacheWritten: 2600 },
    proposes: structuredClone(PROPOSAL),
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
      })),
  };
}
