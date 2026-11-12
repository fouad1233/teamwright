import { useState, type FormEvent, type ReactNode } from 'react';

import type { CalendarDate, Id, Sprint } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { Button } from '@/shared/ui/Button';
import { DateField } from '@/shared/ui/DateField';
import { Dialog } from '@/shared/ui/Dialog';
import { ChoiceInput, Field, TextArea, TextInput } from '@/shared/ui/Field';
import { Form, FormActions, FormNote, Refusal } from '@/shared/ui/Form';
import { ChoiceMenu } from '@/shared/ui/Menu';
import { describeFailure } from '@/shared/ui/Toast';

import styles from './SprintDialogs.module.css';
import { SPRINT_LENGTHS_IN_WEEKS, endAfterWeeks, formatWork, suggestedDates } from './sprintModel';
import type { SprintCommands } from './sprintQueries';

/** The most characters the name of a sprint can have, as the backend says. */
const NAME_MAX_LENGTH = 60;

/** The value of the menu of destinations that stands for the backlog. */
const BACKLOG = 'backlog';

interface SprintDialogProps {
  open: boolean;
  commands: SprintCommands;
  /** The sprint to change. Without one, the dialog plans a new sprint. */
  sprint?: Sprint;
  onClose: () => void;
}

/** The dialog in which a sprint is planned, or changed: its name, its goal and its dates. */
export function SprintDialog({ open, commands, sprint, onClose }: SprintDialogProps) {
  return (
    <Dialog open={open} title={sprint ? `Change ${sprint.name}` : 'New sprint'} onClose={onClose}>
      <SprintForm commands={commands} sprint={sprint} onDone={onClose} />
    </Dialog>
  );
}

function SprintForm({
  commands,
  sprint,
  onDone,
}: {
  commands: SprintCommands;
  sprint?: Sprint;
  onDone: () => void;
}) {
  const [name, setName] = useState(sprint?.name ?? '');
  const [goal, setGoal] = useState(sprint?.goal ?? '');
  const [startOn, setStartOn] = useState(sprint?.startOn ?? null);
  const [endOn, setEndOn] = useState(sprint?.endOn ?? null);
  const submission = useSubmission(onDone);

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    submission.submit(() =>
      sprint
        ? commands.change(sprint.id, { name, goal, startOn, endOn })
        : commands.create({ name, goal, startOn, endOn }),
    );
  }

  return (
    <Form onSubmit={handleSubmit}>
      <NameAndGoal name={name} goal={goal} onName={setName} onGoal={setGoal} isNew={!sprint} />
      <Dates startOn={startOn} endOn={endOn} onStart={setStartOn} onEnd={setEndOn} />
      {!sprint && (
        <FormNote>
          The dates can wait until the sprint starts. Drag tasks into it from the backlog.
        </FormNote>
      )}
      {submission.refusal !== null && <Refusal>{submission.refusal}</Refusal>}
      <FormActions>
        <Button variant="ghost" onClick={onDone}>
          Cancel
        </Button>
        <Button variant="primary" type="submit" disabled={submission.isBusy}>
          {sprint ? 'Save' : 'Create sprint'}
        </Button>
      </FormActions>
    </Form>
  );
}

interface StartSprintDialogProps {
  sprint: Sprint | null;
  commands: SprintCommands;
  /** Today, where the manager is. The sprint is offered to start on it. */
  today: CalendarDate;
  onClose: () => void;
}

/** The dialog in which a planned sprint is started: its name, its goal and its dates. */
export function StartSprintDialog({ sprint, commands, today, onClose }: StartSprintDialogProps) {
  return (
    <Dialog open={sprint !== null} title={`Start ${sprint?.name ?? 'sprint'}`} onClose={onClose}>
      {sprint && (
        <StartSprintForm sprint={sprint} commands={commands} today={today} onDone={onClose} />
      )}
    </Dialog>
  );
}

