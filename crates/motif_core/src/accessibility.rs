//! Accessibility types for screen reader support via AccessKit.

use crate::Rect;
use std::collections::HashMap;

/// Unique identifier for an accessible element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccessId(pub u64);

impl From<AccessId> for accesskit::NodeId {
    fn from(id: AccessId) -> Self {
        accesskit::NodeId(id.0)
    }
}

/// Role of an accessible element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessRole {
    Button,
    Group,
    Label,
    TextInput,
    Window,
}

impl From<AccessRole> for accesskit::Role {
    fn from(role: AccessRole) -> Self {
        match role {
            AccessRole::Button => accesskit::Role::Button,
            AccessRole::Group => accesskit::Role::Group,
            AccessRole::Label => accesskit::Role::Label,
            AccessRole::TextInput => accesskit::Role::TextInput,
            AccessRole::Window => accesskit::Role::Window,
        }
    }
}

/// A node in the accessibility tree.
#[derive(Debug, Clone)]
pub struct AccessNode {
    pub id: AccessId,
    pub role: AccessRole,
    pub name: String,
    pub bounds: Option<Rect>,
    pub children: Vec<AccessId>,
}

impl AccessNode {
    pub fn new(id: AccessId, role: AccessRole, name: String) -> Self {
        Self {
            id,
            role,
            name,
            bounds: None,
            children: Vec::new(),
        }
    }

    pub fn with_bounds(mut self, bounds: Rect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn with_child(mut self, child: AccessId) -> Self {
        self.children.push(child);
        self
    }

    /// Convert to an AccessKit Node.
    pub fn to_accesskit_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(self.role.into());
        node.set_label(self.name.clone());

        if let Some(bounds) = self.bounds {
            node.set_bounds(accesskit::Rect {
                x0: bounds.origin.x as f64,
                y0: bounds.origin.y as f64,
                x1: (bounds.origin.x + bounds.size.width) as f64,
                y1: (bounds.origin.y + bounds.size.height) as f64,
            });
        }

        if !self.children.is_empty() {
            let children: Vec<accesskit::NodeId> =
                self.children.iter().map(|id| (*id).into()).collect();
            node.set_children(children);
        }

        node
    }
}

/// Container for accessibility nodes, parallel to Scene for rendering.
///
/// Build an AccessTree during drawing, then convert to AccessKit TreeUpdate.
#[derive(Debug)]
pub struct AccessTree {
    root_id: AccessId,
    nodes: HashMap<AccessId, AccessNode>,
}

impl AccessTree {
    /// Create a new accessibility tree with the given root ID.
    pub fn new(root_id: AccessId) -> Self {
        Self {
            root_id,
            nodes: HashMap::new(),
        }
    }

    /// Get the root node ID.
    pub fn root_id(&self) -> AccessId {
        self.root_id
    }

    /// Add a node to the tree.
    pub fn push(&mut self, node: AccessNode) {
        self.nodes.insert(node.id, node);
    }

    /// Get a node by ID.
    pub fn get(&self, id: AccessId) -> Option<&AccessNode> {
        self.nodes.get(&id)
    }

    /// Number of nodes in the tree.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Clear all nodes, keeping root ID.
    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    /// Build an initial TreeUpdate for AccessKit (includes Tree info).
    pub fn build_initial_update(&self, focus: Option<AccessId>) -> accesskit::TreeUpdate {
        let nodes: Vec<(accesskit::NodeId, accesskit::Node)> = self
            .nodes
            .values()
            .map(|n| (n.id.into(), n.to_accesskit_node()))
            .collect();

        accesskit::TreeUpdate {
            nodes,
            tree: Some(accesskit::Tree::new(self.root_id.into())),
            tree_id: accesskit::TreeId::ROOT,
            focus: focus.map(|id| id.into()).unwrap_or(self.root_id.into()),
        }
    }
}

/// Manages keyboard focus for accessible elements.
#[derive(Debug, Default)]
pub struct FocusManager {
    /// Currently focused element, if any.
    focused: Option<AccessId>,
    /// Ordered list of focusable elements (tab order).
    focus_order: Vec<AccessId>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the currently focused element.
    pub fn focused(&self) -> Option<AccessId> {
        self.focused
    }

    /// Set focus to a specific element.
    pub fn set_focus(&mut self, id: AccessId) {
        self.focused = Some(id);
    }

    /// Clear focus.
    pub fn clear_focus(&mut self) {
        self.focused = None;
    }

    /// Set the focus order (tab order).
    pub fn set_focus_order(&mut self, order: Vec<AccessId>) {
        self.focus_order = order;
    }

    /// Move focus to the next element in the focus order.
    pub fn focus_next(&mut self) {
        if self.focus_order.is_empty() {
            return;
        }

        let current_idx = self
            .focused
            .and_then(|id| self.focus_order.iter().position(|&fid| fid == id));

        let next_idx = match current_idx {
            Some(idx) => (idx + 1) % self.focus_order.len(),
            None => 0,
        };

        self.focused = Some(self.focus_order[next_idx]);
    }

