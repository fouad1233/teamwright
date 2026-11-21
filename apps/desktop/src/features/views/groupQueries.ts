import { useQuery } from '@tanstack/react-query';

import { useBackend } from '@/shared/api/BackendProvider';
import type { Project, View } from '@/shared/api/types';

import { useAreas } from '../areas/areaQueries';
import { useSprints } from '../sprints/sprintQueries';
import { useTags } from '../tags/tagQueries';
import { useTasks } from '../tasks/taskQueries';
import { useTeamMembers } from '../team/TeamContext';
import type { GroupNames } from './groupModel';

/**
 * The groups of the tasks of a view, as the backend works them out. They
 * are asked for again whenever the tasks, the team, the areas, the sprints
 * or the tags of the project have been loaded again, and the last answer
 * stays on screen meanwhile, so lanes do not blink. An answer for another
 * grouping does not stay: its groups would be drawn as the wrong kind.
 */
export function useTaskGroups(view: View) {
  const backend = useBackend();
  const tasksLoadedAt = useTasks(view.projectId).dataUpdatedAt;
  const tagsLoadedAt = useTags(view.projectId).dataUpdatedAt;
  const areasLoadedAt = useAreas(view.projectId).dataUpdatedAt;
  const sprintsLoadedAt = useSprints(view.projectId).dataUpdatedAt;
  const team = useTeamMembers()
    .map((member) => member.id)
    .join(',');
  return useQuery({
    queryKey: [
      'task-groups',
      view.id,
      view.grouping,
      tasksLoadedAt,
      tagsLoadedAt,
      areasLoadedAt,
      sprintsLoadedAt,
      team,
    ] as const,
    queryFn: () => backend.listTaskGroups(view.id),
    placeholderData: (previous, previousQuery) =>
      previousQuery?.queryKey[2] === view.grouping ? previous : undefined,
  });
}

/** What the names of groups are made from, for the project on screen. */
export function useGroupNames(project: Project): GroupNames {
  return {
    statuses: project.statuses,
    members: useTeamMembers(),
    areas: useAreas(project.id).data ?? NO_AREAS,
    sprints: useSprints(project.id).data ?? NO_SPRINTS,
    tags: useTags(project.id).data ?? NO_TAGS,
  };
}

const NO_AREAS: GroupNames['areas'] = [];
const NO_SPRINTS: GroupNames['sprints'] = [];
const NO_TAGS: GroupNames['tags'] = [];
