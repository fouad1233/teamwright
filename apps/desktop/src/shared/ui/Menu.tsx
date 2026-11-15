import { Check } from 'lucide-react';
import { DropdownMenu } from 'radix-ui';
import type { ReactElement, ReactNode } from 'react';

import { classNames } from '@/shared/lib/classNames';

import { EDGE_GAP } from './edgeGap';
import styles from './Menu.module.css';

/** One of the values a choice menu offers. */
export interface Choice<Value extends string> {
  value: Value;
  label: string;
  /** A small mark shown before the label. */
  icon?: ReactNode;
}

interface ChoiceMenuProps<Value extends string> {
  /** What is being chosen, for screen readers. */
  label: string;
  choices: readonly Choice<Value>[];
  /** The value that is chosen now. It carries a check mark. */
  value: Value;
  onChoose: (value: Value) => void;
  /** The control that opens the menu. */
  children: ReactElement;
}

/** A menu from which exactly one value is chosen. */
export function ChoiceMenu<Value extends string>({
  label,
  choices,
  value,
  onChoose,
  children,
}: ChoiceMenuProps<Value>) {
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>{children}</DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          className={styles.menu}
          align="start"
          sideOffset={4}
          collisionPadding={EDGE_GAP}
          aria-label={label}
        >
          <DropdownMenu.RadioGroup
            value={value}
            onValueChange={(chosen) => {
              onChoose(chosen as Value);
            }}
          >
            {choices.map((choice) => (
              <DropdownMenu.RadioItem
                key={choice.value}
                value={choice.value}
                className={styles.item}
              >
                {choice.icon !== undefined && <span className={styles.icon}>{choice.icon}</span>}
                <span className={styles.label}>{choice.label}</span>
                <DropdownMenu.ItemIndicator className={styles.check}>
                  <Check />
                </DropdownMenu.ItemIndicator>
              </DropdownMenu.RadioItem>
            ))}
          </DropdownMenu.RadioGroup>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}

/** One of the things an action menu can do. */
export interface Action {
  label: string;
  /** A line under the label that says what the action is for. */
  detail?: string;
  icon?: ReactNode;
  /** Shows the action, and does not let it be chosen. */
  isDisabled?: boolean;
  /** Shows the action in the color of danger, for something that destroys. */
  isDestructive?: boolean;
  /**
   * The heading the action stands under. Actions that name the same heading
   * are laid in two columns under it, in the order they are given, so that a
   * long menu stays short. Actions without one come first, one to a line.
   */
  group?: string;
  onSelect: () => void;
}

/** The actions of a menu under their headings, in the order the headings first appear. */
function underHeadings(
  actions: readonly Action[],
): { heading: string | undefined; actions: Action[] }[] {
  const groups: { heading: string | undefined; actions: Action[] }[] = [];
  for (const action of actions) {
    const group = groups.find((other) => other.heading === action.group);
    if (group) {
      group.actions.push(action);
    } else {
      groups.push({ heading: action.group, actions: [action] });
    }
  }
  return groups.sort((a, b) => Number(a.heading !== undefined) - Number(b.heading !== undefined));
}

interface ActionMenuProps {
  /** What the menu is about, for screen readers. */
  label: string;
  actions: readonly Action[];
  /** Which edge of the control the menu lines up with. The end, if left out. */
  align?: 'start' | 'end';
  /**
   * Leaves the focus where the chosen action put it, instead of giving it
   * back to the control, for an action that opens a field to type in.
   */
  leavesFocus?: boolean;
  /** The control that opens the menu. */
  children: ReactElement;
}

/** A menu of things to do. */
export function ActionMenu({
  label,
  actions,
  align = 'end',
  leavesFocus = false,
  children,
}: ActionMenuProps) {
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>{children}</DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          className={styles.menu}
          align={align}
          sideOffset={4}
          collisionPadding={EDGE_GAP}
          aria-label={label}
          onCloseAutoFocus={(event) => {
            if (leavesFocus) {
              event.preventDefault();
            }
          }}
        >
          {underHeadings(actions).map((group) => {
            const items = group.actions.map((action) => (
              <DropdownMenu.Item
                key={action.label}
                className={classNames(
                  styles.item,
                  action.isDestructive && styles.destructive,
                  action.detail !== undefined && styles.withDetail,
                )}
                disabled={action.isDisabled}
                onSelect={action.onSelect}
              >
                {action.icon !== undefined && <span className={styles.icon}>{action.icon}</span>}
                {action.detail === undefined ? (
                  <span className={styles.label}>{action.label}</span>
                ) : (
                  <span className={styles.label}>
                    {action.label}
                    <span className={styles.detail}>{action.detail}</span>
                  </span>
                )}
              </DropdownMenu.Item>
            ));
            return group.heading === undefined ? (
              items
            ) : (
              <DropdownMenu.Group key={group.heading} className={styles.group}>
                <DropdownMenu.Label className={styles.heading}>{group.heading}</DropdownMenu.Label>
                <div className={styles.columns}>{items}</div>
              </DropdownMenu.Group>
            );
          })}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
