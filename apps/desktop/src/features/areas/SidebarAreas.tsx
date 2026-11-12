import type { Area, Id } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import styles from './SidebarAreas.module.css';

interface SidebarAreasProps {
  /** The areas of the project, in their order. Only drawn when there are two or more. */
  areas: readonly Area[];
  /** The area that is open, if one of these is. */
  openAreaId: Id | null;
  onOpen: (areaId: Id) => void;
}

/**
 * The areas of a project in the navigation at the side, indented under its
 * name. Pressing one shows only its tasks; pressing the project above shows
 * them all again.
 */
export function SidebarAreas({ areas, openAreaId, onOpen }: SidebarAreasProps) {
  return (
    <ul className={styles.areas}>
      {areas.map((area) => {
        const isOpen = area.id === openAreaId;
        return (
          <li key={area.id}>
            <button
              type="button"
              className={classNames(styles.area, isOpen && styles.open)}
              aria-current={isOpen ? 'page' : undefined}
              onClick={() => {
                onOpen(area.id);
              }}
            >
              <span className={styles.name}>{area.name}</span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}
