import { describe, expect, it } from 'vitest';

import type { Conversation } from '../types';
import { DemoBackend } from './demoBackend';
import { sampleWorkspace } from './sampleWorkspace';
import { testEnvironment } from './testing';

/** Lets the promises that are already settled call their handlers. */
async function settle(): Promise<void> {
  for (let round = 0; round < 5; round += 1) {
    await Promise.resolve();
  }
}

describe('the sample work the public demo opens with', () => {
  it('lets a specialist begin the first task in work on this computer, after the pause', async () => {
    const environment = testEnvironment();
    const workspace = sampleWorkspace(environment.now());
    const backend = new DemoBackend(workspace, environment);
    const seen: Conversation[] = [];
    backend.onConversationChanged((conversation) => seen.push(conversation));
    const [project] = await backend.listProjects();
    if (!project) {
      throw new Error('the sample has no project');
    }

    backend.beginSampleWork(1500);
    environment.pass(1499);
    await settle();
    expect(await backend.listConversations(project.id)).toHaveLength(0);

    environment.pass(1);
    await settle();
    const [conversation] = await backend.listConversations(project.id);
    const tasks = await backend.listTasks(project.id);
    const task = tasks.find((candidate) => candidate.id === conversation?.taskId);
    // "Draw the schematic", in progress and given to the hardware engineer.
    expect(task?.number).toBe(4);
    expect(conversation?.isHere).toBe(true);
    expect(conversation?.turns[0]?.state).toBe('running');
    expect(seen.some((told) => told.id === conversation?.id)).toBe(true);
  });

  it('ends with a question that waits for the manager', async () => {
    const environment = testEnvironment();
    const backend = new DemoBackend(sampleWorkspace(environment.now()), environment);

    backend.beginSampleWork(0);
    environment.passAll();
    await settle();

    const waiting = await backend.listConversationsWaiting();
    expect(waiting).toHaveLength(1);
    expect(waiting[0]?.turns.at(-1)?.decision?.question).toBe(
      'Which way should I take for "Draw the schematic"?',
    );
  });

  it('does nothing on a demo without projects', async () => {
    const environment = testEnvironment();
    const backend = new DemoBackend(undefined, environment);

    backend.beginSampleWork(0);
    environment.passAll();
    await settle();

    expect(await backend.listProjects()).toHaveLength(0);
  });

  it('does not begin a second time on the same task', async () => {
    const environment = testEnvironment();
    const backend = new DemoBackend(sampleWorkspace(environment.now()), environment);
    const [project] = await backend.listProjects();

    backend.beginSampleWork(0);
    environment.pass(0);
    await settle();
    backend.beginSampleWork(0);
    environment.pass(0);
    await settle();

    expect(await backend.listConversations(project?.id ?? '')).toHaveLength(1);
  });
});
