import { Download, ExternalLink, RotateCw } from 'lucide-react';
import { useState, type CSSProperties } from 'react';

import type { UpdateInstall, UpdateStatus } from '@/shared/api/types';
import { Button } from '@/shared/ui/Button';

import { useSyncCommands } from '../sync/syncQueries';
import styles from './VersionSettings.module.css';
import {
  byHandInWords,
  downloadShare,
  laterInWords,
  newerInWords,
  progressInWords,
  restartWarningInWords,
  sourceInWords,
} from './updateModel';
import { useInstallCommands } from './updateQueries';

interface NewerVersionCardProps {
  status: UpdateStatus;
  install: UpdateInstall;
}

/**
 * A newer version, and installing it (decision 0045). One button installs
 * it: the card says where it comes from, shows the download as it arrives,
 * and asks before the program restarts. A copy that cannot install its own
 * updates keeps the release page, and says why.
 */
export function NewerVersionCard({ status, install }: NewerVersionCardProps) {
  const commands = useInstallCommands();
  const web = useSyncCommands();
  const [isLater, setIsLater] = useState(false);
  const { progress } = install;
  const version = status.newer?.version ?? (progress.phase === 'ready' ? progress.version : '');

  const releasePage =
    status.newer === null ? null : (
      <Button
        variant="ghost"
        icon={<ExternalLink />}
        onClick={() => {
          if (status.newer !== null) void web.openWebAddress(status.newer.address);
        }}
      >
        Open the release page
      </Button>
    );

  const share = downloadShare(progress);
  const warning = restartWarningInWords(install);
  const words = progressInWords(install);

  return (
    <div className={styles.newer} role="status">
      <p className={styles.newerText}>{newerInWords(status, install)}</p>
      {install.updatesItself && progress.phase === 'idle' && (
        <>
          <p className={styles.source}>{sourceInWords(install)}</p>
          <div className={styles.actions}>
            <Button variant="primary" icon={<Download />} onClick={() => void commands.install()}>
              Install {version}
            </Button>
            {releasePage}
          </div>
        </>
      )}
      {(progress.phase === 'looking' || progress.phase === 'downloading') && (
        <div className={styles.progress}>
          <span className={styles.words}>{words}</span>
          <span
            className={styles.track}
            role="progressbar"
            aria-label="Download of the update"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={share === null ? undefined : Math.floor(share * 100)}
            aria-valuetext={words ?? undefined}
            style={{ '--share': share ?? 0 } as CSSProperties}
          >
            <span className={styles.fill} />
          </span>
        </div>
      )}
      {progress.phase === 'ready' && (
        <>
          <p className={styles.newerText}>{words}</p>
          {warning !== null && <p className={styles.warning}>{warning}</p>}
          {isLater && <p className={styles.source}>{laterInWords(install)}</p>}
          <div className={styles.actions}>
            <Button
              variant={isLater ? 'secondary' : 'primary'}
              icon={<RotateCw />}
              onClick={() => void commands.restart(warning !== null)}
            >
              {warning === null ? 'Restart now' : 'Stop them and restart'}
            </Button>
            {!isLater && (
              <Button variant="ghost" onClick={() => setIsLater(true)}>
                Later
              </Button>
            )}
          </div>
        </>
      )}
      {progress.phase === 'failed' && (
        <>
          <p className={styles.problem}>{words}</p>
          <div className={styles.actions}>
            {install.updatesItself && (
              <Button icon={<Download />} onClick={() => void commands.install()}>
                Try again
              </Button>
            )}
            {releasePage}
          </div>
        </>
      )}
      {!install.updatesItself && progress.phase === 'idle' && (
        <>
          <p className={styles.source}>{byHandInWords(install)}</p>
          <div className={styles.actions}>{releasePage}</div>
        </>
      )}
    </div>
  );
}
