import { beforeEach, describe, expect, it } from 'vitest';

import type { Member, Project, Task } from '../types';
import { WAKE_REQUEST, raisesOf, words } from './coordinator';
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

  it('words tokens and raises as the program does', () => {
    expect(words.tokens(1_040_000)).toBe('1,040,000');
    expect(raisesOf(1_234_567)).toEqual([610_000, 1_200_000]);
    expect(raisesOf(1_500)).toEqual([1_000, 1_500]);
  });
});
