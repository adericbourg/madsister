//! Undo/redo (F-ED-7).

use super::song::Song;

/// A song and the net semitones it is transposed by since it was opened, so Reset can undo them. The transposition is
/// kept beside the song, not in it, so it never reaches the file.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub song: Song,
    pub transposition: i32,
}

// shortcut: full Song snapshots instead of inverse commands; revisit only if memory becomes a problem.
#[derive(Debug, Clone)]
pub struct History {
    past: Vec<Snapshot>,
    present: Snapshot,
    future: Vec<Snapshot>,
}

impl History {
    pub fn new(song: Song) -> Self {
        History { past: Vec::new(), present: Snapshot { song, transposition: 0 }, future: Vec::new() }
    }

    pub fn present(&self) -> &Snapshot {
        &self.present
    }

    pub fn song(&self) -> &Song {
        &self.present.song
    }

    /// Records an edit; it keeps the present transposition. A song equal to the present one records nothing.
    pub fn apply(&mut self, song: Song) {
        let transposition = self.present.transposition;
        self.apply_snapshot(Snapshot { song, transposition });
    }

    pub fn apply_snapshot(&mut self, next: Snapshot) {
        if next != self.present {
            self.past.push(std::mem::replace(&mut self.present, next));
            self.future.clear();
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    pub fn undo(&mut self) {
        if let Some(previous) = self.past.pop() {
            self.future.push(std::mem::replace(&mut self.present, previous));
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.future.pop() {
            self.past.push(std::mem::replace(&mut self.present, next));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::song::{Meter, empty_song};

    fn titled(title: &str) -> Song {
        let mut song = empty_song(Meter { beats: 4, unit: 4, extra: Default::default() }, None);
        song.meta.title = title.into();
        song
    }

    #[test]
    fn apply_undo_redo_navigates_snapshots() {
        // Given a history with three applied edits
        let mut h = History::new(titled("s0"));
        assert!(!h.can_undo());
        for title in ["s1", "s2", "s3"] {
            h.apply(titled(title));
        }

        // When undoing twice then redoing once
        h.undo();
        h.undo();
        h.redo();

        // Then the present is the second edit, both directions are available
        assert_eq!(h.song().meta.title, "s2");
        assert!(h.can_undo());
        assert!(h.can_redo());
    }

    #[test]
    fn apply_when_after_undo_clears_redo() {
        // Given an undone edit
        let mut h = History::new(titled("s0"));
        h.apply(titled("s1"));
        h.undo();

        // When applying a new edit
        h.apply(titled("s2"));

        // Then redo is no longer possible, undo returns to the start
        assert!(!h.can_redo());
        h.undo();
        assert_eq!(h.song().meta.title, "s0");
    }

    #[test]
    fn apply_of_an_unchanged_song_records_nothing() {
        // Given a fresh history
        let mut h = History::new(titled("s0"));

        // When applying the same song, and undoing/redoing at the edges
        h.apply(h.song().clone());
        h.undo();
        h.redo();

        // Then nothing is recorded
        assert!(!h.can_undo());
        assert!(!h.can_redo());
    }

    #[test]
    fn apply_keeps_the_transposition_and_undo_restores_the_previous_one() {
        // Given a song transposed by 2
        let mut h = History::new(titled("s0"));
        h.apply_snapshot(Snapshot { song: titled("up"), transposition: 2 });

        // When editing it
        h.apply(titled("edited"));

        // Then the edit is still transposed by 2, and undoing back to the start restores 0
        assert_eq!(h.present().transposition, 2);
        h.undo();
        h.undo();
        assert_eq!(h.present().transposition, 0);
    }
}
