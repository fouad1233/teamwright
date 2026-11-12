/**
 * The Advisor plans a small project, on Windows.
 *
 * It starts the built application with an empty data folder, creates a
 * project, and tells the Advisor in a few sentences what is wanted. It
 * follows the Advisor to its proposal: if the Advisor asks questions, it
 * answers them the way the Advisor recommends, and it checks that nothing
 * is added to the project before the proposal is approved. Then it approves
 * the proposal with the button of the interface, checks that the members
 * and the tasks are there as proposed, and starts the application again to
 * see that all of it is still there.
 *
 * Like the test of a real specialist next to it, this one is not free. It
 * runs the Claude Code that is installed on this computer, signed in as
 * whoever is signed in there. Planning takes two or three turns, and they
 * use a part of that account's plan. So it is not part of any automatic
 * check: run it by hand after a change to the Advisor. It prints what every
 * turn used.
 *
 * Usage:
 *   node tools/smoke/advisor-smoke.mjs <path-to-teamwright-desktop.exe> [folder-for-pictures]
 *
 * With a folder, it also takes pictures of the questions, of the proposal
 * and of the board after the approval, in the light and in the dark theme.
 * The pictures named `advisor-…` in docs/screenshots are taken this way.
 *
 * The Advisor of the test runs on the model named in TEAMWRIGHT_ADVISOR_MODEL,
 * or on `sonnet`: enough for a plan of this size, and cheaper than the
 * default of the program.
 *
 * It needs nothing but Node.js 22 or later. The user's real data is never
 * touched: the application keeps its data, and the folders of its projects,
 * in a temporary folder.
 */

import { mkdirSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';

import { DesktopApplication, removeDataDirectory } from '../shared/desktop-application.mjs';

const [executable, pictureFolder] = process.argv.slice(2);
if (!executable) {
  console.error('usage: node advisor-smoke.mjs <path-to-exe> [folder-for-pictures]');
  process.exit(2);
}

/** The port on which the web view of the application offers its debugging protocol. */
const DEBUGGING_PORT = 9335;

/** How long one turn of the Advisor may take before the test gives up on it. */
const TURN_TIMEOUT_MILLISECONDS = 5 * 60 * 1000;

/** How often the test asks how a turn stands. */
const POLL_MILLISECONDS = 1500;

/** The most rounds of questions the test answers before it expects a proposal. */
const MOST_INTERVIEWS = 3;

/** The model the Advisor of the test runs on. */
const MODEL = process.env.TEAMWRIGHT_ADVISOR_MODEL ?? 'sonnet';

/** What the manager of the test wants. */
const BRIEF =
  'I want to build a small device for my desk that measures the air: temperature, humidity and CO2. ' +
  'It should have a small screen that shows the values, and it should send them over Wi-Fi so that I can see a history on my phone. ' +
  'I want it cheap and compact, on a board of my own design, in a printed case. ' +
  'I can solder and I have a 3D printer. I have built two simple boards before.';

/**
 * The size the interface is shown at, in the pixels of the style sheet, and
 * how many pixels of a picture each of them gets. It is the size of the
 * pictures in docs/screenshots, so that the pictures of this test fit in
 * with them.
 */
const PICTURE = { width: 1712, height: 1000, scale: 1.5 };

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

/** Says what a turn used, for the person who pays for it. */
function report(name, turn) {
  const tokens = turn.tokens;
  const used = tokens
    ? `${tokens.output} written, ${tokens.input} read, ${tokens.cacheRead} read from the cache, ${tokens.cacheWritten} written to the cache`
    : 'no count of tokens';
  const seconds = turn.endedAt
    ? Math.round((Date.parse(turn.endedAt) - Date.parse(turn.startedAt)) / 1000)
    : '?';
  console.log(
    `      ${name}: ${turn.state} after ${seconds} s on ${turn.model ?? 'an unknown model'}; ${used}`,
  );
  for (const step of turn.steps) {
    if (step.kind === 'tool') {
      console.log(`        - ${step.tool} ${step.summary} (${step.outcome})`);
    } else if (step.kind === 'denied') {
      console.log(`        - refused: ${step.tool}`);
    }
  }
  if (turn.failure) {
    console.log(`        ${turn.failure}`);
  }
}

/** The name of the owner of a proposed task, for the printed plan. */
function ownerOf(task) {
  if (task.owner === null) {
    return 'nobody';
  }
  return task.owner.kind === 'manager' ? 'the manager' : task.owner.name;
}

/** Thrown when the test cannot run at all, which is not a failure of the application. */
class CannotRun extends Error {}

const dataDirectory = mkdtempSync(join(tmpdir(), 'teamwright-advisor-'));
/** This test is about the real program, so the application is not given the stand-in. */
const REAL = { withRealAgentProgram: true };

let application = DesktopApplication.start(executable, dataDirectory, DEBUGGING_PORT, REAL);
let couldRun = true;

try {
  let window = await application.openWindow();
  await window.waitForText('No projects yet');

  /**
   * Takes a picture of the window in the dark and in the light theme, if
   * the caller asked for pictures.
   */
  const picture = async (name) => {
    if (!pictureFolder) {
      return;
    }
    mkdirSync(pictureFolder, { recursive: true });
    for (const theme of ['light', 'dark']) {
      await window.showInTheme(theme);
      await sleep(300);
      const path = join(pictureFolder, `advisor-${name}-${theme}.png`);
      await window.screenshot(path);
      console.log(`      picture written to ${path}`);
    }
  };

  /** Calls a command that is expected to work, and returns its answer. */
  const command = async (name, args) => {
    const answer = await window.command(name, args);
    if (answer.refused) {
      throw new Error(`${name} was refused: ${JSON.stringify(answer.refused)}`);
    }
    return answer.ok;
  };

  /** Clicks the tab of a view of the open project. */
  const showView = (name) =>
    window.click(
      `[...document.querySelectorAll('[role="tab"]')].find((tab) => tab.innerText.trim() === ${JSON.stringify(name)})`,
    );

  /** Brings a part of the page of the Advisor to the top of what is shown. */
  const bringToTop = (selector) =>
    window.evaluate(
      `document.querySelector(${JSON.stringify(selector)})?.scrollIntoView({ block: 'start' })`,
    );

  // The program that runs the Advisor has to be there and signed in.
  const program = await command('agent_program');
  console.log(
    `      the agent program: ${program.state}, version ${program.version ?? '?'}, signed in with ${program.signInMethod ?? '?'}`,
  );
  if (program.state !== 'ready') {
    throw new CannotRun(
      `Claude Code cannot be used on this computer (${program.state}${program.problem ? `: ${program.problem}` : ''}).` +
        ' Install it and sign in, then run this test again.',
    );
  }
  check('the agent program is found and signed in', true);

  const project = await command('create_project', {
    input: {
      name: 'Air quality node',
      description: 'A small device for the desk that measures the air.',
      workflow: 'engineering',
    },
  });
  const conversations = () => command('list_conversations', { projectId: project.id });
  const waiting = () => command('list_conversations_waiting');

  /** Waits until no turn of a conversation is running, and returns the conversation. */
  const waitForEnd = async (conversationId) => {
    const deadline = Date.now() + TURN_TIMEOUT_MILLISECONDS;
    while (Date.now() < deadline) {
      const conversation = (await conversations()).find((other) => other.id === conversationId);
      if (conversation && conversation.turns.every((turn) => turn.state !== 'running')) {
        return conversation;
      }
      await sleep(POLL_MILLISECONDS);
    }
    throw new Error('the Advisor did not come to an end in time');
  };

  // The manager opens the project. A new project is planned on the page of
  // the Advisor.
  await window.showAtSize(PICTURE.width, PICTURE.height, PICTURE.scale);
  await window.showInTheme('dark');
  await window.reload();
  await window.waitForText('Air quality node');
  await showView('Advisor');
  await window.waitForText('Plan this project with the Advisor');
  check('the page of the Advisor offers to plan the project', true);

  // The manager says what is wanted.
  const started = await command('advise', {
    projectId: project.id,
    advice: { brief: BRIEF, model: MODEL, effort: 'medium' },
  });
  check(
    'planning opens a conversation with the Advisor that begins with what the manager wants',
    started.isWithAdvisor === true &&
      started.memberId === null &&
      started.taskId === null &&
      started.turns.length === 1 &&
      started.turns[0].origin === 'message' &&
      started.turns[0].request === BRIEF,
    JSON.stringify(started).slice(0, 300),
  );
  await window.waitForText('WORKING');
  check('the window shows that the Advisor is working, without being loaded again', true);
  const second = await window.command('advise', {
    projectId: project.id,
    advice: { brief: 'And a weather station.' },
  });
  check(
    'a second conversation is refused while the Advisor works on the project',
    second.refused?.code === 'advisor_busy',
    JSON.stringify(second),
  );

  // The Advisor may ask before it proposes. The test answers what it asks.
  let conversation = await waitForEnd(started.id);
  let interviews = 0;
  for (;;) {
    const turn = conversation.turns.at(-1);
    report(`turn ${conversation.turns.length}`, turn);
    check('the turn finishes', turn.state === 'finished', turn.failure ?? '');
    if (turn.state !== 'finished' || !turn.interview || interviews === MOST_INTERVIEWS) {
      break;
    }
    interviews += 1;
    const questions = turn.interview.questions;
    console.log(`        the Advisor asks ${questions.length} question(s):`);
    for (const question of questions) {
      console.log(`          ? ${question.question}`);
      for (const [place, option] of question.options.entries()) {
        console.log(
          `              ${place === question.recommended ? '*' : '-'} ${option.label}: ${option.consequence}`,
        );
      }
    }
    check(
      'the Advisor asks one to five questions, each with two to four options or with none',
      questions.length >= 1 &&
        questions.length <= 5 &&
        questions.every(
          (question) =>
            question.question !== '' &&
            (question.options.length === 0 ||
              (question.options.length >= 2 && question.options.length <= 4)),
        ) &&
        turn.interview.answers === null,
      JSON.stringify(turn.interview).slice(0, 400),
    );
    check(
      'open questions are listed among what waits for the manager',
      (await waiting()).some((other) => other.id === conversation.id),
    );
    const instead = await window.command('reply', {
      conversationId: conversation.id,
      message: 'Just propose something.',
    });
    check(
      'an ordinary message is refused while the questions are open',
      instead.refused?.code === 'interview_open',
      JSON.stringify(instead),
    );
    if (interviews === 1) {
      const shown = await window.waitForText('THE ADVISOR ASKS');
      check(
        'the window shows the questions without being loaded again',
        questions.every((question) => shown.includes(question.question)),
        shown.slice(0, 300),
      );
      await bringToTop('form');
      await picture('interview');
      await window.showInTheme('dark');
    }

    // The manager takes what the Advisor recommends, and leaves the rest to it.
    const answers = questions.map((question) =>
      question.options.length > 0
        ? { option: question.recommended ?? 0, note: '' }
        : { option: null, note: 'Nothing is fixed. Choose what is sensible.' },
    );
    const answered = await command('answer_interview', {
      conversationId: conversation.id,
      answers,
    });
    check(
      'the answers are kept with the questions and start the next turn',
      answered.turns.at(-2)?.interview?.answers?.length === questions.length &&
        answered.turns.at(-1)?.origin === 'interview' &&
        answered.turns.at(-1)?.state === 'running',
      JSON.stringify(answered.turns.at(-1)).slice(0, 300),
    );
    conversation = await waitForEnd(conversation.id);
  }

  const proposal = conversation.turns.at(-1)?.proposal;
  check(
    'the Advisor proposes a team and a plan',
    proposal !== null && proposal !== undefined && proposal.approvedAt === null,
    JSON.stringify(conversation.turns.at(-1)).slice(0, 400),
  );
  if (proposal) {
    console.log(`        ${proposal.summary.replace(/\s+/g, ' ').slice(0, 320)}`);
    for (const member of proposal.members) {
      console.log(
        `        member: ${member.name} (${member.model ?? 'default model'}, ${member.effort ?? 'default effort'}): ${member.reason}`,
      );
    }
    for (const [place, task] of proposal.tasks.entries()) {
      const minutes = task.estimateMinutes === null ? '' : `, ${task.estimateMinutes} min`;
      const after =
        task.waitsFor.length === 0
          ? ''
          : `, after ${task.waitsFor.map((earlier) => earlier + 1).join(', ')}`;
      console.log(`        ${place + 1}. ${task.title} [${ownerOf(task)}${minutes}${after}]`);
    }

    const names = new Set(proposal.members.map((member) => member.name.toLowerCase()));
    check(
      'the proposal has a summary, one to eight members with a reason each, and tasks',
      proposal.summary !== '' &&
        proposal.members.length >= 1 &&
        proposal.members.length <= 8 &&
        proposal.members.every((member) => member.name !== '' && member.reason !== '') &&
        proposal.tasks.length >= 3,
      `${proposal.members.length} members, ${proposal.tasks.length} tasks`,
    );
    check(
      'every task is given to the manager or to a proposed member, and waits only for other tasks of the proposal',
      proposal.tasks.every(
        (task, place) =>
          task.title !== '' &&
          (task.owner === null ||
            task.owner.kind === 'manager' ||
            names.has(task.owner.name.toLowerCase())) &&
          task.waitsFor.every(
            (earlier) => earlier >= 0 && earlier < proposal.tasks.length && earlier !== place,
          ),
      ),
      JSON.stringify(proposal.tasks).slice(0, 400),
    );
    check(
      'nothing is added to the project before the manager approves',
      (await command('list_members', { projectId: project.id })).length === 0 &&
        (await command('list_tasks', { projectId: project.id })).length === 0,
    );
    check(
      'the proposal is listed among what waits for the manager',
      (await waiting()).some((other) => other.id === conversation.id),
    );
    const shown = await window.waitForText('THE ADVISOR PROPOSES');
    check(
      'the window shows the proposal without being loaded again',
      proposal.members.every((member) => shown.includes(member.name)) &&
        proposal.tasks.every((task) => shown.includes(task.title)),
      shown.slice(0, 300),
    );
    await bringToTop('[aria-label="The proposal of the Advisor"]');
    await picture('proposal');
    await window.showInTheme('dark');

    // The manager approves with the button, as it stands.
    await window.click(
      `[...document.querySelectorAll('button')].find((button) => button.innerText.trim() === 'Approve')`,
    );
    await window.waitForText('APPROVED');
    const team = await command('list_members', { projectId: project.id });
    const tasks = await command('list_tasks', { projectId: project.id });
    const approved = (await conversations()).find((other) => other.id === conversation.id);
    check(
      'approving adds the proposed members to the team as AI specialists',
      team.length === proposal.members.length &&
        proposal.members.every((proposed) =>
          team.some(
            (member) =>
              member.name === proposed.name &&
              member.kind === 'agent' &&
              member.model === proposed.model &&
              member.effort === proposed.effort,
          ),
        ),
      JSON.stringify(team.map((member) => [member.name, member.model, member.effort])),
    );
    const firstStatus = project.statuses[0];
    const byTitle = (title) => tasks.find((task) => task.title === title);
    check(
      'and creates the tasks in the first status, each given to its owner',
      tasks.length === proposal.tasks.length &&
        tasks.every((task) => task.statusId === firstStatus.id) &&
        proposal.tasks.every((proposed) => {
          const task = byTitle(proposed.title);
          const member =
            proposed.owner?.kind === 'member'
              ? team.find((other) => other.name.toLowerCase() === proposed.owner.name.toLowerCase())
              : undefined;
          return (
            task !== undefined &&
            task.estimateMinutes === proposed.estimateMinutes &&
            task.priority === proposed.priority &&
            (proposed.owner === null
              ? task.assignee === null
              : proposed.owner.kind === 'manager'
                ? task.assignee?.kind === 'manager'
                : task.assignee?.kind === 'member' && task.assignee.memberId === member?.id)
          );
        }),
      `${tasks.length} tasks`,
    );
    check(
      'and every task waits for the tasks the proposal says',
      proposal.tasks.every((proposed) => {
        const blockers = new Set(byTitle(proposed.title)?.blockedBy ?? []);
        return (
          blockers.size === new Set(proposed.waitsFor).size &&
          proposed.waitsFor.every((earlier) =>
            blockers.has(byTitle(proposal.tasks[earlier].title)?.id),
          )
        );
      }),
    );
    check(
      'the proposal is marked as approved, and nothing waits for the manager any more',
      typeof approved?.turns.at(-1)?.proposal?.approvedAt === 'string' &&
        !(await waiting()).some((other) => other.id === conversation.id),
    );
    const twice = await window.command('approve_proposal', {
      conversationId: conversation.id,
      proposal: { members: proposal.members, tasks: proposal.tasks },
    });
    check(
      'a proposal cannot be approved twice',
      twice.refused?.code === 'no_proposal_open',
      JSON.stringify(twice),
    );
    await showView('Board');
    await window.waitForText(proposal.tasks[0].title);
    check('the board shows the tasks of the plan without being loaded again', true);
    await picture('board');
    window.close();

    // Second run: the plan and its record are still there.
    await application.stop();
    application = DesktopApplication.start(executable, dataDirectory, DEBUGGING_PORT, REAL);
    window = await application.openWindow();
    await window.waitForText('Air quality node');
    const stored = (await window.command('list_conversations', { projectId: project.id })).ok;
    const kept = stored?.find((other) => other.id === conversation.id);
    check(
      'the conversation with the Advisor survives a restart, with its questions and its proposal',
      kept?.isWithAdvisor === true &&
        kept.turns.length === conversation.turns.length &&
        kept.turns.filter((turn) => turn.interview?.answers).length === interviews &&
        typeof kept.turns.at(-1)?.proposal?.approvedAt === 'string' &&
        kept.turns.at(-1).proposal.tasks.length === proposal.tasks.length,
      JSON.stringify(kept)?.slice(0, 300),
    );
    check(
      'and so do the team and the tasks it added',
      (await window.command('list_members', { projectId: project.id })).ok?.length ===
        proposal.members.length &&
        (await window.command('list_tasks', { projectId: project.id })).ok?.length ===
          proposal.tasks.length,
    );
  }
  window.close();
} catch (failure) {
  if (failure instanceof CannotRun) {
    couldRun = false;
    console.error(`\n${failure.message}`);
  } else {
    check('the test ran to its end', false, failure.message);
  }
} finally {
  await application.stop();
  removeDataDirectory(dataDirectory);
}

if (!couldRun) {
  process.exit(2);
}
console.log(failures === 0 ? '\nAll checks passed.' : `\n${failures} check(s) failed.`);
process.exit(failures === 0 ? 0 : 1);
