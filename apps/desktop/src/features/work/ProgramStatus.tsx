import { useIsDemo } from '@/shared/api/BackendProvider';
import { classNames } from '@/shared/lib/classNames';

import styles from './ProgramStatus.module.css';
import { signInInWords, whyProgramIsNotReady } from './workModel';
import { useAgentProgram } from './workQueries';

/**
 * Says what the AI specialists of this computer run on: the user's own
 * Claude Code and the plan it is signed in with, or what is missing before
 * they can work.
 */
export function ProgramStatus() {
  const program = useAgentProgram();
  const isDemo = useIsDemo();

  if (isDemo) {
    return (
      <p className={styles.status}>
        This is the demo: the specialists here only play their work. In the desktop program they run
        on your own Claude Code.
      </p>
    );
  }
  if (!program.data) {
    return null;
  }

  const obstacle = whyProgramIsNotReady(program.data);
  if (obstacle !== null) {
    return (
      <p className={classNames(styles.status, styles.obstacle)} role="status">
        {obstacle}
      </p>
    );
  }

  const version = program.data.version === null ? '' : ` ${program.data.version}`;
  return (
    <p className={styles.status}>
      Specialists run on your own Claude Code{version}, signed in with {signInInWords(program.data)}
      . What they use counts against it.
    </p>
  );
}
