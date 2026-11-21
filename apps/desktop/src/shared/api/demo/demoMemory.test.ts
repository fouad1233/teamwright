import { beforeEach, describe, expect, it } from 'vitest';

import type { Member, Project, Task } from '../types';
import { DemoBackend } from './demoBackend';
import { fits, memoryInWords, searchWords } from './memory';
import { sampleWorkspace } from './sampleWorkspace';
import { refusal, testEnvironment } from './testing';

describe('the memory of a project in the DemoBackend', () => {
  let environment: ReturnType<typeof testEnvironment>;
  let backend: DemoBackend;
  let project: Project;
  let hardware: Member;
  let pins: Task;

  beforeEach(async () => {
    environment = testEnvironment();
    backend = new DemoBackend(undefined, environment);
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'engineering' });
    hardware = await backend.addMember({
      projectId: project.id,
      name: 'Hardware engineer',
      kind: 'agent',
    });
    pins = await backend.createTask({
      projectId: project.id,
      title: 'Define the pin map',
      assignee: { kind: 'member', memberId: hardware.id },
    });
  });

  it('starts empty', async () => {
    expect(await backend.projectMemory(project.id)).toEqual([]);
  });

  it('lets the manager add a fact, change it into a newer entry and remove it', async () => {
    const added = await backend.addMemoryFact({
      projectId: project.id,
      title: '  Enclosure ',
      text: 'Under 15 mm.',
      taskId: null,
    });
    expect(added).toMatchObject({ kind: 'fact', title: 'Enclosure', source: 'manager' });

    const changed = await backend.changeMemoryEntry(added.id, 'Enclosure', 'Under 12 mm.');

    const memory = await backend.projectMemory(project.id);
    expect(memory.map((entry) => [entry.text, entry.replacedBy])).toEqual([
      ['Under 12 mm.', null],
      ['Under 15 mm.', changed.id],
    ]);
    expect(await backend.searchMemory(project.id, '12')).toEqual([changed]);
    expect(await backend.searchMemory(project.id, '15')).toEqual([]);
    expect((await refusal(backend.changeMemoryEntry(added.id, 'Again', 'No.'))).code).toBe(
      'rule_broken',
    );

    await backend.removeMemoryEntry(changed.id);
    expect(await backend.searchMemory(project.id, '')).toEqual([]);
  });

  it('refuses an entry that says nothing', async () => {
    const refused = await refusal(
      backend.addMemoryFact({ projectId: project.id, title: ' ', text: 'x', taskId: null }),
    );
    expect(refused.code).toBe('rule_broken');
  });

  it('keeps an answered decision as a decision', async () => {
    const choice = await backend.createTask({
      projectId: project.id,
      title: 'Choose the battery',
      assignee: { kind: 'member', memberId: hardware.id },
    });
    const conversation = await backend.startWork(choice.id);
    environment.passAll();

    await backend.answerDecision(conversation.id, { option: 1, note: 'Keep it small.' });

    const [decision] = await backend.projectMemory(project.id);
    expect(decision).toMatchObject({
      kind: 'decision',
      title: 'Which way should I take for "Choose the battery"?',
      source: 'decision_request',
      memberId: hardware.id,
      taskId: choice.id,
      turnId: conversation.turns[0]?.id,
    });
    expect(decision?.text).toContain('Decision: The compact way');
  });

  it('lets a simulated specialist remember a fact now and then', async () => {
    const conversation = await backend.startWork(pins.id);
    environment.passAll();

    // The pin map is the first task, so its specialist remembers something.
    const memory = await backend.projectMemory(project.id);
    expect(memory).toHaveLength(1);
    expect(memory[0]).toMatchObject({
      kind: 'fact',
      source: 'specialist',
      memberId: hardware.id,
      turnId: conversation.turns[0]?.id,
      taskId: pins.id,
    });
    const steps = (await backend.listConversations(project.id))[0]?.turns[0]?.steps ?? [];
    expect(steps).toContainEqual(
      expect.objectContaining({ kind: 'tool', tool: 'mcp__teamwright__search_memory' }),
    );
  });

  it('forgets the memory of a deleted project but not of a deleted task', async () => {
    await backend.addMemoryFact({
      projectId: project.id,
      title: 'Pins',
      text: 'SPI2.',
      taskId: pins.id,
    });

    await backend.deleteTask(pins.id);
    expect(await backend.projectMemory(project.id)).toHaveLength(1);

    await backend.deleteProject(project.id);
    expect((await refusal(backend.projectMemory(project.id))).code).toBe('not_found');
  });
});

describe('the rule of a search', () => {
  it('finds an entry in which every word begins a word', () => {
    const words = searchWords('SPI, display/spi');
    expect(words).toEqual(['spi', 'display']);
    expect(fits(words, 'Display bus', 'The display is on SPI2.')).toBe(true);
    expect(fits(words, 'Display bus', 'On I2C.')).toBe(false);
    expect(fits(['play'], 'Display', '')).toBe(false);
  });

  it('refuses a search without words or with too many', () => {
    expect(() => searchWords(' ,. ')).toThrow();
    expect(() => searchWords('a b c d e f g h i j k l m')).toThrow();
  });
});

describe('the sample workspace', () => {
  it('remembers decisions, facts and contracts, one of them replaced, and its brief carries them', async () => {
    const environment = testEnvironment();
    const backend = new DemoBackend(sampleWorkspace(environment.now()), environment);
    const [project] = await backend.listProjects();
    if (!project) {
      throw new Error('the sample has no project');
    }
    const memory = await backend.projectMemory(project.id);

    expect(new Set(memory.map((entry) => entry.kind))).toEqual(
      new Set(['decision', 'fact', 'contract']),
    );
    expect(memory.filter((entry) => entry.replacedBy !== null)).toHaveLength(1);
    expect(await backend.searchMemory(project.id, 'enclosure 12')).toHaveLength(1);

    const tasks = await backend.listTasks(project.id);
    const drivers = tasks.find((task) => task.title === 'Write the sensor drivers');
    if (!drivers) {
      throw new Error('the sample has no drivers task');
    }
    const blockers = tasks.filter((task) => drivers.blockedBy.includes(task.id));
    const words = memoryInWords(project, drivers, blockers, memory, tasks) ?? '';
    expect(words).toMatch(/^From the memory of the project/);
    expect(words).toContain('Contract: Interface of SSN-2: Choose the microcontroller');
  });
});
