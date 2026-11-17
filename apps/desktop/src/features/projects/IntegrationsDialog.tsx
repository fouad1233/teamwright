import { ChevronDown, Trash2 } from 'lucide-react';
import { useState, type ReactNode } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Delivery, EventKind, Project, ProjectIntegrations } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatMoment } from '@/shared/lib/format';
import { Button, IconButton } from '@/shared/ui/Button';
import { CheckMenu } from '@/shared/ui/CheckMenu';
import { Dialog } from '@/shared/ui/Dialog';
import { TextInput } from '@/shared/ui/Field';
import { Refusal } from '@/shared/ui/Form';
import { ServiceMark, type MarkedService } from '@/shared/ui/ServiceMark';
import { describeFailure } from '@/shared/ui/Toast';

import { GitHubItems } from '../github/GitHubItems';
import { agoInWords, gitHubInWords, linkedInWords } from '../github/gitHubModel';
import { useGitHubCommands, useGitHubOverview } from '../github/gitHubQueries';
import { googleDriveFactInWords, googleDriveStateInWords } from '../sync/googleDriveModel';
import { useGoogleDrive } from '../sync/syncQueries';
import styles from './IntegrationsDialog.module.css';
import {
  useDeliveryTests,
  useIntegrationCommands,
  useIntegrationDeliveries,
  useProjectIntegrations,
} from './integrationQueries';
import { EVENT_LABELS, chosenInWords, slackInWords, webAddressesInWords } from './integrationModel';

interface IntegrationsDialogProps {
  open: boolean;
  project: Project;
  onClose: () => void;
}

/**
 * The integrations of a project on this computer: GitHub, a Slack channel,
 * web addresses that listen, and whether this computer syncs through
 * Google Drive. What is set here stays on this computer.
 */
export function IntegrationsDialog({ open, project, onClose }: IntegrationsDialogProps) {
  return (
    <Dialog open={open} title={`Integrations of ${project.name}`} size="wide" onClose={onClose}>
      <Integrations project={project} onLeave={onClose} />
    </Dialog>
  );
}

/** What a part of the dialog does when the backend refuses: it says why, under the part. */
type Attempt = (work: () => Promise<void>) => Promise<boolean>;

/**
 * The content of the dialog, a component of its own so that what it says
 * about the last refusal starts empty every time the dialog opens.
 */
function Integrations({ project, onLeave }: { project: Project; onLeave: () => void }) {
  const integrations = useProjectIntegrations(project.id);
  const [refusal, setRefusal] = useState<string | null>(null);

  const attempt: Attempt = async (work) => {
    setRefusal(null);
    try {
      await work();
      return true;
    } catch (failure) {
      setRefusal(describeFailure(failure));
      return false;
    }
  };

  if (integrations.isError) {
    return (
      <p className={styles.note} role="alert">
        The integrations could not be loaded. {describeFailure(integrations.error)}
      </p>
    );
  }
  if (!integrations.data) {
    return <p className={styles.note}>Loading…</p>;
  }

  return (
    <div className={styles.integrations}>
      <p className={styles.note}>
        What is set here stays on this computer. Addresses are secrets, and are never synced to your
        other computers.
      </p>
      <GitHubPart project={project} integrations={integrations.data} attempt={attempt} />
      <SlackPart project={project} integrations={integrations.data} attempt={attempt} />
      <WebAddressesPart project={project} integrations={integrations.data} attempt={attempt} />
      <GoogleDrivePart onLeave={onLeave} />
      {refusal !== null && <Refusal>{refusal}</Refusal>}
    </div>
  );
}

interface PartProps {
  project: Project;
  integrations: ProjectIntegrations;
  attempt: Attempt;
}

/**
 * The head of a part: its name, after the official mark of the service if
 * it has one, and in one line whether it is on.
 */
function PartHead({ name, mark, state }: { name: string; mark?: MarkedService; state: string }) {
  return (
    <header className={styles.head}>
      <h3 className={classNames('label', styles.name)}>
        {mark !== undefined && <ServiceMark service={mark} />}
        {name}
      </h3>
      <span className={styles.state}>{state}</span>
    </header>
  );
}

