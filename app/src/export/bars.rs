use std::borrow::Cow;

use crate::model::song::{Bar, Song, bar_beats};

pub struct PlayedBar<'a> {
    pub bar: Cow<'a, Bar>,
    pub label: Option<&'a str>,
}

/// The bars as played, for score-like exports: repeated sections written out (each pass keeps its section label on its
/// first bar), and a lone "%" replaced by the previous bar's chords when both bars have the same beat count.
pub fn played_bars(song: &Song) -> Vec<PlayedBar<'_>> {
    let mut played: Vec<PlayedBar> = Vec::new();
    for section in &song.sections {
        for _ in 0..section.repeat.unwrap_or(1) {
            for (i, bar) in section.bars.iter().enumerate() {
                // shortcut: a "%" slot inside a split bar is left as is (no harmony, silence in MIDI).
                let is_repeat_bar = bar.chords.len() == 1 && bar.chords[0].chord == "%";
                let bar = match played.last() {
                    Some(previous) if is_repeat_bar && bar_beats(song, &previous.bar) == bar_beats(song, bar) => {
                        Cow::Owned(Bar { chords: previous.bar.chords.clone(), ..bar.clone() })
                    }
                    _ => Cow::Borrowed(bar),
                };
                played.push(PlayedBar { bar, label: (i == 0).then_some(section.label.as_str()) });
            }
        }
    }
    played
}
