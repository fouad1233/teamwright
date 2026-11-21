import type {
  Area,
  CalendarDate,
  Conversation,
  DueOutcome,
  Handoff,
  Member,
  PlanAgainstActual,
  Project,
  ProjectReport,
  ReportDecision,
  ReportTask,
  ReportUsage,
  Sprint,
  SprintOutcome,
  Task,
  UsageLine,
  WorkShare,
} from '../types';
import { dayOfMoment, summaryOf, type CountedTask } from './sprints';
import { usageOfConversations } from './usage';

/**
 * The report of a whole project, for the demo backend. It stands in for
 * `project_report.rs` of the application (decision 0042) and follows the
 * same rules, sentence for sentence, so that the report shows in a browser
 * as it would in the program.
 */

/** Everything the report is made from. */
export interface ReportSources {
  project: Project;
  tasks: readonly Task[];
  sprints: readonly Sprint[];
  /** The members, in the order they joined. */
  members: readonly Member[];
  /** The areas, in their order. */
  areas: readonly Area[];
  /** The conversations with their turns, the oldest first. */
  conversations: readonly Conversation[];
  handoffs: readonly Handoff[];
  today: CalendarDate;
}

const MANAGER = 'The manager';
const NOBODY = 'Nobody';
const ADVISOR = 'The Advisor';
const FORMER_MEMBER = 'A former member';
const FORMER_MEMBERS = 'Former members';
const MODEL_NOT_NAMED = 'Model not named';
const MILLISECONDS_PER_DAY = 24 * 60 * 60 * 1000;

export const TITLE_OF_COMPLETE = 'Report of the project';
export const TITLE_SO_FAR = 'Report of the project so far';

/** The report of a project, worked out from what the demo keeps. */
export function projectReportOf(sources: ReportSources): ProjectReport {
  const { project, today } = sources;
  const tasks = [...sources.tasks].sort((a, b) => a.number - b.number);
  const isFinished = (task: Task) =>
    project.statuses.find((status) => status.id === task.statusId)?.category === 'done';
  const memberName = (memberId: string | null) =>
    sources.members.find((member) => member.id === memberId)?.name ?? FORMER_MEMBER;
  const holderName = (task: Task) =>
    task.assignee === null
      ? NOBODY
      : task.assignee.kind === 'manager'
        ? MANAGER
        : memberName(task.assignee.memberId);
  const worked = workedMinutesByTask(sources.conversations);
  const hasAreas = sources.areas.length > 1;

  const reportTask = (task: Task): ReportTask => {
    const finished = isFinished(task);
    const finishedOn = finished && task.completedAt !== null ? dayOfMoment(task.completedAt) : null;
    return {
      taskId: task.id,
      reference: `${project.key}-${task.number.toString()}`,
      title: task.title,
      area: hasAreas ? (sources.areas.find((area) => area.id === task.areaId)?.name ?? null) : null,
      holder: holderName(task),
      status: project.statuses.find((status) => status.id === task.statusId)?.name ?? null,
      dueOn: task.dueOn,
      finishedOn,
      estimateMinutes: task.estimateMinutes,
      workedMinutes: worked.get(task.id) ?? null,
      due: dueOutcome(finished, task.dueOn, finishedOn, today),
    };
  };

  const finished = tasks
    .filter(isFinished)
    .map(reportTask)
    .sort((a, b) =>
      a.finishedOn === b.finishedOn
        ? 0
        : a.finishedOn === null
          ? 1
          : b.finishedOn === null
            ? -1
            : a.finishedOn.localeCompare(b.finishedOn),
    );
  const open = tasks.filter((task) => !isFinished(task)).map(reportTask);
  const isComplete = tasks.length > 0 && open.length === 0;

  const share = (name: string, matches: (task: Task) => boolean): WorkShare[] => {
    const own = tasks.filter(matches);
    const done = own.filter(isFinished).length;
    return own.length === 0 ? [] : [{ name, finished: done, open: own.length - done }];
  };
  const byArea = hasAreas
    ? sources.areas.flatMap((area) => share(area.name, (task) => task.areaId === area.id))
    : [];
  const byHolder = [
    ...sources.members.flatMap((member) =>
      share(
        member.name,
        (task) => task.assignee?.kind === 'member' && task.assignee.memberId === member.id,
      ),
    ),
    ...share(
      FORMER_MEMBER,
      (task) =>
        task.assignee?.kind === 'member' &&
        !sources.members.some(
          (member) => task.assignee?.kind === 'member' && member.id === task.assignee.memberId,
        ),
    ),
    ...share(MANAGER, (task) => task.assignee?.kind === 'manager'),
    ...share(NOBODY, (task) => task.assignee === null),
  ];

  const hasUntimedWork = tasks.some(
    (task) =>
      isFinished(task) &&
      (task.assignee === null ||
        task.assignee.kind === 'manager' ||
        sources.members.find(
          (member) => task.assignee?.kind === 'member' && member.id === task.assignee.memberId,
        )?.kind === 'human'),
  );
  const decisions = decisionsOf(sources, memberName);
  const waiting = waitingQuestions(sources.conversations);
  const usage = usageOf(sources, memberName);

  return {
    projectId: project.id,
    name: project.name,
    key: project.key,
    description: project.description.trim(),
    madeOn: today,
    begunOn: dayOfMoment(project.createdAt),
    isComplete,
    title: isComplete ? TITLE_OF_COMPLETE : TITLE_SO_FAR,
    summary: reportSummaryOf({
      begunOn: dayOfMoment(project.createdAt),
      finished,
      open: open.length,
      usage,
      decisions: decisions.length,
      waiting,
    }),
    brief: briefOf(sources.conversations),
    byArea,
    byHolder,
    finished,
    plan: planAgainstActual(finished, open, hasUntimedWork),
    sprints: sprintsOf(sources, isFinished),
    usage,
    decisions,
    handoffs: tasks.flatMap((task) => {
      const handoff = sources.handoffs
        .filter((each) => each.taskId === task.id)
        .sort((a, b) => a.publishedAt.localeCompare(b.publishedAt))
        .at(-1);
      return handoff
        ? [
            {
              reference: `${project.key}-${task.number.toString()}`,
              title: task.title,
              author: memberName(handoff.authorId),
              publishedOn: dayOfMoment(handoff.publishedAt),
              summary: handoff.summary,
              contract: handoff.contract,
              files: handoff.files,
            },
          ]
        : [];
    }),
    open,
    waiting,
  };
}

