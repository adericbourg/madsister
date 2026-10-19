//! The printed chart (spec F-EX-1): lays out a sheet (`model/export/sheet.ts`) on A4 pages and draws it, for the GUI and `madsister print` alike.

use krilla::color::luma;
use krilla::geom::{PathBuilder, Point};
use krilla::page::PageSettings;
use krilla::paint::{LineCap, Stroke, StrokeDash};
use krilla::surface::Surface;
use krilla::text::{Font, TextDirection};
use krilla::Document;
use serde::Deserialize;
use skrifa::instance::{LocationRef, Size};
use skrifa::{FontRef, MetadataProvider};

/// The chart as the frontend spells it: `toSheet` in `model/export/sheet.ts`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sheet {
    pub title: String,
    pub artist: Option<String>,
    pub facts: Vec<String>,
    pub font: String,
    pub compact: bool,
    pub bars_per_row: usize,
    pub sections: Vec<Section>,
}

#[derive(Debug, Deserialize)]
pub struct Section {
    pub label: String,
    pub repeat: Option<u32>,
    pub bars: Vec<Bar>,
}

#[derive(Debug, Deserialize)]
pub struct Bar {
    pub meter: Option<String>,
    pub slots: Vec<Slot>,
}

#[derive(Debug, Deserialize)]
pub struct Slot {
    pub text: String,
    pub beats: f32,
}

const MM: f32 = 72.0 / 25.4;
const PAGE_W: f32 = 595.28;
const PAGE_H: f32 = 841.89;
const MARGIN: f32 = 12.0 * MM;
const COLUMN_GAP: f32 = 20.0 * MM;
const ROW_GAP: f32 = 2.0 * MM;
const PAGE_NUMBER: f32 = 8.4;
const TEXT: f32 = 12.0;
/// Bar lines and the section label's box.
const INK: f32 = 1.5;
/// Space around the label's text, and between it and the repeat count.
const LABEL_PAD: f32 = 4.8;

/// Sizes in points, as grid.css sized the printed chart (1rem = 12pt).
struct Metrics {
    /// A section label or a bar row, with the gap under it.
    line: f32,
    chord: f32,
    meter: f32,
    /// Space on each side of a chord.
    pad: f32,
    title: f32,
    header_gap: f32,
}

const FULL: Metrics = Metrics {
    line: 15.0 * MM,
    chord: 16.8,
    meter: 10.8,
    pad: 6.0,
    title: 24.0,
    header_gap: 24.0,
};
const COMPACT: Metrics = Metrics {
    line: 9.0 * MM,
    chord: 11.4,
    meter: 8.4,
    pad: 3.6,
    title: 16.8,
    header_gap: 12.0,
};

/// A bundled font, with its bytes for measuring.
struct Face {
    data: &'static [u8],
    font: Font,
}

impl Face {
    fn new(data: &'static [u8]) -> Self {
        let font = Font::new(data.into(), 0).expect("bundled fonts are valid");
        Face { data, font }
    }

    fn skrifa(&self) -> FontRef<'static> {
        FontRef::new(self.data).expect("bundled fonts are valid")
    }

    fn width(&self, text: &str, size: f32) -> f32 {
        let font = self.skrifa();
        let (charmap, metrics) = (
            font.charmap(),
            font.glyph_metrics(Size::new(size), LocationRef::default()),
        );
        text.chars()
            .filter_map(|c| charmap.map(c))
            .filter_map(|g| metrics.advance_width(g))
            .sum()
    }
}

/// A chart font, and the one drawing what it lacks (Petaluma Text has no letters, none has ♩), as the browser falls back.
struct Type {
    face: Face,
    fallback: &'static Face,
}

