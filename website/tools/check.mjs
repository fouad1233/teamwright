/**
 * Checks the website before it is committed or published.
 *
 *   node website/tools/check.mjs
 *
 * Nothing is needed but Node.js. The root `npm run check` runs it, and so
 * does continuous integration. It fails when:
 *
 * - a part of index.html that is written from film.json or from the
 *   questions is out of date (run `node website/tools/write-page.mjs`);
 * - the chapters or the lines in film.json do not fit together;
 * - a file that a page, the stylesheet or the manifest names is missing;
 * - structured data (JSON-LD) does not parse;
 * - a page asks for anything from another site;
 * - the headings skip a level, there is not exactly one h1, a picture has
 *   no alternative text or no size;
 * - a weight budget is exceeded.
 *
 * The film's media (website/media/film) is not in git and is not checked
 * here: the page works without it.
 */

import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { FILM_DATA, PAGE, WEBSITE, readFilm, writtenPage } from './write-page.mjs';

const ADDRESS = 'https://fouad1233.github.io/teamwright/';

/* Budgets, in bytes. */
const PICTURE_BUDGET = 150 * 1024;
/* What the first view needs: the page, its style, its script, the three
   preloaded fonts and the first still. */
const FIRST_VIEW_BUDGET = 260 * 1024;
/* Everything the published site holds in git, pictures included. */
const SITE_BUDGET = 4 * 1024 * 1024;

const failures = [];
const fail = (message) => failures.push(message);

/* ---- The written parts are up to date. ---- */

const page = readFileSync(PAGE, 'utf8');
if (writtenPage() !== page) {
  fail('index.html is out of date with film.json or its questions: run node website/tools/write-page.mjs');
}

/* ---- film.json fits together. ---- */

const film = readFilm();
const chapters = film.chapters;
if (chapters[0].start !== 0) fail('film.json: the first chapter does not start at 0');
for (let index = 1; index < chapters.length; index += 1) {
  if (chapters[index].start !== chapters[index - 1].end) {
    fail(`film.json: chapter "${chapters[index].id}" does not start where "${chapters[index - 1].id}" ends`);
  }
}
if (chapters.at(-1).end !== film.film.seconds) fail('film.json: the last chapter does not end with the film');
const ids = new Set();
for (const chapter of chapters) {
  if (ids.has(chapter.id)) fail(`film.json: two chapters are called "${chapter.id}"`);
  ids.add(chapter.id);
  if (!(chapter.start <= chapter.stillAt && chapter.stillAt < chapter.end)) {
    fail(`film.json: the still of "${chapter.id}" is not inside the chapter`);
  }
  // The chapter in the page says the same as the chapter in the data.
  const element = page.match(new RegExp(`<section class="chapter" id="${chapter.id}"[^>]*>`));
  if (!element) fail(`index.html has no chapter "${chapter.id}"`);
  else if (!element[0].includes(`data-start="${chapter.start}" data-end="${chapter.end}"`)) {
    fail(`index.html: the times of chapter "${chapter.id}" differ from film.json`);
  }
}
const chaptersInPage = [...page.matchAll(/<section class="chapter" id="([^"]+)"/g)].map((match) => match[1]);
if (chaptersInPage.join() !== chapters.map((chapter) => chapter.id).join()) {
  fail(`index.html has the chapters ${chaptersInPage.join(', ')}; film.json has ${chapters.map((c) => c.id).join(', ')}`);
}
let previousStart = -1;
for (const line of film.lines) {
  if (line.start < previousStart) fail(`film.json: "${line.text}" is not in the order of the film`);
  previousStart = line.start;
  if (line.end !== null && line.end <= line.start) fail(`film.json: "${line.text}" ends before it starts`);
  if (line.start >= film.film.seconds) fail(`film.json: "${line.text}" starts after the film`);
}

/* ---- The pages. ---- */

const PAGES = ['index.html', '404.html'];
const SITE_ROOT = new URL(ADDRESS).pathname; // "/teamwright/"