/** How a task stands against its due date, on `today`. */
export function dueOutcome(
  isFinished: boolean,
  dueOn: CalendarDate | null,
  finishedOn: CalendarDate | null,
  today: CalendarDate,
): DueOutcome {
  if (dueOn === null) {
    return { kind: 'none' };
  }
  const daysAfter = (day: CalendarDate) =>
    Math.max(0, Math.round((Date.parse(day) - Date.parse(dueOn)) / MILLISECONDS_PER_DAY));
  if (isFinished) {
    if (finishedOn === null) {
      return { kind: 'none' };
    }
    return finishedOn <= dueOn ? { kind: 'met' } : { kind: 'missed', days: daysAfter(finishedOn) };
  }
  return today > dueOn ? { kind: 'late', days: daysAfter(today) } : { kind: 'coming' };
}

/** How long the turns of the specialists on each task ran, in whole minutes. */
function workedMinutesByTask(conversations: readonly Conversation[]): Map<string, number> {
  const seconds = new Map<string, number>();
  for (const conversation of conversations) {
    if (conversation.taskId === null) {
      continue;
    }
    for (const turn of conversation.turns) {
      if (turn.endedAt === null) {
        continue;
      }
      const ran = Math.max(
        0,
        Math.floor((Date.parse(turn.endedAt) - Date.parse(turn.startedAt)) / 1000),
      );
      seconds.set(conversation.taskId, (seconds.get(conversation.taskId) ?? 0) + ran);
    }
  }
  return new Map([...seconds].map(([taskId, sum]) => [taskId, Math.ceil(sum / 60)]));
}

