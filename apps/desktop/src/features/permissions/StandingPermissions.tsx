import type { Member, Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { taskReference } from '@/shared/lib/format';
import { Button } from '@/shared/ui/Button';

import { useTeamMembers } from '../team/TeamContext';
import { usePermissionCommands, usePermissionRequests } from './permissionQueries';
import { permissionInWords, scopeInWords, standingPermissionsOf } from './permissionModel';
import styles from './StandingPermissions.module.css';

interface StandingPermissionsProps {
  project: Project;
  tasks: readonly Task[];
}

/**
 * What the manager has allowed the specialists of a project beyond reading
 * and writing in its folder (decision 0038): each standing permission, for
 * a task or always, with the specialist it is for, and a way to take it
 * back. A permission allowed once is gone with its turn and is not listed.
 */
export function StandingPermissions({ project, tasks }: StandingPermissionsProps) {
  const requests = usePermissionRequests(project.id);
  const commands = usePermissionCommands(project.id);
  const members = useTeamMembers();
  const specialists = members.filter((member) => member.kind === 'agent');
  const standing = specialists.flatMap((member) =>
    standingPermissionsOf(requests.data ?? [], member.id).map((request) => ({ member, request })),
  );

  return (
    <section className={styles.section} aria-labelledby="permissions-of-the-team">
      <header className={styles.header}>
        <h2 id="permissions-of-the-team" className="label">
          Permissions
        </h2>
        {standing.length > 0 && (
          <span className={styles.count}>{standing.length.toString()} standing</span>
        )}
      </header>
      {standing.length === 0 ? (
        <p className={styles.empty}>
          The specialists read and write in the folder of the project, and do nothing else on their
          own. When one needs to run a command or fetch from the web, it asks, and the request waits
          in the Inbox. What you allow for a task or always is listed here, and can be taken back.
        </p>
      ) : (
        <ol className={styles.list}>
          {standing.map(({ member, request }) => (
            <li key={request.id} className={styles.item}>
              <Standing
                member={member}
                project={project}
                task={tasks.find((known) => known.id === request.taskId)}
                rule={permissionInWords(
                  request.answer?.kind ?? request.kind,
                  request.answer?.what ?? request.what,
                )}
                scope={request.answer?.scope ?? null}
                isHere={request.answer?.onThisComputer ?? true}
                onWithdraw={() => commands.withdraw(request.id)}
              />
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}

interface StandingProps {
  member: Member;
  project: Project;
  task: Task | undefined;
  rule: string;
  scope: 'once' | 'task' | 'always' | null;
  /** Whether it was allowed on this computer, the only one where it holds. */
  isHere: boolean;
  onWithdraw: () => Promise<void>;
}

function Standing({ member, project, task, rule, scope, isHere, onWithdraw }: StandingProps) {
  return (
    <>
      <div className={styles.words}>
        <p className={styles.who}>
          <span className={styles.member}>{member.name}</span>
          <span className={classNames('label', styles.scope)}>
            {scope === 'task' && task ? `For ${taskReference(project, task)}` : scopeInWords(scope)}
          </span>
        </p>
        <p className={styles.rule} data-selectable>
          {rule}
        </p>
        {!isHere && (
          <p className={styles.elsewhere}>Allowed on another computer, and holds only there.</p>
        )}
      </div>
      <Button
        variant="ghost"
        onClick={() => {
          void onWithdraw();
        }}
      >
        Take back
      </Button>
    </>
  );
}
