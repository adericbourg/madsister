//! User preferences (spec F-ED-1, F-DS-1, F-UI-3), stored in `<config dir>/dev.dericbourg.madsister/settings.json`, the
//! file the Tauri app wrote: not in the song.

use std::path::PathBuf;

use serde_json::Value;

use crate::model::display::DisplayStyle;
use crate::model::nashville::MinorConvention;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartFont {
    Kalam,
    PatrickHand,
    PetalumaScript,
    PetalumaText,
    Serif,
}

impl ChartFont {
    pub const ALL: [ChartFont; 5] =
        [ChartFont::Kalam, ChartFont::PatrickHand, ChartFont::PetalumaScript, ChartFont::PetalumaText, ChartFont::Serif];

    /// The value in settings.json and `madsister print --font`.
    pub fn id(self) -> &'static str {
        match self {
            ChartFont::Kalam => "kalam",
            ChartFont::PatrickHand => "patrick-hand",
            ChartFont::PetalumaScript => "petaluma-script",
            ChartFont::PetalumaText => "petaluma-text",
            ChartFont::Serif => "serif",
        }
    }

    /// The family name the UI draws it with.
    pub fn family(self) -> &'static str {
        match self {
            ChartFont::Kalam => "Kalam",
            ChartFont::PatrickHand => "Patrick Hand",
            ChartFont::PetalumaScript => "Petaluma Script",
            ChartFont::PetalumaText => "Petaluma Text",
            ChartFont::Serif => "DejaVu Serif",
        }
    }

    /// Whether it has its own bold face; the UI thickens the others, as browsers do.
    pub fn has_bold(self) -> bool {
        matches!(self, ChartFont::Kalam | ChartFont::Serif)
    }

    pub fn from_id(id: &str) -> Option<ChartFont> {
        ChartFont::ALL.into_iter().find(|font| font.id() == id)
    }
}

