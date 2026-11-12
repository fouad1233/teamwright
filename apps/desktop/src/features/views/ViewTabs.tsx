import {
  DndContext,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core';
import { SortableContext, horizontalListSortingStrategy, useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { ChevronDown, ListFilter } from 'lucide-react';
import {
  createElement,
  useLayoutEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactElement,
  type ReactNode,
  type RefObject,
} from 'react';

import { PROJECT_PLACES, useNavigation, type ProjectPlace } from '@/app/navigation';
import type { Id, View, ViewKind } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { ActionMenu } from '@/shared/ui/Menu';

import { ViewChooser } from './ViewChooser';
import { kindOf } from './viewKinds';
import { ViewMenu } from './ViewMenu';
import { isFiltered, splitRow } from './viewModel';
import type { ViewCommands } from './viewQueries';
import styles from './ViewTabs.module.css';

/** How far the pointer must move before a press on a tab becomes a drag, in pixels. */
const DRAG_START_DISTANCE = 6;

/** What the places of a project are called on their tabs. */
const PLACE_LABELS: Record<ProjectPlace, string> = {
  team: 'Team',
  office: 'Office',
  advisor: 'Advisor',
};

interface ViewTabsProps {
  views: readonly View[];
  /** The view that is shown, if a view is shown and not a place. */
  shownView: View | undefined;
  /** The place that is shown, if a place is shown. */
  shownPlace: ProjectPlace | null;
  commands: ViewCommands;
}

/**
 * The row of tabs of a project: its views, which the manager adds, names,
 * filters, orders and removes, then a quiet divider, then its places, which
 * are always there. Views that do not fit the window are behind "N more".
 */
export function ViewTabs({ views, shownView, shownPlace, commands }: ViewTabsProps) {
  const navigation = useNavigation();
  const [menuFor, setMenuFor] = useState<Id | null>(null);
  const [renaming, setRenaming] = useState<Id | null>(null);
  const rowRef = useRef<HTMLDivElement>(null);
  const placesRef = useRef<HTMLDivElement>(null);
  const addRef = useRef<HTMLSpanElement>(null);
  const measureRef = useRef<HTMLDivElement>(null);
  const measures = useRowMeasures(views, { rowRef, placesRef, addRef, measureRef });
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: DRAG_START_DISTANCE } }),
  );

  const openIndex = shownView ? views.indexOf(shownView) : -1;
  // The tab being renamed is shown as well, so that its field is never behind "N more".
  const keptIndex = openIndex >= 0 ? openIndex : views.findIndex((view) => view.id === renaming);
  const { shown, hidden } = measures.isMeasured
    ? splitRow({ ...measures, openIndex: keptIndex >= 0 ? keptIndex : null })
    : { shown: views.map((_, index) => index), hidden: [] };
  const shownViews = shown.flatMap((index) => views[index] ?? []);
  const hiddenViews = hidden.flatMap((index) => views[index] ?? []);

  async function add(kind: ViewKind) {
    const added = await commands.add(kind);
    if (added) {
      navigation.showView(added.id);
      setRenaming(added.id);
    }
  }

  function handleDragEnd(event: DragEndEvent) {
    const over = event.over;
    if (!over || over.id === event.active.id) {
      return;
    }
    const index = views.findIndex((view) => view.id === over.id);
    if (index >= 0) {
      void commands.move(String(event.active.id), index);
    }
  }

  return (
    <div ref={rowRef} className={styles.row}>
      <div className={styles.views} role="tablist" aria-label="Views">
        <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
          <SortableContext
            items={shownViews.map((view) => view.id)}
            strategy={horizontalListSortingStrategy}
          >
            {shownViews.map((view) => {
              const index = views.indexOf(view);
              return (
                <ViewTab
                  key={view.id}
                  view={view}
                  isShown={view.id === shownView?.id && shownPlace === null}
                  isRenaming={renaming === view.id}
                  onShow={() => {
                    navigation.showView(view.id);
                  }}
                  onRenamed={(name) => {
                    setRenaming(null);
                    if (name !== null && name !== view.name) {
                      void commands.rename(view.id, name);
                    }
                  }}
                  menu={(trigger) => (
                    <ViewMenu
                      view={view}
                      index={index}
                      count={views.length}
                      open={menuFor === view.id}
                      onOpenChange={(open) => {
                        setMenuFor(open ? view.id : null);
                      }}
                      onRename={() => {
                        navigation.showView(view.id);
                        setRenaming(view.id);
                      }}
                      onFilter={(filter) => {
                        void commands.setFilter(view.id, filter);
                      }}
                      onMove={(target) => {
                        void commands.move(view.id, target);
                      }}
                      onRemove={() => {
                        void commands.remove(view.id);
                      }}
                    >
                      {trigger}
                    </ViewMenu>
                  )}
                  onOpenMenu={() => {
                    setMenuFor(view.id);
                  }}
                  onStartRenaming={() => {
                    setRenaming(view.id);
                  }}
                />
              );
            })}
          </SortableContext>
        </DndContext>

        {hiddenViews.length > 0 && (
          <ActionMenu
            label="More views"
            align="start"
            actions={hiddenViews.map((view) => ({
              label: view.name,
              icon: createElement(kindOf(view.kind).icon),
              onSelect: () => {
                navigation.showView(view.id);
              },
            }))}
          >
            <button type="button" className={styles.more}>
              {hiddenViews.length.toString()} more
              <ChevronDown aria-hidden />
            </button>
          </ActionMenu>
        )}
      </div>

      <span ref={addRef} className={styles.addSlot}>
        <ViewChooser
          views={views}
          onChoose={(kind) => {
            void add(kind);
          }}
        />
      </span>

      <span className={styles.divider} aria-hidden />

      <div ref={placesRef} className={styles.places} role="tablist" aria-label="Places">
        {PROJECT_PLACES.map((place) => (
          <button
            key={place}
            type="button"
            role="tab"
            aria-selected={shownPlace === place}
            className={classNames(styles.tab, styles.place, shownPlace === place && styles.active)}
            onClick={() => {
              navigation.showPlace(place);
            }}
          >
            {PLACE_LABELS[place]}
          </button>
        ))}
      </div>

      {/* A copy of every tab and of the button for the rest, out of sight, to
          know how wide each is before deciding which fit. */}
      <div ref={measureRef} className={styles.measure} aria-hidden>
        {views.map((view) => (
          <span key={view.id} className={styles.slot}>
            <span className={styles.tab}>
              <TabFace view={view} />
            </span>
            <span className={styles.menuButton} />
          </span>
        ))}
        <span className={styles.more}>
          {views.length.toString()} more
          <ChevronDown />
        </span>
      </div>
    </div>
  );
}

