//! Text layout and rendering using parley.

use parley::{FontContext, LayoutContext};
use std::collections::HashMap;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::zeno::Format;
use swash::{FontRef, Metrics as SwashMetrics};

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

/// Font metrics from the OS/2 and hhea tables.
#[derive(Clone, Copy, Debug)]
pub struct FontMetrics {
    /// Distance from baseline to top of the alignment box.
    pub ascent: f32,
    /// Distance from baseline to bottom of the alignment box (typically negative).
    pub descent: f32,
    /// Recommended additional spacing between lines.
    pub leading: f32,
    /// Distance from baseline to top of capital letters.
    pub cap_height: f32,
    /// Distance from baseline to top of lowercase 'x'.
    pub x_height: f32,
    /// Recommended underline position (from baseline).
    pub underline_offset: f32,
    /// Recommended strikeout position (from baseline).
    pub strikeout_offset: f32,
    /// Recommended stroke thickness.
    pub stroke_size: f32,
}

impl FontMetrics {
    /// Create font metrics from swash metrics.
    pub fn from_swash(m: &SwashMetrics) -> Self {
        Self {
            ascent: m.ascent,
            descent: m.descent,
            leading: m.leading,
            cap_height: m.cap_height,
            x_height: m.x_height,
            underline_offset: m.underline_offset,
            strikeout_offset: m.strikeout_offset,
            stroke_size: m.stroke_size,
        }
    }
}

/// Metrics for a line of text.
#[derive(Clone, Copy, Debug)]
pub struct LineLayoutMetrics {
    /// Typographic ascent for this line.
    pub ascent: f32,
    /// Typographic descent for this line.
    pub descent: f32,
    /// Typographic leading for this line.
    pub leading: f32,
    /// Total line height (ascent + descent + leading).
    pub line_height: f32,
    /// Y offset to the baseline from the line's top.
    pub baseline: f32,
    /// Total advance width of the line.
    pub advance: f32,
}

impl TextLayout {
    pub fn width(&self) -> f32 {
        self.layout.width()
    }

    pub fn height(&self) -> f32 {
        self.layout.height()
    }

    /// Get metrics for each line in the layout.
    pub fn line_metrics(&self) -> Vec<LineLayoutMetrics> {
        self.layout
            .lines()
            .map(|line| {
                let m = line.metrics();
                LineLayoutMetrics {
                    ascent: m.ascent,
                    descent: m.descent,
                    leading: m.leading,
                    line_height: m.line_height,
                    baseline: m.baseline,
                    advance: m.advance,
                }
            })
            .collect()
    }

    /// Get font metrics for the first run in the layout.
    /// Returns None if there are no runs.
    pub fn font_metrics(&self) -> Option<FontMetrics> {
        for line in self.layout.lines() {
            for item in line.items() {
                if let parley::layout::PositionedLayoutItem::GlyphRun(run) = item {
                    let font_data = run.run().font();
                    let font_ref =
                        FontRef::from_index(font_data.data.as_ref(), font_data.index as usize)?;
                    let normalized_coords: Vec<i16> = run.run().normalized_coords().to_vec();
                    let swash_metrics = font_ref
                        .metrics(&normalized_coords)
                        .scale(run.run().font_size());
                    return Some(FontMetrics::from_swash(&swash_metrics));
                }
            }
        }
        None
    }

    /// Find the byte offset in the source text closest to the given x position.
    ///
    /// Returns the byte offset where a cursor should be placed if clicking at `x`.
    /// For single-line text, pass y=0. The function determines which cluster the
    /// point is closest to and whether the cursor should be before or after it.
    pub fn index_for_x(&self, x: f32, source_text: &str) -> usize {
        use parley::layout::{Cluster, ClusterSide};

        // Use parley's built-in cluster lookup (y=0 for single-line)
        if let Some((cluster, side)) = Cluster::from_point(&self.layout, x, 0.0) {
            let text_range = cluster.text_range();
            match side {
                ClusterSide::Left => text_range.start,
                ClusterSide::Right => text_range.end,
            }
        } else {
            // Click was outside all clusters
            if x <= 0.0 {
                0
            } else {
                source_text.len()
            }
        }
    }

