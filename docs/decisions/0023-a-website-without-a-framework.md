# 0023. Make the website one static page, with the film drawn from frames

- Status: accepted
- Date: 2026-11-13

## Context

Teamwright needs a public page that explains it to somebody who has never
heard of it, and that search engines find. The owner asked for it on
GitHub Pages, with the film that is being made for the project at the top,
run by scrolling: "when you slide it will navigate through the video".

The film is about a minute and a quarter at 24 frames a second, and is
still being made. Its frames and its video together weigh tens of
megabytes, and every new version of the film would add as much again.

## Decision

- **One page of plain HTML, CSS and JavaScript, in `website/`.** No
  framework, no bundler, nothing to install to build it. What is in the
  folder is what is published. Everything a search engine or a reader
  without JavaScript needs is in the HTML as written: the film's chapters
  and lines are text on the page, and the script only adds the scrubbing.
- **The look is the program's.** The site takes the program's design
  tokens, its three typefaces (served from the site itself) and its
  pictures, and follows [the design philosophy](../design/philosophy.md).
- **The film is drawn on a canvas from frames, not played from a video
  whose time follows the scroll.** A video can only start drawing at its
  key frames, so setting its time while scrolling stutters in most
  browsers. Pictures can be drawn in any order at once. The page uses
  every second frame, in packs: every sixteenth frame of the film first,
  so that the whole film can be scrubbed soon, then the frames between,
  nearest the reader first. A smaller set serves phones and readers who
  ask to save data. The video with sound is there too, for "Watch the
  film".
- **The film's numbers come from the film.** The assembly of the film
  writes a data file of its shots and lines. A tool writes from it the
  numbers in `website/film.json`, and the page from that. A chapter of the
  site begins with a shot, so a new shot lengthens its chapter.
- **The film's media is kept out of git.** The frames, the stills of the
  film and the video are prepared into `website/media/film/`, which git
  ignores, packed into `website-film.zip`, and attached to a release of
  the repository with the tag `website-film`. The publishing workflow
  fetches it from there. Without it, the page is published without the
  film, and every chapter shows a picture of the program and its text.
- **Nothing from anywhere else.** No analytics, no trackers, no cookies,
  no fonts or scripts from another site.

## Consequences

- The page can be changed with a text editor and checked with
  `node website/tools/check.mjs`, which `npm run check` runs.
- The repository stays small however often the film changes.
- Publishing a new film is a release asset, not a commit; the page and
  its data still change in a commit when the film's length or lines do.
- A part of the page is written by a tool (`write-page.mjs`) between
  marked comments. Whoever edits the page must not edit those parts by
  hand; the check fails if they do.
- The scrubbing holds the compressed pictures in memory and decodes only
  those near the reader. How much memory that takes on a real phone has
  not been measured.

## Alternatives considered

- **A site generator or a framework** (Astro, Next.js, Vite with React).
  The page is one page; a generator would add a build, dependencies and
  their updates for nothing a reader sees.
- **Setting the time of a video from the scroll.** Simpler, and one file,
  but it stutters, most of all when scrolling backwards, unless the video
  is encoded with a key frame at every frame, which makes it as large as
  the frames.
- **The film's media in git, or in Git LFS.** In git it would stay in
  the history for ever. Git LFS on GitHub has a small free quota and is
  not served by GitHub Pages.
