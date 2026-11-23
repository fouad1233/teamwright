/**
 * Writes the parts of website/index.html that are made from something else.
 *
 *   node website/tools/write-page.mjs                 writes them
 *   node website/tools/write-page.mjs --without-film  writes them for a site
 *                                                     that has no film media
 *
 * - The chapters of the film, the rail that names them, the first picture
 *   of the stage, the address of the film's media and the structured data
 *   of the film (VideoObject) come from website/film.json. Its numbers are written there by
 *   prepare-film.py from the film's own data file, never by hand.
 * - The structured data of the questions (FAQPage) comes from the section
 *   "Questions" of the page itself, so that the two always say the same.
 *
 * Each part stands between two comments, `<!-- NAME:begin -->` and
 * `<!-- NAME:end -->`. Everything else in the page is written by hand.
 * check.mjs uses the same functions to fail when a part is out of date.
 *
 * A chapter's still is a picture of the program (images/<picture>-960.webp
 * and its other widths), the mark ("mark"), or the film's own frame
 * ("film": media/film/stills/<chapter>.webp, written by prepare-film.py),
 * for a chapter in which no picture of the program is seen.
 *
 * With --without-film the page does not point at website/media/film and
 * does not describe a video; a chapter whose still is the film's frame
 * shows the mark instead. The publishing workflow uses it when there is
 * no film to publish (see website/README.md).
 */

import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

export const WEBSITE = join(dirname(fileURLToPath(import.meta.url)), '..');
export const PAGE = join(WEBSITE, 'index.html');
export const FILM_DATA = join(WEBSITE, 'film.json');

/** The widths in which every picture of the program exists (make-images.py). */
const PICTURE_WIDTHS = [960, 1600, 2400];

export function readFilm() {
  return JSON.parse(readFileSync(FILM_DATA, 'utf8'));
}

export function escapeHtml(text) {
  return text
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');
}

function twoDigits(number) {
  return String(number).padStart(2, '0');
}

/** What a still of the film's own frame becomes when there is no film. */
const MARK_ALT = 'The mark of Teamwright: a letter T laid out of five blocks, the one that holds the others in gold.';

/** The picture a chapter shows, with the film or without it. */
function pictureOf(chapter, withoutFilm) {
  const { picture, alt } = chapter.still;
  if (picture === 'film' && withoutFilm) return { picture: 'mark', alt: MARK_ALT };
  return { picture, alt };
}

const filmStill = (chapter) => `media/film/stills/${chapter.id}.webp`;

/** The lines of the film that begin inside a chapter. */
export function linesOf(film, chapter) {
  return film.lines.filter((line) => line.start >= chapter.start && line.start < chapter.end);
}

function stillOf(chapter, index, withoutFilm) {
  const { picture, alt } = pictureOf(chapter, withoutFilm);
  const loading = index === 0 ? '' : ' loading="lazy"';
  if (picture === 'film') {
    return `<img src="${filmStill(chapter)}" width="1920" height="1080" alt="${escapeHtml(alt)}"${loading} decoding="async">`;
  }
  if (picture === 'mark') {
    return `<img src="images/icons/mark.svg" width="1024" height="1024" alt="${escapeHtml(alt)}"${loading} decoding="async">`;
  }
  const srcset = PICTURE_WIDTHS.map((width) => `images/${picture}-${width}.webp ${width}w`).join(', ');
  return (
    `<img src="images/${picture}-960.webp" srcset="${srcset}" sizes="(min-width: 1200px) 1100px, 100vw" ` +
    `width="1600" height="935" alt="${escapeHtml(alt)}"${loading} decoding="async">`
  );
}

/** The image a chapter shows on the stage before a frame of the film is in. */
function stageStillOf(chapter, withoutFilm) {
  const { picture } = pictureOf(chapter, withoutFilm);
  if (picture === 'film') return filmStill(chapter);
  if (picture === 'mark') return 'images/icons/mark.svg';
  return `images/${picture}-960.webp 960w, images/${picture}-1600.webp 1600w`;
}

/** The first picture of the stage, and the line in the head that asks for it early. */
export function firstStill(film, withoutFilm) {
  const chapter = film.chapters[0];
  const { picture } = pictureOf(chapter, withoutFilm);
  if (picture === 'film' || picture === 'mark') {
    const source = picture === 'film' ? filmStill(chapter) : 'images/icons/mark.svg';
    const [width, height] = picture === 'film' ? [1920, 1080] : [1024, 1024];
    return {
      preload: `<link rel="preload" as="image" href="${source}" fetchpriority="high">`,
      stage: `    <img class="stage-still" data-stage-still src="${source}" width="${width}" height="${height}" alt="" fetchpriority="high">`,
    };
  }
  const srcset = `images/${picture}-960.webp 960w, images/${picture}-1600.webp 1600w`;
  return {
    preload: `<link rel="preload" as="image" href="images/${picture}-960.webp" imagesrcset="${srcset}" imagesizes="100vw" fetchpriority="high">`,
    stage: `    <img class="stage-still" data-stage-still src="images/${picture}-960.webp" srcset="${srcset}" sizes="100vw" width="1600" height="935" alt="" fetchpriority="high">`,
  };
}

