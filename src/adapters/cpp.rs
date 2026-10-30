use std::collections::BTreeSet;
use std::ops::Range;

use tree_sitter::Node;

use crate::ir::{
    Body, CallSite, Chain, Cognitive, CognitiveKind, Decision, File, Forward, Function, Language,
    Member, Param, Span, WaiverLine,
};

pub const ALLOW: &[&str] = &[
    "begin",
    "end",
    "cbegin",
    "cend",
    "rbegin",
    "rend",
    "crbegin",
    "crend",
    "size",
    "empty",
    "at",
    "front",
    "back",
    "data",
    "find",
    "c_str",
    "value",
    "value_or",
    "has_value",
];

const IDENTITY: &[&str] = &["move", "forward"];
const NOISE: &[&str] = &[
    "log", "logger", "debug", "info", "warn", "warning", "error", "trace", "print", "println",
    "eprintln", "dbg",
];
const RESULT: &[&str] = &["Ok", "Some", "Err"];

enum Scope {
    Namespace(String),
    Class(String),
    Function(String),
}

struct Scan<'a> {
    source: &'a [u8],
    scopes: &'a mut Vec<Scope>,
    functions: &'a mut Vec<Function>,
}

struct Defined<'a> {
    name: String,
    item: Node<'a>,
    body: Node<'a>,
    params: Option<Node<'a>>,
}

struct Score<'a> {
    source: &'a [u8],
    receiver: Option<&'a str>,
    ordinary: &'a [String],
}

#[derive(Clone, Copy)]
struct Preproc<'a> {
    condition: Option<Node<'a>>,
    alternative: Option<Node<'a>>,
}

#[derive(Clone, Copy)]
struct NameQuery<'a> {
    source: &'a [u8],
    name: &'a str,
}

struct ParenCursor {
    depth: i32,
    index: usize,
    len: usize,
}

pub fn parse(path: &str, source: &[u8]) -> File {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_cpp::LANGUAGE.into())
        .expect("cpp grammar");
    let Some(tree) = parser.parse(source, None) else {
        return File {
            path: path.to_string(),
            language: Language::Cpp,
            unparsed_ratio: 1.0,
            unscored_macros: 0,
            functions: Vec::new(),
        };
    };
    let root = tree.root_node();
    let mut functions = Vec::new();
    let mut scopes = Vec::new();
    let mut scan = Scan {
        source,
        scopes: &mut scopes,
        functions: &mut functions,
    };
    scan.visit(root);
    File {
        path: path.to_string(),
        language: Language::Cpp,
        unparsed_ratio: super::unparsed_ratio(root, source),
        unscored_macros: unscored_macros(root, source),
        functions,
    }
}

impl<'a> Scan<'a> {
    fn visit(&mut self, node: Node<'a>) {
        match node.kind() {
            "function_definition" => self.visit_function(node),
            "namespace_definition" => self.visit_namespace(node),
            "class_specifier" | "struct_specifier" | "union_specifier" => self.visit_class(node),
            "declaration" => self.visit_declaration(node),
            _ => self.visit_rest(node),
        }
    }

    fn visit_rest(&mut self, node: Node<'a>) {
        if self.visit_preproc_kind(node) {
            return;
        }
        self.visit_children(node);
    }

    fn visit_preproc_kind(&mut self, node: Node<'a>) -> bool {
        let kind = node.kind();
        if preproc_checks_zero(kind) {
            self.visit_preproc(node, true);
            return true;
        }
        if preproc_skips_zero(kind) {
            self.visit_preproc(node, false);
            return true;
        }
        skipped_preproc(kind)
    }

    fn visit_children(&mut self, node: Node<'a>) {
        for child in named_nodes(node) {
            self.visit(child);
        }
    }
}

fn preproc_checks_zero(kind: &str) -> bool {
    matches!(kind, "preproc_if" | "preproc_elif")
}

fn preproc_skips_zero(kind: &str) -> bool {
    matches!(kind, "preproc_ifdef" | "preproc_elifdef" | "preproc_else")
}

fn skipped_preproc(kind: &str) -> bool {
    matches!(
        kind,
        "preproc_def" | "preproc_function_def" | "preproc_include" | "preproc_call" | "comment"
    )
}

fn unscored_macros(node: Node, source: &[u8]) -> u32 {
    if let Some(count) = direct_macro(node) {
        return count;
    }
    if let Some(count) = preproc_macros(node, source) {
        return count;
    }
    named_nodes(node)
        .into_iter()
        .map(|child| unscored_macros(child, source))
        .sum()
}

fn direct_macro(node: Node) -> Option<u32> {
    match node.kind() {
        "preproc_function_def" => Some(1),
        "preproc_def" | "preproc_include" | "preproc_call" | "comment" => Some(0),
        _ => None,
    }
}

fn preproc_macros(node: Node, source: &[u8]) -> Option<u32> {
    match node.kind() {
        "preproc_if" | "preproc_elif" => Some(unscored_in_preproc(node, source, true)),
        "preproc_ifdef" | "preproc_elifdef" | "preproc_else" => {
            Some(unscored_in_preproc(node, source, false))
        }
        _ => None,
    }
}

fn unscored_in_preproc(node: Node, source: &[u8], check_zero: bool) -> u32 {
    let parts = preproc_parts(node);
    let mut count = 0;
    if !preproc_dead(check_zero, parts.condition, source) {
        count += live_macro_count(node, source, parts);
    }
    count + alternative_macros(parts.alternative, source)
}

fn live_macro_count(node: Node, source: &[u8], parts: Preproc) -> u32 {
    let mut count = 0;
    for child in named_nodes(node) {
        if skip_preproc_child(child, parts) {
            continue;
        }
        count += unscored_macros(child, source);
    }
    count
}

fn alternative_macros(alternative: Option<Node>, source: &[u8]) -> u32 {
    let Some(alternative) = alternative else {
        return 0;
    };
    unscored_macros(alternative, source)
}

fn preproc_parts(node: Node<'_>) -> Preproc<'_> {
    Preproc {
        condition: node.child_by_field_name("condition"),
        alternative: node.child_by_field_name("alternative"),
    }
}

fn preproc_dead(check_zero: bool, condition: Option<Node>, source: &[u8]) -> bool {
    if !check_zero {
        return false;
    }
    condition.is_some_and(|condition| is_literal_zero(condition, source))
}

fn skip_preproc_child(child: Node, parts: Preproc) -> bool {
    Some(child) == parts.condition || Some(child) == parts.alternative
}

impl<'a> Scan<'a> {
    fn visit_preproc(&mut self, node: Node<'a>, check_zero: bool) {
        let parts = preproc_parts(node);
        if !preproc_dead(check_zero, parts.condition, self.source) {
            self.visit_live(node, parts);
        }
        self.visit_optional(parts.alternative);
    }

    fn visit_live(&mut self, node: Node<'a>, parts: Preproc<'a>) {
        for child in named_nodes(node) {
            if skip_preproc_child(child, parts) {
                continue;
            }
            self.visit(child);
        }
    }

    fn visit_optional(&mut self, node: Option<Node<'a>>) {
        let Some(node) = node else {
            return;
        };
        self.visit(node);
    }

    fn visit_namespace(&mut self, node: Node<'a>) {
        let parts = scope_parts(node, self.source);
        for part in &parts {
            self.scopes.push(Scope::Namespace(part.clone()));
        }
        self.visit_optional_body(node);
        for _ in &parts {
            self.scopes.pop();
        }
    }

    fn visit_class(&mut self, node: Node<'a>) {
        let parts = scope_parts(node, self.source);
        for part in &parts {
            self.scopes.push(Scope::Class(part.clone()));
        }
        self.visit_optional_body(node);
        for _ in &parts {
            self.scopes.pop();
        }
    }

    fn visit_optional_body(&mut self, node: Node<'a>) {
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        self.visit(body);
    }

    fn visit_function(&mut self, node: Node<'a>) {
        if written_body(node).is_none() {
            return;
        }
        if !is_real_function(node) {
            self.visit_children(node);
            return;
        }
        let (global, parts) = function_parts(node, self.source);
        if parts.is_empty() {
            self.visit_children(node);
            return;
        }
        if macro_wrapped_block(node, self.scopes) {
            self.visit_written(node);
            return;
        }
        self.record_function(node, global, &parts);
    }

    fn visit_written(&mut self, node: Node<'a>) {
        let Some(body) = written_body(node) else {
            return;
        };
        self.visit(body);
    }

    fn record_function(&mut self, node: Node<'a>, global: bool, parts: &[String]) {
        let Some(body) = written_body(node) else {
            return;
        };
        let name = last_part(parts);
        let params = parameter_list(node);
        let qualified = qualify(self.scopes, global, parts);
        self.push_defined(Defined {
            name: qualified,
            item: node,
            body: node,
            params,
        });
        self.scopes.push(Scope::Function(name));
        self.visit(body);
        self.scopes.pop();
    }

    fn visit_declaration(&mut self, node: Node<'a>) {
        if in_function(self.scopes) {
            self.visit_children(node);
            return;
        }
        let bindings = bound_lambdas(node, self.source);
        if bindings.is_empty() {
            self.visit_children(node);
            return;
        }
        self.visit_bindings(node, bindings);
    }

    fn visit_bindings(&mut self, item: Node<'a>, bindings: Vec<(String, Node<'a>)>) {
        for (name, lambda) in bindings {
            self.visit_binding(item, name, lambda);
        }
    }

    fn visit_binding(&mut self, item: Node<'a>, name: String, lambda: Node<'a>) {
        let params = lambda_parameters(lambda);
        let qualified = qualify(self.scopes, false, std::slice::from_ref(&name));
        self.push_defined(Defined {
            name: qualified,
            item,
            body: lambda,
            params,
        });
        self.scopes.push(Scope::Function(name));
        self.visit_optional_body(lambda);
        self.scopes.pop();
    }
}

fn scope_parts(node: Node, source: &[u8]) -> Vec<String> {
    let name = node.child_by_field_name("name");
    name.map(|name| name_parts(name, source))
        .unwrap_or_default()
}

fn is_real_function(node: Node) -> bool {
    let Some(declarator) = node.child_by_field_name("declarator") else {
        return false;
    };
    has_function_declarator(declarator)
}

fn has_function_declarator(declarator: Node) -> bool {
    if find_kind(declarator, "function_declarator").is_some() {
        return true;
    }
    find_kind(declarator, "operator_cast").is_some()
}

fn function_parts(node: Node, source: &[u8]) -> (bool, Vec<String>) {
    let declarator = node.child_by_field_name("declarator");
    declarator
        .map(|declarator| qualified_parts(declarator, source))
        .unwrap_or((false, Vec::new()))
}

fn last_part(parts: &[String]) -> String {
    parts.last().cloned().unwrap_or_default()
}

fn lambda_parameters(lambda: Node<'_>) -> Option<Node<'_>> {
    let declarator = lambda.child_by_field_name("declarator")?;
    declarator.child_by_field_name("parameters")
}

fn bound_lambdas<'a>(node: Node<'a>, source: &[u8]) -> Vec<(String, Node<'a>)> {
    named_nodes(node)
        .into_iter()
        .filter_map(|child| bound_lambda(child, source))
        .collect()
}

fn bound_lambda<'a>(child: Node<'a>, source: &[u8]) -> Option<(String, Node<'a>)> {
    if child.kind() != "init_declarator" {
        return None;
    }
    let value = child.child_by_field_name("value")?;
    let lambda = peel_lambda(value)?;
    let name = declarator_name(child, source);
    if name.is_empty() {
        return None;
    }
    Some((name, lambda))
}

fn declarator_name(node: Node, source: &[u8]) -> String {
    let declarator = node.child_by_field_name("declarator");
    let ident = declarator.and_then(declarator_identifier);
    ident
        .map(|ident| node_text(ident, source))
        .unwrap_or_default()
}

fn peel_lambda(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "lambda_expression" => Some(node),
        "parenthesized_expression" => {
            let children = code_nodes(node);
            let [child] = children.as_slice() else {
                return None;
            };
            peel_lambda(*child)
        }
        _ => None,
    }
}

fn macro_wrapped_block(node: Node<'_>, scopes: &[Scope]) -> bool {
    matches!(scopes.last(), Some(Scope::Function(_))) && folds_into_parent(node)
}

fn folds_into_parent(node: Node<'_>) -> bool {
    let ty = node.child_by_field_name("type");
    ty.is_none() && !has_typed_parameters(node)
}

fn has_typed_parameters(node: Node<'_>) -> bool {
    let Some(params) = parameter_list(node) else {
        return false;
    };
    named_nodes(params)
        .iter()
        .any(|param| typed_parameter(*param))
}

fn typed_parameter(param: Node<'_>) -> bool {
    match param.kind() {
        "explicit_object_parameter_declaration" => named_nodes(param)
            .iter()
            .any(|inner| typed_parameter(*inner)),
        "parameter_declaration"
        | "optional_parameter_declaration"
        | "variadic_parameter_declaration" => declared_parameter(param),
        _ => false,
    }
}

fn declared_parameter(param: Node<'_>) -> bool {
    let declarator = param.child_by_field_name("declarator");
    if declarator.is_some() {
        return true;
    }
    let ty = param.child_by_field_name("type");
    ty.is_some_and(|ty| ty.kind() != "type_identifier")
}

