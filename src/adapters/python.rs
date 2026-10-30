use std::borrow::Cow;
use std::collections::BTreeSet;

use tree_sitter::Node;

use crate::ir::{
    Body, CallSite, Chain, Cognitive, CognitiveKind, Decision, File, Forward, Function, Language,
    Member, Param, Span, WaiverLine,
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

const NOISE: &[&str] = &[
    "log", "logger", "debug", "info", "warn", "warning", "error", "trace", "print", "println",
    "eprintln", "dbg",
];

const RESULT: &[&str] = &["Ok", "Some", "Err"];

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
            unscored_macros: 0,
            functions: Vec::new(),
        };
    };
    let root = tree.root_node();
    let mut functions = Vec::new();
    let mut scopes = Vec::new();
    let mut cursor = Cursor {
        source,
        scopes: &mut scopes,
        out: &mut functions,
    };
    cursor.visit(root);
    File {
        path: path.to_string(),
        language: Language::Python,
        unparsed_ratio: super::unparsed_ratio(root, source),
        unscored_macros: 0,
        functions,
    }
}

struct Cursor<'a> {
    source: &'a [u8],
    scopes: &'a mut Vec<Scope>,
    out: &'a mut Vec<Function>,
}

struct Found<'a> {
    name: &'a str,
    node: Node<'a>,
    touch_start: Option<u32>,
    params: Option<Node<'a>>,
    receiver: bool,
}

impl<'a> Cursor<'a> {
    fn visit(&mut self, node: Node<'_>) {
        match node.kind() {
            "function_definition" => self.visit_function(node),
            "class_definition" => self.visit_class(node),
            "assignment" => self.visit_assignment(node),
            _ => self.visit_children(node),
        }
    }

    fn visit_children(&mut self, node: Node<'_>) {
        for child in named_nodes(node) {
            self.visit(child);
        }
    }

    fn visit_function(&mut self, node: Node<'_>) {
        if is_overload(node, self.source) {
            return;
        }
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = node_text(name_node, self.source);
        let receiver = directly_in_class(self.scopes) && !is_staticmethod(node, self.source);
        let touch_start = decorator_touch(node);
        self.push_found(Found {
            name: &name,
            node,
            touch_start,
            params: node.child_by_field_name("parameters"),
            receiver,
        });
        self.scopes.push(Scope::Function(name));
        self.visit_body(node);
        self.scopes.pop();
    }

    fn visit_class(&mut self, node: Node<'_>) {
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = node_text(name_node, self.source);
        self.scopes.push(Scope::Class(name));
        self.visit_body(node);
        self.scopes.pop();
    }

    fn visit_body(&mut self, node: Node<'_>) {
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        self.visit_children(body);
    }

    fn visit_assignment(&mut self, node: Node<'_>) {
        if in_function(self.scopes) {
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
        let receiver = directly_in_class(self.scopes);
        let name = node_text(left, self.source);
        self.push_found(Found {
            name: &name,
            node: lambda,
            touch_start: None,
            params: lambda.child_by_field_name("parameters"),
            receiver,
        });
    }

    fn push_found(&mut self, found: Found<'_>) {
        let span = span_of(found.node);
        let mut touch = span;
        if let Some(start) = found.touch_start {
            touch.start = start;
        }
        let waivers = waivers_above(found.node, self.source);
        if let Some(first) = waivers.first() {
            touch.start = first.line;
        }
        let params = parameters(found.params, self.source, found.receiver);
        let ordinary = ordinary_names(&params);
        let receiver = receiver_name(&params);
        let body = classify(
            found.node,
            self.source,
            Names {
                receiver: receiver.as_deref(),
                ordinary: &ordinary,
            },
        );
        self.out.push(Function {
            name: qualify(self.scopes, found.name),
            param_types: Vec::new(),
            qualifier: String::new(),
            span,
            touch,
            params,
            body,
            waivers,
        });
    }
}

fn decorator_touch(func: Node<'_>) -> Option<u32> {
    let nodes = decorator_nodes(func);
    let first = nodes.first().copied();
    first.map(span_start)
}

fn span_start(node: Node<'_>) -> u32 {
    let span = span_of(node);
    span.start
}

fn parameters(node: Option<Node<'_>>, source: &[u8], receiver: bool) -> Vec<Param> {
    let names = node.map(|node| parameter_names(node, source));
    let names = names.unwrap_or_default();
    listed_params(names, receiver)
}

fn listed_params(names: Vec<String>, receiver: bool) -> Vec<Param> {
    let mut params = Vec::new();
    for (index, name) in names.into_iter().enumerate() {
        params.push(Param {
            name,
            receiver: receives(receiver, index),
        });
    }
    params
}

fn receives(receiver: bool, index: usize) -> bool {
    receiver && index == 0
}

fn ordinary_names(params: &[Param]) -> Vec<String> {
    params
        .iter()
        .filter(|param| !param.receiver)
        .map(|param| param.name.clone())
        .collect()
}

fn receiver_name(params: &[Param]) -> Option<String> {
    let receiver = params.iter().find(|param| param.receiver);
    receiver.map(|param| param.name.clone())
}

fn waivers_above(node: Node, source: &[u8]) -> Vec<WaiverLine> {
    let statement = statement_anchor(node);
    let mut next_line = span_start(statement);
    let mut found = Vec::new();
    {
        let mut run = WaiverRun {
            source,
            next_line: &mut next_line,
            found: &mut found,
        };
        if run.consume(statement) {
            extend_block_waivers(statement, &mut run);
        }
    }
    found.reverse();
    found
}

struct WaiverRun<'a> {
    source: &'a [u8],
    next_line: &'a mut u32,
    found: &'a mut Vec<WaiverLine>,
}

impl WaiverRun<'_> {
    fn consume(&mut self, node: Node<'_>) -> bool {
        let mut cursor = node.prev_sibling();
        while let Some(sibling) = cursor {
            if !self.take(sibling) {
                return false;
            }
            cursor = sibling.prev_sibling();
        }
        true
    }

    fn take(&mut self, sibling: Node<'_>) -> bool {
        if sibling.kind() != "comment" {
            return false;
        }
        let span = span_of(sibling);
        if !waiver_adjacent(&span, *self.next_line, sibling) {
            return false;
        }
        let text = node_text(sibling, self.source);
        let Some(waiver) = waiver_from_comment(&text, span.start) else {
            return false;
        };
        self.found.push(waiver);
        *self.next_line = span.start;
        true
    }
}

fn extend_block_waivers(statement: Node<'_>, run: &mut WaiverRun<'_>) {
    let Some(block) = statement.parent() else {
        return;
    };
    if block_waivers(block, statement) {
        run.consume(block);
    }
}

fn block_waivers(block: Node<'_>, statement: Node<'_>) -> bool {
    block.kind() == "block" && first_statement(statement)
}

fn waiver_adjacent(span: &Span, next_line: u32, sibling: Node<'_>) -> bool {
    let start = span.start;
    let end = span.end;
    start == end && start + 1 == next_line && !shares_code_line(sibling)
}

fn statement_anchor(node: Node) -> Node {
    let mut current = node;
    loop {
        let Some(parent) = current.parent() else {
            return current;
        };
        if parent.kind() == "module" || parent.kind() == "block" {
            return current;
        }
        current = parent;
    }
}

fn first_statement(statement: Node) -> bool {
    let mut cursor = statement.prev_sibling();
    while let Some(sibling) = cursor {
        if sibling.kind() != "comment" {
            return false;
        }
        cursor = sibling.prev_sibling();
    }
    true
}

fn waiver_from_comment(text: &str, line: u32) -> Option<WaiverLine> {
    let text = comment_body(text);
    let (marker, rest) = take_token(text)?;
    if marker != "slopmeter:" {
        return None;
    }
    let (verb, rest) = take_token(rest)?;
    if verb != "waive" {
        return None;
    }
    Some(parsed_waiver(rest, line))
}

fn comment_body(text: &str) -> &str {
    let trimmed = text.trim();
    let stripped = trimmed.strip_prefix('#');
    let text = stripped.unwrap_or(trimmed);
    text.trim()
}

fn parsed_waiver(rest: &str, line: u32) -> WaiverLine {
    let (rule, rest) = split_rule(rest);
    let (number, reason) = split_number(rest);
    WaiverLine {
        line,
        rule,
        number,
        reason,
    }
}

fn split_rule(rest: &str) -> (Option<String>, &str) {
    match take_token(rest) {
        Some((rule, rest)) => (Some(rule.to_string()), rest),
        None => (None, ""),
    }
}

fn split_number(rest: &str) -> (Option<String>, Option<String>) {
    match take_token(rest) {
        Some((token, after)) if is_number_token(token) => {
            (Some(token.to_string()), non_empty(after))
        }
        Some(_) => (None, non_empty(rest)),
        None => (None, None),
    }
}