    /// Move focus to the previous element in the focus order.
    pub fn focus_prev(&mut self) {
        if self.focus_order.is_empty() {
            return;
        }

        let current_idx = self
            .focused
            .and_then(|id| self.focus_order.iter().position(|&fid| fid == id));

        let prev_idx = match current_idx {
            Some(idx) => {
                if idx == 0 {
                    self.focus_order.len() - 1
                } else {
                    idx - 1
                }
            }
            None => self.focus_order.len() - 1,
        };

        self.focused = Some(self.focus_order[prev_idx]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Point, Rect, Size};

    #[test]
    fn access_node_basic_properties() {
        let node = AccessNode::new(
            AccessId(1),
            AccessRole::Button,
            "Submit".to_string(),
        );

        assert_eq!(node.id, AccessId(1));
        assert_eq!(node.role, AccessRole::Button);
        assert_eq!(node.name, "Submit");
    }

    #[test]
    fn access_node_with_bounds() {
        let bounds = Rect::new(Point::new(10.0, 20.0), Size::new(100.0, 50.0));
        let node = AccessNode::new(AccessId(1), AccessRole::Button, "Click me".to_string())
            .with_bounds(bounds);

        assert_eq!(node.bounds, Some(bounds));
    }

    #[test]
    fn access_node_with_children() {
        let parent = AccessNode::new(AccessId(1), AccessRole::Group, "Container".to_string())
            .with_child(AccessId(2))
            .with_child(AccessId(3));

        assert_eq!(parent.children, vec![AccessId(2), AccessId(3)]);
    }

    #[test]
    fn convert_to_accesskit_node() {
        let bounds = Rect::new(Point::new(10.0, 20.0), Size::new(100.0, 50.0));
        let node = AccessNode::new(AccessId(1), AccessRole::Button, "Submit".to_string())
            .with_bounds(bounds);

        let ak_node = node.to_accesskit_node();

        assert_eq!(ak_node.role(), accesskit::Role::Button);
        assert_eq!(ak_node.label(), Some("Submit"));
        // Bounds should be converted
        let ak_bounds = ak_node.bounds().expect("should have bounds");
        assert_eq!(ak_bounds.x0, 10.0);
        assert_eq!(ak_bounds.y0, 20.0);
        assert_eq!(ak_bounds.x1, 110.0); // x + width
        assert_eq!(ak_bounds.y1, 70.0);  // y + height
    }

    // AccessTree tests

    #[test]
    fn access_tree_new_has_root() {
        let tree = AccessTree::new(AccessId(1));
        assert_eq!(tree.root_id(), AccessId(1));
    }

    #[test]
    fn access_tree_push_and_get_nodes() {
        let mut tree = AccessTree::new(AccessId(1));
        tree.push(AccessNode::new(AccessId(1), AccessRole::Window, "App".to_string()));
        tree.push(AccessNode::new(AccessId(2), AccessRole::Button, "OK".to_string()));

        assert_eq!(tree.node_count(), 2);
        assert_eq!(tree.get(AccessId(2)).map(|n| n.name.as_str()), Some("OK"));
    }

    #[test]
    fn access_tree_generates_initial_tree_update() {
        let mut tree = AccessTree::new(AccessId(1));
        tree.push(
            AccessNode::new(AccessId(1), AccessRole::Window, "App".to_string())
                .with_child(AccessId(2)),
        );
        tree.push(AccessNode::new(AccessId(2), AccessRole::Button, "OK".to_string()));

        let update = tree.build_initial_update(None);

        // Should have tree info set
        assert!(update.tree.is_some());
        // Should have both nodes
        assert_eq!(update.nodes.len(), 2);
    }

    #[test]
    fn access_tree_clear_resets() {
        let mut tree = AccessTree::new(AccessId(1));
        tree.push(AccessNode::new(AccessId(1), AccessRole::Window, "App".to_string()));
        tree.push(AccessNode::new(AccessId(2), AccessRole::Button, "OK".to_string()));

        tree.clear();

        assert_eq!(tree.node_count(), 0);
    }

    // FocusManager tests

    #[test]
    fn focus_manager_starts_with_no_focus() {
        let fm = FocusManager::new();
        assert!(fm.focused().is_none());
    }

    #[test]
    fn focus_manager_set_and_get_focus() {
        let mut fm = FocusManager::new();
        fm.set_focus(AccessId(5));
        assert_eq!(fm.focused(), Some(AccessId(5)));
    }

    #[test]
    fn focus_manager_clear_focus() {
        let mut fm = FocusManager::new();
        fm.set_focus(AccessId(5));
        fm.clear_focus();
        assert!(fm.focused().is_none());
    }

    #[test]
    fn focus_manager_focus_next_cycles() {
        let mut fm = FocusManager::new();
        fm.set_focus_order(vec![AccessId(1), AccessId(2), AccessId(3)]);
        fm.set_focus(AccessId(1));

        fm.focus_next();
        assert_eq!(fm.focused(), Some(AccessId(2)));

        fm.focus_next();
        assert_eq!(fm.focused(), Some(AccessId(3)));

        fm.focus_next(); // wrap around
        assert_eq!(fm.focused(), Some(AccessId(1)));
    }

    #[test]
    fn focus_manager_focus_prev_cycles() {
        let mut fm = FocusManager::new();
        fm.set_focus_order(vec![AccessId(1), AccessId(2), AccessId(3)]);
        fm.set_focus(AccessId(1));

        fm.focus_prev(); // wrap around backwards
        assert_eq!(fm.focused(), Some(AccessId(3)));

        fm.focus_prev();
        assert_eq!(fm.focused(), Some(AccessId(2)));
    }
}
