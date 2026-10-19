//! Keyboard-first editing of the chart (spec F-ED-3..7): cursor, bar selection, clipboard, inline chord or section-name
//! input, read-only view mode and notices. Pure, so the UI only shows it and forwards events.

use crate::export::sheet::nashville_tonic;
use crate::keymap::{Command, EditorState, KeyInput, Selection, clamp_cursor, key_to_command, next_slot, selected_bars};
use crate::model::chord::parse_chord;
use crate::model::commands::{
    BarRef, Position, SlotRef, Step, WHOLE_SONG, add_section, delete_bar, delete_section, double_tempo, halve_tempo,
    insert_bar, merge_with_previous, move_section, remove_slot, rename_section, set_bar_meter, set_chord, set_key,
    set_notation, set_repeat, shift_phase, split_section, split_slot,
};
use crate::model::history::Snapshot;
use crate::model::song::{Meter, Notation};
use crate::model::transpose::{Spelling, transpose_song};
use crate::model::display::{DisplayStyle, display_chord};
use crate::model::history::History;
use crate::model::nashville::{display_nashville, parse_nashville};
use crate::model::song::{Bar, Song};
use crate::settings::Settings;

pub const MOD_LABEL: &str = if cfg!(target_os = "macos") { "⌘" } else { "Ctrl" };

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftKind {
    Chord,
    Label,
}

/// The inline input's text; `select_all` says where the caret lands: the whole text selected, or after a typed first
/// character.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub kind: DraftKind,
    pub text: String,
    pub select_all: bool,
    pub error: Option<String>,
}

/// What the UI does after a key the editor handled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    /// Not a grid key: let it through.
    Ignored,
    Handled,
    Play,
    /// Play from this time in the audio.
    PlayFrom(f64),
}

/// A field of the Song panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaField {
    Title,
    Artist,
    Key,
    Tempo,
}

/// A right-click menu item (F-ED-6), on a chord, a bar line (`bar` is the bar after it) or a section label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Split(SlotRef),
    AddBarBefore(BarRef),
    AddBarAfter(BarRef),
    RemoveBar(BarRef),
    SplitSectionBefore(BarRef),
    SplitSectionAfter(BarRef),
    /// On a bar line: `bar` may be the bar count (the line closing the section), which appends.
    AddBarAt(BarRef),
    SplitSectionAt(BarRef),
    MergeWithPrevious(usize),
}

pub struct Editor {
    pub history: History,
    cursor: SlotRef,
    pub anchor: Option<BarRef>,
    clipboard: Vec<Bar>,
    pub draft: Option<Draft>,
    pub notice: String,
    pub is_editable: bool,
    pub is_help_open: bool,
}

impl Editor {
    pub fn new(song: Song) -> Self {
        Editor {
            history: History::new(song),
            cursor: SlotRef { section: 0, bar: 0, slot: 0 },
            anchor: None,
            clipboard: Vec::new(),
            draft: None,
            notice: String::new(),
            is_editable: true,
            is_help_open: false,
        }
    }

    pub fn song(&self) -> &Song {
        self.history.song()
    }

    /// Undo/redo and structural edits can leave the cursor dangling: it's always read clamped.
    pub fn cursor(&self) -> SlotRef {
        clamp_cursor(self.song(), self.cursor)
    }

    pub fn selection(&self) -> Option<Selection> {
        self.anchor.map(|anchor| selected_bars(self.cursor(), Some(anchor)))
    }

    fn move_to(&mut self, cursor: SlotRef) {
        self.cursor = cursor;
        self.anchor = None;
    }

    /// Chords are typed and shown as degrees of the key's tonic in Nashville notation.
    fn parse(&self, text: &str, settings: &Settings) -> Result<String, String> {
        match nashville_tonic(self.song(), settings) {
            Some(tonic) => parse_nashville(text, tonic),
            None => parse_chord(text),
        }
    }

    fn show(&self, harte: &str, settings: &Settings) -> String {
        match nashville_tonic(self.song(), settings) {
            Some(tonic) => display_nashville(harte, tonic, DisplayStyle::Intl),
            None => display_chord(harte, DisplayStyle::Intl),
        }
    }

