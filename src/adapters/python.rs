use std::borrow::Cow;
use std::collections::BTreeSet;

use tree_sitter::Node;

use crate::ir::{Body, File, Function, Language, Param, Span};

pub const ALLOW: &[&str] = &[
    "get",
    "keys",
    "values",
    "items",
    "append",
    "extend",
    "add",
    "update",
    "pop",
    "split",
    "rsplit",
    "strip",
    "lstrip",
    "rstrip",
    "join",
    "lower",
    "upper",
    "startswith",
    "endswith",
    "encode",
    "decode",
    "format",
    "copy",
];

enum Scope {
    Class(String),
    Function(String),
}

pub fn parse(path: &str, source: &[u8]) -> File {
    let decoded = decode_source(source);
    let source = decoded.as_ref();
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .expect("python grammar");
    let Some(tree) = parser.parse(source, None) else {
        return File {
            path: path.to_string(),
            language: Language::Python,
            unparsed_ratio: 1.0,
            functions: Vec::new(),
        };
    };
    let root = tree.root_node();
    let mut functions = Vec::new();
    let mut scopes = Vec::new();
    visit(root, source, &mut scopes, &mut functions);
    File {
        path: path.to_string(),
        language: Language::Python,
        unparsed_ratio: unparsed_ratio(root, source),
        functions,
    }
}

fn visit(node: Node, source: &[u8], scopes: &mut Vec<Scope>, out: &mut Vec<Function>) {
    match node.kind() {
        "function_definition" => visit_function(node, source, scopes, out),
        "class_definition" => visit_class(node, source, scopes, out),
        "assignment" => visit_assignment(node, source, scopes, out),
        _ => visit_children(node, source, scopes, out),
    }
}

fn visit_children(node: Node, source: &[u8], scopes: &mut Vec<Scope>, out: &mut Vec<Function>) {
    for child in named_nodes(node) {
        visit(child, source, scopes, out);
    }
}

fn visit_function(node: Node, source: &[u8], scopes: &mut Vec<Scope>, out: &mut Vec<Function>) {
    if is_overload(node, source) {
        return;
    }
    let Some(name_node) = node.child_by_field_name("name") else {
        return;
    };
    let name = node_text(name_node, source);
    let receiver = directly_in_class(scopes) && !is_staticmethod(node, source);
    let touch_start = decorator_nodes(node)
        .first()
        .map(|decorator| span_of(*decorator).start);
    push_function(
        out,
        scopes,
        source,
        Found {
            name: &name,
            node,
            touch_start,
            params: node.child_by_field_name("parameters"),
            receiver,
        },
    );
    scopes.push(Scope::Function(name));
    if let Some(body) = node.child_by_field_name("body") {
        visit_children(body, source, scopes, out);
    }
    scopes.pop();
}

fn visit_class(node: Node, source: &[u8], scopes: &mut Vec<Scope>, out: &mut Vec<Function>) {
    let Some(name_node) = node.child_by_field_name("name") else {
        return;
    };
    scopes.push(Scope::Class(node_text(name_node, source)));
    if let Some(body) = node.child_by_field_name("body") {
        visit_children(body, source, scopes, out);
    }
    scopes.pop();
}

fn visit_assignment(node: Node, source: &[u8], scopes: &[Scope], out: &mut Vec<Function>) {
    if in_function(scopes) {
        return;
    }
    let Some(right) = node.child_by_field_name("right") else {
        return;
    };
    let Some(lambda) = lambda_of(right) else {
        return;
    };
    let Some(left) = node.child_by_field_name("left") else {
        return;
    };
    if left.kind() != "identifier" {
        return;
    }
    let receiver = directly_in_class(scopes);
    let name = node_text(left, source);
    push_function(
        out,
        scopes,
        source,
        Found {
            name: &name,
            node: lambda,
            touch_start: None,
            params: lambda.child_by_field_name("parameters"),
            receiver,
        },
    );
}

struct Found<'a> {
    name: &'a str,
    node: Node<'a>,
    touch_start: Option<u32>,
    params: Option<Node<'a>>,
    receiver: bool,
}

fn push_function(out: &mut Vec<Function>, scopes: &[Scope], source: &[u8], found: Found<'_>) {
    let span = span_of(found.node);
    let mut touch = span;
    if let Some(start) = found.touch_start {
        touch.start = start;
    }
    let params = found
        .params
        .map(|node| parameter_names(node, source))
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(index, name)| Param {
            name,
            receiver: found.receiver && index == 0,
        })
        .collect();
    out.push(Function {
        name: qualify(scopes, found.name),
        param_types: Vec::new(),
        qualifier: String::new(),
        span,
        touch,
        params,
        body: Body {
            lloc: 0,
            decisions: Vec::new(),
            cognitive: Vec::new(),
            chains: Vec::new(),
            forward: None,
        },
    });
}

