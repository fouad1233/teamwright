import { createContext, useContext, useMemo, type ReactNode } from 'react';

import type { Id, Member } from '@/shared/api/types';

import { indexMembers } from './teamModel';

/** The team of the project on screen. */
interface Team {
  /** The members in the order they joined. */
  members: readonly Member[];
  membersById: ReadonlyMap<Id, Member>;
}

const NO_TEAM: Team = { members: [], membersById: new Map() };

const TeamContext = createContext<Team>(NO_TEAM);

interface TeamProviderProps {
  /** The members of the team in the order they joined. */
  members: readonly Member[];
  children: ReactNode;
}

/**
 * Makes the team of the project on screen known to everything inside. A
 * card, a row or a menu can then show who a task is given to without the
 * team being handed down through every component in between.
 */
export function TeamProvider({ members, children }: TeamProviderProps) {
  const team = useMemo<Team>(() => ({ members, membersById: indexMembers(members) }), [members]);
  return <TeamContext value={team}>{children}</TeamContext>;
}

/** The members of the team of the project on screen, in the order they joined. */
export function useTeamMembers(): readonly Member[] {
  return useContext(TeamContext).members;
}

/** One member of the team of the project on screen, if it is on the team. */
export function useTeamMember(memberId: Id | undefined): Member | undefined {
  const { membersById } = useContext(TeamContext);
  return memberId === undefined ? undefined : membersById.get(memberId);
}
