import styles from './PlateFigure.module.css';

/** The side of one part and the gap between two parts, in drawing units. */
const BLOCK = 96;
const KERF = 8;
const STEP = BLOCK + KERF;
/** The room left around the assembly for its dimension lines. */
const INSET = 56;
const SIZE = 3 * BLOCK + 2 * KERF;
const EXTENT = SIZE + 2 * INSET;

/** The five parts of the T: where each sits and how it is drawn. */
const PARTS = [
  { column: 0, row: 0, fill: 'url(#plate-diagonal)' },
  { column: 1, row: 0, fill: 'var(--brass)' },
  { column: 2, row: 0, fill: 'url(#plate-dots)' },
  { column: 1, row: 1, fill: 'var(--ink)' },
  { column: 1, row: 2, fill: 'url(#plate-horizontal)' },
] as const;

/**
 * The first figure of the key visual, drawn in the colors of the theme: the
 * logo taken apart into its five parts, with the brass keystone in the
 * middle of the top row.
 *
 * It is the one illustration of the application, used where there is
 * nothing else to show yet. See docs/design.
 */
export function PlateFigure() {
  const end = INSET + SIZE;
  const dimensionY = INSET - 28;
  const centerX = INSET + STEP + BLOCK / 2;

  return (
    <svg
      className={styles.figure}
      viewBox={`0 0 ${EXTENT.toString()} ${EXTENT.toString()}`}
      role="img"
      aria-label="Five parts that fit together to form the letter T"
    >
      <defs>
        <pattern
          id="plate-diagonal"
          width="7"
          height="7"
          patternUnits="userSpaceOnUse"
          patternTransform="rotate(45)"
        >
          <line x1="0" y1="0" x2="0" y2="7" className={styles.hatch} />
        </pattern>
        <pattern id="plate-dots" width="7" height="7" patternUnits="userSpaceOnUse">
          <circle cx="1.75" cy="1.75" r="0.9" className={styles.dot} />
          <circle cx="5.25" cy="5.25" r="0.9" className={styles.dot} />
        </pattern>
        <pattern id="plate-horizontal" width="7" height="5.5" patternUnits="userSpaceOnUse">
          <line x1="0" y1="2.75" x2="7" y2="2.75" className={styles.hatch} />
        </pattern>
      </defs>

      {/* The dimension across the top: three units. */}
      <g className={styles.dimension}>
        <line x1={INSET} y1={INSET - 6} x2={INSET} y2={dimensionY - 6} />
        <line x1={end} y1={INSET - 6} x2={end} y2={dimensionY - 6} />
        <line x1={INSET} y1={dimensionY} x2={centerX - 20} y2={dimensionY} />
        <line x1={centerX + 20} y1={dimensionY} x2={end} y2={dimensionY} />
        <line x1={INSET - 4} y1={dimensionY + 4} x2={INSET + 4} y2={dimensionY - 4} />
        <line x1={end - 4} y1={dimensionY + 4} x2={end + 4} y2={dimensionY - 4} />
      </g>
      <text x={centerX} y={dimensionY + 3.5} className={styles.measure}>
        3 U
      </text>

      {PARTS.map(({ column, row, fill }) => (
        <rect
          key={`${column.toString()}:${row.toString()}`}
          x={INSET + column * STEP}
          y={INSET + row * STEP}
          width={BLOCK}
          height={BLOCK}
          fill={fill}
          className={fill === 'var(--brass)' ? styles.keystone : styles.part}
        />
      ))}

      {/* The register mark on the keystone. */}
      <g className={styles.register}>
        <circle cx={centerX} cy={INSET + BLOCK / 2} r="11" />
        <line x1={centerX - 17} y1={INSET + BLOCK / 2} x2={centerX + 17} y2={INSET + BLOCK / 2} />
        <line x1={centerX} y1={INSET + BLOCK / 2 - 17} x2={centerX} y2={INSET + BLOCK / 2 + 17} />
      </g>
    </svg>
  );
}
