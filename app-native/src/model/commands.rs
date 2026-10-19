//! Editing commands (spec F-ED-3/4/5/6/8/11, F-PB-3/4): each takes a song and returns the edited copy, or why it can't.

use serde_json::Map;

use super::nashville::{MinorConvention, spelling_for, tonic_pc};
use super::song::{Bar, ChordSlot, Meter, Notation, Section, Song, new_section_id};
use super::transpose::transpose_song;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarRef {
    pub section: usize,
    pub bar: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotRef {
    pub section: usize,
    pub bar: usize,
    pub slot: usize,
}

impl SlotRef {
    pub fn bar_ref(self) -> BarRef {
        BarRef { section: self.section, bar: self.bar }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Before,
    After,
}

impl Position {
    fn index(self, bar: usize) -> usize {
        match self {
            Position::Before => bar,
            Position::After => bar + 1,
        }
    }
}

/// One beat (or one slot step) back or forward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Back = -1,
    Forward = 1,
}

type Edit = Result<Song, String>;

fn no(what: &str, index: usize) -> String {
    format!("no {what} at index {index}")
}

fn section_mut(song: &mut Song, index: usize) -> Result<&mut Section, String> {
    song.sections.get_mut(index).ok_or_else(|| no("section", index))
}

fn bar_mut(song: &mut Song, at: BarRef) -> Result<&mut Bar, String> {
    section_mut(song, at.section)?.bars.get_mut(at.bar).ok_or_else(|| no("bar", at.bar))
}

fn bar_at(song: &Song, at: BarRef) -> Result<&Bar, String> {
    song.sections.get(at.section).ok_or_else(|| no("section", at.section))?.bars.get(at.bar).ok_or_else(|| no("bar", at.bar))
}

/// The slots of the bar holding `at`, after checking the slot exists.
fn slots_mut(song: &mut Song, at: SlotRef) -> Result<&mut Vec<ChordSlot>, String> {
    let chords = &mut bar_mut(song, at.bar_ref())?.chords;
    if at.slot >= chords.len() {
        return Err(no("slot", at.slot));
    }
    Ok(chords)
}

fn slot(chord: &str, beats: u32) -> ChordSlot {
    ChordSlot { chord: chord.into(), beats, confidence: None, extra: Map::new() }
}

fn empty_bar(song: &Song) -> Bar {
    Bar { chords: vec![slot("N", song.meta.meter.beats)], start_sec: None, meter: None, extra: Map::new() }
}

fn new_section(song: &Song, label: &str) -> Section {
    Section { id: new_section_id(), label: label.into(), bars: vec![empty_bar(song)], repeat: None, extra: Map::new() }
}

/// In Nashville notation the numbers stay and the chords move: degree 1 goes from the old tonic (C without a key) to the new one.
pub fn set_key(song: &Song, key: Option<&str>, convention: MinorConvention) -> Song {
    let mut song = if song.meta.notation == Some(Notation::Nashville) {
        let to = tonic_pc(key, convention);
        transpose_song(song, to - tonic_pc(song.meta.key.as_deref(), convention), spelling_for(to))
    } else {
        song.clone()
    };
    song.meta.key = key.map(str::to_string);
    song
}

pub fn set_notation(song: &Song, notation: Notation) -> Edit {
    if song.meta.key.is_none() {
        return Err("a key is required to switch the notation".into());
    }
    let mut song = song.clone();
    song.meta.notation = Some(notation);
    Ok(song)
}

pub fn set_chord(song: &Song, at: SlotRef, harte: &str) -> Edit {
    let mut song = song.clone();
    let slot = &mut slots_mut(&mut song, at)?[at.slot];
    slot.chord = harte.into();
    slot.confidence = None;
    Ok(song)
}

pub fn split_slot(song: &Song, at: SlotRef) -> Edit {
    let mut song = song.clone();
    let slots = slots_mut(&mut song, at)?;
    let slot = &mut slots[at.slot];
    if slot.beats < 2 {
        return Err("cannot split a 1-beat slot".into());
    }
    let half = slot.beats / 2;
    slot.beats -= half;
    let second = ChordSlot { beats: half, ..slot.clone() };
    slots.insert(at.slot + 1, second);
    Ok(song)
}

pub fn merge_slot_with_next(song: &Song, at: SlotRef) -> Edit {
    let mut song = song.clone();
    let slots = slots_mut(&mut song, at)?;
    if at.slot + 1 >= slots.len() {
        return Err(no("next slot", at.slot + 1));
    }
    let next = slots.remove(at.slot + 1);
    slots[at.slot].beats += next.beats;
    Ok(song)
}

/// Removes a slot, its beats going to the previous slot (the next one for the first slot); the only slot of a bar becomes `N`.
pub fn remove_slot(song: &Song, at: SlotRef) -> Edit {
    let mut song = song.clone();
    let slots = slots_mut(&mut song, at)?;
    if slots.len() < 2 {
        slots[0].chord = "N".into();
        slots[0].confidence = None;
        return Ok(song);
    }
    let removed = slots.remove(at.slot);
    slots[at.slot.saturating_sub(1)].beats += removed.beats;
    Ok(song)
}

/// Moves one beat between the slot and its next neighbour (previous one for the last slot); a slot left with 0 beats is removed.
pub fn resize_slot(song: &Song, at: SlotRef, step: Step) -> Edit {
    let mut song = song.clone();
    let slots = slots_mut(&mut song, at)?;
    if slots.len() < 2 {
        return Err("cannot resize the only slot of a bar".into());
    }
    let other = if at.slot == slots.len() - 1 { at.slot - 1 } else { at.slot + 1 };
    match step {
        Step::Forward => {
            slots[at.slot].beats += 1;
            slots[other].beats -= 1;
        }
        Step::Back => {
            slots[at.slot].beats -= 1;
            slots[other].beats += 1;
        }
    }
    slots.retain(|s| s.beats > 0);
    Ok(song)
}

/// Sets the bar's meter (F-ED-11): slots are cut from the end, or the last slot grows. `None` or the song meter removes the override.
pub fn set_bar_meter(song: &Song, at: BarRef, meter: Option<Meter>) -> Edit {
    let song_meter = song.meta.meter.clone();
    let mut song = song.clone();
    let bar = bar_mut(&mut song, at)?;
    let target = meter.unwrap_or_else(|| song_meter.clone());
    if target.beats == 0 {
        return Err("a meter needs at least one beat".into());
    }
    // The file format only reads these units back.
    if ![2, 4, 8].contains(&target.unit) {
        return Err(format!("a meter unit must be 2, 4 or 8, got {}", target.unit));
    }
    let mut left = target.beats;
    bar.chords.retain_mut(|slot| {
        slot.beats = slot.beats.min(left);
        left -= slot.beats;
        slot.beats > 0
    });
    bar.chords.last_mut().expect("a bar keeps its first slot").beats += left;
    let is_song_meter = target.beats == song_meter.beats && target.unit == song_meter.unit;
    bar.meter = (!is_song_meter).then_some(target);
    Ok(song)
}

// Only the section is checked, so that bar 0 inserts into an empty section.
pub fn insert_bar(song: &Song, at: BarRef, position: Position) -> Edit {
    let bar = empty_bar(song);
    let mut song = song.clone();
    let bars = &mut section_mut(&mut song, at.section)?.bars;
    bars.insert(position.index(at.bar).min(bars.len()), bar);
    Ok(song)
}

pub fn delete_bar(song: &Song, at: BarRef) -> Edit {
    bar_at(song, at)?;
    let mut song = song.clone();
    song.sections[at.section].bars.remove(at.bar);
    Ok(song)
}

pub fn duplicate_bar(song: &Song, at: BarRef) -> Edit {
    let copy = Bar { start_sec: None, ..bar_at(song, at)?.clone() };
    let mut song = song.clone();
    song.sections[at.section].bars.insert(at.bar + 1, copy);
    Ok(song)
}

pub fn add_section(song: &Song, index: usize, label: &str) -> Song {
    let section = new_section(song, label);
    let mut song = song.clone();
    song.sections.insert(index.min(song.sections.len()), section);
    song
}

pub fn rename_section(song: &Song, index: usize, label: &str) -> Edit {
    let mut song = song.clone();
    section_mut(&mut song, index)?.label = label.into();
    Ok(song)
}

/// Moves the section so that it ends up at index `to`.
pub fn move_section(song: &Song, from: usize, to: usize) -> Edit {
    let count = song.sections.len();
    if from >= count {
        return Err(no("section", from));
    }
    if to >= count {
        return Err(no("section", to));
    }
    let mut song = song.clone();
    let section = song.sections.remove(from);
    song.sections.insert(to, section);
    Ok(song)
}

// A song always keeps at least one section.
pub fn delete_section(song: &Song, index: usize) -> Edit {
    let mut song = song.clone();
    section_mut(&mut song, index)?;
    song.sections.remove(index);
    if song.sections.is_empty() {
        song.sections.push(new_section(&song, "Song"));
    }
    Ok(song)
}

/// Bars from `at` onward go to a new section labelled `<label> (2)`, inserted right after; the repeat count stays on the first part.
pub fn split_section(song: &Song, at: BarRef) -> Edit {
    bar_at(song, at)?;
    let mut song = song.clone();
    let head = &mut song.sections[at.section];
    let tail = Section {
        id: new_section_id(),
        label: format!("{} (2)", head.label),
        bars: head.bars.split_off(at.bar),
        repeat: None,
        extra: head.extra.clone(),
    };
    song.sections.insert(at.section + 1, tail);
    Ok(song)
}

/// The inverse of `split_section`: the section's bars join the previous section, which keeps its label and repeat count.
pub fn merge_with_previous(song: &Song, index: usize) -> Edit {
    if index >= song.sections.len() {
        return Err(no("section", index));
    }
    let Some(previous) = index.checked_sub(1) else {
        return Err("no previous section before the first one".into());
    };
    let mut song = song.clone();
    let section = song.sections.remove(index);
    song.sections[previous].bars.extend(section.bars);
    Ok(song)
}

/// `n <= 1` removes the repeat count.
pub fn set_repeat(song: &Song, index: usize, n: u32) -> Edit {
    let mut song = song.clone();
    section_mut(&mut song, index)?.repeat = (n > 1).then_some(n);
    Ok(song)
}

/// Bars `from..=to` of one section.
pub fn copy_bars(song: &Song, from: BarRef, to: BarRef) -> Result<Vec<Bar>, String> {
    if from.section != to.section || to.bar < from.bar {
        return Err("copy range must be ordered and within one section".into());
    }
    bar_at(song, from)?;
    bar_at(song, to)?;
    Ok(song.sections[from.section].bars[from.bar..=to.bar].to_vec())
}

/// Pasted bars lose `start_sec`; a bar whose beats don't match the song meter gets a `meter` override.
pub fn paste_bars(song: &Song, at: BarRef, bars: &[Bar], position: Position) -> Edit {
    let meter = &song.meta.meter;
    let pasted = bars.iter().map(|copied| {
        let mut bar = Bar { start_sec: None, ..copied.clone() };
        // A pasted chord is the user's decision, like a typed one: it loses its low-confidence flag.
        bar.chords.iter_mut().for_each(|slot| slot.confidence = None);
        let beats: u32 = bar.chords.iter().map(|slot| slot.beats).sum();
        if bar.meter.is_none() && beats != meter.beats {
            bar.meter = Some(Meter { beats, unit: meter.unit, extra: Map::new() });
        }
        bar
    });
    let pasted: Vec<Bar> = pasted.collect();
    let mut song = song.clone();
    let existing = &mut section_mut(&mut song, at.section)?.bars;
    let index = position.index(at.bar).min(existing.len());
    existing.splice(index..index, pasted);
    Ok(song)
}

#[derive(Clone)]
struct Beat<'a> {
    slot: &'a ChordSlot,
    sec: Option<f64>,
    section: usize,
}

