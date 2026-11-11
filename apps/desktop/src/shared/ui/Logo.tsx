interface LogoProps {
  /** The width and height in pixels. */
  size?: number;
}

/**
 * The Teamwright mark: a letter T laid out of five blocks. The block that
 * holds the others together, the keystone, is brass.
 *
 * The mark takes its colors from the theme, so it is ink on paper in the
 * light theme and paper on graphite in the dark one.
 */
export function Logo({ size = 22 }: LogoProps) {
  // Three columns and three rows of 6 by 6 blocks with a gap of 1.5.
  const positions = [1.5, 9, 16.5] as const;
  const [first, middle, last] = positions;
  const block = { width: 6, height: 6, rx: 1 };

  return (
    <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden>
      <rect x={first} y={first} {...block} fill="var(--ink)" />
      <rect x={middle} y={first} {...block} fill="var(--brass)" />
      <rect x={last} y={first} {...block} fill="var(--ink)" />
      <rect x={middle} y={middle} {...block} fill="var(--ink)" />
      <rect x={middle} y={last} {...block} fill="var(--ink)" />
    </svg>
  );
}
