import { useState } from 'react';

import type { Computer, SyncState } from '@/shared/api/types';
import { Button } from '@/shared/ui/Button';

import styles from './OtherComputers.module.css';
import { computersInOrder, nameOfComputer, sinceInWords } from './syncModel';
import { useSyncCommands } from './syncQueries';

interface OtherComputersProps {
  state: SyncState;
  now: Date;
}

/** The other computers that write into the folder, and what this one does with them. */
export function OtherComputers({ state, now }: OtherComputersProps) {
  const computers = computersInOrder(state);

  return (
    <section className={styles.part} aria-label="Other computers">
      <h2 className="label">Other computers</h2>
      {computers.length === 0 ? (
        <p className={styles.empty}>
          No other computer has written into the folder yet. Choose the same folder in Teamwright on
          your other computer, and it shows up here.
        </p>
      ) : (
        <ul className={styles.computers}>
          {computers.map((computer) => (
            <li key={computer.id}>
              <ComputerRow computer={computer} now={now} />
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

/** One other computer: its name, its version, when it was last heard from, and its marks. */
function ComputerRow({ computer, now }: { computer: Computer; now: Date }) {
  const commands = useSyncCommands();
  const [isSending, setIsSending] = useState(false);
  const name = nameOfComputer(computer);
  const isNew = computer.acceptance === 'undecided';

  async function act(request: () => Promise<void>) {
    setIsSending(true);
    await request();
    setIsSending(false);
  }

  return (
    <article className={styles.computer}>
      {isNew && (
        <span className={styles.joints} aria-hidden>
          <span />
        </span>
      )}
      <div className={styles.line}>
        <span className={styles.name}>{name}</span>
        {computer.version !== null && (
          <span className={styles.fact}>Teamwright {computer.version}</span>
        )}
        <span className={styles.fact}>
          {computer.lastHeard === null
            ? 'not heard from yet'
            : `heard ${sinceInWords(computer.lastHeard, now)}`}
        </span>
        <span className={styles.space} />
        {computer.acceptance === 'accepted' && <span className="label">Taken in</span>}
        {computer.acceptance === 'ignored' && <span className="label">Ignored</span>}
      </div>

      {(computer.clockAhead || computer.newerFormat) && (
        <ul className={styles.marks}>
          {computer.newerFormat && (
            <li>
              It uses a newer Teamwright, whose files this one cannot read. Update Teamwright on
              this computer to take in what it writes.
            </li>
          )}
          {computer.clockAhead && (
            <li>
              Its clock is more than five minutes ahead of this computer&apos;s. What it writes is
              still taken in.
            </li>
          )}
        </ul>
      )}

      {isNew && (
        <div className={styles.decide}>
          <p className={styles.question}>
            <span className={styles.waitsForYou}>Waits for you</span>
            {name} writes into the folder. Take in its projects and its work?
          </p>
          <div className={styles.actions}>
            <Button
              variant="primary"
              disabled={isSending}
              onClick={() => void act(() => commands.accept(computer.id))}
            >
              Take in
            </Button>
            <Button
              variant="ghost"
              disabled={isSending}
              onClick={() => void act(() => commands.ignore(computer.id))}
            >
              Ignore
            </Button>
          </div>
        </div>
      )}
      {computer.acceptance === 'ignored' && (
        <div className={styles.actions}>
          <Button
            variant="ghost"
            disabled={isSending}
            onClick={() => void act(() => commands.accept(computer.id))}
          >
            Take in after all
          </Button>
        </div>
      )}
    </article>
  );
}