struct TimedBar<'a> {
    section: usize,
    bar: usize,
    beats: Vec<Beat<'a>>,
    period: Option<f64>,
}

/// The bars from `from` to the end of the song, each with its beats. Beat times are interpolated between bar starts; when
/// the next bar has no time (or for the last bar) the last timed period carries on. A bar without `start_sec` has untimed beats.
fn bars_from(song: &Song, from: BarRef) -> Result<Vec<TimedBar<'_>>, String> {
    bar_at(song, from)?;
    let all: Vec<(usize, usize, &Bar)> = song
        .sections
        .iter()
        .enumerate()
        .flat_map(|(s, section)| section.bars.iter().enumerate().map(move |(b, bar)| (s, b, bar)))
        .collect();
    let mut period = None;
    let mut timed = Vec::with_capacity(all.len());
    for (i, &(s, b, bar)) in all.iter().enumerate() {
        let next = all.get(i + 1).and_then(|(_, _, next)| next.start_sec);
        if let (Some(start), Some(next)) = (bar.start_sec, next) {
            period = Some((next - start) / f64::from(bar.meter.as_ref().unwrap_or(&song.meta.meter).beats));
        }
        let beats = bar
            .chords
            .iter()
            .flat_map(|slot| std::iter::repeat_n(slot, slot.beats as usize))
            .enumerate()
            .map(|(j, slot)| Beat {
                slot,
                section: s,
                sec: match (bar.start_sec, period) {
                    (Some(start), _) if j == 0 => Some(start),
                    (Some(start), Some(period)) => Some(start + j as f64 * period),
                    _ => None,
                },
            })
            .collect();
        timed.push(TimedBar { section: s, bar: b, beats, period });
    }
    let start = timed.iter().position(|t| t.section == from.section && t.bar == from.bar).expect("bar_at checked it");
    Ok(timed.split_off(start))
}

