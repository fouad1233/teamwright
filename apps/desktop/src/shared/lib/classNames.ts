/**
 * Joins the class names that apply into one `className` value. Values that
 * are `false`, `null` or `undefined` are left out, which makes conditions
 * easy to write: `classNames(styles.card, isDragging && styles.dragging)`.
 */
export function classNames(...names: (string | false | null | undefined)[]): string {
  return names.filter(Boolean).join(' ');
}
