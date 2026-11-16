import type { Area, Id, Task } from '@/shared/api/types';
import type { Point } from '@/shared/ui/groundModel';

import { indexSubtasks } from '../tasks/taskModel';

/**
 * The arithmetic of the mind map: which nodes the tree has, where each one
 * lies, how the joins run, and what the view keeps of its own.
 *
 * The tree grows to the right. Nodes that are equally deep stand in one
 * column, so that their left edges meet along a line. Under a node, the
 * nodes below it are stacked, and the node stands in the middle of them.
 */

/**
 * What a node of the map is. A task that is a part of another task is a
 * subtask, however deep it lies. The composer is the place where the title
 * of a new task is typed.
 */
export type NodeKind = 'project' | 'area' | 'task' | 'subtask' | 'composer';

/** The key of the node of the project. There is one in every map. */
export const PROJECT_KEY = 'project';

/** The key of the composer. There is at most one in a map. */
export const COMPOSER_KEY = 'composer';

/** The key of the node of an area. */
export function areaKey(areaId: Id): string {
  return `area:${areaId}`;
}

/** The key of the node of a task or a subtask. */
export function taskKey(taskId: Id): string {
  return `task:${taskId}`;
}

interface NodeBase {
  /** Stable among the nodes: for the selection and for what is folded. */
  key: string;
  /** The nodes under it that are shown: none while it is folded. */
  children: MapNode[];
  /** Whether it has anything under it that can be folded away. */
  canFold: boolean;
  isFolded: boolean;
  /** How many tasks lie under it, however deep, shown or folded away. */
  taskCount: number;
}

/** One node of the tree. */
export type MapNode = NodeBase &
  (
    | { kind: 'project' }
    | { kind: 'area'; area: Area }
    | { kind: 'task' | 'subtask'; task: Task }
    | { kind: 'composer'; makes: 'task' | 'subtask' }
  );

/** What the tree is made from. */
export interface MapSources {
  /**
   * The areas the map shows as branches, in their order. None for a
   * project that is not cut into areas: its tasks then hang on the project.
   */
  areas: readonly Area[];
  /** The tasks to show, subtasks with their tasks. */
  tasks: readonly Task[];
  /** The keys of the nodes that are folded. */
  folded: ReadonlySet<string>;
  /** The key of the node under which a new task is being typed, if any. */
  composingUnder?: string | null;
}

/** What a node says about itself, before what hangs under it is known. */
type NodeSubject = { key: string } & (
  { kind: 'project' } | { kind: 'area'; area: Area } | { kind: 'task' | 'subtask'; task: Task }
);

/**
 * The tree of a project: the project, its areas if it shows any, their
 * tasks by number, and under each task its subtasks by number, to any
 * depth. An area without tasks is a leaf. A task whose area is not among
 * the areas hangs on the project itself, after them, so that no task is
 * lost; so does a subtask whose task is not among the tasks.
 *
 * A folded node keeps the count of the tasks under it and shows none of
 * them. The project cannot be folded. The node a task is being typed
 * under is never folded, and has the composer as its last child.
 */
export function mapTree({ areas, tasks, folded, composingUnder = null }: MapSources): MapNode {
  const shown = new Set(tasks.map((task) => task.id));
  const subtasksByParent = indexSubtasks(tasks);

  const withChildren = (
    subject: NodeSubject,
    all: MapNode[],
    makes: 'task' | 'subtask',
  ): MapNode => {
    const taskCount = all.reduce(
      (count, child) =>
        count + child.taskCount + (child.kind === 'task' || child.kind === 'subtask' ? 1 : 0),
      0,
    );
    const isComposing = composingUnder === subject.key;
    const canFold = subject.kind !== 'project' && all.length > 0;
    const isFolded = canFold && !isComposing && folded.has(subject.key);
    const composer: MapNode[] = isComposing
      ? [
          {
            key: COMPOSER_KEY,
            kind: 'composer',
            makes,
            children: [],
            canFold: false,
            isFolded: false,
            taskCount: 0,
          },
        ]
      : [];
    return {
      ...subject,
      children: isFolded ? [] : [...all, ...composer],
      canFold,
      isFolded,
      taskCount,
    };
  };

  const nodeOfTask = (task: Task, kind: 'task' | 'subtask'): MapNode =>
    withChildren(
      { key: taskKey(task.id), kind, task },
      (subtasksByParent.get(task.id) ?? []).map((part) => nodeOfTask(part, 'subtask')),
      'subtask',
    );

  // The tree is walked down from the tasks that are part of no other, and a
  // task is part of one task at most, so no task is met twice and the walk
  // ends. Tasks that were parts of each other would never be reached.
  const mainTasks = tasks
    .filter((task) => task.parentId === null || !shown.has(task.parentId))
    .sort((first, second) => first.number - second.number);
  const areaIds = new Set(areas.map((area) => area.id));

  const areaNodes = areas.map((area) =>
    withChildren(
      { key: areaKey(area.id), kind: 'area', area },
      mainTasks.filter((task) => task.areaId === area.id).map((task) => nodeOfTask(task, 'task')),
      'task',
    ),
  );
  const looseNodes = mainTasks
    .filter((task) => !areaIds.has(task.areaId))
    .map((task) => nodeOfTask(task, 'task'));

  return withChildren({ key: PROJECT_KEY, kind: 'project' }, [...areaNodes, ...looseNodes], 'task');
}

