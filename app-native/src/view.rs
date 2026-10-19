//! The song as the Slint sheet shows it.

use std::rc::Rc;

use slint::{ModelRc, SharedString, VecModel};

use crate::{BarView, ChartView, RowView, SectionView, SlotView};
use madsister::export::sheet::{chord_text, nashville_tonic};
use madsister::model::display::{display_key, speak_chord};
use madsister::model::nashville::speak_nashville;
use madsister::model::song::{Bar, Song, whole};
use madsister::settings::Settings;

fn model<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    ModelRc::from(Rc::new(VecModel::from(items)))
}

fn bar_view(settings: &Settings, tonic: Option<i32>, section_label: &str, index: usize, bar: &Bar) -> BarView {
    let meter = bar.meter.as_ref().map(|m| format!("{}/{}", m.beats, m.unit)).unwrap_or_default();
    let mut beat = 1;
    let slots = bar
        .chords
        .iter()
        .map(|slot| {
            let low_confidence = slot.confidence.is_some_and(|c| c < settings.low_confidence_threshold);
            let chord = match tonic {
                Some(tonic) => speak_nashville(&slot.chord, tonic),
                None => speak_chord(&slot.chord),
            };
            let in_meter = if meter.is_empty() { String::new() } else { format!(" in {meter}") };
            let doubt = if low_confidence { ", low confidence" } else { "" };
            let spoken = format!("{section_label}, bar {}{in_meter}, beat {beat}: {chord}{doubt}", index + 1);
            beat += slot.beats;
            SlotView {
                text: chord_text(&slot.chord, tonic, settings).into(),
                spoken: spoken.into(),
                beats: slot.beats as i32,
                low_confidence,
            }
        })
        .collect();
    BarView { meter: meter.into(), slots: model(slots) }
}

pub fn chart_view(song: &Song, settings: &Settings) -> ChartView {
    let meta = &song.meta;
    let tonic = nashville_tonic(song, settings);
    let sections = song
        .sections
        .iter()
        .map(|section| {
            let bars: Vec<BarView> =
                section.bars.iter().enumerate().map(|(i, bar)| bar_view(settings, tonic, &section.label, i, bar)).collect();
            let rows = bars.chunks(settings.bars_per_row).map(|row| RowView { bars: model(row.to_vec()) }).collect();
            SectionView {
                label: section.label.as_str().into(),
                bar_count: section.bars.len() as i32,
                repeat: section.repeat.unwrap_or(1) as i32,
                rows: model(rows),
            }
        })
        .collect();
    let tempo = meta.tempo_bpm.map(|t| format!("♩ = {}", whole(t).map_or(t.to_string(), |w| w.to_string())));
    ChartView {
        title: meta.title.as_str().into(),
        artist: meta.artist.clone().unwrap_or_default().into(),
        key: meta.key.as_deref().map(|k| display_key(k, settings.style)).unwrap_or_default().into(),
        tempo: SharedString::from(tempo.unwrap_or_default()),
        bars_per_row: settings.bars_per_row as i32,
        sections: model(sections),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use madsister::model::display::DisplayStyle;
    use madsister::model::song::parse_song;
    use slint::Model;

    #[test]
    fn chart_view_cuts_sections_into_rows_and_spells_each_slot() {
        // Given a 5-bar section, 4 bars per row, a low-confidence split bar and a 2/4 bar
        let song = parse_song(
            r#"{ "version": 1, "meta": { "title": "T", "key": "Am", "tempoBpm": 120, "meter": { "beats": 4, "unit": 4 } }, "sections": [
                { "id": "a", "label": "Verse", "repeat": 2, "bars": [
                    { "chords": [{ "chord": "A:min", "beats": 2, "confidence": 0.3 }, { "chord": "G:maj/3", "beats": 2 }] },
                    { "chords": [{ "chord": "C:maj", "beats": 4 }] },
                    { "chords": [{ "chord": "C:maj", "beats": 4 }] },
                    { "chords": [{ "chord": "C:maj", "beats": 4 }] },
                    { "meter": { "beats": 2, "unit": 4 }, "chords": [{ "chord": "N", "beats": 2 }] }
                ] }
            ] }"#,
        )
        .unwrap();
        let settings = Settings { style: DisplayStyle::Intl, ..Settings::parse(&serde_json::Value::Null) };

        // When
        let chart = chart_view(&song, &settings);

        // Then
        assert_eq!((chart.key.as_str(), chart.tempo.as_str()), ("Am", "♩ = 120"));
        let section = chart.sections.row_data(0).unwrap();
        assert_eq!((section.label.as_str(), section.repeat), ("Verse", 2));
        let rows: Vec<usize> = section.rows.iter().map(|r| r.bars.row_count()).collect();
        assert_eq!(rows, [4, 1]);
        let first = section.rows.row_data(0).unwrap().bars.row_data(0).unwrap();
        let slots: Vec<SlotView> = first.slots.iter().collect();
        assert_eq!((slots[0].text.as_str(), slots[0].low_confidence, slots[0].beats), ("Am", true, 2));
        assert_eq!(slots[0].spoken.as_str(), "Verse, bar 1, beat 1: A minor, low confidence");
        assert_eq!(slots[1].spoken.as_str(), "Verse, bar 1, beat 3: G over B");
        let last = section.rows.row_data(1).unwrap().bars.row_data(0).unwrap();
        assert_eq!(last.meter.as_str(), "2/4");
        assert_eq!(last.slots.row_data(0).unwrap().spoken.as_str(), "Verse, bar 5 in 2/4, beat 1: no chord");
    }
}