impl Type {
    /// `text` cut into runs of the face that draws them.
    fn runs<'t>(&self, text: &'t str) -> Vec<(&Face, &'t str)> {
        let charmap = self.face.skrifa().charmap();
        let mut runs: Vec<(&Face, &str)> = Vec::new();
        let mut start = 0;
        for (i, c) in text.char_indices() {
            let face = if charmap.map(c).is_some() {
                &self.face
            } else {
                self.fallback
            };
            if let Some((last, run)) = runs.last_mut() {
                if std::ptr::eq(*last, face) {
                    *run = &text[start..i + c.len_utf8()];
                    continue;
                }
            }
            start = i;
            runs.push((face, &text[i..i + c.len_utf8()]));
        }
        runs
    }

    fn width(&self, text: &str, size: f32) -> f32 {
        self.runs(text)
            .iter()
            .map(|(face, run)| face.width(run, size))
            .sum()
    }

    /// `size`, or smaller for `text` to fit in `width`; never under a third of it, which a negative width would flip.
    fn fit(&self, text: &str, size: f32, width: f32) -> f32 {
        let natural = self.width(text, size);
        if natural <= width {
            return size;
        }
        (size * width / natural).max(size / 3.0)
    }

    /// Draws `text` from `x` on the `baseline`.
    fn draw(&self, surface: &mut Surface, x: f32, baseline: f32, size: f32, text: &str) {
        surface.set_fill(None);
        surface.set_stroke(None);
        let mut x = x;
        for (face, run) in self.runs(text) {
            surface.draw_text(
                Point::from_xy(x, baseline),
                face.font.clone(),
                size,
                run,
                false,
                TextDirection::Auto,
            );
            x += face.width(run, size);
        }
    }
}

struct Fonts {
    regular: Type,
    bold: Type,
}

static SERIF: &[u8] = include_bytes!("../fonts/DejaVuSerif.ttf");
static SERIF_BOLD: &[u8] = include_bytes!("../fonts/DejaVuSerif-Bold.ttf");
static FALLBACKS: std::sync::LazyLock<(Face, Face)> =
    std::sync::LazyLock::new(|| (Face::new(SERIF), Face::new(SERIF_BOLD)));

impl Fonts {
    /// `font` is a `Settings.font`. A font without a bold face prints bold text in its regular one.
    fn new(font: &str) -> Result<Self, String> {
        let (regular, bold): (&'static [u8], &'static [u8]) = match font {
            "kalam" => (
                include_bytes!("../fonts/Kalam-Regular.ttf"),
                include_bytes!("../fonts/Kalam-Bold.ttf"),
            ),
            "patrick-hand" => {
                let face = include_bytes!("../fonts/PatrickHand-Regular.ttf");
                (face, face)
            }
            "petaluma-script" => {
                let face = include_bytes!("../fonts/PetalumaScript.ttf");
                (face, face)
            }
            "petaluma-text" => {
                let face = include_bytes!("../fonts/PetalumaText.ttf");
                (face, face)
            }
            "serif" => (SERIF, SERIF_BOLD),
            _ => return Err(format!("unknown font {font}")),
        };
        Ok(Fonts {
            regular: Type {
                face: Face::new(regular),
                fallback: &FALLBACKS.0,
            },
            bold: Type {
                face: Face::new(bold),
                fallback: &FALLBACKS.1,
            },
        })
    }
}

/// A run of bars of one section, printed under its (repeated) label. `to` may pass the last bar.
#[derive(Debug, PartialEq)]
struct Chunk {
    section: usize,
    from: usize,
    to: usize,
}

/// How many lines (a label or a bar row) fit in a column; the first page also holds the header.
struct Layout {
    lines_per_column: usize,
    first_page_lines: usize,
    columns_per_page: usize,
}

