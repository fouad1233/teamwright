/*
 * The film of the website, run by scrolling.
 *
 * The page is written so that it reads without this script: every chapter
 * of the film is a still with its title and its lines (index.html, written
 * from film.json by tools/write-page.mjs). This script adds four things:
 *
 * 1. Scrubbing. While the reader scrolls through the chapters, the stage
 *    stays in place and shows the frame of the film at the point they have
 *    scrolled to. The frames are pictures drawn on a canvas. They come in
 *    packs (tools/prepare-film.py): first every sixteenth frame of the
 *    film, so that the whole film can be scrubbed soon, then the frames in
 *    between, nearest to the reader first. They are kept compressed and
 *    decoded only near where the reader is.
 * 2. The lines of the film, laid over the picture word by word at their
 *    moments, as in the film. They are hidden from screen readers, which
 *    read the same lines in the chapters.
 * 3. A rail that names the chapters and goes to them; the arrow keys,
 *    Page Up and Page Down, and a sideways swipe do the same. The page
 *    scrolls there, so the film runs there rather than jumping.
 * 4. "Watch the film": the film as an ordinary video, with sound.
 *
 * Scrubbing is on only when the reader has not asked to reduce motion: the
 * class "scrub" on the root is set by a line in the head of the page,
 * before anything is drawn. Without it the chapters stay stills, and the
 * video is still there.
 */