/**
 * What a "+" on a node would add under it: a subtask under a task, a task
 * under an area, and a task under the project when the project shows no
 * areas. Nothing under a project that does, because a task belongs in one
 * of them, and nothing under the composer.
 */
export function addsUnder(node: MapNode): 'task' | 'subtask' | null {
  switch (node.kind) {
    case 'task':
    case 'subtask':
      return 'subtask';
    case 'area':
      return 'task';
    case 'project':
      return node.children.some((child) => child.kind === 'area') ? null : 'task';
    case 'composer':
      return null;
  }
}

/** The heights of the nodes, in pixels. They are even, so that the middle of a node is a whole pixel. */
const HEIGHTS = { project: 44, area: 28, task: 28, subtask: 24 } as const;

/**
 * The space a node keeps to the nodes that stand under the same node as it
 * does. Between two kinds, the larger space counts.
 */
const GAPS = { project: 20, area: 20, task: 6, subtask: 4 } as const;

/** The space between two columns, in which the joins fan out. */
export const GUTTER = 56;

/**
 * The shortest way a join runs straight from its node before it fans out.
 * The "+" of a selected node stands on it.
 */
export const STEM = 32;

/** The width of the composer. Its text is typed, so it cannot be measured before. */
export const COMPOSER_WIDTH = 248;

/** The kind of node a node is laid out as: a composer as what it makes. */
function looksLike(node: MapNode): Exclude<NodeKind, 'composer'> {
  return node.kind === 'composer' ? node.makes : node.kind;
}

/** The height of a node, in pixels. */
export function heightOf(node: MapNode): number {
  return HEIGHTS[looksLike(node)];
}

/** A node with its place on the ground. */
export interface PlacedNode {
  node: MapNode;
  /** How deep it lies: 0 for the project. */
  depth: number;
  /** The key of the node it hangs on. Nothing for the project. */
  parentKey: string | null;
  /** Its place among the nodes that hang on the same node, from 1. */
  position: number;
  /** How many nodes hang on the same node. */
  siblingCount: number;
  x: number;
  y: number;
  width: number;
  height: number;
}

/** The join between a node and the nodes under it. */
export interface Fan {
  parentKey: string;
  /** Where the join leaves its node: the middle of the node's right edge. */
  from: Point;
  /** Where the join fans out: on the same height, at the edge of the node's column. */
  joint: Point;
  /** Where it arrives: the middle of the left edge of each node under it, from the top. */
  to: Point[];
}

/** Where every node of a tree lies. */
export interface TreeLayout {
  /** The nodes, a node before the nodes under it, and those from the top down. */
  nodes: PlacedNode[];
  byKey: ReadonlyMap<string, PlacedNode>;
  fans: Fan[];
  /** The size of the whole tree. Its top left corner is the origin. */
  width: number;
  height: number;
}

/** The room a node and everything under it takes up, from the top of that room. */
interface Band {
  /** Where the node itself begins. */
  top: number;
  /** How high the room is. */
  height: number;
  /** Where the room of each of its children begins. */
  childTops: number[];
}