/** The file of the site that an address in a page names, or null if the address is elsewhere. */
function localFile(address) {
  if (/^(mailto:|tel:|data:|#)/.test(address)) return null;
  let path = address.split('#')[0].split('?')[0];
  if (path === '') return null;
  if (path.startsWith(ADDRESS)) path = path.slice(ADDRESS.length);
  else if (/^[a-z]+:\/\//i.test(path)) return null;
  else if (path.startsWith(SITE_ROOT)) path = path.slice(SITE_ROOT.length);
  else if (path.startsWith('/')) return { missing: `${address} is outside the site` };
  if (path === '' || path === './' || path.endsWith('/')) path += 'index.html';
  return { file: join(WEBSITE, ...path.replace(/^\.\//, '').split('/')) };
}

function checkAddress(address, where) {
  const local = localFile(address);
  if (!local) return null;
  if (local.missing) {
    fail(`${where}: ${local.missing}`);
    return null;
  }
  // The film's media is fetched at publishing, and is not in git.
  if (local.file.startsWith(join(WEBSITE, 'media') + sep)) return null;
  if (!existsSync(local.file)) fail(`${where}: ${address} does not exist`);
  return local.file;
}

const firstView = new Set([PAGE]);

for (const name of PAGES) {
  const html = readFileSync(join(WEBSITE, name), 'utf8');

  if (!/<html lang="[a-z]{2}/.test(html)) fail(`${name}: the page has no language`);

  // Addresses in attributes.
  for (const [, attribute, value] of html.matchAll(/\s(href|src|content|poster|srcset|imagesrcset|data-still)="([^"]*)"/g)) {
    const addresses =
      attribute.includes('srcset') || attribute === 'data-still'
        ? value.split(',').map((part) => part.trim().split(/\s+/)[0])
        : [value];
    for (const address of addresses) {
      if (attribute === 'content' && !/^(https?:|\.?\/|[\w-]+\/[\w./-]+\.\w+$)/.test(address)) continue;
      checkAddress(address, name);
    }
  }

  // Nothing is loaded from another site: scripts, styles, pictures, fonts.
  for (const [tag] of html.matchAll(/<(script|link|img|source|video|iframe)\b[^>]*>/g)) {
    if (/^<link\b/.test(tag) && /rel="canonical"/.test(tag)) continue;
    const source = tag.match(/\s(?:src|href)="(https?:\/\/[^"]+)"/);
    if (source) fail(`${name}: ${tag.slice(0, 60)}... loads ${source[1]}, from another site`);
  }

  // Structured data parses.
  for (const [, json] of html.matchAll(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/g)) {
    try {
      JSON.parse(json);
    } catch (error) {
      fail(`${name}: structured data does not parse: ${error.message}`);
    }
  }

  // One h1, and no level skipped.
  const levels = [...html.matchAll(/<h([1-6])\b/g)].map((match) => Number(match[1]));
  if (levels.filter((level) => level === 1).length !== 1) fail(`${name}: there must be exactly one h1`);
  levels.forEach((level, index) => {
    if (index > 0 && level > levels[index - 1] + 1) fail(`${name}: an h${level} follows an h${levels[index - 1]}`);
  });

  // Every picture says what it shows, and how large it is.
  for (const [tag] of html.matchAll(/<img\b[^>]*>/g)) {
    if (!/\salt="/.test(tag)) fail(`${name}: a picture has no alternative text: ${tag.slice(0, 80)}`);
    if (!/\swidth="\d+"/.test(tag) || !/\sheight="\d+"/.test(tag)) fail(`${name}: a picture has no size: ${tag.slice(0, 80)}`);
  }
}

// The files the first view of index.html needs.
for (const [tag] of page.matchAll(/<link rel="(?:stylesheet|preload)"[^>]*>/g)) {
  const address = tag.match(/href="([^"]+)"/)[1];
  firstView.add(join(WEBSITE, ...address.split('/')));
}
for (const [, address] of page.matchAll(/<script src="([^"]+)"/g)) firstView.add(join(WEBSITE, ...address.split('/')));

/* ---- The stylesheet and the manifest. ---- */

const stylesheet = join(WEBSITE, 'styles', 'site.css');
for (const [, address] of readFileSync(stylesheet, 'utf8').matchAll(/url\('?(\.\.\/[^')]+)'?\)/g)) {
  if (!existsSync(join(WEBSITE, 'styles', ...address.split('/')))) fail(`site.css: ${address} does not exist`);
}
try {
  const manifest = JSON.parse(readFileSync(join(WEBSITE, 'site.webmanifest'), 'utf8'));
  for (const icon of manifest.icons) checkAddress(icon.src, 'site.webmanifest');
} catch (error) {
  fail(`site.webmanifest does not parse: ${error.message}`);
}
for (const name of ['robots.txt', 'sitemap.xml']) {
  if (!readFileSync(join(WEBSITE, name), 'utf8').includes(ADDRESS)) fail(`${name} does not name ${ADDRESS}`);
}
if (!readFileSync(FILM_DATA, 'utf8').includes(`"address": "${ADDRESS}"`)) fail(`film.json does not name ${ADDRESS}`);

/* ---- Weights. ---- */

function filesUnder(folder) {
  return readdirSync(folder, { withFileTypes: true }).flatMap((entry) => {
    const path = join(folder, entry.name);
    if (entry.isDirectory()) return ['tools', 'media'].includes(entry.name) && folder === WEBSITE ? [] : filesUnder(path);
    return entry.name === 'README.md' && folder === WEBSITE ? [] : [path];
  });
}

const siteFiles = filesUnder(WEBSITE);
let siteBytes = 0;
for (const file of siteFiles) {
  const size = statSync(file).size;
  siteBytes += size;
  if (/\.(webp|png|jpg|svg|ico)$/.test(file) && size > PICTURE_BUDGET) {
    fail(`${relative(WEBSITE, file)} is ${Math.round(size / 1024)} kB, over the ${PICTURE_BUDGET / 1024} kB of a picture`);
  }
}
const firstViewBytes = [...firstView].reduce((sum, file) => sum + (existsSync(file) ? statSync(file).size : 0), 0);
if (firstViewBytes > FIRST_VIEW_BUDGET) {
  fail(`the first view weighs ${Math.round(firstViewBytes / 1024)} kB, over its ${FIRST_VIEW_BUDGET / 1024} kB`);
}
if (siteBytes > SITE_BUDGET) {
  fail(`the site weighs ${Math.round(siteBytes / 1024)} kB, over its ${SITE_BUDGET / 1024} kB`);
}

if (failures.length) {
  console.error(`The website check failed:\n${failures.map((message) => `  - ${message}`).join('\n')}`);
  process.exit(1);
}
console.log(
  `The website is in order: ${chapters.length} chapters, ${film.lines.length} lines, ` +
    `first view ${Math.round(firstViewBytes / 1024)} kB of ${FIRST_VIEW_BUDGET / 1024}, ` +
    `site ${Math.round(siteBytes / 1024)} kB of ${SITE_BUDGET / 1024} in ${siteFiles.length} files.`,
);
