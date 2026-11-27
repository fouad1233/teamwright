import { LockOpen } from 'lucide-react';

import type { Id } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import styles from './FullAccessMark.module.css';
import { useProjectPlaces } from './projectQueries';

/**
 * A small quiet mark that says the specialists of a project have full
 * access on this computer (decision 0044), wherever their work is shown, so
 * that it is never forgotten. Nothing while full access is off.
 */
export function FullAccessMark({ projectId, className }: { projectId: Id; className?: string }) {
  const fullAccess = useProjectPlaces(projectId).data?.fullAccess === true;
  if (!fullAccess) {
    return null;
  }
  return (
    <span
      className={classNames('label', styles.mark, className)}
      title="The specialists of this project may read and change files anywhere your account can, except the protected places. Switch it off in the folders of the project."
    >
      <LockOpen aria-hidden />
      Full access
    </span>
  );
}
