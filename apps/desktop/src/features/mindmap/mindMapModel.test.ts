import { describe, expect, it } from 'vitest';

import type { Area, Task } from '@/shared/api/types';

import {
  COMPOSER_KEY,
  COMPOSER_WIDTH,
  GUTTER,
  PROJECT_KEY,
  STEM,
  addsUnder,
  areaKey,
  contentOf,
  fanPath,
  foldedIn,
  heightOf,
  isUnder,
  layoutTree,
  mapTree,
  neighbourOf,
  taskKey,
  withFoldToggled,
  withoutGoneFolds,
  type MapNode,
  type PlacedNode,
  type TreeLayout,
} from './mindMapModel';

/** A task with only the fields a test cares about filled in. */
function task(overrides: Partial<Task> & Pick<Task, 'id' | 'number'>): Task {
  return {
    projectId: 'project',
    areaId: 'board',
    title: overrides.id,
    description: '',
    statusId: 'todo',
    priority: 'normal',
    assignee: null,
    parentId: null,
    blockedBy: [],
    estimateMinutes: null,
    sprintId: null,
    points: null,
    startOn: null,
    dueOn: null,
    rank: 0,
    createdAt: '2026-11-11T09:00:00.000Z',
    updatedAt: '2026-11-11T09:00:00.000Z',
    completedAt: null,
    ...overrides,
  };
}

const board: Area = { id: 'board', projectId: 'project', name: 'Board' };
const code: Area = { id: 'code', projectId: 'project', name: 'Code' };
const box: Area = { id: 'box', projectId: 'project', name: 'Box' };

const NOTHING = new Set<string>();

/** The keys of the nodes under a node, in their order. */
function keysUnder(node: MapNode | undefined): string[] {
  return (node?.children ?? []).map((child) => child.key);
}

/** The node with a key, wherever it hangs in a tree. */
function find(node: MapNode, key: string): MapNode | undefined {
  if (node.key === key) {
    return node;
  }
  for (const child of node.children) {
    const found = find(child, key);
    if (found) {
      return found;
    }
  }
  return undefined;
}

