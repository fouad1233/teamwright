import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { BudgetScope, Coordinator, Id } from '@/shared/api/types';

/** The name under which the coordinator of a project is cached; the project comes second. */
export function coordinatorKey(projectId: Id) {
  return ['coordinator', projectId] as const;
}

/** The name under which the questions about used-up budgets are cached. */
export const BUDGET_QUESTIONS_KEY = ['budget-questions'] as const;

/**
 * The coordinator of a project. It is asked for again after it acted and
 * whenever a conversation of the project changes, which is when what was
 * spent and who is at work change.
 */
export function useCoordinator(projectId: Id) {
  const backend = useBackend();
  const queryClient = useQueryClient();

  useEffect(() => {
    const again = () => {
      void queryClient.invalidateQueries({ queryKey: coordinatorKey(projectId) });
      void queryClient.invalidateQueries({ queryKey: BUDGET_QUESTIONS_KEY });
    };
    const stopCoordinator = backend.onCoordinatorChanged((change) => {
      if (change.projectId === projectId) {
        again();
      }
    });
    const stopConversations = backend.onConversationChanged((conversation) => {
      if (conversation.projectId === projectId) {
        again();
      }
    });
    return () => {
      stopCoordinator();
      stopConversations();
    };
  }, [backend, queryClient, projectId]);

  return useQuery({
    queryKey: coordinatorKey(projectId),
    queryFn: () => backend.coordinator(projectId),
  });
}

/** The questions about used-up budgets, in every project. */
export function useBudgetQuestions() {
  const backend = useBackend();
  const queryClient = useQueryClient();
  useEffect(
    () =>
      backend.onCoordinatorChanged(() => {
        void queryClient.invalidateQueries({ queryKey: BUDGET_QUESTIONS_KEY });
      }),
    [backend, queryClient],
  );
  return useQuery({ queryKey: BUDGET_QUESTIONS_KEY, queryFn: () => backend.listBudgetQuestions() });
}

/**
 * What the manager changes about the coordinator of a project. Each change
 * answers with the whole coordinator, which replaces the cached one; the
 * tasks and conversations it starts arrive through their own events.
 */
export function useCoordinatorCommands(projectId: Id) {
  const backend = useBackend();
  const queryClient = useQueryClient();
  const settle = (coordinator: Coordinator) => {
    queryClient.setQueryData(coordinatorKey(projectId), coordinator);
    void queryClient.invalidateQueries({ queryKey: BUDGET_QUESTIONS_KEY });
    void queryClient.invalidateQueries({ queryKey: ['tasks', projectId] });
  };
  const switchOn = useMutation({
    mutationFn: (on: boolean) => backend.switchCoordinator(projectId, on),
    onSuccess: settle,
  });
  const limit = useMutation({
    mutationFn: (value: number) => backend.setCoordinatorLimit(projectId, value),
    onSuccess: settle,
  });
  const budget = useMutation({
    mutationFn: ({ scope, tokens }: { scope: BudgetScope; tokens: number | null }) =>
      backend.setBudget(projectId, scope, tokens),
    onSuccess: settle,
  });
  const answer = useMutation({
    mutationFn: ({ scope, raiseBy }: { scope: BudgetScope; raiseBy: number | null }) =>
      backend.answerBudget(projectId, scope, raiseBy),
    onSuccess: settle,
  });
  return {
    switchTo: (on: boolean) => switchOn.mutateAsync(on),
    setLimit: (value: number) => limit.mutateAsync(value),
    setBudget: (scope: BudgetScope, tokens: number | null) => budget.mutateAsync({ scope, tokens }),
    answer: (scope: BudgetScope, raiseBy: number | null) => answer.mutateAsync({ scope, raiseBy }),
  };
}
