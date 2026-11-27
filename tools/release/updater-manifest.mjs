/**
 * Writes `latest.json`, the file the program's updater reads
 * ([decision 0045](../../docs/decisions/0045-installing-updates-from-inside.md)),
 * from the signatures of the four builds of one release.
 *
 * The release workflow runs it once, in a last job, after every build has
 * uploaded its files: the four jobs of the builds never write the file
 * themselves, so they cannot overwrite each other's part of it.
 *
 * Usage:
 *   node tools/release/updater-manifest.mjs <tag> <owner/repository> <folder-of-.sig-files> <output>
 *
 * It refuses (and writes nothing) unless all four computers have their
 * update, and unless every signature says it was signed for the version of
 * the tag: the program refuses an update whose signature names another
 * version (`requireSignedVersion`), so such a file would only break
 * updating.
 */

import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

/** The computers that must each have an update before the file is written. */
export const REQUIRED = ['darwin-aarch64', 'darwin-x86_64', 'linux-x86_64', 'windows-x86_64'];

/** The architecture a file name speaks of, as the updater names it. */
function architectureOf(name) {
  if (/aarch64|arm64/.test(name)) {
    return 'aarch64';
  }
  if (/x64|x86_64|amd64/.test(name)) {
    return 'x86_64';
  }
  return null;
}

/**
 * What an update file is for: its computer and the kind of installation
 * the updater names it by, or null for a file the updater does not use
 * (the `.deb` and the `.rpm`, which the system's package manager owns).
 */
export function platformOf(fileName) {
  const arch = architectureOf(fileName);
  if (arch === null) {
    return null;
  }
  if (fileName.endsWith('.app.tar.gz')) {
    return { os: 'darwin', arch, installer: 'app' };
  }
  if (fileName.endsWith('.AppImage')) {
    return { os: 'linux', arch, installer: 'appimage' };
  }
  if (fileName.endsWith('-setup.exe')) {
    return { os: 'windows', arch, installer: 'nsis' };
  }
  if (fileName.endsWith('.msi')) {
    return { os: 'windows', arch, installer: 'msi' };
  }
  return null;
}

/** The version a signature says it was signed for, read from its trusted comment. */
export function signedVersion(signature) {
  const text = Buffer.from(signature.trim(), 'base64').toString('utf8');
  const comment = text.split('\n').find((line) => line.startsWith('trusted comment:')) ?? '';
  const field = comment
    .slice('trusted comment:'.length)
    .trim()
    .split('\t')
    .find((part) => part.startsWith('version:'));
  return field ? field.slice('version:'.length) : null;
}

/**
 * The content of `latest.json` for a tag, from the signatures of its
 * files (a map from the name of the update file to its signature).
 *
 * Each computer is written twice: as `{os}-{arch}-{installer}`, which the
 * updater looks for first, and as `{os}-{arch}`, which it falls back to.
 * On Windows the fallback is the setup program, the installer the release
 * notes name first.
 */
export function manifestFor({ tag, repository, signatures, date = new Date() }) {
  const version = tag.replace(/^v/, '');
  if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$/.test(version)) {
    throw new Error(`the tag ${tag} is not a version`);
  }
  const platforms = {};
  const fallbacks = {};
  for (const [fileName, signature] of Object.entries(signatures)) {
    const platform = platformOf(fileName);
    if (platform === null) {
      continue;
    }
    const signed = signedVersion(signature);
    if (signed !== version) {
      throw new Error(`${fileName} was signed for version ${signed ?? 'none'}, not ${version}`);
    }
    const entry = {
      signature: signature.trim(),
      url: `https://github.com/${repository}/releases/download/${encodeURIComponent(tag)}/${encodeURIComponent(fileName)}`,
    };
    const base = `${platform.os}-${platform.arch}`;
    platforms[`${base}-${platform.installer}`] = entry;
    if (platform.installer !== 'msi' || fallbacks[base] === undefined) {
      fallbacks[base] = entry;
    }
  }
  const missing = REQUIRED.filter((base) => fallbacks[base] === undefined);
  if (missing.length > 0) {
    throw new Error(`no update for ${missing.join(', ')}; latest.json is not written`);
  }
  return {
    version,
    notes: `Teamwright ${version}. What changed: https://github.com/${repository}/releases/tag/${tag}`,
    pub_date: date.toISOString(),
    platforms: { ...fallbacks, ...platforms },
  };
}

/** Reads every `.sig` file of a folder, keyed by the name of the file it signs. */
function signaturesIn(folder) {
  const signatures = {};
  for (const name of readdirSync(folder)) {
    if (name.endsWith('.sig')) {
      signatures[name.slice(0, -'.sig'.length)] = readFileSync(join(folder, name), 'utf8');
    }
  }
  return signatures;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const [tag, repository, folder, output] = process.argv.slice(2);
  if (!output) {
    console.error(
      'usage: node tools/release/updater-manifest.mjs <tag> <owner/repository> <folder> <output>',
    );
    process.exit(2);
  }
  try {
    const manifest = manifestFor({
      tag,
      repository,
      signatures: signaturesIn(folder),
    });
    writeFileSync(output, `${JSON.stringify(manifest, null, 2)}\n`);
    console.log(
      `Wrote ${output} for ${manifest.version}: ${Object.keys(manifest.platforms).join(', ')}`,
    );
  } catch (error) {
    console.error(`latest.json was not written: ${error.message}`);
    process.exit(1);
  }
}
