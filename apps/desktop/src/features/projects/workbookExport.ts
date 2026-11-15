import { useCallback } from 'react';

import { useBackend } from '@/shared/api/BackendProvider';
import type { Project } from '@/shared/api/types';
import { describeFailure, useNotifier } from '@/shared/ui/Toast';

/** The characters a file name may not hold on Windows, the strictest of the systems. */
const NOT_IN_FILE_NAMES = /[<>:"/\\|?*]/g;

/**
 * The name the save window suggests for the workbook of a project: the
 * project's name, without the characters a file name may not hold, and
 * `.xlsx`.
 */
export function workbookFileName(projectName: string): string {
  const name = projectName.replace(NOT_IN_FILE_NAMES, ' ').replace(/\s+/g, ' ').trim();
  return `${name === '' ? 'Project' : name}.xlsx`;
}

/**
 * Returns a function that exports a project to Excel (decision 0025): it
 * asks the manager where to save the workbook, writes it, and says so with
 * the path and a way to show the file in its folder. A failure is told in
 * plain words, as other failures are.
 */
export function useWorkbookExport(): (project: Project) => Promise<void> {
  const backend = useBackend();
  const notifier = useNotifier();

  return useCallback(
    async (project) => {
      try {
        const path = await backend.chooseWorkbookFile(workbookFileName(project.name));
        if (path === null) {
          return;
        }
        const written = await backend.exportWorkbook(project.id, path);
        notifier.showNotice(`The workbook of ${project.name} is saved as ${written}.`, {
          label: 'Show in folder',
          onSelect: () => {
            backend.showExportedWorkbook(written).catch((failure: unknown) => {
              notifier.showError(describeFailure(failure));
            });
          },
        });
      } catch (failure) {
        notifier.showError(describeFailure(failure));
      }
    },
    [backend, notifier],
  );
}
