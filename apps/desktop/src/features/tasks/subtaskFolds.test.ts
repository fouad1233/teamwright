import { describe, expect, it, vi } from 'vitest';

import { SubtaskFoldStore, type FoldStorage } from './subtaskFolds';

/** Storage that lives in memory, the way the browser's does for a test. */
function memoryStorage(initial: Record<string, string> = {}): FoldStorage & {
  entries: Map<string, string>;
} {
  const entries = new Map(Object.entries(initial));
  return {
    entries,
    getItem: (key) => entries.get(key) ?? null,
    setItem: (key, value) => {
      entries.set(key, value);
    },
  };
}

describe('SubtaskFoldStore', () => {
  it('starts with every task closed', () => {
    const folds = new SubtaskFoldStore(memoryStorage());

    expect(folds.openTasks('project').size).toBe(0);
  });

  it('opens a closed task and closes an open one', () => {
    const folds = new SubtaskFoldStore(memoryStorage());

    folds.toggle('project', 'task');
    const afterOpening = folds.openTasks('project').has('task');
    folds.toggle('project', 'task');
    const afterClosing = folds.openTasks('project').has('task');

    expect([afterOpening, afterClosing]).toEqual([true, false]);
  });

  it('keeps the folds of different projects apart', () => {
    const folds = new SubtaskFoldStore(memoryStorage());

    folds.toggle('sensor', 'task');

    expect(folds.openTasks('sensor').has('task')).toBe(true);
    expect(folds.openTasks('display').has('task')).toBe(false);
  });

  it('remembers the open tasks for the next window', () => {
    const storage = memoryStorage();
    new SubtaskFoldStore(storage).toggle('project', 'task');

    const nextWindow = new SubtaskFoldStore(storage);

    expect([...nextWindow.openTasks('project')]).toEqual(['task']);
  });

  it('returns the same set until something changes', () => {
    const folds = new SubtaskFoldStore(memoryStorage());
    folds.toggle('project', 'task');

    const first = folds.openTasks('project');
    const second = folds.openTasks('project');
    folds.toggle('project', 'other');

    expect(second).toBe(first);
    expect(folds.openTasks('project')).not.toBe(first);
  });

  it('tells its listeners about a change until they stop listening', () => {
    const folds = new SubtaskFoldStore(memoryStorage());
    const listener = vi.fn();
    const stopListening = folds.subscribe(listener);

    folds.toggle('project', 'task');
    stopListening();
    folds.toggle('project', 'task');

    expect(listener).toHaveBeenCalledTimes(1);
  });

  it('starts closed when the stored value cannot be read', () => {
    const storage = memoryStorage({ 'teamwright.openSubtasks.project': 'not json' });

    const folds = new SubtaskFoldStore(storage);

    expect(folds.openTasks('project').size).toBe(0);
  });

  it('works without storage', () => {
    const folds = new SubtaskFoldStore();

    folds.toggle('project', 'task');

    expect(folds.openTasks('project').has('task')).toBe(true);
  });
});
