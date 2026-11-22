import { beforeEach, describe, expect, it } from 'vitest';

import type { Member, Project, Task } from '../types';
import { DemoBackend } from './demoBackend';
import { checkedRule } from './permissions';
import { sampleWorkspace } from './sampleWorkspace';
import { DEMO_COMMAND } from './simulatedSpecialist';
import { refusal, testEnvironment } from './testing';

describe('permission requests in the DemoBackend', () => {
  let environment: ReturnType<typeof testEnvironment>;
  let backend: DemoBackend;
  let project: Project;
  let engineer: Member;
  let check: Task;

  beforeEach(async () => {
    environment = testEnvironment();
    backend = new DemoBackend(undefined, environment);
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'engineering' });
    engineer = await backend.addMember({
      projectId: project.id,
      name: 'Hardware engineer',
      kind: 'agent',
    });
    check = await backend.createTask({
      projectId: project.id,
      title: 'Check the power budget',
      assignee: { kind: 'member', memberId: engineer.id },
    });
  });

  async function asked() {
    await backend.startWork(check.id);
    environment.passAll();
    const [request] = await backend.openPermissionRequests();
    if (!request) {
      throw new Error('nothing was asked');
    }
    return request;
  }

  it('a specialist that has to check something asks to run a command, and waits', async () => {
    const request = await asked();

    expect(request).toMatchObject({
      projectId: project.id,
      memberId: engineer.id,
      taskId: check.id,
      kind: 'command',
      what: DEMO_COMMAND,
      answer: null,
    });
    const [conversation] = await backend.listConversations(project.id);
    expect(conversation?.turns[0]?.steps.some((step) => step.kind === 'notice')).toBe(true);
    // The work waits for the manager: the task stays in work.
    const task = (await backend.listTasks(project.id)).find((listed) => listed.id === check.id);
    expect(project.statuses.find((status) => status.id === task?.statusId)?.category).toBe(
      'active',
    );
  });

  it('an allowed request starts the next turn with the answer, and runs the command', async () => {
    const request = await asked();

    const conversation = await backend.answerPermission(request.id, {
      decision: 'allow',
      scope: 'once',
      kind: 'command',
      what: request.what,
      note: '',
    });
    expect(conversation.turns[1]?.request).toMatch(
      /^Permission: the manager allowed you to run the command `python3 tools\/check.py`, for this turn only\./,
    );
    environment.passAll();
    const [after] = await backend.listConversations(project.id);
    expect(
      after?.turns[1]?.steps.some((step) => step.kind === 'tool' && step.tool === 'Bash'),
    ).toBe(true);
    expect(await backend.openPermissionRequests()).toEqual([]);
  });

  it('a refused request says so, and nothing runs', async () => {
    const request = await asked();

    const conversation = await backend.answerPermission(request.id, {
      decision: 'refuse',
      note: 'Not now.',
    });
    expect(conversation.turns[1]?.request).toMatch(/refused your request/);
    expect(conversation.turns[1]?.request).toMatch(/The manager adds: Not now\.$/);
    environment.passAll();
    const [after] = await backend.listConversations(project.id);
    expect(
      after?.turns[1]?.steps.some((step) => step.kind === 'tool' && step.tool === 'Bash'),
    ).toBe(false);
  });

  it('refuses to allow a rule that could never be allowed, and keeps the request open', async () => {
    const request = await asked();

    const refused = await refusal(
      backend.answerPermission(request.id, {
        decision: 'allow',
        scope: 'once',
        kind: 'command',
        what: '"claude" -p x',
        note: '',
      }),
    );
    expect(refused.code).toBe('rule_broken');
    expect(await backend.openPermissionRequests()).toHaveLength(1);
  });

  it('never allows a command always', async () => {
    const request = await asked();

    const refused = await refusal(
      backend.answerPermission(request.id, {
        decision: 'allow',
        scope: 'always',
        kind: 'command',
        what: request.what,
        note: '',
      }),
    );
    expect(refused.message).toMatch(/never always/);
    expect(await backend.openPermissionRequests()).toHaveLength(1);
  });

  it('gives no command to a specialist with a secret in reach', async () => {
    const request = await asked();
    const library = await backend.addConnector(project.id, {
      name: 'parts-catalog',
      start: {
        kind: 'program',
        program: 'parts-catalog-server',
        arguments: [],
        variables: ['PARTS_TOKEN'],
      },
      tools: ['search_parts'],
    });
    const parts = library.connectors.find((connector) => connector.name === 'parts-catalog');
    if (!parts) {
      throw new Error('the connector was not added');
    }
    await backend.setMemberKit(project.id, engineer.id, {
      skills: [],
      connectors: [{ connectorId: parts.id, tools: ['search_parts'] }],
    });

    const refused = await refusal(
      backend.answerPermission(request.id, {
        decision: 'allow',
        scope: 'once',
        kind: 'command',
        what: request.what,
        note: '',
      }),
    );
    expect(refused.message).toMatch(/secret in an environment variable/);
    expect(await backend.openPermissionRequests()).toHaveLength(1);
  });

  it('lists a standing permission until it is taken back', async () => {
    const request = await asked();
    await backend.answerPermission(request.id, {
      decision: 'allow',
      scope: 'task',
      kind: 'command',
      what: request.what,
      note: '',
    });
    const [standing] = await backend.permissionRequests(project.id);
    expect(standing?.answer).toMatchObject({ decision: 'allowed', scope: 'task' });

    const withdrawn = await backend.withdrawPermission(request.id);
    expect(withdrawn.answer?.withdrawnAt).not.toBeNull();
    expect((await refusal(backend.withdrawPermission(request.id))).code).toBe('rule_broken');
  });

  it('the sample project has standing permissions to show, and no command allowed always', async () => {
    const sample = new DemoBackend(sampleWorkspace(), testEnvironment());
    const [project] = await sample.listProjects();
    const requests = await sample.permissionRequests(project?.id ?? '');
    expect(requests.map((request) => [request.kind, request.answer?.scope])).toEqual([
      ['command', 'task'],
      ['web_domain', 'always'],
    ]);
  });
});

describe('the rules of a permission in the demo', () => {
  it('tidies a command and a domain', () => {
    expect(checkedRule('command', '  npm   test ')).toBe('npm test');
    expect(checkedRule('web_domain', 'Docs.RS')).toBe('docs.rs');
  });

  it('refuses what would allow more than it shows', () => {
    for (const [kind, what] of [
      ['command', 'npm test && rm -rf .'],
      ['command', 'cat /etc/passwd'],
      ['command', 'sudo ls'],
      ['command', 'rm -rf build'],
      ['command', '"claude" --dangerously-skip-permissions -p x'],
      ['command', "'sudo' ls"],
      ['command', '"rm" -r src'],
      ['command', 'py""thon3 -c x'],
      ['command', 'cat .cla""ude/settings.json'],
      ['command', 'git log -1 --output=/home/u/x'],
      ['command', 'gcc -o/home/u/x main.c'],
      ['command', 'tool --out=C:/x'],
      ['command', 'npm test\nls'],
      ['command', 'npm\ttest'],
      ['command', 'npm t\u0435st'],
      ['command', 'env FOO=1 npm test'],
      ['command', 'git -c core.editor=claude commit'],
      ['command', 'git log main..HEAD'],
      ['command_prefix' as 'command', 'npm run'],
      ['web_domain', 'localhost'],
      ['web_domain', 'https://docs.rs'],
    ] as const) {
      expect(() => checkedRule(kind, what)).toThrow();
    }
  });
});