fn qualify(scopes: &[Scope], name: &str) -> String {
    let mut parts = Vec::new();
    for scope in scopes {
        match scope {
            Scope::Class(scope_name) | Scope::Function(scope_name) => {
                parts.push(scope_name.as_str())
            }
        }
    }
    parts.push(name);
    parts.join(".")
}

fn directly_in_class(scopes: &[Scope]) -> bool {
    matches!(scopes.last(), Some(Scope::Class(_)))
}

fn in_function(scopes: &[Scope]) -> bool {
    scopes
        .iter()
        .any(|scope| matches!(scope, Scope::Function(_)))
}

fn parameter_names(node: Node, source: &[u8]) -> Vec<String> {
    named_nodes(node)
        .into_iter()
        .filter_map(|child| parameter_name(child, source))
        .collect()
}

fn parameter_name(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => Some(node_text(node, source)),
        "default_parameter" | "typed_default_parameter" => node
            .child_by_field_name("name")
            .and_then(|name| parameter_name(name, source)),
        "typed_parameter" | "list_splat_pattern" | "dictionary_splat_pattern" => named_nodes(node)
            .into_iter()
            .find(|child| child.kind() != "type")
            .and_then(|child| parameter_name(child, source)),
        _ => None,
    }
}

fn lambda_of(node: Node) -> Option<Node> {
    match node.kind() {
        "lambda" => Some(node),
        "parenthesized_expression" => named_nodes(node).into_iter().find_map(lambda_of),
        _ => None,
    }
}

fn decorator_nodes(func: Node<'_>) -> Vec<Node<'_>> {
    let Some(parent) = func.parent() else {
        return Vec::new();
    };
    if parent.kind() != "decorated_definition" {
        return Vec::new();
    }
    named_nodes(parent)
        .into_iter()
        .filter(|child| child.kind() == "decorator")
        .collect()
}

fn is_overload(func: Node, source: &[u8]) -> bool {
    decorator_names(func, source)
        .iter()
        .any(|name| name == "overload" || name.ends_with(".overload"))
}

fn is_staticmethod(func: Node, source: &[u8]) -> bool {
    decorator_names(func, source)
        .iter()
        .any(|name| name == "staticmethod" || name.ends_with(".staticmethod"))
}

fn decorator_names(func: Node, source: &[u8]) -> Vec<String> {
    decorator_nodes(func)
        .into_iter()
        .map(|decorator| decorator_name(decorator, source))
        .collect()
}

fn decorator_name(node: Node, source: &[u8]) -> String {
    let raw = node_text(node, source);
    let raw = raw.trim().trim_start_matches('@').trim();
    let raw = raw.split('(').next().unwrap_or(raw).trim();
    raw.split_whitespace().next().unwrap_or("").to_string()
}

fn span_of(node: Node) -> Span {
    let start = node.start_position().row as u32 + 1;
    let end_point = node.end_position();
    let end = if end_point.column == 0 && end_point.row > node.start_position().row {
        end_point.row as u32
    } else {
        end_point.row as u32 + 1
    };
    Span {
        start,
        end: end.max(start),
    }
}

fn unparsed_ratio(root: Node, source: &[u8]) -> f64 {
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
        let span = span_of(node);
        for line in span.start..=span.end {
            lines.insert(line);
        }
    }
    for index in 0..node.child_count() {
        if let Some(child) = node.child(index) {
            collect_errors(child, lines);
        }
    }
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

fn named_nodes<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    let mut children = Vec::new();
    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index as u32) {
            children.push(child);
        }
    }
    children
}

fn node_text(node: Node, source: &[u8]) -> String {
    let range = node.byte_range();
    String::from_utf8_lossy(&source[range]).into_owned()
}

fn decode_source(source: &[u8]) -> Cow<'_, [u8]> {
    if latin1_cookie(source) {
        Cow::Owned(latin1_to_utf8(source))
    } else {
        Cow::Borrowed(source)
    }
}

fn latin1_cookie(source: &[u8]) -> bool {
    let header = String::from_utf8_lossy(source);
    header.lines().take(2).any(|line| {
        matches!(
            coding_name(line).map(|name| name.to_ascii_lowercase()),
            Some(name)
                if matches!(
                    name.as_str(),
                    "latin-1" | "latin1" | "iso-8859-1" | "iso8859-1" | "l1" | "cp819" | "csisolatin1"
                )
        )
    })
}

