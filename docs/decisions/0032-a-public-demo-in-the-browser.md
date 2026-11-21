# 0032. Publish the interface on the demo backend as a page of the website

- Status: accepted
- Date: 2026-11-21

## Context

The interface already runs in a plain browser on a backend that lives in
memory (`DemoBackend`), with a sample project and specialists whose work is
simulated. The website (decision 0023) explains Teamwright but cannot let a
visitor touch it, and there is no installer for everybody yet. The roadmap
asked for a public demo in the browser.

A demo on a public page meets people who have not read anything else: they
must see at once that it is a demo, that nothing they do is kept, and that
no AI runs. It also meets phones, for which the window of a desktop
program was never made.

## Decision

- **The same interface, built in a mode of its own.** `npm run build:demo`
  (in `apps/desktop`) runs `vite build --mode public-demo` into
  `apps/desktop/dist-demo`. In that mode the files are named relative to the
  page (`base: './'`), so the demo works under any address. The desktop
  build is unchanged: its mode, its root and its folder (`dist`) are as
  before.
- **Published at `demo/` of the website.** The workflow of the website
  (`.github/workflows/pages.yml`) installs the packages, builds the demo and
  copies it into the site, so that it is served at
  `https://fouad1233.github.io/teamwright/demo/`. The workflow also runs
  when the interface changes, so the demo never lags behind it.
  `website/tools/check.mjs` lets a page link to `demo/` only while the
  workflow builds it there.
- **What only the public demo has** is decided by one constant,
  `isPublicDemo` in `shared/lib/publicDemo.ts`, which Vite settles at build
  time, so the desktop program carries none of it:
  - a quiet band above the window (`app/PublicDemoFrame.tsx`): "Demo", one
    sentence (sample data, nothing saved, no AI, the specialists only play
    their work), and two links, to the website and to the releases;
  - on a screen narrower than 760 pixels or lower than 500, a page that says
    the demo is made for a larger screen, with "Show it anyway", which lays
    the page out 1100 pixels wide so the phone shows the whole window,
    smaller, and lets the visitor zoom in, and "Back to the website";
  - a first scene: 1.5 seconds after it opens, the specialist of the first
    task in work begins on it, works for a few seconds and asks the manager
    to decide, so that a visitor sees work going on and then something that
    waits for them (`DemoBackend.beginSampleWork`).
- **The demo of `npm run dev` stays as it was**, without the band and the
  first scene. It is where the interface is developed and where its
  pictures are taken; a band would be in every picture, and the first scene
  would change the state the tools of pictures expect.
- **What a browser cannot do is not offered in any demo.** Actions that
  need the desktop shell are shown as not available, with "In the desktop
  program, not in the demo" under them (export to Excel, open a file with
  its program, show it in its folder), or are left out (the test
  notification and the question whether to allow notifications), as
  "Show folder" and "Open in Claude Code" already were. The backend
  already answers such requests with `not_available`; the interface only
  stops offering them, through `useIsDemo`, as before.

## Consequences

- A visitor can try every view on sample data without installing anything.
  Nothing leaves their browser: the demo backend sends nothing anywhere.
- The site's first run of the workflow now needs `npm ci`, which takes a
  minute or two. A change to the interface publishes the site again.
- The demo downloads about 520 kB (compressed) on a first visit: one script
  of 380 kB, the style of 54 kB and four fonts of 81 kB. It is one script,
  not split by view; splitting it is left for when it matters.
- What only the demo of the website shows is not seen in the daily work on
  the interface. It is small and kept in two files for that reason.

## Alternatives considered

- **A separate demo application or a copy of the interface.** It would
  drift from the program it shows.
- **A fixed base path such as `/teamwright/demo/`.** Simpler to read, but a
  fork of the repository or a local preview at another address would
  break. Relative names work everywhere, and the interface has no addresses
  of its own to route.
- **The band in every demo, including `npm run dev`.** It would show in the
  pictures of the documentation and the website.
- **Rebuilding the interface for phones.** Out of proportion for a desktop
  program; an honest page and a zoomed-out window are enough.
- **Hiding every action of the shell in the public demo only.** Every demo
  is a browser, so the same actions fail in all of them; one rule for all
  is simpler.
