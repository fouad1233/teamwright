import type { ProjectTemplate, WorkflowTemplate } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';

import { kindOf } from '../views/viewKinds';
import styles from './TemplateChoice.module.css';

interface TemplateChoiceProps {
  templates: readonly ProjectTemplate[];
  chosen: WorkflowTemplate;
  onChoose: (template: WorkflowTemplate) => void;
}

/**
 * The templates a new project can start from, one row each with a small
 * picture of the lanes it gives, and under them what the chosen one is for
 * and what else it starts with (decision 0033). Everything shown is drawn
 * from the templates the backend lists.
 */
export function TemplateChoice({ templates, chosen, onChoose }: TemplateChoiceProps) {
  const chosenTemplate = templates.find((template) => template.key === chosen);

  return (
    <fieldset className={styles.choice}>
      <legend className={classNames('label', styles.legend)}>Start from</legend>
      <div className={styles.rows}>
        {templates.map((template) => (
          <label
            key={template.key}
            className={classNames(styles.row, template.key === chosen && styles.chosen)}
          >
            <input
              type="radio"
              name="template"
              className={styles.radio}
              checked={template.key === chosen}
              onChange={() => {
                onChoose(template.key);
              }}
            />
            <span className={styles.name}>{template.name}</span>
            <LanesPicture template={template} />
          </label>
        ))}
      </div>
      {chosenTemplate && <TemplatePlate template={chosenTemplate} />}
    </fieldset>
  );
}

/**
 * The lanes a template gives, drawn small: one column for each status, in
 * its pigment, with its limit written under it in figures.
 */
export function LanesPicture({ template }: { template: ProjectTemplate }) {
  const description = template.statuses
    .map((status) =>
      status.limit === null ? status.name : `${status.name}, limit ${status.limit.toString()}`,
    )
    .join('; ');
  return (
    <span className={styles.lanes} role="img" aria-label={`Lanes: ${description}`}>
      {template.statuses.map((status) => (
        <span key={status.name} className={styles.lane} title={status.name}>
          <span className={styles.laneBody} data-tone={status.color} />
          <span className={styles.laneLimit}>{status.limit ?? ''}</span>
        </span>
      ))}
    </span>
  );
}

/** What the chosen template is for, and what a project made from it starts with. */
function TemplatePlate({ template }: { template: ProjectTemplate }) {
  const hasOwnAreas = !(template.areas.length === 1 && template.areas[0] === 'General');
  return (
    <div className={styles.plate} aria-live="polite">
      <p className={styles.description}>{template.description}</p>
      <dl className={styles.facts}>
        <dt className="label">Lanes</dt>
        <dd className={styles.statuses}>
          {template.statuses.map((status) => (
            <span key={status.name} className={styles.status}>
              <span className={styles.dot} data-tone={status.color} />
              {status.name}
              {status.limit !== null && (
                <span className={styles.limitFigure}>{status.limit.toString()}</span>
              )}
            </span>
          ))}
        </dd>
        <dt className="label">Views</dt>
        <dd className={styles.views}>
          {template.views.map((kind) => {
            const { icon: Icon, label } = kindOf(kind);
            return (
              <span key={kind} className={styles.view}>
                <Icon aria-hidden />
                {label}
              </span>
            );
          })}
        </dd>
        {hasOwnAreas && (
          <>
            <dt className="label">Areas</dt>
            <dd>{template.areas.join(', ')}</dd>
          </>
        )}
        {template.plansFirstSprint && (
          <>
            <dt className="label">Sprint</dt>
            <dd>Sprint 1, planned and empty</dd>
          </>
        )}
      </dl>
    </div>
  );
}
