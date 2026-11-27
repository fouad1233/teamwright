/**
 * Proof on Linux that the program installs its own update
 * ([decision 0045](../../docs/decisions/0045-installing-updates-from-inside.md)).
 *
 * It builds the program three times as an AppImage, with a key pair made
 * for this run only (never the real key of the project):
 *
 * - an older version (0.0.1) that trusts the throwaway key and asks a
 *   server on this computer for updates;
 * - a newer version (0.0.2) signed with that key;
 * - the same newer version signed with a second throwaway key, which the
 *   older version must refuse.
 *
 * Then it serves a `latest.json` written by hand, the update files, and a
 * list of releases shaped like GitHub's, and drives the older version
 * through its window, as the manager would, on a virtual display: the
 * notice shows 0.0.2, the update signed with the wrong key is refused in
 * words and nothing is replaced, the right one downloads with its progress,
 * is checked and put in place, and "Restart now" starts the program again
 * as 0.0.2.
 *
 * Usage:
 *   node tools/smoke/update-linux.mjs <work-folder> [--reuse-builds]
 *
 * The work folder holds the keys, the builds, the data of the program and
 * the pictures. `--reuse-builds` skips the builds when a folder of an
 * earlier run has them. It needs what the Linux smoke test needs (Xvfb,
 * tauri-driver, WebKitWebDriver), what building an AppImage needs (network
 * for the bundler's tools the first time), and either FUSE or the AppImage's
 * own way around it: this script sets APPIMAGE_EXTRACT_AND_RUN=1, which
 * makes an AppImage unpack itself into /tmp instead of mounting itself.
 * See docs/development/README.md.
 *
 * The updater refuses an address of updates that is not https unless the
 * configuration allows it, so the builds of this run (and only they) set
 * `dangerousInsecureTransportProtocol`; the real configuration does not.
 */

