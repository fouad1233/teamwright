import { BackendError } from '../backend';
import type {
  BudgetQuestion,
  BudgetScope,
  BudgetScopeKind,
  Conversation,
  Coordinator,
  CoordinatorEntry,
  CoordinatorEntryKind,
  CoordinatorSpending,
  Id,
  Member,
  Moment,
  Priority,
  Project,
  StatusCategory,
  Task,
  TeamMessage,
  WaitReason,
} from '../types';

/**
 * The coordinator of the demo: the code that starts work when it is ready
 * (decision 0037).
 *
 * The real rule is `next_steps` in `coordinator.rs` of the application
 * crate, and the words of its record are its module `words`. This is the
 * demo's copy of both, rule for rule and word for word, so that a visitor
 * who switches the coordinator on sees what the program would do. The
 * tests of the rule are in Rust; `demoCoordinator.test.ts` checks that the
 * copy follows it.
 */

/** How many turns for messages between teammates the coordinator gives in an hour of a project. */
export const MESSAGE_TURNS_PER_HOUR = 4;
/** The limit a project starts with, and its range. */
export const DEFAULT_LIMIT = 2;
export const LEAST_LIMIT = 1;
export const MOST_LIMIT = 8;
/** The smallest and the largest budget, in tokens. */
export const LEAST_BUDGET = 1_000;
export const MOST_BUDGET = 1_000_000_000_000;

/** What a specialist is told in a turn the coordinator gives it to answer its teammates. */
export const WAKE_REQUEST =
  'Teammates asked you something; their messages follow. Answer each question with send_message, in a few sentences, and then end your turn. Start no other work in this turn.';

/** A budget as the demo keeps it. */
export interface KeptBudget {
  scope: BudgetScopeKind;
  targetId: Id;
  /** Null once the manager took it away. */
  tokens: number | null;
  question: {
    askedAt: Moment;
    limit: number;
    used: number;
    answer: { raisedBy: number | null; at: Moment } | null;
  } | null;
}

/** A scope of a budget, as one value that can be compared. */
export function scopeKey(scope: BudgetScope): string {
  return `${scope.scope}:${scope.targetId}`;
}

/** Whether `used` tokens use a budget up. A budget without tokens never is. */
export function isUsedUp(budget: KeptBudget, used: number): boolean {
  return budget.tokens !== null && used >= budget.tokens;
}

/** The question that waits for the manager: asked about the budget as it stands, not answered. */
export function openQuestion(budget: KeptBudget): KeptBudget['question'] {
  const question = budget.question;
  return question && question.answer === null && question.limit === budget.tokens ? question : null;
}

/** Whether the manager is to be asked about a budget now: once for each budget as it stands. */
export function needsQuestion(budget: KeptBudget, used: number): boolean {
  return isUsedUp(budget, used) && budget.question?.limit !== budget.tokens;
}

/** A number rounded down to two significant digits, at least a thousand. */
function roundToTwoDigits(count: number): number {
  let unit = 1;
  while (Math.floor(count / unit) >= 100) {
    unit *= 10;
  }
  return Math.max(Math.floor(count / unit) * unit, LEAST_BUDGET);
}

/** The two raises offered when a budget is used up: half of it, and all of it again. */
export function raisesOf(tokens: number): [number, number] {
  return [roundToTwoDigits(Math.floor(tokens / 2)), roundToTwoDigits(tokens)];
}

/** What a project spent, by the turns that have ended, in tokens in all. */
export function spendingOf(conversations: readonly Conversation[]): CoordinatorSpending {
  let project = 0;
  const members = new Map<Id, number>();
  const tasks = new Map<Id, number>();
  for (const conversation of conversations) {
    const spent = conversation.turns
      .filter((turn) => turn.state !== 'running')
      .reduce((sum, turn) => {
        const tokens = turn.tokens;
        return tokens
          ? sum + tokens.input + tokens.output + tokens.cacheRead + tokens.cacheWritten
          : sum;
      }, 0);
    project += spent;
    if (conversation.isWithAdvisor) {
      continue;
    }
    if (conversation.memberId !== null) {
      members.set(conversation.memberId, (members.get(conversation.memberId) ?? 0) + spent);
    }
    if (conversation.taskId !== null) {
      tasks.set(conversation.taskId, (tasks.get(conversation.taskId) ?? 0) + spent);
    }
  }
  const listed = (spent: Map<Id, number>) =>
    [...spent.entries()]
      .sort(([first], [second]) => first.localeCompare(second))
      .map(([id, tokens]) => ({ id, tokens }));
  return { project, members: listed(members), tasks: listed(tasks) };
}

