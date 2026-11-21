import { beforeEach, describe, expect, it } from 'vitest';

import type { Project } from '../types';
import { DemoBackend } from './demoBackend';
import { refusal, testEnvironment } from './testing';

/*
 * The instructions of a specialist and the ready-made specialists in the
 * demo, as `crates/application/src/team.rs` tests them for the program:
 * the same cases, with the same codes and messages.
 */
describe('instructions and ready-made specialists in the DemoBackend', () => {
  let backend: DemoBackend;
  let project: Project;

  beforeEach(async () => {
    backend = new DemoBackend(undefined, testEnvironment());
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'standard' });
  });

  it('offers about a dozen ready-made specialists, hardware as well as software', async () => {
    const specialists = await backend.listReadyMadeSpecialists();

    expect(specialists.length).toBeGreaterThanOrEqual(10);
    expect(specialists.length).toBeLessThanOrEqual(14);
    const names = specialists.map((specialist) => specialist.name);
    expect(names).toContain('Firmware engineer');
    expect(names).toContain('Electronics designer');
    expect(names).toContain('Front-end developer');
    for (const specialist of specialists) {
      expect(specialist.instructions.length).toBeGreaterThan(600);
      expect(specialist.description).not.toBe('');
    }
  });

  it('gives a member added by hand no instructions', async () => {
    const member = await backend.addMember({
      projectId: project.id,
      name: 'Hardware engineer',
      kind: 'agent',
    });

    expect(member.instructions).toBe('');
  });

  it('gives a specialist instructions of its own, tidied', async () => {
    const member = await backend.addMember({
      projectId: project.id,
      name: 'Hardware engineer',
      kind: 'agent',
    });

    const changed = await backend.setMemberInstructions(
      member.id,
      '  Check every pin.\r\nNever guess.  \n',
    );

    expect(changed.instructions).toBe('Check every pin.\nNever guess.');
    const [stored] = await backend.listMembers(project.id);
    expect(stored?.instructions).toBe('Check every pin.\nNever guess.');
    expect((await backend.setMemberInstructions(member.id, ' \n ')).instructions).toBe('');
  });

  it('refuses instructions longer than the limit', async () => {
    const member = await backend.addMember({
      projectId: project.id,
      name: 'Hardware engineer',
      kind: 'agent',
    });

    const error = await refusal(backend.setMemberInstructions(member.id, 'x'.repeat(4001)));

    expect(error.code).toBe('rule_broken');
    expect(error.message).toContain('at most 4000 characters');
  });

  it('gives a person no instructions', async () => {
    const dana = await backend.addMember({ projectId: project.id, name: 'Dana', kind: 'human' });

    const error = await refusal(backend.setMemberInstructions(dana.id, 'Be quick.'));

    expect(error.code).toBe('no_specialist');
    expect(error.message).toBe('Dana is not an AI specialist');
    await backend.setMemberInstructions(dana.id, '');
  });

  it('puts a ready-made specialist on the team with everything of its role', async () => {
    const role = (await backend.listReadyMadeSpecialists()).find(
      (specialist) => specialist.key === 'firmware-engineer',
    );

    const member = await backend.addReadyMadeMember(project.id, 'firmware-engineer');

    expect(member.name).toBe('Firmware engineer');
    expect(member.kind).toBe('agent');
    expect(member.responsibility).toBe(role?.description);
    expect(member.instructions).toBe(role?.instructions);
    expect(member.model).toBe(role?.model);
    expect(member.effort).toBe(role?.effort);
  });

  it('numbers a second ready-made specialist of one role', async () => {
    await backend.addMember({ projectId: project.id, name: 'RESEARCHER', kind: 'agent' });
    await backend.addMember({ projectId: project.id, name: 'Researcher 2', kind: 'agent' });

    const third = await backend.addReadyMadeMember(project.id, 'researcher');

    expect(third.name).toBe('Researcher 3');
  });

  it('refuses a ready-made specialist it does not know', async () => {
    const error = await refusal(backend.addReadyMadeMember(project.id, 'astronaut'));

    expect(error.code).toBe('not_found');
    expect(await backend.listMembers(project.id)).toEqual([]);
  });
});
