/**
 * Smoke test of the desktop application on Windows.
 *
 * It starts the built application with an empty data folder, drives the real
 * window through the debugging protocol of its web view, and checks that the
 * interface, the commands of the shell and the database work together. Then
 * it starts the application a second time on the same folder to prove that
 * what was stored is still there.
 *
 * Usage:
 *   node tools/smoke/desktop-smoke.mjs <path-to-teamwright-desktop.exe> [screenshot.png]
 *
 * It needs nothing but Node.js 22 or later. The user's real data is never
 * touched: the application is pointed at a temporary folder through
 * TEAMWRIGHT_DATA_DIR.
 */

import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';

const [executable, screenshotPath] = process.argv.slice(2);
if (!executable) {
  console.error('usage: node desktop-smoke.mjs <path-to-exe> [screenshot.png]');
  process.exit(2);
}

/** The port on which the web view of the application offers its debugging protocol. */
const DEBUGGING_PORT = 9333;
const WINDOW_TIMEOUT_MILLISECONDS = 30_000;

// ----------------------------------------------------------- the application

/** Starts the application on a data folder, with the debugging protocol switched on. */
function startApplication(dataDirectory) {
  return spawn(executable, [], {
    stdio: 'ignore',
    env: {
      ...process.env,
      TEAMWRIGHT_DATA_DIR: dataDirectory,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${DEBUGGING_PORT}`,
    },
  });
}

/** Stops the application and waits until it is gone, so its port and files are free. */
async function stopApplication(application) {
  if (application.exitCode !== null) {
    return;
  }
  const gone = new Promise((resolve) => application.once('exit', resolve));
  application.kill();
  await gone;
  // The web view runs in processes of its own and takes a moment to let go
  // of the debugging port.
  await sleep(1500);
}

// ------------------------------------------------- the debugging connection

/** A connection to the window of the application. */
class Window {
  #socket;
  #nextId = 1;
  #waiting = new Map();

  constructor(socket) {
    this.#socket = socket;
    socket.addEventListener('message', (event) => {
      const message = JSON.parse(event.data);
      const waiter = this.#waiting.get(message.id);
      if (!waiter) {
        return;
      }
      this.#waiting.delete(message.id);
      if (message.error) {
        waiter.reject(new Error(message.error.message));
      } else {
        waiter.resolve(message.result);
      }
    });
  }

  /** Waits for the window of the application to appear and connects to it. */
  static async connect() {
    const deadline = Date.now() + WINDOW_TIMEOUT_MILLISECONDS;
    while (Date.now() < deadline) {
      try {
        const response = await fetch(`http://127.0.0.1:${DEBUGGING_PORT}/json`);
        const page = (await response.json()).find((target) => target.type === 'page');
        if (page) {
          const socket = new WebSocket(page.webSocketDebuggerUrl);
          await new Promise((resolve, reject) => {
            socket.addEventListener('open', resolve, { once: true });
            socket.addEventListener('error', reject, { once: true });
          });
          return new Window(socket);
        }
      } catch {
        // The application is still starting.
      }
      await sleep(400);
    }
    throw new Error('the window of the application did not appear');
  }

  #send(method, params = {}) {
    const id = this.#nextId;
    this.#nextId += 1;
    return new Promise((resolve, reject) => {
      this.#waiting.set(id, { resolve, reject });
      this.#socket.send(JSON.stringify({ id, method, params }));
    });
  }

  /** Runs JavaScript in the window and returns its value. */
  async evaluate(expression) {
    const { result, exceptionDetails } = await this.#send('Runtime.evaluate', {
      expression,
      awaitPromise: true,
      returnByValue: true,
    });
    if (exceptionDetails) {
      throw new Error(exceptionDetails.exception?.description ?? exceptionDetails.text);
    }
    return result.value;
  }

  /**
   * Calls a command of the shell the way the interface does. Resolves to
   * `{ ok: value }` or, if the command refuses, to `{ refused: error }`.
   */
  command(name, args = {}) {
    return this.evaluate(
      `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(name)}, ${JSON.stringify(args)})
         .then((ok) => ({ ok }), (refused) => ({ refused }))`,
    );
  }

  /** Waits until the page shows a text. */
  async waitForText(text) {
    const deadline = Date.now() + WINDOW_TIMEOUT_MILLISECONDS;
    while (Date.now() < deadline) {
      // Right after the application starts, the page has no body yet.
      const shown = await this.evaluate("document.body?.innerText ?? ''").catch(() => '');
      if (shown.includes(text)) {
        return shown;
      }
      await sleep(300);
    }
    throw new Error(`the window never showed "${text}"`);
  }

  async reload() {
    await this.#send('Page.reload');
    await sleep(1500);
  }

  async screenshot(path) {
    const { data } = await this.#send('Page.captureScreenshot', { format: 'png' });
    writeFileSync(path, Buffer.from(data, 'base64'));
  }

  close() {
    this.#socket.close();
  }
}

