//! The checklist of a task: short items, in an order the manager chooses,
//! each done or not done
//! ([decision 0030](../../../docs/decisions/0030-tags-and-checklists.md)).
//!
//! An item is lighter than a subtask: it has no owner, no status and no
//! dates. A task has one checklist, which may be empty. The checklist is a
//! part of its task: it is changed through the task, and stored with it.

use crate::error::DomainError;
use crate::ids::ChecklistItemId;
use crate::task::Rank;
use crate::text::ChecklistText;

/// One item of a checklist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecklistItem {
    id: ChecklistItemId,
    text: ChecklistText,
    done: bool,
    rank: Rank,
}

impl ChecklistItem {
    /// Recreates an item from stored data.
    #[must_use]
    pub const fn restore(id: ChecklistItemId, text: ChecklistText, done: bool, rank: Rank) -> Self {
        Self {
            id,
            text,
            done,
            rank,
        }
    }

    /// The identifier of the item.
    #[must_use]
    pub const fn id(&self) -> ChecklistItemId {
        self.id
    }

    /// What is to be done.
    #[must_use]
    pub const fn text(&self) -> &ChecklistText {
        &self.text
    }

    /// Whether it is done.
    #[must_use]
    pub const fn is_done(&self) -> bool {
        self.done
    }

    /// Its place among the items of the checklist: items are listed by it,
    /// the smallest first. Moving an item changes only its own rank, so two
    /// computers that each move another item both keep their move.
    #[must_use]
    pub const fn rank(&self) -> Rank {
        self.rank
    }
}

/// The items of the checklist of a task, in their order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Checklist {
    items: Vec<ChecklistItem>,
}

impl Checklist {
    /// The most items a checklist may have. A list longer than this is a
    /// plan, which is better made of tasks.
    pub const MOST_ITEMS: usize = 100;

    /// A checklist of these items, put in their order: by rank, and by
    /// identifier where ranks are equal, which two computers can leave
    /// behind.
    #[must_use]
    pub fn of(mut items: Vec<ChecklistItem>) -> Self {
        items.sort_by_key(|item| (item.rank, item.id));
        Self { items }
    }

    /// The items, in their order.
    #[must_use]
    pub fn items(&self) -> &[ChecklistItem] {
        &self.items
    }

    /// The item with an identifier, if the checklist has it.
    #[must_use]
    pub fn item(&self, id: ChecklistItemId) -> Option<&ChecklistItem> {
        self.items.iter().find(|item| item.id == id)
    }

    /// Whether the checklist has no items.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// How many items are done, and how many there are.
    #[must_use]
    pub fn progress(&self) -> (usize, usize) {
        let done = self.items.iter().filter(|item| item.done).count();
        (done, self.items.len())
    }

    /// Adds an item, not done, at the end.
    pub(crate) fn add(&mut self, text: ChecklistText) -> Result<ChecklistItemId, DomainError> {
        if self.items.len() >= Self::MOST_ITEMS {
            return Err(DomainError::ChecklistFull {
                max: Self::MOST_ITEMS,
            });
        }
        let last = self.items.last().map(|item| item.rank);
        let item = ChecklistItem {
            id: ChecklistItemId::generate(),
            text,
            done: false,
            rank: Rank::between(last, None),
        };
        let id = item.id;
        self.items.push(item);
        Ok(id)
    }

    /// Gives an item another text. Says whether anything changed.
    pub(crate) fn rename(
        &mut self,
        id: ChecklistItemId,
        text: ChecklistText,
    ) -> Result<bool, DomainError> {
        let item = self.item_mut(id)?;
        let changed = item.text != text;
        item.text = text;
        Ok(changed)
    }

    /// Marks an item done or not done. Says whether anything changed.
    pub(crate) fn tick(&mut self, id: ChecklistItemId, done: bool) -> Result<bool, DomainError> {
        let item = self.item_mut(id)?;
        let changed = item.done != done;
        item.done = done;
        Ok(changed)
    }

    /// Moves an item to a place among the others. Place 0 is the first; a
    /// place past the end is the last. Only the moved item gets another
    /// rank, unless no rank fits between its new neighbours: then every
    /// item is given a rank again, a step apart. Says whether anything
    /// changed.
    pub(crate) fn move_to(
        &mut self,
        id: ChecklistItemId,
        place: usize,
    ) -> Result<bool, DomainError> {
        let from = self
            .items
            .iter()
            .position(|item| item.id == id)
            .ok_or(DomainError::NoSuchChecklistItem)?;
        let place = place.min(self.items.len().saturating_sub(1));
        if place == from {
            return Ok(false);
        }
        let mut moving = self.items.remove(from);
        let before = place
            .checked_sub(1)
            .and_then(|index| self.items.get(index))
            .map(|item| item.rank);
        let after = self.items.get(place).map(|item| item.rank);
        let rank = Rank::between(before, after);
        moving.rank = rank;
        self.items.insert(place, moving);
        if !rank.fits_between(before, after) {
            let mut previous = None;
            for item in &mut self.items {
                item.rank = Rank::between(previous, None);
                previous = Some(item.rank);
            }
        }
        Ok(true)
    }

    /// Removes an item.
    pub(crate) fn remove(&mut self, id: ChecklistItemId) -> Result<(), DomainError> {
        let before = self.items.len();
        self.items.retain(|item| item.id != id);
        if self.items.len() == before {
            return Err(DomainError::NoSuchChecklistItem);
        }
        Ok(())
    }

