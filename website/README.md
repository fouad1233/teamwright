# The website

The public page of Teamwright, served by GitHub Pages at
<https://fouad1233.github.io/teamwright/>. Its upper part is the film: the
reader scrolls through it. Below the film it explains the project to
somebody who has never heard of it.

It is one page of plain HTML, CSS and JavaScript. There is no framework and
no build step: what is in this folder is what is published, apart from
`tools/` and this file. One part of the published site is not in this
folder: the demo at `demo/`, which the workflow builds from the interface
(see "The demo" below). Everything a search engine or a reader without
JavaScript needs is in `index.html` as written. Why it is made this way:
[decision 0023](../docs/decisions/0023-a-website-without-a-framework.md).

## What is where

| Path | What |
|---|---|
| `index.html` | The page. Some parts of it are written by a tool, see below |
| `404.html` | The page for an address that does not exist |
| `film.json` | The film on the site: its chapters with their names, texts and stills, and the facts of the video. Its numbers (the length, the seconds of each chapter, the lines) are written from the film's own data by `prepare-film.py`; the page takes them from here only |
| `styles/site.css` | The look. The program's design tokens, at night, and the site's own |
| `scripts/film.js` | Scrubbing, the lines over the film, the rail of chapters, "Watch the film" |
| `fonts/` | Instrument Sans, Instrument Serif and IBM Plex Mono, the Latin subsets as `woff2` (from `@fontsource` 5.3.0), with their licences (SIL Open Font License 1.1) |
| `images/` | Pictures of the program, the picture for shared links (`share.png`), icons |
| `media/film/` | The film's frames, stills and video. **Not in git** (see "The film") |
| `robots.txt`, `sitemap.xml`, `site.webmanifest`, `favicon.ico` | For search engines and browsers |
| `tools/` | The tools below. Not published |
| `demo/` | **Not in git.** The interface built for the browser, put here by the workflow (see "The demo") |

## Run it

```bash
node website/tools/serve.mjs          # or: npm run website
```

Then open <http://localhost:4321/teamwright/>. The site lives under
`/teamwright/` as on GitHub Pages. The demo is at
<http://localhost:4321/teamwright/demo/> once it is built with
`npm run build:demo --workspace @teamwright/desktop`: the server takes it
from `apps/desktop/dist-demo`. The server needs nothing but Node.js and
answers range requests, which the video needs. In `.claude/launch.json` it
is the entry `teamwright-website`.

To look at it without a window, with a headless Microsoft Edge:

```bash
node website/tools/look.mjs <folder>                       # 1440 by 900
node website/tools/look.mjs <folder> --phone --size 360x780
node website/tools/look.mjs <folder> --reduce-motion
node website/tools/look.mjs <folder> --no-script
node website/tools/look.mjs <folder> --mbps 20             # a slower network
```

It takes a picture at many moments of the film and at the sections below
it, and prints what the browser reported: errors, when the film could be
scrubbed, the bytes it loaded.

## Check it

```bash
node website/tools/check.mjs
```

The root `npm run check` runs it first, and so does continuous integration.
It fails when a part of the page is out of date with `film.json`, when the
chapters of the film do not fit together, when a file the pages name is
missing (a link into `demo/` is allowed while the workflow builds the demo
there), when structured data does not parse, when a page loads anything
from another site, when the headings skip a level or a picture has no
alternative text, and when a weight budget is exceeded:

| Budget | Limit | Today |
|---|---|---|
| One picture | 150 kB | 116 kB at most (`advisor-proposal-2400.webp`) |
| The first view: page, style, script, three fonts, first still | 260 kB | 163 kB |
| Everything published from git | 4 MB | 2.9 MB |

## Change the page

Write `index.html` by hand, except the parts between `<!-- NAME:begin -->`
and `<!-- NAME:end -->`. Those are written by

```bash
node website/tools/write-page.mjs
```