// ------------------------------------------------------------------ checks

let failures = 0;

function check(description, passed, detail = '') {
  console.log(`${passed ? 'ok  ' : 'FAIL'}  ${description}${passed || !detail ? '' : `  (${detail})`}`);
  if (!passed) {
    failures += 1;
  }
}

const dataDirectory = mkdtempSync(join(tmpdir(), 'teamwright-smoke-'));
let application = startApplication(dataDirectory);

try {
  // First run: an empty application, used the way the interface uses it.
  let window = await Window.connect();
  await window.waitForText('No projects yet');
  check('a new installation shows the first-run screen', true);

  const noProjects = await window.command('list_projects');
  check('it starts without projects', noProjects.ok?.length === 0, JSON.stringify(noProjects));

  const created = await window.command('create_project', {
    input: { name: 'Smart sensor node', workflow: 'engineering' },
  });
  const project = created.ok;
  check('a project is created with a key derived from its name', project?.key === 'SSN');
  check(
    'it has the five statuses of the engineering workflow',
    project?.statuses.map((status) => status.name).join(', ') ===
      'To do, In progress, In revision, In test, Done',
  );

  const task = async (title) =>
    (await window.command('create_task', { input: { projectId: project.id, title } })).ok;
  const board = await task('Design the board');
  const firmware = await task('Write the firmware');
  const enclosure = await task('Design the enclosure');
  check('tasks are numbered from one', [board, firmware, enclosure].map((t) => t?.number).join() === '1,2,3');

  await window.command('assign_task', { taskId: board.id, assignee: { kind: 'manager' } });
  await window.command('prioritize_task', { taskId: board.id, priority: 'high' });
  await window.command('reschedule_task', { taskId: board.id, startOn: '2026-11-12', dueOn: '2026-11-20' });
  await window.command('add_task_dependency', { taskId: firmware.id, blockerId: board.id });
  await window.command('add_task_dependency', { taskId: enclosure.id, blockerId: firmware.id });

  const cycle = await window.command('add_task_dependency', {
    taskId: board.id,
    blockerId: enclosure.id,
  });
  check(
    'a dependency that would close a loop is refused by name',
    cycle.refused?.code === 'dependency_cycle' &&
      cycle.refused?.message === 'SSN-1 cannot wait for SSN-3, because SSN-3 already waits for SSN-1',
    JSON.stringify(cycle),
  );

  const done = project.statuses.find((status) => status.category === 'done');
  const moved = await window.command('move_task', { taskId: board.id, statusId: done.id, index: 0 });
  check('moving a task to a done status completes it', typeof moved.ok?.completedAt === 'string');

  const badDate = await window.command('reschedule_task', {
    taskId: firmware.id,
    startOn: 'tomorrow',
    dueOn: null,
  });
  check('a date that is not a date is refused', badDate.refused?.code === 'invalid_request');

  // The interface shows what the commands stored.
  await window.reload();
  const shown = await window.waitForText('Write the firmware');
  check('the board shows the project', shown.includes('Smart sensor node'));
  check('the board shows every task', ['Design the board', 'Design the enclosure'].every((title) => shown.includes(title)));
  check('the header counts the tasks', shown.includes('3 tasks · 1 done'), shown.slice(0, 200));
  if (screenshotPath) {
    await window.screenshot(screenshotPath);
    console.log(`      screenshot written to ${screenshotPath}`);
  }
  window.close();

  // Second run: what was stored is still there.
  await stopApplication(application);
  application = startApplication(dataDirectory);
  window = await Window.connect();
  await window.waitForText('Smart sensor node');
  const projects = (await window.command('list_projects')).ok;
  const tasks = (await window.command('list_tasks', { projectId: projects[0]?.id })).ok;
  const storedBoard = tasks?.find((stored) => stored.number === 1);
  check('the project survives a restart', projects?.length === 1 && projects[0].key === 'SSN');
  check('the tasks survive a restart', tasks?.length === 3);
  check(
    'a task keeps everything that was set on it',
    storedBoard?.assignee?.kind === 'manager' &&
      storedBoard.priority === 'high' &&
      storedBoard.startOn === '2026-11-12' &&
      storedBoard.dueOn === '2026-11-20' &&
      storedBoard.completedAt !== null,
    JSON.stringify(storedBoard),
  );
  check(
    'dependencies survive a restart',
    tasks?.find((stored) => stored.number === 2)?.blockedBy.length === 1,
  );
  window.close();
} catch (failure) {
  check('the smoke test ran to its end', false, failure.message);
} finally {
  await stopApplication(application);
  rmSync(dataDirectory, { recursive: true, force: true });
}

console.log(failures === 0 ? '\nAll checks passed.' : `\n${failures} check(s) failed.`);
process.exit(failures === 0 ? 0 : 1);
