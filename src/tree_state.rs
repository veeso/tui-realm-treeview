//! # Tree State
//!
//! This module implements the tree state.

/**
 * MIT License
 *
 * tui-realm-treeview - Copyright (C) 2021 Christian Visintin
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use super::Node;

/// ## TreeState
///
/// Tree state tracks the current state for the component tree.
#[derive(Clone)]
pub struct TreeState {
    /// Tracks open nodes
    open: Vec<String>,
    /// Current selected item
    selected: Option<String>,
}

impl Default for TreeState {
    fn default() -> Self {
        Self {
            open: Vec::default(),
            selected: None,
        }
    }
}

impl TreeState {
    /// ### is_open
    ///
    /// Returns whether `node` is open
    pub fn is_open(&self, node: &Node) -> bool {
        self.open.contains(node.id())
    }

    /// ### is_closed
    ///
    /// Returns whether `node` is closed
    pub fn is_closed(&self, node: &Node) -> bool {
        !self.is_open(node)
    }

    /// ### selected
    ///
    /// Get current selected item
    pub fn selected(&self) -> Option<&str> {
        self.selected.as_ref().map(|x| x.as_str())
    }

    /// ### is_selected
    ///
    /// Returns whether provided node is currently selected
    pub fn is_selected(&self, node: &Node) -> bool {
        self.selected
            .as_ref()
            .map(|x| x == node.id())
            .unwrap_or(false)
    }

    /// ### tree_changed
    ///
    /// The tree has changed, so this method must check whether to keep states or not
    pub fn tree_changed(&mut self, root: &Node, preserve: bool) {
        if preserve {
            // Check whether selected is still valid
            self.selected = match self.selected.take() {
                None => None,
                Some(selected) => {
                    if root.query(&selected).is_some() {
                        Some(selected)
                    } else {
                        None
                    }
                }
            };
            // Check whether open nodes still exist
            self.open.retain(|x| root.query(x).is_some());
        } else {
            // Reset state
            self.open = Vec::new();
            self.selected = None;
        }
    }

    /// ### open_node
    ///
    /// Open `node`. Node can be open only if it is closed and it is NOT a leaf
    pub fn open_node(&mut self, node: &Node) {
        if !node.is_leaf() && self.is_closed(node) {
            self.open.push(node.id().to_string());
        }
    }

    /// ### close_node
    ///
    /// Close `node`
    pub fn close_node(&mut self, node: &Node) {
        if self.is_open(node) {
            // Remove from open nodes
            self.open.retain(|x| x != node.id());
            // Remove children for node
            self.close_children(node);
        }
    }

    /// ### move_down
    ///
    /// Move cursor down in current tree from current position. Rewind if required
    pub fn move_down(&mut self, root: &Node) {
        if let Some(selected) = self.selected.take() {
            // Get current node
            if let Some(node) = root.query(&selected) {
                // If node is open, then move to its first child
                if !node.is_leaf() && self.is_open(node) {
                    // NOTE: unwrap is safe; checked by `is_leaf()`
                    self.selected = Some(node.children().first().unwrap().id().to_string());
                } else {
                    // If has a "next sibling", let's get it
                    if let Some(sibling) = self.next_sibling(root, node) {
                        self.selected = Some(sibling.id().to_string());
                    } else {
                        // Then the next element becomes the next sibling of the parent
                        // this thing has to be performed recursively for all parents, until one is found (or root is reached)
                        let mut current = selected.clone();
                        loop {
                            if let Some(parent) = root.parent(&current) {
                                current = parent.id().to_string();
                                if let Some(sibling) = self.next_sibling(root, parent) {
                                    self.selected = Some(sibling.id().to_string());
                                    break;
                                }
                            } else {
                                // has no parent, keep selectd
                                self.selected = Some(selected);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    /// ### move_up
    ///
    /// Move cursor up in current tree from current position. Rewind if required
    pub fn move_up(&mut self, root: &Node) {
        if let Some(selected) = self.selected.take() {
            // Get parent
            if let Some(parent) = root.parent(&selected) {
                // Selected becomes previous sibling; or if None, the parent
                self.selected = Some(
                    self.previous_sibling(root, root.query(&selected).unwrap())
                        .unwrap_or(parent)
                        .id()
                        .to_string(),
                );
            } else {
                // Is root; then keep selected
                self.selected = Some(selected);
            }
        }
    }

    /// ### select
    ///
    /// Set current selected node
    pub fn select(&mut self, node: &Node) {
        self.selected = Some(node.id().to_string());
    }

    // -- private

    /// ### close_children
    ///
    /// Close all node children recursively
    fn close_children(&mut self, node: &Node) {
        node.iter().for_each(|x| self.close_node(x));
    }

    /// ### previous_sibling
    ///
    /// Returns the previous sibling of `node` in `tree`
    fn previous_sibling<'a>(&mut self, root: &'a Node, node: &'a Node) -> Option<&'a Node> {
        match root.parent(node.id()) {
            None => None,
            Some(parent) => {
                let mut prev_node = None;
                for child in parent.children() {
                    if child.id() == node.id() {
                        break;
                    }
                    prev_node = Some(child);
                }
                prev_node
            }
        }
    }

    /// ### next_sibling
    ///
    /// Returs next sibling of `node` in `tree`
    fn next_sibling<'a>(&mut self, root: &'a Node, node: &'a Node) -> Option<&'a Node> {
        match root.parent(node.id()) {
            None => None,
            Some(parent) => {
                let mut keep_next = false;
                for child in parent.children() {
                    if keep_next {
                        // Return child
                        return Some(child);
                    } else if child.id() == node.id() {
                        // keep next element
                        keep_next = true;
                    }
                }
                // No next sibling
                None
            }
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;

    use pretty_assertions::assert_eq;

    // TODO: impl
}
