import { beforeEach, describe, expect, it } from 'vitest';

import { BackendError } from '../backend';
import type { Project } from '../types';
import { DemoBackend, type DemoEnvironment } from './demoBackend';
import { freeName, isInside, isSameFolder, nameOf } from './places';

/** The folders that exist on the computer these tests pretend to run on. */
const FOLDERS = [
  'C:\\Work',
  'C:\\Work\\sensor-node',
  'C:\\Work\\sensor-node\\docs',
  'D:\\Datasheets',
  'D:\\Datasheets\\Bosch',
];

function testEnvironment(): DemoEnvironment {
  let identifiers = 0;
  return {
    now: () => new Date('2026-11-12T09:00:00.000Z'),
    newId: () => {
      identifiers += 1;
      return `id-${identifiers.toString()}`;
    },
    // Nothing that is put off ever happens: a specialist that starts keeps working.
    later: () => undefined,
    folders: FOLDERS,
    pickFolder: () => 'D:\\Datasheets',
    pickFiles: () => ['D:\\Datasheets\\Bosch\\BME680.pdf'],
  };
}

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

describe('the folders of a project in the DemoBackend', () => {
  let backend: DemoBackend;
  let project: Project;

  beforeEach(async () => {
    backend = new DemoBackend(undefined, testEnvironment());
    project = await backend.createProject({ name: 'Smart sensor node', workflow: 'engineering' });
  });

  it('gives every project a work folder of its own, named by its key', async () => {
    expect(await backend.projectPlaces(project.id)).toEqual({
      workFolder: 'C:\\Users\\you\\Documents\\Teamwright\\SSN',
      isWorkFolderChosen: false,
      referenceFolders: [],
    });
  });

  it('lets the manager choose the work folder, and go back to the one Teamwright gives', async () => {
    const chosen = await backend.setWorkFolder(project.id, '  C:\\Work\\sensor-node ');
    expect(chosen.workFolder).toBe('C:\\Work\\sensor-node');
    expect(chosen.isWorkFolderChosen).toBe(true);

    const back = await backend.setWorkFolder(project.id, null);
    expect(back.workFolder).toBe('C:\\Users\\you\\Documents\\Teamwright\\SSN');
    expect(back.isWorkFolderChosen).toBe(false);
  });

  it('refuses a work folder that does not exist, and one without a name', async () => {
    const missing = await refusal(backend.setWorkFolder(project.id, 'C:\\Nowhere'));
    expect(missing.code).toBe('folder_not_found');
    expect(missing.message).toBe('the folder C:\\Nowhere does not exist');

    const unnamed = await refusal(backend.setWorkFolder(project.id, '   '));
    expect(unnamed.code).toBe('rule_broken');
    expect(unnamed.message).toBe('the folder must be named');
  });

  it('does not change the work folder while a specialist works in the project', async () => {
    const engineer = await backend.addMember({
      projectId: project.id,
      name: 'Hardware engineer',
      kind: 'agent',
    });
    const task = await backend.createTask({
      projectId: project.id,
      title: 'Draw the schematic',
      assignee: { kind: 'member', memberId: engineer.id },
    });
    await backend.startWork(task.id);

    const error = await refusal(backend.setWorkFolder(project.id, 'C:\\Work\\sensor-node'));

    expect(error.code).toBe('work_in_progress');
    expect((await backend.projectPlaces(project.id)).isWorkFolderChosen).toBe(false);
  });

  it('lets the specialists read a reference folder, and lists it once', async () => {
    await backend.addReferenceFolder(project.id, 'D:\\Datasheets');
    const places = await backend.addReferenceFolder(project.id, 'd:/datasheets/');

    expect(places.referenceFolders).toEqual(['D:\\Datasheets']);
  });

  it('takes a reference folder away again, and does nothing for one it does not have', async () => {
    await backend.addReferenceFolder(project.id, 'D:\\Datasheets');

    const still = await backend.removeReferenceFolder(project.id, 'C:\\Work');
    expect(still.referenceFolders).toEqual(['D:\\Datasheets']);
    const gone = await backend.removeReferenceFolder(project.id, 'D:\\Datasheets\\');
    expect(gone.referenceFolders).toEqual([]);
  });

  it('refuses a reference folder that does not exist', async () => {
    const error = await refusal(backend.addReferenceFolder(project.id, 'D:\\Missing'));

    expect(error.code).toBe('folder_not_found');
  });

  it('refuses a reference folder that is the work folder or lies inside it', async () => {
    await backend.setWorkFolder(project.id, 'C:\\Work\\sensor-node');

    const inside = await refusal(
      backend.addReferenceFolder(project.id, 'C:\\Work\\sensor-node\\docs'),
    );
    expect(inside.code).toBe('rule_broken');
    expect(inside.message).toBe(
      'C:\\Work\\sensor-node\\docs lies inside the work folder C:\\Work\\sensor-node, where the specialists read everything already',
    );
    const same = await refusal(backend.addReferenceFolder(project.id, 'c:\\work\\SENSOR-NODE'));
    expect(same.code).toBe('rule_broken');
    expect(same.message).toBe(
      'the work folder and a reference folder cannot be the same folder: C:\\Work\\sensor-node',
    );
  });

  it('refuses a reference folder that contains the work folder', async () => {
    await backend.setWorkFolder(project.id, 'C:\\Work\\sensor-node');

    const error = await refusal(backend.addReferenceFolder(project.id, 'C:\\Work'));

    expect(error.code).toBe('rule_broken');
    expect(error.message).toBe(
      'the work folder C:\\Work\\sensor-node lies inside the reference folder C:\\Work, where the specialists may not write',
    );
  });

  it('refuses a work folder that gets in the way of a reference folder', async () => {
    await backend.addReferenceFolder(project.id, 'D:\\Datasheets');

    const inside = await refusal(backend.setWorkFolder(project.id, 'D:\\Datasheets\\Bosch'));
    expect(inside.code).toBe('rule_broken');
    const same = await refusal(backend.setWorkFolder(project.id, 'D:\\Datasheets'));
    expect(same.code).toBe('rule_broken');
    expect((await backend.projectPlaces(project.id)).isWorkFolderChosen).toBe(false);
  });

  it('copies reference files by their names, and gives a name that is taken a number', async () => {
    const names = await backend.addReferenceFiles(project.id, [
      'C:\\Users\\you\\Downloads\\BME680 datasheet.pdf',
      'D:\\Datasheets\\Bosch\\bme680 DATASHEET.pdf',
      'D:\\Datasheets\\notes',
    ]);
    const again = await backend.addReferenceFiles(project.id, ['D:\\Datasheets\\notes']);

    expect(names).toEqual(['BME680 datasheet.pdf', 'bme680 DATASHEET (2).pdf', 'notes']);
    expect(again).toEqual(['notes (2)']);
  });

  it('forgets the folders of a project that is deleted', async () => {
    await backend.setWorkFolder(project.id, 'C:\\Work\\sensor-node');

    await backend.deleteProject(project.id);

    expect((await refusal(backend.projectPlaces(project.id))).code).toBe('not_found');
  });

  it('asks the manager to pick a folder and to pick files', async () => {
    expect(await backend.chooseFolder()).toBe('D:\\Datasheets');
    expect(await backend.chooseFiles()).toEqual(['D:\\Datasheets\\Bosch\\BME680.pdf']);
  });

  it('has no file manager to show a folder in', async () => {
    expect((await refusal(backend.showFolder())).code).toBe('not_available');
  });
});