    /// A key pressed on the grid (not in the inline input).
    pub fn key(&mut self, e: KeyInput, settings: &Settings) -> Outcome {
        let cursor = self.cursor();
        let state = EditorState {
            song: self.song(),
            cursor,
            anchor: self.anchor,
            clipboard: &self.clipboard,
            bars_per_row: settings.bars_per_row,
            low_confidence_threshold: settings.low_confidence_threshold,
        };
        let Some(command) = key_to_command(e, &state) else { return Outcome::Ignored };
        let is_editing =
            matches!(command, Command::Edit { .. } | Command::Type(_) | Command::Rename | Command::Undo | Command::Redo);
        if !self.is_editable && is_editing {
            self.notice = format!("Read-only: press {MOD_LABEL}+E to edit");
            return Outcome::Handled;
        }
        self.notice = if command == Command::Refused { "Not possible here".into() } else { String::new() };
        match command {
            Command::Move { cursor, anchor } => {
                self.cursor = cursor;
                self.anchor = anchor;
            }
            Command::Edit { song, cursor } => {
                self.history.apply(*song);
                self.move_to(cursor);
            }
            Command::Copy(bars) => self.clipboard = bars,
            Command::Type(text) => self.draft = Some(Draft { kind: DraftKind::Chord, text, select_all: false, error: None }),
            Command::Rename => {
                let text = self.song().sections[cursor.section].label.clone();
                self.draft = Some(Draft { kind: DraftKind::Label, text, select_all: true, error: None });
            }
            Command::Undo => self.history.undo(),
            Command::Redo => self.history.redo(),
            Command::Help => self.is_help_open = true,
            Command::Play => return Outcome::Play,
            Command::PlayFromCursor => {
                let start = self.song().sections[cursor.section].bars.get(cursor.bar).and_then(|b| b.start_sec);
                match start {
                    Some(start) => return Outcome::PlayFrom(start),
                    None => self.notice = "This bar has no time in the audio".into(),
                }
            }
            Command::Refused => {}
        }
        Outcome::Handled
    }

    pub fn set_draft_text(&mut self, text: &str) {
        if let Some(draft) = &mut self.draft {
            draft.text = text.into();
            draft.error = None;
            draft.select_all = false;
        }
    }

    /// A key pressed in the inline input; false lets the input handle it.
    pub fn draft_key(&mut self, e: KeyInput, settings: &Settings) -> bool {
        let Some(draft) = &self.draft else { return false };
        match e.key {
            "/" if (e.ctrl || e.meta) && draft.kind == DraftKind::Chord => self.commit_and_split(settings),
            "Enter" | "Tab" => self.commit(settings, false),
            "Escape" => self.draft = None,
            _ => return false,
        }
        true
    }

    /// Saves the inline input. `is_blur`: it lost focus, so an invalid draft is dropped rather than flagged, and the
    /// cursor stays where the click put it.
    pub fn commit(&mut self, settings: &Settings, is_blur: bool) {
        let Some(draft) = self.draft.clone() else { return };
        let cursor = self.cursor();
        let text = draft.text.trim();
        let flag = |editor: &mut Editor, error: String| {
            editor.draft = if is_blur { None } else { Some(Draft { error: Some(error), ..draft.clone() }) };
        };
        match draft.kind {
            DraftKind::Label => {
                if text.is_empty() {
                    return flag(self, "a section needs a name".into());
                }
                if text != self.song().sections[cursor.section].label {
                    self.apply(rename_section(self.song(), cursor.section, text));
                }
            }
            DraftKind::Chord if text.is_empty() => {
                // Clearing a chord removes its slot (the bar is unsplit); an already empty single slot is left alone.
                let chords = &self.song().sections[cursor.section].bars[cursor.bar].chords;
                if chords.len() > 1 || chords[0].chord != "N" {
                    self.apply(remove_slot(self.song(), cursor));
                    self.cursor = SlotRef { slot: cursor.slot.saturating_sub(1), ..cursor };
                }
            }
            DraftKind::Chord => {
                let harte = match self.parse(text, settings) {
                    Ok(harte) => harte,
                    Err(error) => return flag(self, error),
                };
                let current = &self.song().sections[cursor.section].bars[cursor.bar].chords[cursor.slot];
                // Re-typing a flagged chord confirms it (set_chord drops the confidence), so only an unflagged no-op is skipped.
                if harte != current.chord || current.confidence.is_some() {
                    self.apply(set_chord(self.song(), cursor, &harte));
                }
                if !is_blur {
                    self.cursor = next_slot(self.song(), cursor, Step::Forward);
                }
            }
        }
        self.anchor = None;
        self.draft = None;
    }

