import { beforeEach, describe, expect, it } from 'vitest';

import type { Project, Task } from '../types';
import { DemoBackend } from './demoBackend';
import { sampleWorkspace } from './sampleWorkspace';
import { refusal, testEnvironment } from './testing';

describe('the tags and checklists of the sample workspace', () => {
  it('puts tags and a checklist on some of the sample tasks', async () => {
    const workspace = sampleWorkspace(new Date('2026-11-11T09:00:00.000Z'));
    const backend = new DemoBackend(workspace, testEnvironment());
    const [project] = workspace.projects;

    const tags = await backend.listTags(project?.id ?? '');
    const tasks = await backend.listTasks(project?.id ?? '');

    expect(tags.map((tag) => tag.name)).toEqual([
      'Cost',
      'EMC',
      'Needs a second look in the lab',
      'Power',
      'Waiting for parts',
    ]);
    const schematic = tasks.find((task) => task.title === 'Draw the schematic');
    expect(schematic?.tagIds).toHaveLength(4);
    expect(schematic?.checklist.filter((item) => item.done)).toHaveLength(3);
    expect(schematic?.checklist).toHaveLength(5);
    for (const task of tasks) {
      for (const tagId of task.tagIds) {
        expect(tags.map((tag) => tag.id)).toContain(tagId);
      }
    }
  });
});

