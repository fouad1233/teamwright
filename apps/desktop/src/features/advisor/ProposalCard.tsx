import { X } from 'lucide-react';
import { useState } from 'react';

import type { ApprovedProposal, Effort, Member, Priority, Proposal } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatEstimate } from '@/shared/lib/format';
import { Button, IconButton, ValueButton } from '@/shared/ui/Button';
import { ChoiceMenu } from '@/shared/ui/Menu';
import { Prose } from '@/shared/ui/Prose';

import { MemberAvatar } from '../team/Assignee';
import { EFFORT_CHOICES, LEFT_TO_THE_PROGRAM, modelChoices } from '../team/profileChoices';
import { effortLabel, modelLabel } from '../team/teamModel';
import {
  approvalInWords,
  countNewMembers,
  countWithoutOwner,
  draftOf,
  isOnTeam,
  ownerChoices,
  ownerInWords,
  planInWords,
  teamInWords,
  valueOfOwner,
  waitsForInWords,
  withOwner,
  withoutMember,
  withoutTask,
  withProfile,
} from './advisorModel';
import styles from './ProposalCard.module.css';

/** What a task that is not of the ordinary priority says about itself. */
const PRIORITY_LABELS: Record<Priority, string | null> = {
  low: 'Low',
  normal: null,
  high: 'High',
  urgent: 'Urgent',
};

interface ProposalCardProps {
  /** The proposal that waits for the manager. */
  proposal: Proposal;
  /** The team the project has already. */
  team: readonly Member[];
  /** Approves what is left of the proposal. Resolves to whether it was taken. */
  onApprove: (approved: ApprovedProposal) => Promise<boolean>;
}

/**
 * What the Advisor proposes, for the manager to change and to approve: the
 * project in the Advisor's words, the specialists to add, and the tasks.
 *
 * The manager can take a member or a task out, give a task to somebody
 * else, and choose the model and the effort of a member. Nothing here
 * exists in the project before it is approved.
 */
