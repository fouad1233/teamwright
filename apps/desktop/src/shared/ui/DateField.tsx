import { CalendarDays } from 'lucide-react';
import { Popover } from 'radix-ui';
import { useState } from 'react';
import { DayPicker, type Matcher } from 'react-day-picker';

import type { CalendarDate } from '@/shared/api/types';
import { calendarDateOf, formatDate, localDateOf } from '@/shared/lib/format';

import { Button, ValueButton } from './Button';
import styles from './DateField.module.css';

import 'react-day-picker/style.css';

interface DateFieldProps {
  /** What the date is, for screen readers: "Start date". */
  label: string;
  value: CalendarDate | null;
  onChange: (value: CalendarDate | null) => void;
  /** The earliest day that can be chosen. */
  earliest?: CalendarDate | null;
  /** The latest day that can be chosen. */
  latest?: CalendarDate | null;
}

/** A date that is chosen from a calendar, and can be cleared again. */
export function DateField({ label, value, onChange, earliest, latest }: DateFieldProps) {
  const [isOpen, setIsOpen] = useState(false);
  const selected = value === null ? undefined : localDateOf(value);

  const unavailable: Matcher[] = [];
  if (earliest) {
    unavailable.push({ before: localDateOf(earliest) });
  }
  if (latest) {
    unavailable.push({ after: localDateOf(latest) });
  }

  function choose(day: CalendarDate | null) {
    onChange(day);
    setIsOpen(false);
  }

  return (
    <Popover.Root open={isOpen} onOpenChange={setIsOpen}>
      <Popover.Trigger asChild>
        <ValueButton aria-label={label} isEmpty={value === null}>
          <CalendarDays />
          {value === null ? 'None' : formatDate(value)}
        </ValueButton>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content className={styles.popover} align="start" sideOffset={4}>
          <DayPicker
            mode="single"
            selected={selected}
            defaultMonth={selected}
            weekStartsOn={1}
            disabled={unavailable}
            onSelect={(day) => {
              choose(day ? calendarDateOf(day) : null);
            }}
          />
          {value !== null && (
            <div className={styles.footer}>
              <Button
                variant="ghost"
                onClick={() => {
                  choose(null);
                }}
              >
                Clear date
              </Button>
            </div>
          )}
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}