fn written_body(node: Node<'_>) -> Option<Node<'_>> {
    if let Some(body) = node.child_by_field_name("body") {
        return match body.kind() {
            "compound_statement" | "try_statement" => Some(body),
            _ => None,
        };
    }
    // A function-try-block constructor puts the try beside the declarator.
    named_nodes(node)
        .into_iter()
        .find(|child| child.kind() == "try_statement")
}

impl<'a> Scan<'a> {
    fn push_defined(&mut self, defined: Defined<'a>) {
        let span = function_span(defined.item);
        let attributes = leading_attributes(defined.item);
        let anchor = attribute_anchor(&attributes, defined.item);
        let waivers = waivers_above(anchor, self.source);
        let touch = touch_span(span, &attributes, &waivers);
        let params = parameters_of(defined.params, self.source);
        let ordinary = ordinary_names(&params);
        let receiver = function_receiver(&params, defined.body);
        let (param_types, qualifier) = function_signature(defined.body, self.source);
        let classified = classify(
            defined.body,
            Score {
                source: self.source,
                receiver: receiver.as_deref(),
                ordinary: &ordinary,
            },
        );
        self.functions.push(Function {
            name: defined.name,
            param_types,
            qualifier,
            span,
            touch,
            params,
            body: classified,
            waivers,
        });
    }
}

fn function_span(item: Node) -> Span {
    let mut span = span_of(item);
    span.start = raised_start(item, span.start);
    span.start = absorbed_start(item, span.start);
    span
}

fn raised_start(item: Node, start: u32) -> u32 {
    let Some(content) = content_line(item) else {
        return start;
    };
    if content > start {
        return content;
    }
    start
}

fn absorbed_start(item: Node, start: u32) -> u32 {
    if item.kind() != "function_definition" {
        return start;
    }
    let Some(declarator) = item.child_by_field_name("declarator") else {
        return start;
    };
    if class_before(item, declarator) {
        return line_of(declarator);
    }
    start
}

fn class_before(item: Node, declarator: Node) -> bool {
    named_nodes(item)
        .iter()
        .any(|child| type_absorbed(*child, declarator))
}

fn type_absorbed(child: Node, declarator: Node) -> bool {
    if !absorbed_kind(child.kind()) {
        return false;
    }
    if child.end_byte() > declarator.start_byte() {
        return false;
    }
    has_body(child)
}

fn absorbed_kind(kind: &str) -> bool {
    matches!(
        kind,
        "class_specifier" | "struct_specifier" | "union_specifier" | "enum_specifier"
    )
}

fn has_body(child: Node) -> bool {
    let body = child.child_by_field_name("body");
    body.is_some()
}

fn attribute_anchor<'a>(attributes: &[Node<'a>], item: Node<'a>) -> Node<'a> {
    let first = attributes.first();
    first.copied().unwrap_or(item)
}

fn touch_span(span: Span, attributes: &[Node], waivers: &[WaiverLine]) -> Span {
    let mut touch = span;
    if let Some(attribute) = attributes.first() {
        let attribute_span = span_of(*attribute);
        let start = attribute_span.start;
        touch.start = start.min(touch.start);
    }
    if let Some(first) = waivers.first() {
        touch.start = first.line;
    }
    touch
}

fn ordinary_names(params: &[Param]) -> Vec<String> {
    let mut names = Vec::new();
    for param in params {
        if let Some(name) = ordinary_name(param) {
            names.push(name);
        }
    }
    names
}

fn ordinary_name(param: &Param) -> Option<String> {
    if param.receiver {
        return None;
    }
    let name = &param.name;
    if name.is_empty() {
        return None;
    }
    Some(name.clone())
}

fn function_receiver(params: &[Param], body: Node) -> Option<String> {
    let named = params.iter().find_map(receiver_name);
    named.or_else(|| default_receiver(body))
}

fn receiver_name(param: &Param) -> Option<String> {
    if !param.receiver {
        return None;
    }
    let name = &param.name;
    if name.is_empty() {
        return None;
    }
    Some(name.clone())
}

fn default_receiver(body: Node) -> Option<String> {
    if body.kind() == "lambda_expression" {
        return None;
    }
    Some("this".to_string())
}

fn function_signature(body: Node, source: &[u8]) -> (Vec<String>, String) {
    if body.kind() == "lambda_expression" {
        let params = params_node_of_lambda(body);
        return (parameter_types(params, source), String::new());
    }
    signature(body, source)
}

fn content_line(node: Node<'_>) -> Option<u32> {
    named_nodes(node)
        .into_iter()
        .filter(|child| !is_attribute(child.kind()))
        .map(line_of)
        .min()
}

fn signature(node: Node, source: &[u8]) -> (Vec<String>, String) {
    let Some(declarator) = node.child_by_field_name("declarator") else {
        return (Vec::new(), String::new());
    };
    let Some(function) = find_kind(declarator, "function_declarator") else {
        return (Vec::new(), String::new());
    };
    let types = parameter_types(function.child_by_field_name("parameters"), source);
    let mut qualifier = Vec::new();
    for child in named_nodes(function) {
        if child.kind() == "type_qualifier" || child.kind() == "ref_qualifier" {
            qualifier.push(flat_text(child, source));
        }
    }
    (types, qualifier.join(" "))
}

fn params_node_of_lambda(node: Node<'_>) -> Option<Node<'_>> {
    let declarator = node.child_by_field_name("declarator")?;
    declarator.child_by_field_name("parameters")
}

fn parameter_list(node: Node<'_>) -> Option<Node<'_>> {
    let declarator = node.child_by_field_name("declarator")?;
    find_kind(declarator, "function_declarator")?.child_by_field_name("parameters")
}

fn parameter_types(params: Option<Node>, source: &[u8]) -> Vec<String> {
    params
        .map(|node| parsed_parameters(node, source))
        .unwrap_or_default()
        .into_iter()
        .map(|param| param.ty)
        .collect()
}

struct ParsedParam {
    name: String,
    receiver: bool,
    ty: String,
}

fn parameters_of(params: Option<Node>, source: &[u8]) -> Vec<Param> {
    params
        .map(|node| parsed_parameters(node, source))
        .unwrap_or_default()
        .into_iter()
        .map(|param| Param {
            name: param.name,
            receiver: param.receiver,
        })
        .collect()
}

fn parsed_parameters(node: Node, source: &[u8]) -> Vec<ParsedParam> {
    let mut params = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        push_parameter(&mut params, child, source);
    }
    params
}

fn push_parameter(params: &mut Vec<ParsedParam>, child: Node, source: &[u8]) {
    if !child.is_named() {
        push_ellipsis(params, child, source);
        return;
    }
    push_named_parameter(params, child, source);
}

fn push_ellipsis(params: &mut Vec<ParsedParam>, child: Node, source: &[u8]) {
    if node_text(child, source) != "..." {
        return;
    }
    params.push(ParsedParam {
        name: "...".to_string(),
        receiver: false,
        ty: "...".to_string(),
    });
}

fn push_named_parameter(params: &mut Vec<ParsedParam>, child: Node, source: &[u8]) {
    match child.kind() {
        "explicit_object_parameter_declaration" => push_receiver_parameter(params, child, source),
        "parameter_declaration"
        | "optional_parameter_declaration"
        | "variadic_parameter_declaration" => params.push(one_parameter(child, source)),
        _ => {}
    }
}

fn push_receiver_parameter(params: &mut Vec<ParsedParam>, child: Node, source: &[u8]) {
    let Some(inner) = receiver_declaration(child) else {
        return;
    };
    let mut param = one_parameter(inner, source);
    param.receiver = true;
    params.push(param);
}

fn receiver_declaration(child: Node<'_>) -> Option<Node<'_>> {
    named_nodes(child)
        .into_iter()
        .find(|inner| inner.kind() == "parameter_declaration")
}

fn one_parameter(node: Node, source: &[u8]) -> ParsedParam {
    let declarator = node.child_by_field_name("declarator");
    let name_node = declarator.and_then(declarator_identifier);
    let name = name_node
        .map(|ident| node_text(ident, source))
        .unwrap_or_default();
    ParsedParam {
        name,
        receiver: false,
        ty: parameter_type(node, source, name_node),
    }
}

fn parameter_type(node: Node, source: &[u8], name: Option<Node>) -> String {
    let range = node.byte_range();
    let end = type_end(node, range.end);
    if !type_range_ok(range.start, end, source.len()) {
        return String::new();
    }
    let borrowed = String::from_utf8_lossy(&source[range.start..end]);
    let mut text = borrowed.into_owned();
    blank_name(&mut text, name, range.start);
    normalized_type(&text)
}

fn type_end(node: Node, end: usize) -> usize {
    let Some(value) = node.child_by_field_name("default_value") else {
        return end;
    };
    end.min(value.start_byte())
}

fn type_range_ok(start: usize, end: usize, len: usize) -> bool {
    if end < start {
        return false;
    }
    if end > len {
        return false;
    }
    start <= len
}

fn blank_name(text: &mut String, name: Option<Node>, origin: usize) {
    let Some(name) = name else {
        return;
    };
    erase_name(text, name, origin);
}

fn erase_name(text: &mut String, name: Node, origin: usize) {
    let start = byte_offset(name.start_byte(), origin);
    let stop = byte_offset(name.end_byte(), origin);
    if start < stop && stop <= text.len() {
        text.replace_range(start..stop, " ");
    }
}

fn byte_offset(byte: usize, origin: usize) -> usize {
    byte.saturating_sub(origin)
}

fn normalized_type(text: &str) -> String {
    let trimmed = text.trim_end_matches(type_trailer);
    join_words(trimmed)
}

fn join_words(text: &str) -> String {
    let parts: Vec<_> = text.split_whitespace().collect();
    parts.join(" ")
}

fn type_trailer(ch: char) -> bool {
    ch == '=' || ch.is_whitespace()
}

fn declarator_identifier(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "identifier" | "field_identifier" => Some(node),
        "pointer_declarator"
        | "reference_declarator"
        | "function_declarator"
        | "abstract_function_declarator"
        | "parenthesized_declarator" => nested_declarator(node),
        _ => first_declarator(node),
    }
}

fn nested_declarator(node: Node<'_>) -> Option<Node<'_>> {
    let declarator = node.child_by_field_name("declarator");
    let found = declarator.and_then(declarator_identifier);
    found.or_else(|| first_declarator(node))
}

fn first_declarator(node: Node<'_>) -> Option<Node<'_>> {
    named_nodes(node)
        .into_iter()
        .find_map(declarator_identifier)
}

fn qualified_parts(node: Node, source: &[u8]) -> (bool, Vec<String>) {
    if let Some(parts) = declarator_qualified(node, source) {
        return parts;
    }
    if let Some(parts) = named_qualified(node, source) {
        return parts;
    }
    if let Some(parts) = ident_qualified(node, source) {
        return parts;
    }
    text_qualified(node, source)
}

fn declarator_qualified(node: Node, source: &[u8]) -> Option<(bool, Vec<String>)> {
    let function = find_kind(node, "function_declarator")?;
    let declarator = function.child_by_field_name("declarator")?;
    Some(qualified_parts(declarator, source))
}

fn named_qualified(node: Node, source: &[u8]) -> Option<(bool, Vec<String>)> {
    if !is_qualified_kind(node.kind()) {
        return None;
    }
    let text = flat_text(node, source);
    Some((text.starts_with("::"), name_parts(node, source)))
}

fn is_qualified_kind(kind: &str) -> bool {
    matches!(kind, "qualified_identifier" | "operator_cast")
}

fn ident_qualified(node: Node, source: &[u8]) -> Option<(bool, Vec<String>)> {
    let ident = declarator_identifier(node)?;
    if !ident_is_name(node, ident) {
        return None;
    }
    let parts = name_parts(node, source);
    if parts.is_empty() {
        return None;
    }
    Some((false, parts))
}

fn ident_is_name(node: Node, ident: Node) -> bool {
    if matches!(node.kind(), "identifier" | "field_identifier") {
        return true;
    }
    ident.kind() == "identifier"
}

fn text_qualified(node: Node, source: &[u8]) -> (bool, Vec<String>) {
    let parts = name_parts(node, source);
    let text = flat_text(node, source);
    (text.starts_with("::"), parts)
}

fn name_parts(node: Node, source: &[u8]) -> Vec<String> {
    if node.kind() == "qualified_identifier" {
        return qualified_name_parts(node, source);
    }
    simple_name_parts(node, source)
}

fn qualified_name_parts(node: Node, source: &[u8]) -> Vec<String> {
    let mut parts = Vec::new();
    let scope = node.child_by_field_name("scope");
    append_child_parts(&mut parts, scope, source);
    let name = node.child_by_field_name("name");
    append_child_parts(&mut parts, name, source);
    if parts.is_empty() {
        push_qualified_text(&mut parts, node, source);
    }
    parts
}

fn append_child_parts(parts: &mut Vec<String>, child: Option<Node>, source: &[u8]) {
    let Some(child) = child else {
        return;
    };
    parts.extend(name_parts(child, source));
}

fn push_qualified_text(parts: &mut Vec<String>, node: Node, source: &[u8]) {
    let text = flat_text(node, source);
    if keep_qualified_text(&text) {
        parts.push(trimmed_qualified(&text));
    }
}

fn keep_qualified_text(text: &str) -> bool {
    !text.is_empty() && text != "::"
}

fn trimmed_qualified(text: &str) -> String {
    let trimmed = text.trim_start_matches("::");
    trimmed.to_string()
}

fn simple_name_parts(node: Node, source: &[u8]) -> Vec<String> {
    if let Some(part) = leaf_name(node, source) {
        return vec![part];
    }
    nested_name_parts(node, source)
}

fn leaf_name(node: Node, source: &[u8]) -> Option<String> {
    let kind = node.kind();
    if is_plain_name(kind) {
        return Some(node_text(node, source));
    }
    if is_operator_name(kind) {
        return Some(flat_text(node, source));
    }
    None
}

fn is_plain_name(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "field_identifier"
            | "namespace_identifier"
            | "type_identifier"
            | "primitive_type"
    )
}

fn is_operator_name(kind: &str) -> bool {
    matches!(
        kind,
        "operator_name"
            | "destructor_name"
            | "operator_cast"
            | "template_type"
            | "template_function"
            | "template_method"
    )
}

fn nested_name_parts(node: Node, source: &[u8]) -> Vec<String> {
    named_nodes(node)
        .into_iter()
        .flat_map(|child| name_parts(child, source))
        .collect()
}

fn find_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    if node.kind() == kind {
        return Some(node);
    }
    for child in named_nodes(node) {
        if let Some(found) = find_kind(child, kind) {
            return Some(found);
        }
    }
    None
}

fn leading_attributes(node: Node<'_>) -> Vec<Node<'_>> {
    let mut found = previous_attributes(node);
    append_inner_attributes(&mut found, node);
    found.sort_by_key(node_start);
    found
}

fn node_start(item: &Node) -> usize {
    item.start_byte()
}

fn previous_attributes(node: Node<'_>) -> Vec<Node<'_>> {
    let mut found = Vec::new();
    let mut cursor = node.prev_sibling();
    while let Some(sibling) = cursor {
        if !keep_previous(sibling, &mut found) {
            break;
        }
        cursor = sibling.prev_sibling();
    }
    found.reverse();
    found
}

fn keep_previous<'a>(sibling: Node<'a>, found: &mut Vec<Node<'a>>) -> bool {
    if is_attribute(sibling.kind()) {
        found.push(sibling);
        return true;
    }
    sibling.kind() == "comment"
}

fn append_inner_attributes<'a>(found: &mut Vec<Node<'a>>, node: Node<'a>) {
    let content = content_start(node);
    for child in named_nodes(node) {
        push_inner_attribute(found, child, content);
    }
}

fn content_start(node: Node<'_>) -> Option<usize> {
    named_nodes(node)
        .into_iter()
        .find(not_attribute)
        .map(node_start_owned)
}

fn not_attribute(child: &Node) -> bool {
    !is_attribute(child.kind())
}

fn node_start_owned(child: Node) -> usize {
    child.start_byte()
}

fn push_inner_attribute<'a>(found: &mut Vec<Node<'a>>, child: Node<'a>, content: Option<usize>) {
    if !is_attribute(child.kind()) {
        return;
    }
    if attribute_before(child, content) {
        found.push(child);
    }
}