fn non_empty(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

fn take_token(input: &str) -> Option<(&str, &str)> {
    let input = input.trim_start();
    if input.is_empty() {
        return None;
    }
    let end = input.find(char::is_whitespace).unwrap_or(input.len());
    Some((&input[..end], &input[end..]))
}

fn is_number_token(token: &str) -> bool {
    let mut seen_dot = false;
    let mut seen_digit = false;
    for character in token.chars() {
        match character {
            '0'..='9' => seen_digit = true,
            '.' if !seen_dot => seen_dot = true,
            _ => return false,
        }
    }
    seen_digit
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
    let kind = node.kind();
    if kind == "identifier" {
        return Some(node_text(node, source));
    }
    if is_default_parameter(kind) {
        return named_parameter(node, source);
    }
    if is_pattern_parameter(kind) {
        return untyped_parameter(node, source);
    }
    None
}

fn named_parameter(node: Node, source: &[u8]) -> Option<String> {
    let name = node.child_by_field_name("name");
    name.and_then(|name| parameter_name(name, source))
}

fn untyped_parameter(node: Node, source: &[u8]) -> Option<String> {
    let children = named_nodes(node);
    let child = children.into_iter().find(|child| child.kind() != "type");
    child.and_then(|child| parameter_name(child, source))
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
    let trimmed = raw.trim();
    let without_at = trimmed.trim_start_matches('@');
    let trimmed = without_at.trim();
    let mut pieces = trimmed.split('(');
    let head = pieces.next();
    let trimmed = head.unwrap_or(trimmed);
    let trimmed = trimmed.trim();
    let mut words = trimmed.split_whitespace();
    let name = words.next();
    name.unwrap_or("").to_string()
}

fn span_of(node: Node) -> Span {
    let start_point = node.start_position();
    let end_point = node.end_position();
    let start = start_point.row as u32 + 1;
    let end = exclusive_end(start_point.row, end_point.row, end_point.column);
    Span {
        start,
        end: end.max(start),
    }
}

fn exclusive_end(start_row: usize, end_row: usize, end_column: usize) -> u32 {
    if end_column == 0 && end_row > start_row {
        return end_row as u32;
    }
    end_row as u32 + 1
}

fn except_block(node: Node<'_>) -> Option<Node<'_>> {
    named_nodes(node)
        .into_iter()
        .find(|child| child.kind() == "block")
}

fn quiet_except(block: Node<'_>) -> bool {
    named_nodes(block)
        .iter()
        .all(|child| quiet_statement(*child))
}

fn quiet_statement(node: Node<'_>) -> bool {
    match node.kind() {
        "pass_statement" | "continue_statement" | "break_statement" | "comment" => true,
        "expression_statement" => ellipsis_statement(node),
        _ => false,
    }
}

fn ellipsis_statement(node: Node<'_>) -> bool {
    let children = named_nodes(node);
    let only_marks = children
        .iter()
        .all(|child| matches!(child.kind(), "ellipsis" | "comment"));
    only_marks && children.iter().any(|child| child.kind() == "ellipsis")
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
    let lines = header.lines();
    let mut first = lines.take(2);
    first.any(latin1_name)
}

fn latin1_name(line: &str) -> bool {
    let Some(name) = coding_name(line) else {
        return false;
    };
    let lowered = name.to_ascii_lowercase();
    is_latin1(lowered.as_str())
}

fn is_latin1(name: &str) -> bool {
    matches!(
        name,
        "latin-1" | "latin1" | "iso-8859-1" | "iso8859-1" | "l1" | "cp819" | "csisolatin1"
    )
}

fn coding_name(line: &str) -> Option<&str> {
    let split = line.split_once("coding")?;
    let after = split.1;
    let rest = after.trim_start();
    let stripped = rest.strip_prefix([':', '=']);
    let rest = stripped?;
    let rest = rest.trim_start();
    let end = coding_end(rest);
    let name = &rest[..end];
    if name.is_empty() {
        return None;
    }
    Some(name)
}

fn coding_end(rest: &str) -> usize {
    let found = rest.find(coding_stop);
    found.unwrap_or(rest.len())
}

fn coding_stop(ch: char) -> bool {
    if ch.is_ascii_alphanumeric() {
        return false;
    }
    ch != '-' && ch != '_' && ch != '.'
}

struct Names<'a> {
    receiver: Option<&'a str>,
    ordinary: &'a [String],
}

fn classify(node: Node, source: &[u8], names: Names<'_>) -> Body {
    let mut found = Classifier {
        source,
        receiver: names.receiver,
        skip: None,
        lloc: 0,
        structure: 0,
        decisions: Vec::new(),
        levels: Vec::new(),
        cognitive: Vec::new(),
        chains: Vec::new(),
        swallowing: Vec::new(),
    };
    let forward = found.forwarding(node, names.ordinary);
    match node.kind() {
        "function_definition" => {
            found.walk_signature(node);
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
        levels: found.levels,
        cognitive: found.cognitive,
        chains: found.chains,
        swallowing: found.swallowing,
        io: record_io(node, source),
        mutations: record_mutations(node, source, names.ordinary),
        nulls: Vec::new(),
        forward,
    }
}

struct Classifier<'a> {
    source: &'a [u8],
    receiver: Option<&'a str>,
    skip: Option<std::ops::Range<usize>>,
    lloc: u32,
    structure: u32,
    decisions: Vec<Decision>,
    levels: Vec<Decision>,
    cognitive: Vec<Cognitive>,
    chains: Vec<Chain>,
    swallowing: Vec<u32>,
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

struct Spine<'a> {
    nesting: u32,
    last: &'a mut Option<BoolOp>,
}

fn is_statement_kind(kind: &str) -> bool {
    matches!(
        kind,
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
            | "print_statement"
    )
}

fn is_simple_statement(kind: &str) -> bool {
    matches!(
        kind,
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
            | "print_statement"
    )
}

fn is_lambda_kind(kind: &str) -> bool {
    matches!(kind, "lambda" | "lambda_within_for_in_clause")
}

fn is_comprehension_kind(kind: &str) -> bool {
    matches!(
        kind,
        "list_comprehension"
            | "set_comprehension"
            | "dictionary_comprehension"
            | "generator_expression"
    )
}

fn is_chain_kind(kind: &str) -> bool {
    matches!(
        kind,
        "attribute" | "call" | "subscript" | "parenthesized_expression" | "await"
    )
}

fn is_default_parameter(kind: &str) -> bool {
    matches!(kind, "default_parameter" | "typed_default_parameter")
}

fn is_pattern_parameter(kind: &str) -> bool {
    matches!(
        kind,
        "typed_parameter" | "list_splat_pattern" | "dictionary_splat_pattern"
    )
}

impl<'a> Classifier<'a> {
    fn forwarding(&mut self, node: Node<'a>, ordinary: &[String]) -> Option<Forward> {
        if ordinary.is_empty() {
            return None;
        }
        let (line, call) = forwarding_call(node, self.source)?;
        if contains_call(call) {
            return None;
        }
        if !arguments_allowed(call, self.source, ordinary) {
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
            _ => self.walk_other_statement(node, nesting),
        }
    }

    fn walk_other_statement(&mut self, node: Node<'a>, nesting: u32) {
        match node.kind() {
            "match_statement" => self.walk_match(node, nesting),
            "function_definition" => self.walk_nested_def(node, nesting),
            "class_definition" => self.walk_nested_class(node, nesting),
            "decorated_definition" => self.walk_decorated(node, nesting),
            "assert_statement" => self.walk_assert(node, nesting),
            _ => self.walk_plain_statement(node, nesting),
        }
    }

    fn walk_plain_statement(&mut self, node: Node<'a>, nesting: u32) {
        if is_simple_statement(node.kind()) {
            self.walk_simple(node, nesting);
            return;
        }
        self.walk(node, nesting);
    }

    fn walk(&mut self, node: Node<'a>, nesting: u32) {
        if self.walk_controlled(node, nesting) {
            return;
        }
        self.walk_expression(node, nesting);
    }

    fn walk_controlled(&mut self, node: Node<'a>, nesting: u32) -> bool {
        if is_statement_kind(node.kind()) {
            self.walk_statement(node, nesting);
            return true;
        }
        match node.kind() {
            "block" => self.walk_block(node, nesting, false),
            "boolean_operator" => self.walk_boolean(node, nesting),
            "conditional_expression" => self.walk_conditional(node, nesting),
            _ => return false,
        }
        true
    }

    fn walk_expression(&mut self, node: Node<'a>, nesting: u32) {
        if is_lambda_kind(node.kind()) {
            self.walk_lambda(node, nesting);
            return;
        }
        if is_comprehension_kind(node.kind()) {
            self.walk_comprehension(node, nesting);
            return;
        }
        if is_chain_kind(node.kind()) {
            self.record_chain(node);
            self.walk_named(node, nesting);
            return;
        }
        self.walk_named(node, nesting);
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
            self.inside(|this| this.walk_block(body, nesting + 1, false));
        }
        for alternative in field_nodes(node, "alternative") {
            self.walk_alternative(alternative, nesting, true);
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
            self.inside(|this| this.walk_block(body, nesting + 1, false));
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
            self.inside(|this| this.walk_block(body, nesting + 1, false));
        }
        for alternative in field_nodes(node, "alternative") {
            self.walk_alternative(alternative, nesting, true);
        }
    }

    fn walk_alternative(&mut self, node: Node<'a>, nesting: u32, inside: bool) {
        if node.kind() == "elif_clause" {
            self.decision(line_of(node), nesting);
            self.hybrid(line_of(node));
            if let Some(condition) = node.child_by_field_name("condition") {
                self.walk(condition, nesting);
            }
            if let Some(body) = node.child_by_field_name("consequence") {
                self.descend(inside, |this| this.walk_block(body, nesting + 1, false));
            }
            return;
        }
        self.hybrid(line_of(node));
        if let Some(body) = node.child_by_field_name("body") {
            self.descend(inside, |this| this.walk_block(body, nesting + 1, false));
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
                "else_clause" => self.walk_alternative(child, nesting, false),
                "finally_clause" => self.walk_suite(child, nesting),
                _ => {}
            }
        }
    }

    fn walk_except(&mut self, node: Node<'a>, nesting: u32) {
        self.decision(line_of(node), nesting);
        self.structural(line_of(node), nesting);
        self.note_swallowed_except(node);
        for child in named_nodes(node) {
            if child.kind() == "block" {
                self.inside(|this| this.walk_block(child, nesting + 1, false));
            } else {
                self.walk(child, nesting);
            }
        }
    }

    fn note_swallowed_except(&mut self, node: Node<'a>) {
        let Some(block) = except_block(node) else {
            return;
        };
        if quiet_except(block) {
            self.swallowing.push(line_of(node));
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
            self.walk_match_child(child, nesting);
        }
    }

    fn walk_match_child(&mut self, child: Node<'a>, nesting: u32) {
        if child.kind() == "block" {
            self.walk_cases(child, nesting);
            return;
        }
        self.walk(child, nesting);
    }

    fn walk_cases(&mut self, child: Node<'a>, nesting: u32) {
        for case in named_nodes(child) {
            self.walk_case(case, nesting);
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
            self.inside(|this| this.walk_block(body, nesting + 1, false));
        }
    }

    fn walk_nested_def(&mut self, _node: Node<'a>, _nesting: u32) {
        self.lloc += 1;
    }

    fn walk_nested_class(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.walk_header(node, nesting);
    }

    fn walk_decorated(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        let children = named_nodes(node);
        if children
            .iter()
            .any(|child| child.kind() == "function_definition")
        {
            return;
        }
        for child in children {
            if child.kind() == "class_definition" {
                self.walk_header(child, nesting);
            } else {
                self.walk(child, nesting);
            }
        }
    }

    fn walk_signature(&mut self, node: Node<'a>) {
        self.walk_decorators(node);
        self.walk_defaults(node);
    }

    fn walk_decorators(&mut self, node: Node<'a>) {
        for decorator in decorator_nodes(node) {
            self.walk_decorator_arguments(decorator);
        }
    }

    fn walk_defaults(&mut self, node: Node<'a>) {
        let Some(parameters) = node.child_by_field_name("parameters") else {
            return;
        };
        for child in named_nodes(parameters) {
            self.walk_default(child);
        }
    }

    fn walk_default(&mut self, child: Node<'a>) {
        if !is_default_parameter(child.kind()) {
            return;
        }
        let Some(value) = child.child_by_field_name("value") else {
            return;
        };
        self.walk(value, 0);
    }

    fn walk_decorator_arguments(&mut self, node: Node<'a>) {
        for child in named_nodes(node) {
            if child.kind() != "call" {
                continue;
            }
            if let Some(arguments) = child.child_by_field_name("arguments") {
                self.walk(arguments, 0);
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
        let mut spine = Spine {
            nesting,
            last: &mut last,
        };
        self.bool_spine(node, &mut spine);
    }

    fn bool_spine(&mut self, node: Node<'a>, spine: &mut Spine<'_>) {
        if self.descend_not(node, spine) {
            return;
        }
        if self.descend_group(node, spine.nesting) {
            return;
        }
        if node.kind() != "boolean_operator" {
            self.walk(node, spine.nesting);
            return;
        }
        self.bool_operator(node, spine);
    }

    fn descend_not(&mut self, node: Node<'a>, spine: &mut Spine<'_>) -> bool {
        if node.kind() != "not_operator" {
            return false;
        }
        self.not_argument(node, spine);
        true
    }

    fn not_argument(&mut self, node: Node<'a>, spine: &mut Spine<'_>) {
        let Some(argument) = node.child_by_field_name("argument") else {
            return;
        };
        self.bool_spine(argument, spine);
    }

    fn descend_group(&mut self, node: Node<'a>, nesting: u32) -> bool {
        if node.kind() != "parenthesized_expression" {
            return false;
        }
        self.record_chain(node);
        self.group_children(node, nesting);
        true
    }

    fn group_children(&mut self, node: Node<'a>, nesting: u32) {
        let mut inner = None;
        let mut spine = Spine {
            nesting,
            last: &mut inner,
        };
        for child in named_nodes(node) {
            self.bool_spine(child, &mut spine);
        }
    }

    fn bool_operator(&mut self, node: Node<'a>, spine: &mut Spine<'_>) {
        let operator = bool_op(node, self.source);
        self.spine_side(node, spine, "left");
        let line = operator_line(node);
        self.decision(line, spine.nesting);
        self.note_operator(line, operator, spine);
        self.spine_side(node, spine, "right");
    }

    fn spine_side(&mut self, node: Node<'a>, spine: &mut Spine<'_>, field: &str) {
        let Some(side) = node.child_by_field_name(field) else {
            return;
        };
        self.bool_spine(side, spine);
    }

    fn note_operator(&mut self, line: u32, operator: BoolOp, spine: &mut Spine<'_>) {
        if *spine.last != Some(operator) {
            self.fundamental(line);
            *spine.last = Some(operator);
        }
    }

    fn walk_conditional(&mut self, node: Node<'a>, nesting: u32) {
        let line = token_line(node, self.source, "if");
        self.decision(line, nesting);
        self.structural(line, nesting);
        for (index, child) in named_nodes(node).iter().copied().enumerate() {
            if index == 1 {
                self.walk(child, nesting + 1);
            } else {
                self.inside(|this| this.walk(child, nesting + 1));
            }
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
        self.store_chain(node, peeled);
    }

    fn store_chain(&mut self, node: Node<'a>, peeled: Peeled) {
        if hop_list_empty(&peeled) {
            return;
        }
        if self.skips_bare(&node, &peeled) {
            return;
        }
        let line = first_hop_line(&peeled);
        let rooted = self.chain_rooted(&peeled);
        let members = member_list(peeled);
        self.chains.push(Chain {
            line,
            rooted,
            members,
        });
    }

    fn skips_bare(&self, node: &Node<'_>, peeled: &Peeled) -> bool {
        let skipped = self.skip.as_ref();
        let range = node.byte_range();
        skipped == Some(&range) && no_root(peeled) && one_hop(peeled)
    }

    fn chain_rooted(&self, peeled: &Peeled) -> bool {
        let receiver = self.receiver;
        receiver.is_some_and(|name| root_is(peeled, name))
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
        self.levels.push(Decision {
            line,
            depth: self.structure + 1,
        });
    }

    fn inside(&mut self, f: impl FnOnce(&mut Self)) {
        self.descend(true, f);
    }

    fn descend(&mut self, inside: bool, f: impl FnOnce(&mut Self)) {
        if inside {
            self.structure += 1;
        }
        f(self);
        if inside {
            self.structure -= 1;
        }
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

fn forwarding_call<'a>(node: Node<'a>, source: &[u8]) -> Option<(u32, Node<'a>)> {
    match node.kind() {
        "function_definition" => {
            let body = node.child_by_field_name("body")?;
            let mut statements = named_nodes(body);
            if statements.first().copied().is_some_and(is_docstring) {
                statements.remove(0);
            }
            let statement = sole_forward(&statements, source)?;
            Some((line_of(statement), unwrap_call(statement, source)?))
        }
        "lambda" => {
            let body = node.child_by_field_name("body")?;
            let call = unwrap_call(body, source)?;
            Some((line_of(call), call))
        }
        _ => None,
    }
}

fn sole_forward<'a>(statements: &[Node<'a>], source: &[u8]) -> Option<Node<'a>> {
    let mut kept = Vec::new();
    for (index, statement) in statements.iter().copied().enumerate() {
        if is_dead_assignment(statement, source, &statements[index + 1..]) {
            continue;
        }
        kept.push(statement);
    }
    let has_hand_off = kept.iter().any(|statement| !is_logging(*statement, source));
    if has_hand_off {
        kept.retain(|statement| !is_logging(*statement, source));
    }
    let [statement] = kept.as_slice() else {
        return None;
    };
    Some(*statement)
}

