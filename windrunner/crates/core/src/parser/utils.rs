use crate::types::Position;
use tree_sitter::Node;

pub fn node_to_position(node: &Node, start: bool) -> Position {
    let point = if start {
        node.start_position()
    } else {
        node.end_position()
    };
    Position {
        line: point.row as u32,
        character: point.column as u32,
    }
}
