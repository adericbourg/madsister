//! Verification playback (F-PB-1).

use super::commands::BarRef;
use super::song::Song;

/// The bar playing at `t` seconds: the one with the latest `start_sec` at or before `t`; bars without `start_sec` (added
/// by hand) are skipped, None before the first timed bar. A full scan: moved sections leave `start_sec` out of order.
pub fn bar_at_time(song: &Song, t: f64) -> Option<BarRef> {
    let mut found = None;
    let mut found_sec = f64::NEG_INFINITY;
    for (s, section) in song.sections.iter().enumerate() {
        for (b, bar) in section.bars.iter().enumerate() {
            if let Some(start) = bar.start_sec.filter(|&start| start <= t && start > found_sec) {
                found = Some(BarRef { section: s, bar: b });
                found_sec = start;
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::song::parse_song;

    #[test]
    fn bar_at_time_returns_the_bar_with_the_latest_start_at_or_before_the_time() {
        // Given two sections moved out of time order, and a manually added bar without startSec
        let bar = |start: &str| format!(r#"{{ {start} "chords": [{{ "chord": "C:maj", "beats": 4 }}] }}"#);
        let song = parse_song(&format!(
            r#"{{ "version": 1, "meta": {{ "title": "t", "meter": {{ "beats": 4, "unit": 4 }} }}, "sections": [
                {{ "id": "b", "label": "Chorus", "bars": [{}, {}] }},
                {{ "id": "a", "label": "Verse", "bars": [{}, {}, {}] }}
            ] }}"#,
            bar(r#""startSec": 10,"#),
            bar(r#""startSec": 12,"#),
            bar(r#""startSec": 2,"#),
            bar(""),
            bar(r#""startSec": 4,"#),
        ))
        .unwrap();
        let at = |section, bar| Some(BarRef { section, bar });

        // Then nothing before the first bar
        assert_eq!(bar_at_time(&song, 1.9), None);
        // And a bar from its start until the next timed bar, skipping the untimed one
        assert_eq!(bar_at_time(&song, 2.0), at(1, 0));
        assert_eq!(bar_at_time(&song, 3.99), at(1, 0));
        assert_eq!(bar_at_time(&song, 4.0), at(1, 2));
        assert_eq!(bar_at_time(&song, 11.0), at(0, 0));
        // And the last bar after the end
        assert_eq!(bar_at_time(&song, 99.0), at(0, 1));
        // And nothing in a song without timing
        let mut untimed = song.clone();
        untimed.sections.iter_mut().flat_map(|s| &mut s.bars).for_each(|b| b.start_sec = None);
        assert_eq!(bar_at_time(&untimed, 5.0), None);
    }
}
