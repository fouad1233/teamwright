import { beforeEach, describe, expect, it } from 'vitest';

import type { Conversation, Id, Member, Project, Status, Task } from '../types';
import { briefFor } from './brief';
import { DemoBackend } from './demoBackend';
import { refusal, testEnvironment } from './testing';

function statusOf(project: Project, category: string): Status {
  const status = project.statuses.find((candidate) => candidate.category === category);
  if (!status) {
    throw new Error(`the project has no ${category} status`);
  }
  return status;
}

describe('the work of specialists in the DemoBackend', () => {
  let environment: ReturnType<typeof testEnvironment>;
  let backend: DemoBackend;
  let project: Project;
  let engineer: Member;
  let schematic: Task;

  beforeEach(async () => {
    environment = testEnvironment();
    backend = new DemoBackend(undefined, environment);
    // To do, In progress, In revision, In test, Done.
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'engineering' });
    engineer = await backend.addMember({
      projectId: project.id,
      name: 'Hardware engineer',
      kind: 'agent',
    });
    schematic = await backend.createTask({
      projectId: project.id,
      title: 'Draw the schematic',
      assignee: { kind: 'member', memberId: engineer.id },
    });
  });

  async function storedTask(taskId: Id): Promise<Task> {
    const task = (await backend.listTasks(project.id)).find((listed) => listed.id === taskId);
    if (!task) {
      throw new Error('the task is gone');
    }
    return task;
  }

  async function onlyConversation(): Promise<Conversation> {
    const [conversation] = await backend.listConversations(project.id);
    if (!conversation) {
      throw new Error('there is no conversation');
    }
    return conversation;
  }

  it('says that the program for specialists is ready', async () => {
    expect((await backend.agentProgram()).state).toBe('ready');
  });

  describe('starting on a task', () => {
    it('opens a conversation with the specialist and sends it the brief of the task', async () => {
      const conversation = await backend.startWork(schematic.id);

      expect(conversation).toMatchObject({
        projectId: project.id,
        memberId: engineer.id,
        taskId: schematic.id,
      });
      expect(conversation.turns).toHaveLength(1);
      expect(conversation.turns[0]).toMatchObject({
        origin: 'brief',
        state: 'running',
        request: briefFor(project, schematic, [schematic]),
        answer: '',
        tokens: null,
      });
    });

    it('moves the task to the first status in which work is under way', async () => {
      await backend.startWork(schematic.id);

      expect((await storedTask(schematic.id)).statusId).toBe(statusOf(project, 'active').id);
    });

    it('refuses a task that is given to the manager, to nobody or to a person', async () => {
      const person = await backend.addMember({
        projectId: project.id,
        name: 'Test engineer',
        kind: 'human',
      });
      const mine = await backend.createTask({
        projectId: project.id,
        title: 'Order the parts',
        assignee: { kind: 'manager' },
      });
      const nobodys = await backend.createTask({ projectId: project.id, title: 'Unassigned' });
      const persons = await backend.createTask({
        projectId: project.id,
        title: 'Test the prototype',
        assignee: { kind: 'member', memberId: person.id },
      });

      for (const task of [mine, nobodys, persons]) {
        const error = await refusal(backend.startWork(task.id));
        expect(error.code).toBe('no_specialist');
      }
    });

    it('refuses to start twice on the same task', async () => {
      await backend.startWork(schematic.id);
      environment.passAll();

      const error = await refusal(backend.startWork(schematic.id));

      expect(error.code).toBe('work_already_started');
      expect(error.message).toBe('work on SSN-1 has already started');
    });

    it('refuses a second task while the specialist is working, and names the first', async () => {
      const second = await backend.createTask({
        projectId: project.id,
        title: 'Route the board',
        assignee: { kind: 'member', memberId: engineer.id },
      });
      await backend.startWork(schematic.id);

      const error = await refusal(backend.startWork(second.id));

      expect(error.code).toBe('member_busy');
      expect(error.message).toBe('Hardware engineer is still working on SSN-1');
    });
  });

  describe('following the work', () => {
    it('tells its listeners about every step, with a copy of the conversation', async () => {
      const seen: Conversation[] = [];
      backend.onConversationChanged((conversation) => seen.push(conversation));
      await backend.startWork(schematic.id);

      environment.passAll();

      const kinds = seen.map((conversation) => conversation.turns[0]?.steps.at(-1)?.kind);
      expect(kinds).toEqual(['notice', 'text', 'tool', 'tool', 'text', 'tool', 'tool', 'tool']);
      expect(seen.at(-1)?.turns[0]?.state).toBe('finished');
    });

    it('shows a tool as running until it is done', async () => {
      const seen: Conversation[] = [];
      backend.onConversationChanged((conversation) => seen.push(conversation));
      await backend.startWork(schematic.id);

      environment.passAll();

      const outcomes = seen.flatMap((conversation) => {
        const step = conversation.turns[0]?.steps.find(
          (candidate) => candidate.kind === 'tool' && candidate.id === 'read',
        );
        return step?.kind === 'tool' ? [step.outcome] : [];
      });
      expect(outcomes[0]).toBe('running');
      expect(outcomes.at(-1)).toBe('done');
    });

    it('stops telling a listener that has stopped listening', async () => {
      const seen: Conversation[] = [];
      const stopListening = backend.onConversationChanged((conversation) =>
        seen.push(conversation),
      );
      await backend.startWork(schematic.id);
      stopListening();

      environment.passAll();

      expect(seen).toEqual([]);
    });

    it('ends the turn with a report, the tokens it used, and the time it ended', async () => {
      await backend.startWork(schematic.id);

      environment.passAll();

      const [turn] = (await onlyConversation()).turns;
      expect(turn?.state).toBe('finished');
      expect(turn?.answer).toContain('Done: Draw the schematic.');
      expect(turn?.tokens).toEqual({
        input: 42,
        output: 880,
        cacheRead: 21_500,
        cacheWritten: 6400,
      });
      expect(turn?.endedAt).not.toBeNull();
    });

    it('moves the task on to the first status for work that waits to be checked', async () => {
      await backend.startWork(schematic.id);

      environment.passAll();

      expect((await storedTask(schematic.id)).statusId).toBe(statusOf(project, 'review').id);
    });

    it('leaves the task where it is when the workflow has no status for checking', async () => {
      await backend.removeStatus(project.id, statusOf(project, 'review').id);
      await backend.startWork(schematic.id);

      environment.passAll();

      expect((await storedTask(schematic.id)).statusId).toBe(statusOf(project, 'active').id);
    });
  });

  describe('stopping', () => {
    it('stops the turn that is running and says who stopped it', async () => {
      const started = await backend.startWork(schematic.id);
      environment.pass(1000);

      const stopped = await backend.stopWork(started.id);
      environment.passAll();

      const [turn] = (await onlyConversation()).turns;
      expect(stopped.turns[0]?.state).toBe('stopped');
      expect(turn?.state).toBe('stopped');
      expect(turn?.failure).toBe('Stopped by the manager.');
      expect(turn?.answer).toBe('');
    });

    it('leaves the task in the status in which work is under way', async () => {
      const started = await backend.startWork(schematic.id);

      await backend.stopWork(started.id);
      environment.passAll();

      expect((await storedTask(schematic.id)).statusId).toBe(statusOf(project, 'active').id);
    });

    it('does nothing to a conversation in which nothing is running', async () => {
      const started = await backend.startWork(schematic.id);
      environment.passAll();

      const same = await backend.stopWork(started.id);

      expect(same.turns[0]?.state).toBe('finished');
    });
  });

  describe('replying', () => {
    it('adds a turn with the words of the manager to the same conversation', async () => {
      const started = await backend.startWork(schematic.id);
      environment.passAll();

      const replied = await backend.reply(started.id, '  Use the smaller regulator.  ');

      expect(replied.id).toBe(started.id);
      expect(replied.turns).toHaveLength(2);
      expect(replied.turns[1]).toMatchObject({
        origin: 'message',
        request: 'Use the smaller regulator.',
        state: 'running',
      });
    });

    it('sends a task that waited to be checked back to work', async () => {
      const started = await backend.startWork(schematic.id);
      environment.passAll();

      await backend.reply(started.id, 'Use the smaller regulator.');

      expect((await storedTask(schematic.id)).statusId).toBe(statusOf(project, 'active').id);
    });

    it('refuses an empty message', async () => {
      const started = await backend.startWork(schematic.id);
      environment.passAll();

      const error = await refusal(backend.reply(started.id, '   '));

      expect(error.message).toBe('the message must not be empty');
    });

    it('refuses a message while the specialist is still working', async () => {
      const started = await backend.startWork(schematic.id);

      const error = await refusal(backend.reply(started.id, 'Faster, please.'));

      expect(error.code).toBe('member_busy');
    });
  });

  describe('asking a specialist', () => {
    it('opens a conversation that is not about a task', async () => {
      const conversation = await backend.ask(engineer.id, 'Which regulator do you prefer?');

      expect(conversation).toMatchObject({ memberId: engineer.id, taskId: null });
      expect(conversation.turns[0]).toMatchObject({
        origin: 'message',
        request: 'Which regulator do you prefer?',
      });
    });

    it('refuses to ask a person, who is not run by the program', async () => {
      const person = await backend.addMember({
        projectId: project.id,
        name: 'Test engineer',
        kind: 'human',
      });

      const error = await refusal(backend.ask(person.id, 'Are you there?'));

      expect(error.code).toBe('no_specialist');
    });
  });

  describe('asking the manager to decide', () => {
    let choice: Task;

    beforeEach(async () => {
      // The simulated specialist asks before it works on a task whose title
      // says that something is to be chosen.
      choice = await backend.createTask({
        projectId: project.id,
        title: 'Choose the battery',
        assignee: { kind: 'member', memberId: engineer.id },
      });
    });

    /** Lets the specialist work on the choice until it has asked its question. */
    async function asked(): Promise<Conversation> {
      const started = await backend.startWork(choice.id);
      environment.passAll();
      // What a request answers with is a copy, so the conversation is read
      // again now that the specialist has asked.
      const conversations = await backend.listConversations(project.id);
      const conversation = conversations.find((listed) => listed.id === started.id);
      if (!conversation) {
        throw new Error('the conversation is gone');
      }
      return conversation;
    }

    it('ends the turn with a question instead of a report, and leaves the task in work', async () => {
      const conversation = await asked();

      const turn = conversation.turns[0];
      expect(turn?.state).toBe('finished');
      expect(turn?.decision).toMatchObject({
        question: 'Which way should I take for "Choose the battery"?',
        recommended: 0,
        answer: null,
      });
      expect(turn?.decision?.options.map((option) => option.label)).toEqual([
        'The proven way',
        'The compact way',
      ]);
      expect((await storedTask(choice.id)).statusId).toBe(statusOf(project, 'active').id);
    });

    it('asks nothing in a turn that ends in a report', async () => {
      await backend.startWork(schematic.id);
      environment.passAll();

      expect((await onlyConversation()).turns[0]?.decision).toBeNull();
    });

    it('lists the conversations that wait for the manager, the longest wait first', async () => {
      const writer = await backend.addMember({
        projectId: project.id,
        name: 'Writer',
        kind: 'agent',
      });
      const second = await backend.createTask({
        projectId: project.id,
        title: 'Select the paper',
        assignee: { kind: 'member', memberId: writer.id },
      });
      const first = await asked();
      environment.pass(60_000);
      const later = await backend.startWork(second.id);
      expect((await backend.listConversationsWaiting()).map((waiting) => waiting.id)).toEqual([
        first.id,
      ]);
      environment.passAll();

      const waiting = await backend.listConversationsWaiting();

      expect(waiting.map((conversation) => conversation.id)).toEqual([first.id, later.id]);
    });

    it('does not list a conversation of a project that is put away', async () => {
      await asked();

      await backend.archiveProject(project.id);

      expect(await backend.listConversationsWaiting()).toEqual([]);
    });

    it('takes an option as the answer and sends the specialist on with it', async () => {
      const conversation = await asked();
      environment.pass(60_000);

      const answered = await backend.answerDecision(conversation.id, { option: 1, note: '' });

      expect(answered.turns).toHaveLength(2);
      expect(answered.turns[0]?.decision?.answer).toEqual({
        option: 1,
        note: '',
        at: '2026-11-11T09:02:00.000Z',
      });
      expect(answered.turns[1]).toMatchObject({
        origin: 'decision',
        state: 'running',
        request: 'Decision: The compact way',
        decision: null,
      });
      expect(await backend.listConversationsWaiting()).toEqual([]);

      environment.passAll();
      expect((await storedTask(choice.id)).statusId).toBe(statusOf(project, 'review').id);
    });

    it('takes words of the manager with an option, or instead of one', async () => {
      const conversation = await asked();
      const withBoth = await backend.answerDecision(conversation.id, {
        option: 0,
        note: '  But keep the connector.  ',
      });
      expect(withBoth.turns[1]?.request).toBe(
        'Decision: The proven way\n\nBut keep the connector.',
      );
      expect(withBoth.turns[0]?.decision?.answer).toMatchObject({
        option: 0,
        note: 'But keep the connector.',
      });

      const other = await backend.createTask({
        projectId: project.id,
        title: 'Decide on the color',
        assignee: { kind: 'member', memberId: engineer.id },
      });
      environment.passAll();
      const second = await backend.startWork(other.id);
      environment.passAll();
      const inWords = await backend.answerDecision(second.id, {
        option: null,
        note: 'Neither. Use the gray we already have.',
      });
      expect(inWords.turns[1]?.request).toBe('Neither. Use the gray we already have.');
      expect(inWords.turns[0]?.decision?.answer).toMatchObject({ option: null });
    });

    it('refuses an answer that names no option and says nothing', async () => {
      const conversation = await asked();

      const error = await refusal(
        backend.answerDecision(conversation.id, { option: null, note: '  ' }),
      );

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe('an answer names one of the options or says something in words');
    });

    it('refuses an option that the question does not have', async () => {
      const conversation = await asked();

      const error = await refusal(backend.answerDecision(conversation.id, { option: 2, note: '' }));

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe('the question has no such option');
      expect((await backend.listConversationsWaiting()).length).toBe(1);
    });

    it('refuses an ordinary message while a question is open', async () => {
      const conversation = await asked();

      const error = await refusal(backend.reply(conversation.id, 'Go on.'));

      expect(error.code).toBe('decision_open');
      expect(error.message).toBe(
        'Hardware engineer waits for a decision: answer its question first',
      );
    });

    it('refuses an answer where nothing was asked, or was answered already', async () => {
      await backend.startWork(schematic.id);
      environment.passAll();
      const report = await onlyConversation();
      const nothingAsked = await refusal(
        backend.answerDecision(report.id, { option: 0, note: '' }),
      );
      expect(nothingAsked.code).toBe('no_decision_open');
      expect(nothingAsked.message).toBe('no question is open in this conversation');

      const conversation = await asked();
      await backend.answerDecision(conversation.id, { option: 0, note: '' });
      environment.passAll();
      const twice = await refusal(backend.answerDecision(conversation.id, { option: 1, note: '' }));
      expect(twice.code).toBe('no_decision_open');
    });

    it('lets the specialist start on something else while it waits for a decision', async () => {
      await asked();

      const other = await backend.startWork(schematic.id);

      expect(other.turns[0]?.state).toBe('running');
    });
  });

  describe('what happens to the record of the work', () => {
    it('goes with a task that is deleted', async () => {
      await backend.startWork(schematic.id);
      environment.passAll();

      await backend.deleteTask(schematic.id);

      expect(await backend.listConversations(project.id)).toEqual([]);
    });

    it('stays without the specialist when the specialist leaves the team', async () => {
      await backend.startWork(schematic.id);
      environment.passAll();

      await backend.removeMember(engineer.id);

      expect((await onlyConversation()).memberId).toBeNull();
    });

    it('keeps a specialist on the team while it is working', async () => {
      await backend.startWork(schematic.id);

      const error = await refusal(backend.removeMember(engineer.id));

      expect(error.code).toBe('member_busy');
    });

    it('goes with a project that is deleted', async () => {
      const other = await backend.createProject({ name: 'Display driver', workflow: 'standard' });
      await backend.startWork(schematic.id);
      environment.passAll();

      await backend.deleteProject(project.id);

      expect((await refusal(backend.listConversations(project.id))).code).toBe('not_found');
      expect(await backend.listConversations(other.id)).toEqual([]);
    });

    it('keeps a task, and its project, while a specialist is working on the task', async () => {
      await backend.startWork(schematic.id);

      const errors = [
        await refusal(backend.deleteTask(schematic.id)),
        await refusal(backend.deleteProject(project.id)),
        await refusal(backend.archiveProject(project.id)),
      ];

      expect(errors.map((error) => error.code)).toEqual([
        'work_in_progress',
        'work_in_progress',
        'work_in_progress',
      ]);
      expect(errors[0]?.message).toBe(
        'Hardware engineer is still working on SSN-1; stop the work first',
      );
    });

    it('lets a task go once the work on it has been stopped', async () => {
      const started = await backend.startWork(schematic.id);
      await backend.stopWork(started.id);

      await backend.deleteTask(schematic.id);

      expect(await backend.listConversations(project.id)).toEqual([]);
    });
  });

  it('has no Claude desktop app to open a conversation in', async () => {
    const started = await backend.startWork(schematic.id);

    expect((await refusal(backend.openInClaude(started.id))).code).toBe('not_in_the_demo');
  });
});

