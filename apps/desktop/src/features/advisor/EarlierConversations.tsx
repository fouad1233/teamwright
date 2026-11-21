import { ArrowLeft, History, Trash2 } from 'lucide-react';
import { useState, type FormEvent } from 'react';

import type { DigestDraft, EarlierConversation, Project } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button, IconButton } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { Field, TextArea, TextInput } from '@/shared/ui/Field';
import { describeFailure } from '@/shared/ui/Toast';

import {
  useDigestCommands,
  useDigestDraft,
  useDigests,
  useEarlierConversations,
} from './digestQueries';
import styles from './EarlierConversations.module.css';
import {
  countOf,
  dayOf,
  durationInWords,
  filterFolders,
  nameOf,
  sizeInWords,
} from './earlierModel';

/** What is kept of a conversation, and where it goes, in one plain sentence. */
const WHAT_IS_KEPT =
  'Only what you typed, the names of changed files and the last answer are kept, with what looks like a secret hidden. Read it before you keep it: it goes with the project to your other computers.';

/**
 * The digests of earlier conversations kept with a project, on the page
 * where planning starts: each can be read and removed, and more can be
 * imported. The Advisor starts from them.
 */
export function KeptDigests({ project }: { project: Project }) {
  const digests = useDigests(project.id);
  const commands = useDigestCommands(project.id);
  const [isImporting, setIsImporting] = useState(false);
  const [readId, setReadId] = useState<string | null>(null);
  const kept = digests.data ?? [];

  return (
    <section className={styles.kept} aria-labelledby="kept-digests">
      <header className={styles.keptHeader}>
        <h3 id="kept-digests" className="label">
          Earlier conversations
        </h3>
        <span className={styles.count}>{kept.length > 0 ? kept.length : ''}</span>
        <span className={styles.space} />
        <Button
          icon={<History />}
          onClick={() => {
            setIsImporting(true);
          }}
        >
          Import earlier conversations
        </Button>
      </header>

      {kept.length === 0 ? (
        <p className={styles.hint}>
          Talked this project through with Claude Code before? Import those conversations, and the
          Advisor starts from a short account of each instead of asking again.
        </p>
      ) : (
        <>
          <ul className={styles.digests}>
            {kept.map((digest) => {
              const isRead = readId === digest.id;
              return (
                <li key={digest.id} className={styles.digest}>
                  <div className={styles.digestLine}>
                    <button
                      type="button"
                      className={styles.digestTitle}
                      aria-expanded={isRead}
                      onClick={() => {
                        setReadId(isRead ? null : digest.id);
                      }}
                    >
                      {digest.title}
                    </button>
                    <span className={styles.meta}>
                      {[dayOf(digest.endedAt ?? digest.startedAt), digest.heldIn]
                        .filter(Boolean)
                        .join(' · ')}
                    </span>
                    <IconButton
                      label="Remove this digest"
                      onClick={() => {
                        void commands.remove(digest.id);
                      }}
                    >
                      <Trash2 />
                    </IconButton>
                  </div>
                  {isRead && (
                    <p className={styles.digestText} data-selectable>
                      {digest.text}
                    </p>
                  )}
                </li>
              );
            })}
          </ul>
          <p className={styles.hint}>
            The Advisor reads these when you start planning, and asks where they may be out of date.
          </p>
        </>
      )}

      <ImportEarlierConversations
        project={project}
        open={isImporting}
        onClose={() => {
          setIsImporting(false);
        }}
      />
    </section>
  );
}

interface ImportProps {
  project: Project;
  open: boolean;
  onClose: () => void;
}

/**
 * The dialog that imports earlier conversations: the list of those found on
 * this computer, with a filter, and the digest of one to read, correct and
 * keep.
 */
export function ImportEarlierConversations({ project, open, onClose }: ImportProps) {
  const [chosen, setChosen] = useState<EarlierConversation | null>(null);

  function close() {
    setChosen(null);
    onClose();
  }

  return (
    <Dialog open={open} title="Import earlier conversations" size="wide" onClose={close}>
      {chosen === null ? (
        <FoundConversations project={project} onChoose={setChosen} />
      ) : (
        <DigestEditor
          // A fresh editor for every conversation, so nothing typed for one lingers.
          key={chosen.source}
          project={project}
          conversation={chosen}
          onBack={() => {
            setChosen(null);
          }}
        />
      )}
    </Dialog>
  );
}

interface FoundProps {
  project: Project;
  onChoose: (conversation: EarlierConversation) => void;
}