/// Consecutive beats of the same chord make one slot; a merged slot keeps the lowest confidence (the most doubtful).
fn to_slots(beats: &[Beat]) -> Vec<ChordSlot> {
    let mut slots: Vec<ChordSlot> = Vec::new();
    for beat in beats {
        match slots.last_mut() {
            Some(last) if last.chord == beat.slot.chord => {
                last.beats += 1;
                last.confidence = match (last.confidence, beat.slot.confidence) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (a, b) => a.or(b),
                };
            }
            _ => slots.push(ChordSlot { beats: 1, ..beat.slot.clone() }),
        }
    }
    slots
}

/// A bar made of `beats`, with a meter override when it isn't as long as the song meter.
fn to_bar(song: &Song, beats: &[Beat]) -> Bar {
    let meter = &song.meta.meter;
    let length = beats.len() as u32;
    Bar {
        chords: to_slots(beats),
        start_sec: beats[0].sec,
        meter: (length != meter.beats).then(|| Meter { beats: length, unit: meter.unit, extra: Map::new() }),
        extra: Map::new(),
    }
}

/// Replaces the bars from `from` to the end of the song with `bars`, each going to its section.
fn replace_from(song: &Song, from: BarRef, bars: Vec<(usize, Bar)>) -> Song {
    let mut song = song.clone();
    song.sections[from.section].bars.truncate(from.bar);
    for section in &mut song.sections[from.section + 1..] {
        section.bars.clear();
    }
    for (s, bar) in bars {
        song.sections[s].bars.push(bar);
    }
    song
}

