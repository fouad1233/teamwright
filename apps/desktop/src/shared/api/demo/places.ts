/**
 * Folders, as the demo compares them.
 *
 * The demo pretends to run on Windows, where `C:\Work` and `c:/work/` name
 * the same folder. The real application asks the computer it runs on.
 */

/** A folder in the form in which two folders are compared. */
function comparable(folder: string): string {
  return folder.trim().replaceAll('\\', '/').replace(/\/+$/, '').toLowerCase();
}

/** Whether two paths name the same folder. */
export function isSameFolder(first: string, second: string): boolean {
  return comparable(first) === comparable(second);
}

/** Whether a folder lies inside another one, at any depth. A folder is not inside itself. */
export function isInside(inner: string, outer: string): boolean {
  return comparable(inner).startsWith(`${comparable(outer)}/`);
}

/** The name of a file or folder: the last part of its path. */
export function nameOf(path: string): string {
  const parts = path.trim().replaceAll('\\', '/').replace(/\/+$/, '').split('/');
  return parts.at(-1) ?? '';
}

/**
 * A name for a file that is copied to where the names in `taken` are in
 * use: the name itself if it is free, and otherwise the name with a number
 * in front of its ending, as in `datasheet (2).pdf`.
 */
export function freeName(name: string, taken: readonly string[]): string {
  const isTaken = (candidate: string) =>
    taken.some((other) => other.toLowerCase() === candidate.toLowerCase());
  if (!isTaken(name)) {
    return name;
  }
  const dot = name.lastIndexOf('.');
  const [stem, ending] = dot > 0 ? [name.slice(0, dot), name.slice(dot)] : [name, ''];
  for (let number = 2; ; number += 1) {
    const candidate = `${stem} (${number.toString()})${ending}`;
    if (!isTaken(candidate)) {
      return candidate;
    }
  }
}
