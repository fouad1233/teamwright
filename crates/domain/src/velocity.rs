//! The pace of a team: how much it finishes in a sprint, told from the
//! sprints it has completed, and what that pace says about the work ahead.
//!
//! The pace is a range, never one number: the least and the most the team
//! finished in each of its last completed sprints. With fewer than three
//! completed sprints it is too early to tell one. Everything here is worked
//! out by rules; nothing is written by a model.

use crate::macros::text_enum;
use crate::sprint::{SprintSummary, SprintUnit};

/// How many completed sprints a pace needs before it is told.
pub const FEWEST_SPRINTS_FOR_A_PACE: usize = 3;

/// How many of the latest completed sprints a pace is read from. Older ones
/// say less about the team as it is now.
pub const MOST_SPRINTS_FOR_A_PACE: usize = 6;

/// The pace of a team, read from its completed sprints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Velocity {
    /// No sprint has been completed yet.
    NoSprints,
    /// Too few sprints were completed to tell a pace.
    TooEarly {
        /// What the work of the sprints is counted in.
        unit: SprintUnit,
        /// What each completed sprint finished, the latest first.
        finished: Vec<u32>,
    },
    /// The pace: what the team finished in its last sprints.
    Known(Pace),
}

/// What the team finished in each of its last completed sprints, as a range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pace {
    /// What the work is counted in.
    pub unit: SprintUnit,
    /// The least it finished in one of the sprints.
    pub low: u32,
    /// The most it finished in one of them.
    pub high: u32,
    /// How many sprints the range is read from.
    pub sprints: usize,
}

impl Velocity {
    /// Reads the pace from the summaries of the completed sprints of a
    /// project, the latest first.
    ///
    /// Only sprints counted in the unit of the latest one are read: points
    /// and tasks cannot be added up. Of those, the latest
    /// [`MOST_SPRINTS_FOR_A_PACE`] are read, and a pace needs at least
    /// [`FEWEST_SPRINTS_FOR_A_PACE`].
    #[must_use]
    pub fn of(latest_first: &[SprintSummary]) -> Self {
        let Some(latest) = latest_first.first() else {
            return Self::NoSprints;
        };
        let unit = latest.unit;
        let finished: Vec<u32> = latest_first
            .iter()
            .filter(|summary| summary.unit == unit)
            .take(MOST_SPRINTS_FOR_A_PACE)
            .map(|summary| summary.completed)
            .collect();
        match (finished.iter().min(), finished.iter().max()) {
            (Some(&low), Some(&high)) if finished.len() >= FEWEST_SPRINTS_FOR_A_PACE => {
                Self::Known(Pace {
                    unit,
                    low,
                    high,
                    sprints: finished.len(),
                })
            }
            _ => Self::TooEarly { unit, finished },
        }
    }

    /// The unit of the sprints the pace is read from, if any was completed.
    #[must_use]
    pub const fn unit(&self) -> Option<SprintUnit> {
        match self {
            Self::NoSprints => None,
            Self::TooEarly { unit, .. } => Some(*unit),
            Self::Known(pace) => Some(pace.unit),
        }
    }

    /// The pace, once it can be told.
    #[must_use]
    pub const fn pace(&self) -> Option<Pace> {
        match self {
            Self::Known(pace) => Some(*pace),
            Self::NoSprints | Self::TooEarly { .. } => None,
        }
    }
}

/// How some work compares with the pace of the team.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Load {
    /// Less than the team has finished in any of its last sprints.
    Below,
    /// Within what it has finished.
    Within,
    /// More than it has finished in any of them.
    Above,
}

text_enum!(Load, kind = "load", {
    Below => "below",
    Within => "within",
    Above => "above",
});

/// How many sprints some work would take at a pace: at the most the team
/// finished, and at the least.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SprintsNeeded {
    /// At the best pace.
    pub fewest: u32,
    /// At the worst pace. Nothing when the team once finished nothing in a
    /// sprint: at that pace the work never ends.
    pub most: Option<u32>,
}

