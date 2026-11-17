import gitHubBlack from '@/shared/logos/github-mark.svg';
import gitHubWhite from '@/shared/logos/github-mark-white.svg';
import googleDrive from '@/shared/logos/google-drive.png';

import styles from './ServiceMark.module.css';

/** The services whose official mark Teamwright has. Slack has none: it is named in words. */
export type MarkedService = 'github' | 'google-drive';

interface ServiceMarkProps {
  service: MarkedService;
}

/**
 * The official mark of a service Teamwright works with, small, beside its
 * name. The marks are used as their owners hand them out (see
 * `shared/logos/README.md`): never recoloured, on a plain ground. GitHub's
 * comes in black and in white, and the theme says which one shows;
 * Google Drive's has its own colours and shows as it is. The name beside
 * it says what it is, so the mark itself is hidden from screen readers.
 */
export function ServiceMark({ service }: ServiceMarkProps) {
  if (service === 'google-drive') {
    return <img className={styles.mark} src={googleDrive} alt="" aria-hidden draggable={false} />;
  }
  return (
    <span className={styles.pair} aria-hidden>
      <img className={styles.black} src={gitHubBlack} alt="" draggable={false} />
      <img className={styles.white} src={gitHubWhite} alt="" draggable={false} />
    </span>
  );
}
