use std::collections::BTreeSet;

use tree_sitter::Node;

use crate::ir::{
    Body, Chain, Cognitive, CognitiveKind, Decision, File, Forward, Function, Language, Member,
    Param, Span, WaiverLine,
};

pub const ALLOW: &[&str] = &[
    "unwrap",
    "expect",
    "unwrap_or",
    "unwrap_or_else",
    "unwrap_or_default",
    "as_ref",
    "as_mut",
    "as_deref",
    "as_deref_mut",
    "ok",
    "err",
    "cloned",
    "copied",
    "iter",
    "iter_mut",
    "into_iter",
    "map",
    "filter",
    "filter_map",
    "flat_map",
    "flatten",
    "enumerate",
    "zip",
    "take",
    "skip",
    "chain",
    "collect",
    "rev",
    "inspect",
    "clone",
    "to_owned",
    "to_string",
    "to_vec",
    "as_str",
    "as_bytes",
    "into",
    "get",
    "get_mut",
    "first",
    "last",
    "front",
    "back",
    "entry",
    "keys",
    "values",
    "lock",
    "read",
    "write",
    "borrow",
    "borrow_mut",
];

const IDENTITY: &[&str] = &["clone", "to_owned", "into"];
const ASSERTS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
];

enum Scope {
    Module(String),
    Impl(String),
    Trait(String),
    Function(String),
}

pub fn parse(path: &str, source: &[u8]) -> File {
    parse_with(path, source, false)
}

pub fn parse_with(path: &str, source: &[u8], include_tests: bool) -> File {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("rust grammar");
    let Some(tree) = parser.parse(source, None) else {
        return File {
            path: path.to_string(),
            language: Language::Rust,
            unparsed_ratio: 1.0,
            unscored_macros: 0,
            functions: Vec::new(),
        };
    };
    let root = tree.root_node();
    let mut functions = Vec::new();
    let mut scopes = Vec::new();
    visit(root, source, &mut scopes, &mut functions, include_tests);
    File {
        path: path.to_string(),
        language: Language::Rust,
        unparsed_ratio: super::unparsed_ratio(root, source),
        unscored_macros: 0,
        functions,
    }
}

fn visit(
    node: Node,
    source: &[u8],
    scopes: &mut Vec<Scope>,
    out: &mut Vec<Function>,
    include_tests: bool,
) {
    if !include_tests && is_test_item(node, source) {
        return;
    }
    match node.kind() {
        "function_item" => visit_function(node, source, scopes, out, include_tests),
        "impl_item" => visit_scoped(
            node,
            source,
            scopes,
            out,
            impl_name(node, source),
            true,
            include_tests,
        ),
        "trait_item" => visit_scoped(
            node,
            source,
            scopes,
            out,
            trait_name(node, source),
            false,
            include_tests,
        ),
        "mod_item" => {
            let Some(name) = node.child_by_field_name("name") else {
                return;
            };
            let Some(body) = node.child_by_field_name("body") else {
                return;
            };
            scopes.push(Scope::Module(node_text(name, source)));
            visit(body, source, scopes, out, include_tests);
            scopes.pop();
        }
        "const_item" | "static_item" => visit_binding(node, source, scopes, out, include_tests),
        "macro_invocation"
        | "macro_definition"
        | "attribute_item"
        | "inner_attribute_item"
        | "line_comment"
        | "block_comment" => {}
        _ => visit_children(node, source, scopes, out, include_tests),
    }
}

fn visit_children(
    node: Node,
    source: &[u8],
    scopes: &mut Vec<Scope>,
    out: &mut Vec<Function>,
    include_tests: bool,
) {
    for child in named_nodes(node) {
        visit(child, source, scopes, out, include_tests);
    }
}

fn visit_scoped(
    node: Node,
    source: &[u8],
    scopes: &mut Vec<Scope>,
    out: &mut Vec<Function>,
    name: String,
    inherent: bool,
    include_tests: bool,
) {
    if name.is_empty() {
        return;
    }
    scopes.push(if inherent {
        Scope::Impl(name)
    } else {
        Scope::Trait(name)
    });
    if let Some(body) = node.child_by_field_name("body") {
        visit(body, source, scopes, out, include_tests);
    }
    scopes.pop();
}

fn visit_function(
    node: Node,
    source: &[u8],
    scopes: &mut Vec<Scope>,
    out: &mut Vec<Function>,
    include_tests: bool,
) {
    let Some(name_node) = node.child_by_field_name("name") else {
        return;
    };
    if node.child_by_field_name("body").is_none() {
        return;
    }
    let name = node_text(name_node, source);
    let params = node.child_by_field_name("parameters");
    push_function(out, scopes, source, &name, node, node, params);
    scopes.push(Scope::Function(name));
    if let Some(body) = node.child_by_field_name("body") {
        visit(body, source, scopes, out, include_tests);
    }
    scopes.pop();
}

fn visit_binding(
    node: Node,
    source: &[u8],
    scopes: &mut Vec<Scope>,
    out: &mut Vec<Function>,
    include_tests: bool,
) {
    if !in_function(scopes) {
        if let Some(closure) = bound_closure(node) {
            let Some(name_node) = node.child_by_field_name("name") else {
                return;
            };
            let name = node_text(name_node, source);
            let params = closure.child_by_field_name("parameters");
            push_function(out, scopes, source, &name, node, closure, params);
        }
    }
    if let Some(value) = node.child_by_field_name("value") {
        visit(value, source, scopes, out, include_tests);
    }
}

