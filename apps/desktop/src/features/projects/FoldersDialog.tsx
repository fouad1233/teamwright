import { FilePlus, FolderOpen, FolderPlus, Trash2 } from 'lucide-react';
import { useState } from 'react';

import { useIsDemo } from '@/shared/api/BackendProvider';
import type { Project } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button, IconButton } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { FormActions, Refusal } from '@/shared/ui/Form';
import { Switch } from '@/shared/ui/Switch';
import { describeFailure } from '@/shared/ui/Toast';

import { useCoordinator } from '../coordinator/coordinatorQueries';
import styles from './FoldersDialog.module.css';
import { FULL_ACCESS_WORDS, coordinatorWithFullAccess, copiedInWords } from './placesModel';
import { usePlacesCommands, useProjectPlaces } from './projectQueries';

interface FoldersDialogProps {
  open: boolean;
  project: Project;
  onClose: () => void;
}

/**
 * Where the specialists of a project work on this computer, and what else
 * they may read or change: the work folder, further folders to write in,
 * the folders with reference material, a way to hand them single files,
 * and full access (decisions 0013 and 0044).
 */
export function FoldersDialog({ open, project, onClose }: FoldersDialogProps) {
  return (
    <Dialog open={open} title={`Folders of ${project.name}`} size="wide" onClose={onClose}>
      <Folders project={project} />
    </Dialog>
  );
}

/** A folder in a list of the dialog, with what the specialists may do there. */
function FolderRow({
  folder,
  kind,
  onRemove,
}: {
  folder: string;
  kind: string;
  onRemove?: () => void;
}) {
  return (
    <li className={styles.reference}>
      <span className={styles.path} data-selectable>
        {folder}
      </span>
      <span className={classNames('label', styles.kind)}>{kind}</span>
      {onRemove ? (
        <IconButton label={`Remove ${folder}`} onClick={onRemove}>
          <Trash2 />
        </IconButton>
      ) : (
        // The work folder is not removed; the gap keeps its mark in line
        // with the marks of the rows that can be.
        <span className={styles.noRemoval} aria-hidden />
      )}
    </li>
  );
}

/**
 * The content of the dialog. It is a component of its own so that what it
 * says about the last thing that was done starts empty every time the
 * dialog opens.
 */