describe('the brief of a task', () => {
  let backend: DemoBackend;
  let project: Project;

  beforeEach(async () => {
    backend = new DemoBackend(undefined, testEnvironment());
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'engineering' });
  });

  it('names the task and what the manager expects back', async () => {
    const task = await backend.createTask({ projectId: project.id, title: 'Draw the schematic' });

    expect(briefFor(project, task, [task])).toBe(
      [
        'Your task is SSN-1: Draw the schematic',
        '',
        'When you are done, end with a short report for your manager:',
        '1. what you did,',
        '2. which files you created or changed,',
        '3. what you decided on your own, and why,',
        '4. what is still open.',
      ].join('\n'),
    );
  });

  it('adds the details, what the task is part of, its parts, what it builds on and when it is due', async () => {
    const board = await backend.createTask({ projectId: project.id, title: 'Design the board' });
    const pins = await backend.createTask({ projectId: project.id, title: 'Define the pin map' });
    const schematic = await backend.createTask({
      projectId: project.id,
      title: 'Draw the schematic',
      description: '  Keep the board under 40 by 60 mm.  ',
      parentId: board.id,
      dueOn: '2026-11-14',
    });
    const power = await backend.createTask({
      projectId: project.id,
      title: 'Power supply',
      parentId: schematic.id,
    });
    const waiting = await backend.addTaskDependency(schematic.id, pins.id);

    expect(briefFor(project, waiting, [board, pins, waiting, power])).toBe(
      [
        'Your task is SSN-3: Draw the schematic',
        '',
        'Keep the board under 40 by 60 mm.',
        '',
        'It is a part of SSN-1: Design the board',
        '',
        'Its parts are:',
        '- SSN-4: Power supply (To do)',
        '',
        'It builds on these tasks:',
        '- SSN-2: Define the pin map (To do)',
        '',
        'It is due on 2026-11-14.',
        '',
        'When you are done, end with a short report for your manager:',
        '1. what you did,',
        '2. which files you created or changed,',
        '3. what you decided on your own, and why,',
        '4. what is still open.',
      ].join('\n'),
    );
  });
});
