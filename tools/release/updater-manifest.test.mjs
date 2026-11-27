// Run with: node --test tools/release
import assert from 'node:assert/strict';
import { test } from 'node:test';

import { manifestFor, platformOf, signedVersion } from './updater-manifest.mjs';

/** A signature as Tauri writes it: the text of a minisign signature, in base64. */
function signature(file, version) {
  const comment =
    version === null
      ? `timestamp:1\tfile:${file}`
      : `timestamp:1\tfile:${file}\tversion:${version}`;
  return Buffer.from(
    `untrusted comment: signature from tauri secret key\nRUQabc\ntrusted comment: ${comment}\nxyz\n`,
  ).toString('base64');
}

/** The update files of the four builds, as tauri-action names them. */
const FILES = [
  'Teamwright_aarch64.app.tar.gz',
  'Teamwright_x64.app.tar.gz',
  'Teamwright_0.3.0_amd64.AppImage',
  'Teamwright_0.3.0_x64-setup.exe',
  'Teamwright_0.3.0_x64_en-US.msi',
];

function signaturesOf(files, version = '0.3.0') {
  return Object.fromEntries(files.map((file) => [file, signature(file, version)]));
}

test('every computer is named the two ways the updater looks for it', () => {
  const manifest = manifestFor({
    tag: 'v0.3.0',
    repository: 'o/r',
    signatures: signaturesOf(FILES),
    date: new Date('2026-11-27T12:00:00Z'),
  });
  assert.equal(manifest.version, '0.3.0');
  assert.equal(manifest.pub_date, '2026-11-27T12:00:00.000Z');
  assert.deepEqual(Object.keys(manifest.platforms).sort(), [
    'darwin-aarch64',
    'darwin-aarch64-app',
    'darwin-x86_64',
    'darwin-x86_64-app',
    'linux-x86_64',
    'linux-x86_64-appimage',
    'windows-x86_64',
    'windows-x86_64-msi',
    'windows-x86_64-nsis',
  ]);
  assert.equal(
    manifest.platforms['darwin-aarch64'].url,
    'https://github.com/o/r/releases/download/v0.3.0/Teamwright_aarch64.app.tar.gz',
  );
  // On Windows the fallback is the setup program.
  assert.equal(
    manifest.platforms['windows-x86_64'].url,
    manifest.platforms['windows-x86_64-nsis'].url,
  );
});

test('nothing is written while a computer has no update', () => {
  assert.throws(
    () =>
      manifestFor({
        tag: 'v0.3.0',
        repository: 'o/r',
        signatures: signaturesOf(FILES.slice(1)),
      }),
    /no update for darwin-aarch64/,
  );
});

test('a signature made for another version stops the file', () => {
  const signatures = {
    ...signaturesOf(FILES),
    [FILES[2]]: signature(FILES[2], '0.2.9'),
  };
  assert.throws(
    () => manifestFor({ tag: 'v0.3.0', repository: 'o/r', signatures }),
    /signed for version 0\.2\.9/,
  );
  const unsigned = {
    ...signaturesOf(FILES),
    [FILES[2]]: signature(FILES[2], null),
  };
  assert.throws(
    () => manifestFor({ tag: 'v0.3.0', repository: 'o/r', signatures: unsigned }),
    /none/,
  );
});

test('a tag that is not a version is refused', () => {
  assert.throws(
    () => manifestFor({ tag: 'website-film', repository: 'o/r', signatures: {} }),
    /not a version/,
  );
});

test('the packages of the Linux system are left out', () => {
  assert.equal(platformOf('Teamwright_0.3.0_amd64.deb'), null);
  assert.equal(platformOf('Teamwright-0.3.0-1.x86_64.rpm'), null);
  assert.deepEqual(platformOf('Teamwright_0.3.0_amd64.AppImage'), {
    os: 'linux',
    arch: 'x86_64',
    installer: 'appimage',
  });
});

test('the signed version is read from the trusted comment', () => {
  assert.equal(signedVersion(signature('a', '1.2.3')), '1.2.3');
  assert.equal(signedVersion(signature('a', null)), null);
});
