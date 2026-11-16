import type {
  AgentUsage,
  ContextUse,
  Conversation,
  MemberUsage,
  ProjectUsage,
  TokenCounts,
  Turn,
} from '../types';

/**
 * What the demo backend says about tokens and contexts. It stands in for the
 * application and the domain of the desktop program (`usage.rs`,
 * `ContextUse`), and follows the same rules: the interface never works
 * these figures out itself.
 */

/** The window every simulated model has, in tokens. */
export const SIMULATED_WINDOW = 200_000;

/** From how many hundredths of its window on a context is nearly full. */
const NEARLY_FULL_PERCENT = 80;

/**
 * From how many hundredths of its window on a simulated session makes room,
 * as the agent program does when a session is nearly out of room.
 */
const MAKES_ROOM_PERCENT = 95;

/** What a simulated session holds after it has made room: a summary of what was said, and the end of the turn. */
const HELD_AFTER_MAKING_ROOM = 46_000;

/** A context of a simulated session, with the rule of the domain applied. */
export function contextOf(held: number, window: number | null = SIMULATED_WINDOW): ContextUse {
  return {
    held,
    window,
    isNearlyFull: window !== null && window > 0 && held * 100 >= window * NEARLY_FULL_PERCENT,
  };
}

/**
 * The context of a simulated session after a turn that added `added`
 * tokens to it. A session that would run out of room makes room first, so
 * the context shrinks.
 */
export function contextAfter(previous: ContextUse | null, added: number): ContextUse {
  const held = (previous?.held ?? 0) + added;
  return held * 100 >= SIMULATED_WINDOW * MAKES_ROOM_PERCENT
    ? contextOf(HELD_AFTER_MAKING_ROOM)
    : contextOf(held);
}

/** The context of a conversation: that of its last turn that reported one. */
export function contextOfTurns(turns: readonly Turn[]): ContextUse | null {
  for (let index = turns.length - 1; index >= 0; index -= 1) {
    const context = turns[index]?.context;
    if (context) {
      return context;
    }
  }
  return null;
}

function plus(sum: TokenCounts, more: TokenCounts): TokenCounts {
  return {
    input: sum.input + more.input,
    output: sum.output + more.output,
    cacheRead: sum.cacheRead + more.cacheRead,
    cacheWritten: sum.cacheWritten + more.cacheWritten,
  };
}

/** The tokens, the turns and the last model of some conversations together. */
function spentIn(conversations: readonly Conversation[]): AgentUsage {
  const turns = conversations.flatMap((conversation) => conversation.turns);
  const ended = turns.filter((turn) => turn.state !== 'running');
  const tokens = ended.reduce((sum, turn) => (turn.tokens ? plus(sum, turn.tokens) : sum), {
    input: 0,
    output: 0,
    cacheRead: 0,
    cacheWritten: 0,
  });
  const named = turns
    .filter((turn) => turn.model !== null)
    .sort((a, b) => a.startedAt.localeCompare(b.startedAt));
  return {
    tokens,
    allTokens: tokens.input + tokens.output + tokens.cacheRead + tokens.cacheWritten,
    turns: ended.length,
    lastModel: named.at(-1)?.model ?? null,
    context: null,
  };
}

/** What an agent spent, with the context of its latest conversation. */
function spentBy(conversations: readonly Conversation[]): AgentUsage {
  const latest = [...conversations].sort((a, b) => a.startedAt.localeCompare(b.startedAt)).at(-1);
  return { ...spentIn(conversations), context: latest ? contextOfTurns(latest.turns) : null };
}

/** What a project has spent, from its conversations, the oldest first. */
export function usageOfConversations(conversations: readonly Conversation[]): ProjectUsage {
  const withAdvisor = conversations.filter((conversation) => conversation.isWithAdvisor);
  const ofFormer = conversations.filter(
    (conversation) => !conversation.isWithAdvisor && conversation.memberId === null,
  );
  const memberIds = [
    ...new Set(
      conversations.flatMap((conversation) =>
        conversation.memberId === null ? [] : [conversation.memberId],
      ),
    ),
  ];
  const members: MemberUsage[] = memberIds
    .map((memberId) => ({
      memberId,
      usage: spentBy(conversations.filter((conversation) => conversation.memberId === memberId)),
    }))
    .sort((a, b) => b.usage.allTokens - a.usage.allTokens);
  return {
    members,
    advisor: withAdvisor.length > 0 ? spentBy(withAdvisor) : null,
    former: ofFormer.length > 0 ? spentIn(ofFormer) : null,
    total: spentIn(conversations),
  };
}
