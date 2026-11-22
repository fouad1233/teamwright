import { Check, FolderPlus, Plug, Trash2 } from 'lucide-react';
import { useState, type FormEvent } from 'react';

import type { ConnectorStart, Library, LibraryConnector, Project } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button, IconButton } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { Field, TextInput } from '@/shared/ui/Field';
import { Refusal } from '@/shared/ui/Form';
import { Switch } from '@/shared/ui/Switch';
import { describeFailure } from '@/shared/ui/Toast';

import styles from './LibraryDialog.module.css';
import { confirmationOf, startInWords, variablesOf, wordsOf } from './kitModel';
import { useLibrary, useLibraryCommands, type LibraryCommands } from './kitQueries';
import { useTeamMembers } from './TeamContext';

interface LibraryDialogProps {
  open: boolean;
  project: Project;
  onClose: () => void;
}

/**
 * The library of a project: the skills and connectors the manager can give
 * its specialists (decision 0039). Each specialist is given some of it in
 * its own dialog.
 */
export function LibraryDialog({ open, project, onClose }: LibraryDialogProps) {
  return (
    <Dialog
      open={open}
      title={`Skills and connectors of ${project.name}`}
      size="wide"
      onClose={onClose}
    >
      <LibraryContent project={project} />
    </Dialog>
  );
}

function LibraryContent({ project }: { project: Project }) {
  const library = useLibrary(project.id);
  const commands = useLibraryCommands(project.id);
  const [refusal, setRefusal] = useState<string | null>(null);
  const [isAddingConnector, setIsAddingConnector] = useState(false);

  async function attempt(work: () => Promise<unknown>) {
    setRefusal(null);
    try {
      await work();
    } catch (failure) {
      setRefusal(describeFailure(failure));
    }
  }

  if (library.isError) {
    return (
      <p className={styles.note} role="alert">
        The library could not be loaded. {describeFailure(library.error)}
      </p>
    );
  }
  if (!library.data) {
    return <p className={styles.note}>Loading…</p>;
  }
  const { skills, connectors } = library.data;

  return (
    <div className={styles.library}>
      <section className={styles.section}>
        <h3 className="label">Skills</h3>
        <p className={styles.note}>
          A skill is a folder with a SKILL.md that teaches a specialist how to do a kind of work.
          Teamwright keeps a copy of it on this computer. A skill that would run commands or use
          tools you did not give is refused.
        </p>
        {skills.length > 0 && (
          <ul className={styles.list}>
            {skills.map((skill) => (
              <li key={skill.id} className={styles.item}>
                <div className={styles.words}>
                  <span className={styles.name}>{skill.name}</span>
                  <span className={styles.detail}>{skill.description}</span>
                  {!skill.here && skill.problem === null && (
                    <span className={styles.missing}>
                      Not on this computer: add its folder here again to use it here.
                    </span>
                  )}
                  {skill.problem !== null && (
                    <span className={styles.missing}>
                      No specialist is given it on this computer: {skill.problem}. Add its folder
                      again to replace the copy.
                    </span>
                  )}
                </div>
                <IconButton
                  label={`Remove ${skill.name}`}
                  onClick={() => {
                    void attempt(() => commands.removeSkill(skill.id));
                  }}
                >
                  <Trash2 />
                </IconButton>
              </li>
            ))}
          </ul>
        )}
        <div className={styles.actions}>
          <Button
            variant="secondary"
            icon={<FolderPlus />}
            onClick={() => {
              void attempt(() => commands.addSkill());
            }}
          >
            Add a skill
          </Button>
        </div>
      </section>

      <section className={styles.section}>
        <h3 className="label">Connectors</h3>
        <p className={styles.note}>
          A connector is a tool server that reaches something outside the folder of the project: a
          ticket system, a parts catalog. A specialist given one can reach whatever this connector
          reaches, with the tools you give it, one by one.
        </p>
        {connectors.length > 0 && (
          <ul className={styles.list}>
            {connectors.map((connector) => (
              <li key={connector.id} className={styles.item}>
                <div className={styles.words}>
                  <span className={styles.name}>
                    {connector.name}
                    {!connector.confirmed && (
                      <span className={classNames('label', styles.waiting)}>
                        Waits for you on this computer
                      </span>
                    )}
                  </span>
                  {connector.confirmed ? (
                    <>
                      <span className={classNames(styles.detail, styles.start)} data-selectable>
                        {startInWords(connector.start)}
                      </span>
                      <span className={styles.detail}>Tools: {connector.tools.join(', ')}</span>
                      {variablesOf(connector.start).length > 0 && (
                        <span className={styles.detail}>
                          Secrets from the environment: {variablesOf(connector.start).join(', ')}
                        </span>
                      )}
                    </>
                  ) : (
                    <ConnectorToConfirm
                      connector={connector}
                      onConfirm={() =>
                        attempt(() =>
                          commands.confirmConnector(connector.id, connector.fingerprint),
                        )
                      }
                    />
                  )}
                  {connector.problem !== null && (
                    <span className={styles.missing}>
                      No specialist is given it any more: {connector.problem}. Remove it and add it
                      again.
                    </span>
                  )}
                  {connector.missingVariables.length > 0 && (
                    <span className={styles.missing}>
                      Not set on this computer: {connector.missingVariables.join(', ')}. Until then
                      no specialist here is given it.
                    </span>
                  )}
                </div>
                <IconButton
                  label={`Remove ${connector.name}`}
                  onClick={() => {
                    void attempt(() => commands.removeConnector(connector.id));
                  }}
                >
                  <Trash2 />
                </IconButton>
              </li>
            ))}
          </ul>
        )}
        {isAddingConnector ? (
          <ConnectorForm
            onAdd={(connector) => commands.addConnector(connector)}
            onDone={() => {
              setIsAddingConnector(false);
            }}
          />
        ) : (
          <div className={styles.actions}>
            <Button
              variant="secondary"
              icon={<Plug />}
              onClick={() => {
                setIsAddingConnector(true);
              }}
            >
              Add a connector
            </Button>
          </div>
        )}
      </section>

      {library.data.waitingTools.length > 0 && (
        <WaitingKits
          library={library.data}
          onConfirm={(memberId, shown) => attempt(() => commands.confirmKit(memberId, shown))}
        />
      )}

      {refusal !== null && <Refusal>{refusal}</Refusal>}
    </div>
  );
}