describe('mapTree', () => {
  it('is the project alone when it has no tasks', () => {
    const tree = mapTree({ areas: [], tasks: [], folded: NOTHING });
    expect(tree).toMatchObject({ key: PROJECT_KEY, kind: 'project', children: [], taskCount: 0 });
  });

  it('hangs the tasks on the project by number when it shows no areas', () => {
    const tree = mapTree({
      areas: [],
      tasks: [task({ id: 'later', number: 7 }), task({ id: 'first', number: 2 })],
      folded: NOTHING,
    });
    expect(keysUnder(tree)).toEqual([taskKey('first'), taskKey('later')]);
    expect(tree.children.every((child) => child.kind === 'task')).toBe(true);
  });

  it('hangs the areas on the project in their order, and each task on its area', () => {
    const tree = mapTree({
      areas: [board, code, box],
      tasks: [
        task({ id: 'driver', number: 3, areaId: 'code' }),
        task({ id: 'schematic', number: 2, areaId: 'board' }),
        task({ id: 'parts', number: 1, areaId: 'board' }),
      ],
      folded: NOTHING,
    });
    expect(keysUnder(tree)).toEqual([areaKey('board'), areaKey('code'), areaKey('box')]);
    expect(keysUnder(find(tree, areaKey('board')))).toEqual([
      taskKey('parts'),
      taskKey('schematic'),
    ]);
    expect(keysUnder(find(tree, areaKey('code')))).toEqual([taskKey('driver')]);
  });

  it('shows an area without tasks as a leaf that cannot be folded', () => {
    const tree = mapTree({ areas: [board, box], tasks: [], folded: new Set([areaKey('box')]) });
    expect(find(tree, areaKey('box'))).toMatchObject({
      children: [],
      canFold: false,
      isFolded: false,
      taskCount: 0,
    });
  });

  it('hangs a task whose area is not shown on the project, after the areas', () => {
    const tree = mapTree({
      areas: [board],
      tasks: [task({ id: 'lost', number: 1, areaId: 'gone' }), task({ id: 'kept', number: 2 })],
      folded: NOTHING,
    });
    expect(keysUnder(tree)).toEqual([areaKey('board'), taskKey('lost')]);
  });

  it('hangs subtasks on their task by number, to any depth', () => {
    const tree = mapTree({
      areas: [],
      tasks: [
        task({ id: 'whole', number: 1 }),
        task({ id: 'second part', number: 5, parentId: 'whole' }),
        task({ id: 'first part', number: 3, parentId: 'whole' }),
        task({ id: 'part of a part', number: 9, parentId: 'first part' }),
      ],
      folded: NOTHING,
    });
    const whole = find(tree, taskKey('whole'));
    expect(whole?.kind).toBe('task');
    expect(keysUnder(whole)).toEqual([taskKey('first part'), taskKey('second part')]);
    const firstPart = find(tree, taskKey('first part'));
    expect(firstPart?.kind).toBe('subtask');
    expect(keysUnder(firstPart)).toEqual([taskKey('part of a part')]);
    expect(find(tree, taskKey('part of a part'))?.kind).toBe('subtask');
  });

  it('shows a subtask whose task is not among the tasks as a task of its own', () => {
    const tree = mapTree({
      areas: [],
      tasks: [task({ id: 'orphan', number: 4, parentId: 'not shown' })],
      folded: NOTHING,
    });
    expect(tree.children).toMatchObject([{ key: taskKey('orphan'), kind: 'task' }]);
  });

  it('counts the tasks under a node, however deep', () => {
    const tree = mapTree({
      areas: [board, code],
      tasks: [
        task({ id: 'whole', number: 1 }),
        task({ id: 'part', number: 2, parentId: 'whole' }),
        task({ id: 'part of a part', number: 3, parentId: 'part' }),
        task({ id: 'alone', number: 4, areaId: 'code' }),
      ],
      folded: NOTHING,
    });
    expect(tree.taskCount).toBe(4);
    expect(find(tree, areaKey('board'))?.taskCount).toBe(3);
    expect(find(tree, taskKey('whole'))?.taskCount).toBe(2);
    expect(find(tree, taskKey('alone'))?.taskCount).toBe(0);
  });

  it('shows nothing under a folded node, and still counts what it hides', () => {
    const tree = mapTree({
      areas: [board],
      tasks: [task({ id: 'whole', number: 1 }), task({ id: 'part', number: 2, parentId: 'whole' })],
      folded: new Set([areaKey('board')]),
    });
    expect(find(tree, areaKey('board'))).toMatchObject({
      children: [],
      canFold: true,
      isFolded: true,
      taskCount: 2,
    });
    expect(find(tree, taskKey('whole'))).toBeUndefined();
  });

  it('never folds the project', () => {
    const tree = mapTree({
      areas: [],
      tasks: [task({ id: 'only', number: 1 })],
      folded: new Set([PROJECT_KEY]),
    });
    expect(tree).toMatchObject({ canFold: false, isFolded: false });
    expect(keysUnder(tree)).toEqual([taskKey('only')]);
  });

  it('puts the composer last under the node a task is typed under', () => {
    const tasks = [
      task({ id: 'whole', number: 1 }),
      task({ id: 'part', number: 2, parentId: 'whole' }),
    ];
    const underTask = mapTree({
      areas: [board],
      tasks,
      folded: NOTHING,
      composingUnder: taskKey('whole'),
    });
    expect(keysUnder(find(underTask, taskKey('whole')))).toEqual([taskKey('part'), COMPOSER_KEY]);
    expect(find(underTask, COMPOSER_KEY)).toMatchObject({ kind: 'composer', makes: 'subtask' });

    const underArea = mapTree({
      areas: [board],
      tasks,
      folded: NOTHING,
      composingUnder: areaKey('board'),
    });
    expect(find(underArea, COMPOSER_KEY)).toMatchObject({ makes: 'task' });
    // The composer is no task.
    expect(underArea.taskCount).toBe(2);
  });

  it('unfolds the node a task is typed under', () => {
    const tree = mapTree({
      areas: [],
      tasks: [task({ id: 'whole', number: 1 }), task({ id: 'part', number: 2, parentId: 'whole' })],
      folded: new Set([taskKey('whole')]),
      composingUnder: taskKey('whole'),
    });
    expect(find(tree, taskKey('whole'))?.isFolded).toBe(false);
    expect(keysUnder(find(tree, taskKey('whole')))).toEqual([taskKey('part'), COMPOSER_KEY]);
  });

  it('ends even if two tasks are parts of each other', () => {
    const tree = mapTree({
      areas: [],
      tasks: [
        task({ id: 'plain', number: 1 }),
        // Not possible in the program; the tree must still end.
        task({ id: 'snake', number: 2, parentId: 'tail' }),
        task({ id: 'tail', number: 3, parentId: 'snake' }),
      ],
      folded: NOTHING,
    });
    expect(keysUnder(tree)).toEqual([taskKey('plain')]);
  });
});