interface ViewTabProps {
  view: View;
  isShown: boolean;
  isRenaming: boolean;
  onShow: () => void;
  /** Called with the new name when renaming ends, or with nothing when it was given up. */
  onRenamed: (name: string | null) => void;
  /** Wraps the control that opens the menu of the view in that menu. */
  menu: (trigger: ReactElement) => ReactNode;
  onOpenMenu: () => void;
  onStartRenaming: () => void;
}

/**
 * The tab of one view. It is pressed to show the view, pressed twice to
 * rename it, dragged to move it, and has a menu: behind the mark at its
 * end, or a click with the right button anywhere on it.
 */
function ViewTab({
  view,
  isShown,
  isRenaming,
  onShow,
  onRenamed,
  menu,
  onOpenMenu,
  onStartRenaming,
}: ViewTabProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: view.id,
    disabled: isRenaming,
  });

  return (
    <div
      ref={setNodeRef}
      className={classNames(styles.slot, isShown && styles.shownSlot, isDragging && styles.dragged)}
      style={{
        transform: CSS.Translate.toString(transform),
        transition,
      }}
      onContextMenu={(event) => {
        event.preventDefault();
        onOpenMenu();
      }}
    >
      {isRenaming ? (
        <RenameField view={view} onDone={onRenamed} />
      ) : (
        <button
          {...attributes}
          {...listeners}
          type="button"
          // For the keyboard and a screen reader it is a tab. It is moved
          // with the menu; only the pointer drags it.
          role="tab"
          aria-roledescription={undefined}
          aria-selected={isShown}
          className={classNames(styles.tab, isShown && styles.active)}
          onClick={onShow}
          onDoubleClick={onStartRenaming}
        >
          <TabFace view={view} />
        </button>
      )}
      {menu(
        <button
          type="button"
          className={styles.menuButton}
          aria-label={`Menu of the view ${view.name}`}
        >
          <ChevronDown aria-hidden />
        </button>,
      )}
    </div>
  );
}

