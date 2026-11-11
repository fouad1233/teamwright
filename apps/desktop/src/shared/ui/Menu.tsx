import { Check } from 'lucide-react';
import { DropdownMenu } from 'radix-ui';
import type { ReactElement, ReactNode } from 'react';

import { classNames } from '@/shared/lib/classNames';

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
  icon?: ReactNode;
  /** Shows the action in the color of danger, for something that destroys. */
  isDestructive?: boolean;
  onSelect: () => void;
}

interface ActionMenuProps {
  /** What the menu is about, for screen readers. */
  label: string;
  actions: readonly Action[];
  /** The control that opens the menu. */
  children: ReactElement;
}

/** A menu of things to do. */
export function ActionMenu({ label, actions, children }: ActionMenuProps) {
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>{children}</DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content className={styles.menu} align="end" sideOffset={4} aria-label={label}>
          {actions.map((action) => (
            <DropdownMenu.Item
              key={action.label}
              className={classNames(styles.item, action.isDestructive && styles.destructive)}
              onSelect={action.onSelect}
            >
              {action.icon !== undefined && <span className={styles.icon}>{action.icon}</span>}
              <span className={styles.label}>{action.label}</span>
            </DropdownMenu.Item>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
