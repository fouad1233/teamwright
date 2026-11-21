import type { Id } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import { useMemoryOfProject } from './MemoryContext';
import { rememberedInTurn } from './memoryModel';
import styles from './RememberedInTurn.module.css';

/**
 * The facts a specialist remembered in a turn, set as steps of the turn.
 * Nothing for a turn that remembered none.
 */
export function RememberedInTurn({ turnId }: { turnId: Id }) {
  const facts = rememberedInTurn(useMemoryOfProject() ?? [], turnId);
  if (facts.length === 0) {
    return null;
  }
  return (
    <ol className={styles.facts}>
      {facts.map((fact) => (
        <li key={fact.id} className={styles.line}>
          <span className={classNames('label', styles.name)}>Memory</span>
          <div className={styles.body}>
            <p className={styles.title}>
              Remembered: {fact.title}
              {fact.replacedBy !== null && (
                <span className={styles.replaced}> · replaced since</span>
              )}
            </p>
            <p className={styles.text} data-selectable>
              {fact.text}
            </p>
          </div>
        </li>
      ))}
    </ol>
  );
}