/** What a tab shows: the mark of its kind, its name, and a mark if it is filtered. */
function TabFace({ view }: { view: View }) {
  const kind = kindOf(view.kind);
  const filtered = isFiltered(view.filter);
  return (
    <>
      <span className={styles.kind} title={kind.label}>
        {createElement(kind.icon, { 'aria-hidden': true })}
      </span>
      <span className={styles.name}>{view.name}</span>
      {filtered && (
        <span className={styles.filtered} title="Filtered">
          <ListFilter aria-label="Filtered" />
        </span>
      )}
    </>
  );
}

interface RenameFieldProps {
  view: View;
  onDone: (name: string | null) => void;
}

/** The name of a view, ready to be changed in place. Enter keeps it, Escape gives it up. */
function RenameField({ view, onDone }: RenameFieldProps) {
  const [name, setName] = useState(view.name);
  const isDone = useRef(false);

  function finish(result: string | null) {
    if (isDone.current) {
      return;
    }
    isDone.current = true;
    onDone(result);
  }

  function handleKey(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'Enter') {
      event.preventDefault();
      finish(name);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      finish(null);
    }
  }

  return (
    <span className={styles.renaming}>
      <span className={styles.kind}>{createElement(kindOf(view.kind).icon)}</span>
      <input
        className={styles.rename}
        aria-label={`Name of the view ${view.name}`}
        value={name}
        // The field is as wide as the name in it, so the row does not jump.
        size={Math.max(name.length, 4)}
        maxLength={40}
        // Renaming starts from the menu, a double press or a new view: the
        // name is ready to be typed over.
        autoFocus
        onFocus={(event) => {
          event.currentTarget.select();
        }}
        onChange={(event) => {
          setName(event.target.value);
        }}
        onKeyDown={handleKey}
        onBlur={() => {
          finish(name);
        }}
      />
    </span>
  );
}

/** The parts of the row that are measured. */
interface RowParts {
  rowRef: RefObject<HTMLDivElement | null>;
  placesRef: RefObject<HTMLDivElement | null>;
  addRef: RefObject<HTMLSpanElement | null>;
  measureRef: RefObject<HTMLDivElement | null>;
}

/**
 * How wide the parts of the row are, read from the page when the views
 * change and whenever the row changes its width.
 */
function useRowMeasures(views: readonly View[], parts: RowParts) {
  const { rowRef, placesRef, addRef, measureRef } = parts;
  const [measures, setMeasures] = useState({
    isMeasured: false,
    widths: [] as number[],
    available: 0,
    gap: 0,
    moreWidth: 0,
  });

  useLayoutEffect(() => {
    const row = rowRef.current;
    const measure = measureRef.current;
    if (!row || !measure) {
      return;
    }
    const read = () => {
      const slots = [...measure.children].map((child) => child.getBoundingClientRect().width);
      const moreWidth = slots.pop() ?? 0;
      const rowStyle = getComputedStyle(row);
      const gap = parseFloat(getComputedStyle(measure).columnGap) || 0;
      const rowGap = parseFloat(rowStyle.columnGap) || 0;
      const others =
        (placesRef.current?.getBoundingClientRect().width ?? 0) +
        (addRef.current?.getBoundingClientRect().width ?? 0) +
        // The divider, and the space on either side of the three other parts.
        1 +
        rowGap * 3;
      const available =
        row.clientWidth -
        parseFloat(rowStyle.paddingLeft || '0') -
        parseFloat(rowStyle.paddingRight || '0') -
        others;
      setMeasures({ isMeasured: true, widths: slots, available, gap, moreWidth });
    };
    read();
    const observer = new ResizeObserver(read);
    observer.observe(row);
    return () => {
      observer.disconnect();
    };
  }, [views, rowRef, placesRef, addRef, measureRef]);

  return measures;
}
