import { beforeEach, describe, expect, it } from 'vitest';

import { BackendError } from '../backend';
import type { Project } from '../types';
import { DemoBackend } from './demoBackend';

async function refusal(request: Promise<unknown>): Promise<BackendError> {
  try {
    await request;
  } catch (failure) {
    if (failure instanceof BackendError) {
      return failure;
    }
    throw failure;
  }
  throw new Error('the request was expected to be refused, but it succeeded');
}

describe('the files of a project in the DemoBackend', () => {
  let backend: DemoBackend;
  let project: Project;
  let workFolder: string;

  beforeEach(async () => {
    backend = new DemoBackend(undefined, {
      now: () => new Date('2026-11-12T09:00:00.000Z'),
      newId: (() => {
        let identifiers = 0;
        return () => {
          identifiers += 1;
          return `id-${identifiers.toString()}`;
        };
      })(),
      later: () => undefined,
    });
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'engineering' });
    workFolder = (await backend.projectPlaces(project.id)).workFolder;
  });

  it('lists the folders first, without the noise of the tools', async () => {
    const listing = await backend.listProjectFiles(project.id, workFolder, '', false);

    expect(listing.entries.map((entry) => entry.name)).toEqual([
      'docs',
      'firmware',
      'hardware',
      'references',
      'README.md',
      'vendor-libraries',
    ]);
    expect(listing.entries.find((entry) => entry.name === 'vendor-libraries')?.kind).toBe('link');
    expect(listing.hasMore).toBe(false);
  });

  it('shows hidden names when asked, and never the folder of git', async () => {
    const names = (await backend.listProjectFiles(project.id, workFolder, '', true)).entries.map(
      (entry) => entry.name,
    );

    expect(names).toContain('.gitignore');
    expect(names).not.toContain('.git');
  });

  it('goes inside a folder and gives a file its size and when it changed', async () => {
    const listing = await backend.listProjectFiles(project.id, workFolder, 'firmware/src', false);

    expect(listing.entries[0]).toEqual({
      name: 'bme680.c',
      kind: 'file',
      size: 11_532,
      changedAt: '2026-11-12T06:00:00.000Z',
    });
  });

  it('refuses a path that leaves the folder and a folder that is not of the project', async () => {
    for (const path of ['..', 'firmware/../..', 'C:/Windows', '/etc', 'vendor-libraries']) {
      const failure = await refusal(backend.listProjectFiles(project.id, workFolder, path, false));
      expect(failure.code).toBe('rule_broken');
    }
    const elsewhere = await refusal(backend.listProjectFiles(project.id, 'C:\\Windows', '', false));
    expect(elsewhere.code).toBe('rule_broken');
    const missing = await refusal(backend.listProjectFiles(project.id, workFolder, 'gone', false));
    expect(missing.code).toBe('not_found');
  });

  it('has no program to open a file with and no file manager to show it in', async () => {
    const opened = await refusal(backend.openProjectFile(project.id, workFolder, 'README.md'));
    const shown = await refusal(backend.showProjectFile(project.id, workFolder, 'README.md'));

    expect(opened.code).toBe('not_available');
    expect(shown.code).toBe('not_available');
  });
});
