import { Ellipsis, Pencil, Plus, Search, Trash2 } from 'lucide-react';
import { useDeferredValue, useState } from 'react';

import { useNavigation } from '@/app/navigation';
import type { MemoryEntry, Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatMoment, taskReference } from '@/shared/lib/format';
import { Button, IconButton } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { TextInput } from '@/shared/ui/Field';
import { ActionMenu } from '@/shared/ui/Menu';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

import { useTeamMembers } from '../team/TeamContext';
import { useMemoryOfProject } from './MemoryContext';
import { MemoryEntryDialog } from './MemoryEntryDialog';
import {
  KIND_CHOICES,
  KIND_LABELS,
  listingOf,
  sourceInWords,
  type KindChoice,
} from './memoryModel';
import { useMemoryCommands, useMemorySearch } from './memoryQueries';
import styles from './MemoryView.module.css';

interface MemoryViewProps {
  project: Project;
  /** Every task of the project, to name the task an entry belongs to. */
  tasks: Task[];
}

/** What the dialog of an entry is doing: nothing, adding a fact, or changing an entry. */
type Editing = { kind: 'closed' } | { kind: 'adding' } | { kind: 'changing'; entry: MemoryEntry };

/**
 * The memory of a project: the decisions the manager made, the facts the
 * team recorded and the contracts of handoffs, which every specialist can
 * search. The manager reads it here, searches it with the same rule as the
 * specialists, adds facts, and changes or removes entries.
 */
export function MemoryView({ project, tasks }: MemoryViewProps) {
  const memory = useMemoryOfProject();
  const commands = useMemoryCommands(project.id);
  const notifier = useNotifier();
  const [query, setQuery] = useState('');
  const searched = useDeferredValue(query);
  const search = useMemorySearch(project.id, searched);
  const [kind, setKind] = useState<KindChoice>('all');
  const [showsReplaced, setShowsReplaced] = useState(false);
  const [editing, setEditing] = useState<Editing>({ kind: 'closed' });
  const [removing, setRemoving] = useState<MemoryEntry | null>(null);

  const isSearching = searched.trim() !== '';
  const listing = listingOf(
    memory ?? [],
    isSearching ? (search.data ?? []) : undefined,
    kind,
    showsReplaced,
  );
  const changing = editing.kind === 'changing' ? editing.entry : null;

  async function remove(entry: MemoryEntry) {
    setRemoving(null);
    try {
      await commands.remove(entry.id);
    } catch (failure) {
      notifier.showError(describeFailure(failure));
    }
  }

  return (
    <div className={styles.view}>
      <div className={styles.sheet}>
        <header className={styles.bar}>
          <label className={styles.search}>
            <Search className={styles.searchMark} aria-hidden />
            <TextInput
              type="search"
              value={query}
              onChange={(event) => {
                setQuery(event.target.value);
              }}
              placeholder="Search the memory"
              aria-label="Search the memory"
              className={styles.searchInput}
            />
          </label>
          <div className={styles.switch} role="group" aria-label="Kinds of entries">
            {KIND_CHOICES.map(({ choice, label }) => (
              <button
                key={choice}
                type="button"
                className={classNames(styles.choice, kind === choice && styles.chosen)}
                aria-pressed={kind === choice}
                onClick={() => {
                  setKind(choice);
                }}
              >
                {label}
                <span className={styles.count}>{listing.counts[choice].toString()}</span>
              </button>
            ))}
          </div>
          <span className={styles.space} />
          <Button
            icon={<Plus />}
            onClick={() => {
              setEditing({ kind: 'adding' });
            }}
          >
            Add a fact
          </Button>
        </header>

        {search.isError && isSearching && (
          <p className={styles.note} role="alert">
            {describeFailure(search.error)}
          </p>
        )}

        {memory !== undefined && memory.length === 0 ? (
          <p className={styles.empty}>
            Nothing is remembered yet. When you answer a specialist&apos;s question, the decision is
            kept here; so is the interface contract of a handoff, and every fact the team records.
            Specialists search it before they ask you something twice.
          </p>
        ) : listing.shown.length === 0 && memory !== undefined ? (
          <p className={styles.empty}>
            {isSearching
              ? `No entry fits “${searched.trim()}”. An entry fits when every word begins a word of it.`
              : 'No entry of this kind.'}
          </p>
        ) : (
          <ol className={styles.entries}>
            {listing.shown.map((entry) => (
              <li key={entry.id}>
                <EntryView
                  entry={entry}
                  project={project}
                  tasks={tasks}
                  onChange={() => {
                    setEditing({ kind: 'changing', entry });
                  }}
                  onRemove={() => {
                    setRemoving(entry);
                  }}
                />
              </li>
            ))}
          </ol>
        )}

        {!isSearching && listing.replaced > 0 && (
          <button
            type="button"
            className={styles.replacedSwitch}
            aria-pressed={showsReplaced}
            onClick={() => {
              setShowsReplaced(!showsReplaced);
            }}
          >
            {showsReplaced
              ? 'Hide the entries newer ones replaced'
              : `Show ${listing.replaced.toString()} ${
                  listing.replaced === 1 ? 'entry' : 'entries'
                } newer ones replaced`}
          </button>
        )}
      </div>

      <MemoryEntryDialog
        open={editing.kind !== 'closed'}
        entry={changing}
        onSave={(title, text) =>
          changing === null
            ? commands.addFact(title, text)
            : commands.change(changing.id, title, text)
        }
        onClose={() => {
          setEditing({ kind: 'closed' });
        }}
      />

      <Dialog
        open={removing !== null}
        title="Remove this entry?"
        onClose={() => {
          setRemoving(null);
        }}
      >
        <p className={styles.warning}>
          {removing?.title}: no specialist will find it again, and it cannot be brought back. To say
          that it no longer holds, change it instead.
        </p>
        <div className={styles.dialogActions}>
          <Button
            variant="ghost"
            data-initial-focus
            onClick={() => {
              setRemoving(null);
            }}
          >
            Cancel
          </Button>
          <Button
            variant="danger"
            onClick={() => {
              if (removing) {
                void remove(removing);
              }
            }}
          >
            Remove entry
          </Button>
        </div>
      </Dialog>
    </div>
  );
}