(() => {
  'use strict';

  const root = document.documentElement;
  const film = document.querySelector('[data-film]');
  if (!film) return;

  const stage = film.querySelector('[data-stage]');
  const still = film.querySelector('[data-stage-still]');
  const title = film.querySelector('[data-stage-title]');
  const railLinks = [...film.querySelectorAll('[data-rail-link]')];
  const chapters = [...film.querySelectorAll('[data-chapter]')].map((element) => ({
    element,
    id: element.id,
    start: Number(element.dataset.start),
    end: Number(element.dataset.end),
    still: element.dataset.still,
  }));
  const duration = chapters[chapters.length - 1].end;
  const mediaAddress = document.querySelector('meta[name="teamwright-film-media"]')?.content;
  const mediaBase = mediaAddress ? mediaAddress.replace(/[^/]*$/, '') : '';
  const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');

  /* The aspect of the film's picture, and the shapes of window that it
     fills: a taller window would lose most of its sides, a wider one the
     foot of the picture. */
  const PICTURE_ASPECT = 16 / 9;
  const NARROWEST_COVER = 1.2;
  const WIDEST_COVER = 2;
  /* Below this width the rail runs along the foot of the stage (site.css). */
  const RAIL_AT_FOOT = 720;

  /* The motion of a word of a line, after lines.py of the film: seconds. */
  const STAGGER = 0.075;
  const ARRIVE = 0.45;
  const LEAVE = 0.25;
  const RISE = 0.0085;

  /* Where each kind of line stands on the picture, and how large it is,
     as fractions of the height of the picture (lines.py). */
  const PLACES = {
    caption: { y: 0.915, size: 0.043 },
    saying: { y: 0.735, size: 0.047 },
    statement: { y: 0.732, size: 0.037 },
    invitation: { y: 0.792, size: 0.04 },
    address: { y: 0.862, size: 0.033 },
  };
  /* Under the picture, where the window is tall: the offset from the top of
     the band of text, and the size, in pixels. */
  const PLACES_BELOW = {
    caption: { y: 0, size: 19 },
    saying: { y: 44, size: 20 },
    statement: { y: 44, size: 17 },
    invitation: { y: 72, size: 18 },
    address: { y: 100, size: 14 },
  };

  const clamp = (value, low = 0, high = 1) => Math.min(high, Math.max(low, value));
  const easeOut = (x) => 1 - (1 - clamp(x)) ** 3;
  const easeIn = (x) => clamp(x) ** 3;
  const easeInOut = (x) => (x < 0.5 ? 4 * x ** 3 : 1 - (-2 * x + 2) ** 3 / 2);
  const scrubbing = () => root.classList.contains('scrub');

  /* ---------------------------------------------------------------- *
   * Where the film is on the page, and the time at a scroll position. *
   * ---------------------------------------------------------------- */

  let trackTop = 0;
  let trackHeight = 1;

  function measureTrack() {
    const first = chapters[0].element.getBoundingClientRect();
    const last = chapters[chapters.length - 1].element.getBoundingClientRect();
    trackTop = first.top + window.scrollY;
    trackHeight = Math.max(1, last.bottom - first.top);
  }

  const timeAt = (scroll) => clamp(((scroll - trackTop) / trackHeight) * duration, 0, duration);
  const scrollFor = (time) => trackTop + (time / duration) * trackHeight;

  function chapterAt(time) {
    for (let index = chapters.length - 1; index >= 0; index -= 1) {
      if (time >= chapters[index].start) return index;
    }
    return 0;
  }

  /* ------------------------------------------------ *
   * The picture: how it fits the window, and canvas. *
   * ------------------------------------------------ */

  const canvas = document.createElement('canvas');
  canvas.setAttribute('aria-hidden', 'true');
  const context = canvas.getContext('2d', { alpha: true });
  still.after(canvas);

  const picture = { x: 0, y: 0, width: 0, height: 0, fit: 'cover', scale: 1 };

  function layoutPicture() {
    const width = stage.clientWidth;
    const height = stage.clientHeight;
    const ratio = Math.min(window.devicePixelRatio || 1, 2);
    const aspect = width / height;
    if (aspect > WIDEST_COVER) {
      // A window much wider than the film: filling it would cut the foot of
      // the picture, where the lines stand. The picture fills the height.
      picture.fit = 'height';
      picture.height = height;
      picture.width = height * PICTURE_ASPECT;
      picture.x = (width - picture.width) / 2;
      picture.y = 0;
    } else if (aspect >= NARROWEST_COVER) {
      picture.fit = 'cover';
      picture.width = Math.max(width, height * PICTURE_ASPECT);
      picture.height = picture.width / PICTURE_ASPECT;
      picture.x = (width - picture.width) / 2;
      picture.y = (height - picture.height) / 2;
    } else {
      picture.fit = 'contain';
      picture.width = width;
      picture.height = width / PICTURE_ASPECT;
      picture.x = 0;
      picture.y = Math.max(72, (height - picture.height) * 0.3);
    }
    picture.scale = ratio;
    stage.dataset.fit = picture.fit;
    stage.style.setProperty('--picture-y', `${picture.y}px`);
    stage.style.setProperty('--picture-height', `${picture.height}px`);
    canvas.width = Math.round(width * ratio);
    canvas.height = Math.round(height * ratio);
    drawn = -1;
  }

  /* -------------------------------------------- *
   * The frames: loaded in packs, decoded nearby. *
   * -------------------------------------------- */

  let frames = null;
  let drawn = -1;

  function chooseSet(manifest) {
    const connection = navigator.connection;
    const savesData = Boolean(connection && connection.saveData);
    const phone = Math.min(window.screen.width, window.screen.height) < 700;
    const name = savesData || phone ? 'small' : 'large';
    return manifest.sets[name] ? name : Object.keys(manifest.sets)[0];
  }

  class Frames {
    constructor(manifest, setName) {
      const set = manifest.sets[setName];
      this.type = manifest.type;
      this.perSecond = manifest.framesPerSecond;
      this.count = manifest.frameCount;
      this.packs = set.packs.map((pack) => ({
        ...pack,
        first: pack.frames[0][0],
        last: pack.frames[pack.frames.length - 1][0],
        state: 'waiting',
      }));
      this.blobs = new Array(this.count);
      this.bitmaps = new Map();
      this.decoding = new Map();
      this.wanted = 0;
      this.loading = 0;
      this.limit = setName === 'small' ? 72 : 28;
      this.bytes = 0;
      this.loadedCount = 0;
    }

    start() {
      for (let slot = 0; slot < 3; slot += 1) this.loadNext();
    }

    distance(pack) {
      if (this.wanted < pack.first) return pack.first - this.wanted;
      if (this.wanted > pack.last) return this.wanted - pack.last;
      return 0;
    }

    /* Every sixteenth frame of the film first, then what lies nearest. */
    nextPack() {
      let best = null;
      let bestKey = null;
      for (const pack of this.packs) {
        if (pack.state !== 'waiting') continue;
        const key = [pack.level === 0 ? 0 : 1, this.distance(pack), pack.level];
        if (!bestKey || key[0] < bestKey[0] || (key[0] === bestKey[0] && (key[1] < bestKey[1] || (key[1] === bestKey[1] && key[2] < bestKey[2])))) {
          best = pack;
          bestKey = key;
        }
      }
      return best;
    }

    loadNext() {
      const pack = this.nextPack();
      if (!pack) {
        if (this.loading === 0 && !this.finished) {
          this.finished = true;
          performance.mark('film:all-frames');
        }
        return;
      }
      pack.state = 'loading';
      this.loading += 1;
      fetch(mediaBase + pack.file)
        .then((response) => (response.ok ? response.arrayBuffer() : Promise.reject(new Error(response.status))))
        .then((buffer) => {
          for (const [index, offset, length] of pack.frames) {
            this.blobs[index] = new Blob([new Uint8Array(buffer, offset, length)], { type: this.type });
          }
          this.bytes += buffer.byteLength;
          this.loadedCount += pack.frames.length;
          pack.state = 'loaded';
          if (pack.level === 0 && this.packs.every((each) => each.level !== 0 || each.state === 'loaded')) {
            performance.mark('film:coarse-ready');
          }
          requestRender();
        })
        .catch(() => {
          pack.state = 'failed';
        })
        .finally(() => {
          this.loading -= 1;
          this.loadNext();
        });
    }

    nearestLoaded(index) {
      for (let step = 0; step < this.count; step += 1) {
        if (index - step >= 0 && this.blobs[index - step]) return index - step;
        if (index + step < this.count && this.blobs[index + step]) return index + step;
      }
      return -1;
    }

    decode(index) {
      if (this.bitmaps.has(index) || this.decoding.has(index) || !this.blobs[index]) return;
      const promise = createImageBitmap(this.blobs[index])
        .then((bitmap) => {
          this.bitmaps.set(index, bitmap);
          this.evict();
          requestRender();
        })
        .catch(() => {})
        .finally(() => this.decoding.delete(index));
      this.decoding.set(index, promise);
    }

    /* Decoded pictures take a lot of memory: keep those nearest the reader. */
    evict() {
      while (this.bitmaps.size > this.limit) {
        let farthest = -1;
        for (const index of this.bitmaps.keys()) {
          if (index === drawn) continue;
          if (farthest < 0 || Math.abs(index - this.wanted) > Math.abs(farthest - this.wanted)) farthest = index;
        }
        if (farthest < 0) return;
        this.bitmaps.get(farthest).close();
        this.bitmaps.delete(farthest);
      }
    }

    /* Draws the frame of a moment, or the nearest one there is. */
    show(time, direction) {
      this.wanted = clamp(Math.round(time * this.perSecond), 0, this.count - 1);
      const index = this.nearestLoaded(this.wanted);
      if (index < 0) return false;
      for (let ahead = 1; ahead <= 4; ahead += 1) {
        const next = this.nearestLoaded(clamp(this.wanted + direction * ahead * 2, 0, this.count - 1));
        if (next >= 0) this.decode(next);
      }
      const bitmap = this.bitmaps.get(index);
      if (!bitmap) {
        this.decode(index);
        return drawn >= 0;
      }
      if (index !== drawn) {
        const ratio = picture.scale;
        context.clearRect(0, 0, canvas.width, canvas.height);
        context.drawImage(bitmap, picture.x * ratio, picture.y * ratio, picture.width * ratio, picture.height * ratio);
        if (drawn < 0) performance.mark('film:first-frame');
        drawn = index;
      }
      return true;
    }
  }

  /* --------------------------------------------- *
   * The lines of the film, laid over the picture. *
   * --------------------------------------------- */

  const lineLayer = document.createElement('div');
  lineLayer.className = 'lines';
  lineLayer.setAttribute('aria-hidden', 'true');
  canvas.after(lineLayer);

  // A line whose place is "picture" is drawn into the film itself (the name
  // on the end card), so it is not laid over it a second time.
  const lineItems = [...film.querySelectorAll('.chapter-lines li')].filter((item) => item.dataset.place !== 'picture');
  const lines = lineItems.map((item) => {
    const element = document.createElement('p');
    element.className = 'line';
    element.dataset.place = item.dataset.place;
    const words = item.textContent.trim().split(/\s+/).map((text, index) => {
      if (index > 0) element.append(' ');
      const word = document.createElement('span');
      word.className = 'word';
      word.textContent = text;
      element.append(word);
      return word;
    });
    lineLayer.append(element);
    return {
      element,
      words,
      place: item.dataset.place,
      start: Number(item.dataset.start),
      end: item.dataset.end === '' ? null : Number(item.dataset.end),
      shown: false,
    };
  });

  function layoutLines() {
    const width = stage.clientWidth;
    const height = stage.clientHeight;
    const gutter = Math.max(16, Math.min(48, width * 0.04));
    const below = picture.fit === 'contain';
    const bandTop = picture.y + picture.height + 36;
    for (const line of lines) {
      let top;
      let size;
      if (below) {
        const place = PLACES_BELOW[line.place] || PLACES_BELOW.caption;
        size = place.size;
        top = bandTop + place.y + size / 2;
      } else {
        const place = PLACES[line.place] || PLACES.caption;
        size = Math.max(14, place.size * picture.height);
        // Above the rail where it runs along the foot of a narrow window.
        const room = width < RAIL_AT_FOOT ? 72 : 16;
        top = clamp(picture.y + place.y * picture.height, size, height - room - size * 0.6);
      }
      line.rise = RISE * (below ? 400 : picture.height);
      line.element.style.left = `${gutter}px`;
      line.element.style.width = `${width - 2 * gutter}px`;
      line.element.style.top = `${top}px`;
      line.element.style.fontSize = `${size}px`;
    }
  }

  function showLines(time) {
    for (const line of lines) {
      const leaveBy = line.end === null ? Infinity : line.end + LEAVE + 0.02 * line.words.length + 0.02;
      const active = time >= line.start - 0.01 && time <= leaveBy;
      if (!active) {
        if (line.shown) {
          for (const word of line.words) word.style.opacity = '0';
          line.shown = false;
        }
        continue;
      }
      line.shown = true;
      line.words.forEach((word, index) => {
        const arriving = easeOut((time - line.start - index * STAGGER) / ARRIVE);
        const leaving = line.end === null ? 0 : easeIn((time - line.end - index * 0.02) / LEAVE);
        const opacity = arriving * (1 - leaving);
        const rise = ((1 - arriving) - leaving * 0.6) * line.rise;
        word.style.opacity = opacity.toFixed(3);
        word.style.transform = `translateY(${rise.toFixed(1)}px)`;
      });
    }
  }

  /* ----------------------------------------- *
   * Drawing the moment the reader is at.      *
   * ----------------------------------------- */

  let scheduled = false;
  let lastScroll = window.scrollY;
  let direction = 1;
  let currentChapter = -1;
  let stillShown = '';

  function requestRender() {
    if (scheduled) return;
    scheduled = true;
    window.requestAnimationFrame(render);
  }

  function setStill(chapter) {
    const fromFilm = manifest && manifest.stills && manifest.stills[chapter.id];
    const source = fromFilm ? mediaBase + fromFilm : chapter.still;
    if (source === stillShown) return;
    stillShown = source;
    still.dataset.kind = fromFilm ? 'film' : 'program';
    if (/\s\d+w(,|$)/.test(source)) {
      still.srcset = source;
      still.sizes = '100vw';
      still.src = source.split(' ')[0];
    } else {
      still.removeAttribute('srcset');
      still.src = source;
    }
  }

  function markChapter(index) {
    if (index === currentChapter) return;
    currentChapter = index;
    railLinks.forEach((link, each) => {
      if (each === index) link.setAttribute('aria-current', 'step');
      else link.removeAttribute('aria-current');
    });
  }

  function render() {
    scheduled = false;
    if (!scrubbing()) return;
    const scroll = window.scrollY;
    if (scroll !== lastScroll) direction = scroll > lastScroll ? 1 : -1;
    lastScroll = scroll;
    const time = timeAt(scroll);
    const index = chapterAt(time);
    markChapter(index);

    const titleOpacity = 1 - clamp((time - 0.15) / 0.45);
    title.style.opacity = titleOpacity.toFixed(3);
    title.style.visibility = titleOpacity > 0 ? 'visible' : 'hidden';
    if (watchInStage) watchInStage.hidden = !watchReady || scroll < 80;

    showLines(time);
    const drawing = frames ? frames.show(time, direction) : false;
    if (drawing) stage.dataset.drawing = '';
    else {
      delete stage.dataset.drawing;
      setStill(chapters[index]);
    }
  }

  /* ---------------------------------------- *
   * Going from chapter to chapter.           *
   * ---------------------------------------- */

  let travel = null;

  function stopTravel() {
    if (travel) window.cancelAnimationFrame(travel);
    travel = null;
  }

  /* Scrolls to a place, so that the film runs there. */
  function travelTo(target) {
    stopTravel();
    const from = window.scrollY;
    const seconds = Math.abs(timeAt(target) - timeAt(from));
    const length = clamp(450 + seconds * 70, 500, 1500);
    const began = performance.now();
    const step = (now) => {
      const progress = clamp((now - began) / length);
      window.scrollTo(0, from + (target - from) * easeInOut(progress));
      travel = progress < 1 ? window.requestAnimationFrame(step) : null;
    };
    travel = window.requestAnimationFrame(step);
  }

  function goToChapter(index) {
    const chapter = chapters[clamp(index, 0, chapters.length - 1)];
    measureTrack();
    travelTo(Math.ceil(scrollFor(chapter.start)));
    history.replaceState(null, '', `#${chapter.id}`);
  }

  function onKey(event) {
    if (!scrubbing() || event.defaultPrevented) return;
    if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
    if (event.target.closest && event.target.closest('input, textarea, select, video, [contenteditable], dialog')) return;
    const forward = ['ArrowDown', 'PageDown', 'ArrowRight'].includes(event.key);
    const backward = ['ArrowUp', 'PageUp', 'ArrowLeft'].includes(event.key);
    if (!forward && !backward) return;
    const scroll = window.scrollY;
    if (scroll < trackTop - 2 || scroll > trackTop + trackHeight - 2) return;
    const time = timeAt(scroll);
    const index = chapterAt(time);
    if (forward) {
      if (index === chapters.length - 1) return;
      goToChapter(index + 1);
    } else if (time - chapters[index].start > 0.25) {
      goToChapter(index);
    } else {
      if (index === 0) return;
      goToChapter(index - 1);
    }
    event.preventDefault();
  }

  let touch = null;
  function onTouchStart(event) {
    const point = event.touches[0];
    touch = { x: point.clientX, y: point.clientY };
  }
  function onTouchEnd(event) {
    if (!touch || !scrubbing()) return;
    const point = event.changedTouches[0];
    const dx = point.clientX - touch.x;
    const dy = point.clientY - touch.y;
    touch = null;
    if (Math.abs(dx) < 60 || Math.abs(dx) < 2 * Math.abs(dy)) return;
    const index = chapterAt(timeAt(window.scrollY));
    goToChapter(dx < 0 ? index + 1 : index - 1);
  }

  /* ---------------------------------------- *
   * "Watch the film", with sound.            *
   * ---------------------------------------- */

  const dialog = document.querySelector('[data-watch-dialog]');
  const video = document.querySelector('[data-watch-video]');
  const watchButtons = [...document.querySelectorAll('[data-watch]')];
  let watchInStage = null;
  let watchReady = false;
  let videoFile = '';

  function prepareWatching(manifest) {
    const files = manifest.video || {};
    const connection = navigator.connection;
    const small = (connection && connection.saveData) || Math.min(window.screen.width, window.screen.height) < 700;
    videoFile = (small && files.small) || files.large || files.small || '';
    if (!videoFile || !dialog || typeof dialog.showModal !== 'function') return;
    watchReady = true;
    for (const button of watchButtons) {
      button.hidden = false;
      button.addEventListener('click', openFilm);
    }
    requestRender();
  }

  function openFilm() {
    if (!video.src) video.src = mediaBase + videoFile;
    // The video begins where the reader is in the film, or at the beginning
    // from the end card, where there is nothing left to watch.
    const time = scrubbing() ? timeAt(window.scrollY) : 0;
    try {
      video.currentTime = time < chapters[chapters.length - 1].start ? time : 0;
    } catch {
      // Before the video has loaded, the browser may not take a time yet.
    }
    dialog.showModal();
    video.play().catch(() => {});
  }

  if (dialog) {
    dialog.addEventListener('close', () => video.pause());
    dialog.addEventListener('click', (event) => {
      if (event.target === dialog) dialog.close();
    });
    dialog.querySelector('[data-watch-close]').addEventListener('click', () => dialog.close());
    // A second control on the stage, for when the masthead has scrolled away.
    watchInStage = watchButtons[0] ? watchButtons[0].cloneNode(true) : null;
    if (watchInStage) {
      watchInStage.classList.add('stage-watch');
      stage.append(watchInStage);
      watchButtons.push(watchInStage);
    }
  }

  /* ---------------------------------------- *
   * Starting.                                *
   * ---------------------------------------- */

  let manifest = null;

  function layout() {
    const time = timeAt(window.scrollY);
    const wasInFilm = window.scrollY > trackTop && window.scrollY < trackTop + trackHeight;
    measureTrack();
    if (!scrubbing()) return;
    layoutPicture();
    layoutLines();
    if (wasInFilm && travel === null) window.scrollTo(0, scrollFor(time));
    requestRender();
  }

  function onMotionPreference() {
    root.classList.toggle('scrub', !reducedMotion.matches);
    if (!scrubbing()) {
      title.style.opacity = '';
      title.style.visibility = '';
      still.removeAttribute('style');
    }
    stillShown = '';
    layout();
  }

  function loadMedia() {
    if (!mediaAddress) return;
    fetch(mediaAddress)
      .then((response) => (response.ok ? response.json() : null))
      .then((loaded) => {
        if (!loaded) return;
        manifest = loaded;
        prepareWatching(manifest);
        if (scrubbing() && window.createImageBitmap) {
          frames = new Frames(manifest, chooseSet(manifest));
          frames.wanted = Math.round(timeAt(window.scrollY) * frames.perSecond);
          frames.start();
        }
        stillShown = '';
        requestRender();
      })
      .catch(() => {});
  }

  measureTrack();
  if (scrubbing()) {
    layoutPicture();
    layoutLines();
  }
  render();

  window.addEventListener('scroll', requestRender, { passive: true });
  window.addEventListener('resize', layout);
  window.addEventListener('keydown', onKey);
  for (const kind of ['wheel', 'touchstart', 'mousedown']) {
    window.addEventListener(kind, stopTravel, { passive: true });
  }
  stage.addEventListener('touchstart', onTouchStart, { passive: true });
  stage.addEventListener('touchend', onTouchEnd, { passive: true });
  reducedMotion.addEventListener('change', onMotionPreference);
  railLinks.forEach((link, index) =>
    link.addEventListener('click', (event) => {
      if (!scrubbing()) return;
      event.preventDefault();
      goToChapter(index);
    }),
  );

  // The frames wait until the page itself has loaded, so that they never
  // hold up the first picture.
  if (document.readyState === 'complete') loadMedia();
  else window.addEventListener('load', loadMedia, { once: true });
})();
