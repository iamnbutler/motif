//! Text layout and rendering using parley.

use parley::{FontContext, LayoutContext};
use std::collections::HashMap;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::zeno::Format;
use swash::FontRef;

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

/// Key for caching rasterized glyphs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GlyphKey {
    /// Unique font blob ID.
    font_id: u64,
    /// Font index in collection.
    font_index: u32,
    /// Glyph ID.
    glyph_id: u32,
    /// Font size bits (for exact float comparison).
    font_size_bits: u32,
    /// Normalized coords hash for variable fonts.
    coords_hash: u64,
}

impl GlyphKey {
    fn new(font: &FontData, glyph_id: u32, font_size: f32, normalized_coords: &[i16]) -> Self {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        normalized_coords.hash(&mut hasher);

        Self {
            font_id: font.data.id(),
            font_index: font.index,
            glyph_id,
            font_size_bits: font_size.to_bits(),
            coords_hash: hasher.finish(),
        }
    }
}

/// A rasterized glyph image.
#[derive(Debug, Clone)]
pub struct RasterizedGlyph {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Bearing X (offset from origin).
    pub bearing_x: i32,
    /// Bearing Y (offset from baseline).
    pub bearing_y: i32,
    /// Alpha channel pixel data (row-major, top-to-bottom).
    pub data: Vec<u8>,
}

/// Cache for rasterized glyphs.
pub struct GlyphCache {
    scale_context: ScaleContext,
    cache: HashMap<GlyphKey, RasterizedGlyph>,
}

impl GlyphCache {
    pub fn new() -> Self {
        Self {
            scale_context: ScaleContext::new(),
            cache: HashMap::new(),
        }
    }

    /// Number of cached glyphs.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Rasterize a glyph, using cache if available.
    pub fn rasterize(
        &mut self,
        font: &FontData,
        normalized_coords: &[i16],
        glyph_id: u32,
        font_size: f32,
    ) -> Option<&RasterizedGlyph> {
        let key = GlyphKey::new(font, glyph_id, font_size, normalized_coords);

        // Check cache first
        if self.cache.contains_key(&key) {
            return self.cache.get(&key);
        }

        // Rasterize the glyph
        let font_ref = FontRef::from_index(font.data.as_ref(), font.index as usize)?;

        let mut scaler = self
            .scale_context
            .builder(font_ref)
            .size(font_size)
            .hint(true)
            .normalized_coords(normalized_coords)
            .build();

        let image = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ])
        .format(Format::Alpha)
        .render(&mut scaler, glyph_id as u16)?;

        let rasterized = RasterizedGlyph {
            width: image.placement.width,
            height: image.placement.height,
            bearing_x: image.placement.left,
            bearing_y: image.placement.top,
            data: image.data,
        };

        self.cache.insert(key.clone(), rasterized);
        self.cache.get(&key)
    }
}

impl Default for GlyphCache {
    fn default() -> Self {
        Self::new()
    }
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

    // GlyphCache tests

    #[test]
    fn glyph_cache_can_rasterize_glyph() {
        let mut text_ctx = TextContext::new();
        let layout = text_ctx.layout_text("A", 32.0);

        let mut cache = GlyphCache::new();

        for run in layout.glyph_runs_with_font() {
            let font_data = run.font_data.as_ref().unwrap();
            for glyph in &run.glyphs {
                let rasterized = cache.rasterize(
                    font_data,
                    &run.normalized_coords,
                    glyph.id,
                    run.font_size,
                );

                assert!(rasterized.is_some(), "should rasterize glyph");
                let rast = rasterized.unwrap();
                assert!(rast.width > 0, "rasterized glyph should have width");
                assert!(rast.height > 0, "rasterized glyph should have height");
                assert!(!rast.data.is_empty(), "should have pixel data");
            }
        }
    }

    #[test]
    fn glyph_cache_caches_rasterized_glyphs() {
        let mut text_ctx = TextContext::new();
        let layout = text_ctx.layout_text("A", 32.0);

        let mut cache = GlyphCache::new();

        let run = layout.glyph_runs_with_font().next().unwrap();
        let font_data = run.font_data.as_ref().unwrap();
        let glyph = &run.glyphs[0];

        // First call rasterizes
        let _ = cache.rasterize(font_data, &run.normalized_coords, glyph.id, run.font_size);

        // Cache should now have one entry
        assert_eq!(cache.len(), 1);

        // Second call should hit cache (same result, no additional entry)
        let _ = cache.rasterize(font_data, &run.normalized_coords, glyph.id, run.font_size);
        assert_eq!(cache.len(), 1);
    }
}
