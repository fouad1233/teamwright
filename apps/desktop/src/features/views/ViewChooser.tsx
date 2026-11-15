import { Plus } from 'lucide-react';
import { createElement } from 'react';

import type { View, ViewKind } from '@/shared/api/types';
import { IconButton } from '@/shared/ui/Button';
import { ActionMenu } from '@/shared/ui/Menu';

import { KINDS } from './viewKinds';
import { kindsToOffer } from './viewModel';
import styles from './ViewTabs.module.css';

interface ViewChooserProps {
  views: readonly View[];
  onChoose: (kind: ViewKind) => void;
}

/** What the chooser calls the two groups of kinds. */
const HEADINGS = { tasks: 'Looks at the tasks', pages: 'Pages' } as const;

/**
 * The "+" at the end of the views. It offers every kind that can be added,
 * with what the kind is for, under two headings: the looks at the tasks, and
 * the pages a view keeps. A backlog or a dashboard that the project has
 * already is not offered.
 */
export function ViewChooser({ views, onChoose }: ViewChooserProps) {
  const offered = kindsToOffer(
    KINDS.map((entry) => entry.kind),
    views,
  );
  const actions = KINDS.filter((entry) => offered.includes(entry.kind)).map((entry) => ({
    label: entry.label,
    detail: entry.purpose,
    icon: createElement(entry.icon),
    group: HEADINGS[entry.group],
    onSelect: () => {
      onChoose(entry.kind);
    },
  }));

  return (
    <ActionMenu label="Add a view" actions={actions} align="start" leavesFocus>
      <IconButton label="Add a view" className={styles.add}>
        <Plus />
      </IconButton>
    </ActionMenu>
  );
}
