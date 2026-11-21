import type { Tag } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import styles from './TagChip.module.css';
import { fittingTags } from './tagModel';

interface TagChipProps {
  tag: Pick<Tag, 'name' | 'color'>;
  /** A chip in a table cell or on a card is small; one in the panel of a task is regular. */
  size?: 'small' | 'regular';
  className?: string;
}

/**
 * A tag: its name on a flat wash of its pigment, with a square of the
 * pigment in front, so that the color reads even where the wash is faint.
 * A long name is cut short; the whole name is its tooltip.
 */
export function TagChip({ tag, size = 'small', className }: TagChipProps) {
  return (
    <span
      className={classNames(styles.chip, size === 'regular' && styles.regular, className)}
      data-tone={tag.color}
      title={tag.name}
    >
      <span className={styles.swatch} aria-hidden />
      <span className={styles.name}>{tag.name}</span>
    </span>
  );
}

interface TagChipsProps {
  /** The tags to show, in their order. */
  tags: readonly Tag[];
  /** The most chips the row has, the count of the others included. */
  most: number;
  className?: string;
}

/**
 * The tags of a task in a row that stays short: as many chips as fit in
 * `most`, and "+N" for the others, which name themselves in its tooltip.
 */
export function TagChips({ tags, most, className }: TagChipsProps) {
  if (tags.length === 0) {
    return null;
  }
  const { shown, more } = fittingTags(tags, most);
  const hidden = tags.slice(shown.length);
  return (
    <span className={classNames(styles.row, className)} aria-label="Tags">
      {shown.map((tag) => (
        <TagChip key={tag.id} tag={tag} />
      ))}
      {more > 0 && (
        <span
          className={styles.more}
          title={hidden.map((tag) => tag.name).join(', ')}
          aria-label={`and ${more.toString()} more: ${hidden.map((tag) => tag.name).join(', ')}`}
        >
          +{more}
        </span>
      )}
    </span>
  );
}
