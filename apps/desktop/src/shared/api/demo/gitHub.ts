/**
 * The GitHub of the demo: `gh` is signed in as `you`, the repository is
 * one the work folder would be a clone of, and a few pull requests and
 * issues name the first tasks of a project. Nothing goes to GitHub.
 */

import type { GitHubItem, GitHubOverview, GitHubSignIn, IssueDraft, TaskOnGitHub } from '../types';

/** A task as the pretended GitHub needs it. */
export interface TaskOfDemo {
  number: number;
  title: string;
  description: string;
}

/** Whether a text names a reference such as `SH-4` as a whole, as the real application decides. */
export function mentions(text: string, reference: string): boolean {
  if (reference === '') {
    return false;
  }
  const escaped = reference.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  return new RegExp(`(^|[^A-Za-z0-9])${escaped}($|[^A-Za-z0-9])`, 'i').test(text);
}

function isAbout(item: GitHubItem, reference: string): boolean {
  return mentions(item.title, reference) || mentions(item.branch ?? '', reference);
}

/** A name for a branch, made of a reference and the first words of a title. */
function branchOf(reference: string, title: string): string {
  const words = title
    .toLowerCase()
    .replace(/[^a-z0-9 ]/g, '')
    .split(' ')
    .filter((word) => word !== '')
    .slice(0, 3);
  return [reference.toLowerCase(), ...words].join('-');
}

function referenceOf(key: string, task: TaskOfDemo): string {
  return `${key}-${task.number.toString()}`;
}

/** What the demo keeps of GitHub: when each project was read, and the issues opened. */
export class PretendedGitHub {
  private readonly readAt = new Map<string, string>();
  private readonly opened = new Map<string, GitHubItem[]>();

  signIn(): GitHubSignIn {
    return {
      state: 'signed_in',
      program: 'C:\\Users\\you\\AppData\\Local\\Programs\\gh\\bin\\gh.exe',
      account: 'you',
      problem: null,
    };
  }

  /** The repository: the one named, or one the work folder is a clone of. */
  repositoryOf(projectName: string, named: string | null): string {
    if (named !== null) {
      return named;
    }
    const slug = projectName
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '');
    return `octo-org/${slug === '' ? 'project' : slug}`;
  }

  /** Reads the repository: notes when. */
  read(projectId: string, now: string) {
    this.readAt.set(projectId, now);
  }

  readAtOf(projectId: string): string | null {
    return this.readAt.get(projectId) ?? null;
  }

  /** The pull requests and issues of the repository of a project. */
  itemsOf(
    projectId: string,
    key: string,
    repository: string,
    tasks: readonly TaskOfDemo[],
  ): GitHubItem[] {
    const page = `https://github.com/${repository}`;
    const first = [...tasks].sort((a, b) => a.number - b.number).slice(0, 4);
    const states: GitHubItem['state'][] = ['merged', 'open', 'draft'];
    const items: GitHubItem[] = [];
    let number = 40;
    first.forEach((task, index) => {
      const reference = referenceOf(key, task);
      const state = states[index];
      if (state !== undefined) {
        number += 1;
        items.push({
          kind: 'pull_request',
          number,
          title: task.title,
          state,
          url: `${page}/pull/${number.toString()}`,
          branch: branchOf(reference, task.title),
        });
      }
      if (index >= 2) {
        number += 1;
        items.push({
          kind: 'issue',
          number,
          title: `${reference} ${index === 2 ? 'needs a second look' : 'is done'}`,
          state: index === 2 ? 'open' : 'closed',
          url: `${page}/issues/${number.toString()}`,
          branch: null,
        });
      }
    });
    items.push(
      {
        kind: 'pull_request',
        number: 38,
        title: 'Update the readme',
        state: 'open',
        url: `${page}/pull/38`,
        branch: 'readme',
      },
      {
        kind: 'issue',
        number: 37,
        title: 'The build fails on a clean checkout',
        state: 'open',
        url: `${page}/issues/37`,
        branch: null,
      },
    );
    return [...items, ...(this.opened.get(projectId) ?? [])];
  }

  /** The overview of a project's GitHub, from what was last read. */
  overview(
    projectId: string,
    key: string,
    repository: string,
    isFromWorkFolder: boolean,
    tasks: readonly TaskOfDemo[],
  ): GitHubOverview {
    const readAt = this.readAtOf(projectId);
    const items = readAt === null ? [] : this.itemsOf(projectId, key, repository, tasks);
    const references = tasks.map((task) => referenceOf(key, task));
    const isLinked = (item: GitHubItem) => references.some((reference) => isAbout(item, reference));
    const unlinked = items.filter((item) => !isLinked(item));
    return {
      signIn: this.signIn(),
      repository,
      isFromWorkFolder,
      readAt,
      linkedPullRequests: items.filter((item) => item.kind === 'pull_request' && isLinked(item))
        .length,
      linkedIssues: items.filter((item) => item.kind === 'issue' && isLinked(item)).length,
      unlinked: unlinked.slice(0, 10),
      unlinkedCount: unlinked.length,
    };
  }

  /** What the repository holds about one task. */
  ofTask(
    projectId: string,
    key: string,
    repository: string,
    tasks: readonly TaskOfDemo[],
    task: TaskOfDemo,
  ): TaskOnGitHub {
    const reference = referenceOf(key, task);
    return {
      repository,
      readAt: this.readAtOf(projectId),
      items: this.itemsOf(projectId, key, repository, tasks).filter((item) =>
        isAbout(item, reference),
      ),
    };
  }

  /** What an issue for a task says. */
  draftOf(key: string, task: TaskOfDemo, repository: string): IssueDraft {
    const reference = referenceOf(key, task);
    const description = task.description.trim();
    const note = `Opened from Teamwright for ${reference}.`;
    return {
      repository,
      title: `${reference} ${task.title}`,
      body: description === '' ? note : `${description}\n\n${note}`,
    };
  }

  /** Pretends to open an issue: it is kept with the others. */
  open(projectId: string, draft: IssueDraft): GitHubItem {
    const kept = this.opened.get(projectId) ?? [];
    const number = 60 + kept.length;
    const issue: GitHubItem = {
      kind: 'issue',
      number,
      title: draft.title,
      state: 'open',
      url: `https://github.com/${draft.repository}/issues/${number.toString()}`,
      branch: null,
    };
    this.opened.set(projectId, [...kept, issue]);
    return issue;
  }
}