fn is_logging(node: Node, source: &[u8]) -> bool {
    logging_leaf(node, source).is_some_and(|leaf| NOISE.contains(&leaf.as_str()))
}

fn logging_leaf(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "expression_statement" | "return_statement" | "parenthesized_expression" | "await" => {
            let children = named_nodes(node);
            let [child] = children.as_slice() else {
                return None;
            };
            logging_leaf(*child, source)
        }
        "call" => Some(callee_leaf(node.child_by_field_name("function")?, source)),
        _ => None,
    }
}

fn callee_leaf(node: Node, source: &[u8]) -> String {
    match node.kind() {
        "identifier" => node_text(node, source),
        "attribute" => node
            .child_by_field_name("attribute")
            .map(|attribute| callee_leaf(attribute, source))
            .unwrap_or_default(),
        "call" => node
            .child_by_field_name("function")
            .map(|function| callee_leaf(function, source))
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn is_dead_assignment(node: Node, source: &[u8], later: &[Node]) -> bool {
    let Some(assignment) = assignment_of(node) else {
        return false;
    };
    let names = assigned_names(assignment, source);
    !names.is_empty() && !names.iter().any(|name| mentioned(later, source, name))
}

fn assignment_of(node: Node) -> Option<Node> {
    if node.kind() == "assignment" {
        return Some(node);
    }
    if node.kind() != "expression_statement" {
        return None;
    }
    let children = named_nodes(node);
    let [child] = children.as_slice() else {
        return None;
    };
    if child.kind() == "assignment" {
        return Some(*child);
    }
    None
}

fn assigned_names(node: Node, source: &[u8]) -> Vec<String> {
    let Some(left) = node.child_by_field_name("left") else {
        return Vec::new();
    };
    let mut names = bound_names(left, source);
    if let Some(right) = node.child_by_field_name("right") {
        if right.kind() == "assignment" {
            names.extend(assigned_names(right, source));
        }
    }
    names
}

fn bound_names(node: Node, source: &[u8]) -> Vec<String> {
    match node.kind() {
        "identifier" => vec![node_text(node, source)],
        "pattern_list" | "tuple" | "list" | "parenthesized_list" => named_nodes(node)
            .into_iter()
            .flat_map(|child| bound_names(child, source))
            .collect(),
        _ => Vec::new(),
    }
}

fn mentioned(nodes: &[Node], source: &[u8], name: &str) -> bool {
    nodes.iter().any(|node| contains_name(*node, source, name))
}

fn contains_name(node: Node, source: &[u8], name: &str) -> bool {
    if node.kind() == "identifier" && node_text(node, source) == name {
        return true;
    }
    named_nodes(node)
        .into_iter()
        .any(|child| contains_name(child, source, name))
}

fn unwrap_call<'a>(node: Node<'a>, source: &[u8]) -> Option<Node<'a>> {
    match node.kind() {
        "call" => {
            if result_wrapper(node, source) {
                let argument = sole_argument(node)?;
                return unwrap_call(argument, source);
            }
            Some(node)
        }
        "return_statement" | "expression_statement" | "parenthesized_expression" | "await" => {
            let children = named_nodes(node);
            let [child] = children.as_slice() else {
                return None;
            };
            unwrap_call(*child, source)
        }
        _ => None,
    }
}