describe('folders as the demo compares them', () => {
  it('take two spellings of one folder for the same folder', () => {
    expect(isSameFolder('C:\\Work\\Node', 'c:/work/node/')).toBe(true);
    expect(isSameFolder('C:\\Work\\Node', 'C:\\Work\\Node2')).toBe(false);
  });

  it('know when a folder lies inside another, and that it is not inside itself', () => {
    expect(isInside('C:\\Work\\Node\\docs', 'C:\\Work')).toBe(true);
    expect(isInside('C:\\Work', 'C:\\Work')).toBe(false);
    expect(isInside('C:\\Workshop', 'C:\\Work')).toBe(false);
  });

  it('name a file by the last part of its path', () => {
    expect(nameOf('C:\\Users\\you\\Downloads\\sheet.pdf')).toBe('sheet.pdf');
    expect(nameOf('/home/you/notes/')).toBe('notes');
  });

  it('find a free name by counting up in front of the ending', () => {
    expect(freeName('sheet.pdf', [])).toBe('sheet.pdf');
    expect(freeName('sheet.pdf', ['Sheet.pdf'])).toBe('sheet (2).pdf');
    expect(freeName('sheet.pdf', ['sheet.pdf', 'sheet (2).pdf'])).toBe('sheet (3).pdf');
    expect(freeName('.gitignore', ['.gitignore'])).toBe('.gitignore (2)');
  });
});
