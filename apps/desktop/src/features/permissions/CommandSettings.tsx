import { Switch } from '@/shared/ui/Switch';

import styles from './CommandSettings.module.css';
import { COMMAND_SWITCH_WORDS } from './permissionModel';
import { useCommandPermissions, useSwitchCommandPermissions } from './commandQueries';

/**
 * Whether the specialists may ask to run commands on this computer
 * (decision 0038, note of 22 November 2026): off until the manager
 * switches it on, with plain words about what it allows and what has not
 * been proven. Web domains do not depend on it.
 */
export function CommandSettings() {
  const state = useCommandPermissions().data;
  const switchCommands = useSwitchCommandPermissions();

  if (!state) {
    return null;
  }

  return (
    <section className={styles.part} aria-label="Commands">
      <h2 className="label">Commands</h2>
      <Switch
        on={state.switchedOn}
        label="Let specialists ask to run commands on this computer"
        hint="Off until you switch it on. Your other computers keep their own choice."
        onChange={(on) => {
          void switchCommands(on);
        }}
      />
      <div className={styles.words}>
        {COMMAND_SWITCH_WORDS.map((sentence) => (
          <p key={sentence}>{sentence}</p>
        ))}
      </div>
      {state.heldBack !== null && (
        <p className={styles.heldBack} role="note">
          Even switched on, no command can run on this computer: {state.heldBack}.
        </p>
      )}
    </section>
  );
}
