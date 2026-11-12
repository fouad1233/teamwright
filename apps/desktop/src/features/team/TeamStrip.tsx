import { MemberAvatar } from './Assignee';
import { useTeamMembers } from './TeamContext';
import styles from './TeamStrip.module.css';

/** The most members whose avatars are shown. The rest are counted in a number. */
const MOST_AVATARS = 6;

/**
 * The team of the project at a glance: the avatars of its members in a row.
 * Pressing it shows the team. A project without a team shows nothing.
 */
export function TeamStrip({ onOpen }: { onOpen: () => void }) {
  const members = useTeamMembers();
  if (members.length === 0) {
    return null;
  }
  const shown = members.slice(0, MOST_AVATARS);
  const notShown = members.length - shown.length;
  const names = members.map((member) => member.name).join(', ');

  return (
    <button type="button" className={styles.strip} aria-label={`Team: ${names}`} onClick={onOpen}>
      {shown.map((member) => (
        <MemberAvatar key={member.id} member={member} />
      ))}
      {notShown > 0 && <span className={styles.more}>+{notShown}</span>}
    </button>
  );
}