function StartSprintForm({
  sprint,
  commands,
  today,
  onDone,
}: {
  sprint: Sprint;
  commands: SprintCommands;
  today: CalendarDate;
  onDone: () => void;
}) {
  const suggested = suggestedDates(sprint, today);
  const [name, setName] = useState(sprint.name);
  const [goal, setGoal] = useState(sprint.goal);
  const [startOn, setStartOn] = useState<CalendarDate | null>(suggested.startOn);
  const [endOn, setEndOn] = useState<CalendarDate | null>(suggested.endOn);
  const submission = useSubmission(onDone);

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (startOn === null || endOn === null) {
      return;
    }
    submission.submit(() => commands.start(sprint.id, { name, goal, startOn, endOn }));
  }

  return (
    <Form onSubmit={handleSubmit}>
      <NameAndGoal name={name} goal={goal} onName={setName} onGoal={setGoal} isNew={false} />
      <Dates
        startOn={startOn}
        endOn={endOn}
        onStart={setStartOn}
        onEnd={setEndOn}
        lengths={
          startOn === null ? undefined : (
            <div className={styles.lengths} role="group" aria-label="Length">
              {SPRINT_LENGTHS_IN_WEEKS.map((weeks) => {
                const end = endAfterWeeks(startOn, weeks);
                return (
                  <button
                    key={weeks}
                    type="button"
                    className={classNames(styles.length, end === endOn && styles.chosenLength)}
                    aria-pressed={end === endOn}
                    onClick={() => {
                      setEndOn(end);
                    }}
                  >
                    {weeks === 1 ? '1 week' : `${weeks.toString()} weeks`}
                  </button>
                );
              })}
            </div>
          )
        }
      />
      {submission.refusal !== null && <Refusal>{submission.refusal}</Refusal>}
      <FormActions>
        <Button variant="ghost" onClick={onDone}>
          Cancel
        </Button>
        <Button
          variant="primary"
          type="submit"
          disabled={submission.isBusy || startOn === null || endOn === null}
        >
          Start sprint
        </Button>
      </FormActions>
    </Form>
  );
}

interface CompleteSprintDialogProps {
  sprint: Sprint | null;
  /** How many of the tasks of the sprint are finished, and how many it has. */
  finished: number;
  total: number;
  /** The planned sprints the unfinished tasks can go to. */
  plannedSprints: readonly Sprint[];
  commands: SprintCommands;
  onClose: () => void;
}

/** The dialog in which the active sprint is completed, and its unfinished tasks sent on. */
export function CompleteSprintDialog({
  sprint,
  finished,
  total,
  plannedSprints,
  commands,
  onClose,
}: CompleteSprintDialogProps) {
  return (
    <Dialog open={sprint !== null} title={`Complete ${sprint?.name ?? 'sprint'}`} onClose={onClose}>
      {sprint && (
        <CompleteSprintForm
          sprint={sprint}
          finished={finished}
          total={total}
          plannedSprints={plannedSprints}
          commands={commands}
          onDone={onClose}
        />
      )}
    </Dialog>
  );
}

function CompleteSprintForm({
  sprint,
  finished,
  total,
  plannedSprints,
  commands,
  onDone,
}: Omit<CompleteSprintDialogProps, 'sprint' | 'onClose'> & {
  sprint: Sprint;
  onDone: () => void;
}) {
  const [destination, setDestination] = useState<Id>(BACKLOG);
  const submission = useSubmission(onDone);
  const unfinished = total - finished;
  const choices = [
    { value: BACKLOG, label: 'The backlog' },
    ...plannedSprints.map((planned) => ({ value: planned.id, label: planned.name })),
  ];
  const chosen = choices.find((choice) => choice.value === destination);

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    submission.submit(() =>
      commands.complete(sprint.id, {
        moveUnfinishedTo: destination === BACKLOG ? null : destination,
      }),
    );
  }

  return (
    <Form onSubmit={handleSubmit}>
      <div className={styles.tally}>
        <span className={styles.tallyFigure}>
          {finished.toString()}
          <span className={styles.tallyOf}>/{total.toString()}</span>
        </span>
        <p className={styles.tallyText}>
          tasks finished. The finished ones stay in {sprint.name} as its record.
        </p>
      </div>

      {unfinished > 0 && (
        <Field label={`The ${formatWork(unfinished, 'tasks')} not finished go to`}>
          <ChoiceMenu
            label="Where the unfinished tasks go"
            choices={choices}
            value={destination}
            onChoose={setDestination}
          >
            <ChoiceInput data-initial-focus>{chosen?.label}</ChoiceInput>
          </ChoiceMenu>
        </Field>
      )}

      {submission.refusal !== null && <Refusal>{submission.refusal}</Refusal>}
      <FormActions>
        <Button variant="ghost" onClick={onDone}>
          Cancel
        </Button>
        <Button variant="primary" type="submit" disabled={submission.isBusy}>
          Complete sprint
        </Button>
      </FormActions>
    </Form>
  );
}

