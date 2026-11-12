import { X } from 'lucide-react';

import { useNavigation } from '@/app/navigation';
import type { Project, SyncNotice } from '@/shared/api/types';
import { Button, IconButton } from '@/shared/ui/Button';

import styles from './SyncNotices.module.css';
import { noticeTarget, sinceInWords } from './syncModel';
import { useSyncCommands, useSyncNotices } from './syncQueries';

interface SyncNoticesProps {
  /** The projects that are not put away: a notice leads only to one of them. */
  projects: readonly Project[];
  now: Date;
}

/**
 * What syncing has to tell the manager, the newest first: a task or a
 * project that got a new number or key, a change that was dropped, a task
 * that moved, a clock that is ahead. Nothing is shown while there is
 * nothing to tell.
 */
export function SyncNotices({ projects, now }: SyncNoticesProps) {
  const notices = useSyncNotices().data ?? [];

  if (notices.length === 0) {
    return null;
  }

  return (
    <section className={styles.part} aria-label="Notices">
      <h2 className="label">Notices</h2>
      <ol className={styles.notices}>
        {notices.map((notice) => (
          <li key={notice.id}>
            <NoticeLine notice={notice} projects={projects} now={now} />
          </li>
        ))}
      </ol>
    </section>
  );
}

interface NoticeLineProps {
  notice: SyncNotice;
  projects: readonly Project[];
  now: Date;
}

/** One notice: when, what happened in the words of the application, and where it leads. */
function NoticeLine({ notice, projects, now }: NoticeLineProps) {
  const navigation = useNavigation();
  const commands = useSyncCommands();
  const target = noticeTarget(
    notice,
    projects.map((project) => project.id),
  );

  return (
    <article className={styles.notice}>
      <span className={styles.when}>{sinceInWords(notice.at, now)}</span>
      <p className={styles.message} data-selectable>
        {notice.message}
      </p>
      <div className={styles.actions}>
        {target.kind === 'task' && (
          <Button
            variant="ghost"
            onClick={() => {
              navigation.openTaskOf(target.projectId, target.taskId);
            }}
          >
            Open the task
          </Button>
        )}
        {target.kind === 'project' && (
          <Button
            variant="ghost"
            onClick={() => {
              navigation.openProject(target.projectId);
            }}
          >
            Open the project
          </Button>
        )}
        <IconButton
          label="Dismiss"
          onClick={() => {
            void commands.dismissNotice(notice.id);
          }}
        >
          <X />
        </IconButton>
      </div>
    </article>
  );
}
