import { useState } from 'react';

import type { IssueDraft, Project, Task } from '@/shared/api/types';
import { taskReference } from '@/shared/lib/format';
import { Button } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { FormActions, Refusal } from '@/shared/ui/Form';
import { describeFailure } from '@/shared/ui/Toast';

import { GitHubItems } from './GitHubItems';
import { agoInWords } from './gitHubModel';
import { useGitHubCommands, useTaskOnGitHub } from './gitHubQueries';
import styles from './TaskGitHub.module.css';

interface TaskGitHubProps {
  project: Project;
  task: Task;
}

/**
 * The part "GitHub" of a task: the pull requests and issues that name its
 * reference, read through the manager's own `gh`, and a way to open an
 * issue for it.
 */
export function TaskGitHub({ project, task }: TaskGitHubProps) {
  const onGitHub = useTaskOnGitHub(task.id);
  const commands = useGitHubCommands();
  const [refusal, setRefusal] = useState<string | null>(null);
  const [isReading, setReading] = useState(false);
  const [draft, setDraft] = useState<IssueDraft | null>(null);
  const reference = taskReference(project, task);

  async function attempt(work: () => Promise<unknown>) {
    setRefusal(null);
    try {
      await work();
    } catch (failure) {
      setRefusal(describeFailure(failure));
    }
  }

  async function refresh() {
    setReading(true);
    await attempt(() => commands.refreshTask(task.id));
    setReading(false);
  }

  if (onGitHub.isPending) {
    return <p className={styles.note}>Reading GitHub…</p>;
  }
  if (onGitHub.isError) {
    return (
      <div className={styles.part}>
        <p className={styles.note} role="alert">
          {describeFailure(onGitHub.error)}
        </p>
        <div className={styles.actions}>
          <Button variant="ghost" disabled={isReading} onClick={() => void refresh()}>
            Try again
          </Button>
        </div>
      </div>
    );
  }
  const { repository, readAt, items } = onGitHub.data;
  if (repository === null) {
    return (
      <p className={styles.note}>
        This project has no GitHub repository. Name one in the integrations of the project, or
        choose a work folder that is a clone of one.
      </p>
    );
  }

  return (
    <div className={styles.part}>
      {items.length === 0 ? (
        <p className={styles.note}>Nothing on GitHub names {reference} yet.</p>
      ) : (
        <GitHubItems
          items={items}
          onOpen={(url) => {
            void attempt(() => commands.open(url));
          }}
        />
      )}
      <div className={styles.actions}>
        <Button
          variant="secondary"
          onClick={() => {
            void attempt(async () => {
              setDraft(await commands.issueDraft(task.id));
            });
          }}
        >
          Open an issue for this task
        </Button>
        <Button variant="ghost" disabled={isReading} onClick={() => void refresh()}>
          {isReading ? 'Reading…' : 'Refresh'}
        </Button>
        <span className={styles.age}>
          {repository}
          {readAt !== null && `, read ${agoInWords(readAt)}`}
        </span>
      </div>
      {refusal !== null && <Refusal>{refusal}</Refusal>}

      <Dialog
        open={draft !== null}
        title="Open an issue on GitHub?"
        onClose={() => {
          setDraft(null);
        }}
      >
        {draft !== null && (
          <IssueConfirmation
            draft={draft}
            onCancel={() => {
              setDraft(null);
            }}
            onConfirm={async () => {
              await commands.openIssue(task.id);
              setDraft(null);
            }}
          />
        )}
      </Dialog>
    </div>
  );
}

interface IssueConfirmationProps {
  draft: IssueDraft;
  onCancel: () => void;
  onConfirm: () => Promise<void>;
}

/** What will be sent and where, for the manager to confirm before anything leaves. */
function IssueConfirmation({ draft, onCancel, onConfirm }: IssueConfirmationProps) {
  const [refusal, setRefusal] = useState<string | null>(null);
  const [isSending, setSending] = useState(false);

  return (
    <div className={styles.confirmation}>
      <p className={styles.note}>
        This issue will be opened in <strong>{draft.repository}</strong>, as you, through GitHub’s
        command line program.
      </p>
      <dl className={styles.draft}>
        <dt className="label">Title</dt>
        <dd>{draft.title}</dd>
        <dt className="label">Body</dt>
        <dd className={styles.body}>{draft.body}</dd>
      </dl>
      {refusal !== null && <Refusal>{refusal}</Refusal>}
      <FormActions>
        <Button variant="ghost" data-initial-focus onClick={onCancel}>
          Cancel
        </Button>
        <Button
          disabled={isSending}
          onClick={() => {
            setSending(true);
            setRefusal(null);
            onConfirm()
              .catch((failure: unknown) => {
                setRefusal(describeFailure(failure));
              })
              .finally(() => {
                setSending(false);
              });
          }}
        >
          {isSending ? 'Opening…' : 'Open the issue'}
        </Button>
      </FormActions>
    </div>
  );
}