/// Fills each column of a page in turn, then the next page: pages, of columns, of chunks.
fn paginate(bar_counts: &[usize], bars_per_row: usize, layout: &Layout) -> Vec<Vec<Vec<Chunk>>> {
    // Two lines at least: a label and one row.
    let (per_column, first_page) = (
        layout.lines_per_column.max(2),
        layout.first_page_lines.max(2),
    );
    let mut columns: Vec<Vec<Chunk>> = vec![Vec::new()];
    let mut free = first_page;
    for (section, &bars) in bar_counts.iter().enumerate() {
        // An empty section still prints its label and a placeholder row.
        let rows = bars.div_ceil(bars_per_row).max(1);
        let mut row = 0;
        while row < rows {
            // A label never ends a column alone.
            if free < 2 {
                columns.push(Vec::new());
                free = if columns.len() > layout.columns_per_page {
                    per_column
                } else {
                    first_page
                };
            }
            let take = (rows - row).min(free - 1);
            columns.last_mut().unwrap().push(Chunk {
                section,
                from: row * bars_per_row,
                to: (row + take) * bars_per_row,
            });
            row += take;
            free -= take + 1;
        }
    }
    let mut pages = Vec::new();
    let mut columns = columns.into_iter().peekable();
    while columns.peek().is_some() {
        let mut page: Vec<_> = columns.by_ref().take(layout.columns_per_page).collect();
        page.resize_with(layout.columns_per_page, Vec::new);
        pages.push(page);
    }
    pages
}

fn line(surface: &mut Surface, from: (f32, f32), to: (f32, f32), stroke: Stroke) {
    let mut path = PathBuilder::new();
    path.move_to(from.0, from.1);
    path.line_to(to.0, to.1);
    if let Some(path) = path.finish() {
        surface.set_fill(None);
        surface.set_stroke(Some(stroke));
        surface.draw_path(&path);
    }
}

fn ink() -> Stroke {
    Stroke {
        width: INK,
        ..Default::default()
    }
}

/// grid.css's --hairline, dotted (between slots) or dashed (between columns).
fn hairline(dash: [f32; 2], line_cap: LineCap) -> Stroke {
    Stroke {
        paint: luma::Color::new(0x76).into(),
        width: 0.75,
        line_cap,
        dash: Some(StrokeDash {
            array: dash.to_vec(),
            offset: 0.0,
        }),
        ..Default::default()
    }
}

/// Draws the title, artist and facts from `top` (when there's a surface); returns their height.
fn header(
    sheet: &Sheet,
    fonts: &Fonts,
    m: &Metrics,
    top: f32,
    mut surface: Option<&mut Surface>,
) -> f32 {
    let mut y = top;
    let mut text = |y: f32, ty: &Type, size: f32, s: &str, centered: bool| {
        if let Some(surface) = surface.as_deref_mut() {
            let x = if centered {
                (PAGE_W - ty.width(s, size)) / 2.0
            } else {
                MARGIN
            };
            ty.draw(surface, x, y + size, size, s);
        }
    };
    let title = fonts.bold.fit(&sheet.title, m.title, PAGE_W - 2.0 * MARGIN);
    text(y, &fonts.bold, title, &sheet.title, true);
    y += title * 1.25;
    // shortcut: the artist was italic on screen; no bundled font has an italic face. Bundle some if that matters.
    if let Some(artist) = &sheet.artist {
        text(y + 3.0, &fonts.regular, TEXT, artist, true);
        y += 3.0 + TEXT * 1.5;
    }
    if !sheet.facts.is_empty() {
        y += TEXT;
    }
    for fact in &sheet.facts {
        text(y, &fonts.regular, TEXT, fact, false);
        y += TEXT * 1.5;
    }
    y + m.header_gap - top
}

