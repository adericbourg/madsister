//! Key → command mapping for the grid (spec F-ED-3..7), pure so it's testable without a window.

use crate::model::commands::{
    BarRef, Position, SlotRef, Step, add_section, copy_bars, delete_bar, delete_section, double_tempo, duplicate_bar,
    halve_tempo, insert_bar, merge_slot_with_next, move_section, paste_bars, resize_slot, set_bar_meter, set_repeat,
    shift_phase, split_section, split_slot,
};
use crate::model::song::{Bar, Meter, Song};

/// A key press; `key` is named as in the DOM ("ArrowLeft", "Enter", "a").
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyInput<'a> {
    pub key: &'a str,
    pub ctrl: bool,
    pub meta: bool,
    pub shift: bool,
    pub alt: bool,
}

pub struct EditorState<'a> {
    pub song: &'a Song,
    pub cursor: SlotRef,
    pub anchor: Option<BarRef>,
    pub clipboard: &'a [Bar],
    pub bars_per_row: usize,
    pub low_confidence_threshold: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Move { cursor: SlotRef, anchor: Option<BarRef> },
    Edit { song: Box<Song>, cursor: SlotRef },
    Copy(Vec<Bar>),
    Type(String),
    Rename,
    Undo,
    Redo,
    Help,
    Play,
    PlayFromCursor,
    /// The key maps to an edit that isn't possible here.
    Refused,
}

pub const SHORTCUTS: &[(&str, &str)] = &[
    ("← → ↑ ↓", "Move between slots and rows"),
    ("Home / End", "First / last slot of the section"),
    ("Type a chord, Enter or Tab", "Set the chord and go to the next slot (Escape cancels)"),
    ("F8 / Shift+F8", "Next / previous low-confidence chord"),
    ("/", "Split the slot"),
    ("Mod+/", "While typing a chord: set it and split the slot"),
    ("Backspace", "Merge the slot with the previous one"),
    ("Alt+← / Alt+→", "Shrink / grow the slot by one beat"),
    ("Mod+Enter / Mod+Shift+Enter", "Insert a bar after / before"),
    ("Mod+Backspace", "Delete the bar"),
    ("Mod+D", "Duplicate the bar"),
    ("Mod+B", "Make the bar a short break bar (2/4, or 3/8 in 6/8) / back to the song meter"),
    ("< / >", "Bars start one beat earlier / later, from the cursor bar on (press twice on a missed 2-beat bar to make it 2/4)"),
    ("- / +", "Half tempo (merge bars two by two) / double tempo (split each bar), from the cursor bar on"),
    ("Shift+← / Shift+→", "Select bars"),
    ("Mod+C / Mod+V", "Copy the selected bars / paste after the cursor bar"),
    ("Mod+Z / Mod+Shift+Z", "Undo / redo"),
    ("Mod+K", "Split the section at the cursor bar"),
    ("Mod+Shift+K", "Add a section after this one"),
    ("Mod+Shift+Backspace", "Delete the section"),
    ("F2", "Rename the section"),
    ("Alt+↑ / Alt+↓", "Repeat the section one more / one less time"),
    ("Alt+Shift+↑ / Alt+Shift+↓", "Move the section up / down"),
    ("Space", "Play / pause the source audio"),
    ("Shift+Space", "Play from the cursor bar"),
    ("Mod+E", "Switch between view (read-only) and edit mode"),
    ("Mod+N / Mod+O", "New song / open a file"),
    ("Mod+S / Mod+Shift+S", "Save / save as"),
    ("Mod+P", "Export a PDF to print"),
    ("?", "Show this help"),
];

pub fn clamp_cursor(song: &Song, cursor: SlotRef) -> SlotRef {
    let section = cursor.section.min(song.sections.len() - 1);
    let bars = &song.sections[section].bars;
    if bars.is_empty() {
        return SlotRef { section, bar: 0, slot: 0 };
    }
    let bar = cursor.bar.min(bars.len() - 1);
    SlotRef { section, bar, slot: cursor.slot.min(bars[bar].chords.len() - 1) }
}

// Every cursor position in reading order; an empty section has one, on its "No bars yet" placeholder.
fn positions(song: &Song) -> Vec<SlotRef> {
    let mut all = Vec::new();
    for (s, section) in song.sections.iter().enumerate() {
        if section.bars.is_empty() {
            all.push(SlotRef { section: s, bar: 0, slot: 0 });
        }
        for (b, bar) in section.bars.iter().enumerate() {
            all.extend((0..bar.chords.len()).map(|slot| SlotRef { section: s, bar: b, slot }));
        }
    }
    all
}

fn is_flagged(song: &Song, at: SlotRef, threshold: f64) -> bool {
    song.sections[at.section].bars.get(at.bar).and_then(|bar| bar.chords[at.slot].confidence).is_some_and(|c| c < threshold)
}