function GitHubPart({ project, integrations, attempt }: PartProps) {
  const commands = useIntegrationCommands(project.id);
  const gitHub = useGitHubCommands();
  const overview = useGitHubOverview(project.id);
  const [name, setName] = useState(integrations.repository ?? '');
  const [isReading, setReading] = useState(false);
  const data = overview.data;

  return (
    <section className={styles.part} aria-label="GitHub">
      <PartHead name="GitHub" mark="github" state={data === undefined ? '' : gitHubInWords(data)} />
      <p className={styles.note}>
        The tasks of this project know the pull requests and issues that name their key, such as{' '}
        <code>{project.key}-4</code>. Teamwright asks GitHub’s own command line program, signed in
        as you, and keeps no token.
      </p>
      {data !== undefined && data.signIn.problem !== null && (
        <p className={styles.problem} role="status">
          {data.signIn.problem}.
        </p>
      )}
      {data !== undefined && data.signIn.state === 'signed_in' && (
        <p className={styles.fact}>
          Signed in{data.signIn.account === null ? '' : ` as ${data.signIn.account}`}.{' '}
          {data.repository === null
            ? 'The work folder is not a clone of a repository on GitHub; name one below.'
            : `${data.isFromWorkFolder ? 'The work folder is a clone of' : 'The repository is'} ${data.repository}.`}
        </p>
      )}
      {data !== undefined && data.repository !== null && data.signIn.state === 'signed_in' && (
        <div className={styles.reading}>
          <div className={styles.row}>
            <span className={styles.fact}>
              {data.readAt === null
                ? 'Not read since Teamwright started.'
                : `Read ${agoInWords(data.readAt)}. ${linkedInWords(data)}`}
            </span>
            <Button
              variant="ghost"
              disabled={isReading}
              onClick={() => {
                setReading(true);
                void attempt(() => gitHub.refresh(project.id)).finally(() => {
                  setReading(false);
                });
              }}
            >
              {isReading ? 'Reading…' : data.readAt === null ? 'Read now' : 'Refresh'}
            </Button>
          </div>
          {data.unlinked.length > 0 && (
            <>
              <h4 className={styles.subhead}>
                About no task
                {data.unlinkedCount > data.unlinked.length &&
                  `, the first ${data.unlinked.length.toString()} of ${data.unlinkedCount.toString()}`}
              </h4>
              <GitHubItems
                items={data.unlinked}
                onOpen={(url) => {
                  void attempt(() => gitHub.open(url));
                }}
              />
            </>
          )}
        </div>
      )}
      <form
        className={styles.row}
        onSubmit={(event) => {
          event.preventDefault();
          void attempt(() => commands.setRepository(name.trim() === '' ? null : name)).then(() =>
            overview.refetch(),
          );
        }}
      >
        <TextInput
          className={styles.input}
          aria-label="Repository"
          placeholder="owner/name, or empty to read it from the work folder"
          value={name}
          onChange={(event) => {
            setName(event.target.value);
          }}
        />
        <Button type="submit" variant="secondary">
          Save
        </Button>
      </form>
    </section>
  );
}

function SlackPart({ project, integrations, attempt }: PartProps) {
  const commands = useIntegrationCommands(project.id);
  const tests = useDeliveryTests(project.id);
  const deliveries = useIntegrationDeliveries(project.id);
  const [address, setAddress] = useState('');
  const [isSending, setSending] = useState(false);
  const [tried, setTried] = useState<Delivery | null>(null);
  const slack = integrations.slack;

  return (
    <section className={styles.part} aria-label="Slack">
      <PartHead name="Slack" state={slackInWords(integrations)} />
      <p className={styles.note}>
        What happens in this project is told in a channel, one plain line for each event. Slack
        gives a channel its address when you add an incoming webhook to it.
      </p>
      {slack !== null && (
        <div className={styles.line}>
          <span className={styles.address} data-selectable>
            {slack.address}
          </span>
          <EventsMenu
            label="Events told in the channel"
            events={slack.events}
            onToggle={(kind, on) => {
              void attempt(() => commands.setSlackEvent(kind, on));
            }}
          />
          <IconButton
            label="Remove the channel"
            onClick={() => {
              void attempt(() => commands.setSlackAddress(null));
            }}
          >
            <Trash2 />
          </IconButton>
        </div>
      )}
      {slack !== null && (
        <div className={styles.row}>
          <Button
            variant="ghost"
            disabled={isSending}
            onClick={() => {
              setSending(true);
              setTried(null);
              void attempt(async () => {
                setTried(await tests.testChannel());
              }).finally(() => {
                setSending(false);
              });
            }}
          >
            {isSending ? 'Sending…' : 'Send a test message'}
          </Button>
          {tried !== null && (
            <span className={styles.fact} role="status">
              {tried.succeeded ? 'Slack took it.' : `It could not be sent: ${tried.failure ?? ''}`}
            </span>
          )}
        </div>
      )}
      {slack !== null && tried === null && deliveries.data?.channelFailure && (
        <p className={styles.problem} role="status">
          The last message could not be sent: {deliveries.data.channelFailure.failure}
        </p>
      )}
      <form
        className={styles.row}
        onSubmit={(event) => {
          event.preventDefault();
          void attempt(() => commands.setSlackAddress(address)).then((done) => {
            if (done) {
              setAddress('');
            }
          });
        }}
      >
        <TextInput
          className={styles.input}
          aria-label="Address of the channel"
          placeholder="https://hooks.slack.com/services/…"
          autoComplete="off"
          value={address}
          onChange={(event) => {
            setAddress(event.target.value);
          }}
        />
        <Button type="submit" variant="secondary" disabled={address.trim() === ''}>
          {slack === null ? 'Set the channel' : 'Change the address'}
        </Button>
      </form>
    </section>
  );
}

