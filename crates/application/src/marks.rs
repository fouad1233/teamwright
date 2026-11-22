//! Where the marks of the blocks of teammate words come from
//! ([decision 0029](../../../docs/decisions/0029-handoffs-and-messages-between-specialists.md),
//! note of 22 November 2026).
//!
//! Every prompt of a specialist's turn frames the words its teammates wrote
//! in blocks that carry a mark made for that prompt
//! ([`teamwright_domain::framing`]). The mark must be one that no text
//! written before the turn can know, so it is random; tests use a fixed one
//! so that what they expect stays exact.

use teamwright_domain::WordsMark;

/// Makes the mark of the blocks of one prompt.
pub trait MarkSource: Send + Sync {
    /// A new mark, never handed out before.
    fn new_mark(&self) -> WordsMark;
}

/// Makes random marks ([`WordsMark::generate`]).
#[derive(Debug, Clone, Copy, Default)]
pub struct RandomMarks;

impl MarkSource for RandomMarks {
    fn new_mark(&self) -> WordsMark {
        WordsMark::generate()
    }
}

/// Hands out the same mark every time, for tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedMark(pub WordsMark);

impl FixedMark {
    /// The mark tests expect: `testmark`.
    #[must_use]
    pub fn for_tests() -> Self {
        Self(test_mark())
    }
}

impl MarkSource for FixedMark {
    fn new_mark(&self) -> WordsMark {
        self.0.clone()
    }
}

/// The mark [`FixedMark::for_tests`] hands out, `testmark`.
#[must_use]
pub fn test_mark() -> WordsMark {
    // The text has the form of a mark, so the fallback is never taken; it
    // keeps this function free of a panic outside tests.
    WordsMark::parse("testmark").unwrap_or_else(|_| WordsMark::generate())
}
