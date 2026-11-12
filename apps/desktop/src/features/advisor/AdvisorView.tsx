import { Play, Square } from 'lucide-react';
import { useState, type FormEvent } from 'react';

import { useNavigation } from '@/app/navigation';
import { useIsDemo } from '@/shared/api/BackendProvider';
import type { Effort, NewAdvice, Project, Turn } from '@/shared/api/types';
import { classNames } from '@/shared/lib/classNames';
import { formatMoment } from '@/shared/lib/format';
import { Button } from '@/shared/ui/Button';
import { ChoiceInput, Field, TextArea } from '@/shared/ui/Field';
import { ChoiceMenu } from '@/shared/ui/Menu';

import { useTeamMembers } from '../team/TeamContext';
import { EFFORT_CHOICES, LEFT_TO_THE_PROGRAM, modelChoices } from '../team/profileChoices';
import { effortLabel, modelLabel } from '../team/teamModel';
import { ReplyBox, TurnView, WorkingMark } from '../work/ConversationView';
import { useConversationsOfProject } from '../work/WorkContext';
import { runningTurn, whyProgramIsNotReady } from '../work/workModel';
import { useAgentProgram, useWorkCommands } from '../work/workQueries';
import styles from './AdvisorView.module.css';
import {
  advisorConversation,
  answerInWords,
  openInterview,
  openProposal,
  proposalInWords,
} from './advisorModel';
import { InterviewCard } from './InterviewCard';
import { ProposalCard } from './ProposalCard';

/**
 * Where a project is planned with the Advisor: the manager says what they
 * want, answers what the Advisor asks, and changes and approves what it
 * proposes. Nothing is added to the project before that.
 */
export function AdvisorView({ project }: { project: Project }) {
  const conversations = useConversationsOfProject();
  const team = useTeamMembers();
  const commands = useWorkCommands(project.id);
  const conversation = advisorConversation(conversations);
  // The manager can ask for a plan of something new while an earlier
  // conversation is on the page. Until it starts, the earlier one stays.
  const [isStartingAnew, setIsStartingAnew] = useState(false);

  if (!conversation || isStartingAnew) {
    return (
      <div className={styles.advisor}>
        <StartPlanning
          project={project}
          onStart={async (advice) => {
            const hasStarted = await commands.advise(advice);
            if (hasStarted) {
              setIsStartingAnew(false);
            }
            return hasStarted;
          }}
          onCancel={
            conversation
              ? () => {
                  setIsStartingAnew(false);
                }
              : undefined
          }
        />
      </div>
    );
  }

  const working = runningTurn(conversation);
  const interview = openInterview(conversation);
  const proposal = openProposal(conversation);
  const lastPlace = conversation.turns.length - 1;

  return (
    <div className={styles.advisor}>
      <div className={styles.column}>
        <header className={styles.who}>
          <h2 className={styles.name}>Advisor</h2>
          <span className={styles.space} />
          {working ? (
            <>
              <WorkingMark />
              <Button
                variant="ghost"
                icon={<Square />}
                onClick={() => {
                  void commands.stop(conversation.id);
                }}
              >
                Stop
              </Button>
            </>
          ) : (
            <Button
              variant="ghost"
              onClick={() => {
                setIsStartingAnew(true);
              }}
            >
              Plan something new
            </Button>
          )}
        </header>

        <ol className={styles.turns}>
          {conversation.turns.map((turn, place) => (
            <li key={turn.id}>
              <TurnView turn={turn} reportTitle={null}>
                <Outcome turn={turn} isLatest={place === lastPlace} />
              </TurnView>
            </li>
          ))}
        </ol>

        {interview && (
          <InterviewCard
            // A card of its own for every interview, so nothing put down for one lingers for the next.
            key={conversation.turns.length}
            interview={interview}
            onAnswer={(answers) => commands.answerInterview(conversation.id, answers)}
          />
        )}
      </div>

      {proposal && (
        <div className={styles.wide}>
          <ProposalCard
            key={conversation.turns.length}
            proposal={proposal}
            team={team}
            onApprove={(approved) => commands.approveProposal(conversation.id, approved)}
          />
        </div>
      )}

      {/* While the Advisor waits for answers, answering is the only way to
          go on. Otherwise the manager can write to it. */}
      {!working && !interview && (
        <div className={styles.column}>
          <ReplyBox
            label="Message to the Advisor"
            placeholder={
              proposal
                ? 'Say what to change, and the Advisor proposes again…'
                : 'Say what to plan next…'
            }
            onSend={(message) => commands.reply(conversation.id, message)}
          />
        </div>
      )}
    </div>
  );
}

interface OutcomeProps {
  turn: Turn;
  /** Whether the turn is the last of its conversation. */
  isLatest: boolean;
}

/**
 * What came of a turn of the Advisor, once it lies behind: the questions
 * with the answers the manager gave, or what became of a proposal. What is
 * still open is shown in full under the turns, so it is left out here.
 */
