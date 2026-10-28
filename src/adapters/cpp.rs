use std::collections::BTreeSet;
use std::ops::Range;

use tree_sitter::Node;

use crate::ir::{
    Body, Chain, Cognitive, CognitiveKind, Decision, File, Forward, Function, Language, Member,
    Param, Span, WaiverLine,
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

enum Scope {
    Namespace(String),
    Class(String),
    Function(String),
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
            functions: Vec::new(),
        };
    };
    let root = tree.root_node();
    let mut functions = Vec::new();
    let mut scopes = Vec::new();
    visit(root, source, &mut scopes, &mut functions);
    File {
        path: path.to_string(),
        language: Language::Cpp,
        unparsed_ratio: super::unparsed_ratio(root, source),
        functions,
    }
}

fn visit(node: Node, source: &[u8], scopes: &mut Vec<Scope>, out: &mut Vec<Function>) {
    match node.kind() {
        "function_definition" => visit_function(node, source, scopes, out),
        "namespace_definition" => visit_namespace(node, source, scopes, out),
        "class_specifier" | "struct_specifier" | "union_specifier" => {
            visit_class(node, source, scopes, out);
        }
        "declaration" => visit_declaration(node, source, scopes, out),
        "preproc_if" | "preproc_elif" => visit_preproc(node, source, scopes, out, true),
        "preproc_ifdef" | "preproc_elifdef" | "preproc_else" => {
            visit_preproc(node, source, scopes, out, false);
        }
        "preproc_def" | "preproc_function_def" | "preproc_include" | "preproc_call" | "comment" => {
        }
        _ => visit_children(node, source, scopes, out),
    }
}

fn visit_children(node: Node, source: &[u8], scopes: &mut Vec<Scope>, out: &mut Vec<Function>) {
    for child in named_nodes(node) {
        visit(child, source, scopes, out);
    }
}

fn visit_preproc(
    node: Node,
    source: &[u8],
    scopes: &mut Vec<Scope>,
    out: &mut Vec<Function>,
    check_zero: bool,
) {
    let condition = node.child_by_field_name("condition");
    let alternative = node.child_by_field_name("alternative");
    let dead = check_zero && condition.is_some_and(|condition| is_literal_zero(condition, source));
    if !dead {
        for child in named_nodes(node) {
            if Some(child) == condition || Some(child) == alternative {
                continue;
            }
            visit(child, source, scopes, out);
        }
    }
    if let Some(alternative) = alternative {
        visit(alternative, source, scopes, out);
    }
}

fn visit_namespace(node: Node, source: &[u8], scopes: &mut Vec<Scope>, out: &mut Vec<Function>) {
    let parts = node
        .child_by_field_name("name")
        .map(|name| name_parts(name, source))
        .unwrap_or_default();
    for part in &parts {
        scopes.push(Scope::Namespace(part.clone()));
    }
    if let Some(body) = node.child_by_field_name("body") {
        visit(body, source, scopes, out);
    }
    for _ in &parts {
        scopes.pop();
    }
}

fn visit_class(node: Node, source: &[u8], scopes: &mut Vec<Scope>, out: &mut Vec<Function>) {
    let parts = node
        .child_by_field_name("name")
        .map(|name| name_parts(name, source))
        .unwrap_or_default();
    for part in &parts {
        scopes.push(Scope::Class(part.clone()));
    }
    if let Some(body) = node.child_by_field_name("body") {
        visit(body, source, scopes, out);
    }
    for _ in &parts {
        scopes.pop();
    }
}

fn visit_function(node: Node, source: &[u8], scopes: &mut Vec<Scope>, out: &mut Vec<Function>) {
    let Some(body) = written_body(node) else {
        return;
    };
    let declarator = node.child_by_field_name("declarator");
    let real = declarator.is_some_and(|declarator| {
        find_kind(declarator, "function_declarator").is_some()
            || find_kind(declarator, "operator_cast").is_some()
    });
    if !real {
        visit_children(node, source, scopes, out);
        return;
    }
    let (global, parts) = declarator
        .map(|declarator| qualified_parts(declarator, source))
        .unwrap_or((false, Vec::new()));
    if parts.is_empty() {
        visit_children(node, source, scopes, out);
        return;
    }
    if macro_wrapped_block(node, scopes) {
        visit(body, source, scopes, out);
        return;
    }
    let name = parts.last().cloned().unwrap_or_default();
    let params = parameter_list(node);
    let qualified = qualify(scopes, global, &parts);
    push_function(out, source, &qualified, node, node, params);
    scopes.push(Scope::Function(name));
    visit(body, source, scopes, out);
    scopes.pop();
}