from `film.json` (the chapters, the rail, the structured data of the
video, the address of the film's media) and from the section "Questions" of
the page itself (the structured data of the questions). Change the film's
times, chapters or lines in `film.json` only, then run the tool.

Every claim on the page must be true of the program as it is built: take
features from the finished marks of the [roadmap](../docs/product/roadmap.md)
and from the READMEs, and say what is not built.

The pictures of the program come from `docs/screenshots` (the dark ones).
After the screenshots are taken again, make the site's pictures again:

```bash
python website/tools/make-images.py
```

It needs Python 3 and Pillow, and writes every picture in three widths as
WebP within the budget, the picture for shared links, and the icons from
the program's own mark.

## The film

The film is made outside git. Its frames and video do not go into git
either: a thousand pictures and a video would weigh the repository down for
ever. They are prepared into `website/media/film/` (ignored by git) and
published from a release.

### Prepare it

Putting a new version of the film in is one run of one tool, from the root
of the repository. The film on the site since 23 November 2026 is the third
film, in its dark look (the site has one ground, a dark one). `F` is its
folder, `teamwright-film` beside the repository, outside git; its
`cut/tools/web.py` writes the data and the frames (see `cut/README.md`
there):

```bash
python3 -I $F/cut/tools/web.py dark $F/cut/web-dark
python website/tools/prepare-film.py --web-json $F/cut/web-dark/web.json \
    $F/cut/web-dark/frames \
    --video $F/cut/teamwright-film-3-dark.mp4 \
    --small-video $F/cut/teamwright-film-3-dark-small.mp4 \
    --zip $F/website-film.zip
```

`web.json` is written by the film's assembly from the same tables the film
is made from: the frame rate, the number of frames, every shot with its
seconds, and every line with its moments and how it is set. It is the only
source of the film's numbers. From it the tool first writes the numbers in
`film.json` (the length, the seconds of each chapter, the lines) and the
parts of `index.html` made from them. A chapter of the site begins with a
shot (`fromShot`) and runs to the next chapter, so a shot that is added to
the film lengthens the chapter it falls in. The tool says which chapter
each shot falls in. Without the folder of frames it writes only the data
and the page.

The frames are the film without its lines, one PNG for each frame. The
lines are text on the page, laid over the picture at their moments and in
their type, size and place from `web.json`. The name on the end card is
part of the picture, and is not a line. From the frames the tool writes:

- `manifest.json`, which the page reads first;
- `large/pack-NN.bin` (1920 pixels wide) and `small/pack-NN.bin` (1280):
  every second frame (12 a second for a film at 24 frames a second, 15 for
  the third film at 30; the page reads the rate from the manifest) as
  WebP, laid end to end in packs of about 1.5 MB. The first packs hold every sixteenth frame of the film;
  the later ones fill in between;
- `stills/<chapter>.webp`: the frame `stillAfter` seconds into each
  chapter;
- `teamwright-film.mp4` and `teamwright-film-small.mp4`, copied;
- with `--zip`, all of it in one file, `website-film.zip`.

It took about six minutes for 1728 frames at 1920 by 1080 on the computer
it was written on, and about six minutes for the 2,746 pictures of the
third film on the computer it was prepared on. It needs no ffmpeg. Run
`npm run check` after it, and commit `film.json` and `index.html` if
they changed.

### Publish it

1. Prepare the film with `--zip website-film.zip`, as above.
2. Make a release of the repository with the tag **`website-film`** and
   attach **`website-film.zip`** to it. A new version of the film replaces
   that file in the same release.
3. Run the workflow "Website" (`.github/workflows/pages.yml`), or push a
   change under `website/`.

The workflow downloads `website-film.zip` from the release `website-film`
and unpacks it into `website/media/film/` before it publishes. If there is
no such release, it publishes the page without the film:
`write-page.mjs --without-film` takes out the address of the media and the
structured data of the video, and every chapter shows its still and its
text.

The workflow runs on a push to `main` that changes `website/`, the
workflow, or what the demo is built from (see "The demo"), when a release
is published or edited, and by hand. Before its
first run, GitHub Pages must be set to deploy from GitHub Actions
(Settings, Pages, Source). That is the owner's step.

Set `published` in `film.json` to the day the film is put in the release.
It is the upload date in the structured data of the video.

Publish the zip before the page: `film.json` and the zip belong together
(the chapters' seconds and stills), so a page with new chapters over the
old film's frames is wrong until the zip is replaced.

### The stills of the chapters

Each chapter in `film.json` names its still (`still.picture`) and says what
it shows (`still.alt`). The still is shown on the stage until a frame of the
film is there, and in the chapter itself when the film does not scrub
(reduced motion, no script). It is one of:

- a picture of the program, `images/<picture>-960.webp` and its other
  widths: for a chapter in which that view of the program is seen;
- `mark`, the mark of Teamwright;
- `film`, the film's own frame, `media/film/stills/<chapter>.webp`, for a
  chapter in which no picture of the program is seen (the desk, the
  themes, the device). Without the film's media (`--without-film`) it
  becomes the mark.

The first chapter's still is also the first picture of the stage and is
asked for early in the head; `write-page.mjs` writes both. With a still
from the film it is not in git, so the first view's budget in `check.mjs`
counts it only where the film's media are prepared (the third film's first
still is 39 kB; the first view is then 203 kB of its 260).

## How the scrubbing works

The stage is a `position: sticky` box that fills the window. The chapters
are empty boxes under it, each as tall as its seconds times `--film-pace`
(34 % of the window's height for one second of film). `film.js` turns the
scroll position into a second of the film, draws the nearest frame it has
on a canvas, and lays the lines of that moment over it, word by word as in
the film. Scrolling is ordinary scrolling: the page never takes the wheel,
the keys or a touch away from the browser, except that within the film the
arrow keys and Page Up and Page Down go from chapter to chapter, as the rail
and a sideways swipe do. Going to a chapter scrolls there over half a
second to a second and a half, so the film runs there.

- **Frames on a canvas, not a video.** Setting the time of a video from the
  scroll stutters in most browsers, because a video can only start drawing
  at its key frames. Pictures can be drawn in any order at once.
- **Every second frame.** Twelve a second scrub smoothly, and halve the
  weight.
- **Coarse first.** The first packs hold every sixteenth frame, so that the
  whole film can be scrubbed soon, in steps of two thirds of a second. Then
  the packs in between are loaded, those nearest to where the reader is
  first, three at a time.
- **Packs, not files.** The pictures (864 for 72 seconds) lie in 8 to 30
  files rather than one request each. The manifest says where each picture begins.
- **Kept compressed.** The pictures are kept as they came. Only those near
  the reader are decoded, with `createImageBitmap`, off the main thread: at
  most 16 at 1920 pixels (about 130 MB) or 24 at 1280 (about 90 MB). The
  farthest are let go first.
- **Sharp everywhere.** The large set is as wide as the film itself. A
  screen narrower than 700 pixels, or a reader whose browser asks to save
  data, gets the 1280-pixel set, which is still wider than a phone's
  screen. The video is the film at its full size for everybody; only a
  reader who saves data gets the small one. (Until 14 November 2026 the
  sets were 1280 and 640 pixels and phones got the small video; the owner
  found that soft.)
- **Never in the way of the first picture.** The frames are asked for only
  after the page has loaded. Until a frame is there, each chapter shows a
  still: a picture of the program, framed, or with the film's media, the
  chapter's own still from the film.
- **Reduced motion.** A reader who asked the browser to reduce motion gets
  the chapters as stills with their text, one under the other, and "Watch
  the film". A line in the head of the page decides this before anything is
  drawn, so nothing moves after it has appeared.

How the picture fits the window: from a window one and a fifth times as
wide as it is high to one twice as wide, the film fills it, and its sides
or its top and foot are cut. A wider window shows the film's full height
with the ground at the sides. A taller window, such as a phone held
upright, shows the whole width of the film near the top, and the lines
under it.

## What was measured

On 13 November 2026, on the computer the site was written on (Windows 11,
a processor with its graphics built in, 16 GB, busy with other work at the
time), with `look.mjs`: a headless Edge 157 on a fresh profile, the cache
off, served by `serve.mjs`. With the film of that day, in which most 3D
shots were not rendered yet and stood as dark frames. One run each, so
take the times as rough:

| | Desktop, 1440 by 900 | Phone, 360 by 780 and 390 by 844 |
|---|---|---|
| Set | large, 8 packs, 10.4 MB | small, 5 packs, 3.5 MB |
| Every sixteenth frame | 1.3 MB | 0.4 MB |
| First paint, no limit on the network | 1.3 s | 1.4 s |
| Film can be scrubbed, no limit | 1.5 s | 1.2 s |
| First paint, and film can be scrubbed, at 20 Mbit/s | 1.6 s, 2.6 s | |
| First paint, and film can be scrubbed, at 5 Mbit/s | | 1.3 s, 3.0 s |
| Script memory | 1 MB | 1 MB |

A first paint above a second on a local server is slow; it was not looked
into further. The computer was rendering the film at the same time.

These numbers are of the sets as they were then, 1280 and 640 pixels
wide. The sets are 1920 and 1280 now and weigh about three times as much;
the times were not measured again.

The script memory does not count decoded pictures; their upper bound is
the one computed above. A frame of the interface shots weighs 26 kB at
1280 pixels and 8 kB at 640; a dark frame 3 to 4 kB. The rendered 3D shots
will weigh more: if a frame weighs 40 kB, the large set is about 35 MB and
its first pass about 4 MB, and the small set about 12 MB and its first
pass about 1.4 MB. Measure again with the finished film.

Not measured: other browsers than Edge, a real phone, a real slow network,
and the memory of decoded pictures.

**The third film** (183.1 s at 30 frames a second, 2,746 pictures in each
set), prepared on 23 November 2026 at WebP quality 80 and 76 as before:

| | Large (1920) | Small (1280) |
|---|---|---|
| Every sixteenth frame, before the whole film can be scrubbed | 19.9 MB | 9.9 MB |
| Every picture | 159.8 MB in 106 packs | 79.5 MB in 55 packs |
| The video, played with "Watch the film" | 83.7 MB | 15.4 MB |

`website-film.zip` is 339 MB (the first film's was 100 MB). A Blender
frame weighs about 60 kB at 1920 pixels, three times an interface frame.
Quality 72 would save about a sixth (50 kB a frame) and was not taken: the
owner found the film soft before. Looked at on that day with `look.mjs`
and a headless Chrome 158 (no Edge on that computer), served by `serve.mjs`
with no limit on the network, at 1440 by 900 and at 390 by 844 and 360 by
780 as a phone: the film scrubs from the first frame to the last, the lines
come at their moments, the rail and its jumps, "Watch the film" plays the
video with its sound, reduced motion shows the stills, and the browser
reported nothing. The times on a local server say nothing about a real
network and are not given.

The published site was looked at once, on 14 November 2026, the evening it
went up, with `look.mjs` at 1600 by 900 and at a phone's size: nothing
reported by the browser, the first frame after about 4 s and the film
ready to scrub after about 5 s, with 8 MB of it loaded by then.

## The demo

The interface of the program, on the demo backend that lives in the
browser's memory, with a sample project: <https://fouad1233.github.io/teamwright/demo/>.
Why and how: [decision 0032](../docs/decisions/0032-a-public-demo-in-the-browser.md).
The page links to it from the masthead ("Try the demo", left out on a
screen narrower than 540 pixels) and from "Get it and build it".

It is built from `apps/desktop` in a mode of its own:

```bash
npm ci
npm run build:demo --workspace @teamwright/desktop   # into apps/desktop/dist-demo
```

The workflow runs exactly that, then copies `apps/desktop/dist-demo` into
the site as `demo/`. Its files are named relative to its page, so it works
under any address; GitHub Pages sends `/teamwright/demo` on to
`/teamwright/demo/`. The workflow runs when the interface changes (the
paths under `apps/desktop` it is built from, and the package files), not
only when `website/` does.

What the demo of the website has that the demo of `npm run dev` has not: a
band above the window that says it is a demo on sample data, that nothing is
saved and no AI runs, with links to this site and to the releases; on a
phone, a page that says it is made for a larger screen before it shows it;
and a specialist that starts on a task a moment after it opens and then asks
the visitor to decide.

What a visitor downloads on a first visit, measured on 21 November 2026 from
the build: about 520 kB compressed (1.7 MB as it lies on disk). The script is
380 kB of it, the style 54 kB, the four fonts of the Latin alphabet 81 kB.
The fonts of other alphabets are in the folder but only fetched for text
that needs them. The whole folder is 1.9 MB.

Looked at on 21 November 2026, in a headless Chrome, served under
`/teamwright/` and under another, deeper path: every view and every kind of
view that can be added, the menus and dialogs of the project, the inbox and
the decision of the specialist, the page Computers, at 1440 by 900 in the
light and the dark theme and at 960 by 640; on a phone's size (390 by 844
and 844 by 390), the page for small screens and the window after "Show it
anyway". The browser reported nothing. Not looked at: a real phone, Safari
and Firefox, and the site as GitHub Pages serves it, which needs the first
run of the workflow.

## Search engines

- One `h1`, sections with headings in order, landmarks, `lang="en"`, and
  alternative texts that say what each picture shows.
- Title, description, canonical address, Open Graph and Twitter card with
  `images/share.png` (1200 by 630).
- Structured data: `SoftwareApplication` and `SoftwareSourceCode` (written
  by hand), `VideoObject` with a clip for every chapter and `FAQPage`
  (written by `write-page.mjs`).
- `sitemap.xml` and `robots.txt`. Update `lastmod` in the sitemap when the
  page changes in substance.
- No request to any other site: no analytics, no trackers, no cookies.

**Google Search Console** is the owner's to set up. If it asks for an HTML
tag, put it in `index.html` on the line under the comment
`Google Search Console`, in the head. Then submit
`https://fouad1233.github.io/teamwright/sitemap.xml`.
