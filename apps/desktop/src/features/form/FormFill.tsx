import { useQueryClient } from '@tanstack/react-query';
import { useId, useState, type FormEvent, type ReactNode } from 'react';

import { useNavigation } from '@/app/navigation';
import type { Assignee, Id, Priority, Project, Task } from '@/shared/api/types';
import { taskReference } from '@/shared/lib/format';
import { Button } from '@/shared/ui/Button';
import { DateField } from '@/shared/ui/DateField';
import { ChoiceInput, TextArea, TextInput } from '@/shared/ui/Field';
import { ChoiceMenu, type Choice } from '@/shared/ui/Menu';

import { useSprintCommands } from '../sprints/sprintQueries';
import {
  AssigneeAvatar,
  AssigneeName,
  MANAGER_NAME,
  MemberAvatar,
  NOBODY_NAME,
} from '../team/Assignee';
import { useTeamMembers } from '../team/TeamContext';
import { PRIORITIES, PriorityFlag, priorityLabel } from '../tasks/TaskBadges';
import { tasksKey, useTaskCommands } from '../tasks/taskQueries';
import {
  EMPTY_VALUES,
  problemsOf,
  requestOf,
  type FieldProperty,
  type FormField,
  type FormValues,
  type TaskForm,
} from './formModel';
import styles from './FormView.module.css';

/** What the last press on "Send" came to. */
type Outcome =
  | { kind: 'made'; taskId: Id; reference: string; title: string; pointsNotSet: boolean }
  | { kind: 'refused' };

/** The value of the assignee menu that stands for nobody, and the one for the manager. */
const NOBODY = 'nobody';
const MANAGER = 'manager';

/**
 * The form as it is filled in: one column, the labels above the fields,
 * "Send" at its foot. What is required and missing, or cannot be read, is
 * said at its field when "Send" is pressed. A task that was made is named
 * in a quiet line that opens it, and the form is empty again for the next.
 */
export function FormFill({ project, form }: { project: Project; form: TaskForm }) {
  const commands = useTaskCommands(project.id);
  const sprintCommands = useSprintCommands(project.id);
  const queryClient = useQueryClient();
  const navigation = useNavigation();
  const [values, setValues] = useState<FormValues>(EMPTY_VALUES);
  const [problems, setProblems] = useState<Map<FieldProperty, string>>(new Map());
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  const [isSending, setIsSending] = useState(false);
  // A new round starts the fields again from nothing after a task is made.
  const [round, setRound] = useState(0);
  const idBase = useId();
  const idOf = (property: FieldProperty) => `${idBase}-${property}`;

  function fill<Property extends keyof FormValues>(
    property: Property,
    value: FormValues[Property],
  ) {
    setValues((before) => ({ ...before, [property]: value }));
    if (problems.has(property)) {
      setProblems((before) => {
        const after = new Map(before);
        after.delete(property);
        return after;
      });
    }
  }

  async function send() {
    const found = problemsOf(form, values);
    setProblems(found);
    if (found.size > 0) {
      setOutcome(null);
      const first = form.fields.find((field) => found.has(field.property));
      if (first) {
        focusField(idOf(first.property));
      }
      return;
    }
    const request = requestOf(form, values);
    setIsSending(true);
    const made = await commands.create(request.task);
    if (made === undefined) {
      // The reason has been shown as a message; what was filled in stays.
      setIsSending(false);
      setOutcome({ kind: 'refused' });
      return;
    }
    let pointsNotSet = false;
    if (request.points !== null) {
      await sprintCommands.setPoints(made.id, request.points);
      // A refusal has been shown as a message; the tasks on screen say
      // whether the points were set.
      const stored = queryClient
        .getQueryData<Task[]>(tasksKey(project.id))
        ?.find((task) => task.id === made.id);
      pointsNotSet = stored?.points !== request.points;
    }
    setIsSending(false);
    setOutcome({
      kind: 'made',
      taskId: made.id,
      reference: taskReference(project, made),
      title: made.title,
      pointsNotSet,
    });
    setValues(EMPTY_VALUES);
    setProblems(new Map());
    setRound((count) => count + 1);
    // The fields of the next round are drawn first, then the first takes the keyboard.
    window.requestAnimationFrame(() => {
      focusField(idOf(form.fields[0]?.property ?? 'title'));
    });
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    if (!isSending) {
      void send();
    }
  }

  return (
    <form className={styles.sheet} onSubmit={handleSubmit} noValidate aria-label={form.title}>
      <header className={styles.head}>
        <h2 className={styles.title}>{form.title}</h2>
        <p className={styles.purpose}>What is sent here becomes a task in {project.name}.</p>
        {form.introduction !== '' && <p className={styles.introduction}>{form.introduction}</p>}
      </header>

      <div className={styles.fields}>
        {form.fields.map((field) => (
          <FormRow
            key={`${field.property}:${round.toString()}`}
            field={field}
            id={idOf(field.property)}
            problem={problems.get(field.property)}
          >
            {(describedBy) => (
              <FieldControl
                field={field}
                id={idOf(field.property)}
                describedBy={describedBy}
                isInvalid={problems.has(field.property)}
                values={values}
                onFill={fill}
              />
            )}
          </FormRow>
        ))}
      </div>

      <footer className={styles.foot}>
        <p className={styles.outcome} role="status">
          {outcome?.kind === 'made' && (
            <>
              <span className={styles.outcomeWord}>
                {outcome.pointsNotSet ? 'Made, without its points:' : 'Made'}
              </span>
              <button
                type="button"
                className={styles.madeTask}
                onClick={() => {
                  navigation.openTask(outcome.taskId);
                }}
              >
                <span className={styles.madeReference}>{outcome.reference}</span>
                <span className={styles.madeTitle}>{outcome.title}</span>
              </button>
              {outcome.pointsNotSet && (
                <span className={styles.outcomeWord}>Open it to set them.</span>
              )}
            </>
          )}
          {outcome?.kind === 'refused' && (
            <span className={styles.outcomeRefused}>
              No task was made. What you filled in is still here.
            </span>
          )}
        </p>
        <Button type="submit" variant="primary" disabled={isSending}>
          Send
        </Button>
      </footer>
    </form>
  );
}

