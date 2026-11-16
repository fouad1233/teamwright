import { useQuery } from '@tanstack/react-query';
import {
  File,
  FileImage,
  FileSpreadsheet,
  FileText,
  Folder,
  Link2,
  MoreHorizontal,
  RotateCw,
  SlidersHorizontal,
} from 'lucide-react';
import { useEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactNode } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { FileEntry, Id, Project } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button, IconButton } from '@/shared/ui/Button';
import { TextInput } from '@/shared/ui/Field';
import { ActionMenu, type Action } from '@/shared/ui/Menu';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

import { FoldersDialog } from '../projects/FoldersDialog';
import { useProjectPlaces } from '../projects/projectQueries';
import { sinceInWords } from '../sync/syncModel';
import type { KindProps } from '../views/viewKinds';
import { useSetViewContent } from '../views/viewQueries';
import styles from './FilesView.module.css';
import {
  childPath,
  chosenFolder,
  crumbsOf,
  formatSize,
  fullPathOf,
  lastPartOf,
  markOf,
  narrowed,
  parentPath,
  parseContent,
  partsOf,
  serializeContent,
  type FilesContent,
  type Mark,
} from './filesModel';

/**
 * Where the keyboard lands in a folder once it is read: nowhere, on its
 * first row, or on the row of the folder the manager came up from.
 */
type Landing = boolean | string;

/** How long after the last change the view keeps where the manager is. */
const SAVE_AFTER_MS = 1000;

/** The small mark beside a name, in the ink of the text. */
const MARKS: Record<Mark, ReactNode> = {
  folder: <Folder aria-hidden />,
  text: <FileText aria-hidden />,
  picture: <FileImage aria-hidden />,
  sheet: <FileSpreadsheet aria-hidden />,
  link: <Link2 aria-hidden />,
  other: <File aria-hidden />,
};

/** What a mark says to a screen reader. */
const MARK_WORDS: Record<Mark, string> = {
  folder: 'Folder',
  text: 'Text',
  picture: 'Picture',
  sheet: 'Sheet',
  link: 'Link that leads outside the folder',
  other: 'File',
};

/**
 * The files view: the folders of the project at the left, what lies in the
 * chosen one at the right, as a quiet table that the keyboard walks. It
 * only reads; opening a file hands it to the program the computer has for
 * it. A view of its own for every files view, so that two do not share
 * where they stand.
 */
export function FilesView(props: KindProps) {
  return <FilesPage key={props.view.id} {...props} />;
}

function FilesPage({ view, project }: KindProps) {
  const places = useProjectPlaces(project.id).data;
  const [isChoosingFolders, setIsChoosingFolders] = useState(false);
  const [content, setContent] = useKeptContent(view);
  // Where the keyboard lands once the folder just opened is read, when it
  // was opened from a row, so that the walk can go on.
  const [walksRows, setWalksRows] = useState<Landing>(false);

  const folders = useMemo(
    () => (places === undefined ? [] : [places.workFolder, ...places.referenceFolders]),
    [places],
  );
  const folder = chosenFolder(folders, content.folder);
  // A folder that is no longer the project's starts again at its top.
  const path = folder === content.folder ? content.path : '';

  const listing = useFolderListing(project.id, folder, path, content.showsHidden);
  const isWithoutFiles =
    places !== undefined &&
    places.referenceFolders.length === 0 &&
    path === '' &&
    listing.data?.entries.length === 0;

  function openRoot(root: string) {
    setWalksRows(false);
    setContent({ ...content, folder: root, path: '' });
  }

  if (places === undefined) {
    return <div className={styles.files} aria-busy />;
  }
  if (folder === null || isWithoutFiles) {
    return (
      <div className={styles.files}>
        <div className={styles.empty}>
          <p className={styles.emptyLead}>Nothing lies in the folders of this project yet.</p>
          <p className={styles.emptyNote}>
            What the specialists write in the work folder shows here, with the folders you give them
            to read. Choose a folder that already holds the project, or add one to read.
          </p>
          <div className={styles.emptyActions}>
            <Button variant="secondary" onClick={() => setIsChoosingFolders(true)}>
              Choose folders
            </Button>
          </div>
        </div>
        <FoldersDialog
          open={isChoosingFolders}
          project={project}
          onClose={() => setIsChoosingFolders(false)}
        />
      </div>
    );
  }

  return (
    <div className={styles.files}>
      <nav className={styles.roots} aria-label="Folders of the project">
        <p className={classNames('label', styles.rootsHeading)}>Work folder</p>
        <RootButton
          folder={places.workFolder}
          isChosen={folder === places.workFolder}
          onChoose={() => openRoot(places.workFolder)}
        />
        {places.referenceFolders.length > 0 && (
          <p className={classNames('label', styles.rootsHeading)}>To read</p>
        )}
        {places.referenceFolders.map((reference) => (
          <RootButton
            key={reference}
            folder={reference}
            isChosen={folder === reference}
            onChoose={() => openRoot(reference)}
          />
        ))}
        <button
          type="button"
          className={styles.manageRoots}
          onClick={() => setIsChoosingFolders(true)}
        >
          Change the folders
        </button>
      </nav>
      <Listing
        // A new listing for each folder, so that the keyboard and the
        // narrowing start again at its top.
        key={`${folder}\n${path}`}
        project={project}
        folder={folder}
        path={path}
        content={content}
        listing={listing}
        takesFocus={walksRows}
        onGo={(nextPath, landing) => {
          setWalksRows(landing);
          setContent({ ...content, folder, path: nextPath });
        }}
        onShowHidden={(showsHidden) => setContent({ ...content, folder, path, showsHidden })}
      />
      <FoldersDialog
        open={isChoosingFolders}
        project={project}
        onClose={() => setIsChoosingFolders(false)}
      />
    </div>
  );
}