import { execFileSync, spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  readlinkSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { createServer } from 'node:http';
import { join, resolve } from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';

import { DesktopApplication } from '../shared/linux-desktop-application.mjs';

const ROOT = fileURLToPath(new URL('../..', import.meta.url));
const [workArgument, ...flags] = process.argv.slice(2);
if (!workArgument) {
  console.error('usage: node tools/smoke/update-linux.mjs <work-folder> [--reuse-builds]');
  process.exit(2);
}
const WORK = resolve(workArgument);
const REUSE = flags.includes('--reuse-builds');

const OLD_VERSION = '0.0.1';
const NEW_VERSION = '0.0.2';
const SERVER_PORT = Number(process.env.TEAMWRIGHT_UPDATE_PORT ?? 18473);
const DRIVER_PORT = Number(process.env.TEAMWRIGHT_SMOKE_PORT ?? 4764);
const ADDRESS = `http://127.0.0.1:${SERVER_PORT}`;
const APPIMAGE_NAME = `Teamwright_${NEW_VERSION}_amd64.AppImage`;

// ------------------------------------------------------------------ checks

let failures = 0;
function check(what, isTrue, detail = '') {
  console.log(`${isTrue ? 'ok  ' : 'FAIL'}  ${what}${detail ? ` (${detail})` : ''}`);
  if (!isTrue) {
    failures += 1;
  }
}

function md5(path) {
  return createHash('md5').update(readFileSync(path)).digest('hex');
}

// ------------------------------------------------------------------ builds

/** Makes a throwaway key pair with Tauri's own signer, without a password. */
function makeKey(path) {
  if (existsSync(path) && REUSE) {
    return;
  }
  execFileSync('npx', ['tauri', 'signer', 'generate', '--ci', '--force', '-w', path], {
    cwd: join(ROOT, 'apps/desktop'),
    stdio: 'ignore',
  });
}

/**
 * Builds the program as an AppImage of a version, with the public key it
 * trusts, signed with a private key, and copies the AppImage and its
 * signature into a folder.
 */
function build({ version, trusts, signsWith, into }) {
  const appImage = join(into, `Teamwright_${version}_amd64.AppImage`);
  if (REUSE && existsSync(appImage) && existsSync(`${appImage}.sig`)) {
    console.log(`      reusing the build in ${into}`);
    return appImage;
  }
  mkdirSync(into, { recursive: true });
  const config = join(into, 'tauri.config.json');
  writeFileSync(
    config,
    JSON.stringify({
      version,
      bundle: { createUpdaterArtifacts: true },
      plugins: {
        updater: {
          pubkey: readFileSync(`${trusts}.pub`, 'utf8').trim(),
          endpoints: [`${ADDRESS}/latest.json`],
          dangerousInsecureTransportProtocol: true,
        },
      },
    }),
  );
  console.log(`      building ${version} into ${into} (a few minutes)`);
  const log = join(into, 'build.log');
  try {
    execFileSync(
      'sh',
      [
        '-c',
        `npm run tauri --workspace @teamwright/desktop -- build --debug --bundles appimage --config "${config}" > "${log}" 2>&1`,
      ],
      {
        cwd: ROOT,
        stdio: 'inherit',
        env: {
          ...process.env,
          TAURI_SIGNING_PRIVATE_KEY: signsWith,
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: '',
        },
      },
    );
  } catch {
    throw new Error(`the build of ${version} failed; see ${log}`);
  }
  const bundle = join(ROOT, 'target/debug/bundle/appimage');
  const built = join(bundle, `Teamwright_${version}_amd64.AppImage`);
  copyFileSync(built, appImage);
  copyFileSync(`${built}.sig`, `${appImage}.sig`);
  return appImage;
}

// ------------------------------------------------------------------ server

/** What the server answers with; the scenario changes it. */
const served = { manifest: null, appImage: null, requests: [] };

const server = createServer((request, response) => {
  served.requests.push(request.url);
  if (request.url === '/latest.json') {
    response.writeHead(200, { 'content-type': 'application/json' });
    response.end(JSON.stringify(served.manifest));
  } else if (request.url === `/${APPIMAGE_NAME}`) {
    const size = statSync(served.appImage).size;
    response.writeHead(200, {
      'content-type': 'application/octet-stream',
      'content-length': String(size),
    });
    response.end(readFileSync(served.appImage));
  } else if (request.url.startsWith('/repos/fouad1233/teamwright/releases')) {
    // Shaped like GitHub's list, so that the notice hears of the release.
    response.writeHead(200, { 'content-type': 'application/json' });
    response.end(
      JSON.stringify([
        { tag_name: `v${NEW_VERSION}`, prerelease: true, draft: false },
        { tag_name: `v${OLD_VERSION}`, prerelease: true, draft: false },
      ]),
    );
  } else {
    response.writeHead(404);
    response.end();
  }
});

/** A `latest.json` as the release workflow writes it, for one AppImage. */
function manifestFor(appImage) {
  const entry = {
    signature: readFileSync(`${appImage}.sig`, 'utf8').trim(),
    url: `${ADDRESS}/${APPIMAGE_NAME}`,
  };
  return {
    version: NEW_VERSION,
    notes: 'A version made for the test of updating.',
    pub_date: new Date().toISOString(),
    platforms: { 'linux-x86_64-appimage': entry, 'linux-x86_64': entry },
  };
}

// ------------------------------------------------------------------ processes

/** The processes of the program that use the data folder of this run. */
function programsOn(dataDirectory) {
  const found = [];
  for (const name of readdirSync('/proc')) {
    if (!/^\d+$/.test(name)) {
      continue;
    }
    try {
      const environment = readFileSync(`/proc/${name}/environ`, 'utf8').split('\0');
      const exe = readlinkSync(`/proc/${name}/exe`);
      if (
        environment.includes(`TEAMWRIGHT_DATA_DIR=${dataDirectory}`) &&
        exe.endsWith('/teamwright-desktop')
      ) {
        found.push({ pid: Number(name), exe });
      }
    } catch {
      // Gone, or not ours to read.
    }
  }
  return found;
}

async function stopProgram(pid) {
  try {
    process.kill(pid, 'SIGTERM');
  } catch {
    return;
  }
  for (let i = 0; i < 50; i++) {
    try {
      process.kill(pid, 0);
    } catch {
      return;
    }
    await sleep(200);
  }
  try {
    process.kill(pid, 'SIGKILL');
  } catch {
    // Gone meanwhile.
  }
}

// ------------------------------------------------------------------ window

async function openComputers(window) {
  await window.waitFor(
    `[...document.querySelectorAll('button')].some((b) => /^Computers/.test(b.innerText.trim()))`,
  );
  await window.click(
    `[...document.querySelectorAll('button')].find((b) => /^Computers/.test(b.innerText.trim()))`,
  );
  await window.waitForText('Look now');
}

async function picture(window, name) {
  await window
    .evaluate(`document.querySelector('[aria-label="Version"]')?.scrollIntoView()`)
    .catch(() => undefined);
  await sleep(300);
  await window.screenshot(join(WORK, 'pictures', `${name}.png`)).catch(() => undefined);
}

// ------------------------------------------------------------------ run

mkdirSync(join(WORK, 'pictures'), { recursive: true });
const keys = join(WORK, 'keys');
mkdirSync(keys, { recursive: true });
const rightKey = join(keys, 'right.key');
const wrongKey = join(keys, 'wrong.key');
makeKey(rightKey);
makeKey(wrongKey);

const newRight = build({
  version: NEW_VERSION,
  trusts: rightKey,
  signsWith: rightKey,
  into: join(WORK, 'new-right'),
});
const newWrong = build({
  version: NEW_VERSION,
  trusts: wrongKey,
  signsWith: wrongKey,
  into: join(WORK, 'new-wrong'),
});
const old = build({
  version: OLD_VERSION,
  trusts: rightKey,
  signsWith: rightKey,
  into: join(WORK, 'old'),
});

console.log('The builds');
for (const appImage of [newRight, newWrong, old]) {
  check(
    `${appImage} and its signature are written`,
    existsSync(appImage) && existsSync(`${appImage}.sig`),
  );
}
const signature = Buffer.from(readFileSync(`${newRight}.sig`, 'utf8').trim(), 'base64').toString();
check(
  'the signature names the version it was signed for',
  signature.includes(`version:${NEW_VERSION}`),
  signature.split('\n').find((line) => line.startsWith('trusted comment')) ?? '',
);

const installed = join(WORK, 'installed', 'Teamwright.AppImage');
mkdirSync(join(WORK, 'installed'), { recursive: true });
copyFileSync(old, installed);
execFileSync('chmod', ['+x', installed]);
const dataDirectory = join(WORK, 'data');
// Each run starts on empty data, as a first start of the program does.
rmSync(dataDirectory, { recursive: true, force: true });
mkdirSync(dataDirectory, { recursive: true });

await new Promise((resolve) => server.listen(SERVER_PORT, '127.0.0.1', resolve));

let application;
try {
  application = await DesktopApplication.start(installed, dataDirectory, DRIVER_PORT, {
    environment: {
      APPIMAGE_EXTRACT_AND_RUN: '1',
      TEAMWRIGHT_RELEASES_ADDRESS: ADDRESS,
    },
  });
  let window = await application.openWindow();
  await openComputers(window);

  console.log('The notice');
  check(
    'the older program says it is 0.0.1',
    (await window.waitForText(`Teamwright ${OLD_VERSION}`)) !== '',
  );
  await window.clickText('Look now');
  await window.waitForText(`Install ${NEW_VERSION}`);
  check('it hears of 0.0.2 and offers one button to install it', true);
  const offer = await window.waitForText('From the releases of github.com/fouad1233/teamwright');
  check('it says where the update comes from', offer.includes('signed with the key'));
  check(
    'nothing was asked of the address of updates before the button',
    !served.requests.includes('/latest.json'),
  );
  await picture(window, '1-offer');

  console.log('An update signed with another key');
  served.manifest = manifestFor(newWrong);
  served.appImage = newWrong;
  const before = md5(installed);
  await window.clickText(`Install ${NEW_VERSION}`);
  const refused = await window.waitForText('The download was refused', 300_000);
  check(
    'the refusal reaches the manager in words',
    refused.includes('not signed with the key built into Teamwright'),
  );
  check(
    'the update was downloaded before it was refused',
    served.requests.includes(`/${APPIMAGE_NAME}`),
  );
  check('the installed program was not replaced', md5(installed) === before);
  await picture(window, '2-refused');

  console.log('The update signed with the right key');
  served.manifest = manifestFor(newRight);
  served.appImage = newRight;
  await window.clickText('Try again');
  let sawProgress = false;
  const deadline = Date.now() + 300_000;
  let shown = '';
  while (Date.now() < deadline) {
    shown = await window.evaluate("document.body?.innerText ?? ''").catch(() => '');
    if (/Downloading 0\.0\.2: [\d.]+ MB of [\d.]+ MB \(\d+%\)/.test(shown)) {
      if (!sawProgress) {
        await picture(window, '3-downloading');
      }
      sawProgress = true;
    }
    if (shown.includes('is installed and its signature checked')) {
      break;
    }
    await sleep(150);
  }
  check('the download showed its progress', sawProgress);
  check(
    'it says the new version is installed and checked',
    shown.includes(`${NEW_VERSION} is installed and its signature checked`),
  );
  check('the installed AppImage is now the new one', md5(installed) === md5(newRight));
  check(
    'it did not restart by itself',
    programsOn(dataDirectory).length > 0 && shown.includes('Restart now'),
  );
  await picture(window, '4-ready');

  console.log('The restart');
  const beforeRestart = new Set(programsOn(dataDirectory).map((program) => program.pid));
  await window.clickText('Restart now');
  let restarted = [];
  const restartDeadline = Date.now() + 120_000;
  while (Date.now() < restartDeadline) {
    restarted = programsOn(dataDirectory).filter((program) => !beforeRestart.has(program.pid));
    const oldOnes = programsOn(dataDirectory).filter((program) => beforeRestart.has(program.pid));
    if (restarted.length > 0 && oldOnes.length === 0) {
      break;
    }
    await sleep(500);
  }
  check(
    'the older program ended',
    programsOn(dataDirectory).every((program) => !beforeRestart.has(program.pid)),
  );
  check(
    'a new program started on the same data',
    restarted.length > 0,
    restarted.map((p) => p.exe).join(', '),
  );
  // An AppImage that unpacks itself does so into a folder named after the
  // checksum of its own file, so the folder says which file was started.
  check(
    'the program that started is the new AppImage',
    restarted.some((program) => program.exe.includes(md5(newRight))),
  );
  await sleep(5000);
  check(
    'it is still running five seconds later',
    restarted.length > 0 && restarted.every((program) => existsSync(`/proc/${program.pid}`)),
  );
  for (const program of restarted) {
    await stopProgram(program.pid);
  }

  console.log('The new version, opened again');
  window = await application.openWindow();
  await openComputers(window);
  const now = await window.waitForText(`Teamwright ${NEW_VERSION}`);
  check('the program that opens is 0.0.2', now.includes(`Teamwright ${NEW_VERSION}`));
  check('it no longer offers an update', !now.includes(`Install ${NEW_VERSION}`));
  await picture(window, '5-new-version');
} catch (error) {
  console.log(`FAIL  ${error.message}`);
  failures += 1;
} finally {
  await application?.stop();
  for (const program of programsOn(dataDirectory)) {
    await stopProgram(program.pid);
  }
  server.close();
}

console.log(failures === 0 ? '\nAll checks passed.' : `\n${failures} check(s) failed.`);
process.exit(failures === 0 ? 0 : 1);
