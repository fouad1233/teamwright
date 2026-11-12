import type { Project, Task } from '../types';

/**
 * Writes the brief an AI specialist gets when it starts on a task: what the
 * task is, what it belongs to, what it builds on, and what the manager
 * expects back.
 *
 * The real brief is written by the application (`work.rs` in the application
 * crate). This is the demo's copy of it, word for word, so that the demo
 * shows what a specialist would really be asked.
 */
export function briefFor(project: Project, task: Task, tasks: readonly Task[]): string {
  const reference = (other: Task) => `${project.key}-${other.number.toString()}`;
  const statusName = (other: Task) =>
    project.statuses.find((status) => status.id === other.statusId)?.name ?? 'unknown status';
  const listed = (others: readonly Task[]) =>
    others.map((other) => `- ${reference(other)}: ${other.title} (${statusName(other)})`);
  const byNumber = (first: Task, second: Task) => first.number - second.number;

  const parent = tasks.find((other) => other.id === task.parentId);
  const parts = tasks.filter((other) => other.parentId === task.id).sort(byNumber);
  const blockers = tasks.filter((other) => task.blockedBy.includes(other.id)).sort(byNumber);

  const paragraphs: string[] = [`Your task is ${reference(task)}: ${task.title}`];
  if (task.description.trim() !== '') {
    paragraphs.push(task.description.trim());
  }
  if (parent) {
    paragraphs.push(`It is a part of ${reference(parent)}: ${parent.title}`);
  }
  if (parts.length > 0) {
    paragraphs.push(['Its parts are:', ...listed(parts)].join('\n'));
  }
  if (blockers.length > 0) {
    paragraphs.push(['It builds on these tasks:', ...listed(blockers)].join('\n'));
  }
  if (task.dueOn !== null) {
    paragraphs.push(`It is due on ${task.dueOn}.`);
  }
  paragraphs.push(
    [
      'When you are done, end with a short report for your manager:',
      '1. what you did,',
      '2. which files you created or changed,',
      '3. what you decided on your own, and why,',
      '4. what is still open.',
    ].join('\n'),
  );
  return paragraphs.join('\n\n');
}