/** What was spent against a budget of a scope. */
export function spentOn(spending: CoordinatorSpending, scope: BudgetScope): number {
  if (scope.scope === 'project') {
    return spending.project;
  }
  const list = scope.scope === 'member' ? spending.members : spending.tasks;
  return list.find((spent) => spent.id === scope.targetId)?.tokens ?? 0;
}

/** The state of a project, as far as the coordinator needs it. */
export interface Situation {
  project: Project;
  tasks: readonly Task[];
  members: readonly Member[];
  conversations: readonly Conversation[];
  messages: readonly TeamMessage[];
  budgets: readonly KeptBudget[];
  limit: number;
  messageTurnsInTheLastHour: number;
}

export interface Start {
  taskId: Id;
  memberId: Id;
  /** The tasks it waited for, by their numbers, all done now. */
  after: Id[];
}

export interface Wake {
  memberId: Id;
  /** Who asked it something, the first to ask first. */
  askedBy: Id[];
}

export interface Waiting {
  taskId: Id | null;
  memberId: Id;
  reason: WaitReason;
  /** The scope of a budget that holds it, for `budget`. */
  budget: BudgetScope | null;
  /** The task that waits to be accepted, for `waits_for_check`. */
  checkTaskId: Id | null;
}

export interface Plan {
  starts: Start[];
  wakes: Wake[];
  questions: BudgetScope[];
  waiting: Waiting[];
}

const PRIORITY_ORDER: Record<Priority, number> = { urgent: 0, high: 1, normal: 2, low: 3 };

/**
 * What the coordinator should do now in a project. The rule of
 * `next_steps`: a task starts when it is to do or under way, given to an
 * AI specialist, never worked on, everything it waits for and every part of
 * it is done, its specialist is free, no budget it counts against is used
 * up, and fewer specialists than the limit are at work. A failed task waits
 * for the manager. Then a free specialist that was asked a question gets a
 * turn to answer it, a few times an hour at most. Every used-up budget is
 * asked about, once. The category of a status counts, never its name.
 */
