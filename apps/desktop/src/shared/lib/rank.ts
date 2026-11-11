/**
 * Ranks order the tasks of one status. A lower rank comes first.
 *
 * The real ranking lives in the domain crate (`Rank` in
 * `crates/domain/src/task.rs`). This file repeats the same arithmetic so the
 * interface can show a move at once, before the backend has answered, and so
 * the demo backend orders tasks the same way.
 */

/** The distance left between a new rank and the end of the list it joins. */
const RANK_STEP = 1024;

/** The rank of the only task in a list. */
const FIRST_RANK = 0;

/**
 * A rank that sorts after `before` and ahead of `after`. Either neighbour can
 * be missing: at the top of a list there is nothing before, at the bottom
 * nothing after, and in an empty list neither.
 */
export function rankBetween(before: number | undefined, after: number | undefined): number {
  if (before !== undefined && after !== undefined) {
    return before + (after - before) / 2;
  }
  if (before !== undefined) {
    return before + RANK_STEP;
  }
  if (after !== undefined) {
    return after - RANK_STEP;
  }
  return FIRST_RANK;
}

/**
 * Tells whether a rank lies strictly between two neighbours. When it does
 * not, the neighbours are too close together for another rank and the list
 * has to be spaced out again.
 */
export function rankFitsBetween(
  rank: number,
  before: number | undefined,
  after: number | undefined,
): boolean {
  return (before === undefined || before < rank) && (after === undefined || rank < after);
}

/** Evenly spaced ranks for a list of the given length, in order. */
export function evenlySpacedRanks(count: number): number[] {
  return Array.from({ length: count }, (_, position) => FIRST_RANK + position * RANK_STEP);
}
