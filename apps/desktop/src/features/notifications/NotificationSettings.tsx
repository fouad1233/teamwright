import type { NotificationKind } from '@/shared/api/types';
import { Button } from '@/shared/ui/Button';
import { Switch } from '@/shared/ui/Switch';

import styles from './NotificationSettings.module.css';
import { useNotificationChoice, useNotificationCommands } from './notificationQueries';

/** The kinds, in the order they are shown, with what each tells. */
const KINDS: readonly { kind: NotificationKind; label: string; hint: string }[] = [
  {
    kind: 'waiting',
    label: 'Someone waits for you',
    hint: 'A specialist asks you to decide, or the Advisor has questions or a plan.',
  },
  {
    kind: 'finished',
    label: 'Work is ready',
    hint: 'A specialist finished its work on a task, and the task waits to be checked.',
  },
  {
    kind: 'stopped',
    label: 'Someone stopped',
    hint: 'A specialist or the Advisor could not go on.',
  },
];

/**
 * Whether this computer shows notifications of the system, and which. It
 * is a choice of this computer only: the others keep their own.
 */
export function NotificationSettings() {
  const choice = useNotificationChoice();
  const commands = useNotificationCommands();

  if (!choice.data) {
    return null;
  }
  const allowed = choice.data.permission === 'allowed';

  return (
    <section className={styles.part} aria-label="Notifications">
      <h2 className="label">Notifications</h2>
      <Switch
        on={allowed}
        label="Show notifications on this computer"
        hint="Only while the Teamwright window is not in front. Your other computers keep their own choice."
        onChange={(on) => {
          void (on ? commands.allow() : commands.decline());
        }}
      />
      <div className={styles.kinds}>
        {KINDS.map(({ kind, label, hint }) => (
          <Switch
            key={kind}
            on={choice.data[kind]}
            label={label}
            hint={hint}
            disabled={!allowed}
            onChange={(on) => {
              void commands.setKind(kind, on);
            }}
          />
        ))}
      </div>
      <div>
        <Button
          disabled={!allowed}
          onClick={() => {
            void commands.sendTest();
          }}
        >
          Send a test notification
        </Button>
      </div>
    </section>
  );
}
