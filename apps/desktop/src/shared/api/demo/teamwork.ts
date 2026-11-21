import type { Handoff, Id, Moment, TeamMessage, TeamTalk } from '../types';

/**
 * What the specialists of the demo handed each other: the handoffs of
 * tasks and the messages between teammates, each kept with its project.
 *
 * The real application keeps them in its database (decision 0029). The demo
 * keeps them here, and follows the same rules: a handoff never changes, the
 * latest of a task counts, and a message waits until its receiver starts a
 * turn, which delivers it.
 */
export class PretendedTeamTalk {
  private handoffs: { projectId: Id; handoff: Handoff }[] = [];
  private messages: { projectId: Id; message: TeamMessage }[] = [];

  /** Takes in the handoffs and messages a workspace starts with. */
  constructor(
    handoffs: readonly { projectId: Id; handoff: Handoff }[] = [],
    messages: readonly { projectId: Id; message: TeamMessage }[] = [],
  ) {
    this.handoffs = structuredClone([...handoffs]);
    this.messages = structuredClone([...messages]);
  }

  /** Everything of a project, the oldest first. */
  talkOf(projectId: Id): TeamTalk {
    const byMoment =
      <Kept>(moment: (kept: Kept) => Moment) =>
      (first: Kept, second: Kept) =>
        moment(first).localeCompare(moment(second));
    return structuredClone({
      handoffs: this.handoffs
        .filter((kept) => kept.projectId === projectId)
        .map((kept) => kept.handoff)
        .sort(byMoment((handoff) => handoff.publishedAt)),
      messages: this.messages
        .filter((kept) => kept.projectId === projectId)
        .map((kept) => kept.message)
        .sort(byMoment((message) => message.sentAt)),
    });
  }

  /** The handoffs of a project, the oldest first. */
  handoffsOf(projectId: Id): Handoff[] {
    return this.talkOf(projectId).handoffs;
  }

  publish(projectId: Id, handoff: Handoff): void {
    this.handoffs.push({ projectId, handoff: structuredClone(handoff) });
  }

  send(projectId: Id, message: TeamMessage): void {
    this.messages.push({ projectId, message: structuredClone(message) });
  }

  /**
   * Delivers every message that waits for a member with a turn of it that
   * starts now, and says how many there were.
   */
  deliverTo(memberId: Id, turnId: Id, at: Moment): number {
    const waiting = this.messages.filter(
      ({ message }) => message.toId === memberId && message.deliveredTurnId === null,
    );
    for (const { message } of waiting) {
      message.deliveredTurnId = turnId;
      message.deliveredAt = at;
    }
    return waiting.length;
  }

  /** Forgets the handoffs of tasks that are deleted. */
  forgetTasks(taskIds: ReadonlySet<Id>): void {
    this.handoffs = this.handoffs.filter((kept) => !taskIds.has(kept.handoff.taskId));
  }

  /** Forgets everything of a project that is deleted. */
  forgetProject(projectId: Id): void {
    this.handoffs = this.handoffs.filter((kept) => kept.projectId !== projectId);
    this.messages = this.messages.filter((kept) => kept.projectId !== projectId);
  }
}