export function chaptersHtml(film, withoutFilm = false) {
  return film.chapters
    .map((chapter, index) => {
      const lines = linesOf(film, chapter)
        .map(
          (line) =>
            `      <li data-start="${line.start}" data-end="${line.end ?? ''}" data-type="${line.type}" data-weight="${line.weight}" data-size="${line.size}" data-y="${line.y}" data-tone="${line.tone}">${escapeHtml(line.text)}</li>`,
        )
        .join('\n');
      return [
        `<section class="chapter" id="${chapter.id}" aria-labelledby="${chapter.id}-title" data-chapter data-start="${chapter.start}" data-end="${chapter.end}" data-still="${stageStillOf(chapter, withoutFilm)}" style="--seconds: ${chapter.end - chapter.start}">`,
        `  <figure class="chapter-still">`,
        `    ${stillOf(chapter, index, withoutFilm)}`,
        `  </figure>`,
        `  <div class="chapter-text">`,
        `    <h3 class="chapter-title" id="${chapter.id}-title"><span class="label">${twoDigits(index + 1)} · ${chapter.start} to ${chapter.end} s</span> ${escapeHtml(chapter.title)}</h3>`,
        `    <p class="chapter-seen">${escapeHtml(chapter.seen)}</p>`,
        `    <ol class="chapter-lines" aria-label="Lines of the film">`,
        lines,
        `    </ol>`,
        `  </div>`,
        `</section>`,
      ].join('\n');
    })
    .join('\n');
}

export function railHtml(film) {
  const items = film.chapters
    .map(
      (chapter, index) =>
        `      <li><a href="#${chapter.id}" data-rail-link><span class="rail-number">${twoDigits(index + 1)}</span> <span class="rail-name">${escapeHtml(chapter.title)}</span></a></li>`,
    )
    .join('\n');
  return `    <ol>\n${items}\n    </ol>`;
}

function isoDuration(seconds) {
  const minutes = Math.floor(seconds / 60);
  const rest = seconds - minutes * 60;
  return `PT${minutes ? `${minutes}M` : ''}${rest}S`;
}

export function videoObject(film) {
  const { address } = film.site;
  const data = {
    '@context': 'https://schema.org',
    '@type': 'VideoObject',
    name: film.film.name,
    description: film.film.description,
    thumbnailUrl: [address + film.film.thumbnail],
    uploadDate: film.film.published,
    duration: isoDuration(film.film.seconds),
    contentUrl: `${address}media/film/${film.film.video}`,
    width: film.film.width,
    height: film.film.height,
    inLanguage: 'en',
    transcript: film.lines.map((line) => line.text).join(' '),
    hasPart: film.chapters.map((chapter) => ({
      '@type': 'Clip',
      name: chapter.title,
      startOffset: chapter.start,
      endOffset: chapter.end,
      url: `${address}#${chapter.id}`,
    })),
  };
  return `<script type="application/ld+json">\n${JSON.stringify(data, null, 2)}\n</script>`;
}

const ENTITIES = { '&amp;': '&', '&lt;': '<', '&gt;': '>', '&quot;': '"', '&#39;': "'", '&nbsp;': ' ' };

export function plainText(html) {
  return html
    .replace(/<[^>]+>/g, '')
    .replace(/&[a-z#0-9]+;/g, (entity) => ENTITIES[entity] ?? entity)
    .replace(/\s+/g, ' ')
    .trim();
}

/** The questions and answers of the section "Questions", as plain text. */
export function questionsOf(page) {
  const section = page.match(/<section class="part" id="questions"[\s\S]*?<\/section>/);
  if (!section) throw new Error('index.html has no section with the id "questions"');
  const pairs = [...section[0].matchAll(/<h3>([\s\S]*?)<\/h3>\s*<div class="answer">([\s\S]*?)<\/div>/g)];
  return pairs.map(([, question, answer]) => ({ question: plainText(question), answer: plainText(answer) }));
}

export function questionsData(page) {
  const data = {
    '@context': 'https://schema.org',
    '@type': 'FAQPage',
    mainEntity: questionsOf(page).map(({ question, answer }) => ({
      '@type': 'Question',
      name: question,
      acceptedAnswer: { '@type': 'Answer', text: answer },
    })),
  };
  return `<script type="application/ld+json">\n${JSON.stringify(data, null, 2)}\n</script>`;
}

const MEDIA = '<meta name="teamwright-film-media" content="media/film/manifest.json">';

/** Every part, by the name of its comments. */
export function parts(film, page, { withoutFilm = false } = {}) {
  return {
    'film:media': withoutFilm ? '' : MEDIA,
    'film:first-still': firstStill(film, withoutFilm).preload,
    'film:stage-still': firstStill(film, withoutFilm).stage,
    'film:video-object': withoutFilm ? '' : videoObject(film),
    'film:rail': railHtml(film),
    'film:chapters': chaptersHtml(film, withoutFilm),
    'page:questions-data': questionsData(page),
  };
}

export function fillParts(page, filled) {
  let result = page;
  for (const [name, content] of Object.entries(filled)) {
    const pattern = new RegExp(`(<!-- ${name}:begin -->\\n)[\\s\\S]*?(<!-- ${name}:end -->)`);
    if (!pattern.test(result)) throw new Error(`index.html has no part "${name}"`);
    result = result.replace(pattern, (_, begin, end) => `${begin}${content ? `${content}\n` : ''}${end}`);
  }
  return result;
}

/** The page as it should be. */
export function writtenPage(options) {
  const page = readFileSync(PAGE, 'utf8');
  return fillParts(page, parts(readFilm(), page, options));
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const withoutFilm = process.argv.includes('--without-film');
  writeFileSync(PAGE, writtenPage({ withoutFilm }));
  console.log(`Wrote the parts of ${PAGE}${withoutFilm ? ', without the film' : ''}`);
}