fn bound_closure(node: Node<'_>) -> Option<Node<'_>> {
    peel_closure(node.child_by_field_name("value")?)
}

fn peel_closure(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "closure_expression" => Some(node),
        "parenthesized_expression" | "block" => {
            let children = code_nodes(node);
            let [child] = children.as_slice() else {
                return None;
            };
            peel_closure(*child)
        }
        _ => None,
    }
}

fn push_function(
    out: &mut Vec<Function>,
    scopes: &[Scope],
    source: &[u8],
    name: &str,
    item: Node<'_>,
    body: Node<'_>,
    params: Option<Node<'_>>,
) {
    let span = span_of(item);
    let attributes = leading_attributes(item);
    let anchor = attributes.first().copied().unwrap_or(item);
    let waivers = waivers_above(anchor, source);
    let mut touch = span;
    if let Some(attribute) = attributes.first() {
        touch.start = span_of(*attribute).start;
    }
    if let Some(first) = waivers.first() {
        touch.start = first.line;
    }
    let params = if body.kind() == "closure_expression" {
        closure_params(params, source)
    } else {
        function_params(params, source)
    };
    let ordinary: Vec<String> = params
        .iter()
        .filter(|param| !param.receiver)
        .map(|param| param.name.clone())
        .collect();
    let receiver = params
        .iter()
        .find(|param| param.receiver)
        .map(|param| param.name.clone());
    let body = classify(body, source, receiver.as_deref(), &ordinary);
    out.push(Function {
        name: qualify(scopes, name),
        param_types: Vec::new(),
        qualifier: String::new(),
        span,
        touch,
        params,
        body,
        waivers,
    });
}