function Folders({ project }: { project: Project }) {
  const places = useProjectPlaces(project.id);
  const commands = usePlacesCommands(project.id);
  const coordinator = useCoordinator(project.id).data;
  const isDemo = useIsDemo();
  const [refusal, setRefusal] = useState<string | null>(null);
  const [copied, setCopied] = useState<string | null>(null);
  const [isConfirming, setIsConfirming] = useState(false);

  /** Does something the backend may refuse, and says why when it does. */
  async function attempt(work: () => Promise<void>) {
    setRefusal(null);
    setCopied(null);
    try {
      await work();
    } catch (failure) {
      setRefusal(describeFailure(failure));
    }
  }

  if (places.isError) {
    return (
      <p className={styles.note} role="alert">
        The folders could not be loaded. {describeFailure(places.error)}
      </p>
    );
  }
  if (!places.data) {
    return <p className={styles.note}>Loading…</p>;
  }
  const { workFolder, isWorkFolderChosen, referenceFolders, writtenFolders, fullAccess } =
    places.data;
  const coordinatorNote = coordinatorWithFullAccess(fullAccess, coordinator?.runs === 'here');

  return (
    <div className={styles.folders}>
      <section className={styles.section}>
        <h3 className="label">Work folder</h3>
        <p className={styles.note}>
          The specialists of this project read, create and change files here.
          {isWorkFolderChosen ? '' : ' Teamwright made this folder for the project.'}
        </p>
        <ul className={styles.references}>
          <FolderRow folder={workFolder} kind="work" />
        </ul>
        <div className={styles.actions}>
          <Button
            variant="secondary"
            icon={<FolderOpen />}
            onClick={() => {
              void attempt(async () => {
                const folder = await commands.chooseFolder();
                if (folder !== null) {
                  await commands.setWorkFolder(folder);
                }
              });
            }}
          >
            Choose another folder
          </Button>
          {isWorkFolderChosen && (
            <Button
              variant="ghost"
              onClick={() => {
                void attempt(() => commands.setWorkFolder(null));
              }}
            >
              Use the folder of Teamwright
            </Button>
          )}
          {!isDemo && (
            <Button
              variant="ghost"
              onClick={() => {
                void attempt(() => commands.showWorkFolder());
              }}
            >
              Show the folder
            </Button>
          )}
        </div>
      </section>

      <section className={styles.section}>
        <h3 className="label">Folders to write in</h3>
        <p className={styles.note}>
          Further folders the specialists may read and change, such as a second repository. Never
          one that holds Teamwright’s data, the sync folder, the settings of Claude Code or your
          keys.
        </p>
        {writtenFolders.length > 0 && (
          <ul className={styles.references}>
            {writtenFolders.map((folder) => (
              <FolderRow
                key={folder}
                folder={folder}
                kind="written"
                onRemove={() => {
                  void attempt(() => commands.removeWrittenFolder(folder));
                }}
              />
            ))}
          </ul>
        )}
        <div className={styles.actions}>
          <Button
            variant="secondary"
            icon={<FolderPlus />}
            onClick={() => {
              void attempt(async () => {
                const folder = await commands.chooseFolder();
                if (folder !== null) {
                  await commands.addWrittenFolder(folder);
                }
              });
            }}
          >
            Add a folder to write in
          </Button>
        </div>
      </section>

      <section className={styles.section}>
        <h3 className="label">Reference folders</h3>
        <p className={styles.note}>
          Folders with material the specialists should know: data sheets, an earlier design, a
          standard. They may read these folders and cannot change anything in them.
        </p>
        {referenceFolders.length > 0 && (
          <ul className={styles.references}>
            {referenceFolders.map((folder) => (
              <FolderRow
                key={folder}
                folder={folder}
                kind="read only"
                onRemove={() => {
                  void attempt(() => commands.removeReferenceFolder(folder));
                }}
              />
            ))}
          </ul>
        )}
        <div className={styles.actions}>
          <Button
            variant="secondary"
            icon={<FolderPlus />}
            onClick={() => {
              void attempt(async () => {
                const folder = await commands.chooseFolder();
                if (folder !== null) {
                  await commands.addReferenceFolder(folder);
                }
              });
            }}
          >
            Add a folder to read
          </Button>
        </div>
      </section>

      <section className={styles.section}>
        <h3 className="label">Reference files</h3>
        <p className={styles.note}>
          A single file, such as a PDF, is copied into the folder <code>references</code> inside the
          work folder. The specialists see the copy, and nothing else of the folder the file came
          from.
        </p>
        <div className={styles.actions}>
          <Button
            variant="secondary"
            icon={<FilePlus />}
            onClick={() => {
              void attempt(async () => {
                const files = await commands.chooseFiles();
                if (files.length > 0) {
                  setCopied(copiedInWords(await commands.addReferenceFiles(files)));
                }
              });
            }}
          >
            Add files
          </Button>
          {copied !== null && (
            <span className={styles.done} role="status">
              {copied}
            </span>
          )}
        </div>
      </section>

      <section className={styles.section} aria-label="Full access">
        <Switch
          on={fullAccess}
          label="Full access"
          hint="Off until you switch it on. It holds from the next turn."
          onChange={(on) => {
            if (on) {
              setIsConfirming(true);
            } else {
              void attempt(() => commands.setFullAccess(false));
            }
          }}
        />
        <div className={styles.words}>
          {FULL_ACCESS_WORDS.map((sentence) => (
            <p key={sentence}>{sentence}</p>
          ))}
          {coordinatorNote !== null && <p className={styles.coordinator}>{coordinatorNote}</p>}
        </div>
      </section>

      {refusal !== null && <Refusal>{refusal}</Refusal>}

      <Dialog
        open={isConfirming}
        title={`Give the specialists of ${project.name} full access?`}
        onClose={() => {
          setIsConfirming(false);
        }}
      >
        <div className={styles.words}>
          {FULL_ACCESS_WORDS.map((sentence) => (
            <p key={sentence}>{sentence}</p>
          ))}
          {coordinator?.runs === 'here' && (
            <p className={styles.coordinator}>
              The coordinator is on: the work it starts by itself will have full access too.
            </p>
          )}
        </div>
        <FormActions>
          <Button
            variant="ghost"
            data-initial-focus
            onClick={() => {
              setIsConfirming(false);
            }}
          >
            Cancel
          </Button>
          <Button
            variant="danger"
            onClick={() => {
              setIsConfirming(false);
              void attempt(() => commands.setFullAccess(true));
            }}
          >
            Give full access
          </Button>
        </FormActions>
      </Dialog>
    </div>
  );
}
