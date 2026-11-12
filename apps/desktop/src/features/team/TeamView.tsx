import { Ellipsis, Pencil, Plus, UserMinus } from 'lucide-react';
import { useState } from 'react';

import type { Member, Project, Task } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button, IconButton } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { ActionMenu } from '@/shared/ui/Menu';

import { TaskTally } from '../projects/TaskTally';
import { MANAGER_NAME, ManagerAvatar, MemberAvatar } from './Assignee';
import { MemberDialog } from './MemberDialog';
import { useTeamMembers } from './TeamContext';
import styles from './TeamView.module.css';
import { kindLabel, tasksOfManager, tasksOfMember } from './teamModel';
import { useTeamCommands } from './teamQueries';

interface TeamViewProps {
  project: Project;
  /** Every task of the project, to show what each member has on its plate. */
  tasks: Task[];
}

/**
 * A dialog about a member. The member stays known while the dialog closes,
 * so that its words do not change in the moment it disappears.
 */
interface MemberDialogState {
  isOpen: boolean;
  member?: Member;
}

const CLOSED: MemberDialogState = { isOpen: false };

/**
 * The team of a project: the manager, then every member in the order it
 * joined, each with what it is there for and the tasks it has.
 */
export function TeamView({ project, tasks }: TeamViewProps) {
  const members = useTeamMembers();
  const commands = useTeamCommands(project.id);
  const [form, setForm] = useState(CLOSED);
  const [removal, setRemoval] = useState(CLOSED);
  const hasSpecialists = members.some((member) => member.kind === 'agent');
  const leaving = removal.member;
  const tasksOfLeaving = leaving ? tasksOfMember(tasks, leaving.id).length : 0;

  async function removeLeavingMember() {
    if (leaving) {
      await commands.remove(leaving.id);
    }
    setRemoval((current) => ({ ...current, isOpen: false }));
  }

  return (
    <div className={styles.view}>
      <table className={styles.table}>
        <thead>
          <tr>
            <th className="label">Member</th>
            <th className={classNames('label', styles.kindColumn)}>Kind</th>
            <th className={classNames('label', styles.tasksColumn)}>Tasks</th>
            <th className={styles.actionsColumn}>
              <span className={styles.hidden}>Actions</span>
            </th>
          </tr>
        </thead>
        <tbody>
          <tr className={styles.row}>
            <td>
              <div className={styles.who}>
                <ManagerAvatar size="large" />
                <div className={styles.words}>
                  <span className={styles.name}>{MANAGER_NAME}</span>
                  <span className={styles.responsibility}>
                    You own the project and make the decisions.
                  </span>
                </div>
              </div>
            </td>
            <td className={styles.kind}>Manager</td>
            <td>
              <TaskTally project={project} tasks={tasksOfManager(tasks)} />
            </td>
            <td />
          </tr>

          {members.map((member) => (
            <tr key={member.id} className={styles.row}>
              <td>
                <div className={styles.who}>
                  <MemberAvatar member={member} size="large" />
                  <div className={styles.words}>
                    <span className={styles.name}>{member.name}</span>
                    {member.responsibility !== '' && (
                      <span className={styles.responsibility}>{member.responsibility}</span>
                    )}
                  </div>
                </div>
              </td>
              <td className={styles.kind}>{kindLabel(member.kind)}</td>
              <td>
                <TaskTally project={project} tasks={tasksOfMember(tasks, member.id)} />
              </td>
              <td>
                <ActionMenu
                  label={`Actions for ${member.name}`}
                  actions={[
                    {
                      label: 'Change',
                      icon: <Pencil />,
                      onSelect: () => {
                        setForm({ isOpen: true, member });
                      },
                    },
                    {
                      label: 'Remove from the team',
                      icon: <UserMinus />,
                      isDestructive: true,
                      onSelect: () => {
                        setRemoval({ isOpen: true, member });
                      },
                    },
                  ]}
                >
                  <IconButton label={`Actions for ${member.name}`}>
                    <Ellipsis />
                  </IconButton>
                </ActionMenu>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      {members.length === 0 && (
        <p className={styles.empty}>
          Nobody is on the team yet. A team is the help you bring in: AI specialists and people that
          tasks can be given to.
        </p>
      )}

      <button
        type="button"
        className={styles.add}
        onClick={() => {
          setForm({ isOpen: true });
        }}
      >
        <Plus />
        Add member
      </button>

      {hasSpecialists && (
        <p className={styles.note}>
          AI specialists cannot carry out their tasks yet. That is the next milestone. Until then a
          specialist is a member that tasks can be given to.
        </p>
      )}

      <MemberDialog
        open={form.isOpen}
        project={project}
        member={form.member}
        onClose={() => {
          setForm((current) => ({ ...current, isOpen: false }));
        }}
      />

      <Dialog
        open={removal.isOpen}
        title={`Remove ${leaving?.name ?? 'the member'} from the team?`}
        onClose={() => {
          setRemoval((current) => ({ ...current, isOpen: false }));
        }}
      >
        <p className={styles.warning}>
          {tasksOfLeaving === 0
            ? 'No task is given to this member.'
            : `The ${tasksOfLeaving === 1 ? 'task' : `${tasksOfLeaving.toString()} tasks`} given to this member will be given to nobody.`}{' '}
          This cannot be undone.
        </p>
        <div className={styles.dialogActions}>
          <Button
            variant="ghost"
            data-initial-focus
            onClick={() => {
              setRemoval((current) => ({ ...current, isOpen: false }));
            }}
          >
            Cancel
          </Button>
          <Button
            variant="danger"
            onClick={() => {
              void removeLeavingMember();
            }}
          >
            Remove from the team
          </Button>
        </div>
      </Dialog>
    </div>
  );
}