/// `from` for the whole song.
pub const WHOLE_SONG: BarRef = BarRef { section: 0, bar: 0 };

/// Moves every bar line from bar `from` (`WHOLE_SONG` for all of it) to the end of the song by one beat (spec
/// F-PB-3): Forward = the downbeats come one beat later. Beats are slot beats (eighths in 6/8). Every bar line in the
/// range moves, so inner bars keep their length (a 2/4 break stays 2/4). A short first bar is a pickup, not a bar line,
/// and what follows the last bar line is re-cut into bars of the song meter: a short first and last bar get a meter
/// override. So shifting twice from the bar after a missed break makes it a 2-beat bar. Beat times are interpolated
/// between bar starts (the last timed period carries on); a bar starting on an untimed beat has no `start_sec`. Each
/// section starts at the bar line its first bar line moved to.
pub fn shift_phase(song: &Song, step: Step, from: BarRef) -> Edit {
    let delta = step as i64;
    let size = i64::from(song.meta.meter.beats);
    let range = bars_from(song, from)?;
    let beats: Vec<Beat> = range.iter().flat_map(|t| t.beats.iter().cloned()).collect();
    let count = beats.len() as i64;
    // A short first bar is a pickup: its start is the range start, not a bar line.
    let mut starts = vec![0];
    let mut at = 0;
    for (i, t) in range.iter().enumerate() {
        let line = at + delta;
        if (i > 0 || t.beats.len() as i64 >= size) && line > 0 && line < count {
            starts.push(line);
        }
        at += t.beats.len() as i64;
    }
    let mut q = starts.last().expect("starts has 0") + size;
    while q < count {
        starts.push(q);
        q += size;
    }
    let bars = starts
        .iter()
        .enumerate()
        .map(|(i, &q)| {
            let end = starts.get(i + 1).copied().unwrap_or(count);
            let section = beats[(q - delta).clamp(0, count - 1) as usize].section;
            (section, to_bar(song, &beats[q as usize..end as usize]))
        })
        .collect();
    Ok(replace_from(song, from, bars))
}

