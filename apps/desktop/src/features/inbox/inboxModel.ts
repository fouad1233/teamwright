/** Says how many decisions wait for the manager: "1 decision waits for you". */
export function waitingInWords(count: number): string {
  if (count === 0) {
    return 'Nothing waits for you';
  }
  return count === 1 ? '1 decision waits for you' : `${count.toString()} decisions wait for you`;
}
