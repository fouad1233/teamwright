import { useId, useMemo, useState, type FormEvent } from 'react';

import { Button } from '@/shared/ui/Button';
import { TextInput } from '@/shared/ui/Field';

import type { KindProps } from '../views/viewKinds';
import { useSetViewContent } from '../views/viewQueries';
import styles from './EmbedView.module.css';
import {
  checkAddress,
  embedAddress,
  parseContent,
  serializeContent,
  shortenAddress,
} from './embedModel';

/**
 * The embed view: one page from elsewhere, shown in a frame under a thin
 * bar. The address is checked before it is kept, and the frame is
 * sandboxed: it may run its own scripts, keep its own cookies, send its own
 * forms and open pop-ups, and nothing else.
 */
export function EmbedView({ view }: KindProps) {
  const setContent = useSetViewContent();
  const { address } = useMemo(() => parseContent(view.content), [view.content]);
  // Whether the field is shown over a kept address: the manager pressed "Change".
  const [isChanging, setIsChanging] = useState(false);

  function keep(chosen: string) {
    setIsChanging(false);
    void setContent(view, serializeContent({ address: chosen }));
  }

  if (address === null || isChanging) {
    return (
      <AddressSheet
        // A new sheet for each address, so that its field starts from it.
        key={address ?? ''}
        current={address}
        onKeep={keep}
        onCancel={address === null ? undefined : () => setIsChanging(false)}
      />
    );
  }
  return <Frame address={address} onChange={() => setIsChanging(true)} />;
}

/** The page in its frame, under the bar. */
function Frame({ address, onChange }: { address: string; onChange: () => void }) {
  // A new number makes a new frame, which loads the page again.
  const [loads, setLoads] = useState(0);
  const shown = useMemo(() => embedAddress(address), [address]);

  function openInBrowser() {
    window.open(address, '_blank', 'noopener,noreferrer');
  }

  return (
    <div className={styles.embed}>
      <div className={styles.bar}>
        <span className={styles.address} title={address}>
          {shortenAddress(address)}
        </span>
        <Button variant="ghost" onClick={() => setLoads((count) => count + 1)}>
          Reload
        </Button>
        <Button variant="ghost" onClick={openInBrowser}>
          Open in the browser
        </Button>
        <Button variant="ghost" onClick={onChange}>
          Change
        </Button>
      </div>
      <p className={styles.blank}>
        Blank? Some sites do not allow this.{' '}
        <button type="button" className={styles.link} onClick={openInBrowser}>
          Open it in the browser.
        </button>
      </p>
      <iframe
        key={loads}
        className={styles.frame}
        src={shown}
        title="Page from elsewhere"
        sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
        referrerPolicy="no-referrer"
        allow="fullscreen"
        allowFullScreen
      />
    </div>
  );
}

/** The small sheet where an address is pasted, first or again. */
function AddressSheet({
  current,
  onKeep,
  onCancel,
}: {
  current: string | null;
  onKeep: (address: string) => void;
  onCancel: (() => void) | undefined;
}) {
  const [typed, setTyped] = useState(current ?? '');
  const [refusal, setRefusal] = useState<string | null>(null);
  const refusalId = useId();

  function submit(event: FormEvent) {
    event.preventDefault();
    const checked = checkAddress(typed);
    if (checked.ok) {
      setRefusal(null);
      onKeep(checked.address);
    } else {
      setRefusal(checked.reason);
    }
  }

  return (
    <div className={styles.ground}>
      <form className={styles.sheet} onSubmit={submit} noValidate>
        <h2 className={styles.lead}>A page from elsewhere, inside the project.</h2>
        <p className={styles.note}>
          A published sheet, a design file, a video or a map can be shown here, and stays beside
          your tasks. Some sites refuse to be shown inside other programs; for those, the page can
          be opened in the browser instead.
        </p>
        <label className={styles.field}>
          <span className="label">Address</span>
          <TextInput
            type="text"
            inputMode="url"
            autoFocus
            spellCheck={false}
            autoComplete="off"
            placeholder="https://"
            value={typed}
            aria-invalid={refusal !== null}
            aria-describedby={refusal === null ? undefined : refusalId}
            data-invalid={refusal !== null || undefined}
            className={styles.input}
            onChange={(event) => {
              setTyped(event.target.value);
              setRefusal(null);
            }}
          />
        </label>
        {refusal !== null && (
          <p id={refusalId} className={styles.refusal} role="alert">
            {refusal}
          </p>
        )}
        <div className={styles.actions}>
          <Button type="submit" variant="primary">
            Show the page
          </Button>
          {onCancel !== undefined && (
            <Button variant="ghost" onClick={onCancel}>
              Cancel
            </Button>
          )}
        </div>
      </form>
    </div>
  );
}
