import { FolderOpen, RefreshCw } from 'lucide-react';
import { useState, type FormEvent, type KeyboardEvent } from 'react';

import type { Project, SyncState } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button, ValueButton } from '@/shared/ui/Button';
import { Field, TextInput } from '@/shared/ui/Field';
import { describeFailure } from '@/shared/ui/Toast';

import { waitsForManager } from '../work/waitsForManager';
import styles from './ComputersPlace.module.css';
import { OtherComputers } from './OtherComputers';
import { SyncNotices } from './SyncNotices';
import { isIdentityInUse, problemInWords, sinceInWords, waitsForRestart } from './syncModel';
import { useSyncCommands, useSyncState, type SyncCommands } from './syncQueries';
import { useNow } from './useNow';

interface ComputersPlaceProps {
  /** The projects that are not put away, for the notices that lead to one. */
  projects: readonly Project[];
}

/**
 * Where the manager keeps this computer in step with their other computers,
 * through a folder they own: the folder and the name of this computer, the
 * other computers, and what syncing has to tell.
 */
export function ComputersPlace({ projects }: ComputersPlaceProps) {
  const sync = useSyncState();

  return (
    <section className={styles.place} aria-label="Computers">
      <header className={styles.header}>
        <h1 className={styles.title}>Computers</h1>
      </header>

      {sync.isPending && <p className={styles.note}>Loading…</p>}
      {sync.isError && (
        <p className={styles.note} role="alert">
          Syncing could not be asked about. {describeFailure(sync.error)}
        </p>
      )}
      {sync.data &&
        (sync.data.folder === null && sync.data.phase === 'off' ? (
          <FirstSteps state={sync.data} />
        ) : (
          <InStep state={sync.data} projects={projects} />
        ))}
    </section>
  );
}

/**
 * Before a folder is chosen: what syncing is, the folder, and then the name
 * of this computer.
 */
function FirstSteps({ state }: { state: SyncState }) {
  const commands = useSyncCommands();
  const [folder, setFolder] = useState<string | null>(null);

  async function pick() {
    const picked = await commands.pickFolder();
    if (picked !== null) {
      setFolder(picked);
    }
  }

  if (folder !== null) {
    return (
      <NameThisComputer
        folder={folder}
        suggested={state.computerName ?? state.suggestedName ?? ''}
        commands={commands}
        onBack={() => {
          setFolder(null);
        }}
      />
    );
  }

  return (
    <div className={styles.intro}>
      <p className="label">Not kept in step</p>
      <p className={styles.introText}>
        Teamwright can keep your projects the same on all your computers, through a folder you own,
        such as one that a cloud drive keeps in step. Each computer writes what changed into that
        folder and takes in what the others wrote; nothing goes anywhere else.
      </p>
      <div>
        <Button
          variant="primary"
          icon={<FolderOpen />}
          onClick={() => {
            void pick();
          }}
        >
          Choose a folder
        </Button>
      </div>
    </div>
  );
}

interface NameThisComputerProps {
  folder: string;
  suggested: string;
  commands: SyncCommands;
  onBack: () => void;
}

/** The second step: the name this computer is known by in the folder. */
function NameThisComputer({ folder, suggested, commands, onBack }: NameThisComputerProps) {
  const [name, setName] = useState(suggested);
  const [isSending, setIsSending] = useState(false);

  async function start(event: FormEvent) {
    event.preventDefault();
    setIsSending(true);
    const wasTaken = await commands.chooseFolder(folder, name);
    if (!wasTaken) {
      setIsSending(false);
    }
  }

  return (
    <form className={styles.intro} onSubmit={(event) => void start(event)}>
      <p className="label">The folder</p>
      <p className={styles.path} data-selectable>
        {folder}
      </p>
      <Field
        label="Name of this computer"
        hint="It is written into the folder, where your other computers show it. The name the system gives a computer often holds a person's name, so choose one that only says which computer this is."
      >
        <TextInput
          value={name}
          autoFocus
          maxLength={60}
          onChange={(event) => {
            setName(event.target.value);
          }}
        />
      </Field>
      <div className={styles.actions}>
        <Button variant="primary" type="submit" disabled={isSending || name.trim() === ''}>
          Keep in step
        </Button>
        <Button variant="ghost" onClick={onBack}>
          Back
        </Button>
      </div>
    </form>
  );
}

interface InStepProps {
  state: SyncState;
  projects: readonly Project[];
}

