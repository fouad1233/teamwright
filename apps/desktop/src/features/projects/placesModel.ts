/**
 * Says which files were copied for the specialists, by the names they have
 * now: "Copied sheet.pdf", "Copied sheet.pdf and notes.txt", "Copied 5
 * files".
 */
export function copiedInWords(names: readonly string[]): string {
  const [first, second] = names;
  if (first === undefined) {
    return 'Nothing was copied';
  }
  if (second === undefined) {
    return `Copied ${first}`;
  }
  return names.length === 2
    ? `Copied ${first} and ${second}`
    : `Copied ${names.length.toString()} files`;
}
