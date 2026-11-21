import { beforeEach, describe, expect, it } from 'vitest';

import type { Member, Project, Task } from '../types';
import { resetInWords } from '@/features/work/planLimitModel';
import { GO_ON_REQUEST, WAKE_REQUEST, raisesOf, words } from './coordinator';
import { DemoBackend } from './demoBackend';
import { sampleWorkspace } from './sampleWorkspace';
import { refusal, testEnvironment } from './testing';

/**
 * The coordinator of the demo follows the rule of the program
 * (`coordinator.rs`, whose tests are the rule's own). These tests check
 * that the demo's copy starts, holds, asks and words as the program does.
 */
describe('the coordinator in the DemoBackend', () => {
  let environment: ReturnType<typeof testEnvironment>;
  let backend: DemoBackend;
  let project: Project;
  let hardware: Member;
  let firmware: Member;

  beforeEach(async () => {
    environment = testEnvironment();
    backend = new DemoBackend(undefined, environment);
    // To do, In progress, In revision, In test, Done.
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'engineering' });
    hardware = await backend.addMember({
      projectId: project.id,
      name: 'Hardware engineer',
      kind: 'agent',
    });
    firmware = await backend.addMember({
      projectId: project.id,
      name: 'Firmware engineer',
      kind: 'agent',
    });
  });

  function task(title: string, member: Member): Promise<Task> {
    return backend.createTask({
      projectId: project.id,
      title,
      assignee: { kind: 'member', memberId: member.id },
    });
  }

  async function categoryOf(taskId: string): Promise<string | undefined> {
    const stored = (await backend.listTasks(project.id)).find((listed) => listed.id === taskId);
    return project.statuses.find((status) => status.id === stored?.statusId)?.category;
  }

  async function accept(taskId: string): Promise<void> {
    const done = project.statuses.find((status) => status.category === 'done');
    if (!done) {
      throw new Error('no done status');
    }
    await backend.moveTask(taskId, done.id, 0);
  }

  async function started(): Promise<string[]> {
    return (await backend.listConversations(project.id)).flatMap((conversation) =>
      conversation.taskId === null ? [] : [conversation.taskId],
    );
  }

  it('is off at first, with a limit of two, and starts nothing', async () => {
    await task('Draw the schematic', hardware);
    const coordinator = await backend.coordinator(project.id);
    expect(coordinator).toMatchObject({ runs: 'off', limit: 2, atWork: 0, entries: [] });
    expect(await started()).toEqual([]);
  });

  it('starts the ready tasks when it is switched on, and says why', async () => {
    const schematic = await task('Draw the schematic', hardware);
    const coordinator = await backend.switchCoordinator(project.id, true);
    expect(coordinator.runs).toBe('here');
    expect(coordinator.atWork).toBe(1);
    expect(coordinator.entries.map((entry) => entry.text)).toEqual([
      'Hardware engineer started on SSN-1: it waits for nothing.',
      'Switched on. Tasks start as soon as they are ready, with at most 2 specialists at work at once.',
    ]);
    expect(await started()).toEqual([schematic.id]);
    expect(await categoryOf(schematic.id)).toBe('active');
  });

  it('works through a chain of tasks as the manager accepts each', async () => {
    // A title that begins with Choose would make the simulated specialist ask.
    const parts = await task('List the parts', hardware);
    const schematic = await task('Draw the schematic', hardware);
    const drivers = await task('Write the drivers', firmware);
    await backend.addTaskDependency(schematic.id, parts.id);
    await backend.addTaskDependency(drivers.id, schematic.id);
    await backend.switchCoordinator(project.id, true);
    environment.passAll();
    expect(await categoryOf(parts.id)).toBe('review');
    const waiting = (await backend.coordinator(project.id)).waiting;
    expect(waiting).toEqual([
      {
        taskId: schematic.id,
        memberId: hardware.id,
        reason: 'waits_for_check',
        checkTaskId: parts.id,
        words: 'SSN-1 is finished and waits for you to accept it',
      },
    ]);

    await accept(parts.id);
    expect(await started()).toEqual([parts.id, schematic.id]);
    environment.passAll();
    await accept(schematic.id);
    environment.passAll();
    await accept(drivers.id);
    expect(await started()).toEqual([parts.id, schematic.id, drivers.id]);
    const coordinator = await backend.coordinator(project.id);
    expect(coordinator.entries[0]?.text).toBe('Firmware engineer started on SSN-3: SSN-2 is done.');
  });

  it('keeps the work in a row with a limit of one', async () => {
    await task('Draw the schematic', hardware);
    await task('Write the drivers', firmware);
    await backend.setCoordinatorLimit(project.id, 1);
    const coordinator = await backend.switchCoordinator(project.id, true);
    expect(coordinator.atWork).toBe(1);
    expect(coordinator.waiting.map((waiting) => waiting.reason)).toEqual(['limit']);
    environment.passAll();
    expect(await started()).toHaveLength(2);
    expect(await refusal(backend.setCoordinatorLimit(project.id, 9))).toMatchObject({
      code: 'rule_broken',
    });
  });

  it('stops at a used-up budget, asks once in the inbox, and goes on when it is raised', async () => {
    const first = await task('Draw the schematic', hardware);
    const second = await task('Write the drivers', firmware);
    await backend.setCoordinatorLimit(project.id, 1);
    await backend.setBudget(project.id, { scope: 'project', targetId: project.id }, 1_000);
    await backend.switchCoordinator(project.id, true);
    environment.passAll();
    expect(await started()).toEqual([first.id]);

    const [question, ...more] = await backend.listBudgetQuestions();
    expect(more).toEqual([]);
    expect(question).toMatchObject({
      projectId: project.id,
      scope: 'project',
      name: 'the budget of the project',
      tokens: 1_000,
    });
    const asked = (await backend.coordinator(project.id)).entries.filter(
      (entry) => entry.kind === 'asked',
    );
    expect(asked).toHaveLength(1);
    expect(asked[0]?.text).toMatch(
      /^Did not start SSN-2: the budget of the project is used up \([\d,]+ of 1,000 tokens\)\. You are asked in the inbox\.$/,
    );

    await backend.answerBudget(project.id, { scope: 'project', targetId: project.id }, 100_000);
    expect(await backend.listBudgetQuestions()).toEqual([]);
    expect(await started()).toEqual([first.id, second.id]);
  });

  it('leaves the work stopped when the manager says so, until the budget changes', async () => {
    await task('Draw the schematic', hardware);
    await task('Write the drivers', firmware);
    await backend.setCoordinatorLimit(project.id, 1);
    const scope = { scope: 'member' as const, targetId: hardware.id };
    await backend.setBudget(project.id, { scope: 'project', targetId: project.id }, 1_000);
    await backend.switchCoordinator(project.id, true);
    environment.passAll();
    await backend.answerBudget(project.id, { scope: 'project', targetId: project.id }, null);
    expect(await started()).toHaveLength(1);
    expect(await refusal(backend.answerBudget(project.id, scope, null))).toMatchObject({
      code: 'rule_broken',
    });
    await backend.setBudget(project.id, { scope: 'project', targetId: project.id }, null);
    expect(await started()).toHaveLength(2);
  });

  it('never tries a failed task again by itself', async () => {
    const schematic = await task('Draw the schematic', hardware);
    await backend.switchCoordinator(project.id, true);
    const [conversation] = await backend.listConversations(project.id);
    if (!conversation) {
      throw new Error('nothing started');
    }
    await backend.stopWork(conversation.id);
    environment.passAll();
    expect(await started()).toEqual([schematic.id]);
  });

  it('gives a free specialist a turn to answer a teammate, and the answer goes back', async () => {
    const seed = new DemoBackend(sampleWorkspace(environment.now()), environment);
    const [sample] = await seed.listProjects();
    if (!sample) {
      throw new Error('the sample has no project');
    }
    const team = await seed.listMembers(sample.id);
    const mechanical = team.find((member) => member.name === 'Mechanical engineer');
    const hardwareOfSample = team.find((member) => member.name === 'Hardware engineer');
    // The hardware engineer has a task to start, which brings the question
    // along; take its tasks away so that only the question is left.
    for (const listed of await seed.listTasks(sample.id)) {
      if (listed.assignee?.kind === 'member') {
        await seed.assignTask(listed.id, null);
      }
    }
    await seed.switchCoordinator(sample.id, true);
    const conversations = await seed.listConversations(sample.id);
    expect(conversations).toHaveLength(1);
    expect(conversations[0]?.memberId).toBe(hardwareOfSample?.id);
    expect(conversations[0]?.turns[0]?.request).toBe(WAKE_REQUEST);
    environment.passAll();
    const talk = await seed.teamTalk(sample.id);
    expect(talk.messages.at(-1)).toMatchObject({
      fromId: hardwareOfSample?.id,
      toId: mechanical?.id,
      kind: 'note',
    });
    const entries = (await seed.coordinator(sample.id)).entries.map((entry) => entry.text);
    expect(entries).toContain('Hardware engineer was given a turn to answer Mechanical engineer.');
  });

  it('starts the sample project where it can: the parts of the schematic and of the drivers', async () => {
    const seed = new DemoBackend(sampleWorkspace(environment.now()), environment);
    const [sample] = await seed.listProjects();
    if (!sample) {
      throw new Error('the sample has no project');
    }
    const coordinator = await seed.switchCoordinator(sample.id, true);
    const startedTexts = coordinator.entries
      .filter((entry) => entry.kind === 'started')
      .map((entry) => entry.text);
    // The newest first: the connectors of the schematic, then a driver,
    // both ready, of normal priority and first on the board.
    expect(startedTexts).toEqual([
      'Firmware engineer started on SSN-16: it waits for nothing.',
      'Hardware engineer started on SSN-15: it waits for nothing.',
    ]);
    // The rest that is ready waits for the two who are at work now.
    expect(new Set(coordinator.waiting.map((waiting) => waiting.reason))).toEqual(
      new Set(['busy']),
    );
  });

  it('starts nothing while the limit of the plan is reached, says so once, and goes on after it', async () => {
    const schematic = await task('Draw the schematic', hardware);
    const drivers = await task('Write the drivers', firmware);
    await backend.switchCoordinator(project.id, true);
    environment.pass(100);

    // The limit stops both turns; it resets in a minute.
    const resetsAt = new Date(environment.now().getTime() + 60_000).toISOString();
    backend.pretendPlanLimitReached(60_000);
    environment.pass(1_000);
    let coordinator = await backend.coordinator(project.id);
    expect(coordinator.atWork).toBe(0);
    expect(coordinator.waiting.map((waiting) => waiting.reason)).toEqual([
      'plan_limit',
      'plan_limit',
    ]);
    const told = `Started nothing: the limit of your Claude plan is reached. It resets ${resetInWords(resetsAt, environment.now())}.`;
    expect(
      coordinator.entries.filter((entry) => entry.kind === 'held').map((entry) => entry.text),
    ).toEqual([told]);
    expect(await categoryOf(schematic.id)).toBe('active');

    // Nudges while it is reached start nothing and say nothing more.
    await backend.setCoordinatorLimit(project.id, 3);
    coordinator = await backend.coordinator(project.id);
    expect(coordinator.entries.filter((entry) => entry.kind === 'held')).toHaveLength(1);

    // At the reset the work goes on in the same conversations.
    environment.pass(60_000);
    coordinator = await backend.coordinator(project.id);
    expect(
      coordinator.entries
        .slice(0, 2)
        .map((entry) => entry.text)
        .sort(),
    ).toEqual([
      'Firmware engineer went on with SSN-2: the limit of your Claude plan stopped its last turn, and it has reset.',
      'Hardware engineer went on with SSN-1: the limit of your Claude plan stopped its last turn, and it has reset.',
    ]);
    const conversations = await backend.listConversations(project.id);
    expect(conversations).toHaveLength(2);
    for (const conversation of conversations) {
      expect(conversation.turns.map((turn) => turn.request)).toHaveLength(2);
      expect(conversation.turns.at(-1)?.request).toBe(GO_ON_REQUEST);
    }
    environment.passAll();
    expect(await categoryOf(schematic.id)).toBe('review');
    expect(await categoryOf(drivers.id)).toBe('review');
  });

  it('waits for a limit whose reset is not known until it is lifted, and does not loop', async () => {
    const schematic = await task('Draw the schematic', hardware);
    await backend.switchCoordinator(project.id, true);
    environment.pass(100);
    backend.pretendPlanLimitReached(null);
    environment.pass(1_000);
    let coordinator = await backend.coordinator(project.id);
    expect(coordinator.entries[0]?.text).toBe(
      'Started nothing: the limit of your Claude plan is reached. When it resets is not known: work goes on once a turn you start by hand goes through.',
    );

    backend.liftPlanLimit();
    coordinator = await backend.coordinator(project.id);
    expect(coordinator.atWork).toBe(1);

    // The turn that went on meets the limit at once; it waits for the next reset.
    backend.pretendPlanLimitReached(null);
    environment.pass(1_000);
    const [conversation] = await backend.listConversations(project.id);
    expect(conversation?.turns).toHaveLength(2);
    coordinator = await backend.coordinator(project.id);
    expect(coordinator.waiting).toMatchObject([{ taskId: schematic.id, reason: 'plan_limit' }]);
    backend.liftPlanLimit();
    environment.pass(100);
    expect((await backend.listConversations(project.id))[0]?.turns).toHaveLength(3);
  });

  it('goes on only with work the limit stopped, not with work the manager stopped', async () => {
    const schematic = await task('Draw the schematic', hardware);
    await backend.startWork(schematic.id);
    await backend.stopWork((await backend.listConversations(project.id))[0]?.id ?? '');
    await backend.switchCoordinator(project.id, true);
    backend.pretendPlanLimitReached(null);
    backend.liftPlanLimit();
    expect(await started()).toEqual([schematic.id]);
    expect((await backend.listConversations(project.id))[0]?.turns).toHaveLength(1);
  });

  it('words tokens and raises as the program does', () => {
    expect(words.tokens(1_040_000)).toBe('1,040,000');
    expect(raisesOf(1_234_567)).toEqual([610_000, 1_200_000]);
    expect(raisesOf(1_500)).toEqual([1_000, 1_500]);
  });
});
