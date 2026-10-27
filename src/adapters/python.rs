use std::borrow::Cow;
use std::collections::BTreeSet;

use tree_sitter::Node;

use crate::ir::{
    Body, Chain, Cognitive, CognitiveKind, Decision, File, Forward, Function, Language, Member,
    Param, Span,
};

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
    let params: Vec<Param> = found
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
    let ordinary: Vec<String> = params
        .iter()
        .filter(|param| !param.receiver)
        .map(|param| param.name.clone())
        .collect();
    let receiver = params
        .iter()
        .find(|param| param.receiver)
        .map(|param| param.name.clone());
    let body = classify(found.node, source, receiver.as_deref(), &ordinary);
    out.push(Function {
        name: qualify(scopes, found.name),
        param_types: Vec::new(),
        qualifier: String::new(),
        span,
        touch,
        params,
        body,
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

fn classify(node: Node, source: &[u8], receiver: Option<&str>, ordinary: &[String]) -> Body {
    let mut found = Classifier {
        source,
        receiver,
        skip: None,
        lloc: 0,
        decisions: Vec::new(),
        cognitive: Vec::new(),
        chains: Vec::new(),
    };
    let forward = found.forwarding(node, ordinary);
    match node.kind() {
        "function_definition" => {
            if let Some(body) = node.child_by_field_name("body") {
                found.walk_block(body, 0, true);
            }
        }
        "lambda" => {
            if let Some(body) = node.child_by_field_name("body") {
                found.lloc += 1;
                found.walk(body, 0);
            }
        }
        _ => {}
    }
    Body {
        lloc: found.lloc,
        decisions: found.decisions,
        cognitive: found.cognitive,
        chains: found.chains,
        forward,
    }
}

struct Classifier<'a> {
    source: &'a [u8],
    receiver: Option<&'a str>,
    skip: Option<std::ops::Range<usize>>,
    lloc: u32,
    decisions: Vec<Decision>,
    cognitive: Vec<Cognitive>,
    chains: Vec<Chain>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BoolOp {
    And,
    Or,
}

struct Hop {
    name: String,
    call: bool,
    line: u32,
}

struct Peeled {
    root: Option<String>,
    hops: Vec<Hop>,
}

impl<'a> Classifier<'a> {
    fn forwarding(&mut self, node: Node<'a>, ordinary: &[String]) -> Option<Forward> {
        if ordinary.is_empty() {
            return None;
        }
        let (line, call) = forwarding_call(node)?;
        if contains_call(call) {
            return None;
        }
        self.skip = Some(chain_root(call).byte_range());
        Some(Forward {
            line,
            names: forwarded_names(call, self.source, ordinary),
        })
    }

    fn walk_block(&mut self, node: Node<'a>, nesting: u32, skip_doc: bool) {
        let mut statements = named_nodes(node);
        if skip_doc && statements.first().copied().is_some_and(is_docstring) {
            statements.remove(0);
        }
        for statement in statements {
            self.walk_statement(statement, nesting);
        }
    }

    fn walk_statement(&mut self, node: Node<'a>, nesting: u32) {
        match node.kind() {
            "if_statement" => self.walk_if(node, nesting),
            "for_statement" => self.walk_for(node, nesting),
            "while_statement" => self.walk_while(node, nesting),
            "try_statement" => self.walk_try(node, nesting),
            "with_statement" => self.walk_with(node, nesting),
            "match_statement" => self.walk_match(node, nesting),
            "function_definition" => self.walk_nested_def(node, nesting),
            "class_definition" => self.walk_nested_class(node, nesting),
            "decorated_definition" => self.walk_decorated(node, nesting),
            "assert_statement" => self.walk_assert(node, nesting),
            "expression_statement"
            | "return_statement"
            | "delete_statement"
            | "raise_statement"
            | "pass_statement"
            | "break_statement"
            | "continue_statement"
            | "import_statement"
            | "import_from_statement"
            | "future_import_statement"
            | "global_statement"
            | "nonlocal_statement"
            | "type_alias_statement"
            | "exec_statement"
            | "print_statement" => self.walk_simple(node, nesting),
            _ => self.walk(node, nesting),
        }
    }

    fn walk(&mut self, node: Node<'a>, nesting: u32) {
        match node.kind() {
            "if_statement"
            | "for_statement"
            | "while_statement"
            | "try_statement"
            | "with_statement"
            | "match_statement"
            | "function_definition"
            | "class_definition"
            | "decorated_definition"
            | "assert_statement"
            | "expression_statement"
            | "return_statement"
            | "delete_statement"
            | "raise_statement"
            | "pass_statement"
            | "break_statement"
            | "continue_statement"
            | "import_statement"
            | "import_from_statement"
            | "future_import_statement"
            | "global_statement"
            | "nonlocal_statement"
            | "type_alias_statement"
            | "exec_statement"
            | "print_statement" => self.walk_statement(node, nesting),
            "block" => self.walk_block(node, nesting, false),
            "boolean_operator" => self.walk_boolean(node, nesting),
            "conditional_expression" => self.walk_conditional(node, nesting),
            "lambda" | "lambda_within_for_in_clause" => self.walk_lambda(node, nesting),
            "list_comprehension"
            | "set_comprehension"
            | "dictionary_comprehension"
            | "generator_expression" => self.walk_comprehension(node, nesting),
            "attribute" | "call" | "subscript" | "parenthesized_expression" | "await" => {
                self.record_chain(node);
                self.walk_named(node, nesting);
            }
            _ => self.walk_named(node, nesting),
        }
    }

    fn walk_named(&mut self, node: Node<'a>, nesting: u32) {
        for child in named_nodes(node) {
            self.walk(child, nesting);
        }
    }

    fn walk_simple(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.walk_named(node, nesting);
    }

    fn walk_assert(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.decision(line_of(node), nesting);
        self.walk_named(node, nesting);
    }

    fn walk_if(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.decision(line_of(node), nesting);
        self.structural(line_of(node), nesting);
        if let Some(condition) = node.child_by_field_name("condition") {
            self.walk(condition, nesting);
        }
        if let Some(body) = node.child_by_field_name("consequence") {
            self.walk_block(body, nesting + 1, false);
        }
        for alternative in field_nodes(node, "alternative") {
            self.walk_alternative(alternative, nesting, false);
        }
    }

    fn walk_for(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.decision(line_of(node), nesting);
        self.structural(line_of(node), nesting);
        if let Some(right) = node.child_by_field_name("right") {
            self.walk(right, nesting);
        }
        if let Some(body) = node.child_by_field_name("body") {
            self.walk_block(body, nesting + 1, false);
        }
        for alternative in field_nodes(node, "alternative") {
            self.walk_alternative(alternative, nesting, true);
        }
    }

    fn walk_while(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.decision(line_of(node), nesting);
        self.structural(line_of(node), nesting);
        if let Some(condition) = node.child_by_field_name("condition") {
            self.walk(condition, nesting);
        }
        if let Some(body) = node.child_by_field_name("body") {
            self.walk_block(body, nesting + 1, false);
        }
        for alternative in field_nodes(node, "alternative") {
            self.walk_alternative(alternative, nesting, true);
        }
    }

    fn walk_alternative(&mut self, node: Node<'a>, nesting: u32, decides: bool) {
        if node.kind() == "elif_clause" {
            self.decision(line_of(node), nesting);
            self.hybrid(line_of(node));
            if let Some(condition) = node.child_by_field_name("condition") {
                self.walk(condition, nesting);
            }
            if let Some(body) = node.child_by_field_name("consequence") {
                self.walk_block(body, nesting + 1, false);
            }
            return;
        }
        if decides {
            self.decision(line_of(node), nesting);
        }
        self.hybrid(line_of(node));
        if let Some(body) = node.child_by_field_name("body") {
            self.walk_block(body, nesting + 1, false);
        }
    }

    fn walk_try(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        if let Some(body) = node.child_by_field_name("body") {
            self.walk_block(body, nesting, false);
        }
        for child in named_nodes(node) {
            match child.kind() {
                "except_clause" => self.walk_except(child, nesting),
                "else_clause" => self.walk_alternative(child, nesting, true),
                "finally_clause" => self.walk_suite(child, nesting),
                _ => {}
            }
        }
    }

    fn walk_except(&mut self, node: Node<'a>, nesting: u32) {
        self.decision(line_of(node), nesting);
        self.structural(line_of(node), nesting);
        for child in named_nodes(node) {
            if child.kind() == "block" {
                self.walk_block(child, nesting + 1, false);
            } else {
                self.walk(child, nesting);
            }
        }
    }

    fn walk_with(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        for child in named_nodes(node) {
            if child.kind() == "block" {
                self.walk_block(child, nesting, false);
            } else {
                self.walk(child, nesting);
            }
        }
    }

    fn walk_suite(&mut self, node: Node<'a>, nesting: u32) {
        for child in named_nodes(node) {
            if child.kind() == "block" {
                self.walk_block(child, nesting, false);
            }
        }
    }

    fn walk_match(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.structural(line_of(node), nesting);
        for child in named_nodes(node) {
            if child.kind() == "block" {
                for case in named_nodes(child) {
                    self.walk_case(case, nesting);
                }
            } else {
                self.walk(child, nesting);
            }
        }
    }

    fn walk_case(&mut self, node: Node<'a>, nesting: u32) {
        let line = line_of(node);
        for _ in 0..case_labels(node) {
            self.decision(line, nesting);
        }
        if let Some(guard) = node.child_by_field_name("guard") {
            self.decision(line_of(guard), nesting);
            self.walk_named(guard, nesting);
        }
        if let Some(body) = node.child_by_field_name("consequence") {
            self.walk_block(body, nesting + 1, false);
        }
    }

    fn walk_nested_def(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.walk_header(node, nesting);
    }

    fn walk_nested_class(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.walk_header(node, nesting);
    }

    fn walk_decorated(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        for child in named_nodes(node) {
            if matches!(child.kind(), "function_definition" | "class_definition") {
                self.walk_header(child, nesting);
            } else {
                self.walk(child, nesting);
            }
        }
    }

    fn walk_header(&mut self, node: Node<'a>, nesting: u32) {
        let children = named_nodes(node);
        for (index, child) in children.iter().enumerate() {
            if node.field_name_for_named_child(index as u32) == Some("body") {
                continue;
            }
            self.walk(*child, nesting);
        }
    }

    fn walk_boolean(&mut self, node: Node<'a>, nesting: u32) {
        let mut last = None;
        self.bool_spine(node, nesting, &mut last);
    }

    fn bool_spine(&mut self, node: Node<'a>, nesting: u32, last: &mut Option<BoolOp>) {
        if node.kind() == "not_operator" {
            if let Some(argument) = node.child_by_field_name("argument") {
                self.bool_spine(argument, nesting, last);
            }
            return;
        }
        if node.kind() == "parenthesized_expression" {
            self.record_chain(node);
            let mut inner = None;
            for child in named_nodes(node) {
                self.bool_spine(child, nesting, &mut inner);
            }
            return;
        }
        if node.kind() != "boolean_operator" {
            self.walk(node, nesting);
            return;
        }
        let operator = bool_op(node, self.source);
        if let Some(left) = node.child_by_field_name("left") {
            self.bool_spine(left, nesting, last);
        }
        let line = node
            .child_by_field_name("operator")
            .map(line_of)
            .unwrap_or_else(|| line_of(node));
        self.decision(line, nesting);
        if *last != Some(operator) {
            self.fundamental(line);
            *last = Some(operator);
        }
        if let Some(right) = node.child_by_field_name("right") {
            self.bool_spine(right, nesting, last);
        }
    }

    fn walk_conditional(&mut self, node: Node<'a>, nesting: u32) {
        let line = token_line(node, self.source, "if");
        self.decision(line, nesting);
        self.structural(line, nesting);
        for child in named_nodes(node) {
            self.walk(child, nesting + 1);
        }
    }

    fn walk_lambda(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        if let Some(body) = node.child_by_field_name("body") {
            self.walk(body, nesting + 1);
        }
    }

    fn walk_comprehension(&mut self, node: Node<'a>, nesting: u32) {
        for child in named_nodes(node) {
            match child.kind() {
                "for_in_clause" => {
                    self.decision(line_of(child), nesting);
                    if let Some(right) = child.child_by_field_name("right") {
                        self.walk(right, nesting);
                    }
                }
                "if_clause" => {
                    self.decision(line_of(child), nesting);
                    self.walk_named(child, nesting);
                }
                _ => self.walk(child, nesting),
            }
        }
    }

    fn record_chain(&mut self, node: Node<'a>) {
        if chain_continues(node) {
            return;
        }
        let Some(peeled) = peel(node, self.source) else {
            return;
        };
        if peeled.hops.is_empty() {
            return;
        }
        // A bare forwarding call is the `forward` record. A dotted chain stays.
        let bare_forward = self.skip.as_ref() == Some(&node.byte_range())
            && peeled.root.is_none()
            && peeled.hops.len() == 1;
        if bare_forward {
            return;
        }
        let line = peeled.hops[0].line;
        let rooted = self
            .receiver
            .is_some_and(|receiver| peeled.root.as_deref() == Some(receiver));
        self.chains.push(Chain {
            line,
            rooted,
            members: peeled
                .hops
                .into_iter()
                .map(|hop| Member {
                    name: hop.name,
                    call: hop.call,
                })
                .collect(),
        });
    }

    fn decision(&mut self, line: u32, nesting: u32) {
        self.decisions.push(Decision {
            line,
            depth: nesting + 1,
        });
    }

    fn structural(&mut self, line: u32, nesting: u32) {
        self.cognitive.push(Cognitive {
            line,
            kind: CognitiveKind::Structural,
            depth: nesting,
        });
    }

    fn hybrid(&mut self, line: u32) {
        self.cognitive.push(Cognitive {
            line,
            kind: CognitiveKind::Hybrid,
            depth: 0,
        });
    }

    fn fundamental(&mut self, line: u32) {
        self.cognitive.push(Cognitive {
            line,
            kind: CognitiveKind::Fundamental,
            depth: 0,
        });
    }
}

fn forwarding_call(node: Node) -> Option<(u32, Node)> {
    match node.kind() {
        "function_definition" => {
            let body = node.child_by_field_name("body")?;
            let mut statements = named_nodes(body);
            if statements.first().copied().is_some_and(is_docstring) {
                statements.remove(0);
            }
            let [statement] = statements.as_slice() else {
                return None;
            };
            Some((line_of(*statement), unwrap_call(*statement)?))
        }
        "lambda" => {
            let body = node.child_by_field_name("body")?;
            let call = unwrap_call(body)?;
            Some((line_of(call), call))
        }
        _ => None,
    }
}

fn unwrap_call(node: Node) -> Option<Node> {
    match node.kind() {
        "call" => Some(node),
        "return_statement" | "expression_statement" | "parenthesized_expression" | "await" => {
            let children = named_nodes(node);
            let [child] = children.as_slice() else {
                return None;
            };
            unwrap_call(*child)
        }
        _ => None,
    }
}

fn contains_call(node: Node) -> bool {
    for index in 0..node.child_count() {
        if let Some(child) = node.child(index) {
            if child.kind() == "call" || contains_call(child) {
                return true;
            }
        }
    }
    false
}

fn forwarded_names(call: Node, source: &[u8], ordinary: &[String]) -> Vec<String> {
    let mut found = BTreeSet::new();
    if let Some(function) = call.child_by_field_name("function") {
        if function.kind() == "attribute" {
            if let Some(object) = function.child_by_field_name("object") {
                if let Some(name) = bare_name(object, source) {
                    found.insert(name);
                }
            }
        }
    }
    if let Some(arguments) = call.child_by_field_name("arguments") {
        if arguments.kind() == "argument_list" {
            for argument in named_nodes(arguments) {
                if let Some(name) = argument_name(argument, source) {
                    found.insert(name);
                }
            }
        }
    }
    ordinary
        .iter()
        .filter(|name| found.contains(*name))
        .cloned()
        .collect()
}

fn argument_name(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "keyword_argument" => node
            .child_by_field_name("value")
            .and_then(|value| bare_name(value, source)),
        "list_splat" | "dictionary_splat" => {
            let children = named_nodes(node);
            let [child] = children.as_slice() else {
                return None;
            };
            bare_name(*child, source)
        }
        _ => bare_name(node, source),
    }
}

fn bare_name(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => Some(node_text(node, source)),
        "parenthesized_expression" => {
            let children = named_nodes(node);
            let [child] = children.as_slice() else {
                return None;
            };
            bare_name(*child, source)
        }
        _ => None,
    }
}

