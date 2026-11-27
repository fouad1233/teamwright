import { FolderOpen } from 'lucide-react';
import { useState } from 'react';

import type { Project } from '@/shared/api/types';
import { Button } from '@/shared/ui/Button';
import { Refusal } from '@/shared/ui/Form';
import { describeFailure } from '@/shared/ui/Toast';

import { usePlacesCommands, useProjectPlaces } from '../projects/projectQueries';
import styles from './WorkFolderRow.module.css';

/**
 * The work folder of a project, where it is planned: the folder the team
 * works in and the Advisor reads before it plans, with a way to choose
 * another one right here. It uses the same folder window and the same
 * command as the dialog of the folders, which holds the rest of them.
 */
export function WorkFolderRow({ project }: { project: Project }) {
  const places = useProjectPlaces(project.id);
  const commands = usePlacesCommands(project.id);
  const [refusal, setRefusal] = useState<string | null>(null);
  const [isChoosing, setIsChoosing] = useState(false);

  async function choose() {
    setRefusal(null);
    setIsChoosing(true);
    try {
      const folder = await commands.chooseFolder();
      if (folder !== null) {
        await commands.setWorkFolder(folder);
      }
    } catch (failure) {
      setRefusal(describeFailure(failure));
    } finally {
      setIsChoosing(false);
    }
  }

  if (places.isError) {
    return (
      <p className={styles.note} role="alert">
        The work folder could not be loaded. {describeFailure(places.error)}
      </p>
    );
  }

  const isChosen = places.data?.isWorkFolderChosen ?? false;
  return (
    <section className={styles.row} aria-label="Work folder">
      <div className={styles.what}>
        <span className="label">Work folder</span>
        <span className={styles.path} data-selectable title={places.data?.workFolder}>
          {places.data?.workFolder ?? 'Loading…'}
        </span>
        <p className={styles.note}>
          The team works in this folder. The Advisor reads what is in it before it plans.
          {places.data &&
            !isChosen &&
            ' No folder was chosen, so the work goes into this one, which Teamwright made for the project.'}
        </p>
      </div>
      <Button
        variant="secondary"
        icon={<FolderOpen />}
        disabled={!places.data || isChoosing}
        onClick={() => {
          void choose();
        }}
      >
        {isChosen ? 'Choose another folder' : 'Choose a folder'}
      </Button>
      {refusal !== null && (
        <div className={styles.refusal}>
          <Refusal>{refusal}</Refusal>
        </div>
      )}
    </section>
  );
}
