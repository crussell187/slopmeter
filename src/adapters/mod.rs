pub mod cpp;
pub mod python;
pub mod rust;

use std::collections::BTreeSet;

use tree_sitter::Node;

use crate::ir::{File, Language};

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
    if node.kind() == "ERROR" || node.is_missing() {
        if node.child_count() == 0 {
            mark_lines(node, lines);
        } else {
            for index in 0..node.child_count() {
                if let Some(child) = node.child(index) {
                    if !recovered(child) && child.kind() != "ERROR" && !child.is_missing() {
                        mark_lines(child, lines);
                    }
                }
            }
        }
    }
    for index in 0..node.child_count() {
        if let Some(child) = node.child(index) {
            collect_errors(child, lines);
        }
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
    let start = node.start_position().row as u32 + 1;
    let end_point = node.end_position();
    let end = if end_point.column == 0 && end_point.row > node.start_position().row {
        end_point.row as u32
    } else {
        end_point.row as u32 + 1
    };
    (start, end.max(start))
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

pub fn parse(
    language: Language,
    path: &str,
    source: &[u8],
    include_tests: bool,
) -> Result<File, String> {
    match language {
        Language::Python => Ok(python::parse(path, source)),
        Language::Rust => Ok(rust::parse_with(path, source, include_tests)),
        Language::Cpp => Ok(cpp::parse(path, source)),
    }
}

pub fn allow(language: Language) -> &'static [&'static str] {
    match language {
        Language::Python => python::ALLOW,
        Language::Rust => rust::ALLOW,
        Language::Cpp => cpp::ALLOW,
    }
}
