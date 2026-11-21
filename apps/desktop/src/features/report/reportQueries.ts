import { keepPreviousData, useQuery } from '@tanstack/react-query';
import { useCallback } from 'react';

import { useNavigation } from '@/app/navigation';
import { useBackend } from '@/shared/api/BackendProvider';
import type { Id, Project } from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

import { useTasks } from '../tasks/taskQueries';
import { useSetViewContent, useViewCommands, useViews } from '../views/viewQueries';
import { parseReport, serializeReport, withSheet } from './reportModel';

/**
 * The report of a whole project (decision 0042), as the backend gathers
 * it. It is asked for again whenever the tasks of the project have been
 * loaded again, so a task finished here or on another computer shows. The
 * report on screen stays while the next one is on its way.
 */
export function useProjectReport(projectId: Id, enabled = true) {
  const backend = useBackend();
  const tasksLoadedAt = useTasks(projectId).dataUpdatedAt;
  return useQuery({
    queryKey: ['project-report', projectId, tasksLoadedAt] as const,
    queryFn: () => backend.projectReport(projectId),
    enabled,
    placeholderData: keepPreviousData,
  });
}

/**
 * Returns a function that keeps the report of a project as a Markdown
 * file: the save window opens in the work folder of the project, and the
 * program says where the file is once it is written. A failure is told in
 * plain words, as other failures are.
 */
export function useSaveReport(): (project: Project) => Promise<void> {
  const backend = useBackend();
  const notifier = useNotifier();

  return useCallback(
    async (project) => {
      try {
        const path = await backend.chooseReportFile(project.id);
        if (path === null) {
          return;
        }
        const written = await backend.saveProjectReport(project.id, path);
        notifier.showNotice(`The report of ${project.name} is saved as ${written}.`);
      } catch (failure) {
        notifier.showError(describeFailure(failure));
      }
    },
    [backend, notifier],
  );
}

/**
 * Returns a function that opens the report of the whole project: the
 * project's Report view, turned to the sheet of the whole project. A
 * project without a Report view gets one first, as the "+" of the views
 * would add it.
 */
export function useOpenWholeReport(projectId: Id): () => Promise<void> {
  const navigation = useNavigation();
  const views = useViews(projectId).data;
  const commands = useViewCommands(projectId);
  const setContent = useSetViewContent();

  return useCallback(async () => {
    const view = views?.find((each) => each.kind === 'report') ?? (await commands.add('report'));
    if (view === undefined) {
      return;
    }
    const settings = parseReport(view.content);
    const whole = withSheet(settings, 'whole');
    if (whole !== settings) {
      await setContent(view, serializeReport(whole));
    }
    navigation.showView(view.id);
  }, [views, commands, setContent, navigation]);
}
