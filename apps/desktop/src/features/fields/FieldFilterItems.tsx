import { Check } from 'lucide-react';
import { DropdownMenu } from 'radix-ui';
import type { ReactNode } from 'react';

import type { View, ViewFilter } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import menuStyles from '@/shared/ui/Menu.module.css';

import { TagChip } from '../tags/TagChip';
import viewMenuStyles from '../views/ViewTabs.module.css';
import { filterKey, filterOfKey, filteringFields } from './fieldModel';
import { useFields } from './fieldQueries';
import styles from './FieldFilterItems.module.css';

/** The choice of the menu that stands for the tasks whatever their fields hold. */
const ANY_VALUE = 'any';

interface FieldFilterItemsProps {
  view: View;
  onFilter: (filter: ViewFilter) => void;
}

/**
 * The part of the filter menu of a view that shows only the tasks holding
 * one value of a field: a choice of a choice field, or yes or no. Nothing
 * while the project has no such field.
 */
export function FieldFilterItems({ view, onFilter }: FieldFilterItemsProps) {
  const fields = filteringFields(useFields(view.projectId).data ?? []);
  if (fields.length === 0) {
    return null;
  }
  const filter = view.filter;
  const item = (key: string, content: ReactNode) => (
    <DropdownMenu.RadioItem key={key} value={key} className={menuStyles.item}>
      <span className={viewMenuStyles.mark} />
      <span className={menuStyles.label}>{content}</span>
      <DropdownMenu.ItemIndicator className={menuStyles.check}>
        <Check />
      </DropdownMenu.ItemIndicator>
    </DropdownMenu.RadioItem>
  );

  return (
    <>
      <DropdownMenu.Separator className={menuStyles.separator} />
      <DropdownMenu.Label className={classNames('label', viewMenuStyles.menuHeading)}>
        With the field
      </DropdownMenu.Label>
      <DropdownMenu.RadioGroup
        value={filter.fieldFilter ? filterKey(filter.fieldFilter) : ANY_VALUE}
        onValueChange={(chosen) => {
          onFilter({ ...filter, fieldFilter: filterOfKey(chosen) });
        }}
      >
        {item(ANY_VALUE, 'Whatever the fields hold')}
        {fields.map((field) =>
          field.kind === 'yes_no'
            ? [
                item(
                  filterKey({ kind: 'yes_no', fieldId: field.id, yes: true }),
                  `${field.name}: yes`,
                ),
                item(
                  filterKey({ kind: 'yes_no', fieldId: field.id, yes: false }),
                  `${field.name}: no or not set`,
                ),
              ]
            : field.choices.map((choice) =>
                item(
                  filterKey({ kind: 'choice', fieldId: field.id, choiceId: choice.id }),
                  <span className={styles.fieldChoice}>
                    <span className={styles.fieldName}>{field.name}</span>
                    <TagChip tag={choice} size="regular" />
                  </span>,
                ),
              ),
        )}
      </DropdownMenu.RadioGroup>
    </>
  );
}