interface ConnectorToConfirmProps {
  connector: LibraryConnector;
  onConfirm: () => Promise<void>;
}

/**
 * A connector that arrived or changed through the sync folder: exactly what
 * would be started on this computer, and the confirmation that lets it
 * start here (decision 0039, note of 22 November 2026).
 */
function ConnectorToConfirm({ connector, onConfirm }: ConnectorToConfirmProps) {
  const start = connector.start;
  return (
    <div className={styles.confirm}>
      <p className={styles.detail}>
        Added or changed on another computer. Nothing on this computer starts it until you have
        looked at exactly what it would start, and confirmed it here.
      </p>
      <dl className={styles.definition}>
        {start.kind === 'program' ? (
          <>
            <dt>Program</dt>
            <dd>
              <code className={styles.word} data-selectable>
                {start.program}
              </code>
            </dd>
            <dt>Arguments</dt>
            <dd>
              {start.arguments.length === 0
                ? 'None'
                : start.arguments.map((argument, index) => (
                    <code key={index} className={styles.word} data-selectable>
                      {argument}
                    </code>
                  ))}
            </dd>
            <dt>Variables it is handed</dt>
            <dd>
              {start.variables.length === 0
                ? 'None'
                : start.variables.map((variable) => (
                    <code key={variable} className={styles.word} data-selectable>
                      {variable}
                    </code>
                  ))}
            </dd>
          </>
        ) : (
          <>
            <dt>Address</dt>
            <dd>
              <code className={styles.word} data-selectable>
                {start.url}
              </code>
            </dd>
            <dt>Headers</dt>
            <dd>
              {start.headers.length === 0
                ? 'None'
                : start.headers.map((header) => (
                    <span key={header.name} className={styles.headerLine}>
                      <code className={styles.word}>{header.name}</code>
                      {header.bearer ? ' Bearer and the secret in ' : ' the secret in '}
                      <code className={styles.word}>{header.variable}</code>
                    </span>
                  ))}
            </dd>
          </>
        )}
        <dt>Tools</dt>
        <dd>
          {connector.tools.map((tool) => (
            <code key={tool} className={styles.word}>
              {tool}
            </code>
          ))}
        </dd>
      </dl>
      <div className={styles.actions}>
        <Button
          variant="primary"
          icon={<Check />}
          onClick={() => {
            void onConfirm();
          }}
        >
          Confirm on this computer
        </Button>
      </div>
    </div>
  );
}

interface WaitingKitsProps {
  library: Library;
  onConfirm: LibraryCommands['confirmKit'];
}

