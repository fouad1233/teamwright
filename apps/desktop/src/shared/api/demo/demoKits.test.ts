import { beforeEach, describe, expect, it } from 'vitest';

import type { Member, NewConnector, Project } from '../types';
import { DemoBackend } from './demoBackend';
import { sampleWorkspace } from './sampleWorkspace';
import { refusal, testEnvironment } from './testing';

/*
 * The library of skills and connectors in the demo, as
 * `crates/application/src/kits/tests.rs` tests it for the program.
 */
describe('the library of skills and connectors in the DemoBackend', () => {
  let backend: DemoBackend;
  let project: Project;
  let specialist: Member;

  const parts: NewConnector = {
    name: 'parts',
    start: { kind: 'program', program: 'parts-server', arguments: [], variables: ['PARTS_TOKEN'] },
    tools: ['search', 'order'],
  };

  beforeEach(async () => {
    backend = new DemoBackend(undefined, testEnvironment());
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'standard' });
    specialist = await backend.addMember({
      projectId: project.id,
      name: 'Hardware engineer',
      kind: 'agent',
    });
  });

  it('adds a skill from a folder and refuses one that runs commands', async () => {
    const library = await backend.addSkill(project.id, 'C:\\Users\\you\\skills\\release-notes');
    expect(library.skills.map((skill) => skill.name)).toEqual(['release-notes']);
    expect(library.skills[0]?.here).toBe(true);

    const runs = await refusal(
      backend.addSkill(project.id, 'C:\\Users\\you\\skills\\deploy-helper'),
    );
    expect(runs.code).toBe('rule_broken');
    expect(runs.message).toContain('runs a command');
    const twice = await refusal(
      backend.addSkill(project.id, 'C:\\Users\\you\\skills\\release-notes'),
    );
    expect(twice.message).toContain('already has');
  });

  it('keeps a connector with the names of its variables and never a secret in the address', async () => {
    const library = await backend.addConnector(project.id, parts);
    expect(library.connectors[0]?.missingVariables).toEqual([]);

    const secret = await refusal(
      backend.addConnector(project.id, {
        name: 'tickets',
        start: { kind: 'address', url: 'https://t.example/mcp?token=hunter2', headers: [] },
        tools: ['read'],
      }),
    );
    expect(secret.code).toBe('rule_broken');
    expect(secret.message).not.toContain('hunter2');

    for (const program of ['./parts-server', 'bin/parts-server', '..\\parts.exe']) {
      const relative = await refusal(
        backend.addConnector(project.id, {
          ...parts,
          name: 'relative',
          start: { kind: 'program', program, arguments: [], variables: [] },
        }),
      );
      expect(relative.message).toContain('relative path');
    }
    const full = await backend.addConnector(project.id, {
      ...parts,
      name: 'full',
      start: { kind: 'program', program: 'C:\\Tools\\parts.exe', arguments: [], variables: [] },
    });
    expect(full.connectors.map((connector) => connector.problem)).toEqual([null, null]);
    const claude = await refusal(
      backend.addConnector(project.id, {
        ...parts,
        name: 'other',
        start: { kind: 'program', program: 'x', arguments: [], variables: ['ANTHROPIC_API_KEY'] },
      }),
    );
    expect(claude.message).toContain('belongs to Claude Code');
  });

  it('gives a specialist a connector tool by tool, and takes it back with the library', async () => {
    const added = await backend.addConnector(project.id, parts);
    const connectorId = added.connectors[0]?.id ?? '';

    const given = await backend.setMemberKit(project.id, specialist.id, {
      skills: [],
      connectors: [{ connectorId, tools: ['search'] }],
    });
    expect(given.kits).toEqual([
      { memberId: specialist.id, skills: [], connectors: [{ connectorId, tools: ['search'] }] },
    ]);

    const unknown = await refusal(
      backend.setMemberKit(project.id, specialist.id, {
        skills: [],
        connectors: [{ connectorId, tools: ['delete_everything'] }],
      }),
    );
    expect(unknown.message).toContain('delete_everything');

    const after = await backend.removeConnector(project.id, connectorId);
    expect(after.kits[0]?.connectors).toEqual([]);
  });

  it('confirms here what is added and given here, and nothing that changed unseen', async () => {
    const added = await backend.addConnector(project.id, parts);
    const connector = added.connectors[0];
    expect(connector?.confirmed).toBe(true);
    const given = await backend.setMemberKit(project.id, specialist.id, {
      skills: [],
      connectors: [{ connectorId: connector?.id ?? '', tools: ['search'] }],
    });
    expect(given.waitingTools).toEqual([]);

    const stale = await refusal(
      backend.confirmConnector(project.id, connector?.id ?? '', 'not what was shown'),
    );
    expect(stale.message).toContain('changed since you looked at it');
  });
});

describe('the sample library of the DemoBackend', () => {
  it('holds a connector and a tool that came from another computer and wait here', async () => {
    const backend = new DemoBackend(
      sampleWorkspace(new Date('2026-11-21T09:00:00.000Z')),
      testEnvironment(),
    );
    const projects = await backend.listProjects();
    const libraries = await Promise.all(
      projects.map((project) => backend.projectLibrary(project.id)),
    );
    const index = libraries.findIndex((library) =>
      library.connectors.some((connector) => !connector.confirmed),
    );
    const library = libraries[index];
    const project = projects[index];
    if (!library || !project) throw new Error('the sample library has no waiting connector');
    const waiting = library.connectors.find((connector) => !connector.confirmed);
    expect(waiting?.name).toBe('drawings');
    expect(library.waitingTools).toEqual([
      expect.objectContaining({ connectorId: waiting?.id, tools: ['find_drawing'] }),
    ]);

    const confirmed = await backend.confirmConnector(
      project.id,
      waiting?.id ?? '',
      waiting?.fingerprint ?? '',
    );
    expect(confirmed.connectors.every((connector) => connector.confirmed)).toBe(true);
    const entry = confirmed.waitingTools[0];
    const kits = await backend.confirmMemberKit(project.id, entry?.memberId ?? '', {
      skills: [],
      connectors: [{ connectorId: entry?.connectorId ?? '', tools: entry?.tools ?? [] }],
    });
    expect(kits.waitingTools).toEqual([]);
  });
});