interface EntryViewProps {
  entry: MemoryEntry;
  project: Project;
  tasks: Task[];
  onChange: () => void;
  onRemove: () => void;
}

/** One entry: its kind and title, its text, and where and when it came from. */
function EntryView({ entry, project, tasks, onChange, onRemove }: EntryViewProps) {
  const members = useTeamMembers();
  const navigation = useNavigation();
  const task = tasks.find((other) => other.id === entry.taskId);
  const isReplaced = entry.replacedBy !== null;

  return (
    <article className={classNames(styles.entry, isReplaced && styles.replaced)}>
      <header className={styles.heading}>
        <span className={classNames('label', styles.kind, styles[entry.kind])}>
          {KIND_LABELS[entry.kind]}
        </span>
        <h3 className={styles.title}>{entry.title}</h3>
        {!isReplaced && (
          <ActionMenu
            label={`Actions for ${entry.title}`}
            actions={[
              { label: 'Change', icon: <Pencil />, onSelect: onChange },
              { label: 'Remove', icon: <Trash2 />, isDestructive: true, onSelect: onRemove },
            ]}
          >
            <IconButton label={`Actions for ${entry.title}`}>
              <Ellipsis />
            </IconButton>
          </ActionMenu>
        )}
      </header>
      <p
        className={classNames(styles.text, entry.kind === 'contract' && styles.contract)}
        data-selectable
      >
        {entry.text}
      </p>
      <p className={styles.origin}>
        <span>{sourceInWords(entry, members)}</span>
        {task && (
          <button
            type="button"
            className={styles.task}
            onClick={() => {
              navigation.openTask(task.id);
            }}
          >
            {taskReference(project, task)} {task.title}
          </button>
        )}
        {entry.taskId !== null && !task && <span>A task that was deleted</span>}
        <span className={styles.moment}>{formatMoment(entry.recordedAt)}</span>
        {isReplaced && <span className={styles.replacedNote}>Replaced by a newer entry</span>}
      </p>
    </article>
  );
}