export function nextSteps(situation: Situation): Plan {
  const { project } = situation;
  const spending = spendingOf(situation.conversations);
  const statusPlace = new Map(project.statuses.map((status, place) => [status.id, place]));
  const categoryOf = (task: Task): StatusCategory | undefined =>
    project.statuses.find((status) => status.id === task.statusId)?.category;
  const tasksById = new Map(situation.tasks.map((task) => [task.id, task]));
  const specialists = new Set(
    situation.members.filter((member) => member.kind === 'agent').map((member) => member.id),
  );

  const atWork = new Set<Id>();
  const waitingForManager = new Set<Id>();
  const workedOn = new Set<Id>();
  const failed = new Set<Id>();
  for (const conversation of situation.conversations) {
    const last = conversation.turns.at(-1);
    if (conversation.taskId !== null) {
      workedOn.add(conversation.taskId);
      if (last?.state === 'failed') {
        failed.add(conversation.taskId);
      }
    }
    if (conversation.memberId === null || conversation.isWithAdvisor) {
      continue;
    }
    if (conversation.turns.some((turn) => turn.state === 'running')) {
      atWork.add(conversation.memberId);
    }
    if (last && last.state !== 'running' && last.decision && last.decision.answer === null) {
      waitingForManager.add(conversation.memberId);
    }
  }

  const usedUp = (scope: BudgetScope) =>
    situation.budgets.some(
      (budget) =>
        scopeKey(budget) === scopeKey(scope) && isUsedUp(budget, spentOn(spending, scope)),
    );
  let freePlaces = Math.max(situation.limit - atWork.size, 0);
  const given = new Set<Id>();
  const plan: Plan = { starts: [], wakes: [], questions: [], waiting: [] };

  const reasonFor = (
    memberId: Id,
    scopes: readonly BudgetScope[],
  ): Pick<Waiting, 'reason' | 'budget'> | undefined => {
    const budget = scopes.find(usedUp);
    if (budget) {
      return { reason: 'budget', budget };
    }
    if (atWork.has(memberId) || given.has(memberId)) {
      return { reason: 'busy', budget: null };
    }
    if (waitingForManager.has(memberId)) {
      return { reason: 'waits_for_manager', budget: null };
    }
    return freePlaces === 0 ? { reason: 'limit', budget: null } : undefined;
  };

  const candidates = situation.tasks
    .filter((task) => ['todo', 'active'].includes(categoryOf(task) ?? 'done'))
    .filter((task) => task.assignee?.kind === 'member' && specialists.has(task.assignee.memberId))
    .sort(
      (first, second) =>
        PRIORITY_ORDER[first.priority] - PRIORITY_ORDER[second.priority] ||
        (statusPlace.get(first.statusId) ?? Infinity) -
          (statusPlace.get(second.statusId) ?? Infinity) ||
        first.rank - second.rank ||
        first.number - second.number,
    );

  for (const task of candidates) {
    if (task.assignee?.kind !== 'member') {
      continue;
    }
    const memberId = task.assignee.memberId;
    if (workedOn.has(task.id)) {
      if (failed.has(task.id)) {
        plan.waiting.push({
          taskId: task.id,
          memberId,
          reason: 'failed',
          budget: null,
          checkTaskId: null,
        });
      }
      continue;
    }
    const waitsFor = [
      ...task.blockedBy.flatMap((id) => {
        const blocker = tasksById.get(id);
        return blocker ? [blocker] : [];
      }),
      ...situation.tasks.filter((other) => other.parentId === task.id),
    ].sort((first, second) => first.number - second.number);
    const notDone = waitsFor.filter((other) => categoryOf(other) !== 'done');
    if (notDone.length > 0) {
      const checked = notDone.find((other) => categoryOf(other) === 'review');
      if (checked && notDone.every((other) => categoryOf(other) === 'review')) {
        plan.waiting.push({
          taskId: task.id,
          memberId,
          reason: 'waits_for_check',
          budget: null,
          checkTaskId: checked.id,
        });
      }
      continue;
    }
    const held = reasonFor(memberId, [
      { scope: 'project', targetId: project.id },
      { scope: 'member', targetId: memberId },
      { scope: 'task', targetId: task.id },
    ]);
    if (held) {
      plan.waiting.push({ taskId: task.id, memberId, ...held, checkTaskId: null });
      continue;
    }
    freePlaces -= 1;
    given.add(memberId);
    plan.starts.push({ taskId: task.id, memberId, after: waitsFor.map((other) => other.id) });
  }

  const asked = new Map<Id, TeamMessage[]>();
  for (const message of situation.messages) {
    if (
      message.deliveredTurnId === null &&
      message.kind === 'question' &&
      specialists.has(message.toId)
    ) {
      asked.set(message.toId, [...(asked.get(message.toId) ?? []), message]);
    }
  }
  const byMoment = (first: TeamMessage, second: TeamMessage) =>
    first.sentAt.localeCompare(second.sentAt) || first.id.localeCompare(second.id);
  const askedInOrder = [...asked.entries()]
    .map(([memberId, messages]) => ({ memberId, messages: [...messages].sort(byMoment) }))
    .sort((first, second) => {
      const [a, b] = [first.messages[0], second.messages[0]];
      return a && b ? byMoment(a, b) : 0;
    });
  let messageTurns = situation.messageTurnsInTheLastHour;
  for (const { memberId, messages } of askedInOrder) {
    if (given.has(memberId)) {
      continue;
    }
    const held =
      reasonFor(memberId, [
        { scope: 'project', targetId: project.id },
        { scope: 'member', targetId: memberId },
      ]) ??
      (messageTurns >= MESSAGE_TURNS_PER_HOUR
        ? { reason: 'message_limit' as const, budget: null }
        : undefined);
    if (held) {
      if (held.reason !== 'busy' && held.reason !== 'waits_for_manager') {
        plan.waiting.push({ taskId: null, memberId, ...held, checkTaskId: null });
      }
      continue;
    }
    freePlaces -= 1;
    messageTurns += 1;
    given.add(memberId);
    const askedBy: Id[] = [];
    for (const message of messages) {
      if (!askedBy.includes(message.fromId)) {
        askedBy.push(message.fromId);
      }
    }
    plan.wakes.push({ memberId, askedBy });
  }

  const order: Record<BudgetScopeKind, number> = { project: 0, member: 1, task: 2 };
  plan.questions = situation.budgets
    .filter((budget) => needsQuestion(budget, spentOn(spending, budget)))
    .map((budget) => ({ scope: budget.scope, targetId: budget.targetId }))
    .sort(
      (first, second) =>
        order[first.scope] - order[second.scope] || first.targetId.localeCompare(second.targetId),
    );
  return plan;
}