fn visit_declaration(node: Node, source: &[u8], scopes: &mut Vec<Scope>, out: &mut Vec<Function>) {
    let bindings = if in_function(scopes) {
        Vec::new()
    } else {
        bound_lambdas(node, source)
    };
    if !bindings.is_empty() {
        for (name, lambda) in bindings {
            let params = lambda
                .child_by_field_name("declarator")
                .and_then(|declarator| declarator.child_by_field_name("parameters"));
            let qualified = qualify(scopes, false, std::slice::from_ref(&name));
            push_function(out, source, &qualified, node, lambda, params);
            scopes.push(Scope::Function(name));
            if let Some(body) = lambda.child_by_field_name("body") {
                visit(body, source, scopes, out);
            }
            scopes.pop();
        }
        return;
    }
    visit_children(node, source, scopes, out);
}

fn bound_lambdas<'a>(node: Node<'a>, source: &[u8]) -> Vec<(String, Node<'a>)> {
    named_nodes(node)
        .into_iter()
        .filter(|child| child.kind() == "init_declarator")
        .filter_map(|child| {
            let lambda = peel_lambda(child.child_by_field_name("value")?)?;
            let name = child
                .child_by_field_name("declarator")
                .and_then(|declarator| declarator_identifier(declarator))
                .map(|ident| node_text(ident, source))
                .unwrap_or_default();
            if name.is_empty() {
                None
            } else {
                Some((name, lambda))
            }
        })
        .collect()
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
    node.child_by_field_name("type").is_none() && !has_typed_parameters(node)
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
        | "variadic_parameter_declaration" => {
            if param.child_by_field_name("declarator").is_some() {
                return true;
            }
            param
                .child_by_field_name("type")
                .is_some_and(|ty| ty.kind() != "type_identifier")
        }
        _ => false,
    }
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

fn push_function(
    out: &mut Vec<Function>,
    source: &[u8],
    name: &str,
    item: Node<'_>,
    body: Node<'_>,
    params: Option<Node<'_>>,
) {
    let mut span = span_of(item);
    if let Some(start) = content_line(item) {
        if start > span.start {
            span.start = start;
        }
    }
    if item.kind() == "function_definition" {
        if let Some(declarator) = item.child_by_field_name("declarator") {
            let absorbed = named_nodes(item).iter().any(|child| {
                matches!(
                    child.kind(),
                    "class_specifier" | "struct_specifier" | "union_specifier" | "enum_specifier"
                ) && child.end_byte() <= declarator.start_byte()
                    && child.child_by_field_name("body").is_some()
            });
            if absorbed {
                span.start = line_of(declarator);
            }
        }
    }
    let attributes = leading_attributes(item);
    let anchor = attributes.first().copied().unwrap_or(item);
    let waivers = waivers_above(anchor, source);
    let mut touch = span;
    if let Some(attribute) = attributes.first() {
        touch.start = span_of(*attribute).start.min(touch.start);
    }
    if let Some(first) = waivers.first() {
        touch.start = first.line;
    }
    let params = parameters_of(params, source);
    let ordinary: Vec<String> = params
        .iter()
        .filter(|param| !param.receiver && !param.name.is_empty())
        .map(|param| param.name.clone())
        .collect();
    let receiver = params.iter().find_map(|param| {
        if param.receiver && !param.name.is_empty() {
            Some(param.name.clone())
        } else {
            None
        }
    });
    let receiver = receiver.or_else(|| {
        if body.kind() == "lambda_expression" {
            None
        } else {
            Some("this".to_string())
        }
    });
    let (param_types, qualifier) = if body.kind() == "lambda_expression" {
        (
            parameter_types(params_node_of_lambda(body), source),
            String::new(),
        )
    } else {
        signature(body, source)
    };
    let classified = classify(body, source, receiver.as_deref(), &ordinary);
    out.push(Function {
        name: name.to_string(),
        param_types,
        qualifier,
        span,
        touch,
        params,
        body: classified,
        waivers,
    });
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
    node.child_by_field_name("declarator")?
        .child_by_field_name("parameters")
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
        if !child.is_named() {
            if node_text(child, source) == "..." {
                params.push(ParsedParam {
                    name: "...".to_string(),
                    receiver: false,
                    ty: "...".to_string(),
                });
            }
            continue;
        }
        match child.kind() {
            "explicit_object_parameter_declaration" => {
                if let Some(inner) = named_nodes(child)
                    .into_iter()
                    .find(|inner| inner.kind() == "parameter_declaration")
                {
                    let mut param = one_parameter(inner, source);
                    param.receiver = true;
                    params.push(param);
                }
            }
            "parameter_declaration"
            | "optional_parameter_declaration"
            | "variadic_parameter_declaration" => params.push(one_parameter(child, source)),
            _ => {}
        }
    }
    params
}

