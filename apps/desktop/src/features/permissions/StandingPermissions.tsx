import type { Member, PermissionKind, Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { taskReference } from '@/shared/lib/format';
import { Button } from '@/shared/ui/Button';

import { useTeamMembers } from '../team/TeamContext';
import { useCommandsSwitchedOn } from './commandQueries';
import { usePermissionCommands, usePermissionRequests } from './permissionQueries';
import { kindInWords, scopeInWords, standingPermissionsOf } from './permissionModel';
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
  const commandsSwitchedOn = useCommandsSwitchedOn();
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
          {commandsSwitchedOn
            ? 'The specialists read and write in the folder of the project, and do nothing else on their own. When one needs to run a command or fetch from the web, it asks, and the request waits in the Inbox. What you allow for a task, or a web domain you allow always, is listed here, and can be taken back.'
            : 'The specialists read and write in the folder of the project, and do nothing else on their own. When one needs to fetch from the web, it asks, and the request waits in the Inbox. A web domain you allow for a task or always is listed here, and can be taken back. Commands are switched off on this computer (Computers).'}
        </p>
      ) : (
        <ol className={styles.list}>
          {standing.map(({ member, request }) => (
            <li key={request.id} className={styles.item}>
              <Standing
                member={member}
                project={project}
                task={tasks.find((known) => known.id === request.taskId)}
                kind={request.answer?.kind ?? request.kind}
                what={request.answer?.what ?? request.what}
                scope={request.answer?.scope ?? null}
                isHere={request.answer?.onThisComputer ?? true}
                isNoLongerValid={request.answer?.noLongerValid ?? false}
                isSwitchedOff={
                  !commandsSwitchedOn && (request.answer?.kind ?? request.kind) === 'command'
                }
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
  kind: PermissionKind;
  what: string;
  scope: 'once' | 'task' | 'always' | null;
  /** Whether it was allowed on this computer, the only one where it holds. */
  isHere: boolean;
  /** Whether it grants nothing any more, and can only be taken back. */
  isNoLongerValid: boolean;
  /** Whether it is a command while command permissions are switched off. */
  isSwitchedOff: boolean;
  onWithdraw: () => Promise<void>;
}

function Standing({
  member,
  project,
  task,
  kind,
  what,
  scope,
  isHere,
  isNoLongerValid,
  isSwitchedOff,
  onWithdraw,
}: StandingProps) {
  return (
    <>
      <div className={styles.words}>
        <p className={styles.who}>
          <span className={styles.member}>{member.name}</span>
          <span className={classNames('label', styles.scope)}>
            {scope === 'task' && task ? `For ${taskReference(project, task)}` : scopeInWords(scope)}
          </span>
        </p>
        <p className={styles.rule}>
          <span className={styles.ruleKind}>{kindInWords(kind)}</span>{' '}
          <code className={styles.ruleWhat} data-selectable>
            {what}
          </code>
        </p>
        {isNoLongerValid ? (
          <p className={styles.elsewhere}>
            No longer valid: a command is allowed once or for a task, never always. It grants
            nothing; take it back.
          </p>
        ) : !isHere ? (
          <p className={styles.elsewhere}>
            Allowed on another computer. It grants nothing on this one.
          </p>
        ) : (
          isSwitchedOff && (
            <p className={styles.elsewhere}>
              Command permissions are switched off on this computer. It grants nothing until they
              are switched on again in Computers.
            </p>
          )
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
