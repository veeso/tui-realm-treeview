//! # Widget
//!
//! This module implements the tui widget for rendering a treeview

use tuirealm::ratatui::buffer::Buffer;
use tuirealm::ratatui::layout::Rect;
use tuirealm::ratatui::style::Style;
use tuirealm::ratatui::widgets::{Block, StatefulWidget, Widget};
use unicode_width::UnicodeWidthStr;

use super::{Node, NodeValue, Tree, TreeState};

/// tui-rs widget implementation of a [`crate::TreeView`]
pub struct TreeWidget<'a, V: NodeValue> {
    /// Block properties
    block: Option<Block<'a>>,
    /// Style for tree
    style: Style,
    /// Highlight style
    highlight_style: Style,
    /// Symbol to display on the side of the current highlighted
    highlight_symbol: Option<&'a str>,
    /// Spaces to use for indentation
    indent_size: usize,
    /// [`Tree`] to render
    tree: &'a Tree<V>,
}

impl<'a, V: NodeValue> TreeWidget<'a, V> {
    /// Setup a new [`TreeWidget`]
    pub fn new(tree: &'a Tree<V>) -> Self {
        Self {
            block: None,
            style: Style::default(),
            highlight_style: Style::default(),
            highlight_symbol: None,
            indent_size: 4,
            tree,
        }
    }

    /// Set block to render around the tree view
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set style for tree view
    pub fn style(mut self, s: Style) -> Self {
        self.style = s;
        self
    }

    /// Set highlighted entry style
    pub fn highlight_style(mut self, s: Style) -> Self {
        self.highlight_style = s;
        self
    }

    /// Set symbol to prepend to highlighted entry
    pub fn highlight_symbol(mut self, s: &'a str) -> Self {
        self.highlight_symbol = Some(s);
        self
    }

    /// Size for indentation
    pub fn indent_size(mut self, sz: usize) -> Self {
        self.indent_size = sz;
        self
    }
}

struct Render {
    depth: usize,
    skip_rows: usize,
}

impl<V: NodeValue> Widget for TreeWidget<'_, V> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut state = TreeState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}

impl<V: NodeValue> StatefulWidget for TreeWidget<'_, V> {
    type State = TreeState;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Set style for area
        buf.set_style(area, self.style);
        // Build block
        let area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };
        // Return if too small
        if area.width < 1 || area.height < 1 {
            return;
        }
        // Recurse render
        let mut render = Render {
            depth: 1,
            skip_rows: self.calc_rows_to_skip(state, area.height),
        };
        self.iter_nodes(self.tree.root(), area, buf, state, &mut render);
    }
}

impl<V: NodeValue> TreeWidget<'_, V> {
    fn iter_nodes(
        &self,
        node: &Node<V>,
        mut area: Rect,
        buf: &mut Buffer,
        state: &TreeState,
        render: &mut Render,
    ) -> Rect {
        // Render self
        area = self.render_node(node, area, buf, state, render);
        // Render children if node is open
        if state.is_open(node) {
            // Increment depth
            render.depth += 1;
            for child in node.iter() {
                if area.height == 0 {
                    break;
                }
                area = self.iter_nodes(child, area, buf, state, render);
            }
            // Decrement depth
            render.depth -= 1;
        }
        area
    }

    fn render_node(
        &self,
        node: &Node<V>,
        area: Rect,
        buf: &mut Buffer,
        state: &TreeState,
        render: &mut Render,
    ) -> Rect {
        // If row should skip, then skip
        if render.skip_rows > 0 {
            render.skip_rows -= 1;
            return area;
        }
        let highlight_symbol = match state.is_selected(node) {
            true => Some(self.highlight_symbol.unwrap_or_default()),
            false => None,
        };
        // Get area for current node
        let node_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        // Get style to use
        let style = match state.is_selected(node) {
            false => self.style,
            true => self.highlight_style,
        };
        // Apply style
        buf.set_style(node_area, style);
        // Calc depth for node (is selected?)
        let indent_size = render.depth * self.indent_size;
        let indent_size = match state.is_selected(node) {
            true if highlight_symbol.is_some() => {
                indent_size.saturating_sub(highlight_symbol.unwrap().width() + 1)
            }
            _ => indent_size,
        };
        let width: usize = (area.width + area.x) as usize;
        // Write indentation
        let (start_x, start_y) = buf.set_stringn(
            area.x,
            area.y,
            " ".repeat(indent_size),
            width - indent_size,
            style,
        );
        // Write highlight symbol
        let (start_x, start_y) = highlight_symbol
            .map(|x| buf.set_stringn(start_x, start_y, x, width - start_x as usize, style))
            .map(|(x, y)| buf.set_stringn(x, y, " ", width - start_x as usize, style))
            .unwrap_or((start_x, start_y));

        let mut start_x = start_x;
        let mut start_y = start_y;
        for (text, part_style) in node.value().render_parts_iter() {
            let part_style = part_style.unwrap_or(style);
            // Write node name
            (start_x, start_y) =
                buf.set_stringn(start_x, start_y, text, width - start_x as usize, part_style);
        }
        // Write arrow based on node
        let write_after = if state.is_open(node) {
            // Is open
            " \u{25bc}" // Arrow down
        } else if node.is_leaf() {
            // Is leaf (has no children)
            "  "
        } else {
            // Has children, but is closed
            " \u{25b6}" // Arrow to right
        };
        let _ = buf.set_stringn(
            start_x,
            start_y,
            write_after,
            width - start_x as usize,
            style,
        );
        // Return new area
        Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: area.height - 1,
        }
    }

    /// Calculate rows to skip before starting rendering the current tree
    fn calc_rows_to_skip(&self, state: &TreeState, height: u16) -> usize {
        // if no node is selected, return 0
        let selected = match state.selected() {
            Some(s) => s,
            None => return 0,
        };

        /// Recursive visit each node (excluding closed ones) and calculate full size and index of selected node
        fn visit_nodes<V: NodeValue>(
            node: &Node<V>,
            state: &TreeState,
            selected: &str,
            selected_idx: &mut usize,
            size: &mut usize,
        ) {
            *size += 1;
            if node.id().as_str() == selected {
                *selected_idx = *size;
            }

            if !state.is_closed(node) {
                for child in node.iter() {
                    visit_nodes(child, state, selected, selected_idx, size);
                }
            }
        }

        let selected_idx: &mut usize = &mut 0;
        let size = &mut 0;
        visit_nodes(self.tree.root(), state, selected, selected_idx, size);

        let render_area_h = height as usize;
        let num_lines_to_show_at_top = render_area_h / 2;
        let offset_max = (*size).saturating_sub(render_area_h);
        (*selected_idx)
            .saturating_sub(num_lines_to_show_at_top)
            .min(offset_max)
    }
}