function WebAddressesPart({ project, integrations, attempt }: PartProps) {
  const commands = useIntegrationCommands(project.id);
  const tests = useDeliveryTests(project.id);
  const deliveries = useIntegrationDeliveries(project.id);
  const [address, setAddress] = useState('');
  const [secret, setSecret] = useState('');

  return (
    <section className={styles.part} aria-label="Web addresses">
      <PartHead name="Web addresses" state={webAddressesInWords(integrations)} />
      <p className={styles.note}>
        Each chosen event is sent as JSON to an address. Zapier (Catch Hook), Make and n8n listen
        this way, and so can your own tools. A secret signs what is sent, so the listener can tell
        it comes from you.
      </p>
      {integrations.webAddresses.length > 0 && (
        <ul className={styles.list}>
          {integrations.webAddresses.map((listening, index) => (
            <li key={`${listening.address}-${index.toString()}`} className={styles.entry}>
              <div className={styles.line}>
                <span className={styles.address} data-selectable>
                  {listening.address}
                </span>
                {listening.isSigned && <span className={styles.mark}>Signed</span>}
                <EventsMenu
                  label="Events sent to the address"
                  events={listening.events}
                  onToggle={(kind, on) => {
                    void attempt(() => commands.setWebAddressEvent(index, kind, on));
                  }}
                />
                <Button
                  variant="ghost"
                  onClick={() => {
                    void attempt(async () => {
                      await tests.testWebAddress(index);
                    });
                  }}
                >
                  Send a test event
                </Button>
                <IconButton
                  label={`Remove ${listening.address}`}
                  onClick={() => {
                    void attempt(() => commands.removeWebAddress(index));
                  }}
                >
                  <Trash2 />
                </IconButton>
              </div>
              <DeliveryList deliveries={deliveries.data?.addresses[index] ?? []} />
            </li>
          ))}
        </ul>
      )}
      <form
        className={styles.row}
        onSubmit={(event) => {
          event.preventDefault();
          void attempt(() =>
            commands.addWebAddress(address, secret.trim() === '' ? null : secret),
          ).then((done) => {
            if (done) {
              setAddress('');
              setSecret('');
            }
          });
        }}
      >
        <TextInput
          className={styles.input}
          aria-label="Web address"
          placeholder="https://…"
          autoComplete="off"
          value={address}
          onChange={(event) => {
            setAddress(event.target.value);
          }}
        />
        <TextInput
          className={styles.secret}
          aria-label="Secret"
          placeholder="Secret (optional)"
          type="password"
          autoComplete="off"
          value={secret}
          onChange={(event) => {
            setSecret(event.target.value);
          }}
        />
        <Button type="submit" variant="secondary" disabled={address.trim() === ''}>
          Add the address
        </Button>
      </form>
    </section>
  );
}

/**
 * Google Drive: how syncing through it works, and whether this computer
 * does. Nothing is set per project; the button leads to the page
 * Computers, where it is chosen.
 */
function GoogleDrivePart({ onLeave }: { onLeave: () => void }) {
  const drive = useGoogleDrive();
  const navigation = useNavigation();

  return (
    <section className={styles.part} aria-label="Google Drive">
      <PartHead
        name="Google Drive"
        mark="google-drive"
        state={drive.data === undefined ? '' : googleDriveStateInWords(drive.data)}
      />
      <p className={styles.note}>
        Google Drive for desktop keeps a folder of your Drive on each of your computers, and
        Teamwright keeps all your projects in step through it. It is chosen once for this computer,
        on the page Computers, not for each project.
      </p>
      {drive.data !== undefined && (
        <p className={styles.fact}>{googleDriveFactInWords(drive.data)}</p>
      )}
      <div className={styles.row}>
        <Button
          variant="secondary"
          onClick={() => {
            onLeave();
            navigation.openComputers();
          }}
        >
          Go to Computers
        </Button>
      </div>
    </section>
  );
}

/** The last deliveries of an address since Teamwright started, newest first: when, what, the answer. */
function DeliveryList({ deliveries }: { deliveries: readonly Delivery[] }) {
  if (deliveries.length === 0) {
    return null;
  }
  return (
    <ol className={styles.deliveries} aria-label="Last deliveries">
      {deliveries.map((delivery, index) => (
        <li
          key={`${delivery.at}-${index.toString()}`}
          className={styles.delivery}
          data-failed={!delivery.succeeded || undefined}
        >
          <span className={styles.when}>{formatMoment(delivery.at)}</span>
          <span className={styles.event}>{delivery.event}</span>
          <span>
            {delivery.succeeded
              ? `Answered ${String(delivery.status)}`
              : (delivery.failure ?? 'Not taken')}
          </span>
        </li>
      ))}
    </ol>
  );
}

/** A button that says how many kinds of events are chosen, and opens the menu to choose them. */
function EventsMenu({
  label,
  events,
  onToggle,
}: {
  label: string;
  events: readonly EventKind[];
  onToggle: (kind: EventKind, on: boolean) => void;
}): ReactNode {
  return (
    <CheckMenu
      label={label}
      items={EVENT_LABELS.map(({ kind, label: name }) => ({
        value: kind,
        label: name,
        isChecked: events.includes(kind),
      }))}
      onToggle={(kind) => {
        onToggle(kind, !events.includes(kind));
      }}
    >
      <Button variant="ghost" icon={<ChevronDown />}>
        {chosenInWords(events)}
      </Button>
    </CheckMenu>
  );
}