/**
 * Lays out a tree that grows to the right.
 *
 * Across: every node begins where its column begins, and a column is as
 * wide as its widest node, with room for the stem of a node that has
 * others under it. Columns are a gutter apart.
 *
 * Down: the nodes under a node are stacked with an even space between
 * them, each with everything under it in a room of its own, so nothing
 * overlaps. A node stands in the middle between the first and the last
 * node under it. All places are whole pixels.
 *
 * `widthOf` says how wide a node is: the view measures that on the page.
 */
export function layoutTree(root: MapNode, widthOf: (node: MapNode) => number): TreeLayout {
  const widths = new Map<MapNode, number>();
  const columnWidths: number[] = [];
  const measure = (node: MapNode, depth: number) => {
    const width = Math.ceil(widthOf(node));
    widths.set(node, width);
    const needed = width + (node.children.length > 0 ? STEM : 0);
    columnWidths[depth] = Math.max(columnWidths[depth] ?? 0, needed);
    for (const child of node.children) {
      measure(child, depth + 1);
    }
  };
  measure(root, 0);

  const columnLefts: number[] = [];
  let left = 0;
  for (const width of columnWidths) {
    columnLefts.push(left);
    left += width + GUTTER;
  }

  const bands = new Map<MapNode, Band>();
  const bandOf = (node: MapNode): Band => {
    const height = heightOf(node);
    let cursor = 0;
    const childTops: number[] = [];
    const middles: number[] = [];
    node.children.forEach((child, index) => {
      const above = node.children[index - 1];
      if (above) {
        cursor += Math.max(GAPS[looksLike(above)], GAPS[looksLike(child)]);
      }
      const childBand = bandOf(child);
      childTops.push(cursor);
      middles.push(cursor + childBand.top + heightOf(child) / 2);
      cursor += childBand.height;
    });
    const firstMiddle = middles[0] ?? height / 2;
    const lastMiddle = middles[middles.length - 1] ?? firstMiddle;
    const wantedTop = Math.round((firstMiddle + lastMiddle) / 2 - height / 2);
    // A node that is higher than all that hangs under it: what hangs under
    // it moves down, so that the node still begins inside its own room.
    const shift = Math.max(-wantedTop, 0);
    const top = wantedTop + shift;
    const band = {
      top,
      height: Math.max(cursor + shift, top + height),
      childTops: childTops.map((childTop) => childTop + shift),
    };
    bands.set(node, band);
    return band;
  };
  const whole = bandOf(root);

  const nodes: PlacedNode[] = [];
  const byKey = new Map<string, PlacedNode>();
  const fans: Fan[] = [];
  const place = (
    node: MapNode,
    depth: number,
    bandTop: number,
    parentKey: string | null,
    position: number,
    siblingCount: number,
  ): PlacedNode => {
    const band = bands.get(node);
    const placed: PlacedNode = {
      node,
      depth,
      parentKey,
      position,
      siblingCount,
      x: columnLefts[depth] ?? 0,
      y: bandTop + (band?.top ?? 0),
      width: widths.get(node) ?? 0,
      height: heightOf(node),
    };
    nodes.push(placed);
    byKey.set(node.key, placed);
    const children = node.children.map((child, index) =>
      place(
        child,
        depth + 1,
        bandTop + (band?.childTops[index] ?? 0),
        node.key,
        index + 1,
        node.children.length,
      ),
    );
    if (children.length > 0) {
      const middle = placed.y + placed.height / 2;
      fans.push({
        parentKey: node.key,
        from: { x: placed.x + placed.width, y: middle },
        joint: { x: placed.x + (columnWidths[depth] ?? placed.width), y: middle },
        to: children.map((child) => ({ x: child.x, y: child.y + child.height / 2 })),
      });
    }
    return placed;
  };
  place(root, 0, 0, null, 1, 1);

  const lastColumn = columnWidths.length - 1;
  return {
    nodes,
    byKey,
    fans,
    width: (columnLefts[lastColumn] ?? 0) + (columnWidths[lastColumn] ?? 0),
    height: whole.height,
  };
}

/**
 * The line of a join, as the path of an SVG: straight from the node to the
 * joint, and from there one curve to each node under it. A curve leaves the
 * joint level and arrives level, so it meets both nodes squarely.
 */
