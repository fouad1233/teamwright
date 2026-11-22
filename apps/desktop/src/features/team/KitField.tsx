import type { KitChoice, Library } from '@/shared/api/types';
import { Switch } from '@/shared/ui/Switch';

import styles from './KitField.module.css';
import { isToolGiven, toggleSkill, toggleTool } from './kitModel';

interface KitFieldProps {
  library: Library;
  choice: KitChoice;
  onChange: (choice: KitChoice) => void;
}

/**
 * The skills and connectors of the library given to one specialist. A
 * connector is given tool by tool, never whole (decision 0039).
 */
export function KitField({ library, choice, onChange }: KitFieldProps) {
  if (library.skills.length === 0 && library.connectors.length === 0) {
    return (
      <div className={styles.kit}>
        <span className="label">Skills and connectors</span>
        <p className={styles.note}>
          The library of the project is empty. Add skills and connectors under “Skills and
          connectors” in the Team place, then give them here.
        </p>
      </div>
    );
  }
  return (
    <div className={styles.kit}>
      <span className="label">Skills and connectors</span>
      <p className={styles.note}>
        The specialist gets these and nothing of your own Claude Code setup.
      </p>
      {library.skills.map((skill) => (
        <Switch
          key={skill.id}
          on={choice.skills.includes(skill.id)}
          label={`Skill: ${skill.name}`}
          hint={
            skill.here
              ? skill.description
              : skill.problem !== null
                ? `Not given here: ${skill.problem}.`
                : 'Not on this computer, so not given here.'
          }
          onChange={() => {
            onChange(toggleSkill(choice, skill.id));
          }}
        />
      ))}
      {library.connectors.map((connector) => (
        <fieldset key={connector.id} className={styles.connector}>
          <legend className={styles.legend}>Connector: {connector.name}</legend>
          <p className={styles.warning}>
            This specialist can reach whatever this connector reaches, with the tools you turn on.
          </p>
          {connector.tools.map((tool) => (
            <Switch
              key={tool}
              on={isToolGiven(choice, connector.id, tool)}
              label={tool}
              onChange={() => {
                onChange(toggleTool(choice, connector, tool));
              }}
            />
          ))}
        </fieldset>
      ))}
    </div>
  );
}