/** The conversations found on this computer, by folder, with a filter. */
function FoundConversations({ project, onChoose }: FoundProps) {
  const found = useEarlierConversations(project.id);
  const [filter, setFilter] = useState('');
  const folders = filterFolders(found.data ?? [], filter);
  const total = countOf(found.data ?? []);

  return (
    <div className={styles.found}>
      <p className={styles.lead}>
        Conversations you held with Claude Code on this computer. Choose one to read what would be
        kept of it. The conversation itself stays where it is.
      </p>
      {total > 0 && (
        <TextInput
          data-initial-focus
          aria-label="Filter the conversations"
          placeholder={`Filter ${total.toString()} conversations by words or folder…`}
          value={filter}
          onChange={(event) => {
            setFilter(event.target.value);
          }}
        />
      )}

      {found.isPending ? (
        <p className={styles.quiet}>Looking for conversations…</p>
      ) : found.isError ? (
        <p className={styles.problem}>{describeFailure(found.error)}</p>
      ) : total === 0 ? (
        <p className={styles.quiet}>
          No conversations with Claude Code were found on this computer. Once you have talked with
          it here, they show up in this list.
        </p>
      ) : folders.length === 0 ? (
        <p className={styles.quiet}>No conversation matches the filter.</p>
      ) : (
        <div className={styles.folders}>
          {folders.map((folder) => (
            <section key={folder.folder} className={styles.folder}>
              <h4 className={styles.folderName}>
                <span className={styles.path} title={folder.folder}>
                  {folder.folder}
                </span>
                {folder.isOfProject && (
                  <span className={classNames('label', styles.ofProject)}>
                    Folder of this project
                  </span>
                )}
              </h4>
              <ul className={styles.rows}>
                {folder.conversations.map((conversation) => (
                  <li key={conversation.source}>
                    <button
                      type="button"
                      className={styles.row}
                      onClick={() => {
                        onChoose(conversation);
                      }}
                    >
                      <span className={styles.rowName}>{nameOf(conversation)}</span>
                      {conversation.title !== null && conversation.firstRequest !== null && (
                        <span className={styles.rowRequest}>{conversation.firstRequest}</span>
                      )}
                      <span className={styles.meta}>
                        {[
                          dayOf(conversation.endedAt ?? conversation.startedAt),
                          durationInWords(conversation.startedAt, conversation.endedAt),
                          sizeInWords(conversation.sizeBytes),
                        ]
                          .filter(Boolean)
                          .join(' · ')}
                        {conversation.isKept && (
                          <span className={classNames('label', styles.keptMark)}>Kept</span>
                        )}
                      </span>
                    </button>
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </div>
      )}
    </div>
  );
}

interface EditorProps {
  project: Project;
  conversation: EarlierConversation;
  onBack: () => void;
}

/** The digest of one conversation, to read, cut, correct and keep. */
function DigestEditor({ project, conversation, onBack }: EditorProps) {
  const commands = useDigestCommands(project.id);
  const made = useDigestDraft(project.id, conversation.source);
  // What the manager changed; until then, the digest as the rules made it.
  const [edited, setEdited] = useState<DigestDraft | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [isKeeping, setIsKeeping] = useState(false);
  const draft = edited ?? made.data ?? null;

  async function keep(event: FormEvent) {
    event.preventDefault();
    if (draft === null || isKeeping) {
      return;
    }
    setIsKeeping(true);
    const refusal = await commands.keep(draft);
    setIsKeeping(false);
    if (refusal === null) {
      onBack();
    } else {
      setProblem(refusal);
    }
  }

  return (
    <form className={styles.editor} onSubmit={(event) => void keep(event)}>
      <div>
        <Button variant="ghost" icon={<ArrowLeft />} onClick={onBack}>
          All conversations
        </Button>
      </div>
      {made.isPending ? (
        <p className={styles.quiet}>Reading the conversation…</p>
      ) : draft === null ? (
        <p className={styles.problem}>{describeFailure(made.error)}</p>
      ) : (
        <>
          <Field label="Title">
            <TextInput
              value={draft.title}
              maxLength={120}
              onChange={(event) => {
                setEdited({ ...draft, title: event.target.value });
              }}
            />
          </Field>
          <Field label="What the Advisor will read">
            <TextArea
              data-initial-focus
              className={styles.text}
              value={draft.text}
              rows={16}
              onChange={(event) => {
                setEdited({ ...draft, text: event.target.value });
              }}
            />
          </Field>
          <p className={styles.privacy}>{WHAT_IS_KEPT}</p>
          {problem !== null && <p className={styles.problem}>{problem}</p>}
          <div className={styles.actions}>
            <Button variant="primary" type="submit" disabled={isKeeping || conversation.isKept}>
              Keep this digest
            </Button>
            <span className={styles.quiet}>
              {conversation.isKept
                ? 'A digest of this conversation is kept already. Remove it first to keep another.'
                : `${draft.text.length.toString()} of 8,000 characters`}
            </span>
          </div>
        </>
      )}
    </form>
  );
}