    /// Find the byte offset in the source text closest to the given (x, y) point.
    ///
    /// Works for both single-line and multiline text. Pass the pixel coordinates
    /// of the point (e.g. a mouse click position relative to the text origin).
    /// The function determines which cluster the point is closest to and whether
    /// the cursor should be placed before or after it.
    ///
    /// This is the multiline equivalent of [`index_for_x`] and is needed for
    /// vertical cursor movement — the caller supplies the `y` of the target line
    /// (obtained from [`line_metrics`]) together with the preferred `x`.
    pub fn index_for_point(&self, x: f32, y: f32, source_text: &str) -> usize {
        use parley::layout::{Cluster, ClusterSide};

        if let Some((cluster, side)) = Cluster::from_point(&self.layout, x, y) {
            let text_range = cluster.text_range();
            match side {
                ClusterSide::Left => text_range.start,
                ClusterSide::Right => text_range.end,
            }
        } else {
            // Point is outside all clusters — clamp to nearest text boundary.
            if y <= 0.0 {
                // Above the text: delegate to the single-line helper (y=0),
                // which handles the x-axis clamping for the first line.
                self.index_for_x(x, source_text)
            } else {
                // Below the last line — clamp to end of text.
                source_text.len()
            }
        }
    }

    /// Return the cumulative top-y offset (in pixels, from the text origin) of the
    /// line that contains the given byte `offset`.
    ///
    /// Returns `(line_index, line_top_y)` where `line_index` is 0-based.
    /// If the offset is past the end of text the last line is returned.
    ///
    /// Together with [`line_metrics`] this lets callers compute the baseline y of
    /// the target line for vertical cursor movement:
    ///
    /// ```text
    /// let (idx, top_y) = layout.line_top_for_offset(cursor_offset);
    /// let metrics = layout.line_metrics();
    /// // Move up:  target_y = top_of_previous_line + metrics[idx-1].baseline
    /// // Move down: target_y = top_of_next_line    + metrics[idx+1].baseline
    /// ```
    pub fn line_top_for_offset(&self, offset: usize) -> (usize, f32) {
        let mut cumulative_y = 0.0_f32;
        let mut last_result = (0usize, 0.0_f32);
        for (i, line) in self.layout.lines().enumerate() {
            last_result = (i, cumulative_y);
            // line.text_range() returns the byte span of this line in the source text,
            // including any trailing newline.
            if offset < line.text_range().end {
                return (i, cumulative_y);
            }
            cumulative_y += line.metrics().line_height;
        }
        // offset is at or past the end of the last line — return the last line.
        last_result
    }