/** The questions that wait for the manager, in turns that have ended. */
function waitingQuestions(conversations: readonly Conversation[]): number {
  return conversations
    .flatMap((conversation) => conversation.turns)
    .filter(
      (turn) =>
        turn.state !== 'running' &&
        ((turn.decision !== null && turn.decision.answer === null) ||
          (turn.interview !== null && turn.interview.answers === null)),
    ).length;
}

/** The summary of the latest plan of the Advisor the manager approved. */
function briefOf(conversations: readonly Conversation[]): ProjectReport['brief'] {
  const approved = conversations
    .filter((conversation) => conversation.isWithAdvisor)
    .flatMap((conversation) => conversation.turns)
    .flatMap((turn) =>
      turn.proposal?.approvedAt && turn.proposal.summary.trim() !== ''
        ? [{ at: turn.proposal.approvedAt, summary: turn.proposal.summary.trim() }]
        : [],
    )
    .sort((a, b) => a.at.localeCompare(b.at))
    .at(-1);
  return approved ? { summary: approved.summary, approvedOn: dayOfMoment(approved.at) } : null;
}

/** The questions the manager answered, the one answered first first. */
function decisionsOf(
  sources: ReportSources,
  memberName: (memberId: string | null) => string,
): ReportDecision[] {
  const decisions: { at: string; decision: ReportDecision }[] = [];
  for (const conversation of sources.conversations) {
    const askedBy = conversation.isWithAdvisor ? ADVISOR : memberName(conversation.memberId);
    const task = sources.tasks.find((each) => each.id === conversation.taskId);
    const reference = task ? `${sources.project.key}-${task.number.toString()}` : null;
    for (const turn of conversation.turns) {
      const answer = turn.decision?.answer;
      if (turn.decision && answer) {
        decisions.push({
          at: answer.at,
          decision: {
            askedBy,
            task: reference,
            question: turn.decision.question,
            chosen:
              answer.option === null ? null : (turn.decision.options[answer.option]?.label ?? null),
            note: answer.note.trim(),
            answeredOn: dayOfMoment(answer.at),
          },
        });
      }
      const interview = turn.interview;
      if (interview?.answers && interview.answeredAt) {
        const at = interview.answeredAt;
        interview.questions.forEach((question, place) => {
          const given = interview.answers?.[place];
          if (!given) {
            return;
          }
          decisions.push({
            at,
            decision: {
              askedBy,
              task: reference,
              question: question.question,
              chosen:
                given.option === null ? null : (question.options[given.option]?.label ?? null),
              note: given.note.trim(),
              answeredOn: dayOfMoment(at),
            },
          });
        });
      }
    }
  }
  // A stable sort keeps the questions of one interview in their order.
  return decisions.sort((a, b) => a.at.localeCompare(b.at)).map(({ decision }) => decision);
}

/** What the team used, by who and by model. */
function usageOf(
  sources: ReportSources,
  memberName: (memberId: string | null) => string,
): ReportUsage {
  const usage = usageOfConversations(sources.conversations);
  const line = (name: string, spent: { allTokens: number; turns: number }): UsageLine => ({
    name,
    tokens: spent.allTokens,
    turns: spent.turns,
  });
  const byWho = usage.members.map((member) => line(memberName(member.memberId), member.usage));
  if (usage.advisor) {
    byWho.push(line(ADVISOR, usage.advisor));
  }
  if (usage.former) {
    byWho.push(line(FORMER_MEMBERS, usage.former));
  }
  const byModel = new Map<string, UsageLine>();
  for (const turn of sources.conversations.flatMap((conversation) => conversation.turns)) {
    if (turn.state === 'running') {
      continue;
    }
    const name = turn.model ?? MODEL_NOT_NAMED;
    const kept = byModel.get(name) ?? { name, tokens: 0, turns: 0 };
    const tokens = turn.tokens;
    kept.tokens += tokens
      ? tokens.input + tokens.output + tokens.cacheRead + tokens.cacheWritten
      : 0;
    kept.turns += 1;
    byModel.set(name, kept);
  }
  return {
    byWho,
    byModel: [...byModel.values()].sort(
      (a, b) => b.tokens - a.tokens || a.name.localeCompare(b.name),
    ),
    total: line('Everyone', usage.total),
  };
}

