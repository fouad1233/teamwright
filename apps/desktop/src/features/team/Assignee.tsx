import { UserRound } from 'lucide-react';

import type { Assignee, Member } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import styles from './Assignee.module.css';
import { useTeamMember } from './TeamContext';
import { initialsOf } from './teamModel';

/** What the manager is called in the interface. The manager is the one reading it. */
export const MANAGER_NAME = 'You';

/** What stands where a task is given to nobody. */
export const NOBODY_NAME = 'Unassigned';

/** What stands for a member that is not known, for example while the team is loading. */
const UNKNOWN_MEMBER_NAME = 'Team member';

/** The sizes of an avatar: small on a card or in a row, large where a member is presented. */
type AvatarSize = 'small' | 'large';

interface MemberAvatarProps {
  member: Pick<Member, 'name' | 'kind' | 'color'>;
  size?: AvatarSize;
}

/**
 * The picture of a member of the team: its initials in its own color.
 *
 * An AI specialist is a block, like the parts the logo is built of. A person
 * is round.
 */
export function MemberAvatar({ member, size = 'small' }: MemberAvatarProps) {
  return (
    <span
      className={classNames(
        styles.avatar,
        styles[size],
        styles.member,
        member.kind === 'agent' && styles.block,
      )}
      data-tone={member.color}
      role="img"
      aria-label={member.name}
      title={member.name}
    >
      {initialsOf(member.name)}
    </span>
  );
}

/** The picture of the manager: brass, like the keystone of the logo. */
export function ManagerAvatar({ size = 'small' }: { size?: AvatarSize }) {
  return (
    <span className={classNames(styles.avatar, styles[size], styles.manager)} title={MANAGER_NAME}>
      <UserRound aria-label={MANAGER_NAME} />
    </span>
  );
}

/** A small picture of who is responsible for a task. */
export function AssigneeAvatar({ assignee }: { assignee: Assignee | null }) {
  const member = useTeamMember(assignee?.kind === 'member' ? assignee.memberId : undefined);

  if (assignee === null) {
    return <span className={classNames(styles.avatar, styles.nobody)} title={NOBODY_NAME} />;
  }
  if (assignee.kind === 'manager') {
    return <ManagerAvatar />;
  }
  if (member === undefined) {
    return (
      <span className={classNames(styles.avatar, styles.unknown)} title={UNKNOWN_MEMBER_NAME} />
    );
  }
  return <MemberAvatar member={member} />;
}

/** Says who is responsible for a task, in a word or two. */
export function useAssigneeName(assignee: Assignee | null): string {
  const member = useTeamMember(assignee?.kind === 'member' ? assignee.memberId : undefined);

  if (assignee === null) {
    return NOBODY_NAME;
  }
  if (assignee.kind === 'manager') {
    return MANAGER_NAME;
  }
  return member?.name ?? UNKNOWN_MEMBER_NAME;
}

/**
 * The name of who is responsible for a task. A name that is too long for its
 * place is cut with an ellipsis.
 */
export function AssigneeName({ assignee }: { assignee: Assignee | null }) {
  return <span className={styles.name}>{useAssigneeName(assignee)}</span>;
}