/** Once a folder is chosen: this computer, the folder, the others and the notices. */
function InStep({ state, projects }: InStepProps) {
  const commands = useSyncCommands();
  const now = useNow();
  const [isSyncing, setIsSyncing] = useState(false);
  const identityInUse = isIdentityInUse(state);
  const restart = waitsForRestart(state);

  async function syncNow() {
    setIsSyncing(true);
    await commands.syncNow();
    setIsSyncing(false);
  }

  return (
    <div className={styles.parts}>
      {identityInUse && <IdentityInUse commands={commands} />}
      {restart && (
        <div className={styles.restart} role="status">
          <p className={styles.restartTitle}>Start Teamwright again</p>
          <p className={styles.restartText}>
            This computer has a new identity. Syncing goes on once Teamwright has started again. The
            old identity then counts as another computer, so nothing it wrote is lost.
          </p>
        </div>
      )}

      <section className={styles.part} aria-label="This computer">
        <h2 className="label">This computer</h2>
        <ComputerName name={state.computerName ?? ''} commands={commands} />
      </section>

      <section className={styles.part} aria-label="The folder">
        <h2 className="label">The folder</h2>
        {state.folder !== null && (
          <p className={styles.path} data-selectable>
            {state.folder}
          </p>
        )}
        <p className={styles.round}>
          {state.phase === 'writing'
            ? 'Writing to the folder now.'
            : state.lastRound === null
              ? 'No round since Teamwright started.'
              : state.problem === null
                ? `In step. Last round ${sinceInWords(state.lastRound, now)}.`
                : `Last tried ${sinceInWords(state.lastRound, now)}.`}
        </p>
        {state.problem !== null && !identityInUse && !restart && (
          <div className={styles.trouble} role="alert">
            <p className={styles.problem}>{problemInWords(state.problem)}</p>
            {state.phase === 'unreachable' && (
              <p className={styles.troubleHint}>
                Is the drive or the cloud drive there, and is the folder where it was? Nothing is
                lost meanwhile: this computer keeps its changes and tries again every 20 seconds.
              </p>
            )}
          </div>
        )}
        <div className={styles.actions}>
          <Button
            icon={<RefreshCw />}
            disabled={isSyncing || state.phase === 'writing' || identityInUse || restart}
            onClick={() => {
              void syncNow();
            }}
          >
            Sync now
          </Button>
          {state.folder !== null && (
            <Button
              variant="ghost"
              onClick={() => {
                void commands.stop();
              }}
            >
              Stop syncing
            </Button>
          )}
        </div>
      </section>

      <OtherComputers state={state} now={now} />
      <SyncNotices projects={projects} now={now} />
    </div>
  );
}

interface ComputerNameProps {
  name: string;
  commands: SyncCommands;
}

/** The name of this computer, changed in place. */
function ComputerName({ name, commands }: ComputerNameProps) {
  const [draft, setDraft] = useState<string | null>(null);

  async function save() {
    if (draft === null || draft.trim() === name) {
      setDraft(null);
      return;
    }
    if (await commands.rename(draft)) {
      setDraft(null);
    }
  }

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'Enter') {
      event.preventDefault();
      void save();
    } else if (event.key === 'Escape') {
      event.preventDefault();
      setDraft(null);
    }
  }

  if (draft === null) {
    return (
      <div className={styles.name}>
        <ValueButton
          className={styles.nameValue}
          title="Rename this computer"
          onClick={() => {
            setDraft(name);
          }}
        >
          {name}
        </ValueButton>
        <span className={styles.aside}>Your other computers know it by this name.</span>
      </div>
    );
  }

  return (
    <div className={styles.name}>
      <TextInput
        className={styles.nameInput}
        aria-label="Name of this computer"
        value={draft}
        autoFocus
        maxLength={60}
        onChange={(event) => {
          setDraft(event.target.value);
        }}
        onKeyDown={handleKeyDown}
        onBlur={() => void save()}
      />
    </div>
  );
}

/**
 * What happened when this computer's data folder was copied to another
 * computer, and the way out of it.
 */
function IdentityInUse({ commands }: { commands: SyncCommands }) {
  const [isSending, setIsSending] = useState(false);

  return (
    <div className={classNames(waitsForManager.frame, styles.identity)} role="alert">
      <p className={classNames('label', waitsForManager.part, styles.identityLabel)}>
        Syncing stopped
      </p>
      <h2 className={classNames(waitsForManager.part, styles.identityTitle)}>
        Another computer writes into the folder as this one
      </h2>
      <p className={classNames(waitsForManager.part, styles.identityText)}>
        Teamwright&apos;s data folder was copied from one computer to another, and a copy was
        started there. Both now write into the sync folder under the same identity, so syncing
        stopped before anything is mixed up.
      </p>
      <p className={classNames(waitsForManager.part, styles.identityText)}>
        Press the button on the computer the data folder was copied <em>to</em>. Do not press it on
        the computer where your conversations with the specialists were started: those conversations
        stay with the old identity, and can only be continued on the computer that keeps it.
      </p>
      <div className={classNames(waitsForManager.part, styles.actions)}>
        <Button
          variant="primary"
          disabled={isSending}
          onClick={() => {
            setIsSending(true);
            void commands.giveNewIdentity().finally(() => {
              setIsSending(false);
            });
          }}
        >
          Give this computer a new identity
        </Button>
      </div>
    </div>
  );
}
