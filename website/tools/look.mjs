/**
 * Looks at the website in a headless Microsoft Edge and takes pictures.
 *
 *   node website/tools/look.mjs <folder> [options]
 *
 * Start the site first (`node website/tools/serve.mjs`). This starts Edge
 * on a profile of its own, without a window, drives it through its
 * debugging protocol (as tools/shared/desktop-application.mjs drives the
 * program), and writes into <folder> one picture for every place it
 * visits, named after the size and the place. It prints what the browser
 * reported: errors, the moments the film was ready, the bytes it loaded
 * and the memory the page used.
 *
 * Options:
 *   --url URL            the page (default http://localhost:4321/teamwright/)
 *   --size WxH           the window, in CSS pixels (default 1440x900)
 *   --phone              a phone: touch, a scale of 3 and a phone's screen
 *   --scale N            device pixels per CSS pixel (default 1)
 *   --reduce-motion      as a reader who asked to reduce motion
 *   --no-script          as a browser without JavaScript
 *   --mbps N             a network of N megabits a second, 40 ms away
 *   --at LIST            the places, separated by commas: a second of the
 *                        film ("t12.5"), the id of an element ("#how"), or
 *                        "top" and "bottom" (default: the start of every
 *                        chapter and a moment into it, then #how and #get)
 *   --full               one picture of the whole page as well
 *   --wait MS            how long to wait at each place (default 900)
 *
 * Nothing is needed but Node.js 22 or later and Edge.
 */

import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';

const EDGE = [
  'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
  'C:/Program Files/Microsoft/Edge/Application/msedge.exe',
  '/usr/bin/microsoft-edge',
];

function option(name, fallback) {
  const index = process.argv.indexOf(`--${name}`);
  return index > 0 ? process.argv[index + 1] : fallback;
}
const flag = (name) => process.argv.includes(`--${name}`);

const folder = process.argv[2];
if (!folder || folder.startsWith('--')) {
  console.error('Usage: node website/tools/look.mjs <folder> [options]');
  process.exit(2);
}
const url = option('url', 'http://localhost:4321/teamwright/');
const [width, height] = option('size', flag('phone') ? '390x844' : '1440x900').split('x').map(Number);
const scale = Number(option('scale', flag('phone') ? 3 : 1));
const wait = Number(option('wait', 900));
const mbps = option('mbps');
const label = `${width}x${height}${flag('reduce-motion') ? '-still' : ''}${flag('no-script') ? '-noscript' : ''}`;

class Browser {
  #socket;
  #next = 1;
  #waiting = new Map();
  #listeners = [];

  static async start(port) {
    const profile = mkdtempSync(join(tmpdir(), 'teamwright-look-'));
    const executable = EDGE.find((path) => existsSync(path));
    if (!executable) throw new Error('Microsoft Edge was not found');
    const child = spawn(executable, [
      '--headless=new',
      `--remote-debugging-port=${port}`,
      `--user-data-dir=${profile}`,
      '--no-first-run',
      '--no-default-browser-check',
      '--disable-extensions',
      '--hide-scrollbars',
      'about:blank',
    ], { stdio: 'ignore' });
    for (let attempt = 0; attempt < 50; attempt += 1) {
      try {
        const pages = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
        const page = pages.find((each) => each.type === 'page');
        if (page) {
          const browser = new Browser();
          await browser.#connect(page.webSocketDebuggerUrl);
          browser.child = child;
          browser.profile = profile;
          return browser;
        }
      } catch {
        // Not up yet.
      }
      await sleep(200);
    }
    child.kill();
    throw new Error('Edge did not offer its debugging port');
  }

  #connect(address) {
    return new Promise((resolve, reject) => {
      this.#socket = new WebSocket(address);
      this.#socket.onopen = resolve;
      this.#socket.onerror = reject;
      this.#socket.onmessage = ({ data }) => {
        const message = JSON.parse(data);
        if (message.id && this.#waiting.has(message.id)) {
          const { resolve: done, reject: failed } = this.#waiting.get(message.id);
          this.#waiting.delete(message.id);
          if (message.error) failed(new Error(message.error.message));
          else done(message.result);
        } else if (message.method) {
          for (const listener of this.#listeners) listener(message);
        }
      };
    });
  }

  send(method, params = {}) {
    const id = this.#next++;
    this.#socket.send(JSON.stringify({ id, method, params }));
    return new Promise((resolve, reject) => this.#waiting.set(id, { resolve, reject }));
  }

  on(listener) {
    this.#listeners.push(listener);
  }

  async evaluate(expression) {
    const { result, exceptionDetails } = await this.send('Runtime.evaluate', {
      expression,
      awaitPromise: true,
      returnByValue: true,
    });
    if (exceptionDetails) throw new Error(exceptionDetails.exception?.description || exceptionDetails.text);
    return result.value;
  }

  async stop() {
    this.#socket.close();
    this.child.kill();
    await sleep(500);
    try {
      rmSync(this.profile, { recursive: true, force: true });
    } catch {
      // Edge may still hold a file for a moment; the folder is in the temp folder.
    }
  }
}