fn coding_name(line: &str) -> Option<&str> {
    let rest = line.split_once("coding")?.1.trim_start();
    let rest = rest.strip_prefix([':', '='])?.trim_start();
    let end = rest
        .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.'))
        .unwrap_or(rest.len());
    let name = &rest[..end];
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

fn latin1_to_utf8(source: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(source.len());
    for &byte in source {
        if byte < 0x80 {
            out.push(byte);
        } else {
            out.push(0xC0 | (byte >> 6));
            out.push(0x80 | (byte & 0x3F));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Language;

    #[test]
    fn a_nested_def_is_its_own_function_and_an_inner_lambda_is_not() {
        let source = "\
@deco
def outer(a, b=1, *args, **kwargs):
    \"\"\"doc\"\"\"
    def inner(x):
        return lambda y: y
    return inner

bound = lambda z: z
";
        let file = parse("src/sample.py", source.as_bytes());
        let names: Vec<_> = file.functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["outer", "outer.inner", "bound"]);

        let outer = &file.functions[0];
        assert_eq!(outer.span, crate::ir::Span { start: 2, end: 6 });
        assert_eq!(outer.touch, crate::ir::Span { start: 1, end: 6 });
        assert_eq!(
            outer
                .params
                .iter()
                .map(|p| (p.name.as_str(), p.receiver))
                .collect::<Vec<_>>(),
            vec![
                ("a", false),
                ("b", false),
                ("args", false),
                ("kwargs", false)
            ]
        );
        assert!(outer.param_types.is_empty());
        assert!(outer.qualifier.is_empty());

        let inner = &file.functions[1];
        assert_eq!(inner.span, crate::ir::Span { start: 4, end: 5 });
        assert_eq!(inner.touch, inner.span);
        assert_eq!(inner.params.len(), 1);
        assert!(!inner.params[0].receiver);

        let bound = &file.functions[2];
        assert_eq!(bound.span.start, 8);
        assert_eq!(bound.params[0].name, "z");
        assert!(!bound.params[0].receiver);
        assert_eq!(file.language, Language::Python);
        assert_eq!(file.path, "src/sample.py");
        assert_eq!(file.unparsed_ratio, 0.0);
    }

    #[test]
    fn methods_take_a_receiver_and_a_staticmethod_does_not() {
        let source = "\
class Response:
    def json(self, url):
        return url

    @staticmethod
    def bare(url):
        return url

    @classmethod
    def make(cls, url):
        return url

    def wrap(self):
        def helper(x):
            return x
        return helper
";
        let file = parse("api.py", source.as_bytes());
        let names: Vec<_> = file.functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "Response.json",
                "Response.bare",
                "Response.make",
                "Response.wrap",
                "Response.wrap.helper",
            ]
        );
        assert!(file.functions[0].params[0].receiver);
        assert!(!file.functions[0].params[1].receiver);
        assert!(!file.functions[1].params[0].receiver);
        assert_eq!(file.functions[1].touch.start, 5);
        assert_eq!(file.functions[1].span.start, 6);
        assert!(file.functions[2].params[0].receiver);
        assert!(file.functions[3].params[0].receiver);
        assert!(!file.functions[4].params[0].receiver);
    }

    #[test]
    fn an_overload_signature_is_not_a_function() {
        let source = "\
@overload
def foo(x: int):
    ...

def foo(x):
    return x
";
        let file = parse("api.py", source.as_bytes());
        assert_eq!(file.functions.len(), 1);
        assert_eq!(file.functions[0].name, "foo");
        assert_eq!(file.functions[0].span.start, 5);
    }

    #[test]
    fn a_bare_lambda_is_not_a_function_and_a_broken_file_still_parses() {
        let bare = parse("a.py", b"(lambda x: x)(1)\n");
        assert!(bare.functions.is_empty());
        assert_eq!(bare.unparsed_ratio, 0.0);

        let empty = parse("a.py", b"");
        assert!(empty.functions.is_empty());
        assert_eq!(empty.unparsed_ratio, 0.0);

        let broken = parse("a.py", b"def f(\n");
        assert!(broken.unparsed_ratio > 0.0);
        assert_eq!(broken.language, Language::Python);
    }

    #[test]
    fn a_latin1_cookie_decodes_the_function_name() {
        let mut source = b"# -*- coding: latin-1 -*-\n".to_vec();
        source.extend_from_slice(b"def caf");
        source.push(0xE9);
        source.extend_from_slice(b"():\n    pass\n");
        let file = parse("a.py", &source);
        assert_eq!(file.functions.len(), 1);
        assert_eq!(file.functions[0].name, "caf\u{e9}");
    }

    #[test]
    fn the_default_allow_list_is_the_python_member_names() {
        assert!(ALLOW.contains(&"get"));
        assert!(ALLOW.contains(&"join"));
        assert!(ALLOW.contains(&"copy"));
        assert_eq!(ALLOW.len(), 23);
    }
}