    /// The item with an identifier, to change it.
    fn item_mut(&mut self, id: ChecklistItemId) -> Result<&mut ChecklistItem, DomainError> {
        self.items
            .iter_mut()
            .find(|item| item.id == id)
            .ok_or(DomainError::NoSuchChecklistItem)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(words: &str) -> ChecklistText {
        ChecklistText::parse(words).unwrap()
    }

    fn texts(checklist: &Checklist) -> Vec<&str> {
        checklist
            .items()
            .iter()
            .map(|item| item.text().as_str())
            .collect()
    }

    fn three() -> (Checklist, [ChecklistItemId; 3]) {
        let mut checklist = Checklist::default();
        let first = checklist.add(text("Order the stencil")).unwrap();
        let second = checklist.add(text("Bake the board")).unwrap();
        let third = checklist.add(text("Check the joints")).unwrap();
        (checklist, [first, second, third])
    }

    #[test]
    fn an_item_is_added_at_the_end_and_not_done() {
        let (checklist, [_, _, third]) = three();

        assert_eq!(
            texts(&checklist),
            ["Order the stencil", "Bake the board", "Check the joints"]
        );
        assert!(!checklist.item(third).unwrap().is_done());
        assert_eq!(checklist.progress(), (0, 3));
    }

    #[test]
    fn an_item_is_ticked_renamed_and_removed() {
        let (mut checklist, [first, second, _]) = three();

        assert!(checklist.tick(second, true).unwrap());
        assert!(!checklist.tick(second, true).unwrap());
        assert!(checklist.rename(first, text("Order two stencils")).unwrap());
        checklist.remove(first).unwrap();

        assert_eq!(texts(&checklist), ["Bake the board", "Check the joints"]);
        assert_eq!(checklist.progress(), (1, 2));
    }

    #[test]
    fn an_item_that_is_not_there_is_reported() {
        let (mut checklist, _) = three();
        let other = ChecklistItemId::generate();

        assert_eq!(
            checklist.tick(other, true),
            Err(DomainError::NoSuchChecklistItem)
        );
        assert_eq!(
            checklist.remove(other),
            Err(DomainError::NoSuchChecklistItem)
        );
        assert_eq!(
            checklist.move_to(other, 0),
            Err(DomainError::NoSuchChecklistItem)
        );
        assert_eq!(DomainError::NoSuchChecklistItem.code(), "not_found");
    }

    #[test]
    fn moving_an_item_changes_only_its_own_rank() {
        let (mut checklist, [first, second, third]) = three();
        let ranks_before: Vec<Rank> = checklist.items().iter().map(ChecklistItem::rank).collect();

        assert!(checklist.move_to(third, 0).unwrap());

        assert_eq!(
            texts(&checklist),
            ["Check the joints", "Order the stencil", "Bake the board"]
        );
        assert_eq!(checklist.item(first).unwrap().rank(), ranks_before[0]);
        assert_eq!(checklist.item(second).unwrap().rank(), ranks_before[1]);
        assert!(!checklist.move_to(third, 0).unwrap());
        checklist.move_to(third, 99).unwrap();
        assert_eq!(texts(&checklist)[2], "Check the joints");
    }

    #[test]
    fn items_too_close_to_tell_apart_are_given_ranks_again() {
        let (mut checklist, [_, second, third]) = three();
        // Putting the last item second, again and again, halves the room
        // after the first item each time, until no number fits.
        for round in 0..1200 {
            let moving = if round % 2 == 0 { third } else { second };
            checklist.move_to(moving, 1).unwrap();
        }
        let ranks: Vec<Rank> = checklist.items().iter().map(ChecklistItem::rank).collect();

        assert!(ranks.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(
            texts(&checklist),
            ["Order the stencil", "Bake the board", "Check the joints"]
        );
    }

    #[test]
    fn a_checklist_has_at_most_100_items() {
        let mut checklist = Checklist::default();
        for number in 0..Checklist::MOST_ITEMS {
            checklist.add(text(&format!("Item {number}"))).unwrap();
        }

        let error = checklist.add(text("One more")).unwrap_err();

        assert_eq!(error.to_string(), "a checklist has at most 100 items");
        assert_eq!(error.code(), "rule_broken");
    }

    #[test]
    fn stored_items_are_put_in_order_of_their_ranks() {
        let low = ChecklistItem::restore(
            ChecklistItemId::generate(),
            text("First"),
            false,
            Rank::from_value(-5.0),
        );
        let high = ChecklistItem::restore(
            ChecklistItemId::generate(),
            text("Second"),
            true,
            Rank::from_value(3.0),
        );

        let checklist = Checklist::of(vec![high, low]);

        assert_eq!(texts(&checklist), ["First", "Second"]);
    }

    #[test]
    fn an_item_text_is_tidied_and_has_at_most_200_characters() {
        assert_eq!(
            text("  Solder \n the header ").as_str(),
            "Solder the header"
        );
        assert_eq!(
            ChecklistText::parse("").unwrap_err().to_string(),
            "the checklist item must not be empty"
        );
        assert!(ChecklistText::parse(&"x".repeat(200)).is_ok());
        assert!(ChecklistText::parse(&"x".repeat(201)).is_err());
    }
}