fn one_parameter(node: Node, source: &[u8]) -> ParsedParam {
    let name_node = node
        .child_by_field_name("declarator")
        .and_then(declarator_identifier);
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
    let mut end = range.end;
    if let Some(value) = node.child_by_field_name("default_value") {
        end = end.min(value.start_byte());
    }
    if end < range.start || end > source.len() || range.start > source.len() {
        return String::new();
    }
    let mut text = String::from_utf8_lossy(&source[range.start..end]).into_owned();
    if let Some(name) = name {
        let start = name.start_byte().saturating_sub(range.start);
        let stop = name.end_byte().saturating_sub(range.start);
        if start < stop && stop <= text.len() {
            text.replace_range(start..stop, " ");
        }
    }
    text.trim_end_matches(|ch: char| ch == '=' || ch.is_whitespace())
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn declarator_identifier(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "identifier" | "field_identifier" => Some(node),
        "pointer_declarator"
        | "reference_declarator"
        | "function_declarator"
        | "abstract_function_declarator"
        | "parenthesized_declarator" => node
            .child_by_field_name("declarator")
            .and_then(declarator_identifier)
            .or_else(|| {
                named_nodes(node)
                    .into_iter()
                    .find_map(declarator_identifier)
            }),
        _ => named_nodes(node)
            .into_iter()
            .find_map(declarator_identifier),
    }
}

fn qualified_parts(node: Node, source: &[u8]) -> (bool, Vec<String>) {
    if let Some(function) = find_kind(node, "function_declarator") {
        if let Some(declarator) = function.child_by_field_name("declarator") {
            return qualified_parts(declarator, source);
        }
    }
    if node.kind() == "qualified_identifier" || node.kind() == "operator_cast" {
        let global = flat_text(node, source).starts_with("::");
        return (global, name_parts(node, source));
    }
    if let Some(ident) = declarator_identifier(node) {
        if matches!(node.kind(), "identifier" | "field_identifier") || ident.kind() == "identifier"
        {
            let parts = name_parts(node, source);
            if !parts.is_empty() {
                return (false, parts);
            }
        }
    }
    let parts = name_parts(node, source);
    (flat_text(node, source).starts_with("::"), parts)
}

fn name_parts(node: Node, source: &[u8]) -> Vec<String> {
    match node.kind() {
        "qualified_identifier" => {
            let mut parts = Vec::new();
            if let Some(scope) = node.child_by_field_name("scope") {
                parts.extend(name_parts(scope, source));
            }
            if let Some(name) = node.child_by_field_name("name") {
                parts.extend(name_parts(name, source));
            }
            if parts.is_empty() {
                let text = flat_text(node, source);
                if !text.is_empty() && text != "::" {
                    parts.push(text.trim_start_matches("::").to_string());
                }
            }
            parts
        }
        "identifier"
        | "field_identifier"
        | "namespace_identifier"
        | "type_identifier"
        | "primitive_type" => {
            vec![node_text(node, source)]
        }
        "operator_name" | "destructor_name" | "operator_cast" | "template_type"
        | "template_function" | "template_method" => {
            vec![flat_text(node, source)]
        }
        "nested_namespace_specifier" => named_nodes(node)
            .into_iter()
            .flat_map(|child| name_parts(child, source))
            .collect(),
        _ => named_nodes(node)
            .into_iter()
            .flat_map(|child| name_parts(child, source))
            .collect(),
    }
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
    let mut found = Vec::new();
    let mut cursor = node.prev_sibling();
    while let Some(sibling) = cursor {
        if is_attribute(sibling.kind()) {
            found.push(sibling);
        } else if sibling.kind() != "comment" {
            break;
        }
        cursor = sibling.prev_sibling();
    }
    found.reverse();
    let content = named_nodes(node)
        .into_iter()
        .find(|child| !is_attribute(child.kind()))
        .map(|child| child.start_byte());
    for child in named_nodes(node) {
        if is_attribute(child.kind()) && content.is_none_or(|start| child.start_byte() < start) {
            found.push(child);
        }
    }
    found.sort_by_key(|item| item.start_byte());
    found
}