describe('addsUnder', () => {
  const tree = mapTree({
    areas: [board],
    tasks: [task({ id: 'whole', number: 1 }), task({ id: 'part', number: 2, parentId: 'whole' })],
    folded: NOTHING,
    composingUnder: taskKey('part'),
  });

  it('adds a subtask under a task and under a subtask', () => {
    expect(addsUnder(find(tree, taskKey('whole')) as MapNode)).toBe('subtask');
    expect(addsUnder(find(tree, taskKey('part')) as MapNode)).toBe('subtask');
  });

  it('adds a task under an area', () => {
    expect(addsUnder(find(tree, areaKey('board')) as MapNode)).toBe('task');
  });

  it('adds a task under a project only when it shows no areas', () => {
    expect(addsUnder(tree)).toBeNull();
    expect(addsUnder(mapTree({ areas: [], tasks: [], folded: NOTHING }))).toBe('task');
  });

  it('adds nothing under the composer', () => {
    expect(addsUnder(find(tree, COMPOSER_KEY) as MapNode)).toBeNull();
  });
});

/** A width for every node of a test: what it names, or a hundred pixels. */
function widthsOf(named: Record<string, number> = {}): (node: MapNode) => number {
  return (node) => named[node.key] ?? 100;
}

function placedIn(layout: TreeLayout, key: string): PlacedNode {
  const placed = layout.byKey.get(key);
  if (!placed) {
    throw new Error(`no node ${key}`);
  }
  return placed;
}

function middleOf(placed: PlacedNode): number {
  return placed.y + placed.height / 2;
}

/** A project of two areas: one task with three parts between two plain tasks, and one more task. */
function smallTree(folded: ReadonlySet<string> = NOTHING): MapNode {
  return mapTree({
    areas: [board, code],
    tasks: [
      task({ id: 'before', number: 1 }),
      task({ id: 'whole', number: 2 }),
      task({ id: 'part a', number: 3, parentId: 'whole' }),
      task({ id: 'part b', number: 4, parentId: 'whole' }),
      task({ id: 'part c', number: 5, parentId: 'whole' }),
      task({ id: 'after', number: 6 }),
      task({ id: 'elsewhere', number: 7, areaId: 'code' }),
    ],
    folded,
  });
}