#[cfg(test)]
mod test {

    use pretty_assertions::assert_eq;
    use tuirealm::ratatui::Terminal;
    use tuirealm::ratatui::backend::TestBackend;
    use tuirealm::ratatui::layout::{Constraint, Direction as LayoutDirection, Layout};
    use tuirealm::ratatui::style::Color;

    use super::*;
    use crate::mock::mock_tree;

    #[test]
    fn should_construct_default_widget() {
        let tree = mock_tree();
        let widget = TreeWidget::new(&tree);
        assert_eq!(widget.block, None);
        assert_eq!(widget.highlight_style, Style::default());
        assert_eq!(widget.highlight_symbol, None);
        assert_eq!(widget.indent_size, 4);
        assert_eq!(widget.style, Style::default());
    }

    #[test]
    fn should_construct_widget() {
        let tree = mock_tree();
        let widget = TreeWidget::new(&tree)
            .block(Block::default())
            .highlight_style(Style::default().fg(Color::Red))
            .highlight_symbol(">")
            .indent_size(8)
            .style(Style::default().fg(Color::LightRed));
        assert!(widget.block.is_some());
        assert_eq!(widget.highlight_style.fg.unwrap(), Color::Red);
        assert_eq!(widget.indent_size, 8);
        assert_eq!(widget.highlight_symbol.unwrap(), ">");
        assert_eq!(widget.style.fg.unwrap(), Color::LightRed);
    }

    #[test]
    fn should_have_no_row_to_skip_when_in_first_height_elements() {
        let tree = mock_tree();
        let mut state = TreeState::default();
        // Select aA2
        let aa2 = tree.root().query(&String::from("aA2")).unwrap();
        state.select(tree.root(), aa2);
        // Get rows to skip (no block)
        let widget = TreeWidget::new(&tree);
        // Before end
        assert_eq!(widget.calc_rows_to_skip(&state, 8), 2);
        // At end
        assert_eq!(widget.calc_rows_to_skip(&state, 6), 3);
    }

    #[test]
    fn should_have_rows_to_skip_when_out_of_viewport() {
        let tree = mock_tree();
        let mut state = TreeState::default();
        // Open all previous nodes
        state.force_open(&["/", "a", "aA", "aB", "aC", "b", "bA", "bB"]);
        // Select bB2
        let bb2 = tree.root().query(&String::from("bB2")).unwrap();
        state.select(tree.root(), bb2);
        // Get rows to skip (no block)
        let widget = TreeWidget::new(&tree);
        // 20th element - height (12) + 1
        assert_eq!(widget.calc_rows_to_skip(&state, 8), 17);
    }

    #[test]
    fn should_not_panic_per_layout_direction() {
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
        let tree = mock_tree();
        let constraints = [[50, 50], [100, 0]];
        for direction in [LayoutDirection::Vertical, LayoutDirection::Horizontal] {
            for constraint in constraints {
                let widget = TreeWidget::new(&tree);
                terminal
                    .draw(|frame| {
                        let layout = Layout::default()
                            .direction(direction)
                            .constraints(Constraint::from_percentages(constraint))
                            .split(frame.area());
                        frame.render_widget(widget, layout[1])
                    })
                    .unwrap();
            }
        }
    }

    #[test]
    fn should_not_panic_when_layout_nested() {
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
        let tree = mock_tree();
        let constraints = [[50, 50], [100, 0]];
        let directions = [LayoutDirection::Vertical, LayoutDirection::Horizontal];
        for outer_direction in directions {
            for inner_direction in directions {
                for outer_constraint in constraints {
                    for inner_constraint in constraints {
                        let widget = TreeWidget::new(&tree);
                        terminal
                            .draw(|frame| {
                                let layout = Layout::default()
                                    .direction(outer_direction)
                                    .constraints(Constraint::from_percentages(outer_constraint))
                                    .split(frame.area());
                                let nested_layout = Layout::default()
                                    .direction(inner_direction)
                                    .constraints(inner_constraint)
                                    .split(layout[1]);
                                frame.render_widget(widget, nested_layout[1])
                            })
                            .unwrap();
                    }
                }
            }
        }
    }
}
