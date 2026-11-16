import { ChevronRight, Plus } from 'lucide-react';
import {
  memo,
  useEffect,
  useEffectEvent,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type FocusEvent,
  type FormEvent,
  type KeyboardEvent,
  type MouseEvent,
  type PointerEvent,
} from 'react';

import { useNavigation } from '@/app/navigation';
import type { Area, Project } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { taskReference } from '@/shared/lib/format';
import { TextInput } from '@/shared/ui/Field';
import { GroundControls, GroundFrame, useGround } from '@/shared/ui/Ground';
import type { Box } from '@/shared/ui/groundModel';
import { Tooltip } from '@/shared/ui/Tooltip';

import { useOpenArea } from '../areas/AreaContext';
import { showsAreas } from '../areas/areaModel';
import { useAreas } from '../areas/areaQueries';
import { StatusMark } from '../tasks/TaskBadges';
import { statusOf } from '../tasks/taskModel';
import { useTaskCommands } from '../tasks/taskQueries';
import type { KindProps } from '../views/viewKinds';
import styles from './MindMapView.module.css';
import {
  COMPOSER_KEY,
  COMPOSER_WIDTH,
  PROJECT_KEY,
  addsUnder,
  areaKey,
  fanPath,
  isUnder,
  layoutTree,
  mapTree,
  neighbourOf,
  taskKey,
  withFoldToggled,
  withoutGoneFolds,
  type MapNode,
  type PlacedNode,
  type Step,
  type TreeLayout,
} from './mindMapModel';
import { useKeptFolds } from './useKeptFolds';

/** The width a node is laid out with until the page has measured it, in pixels. */
const UNMEASURED_WIDTH = 200;

/** The size of the "+" beside the selected node, and how far it stands from the node, in pixels. */
const ADD_SIZE = 20;
const ADD_GAP = 6;

/** The areas while they are loading. */
const NO_AREAS: readonly Area[] = [];

/** The widths before any node has been measured. */
const NOTHING_MEASURED: ReadonlyMap<string, number> = new Map();

/** Where each arrow key leads. */
const STEPS: Partial<Record<string, Step>> = {
  ArrowLeft: 'left',
  ArrowRight: 'right',
  ArrowUp: 'up',
  ArrowDown: 'down',
};

/** A new task while its title is typed: the node it will hang on, and the title so far. */
interface Composing {
  under: string;
  title: string;
}

/** The key of the node an event happened on, if it happened on one. */
function nodeKeyOf(target: EventTarget): string | null {
  if (!(target instanceof Element)) {
    return null;
  }
  return target.closest<HTMLElement>('[data-node]')?.dataset.node ?? null;
}

/** The place of a node as a box of the ground. */
function boxOf({ x, y, width, height }: PlacedNode): Box {
  return { x, y, width, height };
}

/** Whether two sets of measured widths say the same. */
function sameWidths(one: ReadonlyMap<string, number>, other: ReadonlyMap<string, number>): boolean {
  if (one.size !== other.size) {
    return false;
  }
  for (const [key, width] of one) {
    if (other.get(key) !== width) {
      return false;
    }
  }
  return true;
}

/**
 * Gives a node or a field the focus without the browser moving anything to
 * show it: the ground brings it into view itself.
 */
function focusQuietly(element: HTMLElement | null) {
  element?.focus({ preventScroll: true });
}

/**
 * The project as a tree that grows to the right: the project, its areas if
 * it has more than one, their tasks, and the subtasks of each task, joined
 * by thin curved lines. The tree lies on a ground that is dragged and
 * zoomed.
 *
 * A press selects a node; a second press, or Enter, opens its task. The
 * arrow keys step from node to node, and Space folds a branch. A selected
 * node has a "+" that adds a task under it. Which branches are folded is
 * kept in the content of the view.
 */
