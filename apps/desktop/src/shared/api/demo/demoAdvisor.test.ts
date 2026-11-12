import { beforeEach, describe, expect, it } from 'vitest';

import type {
  ApprovedProposal,
  Conversation,
  Id,
  InterviewAnswer,
  Project,
  Proposal,
  Turn,
} from '../types';
import { DemoBackend } from './demoBackend';
import { answersInWords, waitInCircle } from './planning';
import { withoutLastMember } from './simulatedAdvisor';
import { refusal, testEnvironment } from './testing';

/** Three answers that fit the three questions the Advisor of the demo asks. */
const ANSWERS: InterviewAnswer[] = [
  { option: 0, note: '' },
  { option: 1, note: '  I have no printer.  ' },
  { option: null, note: 'The microcontroller is an ESP32-C3.' },
];

function latestTurn(conversation: Conversation): Turn {
  const turn = conversation.turns.at(-1);
  if (!turn) {
    throw new Error('the conversation has no turn');
  }
  return turn;
}

/** What the manager approves when nothing of a proposal is changed. */
function unchanged(proposal: Proposal): ApprovedProposal {
  return structuredClone({ members: proposal.members, tasks: proposal.tasks });
}

describe('planning with the Advisor in the DemoBackend', () => {
  let environment: ReturnType<typeof testEnvironment>;
  let backend: DemoBackend;
  let project: Project;

  beforeEach(async () => {
    environment = testEnvironment();
    backend = new DemoBackend(undefined, environment);
    // To do, In progress, In revision, In test, Done.
    project = await backend.createProject({ name: 'Air quality node', workflow: 'engineering' });
  });

  /**
   * A conversation as it is now. The backend answers with copies, so a
   * conversation that a test was handed earlier does not change.
   */
  async function now(conversationId: Id): Promise<Conversation> {
    const all = await backend.listConversations(project.id);
    const found = all.find((conversation) => conversation.id === conversationId);
    if (!found) {
      throw new Error('the project has no such conversation');
    }
    return found;
  }

  /** Starts a conversation with the Advisor and lets its first turn end. */
  async function interviewed(): Promise<Conversation> {
    const conversation = await backend.advise(project.id, { brief: 'A small air sensor.' });
    environment.passAll();
    return now(conversation.id);
  }

  /** Goes on until the Advisor has proposed. */
  async function proposed(): Promise<Conversation> {
    const conversation = await interviewed();
    await backend.answerInterview(conversation.id, ANSWERS);
    environment.passAll();
    return now(conversation.id);
  }

  function proposalOf(conversation: Conversation): Proposal {
    const proposal = latestTurn(conversation).proposal;
    if (!proposal) {
      throw new Error('the latest turn has no proposal');
    }
    return proposal;
  }

  describe('starting', () => {
    it('opens a conversation with the Advisor that begins with what the manager wants', async () => {
      const conversation = await backend.advise(project.id, { brief: '  A small air sensor.  ' });

      expect(conversation).toMatchObject({
        projectId: project.id,
        isWithAdvisor: true,
        memberId: null,
        taskId: null,
      });
      expect(conversation.turns).toHaveLength(1);
      expect(latestTurn(conversation)).toMatchObject({
        request: 'A small air sensor.',
        origin: 'message',
        state: 'running',
      });
    });

    it('lists the conversation with the others of the project', async () => {
      const conversation = await backend.advise(project.id, { brief: 'A small air sensor.' });

      const listed = await backend.listConversations(project.id);

      expect(listed.map((other) => other.id)).toEqual([conversation.id]);
    });

    it('refuses an empty brief', async () => {
      const error = await refusal(backend.advise(project.id, { brief: '   ' }));

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe('the message must not be empty');
    });

    it('refuses a model whose name is not one word', async () => {
      const error = await refusal(
        backend.advise(project.id, { brief: 'A small air sensor.', model: 'two words' }),
      );

      expect(error.code).toBe('rule_broken');
      expect(await backend.listConversations(project.id)).toHaveLength(0);
    });

    it('refuses a project that does not exist', async () => {
      const error = await refusal(backend.advise('nowhere', { brief: 'A small air sensor.' }));

      expect(error.code).toBe('not_found');
    });

    it('refuses a second conversation while the Advisor is working on the project', async () => {
      await backend.advise(project.id, { brief: 'A small air sensor.' });

      const error = await refusal(backend.advise(project.id, { brief: 'And a weather station.' }));

      expect(error.code).toBe('advisor_busy');
      expect(error.message).toBe('the Advisor is still working on this project');
    });

    it('lets the Advisor work on two projects at once', async () => {
      const other = await backend.createProject({ name: 'Weather station', workflow: 'standard' });
      await backend.advise(project.id, { brief: 'A small air sensor.' });

      const second = await backend.advise(other.id, { brief: 'A weather station.' });

      expect(latestTurn(second).state).toBe('running');
    });

    it('can be stopped like the work of a specialist', async () => {
      const conversation = await backend.advise(project.id, { brief: 'A small air sensor.' });

      const stopped = await backend.stopWork(conversation.id);
      environment.passAll();

      expect(latestTurn(stopped)).toMatchObject({ state: 'stopped', interview: null });
      expect(await backend.listConversationsWaiting()).toHaveLength(0);
    });
  });

  describe('the interview', () => {
    it('ends the first turn with questions: some with options, one for words', async () => {
      const conversation = await interviewed();
      const turn = latestTurn(conversation);

      expect(turn).toMatchObject({ state: 'finished', decision: null, proposal: null });
      expect(turn.interview?.questions.map((question) => question.options.length)).toEqual([
        3, 2, 0,
      ]);
      expect(turn.interview?.questions[0]?.recommended).toBe(0);
      expect(turn.interview?.questions[2]?.recommended).toBeNull();
      expect(turn.interview).toMatchObject({ answers: null, answeredAt: null });
    });

    it('counts open questions among what waits for the manager', async () => {
      const conversation = await interviewed();

      const waiting = await backend.listConversationsWaiting();

      expect(waiting.map((other) => other.id)).toEqual([conversation.id]);
    });

    it('takes one answer for each question and sends the Advisor on with them', async () => {
      const conversation = await interviewed();
      environment.pass(60_000);

      const answered = await backend.answerInterview(conversation.id, ANSWERS);

      expect(answered.turns).toHaveLength(2);
      expect(answered.turns[0]?.interview).toMatchObject({
        answers: [
          { option: 0, note: '' },
          { option: 1, note: 'I have no printer.' },
          { option: null, note: 'The microcontroller is an ESP32-C3.' },
        ],
        answeredAt: environment.now().toISOString(),
      });
      expect(latestTurn(answered)).toMatchObject({ origin: 'interview', state: 'running' });
      expect(latestTurn(answered).request).toBe(
        [
          'The manager answered your questions.',
          '',
          '1. What matters most for the first version?',
          '   Answer: A working prototype soon',
          '',
          '2. Who does the work that needs hands?',
          '   Answer: Leave it out of the plan',
          '   The manager adds: I have no printer.',
          '',
          '3. Is anything already decided that must not change?',
          '   Answer: The microcontroller is an ESP32-C3.',
        ].join('\n'),
      );
      expect(await backend.listConversationsWaiting()).toHaveLength(0);
    });

    it('refuses too few or too many answers', async () => {
      const conversation = await interviewed();

      for (const answers of [ANSWERS.slice(0, 2), [...ANSWERS, { option: null, note: 'More.' }]]) {
        const error = await refusal(backend.answerInterview(conversation.id, answers));
        expect(error.code).toBe('rule_broken');
        expect(error.message).toBe('every question needs an answer');
      }
    });

    it('refuses an option that a question does not have', async () => {
      const conversation = await interviewed();

      const error = await refusal(
        backend.answerInterview(conversation.id, [
          ANSWERS[0] ?? { option: 0, note: '' },
          { option: 2, note: '' },
          { option: null, note: 'Nothing.' },
        ]),
      );

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe('question 2 has no such option');
    });

    it('refuses an answer that names no option and says nothing', async () => {
      const conversation = await interviewed();

      const error = await refusal(
        backend.answerInterview(conversation.id, [
          { option: 0, note: '' },
          { option: 0, note: '' },
          { option: null, note: '   ' },
        ]),
      );

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe(
        'the answer to question 3 names one of the options or says something in words',
      );
      expect(latestTurn(await now(conversation.id)).interview?.answers).toBeNull();
    });

    it('refuses an ordinary message while the questions are open', async () => {
      const conversation = await interviewed();

      const error = await refusal(backend.reply(conversation.id, 'Just propose something.'));

      expect(error.code).toBe('interview_open');
      expect(error.message).toBe('the Advisor waits for your answers: answer its questions first');
    });

    it('refuses answers where nothing was asked, or was answered already', async () => {
      const conversation = await interviewed();
      await backend.answerInterview(conversation.id, ANSWERS);
      environment.passAll();

      const again = await refusal(backend.answerInterview(conversation.id, ANSWERS));
      const nowhere = await refusal(backend.answerInterview('nowhere', ANSWERS));

      expect(again.code).toBe('no_interview_open');
      expect(again.message).toBe('no questions are open in this conversation');
      expect(nowhere.code).toBe('not_found');
    });
  });

  describe('the proposal', () => {
    it('ends the turn after the answers with a team and a plan that are not approved', async () => {
      const conversation = await proposed();
      const proposal = proposalOf(conversation);

      expect(latestTurn(conversation)).toMatchObject({ state: 'finished', interview: null });
      expect(proposal.approvedAt).toBeNull();
      expect(proposal.summary).toContain('measures the air');
      expect(proposal.members.map((member) => member.name)).toEqual([
        'Hardware engineer',
        'Firmware engineer',
        'Mechanical engineer',
      ]);
      expect(proposal.members[0]).toMatchObject({ model: null, effort: 'high' });
      expect(proposal.tasks).toHaveLength(12);
      expect(proposal.tasks[0]).toMatchObject({
        title: 'Agree on what the device must do',
        owner: { kind: 'manager' },
        estimateMinutes: 120,
        priority: 'high',
        waitsFor: [],
      });
      expect(proposal.tasks[11]?.waitsFor).toEqual([7, 8, 9, 10]);
    });

    it('creates nothing before the manager approves', async () => {
      await proposed();

      expect(await backend.listMembers(project.id)).toHaveLength(0);
      expect(await backend.listTasks(project.id)).toHaveLength(0);
    });

    it('counts a proposal that is not approved among what waits for the manager', async () => {
      const conversation = await proposed();

      const waiting = await backend.listConversationsWaiting();

      expect(waiting.map((other) => other.id)).toEqual([conversation.id]);
    });

    it('adds the members to the team and creates the tasks when it is approved', async () => {
      const conversation = await proposed();
      environment.pass(60_000);

      const approved = await backend.approveProposal(
        conversation.id,
        unchanged(proposalOf(conversation)),
      );

      const team = await backend.listMembers(project.id);
      expect(team.map((member) => [member.name, member.kind, member.color])).toEqual([
        ['Hardware engineer', 'agent', 'teal'],
        ['Firmware engineer', 'agent', 'blue'],
        ['Mechanical engineer', 'agent', 'purple'],
      ]);
      expect(team[0]).toMatchObject({
        responsibility: 'The schematic, the parts and the board.',
        model: null,
        effort: 'high',
      });
      expect(team[1]).toMatchObject({ model: 'sonnet', effort: null });

      const tasks = await backend.listTasks(project.id);
      expect(tasks.map((task) => task.number)).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
      expect(new Set(tasks.map((task) => task.statusId))).toEqual(
        new Set([project.statuses[0]?.id]),
      );
      expect(tasks[0]).toMatchObject({
        title: 'Agree on what the device must do',
        assignee: { kind: 'manager' },
        estimateMinutes: 120,
        priority: 'high',
        blockedBy: [],
      });
      expect(tasks[1]).toMatchObject({
        assignee: { kind: 'member', memberId: team[0]?.id },
        blockedBy: [tasks[0]?.id],
      });
      expect(tasks[9]?.assignee).toEqual({ kind: 'member', memberId: team[2]?.id });
      expect(tasks[11]?.blockedBy).toEqual([
        tasks[7]?.id,
        tasks[8]?.id,
        tasks[9]?.id,
        tasks[10]?.id,
      ]);
      expect(tasks[2]?.description).toContain('Which pin goes to which part');

      expect(proposalOf(approved).approvedAt).toBe(environment.now().toISOString());
      expect(await backend.listConversationsWaiting()).toHaveLength(0);
    });

    it('tells its listeners when a proposal is approved', async () => {
      const conversation = await proposed();
      const heard: (string | null | undefined)[] = [];
      backend.onConversationChanged((changed) => {
        heard.push(changed.turns.at(-1)?.proposal?.approvedAt);
      });

      await backend.approveProposal(conversation.id, unchanged(proposalOf(conversation)));

      expect(heard).toEqual([environment.now().toISOString()]);
    });

    it('approves what the manager left of it, and keeps that on record', async () => {
      const conversation = await proposed();
      const smaller = withoutLastMember(proposalOf(conversation));

      const approved = await backend.approveProposal(conversation.id, {
        members: smaller.members,
        tasks: smaller.tasks,
      });

      expect(await backend.listMembers(project.id)).toHaveLength(2);
      const tasks = await backend.listTasks(project.id);
      expect(tasks).toHaveLength(11);
      // The last task waited for the enclosure too. That task is gone.
      expect(tasks[10]?.blockedBy).toEqual([tasks[7]?.id, tasks[8]?.id, tasks[9]?.id]);
      expect(proposalOf(approved).members).toHaveLength(2);
      expect(proposalOf(approved).tasks).toHaveLength(11);
    });

    it('gives a task to the member the team already has, and does not add that member again', async () => {
      const there = await backend.addMember({
        projectId: project.id,
        name: 'hardware engineer',
        kind: 'agent',
      });
      const conversation = await proposed();

      await backend.approveProposal(conversation.id, unchanged(proposalOf(conversation)));

      const team = await backend.listMembers(project.id);
      expect(team.map((member) => member.name)).toEqual([
        'hardware engineer',
        'Firmware engineer',
        'Mechanical engineer',
      ]);
      const tasks = await backend.listTasks(project.id);
      expect(tasks[1]?.assignee).toEqual({ kind: 'member', memberId: there.id });
    });

    it('adds to the tasks the project already has', async () => {
      await backend.createTask({ projectId: project.id, title: 'Something that was there' });
      const conversation = await proposed();

      await backend.approveProposal(conversation.id, unchanged(proposalOf(conversation)));

      const tasks = await backend.listTasks(project.id);
      expect(tasks).toHaveLength(13);
      expect(tasks[0]?.title).toBe('Something that was there');
      expect(tasks[12]?.number).toBe(13);
    });

    it('refuses two members of the same name', async () => {
      const conversation = await proposed();
      const changed = unchanged(proposalOf(conversation));
      const second = changed.members[1];
      if (second) {
        second.name = ' hardware ENGINEER ';
      }

      const error = await refusal(backend.approveProposal(conversation.id, changed));

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe('two members of the proposal are called "hardware ENGINEER"');
    });

    it('refuses a task that is given to somebody who is not there', async () => {
      const conversation = await proposed();
      const changed = unchanged(proposalOf(conversation));
      changed.members.pop();

      const error = await refusal(backend.approveProposal(conversation.id, changed));

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe(
        '"Design the enclosure" is given to "Mechanical engineer", who is neither on the team nor in the proposal',
      );
    });

    it('refuses a task that waits for itself, or for a task that is not there', async () => {
      const conversation = await proposed();

      for (const waitsFor of [[3], [12], [-1], [1.5]]) {
        const changed = unchanged(proposalOf(conversation));
        const task = changed.tasks[3];
        if (task) {
          task.waitsFor = waitsFor;
        }
        const error = await refusal(backend.approveProposal(conversation.id, changed));
        expect(error.code).toBe('rule_broken');
        expect(error.message).toBe(
          '"Draw the schematic" waits for a task that is not another task of the proposal',
        );
      }
    });

    it('refuses tasks that wait for each other in a circle', async () => {
      const conversation = await proposed();
      const changed = unchanged(proposalOf(conversation));
      const first = changed.tasks[0];
      if (first) {
        // The first task now waits for the last, which waits for it through others.
        first.waitsFor = [11];
      }

      const error = await refusal(backend.approveProposal(conversation.id, changed));

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe('the tasks of the proposal wait for each other in a circle');
    });

    it('refuses a task without a title', async () => {
      const conversation = await proposed();
      const changed = unchanged(proposalOf(conversation));
      const task = changed.tasks[4];
      if (task) {
        task.title = '   ';
      }

      const error = await refusal(backend.approveProposal(conversation.id, changed));

      expect(error.code).toBe('rule_broken');
    });

    it('refuses a proposal with nothing left in it', async () => {
      const conversation = await proposed();

      const error = await refusal(
        backend.approveProposal(conversation.id, { members: [], tasks: [] }),
      );

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe('the proposal has no members and no tasks');
    });

    it('creates nothing when something is refused, and leaves the proposal open', async () => {
      const conversation = await proposed();
      const changed = unchanged(proposalOf(conversation));
      const last = changed.tasks[11];
      if (last) {
        last.owner = { kind: 'member', name: 'Nobody' };
      }

      await refusal(backend.approveProposal(conversation.id, changed));

      expect(await backend.listMembers(project.id)).toHaveLength(0);
      expect(await backend.listTasks(project.id)).toHaveLength(0);
      expect(proposalOf(await now(conversation.id)).approvedAt).toBeNull();
      expect(await backend.listConversationsWaiting()).toHaveLength(1);
    });

    it('refuses to approve twice, and where nothing was proposed', async () => {
      const asked = await interviewed();
      const nothingYet = await refusal(
        backend.approveProposal(asked.id, { members: [], tasks: [] }),
      );
      await backend.answerInterview(asked.id, ANSWERS);
      environment.passAll();
      const approved = unchanged(proposalOf(await now(asked.id)));
      await backend.approveProposal(asked.id, approved);

      const twice = await refusal(backend.approveProposal(asked.id, approved));

      expect(nothingYet.code).toBe('no_proposal_open');
      expect(nothingYet.message).toBe('no proposal is open in this conversation');
      expect(twice.code).toBe('no_proposal_open');
      expect(await backend.listTasks(project.id)).toHaveLength(12);
    });

    it('takes a message of the manager as a wish for a change, and proposes again', async () => {
      const conversation = await proposed();
      const first = proposalOf(conversation);

      await backend.reply(conversation.id, '  Two specialists are enough.  ');
      environment.passAll();

      const changed = await now(conversation.id);
      expect(changed.turns).toHaveLength(3);
      expect(latestTurn(changed)).toMatchObject({
        request: 'Two specialists are enough.',
        origin: 'message',
        state: 'finished',
      });
      expect(proposalOf(changed).members).toHaveLength(2);
      // The proposal before stays on record as it was, and is not open any more.
      expect(changed.turns[1]?.proposal).toEqual(first);
      const approved = await backend.approveProposal(
        conversation.id,
        unchanged(proposalOf(changed)),
      );
      expect(await backend.listMembers(project.id)).toHaveLength(2);
      expect(approved.turns[1]?.proposal?.approvedAt).toBeNull();
    });

    it('lets the manager start another round after a proposal was approved', async () => {
      const conversation = await proposed();
      await backend.approveProposal(conversation.id, unchanged(proposalOf(conversation)));

      const again = await backend.reply(conversation.id, 'Add a battery.');

      expect(latestTurn(again).state).toBe('running');
    });
  });

  describe('what happens to the record of the planning', () => {
    it('keeps a project while the Advisor is working on it', async () => {
      await backend.advise(project.id, { brief: 'A small air sensor.' });

      const error = await refusal(backend.deleteProject(project.id));

      expect(error.code).toBe('work_in_progress');
      expect(error.message).toBe('The Advisor is still working on a plan; stop the work first');
    });

    it('goes with a project that is deleted', async () => {
      await proposed();

      await backend.deleteProject(project.id);

      expect(await backend.listConversationsWaiting()).toHaveLength(0);
    });

    it('does not wait for the manager in a project that is put away', async () => {
      await proposed();

      await backend.archiveProject(project.id);

      expect(await backend.listConversationsWaiting()).toHaveLength(0);
    });
  });
});

describe('the answers to an interview in words', () => {
  it('gives the words of the manager where no option was chosen', () => {
    const words = answersInWords(
      [{ question: 'What is fixed?', reason: '', options: [], recommended: null }],
      [{ option: null, note: 'Nothing.' }],
    );

    expect(words).toBe(
      'The manager answered your questions.\n\n1. What is fixed?\n   Answer: Nothing.',
    );
  });
});

describe('tasks that wait in a circle', () => {
  const task = (waitsFor: number[]) => ({
    title: 'A task',
    description: '',
    owner: null,
    estimateMinutes: null,
    priority: 'normal' as const,
    waitsFor,
  });

  it('finds none where every task waits only for earlier ones', () => {
    expect(waitInCircle([task([]), task([0]), task([0, 1])])).toBe(false);
  });

  it('finds none where a later task is waited for', () => {
    expect(waitInCircle([task([2]), task([]), task([1])])).toBe(false);
  });

  it('finds two tasks that wait for each other, and a longer circle', () => {
    expect(waitInCircle([task([1]), task([0])])).toBe(true);
    expect(waitInCircle([task([]), task([3]), task([1]), task([2])])).toBe(true);
  });
});
