//! Layout engine using Taffy for flexbox layout.

use crate::{Point, Rect, Size, TextContext};
use taffy::prelude::*;

/// Context attached to layout nodes that need measurement (e.g., text).
#[derive(Clone)]
pub enum MeasureContext {
    /// Text that needs to be measured for layout.
    Text {
        content: String,
        font_size: f32,
    },
}

/// Layout engine wrapping Taffy.
pub struct LayoutEngine {
    taffy: TaffyTree<MeasureContext>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        let mut taffy = TaffyTree::new();
        taffy.enable_rounding();
        Self { taffy }
    }

    /// Clear all nodes for a fresh layout pass.
    pub fn clear(&mut self) {
        self.taffy.clear();
    }

    /// Create a leaf node (no children).
    pub fn new_leaf(&mut self, style: Style) -> NodeId {
        self.taffy.new_leaf(style).expect("taffy new_leaf failed")
    }

    /// Create a leaf node with measure context (for text, images, etc.).
    pub fn new_leaf_with_context(&mut self, style: Style, context: MeasureContext) -> NodeId {
        self.taffy
            .new_leaf_with_context(style, context)
            .expect("taffy new_leaf_with_context failed")
    }

    /// Create a node with children.
    pub fn new_with_children(&mut self, style: Style, children: &[NodeId]) -> NodeId {
        self.taffy
            .new_with_children(style, children)
            .expect("taffy new_with_children failed")
    }

    /// Compute layout for the tree rooted at `root`.
    pub fn compute_layout(
        &mut self,
        root: NodeId,
        available_width: f32,
        available_height: f32,
        text_context: &mut TextContext,
    ) {
        let available_space = taffy::Size {
            width: AvailableSpace::Definite(available_width),
            height: AvailableSpace::Definite(available_height),
        };

        self.taffy
            .compute_layout_with_measure(
                root,
                available_space,
                |known_dimensions, available_space, _node_id, node_context, _style| {
                    measure_node(known_dimensions, available_space, node_context, text_context)
                },
            )
            .expect("taffy compute_layout failed");
    }

    /// Get the computed bounds for a node in absolute coordinates (logical pixels).
    ///
    /// Taffy returns positions relative to parent, so we walk up the tree
    /// to accumulate the absolute position.
    pub fn layout_bounds(&self, id: NodeId) -> Rect {
        let layout = self.taffy.layout(id).expect("node not found");

        // Taffy works in logical pixels
        let mut bounds = Rect::new(
            Point::new(layout.location.x, layout.location.y),
            Size::new(layout.size.width, layout.size.height),
        );

        // Walk up to parent and add its origin
        if let Some(parent_id) = self.taffy.parent(id) {
            let parent_bounds = self.layout_bounds(parent_id);
            bounds.origin.x += parent_bounds.origin.x;
            bounds.origin.y += parent_bounds.origin.y;
        }

        bounds
    }

    /// Get raw layout (relative position, device pixels).
    pub fn layout(&self, id: NodeId) -> &taffy::Layout {
        self.taffy.layout(id).expect("node not found")
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Measure function for nodes that need dynamic sizing.
fn measure_node(
    known_dimensions: taffy::Size<Option<f32>>,
    available_space: taffy::Size<AvailableSpace>,
    node_context: Option<&mut MeasureContext>,
    text_context: &mut TextContext,
) -> taffy::Size<f32> {
    // If both dimensions are already known, use them
    if let taffy::Size {
        width: Some(width),
        height: Some(height),
    } = known_dimensions
    {
        return taffy::Size { width, height };
    }

    // No context means no measurement needed
    let Some(context) = node_context else {
        return taffy::Size::ZERO;
    };

    match context {
        MeasureContext::Text { content, font_size } => {
            measure_text(known_dimensions, available_space, content, *font_size, text_context)
        }
    }
}

/// Measure text using parley.
fn measure_text(
    known_dimensions: taffy::Size<Option<f32>>,
    available_space: taffy::Size<AvailableSpace>,
    content: &str,
    font_size: f32,
    text_context: &mut TextContext,
) -> taffy::Size<f32> {
    // Determine max width for text wrapping
    let max_width = known_dimensions.width.or_else(|| match available_space.width {
        AvailableSpace::Definite(w) => Some(w),
        AvailableSpace::MaxContent => None, // No wrapping
        AvailableSpace::MinContent => Some(0.0), // Force minimum width
    });

    // Layout the text
    // TODO: Support max_width for wrapping (parley's break_all_lines takes Option<f32>)
    let _ = max_width; // Suppress unused warning for now
    let layout = text_context.layout_text(content, font_size);

    taffy::Size {
        width: known_dimensions.width.unwrap_or(layout.width()),
        height: known_dimensions.height.unwrap_or(layout.height()),
    }
}

// Re-export taffy types that users need
pub use taffy::style::{
    AlignContent, AlignItems, AlignSelf, Display, FlexDirection, FlexWrap, JustifyContent,
    Position,
};
pub use taffy::{NodeId, Style};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_layout_engine() {
        let _engine = LayoutEngine::new();
    }

    #[test]
    fn simple_fixed_layout() {
        let mut engine = LayoutEngine::new();
        let mut text_ctx = TextContext::new();

        // Create a 100x50 box
        let node = engine.new_leaf(Style {
            size: taffy::Size {
                width: length(100.0),
                height: length(50.0),
            },
            ..Default::default()
        });

        engine.compute_layout(node, 800.0, 600.0, &mut text_ctx);

        let bounds = engine.layout_bounds(node);
        assert_eq!(bounds.size.width, 100.0);
        assert_eq!(bounds.size.height, 50.0);
        assert_eq!(bounds.origin.x, 0.0);
        assert_eq!(bounds.origin.y, 0.0);
    }

    #[test]
    fn flexbox_row_layout() {
        let mut engine = LayoutEngine::new();
        let mut text_ctx = TextContext::new();

        // Two 50x50 boxes in a row
        let child1 = engine.new_leaf(Style {
            size: taffy::Size {
                width: length(50.0),
                height: length(50.0),
            },
            ..Default::default()
        });
        let child2 = engine.new_leaf(Style {
            size: taffy::Size {
                width: length(50.0),
                height: length(50.0),
            },
            ..Default::default()
        });

        let parent = engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                ..Default::default()
            },
            &[child1, child2],
        );

        engine.compute_layout(parent, 800.0, 600.0, &mut text_ctx);

        let bounds1 = engine.layout_bounds(child1);
        let bounds2 = engine.layout_bounds(child2);

        // First child at (0, 0)
        assert_eq!(bounds1.origin.x, 0.0);
        assert_eq!(bounds1.origin.y, 0.0);

        // Second child at (50, 0) - right next to first
        assert_eq!(bounds2.origin.x, 50.0);
        assert_eq!(bounds2.origin.y, 0.0);
    }

    #[test]
    fn flexbox_column_layout() {
        let mut engine = LayoutEngine::new();
        let mut text_ctx = TextContext::new();

        let child1 = engine.new_leaf(Style {
            size: taffy::Size {
                width: length(50.0),
                height: length(30.0),
            },
            ..Default::default()
        });
        let child2 = engine.new_leaf(Style {
            size: taffy::Size {
                width: length(50.0),
                height: length(30.0),
            },
            ..Default::default()
        });

        let parent = engine.new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            &[child1, child2],
        );

        engine.compute_layout(parent, 800.0, 600.0, &mut text_ctx);

        let bounds1 = engine.layout_bounds(child1);
        let bounds2 = engine.layout_bounds(child2);

        // First child at (0, 0)
        assert_eq!(bounds1.origin.x, 0.0);
        assert_eq!(bounds1.origin.y, 0.0);

        // Second child at (0, 30) - below first
        assert_eq!(bounds2.origin.x, 0.0);
        assert_eq!(bounds2.origin.y, 30.0);
    }

    #[test]
    fn text_measurement() {
        let mut engine = LayoutEngine::new();
        let mut text_ctx = TextContext::new();

        // Create a text node - size determined by content
        let text_node = engine.new_leaf_with_context(
            Style::default(),
            MeasureContext::Text {
                content: "Hello, World!".to_string(),
                font_size: 16.0,
            },
        );

        engine.compute_layout(text_node, 800.0, 600.0, &mut text_ctx);

        let bounds = engine.layout_bounds(text_node);

        // Text should have non-zero size
        assert!(bounds.size.width > 0.0, "text should have width");
        assert!(bounds.size.height > 0.0, "text should have height");
    }

    #[test]
    fn text_in_flexbox() {
        let mut engine = LayoutEngine::new();
        let mut text_ctx = TextContext::new();

        // Text node that sizes to content
        let text = engine.new_leaf_with_context(
            Style::default(),
            MeasureContext::Text {
                content: "Button".to_string(),
                font_size: 14.0,
            },
        );

        // Container with padding
        let button = engine.new_with_children(
            Style {
                display: Display::Flex,
                padding: taffy::Rect {
                    left: length(8.0),
                    right: length(8.0),
                    top: length(4.0),
                    bottom: length(4.0),
                },
                ..Default::default()
            },
            &[text],
        );

        engine.compute_layout(button, 800.0, 600.0, &mut text_ctx);

        let text_bounds = engine.layout_bounds(text);
        let button_bounds = engine.layout_bounds(button);

        // Button should be larger than text (due to padding)
        assert!(
            button_bounds.size.width > text_bounds.size.width,
            "button should be wider than text"
        );
        assert!(
            button_bounds.size.height > text_bounds.size.height,
            "button should be taller than text"
        );

        // Text should be offset by padding
        assert_eq!(text_bounds.origin.x, 8.0, "text should be offset by left padding");
        assert_eq!(text_bounds.origin.y, 4.0, "text should be offset by top padding");
    }

    #[test]
    fn nested_layout_absolute_positions() {
        let mut engine = LayoutEngine::new();
        let mut text_ctx = TextContext::new();

        let inner = engine.new_leaf(Style {
            size: taffy::Size {
                width: length(20.0),
                height: length(20.0),
            },
            ..Default::default()
        });

        let middle = engine.new_with_children(
            Style {
                padding: taffy::Rect {
                    left: length(10.0),
                    right: length(10.0),
                    top: length(10.0),
                    bottom: length(10.0),
                },
                ..Default::default()
            },
            &[inner],
        );

        let outer = engine.new_with_children(
            Style {
                padding: taffy::Rect {
                    left: length(5.0),
                    right: length(5.0),
                    top: length(5.0),
                    bottom: length(5.0),
                },
                ..Default::default()
            },
            &[middle],
        );

        engine.compute_layout(outer, 800.0, 600.0, &mut text_ctx);

        let inner_bounds = engine.layout_bounds(inner);

        // Inner should be at (5 + 10, 5 + 10) = (15, 15) absolute
        assert_eq!(inner_bounds.origin.x, 15.0);
        assert_eq!(inner_bounds.origin.y, 15.0);
    }
}