export function ProposalCard({ proposal, team, onApprove }: ProposalCardProps) {
  const [draft, setDraft] = useState(() => draftOf(proposal));
  const [shownTask, setShownTask] = useState<number | null>(null);
  const [isApproving, setIsApproving] = useState(false);
  const owners = ownerChoices(draft, team);
  const withoutOwner = countWithoutOwner(draft.tasks);
  const newMembers = countNewMembers(draft, team);
  const isEmpty = draft.members.length === 0 && draft.tasks.length === 0;
  const isChanged = JSON.stringify(draft) !== JSON.stringify(draftOf(proposal));

  function change(changed: ApprovedProposal) {
    // The places of the tasks may have moved, so no description stays open.
    setShownTask(null);
    setDraft(changed);
  }

  async function approve() {
    setIsApproving(true);
    const wasTaken = await onApprove(draft);
    if (!wasTaken) {
      setIsApproving(false);
    }
  }

  return (
    <section className={styles.card} aria-label="The proposal of the Advisor">
      <p className={classNames('label', styles.proposes)}>The Advisor proposes</p>
      <Prose text={proposal.summary} />

      <div className={styles.part}>
        <h4 className="label">
          Team · {newMembers === 0 ? 'nobody new' : teamInWords(newMembers)}
        </h4>
        {draft.members.length > 0 && (
          <ul className={styles.members}>
            {draft.members.map((member, place) => (
              <li key={member.name} className={styles.member}>
                {/* A proposed member is not on the team yet, so it has no color of its own. */}
                <MemberAvatar member={{ name: member.name, kind: 'agent', color: 'gray' }} />
                <div className={styles.who}>
                  <span className={styles.memberName}>{member.name}</span>
                  {member.responsibility !== '' && (
                    <span className={styles.responsibility}>{member.responsibility}</span>
                  )}
                  {member.reason !== '' && <span className={styles.reason}>{member.reason}</span>}
                </div>
                {isOnTeam(member.name, team) ? (
                  // The team has this member already. Approving does not add
                  // it again and does not change how it runs.
                  <span className={styles.there}>On the team already</span>
                ) : (
                  <>
                    <ChoiceMenu
                      label={`Model of ${member.name}`}
                      choices={modelChoices(member.model)}
                      value={member.model ?? LEFT_TO_THE_PROGRAM}
                      onChoose={(model) => {
                        change(
                          withProfile(draft, place, {
                            model: model === LEFT_TO_THE_PROGRAM ? null : model,
                          }),
                        );
                      }}
                    >
                      <ValueButton
                        className={styles.value}
                        isEmpty={member.model === null}
                        aria-label={`Model of ${member.name}`}
                      >
                        {member.model === null ? 'Default model' : modelLabel(member.model)}
                      </ValueButton>
                    </ChoiceMenu>
                    <ChoiceMenu
                      label={`Effort of ${member.name}`}
                      choices={EFFORT_CHOICES}
                      value={member.effort ?? LEFT_TO_THE_PROGRAM}
                      onChoose={(effort) => {
                        change(
                          withProfile(draft, place, {
                            effort: effort === LEFT_TO_THE_PROGRAM ? null : (effort as Effort),
                          }),
                        );
                      }}
                    >
                      <ValueButton
                        className={styles.value}
                        isEmpty={member.effort === null}
                        aria-label={`Effort of ${member.name}`}
                      >
                        {member.effort === null
                          ? 'Default effort'
                          : `${effortLabel(member.effort)} effort`}
                      </ValueButton>
                    </ChoiceMenu>
                    <IconButton
                      label={`Take ${member.name} out of the proposal`}
                      onClick={() => {
                        change(withoutMember(draft, place));
                      }}
                    >
                      <X />
                    </IconButton>
                  </>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className={styles.part}>
        <h4 className="label">Plan · {planInWords(draft.tasks)}</h4>
        {draft.tasks.length > 0 && (
          <ol className={styles.tasks}>
            {draft.tasks.map((task, place) => {
              const priority = PRIORITY_LABELS[task.priority];
              const isShown = shownTask === place;
              return (
                // Two tasks may have the same title, so the place is part of what identifies one.
                <li key={`${place.toString()} ${task.title}`} className={styles.task}>
                  <span className={styles.place}>{(place + 1).toString()}</span>
                  <button
                    type="button"
                    className={styles.title}
                    aria-expanded={isShown}
                    disabled={task.description === ''}
                    onClick={() => {
                      setShownTask(isShown ? null : place);
                    }}
                  >
                    {task.title}
                  </button>
                  <ChoiceMenu
                    label={`Who does "${task.title}"`}
                    choices={owners}
                    value={valueOfOwner(task.owner)}
                    onChoose={(value) => {
                      const chosen = owners.find((choice) => choice.value === value);
                      change(withOwner(draft, place, chosen?.owner ?? null));
                    }}
                  >
                    <ValueButton
                      className={styles.value}
                      isEmpty={task.owner === null}
                      aria-label={`Who does "${task.title}"`}
                    >
                      {ownerInWords(task.owner)}
                    </ValueButton>
                  </ChoiceMenu>
                  <span className={styles.size}>
                    {priority !== null && <span className={styles.priority}>{priority}</span>}
                    {task.estimateMinutes !== null && formatEstimate(task.estimateMinutes)}
                  </span>
                  <span
                    className={styles.after}
                    title={
                      task.waitsFor.length > 0
                        ? `Waits for the tasks ${waitsForInWords(task)}`
                        : undefined
                    }
                  >
                    {task.waitsFor.length > 0 && `after ${waitsForInWords(task)}`}
                  </span>
                  <IconButton
                    label={`Take "${task.title}" out of the proposal`}
                    onClick={() => {
                      change(withoutTask(draft, place));
                    }}
                  >
                    <X />
                  </IconButton>
                  {isShown && (
                    <p className={styles.description} data-selectable>
                      {task.description}
                    </p>
                  )}
                </li>
              );
            })}
          </ol>
        )}
      </div>

      <footer className={styles.footer}>
        <p className={styles.effect}>
          {isEmpty ? 'Nothing is left of the proposal.' : approvalInWords(draft, team)}
          {withoutOwner > 0 &&
            ` ${withoutOwner === 1 ? '1 task is' : `${withoutOwner.toString()} tasks are`} given to nobody.`}
        </p>
        {isChanged && (
          <Button
            variant="ghost"
            disabled={isApproving}
            onClick={() => {
              change(draftOf(proposal));
            }}
          >
            Undo my changes
          </Button>
        )}
        <Button
          variant="primary"
          disabled={isApproving || isEmpty}
          onClick={() => {
            void approve();
          }}
        >
          Approve
        </Button>
      </footer>
    </section>
  );
}
