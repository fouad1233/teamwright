import { FilePlus, FolderOpen, FolderPlus, Trash2 } from 'lucide-react';
import { useState } from 'react';

import { useIsDemo } from '@/shared/api/BackendProvider';
import type { Project } from '@/shared/api/types';
import { Button, IconButton } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { Refusal } from '@/shared/ui/Form';
import { describeFailure } from '@/shared/ui/Toast';

import styles from './FoldersDialog.module.css';
import { copiedInWords } from './placesModel';
import { usePlacesCommands, useProjectPlaces } from './projectQueries';

interface FoldersDialogProps {
  open: boolean;
  project: Project;
  onClose: () => void;
}

/**
 * Where the specialists of a project work on this computer, and what else
 * they may read: the work folder, the folders with reference material, and
 * a way to hand them single files.
 */
export function FoldersDialog({ open, project, onClose }: FoldersDialogProps) {
  return (
    <Dialog open={open} title={`Folders of ${project.name}`} size="wide" onClose={onClose}>
      <Folders project={project} />
    </Dialog>
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
  const isDemo = useIsDemo();
  const [refusal, setRefusal] = useState<string | null>(null);
  const [copied, setCopied] = useState<string | null>(null);

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
  const { workFolder, isWorkFolderChosen, referenceFolders } = places.data;

  return (
    <div className={styles.folders}>
      <section className={styles.section}>
        <h3 className="label">Work folder</h3>
        <p className={styles.note}>
          The specialists of this project read, create and change files here, and nowhere else.
          {isWorkFolderChosen ? '' : ' Teamwright made this folder for the project.'}
        </p>
        <p className={styles.path} data-selectable>
          {workFolder}
        </p>
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
        <h3 className="label">Reference folders</h3>
        <p className={styles.note}>
          Folders with material the specialists should know: data sheets, an earlier design, a
          standard. They may read these folders and cannot change anything in them.
        </p>
        {referenceFolders.length > 0 && (
          <ul className={styles.references}>
            {referenceFolders.map((folder) => (
              <li key={folder} className={styles.reference}>
                <span className={styles.path} data-selectable>
                  {folder}
                </span>
                <IconButton
                  label={`Remove ${folder}`}
                  onClick={() => {
                    void attempt(() => commands.removeReferenceFolder(folder));
                  }}
                >
                  <Trash2 />
                </IconButton>
              </li>
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
            Add a folder
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

      {refusal !== null && <Refusal>{refusal}</Refusal>}
    </div>
  );
}
