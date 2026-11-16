import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { Delivery, EventKind, Id, ProjectIntegrations } from '@/shared/api/types';

/** The name under which the integrations of a project are cached. */
function integrationsKey(projectId: Id) {
  return ['project-integrations', projectId] as const;
}

/** The integrations of a project on this computer. */
export function useProjectIntegrations(projectId: Id) {
  const backend = useBackend();
  return useQuery({
    queryKey: integrationsKey(projectId),
    queryFn: () => backend.projectIntegrations(projectId),
  });
}

/** The name under which the deliveries of a project are cached. */
function deliveriesKey(projectId: Id) {
  return ['integration-deliveries', projectId] as const;
}

/**
 * What was delivered to the channel and the addresses of a project. Events
 * are sent on a thread of their own, so this is asked for again every few
 * seconds while it is shown.
 */
export function useIntegrationDeliveries(projectId: Id) {
  const backend = useBackend();
  return useQuery({
    queryKey: deliveriesKey(projectId),
    queryFn: () => backend.integrationDeliveries(projectId),
    refetchInterval: 5000,
  });
}

/** The tests of the channel and of the addresses; each answers with what came of it. */
export function useDeliveryTests(projectId: Id) {
  const backend = useBackend();
  const queryClient = useQueryClient();

  return useMemo(() => {
    const afterwards = async (delivery: Delivery) => {
      await queryClient.invalidateQueries({ queryKey: deliveriesKey(projectId) });
      return delivery;
    };
    return {
      testChannel: async () => afterwards(await backend.testChannel(projectId)),
      testWebAddress: async (index: number) =>
        afterwards(await backend.testWebAddress(projectId, index)),
    };
  }, [backend, queryClient, projectId]);
}

/**
 * Everything the manager can set about the integrations of one project.
 * They are used in a dialog: a refusal by the backend is thrown to it, and
 * it shows the reason next to what was tried.
 */
export interface IntegrationCommands {
  setRepository(name: string | null): Promise<void>;
  setSlackAddress(address: string | null): Promise<void>;
  setSlackEvent(kind: EventKind, on: boolean): Promise<void>;
  addWebAddress(address: string, secret: string | null): Promise<void>;
  removeWebAddress(index: number): Promise<void>;
  setWebAddressEvent(index: number, kind: EventKind, on: boolean): Promise<void>;
}

/** The commands for the integrations of a project. */
export function useIntegrationCommands(projectId: Id): IntegrationCommands {
  const backend = useBackend();
  const queryClient = useQueryClient();

  return useMemo(() => {
    /** Runs a request that answers with the integrations, and shows them. */
    const change = async (request: () => Promise<ProjectIntegrations>) => {
      queryClient.setQueryData<ProjectIntegrations>(integrationsKey(projectId), await request());
    };

    return {
      setRepository: (name) => change(() => backend.setRepository(projectId, name)),
      setSlackAddress: (address) => change(() => backend.setSlackAddress(projectId, address)),
      setSlackEvent: (kind, on) => change(() => backend.setSlackEvent(projectId, kind, on)),
      addWebAddress: (address, secret) =>
        change(() => backend.addWebAddress(projectId, address, secret)),
      removeWebAddress: (index) => change(() => backend.removeWebAddress(projectId, index)),
      setWebAddressEvent: (index, kind, on) =>
        change(() => backend.setWebAddressEvent(projectId, index, kind, on)),
    };
  }, [backend, queryClient, projectId]);
}
