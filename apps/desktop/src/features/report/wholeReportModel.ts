import type { DueOutcome, ProjectReport, SprintOutcome } from '@/shared/api/types';
import { formatDate } from '@/shared/lib/format';

/**
 * The few words the sheet of the whole project puts beside the figures of
 * the report (decision 0042). The report itself, every figure and every
 * sentence, is worked out by the backend; these only say a value in a
 * cell, the way the Markdown file of the report says it.
 */

/** What a task's due date says in a table: "On time", "2 days late", or the date. */
export function dueInWords(due: DueOutcome, dueOn: string | null): string {
  switch (due.kind) {
    case 'none':
      return '';
    case 'met':
      return 'On time';
    case 'missed':
    case 'late':
      return due.days === 1 ? '1 day late' : `${due.days.toString()} days late`;
    case 'coming':
      return dueOn === null ? '' : formatDate(dueOn);
  }
}

/** Whether a due date is to be read as a warning: a task finished or still open past it. */
export function isPastDue(due: DueOutcome): boolean {
  return due.kind === 'missed' || due.kind === 'late';
}

/** "Completed: 5 of 10 points finished". */
export function sprintOutcomeInWords(sprint: SprintOutcome): string {
  const one = sprint.committed === 1;
  const unit = sprint.unit === 'points' ? (one ? 'point' : 'points') : one ? 'task' : 'tasks';
  const state = { planned: 'Planned', active: 'Running', completed: 'Completed' }[sprint.state];
  return `${state}: ${sprint.completed.toString()} of ${sprint.committed.toString()} ${unit} finished`;
}

/** The dates of a sprint: "10 Nov to 15 Nov", the one date it has, or nothing. */
export function sprintDatesInWords(sprint: SprintOutcome): string {
  if (sprint.startOn !== null && sprint.endOn !== null) {
    return `${formatDate(sprint.startOn)} to ${formatDate(sprint.endOn)}`;
  }
  if (sprint.startOn !== null) {
    return `From ${formatDate(sprint.startOn)}`;
  }
  return sprint.endOn === null ? '' : `Until ${formatDate(sprint.endOn)}`;
}

/** "1 question waits for the manager's answer." */
export function waitingInWords(waiting: number): string {
  return waiting === 1
    ? "1 question waits for the manager's answer."
    : `${waiting.toString()} questions wait for the manager's answer.`;
}

/** Whether the part of what is still open has anything to say. */
export function hasOpenPart(report: Pick<ProjectReport, 'open' | 'waiting'>): boolean {
  return report.open.length > 0 || report.waiting > 0;
}

/**
 * The line under who asked a question: "Asked by Hardware engineer about
 * SSN-4. Answered on 12 Nov."
 */
export function askedInWords(
  decision: Pick<ProjectReport['decisions'][number], 'askedBy' | 'task' | 'answeredOn'>,
): string {
  const about = decision.task === null ? '' : ` about ${decision.task}`;
  return `Asked by ${decision.askedBy}${about}. Answered on ${formatDate(decision.answeredOn)}.`;
}
