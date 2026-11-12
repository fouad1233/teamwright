import { describe, expect, it } from 'vitest';

import type { Area, Task } from '@/shared/api/types';

import {
  areaToShow,
  likelyAreaForTasksOf,
  nameForNewArea,
  showsAreas,
  taskCountsByArea,
  tasksOfArea,
  withMovedArea,
} from './areaModel';

function area(id: string, name = id): Area {
  return { id, projectId: 'project', name };
}

function task(id: string, areaId: string): Pick<Task, 'id' | 'areaId'> {
  return { id, areaId };
}

const GENERAL = area('general', 'General');
const FIRMWARE = area('firmware', 'Firmware');
const ENCLOSURE = area('enclosure', 'Enclosure');

describe('whether areas are shown', () => {
  it('shows nothing of areas while a project has one', () => {
    expect(showsAreas([GENERAL])).toBe(false);
    expect(showsAreas([GENERAL, FIRMWARE])).toBe(true);
  });
});

describe('the area to show', () => {
  it('shows the area asked for', () => {
    expect(areaToShow([GENERAL, FIRMWARE], 'firmware')).toBe(FIRMWARE);
  });

  it('shows the whole project when no area is asked for', () => {
    expect(areaToShow([GENERAL, FIRMWARE], null)).toBeUndefined();
  });

  it('shows the whole project when the area asked for is gone', () => {
    expect(areaToShow([GENERAL, FIRMWARE], 'enclosure')).toBeUndefined();
  });

  it('shows the whole project when it has one area left', () => {
    expect(areaToShow([GENERAL], 'general')).toBeUndefined();
  });
});

describe('the tasks of an area', () => {
  const tasks = [task('1', 'general'), task('2', 'firmware'), task('3', 'firmware')] as Task[];

  it('keeps the tasks of the open area, in their order', () => {
    expect(tasksOfArea(tasks, FIRMWARE).map((kept) => kept.id)).toEqual(['2', '3']);
  });

  it('keeps every task when no area is open', () => {
    expect(tasksOfArea(tasks, undefined)).toEqual(tasks);
  });

  it('counts the tasks of each area', () => {
    const counts = taskCountsByArea(tasks);

    expect(counts.get('general')).toBe(1);
    expect(counts.get('firmware')).toBe(2);
    expect(counts.get('enclosure')).toBeUndefined();
  });
});

describe('the name of a new area', () => {
  it('is "New area" while no area has that name', () => {
    expect(nameForNewArea([GENERAL])).toBe('New area');
  });

  it('takes the first free number, whatever the capitals', () => {
    expect(nameForNewArea([GENERAL, area('a', 'new area'), area('b', 'New area 3')])).toBe(
      'New area 2',
    );
  });
});

describe('moving an area', () => {
  it('puts the area at its new place and keeps the others in their order', () => {
    const moved = withMovedArea([GENERAL, FIRMWARE, ENCLOSURE], 'enclosure', 0);

    expect(moved.map((kept) => kept.id)).toEqual(['enclosure', 'general', 'firmware']);
  });

  it('keeps the list as it is for an area it does not have', () => {
    expect(withMovedArea([GENERAL, FIRMWARE], 'gone', 0)).toEqual([GENERAL, FIRMWARE]);
  });
});

describe('where the tasks of a removed area go first', () => {
  it('offers the area above', () => {
    expect(likelyAreaForTasksOf([GENERAL, FIRMWARE, ENCLOSURE], ENCLOSURE)).toBe(FIRMWARE);
  });

  it('offers the area below for the first area', () => {
    expect(likelyAreaForTasksOf([GENERAL, FIRMWARE], GENERAL)).toBe(FIRMWARE);
  });
});
