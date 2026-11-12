import {
  ArrowLeft,
  ArrowRight,
  Check,
  ChevronRight,
  ListFilter,
  Pencil,
  Trash2,
  UsersRound,
} from 'lucide-react';
import { DropdownMenu } from 'radix-ui';
import { useRef, type ReactElement } from 'react';

import type { View, ViewAssignee, ViewFilter } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import menuStyles from '@/shared/ui/Menu.module.css';

import { AssigneeAvatar } from '../team/Assignee';
import { useTeamMembers } from '../team/TeamContext';
import { isFilterable } from './viewModel';
import styles from './ViewTabs.module.css';

interface ViewMenuProps {
  view: View;
  /** The place of the view among the views of its project, from 0. */
  index: number;
  /** How many views the project has. */
  count: number;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onRename: () => void;
  onFilter: (filter: ViewFilter) => void;
  onMove: (index: number) => void;
  onRemove: () => void;
  /** The control that opens the menu. */
  children: ReactElement;
}

/**
 * The menu of the tab of a view: rename it, set its filter, move it left or
 * right, remove it. The last view of a project cannot be removed.
 */
export function ViewMenu({
  view,
  index,
  count,
  open,
  onOpenChange,
  onRename,
  onFilter,
  onMove,
  onRemove,
  children,
}: ViewMenuProps) {
  // Renaming opens a field on the tab. The focus must stay in it, and not
  // go back to the mark that opened the menu.
  const isRenaming = useRef(false);

  return (
    <DropdownMenu.Root open={open} onOpenChange={onOpenChange}>
      <DropdownMenu.Trigger asChild>{children}</DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          className={menuStyles.menu}
          align="start"
          sideOffset={4}
          aria-label={`The view ${view.name}`}
          onCloseAutoFocus={(event) => {
            if (isRenaming.current) {
              isRenaming.current = false;
              event.preventDefault();
            }
          }}
        >
          <Item
            icon={<Pencil />}
            label="Rename"
            onSelect={() => {
              isRenaming.current = true;
              onRename();
            }}
          />
          {isFilterable(view.kind) && (
            <DropdownMenu.Sub>
              <DropdownMenu.SubTrigger className={menuStyles.item}>
                <span className={menuStyles.icon}>
                  <ListFilter />
                </span>
                <span className={menuStyles.label}>Filter</span>
                <span className={menuStyles.more}>
                  <ChevronRight />
                </span>
              </DropdownMenu.SubTrigger>
              <DropdownMenu.Portal>
                <DropdownMenu.SubContent
                  className={menuStyles.menu}
                  sideOffset={2}
                  aria-label="Which tasks the view shows"
                >
                  <FilterItems filter={view.filter} onFilter={onFilter} />
                </DropdownMenu.SubContent>
              </DropdownMenu.Portal>
            </DropdownMenu.Sub>
          )}
          <Item
            icon={<ArrowLeft />}
            label="Move left"
            isDisabled={index === 0}
            onSelect={() => {
              onMove(index - 1);
            }}
          />
          <Item
            icon={<ArrowRight />}
            label="Move right"
            isDisabled={index >= count - 1}
            onSelect={() => {
              onMove(index + 1);
            }}
          />
          <DropdownMenu.Separator className={menuStyles.separator} />
          <Item
            icon={<Trash2 />}
            label={count > 1 ? 'Remove view' : 'The last view stays'}
            isDestructive={count > 1}
            isDisabled={count <= 1}
            onSelect={onRemove}
          />
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}

interface ItemProps {
  icon: ReactElement;
  label: string;
  isDisabled?: boolean;
  isDestructive?: boolean;
  onSelect: () => void;
}

function Item({ icon, label, isDisabled, isDestructive, onSelect }: ItemProps) {
  return (
    <DropdownMenu.Item
      className={classNames(menuStyles.item, isDestructive && menuStyles.destructive)}
      disabled={isDisabled}
      onSelect={onSelect}
    >
      <span className={menuStyles.icon}>{icon}</span>
      <span className={menuStyles.label}>{label}</span>
    </DropdownMenu.Item>
  );
}

/** The whose-tasks choice as one value, for the group of choices of the menu. */
function keyOf(assignee: ViewAssignee): string {
  return assignee.kind === 'member' ? `member:${assignee.memberId}` : assignee.kind;
}

interface FilterItemsProps {
  filter: ViewFilter;
  onFilter: (filter: ViewFilter) => void;
}

/** Whose tasks the view shows, and whether it hides the finished ones. */
function FilterItems({ filter, onFilter }: FilterItemsProps) {
  const members = useTeamMembers();
  const choices: { assignee: ViewAssignee; label: string; mark: ReactElement }[] = [
    { assignee: { kind: 'anyone' }, label: 'Everybody', mark: <UsersRound /> },
    {
      assignee: { kind: 'manager' },
      label: 'You',
      mark: <AssigneeAvatar assignee={{ kind: 'manager' }} />,
    },
    ...members.map((member) => ({
      assignee: { kind: 'member', memberId: member.id } as const,
      label: member.name,
      mark: <AssigneeAvatar assignee={{ kind: 'member', memberId: member.id }} />,
    })),
    { assignee: { kind: 'nobody' }, label: 'Nobody', mark: <AssigneeAvatar assignee={null} /> },
  ];

  return (
    <>
      <DropdownMenu.Label className={classNames('label', styles.menuHeading)}>
        Whose tasks
      </DropdownMenu.Label>
      <DropdownMenu.RadioGroup
        value={keyOf(filter.assignee)}
        onValueChange={(chosen) => {
          const choice = choices.find((candidate) => keyOf(candidate.assignee) === chosen);
          if (choice) {
            onFilter({ ...filter, assignee: choice.assignee });
          }
        }}
      >
        {choices.map((choice) => (
          <DropdownMenu.RadioItem
            key={keyOf(choice.assignee)}
            value={keyOf(choice.assignee)}
            className={menuStyles.item}
          >
            <span className={styles.mark}>{choice.mark}</span>
            <span className={menuStyles.label}>{choice.label}</span>
            <DropdownMenu.ItemIndicator className={menuStyles.check}>
              <Check />
            </DropdownMenu.ItemIndicator>
          </DropdownMenu.RadioItem>
        ))}
      </DropdownMenu.RadioGroup>
      <DropdownMenu.Separator className={menuStyles.separator} />
      <DropdownMenu.CheckboxItem
        className={menuStyles.item}
        checked={filter.hidesFinished}
        onCheckedChange={(checked) => {
          onFilter({ ...filter, hidesFinished: checked });
        }}
        // The menu stays open, so that both halves of a filter can be set at once.
        onSelect={(event) => {
          event.preventDefault();
        }}
      >
        {/* A box that is there while it is empty, so that the line reads as
            something to switch on. */}
        <span className={styles.mark}>
          <span className={styles.box} data-checked={filter.hidesFinished}>
            {filter.hidesFinished && <Check aria-hidden />}
          </span>
        </span>
        <span className={menuStyles.label}>Hide finished tasks</span>
      </DropdownMenu.CheckboxItem>
    </>
  );
}
