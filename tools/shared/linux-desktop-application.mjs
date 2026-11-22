/**
 * Drives the built desktop application from a script, on Linux.
 *
 * On Linux the window of the application is WebKitGTK, which does not speak
 * the debugging protocol of Chromium that the Windows tools use. It speaks
 * WebDriver instead, through `WebKitWebDriver`, and Tauri puts its own
 * `tauri-driver` in front of that. This module starts both on a virtual
 * display of their own (Xvfb), so that nothing appears on a real screen, and
 * talks to them with plain WebDriver requests over HTTP. It offers the same
 * methods as the `Window` of `desktop-application.mjs`, so that a test reads
 * the same on both systems.
 *
 * What it needs, besides Node.js 22 or later:
 *
 * - `Xvfb`, the virtual display (`/usr/bin/Xvfb`).
 * - `tauri-driver` (`cargo install tauri-driver --locked`). Found on the
 *   `PATH` or in `~/.cargo/bin`, or named in `TAURI_DRIVER`.
 * - `WebKitWebDriver`, from Ubuntu's package `webkit2gtk-driver`. Found on
 *   the `PATH` or in the folder of `docs/development/linux-without-admin.md`,
 *   or named in `WEBKIT_WEBDRIVER`.
 *
 * The real data of the user is never touched: the application is pointed at
 * a folder that the caller chooses, through TEAMWRIGHT_DATA_DIR, and the web
 * view keeps its own files in that folder too, through the XDG folders. The
 * application is pointed at a stand-in for Claude Code, through
 * TEAMWRIGHT_AGENT_PROGRAM, unless the caller asks for the real one.
 */

import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { delimiter, join } from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';

/** How long to wait for the window, or for a text in it, before giving up. */
const TIMEOUT_MILLISECONDS = 30_000;

/** How often to look again while waiting. */
const POLL_MILLISECONDS = 300;

/** The program that stands in for Claude Code. */
const STAND_IN_AGENT_PROGRAM = fileURLToPath(new URL('./stand-in-agent.sh', import.meta.url));

/** The version the stand-in says it is. */
export const STAND_IN_AGENT_VERSION = '2.1.322';

/** The size of the virtual screen, and of the window on it. */
const SCREEN = { width: 1440, height: 900 };

/** Finds a program by an environment variable, the PATH, or known folders. */
function findProgram(name, variable, folders) {
  if (process.env[variable]) {
    return process.env[variable];
  }
  const candidates = [
    ...(process.env.PATH ?? '').split(delimiter).map((folder) => join(folder, name)),
    ...folders.map((folder) => join(folder, name)),
  ];
  const found = candidates.find((candidate) => existsSync(candidate));
  if (!found) {
    throw new Error(`${name} was not found; name it in ${variable}`);
  }
  return found;
}

/** Starts a virtual display and resolves to its number once it is ready. */
function startVirtualDisplay() {
  // `-displayfd` lets the server choose a free display and write its number
  // to the given file descriptor, here its standard output.
  const server = spawn(
    'Xvfb',
    ['-displayfd', '1', '-screen', '0', `${SCREEN.width}x${SCREEN.height}x24`, '-nolisten', 'tcp'],
    { stdio: ['ignore', 'pipe', 'ignore'] },
  );
  return new Promise((resolve, reject) => {
    let written = '';
    server.once('error', reject);
    server.once('exit', (code) => reject(new Error(`Xvfb ended with code ${code}`)));
    server.stdout.on('data', (chunk) => {
      written += chunk.toString();
      if (written.includes('\n')) {
        resolve({ server, display: `:${written.trim()}` });
      }
    });
  });
}

/** Stops a child process and waits until it is gone. */
async function stopChild(child, signal = 'SIGTERM') {
  if (!child || child.exitCode !== null || child.signalCode !== null) {
    return;
  }
  const gone = new Promise((resolve) => child.once('exit', resolve));
  child.kill(signal);
  const stopped = await Promise.race([gone.then(() => true), sleep(5000).then(() => false)]);
  if (!stopped) {
    child.kill('SIGKILL');
    await gone;
  }
}

/** The built desktop application, running on a data folder of its own. */
export class DesktopApplication {
  #display;
  #driver;
  #port;
  #executable;
  #session = null;

  constructor(display, driver, port, executable) {
    this.#display = display;
    this.#driver = driver;
    this.#port = port;
    this.#executable = executable;
  }