fn function_params(params: Option<Node>, source: &[u8]) -> Vec<Param> {
    params
        .map(|node| {
            named_nodes(node)
                .into_iter()
                .filter(|child| {
                    matches!(
                        child.kind(),
                        "parameter" | "self_parameter" | "variadic_parameter"
                    )
                })
                .map(|child| Param {
                    receiver: child.kind() == "self_parameter"
                        || binding_name(child, source) == "self",
                    name: binding_name(child, source),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn closure_params(params: Option<Node>, source: &[u8]) -> Vec<Param> {
    params
        .map(|node| {
            named_nodes(node)
                .into_iter()
                .map(|child| Param {
                    name: binding_name(child, source),
                    receiver: false,
                })
                .filter(|param| !param.name.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn binding_name(node: Node, source: &[u8]) -> String {
    match node.kind() {
        "identifier" | "self" => node_text(node, source),
        "self_parameter" => "self".to_string(),
        "parameter" => node
            .child_by_field_name("pattern")
            .map(|pattern| binding_name(pattern, source))
            .unwrap_or_else(|| "_".to_string()),
        "variadic_parameter" => node
            .child_by_field_name("pattern")
            .map(|pattern| binding_name(pattern, source))
            .unwrap_or_else(|| "...".to_string()),
        "mut_pattern" | "ref_pattern" | "reference_pattern" => named_nodes(node)
            .into_iter()
            .find(|child| child.kind() != "mutable_specifier")
            .map(|child| binding_name(child, source))
            .unwrap_or_else(|| flat_text(node, source)),
        _ => flat_text(node, source),
    }
}

fn is_test_item(node: Node, source: &[u8]) -> bool {
    matches!(
        node.kind(),
        "function_item" | "mod_item" | "impl_item" | "trait_item" | "const_item" | "static_item"
    ) && leading_attributes(node)
        .iter()
        .any(|attribute| is_test_attribute(*attribute, source))
}

fn is_test_attribute(node: Node, source: &[u8]) -> bool {
    let Some(attribute) = named_nodes(node)
        .into_iter()
        .find(|child| child.kind() == "attribute")
    else {
        return false;
    };
    let mut named = named_nodes(attribute);
    if named.is_empty() {
        return false;
    }
    let path = node_text(named.remove(0), source);
    if path == "test" {
        return attribute.child_by_field_name("arguments").is_none();
    }
    if path != "cfg" {
        return false;
    }
    let Some(arguments) = attribute.child_by_field_name("arguments") else {
        return false;
    };
    let arguments = named_nodes(arguments);
    arguments.len() == 1
        && arguments[0].kind() == "identifier"
        && node_text(arguments[0], source) == "test"
}

fn leading_attributes(node: Node<'_>) -> Vec<Node<'_>> {
    let mut found = Vec::new();
    let mut cursor = node.prev_sibling();
    while let Some(sibling) = cursor {
        match sibling.kind() {
            "attribute_item" => found.push(sibling),
            "line_comment" | "block_comment" => {}
            _ => break,
        }
        cursor = sibling.prev_sibling();
    }
    found.reverse();
    found
}

fn waivers_above(node: Node, source: &[u8]) -> Vec<WaiverLine> {
    let mut next_line = span_of(node).start;
    let mut found = Vec::new();
    consume_waiver_comments(node, source, &mut next_line, &mut found);
    found.reverse();
    found
}

fn consume_waiver_comments(
    node: Node,
    source: &[u8],
    next_line: &mut u32,
    found: &mut Vec<WaiverLine>,
) -> bool {
    let mut cursor = node.prev_sibling();
    while let Some(sibling) = cursor {
        if sibling.kind() != "line_comment" {
            return false;
        }
        let span = span_of(sibling);
        if span.start != span.end || span.start + 1 != *next_line || shares_code_line(sibling) {
            return false;
        }
        let Some(waiver) = waiver_from_comment(&node_text(sibling, source), span.start) else {
            return false;
        };
        found.push(waiver);
        *next_line = span.start;
        cursor = sibling.prev_sibling();
    }
    true
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

fn qualify(scopes: &[Scope], name: &str) -> String {
    let mut parts = Vec::new();
    for scope in scopes {
        let scope_name = match scope {
            Scope::Module(scope_name)
            | Scope::Impl(scope_name)
            | Scope::Trait(scope_name)
            | Scope::Function(scope_name) => scope_name.as_str(),
        };
        parts.push(scope_name);
    }
    parts.push(name);
    parts.join("::")
}

fn in_function(scopes: &[Scope]) -> bool {
    scopes
        .iter()
        .any(|scope| matches!(scope, Scope::Function(_)))
}

fn impl_name(node: Node, source: &[u8]) -> String {
    let Some(ty) = node.child_by_field_name("type") else {
        return String::new();
    };
    let ty = flat_text(ty, source);
    match node.child_by_field_name("trait") {
        Some(trait_node) => format!("<{ty} as {}>", flat_text(trait_node, source)),
        None => ty,
    }
}

fn trait_name(node: Node, source: &[u8]) -> String {
    let Some(name) = node.child_by_field_name("name") else {
        return String::new();
    };
    let params = node
        .child_by_field_name("type_parameters")
        .map(|params| flat_text(params, source))
        .unwrap_or_default();
    format!("{}{params}", node_text(name, source))
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
        "function_item" => {
            if let Some(body) = node.child_by_field_name("body") {
                classifier.walk_block(body, 0);
            }
        }
        "closure_expression" => classifier.walk_closure_body(node, 0, true),
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
    skip: Option<std::ops::Range<usize>>,
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

    fn walk_block(&mut self, node: Node<'a>, nesting: u32) {
        for child in code_nodes(node) {
            if is_statement(child.kind()) {
                self.walk_statement(child, nesting);
            } else {
                self.note_expression(child, nesting);
            }
        }
    }

    fn walk_statement(&mut self, node: Node<'a>, nesting: u32) {
        match node.kind() {
            "expression_statement" => {
                let children = code_nodes(node);
                match children.as_slice() {
                    [child] => self.note_expression(*child, nesting),
                    _ => {
                        self.lloc += 1;
                        for child in children {
                            self.walk(child, nesting);
                        }
                    }
                }
            }
            "let_declaration" => self.walk_let(node, nesting),
            "empty_statement" | "function_item" | "attribute_item" | "inner_attribute_item" => {}
            "macro_invocation" => {
                self.lloc += 1;
                self.walk_macro(node, nesting);
            }
            "const_item" | "static_item" => {
                self.lloc += 1;
                if let Some(value) = node.child_by_field_name("value") {
                    self.walk(value, nesting);
                }
            }
            _ => self.lloc += 1,
        }
    }

    fn note_expression(&mut self, node: Node<'a>, nesting: u32) {
        if node.kind() == "if_expression" {
            self.lloc += 1;
            self.walk_if(node, nesting, false);
        } else if node.kind() == "match_expression" {
            self.lloc += 1;
            self.walk_match(node, nesting);
        } else if node.kind() == "closure_expression" {
            self.walk_closure_body(node, nesting + 1, true);
        } else if is_control(node.kind()) || is_transparent(node.kind()) {
            self.walk(node, nesting);
        } else {
            self.lloc += 1;
            self.walk(node, nesting);
        }
    }

    fn walk(&mut self, node: Node<'a>, nesting: u32) {
        match node.kind() {
            "if_expression" => self.walk_if(node, nesting, false),
            "while_expression" => self.walk_while(node, nesting),
            "loop_expression" => self.walk_loop(node, nesting),
            "for_expression" => self.walk_for(node, nesting),
            "match_expression" => self.walk_match(node, nesting),
            "let_declaration" | "expression_statement" => self.walk_statement(node, nesting),
            "block" | "unsafe_block" | "async_block" | "gen_block" | "const_block"
            | "try_block" => {
                if let Some(inner) = transparent_block(node) {
                    self.walk_block(inner, nesting);
                }
            }
            "closure_expression" => self.walk_closure(node, nesting),
            "binary_expression" => {
                if bool_op(node, self.source).is_some() {
                    self.walk_boolean(node, nesting);
                } else {
                    self.walk_named(node, nesting);
                }
            }
            "let_chain" => self.walk_let_chain(node, nesting),
            "let_condition" => self.walk_let_condition(node, nesting),
            "try_expression" => {
                self.decision(line_of(node), nesting);
                self.record_chain(node);
                self.walk_named(node, nesting);
            }
            "break_expression" | "continue_expression" => {
                if node.child_by_field_name("label").is_some()
                    || named_nodes(node)
                        .iter()
                        .any(|child| child.kind() == "label")
                {
                    self.fundamental(line_of(node));
                }
                self.walk_named(node, nesting);
            }
            "field_expression"
            | "call_expression"
            | "generic_function"
            | "index_expression"
            | "parenthesized_expression"
            | "await_expression" => {
                self.record_chain(node);
                self.walk_named(node, nesting);
            }
            "macro_invocation" => self.walk_macro(node, nesting),
            _ => self.walk_named(node, nesting),
        }
    }

    fn walk_named(&mut self, node: Node<'a>, nesting: u32) {
        for child in named_nodes(node) {
            self.walk(child, nesting);
        }
    }

    fn walk_if(&mut self, node: Node<'a>, nesting: u32, else_if: bool) {
        let line = line_of(node);
        if else_if {
            self.hybrid(line);
        } else {
            self.structural(line, nesting);
        }
        self.decision(line, nesting);
        if let Some(condition) = node.child_by_field_name("condition") {
            self.walk_condition(condition, nesting);
        }
        if let Some(body) = node.child_by_field_name("consequence") {
            self.walk_block(body, nesting + 1);
        }
        if let Some(alternative) = node.child_by_field_name("alternative") {
            let inner = code_nodes(alternative);
            if let Some(child) = inner.first() {
                if child.kind() == "if_expression" {
                    self.walk_if(*child, nesting, true);
                } else {
                    self.hybrid(line_of(alternative));
                    self.walk_block(*child, nesting + 1);
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
            self.walk_block(body, nesting + 1);
        }
    }

    fn walk_loop(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.decision(line_of(node), nesting);
        self.structural(line_of(node), nesting);
        if let Some(body) = node.child_by_field_name("body") {
            self.walk_block(body, nesting + 1);
        }
    }

    fn walk_for(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.decision(line_of(node), nesting);
        self.structural(line_of(node), nesting);
        if let Some(value) = node.child_by_field_name("value") {
            self.walk(value, nesting);
        }
        if let Some(body) = node.child_by_field_name("body") {
            self.walk_block(body, nesting + 1);
        }
    }

    fn walk_match(&mut self, node: Node<'a>, nesting: u32) {
        self.structural(line_of(node), nesting);
        if let Some(value) = node.child_by_field_name("value") {
            self.walk(value, nesting);
        }
        if let Some(body) = node.child_by_field_name("body") {
            for arm in named_nodes(body) {
                if arm.kind() == "match_arm" {
                    self.walk_arm(arm, nesting);
                }
            }
        }
    }

    fn walk_arm(&mut self, node: Node<'a>, nesting: u32) {
        let line = line_of(node);
        let labels = node
            .child_by_field_name("pattern")
            .map(|pattern| count_specified(pattern, self.source))
            .unwrap_or(0);
        for _ in 0..labels {
            self.decision(line, nesting);
        }
        if let Some(pattern) = node.child_by_field_name("pattern") {
            if let Some(guard) = pattern.child_by_field_name("condition") {
                self.decision(line_of(guard), nesting);
                self.walk_condition(guard, nesting);
            }
        }
        if let Some(value) = node.child_by_field_name("value") {
            if value.kind() == "block" || is_transparent(value.kind()) {
                self.walk(value, nesting + 1);
            } else {
                self.note_expression(value, nesting + 1);
            }
        }
    }

    fn walk_let(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        if let Some(value) = node.child_by_field_name("value") {
            self.walk(value, nesting);
        }
        if let Some(alternative) = node.child_by_field_name("alternative") {
            let line = token_line(node, self.source, "else");
            self.decision(line, nesting);
            self.hybrid(line);
            self.walk_block(alternative, nesting + 1);
        }
    }

    fn walk_condition(&mut self, node: Node<'a>, nesting: u32) {
        match node.kind() {
            "let_condition" => self.walk_let_condition(node, nesting),
            "let_chain" => self.walk_let_chain(node, nesting),
            _ => self.walk(node, nesting),
        }
    }

    fn walk_let_condition(&mut self, node: Node<'a>, nesting: u32) {
        if let Some(value) = node.child_by_field_name("value") {
            self.walk(value, nesting);
        }
    }

    fn walk_let_chain(&mut self, node: Node<'a>, nesting: u32) {
        let mut lets = 0u32;
        let mut last = None;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if !child.is_named() && node_text(child, self.source) == "&&" {
                self.decision(line_of(child), nesting);
                if last != Some(BoolOp::And) {
                    self.fundamental(line_of(child));
                    last = Some(BoolOp::And);
                }
            } else if child.kind() == "let_condition" {
                lets += 1;
                if lets > 1 {
                    self.decision(line_of(child), nesting);
                }
                self.walk_let_condition(child, nesting);
            } else if child.kind() == "let_chain" {
                self.walk_let_chain(child, nesting);
            } else if child.is_named() {
                self.bool_spine(child, nesting, &mut last);
            }
        }
    }

    fn walk_closure(&mut self, node: Node<'a>, nesting: u32) {
        self.walk_closure_body(node, nesting + 1, false);
    }

    fn walk_closure_body(&mut self, node: Node<'a>, nesting: u32, count_expression: bool) {
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        if body.kind() == "block" {
            self.walk_block(body, nesting);
        } else if count_expression {
            self.lloc += 1;
            self.walk(body, nesting);
        } else {
            self.walk(body, nesting);
        }
    }

    fn walk_boolean(&mut self, node: Node<'a>, nesting: u32) {
        let mut last = None;
        self.bool_spine(node, nesting, &mut last);
    }

    fn bool_spine(&mut self, node: Node<'a>, nesting: u32, last: &mut Option<BoolOp>) {
        if node.kind() == "unary_expression" && operator_text(node, self.source) == "!" {
            if let Some(inner) = named_nodes(node).into_iter().next() {
                self.bool_spine(inner, nesting, last);
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
        if node.kind() == "let_chain" {
            self.walk_let_chain(node, nesting);
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

    fn walk_macro(&mut self, node: Node<'a>, nesting: u32) {
        if ASSERTS.contains(&macro_name(node, self.source).as_str()) {
            self.decision(line_of(node), nesting);
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
        "function_item" => {
            let body = node.child_by_field_name("body")?;
            let children = code_nodes(body);
            let [statement] = children.as_slice() else {
                return None;
            };
            Some((line_of(*statement), unwrap_call(*statement)?))
        }
        "closure_expression" => {
            let body = node.child_by_field_name("body")?;
            if body.kind() == "block" {
                let children = code_nodes(body);
                let [statement] = children.as_slice() else {
                    return None;
                };
                Some((line_of(*statement), unwrap_call(*statement)?))
            } else {
                Some((line_of(body), unwrap_call(body)?))
            }
        }
        _ => None,
    }
}

fn unwrap_call(node: Node) -> Option<Node> {
    match node.kind() {
        "call_expression" => Some(node),
        "expression_statement"
        | "return_expression"
        | "parenthesized_expression"
        | "await_expression"
        | "try_expression" => {
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
        return wrapper_receiver(node).is_some_and(contains_call);
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

fn wrapper_receiver(node: Node) -> Option<Node> {
    node.child_by_field_name("function")?
        .child_by_field_name("value")
}

fn forwarded_names(call: Node, source: &[u8], ordinary: &[String]) -> Vec<String> {
    let mut found = BTreeSet::new();
    if let Some(function) = call.child_by_field_name("function") {
        if function.kind() == "field_expression" {
            if let Some(object) = function.child_by_field_name("value") {
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
        "reference_expression" => node
            .child_by_field_name("value")
            .and_then(|value| bare_name(value, source)),
        "unary_expression" if operator_text(node, source) == "*" => code_nodes(node)
            .first()
            .and_then(|child| bare_name(*child, source)),
        "call_expression" if identity_named(node, source) => {
            wrapper_receiver(node).and_then(|receiver| bare_name(receiver, source))
        }
        _ => None,
    }
}

fn identity_named(node: Node, source: &[u8]) -> bool {
    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    if function.kind() != "field_expression" {
        return false;
    }
    let Some(field) = function.child_by_field_name("field") else {
        return false;
    };
    if !IDENTITY.contains(&node_text(field, source).as_str()) {
        return false;
    }
    node.child_by_field_name("arguments")
        .is_none_or(|arguments| code_nodes(arguments).is_empty())
}

fn count_specified(node: Node, source: &[u8]) -> u32 {
    match node.kind() {
        "match_pattern" => {
            let patterns: Vec<Node> = named_nodes(node)
                .into_iter()
                .filter(|child| node.child_by_field_name("condition") != Some(*child))
                .collect();
            if patterns.is_empty() {
                0
            } else {
                patterns
                    .into_iter()
                    .map(|pattern| count_specified(pattern, source))
                    .sum()
            }
        }
        "or_pattern" => named_nodes(node)
            .into_iter()
            .map(|pattern| count_specified(pattern, source))
            .sum(),
        "identifier" if node_text(node, source) == "_" => 0,
        _ => 1,
    }
}

fn macro_name(node: Node, source: &[u8]) -> String {
    let Some(name) = node.child_by_field_name("macro") else {
        return String::new();
    };
    match name.kind() {
        "scoped_identifier" => name
            .child_by_field_name("name")
            .map(|name| node_text(name, source))
            .unwrap_or_default(),
        _ => node_text(name, source),
    }
}

fn peel(node: Node, source: &[u8]) -> Option<Peeled> {
    match node.kind() {
        "identifier" | "self" => Some(Peeled {
            root: Some(node_text(node, source)),
            hops: Vec::new(),
        }),
        "scoped_identifier" | "scoped_type_identifier_in_expression_position" => Some(Peeled {
            root: None,
            hops: Vec::new(),
        }),
        "field_expression" => {
            let object = node.child_by_field_name("value")?;
            let mut peeled = peel(object, source).unwrap_or(Peeled {
                root: None,
                hops: Vec::new(),
            });
            let name = node.child_by_field_name("field")?;
            peeled.hops.push(Hop {
                name: node_text(name, source),
                call: false,
                line: line_of(name),
            });
            Some(peeled)
        }
        "generic_function" => node
            .child_by_field_name("function")
            .and_then(|function| peel(function, source)),
        "call_expression" => {
            let function = node.child_by_field_name("function")?;
            if let Some(name) = bare_call_name(function, source) {
                if bare_call_roots_a_chain(node) {
                    return Some(Peeled {
                        root: None,
                        hops: Vec::new(),
                    });
                }
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
        "index_expression" => code_nodes(node)
            .first()
            .and_then(|value| peel(*value, source)),
        "parenthesized_expression" | "await_expression" | "try_expression" => {
            let children = code_nodes(node);
            let [child] = children.as_slice() else {
                return None;
            };
            peel(*child, source)
        }
        _ => None,
    }
}

fn bare_call_name(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => Some(node_text(node, source)),
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

fn shares_code_line(comment: Node) -> bool {
    let line = comment.start_position().row;
    let mut cursor = comment.prev_sibling();
    while let Some(sibling) = cursor {
        if sibling.kind() == "line_comment" {
            if occupied_end(sibling) < line {
                return false;
            }
            cursor = sibling.prev_sibling();
            continue;
        }
        return sibling.start_position().row <= line && occupied_end(sibling) >= line;
    }
    false
}

fn occupied_end(node: Node) -> usize {
    let end = node.end_position();
    if end.column == 0 && end.row > node.start_position().row {
        end.row - 1
    } else {
        end.row
    }
}

fn bare_call_roots_a_chain(node: Node) -> bool {
    let mut current = node;
    loop {
        let Some(parent) = current.parent() else {
            return false;
        };
        match parent.kind() {
            "field_expression" => {
                return parent.child_by_field_name("value") == Some(current);
            }
            "index_expression" => {
                if code_nodes(parent).first() != Some(&current) {
                    return false;
                }
                current = parent;
            }
            "parenthesized_expression" | "await_expression" | "try_expression" => {
                let children = code_nodes(parent);
                if children.len() != 1 || children.first() != Some(&current) {
                    return false;
                }
                current = parent;
            }
            "call_expression" | "generic_function" => {
                if parent.child_by_field_name("function") != Some(current) {
                    return false;
                }
                current = parent;
            }
            _ => return false,
        }
    }
}

fn chain_continues(node: Node) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    match parent.kind() {
        "field_expression" => parent.child_by_field_name("value") == Some(node),
        "call_expression" | "generic_function" => {
            parent.child_by_field_name("function") == Some(node)
        }
        "index_expression" => code_nodes(parent).first() == Some(&node),
        "parenthesized_expression" | "await_expression" | "try_expression" => {
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
    match node
        .child_by_field_name("operator")
        .map(|operator| node_text(operator, source))
        .as_deref()
    {
        Some("&&") => Some(BoolOp::And),
        Some("||") => Some(BoolOp::Or),
        _ => None,
    }
}

fn is_statement(kind: &str) -> bool {
    matches!(
        kind,
        "expression_statement"
            | "empty_statement"
            | "let_declaration"
            | "const_item"
            | "static_item"
            | "function_item"
            | "function_signature_item"
            | "macro_invocation"
            | "macro_definition"
            | "use_declaration"
            | "type_item"
            | "struct_item"
            | "enum_item"
            | "union_item"
            | "impl_item"
            | "trait_item"
            | "mod_item"
            | "foreign_mod_item"
            | "extern_crate_declaration"
            | "associated_type"
            | "attribute_item"
            | "inner_attribute_item"
    )
}

fn is_control(kind: &str) -> bool {
    matches!(
        kind,
        "if_expression"
            | "while_expression"
            | "loop_expression"
            | "for_expression"
            | "match_expression"
    )
}

fn is_transparent(kind: &str) -> bool {
    matches!(
        kind,
        "block" | "unsafe_block" | "async_block" | "gen_block" | "const_block" | "try_block"
    )
}

fn transparent_block(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "block" {
        Some(node)
    } else {
        named_nodes(node)
            .into_iter()
            .find(|child| child.kind() == "block")
    }
}

fn code_nodes(node: Node<'_>) -> Vec<Node<'_>> {
    named_nodes(node)
        .into_iter()
        .filter(|child| !matches!(child.kind(), "line_comment" | "block_comment"))
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
        return node.utf8_text(b"").unwrap_or("").to_string();
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

fn token_line(node: Node, source: &[u8], token: &str) -> u32 {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !child.is_named() && node_text(child, source) == token {
            return line_of(child);
        }
    }
    line_of(node)
}

fn operator_text(node: Node, source: &[u8]) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !child.is_named() {
            return node_text(child, source);
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{CognitiveKind, Decision};

    #[test]
    fn the_allow_list_is_the_rust_member_list() {
        assert!(ALLOW.contains(&"unwrap"));
        assert!(ALLOW.contains(&"clone"));
        assert!(ALLOW.contains(&"borrow_mut"));
        assert_eq!(ALLOW.len(), 50);
    }

    #[test]
    fn methods_closures_and_waivers_follow_the_rust_names() {
        let source = "\
mod demo {
    impl Take {
        #[inline]
        fn into_inner(self, n: i32) -> i32 {
            n
        }
    }

    impl<T> Buf for Take<T> {
        fn chunks_vectored(&mut self, a: i32) {}
    }

    trait Buf<T> {
        fn chunks_vectored(&self, a: i32) {}
        fn required(&self);
    }

    fn outer(a: bool) -> i32 {
        fn inner(x: i32) -> i32 {
            if x > 0 { x } else { 0 }
        }
        let add = |y| y + 1;
        if a { inner(add(1)) } else { 0 }
    }
}

// slopmeter: waive F01 9 measured on purpose
const ADD: fn(i32) -> i32 = |x| x + 1;
";
        let file = parse("src/lib.rs", source.as_bytes());
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
                "demo::<Take<T> as Buf>::chunks_vectored",
                "demo::Buf<T>::chunks_vectored",
                "demo::outer",
                "demo::outer::inner",
                "ADD",
            ]
        );

        let method = function(&file, "demo::Take::into_inner");
        assert_eq!(
            method
                .params
                .iter()
                .map(|param| (param.name.as_str(), param.receiver))
                .collect::<Vec<_>>(),
            vec![("self", true), ("n", false)]
        );
        assert_eq!(method.span.start, 4);
        assert_eq!(method.touch.start, 3);

        let added = function(&file, "ADD");
        assert_eq!(added.params.len(), 1);
        assert_eq!(added.params[0].name, "x");
        assert!(!added.params[0].receiver);
        assert_eq!(
            added.waivers,
            vec![waiver(
                27,
                Some("F01"),
                Some("9"),
                Some("measured on purpose")
            )]
        );
        assert_eq!(added.touch.start, 27);
        assert!(added.span.start > added.touch.start);
    }

    #[test]
    fn a_block_comment_ends_the_waiver_run() {
        let source = "\
// slopmeter: waive F01 9 measured on purpose
/* note */
fn kept() {}
";
        let file = parse("src/lib.rs", source.as_bytes());
        assert!(function(&file, "kept").waivers.is_empty());
    }

    #[test]
    fn cognitive_nesting_follows_the_white_paper() {
        let source = "\
fn nested(a: bool, b: bool, c: bool) -> i32 {
    if a {
        if b {
            return 1;
        } else if c {
            return 2;
        }
        0
    } else {
        0
    }
}

fn other(a: bool, b: bool) -> i32 {
    if a {
        return 1;
    } else {
        if b {
            return 2;
        }
        0
    }
}
";
        let file = parse("a.rs", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0);
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
        assert_eq!(
            other.body.decisions,
            vec![
                Decision { line: 15, depth: 1 },
                Decision { line: 18, depth: 2 },
            ]
        );
        assert_eq!(cognitive_total(&other.body), 4);
        assert!(other
            .body
            .cognitive
            .iter()
            .any(|item| { item.kind == CognitiveKind::Hybrid && item.line == 17 }));
    }

    #[test]
    fn boolean_runs_split_on_a_changed_operator_and_on_parentheses() {
        let source = "\
fn same(a: bool, b: bool, c: bool) -> bool { a && b && c }
fn changed(a: bool, b: bool, c: bool) -> bool { a && b || c }
fn grouped(a: bool, b: bool, c: bool) -> bool { (a && b) && c }
fn negated(a: bool, b: bool) -> bool { !a && !b }
fn resumed(a: bool, b: bool, c: bool, d: bool) -> bool { a && (b || c) && d }
fn bits(a: i32, b: i32, c: i32) -> i32 { a & b | c }
";
        let file = parse("a.rs", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0);
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
        assert!(function(&file, "bits").body.decisions.is_empty());
    }

    #[test]
    fn decisions_follow_the_cyclomatic_rules() {
        let source = "\
fn paths(xs: &[i32]) -> i32 {
    for x in xs {
        while *x > 0 {
            return *x;
        }
    }
    loop {
        break;
    }
    let Some(first) = xs.first() else {
        return 0;
    };
    assert!(*first > 0);
    ready()?;
    *first
}

fn question(value: Option<i32>) -> Result<i32, ()> {
    let found = value?;
    Ok(found)
}
";
        let file = parse("a.rs", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{source}");
        let paths = function(&file, "paths");
        assert_eq!(paths.body.decisions.len(), 6);
        assert!(paths
            .body
            .decisions
            .iter()
            .any(|decision| decision.line == 10));
        assert_eq!(
            paths
                .body
                .cognitive
                .iter()
                .filter(|item| item.kind == CognitiveKind::Structural)
                .count(),
            3
        );
        assert!(!function(&file, "question").body.decisions.is_empty());
        assert_eq!(cognitive_total(&function(&file, "question").body), 0);
    }

    #[test]
    fn a_match_counts_specified_labels_and_one_structural_increment() {
        let source = "\
fn classify(x: i32) -> i32 {
    match x {
        1 | 2 => 1,
        _ => 2,
        3 if x > 0 => 3,
    }
}
";
        let file = parse("a.rs", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0);
        let classify = function(&file, "classify");
        assert_eq!(classify.body.decisions.len(), 4);
        assert_eq!(cognitive_total(&classify.body), 1);
        assert_eq!(classify.body.cognitive[0].kind, CognitiveKind::Structural);
    }

    #[test]
    fn a_closure_folds_in_and_an_inner_fn_does_not() {
        let source = "\
fn outer(a: bool) -> i32 {
    fn inner(x: bool) -> i32 {
        if x { 1 } else { 0 }
    }
    let pick = |y| if y { 1 } else { 0 };
    if a { pick(true) } else { inner(false) }
}
";
        let file = parse("a.rs", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0);
        let names: Vec<_> = file
            .functions
            .iter()
            .map(|function| function.name.as_str())
            .collect();
        assert_eq!(names, vec!["outer", "outer::inner"]);
        let outer = function(&file, "outer");
        assert!(outer
            .body
            .decisions
            .iter()
            .any(|decision| decision.depth >= 2));
        assert!(outer.body.lloc >= 3);
        let inner = function(&file, "outer::inner");
        assert_eq!(inner.body.decisions.len(), 1);
        assert_eq!(cognitive_total(&inner.body), 2);
    }

    #[test]
    fn an_expression_position_if_or_match_is_not_another_statement() {
        let source = "\
fn bound(a: bool) -> i32 {
    let x = if a { 1 } else { 2 };
    x
}

fn returned(a: bool) -> i32 {
    return if a { 1 } else { 2 };
}

fn tail(a: bool) -> i32 {
    if a { 1 } else { 2 }
}

fn chained(a: bool, b: bool) -> i32 {
    if a {
        1
    } else if b {
        2
    } else {
        3
    }
}

fn matched(x: i32) -> i32 {
    let y = match x {
        1 => 1,
        _ => 2,
    };
    y
}

fn stated(a: bool) {
    if a {
        ready();
        go();
    }
}

fn closed(a: bool) -> i32 {
    let pick = |y| if y { 1 } else { 0 };
    if a { pick(true) } else { 0 }
}

fn expr_closure() -> i32 {
    let pick = |y: i32| y + 1;
    pick(1)
}

fn closure_tail() -> impl Fn(i32) -> i32 {
    |y| y + 1
}

fn block_closure() {
    let pick = |_y| {
        ready();
        go();
    };
    pick(1);
}
";
        let file = parse("a.rs", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0);
        assert_eq!(function(&file, "bound").body.lloc, 4);
        assert_eq!(function(&file, "returned").body.lloc, 3);
        assert_eq!(function(&file, "tail").body.lloc, 3);
        assert_eq!(function(&file, "chained").body.lloc, 4);
        assert_eq!(function(&file, "matched").body.lloc, 4);
        assert_eq!(function(&file, "stated").body.lloc, 3);
        assert_eq!(function(&file, "closed").body.lloc, 6);
        assert_eq!(function(&file, "expr_closure").body.lloc, 2);
        assert_eq!(function(&file, "closure_tail").body.lloc, 1);
        assert_eq!(function(&file, "block_closure").body.lloc, 4);
    }

    #[test]
    fn logical_lines_count_a_tail_and_skip_an_inner_fn() {
        let source = "\
fn empty() {}

fn tail() -> i32 {
    1
}

fn parent() -> fn() -> i32 {
    fn child() -> i32 {
        1
    }
    child
}
";
        let file = parse("a.rs", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0);
        assert_eq!(function(&file, "empty").body.lloc, 0);
        assert_eq!(function(&file, "tail").body.lloc, 1);
        assert_eq!(function(&file, "parent").body.lloc, 1);
        assert_eq!(function(&file, "parent::child").body.lloc, 1);
    }

    #[test]
    fn member_chains_keep_every_hop_and_a_call_can_forward() {
        let source = "\
fn run(self_name: &Repo, dst: &str) -> usize {
    self_name.repo.save(dst);
    dst.len().min(1)
}

impl Repo {
    fn kept(&self) -> usize {
        self.get().copy()
    }

    fn only(&self, a: &str) -> usize {
        a.b[0].c()
    }
}

fn get(url: &str, params: i32) -> i32 {
    request(url, params)
}

fn cloned(value: &String) -> String {
    value.clone()
}

fn wrapped(value: String) -> String {
    send(value.clone())
}

fn doubled(value: String) -> String {
    send(value.clone().into())
}
";
        let file = parse("a.rs", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{source}");
        let run = function(&file, "run");
        assert!(run.body.forward.is_none());
        assert!(run
            .body
            .chains
            .iter()
            .any(|chain| { !chain.rooted && member_names(chain) == ["len", "min"] }));

        let kept = function(&file, "Repo::kept");
        assert_eq!(member_names(&kept.body.chains[0]), vec!["get", "copy"]);
        assert!(kept.body.chains[0].rooted);

        let only = function(&file, "Repo::only");
        assert_eq!(member_names(&only.body.chains[0]), vec!["b", "c"]);
        assert!(!only.body.chains[0].rooted);
        assert!(!only.body.chains[0].members[0].call);
        assert!(only.body.chains[0].members[1].call);

        let get = function(&file, "get");
        assert!(get.body.chains.is_empty());
        assert_eq!(
            get.body.forward.as_ref().unwrap().names,
            vec!["url".to_string(), "params".to_string()]
        );
        assert_eq!(
            function(&file, "cloned")
                .body
                .forward
                .as_ref()
                .unwrap()
                .names,
            vec!["value".to_string()]
        );
        assert_eq!(
            function(&file, "wrapped")
                .body
                .forward
                .as_ref()
                .unwrap()
                .names,
            vec!["value".to_string()]
        );
        assert!(function(&file, "doubled").body.forward.is_none());
    }

    #[test]
    fn a_broken_file_still_returns_a_record() {
        let file = parse("a.rs", b"fn (\n");
        assert!(file.unparsed_ratio > 0.0);
        assert_eq!(file.language, Language::Rust);
    }

    #[test]
    fn a_broken_line_does_not_mark_the_functions_after_it() {
        let source = "\
fn ok() -> i32 {
    let x = (
}
fn also() -> i32 { 1 }
fn more() -> i32 { 2 }
";
        let file = parse("wrap.rs", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.2);
        assert_eq!(file.functions.len(), 3);
    }

    #[test]
    fn a_cfg_test_module_and_a_test_function_are_not_scored() {
        let source = b"\
fn kept() {}

#[test]
fn eleven() {
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
    assert!(true);
}

#[cfg(test)]
fn hidden() {}

#[cfg(test)]
mod tests {
    fn helper() {}

    #[test]
    fn checked() {}
}

#[cfg(not(test))]
fn shipped() {}
";
        let file = parse("src/lib.rs", source);
        let names: Vec<_> = file
            .functions
            .iter()
            .map(|function| function.name.as_str())
            .collect();
        assert_eq!(names, vec!["kept", "shipped"]);

        let file = parse_with("src/lib.rs", source, true);
        let names: Vec<_> = file
            .functions
            .iter()
            .map(|function| function.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec![
                "kept",
                "eleven",
                "hidden",
                "tests::helper",
                "tests::checked",
                "shipped",
            ]
        );
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

    fn function<'a>(file: &'a File, name: &str) -> &'a Function {
        file.functions
            .iter()
            .find(|function| function.name == name)
            .unwrap_or_else(|| panic!("missing {name}"))
    }

    fn cognitive_total(body: &Body) -> u32 {
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
fn call_result() -> i32 {
    make().a.b.c
}

fn wrapped() -> i32 {
    (make()).a.b
}

fn method_call(obj: Obj) -> i32 {
    obj.method().extra
}

fn calls() {
    send();
}
";
        let file = parse("a.rs", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{source}");
        assert_eq!(
            member_names(&function(&file, "call_result").body.chains[0]),
            vec!["a", "b", "c"]
        );
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
        assert_eq!(
            member_names(&function(&file, "calls").body.chains[0]),
            vec!["send"]
        );
    }

    #[test]
    fn a_trailing_comment_on_a_code_line_is_not_a_waiver() {
        let source = "\
fn earlier() -> i32 { 1 } // slopmeter: waive F01 9 measured on purpose
fn later() -> i32 { 2 }

// slopmeter: waive F01 9 measured on purpose
fn whole_line() -> i32 { 3 }
";
        let file = parse("a.rs", source.as_bytes());
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

    fn member_names(chain: &Chain) -> Vec<&str> {
        chain
            .members
            .iter()
            .map(|member| member.name.as_str())
            .collect()
    }
}