describe('layoutTree', () => {
  it('begins every node of a column at the same place', () => {
    const layout = layoutTree(smallTree(), widthsOf({ [taskKey('before')]: 60 }));
    const tasks = ['before', 'whole', 'after', 'elsewhere'].map((id) =>
      placedIn(layout, taskKey(id)),
    );
    expect(new Set(tasks.map((placed) => placed.x)).size).toBe(1);
    expect(placedIn(layout, areaKey('board')).x).toBe(placedIn(layout, areaKey('code')).x);
    expect(placedIn(layout, PROJECT_KEY).x).toBe(0);
  });

  it('makes a column as wide as its widest node, and a gutter lies before the next', () => {
    const layout = layoutTree(
      smallTree(),
      widthsOf({ [taskKey('after')]: 300, [taskKey('whole')]: 120 }),
    );
    const tasksLeft = placedIn(layout, taskKey('whole')).x;
    // The widest task has nothing under it, so it needs no stem.
    expect(placedIn(layout, taskKey('part a')).x).toBe(tasksLeft + 300 + GUTTER);
  });

  it('leaves room for the stem of a node that has others under it', () => {
    const layout = layoutTree(smallTree(), widthsOf({ [taskKey('whole')]: 300 }));
    const tasksLeft = placedIn(layout, taskKey('whole')).x;
    expect(placedIn(layout, taskKey('part a')).x).toBe(tasksLeft + 300 + STEM + GUTTER);
    // The project and the areas all have nodes under them.
    expect(placedIn(layout, areaKey('board')).x).toBe(100 + STEM + GUTTER);
  });

  it('stacks the nodes under a node with an even space between them', () => {
    const layout = layoutTree(smallTree(), widthsOf());
    const a = placedIn(layout, taskKey('part a'));
    const b = placedIn(layout, taskKey('part b'));
    const c = placedIn(layout, taskKey('part c'));
    expect(b.y - (a.y + a.height)).toBe(4);
    expect(c.y - (b.y + b.height)).toBe(4);
    const elsewhere = placedIn(layout, taskKey('elsewhere'));
    const after = placedIn(layout, taskKey('after'));
    // Between the last task of one area and the first of the next lies the space of two areas.
    expect(elsewhere.y - (after.y + after.height)).toBe(20);
  });

  it('stands a node in the middle between the first and the last node under it', () => {
    const layout = layoutTree(smallTree(), widthsOf());
    const whole = placedIn(layout, taskKey('whole'));
    const first = placedIn(layout, taskKey('part a'));
    const last = placedIn(layout, taskKey('part c'));
    expect(middleOf(whole)).toBe((middleOf(first) + middleOf(last)) / 2);
    const project = placedIn(layout, PROJECT_KEY);
    const firstArea = placedIn(layout, areaKey('board'));
    const lastArea = placedIn(layout, areaKey('code'));
    // To the whole pixel: a middle between two pixels is rounded.
    const between = (middleOf(firstArea) + middleOf(lastArea)) / 2;
    expect(Math.abs(middleOf(project) - between)).toBeLessThan(1);
  });

  it('gives a node and everything under it a room of its own', () => {
    const layout = layoutTree(smallTree(), widthsOf());
    const before = placedIn(layout, taskKey('before'));
    const after = placedIn(layout, taskKey('after'));
    const firstPart = placedIn(layout, taskKey('part a'));
    const lastPart = placedIn(layout, taskKey('part c'));
    // The parts of the task in the middle lie wholly between its neighbours.
    expect(firstPart.y).toBe(before.y + before.height + 6);
    expect(after.y).toBe(lastPart.y + lastPart.height + 6);
  });

  it('overlaps nothing, whatever is folded', () => {
    const everyFold = [NOTHING, new Set([taskKey('whole')]), new Set([areaKey('board')])];
    for (const folded of everyFold) {
      const { nodes } = layoutTree(smallTree(folded), widthsOf({ [taskKey('whole')]: 240 }));
      for (const one of nodes) {
        for (const other of nodes) {
          if (one === other) {
            continue;
          }
          const apartAcross = one.x + one.width <= other.x || other.x + other.width <= one.x;
          const apartDown = one.y + one.height <= other.y || other.y + other.height <= one.y;
          expect(apartAcross || apartDown).toBe(true);
        }
      }
    }
  });

  it('keeps a node that is higher than what hangs under it inside the tree', () => {
    const tree = mapTree({ areas: [], tasks: [task({ id: 'only', number: 1 })], folded: NOTHING });
    const layout = layoutTree(tree, widthsOf());
    const project = placedIn(layout, PROJECT_KEY);
    const only = placedIn(layout, taskKey('only'));
    expect(project.y).toBe(0);
    expect(middleOf(only)).toBe(middleOf(project));
    expect(layout.height).toBe(project.height);
  });

  it('lies on whole pixels', () => {
    const layout = layoutTree(smallTree(), widthsOf({ [taskKey('whole')]: 187.4 }));
    for (const placed of layout.nodes) {
      expect(Number.isInteger(placed.x)).toBe(true);
      expect(Number.isInteger(placed.y)).toBe(true);
      expect(Number.isInteger(placed.width)).toBe(true);
    }
    expect(placedIn(layout, taskKey('whole')).width).toBe(188);
  });

  it('is as large as all its nodes together', () => {
    const layout = layoutTree(smallTree(), widthsOf());
    expect(layout.width).toBe(Math.max(...layout.nodes.map((placed) => placed.x + placed.width)));
    expect(layout.height).toBe(Math.max(...layout.nodes.map((placed) => placed.y + placed.height)));
    expect(Math.min(...layout.nodes.map((placed) => placed.y))).toBe(0);
  });

  it('says for every node where it hangs and which place it has there', () => {
    const layout = layoutTree(smallTree(), widthsOf());
    expect(placedIn(layout, PROJECT_KEY)).toMatchObject({ depth: 0, parentKey: null });
    expect(placedIn(layout, taskKey('part b'))).toMatchObject({
      depth: 3,
      parentKey: taskKey('whole'),
      position: 2,
      siblingCount: 3,
    });
    // A node comes before the nodes under it, and those from the top down.
    expect(layout.nodes.map((placed) => placed.node.key).slice(0, 4)).toEqual([
      PROJECT_KEY,
      areaKey('board'),
      taskKey('before'),
      taskKey('whole'),
    ]);
  });

  it('gives the composer the width it is typed in and the height of what it makes', () => {
    const tree = mapTree({
      areas: [],
      tasks: [task({ id: 'whole', number: 1 }), task({ id: 'part', number: 2, parentId: 'whole' })],
      folded: NOTHING,
      composingUnder: taskKey('whole'),
    });
    const layout = layoutTree(tree, (node) => (node.kind === 'composer' ? COMPOSER_WIDTH : 100));
    const composer = placedIn(layout, COMPOSER_KEY);
    expect(composer.width).toBe(COMPOSER_WIDTH);
    expect(composer.height).toBe(placedIn(layout, taskKey('part')).height);
    expect(heightOf(composer.node)).toBe(composer.height);
  });

  it('joins a node to the nodes under it: from its edge, by the edge of its column, to theirs', () => {
    const layout = layoutTree(smallTree(), widthsOf({ [taskKey('after')]: 300 }));
    const fan = layout.fans.find((each) => each.parentKey === taskKey('whole'));
    const whole = placedIn(layout, taskKey('whole'));
    const parts = ['part a', 'part b', 'part c'].map((id) => placedIn(layout, taskKey(id)));

    expect(fan?.from).toEqual({ x: whole.x + whole.width, y: middleOf(whole) });
    expect(fan?.joint).toEqual({ x: whole.x + 300, y: middleOf(whole) });
    expect(fan?.to).toEqual(parts.map((part) => ({ x: part.x, y: middleOf(part) })));
    // Every node with others under it has a join, and no other.
    expect(layout.fans.map((each) => each.parentKey).sort()).toEqual(
      [PROJECT_KEY, areaKey('board'), areaKey('code'), taskKey('whole')].sort(),
    );
  });
});