fn attribute_before(child: Node, content: Option<usize>) -> bool {
    content.is_none_or(|start| child.start_byte() < start)
}

fn is_attribute(kind: &str) -> bool {
    matches!(kind, "attribute_declaration" | "attribute_specifier")
}

struct WaiverStep {
    line_item: WaiverLine,
    next: u32,
}

struct CommentPlace<'a> {
    span: Span,
    sibling: Node<'a>,
}

fn waivers_above(node: Node, source: &[u8]) -> Vec<WaiverLine> {
    let span = span_of(node);
    let mut next_line = span.start;
    let mut found = Vec::new();
    let mut cursor = node.prev_sibling();
    while let Some(sibling) = cursor {
        let Some(step) = waiver_before(sibling, source, next_line) else {
            break;
        };
        found.push(step.line_item);
        next_line = step.next;
        cursor = sibling.prev_sibling();
    }
    found.reverse();
    found
}

fn waiver_before(sibling: Node, source: &[u8], next_line: u32) -> Option<WaiverStep> {
    if !is_line_comment(sibling, source) {
        return None;
    }
    let span = span_of(sibling);
    let place = CommentPlace { span, sibling };
    if !adjacent_comment(place, next_line, source) {
        return None;
    }
    let text = node_text(sibling, source);
    let waiver = waiver_from_comment(&text, span.start)?;
    Some(WaiverStep {
        line_item: waiver,
        next: span.start,
    })
}

fn adjacent_comment(place: CommentPlace, next_line: u32, source: &[u8]) -> bool {
    let span = place.span;
    if span.start != span.end {
        return false;
    }
    if span.start + 1 != next_line {
        return false;
    }
    !shares_code_line(place.sibling, source)
}

fn is_line_comment(node: Node, source: &[u8]) -> bool {
    if node.kind() != "comment" {
        return false;
    }
    comment_text_is_line(&node_text(node, source))
}

fn comment_text_is_line(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with("//")
}

fn shares_code_line(comment: Node, source: &[u8]) -> bool {
    let line = row_of(comment);
    let mut cursor = comment.prev_sibling();
    while let Some(sibling) = cursor {
        if !is_line_comment(sibling, source) {
            return sibling_shares(sibling, line);
        }
        if occupied_end(sibling) < line {
            return false;
        }
        cursor = sibling.prev_sibling();
    }
    false
}

fn sibling_shares(sibling: Node, line: usize) -> bool {
    let start = row_of(sibling);
    start <= line && occupied_end(sibling) >= line
}

fn occupied_end(node: Node) -> usize {
    let end = node.end_position();
    if blank_terminal(end.column, end.row, row_of(node)) {
        return end.row - 1;
    }
    end.row
}

fn blank_terminal(column: usize, row: usize, start: usize) -> bool {
    column == 0 && row > start
}

fn waiver_from_comment(text: &str, line: u32) -> Option<WaiverLine> {
    let body = comment_body(text);
    let (marker, rest) = take_token(body)?;
    if marker != "slopmeter:" {
        return None;
    }
    let (verb, rest) = take_token(rest)?;
    if verb != "waive" {
        return None;
    }
    let (rule, rest) = waiver_rule(rest);
    let (number, reason) = waiver_number(rest);
    Some(WaiverLine {
        line,
        rule,
        number,
        reason,
    })
}

fn comment_body(text: &str) -> &str {
    let trimmed = text.trim();
    without_slashes(trimmed)
}

fn without_slashes(text: &str) -> &str {
    let stripped = text.strip_prefix("//");
    let body = stripped.unwrap_or(text);
    body.trim()
}

fn waiver_rule(rest: &str) -> (Option<String>, &str) {
    match take_token(rest) {
        Some((rule, rest)) => (Some(rule.to_string()), rest),
        None => (None, ""),
    }
}

fn waiver_number(rest: &str) -> (Option<String>, Option<String>) {
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

fn qualify(scopes: &[Scope], global: bool, parts: &[String]) -> String {
    let mut names = Vec::new();
    if !global {
        push_scopes(&mut names, scopes);
    }
    for part in parts {
        names.push(part);
    }
    names.join("::")
}

fn push_scopes<'a>(names: &mut Vec<&'a str>, scopes: &'a [Scope]) {
    for scope in scopes {
        names.push(scope_name(scope));
    }
}

fn scope_name(scope: &Scope) -> &str {
    match scope {
        Scope::Namespace(name) | Scope::Class(name) | Scope::Function(name) => name,
    }
}

fn in_function(scopes: &[Scope]) -> bool {
    scopes
        .iter()
        .any(|scope| matches!(scope, Scope::Function(_)))
}

fn is_literal_zero(node: Node, source: &[u8]) -> bool {
    let mut text = flat_text(node, source).replace(' ', "");
    loop {
        if text.len() < 2 || !text.starts_with('(') || !text.ends_with(')') {
            break;
        }
        let inner = &text[1..text.len() - 1];
        if !parentheses_wrap(inner) {
            break;
        }
        text = inner.to_string();
    }
    text == "0"
}

fn parentheses_wrap(text: &str) -> bool {
    let mut cursor = ParenCursor {
        depth: 0,
        index: 0,
        len: text.len(),
    };
    for (index, character) in text.chars().enumerate() {
        cursor.index = index;
        if !paren_step(&mut cursor, character) {
            return false;
        }
    }
    cursor.depth == 0
}

fn paren_step(cursor: &mut ParenCursor, character: char) -> bool {
    match character {
        '(' => {
            cursor.depth += 1;
            true
        }
        ')' => close_paren(cursor),
        _ => true,
    }
}

fn close_paren(cursor: &mut ParenCursor) -> bool {
    cursor.depth -= 1;
    if cursor.depth < 0 {
        return false;
    }
    !closes_early(cursor.depth, cursor.index, cursor.len)
}

fn closes_early(depth: i32, index: usize, len: usize) -> bool {
    depth == 0 && index + 1 != len
}

fn classify(node: Node<'_>, score: Score<'_>) -> Body {
    let mut classifier = Classifier {
        source: score.source,
        receiver: score.receiver,
        skip: None,
        lloc: 0,
        structure: 0,
        decisions: Vec::new(),
        levels: Vec::new(),
        cognitive: Vec::new(),
        chains: Vec::new(),
        swallowing: Vec::new(),
    };
    let forward = classifier.forwarding(node, score.ordinary);
    classifier.walk_root(node);
    Body {
        lloc: classifier.lloc,
        decisions: classifier.decisions,
        levels: classifier.levels,
        cognitive: classifier.cognitive,
        chains: classifier.chains,
        swallowing: classifier.swallowing,
        io: record_io(node, score.source),
        mutations: record_mutations(node, score.source),
        nulls: record_nulls(node),
        forward,
    }
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

struct Classifier<'a> {
    source: &'a [u8],
    receiver: Option<&'a str>,
    skip: Option<Range<usize>>,
    lloc: u32,
    structure: u32,
    decisions: Vec<Decision>,
    levels: Vec<Decision>,
    cognitive: Vec<Cognitive>,
    chains: Vec<Chain>,
    swallowing: Vec<u32>,
}