/// Draws a chunk in the `(x, width)` column from `top`; returns its height.
fn chunk(
    surface: &mut Surface,
    sheet: &Sheet,
    fonts: &Fonts,
    m: &Metrics,
    chunk: &Chunk,
    (x, width): (f32, f32),
    top: f32,
) -> f32 {
    let section = &sheet.sections[chunk.section];
    let row_h = m.line - ROW_GAP;

    // The label in a box sitting on the line's bottom, then its repeat count.
    let label_w = fonts.bold.width(&section.label, TEXT) + 2.0 * LABEL_PAD;
    let (box_top, bottom) = (top + row_h - TEXT * 1.4, top + row_h);
    let right = x + label_w;
    for (from, to) in [
        ((x, box_top), (right, box_top)),
        ((right, box_top), (right, bottom)),
        ((right, bottom), (x, bottom)),
        ((x, bottom), (x, box_top)),
    ] {
        line(surface, from, to, ink());
    }
    fonts.bold.draw(
        surface,
        x + LABEL_PAD,
        bottom - TEXT * 0.4,
        TEXT,
        &section.label,
    );
    if let Some(repeat) = section.repeat {
        fonts.bold.draw(
            surface,
            right + LABEL_PAD,
            bottom - TEXT * 0.4,
            TEXT,
            &format!("x{repeat}"),
        );
    }

    let bars = &section.bars[chunk.from.min(section.bars.len())..chunk.to.min(section.bars.len())];
    let bar_w = width / sheet.bars_per_row as f32;
    let rows: Vec<&[Bar]> = if bars.is_empty() {
        vec![&[]]
    } else {
        bars.chunks(sheet.bars_per_row).collect()
    };
    for (r, row) in rows.iter().enumerate() {
        let (row_top, row_bottom) = (
            top + m.line * (r + 1) as f32,
            top + m.line * (r + 1) as f32 + row_h,
        );
        line(surface, (x, row_top), (x, row_bottom), ink());
        // A section without bars gets an empty one as wide as the row.
        if row.is_empty() {
            line(
                surface,
                (x + width, row_top),
                (x + width, row_bottom),
                ink(),
            );
        }
        for (i, bar) in row.iter().enumerate() {
            let bar_x = x + bar_w * i as f32;
            line(
                surface,
                (bar_x + bar_w, row_top),
                (bar_x + bar_w, row_bottom),
                ink(),
            );
            let middle = row_top + row_h / 2.0;
            // A meter override opens the bar, like a time signature; the slots share what's left.
            let mut slot_x = bar_x;
            if let Some(meter) = &bar.meter {
                fonts.bold.draw(
                    surface,
                    bar_x + LABEL_PAD,
                    middle + m.meter * 0.35,
                    m.meter,
                    meter,
                );
                slot_x += LABEL_PAD + fonts.bold.width(meter, m.meter);
            }
            let beats: f32 = bar.slots.iter().map(|s| s.beats).sum();
            let slots_w = bar_x + bar_w - slot_x;
            for (k, slot) in bar.slots.iter().enumerate() {
                let slot_w = slots_w * slot.beats / beats;
                if k > 0 {
                    line(
                        surface,
                        (slot_x, row_top),
                        (slot_x, row_bottom),
                        hairline([0.0, 2.0], LineCap::Round),
                    );
                }
                let size = fonts.bold.fit(&slot.text, m.chord, slot_w - 2.0 * m.pad);
                fonts.bold.draw(
                    surface,
                    slot_x + m.pad,
                    middle + m.chord * 0.35,
                    size,
                    &slot.text,
                );
                slot_x += slot_w;
            }
        }
    }
    m.line * (rows.len() + 1) as f32
}