/** Gives the keyboard to a field: its input, or the button of a date. */
function focusField(id: string) {
  const holder = document.getElementById(id);
  const target = holder?.matches('input, textarea, button')
    ? holder
    : holder?.querySelector<HTMLElement>('button');
  target?.focus();
}

interface FormRowProps {
  field: FormField;
  id: string;
  problem: string | undefined;
  /** The control, given the id of the sentence that says what is wrong with it. */
  children: (describedBy: string | undefined) => ReactNode;
}

/** One field: its label above, the control, and under it what is wrong, if anything. */
function FormRow({ field, id, problem, children }: FormRowProps) {
  const problemId = `${id}-problem`;
  return (
    <div className={styles.row}>
      <div className={styles.labelLine}>
        <label className={styles.fieldLabel} htmlFor={id} id={`${id}-label`}>
          {field.label}
        </label>
        {field.required && <span className={styles.required}>Required</span>}
      </div>
      {children(problem === undefined ? undefined : problemId)}
      {problem !== undefined && (
        <p id={problemId} className={styles.problem}>
          {problem}
        </p>
      )}
    </div>
  );
}

interface FieldControlProps {
  field: FormField;
  id: string;
  describedBy: string | undefined;
  isInvalid: boolean;
  values: FormValues;
  onFill: <Property extends keyof FormValues>(
    property: Property,
    value: FormValues[Property],
  ) => void;
}