/** The sentences of the record, as the program words them (`words` in `coordinator.rs`). */
export const words = {
  memberName(members: readonly Member[], memberId: Id): string {
    return (
      members.find((member) => member.id === memberId)?.name ?? 'A specialist who left the team'
    );
  },

  taskName(project: Project, tasks: readonly Task[], taskId: Id): string {
    const task = tasks.find((candidate) => candidate.id === taskId);
    return task ? `${project.key}-${task.number.toString()}` : 'a task';
  },

  tokens(count: number): string {
    return Math.round(count)
      .toString()
      .replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  },

  joined(names: readonly string[]): string {
    if (names.length <= 1) {
      return names[0] ?? '';
    }
    return `${names.slice(0, -1).join(', ')} and ${names.at(-1) ?? ''}`;
  },

  budgetName(
    scope: BudgetScope,
    project: Project,
    tasks: readonly Task[],
    members: readonly Member[],
  ): string {
    if (scope.scope === 'project') {
      return 'the budget of the project';
    }
    return scope.scope === 'member'
      ? `the budget of ${words.memberName(members, scope.targetId)}`
      : `the budget of ${words.taskName(project, tasks, scope.targetId)}`;
  },

  started(
    project: Project,
    tasks: readonly Task[],
    members: readonly Member[],
    start: Start,
  ): string {
    const who = words.memberName(members, start.memberId);
    const what = words.taskName(project, tasks, start.taskId);
    const names = start.after.map((id) => words.taskName(project, tasks, id));
    if (names.length === 0) {
      return `${who} started on ${what}: it waits for nothing.`;
    }
    return `${who} started on ${what}: ${words.joined(names)} ${names.length === 1 ? 'is' : 'are'} done.`;
  },

  woke(members: readonly Member[], wake: Wake): string {
    const names = wake.askedBy.map((id) => words.memberName(members, id));
    return `${words.memberName(members, wake.memberId)} was given a turn to answer ${words.joined(names)}.`;
  },

  reason(
    waiting: Waiting,
    project: Project,
    tasks: readonly Task[],
    members: readonly Member[],
  ): string {
    const who = words.memberName(members, waiting.memberId);
    switch (waiting.reason) {
      case 'budget':
        return `${waiting.budget ? words.budgetName(waiting.budget, project, tasks, members) : 'a budget'} is used up`;
      case 'limit':
        return 'as many specialists as the limit allows are at work';
      case 'busy':
        return `${who} is at work on something else`;
      case 'waits_for_manager':
        return `${who} waits for your answer to a question`;
      case 'waits_for_check':
        return `${words.taskName(project, tasks, waiting.checkTaskId ?? '')} is finished and waits for you to accept it`;
      case 'failed':
        return 'its last turn failed; reply to go on';
      case 'message_limit':
        return `the specialists had ${MESSAGE_TURNS_PER_HOUR.toString()} turns for messages in the last hour`;
    }
  },

  heldByBudget(action: string, budget: string, used: number, limit: number): string {
    return `Did not ${action}: ${budget} is used up (${words.tokens(used)} of ${words.tokens(limit)} tokens). You are asked in the inbox.`;
  },

  usedUp(budget: string, used: number, limit: number): string {
    const named = budget.charAt(0).toUpperCase() + budget.slice(1);
    return `${named} is used up (${words.tokens(used)} of ${words.tokens(limit)} tokens). You are asked in the inbox.`;
  },
};

/** A new entry of the record. */
export function entryOf(
  id: Id,
  at: Moment,
  kind: CoordinatorEntryKind,
  text: string,
  about: { taskId?: Id | null; memberId?: Id | null } = {},
): CoordinatorEntry {
  return { id, at, kind, text, taskId: about.taskId ?? null, memberId: about.memberId ?? null };
}

