import type {
  DecisionRequest,
  InterviewQuestion,
  Member,
  Proposal,
  Task,
  TokenCounts,
} from '../types';

/**
 * What a specialist does in the demo.
 *
 * The demo runs in a browser and has no Claude Code, so nobody really works
 * there. To show how the interface follows a specialist, the demo plays a
 * short script instead: a few steps with pauses between them, and a report
 * at the end. The first step of every script says so.
 */

/** One thing a simulated specialist does. */
export type Act =
  | { kind: 'say'; text: string }
  | { kind: 'note'; message: string }
  /** Begins to use a tool. */
  | { kind: 'use'; id: string; tool: string; summary: string }
  /** Finishes the use of a tool that was begun. */
  | { kind: 'finish'; id: string };

/** An act, and how long after the act before it happens, in milliseconds. */
export interface TimedAct {
  after: number;
  act: Act;
}

/** Everything a simulated turn does, and how it ends. */
export interface Script {
  acts: TimedAct[];
  /** How long after the last act the turn ends, in milliseconds. */
  endsAfter: number;
  /** The report the specialist ends with, or what it says when it asks. */
  answer: string;
  tokens: TokenCounts;
  /** What the specialist asks the manager to decide, if it ends the turn with a question. */
  asks?: Omit<DecisionRequest, 'answer'>;
  /** The questions the Advisor asks, if it ends the turn with an interview. */
  interviews?: InterviewQuestion[];
  /** What the Advisor proposes, if it ends the turn with a proposal. */
  proposes?: Omit<Proposal, 'approvedAt'>;
}

/** The name the demo gives the model of a simulated turn. */
export const SIMULATED_MODEL = 'simulated';

/** What the first step of every simulated turn says. */
export const WHAT_THIS_IS =
  'This is the demo. The work is simulated: nothing is sent to a model and no file is written.';

/**
 * Whether a simulated specialist asks the manager before it works on a
 * task. It does when the title says that something is to be chosen, which
 * makes the demo easy to steer: a task called "Choose the battery" ends in
 * a question, any other task in a report.
 */
export function asksBeforeItWorks(task: Task): boolean {
  return /^(choose|select|decide)\b/i.test(task.title.trim());
}

/** The script of a specialist that starts on a task about a choice: it asks the manager. */
export function scriptForChoice(task: Task): Script {
  return {
    acts: [
      { after: 0, act: { kind: 'note', message: WHAT_THIS_IS } },
      {
        after: 700,
        act: { kind: 'say', text: 'I am reading the task and what the project already has.' },
      },
      { after: 600, act: { kind: 'use', id: 'read', tool: 'Read', summary: 'README.md' } },
      { after: 500, act: { kind: 'finish', id: 'read' } },
      {
        after: 800,
        act: {
          kind: 'say',
          text: 'This choice changes what is built and what it costs, so it is yours to make.',
        },
      },
    ],
    endsAfter: 600,
    answer: 'I am waiting for your decision before I go on.',
    tokens: { input: 30, output: 410, cacheRead: 21_500, cacheWritten: 5200 },
    asks: {
      question: `Which way should I take for "${task.title}"?`,
      reason:
        'The two ways lead to different parts and a different price, and the later tasks build on the answer.',
      options: [
        {
          label: 'The proven way',
          consequence:
            'Parts that are easy to get and well documented. A little larger and dearer.',
        },
        {
          label: 'The compact way',
          consequence: 'Smaller and cheaper for each unit. Fewer suppliers, and more to test.',
        },
      ],
      recommended: 0,
    },
  };
}

/** The script of a specialist that goes on after the manager has decided. */
export function scriptAfterDecision(decision: string): Script {
  return {
    acts: [
      { after: 0, act: { kind: 'note', message: WHAT_THIS_IS } },
      { after: 700, act: { kind: 'say', text: 'Thank you. I am going on with your decision.' } },
      { after: 600, act: { kind: 'use', id: 'write', tool: 'Write', summary: 'decision.md' } },
      { after: 900, act: { kind: 'finish', id: 'write' } },
    ],
    endsAfter: 600,
    answer: [
      'Done, the way you decided.',
      '',
      `1. **What I did:** I worked on with your decision (${decision}) and wrote it down in \`decision.md\`.`,
      '2. **Files:** `decision.md` (new).',
      '3. **What I decided on my own:** nothing.',
      '4. **What is open:** please read the result, and accept it or send it back.',
    ].join('\n'),
    tokens: { input: 36, output: 640, cacheRead: 22_900, cacheWritten: 900 },
  };
}

/** The script of a specialist that starts on a task. */
export function scriptForTask(task: Task): Script {
  if (asksBeforeItWorks(task)) {
    return scriptForChoice(task);
  }
  const file = `${fileNameFor(task.title)}.md`;
  return {
    acts: [
      { after: 0, act: { kind: 'note', message: WHAT_THIS_IS } },
      {
        after: 700,
        act: { kind: 'say', text: 'I am reading the task and what the project already has.' },
      },
      { after: 600, act: { kind: 'use', id: 'read', tool: 'Read', summary: 'README.md' } },
      { after: 500, act: { kind: 'finish', id: 'read' } },
      {
        after: 900,
        act: { kind: 'say', text: 'The task is clear. I am writing the result down.' },
      },
      { after: 700, act: { kind: 'use', id: 'write', tool: 'Write', summary: file } },
      { after: 900, act: { kind: 'finish', id: 'write' } },
    ],
    endsAfter: 600,
    // Written the way a model writes a report: in Markdown.
    answer: [
      `Done: ${task.title}.`,
      '',
      `1. **What I did:** I worked the task out and wrote the result into \`${file}\`.`,
      `2. **Files:** \`${file}\` (new).`,
      '3. **What I decided on my own:** nothing that changes the plan.',
      '4. **What is open:** please read the result, and accept it or send it back with what to change.',
    ].join('\n'),
    tokens: { input: 42, output: 880, cacheRead: 21_500, cacheWritten: 6400 },
  };
}

/** The script of a specialist that answers a message of the manager. */
export function scriptForMessage(member: Member): Script {
  return {
    acts: [
      { after: 0, act: { kind: 'note', message: WHAT_THIS_IS } },
      { after: 800, act: { kind: 'say', text: 'I am reading your message.' } },
    ],
    endsAfter: 900,
    answer: `This is the demo, so there is nobody to answer. In the desktop program, ${member.name} would reply here and carry on with the work.`,
    tokens: { input: 18, output: 120, cacheRead: 22_300, cacheWritten: 310 },
  };
}

/** A file name made from a title: small letters and digits, joined by hyphens. */
function fileNameFor(title: string): string {
  const name = title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 40)
    .replace(/-+$/, '');
  return name === '' ? 'result' : name;
}