export function MindMapView({ view, project, tasks, everyTask, assigneeOfNewTasks }: KindProps) {
  const navigation = useNavigation();
  const commands = useTaskCommands(project.id);
  const folds = useKeptFolds(view);
  const loadedAreas = useAreas(project.id).data;
  const everyArea = loadedAreas ?? NO_AREAS;
  const openArea = useOpenArea();
  const sheet = useRef<HTMLDivElement>(null);
  // The node a press began on, if it was the selected one: only its click opens the task.
  const pressedSelected = useRef<string | null>(null);
  // Whether a task typed in the composer is on its way to the backend, so
  // that a second Enter does not make it twice.
  const isCreating = useRef(false);
  // Whether the focus is being moved by a step of the arrow keys, which
  // selects its node itself.
  const isStepping = useRef(false);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [composing, setComposing] = useState<Composing | null>(null);
  const [widths, setWidths] = useState(NOTHING_MEASURED);

  // With one area open, the tree is that branch alone. A project that is
  // not cut into areas shows none.
  const areas = useMemo(
    () => (openArea ? [openArea] : showsAreas(everyArea) ? everyArea : NO_AREAS),
    [openArea, everyArea],
  );
  const composingUnder = composing?.under ?? null;
  const tree = useMemo(
    () => mapTree({ areas, tasks, folded: folds.folded, composingUnder }),
    [areas, tasks, folds.folded, composingUnder],
  );
  const widthOf = useMemo(
    () => (node: MapNode) =>
      node.kind === 'composer' ? COMPOSER_WIDTH : (widths.get(node.key) ?? UNMEASURED_WIDTH),
    [widths],
  );
  const layout = useMemo(() => layoutTree(tree, widthOf), [tree, widthOf]);
  const content = useMemo(
    () => ({ x: 0, y: 0, width: layout.width, height: layout.height }),
    [layout.width, layout.height],
  );
  const ground = useGround({ content });
  // The keys are set as wide as the longest of them, so that the titles
  // begin on one line down a column.
  const longestReference = useMemo(
    () =>
      tasks.reduce((longest, task) => Math.max(longest, taskReference(project, task).length), 0),
    [tasks, project],
  );

  // A node is as wide as its words. The page measures every node once it is
  // drawn, and again whenever one changes its size, as when a typeface
  // arrives; the columns are then laid out by what was measured.
  useLayoutEffect(() => {
    const element = sheet.current;
    if (!element) {
      return;
    }
    const read = () => {
      const measured = new Map<string, number>();
      for (const node of element.querySelectorAll<HTMLElement>('[data-node]')) {
        measured.set(node.dataset.node ?? '', node.offsetWidth);
      }
      setWidths((known) => (sameWidths(known, measured) ? known : measured));
    };
    read();
    const observer = new ResizeObserver(read);
    for (const node of element.querySelectorAll('[data-node]')) {
      observer.observe(node);
    }
    return () => {
      observer.disconnect();
    };
  }, [tree]);

  const selected = selectedKey === null ? undefined : layout.byKey.get(selectedKey);
  const composer = layout.byKey.get(COMPOSER_KEY);
  const composerParent = composingUnder === null ? undefined : layout.byKey.get(composingUnder);
  const adds = selected ? addsUnder(selected.node) : null;

  // The composer comes into view when it opens, and stays there while the
  // tasks made with it are added above it.
  const showComposer = useEffectEvent(() => {
    if (composer) {
      ground.reveal(boxOf(composer));
    }
  });
  const composerX = composer?.x;
  const composerY = composer?.y;
  useEffect(() => {
    if (composerX !== undefined && composerY !== undefined) {
      showComposer();
    }
  }, [composerX, composerY]);

  /** Whether a fold still has a node: the folds of tasks and areas that are gone are forgotten. */
  function isKnownFold(key: string): boolean {
    if (key.startsWith('area:')) {
      // While the areas load, none of them is taken for gone.
      return loadedAreas === undefined || everyArea.some((area) => areaKey(area.id) === key);
    }
    return everyTask.some((task) => taskKey(task.id) === key);
  }

  /**
   * Changes what the tree shows, and holds one node where it is on screen:
   * the tree is laid out again around it, so that what was pressed does
   * not slip away from under the pointer.
   */
  function retree(
    change: { folded?: ReadonlySet<string>; composing?: Composing | null },
    heldKey: string,
  ) {
    const nextFolded = change.folded ?? folds.folded;
    const nextComposing = change.composing === undefined ? composing : change.composing;
    const before = layout.byKey.get(heldKey);
    const after = layoutTree(
      mapTree({ areas, tasks, folded: nextFolded, composingUnder: nextComposing?.under ?? null }),
      widthOf,
    ).byKey.get(heldKey);
    if (change.folded !== undefined) {
      folds.fold(nextFolded);
    }
    if (change.composing !== undefined) {
      setComposing(nextComposing);
    }
    if (before && after) {
      const { scale } = ground.placement;
      ground.moveBy((before.x - after.x) * scale, (before.y - after.y) * scale);
    }
  }

  function toggleFold(key: string) {
    const folded = withFoldToggled(withoutGoneFolds(folds.folded, isKnownFold), key);
    // What is typed under the branch goes with it.
    const hidesComposer =
      composingUnder !== null && (composingUnder === key || isUnder(layout, composingUnder, key));
    retree({ folded, ...(hidesComposer ? { composing: null } : {}) }, key);
    if (selectedKey !== null && isUnder(layout, selectedKey, key)) {
      setSelectedKey(key);
    }
  }

  function select(key: string) {
    if (composing && composing.under !== key && composing.title.trim() === '') {
      // An empty line under another node has served its purpose.
      retree({ composing: null }, key);
    }
    setSelectedKey(key);
  }

  function open(key: string) {
    const node = layout.byKey.get(key)?.node;
    if (node?.kind === 'task' || node?.kind === 'subtask') {
      navigation.openTask(node.task.id);
    }
  }

  function focusNode(key: string) {
    focusQuietly(
      sheet.current?.querySelector<HTMLElement>(`[data-node="${CSS.escape(key)}"]`) ?? null,
    );
  }

  /** Steps to a node with the keyboard: it is in view, is selected, and has the focus. */
  function goTo(key: string) {
    const placed = layout.byKey.get(key);
    if (!placed) {
      return;
    }
    // First into view, where it lies now; selecting it may lay the tree out
    // again, and holds it where it then is.
    ground.reveal(boxOf(placed));
    select(key);
    isStepping.current = true;
    focusNode(key);
    isStepping.current = false;
  }

  function startComposing(key: string) {
    const placed = layout.byKey.get(key);
    if (!placed) {
      return;
    }
    const folded = placed.node.isFolded ? withFoldToggled(folds.folded, key) : undefined;
    retree({ composing: { under: key, title: '' }, ...(folded ? { folded } : {}) }, key);
  }

  function stopComposing(returnsFocus: boolean) {
    if (!composing) {
      return;
    }
    retree({ composing: null }, composing.under);
    if (returnsFocus) {
      setSelectedKey(composing.under);
      focusNode(composing.under);
    }
  }

  async function createTask() {
    if (!composing || !composerParent || isCreating.current) {
      return;
    }
    const title = composing.title.trim();
    if (title === '') {
      stopComposing(true);
      return;
    }
    const parent = composerParent.node;
    isCreating.current = true;
    const created = await commands.create({
      title,
      assignee: assigneeOfNewTasks,
      ...(parent.kind === 'task' || parent.kind === 'subtask' ? { parentId: parent.task.id } : {}),
      ...(parent.kind === 'area' ? { areaId: parent.area.id } : {}),
    });
    isCreating.current = false;
    if (created) {
      // The line stays open for the next one, as it does on the board.
      setComposing((current) =>
        current?.under === composing.under ? { ...current, title: '' } : current,
      );
    }
  }

  function handleTreePointerDown(event: PointerEvent<HTMLDivElement>) {
    const key = nodeKeyOf(event.target);
    pressedSelected.current = key !== null && key === selectedKey ? key : null;
  }

  function handleTreeClick(event: MouseEvent<HTMLDivElement>) {
    const key = nodeKeyOf(event.target);
    if (key === null) {
      return;
    }
    if (event.target instanceof Element && event.target.closest('[data-fold]')) {
      toggleFold(key);
    } else if (pressedSelected.current === key) {
      open(key);
    } else {
      select(key);
    }
  }

  function handleTreeFocus(event: FocusEvent<HTMLDivElement>) {
    // A node the Tab key stops at is brought into view and selected. A node
    // that is pressed is selected by its click, and a node that is stepped
    // to by the step itself.
    const node: EventTarget = event.target;
    if (isStepping.current || !(node instanceof HTMLElement)) {
      return;
    }
    if (node.matches('[data-node]:focus-visible')) {
      const placed = layout.byKey.get(node.dataset.node ?? '');
      if (placed) {
        ground.reveal(boxOf(placed));
        select(placed.node.key);
      }
    }
  }

  function handleTreeKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const node = event.target;
    // A key on the fold of a node, or with Ctrl or Alt, is not a step.
    if (!(node instanceof HTMLElement) || !node.matches('[data-node]')) {
      return;
    }
    if (event.ctrlKey || event.metaKey || event.altKey) {
      return;
    }
    const placed = layout.byKey.get(node.dataset.node ?? '');
    if (!placed) {
      return;
    }
    const { key } = placed.node;
    const step = STEPS[event.key];
    if (event.key === 'Enter') {
      open(key);
    } else if (event.key === ' ') {
      if (placed.node.canFold) {
        toggleFold(key);
      }
    } else if (event.key === 'Home') {
      goTo(PROJECT_KEY);
    } else if (step === 'right' && placed.node.isFolded) {
      toggleFold(key);
    } else if (step !== undefined) {
      const next = neighbourOf(layout, key, step);
      if (next !== null) {
        goTo(next);
      }
    } else {
      return;
    }
    event.preventDefault();
  }

  function handleGroundClick(event: MouseEvent<HTMLDivElement>) {
    // A press on the bare ground lets the selection go.
    if (event.target instanceof Element && !event.target.closest('[data-node], button, form')) {
      setSelectedKey(null);
    }
  }

  if (tasks.length === 0) {
    return (
      <div className={styles.empty}>
        <p className={styles.emptyLead}>The project as a tree.</p>
        <p className={styles.emptyNote}>
          The project stands at the left, and its areas, its tasks and their subtasks branch out to
          the right. Make a first task with “New task”, and the tree begins here.
        </p>
      </div>
    );
  }

  const tabStopKey = selected ? selected.node.key : PROJECT_KEY;

  return (
    <div className={styles.view}>
      <GroundFrame
        ground={ground}
        className={styles.frame}
        tabIndex={-1}
        onClick={handleGroundClick}
      >
        <div
          ref={sheet}
          className={styles.sheet}
          style={
            {
              width: layout.width,
              height: layout.height,
              '--reference-length': longestReference,
            } as CSSProperties
          }
        >
          <Joins layout={layout} />

          <div
            role="tree"
            aria-label={`Mind map of ${project.name}`}
            onPointerDown={handleTreePointerDown}
            onClick={handleTreeClick}
            onFocus={handleTreeFocus}
            onKeyDown={handleTreeKeyDown}
          >
            {layout.nodes.map((placed) =>
              placed.node.kind === 'composer' ? null : (
                <NodePlate
                  key={placed.node.key}
                  project={project}
                  placed={placed}
                  isSelected={placed === selected}
                  isTabStop={placed.node.key === tabStopKey}
                  isOpenInPanel={
                    (placed.node.kind === 'task' || placed.node.kind === 'subtask') &&
                    placed.node.task.id === navigation.taskId
                  }
                />
              ),
            )}
          </div>

          {selected && adds !== null && composingUnder !== selected.node.key && (
            <Tooltip label={adds === 'subtask' ? 'Add a subtask' : 'Add a task'} side="top">
              <button
                type="button"
                className={styles.add}
                style={{
                  left: selected.x + selected.width + ADD_GAP,
                  top: selected.y + (selected.height - ADD_SIZE) / 2,
                  width: ADD_SIZE,
                  height: ADD_SIZE,
                }}
                aria-label={adds === 'subtask' ? 'Add a subtask' : 'Add a task'}
                onClick={() => {
                  startComposing(selected.node.key);
                }}
              >
                <Plus aria-hidden />
              </button>
            </Tooltip>
          )}

          {composing && composer && composer.node.kind === 'composer' && (
            <form
              className={styles.composer}
              style={{
                left: composer.x,
                top: composer.y,
                width: composer.width,
                height: composer.height,
              }}
              onSubmit={(event: FormEvent) => {
                event.preventDefault();
                void createTask();
              }}
            >
              <TextInput
                // A line of its own under every node, so that nothing typed
                // under one lingers under another.
                key={composing.under}
                ref={focusQuietly}
                value={composing.title}
                placeholder={
                  composer.node.makes === 'subtask'
                    ? 'A part of this task'
                    : 'What needs to be done?'
                }
                aria-label={
                  composer.node.makes === 'subtask'
                    ? 'Title of the new subtask'
                    : 'Title of the new task'
                }
                onChange={(event) => {
                  setComposing({ under: composing.under, title: event.target.value });
                }}
                onKeyDown={(event) => {
                  if (event.key === 'Escape') {
                    stopComposing(true);
                  }
                }}
                onBlur={(event) => {
                  // Leaving the empty line closes it. A press on a node
                  // closes it by its click, once the click is over.
                  const goesToNode = event.relatedTarget?.closest('[data-node]') != null;
                  if (composing.title.trim() === '' && !goesToNode) {
                    stopComposing(false);
                  }
                }}
              />
            </form>
          )}
        </div>
      </GroundFrame>

      <GroundControls ground={ground} className={styles.controls} />
    </div>
  );
}

