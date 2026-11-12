import type { Member, Task, TokenCounts } from '../types';

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
  /** The report the specialist ends with. */
  answer: string;
  tokens: TokenCounts;
}

/** The name the demo gives the model of a simulated turn. */
export const SIMULATED_MODEL = 'simulated';

const WHAT_THIS_IS =
  'This is the demo. The work is simulated: nothing is sent to a model and no file is written.';

/** The script of a specialist that starts on a task. */
export function scriptForTask(task: Task): Script {
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
