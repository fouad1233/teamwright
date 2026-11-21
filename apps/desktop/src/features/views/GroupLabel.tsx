import type { GroupKey } from '@/shared/api/types';

import { TagChip } from '../tags/TagChip';
import { AssigneeAvatar } from '../team/Assignee';
import { PriorityFlag, StatusLabel } from '../tasks/TaskBadges';
import styles from './GroupLabel.module.css';
import { groupName, type GroupNames } from './groupModel';

interface GroupLabelProps {
  groupKey: GroupKey;
  names: GroupNames;
}

/**
 * What stands at the head of a lane or a section: a status with its mark,
 * a person with their picture, a priority with its flag, a tag as a chip,
 * and an area or a sprint by its name.
 */
export function GroupLabel({ groupKey, names }: GroupLabelProps) {
  const name = groupName(groupKey, names);
  switch (groupKey.kind) {
    case 'status': {
      const status = names.statuses.find((candidate) => candidate.id === groupKey.id);
      return status ? <StatusLabel status={status} /> : <span>{name}</span>;
    }
    case 'manager':
      return (
        <span className={styles.label}>
          <AssigneeAvatar assignee={{ kind: 'manager' }} />
          <span className={styles.text}>{name}</span>
        </span>
      );
    case 'member':
      return (
        <span className={styles.label}>
          <AssigneeAvatar assignee={{ kind: 'member', memberId: groupKey.id }} />
          <span className={styles.text}>{name}</span>
        </span>
      );
    case 'nobody':
      return (
        <span className={styles.label}>
          <AssigneeAvatar assignee={null} />
          <span className={styles.text}>{name}</span>
        </span>
      );
    case 'priority':
      return <PriorityFlag priority={groupKey.priority} withLabel />;
    case 'tag': {
      const tag = names.tags.find((candidate) => candidate.id === groupKey.id);
      return tag ? <TagChip tag={tag} size="regular" /> : <span>{name}</span>;
    }
    case 'noArea':
    case 'noTag':
    case 'backlog':
      return <span className={styles.quiet}>{name}</span>;
    case 'area':
    case 'sprint':
      return <span className={styles.text}>{name}</span>;
  }
}