impl<'a> Classifier<'a> {
    fn forwarding(&mut self, node: Node<'a>, ordinary: &[String]) -> Option<Forward> {
        if ordinary.is_empty() {
            return None;
        }
        let (line, call) = forwarding_call(node, self.source)?;
        if !call_is_clean(call, self.source) {
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

    fn walk_root(&mut self, node: Node<'a>) {
        match node.kind() {
            "function_definition" => self.walk_function_root(node),
            "lambda_expression" => self.walk_lambda_root(node),
            _ => {}
        }
    }

    fn walk_function_root(&mut self, node: Node<'a>) {
        for child in code_nodes(node) {
            self.walk_initializer(child);
        }
        self.walk_written_root(node);
    }

    fn walk_initializer(&mut self, child: Node<'a>) {
        if child.kind() == "field_initializer_list" {
            self.walk(child, 0);
        }
    }

    fn walk_written_root(&mut self, node: Node<'a>) {
        let Some(body) = written_body(node) else {
            return;
        };
        self.walk(body, 0);
    }

    fn walk_lambda_root(&mut self, node: Node<'a>) {
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        self.walk(body, 0);
    }

    fn walk(&mut self, node: Node<'a>, nesting: u32) {
        match node.kind() {
            "if_statement" => self.walk_if(node, nesting, false),
            "while_statement" => self.walk_while(node, nesting),
            "do_statement" => self.walk_do(node, nesting),
            "for_statement" | "for_range_loop" | "expansion_statement" => {
                self.walk_for(node, nesting)
            }
            "switch_statement" => self.walk_switch(node, nesting),
            "try_statement" => self.walk_try(node, nesting),
            "catch_clause" => self.walk_catch(node, nesting),
            _ => self.walk_branch(node, nesting),
        }
    }

    fn walk_branch(&mut self, node: Node<'a>, nesting: u32) {
        match node.kind() {
            "conditional_expression" => self.walk_ternary(node, nesting),
            "lambda_expression" => self.walk_lambda(node, nesting),
            "binary_expression" => self.walk_binary(node, nesting),
            "preproc_if" | "preproc_elif" => self.walk_preproc(node, nesting, true),
            "preproc_ifdef" | "preproc_elifdef" | "preproc_else" => {
                self.walk_preproc(node, nesting, false)
            }
            "preproc_def" => {}
            _ => self.walk_stmt(node, nesting),
        }
    }

    fn walk_stmt(&mut self, node: Node<'a>, nesting: u32) {
        match node.kind() {
            "preproc_function_def" | "preproc_include" | "preproc_call" => {}
            "compound_statement" => self.walk_compound(node, nesting),
            "expression_statement" => self.walk_counted(node, nesting),
            "declaration" => self.walk_declaration(node, nesting),
            "return_statement" | "co_return_statement" | "co_yield_statement" => {
                self.walk_counted(node, nesting)
            }
            _ => self.walk_jump(node, nesting),
        }
    }

    fn walk_jump(&mut self, node: Node<'a>, nesting: u32) {
        match node.kind() {
            "throw_statement" => self.walk_counted(node, nesting),
            "break_statement" | "continue_statement" | "seh_leave_statement" => self.walk_flow(),
            "goto_statement" => self.walk_goto(node),
            "case_statement" => self.walk_case(node, nesting),
            "labeled_statement" | "attributed_statement" | "linkage_specification" => {
                self.walk_named(node, nesting)
            }
            _ => self.walk_decl(node, nesting),
        }
    }

    fn walk_decl(&mut self, node: Node<'a>, nesting: u32) {
        match node.kind() {
            "empty_statement" | "comment" => {}
            "function_definition" => self.walk_folded(node, nesting),
            "class_specifier" | "struct_specifier" | "union_specifier" | "enum_specifier"
            | "alias_declaration" | "using_declaration" => self.walk_type(),
            _ => self.walk_tail(node, nesting),
        }
    }

    fn walk_tail(&mut self, node: Node<'a>, nesting: u32) {
        match node.kind() {
            "static_assert_declaration" | "type_definition" => self.walk_type(),
            "field_expression"
            | "call_expression"
            | "subscript_expression"
            | "parenthesized_expression"
            | "template_function"
            | "template_method" => self.walk_access(node, nesting),
            _ => self.walk_named(node, nesting),
        }
    }

    fn walk_binary(&mut self, node: Node<'a>, nesting: u32) {
        if bool_op(node, self.source).is_some() {
            self.walk_boolean(node, nesting);
            return;
        }
        self.walk_named(node, nesting);
    }

    fn walk_counted(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.walk_named(node, nesting);
    }

    fn walk_flow(&mut self) {
        self.lloc += 1;
    }

    fn walk_goto(&mut self, node: Node<'a>) {
        self.lloc += 1;
        self.fundamental(line_of(node));
    }

    fn walk_type(&mut self) {
        self.lloc += 1;
    }

    fn walk_folded(&mut self, node: Node<'a>, nesting: u32) {
        if !folds_into_parent(node) {
            return;
        }
        self.walk_written_body(node, nesting);
    }

    fn walk_written_body(&mut self, node: Node<'a>, nesting: u32) {
        let Some(body) = written_body(node) else {
            return;
        };
        self.walk(body, nesting);
    }

    fn walk_access(&mut self, node: Node<'a>, nesting: u32) {
        if node.kind() == "call_expression" && callee_is_assert(node, self.source) {
            self.decision(line_of(node), nesting);
        }
        self.record_chain(node);
        self.walk_named(node, nesting);
    }

    fn walk_named(&mut self, node: Node<'a>, nesting: u32) {
        for child in code_nodes(node) {
            self.walk(child, nesting);
        }
    }

    fn walk_compound(&mut self, node: Node<'a>, nesting: u32) {
        for child in code_nodes(node) {
            self.walk(child, nesting);
        }
    }

    fn walk_preproc(&mut self, node: Node<'a>, nesting: u32, check_zero: bool) {
        let parts = preproc_parts(node);
        if !preproc_dead(check_zero, parts.condition, self.source) {
            self.walk_preproc_live(node, nesting, parts);
        }
        self.walk_optional(parts.alternative, nesting);
    }

    fn walk_preproc_live(&mut self, node: Node<'a>, nesting: u32, parts: Preproc<'a>) {
        for child in code_nodes(node) {
            if skip_preproc_child(child, parts) {
                continue;
            }
            self.walk(child, nesting);
        }
    }

    fn walk_optional(&mut self, node: Option<Node<'a>>, nesting: u32) {
        let Some(node) = node else {
            return;
        };
        self.walk(node, nesting);
    }

    fn walk_if(&mut self, node: Node<'a>, nesting: u32, else_if: bool) {
        let line = line_of(node);
        self.note_if(line, nesting, else_if);
        self.decision(line, nesting);
        self.walk_if_condition(node, nesting);
        self.walk_if_body(node, nesting);
        self.walk_if_alternative(node, nesting);
    }

    fn note_if(&mut self, line: u32, nesting: u32, else_if: bool) {
        if else_if {
            self.hybrid(line);
            return;
        }
        self.lloc += 1;
        self.structural(line, nesting);
    }

    fn walk_if_condition(&mut self, node: Node<'a>, nesting: u32) {
        let Some(condition) = node.child_by_field_name("condition") else {
            return;
        };
        self.walk_condition(condition, nesting);
    }

    fn walk_if_body(&mut self, node: Node<'a>, nesting: u32) {
        let Some(body) = node.child_by_field_name("consequence") else {
            return;
        };
        self.inside(|this| this.walk(body, nesting + 1));
    }

    fn walk_if_alternative(&mut self, node: Node<'a>, nesting: u32) {
        let Some(alternative) = node.child_by_field_name("alternative") else {
            return;
        };
        self.walk_else(alternative, nesting);
    }

    fn walk_else(&mut self, alternative: Node<'a>, nesting: u32) {
        let inner = code_nodes(alternative);
        let Some(child) = inner.first().copied() else {
            return;
        };
        if child.kind() == "if_statement" {
            self.walk_if(child, nesting, true);
            return;
        }
        self.hybrid(line_of(alternative));
        self.inside(|this| this.walk(child, nesting + 1));
    }

    fn walk_while(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.decision(line_of(node), nesting);
        self.structural(line_of(node), nesting);
        if let Some(condition) = node.child_by_field_name("condition") {
            self.walk_condition(condition, nesting);
        }
        if let Some(body) = node.child_by_field_name("body") {
            self.inside(|this| this.walk(body, nesting + 1));
        }
    }

    fn walk_do(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.decision(line_of(node), nesting);
        self.structural(line_of(node), nesting);
        if let Some(body) = node.child_by_field_name("body") {
            self.inside(|this| this.walk(body, nesting + 1));
        }
        if let Some(condition) = node.child_by_field_name("condition") {
            self.walk_condition(condition, nesting);
        }
    }

    fn walk_for(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.decision(line_of(node), nesting);
        self.structural(line_of(node), nesting);
        for field in ["initializer", "condition", "update", "right"] {
            if let Some(child) = node.child_by_field_name(field) {
                self.walk_value(child, nesting);
            }
        }
        if let Some(body) = node.child_by_field_name("body") {
            self.inside(|this| this.walk(body, nesting + 1));
        }
    }

    fn walk_switch(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.structural(line_of(node), nesting);
        if let Some(condition) = node.child_by_field_name("condition") {
            self.walk_condition(condition, nesting);
        }
        if let Some(body) = node.child_by_field_name("body") {
            self.walk_compound(body, nesting);
        }
    }

    fn walk_case(&mut self, node: Node<'a>, nesting: u32) {
        let value = node.child_by_field_name("value");
        if value.is_some() {
            self.decision(line_of(node), nesting);
        }
        for child in code_nodes(node) {
            if Some(child) == value {
                continue;
            }
            self.inside(|this| this.walk(child, nesting + 1));
        }
    }

    fn walk_try(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.walk_try_initializers(node, nesting);
        self.walk_try_body(node, nesting);
        self.walk_try_catches(node, nesting);
    }

    fn walk_try_initializers(&mut self, node: Node<'a>, nesting: u32) {
        for child in code_nodes(node) {
            self.walk_try_initializer(child, nesting);
        }
    }

    fn walk_try_initializer(&mut self, child: Node<'a>, nesting: u32) {
        if child.kind() == "field_initializer_list" {
            self.walk(child, nesting);
        }
    }

    fn walk_try_body(&mut self, node: Node<'a>, nesting: u32) {
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        self.walk(body, nesting);
    }

    fn walk_try_catches(&mut self, node: Node<'a>, nesting: u32) {
        for child in code_nodes(node) {
            self.walk_try_catch(child, nesting);
        }
    }

    fn walk_try_catch(&mut self, child: Node<'a>, nesting: u32) {
        if child.kind() == "catch_clause" {
            self.walk_catch(child, nesting);
        }
    }

    fn walk_catch(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.decision(line_of(node), nesting);
        self.structural(line_of(node), nesting);
        self.note_swallowed_catch(node);
        if let Some(body) = node.child_by_field_name("body") {
            self.inside(|this| this.walk(body, nesting + 1));
        }
    }

    fn note_swallowed_catch(&mut self, node: Node<'a>) {
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        if quiet_catch(body) {
            self.swallowing.push(line_of(node));
        }
    }

    fn walk_ternary(&mut self, node: Node<'a>, nesting: u32) {
        self.decision(line_of(node), nesting);
        self.structural(line_of(node), nesting);
        if let Some(condition) = node.child_by_field_name("condition") {
            self.walk(condition, nesting);
        }
        if let Some(consequence) = node.child_by_field_name("consequence") {
            self.inside(|this| this.walk(consequence, nesting + 1));
        }
        if let Some(alternative) = node.child_by_field_name("alternative") {
            self.inside(|this| this.walk(alternative, nesting + 1));
        }
    }

    fn walk_lambda(&mut self, node: Node<'a>, nesting: u32) {
        if let Some(body) = node.child_by_field_name("body") {
            self.walk(body, nesting + 1);
        }
    }

    fn walk_declaration(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.walk_value(node, nesting);
    }

    fn walk_condition(&mut self, node: Node<'a>, nesting: u32) {
        if let Some(initializer) = node.child_by_field_name("initializer") {
            self.walk_value(initializer, nesting);
        }
        if let Some(value) = node.child_by_field_name("value") {
            self.walk_value(value, nesting);
        } else if node.kind() != "condition_clause" {
            self.walk(node, nesting);
        }
    }

    fn walk_value(&mut self, node: Node<'a>, nesting: u32) {
        if is_declared_value(node) {
            self.walk_declared_values(node, nesting);
            return;
        }
        self.walk(node, nesting);
    }

    fn walk_declared_values(&mut self, node: Node<'a>, nesting: u32) {
        for child in code_nodes(node) {
            self.walk_declared_child(child, nesting);
        }
    }

    fn walk_declared_child(&mut self, child: Node<'a>, nesting: u32) {
        if let Some(value) = child.child_by_field_name("value") {
            self.walk(value, nesting);
            return;
        }
        if child.kind() != "init_declarator" {
            self.walk_value(child, nesting);
        }
    }

    fn walk_boolean(&mut self, node: Node<'a>, nesting: u32) {
        let mut last = None;
        self.bool_spine(node, nesting, &mut last);
    }

    fn bool_spine(&mut self, node: Node<'a>, nesting: u32, last: &mut Option<BoolOp>) {
        if self.walk_negation(node, nesting, last) {
            return;
        }
        if self.walk_parenthesized_bool(node, nesting) {
            return;
        }
        self.walk_bool_operator(node, nesting, last);
    }

    fn walk_negation(&mut self, node: Node<'a>, nesting: u32, last: &mut Option<BoolOp>) -> bool {
        if !is_negation(node, self.source) {
            return false;
        }
        self.walk_negated(node, nesting, last);
        true
    }

    fn walk_negated(&mut self, node: Node<'a>, nesting: u32, last: &mut Option<BoolOp>) {
        let Some(inner) = negation_inner(node) else {
            return;
        };
        self.bool_spine(inner, nesting, last);
    }

    fn walk_parenthesized_bool(&mut self, node: Node<'a>, nesting: u32) -> bool {
        if node.kind() != "parenthesized_expression" {
            return false;
        }
        self.record_chain(node);
        self.walk_bool_children(node, nesting);
        true
    }

    fn walk_bool_children(&mut self, node: Node<'a>, nesting: u32) {
        let mut inner = None;
        for child in code_nodes(node) {
            self.bool_spine(child, nesting, &mut inner);
        }
    }

    fn walk_bool_operator(&mut self, node: Node<'a>, nesting: u32, last: &mut Option<BoolOp>) {
        let Some(operator) = bool_op(node, self.source) else {
            self.walk(node, nesting);
            return;
        };
        self.walk_bool_left(node, nesting, last);
        let line = bool_operator_line(node);
        self.decision(line, nesting);
        self.note_bool_change(line, operator, last);
        self.walk_bool_right(node, nesting, last);
    }

    fn walk_bool_left(&mut self, node: Node<'a>, nesting: u32, last: &mut Option<BoolOp>) {
        let Some(left) = node.child_by_field_name("left") else {
            return;
        };
        self.bool_spine(left, nesting, last);
    }

    fn walk_bool_right(&mut self, node: Node<'a>, nesting: u32, last: &mut Option<BoolOp>) {
        let Some(right) = node.child_by_field_name("right") else {
            return;
        };
        self.bool_spine(right, nesting, last);
    }

    fn note_bool_change(&mut self, line: u32, operator: BoolOp, last: &mut Option<BoolOp>) {
        if *last == Some(operator) {
            return;
        }
        self.fundamental(line);
        *last = Some(operator);
    }

    fn record_chain(&mut self, node: Node<'a>) {
        if chain_continues(node) {
            return;
        }
        let Some(peeled) = peel(node, self.source) else {
            return;
        };
        if skip_peeled(&self.skip, &peeled, node) {
            return;
        }
        self.store_chain(peeled);
    }

    fn store_chain(&mut self, peeled: Peeled) {
        let line = hop_line(&peeled);
        let rooted = receiver_rooted(self.receiver, &peeled);
        let members = member_list(peeled);
        self.chains.push(Chain {
            line,
            rooted,
            members,
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
        self.levels.push(Decision {
            line,
            depth: self.structure + 1,
        });
    }

    fn inside(&mut self, f: impl FnOnce(&mut Self)) {
        self.structure += 1;
        f(self);
        self.structure -= 1;
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
    let body = match node.kind() {
        "function_definition" => written_body(node)?,
        "lambda_expression" => node.child_by_field_name("body")?,
        _ => return None,
    };
    if body.kind() != "compound_statement" {
        return None;
    }
    let statement = sole_forward(&code_nodes(body), source)?;
    Some((line_of(statement), unwrap_call(statement, source)?))
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
        "expression_statement"
        | "return_statement"
        | "co_return_statement"
        | "parenthesized_expression" => {
            let children = code_nodes(node);
            let [child] = children.as_slice() else {
                return None;
            };
            logging_leaf(*child, source)
        }
        "call_expression" => Some(callee_leaf(node.child_by_field_name("function")?, source)),
        _ => None,
    }
}

fn is_dead_assignment(node: Node, source: &[u8], later: &[Node]) -> bool {
    let names = match node.kind() {
        "declaration" => declared_names(node, source),
        "expression_statement" => {
            let children = code_nodes(node);
            let [child] = children.as_slice() else {
                return false;
            };
            if child.kind() != "assignment_expression" {
                return false;
            }
            child
                .child_by_field_name("left")
                .filter(|left| left.kind() == "identifier")
                .map(|left| vec![node_text(left, source)])
                .unwrap_or_default()
        }
        _ => return false,
    };
    !names.is_empty() && !names.iter().any(|name| mentioned(later, source, name))
}

fn declared_names(node: Node, source: &[u8]) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(declarator) = node.child_by_field_name("declarator") {
        names.extend(declarator_names(declarator, source));
    }
    for child in named_nodes(node) {
        if child.kind() == "init_declarator"
            && node.child_by_field_name("declarator") != Some(child)
        {
            names.extend(declarator_names(child, source));
        }
    }
    names
}

fn declarator_names(node: Node, source: &[u8]) -> Vec<String> {
    match node.kind() {
        "identifier" | "field_identifier" => vec![node_text(node, source)],
        "init_declarator" => node
            .child_by_field_name("declarator")
            .map(|declarator| declarator_names(declarator, source))
            .unwrap_or_default(),
        _ => named_nodes(node)
            .into_iter()
            .filter(|child| child.kind() != "number_literal" && child.kind() != "string_literal")
            .flat_map(|child| declarator_names(child, source))
            .collect(),
    }
}

fn mentioned(nodes: &[Node], source: &[u8], name: &str) -> bool {
    nodes.iter().any(|node| contains_name(*node, source, name))
}

fn contains_name(node: Node, source: &[u8], name: &str) -> bool {
    name_at(node, NameQuery { source, name })
}

fn name_at(node: Node, query: NameQuery) -> bool {
    if same_name(node, query) {
        return true;
    }
    name_in_children(node, query)
}

fn same_name(node: Node, query: NameQuery) -> bool {
    if !is_ident_kind(node.kind()) {
        return false;
    }
    node_text(node, query.source) == query.name
}

fn is_ident_kind(kind: &str) -> bool {
    matches!(kind, "identifier" | "field_identifier")
}

fn name_in_children(node: Node, query: NameQuery) -> bool {
    for index in 0..node.child_count() {
        if named_child(node, index, query) {
            return true;
        }
    }
    false
}

fn named_child(node: Node, index: u32, query: NameQuery) -> bool {
    let Some(child) = node.child(index) else {
        return false;
    };
    name_at(child, query)
}

fn unwrap_call<'a>(node: Node<'a>, source: &[u8]) -> Option<Node<'a>> {
    match node.kind() {
        "call_expression" => {
            if result_wrapper(node, source) {
                let argument = sole_argument(node)?;
                return unwrap_call(argument, source);
            }
            Some(node)
        }
        "expression_statement"
        | "return_statement"
        | "co_return_statement"
        | "parenthesized_expression" => {
            let children = code_nodes(node);
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
    let children = code_nodes(arguments);
    let [argument] = children.as_slice() else {
        return None;
    };
    Some(*argument)
}

fn arguments_allowed(call: Node, source: &[u8], ordinary: &[String]) -> bool {
    let Some(arguments) = call.child_by_field_name("arguments") else {
        return true;
    };
    code_nodes(arguments)
        .into_iter()
        .all(|argument| argument_allowed(argument, source, ordinary))
}

fn argument_allowed(node: Node, source: &[u8], ordinary: &[String]) -> bool {
    if let Some(name) = bare_name(node, source) {
        return ordinary.iter().any(|item| item == &name);
    }
    is_literal(node)
}

fn is_literal(node: Node) -> bool {
    if node.kind() == "parenthesized_expression" {
        return parenthesized_literal(node);
    }
    literal_kind(node.kind())
}

fn literal_kind(kind: &str) -> bool {
    matches!(
        kind,
        "number_literal"
            | "string_literal"
            | "char_literal"
            | "raw_string_literal"
            | "concatenated_string"
            | "true"
            | "false"
            | "null"
            | "nullptr"
    )
}

fn parenthesized_literal(node: Node) -> bool {
    let children = code_nodes(node);
    let [child] = children.as_slice() else {
        return false;
    };
    is_literal(*child)
}

fn call_is_clean(call: Node, source: &[u8]) -> bool {
    if function_is_nested(call, source) {
        return false;
    }
    arguments_are_clean(call, source)
}

fn function_is_nested(call: Node, source: &[u8]) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    nested_call(function, source)
}

fn arguments_are_clean(call: Node, source: &[u8]) -> bool {
    let Some(arguments) = call.child_by_field_name("arguments") else {
        return true;
    };
    clean_arguments(arguments, source)
}

fn clean_arguments(arguments: Node, source: &[u8]) -> bool {
    for argument in code_nodes(arguments) {
        if nested_call(argument, source) {
            return false;
        }
    }
    true
}

fn nested_call(node: Node, source: &[u8]) -> bool {
    if identity_named(node, source) {
        return identity_contains_call(node);
    }
    if node.kind() == "call_expression" {
        return true;
    }
    child_is_nested(node, source)
}

fn identity_contains_call(node: Node) -> bool {
    let arguments = node.child_by_field_name("arguments");
    arguments.is_some_and(call_in_arguments)
}

fn call_in_arguments(arguments: Node) -> bool {
    code_nodes(arguments)
        .iter()
        .any(|child| contains_call(*child))
}

fn child_is_nested(node: Node, source: &[u8]) -> bool {
    for index in 0..node.child_count() {
        if nested_at(node, index, source) {
            return true;
        }
    }
    false
}

fn nested_at(node: Node, index: u32, source: &[u8]) -> bool {
    let Some(child) = node.child(index) else {
        return false;
    };
    nested_call(child, source)
}

fn contains_call(node: Node) -> bool {
    if node.kind() == "call_expression" {
        return true;
    }
    call_in_children(node)
}

fn call_in_children(node: Node) -> bool {
    for index in 0..node.child_count() {
        if call_at(node, index) {
            return true;
        }
    }
    false
}

fn call_at(node: Node, index: u32) -> bool {
    let Some(child) = node.child(index) else {
        return false;
    };
    contains_call(child)
}

fn forwarded_names(call: Node, source: &[u8], ordinary: &[String]) -> Vec<String> {
    let mut found = BTreeSet::new();
    collect_receiver(&mut found, call, source);
    collect_arguments(&mut found, call, source);
    ordinary
        .iter()
        .filter(|name| found.contains(*name))
        .cloned()
        .collect()
}

fn collect_receiver(found: &mut BTreeSet<String>, call: Node, source: &[u8]) {
    let Some(function) = call.child_by_field_name("function") else {
        return;
    };
    insert_field_name(found, function, source);
}

fn insert_field_name(found: &mut BTreeSet<String>, function: Node, source: &[u8]) {
    if function.kind() != "field_expression" {
        return;
    }
    let Some(object) = function.child_by_field_name("argument") else {
        return;
    };
    insert_bare(found, object, source);
}

fn insert_bare(found: &mut BTreeSet<String>, node: Node, source: &[u8]) {
    let Some(name) = bare_name(node, source) else {
        return;
    };
    found.insert(name);
}

fn collect_arguments(found: &mut BTreeSet<String>, call: Node, source: &[u8]) {
    let Some(arguments) = call.child_by_field_name("arguments") else {
        return;
    };
    for argument in code_nodes(arguments) {
        insert_bare(found, argument, source);
    }
}

fn bare_name(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => Some(node_text(node, source)),
        "parenthesized_expression" => sole_child_name(node, source),
        "pointer_expression" if pointer_name_operator(node, source) => pointed_name(node, source),
        "parameter_pack_expansion" => pack_name(node, source),
        "call_expression" if identity_named(node, source) => identity_name(node, source),
        _ => None,
    }
}

fn pointer_name_operator(node: Node, source: &[u8]) -> bool {
    let text = operator_text(node, source);
    matches!(text.as_str(), "&" | "*")
}

fn sole_child_name(node: Node, source: &[u8]) -> Option<String> {
    let children = code_nodes(node);
    let [child] = children.as_slice() else {
        return None;
    };
    bare_name(*child, source)
}

fn pointed_name(node: Node, source: &[u8]) -> Option<String> {
    let argument = node.child_by_field_name("argument")?;
    bare_name(argument, source)
}

fn pack_name(node: Node, source: &[u8]) -> Option<String> {
    let pattern = node.child_by_field_name("pattern")?;
    bare_name(pattern, source)
}

fn identity_name(node: Node, source: &[u8]) -> Option<String> {
    let arguments = node.child_by_field_name("arguments")?;
    sole_child_name(arguments, source)
}

fn identity_named(node: Node, source: &[u8]) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    IDENTITY.contains(&callee_leaf(function, source).as_str())
}

