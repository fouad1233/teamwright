import { classNames } from '@/shared/lib/classNames';

import { MANAGER_NAME } from '../team/Assignee';
import { Portrait } from './characters/Portrait';
import type { Place, PlaceFigure } from './officeModel';
import styles from './OfficePlace.module.css';
import { sitterOf } from './roomModel';

/** Who sits at a place: the head and shoulders of their character (decision 0019). */
function Figure({ figure }: { figure: PlaceFigure }) {
  switch (figure.kind) {
    case 'manager':
      return <Portrait sitter={sitterOf(figure)} name={MANAGER_NAME} />;
    case 'advisor':
      return <Portrait sitter={sitterOf(figure)} name="Advisor" />;
    case 'member':
      return (
        <Portrait
          sitter={sitterOf(figure)}
          name={figure.member.name}
          isPerson={figure.member.kind === 'human'}
        />
      );
  }
}

interface OfficePlaceProps {
  place: Place;
  /** Where the place stands on the grid of the drawing, such as B2. */
  reference: string;
  /** Goes where the place leads. Not given for a place that leads nowhere. */
  onPress?: () => void;
}

/**
 * One bench of the office: who sits there, what it is doing now, and what
 * it has used. A lamp hangs over every bench and is lit only while work
 * goes on there; a bench that waits for the manager is held by four gold
 * corners. The words say the same as the lights, so the bench still reads
 * in a picture and with motion turned down.
 */
export function OfficePlace({ place, reference, onPress }: OfficePlaceProps) {
  const isWorking = place.state === 'working';
  const isWaiting = place.state === 'waiting';
  const className = classNames(
    styles.place,
    onPress && styles.pressable,
    isWorking && styles.lit,
    place.state === 'manager' && styles.keystone,
  );

  const contents = (
    <>
      <span className={styles.lamp} aria-hidden />
      {(isWaiting || place.alsoWaits !== null) && (
        <span className={styles.joints} aria-hidden>
          <span />
        </span>
      )}

      <span className={classNames('label', styles.reference)} aria-hidden>
        {reference}
      </span>

      <span className={styles.who}>
        <Figure figure={place.figure} />
        <span className={styles.identity}>
          <span className={styles.name}>{place.name}</span>
          <span className={styles.role}>{place.role}</span>
        </span>
      </span>

      <span className={classNames('label', styles.state)}>
        {isWorking && <span className={styles.atWork} role="img" aria-label="Working now" />}
        {isWaiting && <span className={styles.waits} aria-hidden />}
        {place.stateInWords}
      </span>

      {place.subject && (
        <span className={styles.subject}>
          {place.subject.reference !== null && (
            <span className={styles.taskReference}>{place.subject.reference}</span>
          )}
          <span className={styles.title}>{place.subject.title}</span>
        </span>
      )}

      {place.detail !== null && <span className={styles.detail}>{place.detail}</span>}

      {place.measures.length > 0 && (
        <span className={styles.measures}>
          {place.measures.map((measure) => (
            <span key={measure.label} className={styles.measure}>
              <span className="label">{measure.label}</span>
              <span className={styles.value}>{measure.value}</span>
            </span>
          ))}
        </span>
      )}

      {place.alsoWaits !== null && <span className={styles.alsoWaits}>{place.alsoWaits}</span>}

      <span className={styles.usage}>
        <span className="label">{place.usage.label}</span>
        <span className={styles.value}>{place.usage.value}</span>
      </span>
    </>
  );

  if (!onPress) {
    return (
      <div className={className} data-state={place.state}>
        {contents}
      </div>
    );
  }
  return (
    <button
      type="button"
      className={className}
      data-state={place.state}
      title={place.actionInWords ?? undefined}
      onClick={onPress}
    >
      {contents}
    </button>
  );
}
