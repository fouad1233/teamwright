import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { Effort, Id, Member, NewMember } from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

import { tasksKey } from '../tasks/taskQueries';

/** The name under which the team of a project is cached. */
export function teamKey(projectId: Id) {
  return ['team', projectId] as const;
}

/** The team of a project. The member who joined first comes first. */
export function useTeam(projectId: Id) {
  const backend = useBackend();
  return useQuery({ queryKey: teamKey(projectId), queryFn: () => backend.listMembers(projectId) });
}

/** What can be changed about a member after it joined. */
export interface MemberChanges {
  name: string;
  responsibility: string;
  /** The model and the effort of an AI specialist. A person has neither. */
  model: string | null;
  effort: Effort | null;
}

/**
 * Everything the interface can do to the team of one project.
 *
 * Adding and changing a member happen in a form. A refusal by the backend is
 * thrown to the form, which shows it next to its fields.
 */
export interface TeamCommands {
  /** Puts someone on the team. */
  add(member: Omit<NewMember, 'projectId'>): Promise<Member>;
  /** Changes what differs: the name, the responsibility, the way a specialist runs. */
  change(member: Member, changes: MemberChanges): Promise<void>;
  /**
   * Takes a member off the team. Its tasks are given to nobody. Resolves to
   * whether the member was removed; a refusal is shown to the user.
   */
  remove(memberId: Id): Promise<boolean>;
}

/** The commands for the team of a project. */
export function useTeamCommands(projectId: Id): TeamCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();
  const notifier = useNotifier();

  return useMemo(() => {
    const queryKey = teamKey(projectId);

    /** Puts a member the backend answered with into the team on screen. */
    const show = (member: Member) => {
      queryClient.setQueryData<Member[]>(queryKey, (team = []) =>
        team.some((other) => other.id === member.id)
          ? team.map((other) => (other.id === member.id ? member : other))
          : [...team, member],
      );
    };

    return {
      add: async (member) => {
        const added = await backend.addMember({ ...member, projectId });
        show(added);
        return added;
      },
      change: async (member, changes) => {
        if (changes.name.trim() !== member.name) {
          show(await backend.renameMember(member.id, changes.name));
        }
        if (changes.responsibility.trim() !== member.responsibility) {
          show(await backend.setMemberResponsibility(member.id, changes.responsibility));
        }
        if (changes.model !== member.model || changes.effort !== member.effort) {
          show(
            await backend.setMemberProfile(member.id, {
              model: changes.model,
              effort: changes.effort,
            }),
          );
        }
      },
      remove: async (memberId) => {
        let wasRemoved = true;
        try {
          await backend.removeMember(memberId);
        } catch (failure) {
          notifier.showError(describeFailure(failure));
          wasRemoved = false;
        }
        // The tasks of the member were given to nobody, so the tasks are
        // loaded again together with the team.
        await Promise.all([
          queryClient.invalidateQueries({ queryKey }),
          queryClient.invalidateQueries({ queryKey: tasksKey(projectId) }),
        ]);
        return wasRemoved;
      },
    };
  }, [backend, queryClient, notifier, projectId]);
}