fn is_attribute(kind: &str) -> bool {
    matches!(kind, "attribute_declaration" | "attribute_specifier")
}

fn waivers_above(node: Node, source: &[u8]) -> Vec<WaiverLine> {
    let mut next_line = span_of(node).start;
    let mut found = Vec::new();
    let mut cursor = node.prev_sibling();
    while let Some(sibling) = cursor {
        if !is_line_comment(sibling, source) {
            break;
        }
        let span = span_of(sibling);
        if span.start != span.end || span.start + 1 != next_line {
            break;
        }
        let Some(waiver) = waiver_from_comment(&node_text(sibling, source), span.start) else {
            break;
        };
        found.push(waiver);
        next_line = span.start;
        cursor = sibling.prev_sibling();
    }
    found.reverse();
    found
}

fn is_line_comment(node: Node, source: &[u8]) -> bool {
    node.kind() == "comment" && node_text(node, source).trim_start().starts_with("//")
}

fn waiver_from_comment(text: &str, line: u32) -> Option<WaiverLine> {
    let text = text.trim();
    let text = text.strip_prefix("//").unwrap_or(text).trim();
    let (marker, rest) = take_token(text)?;
    if marker != "slopmeter:" {
        return None;
    }
    let (verb, rest) = take_token(rest)?;
    if verb != "waive" {
        return None;
    }
    let (rule, rest) = match take_token(rest) {
        Some((rule, rest)) => (Some(rule.to_string()), rest),
        None => (None, ""),
    };
    let (number, reason) = match take_token(rest) {
        Some((token, after)) if is_number_token(token) => {
            (Some(token.to_string()), non_empty(after))
        }
        Some(_) => (None, non_empty(rest)),
        None => (None, None),
    };
    Some(WaiverLine {
        line,
        rule,
        number,
        reason,
    })
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
        for scope in scopes {
            let name = match scope {
                Scope::Namespace(name) | Scope::Class(name) | Scope::Function(name) => name,
            };
            names.push(name.as_str());
        }
    }
    for part in parts {
        names.push(part);
    }
    names.join("::")
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
    let mut depth = 0i32;
    for (index, character) in text.chars().enumerate() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 || (depth == 0 && index + 1 != text.len()) {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

fn classify(node: Node<'_>, source: &[u8], receiver: Option<&str>, ordinary: &[String]) -> Body {
    let mut classifier = Classifier {
        source,
        receiver,
        skip: None,
        lloc: 0,
        decisions: Vec::new(),
        cognitive: Vec::new(),
        chains: Vec::new(),
    };
    let forward = classifier.forwarding(node, ordinary);
    match node.kind() {
        "function_definition" => {
            for child in code_nodes(node) {
                if child.kind() == "field_initializer_list" {
                    classifier.walk(child, 0);
                }
            }
            if let Some(body) = written_body(node) {
                classifier.walk(body, 0);
            }
        }
        "lambda_expression" => {
            if let Some(body) = node.child_by_field_name("body") {
                classifier.walk(body, 0);
            }
        }
        _ => {}
    }
    Body {
        lloc: classifier.lloc,
        decisions: classifier.decisions,
        cognitive: classifier.cognitive,
        chains: classifier.chains,
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
    decisions: Vec<Decision>,
    cognitive: Vec<Cognitive>,
    chains: Vec<Chain>,
}

impl<'a> Classifier<'a> {
    fn forwarding(&mut self, node: Node<'a>, ordinary: &[String]) -> Option<Forward> {
        if ordinary.is_empty() {
            return None;
        }
        let (line, call) = forwarding_call(node)?;
        if !call_is_clean(call, self.source) {
            return None;
        }
        self.skip = Some(chain_root(call).byte_range());
        Some(Forward {
            line,
            names: forwarded_names(call, self.source, ordinary),
        })
    }

    fn walk(&mut self, node: Node<'a>, nesting: u32) {
        match node.kind() {
            "if_statement" => self.walk_if(node, nesting, false),
            "while_statement" => self.walk_while(node, nesting),
            "do_statement" => self.walk_do(node, nesting),
            "for_statement" | "for_range_loop" | "expansion_statement" => {
                self.walk_for(node, nesting);
            }
            "switch_statement" => self.walk_switch(node, nesting),
            "try_statement" => self.walk_try(node, nesting),
            "catch_clause" => self.walk_catch(node, nesting),
            "conditional_expression" => self.walk_ternary(node, nesting),
            "lambda_expression" => self.walk_lambda(node, nesting),
            "binary_expression" => {
                if bool_op(node, self.source).is_some() {
                    self.walk_boolean(node, nesting);
                } else {
                    self.walk_named(node, nesting);
                }
            }
            "preproc_if" | "preproc_elif" => self.walk_preproc(node, nesting, true),
            "preproc_ifdef" | "preproc_elifdef" | "preproc_else" => {
                self.walk_preproc(node, nesting, false);
            }
            "preproc_def" | "preproc_function_def" | "preproc_include" | "preproc_call" => {}
            "compound_statement" => self.walk_compound(node, nesting),
            "expression_statement" => {
                self.lloc += 1;
                self.walk_named(node, nesting);
            }
            "declaration" => self.walk_declaration(node, nesting),
            "return_statement"
            | "co_return_statement"
            | "co_yield_statement"
            | "throw_statement" => {
                self.lloc += 1;
                self.walk_named(node, nesting);
            }
            "break_statement" | "continue_statement" | "seh_leave_statement" => self.lloc += 1,
            "goto_statement" => {
                self.lloc += 1;
                self.fundamental(line_of(node));
            }
            "case_statement" => self.walk_case(node, nesting),
            "labeled_statement" | "attributed_statement" | "linkage_specification" => {
                self.walk_named(node, nesting);
            }
            "empty_statement" | "comment" => {}
            "function_definition" => {
                if folds_into_parent(node) {
                    if let Some(body) = written_body(node) {
                        self.walk(body, nesting);
                    }
                }
            }
            "class_specifier"
            | "struct_specifier"
            | "union_specifier"
            | "enum_specifier"
            | "alias_declaration"
            | "using_declaration"
            | "static_assert_declaration"
            | "type_definition" => self.lloc += 1,
            "field_expression"
            | "call_expression"
            | "subscript_expression"
            | "parenthesized_expression"
            | "template_function"
            | "template_method" => {
                if node.kind() == "call_expression" && callee_leaf(node, self.source) == "assert" {
                    self.decision(line_of(node), nesting);
                }
                self.record_chain(node);
                self.walk_named(node, nesting);
            }
            _ => self.walk_named(node, nesting),
        }
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
        let condition = node.child_by_field_name("condition");
        let alternative = node.child_by_field_name("alternative");
        let dead = check_zero
            && condition.is_some_and(|condition| is_literal_zero(condition, self.source));
        if !dead {
            for child in code_nodes(node) {
                if Some(child) == condition || Some(child) == alternative {
                    continue;
                }
                self.walk(child, nesting);
            }
        }
        if let Some(alternative) = alternative {
            self.walk(alternative, nesting);
        }
    }

    fn walk_if(&mut self, node: Node<'a>, nesting: u32, else_if: bool) {
        let line = line_of(node);
        if else_if {
            self.hybrid(line);
        } else {
            self.lloc += 1;
            self.structural(line, nesting);
        }
        self.decision(line, nesting);
        if let Some(condition) = node.child_by_field_name("condition") {
            self.walk_condition(condition, nesting);
        }
        if let Some(body) = node.child_by_field_name("consequence") {
            self.walk(body, nesting + 1);
        }
        if let Some(alternative) = node.child_by_field_name("alternative") {
            let inner = code_nodes(alternative);
            if let Some(child) = inner.first() {
                if child.kind() == "if_statement" {
                    self.walk_if(*child, nesting, true);
                } else {
                    self.hybrid(line_of(alternative));
                    self.walk(*child, nesting + 1);
                }
            }
        }
    }

    fn walk_while(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.decision(line_of(node), nesting);
        self.structural(line_of(node), nesting);
        if let Some(condition) = node.child_by_field_name("condition") {
            self.walk_condition(condition, nesting);
        }
        if let Some(body) = node.child_by_field_name("body") {
            self.walk(body, nesting + 1);
        }
    }

    fn walk_do(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.decision(line_of(node), nesting);
        self.structural(line_of(node), nesting);
        if let Some(body) = node.child_by_field_name("body") {
            self.walk(body, nesting + 1);
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
            self.walk(body, nesting + 1);
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
            self.walk(child, nesting + 1);
        }
    }

    fn walk_try(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        for child in code_nodes(node) {
            if child.kind() == "field_initializer_list" {
                self.walk(child, nesting);
            }
        }
        if let Some(body) = node.child_by_field_name("body") {
            self.walk(body, nesting);
        }
        for child in code_nodes(node) {
            if child.kind() == "catch_clause" {
                self.walk_catch(child, nesting);
            }
        }
    }

    fn walk_catch(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.decision(line_of(node), nesting);
        self.structural(line_of(node), nesting);
        if let Some(body) = node.child_by_field_name("body") {
            self.walk(body, nesting + 1);
        }
    }

    fn walk_ternary(&mut self, node: Node<'a>, nesting: u32) {
        self.decision(line_of(node), nesting);
        self.structural(line_of(node), nesting);
        if let Some(condition) = node.child_by_field_name("condition") {
            self.walk(condition, nesting);
        }
        if let Some(consequence) = node.child_by_field_name("consequence") {
            self.walk(consequence, nesting + 1);
        }
        if let Some(alternative) = node.child_by_field_name("alternative") {
            self.walk(alternative, nesting + 1);
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
        if node.kind() == "declaration" || node.kind() == "init_statement" {
            for child in code_nodes(node) {
                if let Some(value) = child.child_by_field_name("value") {
                    self.walk(value, nesting);
                } else if child.kind() != "init_declarator" {
                    self.walk_value(child, nesting);
                }
            }
            return;
        }
        self.walk(node, nesting);
    }

    fn walk_boolean(&mut self, node: Node<'a>, nesting: u32) {
        let mut last = None;
        self.bool_spine(node, nesting, &mut last);
    }

    fn bool_spine(&mut self, node: Node<'a>, nesting: u32, last: &mut Option<BoolOp>) {
        if is_negation(node, self.source) {
            if let Some(inner) = negation_inner(node) {
                self.bool_spine(inner, nesting, last);
            }
            return;
        }
        if node.kind() == "parenthesized_expression" {
            self.record_chain(node);
            let mut inner = None;
            for child in code_nodes(node) {
                self.bool_spine(child, nesting, &mut inner);
            }
            return;
        }
        let Some(operator) = bool_op(node, self.source) else {
            self.walk(node, nesting);
            return;
        };
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
        let bare_forward = self.skip.as_ref() == Some(&node.byte_range())
            && peeled.root.is_none()
            && peeled.hops.len() == 1;
        if bare_forward {
            return;
        }
        let line = peeled.hops[0].line;
        let rooted = self.receiver.is_some_and(|receiver| {
            peeled.root.as_deref() == Some(receiver) || peeled.root.as_deref() == Some("this")
        });
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
    let body = match node.kind() {
        "function_definition" => written_body(node)?,
        "lambda_expression" => node.child_by_field_name("body")?,
        _ => return None,
    };
    if body.kind() != "compound_statement" {
        return None;
    }
    let children = code_nodes(body);
    let [statement] = children.as_slice() else {
        return None;
    };
    Some((line_of(*statement), unwrap_call(*statement)?))
}

fn unwrap_call(node: Node) -> Option<Node> {
    match node.kind() {
        "call_expression" => Some(node),
        "expression_statement"
        | "return_statement"
        | "co_return_statement"
        | "parenthesized_expression" => {
            let children = code_nodes(node);
            let [child] = children.as_slice() else {
                return None;
            };
            unwrap_call(*child)
        }
        _ => None,
    }
}

fn call_is_clean(call: Node, source: &[u8]) -> bool {
    if let Some(function) = call.child_by_field_name("function") {
        if nested_call(function, source) {
            return false;
        }
    }
    if let Some(arguments) = call.child_by_field_name("arguments") {
        for argument in code_nodes(arguments) {
            if nested_call(argument, source) {
                return false;
            }
        }
    }
    true
}

fn nested_call(node: Node, source: &[u8]) -> bool {
    if identity_named(node, source) {
        return node
            .child_by_field_name("arguments")
            .is_some_and(|arguments| {
                code_nodes(arguments)
                    .iter()
                    .any(|child| contains_call(*child))
            });
    }
    if node.kind() == "call_expression" {
        return true;
    }
    for index in 0..node.child_count() {
        if let Some(child) = node.child(index) {
            if nested_call(child, source) {
                return true;
            }
        }
    }
    false
}

fn contains_call(node: Node) -> bool {
    if node.kind() == "call_expression" {
        return true;
    }
    for index in 0..node.child_count() {
        if let Some(child) = node.child(index) {
            if contains_call(child) {
                return true;
            }
        }
    }
    false
}

fn forwarded_names(call: Node, source: &[u8], ordinary: &[String]) -> Vec<String> {
    let mut found = BTreeSet::new();
    if let Some(function) = call.child_by_field_name("function") {
        if function.kind() == "field_expression" {
            if let Some(object) = function.child_by_field_name("argument") {
                if let Some(name) = bare_name(object, source) {
                    found.insert(name);
                }
            }
        }
    }
    if let Some(arguments) = call.child_by_field_name("arguments") {
        for argument in code_nodes(arguments) {
            if let Some(name) = bare_name(argument, source) {
                found.insert(name);
            }
        }
    }
    ordinary
        .iter()
        .filter(|name| found.contains(*name))
        .cloned()
        .collect()
}

fn bare_name(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => Some(node_text(node, source)),
        "parenthesized_expression" => {
            let children = code_nodes(node);
            let [child] = children.as_slice() else {
                return None;
            };
            bare_name(*child, source)
        }
        "pointer_expression" if matches!(operator_text(node, source).as_str(), "&" | "*") => node
            .child_by_field_name("argument")
            .and_then(|argument| bare_name(argument, source)),
        "call_expression" if identity_named(node, source) => {
            node.child_by_field_name("arguments").and_then(|arguments| {
                let children = code_nodes(arguments);
                let [child] = children.as_slice() else {
                    return None;
                };
                bare_name(*child, source)
            })
        }
        _ => None,
    }
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
        "identifier" => Some(Peeled {
            root: Some(node_text(node, source)),
            hops: Vec::new(),
        }),
        "this" => Some(Peeled {
            root: Some("this".to_string()),
            hops: Vec::new(),
        }),
        "qualified_identifier" => Some(Peeled {
            root: None,
            hops: Vec::new(),
        }),
        "field_expression" => {
            let object = node.child_by_field_name("argument")?;
            let mut peeled = peel(object, source).unwrap_or(Peeled {
                root: None,
                hops: Vec::new(),
            });
            let operator = node
                .child_by_field_name("operator")
                .map(|operator| node_text(operator, source))
                .unwrap_or_default();
            if operator == "." || operator == "->" {
                let name = node.child_by_field_name("field")?;
                peeled.hops.push(Hop {
                    name: field_label(name, source),
                    call: false,
                    line: line_of(name),
                });
            }
            Some(peeled)
        }
        "template_function" | "template_method" => node
            .child_by_field_name("name")
            .and_then(|name| peel(name, source)),
        "call_expression" => {
            let function = node.child_by_field_name("function")?;
            if let Some(name) = bare_call_name(function, source) {
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
            if let Some(last) = peeled.hops.last_mut() {
                last.call = true;
            }
            Some(peeled)
        }
        "subscript_expression" => node
            .child_by_field_name("argument")
            .and_then(|argument| peel(argument, source)),
        "parenthesized_expression" => {
            let children = code_nodes(node);
            let [child] = children.as_slice() else {
                return None;
            };
            peel(*child, source)
        }
        _ => None,
    }
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
    node.child_by_field_name("argument")
        .or_else(|| code_nodes(node).into_iter().next())
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

fn code_nodes(node: Node<'_>) -> Vec<Node<'_>> {
    named_nodes(node)
        .into_iter()
        .filter(|child| child.kind() != "comment")
        .collect()
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
    node_text(node, source)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn line_of(node: Node) -> u32 {
    node.start_position().row as u32 + 1
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
    fn a_broken_file_still_returns_a_cpp_file() {
        let source = b"int broken( { return 1; }\n";
        let file = parse("broken.hpp", source);
        assert_eq!(file.language, Language::Cpp);
        assert!(file.unparsed_ratio > 0.0);
    }
}