fn callee_leaf(node: Node, source: &[u8]) -> String {
    match node.kind() {
        "identifier" | "field_identifier" => node_text(node, source),
        "qualified_identifier" | "template_function" | "template_method" | "field_expression" => {
            let field = if node.kind() == "field_expression" {
                "field"
            } else {
                "name"
            };
            node.child_by_field_name(field)
                .map(|child| callee_leaf(child, source))
                .unwrap_or_default()
        }
        "call_expression" => node
            .child_by_field_name("function")
            .map(|function| callee_leaf(function, source))
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn peel(node: Node, source: &[u8]) -> Option<Peeled> {
    match node.kind() {
        "identifier" => Some(peeled_name(node_text(node, source))),
        "this" => Some(peeled_name("this".to_string())),
        "qualified_identifier" => Some(Peeled {
            root: None,
            hops: Vec::new(),
        }),
        "field_expression" => peel_field(node, source),
        "template_function" | "template_method" => peel_named(node, source),
        "call_expression" => peel_call(node, source),
        "subscript_expression" => peel_argument(node, source),
        "parenthesized_expression" => peel_parentheses(node, source),
        _ => None,
    }
}

fn peeled_name(name: String) -> Peeled {
    Peeled {
        root: Some(name),
        hops: Vec::new(),
    }
}

fn peel_field(node: Node, source: &[u8]) -> Option<Peeled> {
    let object = node.child_by_field_name("argument")?;
    let mut peeled = peel(object, source).unwrap_or(Peeled {
        root: None,
        hops: Vec::new(),
    });
    if member_operator(node, source) {
        push_field(&mut peeled, node, source)?;
    }
    Some(peeled)
}

fn member_operator(node: Node, source: &[u8]) -> bool {
    let operator = node.child_by_field_name("operator");
    let text = operator_label(operator, source);
    dot_or_arrow(&text)
}

fn operator_label(operator: Option<Node>, source: &[u8]) -> String {
    let Some(operator) = operator else {
        return String::new();
    };
    node_text(operator, source)
}

fn dot_or_arrow(text: &str) -> bool {
    text == "." || text == "->"
}

fn push_field(peeled: &mut Peeled, node: Node, source: &[u8]) -> Option<()> {
    let name = node.child_by_field_name("field")?;
    let hops = &mut peeled.hops;
    hops.push(Hop {
        name: field_label(name, source),
        call: false,
        line: line_of(name),
    });
    Some(())
}

fn peel_named(node: Node, source: &[u8]) -> Option<Peeled> {
    let name = node.child_by_field_name("name")?;
    peel(name, source)
}

fn peel_call(node: Node, source: &[u8]) -> Option<Peeled> {
    let function = node.child_by_field_name("function")?;
    if let Some(name) = bare_call_name(function, source) {
        return Some(named_call(node, function, name));
    }
    called(peel(function, source)?)
}

fn named_call(node: Node, function: Node, name: String) -> Peeled {
    if bare_call_roots_a_chain(node) {
        return Peeled {
            root: None,
            hops: Vec::new(),
        };
    }
    Peeled {
        root: None,
        hops: vec![Hop {
            name,
            call: true,
            line: line_of(function),
        }],
    }
}

fn called(mut peeled: Peeled) -> Option<Peeled> {
    let hops = &mut peeled.hops;
    if let Some(last) = hops.last_mut() {
        last.call = true;
    }
    Some(peeled)
}

fn peel_argument(node: Node, source: &[u8]) -> Option<Peeled> {
    let argument = node.child_by_field_name("argument")?;
    peel(argument, source)
}

fn peel_parentheses(node: Node, source: &[u8]) -> Option<Peeled> {
    let children = code_nodes(node);
    let [child] = children.as_slice() else {
        return None;
    };
    peel(*child, source)
}

fn field_label(node: Node, source: &[u8]) -> String {
    match node.kind() {
        "template_method" | "template_function" => node
            .child_by_field_name("name")
            .map(|name| field_label(name, source))
            .unwrap_or_else(|| flat_text(node, source)),
        _ => flat_text(node, source),
    }
}

fn bare_call_name(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => Some(node_text(node, source)),
        "template_function" | "template_method" => {
            let name = node.child_by_field_name("name")?;
            bare_call_name(name, source)
        }
        "parenthesized_expression" => {
            let children = code_nodes(node);
            let [child] = children.as_slice() else {
                return None;
            };
            bare_call_name(*child, source)
        }
        _ => None,
    }
}

enum ParentStep<'a> {
    Rooted(bool),
    Climb(Node<'a>),
}

fn bare_call_roots_a_chain(node: Node) -> bool {
    let mut current = node;
    loop {
        let Some(parent) = current.parent() else {
            return false;
        };
        match parent_step(parent, current) {
            ParentStep::Rooted(value) => return value,
            ParentStep::Climb(next) => current = next,
        }
    }
}

fn parent_step<'a>(parent: Node<'a>, current: Node<'a>) -> ParentStep<'a> {
    match parent.kind() {
        "field_expression" => ParentStep::Rooted(field_argument_is(parent, current)),
        "subscript_expression" => subscript_step(parent, current),
        "parenthesized_expression" => grouped_step(parent, current),
        "call_expression" => call_step(parent, current),
        "template_function" | "template_method" => template_step(parent, current),
        _ => ParentStep::Rooted(false),
    }
}

fn field_argument_is(parent: Node, current: Node) -> bool {
    parent.child_by_field_name("argument") == Some(current)
}

fn subscript_step<'a>(parent: Node<'a>, current: Node<'a>) -> ParentStep<'a> {
    if parent.child_by_field_name("argument") != Some(current) {
        return ParentStep::Rooted(false);
    }
    ParentStep::Climb(parent)
}

fn grouped_step<'a>(parent: Node<'a>, current: Node<'a>) -> ParentStep<'a> {
    if !single_child(parent, current) {
        return ParentStep::Rooted(false);
    }
    ParentStep::Climb(parent)
}

fn single_child(parent: Node, current: Node) -> bool {
    let children = code_nodes(parent);
    if children.len() != 1 {
        return false;
    }
    let first = children.first();
    first == Some(&current)
}

fn call_step<'a>(parent: Node<'a>, current: Node<'a>) -> ParentStep<'a> {
    if parent.child_by_field_name("function") != Some(current) {
        return ParentStep::Rooted(false);
    }
    ParentStep::Climb(parent)
}

fn template_step<'a>(parent: Node<'a>, current: Node<'a>) -> ParentStep<'a> {
    if parent.child_by_field_name("name") != Some(current) {
        return ParentStep::Rooted(false);
    }
    ParentStep::Climb(parent)
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
        "field_expression" | "subscript_expression" => {
            parent.child_by_field_name("argument") == Some(node)
        }
        "call_expression" => parent.child_by_field_name("function") == Some(node),
        "template_function" | "template_method" => parent.child_by_field_name("name") == Some(node),
        "parenthesized_expression" => {
            let children = code_nodes(parent);
            children.len() == 1 && children.first() == Some(&node)
        }
        _ => false,
    }
}