function Outcome({ turn, isLatest }: OutcomeProps) {
  const navigation = useNavigation();
  const { interview, proposal } = turn;

  if (interview?.answers) {
    const answers = interview.answers;
    return (
      <dl className={styles.answered}>
        {interview.questions.map((question, place) => {
          const answer = answers[place];
          return (
            // The questions of an interview never change, so their place identifies them.
            <div key={place} className={styles.pair}>
              <dt className={styles.asked} data-selectable>
                {question.question}
              </dt>
              <dd className={styles.given} data-selectable>
                {answer ? answerInWords(question, answer) : ''}
              </dd>
            </div>
          );
        })}
      </dl>
    );
  }

  if (proposal?.approvedAt) {
    return (
      <p className={styles.became}>
        <span className={classNames('label', styles.approved)}>Approved</span>
        <span>
          {formatMoment(proposal.approvedAt)} · {proposalInWords(proposal)}
        </span>
        <button
          type="button"
          className={styles.link}
          onClick={() => {
            navigation.showView('board');
          }}
        >
          Open the board
        </button>
      </p>
    );
  }

  if (proposal && !isLatest) {
    return (
      <p className={styles.became}>
        <span className="label">Proposed</span>
        <span>{proposalInWords(proposal)}. A later proposal took its place.</span>
      </p>
    );
  }

  return null;
}

interface StartPlanningProps {
  project: Project;
  /** Starts the conversation. Resolves to whether it started. */
  onStart: (advice: NewAdvice) => Promise<boolean>;
  /** Goes back to the conversation that is there, if there is one. */
  onCancel?: () => void;
}

/** The beginning: what the Advisor does, and room to say what is wanted. */
function StartPlanning({ project, onStart, onCancel }: StartPlanningProps) {
  const program = useAgentProgram();
  const isDemo = useIsDemo();
  const [brief, setBrief] = useState(project.description);
  const [model, setModel] = useState<string | null>(null);
  const [effort, setEffort] = useState<Effort | null>(null);
  const [isStarting, setIsStarting] = useState(false);
  const obstacle = program.data ? whyProgramIsNotReady(program.data) : null;
  const canStart = brief.trim() !== '' && !isStarting && !program.isPending && obstacle === null;
  // Asking the program whether it is there takes a few seconds. The button
  // waits for the answer, and the line next to it says what it waits for.
  const hint = isDemo
    ? 'This is the demo. The Advisor here asks the same questions and proposes the same plan, whatever you write.'
    : program.isPending
      ? 'Looking for Claude Code on this computer…'
      : (obstacle ??
        'The Advisor reads the folders of the project and changes nothing. What it uses counts against your Claude plan.');

  async function start() {
    if (!canStart) {
      return;
    }
    setIsStarting(true);
    const hasStarted = await onStart({ brief, model, effort });
    if (!hasStarted) {
      setIsStarting(false);
    }
  }

  function handleSubmit(event: FormEvent) {
    event.preventDefault();
    void start();
  }

  return (
    <form className={styles.start} onSubmit={handleSubmit}>
      <h2 className={styles.title}>Plan this project with the Advisor</h2>
      <p className={styles.lead}>
        Say what you want, in your own words. The Advisor asks what it needs to know. Then it
        proposes the smallest team that can do the job, with a reason for every member, and a plan
        of tasks. You change what does not fit and approve it. Nothing is added to the project
        before that.
      </p>

      <Field label="What you want">
        <TextArea
          className={styles.brief}
          value={brief}
          rows={6}
          placeholder="What is to be built, what you have already, what matters most…"
          onChange={(event) => {
            setBrief(event.target.value);
          }}
        />
      </Field>

      <div className={styles.profile}>
        <Field label="Model">
          <ChoiceMenu
            label="Model"
            choices={modelChoices(model)}
            value={model ?? LEFT_TO_THE_PROGRAM}
            onChoose={(chosen) => {
              setModel(chosen === LEFT_TO_THE_PROGRAM ? null : chosen);
            }}
          >
            <ChoiceInput aria-label="Model" isEmpty={model === null}>
              {model === null ? 'The default of Claude Code' : modelLabel(model)}
            </ChoiceInput>
          </ChoiceMenu>
        </Field>
        <Field label="Effort">
          <ChoiceMenu
            label="Effort"
            choices={EFFORT_CHOICES}
            value={effort ?? LEFT_TO_THE_PROGRAM}
            onChoose={(chosen) => {
              setEffort(chosen === LEFT_TO_THE_PROGRAM ? null : (chosen as Effort));
            }}
          >
            <ChoiceInput aria-label="Effort" isEmpty={effort === null}>
              {effort === null ? 'The default' : effortLabel(effort)}
            </ChoiceInput>
          </ChoiceMenu>
        </Field>
      </div>

      <div className={styles.actions}>
        <Button variant="primary" type="submit" icon={<Play />} disabled={!canStart}>
          Start planning
        </Button>
        {onCancel && (
          <Button variant="ghost" onClick={onCancel}>
            Cancel
          </Button>
        )}
        <p className={styles.hint}>{hint}</p>
      </div>
    </form>
  );
}