fn result_wrapper(node: Node, source: &[u8]) -> bool {
    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    RESULT.contains(&callee_leaf(function, source).as_str()) && sole_argument(node).is_some()
}

fn sole_argument(node: Node) -> Option<Node> {
    let arguments = node.child_by_field_name("arguments")?;
    if arguments.kind() != "argument_list" {
        return None;
    }
    let children = named_nodes(arguments);
    let [argument] = children.as_slice() else {
        return None;
    };
    if argument.kind() == "keyword_argument" {
        return None;
    }
    Some(*argument)
}

fn arguments_allowed(call: Node, source: &[u8], ordinary: &[String]) -> bool {
    let Some(arguments) = call.child_by_field_name("arguments") else {
        return true;
    };
    if arguments.kind() != "argument_list" {
        return true;
    }
    named_nodes(arguments)
        .into_iter()
        .all(|argument| argument_allowed(argument, source, ordinary))
}

fn argument_allowed(node: Node, source: &[u8], ordinary: &[String]) -> bool {
    let expr = if node.kind() == "keyword_argument" {
        node.child_by_field_name("value").unwrap_or(node)
    } else {
        node
    };
    if let Some(name) = argument_name(node, source) {
        return ordinary.iter().any(|item| item == &name);
    }
    is_literal(expr, source)
}

fn is_literal(node: Node, source: &[u8]) -> bool {
    if nested_literal(node, source) {
        return true;
    }
    if signed_literal(node, source) {
        return true;
    }
    plain_literal(node.kind())
}

fn nested_literal(node: Node, source: &[u8]) -> bool {
    if node.kind() != "parenthesized_expression" {
        return false;
    }
    literal_child(node, source)
}

fn literal_child(node: Node, source: &[u8]) -> bool {
    let children = named_nodes(node);
    let [child] = children.as_slice() else {
        return false;
    };
    is_literal(*child, source)
}

fn signed_literal(node: Node, source: &[u8]) -> bool {
    if node.kind() != "unary_operator" {
        return false;
    }
    let text = node_text(node, source);
    signed_text(&text) && numeric_operand(node)
}

fn signed_text(text: &str) -> bool {
    text.starts_with('-') || text.starts_with('+')
}

fn numeric_operand(node: Node) -> bool {
    let children = named_nodes(node);
    children.iter().any(|child| number_kind(child.kind()))
}

fn number_kind(kind: &str) -> bool {
    matches!(kind, "integer" | "float")
}

fn plain_literal(kind: &str) -> bool {
    matches!(
        kind,
        "integer" | "float" | "string" | "true" | "false" | "none" | "concatenated_string"
    )
}

fn contains_call(node: Node) -> bool {
    for index in 0..node.child_count() {
        if child_calls(node, index) {
            return true;
        }
    }
    false
}

fn child_calls(node: Node, index: u32) -> bool {
    let Some(child) = node.child(index) else {
        return false;
    };
    call_node(child)
}

fn call_node(child: Node) -> bool {
    child.kind() == "call" || contains_call(child)
}

fn forwarded_names(call: Node, source: &[u8], ordinary: &[String]) -> Vec<String> {
    let mut found = BTreeSet::new();
    collect_function_name(call, source, &mut found);
    collect_argument_names(call, source, &mut found);
    ordinary
        .iter()
        .filter(|name| found.contains(*name))
        .cloned()
        .collect()
}

fn collect_function_name(call: Node, source: &[u8], found: &mut BTreeSet<String>) {
    let Some(function) = call.child_by_field_name("function") else {
        return;
    };
    if function.kind() != "attribute" {
        return;
    }
    remember_object(function, source, found);
}

fn remember_object(function: Node, source: &[u8], found: &mut BTreeSet<String>) {
    let Some(object) = function.child_by_field_name("object") else {
        return;
    };
    let Some(name) = bare_name(object, source) else {
        return;
    };
    found.insert(name);
}

fn collect_argument_names(call: Node, source: &[u8], found: &mut BTreeSet<String>) {
    let Some(arguments) = call.child_by_field_name("arguments") else {
        return;
    };
    if arguments.kind() != "argument_list" {
        return;
    }
    remember_arguments(arguments, source, found);
}

fn remember_arguments(arguments: Node, source: &[u8], found: &mut BTreeSet<String>) {
    for argument in named_nodes(arguments) {
        remember_argument(argument, source, found);
    }
}

fn remember_argument(argument: Node, source: &[u8], found: &mut BTreeSet<String>) {
    let Some(name) = argument_name(argument, source) else {
        return;
    };
    found.insert(name);
}

fn argument_name(node: Node, source: &[u8]) -> Option<String> {
    if node.kind() == "keyword_argument" {
        return keyword_value(node, source);
    }
    if splat_kind(node.kind()) {
        return splat_name(node, source);
    }
    bare_name(node, source)
}

fn splat_kind(kind: &str) -> bool {
    matches!(kind, "list_splat" | "dictionary_splat")
}

fn keyword_value(node: Node, source: &[u8]) -> Option<String> {
    let value = node.child_by_field_name("value");
    value.and_then(|value| bare_name(value, source))
}

fn splat_name(node: Node, source: &[u8]) -> Option<String> {
    let children = named_nodes(node);
    let [child] = children.as_slice() else {
        return None;
    };
    bare_name(*child, source)
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
    kind_in_children(node, kind)
}

fn kind_in_children(node: Node, kind: &str) -> bool {
    for index in 0..node.child_count() {
        if child_kind(node, index, kind) {
            return true;
        }
    }
    false
}