/** What the coordinators of the demo need of the rest of the demo backend. */
export interface CoordinatorWorld {
  now(): Moment;
  newId(): Id;
  /** The project, or a `not_found` refusal. */
  project(projectId: Id): Project;
  tasks(projectId: Id): Task[];
  members(projectId: Id): Member[];
  conversations(projectId: Id): Conversation[];
  messages(projectId: Id): TeamMessage[];
  /** Starts a specialist on a task, as the manager would. Throws a refusal. */
  startWork(taskId: Id): void;
  /** Gives a specialist a turn to answer its teammates. Throws a refusal. */
  wake(memberId: Id): void;
}

/** The switch and the limit of the coordinator of one project. */
interface Switch {
  limit: number;
  /** The demo has one computer, so it runs here or nowhere. */
  isOn: boolean;
  switchedAt: Moment | null;
}

/**
 * The coordinators of the projects of the demo, with their budgets and
 * their records, kept in memory, and what they do, by the rule above. The
 * real application keeps them in its database (migration 24) and acts on
 * a thread of its own; the demo acts at once when it is nudged.
 */
export class PretendedCoordinators {
  private readonly switches = new Map<Id, Switch>();
  private readonly budgets = new Map<Id, KeptBudget[]>();
  /** The record of each project, the oldest first. */
  private readonly entries = new Map<Id, CoordinatorEntry[]>();

  constructor(private readonly world: CoordinatorWorld) {}

  private switchOf(projectId: Id): Switch {
    let kept = this.switches.get(projectId);
    if (!kept) {
      kept = { limit: DEFAULT_LIMIT, isOn: false, switchedAt: null };
      this.switches.set(projectId, kept);
    }
    return kept;
  }

  private budgetsOf(projectId: Id): KeptBudget[] {
    let kept = this.budgets.get(projectId);
    if (!kept) {
      kept = [];
      this.budgets.set(projectId, kept);
    }
    return kept;
  }

  private record(projectId: Id, added: readonly CoordinatorEntry[]): void {
    this.entries.set(projectId, [...(this.entries.get(projectId) ?? []), ...added]);
  }

  private situationOf(projectId: Id): Situation {
    const project = this.world.project(projectId);
    const anHourAgo = new Date(new Date(this.world.now()).getTime() - 60 * 60 * 1000).toISOString();
    return {
      project,
      tasks: this.world.tasks(projectId),
      members: this.world.members(projectId),
      conversations: this.world.conversations(projectId),
      messages: this.world.messages(projectId),
      budgets: this.budgetsOf(projectId),
      limit: this.switchOf(projectId).limit,
      messageTurnsInTheLastHour: (this.entries.get(projectId) ?? []).filter(
        (entry) => entry.kind === 'woke' && entry.at >= anHourAgo,
      ).length,
    };
  }

  /** Whether the coordinator of a project is on. */
  isOn(projectId: Id): boolean {
    return this.switchOf(projectId).isOn;
  }

  /** The coordinator of a project as the interface sees it. */
  stateOf(projectId: Id): Coordinator {
    const situation = this.situationOf(projectId);
    const { project, tasks, members } = situation;
    const kept = this.switchOf(projectId);
    const spending = spendingOf(situation.conversations);
    const plan = nextSteps(situation);
    const atWork = new Set(
      situation.conversations
        .filter(
          (conversation) =>
            !conversation.isWithAdvisor &&
            conversation.turns.some((turn) => turn.state === 'running'),
        )
        .flatMap((conversation) => (conversation.memberId === null ? [] : [conversation.memberId])),
    );
    return {
      projectId,
      runs: kept.isOn ? 'here' : 'off',
      runsOnName: null,
      switchedAt: kept.switchedAt,
      limit: kept.limit,
      leastLimit: LEAST_LIMIT,
      mostLimit: MOST_LIMIT,
      atWork: atWork.size,
      budgets: this.budgetsOf(projectId).flatMap((budget) =>
        budget.tokens === null
          ? []
          : [
              {
                scope: budget.scope,
                targetId: budget.targetId,
                tokens: budget.tokens,
                used: spentOn(spending, budget),
                askedAt: openQuestion(budget)?.askedAt ?? null,
                raises: raisesOf(budget.tokens),
              },
            ],
      ),
      spent: spending,
      waiting: plan.waiting.map((waiting) => ({
        taskId: waiting.taskId,
        memberId: waiting.memberId,
        reason: waiting.reason,
        checkTaskId: waiting.checkTaskId,
        words: words.reason(waiting, project, tasks, members),
      })),
      entries: [...(this.entries.get(projectId) ?? [])].reverse().slice(0, 30),
    };
  }