/** The program's own control for the property a field fills. */
function FieldControl({ field, id, describedBy, isInvalid, values, onFill }: FieldControlProps) {
  const shared = {
    id,
    'aria-describedby': describedBy,
    'aria-invalid': isInvalid || undefined,
    'aria-required': field.required || undefined,
    'data-invalid': isInvalid || undefined,
  };

  switch (field.property) {
    case 'title':
      return (
        <TextInput
          {...shared}
          className={styles.input}
          value={values.title}
          maxLength={200}
          autoComplete="off"
          placeholder="What needs to be done?"
          onChange={(event) => {
            onFill('title', event.target.value);
          }}
        />
      );
    case 'description':
      return (
        <TextArea
          {...shared}
          className={styles.input}
          value={values.description}
          rows={4}
          onChange={(event) => {
            onFill('description', event.target.value);
          }}
        />
      );
    case 'points':
      return (
        <TextInput
          {...shared}
          className={`${styles.input} ${styles.figure}`}
          value={values.points}
          inputMode="numeric"
          maxLength={3}
          autoComplete="off"
          placeholder="None"
          title="Story points: a whole number"
          onChange={(event) => {
            onFill('points', event.target.value);
          }}
        />
      );
    case 'estimate':
      return (
        <TextInput
          {...shared}
          className={`${styles.input} ${styles.figure}`}
          value={values.estimate}
          autoComplete="off"
          placeholder="None"
          title="Write it like 90m, 2h or 1h 30m"
          onChange={(event) => {
            onFill('estimate', event.target.value);
          }}
        />
      );
    case 'assignee':
      return (
        <AssigneeChoice
          {...shared}
          label={field.label}
          value={values.assignee}
          onChoose={(assignee) => {
            onFill('assignee', assignee);
          }}
        />
      );
    case 'priority':
      return (
        <PriorityChoice
          {...shared}
          label={field.label}
          value={values.priority}
          onChoose={(priority) => {
            onFill('priority', priority);
          }}
        />
      );
    case 'startOn':
    case 'dueOn': {
      const property = field.property;
      return (
        <span
          {...shared}
          className={styles.dateBox}
          tabIndex={-1}
          aria-labelledby={`${id}-label`}
          role="group"
        >
          <DateField
            label={field.label}
            value={values[property]}
            earliest={property === 'dueOn' ? values.startOn : null}
            latest={property === 'startOn' ? values.dueOn : null}
            onChange={(date) => {
              onFill(property, date);
            }}
          />
        </span>
      );
    }
  }
}

interface ChoiceProps<Value> {
  id: string;
  label: string;
  value: Value | null;
  onChoose: (value: Value | null) => void;
  'aria-describedby': string | undefined;
  'aria-invalid': true | undefined;
  'aria-required': true | undefined;
  'data-invalid': true | undefined;
}

/** Who the task is given to: nobody, the manager, or a member of the team. */
function AssigneeChoice({ label, value, onChoose, ...control }: ChoiceProps<Assignee>) {
  const team = useTeamMembers();
  const choices: Choice<string>[] = [
    { value: NOBODY, label: NOBODY_NAME, icon: <AssigneeAvatar assignee={null} /> },
    {
      value: MANAGER,
      label: MANAGER_NAME,
      icon: <AssigneeAvatar assignee={{ kind: 'manager' }} />,
    },
    ...team.map((member) => ({
      value: member.id,
      label: member.name,
      icon: <MemberAvatar member={member} />,
    })),
  ];
  const chosen = value === null ? NOBODY : value.kind === 'manager' ? MANAGER : value.memberId;

  return (
    <ChoiceMenu
      label={label}
      choices={choices}
      value={chosen}
      onChoose={(picked) => {
        onChoose(
          picked === NOBODY
            ? null
            : picked === MANAGER
              ? { kind: 'manager' }
              : { kind: 'member', memberId: picked },
        );
      }}
    >
      <ChoiceInput {...control} className={styles.input} isEmpty={value === null}>
        <span className={styles.chosenValue}>
          <AssigneeAvatar assignee={value} />
          <AssigneeName assignee={value} />
        </span>
      </ChoiceInput>
    </ChoiceMenu>
  );
}

/** How much the task matters. Left alone, a task is of normal priority. */
function PriorityChoice({ label, value, onChoose, ...control }: ChoiceProps<Priority>) {
  const choices: Choice<Priority>[] = PRIORITIES.map((priority) => ({
    value: priority,
    label: priorityLabel(priority),
    icon: <PriorityFlag priority={priority} />,
  }));

  return (
    <ChoiceMenu
      label={label}
      choices={choices}
      value={value ?? ('' as Priority)}
      onChoose={onChoose}
    >
      <ChoiceInput {...control} className={styles.input} isEmpty={value === null}>
        {value === null ? (
          'Normal unless chosen'
        ) : (
          <span className={styles.chosenValue}>
            <PriorityFlag priority={value} withLabel />
          </span>
        )}
      </ChoiceInput>
    </ChoiceMenu>
  );
}