    /// Mod+/ while typing: sets the chord, splits the slot and edits the new half, in one undo step.
    fn commit_and_split(&mut self, settings: &Settings) {
        let Some(draft) = self.draft.clone() else { return };
        let cursor = self.cursor();
        let harte = match self.parse(&draft.text, settings) {
            Ok(harte) => harte,
            Err(error) => {
                self.draft = Some(Draft { error: Some(error), ..draft });
                return;
            }
        };
        match set_chord(self.song(), cursor, &harte).and_then(|song| split_slot(&song, cursor)) {
            Ok(song) => {
                self.history.apply(song);
                self.cursor = SlotRef { slot: cursor.slot + 1, ..cursor };
                let text = self.show(&harte, settings);
                self.draft = Some(Draft { kind: DraftKind::Chord, text, select_all: true, error: None });
            }
            Err(_) => self.notice = "Not possible here".into(),
        }
    }

    /// Records an edit; a refused one only shows why.
    pub fn apply(&mut self, edit: Result<Song, String>) {
        match edit {
            Ok(song) => self.history.apply(song),
            Err(e) => self.notice = e,
        }
    }

    /// A click on a slot: an open input is saved first (it loses focus), then the slot is edited in edit mode.
    pub fn click(&mut self, at: SlotRef, settings: &Settings) {
        self.commit(settings, true);
        self.move_to(at);
        if self.is_editable {
            let chord = self.song().sections[at.section].bars[at.bar].chords[at.slot].chord.clone();
            let text = self.show(&chord, settings);
            self.draft = Some(Draft { kind: DraftKind::Chord, text, select_all: true, error: None });
        }
    }

    /// Runs a right-click menu item, and puts the cursor where its edit is.
    pub fn run(&mut self, action: MenuAction) {
        let song = self.song();
        let (edit, cursor) = match action {
            MenuAction::Split(at) => (split_slot(song, at), SlotRef { slot: at.slot + 1, ..at }),
            MenuAction::AddBarBefore(at) => (insert_bar(song, at, Position::Before), first_slot(at)),
            MenuAction::AddBarAfter(at) => (insert_bar(song, at, Position::After), first_slot(BarRef { bar: at.bar + 1, ..at })),
            MenuAction::RemoveBar(at) => (delete_bar(song, at), first_slot(at)),
            MenuAction::SplitSectionBefore(at) => (split_section(song, at), first_slot(BarRef { section: at.section + 1, bar: 0 })),
            MenuAction::SplitSectionAfter(at) => (split_section(song, BarRef { bar: at.bar + 1, ..at }), first_slot(at)),
            MenuAction::AddBarAt(at) => (insert_bar(song, at, Position::Before), first_slot(at)),
            MenuAction::SplitSectionAt(at) => (split_section(song, at), first_slot(BarRef { section: at.section + 1, bar: 0 })),
            MenuAction::MergeWithPrevious(index) => {
                let before = index.checked_sub(1).map_or(0, |previous| song.sections[previous].bars.len());
                (merge_with_previous(song, index), first_slot(BarRef { section: index.saturating_sub(1), bar: before }))
            }
        };
        self.draft = None;
        if edit.is_ok() {
            self.move_to(cursor);
        }
        self.apply(edit);
    }

    /// The Song panel's fields (F-ED-9 for the key): an empty artist, key or tempo removes it. The error is for the field.
    pub fn set_meta(&mut self, field: MetaField, text: &str, settings: &Settings) -> Result<(), String> {
        let text = text.trim();
        let mut song = self.song().clone();
        match field {
            MetaField::Title if text.is_empty() => return Err("a song needs a title".into()),
            MetaField::Title => song.meta.title = text.into(),
            MetaField::Artist => song.meta.artist = (!text.is_empty()).then(|| text.into()),
            MetaField::Key => {
                let is_key = |t: &str| {
                    let rest = t.strip_prefix(|c: char| ('A'..='G').contains(&c));
                    let rest = rest.map(|r| r.strip_prefix(['#', 'b']).unwrap_or(r));
                    matches!(rest, Some("" | "m"))
                };
                if !text.is_empty() && !is_key(text) {
                    return Err("a key is a note, optionally followed by m (e.g. F#m)".into());
                }
                song = set_key(&song, (!text.is_empty()).then_some(text), settings.minor_convention);
            }
            MetaField::Tempo if text.is_empty() => song.meta.tempo_bpm = None,
            MetaField::Tempo => match text.parse::<f64>() {
                Ok(bpm) if bpm > 0.0 && bpm.is_finite() => song.meta.tempo_bpm = Some(bpm),
                _ => return Err("the tempo must be a positive number".into()),
            },
        }
        self.history.apply(song);
        Ok(())
    }

    pub fn set_notation(&mut self, notation: Notation) {
        let edit = set_notation(self.song(), notation);
        self.apply(edit);
    }