  switchOn(projectId: Id): void {
    const kept = this.switchOf(projectId);
    if (kept.isOn) {
      return;
    }
    kept.isOn = true;
    kept.switchedAt = this.world.now();
    const specialists = kept.limit === 1 ? 'specialist' : 'specialists';
    this.record(projectId, [
      entryOf(
        this.world.newId(),
        kept.switchedAt,
        'switched_on',
        `Switched on. Tasks start as soon as they are ready, with at most ${kept.limit.toString()} ${specialists} at work at once.`,
      ),
    ]);
  }

  switchOff(
    projectId: Id,
    why = 'Switched off. Work under way goes on; nothing new starts.',
  ): void {
    const kept = this.switchOf(projectId);
    if (!kept.isOn) {
      return;
    }
    kept.isOn = false;
    kept.switchedAt = this.world.now();
    this.record(projectId, [entryOf(this.world.newId(), kept.switchedAt, 'switched_off', why)]);
  }

  setLimit(projectId: Id, limit: number): void {
    if (!Number.isInteger(limit) || limit < LEAST_LIMIT || limit > MOST_LIMIT) {
      throw new BackendError(
        'rule_broken',
        `the coordinator lets ${LEAST_LIMIT.toString()} to ${MOST_LIMIT.toString()} specialists work at once`,
      );
    }
    this.switchOf(projectId).limit = limit;
  }

  setBudget(projectId: Id, scope: BudgetScope, tokens: number | null): void {
    const targetId = scope.scope === 'project' ? projectId : scope.targetId;
    const belongs =
      scope.scope === 'project' ||
      (scope.scope === 'member'
        ? this.world.members(projectId).some((member) => member.id === targetId)
        : this.world.tasks(projectId).some((task) => task.id === targetId));
    if (!belongs) {
      throw new BackendError('not_found', `there is no ${scope.scope} ${targetId} in the project`);
    }
    if (
      tokens !== null &&
      (!Number.isInteger(tokens) || tokens < LEAST_BUDGET || tokens > MOST_BUDGET)
    ) {
      throw new BackendError(
        'rule_broken',
        `a budget is a whole number of tokens from ${words.tokens(LEAST_BUDGET)} to ${words.tokens(MOST_BUDGET)}`,
      );
    }
    const budgets = this.budgetsOf(projectId);
    const kept = budgets.find(
      (budget) => budget.scope === scope.scope && budget.targetId === targetId,
    );
    if (kept) {
      kept.tokens = tokens;
    } else if (tokens !== null) {
      budgets.push({ scope: scope.scope, targetId, tokens, question: null });
    }
  }

  answer(projectId: Id, scope: BudgetScope, raiseBy: number | null): void {
    const targetId = scope.scope === 'project' ? projectId : scope.targetId;
    const budget = this.budgetsOf(projectId).find(
      (candidate) => candidate.scope === scope.scope && candidate.targetId === targetId,
    );
    const question = budget ? openQuestion(budget) : null;
    if (!budget || !question) {
      throw new BackendError('rule_broken', 'nothing is asked about this budget');
    }
    if (raiseBy !== null) {
      if (!Number.isInteger(raiseBy) || raiseBy <= 0) {
        throw new BackendError('rule_broken', 'a raise is a whole number of tokens');
      }
      budget.tokens = Math.min(question.limit + raiseBy, MOST_BUDGET);
    }
    question.answer = { raisedBy: raiseBy, at: this.world.now() };
  }