/** Every sprint, in the order it began. */
function sprintsOf(sources: ReportSources, isFinished: (task: Task) => boolean): SprintOutcome[] {
  const key = (sprint: Sprint) =>
    `${sprint.startOn === null ? '1' : '0'}${sprint.startOn ?? ''}${sprint.createdAt}${sprint.id}`;
  return [...sources.sprints]
    .sort((a, b) => key(a).localeCompare(key(b)))
    .map((sprint) => {
      const summary =
        sprint.summary ??
        summaryOf(
          sources.tasks
            .filter((task) => task.sprintId === sprint.id && task.parentId === null)
            .map((task): CountedTask => ({
              points: task.points,
              completedAt: task.completedAt,
              isFinished: isFinished(task),
            })),
        );
      return {
        name: sprint.name,
        goal: sprint.goal.trim(),
        state: sprint.state,
        startOn: sprint.startOn,
        endOn: sprint.endOn,
        unit: summary.unit,
        committed: summary.committed,
        completed: summary.completed,
      };
    });
}

/** The plan against what happened, from the finished and the open tasks. */
function planAgainstActual(
  finished: readonly ReportTask[],
  open: readonly ReportTask[],
  hasUntimedWork: boolean,
): PlanAgainstActual {
  const plan: PlanAgainstActual = {
    finished: finished.length,
    estimated: 0,
    estimatedMinutes: 0,
    worked: 0,
    workedMinutes: 0,
    compared: 0,
    comparedEstimateMinutes: 0,
    comparedWorkedMinutes: 0,
    withDueDate: 0,
    met: 0,
    missed: 0,
    late: open.filter((task) => task.due.kind === 'late').length,
    sentences: [],
  };
  for (const task of finished) {
    if (task.estimateMinutes !== null) {
      plan.estimated += 1;
      plan.estimatedMinutes += task.estimateMinutes;
    }
    if (task.workedMinutes !== null) {
      plan.worked += 1;
      plan.workedMinutes += task.workedMinutes;
    }
    if (task.estimateMinutes !== null && task.workedMinutes !== null) {
      plan.compared += 1;
      plan.comparedEstimateMinutes += task.estimateMinutes;
      plan.comparedWorkedMinutes += task.workedMinutes;
    }
    if (task.due.kind === 'met' || task.due.kind === 'missed') {
      plan.withDueDate += 1;
      if (task.due.kind === 'met') {
        plan.met += 1;
      } else {
        plan.missed += 1;
      }
    }
  }
  plan.sentences = planInWords(plan, hasUntimedWork);
  return plan;
}

function planInWords(plan: PlanAgainstActual, hasUntimedWork: boolean): string[] {
  if (plan.finished === 0) {
    return [
      'No task is finished yet, so nothing can be compared.',
      ...(plan.late > 0 ? [lateInWords(plan.late)] : []),
    ];
  }
  const sentences: string[] = [];
  sentences.push(
    plan.estimated === 0
      ? 'No finished task had an estimate.'
      : `${ofTheFinished(plan.estimated, plan.finished)} had an estimate: ${durationInWords(plan.estimatedMinutes)} together.`,
  );
  sentences.push(
    plan.worked === 0
      ? 'No specialist had a turn on a finished task, so no time was measured.'
      : `The specialists' turns on ${tasksInWords(plan.worked, 'finished task')} ran ${durationInWords(plan.workedMinutes)} together.`,
  );
  if (plan.compared > 0) {
    sentences.push(
      `On the ${tasksInWords(plan.compared, 'task')} with both, the estimates came to ${durationInWords(plan.comparedEstimateMinutes)} and the turns ran ${durationInWords(plan.comparedWorkedMinutes)}. An estimate is the time a skilled person would need; the turns are how long the specialists worked.`,
    );
  }
  if (hasUntimedWork) {
    sentences.push(
      'Teamwright does not time the work of people, so their tasks are counted and not measured.',
    );
  }
  sentences.push(
    plan.withDueDate === 0
      ? 'No finished task had a due date.'
      : plan.missed === 0
        ? `${plan.withDueDate === 1 ? 'One finished task had' : `${plan.withDueDate.toString()} finished tasks had`} a due date, and every one was finished by it.`
        : `Of the ${tasksInWords(plan.withDueDate, 'finished task')} with a due date, ${plan.met.toString()} finished by it and ${plan.missed.toString()} after it.`,
  );
  if (plan.late > 0) {
    sentences.push(lateInWords(plan.late));
  }
  return sentences;
}