/** The joins of the tree: one line from every node to the nodes under it. */
const Joins = memo(function Joins({ layout }: { layout: TreeLayout }) {
  return (
    <svg className={styles.joins} width={layout.width} height={layout.height} aria-hidden>
      {/* Half a pixel down: a level line then fills one row of pixels, and is sharp. */}
      <g transform="translate(0 0.5)">
        {layout.fans.map((fan) => (
          <path key={fan.parentKey} d={fanPath(fan)} />
        ))}
      </g>
    </svg>
  );
});

function tasksWord(count: number): string {
  return count === 1 ? 'task' : 'tasks';
}

interface NodePlateProps {
  project: Project;
  placed: PlacedNode;
  isSelected: boolean;
  /** Whether the Tab key stops at this node: one node of the tree does. */
  isTabStop: boolean;
  /** Whether the panel at the side shows the task of this node. */
  isOpenInPanel: boolean;
}

/**
 * One node of the tree, as a small plate drawn by what it is: the project
 * in the serif, an area as a label, a task with the mark of its status,
 * its key and its title, and a subtask the same, a little smaller and
 * quieter. A node with others under it ends in the mark that folds them,
 * which says how many tasks it hides while it is folded.
 *
 * It handles nothing itself: the tree listens for all its nodes.
 */