  /**
   * Starts the virtual display and the driver. The application itself
   * starts when `openWindow` asks the driver for a session.
   *
   * @param {string} executable     The path of the built application.
   * @param {string} dataDirectory  The folder in which it keeps its data.
   * @param {number} port           The port of tauri-driver; the one after it is WebKitWebDriver's.
   * @param {object} [options]
   * @param {boolean} [options.withRealAgentProgram]  Lets the application find the real Claude Code.
   */
  static async start(executable, dataDirectory, port, { withRealAgentProgram = false } = {}) {
    const tauriDriver = findProgram('tauri-driver', 'TAURI_DRIVER', [
      join(homedir(), '.cargo', 'bin'),
    ]);
    const webDriver = findProgram('WebKitWebDriver', 'WEBKIT_WEBDRIVER', [
      '/usr/bin',
      join(homedir(), '.local', 'opt', 'tauri-sysroot', 'usr', 'bin'),
    ]);
    const display = await startVirtualDisplay();
    // The web view keeps what the interface remembers about itself in the
    // XDG folders. Left alone, it would share them with the installed
    // application and with every earlier run of a tool.
    const webView = join(dataDirectory, 'web-view');
    for (const folder of ['data', 'cache', 'config']) {
      mkdirSync(join(webView, folder), { recursive: true });
    }
    const driver = spawn(
      tauriDriver,
      ['--port', String(port), '--native-port', String(port + 1), '--native-driver', webDriver],
      {
        stdio: ['ignore', 'ignore', 'pipe'],
        env: {
          ...process.env,
          DISPLAY: display.display,
          WAYLAND_DISPLAY: '',
          TEAMWRIGHT_DATA_DIR: dataDirectory,
          // An empty name means: look for the real program.
          TEAMWRIGHT_AGENT_PROGRAM: withRealAgentProgram ? '' : STAND_IN_AGENT_PROGRAM,
          XDG_DATA_HOME: join(webView, 'data'),
          XDG_CACHE_HOME: join(webView, 'cache'),
          XDG_CONFIG_HOME: join(webView, 'config'),
        },
      },
    );
    driver.stderr.on('data', (chunk) => {
      if (process.env.TEAMWRIGHT_SMOKE_VERBOSE) {
        process.stderr.write(chunk);
      }
    });
    const application = new DesktopApplication(display.server, driver, port, executable);
    await application.#waitForDriver();
    return application;
  }