describe('fanPath', () => {
  it('runs straight to the joint, and from there in one curve to each node', () => {
    const path = fanPath({
      parentKey: 'parent',
      from: { x: 100, y: 50 },
      joint: { x: 140, y: 50 },
      to: [
        { x: 200, y: 20 },
        { x: 200, y: 80 },
      ],
    });
    expect(path).toBe('M100 50H140M140 50C170 50 170 20 200 20M140 50C170 50 170 80 200 80');
  });
});

describe('neighbourOf', () => {
  const layout = layoutTree(smallTree(), widthsOf());

  it('goes left to the node a node hangs on', () => {
    expect(neighbourOf(layout, taskKey('part b'), 'left')).toBe(taskKey('whole'));
    expect(neighbourOf(layout, PROJECT_KEY, 'left')).toBeNull();
  });

  it('goes right to the node under it that is nearest to its own height', () => {
    expect(neighbourOf(layout, taskKey('whole'), 'right')).toBe(taskKey('part b'));
    expect(neighbourOf(layout, areaKey('code'), 'right')).toBe(taskKey('elsewhere'));
    expect(neighbourOf(layout, taskKey('after'), 'right')).toBeNull();
  });

  it('goes up and down the column, from one branch into the next', () => {
    expect(neighbourOf(layout, taskKey('whole'), 'up')).toBe(taskKey('before'));
    expect(neighbourOf(layout, taskKey('whole'), 'down')).toBe(taskKey('after'));
    expect(neighbourOf(layout, taskKey('after'), 'down')).toBe(taskKey('elsewhere'));
    expect(neighbourOf(layout, taskKey('before'), 'up')).toBeNull();
    expect(neighbourOf(layout, taskKey('elsewhere'), 'down')).toBeNull();
  });

  it('passes over the composer', () => {
    const composing = layoutTree(
      mapTree({
        areas: [],
        tasks: [task({ id: 'whole', number: 1 })],
        folded: NOTHING,
        composingUnder: taskKey('whole'),
      }),
      widthsOf(),
    );
    expect(neighbourOf(composing, taskKey('whole'), 'right')).toBeNull();
  });

  it('knows no way from a node that is not in the tree', () => {
    expect(neighbourOf(layout, 'nowhere', 'down')).toBeNull();
  });
});

