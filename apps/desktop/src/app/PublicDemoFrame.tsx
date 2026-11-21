import { useState, type ReactNode } from 'react';

import { RELEASES_ADDRESS, WEBSITE_ADDRESS } from '@/shared/lib/publicDemo';
import { Button } from '@/shared/ui/Button';
import { Logo } from '@/shared/ui/Logo';

import styles from './PublicDemoFrame.module.css';

/**
 * A screen too small for the window of the desktop program: a phone held
 * either way. The window is never narrower than 960 pixels on a computer.
 */
const SMALL_SCREEN = '(max-width: 759px), (max-height: 499px)';

/**
 * How wide the page is laid out on a small screen once the visitor chose
 * to see the demo anyway: about the narrowest window of the program, so the
 * browser shows all of it, smaller, and the visitor can zoom in.
 */
const WIDTH_ON_A_SMALL_SCREEN = 1100;

/** Remembers, for this tab, that the visitor chose to see the demo on a small screen. */
const SEES_ANYWAY_KEY = 'teamwright.demo.seesAnyway';

interface PublicDemoFrameProps {
  children: ReactNode;
}

/**
 * The frame of the public demo on the website (decision 0032): a quiet
 * band above the window that says what a visitor is looking at, and on a
 * phone a page that says the demo is made for a larger screen before it
 * shows it. Only the build of the public demo has it.
 */
export function PublicDemoFrame({ children }: PublicDemoFrameProps) {
  const [isOnSmallScreen] = useState(() => window.matchMedia(SMALL_SCREEN).matches);
  const [seesAnyway, setSeesAnyway] = useState(() => {
    const anyway = rememberedChoice();
    if (anyway) {
      layOutWide();
    }
    return anyway;
  });

  if (isOnSmallScreen && !seesAnyway) {
    return (
      <SmallScreenNotice
        onSeeAnyway={() => {
          rememberChoice();
          layOutWide();
          setSeesAnyway(true);
        }}
      />
    );
  }

  return (
    <div className={styles.frame}>
      <DemoBand />
      <div className={styles.window}>{children}</div>
    </div>
  );
}

/** The band above the window. One sentence, and the two ways out. */
function DemoBand() {
  return (
    <aside className={styles.band} aria-label="About this demo">
      <span className={styles.mark} aria-hidden />
      <span className={`label ${styles.kind}`}>Demo</span>
      <p className={styles.sentence}>
        Sample data, in your browser. Nothing is saved, and no AI runs here: the specialists only
        play their work.
      </p>
      <nav className={styles.ways} aria-label="Teamwright">
        <a className={styles.way} href={WEBSITE_ADDRESS}>
          About Teamwright
        </a>
        <a className={styles.way} href={RELEASES_ADDRESS}>
          Get the program
        </a>
      </nav>
    </aside>
  );
}

interface SmallScreenNoticeProps {
  onSeeAnyway: () => void;
}

/** What a phone shows first: that the demo is made for a larger screen. */
function SmallScreenNotice({ onSeeAnyway }: SmallScreenNoticeProps) {
  return (
    <main className={styles.notice}>
      <div className={styles.brand}>
        <Logo size={20} />
        <span className={styles.wordmark}>Teamwright</span>
      </div>
      <p className="label">The demo</p>
      <h1 className={styles.title}>Made for a larger screen</h1>
      <p className={styles.text}>
        Teamwright is a program for a computer, and this demo shows its window as it is, on a sample
        project. On a phone it is small and cramped. It is best on a computer.
      </p>
      <p className={styles.text}>Nothing is saved, and no AI runs here.</p>
      <div className={styles.choices}>
        <Button variant="primary" onClick={onSeeAnyway}>
          Show it anyway
        </Button>
        <a className={styles.way} href={WEBSITE_ADDRESS}>
          Back to the website
        </a>
      </div>
    </main>
  );
}

/**
 * Lays the page out as wide as a small window of the program. A phone
 * then shows the whole window, smaller, and lets the visitor zoom in.
 */
function layOutWide(): void {
  document
    .querySelector('meta[name="viewport"]')
    ?.setAttribute('content', `width=${WIDTH_ON_A_SMALL_SCREEN.toString()}`);
}

function rememberedChoice(): boolean {
  try {
    return sessionStorage.getItem(SEES_ANYWAY_KEY) === 'yes';
  } catch {
    // A browser that keeps nothing asks again.
    return false;
  }
}

function rememberChoice(): void {
  try {
    sessionStorage.setItem(SEES_ANYWAY_KEY, 'yes');
  } catch {
    // Nothing to do: the choice holds until the page is loaded again.
  }
}