fn child_kind(node: Node, index: u32, kind: &str) -> bool {
    let Some(child) = node.child(index) else {
        return false;
    };
    has_kind(child, kind)
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

fn shares_code_line(comment: Node) -> bool {
    let line = row_of(comment);
    let mut cursor = comment.prev_sibling();
    while let Some(sibling) = cursor {
        match earlier_comment(sibling, line) {
            CommentScan::Stop(shares) => return shares,
            CommentScan::Advance(next) => cursor = next,
        }
    }
    false
}

enum CommentScan<'a> {
    Stop(bool),
    Advance(Option<Node<'a>>),
}

fn earlier_comment(sibling: Node<'_>, line: usize) -> CommentScan<'_> {
    if sibling.kind() != "comment" {
        return CommentScan::Stop(code_shares_line(sibling, line));
    }
    if occupied_end(sibling) < line {
        return CommentScan::Stop(false);
    }
    CommentScan::Advance(sibling.prev_sibling())
}

fn code_shares_line(sibling: Node, line: usize) -> bool {
    let start = row_of(sibling);
    start <= line && occupied_end(sibling) >= line
}

fn occupied_end(node: Node) -> usize {
    let end = node.end_position();
    let start = node.start_position();
    if blank_line_end(end.column, end.row, start.row) {
        return end.row - 1;
    }
    end.row
}

fn blank_line_end(column: usize, end_row: usize, start_row: usize) -> bool {
    column == 0 && end_row > start_row
}

enum ChainStep<'a> {
    Done(bool),
    Continue(Node<'a>),
}

fn bare_call_roots_a_chain(node: Node) -> bool {
    let mut current = node;
    loop {
        let Some(parent) = current.parent() else {
            return false;
        };
        match step_chain(parent, current) {
            ChainStep::Done(rooted) => return rooted,
            ChainStep::Continue(next) => current = next,
        }
    }
}

fn step_chain<'a>(parent: Node<'a>, current: Node<'a>) -> ChainStep<'a> {
    match parent.kind() {
        "attribute" => ChainStep::Done(attribute_object(parent, current)),
        "subscript" => step_field(parent, current, "value"),
        "parenthesized_expression" | "await" => step_wrapper(parent, current),
        "call" => step_field(parent, current, "function"),
        _ => ChainStep::Done(false),
    }
}

fn attribute_object(parent: Node, current: Node) -> bool {
    parent.child_by_field_name("object") == Some(current)
}

fn step_field<'a>(parent: Node<'a>, current: Node<'a>, field: &str) -> ChainStep<'a> {
    if parent.child_by_field_name(field) != Some(current) {
        return ChainStep::Done(false);
    }
    ChainStep::Continue(parent)
}

fn step_wrapper<'a>(parent: Node<'a>, current: Node<'a>) -> ChainStep<'a> {
    if wraps_one(parent, current) {
        return ChainStep::Continue(parent);
    }
    ChainStep::Done(false)
}

fn wraps_one(parent: Node, current: Node) -> bool {
    let children = named_nodes(parent);
    one_child(&children, current)
}

fn one_child(children: &[Node], current: Node) -> bool {
    let count = children.len();
    let first = children.first();
    count == 1 && first == Some(&current)
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
        "identifier" => Some(identifier_peel(node, source)),
        "attribute" => peel_attribute(node, source),
        "call" => peel_call(node, source),
        "subscript" => peel_subscript(node, source),
        "parenthesized_expression" | "await" => peel_wrapped(node, source),
        _ => None,
    }
}

fn identifier_peel(node: Node, source: &[u8]) -> Peeled {
    Peeled {
        root: Some(node_text(node, source)),
        hops: Vec::new(),
    }
}

fn peel_attribute(node: Node, source: &[u8]) -> Option<Peeled> {
    let object = node.child_by_field_name("object")?;
    let mut peeled = peel(object, source).unwrap_or_else(empty_peel);
    let name = node.child_by_field_name("attribute")?;
    let label = node_text(name, source);
    let line = line_of(name);
    push_hop(
        &mut peeled,
        Hop {
            name: label,
            call: false,
            line,
        },
    );
    Some(peeled)
}

fn peel_call(node: Node, source: &[u8]) -> Option<Peeled> {
    let function = node.child_by_field_name("function")?;
    if let Some(name) = bare_name(function, source) {
        return named_call(node, function, name);
    }
    called_attribute(function, source)
}

fn named_call(node: Node, function: Node, name: String) -> Option<Peeled> {
    if bare_call_roots_a_chain(node) {
        return Some(empty_peel());
    }
    Some(call_hop(name, function))
}

fn call_hop(name: String, function: Node) -> Peeled {
    Peeled {
        root: None,
        hops: vec![Hop {
            name,
            call: true,
            line: line_of(function),
        }],
    }
}

fn called_attribute(function: Node, source: &[u8]) -> Option<Peeled> {
    let mut peeled = peel(function, source)?;
    let hops = &mut peeled.hops;
    let last = hops.last_mut()?;
    last.call = true;
    Some(peeled)
}

fn peel_subscript(node: Node, source: &[u8]) -> Option<Peeled> {
    let value = node.child_by_field_name("value")?;
    peel(value, source)
}

fn peel_wrapped(node: Node, source: &[u8]) -> Option<Peeled> {
    let children = named_nodes(node);
    let [child] = children.as_slice() else {
        return None;
    };
    peel(*child, source)
}

fn empty_peel() -> Peeled {
    Peeled {
        root: None,
        hops: Vec::new(),
    }
}

fn push_hop(peeled: &mut Peeled, hop: Hop) {
    let hops = &mut peeled.hops;
    hops.push(hop);
}

fn hop_list_empty(peeled: &Peeled) -> bool {
    let hops = &peeled.hops;
    hops.is_empty()
}

fn no_root(peeled: &Peeled) -> bool {
    let root = &peeled.root;
    root.is_none()
}

fn one_hop(peeled: &Peeled) -> bool {
    let hops = &peeled.hops;
    hops.len() == 1
}

fn first_hop_line(peeled: &Peeled) -> u32 {
    let hops = &peeled.hops;
    let hop = &hops[0];
    hop.line
}

fn member_list(peeled: Peeled) -> Vec<Member> {
    let hops = peeled.hops;
    hops.into_iter()
        .map(|hop| Member {
            name: hop.name,
            call: hop.call,
        })
        .collect()
}

fn root_is(peeled: &Peeled, receiver: &str) -> bool {
    let root = peeled.root.as_deref();
    root == Some(receiver)
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

fn operator_line(node: Node) -> u32 {
    let operator = node.child_by_field_name("operator");
    operator.map(line_of).unwrap_or_else(|| line_of(node))
}

fn line_of(node: Node) -> u32 {
    row_of(node) as u32 + 1
}

fn row_of(node: Node) -> usize {
    let position = node.start_position();
    position.row
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

const IO_NAMES: &[&str] = &[
    "open",
    "sqlite3.connect",
    "socket.socket",
    "subprocess.run",
    "subprocess.Popen",
    "urllib.request.urlopen",
    "requests.Session",
    "httpx.Client",
    "httpx.AsyncClient",
];

struct IoWalk<'a> {
    source: &'a [u8],
    found: &'a mut Vec<CallSite>,
}

fn record_io(node: Node, source: &[u8]) -> Vec<CallSite> {
    let mut found = Vec::new();
    let mut walk = IoWalk {
        source,
        found: &mut found,
    };
    walk_io(node, &mut walk, true);
    found
}

fn walk_io(node: Node, walk: &mut IoWalk<'_>, root: bool) {
    if skipped_io(node, root) {
        return;
    }
    note_io(node, walk);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_io(child, walk, false);
    }
}

fn skipped_io(node: Node, root: bool) -> bool {
    !root && node.kind() == "function_definition"
}

fn note_io(node: Node, walk: &mut IoWalk<'_>) {
    let Some(name) = io_name(node, walk.source) else {
        return;
    };
    if !io_listed(&name) {
        return;
    }
    let line = line_of(node);
    let found = &mut walk.found;
    found.push(CallSite { line, name });
}

fn io_listed(name: &str) -> bool {
    IO_NAMES.contains(&name)
}

fn io_name(node: Node, source: &[u8]) -> Option<String> {
    if node.kind() != "call" {
        return None;
    }
    let function = node.child_by_field_name("function")?;
    written_name(function, source)
}

fn written_name(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => Some(node_text(node, source)),
        "attribute" => dotted_name(node, source),
        _ => None,
    }
}

fn dotted_name(node: Node, source: &[u8]) -> Option<String> {
    let object = node.child_by_field_name("object")?;
    let left = written_name(object, source)?;
    let attribute = node.child_by_field_name("attribute")?;
    let right = node_text(attribute, source);
    Some(format!("{left}.{right}"))
}

const MUTATORS: &[&str] = &[
    "append",
    "extend",
    "insert",
    "pop",
    "remove",
    "clear",
    "sort",
    "reverse",
    "update",
    "setdefault",
    "popitem",
];

struct MutationWalk<'a> {
    source: &'a [u8],
    names: &'a [String],
    found: &'a mut Vec<u32>,
}

fn record_mutations(node: Node, source: &[u8], ordinary: &[String]) -> Vec<u32> {
    if node.kind() == "lambda" {
        return Vec::new();
    }
    if returns_value(node) {
        return Vec::new();
    }
    let mut found = Vec::new();
    let mut walk = MutationWalk {
        source,
        names: ordinary,
        found: &mut found,
    };
    walk_mutations(node, &mut walk, true);
    found
}

fn returns_value(node: Node) -> bool {
    let Some(body) = node.child_by_field_name("body") else {
        return false;
    };
    value_return(body)
}