interface DeleteSprintDialogProps {
  sprint: Sprint | null;
  /** How many tasks are in the sprint. */
  taskCount: number;
  onDelete: (sprintId: Id) => void;
  onClose: () => void;
}

/** Asks before a planned sprint is deleted. */
export function DeleteSprintDialog({
  sprint,
  taskCount,
  onDelete,
  onClose,
}: DeleteSprintDialogProps) {
  return (
    <Dialog open={sprint !== null} title={`Delete ${sprint?.name ?? 'sprint'}?`} onClose={onClose}>
      <p className={styles.warning}>
        {taskCount === 0
          ? 'The sprint has no tasks.'
          : `Its ${formatWork(taskCount, 'tasks')} go back to the backlog.`}{' '}
        This cannot be undone.
      </p>
      <FormActions>
        <Button variant="ghost" data-initial-focus onClick={onClose}>
          Cancel
        </Button>
        <Button
          variant="danger"
          onClick={() => {
            if (sprint) {
              onDelete(sprint.id);
            }
          }}
        >
          Delete sprint
        </Button>
      </FormActions>
    </Dialog>
  );
}

interface NameAndGoalProps {
  name: string;
  goal: string;
  onName: (name: string) => void;
  onGoal: (goal: string) => void;
  /** A new sprint may be left without a name: it is then numbered. */
  isNew: boolean;
}

function NameAndGoal({ name, goal, onName, onGoal, isNew }: NameAndGoalProps) {
  return (
    <>
      <Field label="Name" hint={isNew ? 'Left empty, the sprint is numbered.' : undefined}>
        <TextInput
          value={name}
          onChange={(event) => {
            onName(event.target.value);
          }}
          maxLength={NAME_MAX_LENGTH}
          required={!isNew}
          data-initial-focus
        />
      </Field>
      <Field label="Goal">
        <TextArea
          value={goal}
          onChange={(event) => {
            onGoal(event.target.value);
          }}
          placeholder="What the sprint is for, in a sentence."
          rows={2}
        />
      </Field>
    </>
  );
}

interface DatesProps {
  startOn: CalendarDate | null;
  endOn: CalendarDate | null;
  onStart: (date: CalendarDate | null) => void;
  onEnd: (date: CalendarDate | null) => void;
  /** Ways to set the length in one step, shown under the dates. */
  lengths?: ReactNode;
}

/** The first and the last day of a sprint, side by side, ruled like a title block. */
function Dates({ startOn, endOn, onStart, onEnd, lengths }: DatesProps) {
  return (
    <fieldset className={styles.dates}>
      <legend className={classNames('label', styles.legend)}>Dates</legend>
      <div className={styles.dateCells}>
        <div className={styles.dateCell}>
          <span className={classNames('label', styles.dateName)}>Start</span>
          <DateField label="Start date" value={startOn} latest={endOn} onChange={onStart} />
        </div>
        <div className={styles.dateCell}>
          <span className={classNames('label', styles.dateName)}>End</span>
          <DateField label="End date" value={endOn} earliest={startOn} onChange={onEnd} />
        </div>
      </div>
      {lengths}
    </fieldset>
  );
}

/** What a dialog does while it sends a request: whether it waits, and why it was refused. */
function useSubmission(onDone: () => void) {
  const [refusal, setRefusal] = useState<string | null>(null);
  const [isBusy, setIsBusy] = useState(false);
  return {
    refusal,
    isBusy,
    submit: (request: () => Promise<unknown>) => {
      setIsBusy(true);
      setRefusal(null);
      request().then(onDone, (failure: unknown) => {
        setRefusal(describeFailure(failure));
        setIsBusy(false);
      });
    },
  };
}