fn bool_op(node: Node, source: &[u8]) -> Option<BoolOp> {
    if node.kind() != "binary_expression" {
        return None;
    }
    match operator_text(node, source).as_str() {
        "&&" | "and" => Some(BoolOp::And),
        "||" | "or" => Some(BoolOp::Or),
        _ => None,
    }
}

fn is_negation(node: Node, source: &[u8]) -> bool {
    node.kind() == "unary_expression" && matches!(operator_text(node, source).as_str(), "!" | "not")
}

fn negation_inner(node: Node<'_>) -> Option<Node<'_>> {
    let argument = node.child_by_field_name("argument");
    argument.or_else(|| first_code_node(node))
}

fn first_code_node(node: Node<'_>) -> Option<Node<'_>> {
    let nodes = code_nodes(node);
    nodes.into_iter().next()
}

fn operator_text(node: Node, source: &[u8]) -> String {
    if let Some(operator) = node.child_by_field_name("operator") {
        return node_text(operator, source);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !child.is_named() {
            return node_text(child, source);
        }
    }
    String::new()
}

fn quiet_catch(body: Node<'_>) -> bool {
    if body.kind() == "compound_statement" {
        return code_nodes(body).iter().all(quiet_catch_stmt);
    }
    quiet_catch_stmt(&body)
}

fn quiet_catch_stmt(node: &Node<'_>) -> bool {
    matches!(node.kind(), "continue_statement" | "break_statement")
}

fn code_nodes(node: Node<'_>) -> Vec<Node<'_>> {
    named_nodes(node)
        .into_iter()
        .filter(|child| child.kind() != "comment")
        .collect()
}

fn span_of(node: Node) -> Span {
    let start = row_of(node) as u32 + 1;
    let end_point = node.end_position();
    let end = span_end(node, end_point.column, end_point.row);
    Span {
        start,
        end: end.max(start),
    }
}

fn span_end(node: Node, column: usize, row: usize) -> u32 {
    if column == 0 && row > row_of(node) {
        return row as u32;
    }
    row as u32 + 1
}

fn row_of(node: Node) -> usize {
    let position = node.start_position();
    position.row
}

fn named_nodes(node: Node<'_>) -> Vec<Node<'_>> {
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
    if range.end > source.len() {
        return String::new();
    }
    String::from_utf8_lossy(&source[range]).into_owned()
}

fn flat_text(node: Node, source: &[u8]) -> String {
    let text = node_text(node, source);
    join_words(&text)
}

fn line_of(node: Node) -> u32 {
    row_of(node) as u32 + 1
}

fn is_declared_value(node: Node) -> bool {
    node.kind() == "declaration" || node.kind() == "init_statement"
}

fn bool_operator_line(node: Node) -> u32 {
    let operator = node.child_by_field_name("operator");
    let line = operator.map(line_of);
    line.unwrap_or_else(|| line_of(node))
}

fn callee_is_assert(node: Node, source: &[u8]) -> bool {
    callee_leaf(node, source) == "assert"
}

fn skip_peeled(skip: &Option<Range<usize>>, peeled: &Peeled, node: Node) -> bool {
    if hop_list(peeled).is_empty() {
        return true;
    }
    bare_forward(skip, peeled, node)
}

fn hop_list(peeled: &Peeled) -> &[Hop] {
    &peeled.hops
}

fn bare_forward(skip: &Option<Range<usize>>, peeled: &Peeled, node: Node) -> bool {
    if skip.as_ref() != Some(&node.byte_range()) {
        return false;
    }
    if root_present(peeled) {
        return false;
    }
    hop_list(peeled).len() == 1
}

fn root_present(peeled: &Peeled) -> bool {
    let root = &peeled.root;
    root.is_some()
}

fn hop_line(peeled: &Peeled) -> u32 {
    let hops = hop_list(peeled);
    let hop = &hops[0];
    hop.line
}

fn receiver_rooted(receiver: Option<&str>, peeled: &Peeled) -> bool {
    receiver.is_some_and(|name| root_is(peeled, name))
}

fn root_is(peeled: &Peeled, name: &str) -> bool {
    let root = peeled_root(peeled);
    root == Some(name) || root == Some("this")
}

fn peeled_root(peeled: &Peeled) -> Option<&str> {
    let root = &peeled.root;
    root.as_deref()
}

fn member_list(peeled: Peeled) -> Vec<Member> {
    let hops = peeled.hops;
    hops.into_iter().map(member_from_hop).collect()
}

fn member_from_hop(hop: Hop) -> Member {
    Member {
        name: hop.name,
        call: hop.call,
    }
}

const IO_NAMES: &[&str] = &[
    "std::ifstream",
    "std::ofstream",
    "std::fstream",
    "fopen",
    "socket",
    "popen",
    "system",
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
    match node.kind() {
        "call_expression" => call_io(node, source),
        "declaration" => declaration_io(node, source),
        _ => None,
    }
}

fn call_io(node: Node, source: &[u8]) -> Option<String> {
    let function = node.child_by_field_name("function")?;
    written_name(function, source)
}

fn declaration_io(node: Node, source: &[u8]) -> Option<String> {
    if !constructed(node) {
        return None;
    }
    let ty = node.child_by_field_name("type")?;
    written_name(ty, source)
}

fn constructed(node: Node) -> bool {
    let value = node.child_by_field_name("value");
    if value.is_some() {
        return true;
    }
    declarator_args(node)
}

fn declarator_args(node: Node) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if argument_list(child) {
            return true;
        }
        if child.kind() == "init_declarator" && constructed(child) {
            return true;
        }
    }
    false
}

fn argument_list(node: Node) -> bool {
    matches!(
        node.kind(),
        "argument_list" | "initializer_list" | "function_declarator"
    )
}

fn written_name(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" | "type_identifier" | "namespace_identifier" => Some(node_text(node, source)),
        "qualified_identifier" => qualified_name(node, source),
        _ => None,
    }
}

fn qualified_name(node: Node, source: &[u8]) -> Option<String> {
    let scope = node.child_by_field_name("scope")?;
    let left = written_name(scope, source)?;
    let name = node.child_by_field_name("name")?;
    let right = written_name(name, source)?;
    Some(format!("{left}::{right}"))
}

fn record_nulls(node: Node) -> Vec<u32> {
    let mut found = Vec::new();
    walk_nulls(node, &mut found, true);
    found
}

fn walk_nulls(node: Node, found: &mut Vec<u32>, root: bool) {
    if skipped_null(node, root) {
        return;
    }
    note_null(node, found);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_nulls(child, found, false);
    }
}

fn skipped_null(node: Node, root: bool) -> bool {
    !root && node.kind() == "function_definition"
}

fn note_null(node: Node, found: &mut Vec<u32>) {
    match node.kind() {
        "return_statement" => note_returned(node, found),
        "call_expression" => note_arguments(node, found),
        _ => {}
    }
}

fn note_returned(node: Node, found: &mut Vec<u32>) {
    let Some(expr) = node.named_child(0) else {
        return;
    };
    if !null_literal(expr) {
        return;
    }
    found.push(line_of(node));
}

fn note_arguments(node: Node, found: &mut Vec<u32>) {
    let Some(arguments) = node.child_by_field_name("arguments") else {
        return;
    };
    for argument in code_nodes(arguments) {
        note_argument(argument, found);
    }
}

fn note_argument(argument: Node, found: &mut Vec<u32>) {
    if !null_literal(argument) {
        return;
    }
    found.push(line_of(argument));
}

fn null_literal(node: Node) -> bool {
    if node.kind() == "null" {
        return true;
    }
    peeled_null(node)
}

fn peeled_null(node: Node) -> bool {
    if node.kind() != "parenthesized_expression" {
        return false;
    }
    let children = code_nodes(node);
    let [child] = children.as_slice() else {
        return false;
    };
    null_literal(*child)
}

fn record_mutations(node: Node, source: &[u8]) -> Vec<u32> {
    if !written_void(node, source) {
        return Vec::new();
    }
    let Some(params) = mutation_params(node) else {
        return Vec::new();
    };
    mutable_lines(params, source)
}

fn written_void(node: Node, source: &[u8]) -> bool {
    if type_is_void(node, source) {
        return true;
    }
    trailing_is_void(node, source)
}

fn type_is_void(node: Node, source: &[u8]) -> bool {
    let Some(ty) = node.child_by_field_name("type") else {
        return false;
    };
    node_text(ty, source) == "void"
}

fn trailing_is_void(node: Node, source: &[u8]) -> bool {
    let Some(declarator) = node.child_by_field_name("declarator") else {
        return false;
    };
    let Some(function) = own_declarator(declarator) else {
        return false;
    };
    direct_trailing_void(function, source)
}

fn own_declarator(node: Node<'_>) -> Option<Node<'_>> {
    if declarator_kind(node.kind()) {
        return Some(node);
    }
    let inner = node.child_by_field_name("declarator")?;
    own_declarator(inner)
}

fn declarator_kind(kind: &str) -> bool {
    kind == "function_declarator" || kind == "lambda_declarator"
}

fn direct_trailing_void(function: Node, source: &[u8]) -> bool {
    for child in named_nodes(function) {
        if trailing_void(child, source) {
            return true;
        }
    }
    false
}

fn trailing_void(child: Node, source: &[u8]) -> bool {
    if child.kind() != "trailing_return_type" {
        return false;
    }
    let Some(descriptor) = child.named_child(0) else {
        return false;
    };
    let Some(ty) = descriptor.child_by_field_name("type") else {
        return false;
    };
    node_text(ty, source) == "void"
}

fn mutation_params(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "lambda_expression" {
        return params_node_of_lambda(node);
    }
    parameter_list(node)
}

fn mutable_lines(params: Node, source: &[u8]) -> Vec<u32> {
    let mut found = Vec::new();
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        note_mutable(&mut found, child, source);
    }
    found
}

fn note_mutable(found: &mut Vec<u32>, child: Node, source: &[u8]) {
    if !param_kind(child.kind()) {
        return;
    }
    if specifier_const(child, source) {
        return;
    }
    if !reference_or_pointer(child) {
        return;
    }
    found.push(line_of(child));
}

fn param_kind(kind: &str) -> bool {
    if kind == "parameter_declaration" {
        return true;
    }
    if kind == "optional_parameter_declaration" {
        return true;
    }
    kind == "variadic_parameter_declaration"
}

fn specifier_const(node: Node, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if const_qualifier(child, source) {
            return true;
        }
    }
    false
}

fn const_qualifier(child: Node, source: &[u8]) -> bool {
    if child.kind() != "type_qualifier" {
        return false;
    }
    node_text(child, source) == "const"
}

fn reference_or_pointer(node: Node) -> bool {
    let Some(declarator) = node.child_by_field_name("declarator") else {
        return false;
    };
    ref_or_ptr(declarator)
}

fn ref_or_ptr(node: Node) -> bool {
    if ref_or_ptr_kind(node.kind()) {
        return true;
    }
    if let Some(inner) = node.child_by_field_name("declarator") {
        if ref_or_ptr(inner) {
            return true;
        }
    }
    wrapped_ref(node)
}

fn ref_or_ptr_kind(kind: &str) -> bool {
    if kind == "reference_declarator" {
        return true;
    }
    if kind == "abstract_reference_declarator" {
        return true;
    }
    if kind == "pointer_declarator" {
        return true;
    }
    kind == "abstract_pointer_declarator"
}

fn wrapped_ref(node: Node) -> bool {
    for child in named_nodes(node) {
        if wrapped_declarator(child) {
            return true;
        }
    }
    false
}

fn wrapped_declarator(child: Node) -> bool {
    if child.kind() != "parenthesized_declarator" {
        return false;
    }
    ref_or_ptr(child)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::CognitiveKind;

    fn function<'a>(file: &'a File, name: &str) -> &'a Function {
        file.functions
            .iter()
            .find(|function| function.name == name)
            .unwrap_or_else(|| panic!("missing {name}"))
    }

    fn member_names(chain: &crate::ir::Chain) -> Vec<&str> {
        chain
            .members
            .iter()
            .map(|member| member.name.as_str())
            .collect()
    }

    fn cognitive_total(body: &Body) -> u32 {
        body.cognitive
            .iter()
            .map(|increment| match increment.kind {
                CognitiveKind::Structural => 1 + increment.depth,
                CognitiveKind::Hybrid | CognitiveKind::Fundamental => 1,
            })
            .sum()
    }

    fn waiver(
        line: u32,
        rule: Option<&str>,
        number: Option<&str>,
        reason: Option<&str>,
    ) -> WaiverLine {
        WaiverLine {
            line,
            rule: rule.map(str::to_string),
            number: number.map(str::to_string),
            reason: reason.map(str::to_string),
        }
    }

    #[test]
    fn the_allow_list_is_the_cpp_member_list() {
        assert!(ALLOW.contains(&"begin"));
        assert!(ALLOW.contains(&"c_str"));
        assert!(ALLOW.contains(&"has_value"));
        assert_eq!(ALLOW.len(), 19);
    }

    #[test]
    fn methods_lambdas_and_waivers_follow_the_cpp_names() {
        let source = "\
namespace demo {
class Take {
    [[nodiscard]]
    int into_inner(int n) const { return n; }
};

template <typename T>
T id(T value) { return value; }
}

#if 0
int hidden() { return 1; }
#else
int shown() { return 2; }
#endif

struct Proto {
    int method();
    int gone() = default;
    int removed() = delete;
    virtual int pure() = 0;
    int kept() { return 1; }
};

int CLI::detail::join(std::string &name, int flags) const {
    return flags;
}

int outer(bool a) {
    struct Box {
        int inner(bool x) { if (x) return 1; return 0; }
    };
    auto pick = [](bool y) { if (y) return 1; return 0; };
    if (a) return 1;
    return 0;
}