/// Whether any chord comes with an engine confidence, i.e. the song went through chord recognition.
pub fn has_confidence(song: &Song) -> bool {
    song.sections.iter().flat_map(|s| &s.bars).flat_map(|b| &b.chords).any(|c| c.confidence.is_some())
}

/// Number of slots whose engine confidence is below the threshold (spec F-ED-8).
pub fn count_flagged(song: &Song, threshold: f64) -> usize {
    positions(song).into_iter().filter(|&p| is_flagged(song, p, threshold)).count()
}

/// The next (or previous) flagged slot after the cursor, wrapping around the song.
fn next_flagged(song: &Song, cursor: SlotRef, threshold: f64, step: Step) -> Option<SlotRef> {
    let mut all = positions(song);
    if step == Step::Back {
        all.reverse();
    }
    let i = all.iter().position(|&p| p == cursor).map_or(0, |i| i + 1);
    all[i..].iter().chain(&all[..i]).copied().find(|&p| is_flagged(song, p, threshold))
}

/// Like `next_flagged`, but skips the rest of the cursor's bar: a bar with several flagged slots counts once.
pub fn next_flagged_bar(song: &Song, cursor: SlotRef, threshold: f64, step: Step) -> Option<SlotRef> {
    let last_slot = song.sections[cursor.section].bars.get(cursor.bar).map_or(1, |b| b.chords.len()) - 1;
    let from = SlotRef { slot: if step == Step::Forward { last_slot } else { 0 }, ..cursor };
    next_flagged(song, from, threshold, step)
}

pub fn next_slot(song: &Song, cursor: SlotRef, step: Step) -> SlotRef {
    let all = positions(song);
    let Some(i) = all.iter().position(|&p| p == cursor) else { return cursor };
    let next = match step {
        Step::Forward => all.get(i + 1),
        Step::Back => i.checked_sub(1).map(|j| &all[j]),
    };
    next.copied().unwrap_or(cursor)
}

