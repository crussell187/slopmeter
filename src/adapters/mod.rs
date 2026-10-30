pub mod cpp;
pub mod python;
pub mod rust;

use std::collections::BTreeSet;

use tree_sitter::Node;

use crate::ir::{File, Language};

pub(crate) struct ParseInput<'a> {
    pub(crate) path: &'a str,
    pub(crate) source: &'a [u8],
    pub(crate) include_tests: bool,
}

pub(crate) fn unparsed_ratio(root: Node, source: &[u8]) -> f64 {
    let lines = line_count(source);
    if lines == 0 {
        return 0.0;
    }
    let mut bad = BTreeSet::new();
    collect_errors(root, &mut bad);
    bad.len() as f64 / f64::from(lines)
}

fn collect_errors(node: Node, lines: &mut BTreeSet<u32>) {
    if error_node(node) {
        mark_error(node, lines);
    }
    walk_children(node, lines);
}

fn error_node(node: Node) -> bool {
    let kind = node.kind();
    kind == "ERROR" || node.is_missing()
}

fn mark_error(node: Node, lines: &mut BTreeSet<u32>) {
    if node.child_count() == 0 {
        mark_lines(node, lines);
        return;
    }
    mark_kept_children(node, lines);
}

fn mark_kept_children(node: Node, lines: &mut BTreeSet<u32>) {
    let count = node.child_count();
    for index in 0..count {
        mark_kept_child(node, index, lines);
    }
}

fn mark_kept_child(node: Node, index: u32, lines: &mut BTreeSet<u32>) {
    let Some(child) = node.child(index) else {
        return;
    };
    if kept_child(child) {
        mark_lines(child, lines);
    }
}

fn kept_child(child: Node) -> bool {
    if recovered(child) {
        return false;
    }
    let kind = child.kind();
    kind != "ERROR" && !child.is_missing()
}

fn walk_children(node: Node, lines: &mut BTreeSet<u32>) {
    let count = node.child_count();
    for index in 0..count {
        walk_child(node, index, lines);
    }
}

fn walk_child(node: Node, index: u32, lines: &mut BTreeSet<u32>) {
    if let Some(child) = node.child(index) {
        collect_errors(child, lines);
    }
}

fn recovered(node: Node) -> bool {
    node.is_named() && node.child_count() > 0 && node.kind() != "ERROR" && !node.is_missing()
}

fn mark_lines(node: Node, lines: &mut BTreeSet<u32>) {
    let (start, end) = span_of(node);
    for line in start..=end {
        lines.insert(line);
    }
}

fn span_of(node: Node) -> (u32, u32) {
    let start_point = node.start_position();
    let start = start_point.row as u32 + 1;
    let end_point = node.end_position();
    let end = end_row(end_point, start_point.row);
    (start, end.max(start))
}

fn end_row(end_point: tree_sitter::Point, start_row: usize) -> u32 {
    let column = end_point.column;
    let row = end_point.row;
    if column == 0 && row > start_row {
        return row as u32;
    }
    row as u32 + 1
}

fn line_count(source: &[u8]) -> u32 {
    if source.is_empty() {
        return 0;
    }
    let newlines = source.iter().filter(|byte| **byte == b'\n').count() as u32;
    if source.last() == Some(&b'\n') {
        newlines
    } else {
        newlines + 1
    }
}

pub(crate) fn parse(language: Language, file: ParseInput<'_>) -> Result<File, String> {
    match language {
        Language::Python => Ok(python::parse(file.path, file.source)),
        Language::Rust => Ok(rust::parse_with(file.path, file.source, file.include_tests)),
        Language::Cpp => Ok(cpp::parse(file.path, file.source)),
    }
}

pub fn allow(language: Language) -> &'static [&'static str] {
    match language {
        Language::Python => python::ALLOW,
        Language::Rust => rust::ALLOW,
        Language::Cpp => cpp::ALLOW,
    }
}