/** The tools given to specialists on another computer, which wait to be confirmed here. */
function WaitingKits({ library, onConfirm }: WaitingKitsProps) {
  const members = useTeamMembers();
  const memberIds = [...new Set(library.waitingTools.map((waiting) => waiting.memberId))];
  return (
    <section className={styles.section}>
      <h3 className="label">Given on another computer</h3>
      <p className={styles.note}>
        These tools were given to a specialist on another computer. No specialist on this computer
        uses them until you confirm them here.
      </p>
      <ul className={styles.list}>
        {memberIds.map((memberId) => {
          const waiting = library.waitingTools.filter((entry) => entry.memberId === memberId);
          const name =
            members.find((member) => member.id === memberId)?.name ?? 'A former specialist';
          return (
            <li key={memberId} className={styles.item}>
              <div className={styles.words}>
                <span className={styles.name}>{name}</span>
                {waiting.map((entry) => {
                  const connector = library.connectors.find(
                    (known) => known.id === entry.connectorId,
                  );
                  return (
                    <span key={entry.connectorId} className={styles.detail}>
                      {connector?.name ?? 'A removed connector'}: {entry.tools.join(', ')}
                      {connector?.confirmed === false &&
                        '. The connector itself waits for you too, above.'}
                    </span>
                  );
                })}
              </div>
              <Button
                variant="secondary"
                icon={<Check />}
                onClick={() => {
                  void onConfirm(memberId, confirmationOf(waiting));
                }}
              >
                Confirm
              </Button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

interface ConnectorFormProps {
  onAdd: (connector: { name: string; start: ConnectorStart; tools: string[] }) => Promise<void>;
  onDone: () => void;
}

/** The small form in which a connector is described. */
function ConnectorForm({ onAdd, onDone }: ConnectorFormProps) {
  const [kind, setKind] = useState<ConnectorStart['kind']>('program');
  const [name, setName] = useState('');
  const [command, setCommand] = useState('');
  const [variables, setVariables] = useState('');
  const [url, setUrl] = useState('');
  const [header, setHeader] = useState('Authorization');
  const [headerVariable, setHeaderVariable] = useState('');
  const [bearer, setBearer] = useState(true);
  const [tools, setTools] = useState('');
  const [refusal, setRefusal] = useState<string | null>(null);

  async function submit(event: FormEvent) {
    event.preventDefault();
    setRefusal(null);
    const [program = '', ...programArguments] = command.trim().split(/\s+/);
    const start: ConnectorStart =
      kind === 'program'
        ? { kind, program, arguments: programArguments, variables: wordsOf(variables) }
        : {
            kind,
            url: url.trim(),
            headers:
              headerVariable.trim() === ''
                ? []
                : [{ name: header.trim(), variable: headerVariable.trim(), bearer }],
          };
    try {
      await onAdd({ name, start, tools: wordsOf(tools) });
      onDone();
    } catch (failure) {
      setRefusal(describeFailure(failure));
    }
  }

  return (
    <form className={styles.form} onSubmit={(event) => void submit(event)}>
      <Field label="Name" hint="Lower-case letters, digits and hyphens, such as parts-catalog.">
        <TextInput
          value={name}
          onChange={(event) => {
            setName(event.target.value);
          }}
          data-initial-focus
          required
        />
      </Field>
      <div className={styles.kinds} role="group" aria-label="How it is started">
        <Button
          variant={kind === 'program' ? 'primary' : 'ghost'}
          aria-pressed={kind === 'program'}
          onClick={() => {
            setKind('program');
          }}
        >
          A program on this computer
        </Button>
        <Button
          variant={kind === 'address' ? 'primary' : 'ghost'}
          aria-pressed={kind === 'address'}
          onClick={() => {
            setKind('address');
          }}
        >
          A web address
        </Button>
      </div>
      {kind === 'program' ? (
        <>
          <Field
            label="Command"
            hint="The program and its arguments, such as npx -y parts-server. The program is a name found on the search path or a full path,, never a relative path such as ./server. It starts in an empty folder of Teamwright's own, not the folder of the project, with only the variables named below and the few every program needs."
          >
            <TextInput
              value={command}
              onChange={(event) => {
                setCommand(event.target.value);
              }}
              required
            />
          </Field>
          <Field
            label="Environment variables"
            hint="The names of the variables that hold its secrets, never their values. They are read from the environment Teamwright was started in, handed to this connector alone, and kept from the specialists' sessions."
          >
            <TextInput
              value={variables}
              onChange={(event) => {
                setVariables(event.target.value);
              }}
              placeholder="PARTS_TOKEN"
            />
          </Field>
        </>
      ) : (
        <>
          <Field
            label="Address"
            hint="It travels with the project to your other computers, so it must not hold a secret."
          >
            <TextInput
              value={url}
              onChange={(event) => {
                setUrl(event.target.value);
              }}
              placeholder="https://tickets.example.com/mcp"
              required
            />
          </Field>
          <div className={styles.header}>
            <Field label="Header">
              <TextInput
                value={header}
                onChange={(event) => {
                  setHeader(event.target.value);
                }}
              />
            </Field>
            <Field label="Its secret is in the variable">
              <TextInput
                value={headerVariable}
                onChange={(event) => {
                  setHeaderVariable(event.target.value);
                }}
                placeholder="TICKETS_TOKEN"
              />
            </Field>
          </div>
          <Switch on={bearer} label="Send it as Bearer and the secret" onChange={setBearer} />
        </>
      )}
      <Field
        label="Tools"
        hint="The names of the tools it offers that you may give, separated by commas."
      >
        <TextInput
          value={tools}
          onChange={(event) => {
            setTools(event.target.value);
          }}
          placeholder="search_parts, get_part"
          required
        />
      </Field>
      {refusal !== null && <Refusal>{refusal}</Refusal>}
      <div className={styles.formActions}>
        <Button variant="ghost" onClick={onDone}>
          Cancel
        </Button>
        <Button variant="primary" type="submit">
          Add the connector
        </Button>
      </div>
    </form>
  );
}
