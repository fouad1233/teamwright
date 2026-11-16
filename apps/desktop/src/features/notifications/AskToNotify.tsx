import { Button } from '@/shared/ui/Button';

import styles from './AskToNotify.module.css';
import { shouldAsk } from './notificationModel';
import { useNotificationChoice, useNotificationCommands } from './notificationQueries';

/**
 * Asks once whether Teamwright may tell the manager, with a notification of
 * the system, when someone needs them. It shows where something first
 * waits for the manager, and never again once answered either way.
 */
export function AskToNotify() {
  const choice = useNotificationChoice();
  const commands = useNotificationCommands();

  if (!shouldAsk(choice.data)) {
    return null;
  }

  return (
    <div className={styles.ask} role="status">
      <p className={styles.text}>
        Teamwright can tell you when a specialist needs you or is ready, while you work in another
        window.
      </p>
      <div className={styles.actions}>
        <Button
          variant="secondary"
          onClick={() => {
            void commands.allow();
          }}
        >
          Allow notifications
        </Button>
        <Button
          variant="ghost"
          onClick={() => {
            void commands.decline();
          }}
        >
          No, thanks
        </Button>
      </div>
    </div>
  );
}