fn value_return(node: Node) -> bool {
    if node.kind() == "function_definition" {
        return false;
    }
    if carrying_return(node) {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if value_return(child) {
            return true;
        }
    }
    false
}

fn carrying_return(node: Node) -> bool {
    if node.kind() != "return_statement" {
        return false;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if returned_value(child) {
            return true;
        }
    }
    false
}

fn returned_value(child: Node) -> bool {
    if !child.is_named() {
        return false;
    }
    child.kind() != "comment"
}

fn walk_mutations(node: Node, walk: &mut MutationWalk<'_>, root: bool) {
    if skipped_mutation(node, root) {
        return;
    }
    note_mutation(node, walk);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_mutations(child, walk, false);
    }
}

fn skipped_mutation(node: Node, root: bool) -> bool {
    if root {
        return false;
    }
    node.kind() == "function_definition"
}

fn note_mutation(node: Node, walk: &mut MutationWalk<'_>) {
    if stored_mutation(node, walk) {
        return;
    }
    called_mutation(node, walk);
}

fn stored_mutation(node: Node, walk: &mut MutationWalk<'_>) -> bool {
    if !stores(node) {
        return false;
    }
    let Some(left) = node.child_by_field_name("left") else {
        return false;
    };
    if !mutated_target(left, walk) {
        return false;
    }
    let line = line_of(node);
    let found = &mut walk.found;
    found.push(line);
    true
}

fn stores(node: Node) -> bool {
    if node.kind() == "augmented_assignment" {
        return true;
    }
    if node.kind() != "assignment" {
        return false;
    }
    let right = node.child_by_field_name("right");
    right.is_some()
}

fn called_mutation(node: Node, walk: &mut MutationWalk<'_>) {
    if node.kind() != "call" {
        return;
    }
    let Some(function) = node.child_by_field_name("function") else {
        return;
    };
    if !mutator_call(function, walk) {
        return;
    }
    let line = line_of(node);
    let found = &mut walk.found;
    found.push(line);
}

fn mutator_call(function: Node, walk: &MutationWalk<'_>) -> bool {
    if function.kind() != "attribute" {
        return false;
    }
    let Some(object) = function.child_by_field_name("object") else {
        return false;
    };
    let Some(attribute) = function.child_by_field_name("attribute") else {
        return false;
    };
    if !param_name(object, walk) {
        return false;
    }
    mutator(&node_text(attribute, walk.source))
}

fn mutator(name: &str) -> bool {
    MUTATORS.contains(&name)
}

fn mutated_target(node: Node, walk: &MutationWalk<'_>) -> bool {
    if attribute_or_subscript(node) {
        return rooted_on_param(node, walk);
    }
    if pattern_group(node) {
        return any_mutated(node, walk);
    }
    false
}

fn attribute_or_subscript(node: Node) -> bool {
    let kind = node.kind();
    kind == "attribute" || kind == "subscript"
}

fn pattern_group(node: Node) -> bool {
    let kind = node.kind();
    if kind == "pattern_list" {
        return true;
    }
    if kind == "tuple_pattern" {
        return true;
    }
    if kind == "list_pattern" {
        return true;
    }
    kind == "list_splat_pattern"
}

fn any_mutated(node: Node, walk: &MutationWalk<'_>) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if mutated_target(child, walk) {
            return true;
        }
    }
    false
}

fn rooted_on_param(node: Node, walk: &MutationWalk<'_>) -> bool {
    let node = opened(node);
    if node.kind() == "identifier" {
        return param_name(node, walk);
    }
    let Some(inner) = target_inner(node) else {
        return false;
    };
    rooted_on_param(inner, walk)
}

fn target_inner(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "attribute" {
        return node.child_by_field_name("object");
    }
    if node.kind() == "subscript" {
        return node.child_by_field_name("value");
    }
    None
}

fn param_name(node: Node, walk: &MutationWalk<'_>) -> bool {
    let node = opened(node);
    if node.kind() != "identifier" {
        return false;
    }
    listed(&node_text(node, walk.source), walk.names)
}