    /// Iterate over glyph runs for rendering.
    pub fn glyph_runs(&self) -> impl Iterator<Item = GlyphRun> + '_ {
        self.layout.lines().flat_map(|line| {
            line.items().filter_map(|item| {
                match item {
                    parley::layout::PositionedLayoutItem::GlyphRun(run) => {
                        // Use positioned_glyphs() which handles advance accumulation
                        let glyphs: Vec<PositionedGlyph> = run
                            .positioned_glyphs()
                            .map(|g| PositionedGlyph {
                                id: g.id,
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
                        // Use positioned_glyphs() which handles advance accumulation
                        let glyphs: Vec<PositionedGlyph> = run
                            .positioned_glyphs()
                            .map(|g| PositionedGlyph {
                                id: g.id,
                                x: g.x,
                                y: g.y,
                                advance: g.advance,
                            })
                            .collect();

                        let inner_run = run.run();
                        let font = inner_run.font();

                        // Get normalized coordinates for variable fonts
                        let normalized_coords: Vec<i16> = inner_run.normalized_coords().to_vec();

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
                let rasterized =
                    cache.rasterize(font_data, &run.normalized_coords, glyph.id, run.font_size);

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

    // index_for_point tests

    #[test]
    fn index_for_point_at_origin_returns_zero() {
        let mut ctx = TextContext::new();
        let text = "Hello";
        let layout = ctx.layout_text(text, 16.0);

        // Click at the very start
        let idx = layout.index_for_point(0.0, 0.0, text);
        assert_eq!(idx, 0);
    }

    #[test]
    fn index_for_point_far_right_returns_end() {
        let mut ctx = TextContext::new();
        let text = "Hi";
        let layout = ctx.layout_text(text, 16.0);

        // Click well beyond the text width
        let idx = layout.index_for_point(10_000.0, 0.0, text);
        assert_eq!(idx, text.len());
    }

    #[test]
    fn index_for_point_below_text_returns_end() {
        let mut ctx = TextContext::new();
        let text = "Hello";
        let layout = ctx.layout_text(text, 16.0);

        // Click far below the text
        let idx = layout.index_for_point(10.0, 10_000.0, text);
        assert_eq!(idx, text.len());
    }

    #[test]
    fn index_for_point_matches_index_for_x_on_single_line() {
        let mut ctx = TextContext::new();
        let text = "Hello world";
        let layout = ctx.layout_text(text, 16.0);

        // For single-line text, index_for_point(x, 0) should match index_for_x(x)
        let x = layout.width() / 2.0;
        let from_x = layout.index_for_x(x, text);
        let from_point = layout.index_for_point(x, 0.0, text);
        assert_eq!(from_x, from_point);
    }

    #[test]
    fn index_for_point_multiline_first_line() {
        let mut ctx = TextContext::new();
        let text = "Line one\nLine two";
        let layout = ctx.layout_text(text, 16.0);

        // Clicking near the start of the first line should return an offset in line one
        let idx = layout.index_for_point(1.0, 0.0, text);
        assert!(idx < 9, "expected offset within first line, got {idx}");
    }

    #[test]
    fn index_for_point_multiline_second_line() {
        let mut ctx = TextContext::new();
        let text = "Line one\nLine two";
        let layout = ctx.layout_text(text, 16.0);

        let line_metrics = layout.line_metrics();
        assert!(line_metrics.len() >= 2, "expected at least 2 lines");

        // Y in the middle of the second line
        let second_line_y = line_metrics[0].line_height + line_metrics[1].line_height / 2.0;
        let idx = layout.index_for_point(0.0, second_line_y, text);

        // The offset should be in the second line (past the '\n' at index 8)
        assert!(idx > 8, "expected offset past newline, got {idx}");
    }

    // line_top_for_offset tests

    #[test]
    fn line_top_for_offset_single_line_is_zero() {
        let mut ctx = TextContext::new();
        let text = "Hello";
        let layout = ctx.layout_text(text, 16.0);

        let (line_idx, top_y) = layout.line_top_for_offset(0);
        assert_eq!(line_idx, 0);
        assert_eq!(top_y, 0.0);
    }

    #[test]
    fn line_top_for_offset_multiline_first_line() {
        let mut ctx = TextContext::new();
        let text = "First line\nSecond line";
        let layout = ctx.layout_text(text, 16.0);

        let (line_idx, top_y) = layout.line_top_for_offset(0);
        assert_eq!(line_idx, 0);
        assert_eq!(top_y, 0.0);
    }

    #[test]
    fn line_top_for_offset_multiline_second_line() {
        let mut ctx = TextContext::new();
        let text = "First line\nSecond line";
        let layout = ctx.layout_text(text, 16.0);

        let line_metrics = layout.line_metrics();
        assert!(line_metrics.len() >= 2, "expected at least 2 lines");

        // Offset 11 is the start of "Second line"
        let (line_idx, top_y) = layout.line_top_for_offset(11);
        assert_eq!(line_idx, 1);
        assert!(
            (top_y - line_metrics[0].line_height).abs() < 1.0,
            "expected top_y ≈ first line_height, got {top_y}"
        );
    }
}