pub fn style_from_id(id: &str) -> Option<DisplayStyle> {
    match id {
        "fr" => Some(DisplayStyle::Fr),
        "intl" => Some(DisplayStyle::Intl),
        "latin" => Some(DisplayStyle::Latin),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub style: DisplayStyle,
    pub bars_per_row: usize,
    pub low_confidence_threshold: f64,
    pub compact_print: bool,
    pub font: ChartFont,
    pub minor_convention: MinorConvention,
    pub is_sidebar_collapsed: bool,
    pub reopen_tabs: bool,
}

pub const BARS_PER_ROW: [usize; 3] = [2, 4, 8];

impl Settings {
    /// Anything missing or invalid falls back to the default.
    pub fn parse(json: &Value) -> Settings {
        let str_of = |key| json.get(key).and_then(Value::as_str).unwrap_or_default();
        let bool_of = |key| json.get(key).and_then(Value::as_bool);
        Settings {
            style: style_from_id(str_of("style")).unwrap_or(DisplayStyle::Fr),
            bars_per_row: json
                .get("barsPerRow")
                .and_then(Value::as_u64)
                .map(|n| n as usize)
                .filter(|n| BARS_PER_ROW.contains(n))
                .unwrap_or(4),
            low_confidence_threshold: json
                .get("lowConfidenceThreshold")
                .and_then(Value::as_f64)
                .filter(|t| (0.0..=1.0).contains(t))
                .unwrap_or(0.5),
            compact_print: bool_of("compactPrint") == Some(true),
            font: ChartFont::from_id(str_of("font")).unwrap_or(ChartFont::PetalumaScript),
            minor_convention: if str_of("minorConvention") == "tonic" {
                MinorConvention::Tonic
            } else {
                MinorConvention::Relative
            },
            is_sidebar_collapsed: bool_of("isSidebarCollapsed") == Some(true),
            reopen_tabs: bool_of("reopenTabs") != Some(false),
        }
    }

    /// `saved` (settings.json as read) with these settings written over it: the keys this app doesn't know are kept.
    pub fn write_over(&self, saved: &Value) -> Value {
        let mut json = saved.as_object().cloned().unwrap_or_default();
        let style = match self.style {
            DisplayStyle::Fr => "fr",
            DisplayStyle::Intl => "intl",
            DisplayStyle::Latin => "latin",
        };
        let minor_convention = match self.minor_convention {
            MinorConvention::Relative => "relative",
            MinorConvention::Tonic => "tonic",
        };
        json.extend([
            ("style".into(), style.into()),
            ("barsPerRow".into(), self.bars_per_row.into()),
            ("lowConfidenceThreshold".into(), self.low_confidence_threshold.into()),
            ("compactPrint".into(), self.compact_print.into()),
            ("font".into(), self.font.id().into()),
            ("minorConvention".into(), minor_convention.into()),
            ("isSidebarCollapsed".into(), self.is_sidebar_collapsed.into()),
            ("reopenTabs".into(), self.reopen_tabs.into()),
        ]);
        Value::Object(json)
    }

    /// The saved settings, or the defaults when there are none or they can't be read.
    pub fn load() -> Settings {
        let saved = config_dir()
            .and_then(|dir| std::fs::read_to_string(dir.join("settings.json")).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or(Value::Null);
        Settings::parse(&saved)
    }
}

/// The app's config folder, where the Tauri app (identifier `dev.dericbourg.madsister`) kept it.
pub fn config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("dev.dericbourg.madsister"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_of_nothing_returns_the_defaults() {
        // Given / When
        let settings = Settings::parse(&Value::Null);

        // Then
        assert_eq!(
            settings,
            Settings {
                style: DisplayStyle::Fr,
                bars_per_row: 4,
                low_confidence_threshold: 0.5,
                compact_print: false,
                font: ChartFont::PetalumaScript,
                minor_convention: MinorConvention::Relative,
                is_sidebar_collapsed: false,
                reopen_tabs: true,
            }
        );
    }

    #[test]
    fn parse_of_saved_settings_reads_every_key() {
        // Given settings.json as the app wrote it
        let saved = json!({
            "style": "latin", "barsPerRow": 8, "lowConfidenceThreshold": 0.3, "compactPrint": true, "font": "patrick-hand",
            "minorConvention": "tonic", "isSidebarCollapsed": true, "reopenTabs": false
        });

        // When / Then
        assert_eq!(
            Settings::parse(&saved),
            Settings {
                style: DisplayStyle::Latin,
                bars_per_row: 8,
                low_confidence_threshold: 0.3,
                compact_print: true,
                font: ChartFont::PatrickHand,
                minor_convention: MinorConvention::Tonic,
                is_sidebar_collapsed: true,
                reopen_tabs: false,
            }
        );
    }

    #[test]
    fn write_over_writes_every_key_back_and_keeps_the_unknown_ones() {
        // Given settings.json with a key this app doesn't know, and settings that differ from it
        let saved = json!({ "style": "fr", "theme": "dark" });
        let settings = Settings::parse(&json!({
            "style": "intl", "barsPerRow": 2, "lowConfidenceThreshold": 0.25, "compactPrint": true, "font": "kalam",
            "minorConvention": "tonic", "isSidebarCollapsed": true, "reopenTabs": false
        }));

        // When writing them over it
        let written = settings.write_over(&saved);

        // Then the unknown key stays, and the file reads back as the same settings
        assert_eq!(written["theme"], "dark");
        assert_eq!(Settings::parse(&written), settings);
        assert_eq!(Settings::parse(&Settings::parse(&Value::Null).write_over(&Value::Null)), Settings::parse(&Value::Null));
    }

    #[test]
    fn parse_of_invalid_values_falls_back_to_the_defaults() {
        // Given
        let saved = json!({ "style": "de", "barsPerRow": 3, "lowConfidenceThreshold": 2, "font": "comic", "reopenTabs": "no" });

        // When / Then
        assert_eq!(Settings::parse(&saved), Settings::parse(&Value::Null));
    }
}