const NodePlate = memo(function NodePlate({
  project,
  placed,
  isSelected,
  isTabStop,
  isOpenInPanel,
}: NodePlateProps) {
  const { node } = placed;
  const isTask = node.kind === 'task' || node.kind === 'subtask';
  const status = isTask ? statusOf(project, node.task) : undefined;
  const hidden = `${node.taskCount.toString()} ${tasksWord(node.taskCount)}`;
  let name = '';
  let hint: string | undefined;
  if (node.kind === 'project') {
    name = `Project ${project.name}`;
    hint = project.description || undefined;
  } else if (node.kind === 'area') {
    name = `Area ${node.area.name}, ${hidden}`;
    hint = `${node.area.name}: ${hidden}`;
  } else if (isTask) {
    const reference = taskReference(project, node.task);
    name = `${reference} ${node.task.title}${status ? `, ${status.name}` : ''}`;
    hint = `${reference} ${node.task.title}${status ? `\n${status.name}` : ''}`;
  }

  return (
    <div
      role="treeitem"
      tabIndex={isTabStop ? 0 : -1}
      aria-level={placed.depth + 1}
      aria-posinset={placed.position}
      aria-setsize={placed.siblingCount}
      aria-expanded={node.canFold ? !node.isFolded : undefined}
      aria-selected={isSelected}
      aria-label={name}
      title={hint}
      data-node={node.key}
      data-progress={status?.category}
      className={classNames(
        styles.node,
        styles[node.kind],
        isSelected && styles.selected,
        isOpenInPanel && styles.open,
      )}
      style={{ left: placed.x, top: placed.y, height: placed.height }}
    >
      <span className={styles.face}>
        {node.kind === 'project' && <span className={styles.projectName}>{project.name}</span>}
        {node.kind === 'area' && (
          <span className={classNames('label', styles.areaName)}>{node.area.name}</span>
        )}
        {isTask && (
          <>
            {status && <StatusMark status={status} />}
            <span className={styles.reference}>{taskReference(project, node.task)}</span>
            <span className={styles.title}>{node.task.title}</span>
          </>
        )}
      </span>

      {node.canFold && (
        <button
          type="button"
          // The keys of the node fold it; the Tab key need not stop here too.
          tabIndex={-1}
          data-fold
          className={styles.fold}
          aria-label={node.isFolded ? `Unfold, ${hidden} hidden` : 'Fold'}
          title={node.isFolded ? `${hidden} hidden. Press to unfold` : 'Fold this branch'}
        >
          <ChevronRight className={styles.chevron} aria-hidden />
          {node.isFolded && node.taskCount}
        </button>
      )}
    </div>
  );
});
