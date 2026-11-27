import { RefreshCw } from 'lucide-react';
import { useState } from 'react';

import { Button } from '@/shared/ui/Button';
import { Switch } from '@/shared/ui/Switch';

import { useNow } from '../sync/useNow';
import { NewerVersionCard } from './NewerVersionCard';
import styles from './VersionSettings.module.css';
import { lastLookInWords } from './updateModel';
import { useUpdateCommands, useUpdateInstall, useUpdateStatus } from './updateQueries';

/**
 * The version of this program, and whether a newer one is out. Looking by
 * itself is off until the manager switches it on; "Look now" looks once
 * either way. A newer version is installed when the manager presses its
 * button, and the program restarts when the manager says so (decision 0045).
 */
export function VersionSettings() {
  const status = useUpdateStatus().data;
  const install = useUpdateInstall().data;
  const commands = useUpdateCommands();
  const now = useNow();
  const [isLooking, setIsLooking] = useState(false);

  if (!status) {
    return null;
  }

  async function lookNow() {
    setIsLooking(true);
    await commands.lookNow();
    setIsLooking(false);
  }

  return (
    <section className={styles.part} aria-label="Version">
      <h2 className="label">Version</h2>
      <p className={styles.version}>
        Teamwright <span className={styles.number}>{status.thisVersion}</span>
      </p>
      {install !== undefined && (status.newer !== null || install.progress.phase !== 'idle') && (
        <NewerVersionCard status={status} install={install} />
      )}
      <Switch
        on={status.isOn}
        label="Look for a newer version once a day"
        hint="Teamwright then asks GitHub for the list of its releases, at most once a day. The request says only the address of the list and the version of this program. Off until you switch it on; your other computers keep their own choice."
        onChange={(on) => {
          void commands.switchLook(on);
        }}
      />
      <div className={styles.look}>
        <Button
          icon={<RefreshCw />}
          disabled={isLooking}
          onClick={() => {
            void lookNow();
          }}
        >
          Look now
        </Button>
        <span className={styles.words}>{lastLookInWords(status, now)}</span>
      </div>
    </section>
  );
}
