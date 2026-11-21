import { useState } from 'react';

import type { CustomField, Project, Task } from '@/shared/api/types';
import { taskReference } from '@/shared/lib/format';
import { Button } from '@/shared/ui/Button';

import { FieldsDialog } from './FieldsDialog';
import { FieldValueControl } from './FieldValueControl';
import { inOrder } from './fieldModel';
import { useFieldCommands, useFields } from './fieldQueries';
import styles from './TaskFields.module.css';

/** The fields while they are loading. */
const NO_FIELDS: readonly CustomField[] = [];

interface TaskFieldsProps {
  project: Project;
  task: Task;
}

/**
 * The custom fields of a task in its panel: one line for each field of the
 * project, its name and its value, which is changed where it stands. A
 * project without fields says what they are for.
 */
export function TaskFields({ project, task }: TaskFieldsProps) {
  const fields = inOrder(useFields(project.id).data ?? NO_FIELDS);
  const commands = useFieldCommands(project.id);
  const [isEditing, setEditing] = useState(false);
  const reference = taskReference(project, task);

  return (
    <div className={styles.fields}>
      {fields.length === 0 ? (
        <p className={styles.none}>
          The project has no fields yet. A field holds what tasks of a kind need, such as a
          supplier, a cost or a datasheet.
        </p>
      ) : (
        <dl className={styles.list}>
          {fields.map((field) => (
            <div key={field.id} className={styles.line}>
              <dt className={styles.name} title={field.name}>
                {field.name}
              </dt>
              <dd className={styles.value}>
                <FieldValueControl
                  field={field}
                  value={task.fieldValues[field.id]}
                  label={`${field.name} of ${reference}`}
                  look="panel"
                  onChange={(value) => commands.setValue(task.id, field.id, value)}
                />
              </dd>
            </div>
          ))}
        </dl>
      )}
      <div>
        <Button
          variant="ghost"
          onClick={() => {
            setEditing(true);
          }}
        >
          {fields.length === 0 ? 'Add fields' : 'Edit the fields'}
        </Button>
      </div>
      <FieldsDialog
        open={isEditing}
        project={project}
        onClose={() => {
          setEditing(false);
        }}
      />
    </div>
  );
}
