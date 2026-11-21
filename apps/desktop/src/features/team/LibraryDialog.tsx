import { FolderPlus, Plug, Trash2 } from 'lucide-react';
import { useState, type FormEvent } from 'react';

import type { ConnectorStart, Project } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button, IconButton } from '@/shared/ui/Button';
import { Dialog } from '@/shared/ui/Dialog';
import { Field, TextInput } from '@/shared/ui/Field';
import { Refusal } from '@/shared/ui/Form';
import { Switch } from '@/shared/ui/Switch';
import { describeFailure } from '@/shared/ui/Toast';

import styles from './LibraryDialog.module.css';
import { startInWords, variablesOf, wordsOf } from './kitModel';
import { useLibrary, useLibraryCommands } from './kitQueries';

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
                  {!skill.here && (
                    <span className={styles.missing}>
                      Not on this computer: add its folder here again to use it here.
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
                  <span className={styles.name}>{connector.name}</span>
                  <span className={classNames(styles.detail, styles.start)} data-selectable>
                    {startInWords(connector.start)}
                  </span>
                  <span className={styles.detail}>Tools: {connector.tools.join(', ')}</span>
                  {variablesOf(connector.start).length > 0 && (
                    <span className={styles.detail}>
                      Secrets from the environment: {variablesOf(connector.start).join(', ')}
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

      {refusal !== null && <Refusal>{refusal}</Refusal>}
    </div>
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
          <Field label="Command" hint="The program and its arguments, such as npx -y parts-server.">
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
            hint="The names of the variables that hold its secrets, never their values. They are read from the environment Teamwright was started in."
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
