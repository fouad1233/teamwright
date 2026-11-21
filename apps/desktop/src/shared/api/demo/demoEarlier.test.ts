import { beforeEach, describe, expect, it } from 'vitest';

import type { Project } from '../types';
import { DemoBackend } from './demoBackend';
import { sampleEarlierConversations, type PretendedConversation } from './earlierConversations';
import { refusal, testEnvironment } from './testing';

const TODAY = new Date('2026-11-11T09:00:00.000Z');

describe('earlier conversations and their digests in the DemoBackend', () => {
  let environment: ReturnType<typeof testEnvironment>;
  let backend: DemoBackend;
  let project: Project;

  function withConversations(conversations: PretendedConversation[]) {
    environment = testEnvironment();
    backend = new DemoBackend(
      { projects: [], tasks: [], members: [], earlierConversations: conversations },
      environment,
    );
  }

  beforeEach(async () => {
    withConversations(sampleEarlierConversations(TODAY));
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'engineering' });
  });

  it('finds nothing on a computer where Claude Code was never used', async () => {
    withConversations([]);
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'engineering' });

    expect(await backend.findEarlierConversations(project.id)).toEqual([]);
  });

  it('lists the folders of the project first, then the others with the newest first', async () => {
    const found = await backend.findEarlierConversations(project.id);

    expect(found.map((folder) => [folder.folder, folder.isOfProject])).toEqual([
      // Both are the project's; the one with the newer conversation first.
      ['C:\\Users\\you\\Documents\\Teamwright\\SSN\\firmware', true],
      ['C:\\Users\\you\\Documents\\Teamwright\\SSN', true],
      ['C:\\Users\\you', false],
      ['C:\\Users\\you\\Projects\\website', false],
      ['C:\\Users\\you\\Projects\\weather-station', false],
    ]);
    // Inside a folder, the newest first.
    expect(found[1]?.conversations.map((conversation) => conversation.title)).toEqual([
      null,
      'Choosing the sensors for the air node',
    ]);
  });

  it('makes a digest by the rules and hides what looks like a secret', async () => {
    const draft = await backend.draftDigest(
      project.id,
      '-Users-you-Documents-Teamwright-SSN-firmware/2a7f3c88-firmware-start',
    );

    expect(draft.title).toBe('First firmware for the ESP32-C3');
    expect(draft.text).toContain('First request:\nSet up a PlatformIO project');
    expect(draft.text).toContain('2. Use pins 4 and 5 for I2C.');
    expect(draft.text).toContain('WIFI_PASSWORD=[hidden]');
    expect(draft.text).not.toContain('hunter2');
    expect(draft.text).toContain('Files written or changed (4):\n- platformio.ini\n- src/main.cpp');
    expect(draft.text).toContain('Last answer:\nThe Wi-Fi settings');
  });

  it('keeps a digest as the manager corrected it, once, and removes it', async () => {
    const source = '-Users-you-Projects-website/0e66d2b9-site';
    const draft = await backend.draftDigest(project.id, source);

    const kept = await backend.keepDigest(project.id, {
      ...draft,
      title: ' The website ',
      text: 'We made a plain page.\r\n',
    });

    expect(kept).toMatchObject({ title: 'The website', text: 'We made a plain page.', source });
    expect(await backend.listDigests(project.id)).toEqual([kept]);
    const found = await backend.findEarlierConversations(project.id);
    const listed = found.flatMap((folder) => folder.conversations);
    expect(listed.find((conversation) => conversation.source === source)?.isKept).toBe(true);

    const again = await refusal(backend.keepDigest(project.id, draft));
    expect(again.code).toBe('digest_already_kept');

    await backend.removeDigest(kept.id);
    expect(await backend.listDigests(project.id)).toEqual([]);
    expect((await refusal(backend.removeDigest(kept.id))).code).toBe('not_found');
  });

  it('refuses an empty digest', async () => {
    const draft = await backend.draftDigest(project.id, '-Users-you/d3f4e5a6-question');

    const refused = await refusal(backend.keepDigest(project.id, { ...draft, text: ' \n ' }));

    expect(refused.code).toBe('rule_broken');
  });

  it('lets the simulated Advisor say that it read the digests', async () => {
    const draft = await backend.draftDigest(project.id, '-Users-you/d3f4e5a6-question');
    await backend.keepDigest(project.id, draft);

    const conversation = await backend.advise(project.id, { brief: 'A small air sensor.' });
    environment.passAll();

    const [listed] = await backend.listConversations(project.id);
    const said = listed?.turns[0]?.steps.flatMap((step) =>
      step.kind === 'text' ? [step.text] : [],
    );
    expect(conversation.turns[0]?.request).toBe('A small air sensor.');
    expect(said).toContain(
      'I have read the account of the earlier conversation you imported. I will ask where it may be out of date.',
    );
  });
});