fn is_docstring(node: Node) -> bool {
    if node.kind() != "expression_statement" {
        return false;
    }
    let children = named_nodes(node);
    let [child] = children.as_slice() else {
        return false;
    };
    matches!(child.kind(), "string" | "concatenated_string") && !has_kind(*child, "interpolation")
}

fn has_kind(node: Node, kind: &str) -> bool {
    if node.kind() == kind {
        return true;
    }
    for index in 0..node.child_count() {
        if let Some(child) = node.child(index) {
            if has_kind(child, kind) {
                return true;
            }
        }
    }
    false
}

fn case_labels(clause: Node) -> u32 {
    let patterns: Vec<Node> = named_nodes(clause)
        .into_iter()
        .filter(|child| child.kind() == "case_pattern")
        .collect();
    match patterns.as_slice() {
        [] => 0,
        [pattern] => specified_alts(*pattern),
        patterns => u32::from(patterns.iter().any(|pattern| specified_alts(*pattern) > 0)),
    }
}

fn specified_alts(pattern: Node) -> u32 {
    let inners = named_nodes(pattern);
    match inners.as_slice() {
        [] => 0,
        [inner] if inner.kind() == "union_pattern" => named_nodes(*inner).len() as u32,
        _ => 1,
    }
}

fn chain_root(node: Node) -> Node {
    let mut current = node;
    while chain_continues(current) {
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }
    current
}

