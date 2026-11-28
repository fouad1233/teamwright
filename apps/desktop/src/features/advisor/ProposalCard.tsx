import { ArrowRightLeft, ChevronRight, CornerDownRight, X } from 'lucide-react';
import { useState, type CSSProperties, type KeyboardEvent, type ReactNode } from 'react';

import type {
  ApprovedProposal,
  Effort,
  Member,
  Priority,
  Project,
  Proposal,
  ProposalChange,
  ProposedMove,
  ProposedTask,
  Task,
} from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatEstimate, parseEstimate, taskReference } from '@/shared/lib/format';
import { Button, IconButton, ValueButton } from '@/shared/ui/Button';
import { TextArea } from '@/shared/ui/Field';
import { ChoiceMenu } from '@/shared/ui/Menu';
import { Prose } from '@/shared/ui/Prose';

import { PRIORITIES, PriorityFlag, priorityLabel } from '../tasks/TaskBadges';
import { TaskComposer } from '../tasks/TaskComposer';
import { Picker } from '../tasks/TaskPicker';
import { MemberAvatar } from '../team/Assignee';
import { EFFORT_CHOICES, LEFT_TO_THE_PROGRAM, modelChoices } from '../team/profileChoices';
import { effortLabel, modelLabel } from '../team/teamModel';
import { appearsAt, waitsForManager } from '../work/waitsForManager';
import {
  approvalInWords,
  boardWholeChoices,
  countNewMembers,
  countWithoutOwner,
  draftOf,
  isOnTeam,
  keyOfPlace,
  moveChoices,
  movesElsewhere,
  movesUnder,
  ownerChoices,
  ownerInWords,
  partOfChoices,
  planInWords,
  planRows,
  teamInWords,
  valueOfOwner,
  waitChoices,
  waitedFor,
  waitsForInWords,
  wholeOfKey,
} from './advisorModel';
import styles from './ProposalCard.module.css';

/** What a task that is not of the ordinary priority says about itself. */
const PRIORITY_LABELS: Record<Priority, string | null> = {
  low: 'Low',
  normal: null,
  high: 'High',
  urgent: 'Urgent',
};

const PRIORITY_CHOICES = PRIORITIES.map((priority) => ({
  value: priority,
  label: priorityLabel(priority),
  icon: <PriorityFlag priority={priority} />,
}));

/** Where a refused change is said: "task 3", "member 1", or "new task". */
type Place = string;

interface Refusal {
  place: Place;
  reason: string;
}

interface ProposalCardProps {
  /** The proposal that waits for the manager, with the changes made so far. */
  proposal: Proposal;
  /** The project, to name its tasks as the manager knows them. */
  project: Project;
  /** The tasks the project has, which the proposal may name and move. */
  board: readonly Task[];
  /** The team the project has already. */
  team: readonly Member[];
  /**
   * Makes one change to the proposal, which is stored at once. Resolves to
   * nothing if it was taken, or to why it was refused, in words.
   */
  onChange: (change: ProposalChange) => Promise<string | null>;
  /** Approves the proposal as it reads. Resolves to whether it was taken. */
  onApprove: (approved: ApprovedProposal) => Promise<boolean>;
}

/**
 * What the Advisor proposes, for the manager to change and to approve: the
 * project in the Advisor's words, the specialists to add, and the tasks.
 *
 * Every task is changed in place: its title, its owner and its size in its
 * row, and its description, priority, the tasks it waits for and the task
 * it is a part of under it. A part stands under its task, set in, as a
 * subtask does on the board, and the parts of a task can be folded away.
 * The manager can take a member or a task out, add a task or a part, and
 * choose the model and the effort of a member. When the manager asked to
 * regroup tasks that exist, the tasks moved under a new task stand under it
 * ("moves here"), and those moved under another task of the project, or
 * out from under one, are listed after the plan; each move can be sent
 * elsewhere or taken out. Each change is stored as it is made, and a change
 * that breaks a rule is refused with the reason beside it. Nothing here
 * exists in the project, and nothing is moved, before it is approved.
 */
