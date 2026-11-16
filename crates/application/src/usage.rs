//! What the specialists and the Advisor of a project have spent, in tokens,
//! and how full the context of each of them is.
//!
//! The figures are read from the turns of the conversations and never
//! stored. The interface only words and draws them, so that no figure is
//! worked out in two places.

use teamwright_domain::{ContextUse, MemberId, TokenCounts, Turn, context_of};

use crate::work::ConversationWithTurns;

/// What one agent has spent, or a group of them together.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentUsage {
    /// The tokens of every turn that has ended, by kind.
    pub tokens: TokenCounts,
    /// How many turns have ended.
    pub turns: u32,
    /// The model the latest turn that named one ran on.
    pub last_model: Option<String>,
    /// How full the context of the agent is: that of its latest
    /// conversation, as its last turn that reported one says. Nothing for
    /// a group, and nothing while that conversation has not reported one.
    pub context: Option<ContextUse>,
}

/// What a member of the team has spent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberUsage {
    /// The member.
    pub member_id: MemberId,
    /// What it has spent.
    pub usage: AgentUsage,
}

/// What a project has spent, and who spent it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectUsage {
    /// Every member that has had a conversation, the one that spent the
    /// most first.
    pub members: Vec<MemberUsage>,
    /// The Advisor, if the project has spoken with it.
    pub advisor: Option<AgentUsage>,
    /// The members who have left the team since, together, if they had
    /// conversations.
    pub former: Option<AgentUsage>,
    /// The whole project.
    pub total: AgentUsage,
}

/// Who a conversation's spending counts for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Spender {
    Member(MemberId),
    Advisor,
    Former,
}

fn spender_of(conversation: &ConversationWithTurns) -> Spender {
    if conversation.conversation.is_with_advisor() {
        return Spender::Advisor;
    }
    conversation
        .conversation
        .member_id()
        .map_or(Spender::Former, Spender::Member)
}

/// What a project has spent, from its conversations with their turns, the
/// oldest conversation first.
#[must_use]
pub fn usage_of(conversations: &[ConversationWithTurns]) -> ProjectUsage {
    let mut spenders: Vec<Spender> = Vec::new();
    for conversation in conversations {
        let spender = spender_of(conversation);
        if !spenders.contains(&spender) {
            spenders.push(spender);
        }
    }
    let of = |spender: Spender| {
        let own: Vec<&ConversationWithTurns> = conversations
            .iter()
            .filter(|conversation| spender_of(conversation) == spender)
            .collect();
        let mut usage = spent_in(&own);
        if spender != Spender::Former {
            usage.context = latest(&own).and_then(|latest| context_of(&latest.turns));
        }
        usage
    };

    let mut members: Vec<MemberUsage> = spenders
        .iter()
        .filter_map(|spender| match spender {
            Spender::Member(member_id) => Some(MemberUsage {
                member_id: *member_id,
                usage: of(*spender),
            }),
            _ => None,
        })
        .collect();
    // The order of joining is kept among those who spent the same.
    members.sort_by_key(|member| std::cmp::Reverse(member.usage.tokens.all()));

    ProjectUsage {
        members,
        advisor: spenders
            .contains(&Spender::Advisor)
            .then(|| of(Spender::Advisor)),
        former: spenders
            .contains(&Spender::Former)
            .then(|| of(Spender::Former)),
        total: spent_in(&conversations.iter().collect::<Vec<_>>()),
    }
}

/// The latest of some conversations: the one started last. Of two started
/// at the same moment, the later in the list.
fn latest<'a>(conversations: &[&'a ConversationWithTurns]) -> Option<&'a ConversationWithTurns> {
    conversations
        .iter()
        .copied()
        .max_by_key(|conversation| conversation.conversation.started_at())
}

/// The tokens, the turns and the last model of some conversations
/// together, without a context.
fn spent_in(conversations: &[&ConversationWithTurns]) -> AgentUsage {
    let turns: Vec<&Turn> = conversations
        .iter()
        .flat_map(|conversation| &conversation.turns)
        .collect();
    let ended: Vec<&&Turn> = turns.iter().filter(|turn| !turn.is_running()).collect();
    AgentUsage {
        tokens: ended
            .iter()
            .filter_map(|turn| turn.tokens())
            .fold(TokenCounts::default(), plus),
        turns: u32::try_from(ended.len()).unwrap_or(u32::MAX),
        last_model: turns
            .iter()
            .filter(|turn| turn.model().is_some())
            .max_by_key(|turn| turn.started_at())
            .and_then(|turn| turn.model())
            .map(str::to_owned),
        context: None,
    }
}