  async #waitForDriver() {
    const deadline = Date.now() + TIMEOUT_MILLISECONDS;
    while (Date.now() < deadline) {
      if (this.#driver.exitCode !== null) {
        throw new Error(`tauri-driver ended with code ${this.#driver.exitCode}`);
      }
      try {
        await fetch(`http://127.0.0.1:${this.#port}/status`);
        return;
      } catch {
        await sleep(POLL_MILLISECONDS);
      }
    }
    throw new Error('tauri-driver did not start');
  }

  /** Starts the application through a WebDriver session and connects to its window. */
  async openWindow() {
    const base = `http://127.0.0.1:${this.#port}`;
    const answer = await webDriverRequest(base, 'POST', '/session', {
      capabilities: {
        alwaysMatch: {
          browserName: 'wry',
          'tauri:options': { application: this.#executable },
        },
      },
    });
    this.#session = `${base}/session/${answer.sessionId}`;
    const window = new Window(this.#session);
    await window.waitFor('window.__TAURI_INTERNALS__ !== undefined && document.body !== null');
    return window;
  }

  /** Ends the session, which closes the application, then stops the driver and the display. */
  async stop() {
    if (this.#session) {
      await webDriverRequest(this.#session, 'DELETE', '').catch(() => undefined);
      this.#session = null;
    }
    await stopChild(this.#driver);
    await stopChild(this.#display);
    // The application lets go of its database a moment after it is gone.
    await sleep(1000);
  }
}

/** Sends one WebDriver request and returns its value, or throws its error. */
async function webDriverRequest(base, method, path, body) {
  const response = await fetch(`${base}${path}`, {
    method,
    headers: body === undefined ? {} : { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  const parsed = text ? JSON.parse(text) : {};
  const value = parsed.value ?? parsed;
  if (!response.ok || (value && typeof value === 'object' && 'error' in value && value.error)) {
    throw new Error(
      `${method} ${path}: ${value?.error ?? response.status}: ${value?.message ?? text}`,
    );
  }
  // A new session answers with its id beside the capabilities.
  return path === '/session' ? { sessionId: value.sessionId ?? parsed.sessionId } : value;
}

/** Removes a data folder after the application that used it has stopped. */
export function removeDataDirectory(dataDirectory) {
  rmSync(dataDirectory, { recursive: true, force: true, maxRetries: 20, retryDelay: 250 });
}

/** The key WebDriver uses for the Escape key. */
const ESCAPE_KEY = '';

/** A connection to the window of the application, through a WebDriver session. */
export class Window {
  #session;

  constructor(session) {
    this.#session = session;
  }

  #request(method, path, body) {
    return webDriverRequest(this.#session, method, path, body);
  }

  /**
   * Runs JavaScript in the window and returns its value. The expression may
   * be a promise; its result is awaited.
   */
  async evaluate(expression) {
    const result = await this.#request('POST', '/execute/async', {
      script: `const done = arguments[arguments.length - 1];
        Promise.resolve()
          .then(() => (${expression}))
          .then((value) => done({ value }), (error) => done({ error: String(error && (error.stack || error.message) || error) }));`,
      args: [],
    });
    if (result && 'error' in result) {
      throw new Error(result.error);
    }
    return result?.value;
  }

  /** Waits until a JavaScript expression is true in the window. */
  async waitFor(expression, timeout = TIMEOUT_MILLISECONDS) {
    const deadline = Date.now() + timeout;
    let lastFailure = null;
    while (Date.now() < deadline) {
      try {
        if (await this.evaluate(expression)) {
          return;
        }
      } catch (failure) {
        lastFailure = failure;
      }
      await sleep(POLL_MILLISECONDS);
    }
    throw new Error(
      `the window never made true: ${expression}${lastFailure ? ` (${lastFailure.message})` : ''}`,
    );
  }

  /**
   * Calls a command of the shell the way the interface does. Resolves to
   * `{ ok: value }` or, if the command refuses, to `{ refused: error }`.
   */
  command(name, args = {}) {
    return this.evaluate(
      `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(name)}, ${JSON.stringify(args)})
         .then((ok) => ({ ok: ok === undefined ? null : ok }), (refused) => ({ refused }))`,
    );
  }

  /** Waits until the window shows a text, and returns everything it shows. */
  async waitForText(text, timeout = TIMEOUT_MILLISECONDS) {
    const deadline = Date.now() + timeout;
    let shown = '';
    while (Date.now() < deadline) {
      shown = await this.evaluate("document.body?.innerText ?? ''").catch(() => '');
      if (shown.includes(text)) {
        return shown;
      }
      await sleep(POLL_MILLISECONDS);
    }
    throw new Error(`the window never showed "${text}"; it showed: ${shown.slice(0, 300)}`);
  }

  /** Loads the interface again, so that it shows what the commands stored. */
  async reload() {
    await this.evaluate('location.reload()').catch(() => undefined);
    await sleep(1500);
    await this.waitFor('window.__TAURI_INTERNALS__ !== undefined && document.body !== null');
  }

  /** Clicks the element that a JavaScript expression finds in the window. */
  async click(expression) {
    await this.evaluate(`(${expression}).click()`);
  }

  /** Clicks the first button, tab or link whose text is exactly this. */
  async clickText(text, selector = 'button, [role="tab"], a, [role="menuitem"], [role="option"]') {
    await this.waitFor(
      `[...document.querySelectorAll(${JSON.stringify(selector)})].some((element) => element.innerText.trim() === ${JSON.stringify(text)})`,
    );
    await this.click(
      `[...document.querySelectorAll(${JSON.stringify(selector)})].find((element) => element.innerText.trim() === ${JSON.stringify(text)})`,
    );
  }

  /** Finds an element with a CSS selector, through WebDriver itself. */
  async find(selector) {
    const found = await this.#request('POST', '/element', {
      using: 'css selector',
      value: selector,
    });
    return Object.values(found)[0];
  }

  /** Clicks an element the way a person would, through WebDriver. */
  async clickElement(selector) {
    const element = await this.find(selector);
    await this.#request('POST', `/element/${element}/click`, {});
  }

  /** Types into an element, key by key, through WebDriver. */
  async type(selector, text) {
    const element = await this.find(selector);
    await this.#request('POST', `/element/${element}/value`, { text });
  }

  /** Presses and releases the Escape key. */
  async pressEscape() {
    await this.#request('POST', '/actions', {
      actions: [
        {
          type: 'key',
          id: 'keyboard',
          actions: [
            { type: 'keyDown', value: ESCAPE_KEY },
            { type: 'keyUp', value: ESCAPE_KEY },
          ],
        },
      ],
    });
  }

  /** Gives the window a size, in the pixels of the screen. */
  async showAtSize(width, height) {
    await this.#request('POST', '/window/rect', { x: 0, y: 0, width, height });
  }

  /** Writes a picture of what the window shows to a PNG file. */
  async screenshot(path) {
    const data = await this.#request('GET', '/screenshot');
    writeFileSync(path, Buffer.from(data, 'base64'));
  }

  close() {
    // The session is ended by DesktopApplication.stop.
  }
}