describe('isUnder', () => {
  const layout = layoutTree(smallTree(), widthsOf());

  it('says whether a node hangs under another, however deep', () => {
    expect(isUnder(layout, taskKey('part a'), taskKey('whole'))).toBe(true);
    expect(isUnder(layout, taskKey('part a'), areaKey('board'))).toBe(true);
    expect(isUnder(layout, taskKey('part a'), areaKey('code'))).toBe(false);
    expect(isUnder(layout, taskKey('whole'), taskKey('whole'))).toBe(false);
  });
});

describe('what the view keeps', () => {
  it('reads no content as nothing folded', () => {
    expect(foldedIn(null).size).toBe(0);
    expect(foldedIn('').size).toBe(0);
    expect(foldedIn('  ').size).toBe(0);
  });

  it('reads damaged content, or content of another shape, as nothing folded', () => {
    expect(foldedIn('{"folded": ["task:a"').size).toBe(0);
    expect(foldedIn('"folded"').size).toBe(0);
    expect(foldedIn('null').size).toBe(0);
    expect(foldedIn('[1, 2]').size).toBe(0);
    expect(foldedIn('{"text": "a page of another kind"}').size).toBe(0);
    expect(foldedIn('{"folded": "task:a"}').size).toBe(0);
  });

  it('keeps the keys it can read and leaves out what is no key', () => {
    expect([...foldedIn('{"folded": ["task:a", 7, null, "area:b", {}]}')]).toEqual([
      'task:a',
      'area:b',
    ]);
  });

  it('keeps nothing when nothing is folded', () => {
    expect(contentOf(NOTHING)).toBeNull();
  });

  it('writes the same folds as the same text, in whatever order they were made', () => {
    const one = contentOf(new Set(['task:b', 'area:a']));
    const other = contentOf(new Set(['area:a', 'task:b']));
    expect(one).toBe(other);
    expect(one).toBe('{"version":1,"folded":["area:a","task:b"]}');
  });

  it('reads back what it wrote', () => {
    const folded = new Set([taskKey('whole'), areaKey('board')]);
    expect(foldedIn(contentOf(folded))).toEqual(folded);
  });

  it('folds a node that is open and unfolds one that is folded', () => {
    const folded = withFoldToggled(NOTHING, 'task:a');
    expect([...folded]).toEqual(['task:a']);
    expect(withFoldToggled(folded, 'task:a').size).toBe(0);
    // What it was given stays as it was.
    expect(NOTHING.size).toBe(0);
  });

  it('forgets the folds of nodes that are gone', () => {
    const folded = new Set(['task:kept', 'task:gone']);
    expect([...withoutGoneFolds(folded, (key) => key !== 'task:gone')]).toEqual(['task:kept']);
    expect(withoutGoneFolds(folded, () => true)).toBe(folded);
  });
});