fn next_row(song: &Song, cursor: SlotRef, bars_per_row: usize, step: Step) -> SlotRef {
    let rows: Vec<(usize, usize)> = song
        .sections
        .iter()
        .enumerate()
        .flat_map(|(s, section)| (0..section.bars.len().div_ceil(bars_per_row).max(1)).map(move |r| (s, r * bars_per_row)))
        .collect();
    let in_row = |&(s, start): &(usize, usize)| s == cursor.section && cursor.bar >= start && cursor.bar < start + bars_per_row;
    let Some(i) = rows.iter().position(in_row) else { return cursor };
    let target = match step {
        Step::Forward => rows.get(i + 1),
        Step::Back => i.checked_sub(1).map(|j| &rows[j]),
    };
    match target {
        Some(&(section, start)) => clamp_cursor(song, SlotRef { section, bar: start + cursor.bar - rows[i].1, ..cursor }),
        None => cursor,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub section: usize,
    pub from: usize,
    pub to: usize,
}

/// The selected bar range: anchor..cursor, or just the cursor bar.
pub fn selected_bars(cursor: SlotRef, anchor: Option<BarRef>) -> Selection {
    let from = anchor.map_or(cursor.bar, |a| a.bar);
    Selection { section: cursor.section, from: from.min(cursor.bar), to: from.max(cursor.bar) }
}

fn command_for(e: KeyInput, state: &EditorState) -> Result<Option<Command>, String> {
    let song = state.song;
    let cursor = state.cursor;
    let bar = cursor.bar_ref();
    let section = cursor.section;
    let moved = |to: SlotRef| Ok(Some(Command::Move { cursor: to, anchor: None }));
    let edit = |next: Song, to: SlotRef| {
        let cursor = clamp_cursor(&next, to);
        Ok(Some(Command::Edit { song: Box::new(next), cursor }))
    };
    let is_up = e.key == "ArrowUp";

    // Either modifier is accepted as Mod; the help shows the platform's one.
    if e.ctrl || e.meta {
        return match e.key.to_lowercase().as_str() {
            "z" => Ok(Some(if e.shift { Command::Redo } else { Command::Undo })),
            "enter" if e.shift => edit(insert_bar(song, bar, Position::Before)?, SlotRef { slot: 0, ..cursor }),
            "enter" => edit(insert_bar(song, bar, Position::After)?, SlotRef { bar: bar.bar + 1, slot: 0, ..cursor }),
            "backspace" if e.shift => edit(delete_section(song, section)?, cursor),
            "backspace" => edit(delete_bar(song, bar)?, cursor),
            "d" => edit(duplicate_bar(song, bar)?, cursor),
            "b" => {
                let unit = song.meta.meter.unit;
                let is_overridden = song.sections[section].bars.get(bar.bar).is_some_and(|b| b.meter.is_some());
                let meter = (!is_overridden).then(|| Meter { beats: if unit == 8 { 3 } else { 2 }, unit, extra: Default::default() });
                edit(set_bar_meter(song, bar, meter)?, cursor)
            }
            "k" => {
                let next = if e.shift { Ok(add_section(song, section + 1, "New section")) } else { split_section(song, bar) };
                edit(next?, SlotRef { section: section + 1, bar: 0, slot: 0 })
            }
            "c" => {
                let range = selected_bars(cursor, state.anchor);
                let from = BarRef { section, bar: range.from };
                Ok(Some(Command::Copy(copy_bars(song, from, BarRef { bar: range.to, ..from })?)))
            }
            "v" if !state.clipboard.is_empty() => edit(paste_bars(song, bar, state.clipboard, Position::After)?, cursor),
            _ => Ok(None),
        };
    }
    if e.alt {
        return match e.key {
            "ArrowLeft" | "ArrowRight" => {
                let step = if e.key == "ArrowRight" { Step::Forward } else { Step::Back };
                edit(resize_slot(song, cursor, step)?, cursor)
            }
            "ArrowUp" | "ArrowDown" if !e.shift => {
                let repeat = song.sections[section].repeat.unwrap_or(1);
                edit(set_repeat(song, section, if is_up { repeat + 1 } else { repeat.saturating_sub(1) })?, cursor)
            }
            "ArrowUp" | "ArrowDown" => {
                let to = if is_up { section.checked_sub(1) } else { Some(section + 1) };
                let to = to.ok_or("no section before the first one")?;
                edit(move_section(song, section, to)?, SlotRef { section: to, ..cursor })
            }
            _ => Ok(None),
        };
    }
    if e.shift && (e.key == "ArrowLeft" || e.key == "ArrowRight") {
        let b = if e.key == "ArrowRight" { Some(cursor.bar + 1) } else { cursor.bar.checked_sub(1) };
        return Ok(b.filter(|&b| b < song.sections[section].bars.len()).map(|b| Command::Move {
            cursor: SlotRef { section, bar: b, slot: 0 },
            anchor: Some(state.anchor.unwrap_or(bar)),
        }));
    }
    let step = |forward: bool| if forward { Step::Forward } else { Step::Back };
    match e.key {
        "ArrowLeft" | "ArrowRight" => return moved(next_slot(song, cursor, step(e.key == "ArrowRight"))),
        "ArrowUp" | "ArrowDown" => return moved(next_row(song, cursor, state.bars_per_row, step(!is_up))),
        "Home" | "End" => {
            let in_section: Vec<SlotRef> = positions(song).into_iter().filter(|p| p.section == section).collect();
            return moved(if e.key == "Home" { in_section[0] } else { in_section[in_section.len() - 1] });
        }
        "/" => return edit(split_slot(song, cursor)?, cursor),
        "<" | ">" => return edit(shift_phase(song, step(e.key == ">"), bar)?, SlotRef { slot: 0, ..cursor }),
        "-" => return edit(halve_tempo(song, bar)?, SlotRef { slot: 0, ..cursor }),
        "+" => return edit(double_tempo(song, bar)?, SlotRef { slot: 0, ..cursor }),
        "Backspace" => {
            let previous = SlotRef { slot: cursor.slot.checked_sub(1).ok_or("no slot before the first one")?, ..cursor };
            return edit(merge_slot_with_next(song, previous)?, previous);
        }
        "?" => return Ok(Some(Command::Help)),
        " " => return Ok(Some(if e.shift { Command::PlayFromCursor } else { Command::Play })),
        "F2" => return Ok(Some(Command::Rename)),
        "F8" => {
            let target = next_flagged(song, cursor, state.low_confidence_threshold, step(!e.shift));
            return moved(target.ok_or("no low-confidence chord")?);
        }
        _ => {}
    }
    let mut chars = e.key.chars();
    // Slint sends keys like F5 or Insert as private-use characters: not something to type.
    let is_private_use = |c: char| ('\u{E000}'..='\u{F8FF}').contains(&c);
    let is_printable =
        matches!((chars.next(), chars.next()), (Some(c), None) if !c.is_whitespace() && !c.is_control() && !is_private_use(c));
    Ok((is_printable && !song.sections[section].bars.is_empty()).then(|| Command::Type(e.key.into())))
}

/// Commands fail on impossible edits (e.g. merging the first slot): the key is then refused.
pub fn key_to_command(e: KeyInput, state: &EditorState) -> Option<Command> {
    command_for(e, state).unwrap_or(Some(Command::Refused))
}

#[cfg(test)]
mod tests;