/** One folder of the project in the column at the left. */
function RootButton({
  folder,
  isChosen,
  onChoose,
}: {
  folder: string;
  isChosen: boolean;
  onChoose: () => void;
}) {
  return (
    <button
      type="button"
      className={classNames(styles.root, isChosen && styles.chosenRoot)}
      aria-current={isChosen || undefined}
      title={folder}
      onClick={onChoose}
    >
      <Folder aria-hidden />
      <span className={styles.rootName}>{lastPartOf(folder)}</span>
    </button>
  );
}

interface ListingProps {
  project: Project;
  folder: string;
  path: string;
  content: FilesContent;
  listing: ReturnType<typeof useFolderListing>;
  /** Where the keyboard lands once the folder is read. */
  takesFocus: Landing;
  onGo: (path: string, landing: Landing) => void;
  onShowHidden: (showsHidden: boolean) => void;
}

/** The crumbs, the bar and the table of one folder. */
function Listing({
  project,
  folder,
  path,
  content,
  listing,
  takesFocus,
  onGo,
  onShowHidden,
}: ListingProps) {
  const backend = useBackend();
  const notifier = useNotifier();
  const [typed, setTyped] = useState('');
  const [active, setActive] = useState(0);
  const rows = useRef<(HTMLTableRowElement | null)[]>([]);
  const entries = useMemo(
    () => narrowed(listing.data?.entries ?? [], typed),
    [listing.data, typed],
  );
  const crumbs = crumbsOf(folder, path);
  const hasRows = entries.length > 0;

  const hasLanded = useRef(false);
  const landingIndex =
    typeof takesFocus === 'string'
      ? Math.max(
          0,
          entries.findIndex((entry) => entry.name === takesFocus),
        )
      : 0;

  useEffect(() => {
    if (takesFocus !== false && hasRows && !hasLanded.current) {
      hasLanded.current = true;
      setActive(landingIndex);
      rows.current[landingIndex]?.focus();
    }
  }, [takesFocus, hasRows, landingIndex]);

  function focusRow(index: number) {
    const bounded = Math.max(0, Math.min(entries.length - 1, index));
    setActive(bounded);
    rows.current[bounded]?.focus();
  }

  function fail(failure: unknown) {
    notifier.showError(describeFailure(failure));
  }

  function open(entry: FileEntry) {
    if (entry.kind === 'folder') {
      onGo(childPath(path, entry.name), true);
    } else if (entry.kind === 'file') {
      backend.openProjectFile(project.id, folder, childPath(path, entry.name)).catch(fail);
    }
  }

  function show(entryPath: string) {
    backend.showProjectFile(project.id, folder, entryPath).catch(fail);
  }

  function copyPath(fullPath: string) {
    const tell = () => notifier.showNotice(`The path is ${fullPath}`);
    try {
      navigator.clipboard.writeText(fullPath).then(() => {
        notifier.showNotice('The path is copied.');
      }, tell);
    } catch {
      // A web view that lets nothing be copied.
      tell();
    }
  }

  function actionsOf(entry: FileEntry): Action[] {
    const entryPath = childPath(path, entry.name);
    return [
      {
        label: entry.kind === 'folder' ? 'Open' : 'Open with its program',
        isDisabled: entry.kind === 'link',
        onSelect: () => open(entry),
      },
      {
        label: 'Show in folder',
        isDisabled: entry.kind === 'link',
        onSelect: () => show(entryPath),
      },
      { label: 'Copy path', onSelect: () => copyPath(fullPathOf(folder, path, entry.name)) },
    ];
  }

  function onRowKey(event: KeyboardEvent, index: number, entry: FileEntry) {
    switch (event.key) {
      case 'ArrowDown':
        focusRow(index + 1);
        break;
      case 'ArrowUp':
        focusRow(index - 1);
        break;
      case 'Home':
        focusRow(0);
        break;
      case 'End':
        focusRow(entries.length - 1);
        break;
      case 'Enter':
        open(entry);
        break;
      case 'Backspace':
        if (path === '') {
          return;
        }
        onGo(parentPath(path), partsOf(path).at(-1) ?? true);
        break;
      default:
        return;
    }
    event.preventDefault();
  }

  const barActions: Action[] = [
    {
      label: content.showsHidden ? 'Hide hidden names' : 'Show hidden names',
      detail: 'Names that begin with a dot',
      onSelect: () => onShowHidden(!content.showsHidden),
    },
    { label: 'Show in folder', onSelect: () => show(path) },
    { label: 'Copy path', onSelect: () => copyPath(fullPathOf(folder, path)) },
  ];

  return (
    <section className={styles.listing} aria-label={`What lies in ${lastPartOf(folder)}`}>
      <div className={styles.bar}>
        <ol className={styles.crumbs} aria-label="Where you are">
          {crumbs.map((crumb, index) => {
            const isHere = index === crumbs.length - 1;
            return (
              <li key={crumb.path} className={styles.crumb}>
                {index > 0 && (
                  <span className={styles.slash} aria-hidden>
                    /
                  </span>
                )}
                <button
                  type="button"
                  className={classNames(styles.crumbButton, isHere && styles.here)}
                  aria-current={isHere ? 'location' : undefined}
                  disabled={isHere}
                  onClick={() => onGo(crumb.path, false)}
                >
                  {crumb.name}
                </button>
              </li>
            );
          })}
        </ol>
        <div className={styles.tools}>
          <TextInput
            type="search"
            className={styles.narrow}
            placeholder="Narrow the names"
            aria-label="Narrow the names"
            spellCheck={false}
            value={typed}
            onChange={(event) => {
              setTyped(event.target.value);
              setActive(0);
            }}
            onKeyDown={(event) => {
              if (event.key === 'ArrowDown' && entries.length > 0) {
                event.preventDefault();
                focusRow(0);
              }
            }}
          />
          <IconButton
            label="Read the folder again"
            className={styles.tool}
            onClick={() => void listing.refetch()}
          >
            <RotateCw aria-hidden />
          </IconButton>
          <ActionMenu label="This folder" actions={barActions}>
            <IconButton label="More" className={styles.tool}>
              <SlidersHorizontal aria-hidden />
            </IconButton>
          </ActionMenu>
        </div>
      </div>

      <div className={styles.scroller}>
        {listing.error !== null ? (
          <p className={styles.note} role="alert">
            {describeFailure(listing.error)}
          </p>
        ) : (
          <table className={styles.grid}>
            <colgroup>
              <col />
              <col className={styles.sizeColumn} />
              <col className={styles.changedColumn} />
              <col className={styles.menuColumn} />
            </colgroup>
            <thead>
              <tr>
                <th scope="col" className="label">
                  Name
                </th>
                <th scope="col" className={classNames('label', styles.figure)}>
                  Size
                </th>
                <th scope="col" className="label">
                  Changed
                </th>
                <th scope="col" aria-label="Actions" />
              </tr>
            </thead>
            <tbody>
              {entries.map((entry, index) => {
                const mark = markOf(entry);
                return (
                  <tr
                    key={entry.name}
                    ref={(row) => {
                      rows.current[index] = row;
                    }}
                    className={classNames(styles.row, entry.kind === 'link' && styles.linkRow)}
                    tabIndex={index === Math.min(active, entries.length - 1) ? 0 : -1}
                    onFocus={() => setActive(index)}
                    onKeyDown={(event) => onRowKey(event, index, entry)}
                    onDoubleClick={() => open(entry)}
                    title={
                      entry.kind === 'link'
                        ? 'This link leads outside the folder, and is not followed.'
                        : undefined
                    }
                  >
                    <td className={styles.nameCell}>
                      <div className={styles.nameLine}>
                        <span className={styles.mark} role="img" aria-label={MARK_WORDS[mark]}>
                          {MARKS[mark]}
                        </span>
                        <span
                          className={classNames(
                            styles.name,
                            entry.kind === 'folder' && styles.folderName,
                          )}
                        >
                          {entry.name}
                        </span>
                      </div>
                    </td>
                    <td className={classNames(styles.cell, styles.figure)}>
                      {formatSize(entry.size)}
                    </td>
                    <td className={styles.cell}>
                      {entry.changedAt === null ? '' : sinceInWords(entry.changedAt)}
                    </td>
                    <td className={styles.menuCell}>
                      <ActionMenu label={entry.name} actions={actionsOf(entry)}>
                        <IconButton label={`Actions for ${entry.name}`} className={styles.rowMenu}>
                          <MoreHorizontal aria-hidden />
                        </IconButton>
                      </ActionMenu>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
        {listing.data !== undefined && entries.length === 0 && listing.error === null && (
          <p className={styles.note}>
            {typed.trim() === '' ? 'This folder is empty.' : 'No name here holds that word.'}
          </p>
        )}
        {listing.data?.hasMore === true && (
          <p className={styles.note}>
            Only the first 2000 entries are shown. Open the folder in the file manager to see the
            rest.
          </p>
        )}
      </div>
    </section>
  );
}

/**
 * What lies in a folder, read again each time the view is shown and when
 * the manager asks. The disk is not watched.
 */
function useFolderListing(
  projectId: Id,
  folder: string | null,
  path: string,
  showsHidden: boolean,
) {
  const backend = useBackend();
  return useQuery({
    queryKey: ['projectFiles', projectId, folder, path, showsHidden],
    queryFn: () => backend.listProjectFiles(projectId, folder ?? '', path, showsHidden),
    enabled: folder !== null,
    staleTime: 0,
    refetchOnMount: 'always',
    retry: false,
  });
}

/**
 * Where the manager stands, kept in the content of the view about a second
 * after the last change, and at once when the view is left.
 */
function useKeptContent(view: KindProps['view']): [FilesContent, (next: FilesContent) => void] {
  const setViewContent = useSetViewContent();
  const [content, setContent] = useState(() => parseContent(view.content));
  const pending = useRef<string | null>(null);
  const timer = useRef<number | undefined>(undefined);
  const latestView = useRef(view);
  useEffect(() => {
    latestView.current = view;
  }, [view]);

  useEffect(() => {
    return () => {
      window.clearTimeout(timer.current);
      if (pending.current !== null) {
        void setViewContent(latestView.current, pending.current);
      }
    };
  }, [setViewContent]);

  function change(next: FilesContent) {
    setContent(next);
    const written = serializeContent(next);
    if (written === latestView.current.content) {
      pending.current = null;
      window.clearTimeout(timer.current);
      return;
    }
    pending.current = written;
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      if (pending.current !== null) {
        void setViewContent(latestView.current, pending.current);
        pending.current = null;
      }
    }, SAVE_AFTER_MS);
  }

  return [content, change];
}