export function fanPath({ from, joint, to }: Fan): string {
  const stem = `M${from.x.toString()} ${from.y.toString()}H${joint.x.toString()}`;
  const curves = to.map((end) => {
    const bend = ((joint.x + end.x) / 2).toString();
    const start = `M${joint.x.toString()} ${joint.y.toString()}`;
    return `${start}C${bend} ${joint.y.toString()} ${bend} ${end.y.toString()} ${end.x.toString()} ${end.y.toString()}`;
  });
  return [stem, ...curves].join('');
}

/** A way to step from one node to the next with the arrow keys. */
export type Step = 'left' | 'right' | 'up' | 'down';

/**
 * The node an arrow key leads to, by its key: to the left the node this
 * one hangs on; to the right, of the nodes under it, the one nearest to
 * its own height; up and down the next node of the same column, whatever
 * it hangs on. Nothing where the tree ends. The composer is passed over:
 * it is typed in, not selected.
 */
export function neighbourOf(layout: TreeLayout, key: string, step: Step): string | null {
  const from = layout.byKey.get(key);
  if (!from) {
    return null;
  }
  if (step === 'left') {
    return from.parentKey;
  }
  const middleOf = (placed: PlacedNode) => placed.y + placed.height / 2;
  if (step === 'right') {
    let nearest: PlacedNode | null = null;
    for (const child of from.node.children) {
      const placed = layout.byKey.get(child.key);
      if (!placed || child.kind === 'composer') {
        continue;
      }
      const distance = Math.abs(middleOf(placed) - middleOf(from));
      if (nearest === null || distance < Math.abs(middleOf(nearest) - middleOf(from))) {
        nearest = placed;
      }
    }
    return nearest?.node.key ?? null;
  }
  const column = layout.nodes.filter(
    (placed) => placed.depth === from.depth && placed.node.kind !== 'composer',
  );
  const index = column.indexOf(from);
  return column[index + (step === 'up' ? -1 : 1)]?.node.key ?? null;
}

/** Whether a node hangs under another, however deep. */
export function isUnder(layout: TreeLayout, key: string, ancestorKey: string): boolean {
  let parentKey = layout.byKey.get(key)?.parentKey ?? null;
  while (parentKey !== null) {
    if (parentKey === ancestorKey) {
      return true;
    }
    parentKey = layout.byKey.get(parentKey)?.parentKey ?? null;
  }
  return false;
}

/** What the view keeps when nothing is folded, and what unreadable content is read as. */
const NOTHING_FOLDED: ReadonlySet<string> = new Set();

/**
 * Reads what a mind map keeps of its own: the keys of the folded nodes.
 * Content that is missing, empty, damaged or of another shape is read as
 * nothing folded; it never fails.
 */
export function foldedIn(content: string | null): ReadonlySet<string> {
  if (content === null || content.trim() === '') {
    return NOTHING_FOLDED;
  }
  try {
    const kept: unknown = JSON.parse(content);
    if (typeof kept !== 'object' || kept === null || !('folded' in kept)) {
      return NOTHING_FOLDED;
    }
    const { folded } = kept;
    if (!Array.isArray(folded)) {
      return NOTHING_FOLDED;
    }
    const keys = folded.filter((key): key is string => typeof key === 'string');
    return keys.length === 0 ? NOTHING_FOLDED : new Set(keys);
  } catch {
    return NOTHING_FOLDED;
  }
}

/**
 * Writes what a mind map keeps: the keys of the folded nodes, in a fixed
 * order, so that the same folds are always the same text. A map with
 * nothing folded keeps nothing.
 */
export function contentOf(folded: ReadonlySet<string>): string | null {
  if (folded.size === 0) {
    return null;
  }
  return JSON.stringify({ version: 1, folded: [...folded].sort() });
}

/** The folds after one node was folded, or unfolded if it was folded. */
export function withFoldToggled(folded: ReadonlySet<string>, key: string): ReadonlySet<string> {
  const next = new Set(folded);
  if (!next.delete(key)) {
    next.add(key);
  }
  return next;
}

/**
 * The folds without those of nodes that are gone: of a task or an area
 * that was removed since. `isKnown` says whether a key still has a node.
 */
export function withoutGoneFolds(
  folded: ReadonlySet<string>,
  isKnown: (key: string) => boolean,
): ReadonlySet<string> {
  const kept = [...folded].filter(isKnown);
  return kept.length === folded.size ? folded : new Set(kept);
}
