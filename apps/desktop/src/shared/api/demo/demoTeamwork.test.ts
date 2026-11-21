import { beforeEach, describe, expect, it } from 'vitest';

import type { Member, Project, Task } from '../types';
import { briefFor } from './brief';
import { DemoBackend } from './demoBackend';
import { sampleWorkspace } from './sampleWorkspace';
import { testEnvironment } from './testing';

describe('handoffs and messages in the DemoBackend', () => {
  let environment: ReturnType<typeof testEnvironment>;
  let backend: DemoBackend;
  let project: Project;
  let hardware: Member;
  let firmware: Member;
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
    firmware = await backend.addMember({
      projectId: project.id,
      name: 'Firmware engineer',
      kind: 'agent',
    });
    pins = await backend.createTask({
      projectId: project.id,
      title: 'Define the pin map',
      assignee: { kind: 'member', memberId: hardware.id },
    });
  });

  it('starts with nothing handed over', async () => {
    expect(await backend.teamTalk(project.id)).toEqual({ handoffs: [], messages: [] });
  });

  it('lets a simulated specialist hand its result over before it reports', async () => {
    const conversation = await backend.startWork(pins.id);
    environment.passAll();

    const { handoffs } = await backend.teamTalk(project.id);

    expect(handoffs).toHaveLength(1);
    expect(handoffs[0]).toMatchObject({
      taskId: pins.id,
      authorId: hardware.id,
      conversationId: conversation.id,
      turnId: conversation.turns[0]?.id,
      files: ['define-the-pin-map.md'],
    });
  });

  it('starts a task that builds on another with what the other handed over', async () => {
    await backend.startWork(pins.id);
    environment.passAll();
    const drivers = await backend.createTask({
      projectId: project.id,
      title: 'Write the sensor drivers',
      assignee: { kind: 'member', memberId: firmware.id },
    });
    const waiting = await backend.addTaskDependency(drivers.id, pins.id);
    const { handoffs } = await backend.teamTalk(project.id);

    const conversation = await backend.startWork(drivers.id);

    const brief = conversation.turns[0]?.request ?? '';
    expect(brief).toBe(
      briefFor(
        project,
        waiting,
        await backend.listTasks(project.id),
        handoffs,
        await backend.projectMemory(project.id),
      ),
    );
    expect(brief).toContain('What the tasks it builds on handed over:');
    expect(brief).toContain('- define-the-pin-map.md');
  });

  it('forgets the handoffs of a task that is deleted', async () => {
    await backend.startWork(pins.id);
    environment.passAll();
    await backend.stopWork((await backend.listConversations(project.id))[0]?.id ?? '');

    await backend.deleteTask(pins.id);

    expect((await backend.teamTalk(project.id)).handoffs).toEqual([]);
  });
});

describe('the sample workspace', () => {
  it('has handoffs and messages, and a waiting message comes with the next turn of its receiver', async () => {
    const environment = testEnvironment();
    const backend = new DemoBackend(sampleWorkspace(environment.now()), environment);
    const [project] = await backend.listProjects();
    if (!project) {
      throw new Error('the sample has no project');
    }
    const before = await backend.teamTalk(project.id);
    expect(before.handoffs.length).toBeGreaterThan(0);
    const waiting = before.messages.filter((message) => message.deliveredTurnId === null);
    const receiver = waiting[0]?.toId ?? '';
    expect(waiting.length).toBeGreaterThan(0);

    const conversation = await backend.ask(receiver, 'What is new?');

    const after = await backend.teamTalk(project.id);
    const delivered = after.messages.filter(
      (message) =>
        message.toId === receiver && message.deliveredTurnId === conversation.turns[0]?.id,
    );
    expect(delivered.length).toBe(waiting.filter((message) => message.toId === receiver).length);
  });
});