export function ProposalCard({
  proposal,
  project,
  board,
  team,
  onChange,
  onApprove,
}: ProposalCardProps) {
  const [shownTask, setShownTask] = useState<number | null>(null);
  // The tasks whose parts are folded away. All are shown at first: the
  // manager approves what is in the plan, so nothing starts hidden.
  const [folded, setFolded] = useState<ReadonlySet<number>>(new Set());
  const [refusal, setRefusal] = useState<Refusal | null>(null);
  const [isApproving, setIsApproving] = useState(false);
  const owners = ownerChoices(proposal, team);
  const withoutOwner = countWithoutOwner(proposal.tasks);
  const newMembers = countNewMembers(proposal, team);

  /** Sends one change. Resolves to whether it was taken. */
  async function change(place: Place, proposalChange: ProposalChange): Promise<boolean> {
    const reason = await onChange(proposalChange);
    setRefusal(reason === null ? null : { place, reason });
    return reason === null;
  }

  function refusalAt(place: Place) {
    return refusal?.place === place ? (
      <p className={styles.refusal} role="alert">
        {refusal.reason}
      </p>
    ) : null;
  }

  async function approve() {
    setIsApproving(true);
    const wasTaken = await onApprove(draftOf(proposal));
    if (!wasTaken) {
      setIsApproving(false);
    }
  }

  return (
    <section
      // The proposal is large and holds two tables: the sun lights its head.
      className={classNames(waitsForManager.frame, waitsForManager.lightAtHead, styles.card)}
      aria-label="The proposal of the Advisor"
    >
      <div className={classNames(waitsForManager.part, styles.head)}>
        <p className={classNames('label', styles.proposes)}>The Advisor proposes</p>
        <Prose text={proposal.summary} />
      </div>

      <div className={classNames(waitsForManager.part, styles.part)} style={appearsAt(1)}>
        <h4 className="label">
          Team · {newMembers === 0 ? 'nobody new' : teamInWords(newMembers)}
        </h4>
        {proposal.members.length > 0 && (
          <ul className={styles.members}>
            {proposal.members.map((member, place) => (
              <li key={member.name} className={styles.member}>
                {/* A proposed member is not on the team yet, so it has no color of its own. */}
                <MemberAvatar member={{ name: member.name, kind: 'agent', color: 'gray' }} />
                <div className={styles.who}>
                  <span className={styles.memberName}>{member.name}</span>
                  {member.responsibility !== '' && (
                    <span className={styles.responsibility}>{member.responsibility}</span>
                  )}
                  {member.reason !== '' && <span className={styles.reason}>{member.reason}</span>}
                </div>
                {isOnTeam(member.name, team) ? (
                  // The team has this member already. Approving does not add
                  // it again and does not change how it runs.
                  <span className={styles.there}>On the team already</span>
                ) : (
                  <>
                    <ChoiceMenu
                      label={`Model of ${member.name}`}
                      choices={modelChoices(member.model)}
                      value={member.model ?? LEFT_TO_THE_PROGRAM}
                      onChoose={(model) => {
                        void change(`member ${place.toString()}`, {
                          kind: 'setProfile',
                          member: place,
                          model: model === LEFT_TO_THE_PROGRAM ? null : model,
                          effort: member.effort,
                        });
                      }}
                    >
                      <ValueButton
                        className={styles.value}
                        isEmpty={member.model === null}
                        aria-label={`Model of ${member.name}`}
                      >
                        {member.model === null ? 'Default model' : modelLabel(member.model)}
                      </ValueButton>
                    </ChoiceMenu>
                    <ChoiceMenu
                      label={`Effort of ${member.name}`}
                      choices={EFFORT_CHOICES}
                      value={member.effort ?? LEFT_TO_THE_PROGRAM}
                      onChoose={(effort) => {
                        void change(`member ${place.toString()}`, {
                          kind: 'setProfile',
                          member: place,
                          model: member.model,
                          effort: effort === LEFT_TO_THE_PROGRAM ? null : (effort as Effort),
                        });
                      }}
                    >
                      <ValueButton
                        className={styles.value}
                        isEmpty={member.effort === null}
                        aria-label={`Effort of ${member.name}`}
                      >
                        {member.effort === null
                          ? 'Default effort'
                          : `${effortLabel(member.effort)} effort`}
                      </ValueButton>
                    </ChoiceMenu>
                    <IconButton
                      label={`Take ${member.name} out of the proposal`}
                      onClick={() => {
                        void change(`member ${place.toString()}`, {
                          kind: 'removeMember',
                          member: place,
                        });
                      }}
                    >
                      <X />
                    </IconButton>
                  </>
                )}
                {refusalAt(`member ${place.toString()}`)}
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className={classNames(waitsForManager.part, styles.part)} style={appearsAt(2)}>
        <h4 className="label">
          Plan · {planInWords(proposal.tasks)}
          {proposal.moves.length > 0 &&
            ` · ${proposal.moves.length.toString()} ${proposal.moves.length === 1 ? 'task' : 'tasks'} moved`}
        </h4>
        {proposal.tasks.length > 0 && (
          <ol className={styles.tasks}>
            {planRows(proposal.tasks, folded).map(({ place, depth, parts }) => {
              const task = proposal.tasks[place];
              if (!task) {
                return null;
              }
              const where: Place = `task ${place.toString()}`;
              const priority = PRIORITY_LABELS[task.priority];
              const isShown = shownTask === place;
              const number = (place + 1).toString();
              const isFolded = folded.has(place);
              return (
                // A row stays at its place. The fields in it are keyed by
                // their values, so a stored change shows at once. A part is
                // set in by how deep it lies.
                <li
                  key={place}
                  className={classNames(styles.task, depth > 0 && styles.isPart)}
                  style={{ '--depth': depth } as CSSProperties}
                >
                  <span className={styles.place}>{number}</span>
                  <span className={styles.titleCell}>
                    <TitleField
                      key={task.title}
                      title={task.title}
                      label={`Title of task ${number}`}
                      onRename={(title) => change(where, { kind: 'retitle', task: place, title })}
                    />
                    {parts > 0 && (
                      <button
                        type="button"
                        className={styles.fold}
                        aria-expanded={!isFolded}
                        title={
                          isFolded
                            ? `Show the parts of "${task.title}"`
                            : `Fold the parts of "${task.title}" away`
                        }
                        onClick={() => {
                          const next = new Set(folded);
                          if (isFolded) {
                            next.delete(place);
                          } else {
                            next.add(place);
                          }
                          setFolded(next);
                        }}
                      >
                        <ChevronRight className={styles.foldMark} aria-hidden />
                        {parts === 1 ? '1 part' : `${parts.toString()} parts`}
                      </button>
                    )}
                  </span>
                  <ChoiceMenu
                    label={`Who does "${task.title}"`}
                    choices={owners}
                    value={valueOfOwner(task.owner)}
                    onChoose={(value) => {
                      const chosen = owners.find((choice) => choice.value === value);
                      void change(where, {
                        kind: 'giveTo',
                        task: place,
                        owner: chosen?.owner ?? null,
                      });
                    }}
                  >
                    <ValueButton
                      className={classNames(styles.value, styles.owner)}
                      isEmpty={task.owner === null}
                      aria-label={`Who does "${task.title}"`}
                    >
                      {ownerInWords(task.owner)}
                    </ValueButton>
                  </ChoiceMenu>
                  <span className={styles.size}>
                    {priority !== null && <span className={styles.priority}>{priority}</span>}
                    <SizeField
                      key={String(task.estimateMinutes)}
                      minutes={task.estimateMinutes}
                      label={`Size of task ${number}`}
                      onEstimate={(estimateMinutes) =>
                        change(where, { kind: 'estimate', task: place, estimateMinutes })
                      }
                      onNotUnderstood={(reason) => {
                        setRefusal({ place: where, reason });
                      }}
                    />
                  </span>
                  <span
                    className={styles.after}
                    title={
                      task.waitsFor.length > 0
                        ? `Waits for the tasks ${waitsForInWords(task)}`
                        : undefined
                    }
                  >
                    {task.waitsFor.length > 0 && `after ${waitsForInWords(task)}`}
                  </span>
                  <span className={styles.actions}>
                    <IconButton
                      label={
                        isShown
                          ? `Hide the details of task ${number}`
                          : `Show the details of task ${number}`
                      }
                      aria-expanded={isShown}
                      className={classNames(styles.disclosure, isShown && styles.open)}
                      onClick={() => {
                        setShownTask(isShown ? null : place);
                      }}
                    >
                      <ChevronRight />
                    </IconButton>
                    <IconButton
                      label={`Take "${task.title}" out of the proposal`}
                      onClick={() => {
                        void change(where, { kind: 'removeTask', task: place }).then((wasTaken) => {
                          // The places of the tasks after it move up, so no
                          // details stay open and nothing stays folded.
                          if (wasTaken) {
                            setShownTask(null);
                            setFolded(new Set());
                          }
                        });
                      }}
                    >
                      <X />
                    </IconButton>
                  </span>
                  {refusalAt(where)}
                  {isShown && (
                    <TaskDetails
                      tasks={proposal.tasks}
                      place={place}
                      task={task}
                      project={project}
                      board={board}
                      onChange={(proposalChange) => change(where, proposalChange)}
                    />
                  )}
                  {movesUnder(proposal.moves, place).length > 0 && (
                    <ul
                      className={styles.movesHere}
                      aria-label={`Tasks that exist moved under "${task.title}"`}
                    >
                      {movesUnder(proposal.moves, place).map((move, index) => (
                        <MoveRow
                          key={move.taskId}
                          move={move}
                          proposal={proposal}
                          project={project}
                          board={board}
                          isHere
                          // Said once, on the first of the tasks moved here.
                          isFirst={index === 0}
                          onChange={(proposalChange) =>
                            change(`move ${move.taskId}`, proposalChange)
                          }
                          refusal={refusalAt(`move ${move.taskId}`)}
                        />
                      ))}
                    </ul>
                  )}
                </li>
              );
            })}
          </ol>
        )}
        <div className={styles.composer}>
          <TaskComposer onCreate={(title) => change('new task', { kind: 'addTask', title })} />
          {refusalAt('new task')}
        </div>
      </div>

      {movesElsewhere(proposal.moves).length > 0 && (
        <div className={classNames(waitsForManager.part, styles.part)} style={appearsAt(2)}>
          <h4 className="label">Tasks that exist · moved elsewhere</h4>
          <ul className={styles.moves}>
            {movesElsewhere(proposal.moves).map((move) => (
              <MoveRow
                key={move.taskId}
                move={move}
                proposal={proposal}
                project={project}
                board={board}
                isHere={false}
                isFirst={false}
                onChange={(proposalChange) => change(`move ${move.taskId}`, proposalChange)}
                refusal={refusalAt(`move ${move.taskId}`)}
              />
            ))}
          </ul>
        </div>
      )}

      <footer className={classNames(waitsForManager.part, styles.footer)} style={appearsAt(3)}>
        <p className={styles.effect}>
          {approvalInWords(proposal, team)}
          {withoutOwner > 0 &&
            ` ${withoutOwner === 1 ? '1 task is' : `${withoutOwner.toString()} tasks are`} given to nobody.`}
        </p>
        <Button
          variant="primary"
          disabled={isApproving}
          onClick={() => {
            void approve();
          }}
        >
          Approve
        </Button>
      </footer>
    </section>
  );
}

interface TitleFieldProps {
  title: string;
  label: string;
  onRename: (title: string) => Promise<boolean>;
}

/**
 * The title of a proposed task, changed in place. Enter or leaving the
 * field saves it; Escape puts back what is stored.
 */
function TitleField({ title, label, onRename }: TitleFieldProps) {
  const [draft, setDraft] = useState(title);

  function save() {
    if (draft.trim() !== title) {
      void onRename(draft);
    }
  }

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === 'Enter') {
      event.preventDefault();
      event.currentTarget.blur();
    } else if (event.key === 'Escape') {
      event.stopPropagation();
      setDraft(title);
    }
  }

  return (
    <input
      className={classNames(styles.inline, styles.title)}
      value={draft}
      title={title}
      aria-label={label}
      onChange={(event) => {
        setDraft(event.target.value);
      }}
      onBlur={save}
      onKeyDown={handleKeyDown}
    />
  );
}

interface SizeFieldProps {
  minutes: number | null;
  label: string;
  onEstimate: (minutes: number | null) => Promise<boolean>;
  /** Says that what was typed is not a size, with how to write one. */
  onNotUnderstood: (reason: string) => void;
}

/** The size of a proposed task, typed as "90m", "2h" or "1h 30m". Empty is no size. */
function SizeField({ minutes, label, onEstimate, onNotUnderstood }: SizeFieldProps) {
  const stored = minutes === null ? '' : formatEstimate(minutes);
  const [draft, setDraft] = useState(stored);
  const parsed = parseEstimate(draft);
  const isUnderstood = parsed !== undefined;

  return (
    <input
      className={classNames(styles.inline, styles.estimate, !isUnderstood && styles.notUnderstood)}
      value={draft}
      placeholder="—"
      aria-label={label}
      aria-invalid={!isUnderstood}
      title="Write it like 90m, 2h or 1h 30m"
      onChange={(event) => {
        setDraft(event.target.value);
      }}
      onBlur={() => {
        if (parsed === undefined) {
          onNotUnderstood('A size is written like 90m, 2h or 1h 30m.');
        } else if (parsed !== minutes) {
          void onEstimate(parsed);
        }
      }}
      onKeyDown={(event) => {
        if (event.key === 'Enter') {
          event.currentTarget.blur();
        } else if (event.key === 'Escape') {
          event.stopPropagation();
          setDraft(stored);
        }
      }}
    />
  );
}

interface MoveRowProps {
  move: ProposedMove;
  proposal: Proposal;
  project: Project;
  board: readonly Task[];
  /** Whether the row stands under the task of the proposal it moves under. */
  isHere: boolean;
  /** Whether it is the first of the tasks moved under one task, which says "moves here". */
  isFirst: boolean;
  onChange: (change: ProposalChange) => Promise<boolean>;
  /** Why the latest change of this move was refused, if it was. */
  refusal: ReactNode;
}

/**
 * One task of the project that the proposal moves: its reference and its
 * title, where it goes (unless it stands under that task already), a
 * picker that sends it elsewhere, and a cross that leaves it where it is.
 */
function MoveRow({
  move,
  proposal,
  project,
  board,
  isHere,
  isFirst,
  onChange,
  refusal,
}: MoveRowProps) {
  const moved = board.find((task) => task.id === move.taskId);
  const reference = moved ? taskReference(project, moved) : 'A task that is gone';
  const target =
    move.partOfTask !== null ? board.find((task) => task.id === move.partOfTask) : undefined;
  const whereTo =
    move.partOfTask !== null
      ? target
        ? `under ${taskReference(project, target)} ${target.title}`
        : 'under a task that is gone'
      : move.partOf === null
        ? 'a task of its own'
        : `under ${(move.partOf + 1).toString()} ${proposal.tasks[move.partOf]?.title ?? ''}`;
  return (
    <li className={styles.move}>
      <CornerDownRight className={styles.moveMark} aria-hidden />
      <span className={styles.moveReference}>{reference}</span>
      <span className={styles.moveTitle} title={moved?.title}>
        {moved?.title}
      </span>
      <span className={styles.whereTo}>
        {isHere ? (isFirst ? 'moves here' : '') : `→ ${whereTo}`}
      </span>
      <span className={styles.actions}>
        <Picker
          label={`Move ${reference} under another task`}
          entries={moveChoices(proposal, project, board, move)}
          onPick={(key) => {
            void onChange({ kind: 'move', taskId: move.taskId, ...wholeOfKey(key) });
          }}
          trigger={
            <IconButton label={`Move ${reference} under another task`}>
              <ArrowRightLeft />
            </IconButton>
          }
          align="end"
        />
        <IconButton
          label={`Leave ${reference} where it is`}
          onClick={() => {
            void onChange({ kind: 'removeMove', taskId: move.taskId });
          }}
        >
          <X />
        </IconButton>
      </span>
      {refusal}
    </li>
  );
}

interface TaskDetailsProps {
  tasks: readonly ProposedTask[];
  place: number;
  task: ProposedTask;
  project: Project;
  /** The tasks of the project, which a new task can be made a part of. */
  board: readonly Task[];
  onChange: (change: ProposalChange) => Promise<boolean>;
}

/**
 * What is under a proposed task when it is opened: what the Advisor wrote
 * about it, its priority, the tasks of the plan it waits for, the task it
 * is a part of, and a line to add a part to it.
 */
function TaskDetails({ tasks, place, task, project, board, onChange }: TaskDetailsProps) {
  const waits = waitedFor(tasks, place);
  const choices = waitChoices(tasks, place);
  const whole = task.partOf === null ? undefined : tasks[task.partOf];
  const wholeOnBoard =
    task.partOfTask === null ? undefined : board.find((other) => other.id === task.partOfTask);
  const wholes = [
    ...partOfChoices(tasks, place).map((choice) => ({
      key: keyOfPlace(choice.place),
      reference: (choice.place + 1).toString(),
      title: choice.title,
    })),
    ...boardWholeChoices(project, board, task),
  ];

  return (
    <div className={styles.details}>
      <DescriptionField
        key={task.description}
        description={task.description}
        label={`Description of task ${(place + 1).toString()}`}
        onDescribe={(description) => onChange({ kind: 'describe', task: place, description })}
      />
      <dl className={styles.facts}>
        <dt className="label">Priority</dt>
        <dd>
          <ChoiceMenu
            label={`Priority of "${task.title}"`}
            choices={PRIORITY_CHOICES}
            value={task.priority}
            onChoose={(priority) => {
              void onChange({ kind: 'prioritize', task: place, priority });
            }}
          >
            <ValueButton className={styles.value} aria-label={`Priority of "${task.title}"`}>
              <PriorityFlag priority={task.priority} withLabel />
            </ValueButton>
          </ChoiceMenu>
        </dd>
        <dt className="label">Waits for</dt>
        <dd className={styles.waits}>
          {waits.length === 0 ? (
            <span className={styles.nothing}>Nothing. It can start at any time.</span>
          ) : (
            <ul className={styles.waitList}>
              {waits.map((wait) => (
                <li key={wait.place} className={styles.wait}>
                  <span className={styles.place}>{(wait.place + 1).toString()}</span>
                  <span className={styles.waitTitle}>{wait.title}</span>
                  <IconButton
                    label={`Stop waiting for "${wait.title}"`}
                    onClick={() => {
                      void onChange({ kind: 'removeWait', task: place, waitsFor: wait.place });
                    }}
                  >
                    <X />
                  </IconButton>
                </li>
              ))}
            </ul>
          )}
          {choices.length > 0 && (
            <Picker
              label={`Let "${task.title}" wait for a task`}
              entries={choices.map((choice) => ({
                key: choice.place.toString(),
                reference: (choice.place + 1).toString(),
                title: choice.title,
              }))}
              onPick={(key) => {
                void onChange({ kind: 'addWait', task: place, waitsFor: Number(key) });
              }}
            />
          )}
        </dd>
        <dt className="label">Part of</dt>
        <dd className={styles.waits}>
          {task.partOfTask !== null ? (
            <ul className={styles.waitList}>
              <li className={styles.wait}>
                <span className={styles.place}>
                  {wholeOnBoard ? taskReference(project, wholeOnBoard) : '—'}
                </span>
                <span className={styles.waitTitle}>
                  {wholeOnBoard?.title ?? 'A task that is gone'}
                </span>
                <IconButton
                  label={`Make "${task.title}" a task of its own`}
                  onClick={() => {
                    void onChange({ kind: 'makePartOf', task: place, partOf: null });
                  }}
                >
                  <X />
                </IconButton>
              </li>
            </ul>
          ) : whole === undefined || task.partOf === null ? (
            <span className={styles.nothing}>Nothing. It is a task of its own.</span>
          ) : (
            <ul className={styles.waitList}>
              <li className={styles.wait}>
                <span className={styles.place}>{(task.partOf + 1).toString()}</span>
                <span className={styles.waitTitle}>{whole.title}</span>
                <IconButton
                  label={`Make "${task.title}" a task of its own`}
                  onClick={() => {
                    void onChange({ kind: 'makePartOf', task: place, partOf: null });
                  }}
                >
                  <X />
                </IconButton>
              </li>
            </ul>
          )}
          {wholes.length > 0 && (
            <Picker
              label={
                whole === undefined && wholeOnBoard === undefined
                  ? `Make "${task.title}" a part of a task`
                  : `Move "${task.title}" to another task`
              }
              entries={wholes}
              onPick={(key) => {
                void onChange({ kind: 'makePartOf', task: place, ...wholeOfKey(key) });
              }}
            />
          )}
        </dd>
        <dt className="label">Parts</dt>
        <dd className={styles.parts}>
          <TaskComposer
            label="Add a part"
            onCreate={(title) => onChange({ kind: 'addTask', title, partOf: place })}
          />
        </dd>
      </dl>
    </div>
  );
}

interface DescriptionFieldProps {
  description: string;
  label: string;
  onDescribe: (description: string) => Promise<boolean>;
}

/** What the result of a proposed task is. It is saved when the field loses the focus. */
function DescriptionField({ description, label, onDescribe }: DescriptionFieldProps) {
  const [draft, setDraft] = useState(description);

  return (
    <TextArea
      className={styles.description}
      value={draft}
      rows={3}
      placeholder="What the result is, and how to tell that it is done…"
      aria-label={label}
      onChange={(event) => {
        setDraft(event.target.value);
      }}
      onBlur={() => {
        if (draft !== description) {
          void onDescribe(draft);
        }
      }}
      onKeyDown={(event) => {
        if (event.key === 'Escape') {
          event.stopPropagation();
          setDraft(description);
        }
      }}
    />
  );
}
