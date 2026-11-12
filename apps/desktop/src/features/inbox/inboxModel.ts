/**
 * Says how many things wait for the manager: "1 thing waits for you". A
 * thing is a question of a specialist, or questions or a proposal of the
 * Advisor.
 */
export function waitingInWords(count: number): string {
  if (count === 0) {
    return 'Nothing waits for you';
  }
  return count === 1 ? '1 thing waits for you' : `${count.toString()} things wait for you`;
}