  /** The open questions about budgets of some projects, the oldest first. */
  questions(projectIds: readonly Id[]): BudgetQuestion[] {
    return projectIds
      .flatMap((projectId) => {
        const budgets = this.budgetsOf(projectId).filter((budget) => openQuestion(budget));
        if (budgets.length === 0) {
          return [];
        }
        const project = this.world.project(projectId);
        const tasks = this.world.tasks(projectId);
        const members = this.world.members(projectId);
        const spending = spendingOf(this.world.conversations(projectId));
        return budgets.flatMap((budget) => {
          const question = openQuestion(budget);
          return question && budget.tokens !== null
            ? [
                {
                  projectId,
                  projectName: project.name,
                  scope: budget.scope,
                  targetId: budget.targetId,
                  name: words.budgetName(budget, project, tasks, members),
                  tokens: budget.tokens,
                  used: spentOn(spending, budget),
                  askedAt: question.askedAt,
                  raises: raisesOf(budget.tokens),
                },
              ]
            : [];
        });
      })
      .sort((first, second) => first.askedAt.localeCompare(second.askedAt));
  }

  /**
   * Does what the rule says for a project, if its coordinator is on, and
   * writes down what it did. Answers with the entries it wrote.
   */
  act(projectId: Id): CoordinatorEntry[] {
    if (!this.isOn(projectId)) {
      return [];
    }
    const situation = this.situationOf(projectId);
    const { project, tasks, members } = situation;
    const plan = nextSteps(situation);
    const at = this.world.now();
    const added: CoordinatorEntry[] = [];
    const entry = (
      kind: CoordinatorEntryKind,
      text: string,
      about: { taskId?: Id | null; memberId?: Id | null } = {},
    ) => {
      added.push(entryOf(this.world.newId(), at, kind, text, about));
    };
    const refusalOf = (failure: unknown) =>
      failure instanceof Error ? failure.message : String(failure);

    for (const start of plan.starts) {
      try {
        this.world.startWork(start.taskId);
        entry('started', words.started(project, tasks, members, start), start);
      } catch (failure) {
        entry(
          'held',
          `Could not start ${words.taskName(project, tasks, start.taskId)}: ${refusalOf(failure)}.`,
          { taskId: start.taskId },
        );
      }
    }
    for (const wake of plan.wakes) {
      try {
        this.world.wake(wake.memberId);
        entry('woke', words.woke(members, wake), { memberId: wake.memberId });
      } catch (failure) {
        entry(
          'held',
          `Could not give ${words.memberName(members, wake.memberId)} a turn to answer its teammates: ${refusalOf(failure)}.`,
          { memberId: wake.memberId },
        );
      }
    }

    const spending = spendingOf(situation.conversations);
    const actionOf = (waiting: Waiting) =>
      waiting.taskId === null
        ? `give ${words.memberName(members, waiting.memberId)} a turn to answer its teammates`
        : `start ${words.taskName(project, tasks, waiting.taskId)}`;
    for (const scope of plan.questions) {
      const budget = this.budgetsOf(projectId).find(
        (candidate) => scopeKey(candidate) === scopeKey(scope),
      );
      if (!budget || budget.tokens === null) {
        continue;
      }
      const used = spentOn(spending, scope);
      budget.question = { askedAt: at, limit: budget.tokens, used, answer: null };
      const held = plan.waiting.find(
        (waiting) => waiting.budget !== null && scopeKey(waiting.budget) === scopeKey(scope),
      );
      const name = words.budgetName(scope, project, tasks, members);
      entry(
        'asked',
        held
          ? words.heldByBudget(actionOf(held), name, used, budget.tokens)
          : words.usedUp(name, used, budget.tokens),
        held ?? {},
      );
    }

    const anHourAgo = new Date(new Date(at).getTime() - 60 * 60 * 1000).toISOString();
    const told = (this.entries.get(projectId) ?? [])
      .filter((kept) => kept.kind === 'held' && kept.at >= anHourAgo)
      .map((kept) => kept.text);
    for (const waiting of plan.waiting.filter(
      (candidate) => candidate.reason === 'message_limit',
    )) {
      const text = `Did not ${actionOf(waiting)}: ${words.reason(waiting, project, tasks, members)}. The question waits for its next turn.`;
      if (!told.includes(text)) {
        entry('held', text, { memberId: waiting.memberId });
      }
    }
    this.record(projectId, added);
    return added;
  }

  /** Forgets everything of a project that is deleted. */
  forgetProject(projectId: Id): void {
    this.switches.delete(projectId);
    this.budgets.delete(projectId);
    this.entries.delete(projectId);
  }
}