    /// A key-less Nashville song keeps its chords relative to C: transposing would break that.
    pub fn is_transpose_locked(&self) -> bool {
        let meta = &self.song().meta;
        meta.notation == Some(Notation::Nashville) && meta.key.is_none()
    }

    /// Transposes by `semitones` (F-ED-9); the net offset since the song was opened goes with the snapshot, so undo and
    /// Reset see it.
    pub fn transpose(&mut self, semitones: i32, spelling: Spelling) {
        let Snapshot { song, transposition } = self.history.present();
        let next = Snapshot { song: transpose_song(song, semitones, spelling), transposition: transposition + semitones };
        self.history.apply_snapshot(next);
    }

    /// Edits the cursor's section, then puts the cursor at the start of section `to`.
    fn edit_section(&mut self, edit: Result<Song, String>, to: usize) {
        if edit.is_ok() {
            self.move_to(SlotRef { section: to, bar: 0, slot: 0 });
        }
        self.apply(edit);
    }

    pub fn rename_cursor_section(&mut self, label: &str) -> Result<(), String> {
        let label = label.trim();
        if label.is_empty() {
            return Err("a section needs a name".into());
        }
        let section = self.cursor().section;
        if label != self.song().sections[section].label {
            self.apply(rename_section(self.song(), section, label));
        }
        Ok(())
    }

    pub fn set_cursor_repeat(&mut self, n: u32) {
        let section = self.cursor().section;
        if n >= 1 && n != self.song().sections[section].repeat.unwrap_or(1) {
            self.apply(set_repeat(self.song(), section, n));
        }
    }

    pub fn move_cursor_section(&mut self, step: Step) {
        let from = self.cursor().section;
        let to = match step {
            Step::Back => from.checked_sub(1),
            Step::Forward => Some(from + 1),
        };
        match to {
            Some(to) => self.edit_section(move_section(self.song(), from, to), to),
            None => self.notice = "Not possible here".into(),
        }
    }

    pub fn add_section_after_cursor(&mut self) {
        let to = self.cursor().section + 1;
        self.edit_section(Ok(add_section(self.song(), to, "New section")), to);
    }

    /// Whether deleting the cursor's section loses chords, and so asks first.
    pub fn cursor_section_has_chords(&self) -> bool {
        self.song().sections[self.cursor().section].bars.iter().flat_map(|b| &b.chords).any(|c| c.chord != "N")
    }

    pub fn delete_cursor_section(&mut self) {
        let section = self.cursor().section;
        self.edit_section(delete_section(self.song(), section), section.saturating_sub(1));
    }

    /// The cursor bar's meter (F-ED-11): its own or the song's.
    pub fn cursor_bar_meter(&self) -> Option<Meter> {
        let cursor = self.cursor();
        let bar = self.song().sections[cursor.section].bars.get(cursor.bar)?;
        Some(bar.meter.clone().unwrap_or_else(|| self.song().meta.meter.clone()))
    }

    pub fn set_cursor_bar_beats(&mut self, beats: u32) {
        let Some(meter) = self.cursor_bar_meter() else { return };
        if beats >= 1 && beats != meter.beats {
            let edit = set_bar_meter(self.song(), self.cursor().bar_ref(), Some(Meter { beats, ..meter }));
            self.apply(edit);
        }
    }

    /// The bar lines move one beat (F-PB-3), from the cursor bar or for the whole song.
    pub fn shift(&mut self, step: Step, from_cursor: bool) {
        let from = if from_cursor { self.cursor().bar_ref() } else { WHOLE_SONG };
        let edit = shift_phase(self.song(), step, from);
        self.apply(edit);
    }

    /// Half or double tempo (F-PB-4), from the cursor bar or for the whole song.
    pub fn fix_tempo(&mut self, is_half: bool, from_cursor: bool) {
        let from = if from_cursor { self.cursor().bar_ref() } else { WHOLE_SONG };
        let edit = if is_half { halve_tempo(self.song(), from) } else { double_tempo(self.song(), from) };
        self.apply(edit);
    }

    /// Puts the cursor on a bar without editing it (a click on its time signature).
    pub fn select_bar(&mut self, at: SlotRef) {
        self.draft = None;
        self.move_to(at);
    }

    /// A click on a section label, or a pick in the Section panel.
    pub fn select_section(&mut self, index: usize) {
        self.move_to(SlotRef { section: index, bar: 0, slot: 0 });
        self.draft = None;
    }
}

fn first_slot(at: BarRef) -> SlotRef {
    SlotRef { section: at.section, bar: at.bar, slot: 0 }
}

#[cfg(test)]
mod tests;