const fn plus(sum: TokenCounts, more: TokenCounts) -> TokenCounts {
    TokenCounts {
        input: sum.input.saturating_add(more.input),
        output: sum.output.saturating_add(more.output),
        cache_read: sum.cache_read.saturating_add(more.cache_read),
        cache_written: sum.cache_written.saturating_add(more.cache_written),
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use teamwright_domain::{ComputerId, Conversation, ProjectId, Timestamp, TurnOrigin};

    use super::*;

    fn moment(minute: u32) -> Timestamp {
        chrono::Utc
            .with_ymd_and_hms(2026, 11, 16, 9, minute, 0)
            .unwrap()
    }

    fn counts(output: u64) -> TokenCounts {
        TokenCounts {
            input: 10,
            output,
            cache_read: 1000,
            cache_written: 200,
        }
    }

    fn window(held: u64) -> Option<ContextUse> {
        Some(ContextUse {
            held,
            window: Some(200_000),
        })
    }

    /// A turn that started at `minute`, ran on `model`, and ended with these
    /// tokens and context; or is still running if it has no tokens.
    fn turn(
        conversation: &Conversation,
        minute: u32,
        model: &str,
        tokens: Option<TokenCounts>,
        context: Option<ContextUse>,
    ) -> Turn {
        let mut turn = Turn::start(
            conversation.id(),
            "Work on it",
            TurnOrigin::Brief,
            moment(minute),
        )
        .unwrap();
        turn.record_model(model.to_owned()).unwrap();
        if let Some(context) = context {
            turn.record_context(context).unwrap();
        }
        if let Some(tokens) = tokens {
            turn.finish("Done.", tokens, moment(minute + 1)).unwrap();
        }
        turn
    }

    fn with_member(
        member: MemberId,
        minute: u32,
        turns: &[(u32, Option<u64>, Option<u64>)],
    ) -> ConversationWithTurns {
        let conversation = Conversation::start(
            ProjectId::generate(),
            member,
            None,
            moment(minute),
            ComputerId::generate(),
        );
        let turns = turns
            .iter()
            .map(|(at, output, held)| {
                turn(
                    &conversation,
                    *at,
                    "claude-sonnet",
                    output.map(counts),
                    held.and_then(window),
                )
            })
            .collect();
        ConversationWithTurns {
            conversation,
            turns,
        }
    }

    #[test]
    fn nothing_spent_is_an_empty_project() {
        assert_eq!(usage_of(&[]), ProjectUsage::default());
    }

    #[test]
    fn the_context_of_an_agent_is_that_of_its_latest_conversation() {
        let engineer = MemberId::generate();
        let older = with_member(engineer, 0, &[(1, Some(100), Some(150_000))]);
        // Started later: its first turn made room and holds less.
        let newer = with_member(
            engineer,
            5,
            &[(6, Some(50), Some(30_000)), (8, Some(20), None)],
        );

        let usage = usage_of(&[older.clone(), newer.clone()]);

        let [member] = usage.members.as_slice() else {
            panic!("{usage:?}");
        };
        assert_eq!(member.member_id, engineer);
        assert_eq!(member.usage.context, window(30_000));
        assert_eq!(member.usage.turns, 3);
        assert_eq!(
            member.usage.tokens,
            plus(plus(counts(100), counts(50)), counts(20))
        );
        assert_eq!(member.usage.last_model.as_deref(), Some("claude-sonnet"));

        // A latest conversation that has not reported a context shows none,
        // even if an older one did.
        let started = with_member(engineer, 9, &[(10, None, None)]);
        let usage = usage_of(&[older, newer, started]);
        assert_eq!(usage.members[0].usage.context, None);
        // The running turn counts nothing yet.
        assert_eq!(usage.members[0].usage.turns, 3);
    }

    #[test]
    fn members_come_by_what_they_spent_and_the_advisor_and_who_left_apart() {
        let small = MemberId::generate();
        let large = MemberId::generate();
        let advisor_conversation = Conversation::start_with_advisor(
            ProjectId::generate(),
            None,
            None,
            moment(0),
            ComputerId::generate(),
        );
        let advisor = ConversationWithTurns {
            turns: vec![turn(
                &advisor_conversation,
                1,
                "claude-opus",
                Some(counts(900)),
                window(170_000),
            )],
            conversation: advisor_conversation,
        };
        let mut left = with_member(MemberId::generate(), 2, &[(3, Some(5), Some(9000))]);
        left.conversation.forget_member();

        let usage = usage_of(&[
            with_member(small, 4, &[(5, Some(1), None)]),
            advisor,
            with_member(large, 6, &[(7, Some(400), None)]),
            left,
        ]);

        let order: Vec<MemberId> = usage
            .members
            .iter()
            .map(|member| member.member_id)
            .collect();
        assert_eq!(order, [large, small]);
        let advisor = usage.advisor.unwrap();
        assert_eq!(advisor.context, window(170_000));
        assert!(advisor.context.unwrap().is_nearly_full());
        assert_eq!(advisor.last_model.as_deref(), Some("claude-opus"));
        let former = usage.former.unwrap();
        assert_eq!(former.tokens, counts(5));
        assert_eq!(former.context, None);
        assert_eq!(usage.total.turns, 4);
        assert_eq!(
            usage.total.tokens.all(),
            [1, 900, 400, 5]
                .iter()
                .map(|output| counts(*output).all())
                .sum::<u64>()
        );
        assert_eq!(usage.total.context, None);
    }
}