const reports = [];
const browser = await Browser.start(9300 + Math.floor(Math.random() * 500));
try {
  browser.on((message) => {
    if (message.method === 'Runtime.exceptionThrown') {
      reports.push(`exception: ${message.params.exceptionDetails.exception?.description || message.params.exceptionDetails.text}`);
    } else if (message.method === 'Runtime.consoleAPICalled' && ['error', 'warning'].includes(message.params.type)) {
      reports.push(`console ${message.params.type}: ${message.params.args.map((arg) => arg.value ?? arg.description).join(' ')}`);
    } else if (message.method === 'Log.entryAdded' && ['error', 'warning'].includes(message.params.entry.level)) {
      reports.push(`log ${message.params.entry.level}: ${message.params.entry.text} ${message.params.entry.url || ''}`);
    }
  });
  await browser.send('Runtime.enable');
  await browser.send('Log.enable');
  await browser.send('Page.enable');
  await browser.send('Network.enable');
  await browser.send('Performance.enable');
  if (mbps) {
    const bytes = (Number(mbps) * 1e6) / 8;
    await browser.send('Network.emulateNetworkConditions', {
      offline: false,
      latency: 40,
      downloadThroughput: bytes,
      uploadThroughput: bytes,
    });
  }
  await browser.send('Network.setCacheDisabled', { cacheDisabled: true });
  await browser.send('Emulation.setDeviceMetricsOverride', {
    width,
    height,
    deviceScaleFactor: scale,
    mobile: flag('phone'),
    screenWidth: width,
    screenHeight: height,
  });
  if (flag('phone')) await browser.send('Emulation.setTouchEmulationEnabled', { enabled: true, maxTouchPoints: 5 });
  await browser.send('Emulation.setEmulatedMedia', {
    features: [{ name: 'prefers-reduced-motion', value: flag('reduce-motion') ? 'reduce' : 'no-preference' }],
  });
  if (flag('no-script')) await browser.send('Emulation.setScriptExecutionDisabled', { value: true });

  const began = Date.now();
  await browser.send('Page.navigate', { url });
  for (let attempt = 0; attempt < 100; attempt += 1) {
    await sleep(100);
    const state = await browser.evaluate('document.readyState').catch(() => '');
    if (state === 'complete') break;
  }
  const loaded = Date.now() - began;

  // Wait until the coarse frames are in, or give up after twenty seconds.
  let marks = [];
  if (!flag('no-script')) {
    for (let attempt = 0; attempt < 100; attempt += 1) {
      marks = await browser.evaluate(
        "performance.getEntriesByType('mark').filter((m) => m.name.startsWith('film:')).map((m) => [m.name, Math.round(m.startTime)])",
      );
      if (marks.some(([name]) => name === 'film:coarse-ready') || !(await browser.evaluate("Boolean(document.querySelector('meta[name=teamwright-film-media]'))"))) break;
      await sleep(200);
    }
  }

  const places = (option('at') ||
    't0,t0.4,t1.8,t11,t13,t16,t19,t24,t28,t32,t34,t36.5,t39,t42,t45,t47,t50,t53,t56,t58,t60,t62,t70,#what,#how,#today,#get,#questions,bottom')
    .split(',');
  mkdirSync(folder, { recursive: true });
  for (const place of places) {
    const scrollTo = place.startsWith('t')
      ? `(() => { const c = [...document.querySelectorAll('[data-chapter]')]; const first = c[0].getBoundingClientRect(); const last = c[c.length - 1].getBoundingClientRect(); const end = Number(c[c.length - 1].dataset.end); return Math.ceil(first.top + scrollY + ${Number(place.slice(1))} / end * (last.bottom - first.top)); })()`
      : place === 'top'
        ? '0'
        : place === 'bottom'
          ? 'document.documentElement.scrollHeight'
          : `document.querySelector('${place}').getBoundingClientRect().top + scrollY`;
    await browser.evaluate(`window.scrollTo(0, ${scrollTo})`);
    await sleep(wait);
    const { data } = await browser.send('Page.captureScreenshot', { format: 'png' });
    const name = `${label}-${place.replace('#', '').replace('.', '_')}.png`;
    writeFileSync(join(folder, name), Buffer.from(data, 'base64'));
  }
  if (flag('full')) {
    const { data } = await browser.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true });
    writeFileSync(join(folder, `${label}-full.png`), Buffer.from(data, 'base64'));
  }

  marks = flag('no-script')
    ? []
    : await browser.evaluate(
        "performance.getEntriesByType('mark').filter((m) => m.name.startsWith('film:')).map((m) => [m.name, Math.round(m.startTime)])",
      );
  const resources = flag('no-script')
    ? null
    : await browser.evaluate(
        "(() => { const r = performance.getEntriesByType('resource'); const sum = (f) => r.filter(f).reduce((s, e) => s + e.transferSize, 0); return { requests: r.length, bytes: sum(() => true), film: sum((e) => e.name.includes('/media/film/')), paint: performance.getEntriesByType('paint').map((p) => [p.name, Math.round(p.startTime)]) }; })()",
      );
  const { metrics } = await browser.send('Performance.getMetrics');
  const heap = metrics.find((metric) => metric.name === 'JSHeapUsedSize')?.value;
  console.log(`${label}: loaded in ${loaded} ms`);
  if (resources) {
    console.log(`  paint: ${resources.paint.map(([name, time]) => `${name} ${time} ms`).join(', ')}`);
    console.log(`  ${resources.requests} requests, ${Math.round(resources.bytes / 1024)} kB, of which the film ${Math.round(resources.film / 1024)} kB`);
  }
  console.log(`  film: ${marks.map(([name, time]) => `${name} at ${time} ms`).join(', ') || 'no frames'}`);
  console.log(`  script memory: ${heap ? Math.round(heap / 1048576) : '?'} MB`);
  console.log(reports.length ? `  reported:\n    ${reports.join('\n    ')}` : '  reported: nothing');
} finally {
  await browser.stop();
}
