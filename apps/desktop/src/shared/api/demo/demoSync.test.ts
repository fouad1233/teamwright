import { beforeEach, describe, expect, it } from 'vitest';

import type { Conversation, DataChanged, Project, SyncState, Task } from '../types';
import { DemoBackend } from './demoBackend';
import {
  IDENTITY_IN_USE,
  LAPTOP,
  LAPTOP_SHOWS_UP_AFTER,
  NEW_IDENTITY_TAKEN,
  SUGGESTED_NAME,
  THIS_COMPUTER,
} from './sync';
import { refusal, testEnvironment } from './testing';

const FOLDER = 'D:\\Cloud Drive\\Teamwright';

describe('syncing through a folder in the DemoBackend', () => {
  let environment: ReturnType<typeof testEnvironment>;
  let backend: DemoBackend;
  let project: Project;
  let schematic: Task;
  let states: SyncState[];
  let changes: DataChanged[];
  let conversations: Conversation[];

  beforeEach(async () => {
    environment = testEnvironment();
    backend = new DemoBackend(undefined, { ...environment, folders: [FOLDER] });
    // To do, In progress, In revision, In test, Done.
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'engineering' });
    const engineer = await backend.addMember({
      projectId: project.id,
      name: 'Hardware engineer',
      kind: 'agent',
    });
    await backend.createTask({ projectId: project.id, title: 'Agree on what the device must do' });
    schematic = await backend.createTask({
      projectId: project.id,
      title: 'Draw the schematic',
      assignee: { kind: 'member', memberId: engineer.id },
    });
    const inProgress = project.statuses[1];
    if (!inProgress) {
      throw new Error('the workflow has no second status');
    }
    schematic = await backend.moveTask(schematic.id, inProgress.id, 0);
    states = [];
    changes = [];
    conversations = [];
    backend.onSyncChanged((state) => states.push(state));
    backend.onDataChanged((change) => changes.push(change));
    backend.onConversationChanged((conversation) => conversations.push(conversation));
  });

  /** Chooses the folder and lets Laptop show up in it. */
  async function inStepWithLaptopWaiting(): Promise<SyncState> {
    await backend.chooseSyncFolder(FOLDER, 'Desk');
    environment.pass(LAPTOP_SHOWS_UP_AFTER);
    return backend.syncState();
  }

  describe('before a folder is chosen', () => {
    it('is off, names no folder and offers the name of the computer', async () => {
      expect(await backend.syncState()).toEqual({
        folder: null,
        thisComputer: THIS_COMPUTER,
        computerName: null,
        suggestedName: SUGGESTED_NAME,
        phase: 'off',
        lastRound: null,
        problem: null,
        problemCode: null,
        computers: [],
      });
    });

    it('refuses a round', async () => {
      const refused = await refusal(backend.syncNow());
      expect(refused.code).toBe('sync_off');
      expect(refused.message).toBe('no sync folder is chosen');
    });

    it('has no notices', async () => {
      expect(await backend.syncNotices()).toEqual([]);
    });

    it('lets the sample folder of a cloud drive be picked third', async () => {
      const browserDemo = new DemoBackend();
      await browserDemo.chooseFolder();
      await browserDemo.chooseFolder();
      expect(await browserDemo.chooseFolder()).toContain('Cloud Drive');
    });
  });

  describe('choosing the folder', () => {
    it('is in step at once, with a tidy name, after a first round', async () => {
      const state = await backend.chooseSyncFolder(FOLDER, '  My   Desk ');

      expect(state).toMatchObject({
        folder: FOLDER,
        computerName: 'My Desk',
        phase: 'in_step',
        lastRound: '2026-11-11T09:00:00.000Z',
        problem: null,
        problemCode: null,
        computers: [],
      });
      expect(states.at(-1)?.phase).toBe('in_step');
    });

    it('refuses an empty name of the computer', async () => {
      const refused = await refusal(backend.chooseSyncFolder(FOLDER, '   '));
      expect(refused.code).toBe('rule_broken');
      expect(refused.message).toBe('the name of the computer must not be empty');
      expect((await backend.syncState()).phase).toBe('off');
    });

    it('refuses a name of more than 60 characters', async () => {
      const refused = await refusal(backend.chooseSyncFolder(FOLDER, 'x'.repeat(61)));
      expect(refused.code).toBe('rule_broken');
      expect(refused.message).toBe(
        'the name of the computer can have at most 60 characters, but this one has 61',
      );
    });

    it('refuses a folder that is not named', async () => {
      const refused = await refusal(backend.chooseSyncFolder('', 'Desk'));
      expect(refused.code).toBe('rule_broken');
      expect(refused.message).toBe('the folder must be named');
    });

    it('refuses a folder that does not exist', async () => {
      const refused = await refusal(backend.chooseSyncFolder('D:\\Nowhere', 'Desk'));
      expect(refused.code).toBe('folder_not_found');
      expect(refused.message).toBe('the folder D:\\Nowhere does not exist');
    });

    it('finds a second computer, Laptop, undecided a moment later', async () => {
      await backend.chooseSyncFolder(FOLDER, 'Desk');
      expect((await backend.syncState()).computers).toEqual([]);

      environment.pass(LAPTOP_SHOWS_UP_AFTER);

      expect((await backend.syncState()).computers).toEqual([
        {
          id: LAPTOP,
          name: 'Laptop',
          version: '0.1.0',
          acceptance: 'undecided',
          lastHeard: '2026-11-11T09:00:01.500Z',
          clockAhead: false,
          newerFormat: false,
        },
      ]);
      expect(states.at(-1)?.computers).toHaveLength(1);
    });

    it('takes nothing in from a computer that is not accepted', async () => {
      await inStepWithLaptopWaiting();
      await backend.syncNow();

      expect(changes).toEqual([]);
      expect(await backend.syncNotices()).toEqual([]);
    });
  });

  describe('taking Laptop in', () => {
    it('takes in a task, renumbers the one here and tells the manager', async () => {
      await inStepWithLaptopWaiting();

      const state = await backend.acceptComputer(LAPTOP);

      expect(state.computers[0]?.acceptance).toBe('accepted');
      const tasks = await backend.listTasks(project.id);
      expect(tasks.find((task) => task.id === schematic.id)?.number).toBe(3);
      expect(tasks.find((task) => task.number === 2)?.title).toBe('Measure the current in sleep');
      const notices = await backend.syncNotices();
      expect(notices).toEqual([
        {
          id: 1,
          at: '2026-11-11T09:00:01.500Z',
          kind: 'task_renumbered',
          projectId: project.id,
          taskId: schematic.id,
          message: `${project.key}-2 is now ${project.key}-3: another computer gave the number 2 to another task at the same time.`,
        },
      ]);
      expect(changes).toEqual([{ projectIds: [project.id] }]);
    });

    it('brings a conversation that runs on Laptop, whose steps arrive as it goes on', async () => {
      await inStepWithLaptopWaiting();
      await backend.acceptComputer(LAPTOP);

      const [conversation] = await backend.listConversations(project.id);
      expect(conversation).toMatchObject({
        taskId: schematic.id,
        homeComputer: LAPTOP,
        isHere: false,
      });
      expect(conversations.at(-1)?.isHere).toBe(false);

      environment.passAll();

      const [later] = await backend.listConversations(project.id);
      expect(later?.turns[0]?.steps.length).toBeGreaterThan(0);
      expect(later?.turns[0]?.decision?.answer).toBeNull();
      const waiting = await backend.listConversationsWaiting();
      expect(waiting.map((each) => each.isHere)).toEqual([false]);
    });

    it('marks the oldest conversation of the project as running on Laptop, if it has one', async () => {
      const here = await backend.startWork(schematic.id);
      expect(here).toMatchObject({ homeComputer: THIS_COMPUTER, isHere: true });
      await inStepWithLaptopWaiting();

      await backend.acceptComputer(LAPTOP);

      const listed = await backend.listConversations(project.id);
      expect(listed.map((each) => [each.id, each.isHere])).toEqual([[here.id, false]]);
    });

    it('takes in only once', async () => {
      await inStepWithLaptopWaiting();
      await backend.acceptComputer(LAPTOP);
      await backend.syncNow();

      expect(await backend.syncNotices()).toHaveLength(1);
      expect(changes).toHaveLength(1);
      expect((await backend.syncState()).computers[0]?.lastHeard).toBe('2026-11-11T09:00:01.500Z');
    });

    it('refuses to continue a conversation that runs on Laptop', async () => {
      await inStepWithLaptopWaiting();
      await backend.acceptComputer(LAPTOP);
      environment.passAll();
      const [conversation] = await backend.listConversations(project.id);
      if (!conversation) {
        throw new Error('there is no conversation');
      }

      for (const request of [
        backend.reply(conversation.id, 'Go on.'),
        backend.answerDecision(conversation.id, { option: 0, note: '' }),
        backend.answerInterview(conversation.id, []),
        backend.approveProposal(conversation.id, { members: [], tasks: [] }),
        backend.stopWork(conversation.id),
      ]) {
        const refused = await refusal(request);
        expect(refused.code).toBe('runs_elsewhere');
        expect(refused.message).toBe(
          'this conversation runs on another computer, and can only be continued there',
        );
      }
    });

    it('can ignore Laptop instead', async () => {
      await inStepWithLaptopWaiting();

      const state = await backend.ignoreComputer(LAPTOP);

      expect(state.computers[0]?.acceptance).toBe('ignored');
      expect(await backend.listConversations(project.id)).toEqual([]);
    });

    it('refuses a computer it does not know', async () => {
      await inStepWithLaptopWaiting();
      for (const request of [
        backend.acceptComputer('0196f3a2-0000-0000-0000-000000000000'),
        backend.ignoreComputer('0196f3a2-0000-0000-0000-000000000000'),
      ]) {
        const refused = await refusal(request);
        expect(refused.code).toBe('not_found');
        expect(refused.message).toBe(
          'there is no computer with the identifier 0196f3a2-0000-0000-0000-000000000000',
        );
      }
    });
  });

  describe('the notices', () => {
    it('forgets a notice that is dismissed, and ignores one that is not kept', async () => {
      await inStepWithLaptopWaiting();
      await backend.acceptComputer(LAPTOP);

      await backend.dismissSyncNotice(1);
      await backend.dismissSyncNotice(99);

      expect(await backend.syncNotices()).toEqual([]);
    });
  });

  describe('the name of this computer', () => {
    it('can change, tidied', async () => {
      await backend.chooseSyncFolder(FOLDER, 'Desk');
      expect((await backend.renameThisComputer(' Big  desk ')).computerName).toBe('Big desk');
    });

    it('refuses an empty name', async () => {
      const refused = await refusal(backend.renameThisComputer(''));
      expect(refused.code).toBe('rule_broken');
      expect(refused.message).toBe('the name of the computer must not be empty');
    });
  });

  describe('stopping', () => {
    it('is off again, and keeps the computers it knows', async () => {
      await inStepWithLaptopWaiting();

      const state = await backend.stopSyncing();

      expect(state).toMatchObject({ folder: null, phase: 'off', lastRound: null, problem: null });
      expect(state.computers).toHaveLength(1);
      expect((await refusal(backend.syncNow())).code).toBe('sync_off');
    });
  });

  describe('trouble a scenario can pretend', () => {
    it('shows a round under way until the next round', async () => {
      await backend.chooseSyncFolder(FOLDER, 'Desk');

      expect((await backend.pretendSync('writing')).phase).toBe('writing');
      expect(states.at(-1)?.phase).toBe('writing');
      expect((await backend.syncNow()).phase).toBe('in_step');
    });

    it('shows a folder that cannot be reached, and refuses rounds until it is back', async () => {
      await backend.chooseSyncFolder(FOLDER, 'Desk');
      environment.pass(60_000);

      const state = await backend.pretendSync('unreachable');

      expect(state).toMatchObject({
        phase: 'unreachable',
        problem: `the sync folder ${FOLDER} cannot be reached`,
        problemCode: 'sync_folder_unreachable',
        lastRound: '2026-11-11T09:01:00.000Z',
      });
      const refused = await refusal(backend.syncNow());
      expect(refused.code).toBe('sync_folder_unreachable');
      expect(states.at(-1)?.phase).toBe('unreachable');

      expect((await backend.pretendSync(null)).phase).toBe('in_step');
    });

    it('stops when another computer uses the identity of this one', async () => {
      await backend.chooseSyncFolder(FOLDER, 'Desk');

      const state = await backend.pretendSync('identity_in_use');

      expect(state).toMatchObject({
        phase: 'stopped',
        problem: IDENTITY_IN_USE,
        problemCode: 'identity_in_use',
      });
      expect((await refusal(backend.syncNow())).code).toBe('identity_in_use');
    });

    it('lets another computer show up, with a clock that is ahead', async () => {
      await inStepWithLaptopWaiting();

      const state = await backend.pretendComputer('Workshop', { clockAhead: true });

      expect(state.computers.map((computer) => [computer.name, computer.clockAhead])).toEqual([
        ['Laptop', false],
        ['Workshop', true],
      ]);
      expect(state.computers[1]?.acceptance).toBe('undecided');
    });
  });

  describe('a new identity', () => {
    it('stops syncing until Teamwright starts again', async () => {
      await backend.chooseSyncFolder(FOLDER, 'Desk');
      await backend.pretendSync('identity_in_use');

      const state = await backend.giveThisComputerANewIdentity();

      expect(state).toMatchObject({
        phase: 'stopped',
        problem: NEW_IDENTITY_TAKEN,
        problemCode: 'restart_needed',
      });
      const refused = await refusal(backend.syncNow());
      expect(refused.code).toBe('identity_in_use');
      expect(refused.message).toBe(IDENTITY_IN_USE);
      expect((await backend.syncState()).problem).toBe(NEW_IDENTITY_TAKEN);
    });
  });
});