fn opened(node: Node<'_>) -> Node<'_> {
    if node.kind() != "parenthesized_expression" {
        return node;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.is_named() {
            return opened(child);
        }
    }
    node
}

fn listed(name: &str, names: &[String]) -> bool {
    if !kept_name(name) {
        return false;
    }
    let mut kept = names.iter();
    kept.any(|item| item == name)
}

fn kept_name(name: &str) -> bool {
    if name == "self" {
        return false;
    }
    name != "cls"
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
    fn a_broken_line_does_not_mark_the_functions_after_it() {
        let source = "\
def ok():
    return (
def also():
    return 1
def more():
    return 2
";
        let file = parse("wrap.py", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 1.0 / 6.0);
        assert_eq!(file.functions.len(), 3);
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
                Decision { line: 6, depth: 2 },
                Decision { line: 10, depth: 1 },
                Decision { line: 12, depth: 1 },
                Decision { line: 18, depth: 1 },
                Decision { line: 18, depth: 2 },
                Decision { line: 18, depth: 2 },
            ]
        );
        assert!(paths.body.cognitive.iter().any(|item| {
            item.kind == CognitiveKind::Hybrid && (item.line == 5 || item.line == 14)
        }));
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
        assert!(function(&file, "field").body.forward.is_none());
        let field = function(&file, "field");
        assert!(field
            .body
            .chains
            .iter()
            .any(|chain| member_names(chain) == ["b"]));
        assert!(field
            .body
            .chains
            .iter()
            .any(|chain| member_names(chain) == ["f"] && chain.members[0].call));
        assert!(function(&file, "none").body.forward.is_none());
    }

    #[test]
    fn a_pass_through_ignores_logging_and_a_dead_assignment() {
        let source = "\
def logged(a, b, c):
    log(a)
    return run(a, b, c)

def dead(a, b):
    unused = 1
    return run(a, b)

def alias(a):
    tmp = a
    return f(tmp)

def wrapped(a, b):
    return Ok(run(a, b))

def layered(a, b):
    return Ok(Some(run(a, b)))

def boxed(a, b):
    return Box.new(run(a, b))

def keyed(xs):
    return sorted(xs, key=str.lower)

def keyed_lambda(xs):
    return sorted(xs, key=lambda x: x)

def literal(xs):
    return sorted(xs, reverse=True)

def negative(a):
    return send(a, -1)

def method(repo, user):
    return repo.save(user)
";
        let file = parse("a.py", source.as_bytes());
        assert_eq!(
            function(&file, "logged")
                .body
                .forward
                .as_ref()
                .unwrap()
                .names,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
        assert_eq!(
            function(&file, "dead").body.forward.as_ref().unwrap().names,
            vec!["a".to_string(), "b".to_string()]
        );
        assert!(function(&file, "alias").body.forward.is_none());
        assert_eq!(
            function(&file, "wrapped")
                .body
                .forward
                .as_ref()
                .unwrap()
                .names,
            vec!["a".to_string(), "b".to_string()]
        );
        assert_eq!(
            function(&file, "layered")
                .body
                .forward
                .as_ref()
                .unwrap()
                .names,
            vec!["a".to_string(), "b".to_string()]
        );
        assert!(function(&file, "boxed").body.forward.is_none());
        assert!(function(&file, "keyed").body.forward.is_none());
        assert!(function(&file, "keyed_lambda").body.forward.is_none());
        assert_eq!(
            function(&file, "literal")
                .body
                .forward
                .as_ref()
                .unwrap()
                .names,
            vec!["xs".to_string()]
        );
        assert_eq!(
            function(&file, "negative")
                .body
                .forward
                .as_ref()
                .unwrap()
                .names,
            vec!["a".to_string()]
        );
        assert_eq!(
            function(&file, "method")
                .body
                .forward
                .as_ref()
                .unwrap()
                .names,
            vec!["repo".to_string(), "user".to_string()]
        );
    }

    #[test]
    fn nesting_is_the_deepest_structural_level() {
        let cases = [
            (
                "\
def boolean(a, b):
    return a and b
",
                "boolean",
                0,
                None,
            ),
            (
                "\
def conditional(a, b):
    if a and b:
        return 1
",
                "conditional",
                1,
                Some(2),
            ),
            (
                "\
def ternary_bool(a, b):
    return 1 if a and b else 0
",
                "ternary_bool",
                1,
                Some(2),
            ),
            (
                "\
def in_lambda(a):
    return lambda y: y if a else 0
",
                "in_lambda",
                1,
                Some(2),
            ),
            (
                "\
def nested_keeps_its_own(a):
    def inner(x):
        if x:
            return x
    return a
",
                "nested_keeps_its_own",
                0,
                None,
            ),
            (
                "\
def nested_keeps_its_own(a):
    def inner(x):
        if x:
            return x
    return a
",
                "nested_keeps_its_own.inner",
                1,
                Some(3),
            ),
            (
                "\
def chained(a, b):
    return 1 if a else (
        2 if b else 3
    )
",
                "chained",
                2,
                Some(3),
            ),
            (
                "\
def in_condition(a):
    if 1 if a else 0:
        return 1
",
                "in_condition",
                1,
                Some(2),
            ),
            (
                "\
def operand(a):
    return 1 if (2 if a else 0) else 3
",
                "operand",
                1,
                Some(2),
            ),
            (
                "\
def in_body(a, b):
    if a:
        return 1 if b else 0
",
                "in_body",
                2,
                Some(3),
            ),
            (
                "\
def if_else(a):
    if a:
        return 1
    else:
        return 0
",
                "if_else",
                1,
                Some(2),
            ),
            (
                "\
def if_elif(a, b):
    if a:
        return 1
    elif b:
        return 2
    else:
        return 0
",
                "if_elif",
                1,
                Some(2),
            ),
            (
                "\
def else_holds_if(a, b):
    if a:
        return 1
    else:
        if b:
            return 2
",
                "else_holds_if",
                2,
                Some(5),
            ),
            (
                "\
def guard(x):
    match x:
        case y if y > 0:
            return 1
",
                "guard",
                1,
                Some(2),
            ),
            (
                "\
def arm_if(x):
    match x:
        case y:
            if y:
                return 1
",
                "arm_if",
                2,
                Some(4),
            ),
            (
                "\
def comprehension(xs):
    return [y for y in xs if y > 0]
",
                "comprehension",
                0,
                None,
            ),
            (
                "\
def comprehension_ternary(xs):
    return [1 if y else 0 for y in xs]
",
                "comprehension_ternary",
                1,
                Some(2),
            ),
            (
                "\
def try_if(a):
    try:
        if a:
            return 1
    except Exception:
        return 0
",
                "try_if",
                1,
                Some(3),
            ),
            (
                "\
def except_if(a):
    try:
        return 0
    except Exception:
        if a:
            return 1
",
                "except_if",
                2,
                Some(5),
            ),
            (
                "\
def with_if(a):
    with a:
        if a:
            return 1
",
                "with_if",
                1,
                Some(3),
            ),
            (
                "\
def finally_if(a):
    try:
        return 0
    finally:
        if a:
            return 1
",
                "finally_if",
                1,
                Some(5),
            ),
            (
                "\
def two_handlers(a):
    try:
        return 0
    except TypeError:
        return 1
    except ValueError:
        if a:
            return 2
",
                "two_handlers",
                2,
                Some(7),
            ),
            (
                "\
def nested_handlers(a):
    try:
        return 0
    except Exception:
        try:
            return 1
        except Exception:
            if a:
                return 2
",
                "nested_handlers",
                3,
                Some(8),
            ),
            (
                "\
def try_else(a):
    try:
        return 0
    except Exception:
        return 1
    else:
        if a:
            return 2
",
                "try_else",
                1,
                Some(4),
            ),
            (
                "\
def loop_else(xs):
    for x in xs:
        return x
    else:
        if xs:
            return 0
",
                "loop_else",
                2,
                Some(5),
            ),
            (
                "\
def tied(a, b):
    if a:
        if b:
            return 1
    else:
        if b:
            return 2
",
                "tied",
                2,
                Some(3),
            ),
        ];
        for (source, name, depth, line) in cases {
            assert_eq!(structural_nesting(source, name), (depth, line), "{name}");
        }
    }

    fn structural_nesting(source: &str, name: &str) -> (u32, Option<u32>) {
        let file = parse("nesting.py", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{name}\n{source}");
        let measured = crate::analyzers::measure(&file, &[]);
        let found = measured
            .iter()
            .find(|item| item.name == name)
            .unwrap_or_else(|| panic!("missing {name}"));
        (found.nesting, found.nesting_line)
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

    #[test]
    fn a_bare_call_that_starts_a_member_chain_is_not_a_hop() {
        let source = "\
def call_result():
    return make().a.b.c

def wrapped():
    return (make()).a.b

def method_call(obj):
    return obj.method().extra

def calls(a):
    send()
    return a
";
        let file = parse("a.py", source.as_bytes());
        let call_result = function(&file, "call_result");
        assert_eq!(
            member_names(&call_result.body.chains[0]),
            vec!["a", "b", "c"]
        );
        assert!(call_result.body.chains[0]
            .members
            .iter()
            .all(|member| !member.call));
        assert_eq!(
            member_names(&function(&file, "wrapped").body.chains[0]),
            vec!["a", "b"]
        );
        let method_call = function(&file, "method_call");
        assert_eq!(
            member_names(&method_call.body.chains[0]),
            vec!["method", "extra"]
        );
        assert!(method_call.body.chains[0].members[0].call);
        assert!(!method_call.body.chains[0].members[1].call);
        assert_eq!(
            member_names(&function(&file, "calls").body.chains[0]),
            vec!["send"]
        );
    }

    #[test]
    fn a_lambda_in_a_default_or_decorator_counts_on_that_function() {
        let source = "\
def defaults(a, key=lambda x: x if x and x > 1 else 0):
    return a

@register(lambda x: x if x else 0, when=a and b)
def decorated(a):
    return a

def outer():
    def inner(key=lambda x: x if x else 0):
        return key
    return inner
";
        let file = parse("a.py", source.as_bytes());
        let defaults = function(&file, "defaults");
        assert_eq!(defaults.body.lloc, 2);
        assert_eq!(
            defaults.body.decisions,
            vec![
                Decision { line: 1, depth: 2 },
                Decision { line: 1, depth: 3 },
            ]
        );
        let decorated = function(&file, "decorated");
        assert_eq!(decorated.body.lloc, 2);
        assert_eq!(
            decorated.body.decisions,
            vec![
                Decision { line: 4, depth: 2 },
                Decision { line: 4, depth: 1 },
            ]
        );
        assert!(decorated.body.chains.is_empty());
        let outer = function(&file, "outer");
        assert_eq!(outer.body.lloc, 2);
        assert!(outer.body.decisions.is_empty());
        let inner = function(&file, "outer.inner");
        assert_eq!(inner.body.lloc, 2);
        assert_eq!(inner.body.decisions, vec![Decision { line: 9, depth: 2 }]);
    }

    #[test]
    fn a_trailing_comment_on_a_code_line_is_not_a_waiver() {
        let source = "\
x = 1  # slopmeter: waive F01 12 trailing on code line
def after_trailing(a):
    return a

# slopmeter: waive F01 12 ok
def whole_line(a):
    return a
";
        let file = parse("a.py", source.as_bytes());
        let after = function(&file, "after_trailing");
        assert!(after.waivers.is_empty());
        assert_eq!(after.touch.start, after.span.start);
        assert_eq!(
            function(&file, "whole_line").waivers,
            vec![waiver(5, Some("F01"), Some("12"), Some("ok"))]
        );
    }

    fn member_names(chain: &crate::ir::Chain) -> Vec<&str> {
        chain
            .members
            .iter()
            .map(|member| member.name.as_str())
            .collect()
    }

    #[test]
    fn a_run_of_waiver_lines_joins_the_touch_span_and_not_the_body_span() {
        let source = "\
# slopmeter: waive F06 40 The transitions belong in one function.
# slopmeter: waive F01 12 The match arms are the states.
def reduce(state, event):
    return state
";
        let file = parse("src/reduce.py", source.as_bytes());
        let reduce = function(&file, "reduce");
        assert_eq!(
            reduce.waivers,
            vec![
                waiver(
                    1,
                    Some("F06"),
                    Some("40"),
                    Some("The transitions belong in one function.")
                ),
                waiver(
                    2,
                    Some("F01"),
                    Some("12"),
                    Some("The match arms are the states.")
                ),
            ]
        );
        assert_eq!(reduce.span, crate::ir::Span { start: 3, end: 4 });
        assert_eq!(reduce.touch, crate::ir::Span { start: 1, end: 4 });
    }

    #[test]
    fn a_waiver_above_a_decorator_joins_the_touch_span() {
        let source = "\
# slopmeter: waive F06 25 The parser is one table-driven loop.
@decorator
def parse(src):
    return src
";
        let file = parse("src/parse.py", source.as_bytes());
        let parse = function(&file, "parse");
        assert_eq!(
            parse.waivers,
            vec![waiver(
                1,
                Some("F06"),
                Some("25"),
                Some("The parser is one table-driven loop.")
            )]
        );
        assert_eq!(parse.span.start, 3);
        assert_eq!(parse.touch.start, 1);
    }

    #[test]
    fn a_waiver_between_a_decorator_and_the_signature_does_not_attach() {
        let source = "\
@decorator
# slopmeter: waive F06 25 The parser is one table-driven loop.
def parse(src):
    return src
";
        let file = parse("src/parse.py", source.as_bytes());
        let parse = function(&file, "parse");
        assert!(parse.waivers.is_empty());
        assert_eq!(parse.touch.start, 1);
    }

    #[test]
    fn a_blank_line_ends_the_waiver_run() {
        let source = "\
# slopmeter: waive F06 25 The parser is one table-driven loop.

def parse(src):
    return src
";
        let file = parse("src/parse.py", source.as_bytes());
        assert!(function(&file, "parse").waivers.is_empty());
    }

    #[test]
    fn an_ordinary_comment_ends_the_waiver_run() {
        let source = "\
# slopmeter: waive F06 25 The parser is one table-driven loop.
# TODO split later
def parse(src):
    return src
";
        let file = parse("src/parse.py", source.as_bytes());
        assert!(function(&file, "parse").waivers.is_empty());
    }

    #[test]
    fn a_comment_inside_the_body_does_not_waive_that_function() {
        let source = "\
def parse(src):
    # slopmeter: waive F06 25 The parser is one table-driven loop.
    return src
";
        let file = parse("src/parse.py", source.as_bytes());
        let parse = function(&file, "parse");
        assert!(parse.waivers.is_empty());
        assert_eq!(parse.span, crate::ir::Span { start: 1, end: 3 });
    }

    #[test]
    fn a_waiver_above_a_nested_function_attaches_to_the_inner_function() {
        let source = "\
def outer():
    # slopmeter: waive F06 30 The inner parse is one table.
    def inner():
        return 1
    return inner
";
        let file = parse("src/outer.py", source.as_bytes());
        let outer = function(&file, "outer");
        let inner = function(&file, "outer.inner");
        assert!(outer.waivers.is_empty());
        assert!(outer.span.start < 2 && outer.span.end >= 2);
        assert_eq!(
            inner.waivers,
            vec![waiver(
                2,
                Some("F06"),
                Some("30"),
                Some("The inner parse is one table.")
            )]
        );
        assert_eq!(inner.span.start, 3);
        assert_eq!(inner.touch.start, 2);
    }

    #[test]
    fn an_incomplete_waiver_line_stays_in_the_run() {
        let source = "\
# slopmeter: waive F01 12 The match arms are the states.
# slopmeter: waive F06
def reduce(state, event):
    return state
";
        let file = parse("src/reduce.py", source.as_bytes());
        assert_eq!(
            function(&file, "reduce").waivers,
            vec![
                waiver(
                    1,
                    Some("F01"),
                    Some("12"),
                    Some("The match arms are the states.")
                ),
                waiver(2, Some("F06"), None, None),
            ]
        );
    }

    #[test]
    fn a_marker_with_no_rule_is_still_recorded() {
        let source = "\
# slopmeter: waive
def short():
    return 1
";
        let file = parse("src/short.py", source.as_bytes());
        assert_eq!(
            function(&file, "short").waivers,
            vec![waiver(1, None, None, None)]
        );
    }

    #[test]
    fn a_fractional_number_is_kept_as_written() {
        let source = "\
# slopmeter: waive F02 8.5 Coverage is thin on the error arms.
def parse(src):
    return src
";
        let file = parse("src/parse.py", source.as_bytes());
        assert_eq!(
            function(&file, "parse").waivers,
            vec![waiver(
                1,
                Some("F02"),
                Some("8.5"),
                Some("Coverage is thin on the error arms.")
            )]
        );
    }

    #[test]
    fn a_named_lambda_keeps_the_waiver_above_its_assignment() {
        let source = "\
# slopmeter: waive F04 5 The callback takes the framework args.
bound = lambda z: z
";
        let file = parse("src/bound.py", source.as_bytes());
        let bound = function(&file, "bound");
        assert_eq!(
            bound.waivers,
            vec![waiver(
                1,
                Some("F04"),
                Some("5"),
                Some("The callback takes the framework args.")
            )]
        );
        assert_eq!(bound.touch.start, 1);
        assert_eq!(bound.span.start, 2);
    }

    fn waiver(
        line: u32,
        rule: Option<&str>,
        number: Option<&str>,
        reason: Option<&str>,
    ) -> crate::ir::WaiverLine {
        crate::ir::WaiverLine {
            line,
            rule: rule.map(str::to_string),
            number: number.map(str::to_string),
            reason: reason.map(str::to_string),
        }
    }

    fn function<'a>(file: &'a crate::ir::File, name: &str) -> &'a crate::ir::Function {
        file.functions
            .iter()
            .find(|function| function.name == name)
            .unwrap_or_else(|| panic!("missing {name}"))
    }

    #[test]
    fn an_except_that_does_nothing_records_its_line() {
        let cases = [
            (
                "\
def load(path):
    try:
        path.read()
    except Exception:
        pass
",
                vec![4],
            ),
            (
                "\
def load(path):
    try:
        path.read()
    except Exception:
        ...
",
                vec![4],
            ),
            (
                "\
def load(items):
    for item in items:
        try:
            item.read()
        except Exception:
            continue
",
                vec![5],
            ),
            (
                "\
def load(items):
    for item in items:
        try:
            item.read()
        except Exception:
            break
",
                vec![5],
            ),
            (
                "\
def load(path):
    try:
        path.read()
    except Exception:
        # swallowed
        pass
",
                vec![4],
            ),
            (
                "\
def load(path):
    try:
        path.read()
    except Exception:
        # swallowed
",
                vec![4],
            ),
            (
                "\
def load(path):
    try:
        path.read()
    except Exception as err:
        pass
",
                vec![4],
            ),
            (
                "\
def load(path):
    try:
        path.read()
    except TypeError:
        pass
    except ValueError:
        ...
",
                vec![4, 6],
            ),
        ];
        for (source, lines) in cases {
            let file = parse("a.py", source.as_bytes());
            assert_eq!(file.unparsed_ratio, 0.0, "{source}");
            assert_eq!(file.functions[0].body.swallowing, lines, "{source}");
        }
    }

    #[test]
    fn an_except_that_handles_the_error_records_nothing() {
        let cases = [
            "\
def load(path):
    try:
        path.read()
    except Exception:
        return None
",
            "\
def load(path):
    try:
        path.read()
    except Exception:
        raise
",
            "\
def load(path):
    try:
        path.read()
    except Exception as err:
        err = None
",
            "\
def load(path):
    try:
        path.read()
    except Exception:
        log(path)
",
            "\
def load(path):
    try:
        path.read()
    except Exception:
        pass
        return None
",
        ];
        for source in cases {
            let file = parse("a.py", source.as_bytes());
            assert_eq!(file.unparsed_ratio, 0.0, "{source}");
            assert!(file.functions[0].body.swallowing.is_empty(), "{source}");
        }
    }

    #[test]
    fn a_swallowing_except_stays_on_the_function_that_writes_it() {
        let source = "\
def outer():
    def inner():
        try:
            read()
        except Exception:
            pass
    return inner
";
        let file = parse("a.py", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0);
        assert!(function(&file, "outer").body.swallowing.is_empty());
        assert_eq!(function(&file, "outer.inner").body.swallowing, vec![5]);
    }

    #[test]
    fn an_inline_open_stays_on_the_function_that_writes_it() {
        let source = "\
def outer(path):
    def inner():
        return sqlite3.connect(path)
    handle = open(path)
    return requests.get(path)
";
        let file = parse("a.py", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{source}");
        let outer = &function(&file, "outer").body.io;
        assert_eq!(outer.len(), 1, "{outer:?}");
        assert_eq!(outer[0].name, "open");
        assert_eq!(outer[0].line, 4);
        let inner = &function(&file, "outer.inner").body.io;
        assert_eq!(inner.len(), 1, "{inner:?}");
        assert_eq!(inner[0].name, "sqlite3.connect");
        assert_eq!(inner[0].line, 3);
    }

    #[test]
    fn a_void_function_records_a_parameter_mutation() {
        let source = "\
def outer(items, mapping):
    def inner(chunk):
        chunk.append(1)
    items.append(2)
    items[0] = 3
    items.x += 4
    mapping.update(5)
    head, items.y = 1, 2
    return

def give(items):
    items.append(1)
    return items
";
        let file = parse("a.py", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{source}");
        assert_eq!(function(&file, "outer").body.mutations, vec![4, 5, 6, 7, 8]);
        assert_eq!(function(&file, "outer.inner").body.mutations, vec![3]);
        assert!(function(&file, "give").body.mutations.is_empty());
    }

    #[test]
    fn self_cls_and_a_rebound_name_are_not_mutations() {
        let source = "\
def rebound(items):
    items = []
    other.append(items)

def method(self, items):
    self.x = 1
    items.box.append(1)

def named(self):
    self.append(1)

def factory(cls):
    cls.update(1)

bound = lambda items: items.append(1)

class Box:
    def take(self, items):
        self.x = 1
        items.append(2)
";
        let file = parse("a.py", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{source}");
        assert!(function(&file, "rebound").body.mutations.is_empty());
        assert!(function(&file, "method").body.mutations.is_empty());
        assert!(function(&file, "named").body.mutations.is_empty());
        assert!(function(&file, "factory").body.mutations.is_empty());
        assert!(function(&file, "bound").body.mutations.is_empty());
        assert_eq!(function(&file, "Box.take").body.mutations, vec![20]);
    }

    #[test]
    fn a_none_literal_records_nothing() {
        let source = "\
def missing(items):
    take(None)
    return None
";
        let file = parse("a.py", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{source}");
        assert!(function(&file, "missing").body.nulls.is_empty());
    }
}