impl Pace {
    /// How some work compares with the pace.
    #[must_use]
    pub const fn load_of(&self, work: u32) -> Load {
        if work < self.low {
            Load::Below
        } else if work > self.high {
            Load::Above
        } else {
            Load::Within
        }
    }

    /// How many sprints some work would take at this pace.
    #[must_use]
    pub fn sprints_for(&self, work: u32) -> SprintsNeeded {
        let sprints = |per_sprint: u32| work.div_ceil(per_sprint);
        if work == 0 {
            return SprintsNeeded {
                fewest: 0,
                most: Some(0),
            };
        }
        SprintsNeeded {
            fewest: if self.high == 0 {
                u32::MAX
            } else {
                sprints(self.high)
            },
            most: (self.low > 0).then(|| sprints(self.low)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn points(completed: u32) -> SprintSummary {
        SprintSummary {
            unit: SprintUnit::Points,
            committed: completed + 2,
            completed,
        }
    }

    fn tasks(completed: u32) -> SprintSummary {
        SprintSummary {
            unit: SprintUnit::Tasks,
            committed: completed,
            completed,
        }
    }

    #[test]
    fn without_completed_sprints_there_is_no_pace() {
        assert_eq!(Velocity::of(&[]), Velocity::NoSprints);
        assert_eq!(Velocity::of(&[]).unit(), None);
    }

    #[test]
    fn with_fewer_than_three_sprints_it_is_too_early() {
        assert_eq!(
            Velocity::of(&[points(8), points(5)]),
            Velocity::TooEarly {
                unit: SprintUnit::Points,
                finished: vec![8, 5]
            }
        );
    }

    #[test]
    fn the_pace_is_the_range_of_what_the_last_sprints_finished() {
        let velocity = Velocity::of(&[points(5), points(11), points(9)]);

        assert_eq!(
            velocity.pace(),
            Some(Pace {
                unit: SprintUnit::Points,
                low: 5,
                high: 11,
                sprints: 3
            })
        );
    }

    #[test]
    fn only_the_last_six_sprints_count() {
        let velocity = Velocity::of(&[8, 9, 10, 8, 9, 10, 1, 40].map(points))
            .pace()
            .unwrap();

        assert_eq!((velocity.low, velocity.high, velocity.sprints), (8, 10, 6));
    }

    #[test]
    fn sprints_counted_in_another_unit_are_left_out() {
        let velocity = Velocity::of(&[points(5), tasks(3), points(7), tasks(2), points(6)]);

        assert_eq!(
            velocity.pace(),
            Some(Pace {
                unit: SprintUnit::Points,
                low: 5,
                high: 7,
                sprints: 3
            })
        );
        assert!(matches!(
            Velocity::of(&[tasks(3), points(7), points(8), points(9)]),
            Velocity::TooEarly { .. }
        ));
    }

    #[test]
    fn work_is_compared_with_the_range() {
        let pace = Velocity::of(&[points(5), points(11), points(9)])
            .pace()
            .unwrap();

        assert_eq!(pace.load_of(4), Load::Below);
        assert_eq!(pace.load_of(5), Load::Within);
        assert_eq!(pace.load_of(11), Load::Within);
        assert_eq!(pace.load_of(12), Load::Above);
    }

    #[test]
    fn the_rest_of_the_work_takes_a_range_of_sprints() {
        let pace = Velocity::of(&[points(5), points(11), points(9)])
            .pace()
            .unwrap();

        assert_eq!(
            pace.sprints_for(34),
            SprintsNeeded {
                fewest: 4,
                most: Some(7)
            }
        );
        assert_eq!(
            pace.sprints_for(0),
            SprintsNeeded {
                fewest: 0,
                most: Some(0)
            }
        );
    }

    #[test]
    fn a_sprint_that_finished_nothing_leaves_the_longest_forecast_open() {
        let pace = Velocity::of(&[points(0), points(6), points(4)])
            .pace()
            .unwrap();

        assert_eq!(
            pace.sprints_for(12),
            SprintsNeeded {
                fewest: 2,
                most: None
            }
        );
    }
}
