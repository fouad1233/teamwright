/**
 * Drives the built desktop application from a script, on Windows.
 *
 * The web view that shows the interface (WebView2) speaks the debugging
 * protocol of Chromium when it is started with a debugging port. The tools
 * next to this folder use that to act in the real window: they call the
 * commands of the shell the way the interface does, read what the window
 * shows, and take pictures of it.
 *
 * Nothing is needed but Node.js 22 or later. The real data of the user is
 * never touched: the application is pointed at a folder that the caller
 * chooses, through TEAMWRIGHT_DATA_DIR, and its web view keeps its own files
 * in that folder too.
 *
 * Nor is the user's Claude Code, unless the caller asks for it. The
 * application is pointed at a stand-in, through TEAMWRIGHT_AGENT_PROGRAM,
 * that says it is installed and signed in and cannot do any work. What a
 * tool sees therefore does not depend on the computer it runs on, and a
 * picture never shows how somebody is signed in.
 */

import { spawn } from 'node:child_process';
import { rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';

/** How long to wait for the window, or for a text in it, before giving up. */
const TIMEOUT_MILLISECONDS = 30_000;

/** How often to look again while waiting. */
const POLL_MILLISECONDS = 300;

/** The program that stands in for Claude Code. */
const STAND_IN_AGENT_PROGRAM = fileURLToPath(new URL('./stand-in-agent.cmd', import.meta.url));

/** The version the stand-in says it is. */
export const STAND_IN_AGENT_VERSION = '2.1.322';

/** The built desktop application, running on a data folder of its own. */
export class DesktopApplication {
  #child;
  #debuggingPort;
  #startFailure = null;

  constructor(child, debuggingPort) {
    this.#child = child;
    this.#debuggingPort = debuggingPort;
    child.once('error', (failure) => {
      this.#startFailure = failure;
    });
  }

  /**
   * Starts the application.
   *
   * @param {string} executable     The path of the built application.
   * @param {string} dataDirectory  The folder in which it keeps its data.
   * @param {number} debuggingPort  The port on which its web view offers the debugging protocol.
   * @param {object} [options]
   * @param {boolean} [options.withRealAgentProgram]  Lets the application find and use the
   *   Claude Code of this computer, under the account that is signed in there, instead of the
   *   stand-in.
   */
  static start(executable, dataDirectory, debuggingPort, { withRealAgentProgram = false } = {}) {
    const child = spawn(executable, [], {
      stdio: 'ignore',
      env: {
        ...process.env,
        TEAMWRIGHT_DATA_DIR: dataDirectory,
        // An empty name means: look for the real program.
        TEAMWRIGHT_AGENT_PROGRAM: withRealAgentProgram ? '' : STAND_IN_AGENT_PROGRAM,
        // The web view keeps what the interface remembers about itself, such
        // as the view that was open, in a folder of its own. Left alone, it
        // would share that folder with the installed application and with
        // every earlier run of a tool.
        WEBVIEW2_USER_DATA_FOLDER: join(dataDirectory, 'web-view'),
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${debuggingPort}`,
      },
    });
    return new DesktopApplication(child, debuggingPort);
  }

  /** Waits for the window of the application to appear and connects to it. */
  async openWindow() {
    const deadline = Date.now() + TIMEOUT_MILLISECONDS;
    while (Date.now() < deadline) {
      if (this.#startFailure) {
        throw new Error(`the application could not be started: ${this.#startFailure.message}`);
      }
      try {
        const response = await fetch(`http://127.0.0.1:${this.#debuggingPort}/json`);
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
      await sleep(POLL_MILLISECONDS);
    }
    throw new Error('the window of the application did not appear');
  }

  /** Stops the application and waits until it is gone, so its port and files are free. */
  async stop() {
    const alreadyGone =
      this.#startFailure !== null || this.#child.exitCode !== null || this.#child.signalCode !== null;
    if (alreadyGone) {
      return;
    }
    const gone = new Promise((resolve) => this.#child.once('exit', resolve));
    this.#child.kill();
    await gone;
    // The web view runs in processes of its own and takes a moment to let go
    // of the debugging port.
    await sleep(1500);
  }
}

/**
 * Removes a data folder after the application that used it has stopped.
 *
 * The web view lets go of its files a moment after the application is gone,
 * so the removal is tried again for a few seconds before it gives up.
 */
export function removeDataDirectory(dataDirectory) {
  rmSync(dataDirectory, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
}

/** A connection to the window of the application. */
export class Window {
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

  /** Sends one request of the debugging protocol and waits for its answer. */
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

  /**
   * Waits until the window shows a text, and returns everything it shows.
   *
   * The text is compared the way a person reads it: a label that the style
   * sheet shows in capitals is found in capitals.
   */
  async waitForText(text) {
    const deadline = Date.now() + TIMEOUT_MILLISECONDS;
    while (Date.now() < deadline) {
      // Right after the application starts, the page has no body yet.
      const shown = await this.evaluate("document.body?.innerText ?? ''").catch(() => '');
      if (shown.includes(text)) {
        return shown;
      }
      await sleep(POLL_MILLISECONDS);
    }
    throw new Error(`the window never showed "${text}"`);
  }

  /** Loads the interface again, so that it shows what the commands stored. */
  async reload() {
    await this.#send('Page.reload');
    await sleep(1500);
  }

  /** Clicks the element that a JavaScript expression finds in the window. */
  async click(expression) {
    await this.evaluate(`(${expression}).click()`);
  }

  /** Presses and releases the Escape key. */
  async pressEscape() {
    for (const type of ['keyDown', 'keyUp']) {
      await this.#send('Input.dispatchKeyEvent', {
        type,
        key: 'Escape',
        code: 'Escape',
        windowsVirtualKeyCode: 27,
      });
    }
  }

  /**
   * Shows the interface at a fixed size instead of the size of the real
   * window, so that pictures of it come out the same on every screen.
   *
   * @param {number} width   In the pixels of the style sheet.
   * @param {number} height  In the pixels of the style sheet.
   * @param {number} scale   Pixels of the picture for each of those. More is sharper.
   */
  async showAtSize(width, height, scale) {
    await this.#send('Emulation.setDeviceMetricsOverride', {
      width,
      height,
      deviceScaleFactor: scale,
      mobile: false,
    });
  }

  /**
   * Shows the interface in one theme, whatever the operating system is set to.
   *
   * @param {'dark' | 'light'} theme
   */
  async showInTheme(theme) {
    await this.#send('Emulation.setEmulatedMedia', {
      features: [{ name: 'prefers-color-scheme', value: theme }],
    });
  }

  /** Writes a picture of what the window shows to a PNG file. */
  async screenshot(path) {
    const { data } = await this.#send('Page.captureScreenshot', { format: 'png' });
    writeFileSync(path, Buffer.from(data, 'base64'));
  }

  close() {
    this.#socket.close();
  }
}
