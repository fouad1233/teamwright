import type { CSSProperties } from 'react';

import type { Member, ProjectUsage } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import { usageInWords } from '../office/officeModel';
import { formatTokens } from '../work/workModel';
import { DashboardSection } from './DashboardSection';
import { PlanLimitPart } from './PlanLimitPart';
import { shareOf, usageTable, type UsageRow } from './dashboardModel';
import styles from './UsagePart.module.css';

interface UsagePartProps {
  members: readonly Member[];
  /** What the agents spent, as the backend says, or nothing while it is asked for. */
  usage: ProjectUsage | undefined;
}

/** The columns of counted tokens, by kind, in the program's words. */
const KINDS = [
  { key: 'output', title: 'Written', isWide: false },
  { key: 'input', title: 'Read', isWide: false },
  { key: 'cacheRead', title: 'Read again from the cache', isWide: true },
  { key: 'cacheWritten', title: 'Written to the cache', isWide: true },
] as const;

function Row({ row, largest }: { row: UsageRow; largest: number }) {
  const { usage } = row;
  const isProject = row.kind === 'project';
  return (
    <tr className={classNames(styles.row, isProject && styles.project)}>
      <th scope="row" className={styles.name}>
        {row.name}
      </th>
      {KINDS.map((kind) => (
        <td key={kind.key} className={styles.count}>
          {formatTokens(usage.tokens[kind.key])}
        </td>
      ))}
      <td className={styles.all}>
        <span className={styles.allCell}>
          {!isProject && (
            <span
              className={styles.track}
              aria-hidden
              style={{ '--share': shareOf(usage.allTokens, largest) } as CSSProperties}
            >
              <span className={styles.bar} />
            </span>
          )}
          <span className={styles.allCount}>{formatTokens(usage.allTokens)}</span>
        </span>
      </td>
      <td className={styles.count}>{usage.turns}</td>
      <td className={styles.model} title={usage.lastModel ?? undefined}>
        {isProject ? '' : (usage.lastModel ?? '—')}
      </td>
    </tr>
  );
}

/**
 * What the project spent in tokens: one row for every AI specialist, the
 * Advisor and those who left, the one that spent the most first, with the
 * tokens of every kind, the sum, the turns and the model it ran on last;
 * and the project in all at the foot. The bars are quiet and on one scale.
 * A person spends nothing and is not listed. Under it, what is known about
 * the limits of the Claude plan the specialists share.
 */
export function UsagePart({ members, usage }: UsagePartProps) {
  const table = usage ? usageTable(members, usage) : undefined;
  const hasTeam = members.length > 0;
  const largest = Math.max(1, ...(table?.rows.map((row) => row.usage.allTokens) ?? []));
  return (
    <DashboardSection
      title="Usage"
      aside={
        table && (
          <span className={styles.total}>
            The project: {usageInWords(table.total.usage.allTokens)}
          </span>
        )
      }
    >
      {!table || table.rows.length === 0 ? (
        <p className={styles.quiet}>
          {!table
            ? 'Counting what was used…'
            : hasTeam
              ? 'Nothing has used tokens yet. The count grows as the specialists and the Advisor work.'
              : 'Nothing has used tokens yet. Nobody is on the team, and the Advisor has not planned the project.'}
        </p>
      ) : (
        <table className={styles.table}>
          <thead>
            <tr>
              <th className={classNames('label', styles.nameColumn)}>Who</th>
              {KINDS.map((kind) => (
                <th
                  key={kind.key}
                  className={classNames('label', styles.countColumn, kind.isWide && styles.wide)}
                >
                  {kind.title}
                </th>
              ))}
              <th className={classNames('label', styles.allColumn)}>In all</th>
              <th className={classNames('label', styles.turnsColumn)}>Turns</th>
              <th className={classNames('label', styles.modelColumn)}>Model last</th>
            </tr>
          </thead>
          <tbody>
            {table.rows.map((row) => (
              <Row key={row.key} row={row} largest={largest} />
            ))}
          </tbody>
          <tfoot>
            <Row row={table.total} largest={largest} />
          </tfoot>
        </table>
      )}
      <PlanLimitPart />
    </DashboardSection>
  );
}
