import {
  ChevronsUpDown,
  Ellipsis,
  ImageMinus,
  ImagePlus,
  SquareArrowOutUpRight,
} from 'lucide-react';
import {
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type ClipboardEvent,
  type DragEvent,
} from 'react';

import { useNavigation } from '@/app/navigation';
import type { Id, Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { taskReference } from '@/shared/lib/format';
import { IconButton, ValueButton } from '@/shared/ui/Button';
import { ActionMenu, ChoiceMenu, type Action, type Choice } from '@/shared/ui/Menu';
import { useNotifier } from '@/shared/ui/Toast';

import { StatusMark } from '../tasks/TaskBadges';
import { statusOf } from '../tasks/taskModel';
import { AssigneeAvatar } from '../team/Assignee';
import type { KindProps } from '../views/viewKinds';
import styles from './GalleryView.module.css';
import {
  addressOfPicture,
  fitPicture,
  isPictureFile,
  roomNote,
  weightInWords,
  withSize,
  withoutPicture,
  withoutPicturesOfGoneTasks,
  type CardSize,
  type GalleryContent,
} from './galleryModel';
import { reducePicture } from './reducePicture';
import { useGalleryContent } from './useGalleryContent';

const SIZES: readonly Choice<CardSize>[] = [
  { value: 'small', label: 'Small cards' },
  { value: 'medium', label: 'Medium cards' },
  { value: 'large', label: 'Large cards' },
];

/** What is said when a file is not a picture the gallery can keep. */
const NOT_A_PICTURE = 'Only a picture can go on a card: a JPEG, PNG, WebP or GIF file.';

/** What is said when the browser could not read a picture file. */
const UNREADABLE = 'This file could not be read as a picture.';

/** The one line that says how a picture gets onto a card. */
const HOW_TO_ADD =
  'Drop a picture on a card, paste one into a card that has the focus, or choose one from the card’s menu.';

/** Whether a drag carries files, which is all that can be known before the drop. */
function carriesFiles(event: DragEvent): boolean {
  return event.dataTransfer.types.includes('Files');
}

/**
 * Tasks as pictures in a grid. A card shows the picture the view keeps for
 * its task, or the task's key on a plate of its own, then the title, the
 * status and the person. A picture is dropped on a card, pasted into it, or
 * chosen from its menu; it is reduced and kept in the view's content, whose
 * weight is counted before anything is sent.
 */
export function GalleryView({ view, project, tasks, everyTask }: KindProps) {
  const navigation = useNavigation();
  const notifier = useNotifier();
  const [content, change] = useGalleryContent(view);
  // The tasks whose picture is being reduced right now.
  const [reducing, setReducing] = useState<ReadonlySet<Id>>(new Set());
  // The card a picture is being chosen for through the file field.
  const choosingFor = useRef<Id | null>(null);
  const fileField = useRef<HTMLInputElement>(null);
  const taskIds = useMemo(() => new Set(everyTask.map((task) => task.id)), [everyTask]);
  const note = roomNote(content);
  const hasPictures = tasks.some((task) => task.id in content.pictures);

  function changeContent(how: (content: GalleryContent) => GalleryContent) {
    change((before) => {
      const next = how(before);
      return next === before ? before : withoutPicturesOfGoneTasks(next, taskIds);
    });
  }

  /** Reduces a file and puts it on a task's card, or says why it cannot. */
  async function putPicture(taskId: Id, file: File | undefined) {
    if (file === undefined || !isPictureFile(file)) {
      notifier.showError(NOT_A_PICTURE);
      return;
    }
    setReducing((now) => new Set(now).add(taskId));
    const picture = await reducePicture(file);
    setReducing((now) => {
      const next = new Set(now);
      next.delete(taskId);
      return next;
    });
    if (picture === null) {
      notifier.showError(UNREADABLE);
      return;
    }
    // The weight is counted on the content as it is at this moment, which
    // `changeContent` hands over at once, before anything is sent.
    const outcome: { refusal: string | null } = { refusal: null };
    changeContent((before) => {
      const fitting = fitPicture(before, taskId, picture);
      if (!fitting.fits) {
        outcome.refusal = fitting.reason;
        return before;
      }
      return fitting.content;
    });
    if (outcome.refusal !== null) {
      notifier.showError(outcome.refusal);
    }
  }

  function chooseFor(taskId: Id) {
    choosingFor.current = taskId;
    fileField.current?.click();
  }

  function handleChosen(event: ChangeEvent<HTMLInputElement>) {
    const taskId = choosingFor.current;
    const file = event.target.files?.[0];
    // The same file may be chosen again for another card.
    event.target.value = '';
    choosingFor.current = null;
    if (taskId !== null && file !== undefined) {
      void putPicture(taskId, file);
    }
  }

  if (tasks.length === 0) {
    return (
      <div className={classNames(styles.empty, 'pattern-band')}>
        <p className={styles.emptyLead}>Tasks as pictures in a grid.</p>
        <p className={styles.emptyNote}>
          Every task of this view gets a card here, and a card can carry a picture: a photograph of
          the board, a drawing of the enclosure, a capture of the screen. Make a task first, then
          drop a picture on its card.
        </p>
      </div>
    );
  }

  return (
    <div
      className={styles.gallery}
      data-size={content.size}
      // A file dropped between the cards must not be opened by the window.
      onDragOver={(event) => {
        if (carriesFiles(event)) {
          event.preventDefault();
          event.dataTransfer.dropEffect = 'none';
        }
      }}
      onDrop={(event) => {
        event.preventDefault();
      }}
    >
      <div className={styles.bar}>
        {note !== null ? (
          <p className={classNames(styles.note, styles.little)} role="status">
            {note}
          </p>
        ) : hasPictures ? (
          <p className={classNames(styles.note, styles.weight)}>{weightInWords(content)}</p>
        ) : (
          <p className={styles.note}>{HOW_TO_ADD}</p>
        )}
        <ChoiceMenu
          label="Size of the cards"
          choices={SIZES}
          value={content.size}
          onChoose={(size) => {
            changeContent((before) => withSize(before, size));
          }}
        >
          <ValueButton className={styles.size} aria-label="Size of the cards">
            {SIZES.find((choice) => choice.value === content.size)?.label}
            <ChevronsUpDown aria-hidden />
          </ValueButton>
        </ChoiceMenu>
      </div>

      <ul className={styles.grid} aria-label="Tasks">
        {tasks.map((task) => (
          <li key={task.id} className={styles.cell}>
            <Card
              project={project}
              task={task}
              picture={content.pictures[task.id]}
              isReducing={reducing.has(task.id)}
              isOpenInPanel={navigation.taskId === task.id}
              onOpen={() => {
                navigation.openTask(task.id);
              }}
              onPicture={(file) => {
                void putPicture(task.id, file);
              }}
              onChoose={() => {
                chooseFor(task.id);
              }}
              onTakeOff={() => {
                changeContent((before) => withoutPicture(before, task.id));
              }}
            />
          </li>
        ))}
      </ul>

      <input
        ref={fileField}
        type="file"
        accept="image/*"
        className={styles.fileField}
        tabIndex={-1}
        aria-hidden
        onChange={handleChosen}
      />
    </div>
  );
}

interface CardProps {
  project: Project;
  task: Task;
  /** The picture the view keeps for the task, if it has one. */
  picture: string | undefined;
  isReducing: boolean;
  isOpenInPanel: boolean;
  onOpen: () => void;
  /** A picture file was dropped on the card or pasted into it. */
  onPicture: (file: File | undefined) => void;
  onChoose: () => void;
  onTakeOff: () => void;
}

/**
 * One task: its picture or its plate, its title, its status and its person.
 * The face opens the task; the menu in the corner chooses or takes off the
 * picture. A picture file can be dropped on the card, or pasted while the
 * face or the menu has the focus.
 */
function Card({
  project,
  task,
  picture,
  isReducing,
  isOpenInPanel,
  onOpen,
  onPicture,
  onChoose,
  onTakeOff,
}: CardProps) {
  const [isDropTarget, setIsDropTarget] = useState(false);
  const status = statusOf(project, task);
  const reference = taskReference(project, task);

  function handleDragOver(event: DragEvent) {
    if (!carriesFiles(event)) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    event.dataTransfer.dropEffect = 'copy';
    setIsDropTarget(true);
  }

  function handleDrop(event: DragEvent) {
    if (!carriesFiles(event)) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    setIsDropTarget(false);
    const files = [...event.dataTransfer.files];
    onPicture(files.find(isPictureFile) ?? files[0]);
  }

  function handlePaste(event: ClipboardEvent) {
    const files = [...event.clipboardData.files];
    if (files.length === 0) {
      return;
    }
    event.preventDefault();
    onPicture(files.find(isPictureFile) ?? files[0]);
  }

  const actions: Action[] = [
    {
      label: picture === undefined ? 'Choose a picture' : 'Choose another picture',
      icon: <ImagePlus />,
      onSelect: onChoose,
    },
    ...(picture === undefined
      ? []
      : [{ label: 'Take the picture off', icon: <ImageMinus />, onSelect: onTakeOff }]),
    { label: 'Open the task', icon: <SquareArrowOutUpRight />, onSelect: onOpen },
  ];

  return (
    <article
      className={classNames(
        styles.card,
        isDropTarget && styles.dropTarget,
        isOpenInPanel && styles.isOpen,
      )}
      aria-label={`${reference} ${task.title}`}
      onDragOver={handleDragOver}
      onDragEnter={handleDragOver}
      onDragLeave={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) {
          setIsDropTarget(false);
        }
      }}
      onDrop={handleDrop}
      onPaste={handlePaste}
    >
      <button type="button" className={styles.face} onClick={onOpen}>
        <span className={classNames(styles.plate, picture === undefined && styles.bare)}>
          {picture !== undefined && (
            <img className={styles.picture} src={addressOfPicture(picture)} alt="" />
          )}
          {picture === undefined && !isDropTarget && !isReducing && (
            <span className={styles.key} aria-hidden>
              {reference}
            </span>
          )}
          {(isDropTarget || isReducing) && (
            <span className={styles.drop} aria-hidden>
              {isReducing ? 'Reducing the picture' : 'Drop to put it on the card'}
            </span>
          )}
        </span>
        <span className={styles.caption}>
          <span className={styles.title}>{task.title}</span>
          <span className={styles.facts}>
            {status && (
              <span className={styles.mark} aria-hidden>
                <StatusMark status={status} />
              </span>
            )}
            <span className={styles.status}>{status?.name}</span>
            {picture !== undefined && <span className={styles.reference}>{reference}</span>}
            <span className={styles.person}>
              <AssigneeAvatar assignee={task.assignee} />
            </span>
          </span>
        </span>
      </button>

      <span className={styles.menu}>
        <ActionMenu label={`Picture of ${reference}`} actions={actions}>
          <IconButton label="Picture and task">
            <Ellipsis />
          </IconButton>
        </ActionMenu>
      </span>
    </article>
  );
}