describe('tags and checklists in the DemoBackend', () => {
  let backend: DemoBackend;
  let project: Project;
  let task: Task;

  beforeEach(async () => {
    backend = new DemoBackend(undefined, testEnvironment());
    project = await backend.createProject({ name: 'Air quality node', workflow: 'standard' });
    task = await backend.createTask({ projectId: project.id, title: 'Shield the radio' });
  });

  async function names(): Promise<string[]> {
    return (await backend.listTags(project.id)).map((tag) => tag.name);
  }

  async function stored(): Promise<Task | undefined> {
    return (await backend.listTasks(project.id)).find((candidate) => candidate.id === task.id);
  }

  async function otherProject(): Promise<Project> {
    return backend.createProject({ name: 'Weather station', workflow: 'standard' });
  }

  describe('the tags of a project', () => {
    it('lists the tags by name, whatever the capitals', async () => {
      for (const name of ['power', 'EMC', 'Enclosure']) {
        await backend.addTag(project.id, name);
      }

      expect(await names()).toEqual(['EMC', 'Enclosure', 'power']);
    });

    it('gives a new tag the next color in turn unless one is named', async () => {
      const first = await backend.addTag(project.id, 'EMC');
      const second = await backend.addTag(project.id, 'Power');
      const chosen = await backend.addTag(project.id, 'Urgent', 'red');

      expect([first.color, second.color, chosen.color]).toEqual(['blue', 'green', 'red']);
    });

    it('refuses a name another tag has, whatever the capitals', async () => {
      const emc = await backend.addTag(project.id, 'EMC');
      const power = await backend.addTag(project.id, 'Power');

      const taken = await refusal(backend.addTag(project.id, ' emc '));
      expect(taken.code).toBe('tag_name_taken');
      expect(taken.message).toBe('the project already has a tag called "emc"');
      expect((await refusal(backend.renameTag(power.id, 'Emc'))).code).toBe('tag_name_taken');
      expect((await backend.renameTag(emc.id, 'emc')).name).toBe('emc');
      expect((await refusal(backend.addTag(project.id, ' '))).code).toBe('rule_broken');
      expect((await refusal(backend.addTag(project.id, 'x'.repeat(31)))).code).toBe('rule_broken');
    });

    it('lets two projects each have a tag with one name', async () => {
      await backend.addTag(project.id, 'EMC');

      await expect(backend.addTag((await otherProject()).id, 'EMC')).resolves.toBeDefined();
    });

    it('gives a project at most a hundred tags', async () => {
      for (let number = 0; number < 100; number += 1) {
        await backend.addTag(project.id, `Tag ${number.toString()}`);
      }

      const error = await refusal(backend.addTag(project.id, 'One more'));

      expect(error.code).toBe('rule_broken');
      expect(error.message).toBe('a project has at most 100 tags');
    });

    it('recolors a tag', async () => {
      const tag = await backend.addTag(project.id, 'EMC');

      await backend.recolorTag(tag.id, 'purple');

      expect((await backend.listTags(project.id))[0]?.color).toBe('purple');
    });
  });

  describe('the tags of a task', () => {
    it('puts a tag on a task once, takes it off, and the history says so', async () => {
      const tag = await backend.addTag(project.id, 'EMC');

      const tagged = await backend.putTagOnTask(task.id, tag.id);
      expect(tagged.tagIds).toEqual([tag.id]);
      expect((await backend.putTagOnTask(task.id, tag.id)).tagIds).toEqual([tag.id]);
      expect((await backend.takeTagOffTask(task.id, tag.id)).tagIds).toEqual([]);

      const history = await backend.listTaskHistory(task.id);
      expect(history.map((change) => change.kind)).toEqual(['created', 'tagged', 'untagged']);
    });

    it('refuses a tag of another project', async () => {
      const foreign = await backend.addTag((await otherProject()).id, 'EMC');

      const error = await refusal(backend.putTagOnTask(task.id, foreign.id));

      expect(error.code).toBe('different_projects');
      expect((await stored())?.tagIds).toEqual([]);
    });

    it('puts a tag typed by name: the one the project has, or a new one', async () => {
      const emc = await backend.addTag(project.id, 'EMC');

      const found = await backend.putNamedTagOnTask(task.id, ' emc');
      const made = await backend.putNamedTagOnTask(task.id, 'Waiting for parts');

      expect(found.tag.id).toBe(emc.id);
      expect(made.tag.color).toBe('green');
      expect(made.task.tagIds).toHaveLength(2);
      expect(await names()).toEqual(['EMC', 'Waiting for parts']);
    });

    it('takes a removed tag off its tasks and out of the filters of views', async () => {
      const tag = await backend.addTag(project.id, 'EMC');
      const kept = await backend.addTag(project.id, 'Power');
      await backend.putTagOnTask(task.id, tag.id);
      await backend.putTagOnTask(task.id, kept.id);
      const [board] = await backend.listViews(project.id);
      const filter = { assignee: { kind: 'manager' }, hidesFinished: true } as const;
      await backend.setViewFilter(board?.id ?? '', { ...filter, tagId: tag.id });

      await backend.removeTag(tag.id);

      expect((await stored())?.tagIds).toEqual([kept.id]);
      expect((await backend.listTaskHistory(task.id)).at(-1)).toMatchObject({
        kind: 'untagged',
        tagId: tag.id,
      });
      expect((await backend.listViews(project.id))[0]?.filter).toEqual(filter);
      expect(await names()).toEqual(['Power']);
      expect((await refusal(backend.renameTag(tag.id, 'EMC'))).code).toBe('not_found');
    });

    it('filters a view only by a tag of its own project', async () => {
      const foreign = await backend.addTag((await otherProject()).id, 'EMC');
      const [board] = await backend.listViews(project.id);
      const everyTask = { assignee: { kind: 'anyone' }, hidesFinished: false } as const;

      const error = await refusal(
        backend.setViewFilter(board?.id ?? '', { ...everyTask, tagId: foreign.id }),
      );
      const missing = await refusal(
        backend.setViewFilter(board?.id ?? '', { ...everyTask, tagId: 'nothing' }),
      );

      expect(error.code).toBe('different_projects');
      expect(missing.code).toBe('not_found');
    });
  });

  describe('the checklist of a task', () => {
    it('adds, ticks, moves, renames and removes items, and keeps them', async () => {
      await backend.addChecklistItem(task.id, ' Order the  stencil ');
      const added = await backend.addChecklistItem(task.id, 'Bake the board');
      const [stencil, bake] = added.checklist.map((item) => item.id);
      await backend.tickChecklistItem(task.id, stencil ?? '', true);
      await backend.moveChecklistItem(task.id, bake ?? '', -3);
      await backend.renameChecklistItem(task.id, bake ?? '', 'Bake it at 240 degrees');

      expect((await stored())?.checklist).toEqual([
        { id: bake, text: 'Bake it at 240 degrees', done: false },
        { id: stencil, text: 'Order the stencil', done: true },
      ]);

      const removed = await backend.removeChecklistItem(task.id, stencil ?? '');
      expect(removed.checklist.map((item) => item.text)).toEqual(['Bake it at 240 degrees']);
    });

    it('tells a tick in the history, and not an item added or moved', async () => {
      const added = await backend.addChecklistItem(task.id, 'Order the stencil');
      await backend.addChecklistItem(task.id, 'Bake the board');
      const item = added.checklist[0]?.id ?? '';
      await backend.moveChecklistItem(task.id, item, 1);
      await backend.tickChecklistItem(task.id, item, true);
      await backend.tickChecklistItem(task.id, item, true);

      const history = await backend.listTaskHistory(task.id);

      expect(history.map((change) => change.kind)).toEqual(['created', 'ticked']);
      expect(history.at(-1)).toMatchObject({ item: 'Order the stencil' });
    });

    it('refuses an empty or long text, and an item the task does not have', async () => {
      expect((await refusal(backend.addChecklistItem(task.id, '  '))).code).toBe('rule_broken');
      expect((await refusal(backend.addChecklistItem(task.id, 'x'.repeat(201)))).code).toBe(
        'rule_broken',
      );
      expect((await refusal(backend.tickChecklistItem(task.id, 'nothing', true))).code).toBe(
        'not_found',
      );
      expect((await refusal(backend.addChecklistItem('nothing', 'Anything'))).code).toBe(
        'not_found',
      );
    });

    it('gives a checklist at most a hundred items', async () => {
      for (let number = 0; number < 100; number += 1) {
        await backend.addChecklistItem(task.id, `Item ${number.toString()}`);
      }

      const error = await refusal(backend.addChecklistItem(task.id, 'One more'));

      expect(error.message).toBe('a checklist has at most 100 items');
    });
  });
});