/// The sheet as a PDF document.
pub fn render(sheet: &Sheet) -> Result<Vec<u8>, String> {
    if sheet.bars_per_row == 0 {
        return Err("no bars per row".into());
    }
    let fonts = Fonts::new(&sheet.font)?;
    let (m, columns_per_page) = if sheet.compact {
        (&COMPACT, 2)
    } else {
        (&FULL, 1)
    };
    let column_w = (PAGE_W - 2.0 * MARGIN - COLUMN_GAP * (columns_per_page - 1) as f32)
        / columns_per_page as f32;
    let content_top = MARGIN + PAGE_NUMBER * 1.2 + ROW_GAP;
    let header_h = header(sheet, &fonts, m, content_top, None);
    // The last line's gap may run into the bottom margin.
    let lines = |height: f32| ((height + ROW_GAP) / m.line).floor().max(0.0) as usize;
    let layout = Layout {
        lines_per_column: lines(PAGE_H - MARGIN - content_top),
        first_page_lines: lines(PAGE_H - MARGIN - content_top - header_h),
        columns_per_page,
    };
    let bar_counts: Vec<usize> = sheet.sections.iter().map(|s| s.bars.len()).collect();
    let pages = paginate(&bar_counts, sheet.bars_per_row, &layout);

    let mut document = Document::new();
    for (p, columns) in pages.iter().enumerate() {
        let mut page = document.start_page_with(PageSettings::from_wh(PAGE_W, PAGE_H).unwrap());
        let mut surface = page.surface();
        let number = format!("{} / {}", p + 1, pages.len());
        let number_x = PAGE_W - MARGIN - fonts.regular.width(&number, PAGE_NUMBER);
        fonts.regular.draw(
            &mut surface,
            number_x,
            MARGIN + PAGE_NUMBER,
            PAGE_NUMBER,
            &number,
        );
        let top = if p == 0 {
            content_top + header(sheet, &fonts, m, content_top, Some(&mut surface))
        } else {
            content_top
        };
        let mut tallest: f32 = 0.0;
        for (c, column) in columns.iter().enumerate() {
            let x = MARGIN + (column_w + COLUMN_GAP) * c as f32;
            let mut y = top;
            for piece in column {
                y += chunk(&mut surface, sheet, &fonts, m, piece, (x, column_w), y);
            }
            tallest = tallest.max(y - top - ROW_GAP);
        }
        // A rule down the middle of the gap, as tall as the longer column.
        for c in 1..columns_per_page {
            let x = MARGIN + (column_w + COLUMN_GAP) * c as f32 - COLUMN_GAP / 2.0;
            line(
                &mut surface,
                (x, top),
                (x, top + tallest),
                hairline([3.0, 3.0], LineCap::Butt),
            );
        }
        surface.finish();
        page.finish();
    }
    document
        .finish()
        .map_err(|e| format!("can't write the PDF: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_columns(lines_per_column: usize, first_page_lines: usize) -> Layout {
        Layout {
            lines_per_column,
            first_page_lines,
            columns_per_page: 2,
        }
    }

    fn chunk(section: usize, from: usize, to: usize) -> Chunk {
        Chunk { section, from, to }
    }

    #[test]
    fn fills_the_left_column_then_the_right_one_then_the_next_page() {
        // Given three 2-row sections (3 lines each), 3 lines per column on the first page and 6 after
        let pages = paginate(&[8, 8, 8], 4, &two_columns(6, 3));

        // Then page 1 holds one section per column, page 2 the third one
        assert_eq!(
            pages,
            [
                vec![vec![chunk(0, 0, 8)], vec![chunk(1, 0, 8)]],
                vec![vec![chunk(2, 0, 8)], vec![]]
            ]
        );
    }

    #[test]
    fn splits_a_long_section_between_rows_and_repeats_its_label() {
        // Given a 5-row section and 4-line columns
        let pages = paginate(&[10], 2, &two_columns(4, 4));

        // Then it continues in the right column, under its label again
        assert_eq!(pages, [vec![vec![chunk(0, 0, 6)], vec![chunk(0, 6, 10)]]]);
    }

    #[test]
    fn never_leaves_a_label_alone_at_the_bottom_of_a_column() {
        // Given a first section leaving one free line
        let pages = paginate(&[4, 4], 4, &two_columns(3, 3));

        // Then the second section starts the right column
        assert_eq!(pages[0], [vec![chunk(0, 0, 4)], vec![chunk(1, 0, 4)]]);
    }

    #[test]
    fn gives_an_empty_section_its_label_and_a_placeholder_row() {
        assert_eq!(
            paginate(&[0], 4, &two_columns(25, 21)),
            [vec![vec![chunk(0, 0, 4)], vec![]]]
        );
    }

    #[test]
    fn lays_out_one_column_per_page() {
        // Given two 2-row sections, 3 lines on the first page and 4 after
        let pages = paginate(
            &[8, 8],
            4,
            &Layout {
                lines_per_column: 4,
                first_page_lines: 3,
                columns_per_page: 1,
            },
        );

        // Then each section gets its own page
        assert_eq!(
            pages,
            [vec![vec![chunk(0, 0, 8)]], vec![vec![chunk(1, 0, 8)]]]
        );
    }

    #[test]
    fn falls_back_to_the_serif_for_what_the_font_lacks() {
        // Given Petaluma Script, which has no ♩
        let fonts = Fonts::new("petaluma-script").unwrap();

        // When
        let runs: Vec<_> = fonts
            .regular
            .runs("♩ = 120")
            .into_iter()
            .map(|(face, run)| (std::ptr::eq(face, &FALLBACKS.0), run))
            .collect();

        // Then
        assert_eq!(runs, [(true, "♩"), (false, " = 120")]);
    }

    #[test]
    fn shrinks_text_to_fit_but_never_flips_it() {
        let fonts = Fonts::new("kalam").unwrap();
        let natural = fonts.bold.width("Cmaj7", 12.0);

        assert_eq!(fonts.bold.fit("Cmaj7", 12.0, natural + 1.0), 12.0);
        assert!((fonts.bold.fit("Cmaj7", 12.0, natural / 2.0) - 6.0).abs() < 0.01);
        assert_eq!(fonts.bold.fit("Cmaj7", 12.0, -5.0), 4.0);
    }

    fn sheet(font: &str, compact: bool, sections: usize) -> Sheet {
        let slot = |text: &str, beats| Slot {
            text: text.into(),
            beats,
        };
        let bar = || Bar {
            meter: None,
            slots: vec![slot("Cmaj7", 2.0), slot("Am7/G", 2.0)],
        };
        Sheet {
            title: "Song".into(),
            artist: Some("Band".into()),
            facts: vec!["Key Ré".into(), "♩ = 120".into()],
            font: font.into(),
            compact,
            bars_per_row: 4,
            sections: (0..sections)
                .map(|i| Section {
                    label: format!("Part {i}"),
                    repeat: Some(2),
                    bars: (0..8).map(|_| bar()).collect(),
                })
                .chain([Section {
                    label: "Empty".into(),
                    repeat: None,
                    bars: vec![Bar {
                        meter: Some("2/4".into()),
                        slots: vec![slot("ø7", 2.0)],
                    }],
                }])
                .collect(),
        }
    }

    fn page_count(pdf: &[u8]) -> usize {
        let pdf = String::from_utf8_lossy(pdf);
        let count = pdf.split("/Type/Pages/Count ").nth(1).expect("a page tree");
        count[..count.find(|c: char| !c.is_ascii_digit()).unwrap()]
            .parse()
            .unwrap()
    }

    #[test]
    fn renders_every_font_to_a_pdf() {
        for font in [
            "kalam",
            "patrick-hand",
            "petaluma-script",
            "petaluma-text",
            "serif",
        ] {
            let pdf = render(&sheet(font, false, 2)).unwrap();
            assert!(pdf.starts_with(b"%PDF"), "{font}");
        }
        assert_eq!(
            render(&sheet("comic", false, 1)).err(),
            Some("unknown font comic".into())
        );
    }

    #[test]
    fn a_long_chart_takes_more_pages_than_its_compact_print() {
        let full = render(&sheet("kalam", false, 8)).unwrap();
        let compact = render(&sheet("kalam", true, 8)).unwrap();
        assert!(
            page_count(&full) > page_count(&compact),
            "{} vs {}",
            page_count(&full),
            page_count(&compact)
        );
        assert!(page_count(&compact) >= 1);
    }
}
