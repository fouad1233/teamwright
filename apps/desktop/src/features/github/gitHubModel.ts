import type { Color, GitHubItem, GitHubOverview, StatusCategory } from '@/shared/api/types';

/** How the state of a pull request or an issue is said. */
export function stateInWords(item: Pick<GitHubItem, 'state'>): string {
  switch (item.state) {
    case 'open':
      return 'Open';
    case 'draft':
      return 'Draft';
    case 'merged':
      return 'Merged';
    case 'closed':
      return 'Closed';
  }
}

/**
 * The mark of a state, drawn as a status of the program is: a draft has
 * not started, an open one is under way, a merged or closed one is over.
 */
export function markOf(item: Pick<GitHubItem, 'state'>): {
  category: StatusCategory;
  color: Color;
} {
  switch (item.state) {
    case 'draft':
      return { category: 'todo', color: 'gray' };
    case 'open':
      return { category: 'active', color: 'green' };
    case 'merged':
      return { category: 'done', color: 'purple' };
    case 'closed':
      return { category: 'done', color: 'gray' };
  }
}

/** How an item is named in a list: its kind and its number. */
export function numberInWords(item: Pick<GitHubItem, 'kind' | 'number'>): string {
  return `${item.kind === 'pull_request' ? 'Pull request' : 'Issue'} #${item.number.toString()}`;
}

/** How long ago something was read, in words: "just now", "4 minutes ago", "2 hours ago". */
export function agoInWords(moment: string, now: Date = new Date()): string {
  const minutes = Math.floor((now.getTime() - new Date(moment).getTime()) / 60_000);
  if (minutes < 1) {
    return 'just now';
  }
  if (minutes < 60) {
    return minutes === 1 ? '1 minute ago' : `${minutes.toString()} minutes ago`;
  }
  const hours = Math.floor(minutes / 60);
  if (hours < 24) {
    return hours === 1 ? '1 hour ago' : `${hours.toString()} hours ago`;
  }
  const days = Math.floor(hours / 24);
  return days === 1 ? '1 day ago' : `${days.toString()} days ago`;
}

/** The one line that says whether GitHub is on for a project. */
export function gitHubInWords(overview: GitHubOverview): string {
  if (overview.signIn.state !== 'signed_in') {
    return overview.signIn.state === 'not_installed'
      ? 'Off: gh is not installed'
      : 'Off: gh is not signed in';
  }
  if (overview.repository === null) {
    return 'Off: no repository';
  }
  return `On: ${overview.repository}`;
}

/** What was read, counted, in one sentence. */
export function linkedInWords(overview: GitHubOverview): string {
  const pulls = overview.linkedPullRequests;
  const issues = overview.linkedIssues;
  const counted = (count: number, one: string, many: string) =>
    `${count.toString()} ${count === 1 ? one : many}`;
  return `${counted(pulls, 'pull request', 'pull requests')} and ${counted(issues, 'issue', 'issues')} are about tasks.`;
}