// slopmeter: waive F01 9 measured on purpose
const auto add = [](int x) { return x + 1; };
";
        let file = parse("src/join.hpp", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{source}");
        let names: Vec<_> = file
            .functions
            .iter()
            .map(|function| function.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec![
                "demo::Take::into_inner",
                "demo::id",
                "shown",
                "Proto::kept",
                "CLI::detail::join",
                "outer",
                "outer::Box::inner",
                "add",
            ]
        );

        let method = function(&file, "demo::Take::into_inner");
        assert_eq!(method.param_types, vec!["int".to_string()]);
        assert_eq!(method.qualifier, "const");
        assert_eq!(method.params.len(), 1);
        assert!(!method.params[0].receiver);
        assert!(method.touch.start < method.span.start);

        let join = function(&file, "CLI::detail::join");
        assert_eq!(
            join.param_types,
            vec!["std::string &".to_string(), "int".to_string()]
        );
        assert_eq!(join.qualifier, "const");

        let outer = function(&file, "outer");
        assert!(outer
            .body
            .decisions
            .iter()
            .any(|decision| decision.depth >= 2));
        let inner = function(&file, "outer::Box::inner");
        assert_eq!(inner.body.decisions.len(), 1);

        let added = function(&file, "add");
        assert_eq!(added.params.len(), 1);
        assert_eq!(added.params[0].name, "x");
        assert_eq!(
            added.waivers,
            vec![waiver(
                38,
                Some("F01"),
                Some("9"),
                Some("measured on purpose")
            )]
        );
        assert_eq!(added.touch.start, 38);
    }

    #[test]
    fn a_block_comment_ends_the_waiver_run() {
        let source = "\
// slopmeter: waive F01 9 measured on purpose
/* note */
int kept() { return 1; }
";
        let file = parse("kept.hpp", source.as_bytes());
        assert!(function(&file, "kept").waivers.is_empty());
    }

    #[test]
    fn a_trailing_comment_on_a_code_line_is_not_a_waiver() {
        let source = "\
int earlier() { return 1; } // slopmeter: waive F01 9 measured on purpose
int later() { return 2; }

// slopmeter: waive F01 9 measured on purpose
int whole_line() { return 3; }
";
        let file = parse("kept.hpp", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{source}");
        let later = function(&file, "later");
        assert!(later.waivers.is_empty());
        assert_eq!(later.touch.start, later.span.start);
        assert_eq!(
            function(&file, "whole_line").waivers,
            vec![waiver(
                4,
                Some("F01"),
                Some("9"),
                Some("measured on purpose")
            )]
        );
    }

    #[test]
    fn a_literal_zero_arm_is_dropped_and_a_folded_condition_is_not() {
        let source = "\
int dead(int flag) {
#if 0
    if (flag) return 1;
#else
    return 2;
#endif
}

int folded(int flag) {
#if 0 && FLAG
    if (flag) return 1;
#else
    return 2;
#endif
}

int arms() {
#if defined(A) && defined(B)
    return 1;
#else
    return 2;
#endif
}
";
        let file = parse("arms.hpp", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{source}");
        let dead = function(&file, "dead");
        assert!(dead.body.decisions.is_empty());
        assert_eq!(dead.body.lloc, 1);
        let folded = function(&file, "folded");
        assert_eq!(folded.body.decisions.len(), 1);
        assert_eq!(folded.body.lloc, 3);
        let arms = function(&file, "arms");
        assert!(arms.body.decisions.is_empty());
        assert_eq!(arms.body.lloc, 2);
        assert!(arms.body.cognitive.is_empty());
    }

    #[test]
    fn cognitive_nesting_follows_the_white_paper() {
        let source = "\
int nested(bool a, bool b, bool c) {
    if (a) {
        if (b) {
            return 1;
        } else if (c) {
            return 2;
        }
        return 0;
    } else {
        return 0;
    }
}

int other(bool a, bool b) {
    if (a) {
        return 1;
    } else {
        if (b) {
            return 2;
        }
        return 0;
    }
}
";
        let file = parse("nest.hpp", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{source}");
        let nested = function(&file, "nested");
        assert_eq!(
            nested.body.decisions,
            vec![
                Decision { line: 2, depth: 1 },
                Decision { line: 3, depth: 2 },
                Decision { line: 5, depth: 2 },
            ]
        );
        assert_eq!(cognitive_total(&nested.body), 5);
        let other = function(&file, "other");
        assert_eq!(other.body.decisions.len(), 2);
        assert_eq!(cognitive_total(&other.body), 4);
    }

    #[test]
    fn boolean_runs_and_switch_labels_follow_the_rules() {
        let source = "\
bool same(bool a, bool b, bool c) { return a && b && c; }
bool changed(bool a, bool b, bool c) { return a && b || c; }
bool grouped(bool a, bool b, bool c) { return (a && b) && c; }
bool negated(bool a, bool b) { return !a && !b; }
bool words(bool a, bool b, bool c) { return a and b or c; }
int bits(int a, int b, int c) { return a & b | c; }

int classify(int x) {
    switch (x) {
    case 1:
    case 2:
        return 1;
    default:
        return 2;
    case 3:
        return 3;
    }
}

int paths(int x) {
    for (int i = 0; i < x; i++) {
        while (x > 0) {
            return x;
        }
    }
    do {
        break;
    } while (x);
    try {
        if (x < 0) return x ? -1 : 0;
    } catch (int) {
        return 0;
    }
    assert(x > 0);
    goto done;
done:
    return x;
}
";
        let file = parse("rules.hpp", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{source}");
        assert_eq!(function(&file, "same").body.decisions.len(), 2);
        assert_eq!(cognitive_total(&function(&file, "same").body), 1);
        assert_eq!(function(&file, "changed").body.decisions.len(), 2);
        assert_eq!(cognitive_total(&function(&file, "changed").body), 2);
        assert_eq!(function(&file, "grouped").body.decisions.len(), 2);
        assert_eq!(cognitive_total(&function(&file, "grouped").body), 2);
        assert_eq!(function(&file, "negated").body.decisions.len(), 1);
        assert_eq!(cognitive_total(&function(&file, "negated").body), 1);
        assert_eq!(function(&file, "words").body.decisions.len(), 2);
        assert_eq!(cognitive_total(&function(&file, "words").body), 2);
        assert!(function(&file, "bits").body.decisions.is_empty());

        let classify = function(&file, "classify");
        assert_eq!(classify.body.decisions.len(), 3);
        assert_eq!(cognitive_total(&classify.body), 1);

        let paths = function(&file, "paths");
        assert_eq!(paths.body.decisions.len(), 7);
        assert!(paths
            .body
            .cognitive
            .iter()
            .any(|item| item.kind == CognitiveKind::Fundamental));
    }

    #[test]
    fn nesting_is_the_deepest_structural_level() {
        let cases = [
            (
                "\
int boolean(bool a, bool b) {
    return a && b;
}
",
                "boolean",
                0,
                None,
            ),
            (
                "\
int conditional(bool a, bool b) {
    if (a && b) return 1;
    return 0;
}
",
                "conditional",
                1,
                Some(2),
            ),
            (
                "\
int ternary_bool(bool a, bool b) {
    return a && b ? 1 : 0;
}
",
                "ternary_bool",
                1,
                Some(2),
            ),
            (
                "\
int in_lambda(bool a) {
    auto pick = [](int y, bool flag) { return flag ? y : 0; };
    return pick(1, a);
}
",
                "in_lambda",
                1,
                Some(2),
            ),
            (
                "\
int nested_keeps_its_own(bool a) {
    struct Inner {
        static bool method(bool x) {
            if (x) return x;
            return false;
        }
    };
    return Inner::method(a);
}
",
                "nested_keeps_its_own",
                0,
                None,
            ),
            (
                "\
int nested_keeps_its_own(bool a) {
    struct Inner {
        static bool method(bool x) {
            if (x) return x;
            return false;
        }
    };
    return Inner::method(a);
}
",
                "nested_keeps_its_own::Inner::method",
                1,
                Some(4),
            ),
            (
                "\
int chained(bool a, bool b) {
    return a ? 1 : b ? 2 : 3;
}
",
                "chained",
                2,
                Some(2),
            ),
            (
                "\
int in_condition(bool a) {
    if (a ? 1 : 0) return 1;
    return 0;
}
",
                "in_condition",
                1,
                Some(2),
            ),
            (
                "\
int operand(bool a) {
    return (a ? 2 : 0) ? 1 : 3;
}
",
                "operand",
                1,
                Some(2),
            ),
            (
                "\
int in_body(bool a, bool b) {
    if (a) return b ? 1 : 0;
    return 0;
}
",
                "in_body",
                2,
                Some(2),
            ),
            (
                "\
int if_else(bool a) {
    if (a) return 1;
    else return 0;
}
",
                "if_else",
                1,
                Some(2),
            ),
            (
                "\
int else_if_chain(bool a, bool b) {
    if (a) return 1;
    else if (b) return 2;
    else return 0;
}
",
                "else_if_chain",
                1,
                Some(2),
            ),
            (
                "\
int else_holds_if(bool a, bool b) {
    if (a) return 1;
    else {
        if (b) return 2;
    }
    return 0;
}
",
                "else_holds_if",
                2,
                Some(4),
            ),
            (
                "\
int switches(int x) {
    switch (x) {
    case 1: return 1;
    case 2: return 2;
    }
    return 0;
}
",
                "switches",
                1,
                Some(2),
            ),
            (
                "\
int arm_if(int x) {
    switch (x) {
    case 1:
        if (x) return 1;
        break;
    }
    return 0;
}
",
                "arm_if",
                2,
                Some(4),
            ),
            (
                "\
int try_if(bool a) {
    try {
        if (a) return 1;
    } catch (...) {
        return 0;
    }
    return 0;
}
",
                "try_if",
                1,
                Some(3),
            ),
            (
                "\
int catch_if(bool a) {
    try {
        return 0;
    } catch (...) {
        if (a) return 1;
    }
    return 0;
}
",
                "catch_if",
                2,
                Some(5),
            ),
            (
                "\
int two_handlers(bool a) {
    try {
        return 0;
    } catch (int) {
        return 1;
    } catch (...) {
        if (a) return 2;
    }
    return 0;
}
",
                "two_handlers",
                2,
                Some(7),
            ),
            (
                "\
int nested_handlers(bool a) {
    try {
        return 0;
    } catch (...) {
        try {
            return 1;
        } catch (...) {
            if (a) return 2;
        }
    }
    return 0;
}
",
                "nested_handlers",
                3,
                Some(8),
            ),
            (
                "\
int preproc(bool a) {
#if 1
    if (a) return 1;
#endif
    return 0;
}
",
                "preproc",
                1,
                Some(3),
            ),
            (
                "\
int tied(bool a, bool b) {
    if (a) {
        if (b) return 1;
    } else {
        if (b) return 2;
    }
    return 0;
}
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
        let file = parse("nesting.hpp", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{name}\n{source}");
        let measured = crate::analyzers::measure(&file, &[]);
        let found = measured
            .iter()
            .find(|item| item.name == name)
            .unwrap_or_else(|| panic!("missing {name}"));
        (found.nesting, found.nesting_line)
    }

    #[test]
    fn logical_lines_skip_a_nested_method_and_count_a_lambda() {
        let source = "\
int empty() {}
int one() { return 1; }
int parent() {
    struct Inner {
        int method() { return 1; }
    };
    return 0;
}
int folded(bool y) {
    auto pick = [](bool y) { if (y) return 1; return 0; };
    return 0;
}
";
        let file = parse("lines.hpp", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{source}");
        assert_eq!(function(&file, "empty").body.lloc, 0);
        assert_eq!(function(&file, "one").body.lloc, 1);
        assert_eq!(function(&file, "parent").body.lloc, 2);
        assert_eq!(function(&file, "parent::Inner::method").body.lloc, 1);
        assert!(function(&file, "folded").body.lloc >= 3);
    }

    #[test]
    fn chains_and_forwarding_use_moves_and_arrows() {
        let source = "\
int send(int left, int right);
int request(int url, int params) { return send(url, params); }
int moved(int left, int right) { return send(std::move(left), right); }
int doubled(int value) { return send(std::move(std::forward<int>(value)), 0); }
int read(Hub& value) { return value.hop.again; }
int member(Repo& repo) { return repo.save(1); }
int rooted(Box& box) { return box.self.keep; }
struct Repo {
    int save(int user) { return this->store->put(user); }
};
";
        let file = parse("chain.hpp", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{source}");
        let request = function(&file, "request");
        assert!(request.body.chains.is_empty());
        assert_eq!(
            request
                .body
                .forward
                .as_ref()
                .map(|forward| forward.names.clone()),
            Some(vec!["url".to_string(), "params".to_string()])
        );
        let moved = function(&file, "moved");
        assert_eq!(
            moved
                .body
                .forward
                .as_ref()
                .map(|forward| forward.names.clone()),
            Some(vec!["left".to_string(), "right".to_string()])
        );
        assert!(function(&file, "doubled").body.forward.is_none());
        let read = function(&file, "read");
        assert_eq!(read.body.chains.len(), 1);
        assert!(!read.body.chains[0].rooted);
        assert_eq!(read.body.chains[0].members.len(), 2);
        let save = function(&file, "Repo::save");
        assert!(save.body.chains.iter().any(|chain| chain.rooted));
        assert!(save
            .body
            .chains
            .iter()
            .any(|chain| chain.members.len() == 2));
        assert!(function(&file, "member")
            .body
            .chains
            .iter()
            .any(|chain| !chain.rooted && chain.members.len() == 1));
    }

    #[test]
    fn a_bare_call_that_starts_a_member_chain_is_not_a_hop() {
        let source = "\
int call_result() { return make().a.b.c; }
int wrapped() { return (make()).a.b; }
int arrow() { return make()->a.b; }
int method_call(Obj& obj) { return obj.method().extra; }
int calls() { send(); return 0; }
";
        let file = parse("chain.hpp", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{source}");
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
        assert_eq!(
            member_names(&function(&file, "arrow").body.chains[0]),
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
        assert!(function(&file, "calls").body.chains[0].members[0].call);
    }

    #[test]
    fn a_pass_through_ignores_logging_a_dead_assignment_and_a_pack() {
        let source = "\
int logged(int a, int b, int c) {
    log(a);
    return run(a, b, c);
}

int dead(int a, int b) {
    int unused = 1;
    return run(a, b);
}

int alias(int a) {
    int tmp = a;
    return f(tmp);
}

int keyed(int xs) {
    return sorted(xs, flag);
}

int literal(int xs) {
    return sorted(xs, true);
}

template <typename... Args>
int relay(Args&&... args) {
    return send(std::forward<Args>(args)...);
}

template <typename... Args>
int bare_pack(Args&&... args) {
    return send(args...);
}

template <typename... Args>
int mapped(Args&&... args) {
    return send(foo(args)...);
}
";
        let file = parse("a.cpp", source.as_bytes());
        let names = |function_name: &str| {
            file.functions
                .iter()
                .find(|function| function.name == function_name)
                .map(|function| {
                    function
                        .body
                        .forward
                        .as_ref()
                        .map(|forward| forward.names.clone())
                })
        };
        assert_eq!(
            names("logged").flatten(),
            Some(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
            "{:?}",
            file.functions
                .iter()
                .map(|function| function.name.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            names("dead").flatten(),
            Some(vec!["a".to_string(), "b".to_string()])
        );
        assert_eq!(names("alias").flatten(), None);
        assert_eq!(names("keyed").flatten(), None);
        assert_eq!(names("literal").flatten(), Some(vec!["xs".to_string()]));
        assert_eq!(names("relay").flatten(), Some(vec!["args".to_string()]));
        assert_eq!(names("bare_pack").flatten(), Some(vec!["args".to_string()]));
        assert_eq!(names("mapped").flatten(), None);
    }

    #[test]
    fn a_class_is_not_part_of_a_later_function_span() {
        let source = "\
class App {
    int count() const { return 1; }
    CLI11_NODISCARD std::vector<App*> get_subcommands() const { return 2; }
    int after() const { return 3; }
};
";
        let file = parse("App.hpp", source.as_bytes());
        let after = function(&file, "App::after");
        assert_eq!(after.span.end - after.span.start, 0, "{:?}", file.functions);
        assert_eq!(
            function(&file, "App::count").span.end - function(&file, "App::count").span.start,
            0
        );
    }

    #[test]
    fn a_wrapping_error_counts_only_the_broken_line() {
        let source = "\
int ok() { return 1; }
int bad( { return 2; }
int also() { return 3; }
int more() { return 4; }
";
        let file = parse("wrap.hpp", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.25, "{}", file.unparsed_ratio);
    }

    #[test]
    fn a_split_if_reports_a_nonzero_ratio() {
        let source = "\
bool p_split_cond(int a, int b) {
    if (a > 0 &&
#ifdef X
        b > 0
#else
        b < 0
#endif
    ) {
        return true;
    }
    return false;
}
";
        let file = parse("split.hpp", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.25, "{}", file.unparsed_ratio);
    }

    #[test]
    fn a_function_try_block_constructor_is_scored() {
        let source = "\
struct P {
    int x, y;
    P(int a, int b) try : x(a), y(b) { if (a) throw 1; } catch (...) {}
};
";
        let file = parse("p.hpp", source.as_bytes());
        let names: Vec<_> = file
            .functions
            .iter()
            .map(|function| function.name.as_str())
            .collect();
        assert_eq!(names, vec!["P::P"]);
        let ctor = function(&file, "P::P");
        assert_eq!(ctor.param_types, vec!["int".to_string(), "int".to_string()]);
        assert_eq!(ctor.body.decisions.len(), 2);
    }

    #[test]
    fn a_member_initializer_list_counts_on_the_constructor() {
        let source = "\
struct Q {
    int x, y;
    Q() : x(flag_ ? 1 : 2), y(flag_ && other_ ? 3 : 4) {}
    Q(int a) : x([&] { if (a > 0) { if (a > 10) return 2; return 1; } return 0; }()) {}
    Q(int a, int b) try : x(a > 0 ? a : -a) { } catch (...) {}
};
";
        let file = parse("q.hpp", source.as_bytes());
        let names: Vec<_> = file
            .functions
            .iter()
            .map(|function| function.name.as_str())
            .collect();
        assert_eq!(names, vec!["Q::Q", "Q::Q", "Q::Q"]);
        let plain = file
            .functions
            .iter()
            .find(|function| function.param_types.is_empty())
            .unwrap();
        assert_eq!(plain.body.decisions.len(), 3);
        let hidden = file
            .functions
            .iter()
            .find(|function| function.param_types == vec!["int".to_string()])
            .unwrap();
        let depths: Vec<_> = hidden
            .body
            .decisions
            .iter()
            .map(|decision| decision.depth)
            .collect();
        assert_eq!(depths, vec![2, 3]);
        let tried = file
            .functions
            .iter()
            .find(|function| function.param_types.len() == 2)
            .unwrap();
        assert_eq!(tried.body.decisions.len(), 2);
    }

    #[test]
    fn a_macro_wrapped_block_stays_on_its_parent() {
        let source = "\
#define SECTION(n)
#define BOOST_FOREACH(v, c) for (auto v : c)
int ph_sections(int a) {
    int r = 0;
    SECTION(first) {
        if (a > 0) r++;
        if (a > 1) r++;
    }
    SECTION(\"alpha\") {
        if (a > 2) r++;
    }
    return r;
}
int ph_foreach(int v) {
    int r = 0;
    BOOST_FOREACH(x, v) {
        if (x) r++;
    }
    return r;
}
int ph_gnu(int a) {
    int helper(int x) { if (x) return 1; return 0; }
    if (a) return helper(a);
    return 0;
}
int parent(int a) {
    struct Local {
        int x;
        Local() : x(a > 0 ? 1 : 0) {}
    };
    return 0;
}
";
        let file = parse("macros.hpp", source.as_bytes());
        let names: Vec<_> = file
            .functions
            .iter()
            .map(|function| function.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec![
                "ph_sections",
                "ph_foreach",
                "ph_gnu",
                "ph_gnu::helper",
                "parent",
                "parent::Local::Local",
            ]
        );
        assert_eq!(function(&file, "ph_sections").body.decisions.len(), 3);
        assert_eq!(function(&file, "ph_foreach").body.decisions.len(), 1);
        assert_eq!(function(&file, "ph_gnu").body.decisions.len(), 1);
        assert_eq!(function(&file, "ph_gnu::helper").body.decisions.len(), 1);
        assert_eq!(
            function(&file, "parent::Local::Local").body.decisions.len(),
            1
        );
    }

    #[test]
    fn a_function_like_define_is_counted_on_the_file() {
        let source = "\
#define MAX 3
#define CHECK(x) if (!(x)) return false;
#define NOOP(x)
#if 0
#define DEAD(x) if (x) return;
#else
#define ELSE_ARM(x) if (x) return;
#endif
#ifdef FLAG
#define A(x) if (x) return 1;
#else
#define B(x) if (x) return 2;
#endif
void local() {
#define INNER(x) if (x) return;
}
bool run(int x) { CHECK(x); return true; }
bool both(int a, int b) { return CHECK(a && b); }
";
        let file = parse("macros.hpp", source.as_bytes());
        assert_eq!(file.unscored_macros, 6, "{source}");
        assert_eq!(function(&file, "run").body.decisions.len(), 0);
        assert_eq!(function(&file, "both").body.decisions.len(), 1);
    }

    #[test]
    fn an_empty_catch_records_its_line() {
        let cases = [
            (
                "\
void load() {
    try {
        read();
    } catch (...) {
    }
}
",
                "load",
                vec![4],
            ),
            (
                "\
void load() {
    try {
        read();
    } catch (...) {
        // swallowed
    }
}
",
                "load",
                vec![4],
            ),
            (
                "\
void load() {
    while (ready()) {
        try {
            read();
        } catch (...) {
            continue;
        }
    }
}
",
                "load",
                vec![5],
            ),
            (
                "\
void load() {
    while (ready()) {
        try {
            read();
        } catch (...) {
            break;
        }
    }
}
",
                "load",
                vec![5],
            ),
            (
                "\
void load() {
    while (ready()) {
        try {
            read();
        } catch (int) {
        } catch (...) {
            continue;
        }
    }
}
",
                "load",
                vec![5, 6],
            ),
            (
                "\
void load() {
    auto inner = [] {
        try {
            read();
        } catch (...) {
        }
    };
}
",
                "load",
                vec![5],
            ),
            (
                "\
struct P {
    int x, y;
    P(int a, int b) try : x(a), y(b) { if (a) throw 1; } catch (...) {}
};
",
                "P::P",
                vec![3],
            ),
        ];
        for (source, name, lines) in cases {
            let file = parse("a.hpp", source.as_bytes());
            assert_eq!(file.unparsed_ratio, 0.0, "{source}");
            assert_eq!(function(&file, name).body.swallowing, lines, "{source}");
        }
    }

    #[test]
    fn a_catch_that_handles_the_error_records_nothing() {
        let cases = [
            "\
void load() {
    try {
        read();
    } catch (...) {
        throw;
    }
}
",
            "\
void load() {
    try {
        read();
    } catch (...) {
        return;
    }
}
",
            "\
void load() {
    try {
        read();
    } catch (...) {
        log();
    }
}
",
            "\
void load() {
    int x = 0;
    try {
        read();
    } catch (...) {
        x = 1;
    }
}
",
            "\
void load() {
    try {
        read();
    } catch (...) {
        ;
    }
}
",
        ];
        for source in cases {
            let file = parse("a.hpp", source.as_bytes());
            assert_eq!(file.unparsed_ratio, 0.0, "{source}");
            assert!(
                function(&file, "load").body.swallowing.is_empty(),
                "{source}"
            );
        }
    }

    #[test]
    fn a_broken_file_still_returns_a_cpp_file() {
        let source = b"int broken( { return 1; }\n";
        let file = parse("broken.hpp", source);
        assert_eq!(file.language, Language::Cpp);
        assert!(file.unparsed_ratio > 0.0);
    }

    #[test]
    fn an_inline_open_records_the_call_and_the_constructed_stream() {
        let source = "\
void load(const char* path) {
    fopen(path, \"r\");
    std::ifstream in(path);
    std::ifstream bare;
    socket(1, 2, 3);
}
void take(std::ifstream& in) {}
";
        let file = parse("a.cpp", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{}", file.unparsed_ratio);
        let names: Vec<_> = function(&file, "load")
            .body
            .io
            .iter()
            .map(|site| site.name.as_str())
            .collect();
        assert_eq!(names, vec!["fopen", "std::ifstream", "socket"], "{names:?}");
        assert!(function(&file, "take").body.io.is_empty());
    }

    #[test]
    fn a_void_function_records_a_non_const_reference_or_pointer() {
        let source = "\
void fill(
    int& n) {
    n = 1;
}
void point(int* n) {}
void pinned(int* const n) {}
void moved(int&& n) {}
void frozen(const int& n) {}
void east(int const& n) {}
void readonly(const int* n) {}
void read(const char* path) {}
void value(int n) {}
int give(int& n) { return n; }
void frozen_ref(const int&& n) {}
auto trailing(int& n) -> void {}
const auto lamb = [](int& n) -> void {};
struct Box {
    Box(int& n) {}
    void take(int& n) {}
};
void text(std::string& name) {}
void mixed(
    const int& a,
    int* b) {}
";
        let file = parse("a.cpp", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{source}");
        assert_eq!(function(&file, "fill").body.mutations, vec![2]);
        assert_eq!(function(&file, "point").body.mutations, vec![5]);
        assert_eq!(function(&file, "pinned").body.mutations, vec![6]);
        assert_eq!(function(&file, "moved").body.mutations, vec![7]);
        assert!(function(&file, "frozen").body.mutations.is_empty());
        assert!(function(&file, "east").body.mutations.is_empty());
        assert!(function(&file, "readonly").body.mutations.is_empty());
        assert!(function(&file, "read").body.mutations.is_empty());
        assert!(function(&file, "value").body.mutations.is_empty());
        assert!(function(&file, "give").body.mutations.is_empty());
        assert!(function(&file, "frozen_ref").body.mutations.is_empty());
        assert_eq!(function(&file, "trailing").body.mutations, vec![15]);
        assert_eq!(function(&file, "lamb").body.mutations, vec![16]);
        assert!(function(&file, "Box::Box").body.mutations.is_empty());
        assert_eq!(function(&file, "Box::take").body.mutations, vec![19]);
        assert_eq!(function(&file, "text").body.mutations, vec![21]);
        assert_eq!(function(&file, "mixed").body.mutations, vec![24]);
    }

    #[test]
    fn a_null_literal_is_a_return_or_a_call_argument() {
        let source = "\
int* missing() {
    return nullptr;
}
int* absent() {
    return NULL;
}
void send() {
    take(nullptr);
    take(NULL);
}
void compare(int* p) {
    if (p == nullptr) {
        return;
    }
    if (p == NULL) {
        return;
    }
}
int* stored() {
    int* p = nullptr;
    return p;
}
int* wrapped() {
    return (nullptr);
}
void pair() {
    take(nullptr, NULL);
}
int* parent() {
    struct Inner {
        int* method() { return nullptr; }
    };
    return 0;
}
int* folded() {
    auto pick = []() { return nullptr; };
    return pick();
}
";
        let file = parse("a.cpp", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{source}");
        assert_eq!(function(&file, "missing").body.nulls, vec![2]);
        assert_eq!(function(&file, "absent").body.nulls, vec![5]);
        assert_eq!(function(&file, "send").body.nulls, vec![8, 9]);
        assert!(function(&file, "compare").body.nulls.is_empty());
        assert!(function(&file, "stored").body.nulls.is_empty());
        assert_eq!(function(&file, "wrapped").body.nulls, vec![24]);
        assert_eq!(function(&file, "pair").body.nulls, vec![27, 27]);
        assert!(function(&file, "parent").body.nulls.is_empty());
        assert_eq!(
            function(&file, "parent::Inner::method").body.nulls,
            vec![31]
        );
        assert_eq!(function(&file, "folded").body.nulls, vec![36]);
    }
}
