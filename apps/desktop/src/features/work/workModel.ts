import type { AgentProgram, Conversation, Id, Moment, TokenCounts, Turn } from '@/shared/api/types';

/**
 * Pure functions about the work of AI specialists: which conversation
 * belongs to a task, whether somebody is working, and how to put counts of
 * tokens and stretches of time into words.
 */

/** The conversation about a task, if work on it has started. */
export function conversationOfTask(
  conversations: readonly Conversation[],
  taskId: Id,
): Conversation | undefined {
  return conversations.find((conversation) => conversation.taskId === taskId);
}

/** The turn of a conversation that is running, if one is. */
export function runningTurn(conversation: Conversation): Turn | undefined {
  return conversation.turns.find((turn) => turn.state === 'running');
}

/** The identifiers of the tasks a specialist is working on at this moment. */
/**
 * Whether the program has started the session of a conversation. Only then
 * is there something to open in the Claude app. A turn learns its model at
 * the moment its session starts, so a model on any turn says that it has.
 */
export function hasSession(conversation: Conversation): boolean {
  return conversation.turns.some((turn) => turn.model !== null);
}

export function tasksBeingWorkedOn(conversations: readonly Conversation[]): Set<Id> {
  return new Set(
    conversations.flatMap((conversation) =>
      conversation.taskId !== null && runningTurn(conversation) ? [conversation.taskId] : [],
    ),
  );
}

/** The conversation in which a member is working at this moment, if it is. */
export function whatMemberIsWorkingOn(
  conversations: readonly Conversation[],
  memberId: Id,
): Conversation | undefined {
  return conversations.find(
    (conversation) => conversation.memberId === memberId && runningTurn(conversation),
  );
}

/**
 * The conversations the manager had with a member that are not about a
 * task, the oldest first.
 */
export function chatsWith(conversations: readonly Conversation[], memberId: Id): Conversation[] {
  return conversations.filter(
    (conversation) => conversation.memberId === memberId && conversation.taskId === null,
  );
}

/** A copy of the list with one conversation added, or replaced if it is already in it. */
export function withConversation(
  conversations: readonly Conversation[],
  changed: Conversation,
): Conversation[] {
  const exists = conversations.some((conversation) => conversation.id === changed.id);
  return exists
    ? conversations.map((conversation) => (conversation.id === changed.id ? changed : conversation))
    : [...conversations, changed];
}

/**
 * Tells whether a turn has started or ended between two versions of a
 * conversation. When work starts or ends, the task moves to another status,
 * so the tasks on screen have to be loaded again; a step in between leaves
 * them as they are.
 */
export function hasTurnStartedOrEnded(
  before: Conversation | undefined,
  after: Conversation,
): boolean {
  const states = (conversation: Conversation | undefined) =>
    (conversation?.turns ?? []).map((turn) => turn.state).join(',');
  return states(before) !== states(after);
}

/** The tokens of several turns together, kind by kind. Turns that are still running count nothing. */
export function tokensOfTurns(turns: readonly Turn[]): TokenCounts {
  const sum: TokenCounts = { input: 0, output: 0, cacheRead: 0, cacheWritten: 0 };
  for (const { tokens } of turns) {
    if (tokens) {
      sum.input += tokens.input;
      sum.output += tokens.output;
      sum.cacheRead += tokens.cacheRead;
      sum.cacheWritten += tokens.cacheWritten;
    }
  }
  return sum;
}

/** Every token a count holds, whatever its kind. */
export function allTokens(tokens: TokenCounts): number {
  return tokens.input + tokens.output + tokens.cacheRead + tokens.cacheWritten;
}

const THOUSAND = 1000;
const MILLION = 1_000_000;

/** Writes a number of tokens short: "880", "1.2k", "21.5k", "162k", "1.3M". */
export function formatTokens(count: number): string {
  if (count < THOUSAND) {
    return count.toString();
  }
  const [scaled, unit] = count < MILLION ? [count / THOUSAND, 'k'] : [count / MILLION, 'M'];
  // One digit after the point is enough below a hundred, and noise above.
  const rounded = scaled < 100 ? Math.round(scaled * 10) / 10 : Math.round(scaled);
  return `${rounded.toString()}${unit}`;
}

/**
 * Says what a turn used, kind by kind: what the model wrote, what it read
 * new, and what it read again from its cache. Kinds that count nothing are
 * left out.
 */
export function tokensInWords(tokens: TokenCounts): string {
  const parts = [
    { count: tokens.output, words: 'written' },
    { count: tokens.input + tokens.cacheWritten, words: 'read' },
    { count: tokens.cacheRead, words: 'read again from the cache' },
  ].filter((part) => part.count > 0);
  if (parts.length === 0) {
    return 'no tokens';
  }
  return parts
    .map((part, position) =>
      position === 0
        ? `${formatTokens(part.count)} tokens ${part.words}`
        : `${formatTokens(part.count)} ${part.words}`,
    )
    .join(' · ');
}

const MILLISECONDS_PER_SECOND = 1000;
const SECONDS_PER_MINUTE = 60;
const MINUTES_PER_HOUR = 60;

/** Says how long it was from one moment to another: "38 s", "2 min 5 s", "1 h 12 min". */
export function durationInWords(from: Moment, to: Moment): string {
  const seconds = Math.max(
    0,
    Math.round((new Date(to).getTime() - new Date(from).getTime()) / MILLISECONDS_PER_SECOND),
  );
  if (seconds < SECONDS_PER_MINUTE) {
    return `${seconds.toString()} s`;
  }
  const minutes = Math.floor(seconds / SECONDS_PER_MINUTE);
  if (minutes < MINUTES_PER_HOUR) {
    const rest = seconds % SECONDS_PER_MINUTE;
    return rest === 0
      ? `${minutes.toString()} min`
      : `${minutes.toString()} min ${rest.toString()} s`;
  }
  const hours = Math.floor(minutes / MINUTES_PER_HOUR);
  const rest = minutes % MINUTES_PER_HOUR;
  return rest === 0 ? `${hours.toString()} h` : `${hours.toString()} h ${rest.toString()} min`;
}

/**
 * Says why specialists cannot work on this computer, and what to do about
 * it. Nothing is said when the program is ready.
 */
export function whyProgramIsNotReady(program: AgentProgram): string | null {
  switch (program.state) {
    case 'ready':
      return null;
    case 'not_installed':
      return 'Claude Code is not installed on this computer. Specialists run on it. Install it, sign in, and come back.';
    case 'signed_out':
      return 'Claude Code is not signed in. Open a terminal, run "claude auth login", and sign in.';
    case 'unavailable':
      return program.problem === null
        ? 'Claude Code could not be asked whether it is ready.'
        : `Claude Code could not be asked whether it is ready: ${program.problem}`;
  }
}

/** Says how the user is signed in to the program, for a line of status: "a Team plan". */
export function signInInWords(program: AgentProgram): string {
  if (program.subscription !== null && program.subscription !== '') {
    const plan = program.subscription;
    return `the ${plan.charAt(0).toUpperCase()}${plan.slice(1)} plan`;
  }
  if (program.signInMethod === 'claude.ai') {
    return 'a Claude plan';
  }
  return program.signInMethod === null ? 'your account' : 'an API key';
}
