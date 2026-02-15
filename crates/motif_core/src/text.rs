//! Text layout and rendering using parley.

use parley::{FontContext, LayoutContext};

/// Shared resources for text layout.
pub struct TextContext {
    font_cx: FontContext,
    layout_cx: LayoutContext<()>,
}

impl TextContext {
    pub fn new() -> Self {
        Self {
            font_cx: FontContext::new(),
            layout_cx: LayoutContext::new(),
        }
    }

    /// Layout text with given font size, using system default font.
    pub fn layout_text(&mut self, text: &str, font_size: f32) -> TextLayout {
        let mut builder = self
            .layout_cx
            .ranged_builder(&mut self.font_cx, text, 1.0, false);
        builder.push_default(parley::style::StyleProperty::FontSize(font_size));
        let mut layout = builder.build(text);
        layout.break_all_lines(None);
        layout.align(
            None,
            parley::layout::Alignment::Start,
            parley::layout::AlignmentOptions::default(),
        );
        TextLayout { layout }
    }
}

impl Default for TextContext {
    fn default() -> Self {
        Self::new()
    }
}

/// A laid-out piece of text ready for rendering.
pub struct TextLayout {
    layout: parley::Layout<()>,
}

impl TextLayout {
    pub fn width(&self) -> f32 {
        self.layout.width()
    }

    pub fn height(&self) -> f32 {
        self.layout.height()
    }

    /// Iterate over glyph runs for rendering.
    pub fn glyph_runs(&self) -> impl Iterator<Item = GlyphRun> + '_ {
        self.layout.lines().flat_map(|line| {
            line.items().filter_map(|item| {
                match item {
                    parley::layout::PositionedLayoutItem::GlyphRun(run) => {
                        let glyphs: Vec<PositionedGlyph> = run
                            .glyphs()
                            .map(|g| PositionedGlyph {
                                id: g.id as u32,
                                x: g.x,
                                y: g.y,
                                advance: g.advance,
                            })
                            .collect();
                        Some(GlyphRun {
                            glyphs,
                            font_size: run.run().font_size(),
                        })
                    }
                    _ => None,
                }
            })
        })
    }
}

/// A run of glyphs with the same styling.
#[derive(Debug)]
pub struct GlyphRun {
    pub glyphs: Vec<PositionedGlyph>,
    pub font_size: f32,
}

/// A positioned glyph ready for rendering.
#[derive(Debug, Clone)]
pub struct PositionedGlyph {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub advance: f32,
}

/// Re-export parley's FontData for users who need font bytes.
pub use parley::FontData;

/// A glyph run with font data for rasterization.
#[derive(Debug, Clone)]
pub struct GlyphRunWithFont {
    pub glyphs: Vec<PositionedGlyph>,
    pub font_size: f32,
    pub font_data: Option<FontData>,
    pub normalized_coords: Vec<i16>,
}

impl TextLayout {
    /// Iterate over glyph runs with font data for rasterization.
    pub fn glyph_runs_with_font(&self) -> impl Iterator<Item = GlyphRunWithFont> + '_ {
        self.layout.lines().flat_map(|line| {
            line.items().filter_map(|item| {
                match item {
                    parley::layout::PositionedLayoutItem::GlyphRun(run) => {
                        let glyphs: Vec<PositionedGlyph> = run
                            .glyphs()
                            .map(|g| PositionedGlyph {
                                id: g.id as u32,
                                x: g.x,
                                y: g.y,
                                advance: g.advance,
                            })
                            .collect();

                        let inner_run = run.run();
                        let font = inner_run.font();

                        // Get normalized coordinates for variable fonts
                        let normalized_coords: Vec<i16> = inner_run
                            .normalized_coords()
                            .to_vec();

                        Some(GlyphRunWithFont {
                            glyphs,
                            font_size: inner_run.font_size(),
                            font_data: Some(font.clone()),
                            normalized_coords,
                        })
                    }
                    _ => None,
                }
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_context_can_be_created() {
        let _ctx = TextContext::new();
    }

    #[test]
    fn layout_simple_text() {
        let mut ctx = TextContext::new();
        let layout = ctx.layout_text("Hello", 16.0);

        assert!(layout.width() > 0.0);
        assert!(layout.height() > 0.0);
    }

    #[test]
    fn layout_returns_glyph_runs() {
        let mut ctx = TextContext::new();
        let layout = ctx.layout_text("Hi", 16.0);

        let runs: Vec<_> = layout.glyph_runs().collect();
        assert!(!runs.is_empty(), "should have at least one glyph run");
    }

    #[test]
    fn glyph_run_has_glyphs_with_positions() {
        let mut ctx = TextContext::new();
        let layout = ctx.layout_text("A", 16.0);

        let runs: Vec<_> = layout.glyph_runs().collect();
        let first_run = &runs[0];

        assert!(!first_run.glyphs.is_empty(), "should have glyphs");

        let glyph = &first_run.glyphs[0];
        // Glyph should have an ID and position
        assert!(glyph.id != 0, "glyph should have a valid ID");
    }

    #[test]
    fn glyph_run_has_font_data_for_rasterization() {
        let mut ctx = TextContext::new();
        let layout = ctx.layout_text("A", 16.0);

        for run in layout.glyph_runs_with_font() {
            // Should have font data we can use with swash
            assert!(run.font_data.is_some(), "should have font data");
            let font = run.font_data.as_ref().unwrap();
            assert!(!font.data.is_empty(), "font data should not be empty");
        }
    }
}