function ofTheFinished(part: number, finished: number): string {
  if (part === finished) {
    return finished === 1 ? 'The finished task' : `All ${finished.toString()} finished tasks`;
  }
  return `${part.toString()} of the ${finished.toString()} finished tasks`;
}

function tasksInWords(count: number, noun: string): string {
  return count === 1 ? `1 ${noun}` : `${count.toString()} ${noun}s`;
}

function lateInWords(late: number): string {
  return late === 1
    ? '1 open task is past its due date.'
    : `${late.toString()} open tasks are past their due date.`;
}

interface SummarySources {
  begunOn: CalendarDate;
  finished: readonly ReportTask[];
  open: number;
  usage: ReportUsage;
  decisions: number;
  waiting: number;
}

function reportSummaryOf(sources: SummarySources): string[] {
  const finished = sources.finished.length;
  const { open } = sources;
  const all = finished + open;
  const sentences: string[] = [];
  const begun = `The project was begun on ${dateInWords(sources.begunOn)}`;
  if (all === 0) {
    sentences.push(`${begun}. It has no tasks yet.`);
  } else if (open === 0) {
    const last = sources.finished
      .flatMap((task) => (task.finishedOn === null ? [] : [task.finishedOn]))
      .sort()
      .at(-1);
    sentences.push(
      last === undefined
        ? `${begun}.`
        : `${begun}, and its last task was finished on ${dateInWords(last)}.`,
    );
    sentences.push(
      all === 1 ? 'Its one task is finished.' : `All ${all.toString()} of its tasks are finished.`,
    );
  } else {
    sentences.push(`${begun}.`);
    sentences.push(
      `${finished.toString()} of its ${tasksInWords(all, 'task')} ${finished === 1 ? 'is' : 'are'} finished, and ${open === 1 ? '1 is' : `${open.toString()} are`} still open.`,
    );
  }
  const { total } = sources.usage;
  if (total.turns > 0) {
    sentences.push(
      `The team took ${total.turns.toString()} ${total.turns === 1 ? 'turn' : 'turns'} and used ${numberInWords(total.tokens)} tokens.`,
    );
  }
  if (sources.decisions > 0) {
    sentences.push(
      sources.decisions === 1
        ? 'The manager answered 1 question.'
        : `The manager answered ${sources.decisions.toString()} questions.`,
    );
  }
  if (sources.waiting > 0) {
    sentences.push(
      sources.waiting === 1
        ? '1 question still waits for the manager.'
        : `${sources.waiting.toString()} questions still wait for the manager.`,
    );
  }
  return sentences;
}

const MONTHS = [
  'January',
  'February',
  'March',
  'April',
  'May',
  'June',
  'July',
  'August',
  'September',
  'October',
  'November',
  'December',
];

/** A day in words: "21 November 2026", as the application says it. */
export function dateInWords(date: CalendarDate): string {
  const [year, month, day] = date.split('-').map(Number);
  return `${(day ?? 0).toString()} ${MONTHS[(month ?? 1) - 1] ?? ''} ${(year ?? 0).toString()}`;
}

/** A length of time in words: "45 minutes", "1 hour", "12.5 hours". */
export function durationInWords(minutes: number): string {
  if (minutes < 60) {
    return minutes === 1 ? '1 minute' : `${minutes.toString()} minutes`;
  }
  const tenths = Math.floor((minutes * 10 + 30) / 60);
  const hours =
    tenths % 10 === 0
      ? Math.floor(tenths / 10).toString()
      : `${Math.floor(tenths / 10).toString()}.${(tenths % 10).toString()}`;
  return hours === '1' ? '1 hour' : `${hours} hours`;
}

/** A number with its thousands set apart: "1,234,567". */
export function numberInWords(number: number): string {
  return Math.round(number)
    .toString()
    .replace(/\B(?=(\d{3})+(?!\d))/g, ',');
}
