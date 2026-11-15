import { Check } from 'lucide-react';
import { DropdownMenu } from 'radix-ui';
import type { ReactElement } from 'react';

import styles from './Menu.module.css';

/** One thing a check menu turns on or off. */
export interface CheckItem<Value extends string> {
  value: Value;
  label: string;
  isChecked: boolean;
  /** Shows the item as it is, and does not let it be changed. */
  isDisabled?: boolean;
}

interface CheckMenuProps<Value extends string> {
  /** What the menu is about, for screen readers. */
  label: string;
  items: readonly CheckItem<Value>[];
  onToggle: (value: Value) => void;
  /** Which edge of the control the menu lines up with. The end, if left out. */
  align?: 'start' | 'end';
  /** The control that opens the menu. */
  children: ReactElement;
}

/**
 * A menu of things that are each on or off, such as the columns a table
 * shows. It stays open while items are turned, so that several can be
 * changed at once; Escape or a press outside closes it.
 */
export function CheckMenu<Value extends string>({
  label,
  items,
  onToggle,
  align = 'end',
  children,
}: CheckMenuProps<Value>) {
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>{children}</DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          className={styles.menu}
          align={align}
          sideOffset={4}
          aria-label={label}
        >
          {items.map((item) => (
            <DropdownMenu.CheckboxItem
              key={item.value}
              className={styles.item}
              checked={item.isChecked}
              disabled={item.isDisabled}
              onCheckedChange={() => {
                onToggle(item.value);
              }}
              onSelect={(event) => {
                event.preventDefault();
              }}
            >
              <span className={styles.label}>{item.label}</span>
              <DropdownMenu.ItemIndicator className={styles.check}>
                <Check />
              </DropdownMenu.ItemIndicator>
            </DropdownMenu.CheckboxItem>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  );
}