// The song tempo only changes when the whole song does.
fn scale_tempo(mut song: Song, from: BarRef, factor: f64) -> Song {
    if from == WHOLE_SONG {
        song.meta.tempo_bpm = song.meta.tempo_bpm.map(|bpm| bpm * factor);
    }
    song
}

/// Fixes a double tempo (spec F-PB-4): from bar `from` on, bars are merged two by two within each section and every two
/// beats make one, taking the chord and time of the first. So a chord change on an odd beat moves to the next beat, and
/// a 1-beat chord there is dropped. A section's odd last bar becomes a half bar (meter override).
pub fn halve_tempo(song: &Song, from: BarRef) -> Edit {
    let range = bars_from(song, from)?;
    let mut bars = Vec::new();
    for (i, t) in range.iter().enumerate() {
        let first = if t.section == from.section { from.bar } else { 0 };
        if (t.bar - first) % 2 == 1 {
            continue;
        }
        let mut pair = t.beats.clone();
        if let Some(next) = range.get(i + 1).filter(|next| next.section == t.section) {
            pair.extend(next.beats.iter().cloned());
        }
        let halved: Vec<Beat> = pair.into_iter().step_by(2).collect();
        bars.push((t.section, to_bar(song, &halved)));
    }
    Ok(scale_tempo(replace_from(song, from, bars), from, 0.5))
}

/// Fixes a half tempo (spec F-PB-4): from bar `from` on, each bar is split in two bars of its meter, every beat becoming two.
pub fn double_tempo(song: &Song, from: BarRef) -> Edit {
    let mut bars = Vec::new();
    for t in bars_from(song, from)? {
        let halves: Vec<Beat> = t
            .beats
            .iter()
            .flat_map(|beat| {
                let sec = beat.sec.zip(t.period).map(|(sec, period)| sec + period / 2.0);
                [beat.clone(), Beat { sec, ..beat.clone() }]
            })
            .collect();
        let (first, second) = halves.split_at(t.beats.len());
        bars.push((t.section, to_bar(song, first)));
        bars.push((t.section, to_bar(song, second)));
    }
    Ok(scale_tempo(replace_from(song, from, bars), from, 2.0))
}

#[cfg(test)]
mod tests;