fn chain_continues(node: Node) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    match parent.kind() {
        "attribute" => parent.child_by_field_name("object") == Some(node),
        "subscript" => parent.child_by_field_name("value") == Some(node),
        "call" => parent.child_by_field_name("function") == Some(node),
        "parenthesized_expression" | "await" => {
            let children = named_nodes(parent);
            children.len() == 1 && children.first() == Some(&node)
        }
        _ => false,
    }
}

fn peel(node: Node, source: &[u8]) -> Option<Peeled> {
    match node.kind() {
        "identifier" => Some(Peeled {
            root: Some(node_text(node, source)),
            hops: Vec::new(),
        }),
        "attribute" => {
            let object = node.child_by_field_name("object")?;
            let mut peeled = peel(object, source).unwrap_or(Peeled {
                root: None,
                hops: Vec::new(),
            });
            let name = node.child_by_field_name("attribute")?;
            peeled.hops.push(Hop {
                name: node_text(name, source),
                call: false,
                line: line_of(name),
            });
            Some(peeled)
        }
        "call" => {
            let function = node.child_by_field_name("function")?;
            if let Some(name) = bare_name(function, source) {
                return Some(Peeled {
                    root: None,
                    hops: vec![Hop {
                        name,
                        call: true,
                        line: line_of(function),
                    }],
                });
            }
            let mut peeled = peel(function, source)?;
            let last = peeled.hops.last_mut()?;
            last.call = true;
            Some(peeled)
        }
        "subscript" => peel(node.child_by_field_name("value")?, source),
        "parenthesized_expression" | "await" => {
            let children = named_nodes(node);
            let [child] = children.as_slice() else {
                return None;
            };
            peel(*child, source)
        }
        _ => None,
    }
}

