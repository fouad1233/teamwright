import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { Id, KitChoice, Library, NewConnector } from '@/shared/api/types';

/** The name under which the library of a project is cached. */
export function libraryKey(projectId: Id) {
  return ['library', projectId] as const;
}

/** The library of skills and connectors of a project, and the kits of its specialists. */
export function useLibrary(projectId: Id) {
  const backend = useBackend();
  return useQuery({
    queryKey: libraryKey(projectId),
    queryFn: () => backend.projectLibrary(projectId),
  });
}

/** Everything the interface can do to the library of a project. A refusal is thrown. */
export interface LibraryCommands {
  /** Lets the manager choose a folder and adds the skill in it. Resolves to false if none was chosen. */
  addSkill(): Promise<boolean>;
  removeSkill(skillId: Id): Promise<void>;
  addConnector(connector: NewConnector): Promise<void>;
  removeConnector(connectorId: Id): Promise<void>;
  setKit(memberId: Id, kit: KitChoice): Promise<void>;
  /** Confirms on this computer a connector exactly as the manager was shown it. */
  confirmConnector(connectorId: Id, fingerprint: string): Promise<void>;
  /** Confirms on this computer the tools of a specialist the manager was shown. */
  confirmKit(memberId: Id, shown: KitChoice): Promise<void>;
}

/** The commands for the library of a project. */
export function useLibraryCommands(projectId: Id): LibraryCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();

  return useMemo(() => {
    const show = (library: Library) => {
      queryClient.setQueryData(libraryKey(projectId), library);
    };
    return {
      addSkill: async () => {
        const folder = await backend.chooseSkillFolder();
        if (folder === null) return false;
        show(await backend.addSkill(projectId, folder));
        return true;
      },
      removeSkill: async (skillId) => {
        show(await backend.removeSkill(projectId, skillId));
      },
      addConnector: async (connector) => {
        show(await backend.addConnector(projectId, connector));
      },
      removeConnector: async (connectorId) => {
        show(await backend.removeConnector(projectId, connectorId));
      },
      setKit: async (memberId, kit) => {
        show(await backend.setMemberKit(projectId, memberId, kit));
      },
      confirmConnector: async (connectorId, fingerprint) => {
        show(await backend.confirmConnector(projectId, connectorId, fingerprint));
      },
      confirmKit: async (memberId, shown) => {
        show(await backend.confirmMemberKit(projectId, memberId, shown));
      },
    };
  }, [backend, queryClient, projectId]);
}
