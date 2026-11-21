import { describe, expect, it } from 'vitest';

import { DemoBackend } from './demoBackend';
import { dateInWords, dueOutcome, durationInWords, numberInWords } from './projectReport';
import { sampleWorkspace } from './sampleWorkspace';
import { refusal, testEnvironment } from './testing';

describe('the report of a whole project in the DemoBackend', () => {
  it('reports a new project so far, with nothing but where it began', async () => {
    const backend = new DemoBackend(undefined, testEnvironment());
    const project = await backend.createProject({ name: 'Smart home hub', workflow: 'standard' });

    const report = await backend.projectReport(project.id);

    expect(report.isComplete).toBe(false);
    expect(report.title).toBe('Report of the project so far');
    expect(report.madeOn).toBe('2026-11-11');
    expect(report.summary).toEqual([
      'The project was begun on 11 November 2026. It has no tasks yet.',
    ]);
    expect(report.plan.sentences).toEqual(['No task is finished yet, so nothing can be compared.']);
    expect(report.finished).toEqual([]);
    expect(report.sprints).toEqual([]);
    expect(report.decisions).toEqual([]);
    expect(report.handoffs).toEqual([]);
    expect(report.brief).toBeNull();
  });

  it('reports the sample project with its areas, sprints and handoffs', async () => {
    const environment = testEnvironment();
    const backend = new DemoBackend(sampleWorkspace(environment.now()), environment);
    const [project] = await backend.listProjects();
    if (!project) {
      throw new Error('the sample has a project');
    }

    const report = await backend.projectReport(project.id);

    expect(report.isComplete).toBe(false);
    expect(report.byArea.length).toBeGreaterThan(1);
    expect(report.sprints.map((sprint) => sprint.name)).toEqual([
      'Sprint 1',
      'Sprint 2',
      'Sprint 3',
      'Sprint 4',
      'Sprint 5',
    ]);
    expect(report.sprints[0]).toMatchObject({ state: 'completed', committed: 13, completed: 11 });
    expect(report.sprints[3]).toMatchObject({ state: 'active' });
    expect(report.handoffs.length).toBeGreaterThan(0);
    expect(report.finished.length + report.open.length).toBe(
      (await backend.listTasks(project.id)).length,
    );
  });

  it('calls a project whose every task is done complete, whatever its statuses are called', async () => {
    const environment = testEnvironment();
    const backend = new DemoBackend(sampleWorkspace(environment.now()), environment);
    const [project] = await backend.listProjects();
    if (!project) {
      throw new Error('the sample has a project');
    }
    const done = project.statuses.find((status) => status.category === 'done');
    if (!done) {
      throw new Error('the sample has a status of the category done');
    }
    for (const task of await backend.listTasks(project.id)) {
      if (task.statusId !== done.id) {
        await backend.moveTask(task.id, done.id, 0);
      }
    }

    const report = await backend.projectReport(project.id);

    expect(report.isComplete).toBe(true);
    expect(report.title).toBe('Report of the project');
    expect(report.open).toEqual([]);
    expect(report.summary[1]).toBe(
      `All ${report.finished.length.toString()} of its tasks are finished.`,
    );
  });

  it('cannot save the report in a browser, which has no files', async () => {
    const backend = new DemoBackend(undefined, testEnvironment());
    const project = await backend.createProject({ name: 'Smart home hub', workflow: 'standard' });

    const path = await backend.chooseReportFile(project.id);
    expect(path).toMatch(/Smart home hub report 2026-11-11\.md$/);
    if (path === null) {
      throw new Error('the demo suggests a path');
    }
    expect((await refusal(backend.saveProjectReport(project.id, path))).code).toBe('not_available');
  });

  it('says dates, lengths of time and numbers as the application does', () => {
    expect(dateInWords('2026-11-01')).toBe('1 November 2026');
    expect(durationInWords(45)).toBe('45 minutes');
    expect(durationInWords(60)).toBe('1 hour');
    expect(durationInWords(90)).toBe('1.5 hours');
    expect(durationInWords(125)).toBe('2.1 hours');
    expect(numberInWords(1_234_567)).toBe('1,234,567');
    expect(numberInWords(999)).toBe('999');
  });

  it('tells how a task met its due date', () => {
    expect(dueOutcome(true, null, '2026-11-12', '2026-11-20')).toEqual({ kind: 'none' });
    expect(dueOutcome(true, '2026-11-12', '2026-11-12', '2026-11-20')).toEqual({ kind: 'met' });
    expect(dueOutcome(true, '2026-11-10', '2026-11-12', '2026-11-20')).toEqual({
      kind: 'missed',
      days: 2,
    });
    expect(dueOutcome(false, '2026-11-25', null, '2026-11-20')).toEqual({ kind: 'coming' });
    expect(dueOutcome(false, '2026-11-17', null, '2026-11-20')).toEqual({ kind: 'late', days: 3 });
  });
});