fn bool_op(node: Node, source: &[u8]) -> BoolOp {
    match node
        .child_by_field_name("operator")
        .map(|operator| node_text(operator, source))
        .as_deref()
    {
        Some("or") => BoolOp::Or,
        _ => BoolOp::And,
    }
}

fn field_nodes<'a>(node: Node<'a>, field: &str) -> Vec<Node<'a>> {
    let mut cursor = node.walk();
    node.children_by_field_name(field, &mut cursor).collect()
}

fn line_of(node: Node) -> u32 {
    node.start_position().row as u32 + 1
}

fn token_line(node: Node, source: &[u8], token: &str) -> u32 {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !child.is_named() && node_text(child, source) == token {
            return line_of(child);
        }
    }
    line_of(node)
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
    use crate::ir::{Cognitive, CognitiveKind, Decision, Forward, Language};

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

    #[test]
    fn an_if_else_boolean_and_lambda_are_classified_and_a_one_line_call_forwards() {
        let source = "\
def choose(a, b):
    if a and b:
        return (lambda x: x + 1)(a)
    else:
        held = b
        return held

def ship(url, params):
    \"\"\"Send it.\"\"\"
    return send(url, params=params)
";
        let file = parse("src/sample.py", source.as_bytes());
        let choose = function(&file, "choose");
        assert_eq!(choose.body.lloc, 5);
        assert_eq!(
            choose.body.decisions,
            vec![
                Decision { line: 2, depth: 1 },
                Decision { line: 2, depth: 1 },
            ]
        );
        assert_eq!(
            choose.body.cognitive,
            vec![
                Cognitive {
                    line: 2,
                    kind: CognitiveKind::Structural,
                    depth: 0,
                },
                Cognitive {
                    line: 2,
                    kind: CognitiveKind::Fundamental,
                    depth: 0,
                },
                Cognitive {
                    line: 4,
                    kind: CognitiveKind::Hybrid,
                    depth: 0,
                },
            ]
        );
        assert!(choose.body.chains.is_empty());
        assert!(choose.body.forward.is_none());

        let ship = function(&file, "ship");
        assert_eq!(ship.body.lloc, 1);
        assert!(ship.body.decisions.is_empty());
        assert!(ship.body.cognitive.is_empty());
        assert!(ship.body.chains.is_empty());
        assert_eq!(
            ship.body.forward,
            Some(Forward {
                line: 10,
                names: vec!["url".to_string(), "params".to_string()],
            })
        );
    }

    #[test]
    fn cognitive_nesting_follows_the_white_paper() {
        let source = "\
def nested(a, b, c):
    if a:
        if b:
            return 1
        elif c:
            return 2

def other(a, b):
    if a:
        return 1
    else:
        if b:
            return 2
";
        let file = parse("a.py", source.as_bytes());
        let nested = function(&file, "nested");
        assert_eq!(nested.body.lloc, 4);
        assert_eq!(
            nested.body.decisions,
            vec![
                Decision { line: 2, depth: 1 },
                Decision { line: 3, depth: 2 },
                Decision { line: 5, depth: 2 },
            ]
        );
        assert_eq!(cognitive_total(&nested.body), 4);

        let other = function(&file, "other");
        assert_eq!(
            other.body.decisions,
            vec![
                Decision { line: 9, depth: 1 },
                Decision { line: 12, depth: 2 },
            ]
        );
        assert_eq!(cognitive_total(&other.body), 4);
        assert!(other
            .body
            .cognitive
            .iter()
            .any(|item| item.kind == CognitiveKind::Hybrid && item.line == 11));
    }

    #[test]
    fn boolean_runs_split_on_a_changed_operator_and_on_parentheses() {
        let source = "\
def same(a, b, c):
    return a and b and c

def changed(a, b, c):
    return a and b or c

def grouped(a, b, c):
    return (a and b) and c

def negated(a, b):
    return not a and not b

def resumed(a, b, c, d):
    return a and (b or c) and d
";
        let file = parse("a.py", source.as_bytes());
        assert_eq!(function(&file, "same").body.decisions.len(), 2);
        assert_eq!(cognitive_total(&function(&file, "same").body), 1);
        assert_eq!(function(&file, "changed").body.decisions.len(), 2);
        assert_eq!(cognitive_total(&function(&file, "changed").body), 2);
        assert_eq!(function(&file, "grouped").body.decisions.len(), 2);
        assert_eq!(cognitive_total(&function(&file, "grouped").body), 2);
        assert_eq!(function(&file, "negated").body.decisions.len(), 1);
        assert_eq!(cognitive_total(&function(&file, "negated").body), 1);
        assert_eq!(function(&file, "resumed").body.decisions.len(), 3);
        assert_eq!(cognitive_total(&function(&file, "resumed").body), 2);
    }

    #[test]
    fn decisions_follow_the_cyclomatic_rules() {
        let source = "\
def paths(xs):
    for x in xs:
        while x:
            return x
    else:
        assert xs
    with xs:
        try:
            pass
        except xs:
            pass
        except (xs, xs):
            pass
        else:
            pass
        finally:
            pass
    return [x for x in xs if x] if xs else xs

def bits(a, b, c):
    return a & b | c
";
        let file = parse("a.py", source.as_bytes());
        let paths = function(&file, "paths");
        assert_eq!(paths.body.lloc, 12);
        assert_eq!(
            paths.body.decisions,
            vec![
                Decision { line: 2, depth: 1 },
                Decision { line: 3, depth: 2 },
                Decision { line: 5, depth: 1 },
                Decision { line: 6, depth: 2 },
                Decision { line: 10, depth: 1 },
                Decision { line: 12, depth: 1 },
                Decision { line: 14, depth: 1 },
                Decision { line: 18, depth: 1 },
                Decision { line: 18, depth: 2 },
                Decision { line: 18, depth: 2 },
            ]
        );
        assert_eq!(cognitive_total(&paths.body), 8);
        assert!(function(&file, "bits").body.decisions.is_empty());
        assert!(function(&file, "bits").body.cognitive.is_empty());
    }

    #[test]
    fn a_match_counts_specified_labels_and_one_structural_increment() {
        let source = "\
def classify(x):
    match x:
        case 1 | 2:
            return 1
        case _:
            return 2
        case 3 if x:
            return 3

def sequence(x):
    match x:
        case 1, 2:
            return 1
";
        let file = parse("a.py", source.as_bytes());
        let classify = function(&file, "classify");
        assert_eq!(classify.body.lloc, 4);
        assert_eq!(classify.body.decisions.len(), 4);
        assert_eq!(
            classify
                .body
                .decisions
                .iter()
                .filter(|decision| decision.line == 3)
                .count(),
            2
        );
        assert_eq!(cognitive_total(&classify.body), 1);
        assert_eq!(classify.body.cognitive[0].kind, CognitiveKind::Structural);
        assert_eq!(classify.body.cognitive[0].depth, 0);
        assert_eq!(function(&file, "sequence").body.decisions.len(), 1);
    }

    #[test]
    fn a_lambda_folds_into_its_function_and_a_nested_def_does_not() {
        let source = "\
def outer(a):
    def inner(x):
        if x:
            return x
    return lambda y: y if a else 0
";
        let file = parse("a.py", source.as_bytes());
        assert_eq!(
            file.functions
                .iter()
                .map(|function| function.name.as_str())
                .collect::<Vec<_>>(),
            vec!["outer", "outer.inner"]
        );
        let outer = function(&file, "outer");
        assert_eq!(outer.body.lloc, 3);
        assert_eq!(outer.body.decisions, vec![Decision { line: 5, depth: 2 }]);
        assert_eq!(cognitive_total(&outer.body), 2);
        let inner = function(&file, "outer.inner");
        assert_eq!(inner.body.lloc, 2);
        assert_eq!(inner.body.decisions, vec![Decision { line: 3, depth: 1 }]);
        assert_eq!(cognitive_total(&inner.body), 1);
    }

    #[test]
    fn logical_lines_are_statements() {
        let source = "\
def empty():
    \"\"\"doc\"\"\"

def two():
    x = 1; y = 2

def parent():
    def child():
        return 1
    return child
";
        let file = parse("a.py", source.as_bytes());
        assert_eq!(function(&file, "empty").body.lloc, 0);
        assert_eq!(function(&file, "two").body.lloc, 2);
        assert_eq!(function(&file, "parent").body.lloc, 2);
        assert_eq!(function(&file, "parent.child").body.lloc, 1);
    }

    #[test]
    fn member_chains_keep_every_hop() {
        let source = "\
class Box:
    def run(self, dst):
        self.repo.save(dst)
        return dst.len().min(1)

    def kept(self):
        return self.get().copy()

def indexed(a):
    value = a.b[0].c()
    return value

def only(a):
    return a.b[0].c()

def calls(a):
    send()
    return a
";
        let file = parse("a.py", source.as_bytes());
        let run = function(&file, "Box.run");
        assert_eq!(run.body.chains.len(), 2);
        assert!(run.body.chains[0].rooted);
        assert_eq!(member_names(&run.body.chains[0]), vec!["repo", "save"]);
        assert!(!run.body.chains[0].members[0].call);
        assert!(run.body.chains[0].members[1].call);
        assert!(!run.body.chains[1].rooted);
        assert_eq!(member_names(&run.body.chains[1]), vec!["len", "min"]);
        assert!(run.body.chains[1].members.iter().all(|member| member.call));

        let kept = function(&file, "Box.kept");
        assert!(kept.body.chains[0].rooted);
        assert_eq!(member_names(&kept.body.chains[0]), vec!["get", "copy"]);
        assert!(kept.body.forward.is_none());

        let indexed = function(&file, "indexed");
        assert_eq!(indexed.body.chains.len(), 1);
        assert_eq!(member_names(&indexed.body.chains[0]), vec!["b", "c"]);
        assert!(!indexed.body.chains[0].members[0].call);
        assert!(indexed.body.chains[0].members[1].call);
        let only = function(&file, "only");
        assert_eq!(member_names(&only.body.chains[0]), vec!["b", "c"]);
        assert_eq!(
            only.body.forward.as_ref().unwrap().names,
            Vec::<String>::new()
        );

        let calls = function(&file, "calls");
        assert_eq!(calls.body.chains.len(), 1);
        assert_eq!(member_names(&calls.body.chains[0]), vec!["send"]);
        assert!(calls.body.chains[0].members[0].call);
        assert!(!calls.body.chains[0].rooted);
    }

    #[test]
    fn a_single_call_forwards_ordinary_parameters() {
        let source = "\
def get(url, params, kwargs):
    \"\"\"doc\"\"\"
    return request(\"get\", url, params=params, **kwargs)

class C:
    def m(self, url):
        return self.helper(url)

    def only_self(self):
        return g()

def bad(a):
    return f(g(a))

def two(a):
    x = a
    return f(x)

async def waited(url):
    return await send(url)

def splat(args, kwargs):
    return send(*args, **kwargs)

def renamed(url, params):
    return send(params, url=url)

def field(a):
    return f(a.b)

def none():
    return g()
";
        let file = parse("a.py", source.as_bytes());
        assert_eq!(
            function(&file, "get").body.forward,
            Some(Forward {
                line: 3,
                names: vec![
                    "url".to_string(),
                    "params".to_string(),
                    "kwargs".to_string()
                ],
            })
        );
        assert!(function(&file, "get").body.chains.is_empty());
        assert_eq!(
            function(&file, "C.m").body.forward.as_ref().unwrap().names,
            vec!["url".to_string()]
        );
        assert!(function(&file, "C.m").body.chains[0].rooted);
        assert_eq!(
            member_names(&function(&file, "C.m").body.chains[0]),
            vec!["helper"]
        );
        assert!(function(&file, "C.only_self").body.forward.is_none());
        assert_eq!(
            member_names(&function(&file, "C.only_self").body.chains[0]),
            vec!["g"]
        );
        assert!(function(&file, "bad").body.forward.is_none());
        assert_eq!(function(&file, "bad").body.chains.len(), 2);
        assert!(function(&file, "two").body.forward.is_none());
        assert_eq!(
            function(&file, "waited")
                .body
                .forward
                .as_ref()
                .unwrap()
                .names,
            vec!["url".to_string()]
        );
        assert_eq!(
            function(&file, "splat")
                .body
                .forward
                .as_ref()
                .unwrap()
                .names,
            vec!["args".to_string(), "kwargs".to_string()]
        );
        assert_eq!(
            function(&file, "renamed")
                .body
                .forward
                .as_ref()
                .unwrap()
                .names,
            vec!["url".to_string(), "params".to_string()]
        );
        assert_eq!(
            function(&file, "field")
                .body
                .forward
                .as_ref()
                .unwrap()
                .names,
            Vec::<String>::new()
        );
        assert_eq!(
            member_names(&function(&file, "field").body.chains[0]),
            vec!["b"]
        );
        assert!(function(&file, "none").body.forward.is_none());
    }

    fn cognitive_total(body: &crate::ir::Body) -> u32 {
        body.cognitive
            .iter()
            .map(|item| match item.kind {
                CognitiveKind::Structural => 1 + item.depth,
                CognitiveKind::Hybrid | CognitiveKind::Fundamental => 1,
            })
            .sum()
    }

    fn member_names(chain: &crate::ir::Chain) -> Vec<&str> {
        chain
            .members
            .iter()
            .map(|member| member.name.as_str())
            .collect()
    }

    fn function<'a>(file: &'a crate::ir::File, name: &str) -> &'a crate::ir::Function {
        file.functions
            .iter()
            .find(|function| function.name == name)
            .unwrap_or_else(|| panic!("missing {name}"))
    }
}
