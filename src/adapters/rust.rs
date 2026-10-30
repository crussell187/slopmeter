use std::collections::BTreeSet;

use tree_sitter::Node;

use crate::ir::{
    Body, CallSite, Chain, Cognitive, CognitiveKind, Decision, File, Forward, Function, Language,
    Member, Param, Span, WaiverLine,
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
const NOISE: &[&str] = &[
    "log", "logger", "debug", "info", "warn", "warning", "error", "trace", "print", "println",
    "eprintln", "dbg",
];
const RESULT: &[&str] = &["Ok", "Some", "Err"];
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
    let mut visit = Visit {
        source,
        scopes: Vec::new(),
        out: Vec::new(),
        include_tests,
    };
    visit.visit(root);
    File {
        path: path.to_string(),
        language: Language::Rust,
        unparsed_ratio: super::unparsed_ratio(root, source),
        unscored_macros: 0,
        functions: visit.out,
    }
}

struct Visit<'a> {
    source: &'a [u8],
    scopes: Vec<Scope>,
    out: Vec<Function>,
    include_tests: bool,
}

struct FnSite<'a> {
    item: Node<'a>,
    body: Node<'a>,
    params: Option<Node<'a>>,
}

struct Names<'a> {
    receiver: Option<&'a str>,
    ordinary: &'a [String],
}

impl<'a> Visit<'a> {
    fn visit(&mut self, node: Node<'a>) {
        if self.skips_tests(node) {
            return;
        }
        if self.visit_defined(node) {
            return;
        }
        self.visit_rest(node);
    }

    fn skips_tests(&self, node: Node<'a>) -> bool {
        !self.include_tests && is_test_item(node, self.source)
    }

    fn visit_defined(&mut self, node: Node<'a>) -> bool {
        match node.kind() {
            "function_item" => self.visit_function(node),
            "impl_item" => self.visit_impl(node),
            "trait_item" => self.visit_trait(node),
            "mod_item" => self.visit_module(node),
            "const_item" | "static_item" => self.visit_binding(node),
            _ => return false,
        }
        true
    }

    fn visit_rest(&mut self, node: Node<'a>) {
        match node.kind() {
            "macro_invocation" => self.visit_macro_invocation(node),
            "macro_definition" => self.visit_macro_definition(node),
            "attribute_item" | "inner_attribute_item" | "line_comment" | "block_comment" => {}
            _ => self.visit_children(node),
        }
    }

    fn visit_impl(&mut self, node: Node<'a>) {
        self.visit_scoped(node, impl_name(node, self.source), true);
    }

    fn visit_trait(&mut self, node: Node<'a>) {
        self.visit_scoped(node, trait_name(node, self.source), false);
    }

    fn visit_module(&mut self, node: Node<'a>) {
        let Some(name) = node.child_by_field_name("name") else {
            return;
        };
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        let module = node_text(name, self.source);
        self.scopes.push(Scope::Module(module));
        self.visit(body);
        self.scopes.pop();
    }

    fn visit_macro_invocation(&mut self, node: Node<'a>) {
        if in_function(&self.scopes) {
            self.push_token_fns(node);
        }
    }

    fn visit_children(&mut self, node: Node<'a>) {
        for child in named_nodes(node) {
            self.visit(child);
        }
    }

    fn visit_scoped(&mut self, node: Node<'a>, name: String, inherent: bool) {
        if name.is_empty() {
            return;
        }
        self.scopes.push(if inherent {
            Scope::Impl(name)
        } else {
            Scope::Trait(name)
        });
        if let Some(body) = node.child_by_field_name("body") {
            self.visit(body);
        }
        self.scopes.pop();
    }

    fn visit_function(&mut self, node: Node<'a>) {
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        let name = node_text(name_node, self.source);
        let params = node.child_by_field_name("parameters");
        self.push_function(
            &name,
            FnSite {
                item: node,
                body: node,
                params,
            },
        );
        self.scopes.push(Scope::Function(name));
        self.visit(body);
        self.scopes.pop();
    }

    fn visit_binding(&mut self, node: Node<'a>) {
        if !in_function(&self.scopes) {
            self.visit_bound_closure(node);
        }
        if let Some(value) = node.child_by_field_name("value") {
            self.visit(value);
        }
    }

    fn visit_bound_closure(&mut self, node: Node<'a>) {
        let Some(closure) = bound_closure(node) else {
            return;
        };
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = node_text(name_node, self.source);
        let params = closure.child_by_field_name("parameters");
        self.push_function(
            &name,
            FnSite {
                item: node,
                body: closure,
                params,
            },
        );
    }

    fn visit_macro_definition(&mut self, node: Node<'a>) {
        if in_function(&self.scopes) {
            self.push_token_fns(node);
            return;
        }
        let Some(name_node) = node.child_by_field_name("name") else {
            return;
        };
        let name = node_text(name_node, self.source);
        self.push_function(
            &name,
            FnSite {
                item: node,
                body: node,
                params: None,
            },
        );
        self.scopes.push(Scope::Function(name));
        self.push_token_fns(node);
        self.scopes.pop();
    }

    fn push_token_fns(&mut self, node: Node<'a>) {
        for (name, body) in fns_in_tokens(node, self.source) {
            self.push_function(
                &name,
                FnSite {
                    item: body,
                    body,
                    params: None,
                },
            );
            self.scopes.push(Scope::Function(name));
            self.push_token_fns(body);
            self.scopes.pop();
        }
    }

    fn push_function(&mut self, name: &str, site: FnSite<'a>) {
        let FnSite { item, body, params } = site;
        let span = span_of(item);
        let attributes = leading_attributes(item);
        let anchor = attributes.first().copied().unwrap_or(item);
        let waivers = waivers_above(anchor, self.source);
        let mut touch = span;
        if let Some(attribute) = attributes.first() {
            let attribute_span = span_of(*attribute);
            touch.start = attribute_span.start;
        }
        if let Some(first) = waivers.first() {
            touch.start = first.line;
        }
        let params = chosen_params(body, params, self.source);
        let ordinary = ordinary_names(&params);
        let receiver = receiver_name(&params);
        let body = classify(
            body,
            self.source,
            Names {
                receiver: receiver.as_deref(),
                ordinary: &ordinary,
            },
        );
        self.out.push(Function {
            name: qualify(&self.scopes, name),
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

fn peel(node: Node, source: &[u8]) -> Option<Peeled> {
    match node.kind() {
        "identifier"
        | "self"
        | "scoped_identifier"
        | "scoped_type_identifier_in_expression_position" => Some(peel_name(node, source)),
        _ => peel_compound(node, source),
    }
}

fn peel_name(node: Node, source: &[u8]) -> Peeled {
    match node.kind() {
        "identifier" | "self" => rooted_peel(node, source),
        _ => empty_peel(),
    }
}

fn rooted_peel(node: Node, source: &[u8]) -> Peeled {
    Peeled {
        root: Some(node_text(node, source)),
        hops: Vec::new(),
    }
}

fn empty_peel() -> Peeled {
    Peeled {
        root: None,
        hops: Vec::new(),
    }
}

fn peel_compound(node: Node, source: &[u8]) -> Option<Peeled> {
    match node.kind() {
        "field_expression" => peel_field(node, source),
        "generic_function" => peel_generic(node, source),
        "call_expression" => peel_call(node, source),
        "index_expression" => peel_index(node, source),
        "parenthesized_expression" | "await_expression" | "try_expression" => {
            peel_wrapped(node, source)
        }
        _ => None,
    }
}

fn peel_field(node: Node, source: &[u8]) -> Option<Peeled> {
    let object = node.child_by_field_name("value")?;
    let mut peeled = peel(object, source).unwrap_or_else(empty_peel);
    let name = node.child_by_field_name("field")?;
    push_hop(
        &mut peeled,
        Hop {
            name: node_text(name, source),
            call: false,
            line: line_of(name),
        },
    );
    Some(peeled)
}

fn push_hop(peeled: &mut Peeled, hop: Hop) {
    let hops = &mut peeled.hops;
    hops.push(hop);
}

fn peel_generic(node: Node, source: &[u8]) -> Option<Peeled> {
    let function = node.child_by_field_name("function")?;
    peel(function, source)
}

fn peel_call(node: Node, source: &[u8]) -> Option<Peeled> {
    let function = node.child_by_field_name("function")?;
    if let Some(name) = bare_call_name(function, source) {
        return peel_bare_call(node, name, function);
    }
    let mut peeled = peel(function, source)?;
    mark_call(&mut peeled);
    Some(peeled)
}

fn peel_bare_call(node: Node, name: String, function: Node) -> Option<Peeled> {
    if bare_call_roots_a_chain(node) {
        return Some(empty_peel());
    }
    Some(Peeled {
        root: None,
        hops: vec![Hop {
            name,
            call: true,
            line: line_of(function),
        }],
    })
}

fn mark_call(peeled: &mut Peeled) {
    let hops = &mut peeled.hops;
    let Some(last) = hops.last_mut() else {
        return;
    };
    last.call = true;
}

fn peel_index(node: Node, source: &[u8]) -> Option<Peeled> {
    let children = code_nodes(node);
    let value = children.first()?;
    peel(*value, source)
}

fn peel_wrapped(node: Node, source: &[u8]) -> Option<Peeled> {
    let children = code_nodes(node);
    let [child] = children.as_slice() else {
        return None;
    };
    peel(*child, source)
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
        .map(|node| closure_param_list(node, source))
        .unwrap_or_default()
}

fn closure_param_list(node: Node, source: &[u8]) -> Vec<Param> {
    named_nodes(node)
        .into_iter()
        .map(|child| Param {
            name: binding_name(child, source),
            receiver: false,
        })
        .filter(kept_param)
        .collect()
}

fn kept_param(param: &Param) -> bool {
    let name = &param.name;
    !name.is_empty()
}

fn chosen_params(body: Node, params: Option<Node>, source: &[u8]) -> Vec<Param> {
    if body.kind() == "closure_expression" {
        closure_params(params, source)
    } else {
        function_params(params, source)
    }
}

fn ordinary_names(params: &[Param]) -> Vec<String> {
    params
        .iter()
        .filter(|param| !param.receiver)
        .map(|param| {
            let name = &param.name;
            name.clone()
        })
        .collect()
}

fn receiver_name(params: &[Param]) -> Option<String> {
    params.iter().find(|param| param.receiver).map(|param| {
        let name = &param.name;
        name.clone()
    })
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
        return missing_arguments(attribute);
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

struct WaiverScan<'a> {
    source: &'a [u8],
    next_line: u32,
    found: Vec<WaiverLine>,
}

fn waivers_above(node: Node, source: &[u8]) -> Vec<WaiverLine> {
    let start = span_of(node);
    let mut scan = WaiverScan {
        source,
        next_line: start.start,
        found: Vec::new(),
    };
    consume_waiver_comments(node, &mut scan);
    reversed(scan.found)
}

fn reversed(mut found: Vec<WaiverLine>) -> Vec<WaiverLine> {
    found.reverse();
    found
}

fn consume_waiver_comments(node: Node, scan: &mut WaiverScan<'_>) -> bool {
    let mut cursor = node.prev_sibling();
    while let Some(sibling) = cursor {
        if !take_waiver(scan, sibling) {
            return false;
        }
        cursor = sibling.prev_sibling();
    }
    true
}

fn take_waiver(scan: &mut WaiverScan<'_>, sibling: Node) -> bool {
    if sibling.kind() != "line_comment" {
        return false;
    }
    let span = span_of(sibling);
    if !comment_is_adjacent(span, scan.next_line, sibling) {
        return false;
    }
    let text = node_text(sibling, scan.source);
    let Some(waiver) = waiver_from_comment(&text, span.start) else {
        return false;
    };
    push_waiver(scan, waiver);
    scan.next_line = span.start;
    true
}

fn push_waiver(scan: &mut WaiverScan<'_>, waiver: WaiverLine) {
    let found = &mut scan.found;
    found.push(waiver);
}

fn comment_is_adjacent(span: Span, next_line: u32, sibling: Node) -> bool {
    span.start == span.end && span.start + 1 == next_line && !shares_code_line(sibling)
}

fn waiver_from_comment(text: &str, line: u32) -> Option<WaiverLine> {
    let text = trimmed_comment(text);
    let (marker, rest) = take_token(&text)?;
    if marker != "slopmeter:" {
        return None;
    }
    let (verb, rest) = take_token(rest)?;
    if verb != "waive" {
        return None;
    }
    let (rule, rest) = optional_token(rest);
    let (number, reason) = waiver_number(rest);
    Some(WaiverLine {
        line,
        rule,
        number,
        reason,
    })
}

fn trimmed_comment(text: &str) -> String {
    let text = text.trim();
    let stripped = text.strip_prefix("//").unwrap_or(text);
    stripped.trim().to_string()
}

fn optional_token(input: &str) -> (Option<String>, &str) {
    match take_token(input) {
        Some((rule, rest)) => (Some(rule.to_string()), rest),
        None => (None, ""),
    }
}

fn waiver_number(input: &str) -> (Option<String>, Option<String>) {
    match take_token(input) {
        Some((token, after)) if is_number_token(token) => {
            (Some(token.to_string()), non_empty(after))
        }
        Some(_) => (None, non_empty(input)),
        None => (None, None),
    }
}

fn missing_arguments(attribute: Node) -> bool {
    let arguments = attribute.child_by_field_name("arguments");
    arguments.is_none()
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

fn fns_in_tokens<'a>(node: Node<'a>, source: &[u8]) -> Vec<(String, Node<'a>)> {
    let mut found = Vec::new();
    collect_fns(node, source, &mut found);
    found
}

struct FnSearch<'a, 's> {
    source: &'s [u8],
    found: &'s mut Vec<(String, Node<'a>)>,
    skip: BTreeSet<(usize, usize)>,
}

fn collect_fns<'a>(node: Node<'a>, source: &[u8], found: &mut Vec<(String, Node<'a>)>) {
    let mut search = FnSearch {
        source,
        found,
        skip: BTreeSet::new(),
    };
    search.walk(node);
}

impl<'a, 's> FnSearch<'a, 's> {
    fn walk(&mut self, node: Node<'a>) {
        let children = all_children(node);
        for index in 0..children.len() {
            self.note(&children, index);
        }
    }

    fn note(&mut self, siblings: &[Node<'a>], index: usize) {
        let child = siblings[index];
        if self.already_skipped(child) {
            return;
        }
        if self.record(siblings, index) {
            return;
        }
        if child.child_count() > 0 {
            self.walk(child);
        }
    }

    fn already_skipped(&self, child: Node<'a>) -> bool {
        let skip = &self.skip;
        skip.contains(&range_key(child))
    }

    fn record(&mut self, siblings: &[Node<'a>], index: usize) -> bool {
        let child = siblings[index];
        if !is_fn_token(child, self.source) {
            return false;
        }
        let Some((name, body)) = fn_after(siblings, index, self.source) else {
            return false;
        };
        self.skip.insert(range_key(body));
        self.found.push((name, body));
        true
    }
}

fn is_fn_token(child: Node, source: &[u8]) -> bool {
    anonymous_token(child, source).as_deref() == Some("fn")
}

fn fn_body_after<'a>(siblings: &[Node<'a>], index: usize, source: &[u8]) -> Option<Node<'a>> {
    fn_after(siblings, index, source).map(|(_, body)| body)
}

fn fn_after<'a>(siblings: &[Node<'a>], index: usize, source: &[u8]) -> Option<(String, Node<'a>)> {
    let mut name = None;
    for child in &siblings[index + 1..] {
        if take_fn_part(*child, source, &mut name) {
            return name.map(|name| (name, *child));
        }
    }
    None
}

fn take_fn_part(child: Node, source: &[u8], name: &mut Option<String>) -> bool {
    if name_missing(name) && child.kind() == "identifier" {
        *name = Some(node_text(child, source));
        return false;
    }
    name_present(name) && opens_with(child, "{")
}

fn name_missing(name: &Option<String>) -> bool {
    let missing = name.is_none();
    missing
}

fn name_present(name: &Option<String>) -> bool {
    let present = name.is_some();
    present
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

fn classify(node: Node<'_>, source: &[u8], names: Names<'_>) -> Body {
    let mut classifier = Classifier {
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
    let forward = classifier.forwarding(node, names.ordinary);
    match node.kind() {
        "function_item" => {
            if let Some(body) = node.child_by_field_name("body") {
                classifier.walk_block(body, 0);
            }
        }
        "closure_expression" => classifier.walk_closure_body(node, 0, true),
        "macro_definition" => classifier.walk_macro_definition(node, 0),
        "token_tree" => classifier.walk_tokens(node, 0),
        _ => {}
    }
    Body {
        lloc: classifier.lloc,
        decisions: classifier.decisions,
        levels: classifier.levels,
        cognitive: classifier.cognitive,
        chains: classifier.chains,
        swallowing: classifier.swallowing,
        io: record_io(node, source),
        forward,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BoolOp {
    And,
    Or,
}

struct TokenChain {
    rooted: bool,
    members: Vec<Member>,
    line: u32,
    after_dot: bool,
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

struct LetCursor {
    lets: u32,
    last: Option<BoolOp>,
}

#[derive(Clone, Copy)]
struct TokenAt<'a, 's> {
    siblings: &'s [Node<'a>],
    index: usize,
}

impl<'a, 's> TokenAt<'a, 's> {
    fn node(self) -> Node<'a> {
        let siblings = self.siblings;
        siblings[self.index]
    }
}

struct TokenState<'a> {
    nesting: u32,
    last_bool: &'a mut Option<BoolOp>,
    skipped: &'a mut BTreeSet<(usize, usize)>,
    chain: &'a mut Option<TokenChain>,
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
        if self.walk_active(node, nesting) {
            return;
        }
        self.note_inert(node);
    }

    fn walk_active(&mut self, node: Node<'a>, nesting: u32) -> bool {
        match node.kind() {
            "expression_statement" => self.walk_expression_statement(node, nesting),
            "let_declaration" => self.walk_let(node, nesting),
            "macro_invocation" => self.count_macro(node, nesting),
            "macro_definition" => self.count_macro_def(node, nesting),
            "const_item" | "static_item" => self.walk_const(node, nesting),
            _ => return false,
        }
        true
    }

    fn walk_expression_statement(&mut self, node: Node<'a>, nesting: u32) {
        let children = code_nodes(node);
        self.note_ok_statement(node, &children);
        match children.as_slice() {
            [child] => self.note_expression(*child, nesting),
            _ => self.walk_expression_parts(&children, nesting),
        }
    }

    fn note_ok_statement(&mut self, node: Node<'a>, children: &[Node<'a>]) {
        let [only] = children else {
            return;
        };
        if ends_ok(*only, self.source) {
            self.swallowing.push(line_of(node));
        }
    }

    fn note_swallowed_if(&mut self, node: Node<'a>) {
        let Some(condition) = node.child_by_field_name("condition") else {
            return;
        };
        if !swallows_err(condition, self.source) {
            return;
        }
        let Some(body) = node.child_by_field_name("consequence") else {
            return;
        };
        if quiet_body(body) {
            self.swallowing.push(line_of(node));
        }
    }

    fn note_swallowed_arm(&mut self, node: Node<'a>) {
        let Some(pattern) = node.child_by_field_name("pattern") else {
            return;
        };
        if !err_pattern(pattern, self.source) {
            return;
        }
        let Some(value) = node.child_by_field_name("value") else {
            return;
        };
        if quiet_body(value) {
            self.swallowing.push(line_of(node));
        }
    }

    fn note_discarded_let(&mut self, node: Node<'a>) {
        if discarded_call(node) {
            self.swallowing.push(line_of(node));
        }
    }

    fn walk_expression_parts(&mut self, children: &[Node<'a>], nesting: u32) {
        self.lloc += 1;
        for child in children {
            self.walk(*child, nesting);
        }
    }

    fn count_macro(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.walk_macro(node, nesting);
    }

    fn count_macro_def(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.walk_macro_definition(node, nesting);
    }

    fn walk_const(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        if let Some(value) = node.child_by_field_name("value") {
            self.walk(value, nesting);
        }
    }

    fn note_inert(&mut self, node: Node<'a>) {
        match node.kind() {
            "empty_statement" | "function_item" | "attribute_item" | "inner_attribute_item" => {}
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
        if self.walk_control(node, nesting) {
            return;
        }
        if self.walk_blockish(node, nesting) {
            return;
        }
        if self.walk_chain_expr(node, nesting) {
            return;
        }
        self.walk_other(node, nesting);
    }

    fn walk_control(&mut self, node: Node<'a>, nesting: u32) -> bool {
        match node.kind() {
            "if_expression" => self.walk_if(node, nesting, false),
            "while_expression" => self.walk_while(node, nesting),
            "loop_expression" => self.walk_loop(node, nesting),
            "for_expression" => self.walk_for(node, nesting),
            "match_expression" => self.walk_match(node, nesting),
            _ => return false,
        }
        true
    }

    fn walk_blockish(&mut self, node: Node<'a>, nesting: u32) -> bool {
        match node.kind() {
            "block" | "unsafe_block" | "async_block" | "gen_block" | "const_block"
            | "try_block" => {
                self.walk_transparent(node, nesting);
            }
            "closure_expression" => self.walk_closure(node, nesting),
            _ => return false,
        }
        true
    }

    fn walk_transparent(&mut self, node: Node<'a>, nesting: u32) {
        if let Some(inner) = transparent_block(node) {
            self.walk_block(inner, nesting);
        }
    }

    fn walk_chain_expr(&mut self, node: Node<'a>, nesting: u32) -> bool {
        match node.kind() {
            "field_expression"
            | "call_expression"
            | "generic_function"
            | "index_expression"
            | "parenthesized_expression"
            | "await_expression" => self.record_and_walk(node, nesting),
            "try_expression" => self.walk_try(node, nesting),
            _ => return false,
        }
        true
    }

    fn record_and_walk(&mut self, node: Node<'a>, nesting: u32) {
        self.record_chain(node);
        self.walk_named(node, nesting);
    }

    fn walk_try(&mut self, node: Node<'a>, nesting: u32) {
        self.decision(line_of(node), nesting);
        self.record_chain(node);
        self.walk_named(node, nesting);
    }

    fn walk_other(&mut self, node: Node<'a>, nesting: u32) {
        match node.kind() {
            "let_declaration" | "expression_statement" => self.walk_statement(node, nesting),
            "binary_expression" => self.walk_binary(node, nesting),
            "let_chain" => self.walk_let_chain(node, nesting),
            "let_condition" => self.walk_let_condition(node, nesting),
            "break_expression" | "continue_expression" => self.walk_jump(node, nesting),
            "macro_invocation" => self.walk_macro(node, nesting),
            _ => self.walk_named(node, nesting),
        }
    }

    fn walk_binary(&mut self, node: Node<'a>, nesting: u32) {
        if bool_op(node, self.source).is_some() {
            self.walk_boolean(node, nesting);
        } else {
            self.walk_named(node, nesting);
        }
    }

    fn walk_jump(&mut self, node: Node<'a>, nesting: u32) {
        if has_label(node) {
            self.fundamental(line_of(node));
        }
        self.walk_named(node, nesting);
    }
    fn walk_named(&mut self, node: Node<'a>, nesting: u32) {
        for child in named_nodes(node) {
            self.walk(child, nesting);
        }
    }

    fn walk_if(&mut self, node: Node<'a>, nesting: u32, else_if: bool) {
        let line = line_of(node);
        self.note_if_branch(line, nesting, else_if);
        self.decision(line, nesting);
        self.note_swallowed_if(node);
        self.walk_if_condition(node, nesting);
        self.walk_if_body(node, nesting);
        self.walk_if_else(node, nesting);
    }

    fn note_if_branch(&mut self, line: u32, nesting: u32, else_if: bool) {
        if else_if {
            self.hybrid(line);
        } else {
            self.structural(line, nesting);
        }
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
        self.inside(|this| this.walk_block(body, nesting + 1));
    }

    fn walk_if_else(&mut self, node: Node<'a>, nesting: u32) {
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
        if child.kind() == "if_expression" {
            self.walk_if(child, nesting, true);
            return;
        }
        self.hybrid(line_of(alternative));
        self.inside(|this| this.walk_block(child, nesting + 1));
    }
    fn walk_while(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.decision(line_of(node), nesting);
        self.structural(line_of(node), nesting);
        if let Some(condition) = node.child_by_field_name("condition") {
            self.walk_condition(condition, nesting);
        }
        if let Some(body) = node.child_by_field_name("body") {
            self.inside(|this| this.walk_block(body, nesting + 1));
        }
    }

    fn walk_loop(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.decision(line_of(node), nesting);
        self.structural(line_of(node), nesting);
        if let Some(body) = node.child_by_field_name("body") {
            self.inside(|this| this.walk_block(body, nesting + 1));
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
            self.inside(|this| this.walk_block(body, nesting + 1));
        }
    }

    fn walk_match(&mut self, node: Node<'a>, nesting: u32) {
        self.structural(line_of(node), nesting);
        self.walk_match_value(node, nesting);
        self.walk_match_arms(node, nesting);
    }

    fn walk_match_value(&mut self, node: Node<'a>, nesting: u32) {
        let Some(value) = node.child_by_field_name("value") else {
            return;
        };
        self.walk(value, nesting);
    }

    fn walk_match_arms(&mut self, node: Node<'a>, nesting: u32) {
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        self.walk_arms(body, nesting);
    }

    fn walk_arms(&mut self, body: Node<'a>, nesting: u32) {
        for arm in named_nodes(body) {
            self.walk_one_arm(arm, nesting);
        }
    }

    fn walk_one_arm(&mut self, arm: Node<'a>, nesting: u32) {
        if arm.kind() == "match_arm" {
            self.walk_arm(arm, nesting);
        }
    }
    fn walk_arm(&mut self, node: Node<'a>, nesting: u32) {
        let line = line_of(node);
        self.note_arm_labels(node, line, nesting);
        self.note_swallowed_arm(node);
        self.walk_arm_guard(node, nesting);
        self.walk_arm_value(node, nesting);
    }

    fn note_arm_labels(&mut self, node: Node<'a>, line: u32, nesting: u32) {
        let pattern = node.child_by_field_name("pattern");
        let labels = pattern
            .map(|pattern| count_specified(pattern, self.source))
            .unwrap_or(0);
        for _ in 0..labels {
            self.decision(line, nesting);
        }
    }

    fn walk_arm_guard(&mut self, node: Node<'a>, nesting: u32) {
        let Some(pattern) = node.child_by_field_name("pattern") else {
            return;
        };
        self.walk_guard(pattern, nesting);
    }

    fn walk_guard(&mut self, pattern: Node<'a>, nesting: u32) {
        let Some(guard) = pattern.child_by_field_name("condition") else {
            return;
        };
        self.decision(line_of(guard), nesting);
        self.walk_condition(guard, nesting);
    }

    fn walk_arm_value(&mut self, node: Node<'a>, nesting: u32) {
        let Some(value) = node.child_by_field_name("value") else {
            return;
        };
        self.walk_arm_body(value, nesting);
    }

    fn walk_arm_body(&mut self, value: Node<'a>, nesting: u32) {
        if arm_is_block(value) {
            self.inside(|this| this.walk(value, nesting + 1));
            return;
        }
        self.inside(|this| this.note_expression(value, nesting + 1));
    }
    fn walk_let(&mut self, node: Node<'a>, nesting: u32) {
        self.lloc += 1;
        self.note_discarded_let(node);
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
        let mut state = LetCursor {
            lets: 0,
            last: None,
        };
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk_let_link(child, nesting, &mut state);
        }
    }

    fn walk_let_link(&mut self, child: Node<'a>, nesting: u32, state: &mut LetCursor) {
        if self.note_let_and(child, nesting, state) {
            return;
        }
        self.walk_let_piece(child, nesting, state);
    }

    fn note_let_and(&mut self, child: Node<'a>, nesting: u32, state: &mut LetCursor) -> bool {
        if child.is_named() || node_text(child, self.source) != "&&" {
            return false;
        }
        self.decision(line_of(child), nesting);
        self.note_and(child, state);
        true
    }

    fn note_and(&mut self, child: Node<'a>, state: &mut LetCursor) {
        if state.last == Some(BoolOp::And) {
            return;
        }
        self.fundamental(line_of(child));
        state.last = Some(BoolOp::And);
    }

    fn walk_let_piece(&mut self, child: Node<'a>, nesting: u32, state: &mut LetCursor) {
        if child.kind() == "let_condition" {
            self.walk_extra_let(child, nesting, state);
            return;
        }
        if child.kind() == "let_chain" {
            self.walk_let_chain(child, nesting);
            return;
        }
        if child.is_named() {
            self.bool_spine(child, nesting, &mut state.last);
        }
    }

    fn walk_extra_let(&mut self, child: Node<'a>, nesting: u32, state: &mut LetCursor) {
        state.lets += 1;
        if state.lets > 1 {
            self.decision(line_of(child), nesting);
        }
        self.walk_let_condition(child, nesting);
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
        if self.spine_unary(node, nesting, last) {
            return;
        }
        if self.spine_group(node, nesting) {
            return;
        }
        if self.spine_let(node, nesting) {
            return;
        }
        self.spine_operator(node, nesting, last);
    }

    fn spine_unary(&mut self, node: Node<'a>, nesting: u32, last: &mut Option<BoolOp>) -> bool {
        if node.kind() != "unary_expression" || operator_text(node, self.source) != "!" {
            return false;
        }
        self.spine_negated(node, nesting, last);
        true
    }

    fn spine_negated(&mut self, node: Node<'a>, nesting: u32, last: &mut Option<BoolOp>) {
        let Some(inner) = first_named(node) else {
            return;
        };
        self.bool_spine(inner, nesting, last);
    }

    fn spine_group(&mut self, node: Node<'a>, nesting: u32) -> bool {
        if node.kind() != "parenthesized_expression" {
            return false;
        }
        self.record_chain(node);
        self.spine_grouped(node, nesting);
        true
    }

    fn spine_grouped(&mut self, node: Node<'a>, nesting: u32) {
        let mut inner = None;
        for child in named_nodes(node) {
            self.bool_spine(child, nesting, &mut inner);
        }
    }

    fn spine_let(&mut self, node: Node<'a>, nesting: u32) -> bool {
        if node.kind() != "let_chain" {
            return false;
        }
        self.walk_let_chain(node, nesting);
        true
    }

    fn spine_operator(&mut self, node: Node<'a>, nesting: u32, last: &mut Option<BoolOp>) {
        let Some(operator) = bool_op(node, self.source) else {
            self.walk(node, nesting);
            return;
        };
        self.spine_left(node, nesting, last);
        let line = operator_line(node);
        self.decision(line, nesting);
        self.note_operator(line, operator, last);
        self.spine_right(node, nesting, last);
    }

    fn spine_left(&mut self, node: Node<'a>, nesting: u32, last: &mut Option<BoolOp>) {
        let Some(child) = node.child_by_field_name("left") else {
            return;
        };
        self.bool_spine(child, nesting, last);
    }

    fn spine_right(&mut self, node: Node<'a>, nesting: u32, last: &mut Option<BoolOp>) {
        let Some(child) = node.child_by_field_name("right") else {
            return;
        };
        self.bool_spine(child, nesting, last);
    }

    fn note_operator(&mut self, line: u32, operator: BoolOp, last: &mut Option<BoolOp>) {
        if *last == Some(operator) {
            return;
        }
        self.fundamental(line);
        *last = Some(operator);
    }
    fn walk_macro(&mut self, node: Node<'a>, nesting: u32) {
        if ASSERTS.contains(&macro_name(node, self.source).as_str()) {
            self.decision(line_of(node), nesting);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "token_tree" {
                self.walk_tokens(child, nesting);
            }
        }
    }

    fn walk_tokens(&mut self, node: Node<'a>, nesting: u32) {
        let mut last_bool = None;
        let mut skipped = BTreeSet::new();
        let mut chain = None;
        let mut state = TokenState {
            nesting,
            last_bool: &mut last_bool,
            skipped: &mut skipped,
            chain: &mut chain,
        };
        self.walk_token_children(node, &mut state);
    }

    fn walk_token_children(&mut self, node: Node<'a>, state: &mut TokenState<'_>) {
        let children = all_children(node);
        for (index, child) in children.iter().copied().enumerate() {
            if skip_token(child, state.skipped) {
                continue;
            }
            let at = TokenAt {
                siblings: &children,
                index,
            };
            self.walk_token(at, state);
        }
        self.finish_chain(state);
    }

    fn walk_token(&mut self, at: TokenAt<'a, '_>, state: &mut TokenState<'_>) {
        if self.descend_token(at, state) {
            return;
        }
        if self.consume_chain(at, state) {
            return;
        }
        self.finish_chain(state);
        self.walk_control_token(at, state);
    }

    fn descend_token(&mut self, at: TokenAt<'a, '_>, state: &mut TokenState<'_>) -> bool {
        let node = at.node();
        if !token_group(node) {
            return false;
        }
        if opens_with(node, "{") {
            self.finish_chain(state);
        }
        self.walk_nested_tokens(node, state);
        true
    }

    fn walk_nested_tokens(&mut self, node: Node<'a>, state: &mut TokenState<'_>) {
        if grouped_tokens(node) {
            self.walk_grouped_tokens(node, state.nesting);
            return;
        }
        self.walk_open_tokens(node, state);
    }

    fn walk_grouped_tokens(&mut self, node: Node<'a>, nesting: u32) {
        let mut inner_bool = None;
        let mut inner_skipped = BTreeSet::new();
        let mut inner_chain = None;
        let mut state = TokenState {
            nesting,
            last_bool: &mut inner_bool,
            skipped: &mut inner_skipped,
            chain: &mut inner_chain,
        };
        self.walk_token_children(node, &mut state);
    }

    fn walk_open_tokens(&mut self, node: Node<'a>, state: &mut TokenState<'_>) {
        let mut inner_chain = None;
        let nesting = state.nesting;
        let mut inner = TokenState {
            nesting,
            last_bool: state.last_bool,
            skipped: state.skipped,
            chain: &mut inner_chain,
        };
        self.walk_token_children(node, &mut inner);
    }

    fn walk_control_token(&mut self, at: TokenAt<'a, '_>, state: &mut TokenState<'_>) {
        let node = at.node();
        let Some(token) = control_token(node, self.source) else {
            return;
        };
        self.dispatch_token(&token, at, state);
    }

    fn dispatch_token(&mut self, token: &str, at: TokenAt<'a, '_>, state: &mut TokenState<'_>) {
        if self.walk_loop_token(token, at, state) {
            return;
        }
        self.dispatch_other_token(token, at, state);
    }

    fn walk_loop_token(
        &mut self,
        token: &str,
        at: TokenAt<'a, '_>,
        state: &mut TokenState<'_>,
    ) -> bool {
        match token {
            "if" | "while" | "for" | "loop" => self.walk_token_loop(token, at, state),
            _ => return false,
        }
        true
    }

    fn walk_token_loop(&mut self, token: &str, at: TokenAt<'a, '_>, state: &mut TokenState<'_>) {
        let node = at.node();
        let line = line_of(node);
        if token == "if" && is_guard(at.siblings, at.index, self.source) {
            self.decision(line, state.nesting);
            return;
        }
        if token == "if" && previous_is_else(at.siblings, at.index, self.source) {
            self.decision(line, state.nesting);
            self.inside(|this| this.walk_brace_body(at, state));
            return;
        }
        self.structural(line, state.nesting);
        self.decision(line, state.nesting);
        self.inside(|this| this.walk_brace_body(at, state));
    }

    fn dispatch_other_token(
        &mut self,
        token: &str,
        at: TokenAt<'a, '_>,
        state: &mut TokenState<'_>,
    ) {
        match token {
            "match" => self.walk_token_match(at, state),
            "else" => self.walk_token_else(at, state),
            "&&" => self.bool_token(at.node(), BoolOp::And, state),
            "||" => self.bool_token(at.node(), BoolOp::Or, state),
            "fn" => self.skip_fn_body(at, state),
            _ => {}
        }
    }

    fn walk_token_match(&mut self, at: TokenAt<'a, '_>, state: &mut TokenState<'_>) {
        let node = at.node();
        self.structural(line_of(node), state.nesting);
        self.inside(|this| this.walk_brace_body(at, state));
    }

    fn walk_token_else(&mut self, at: TokenAt<'a, '_>, state: &mut TokenState<'_>) {
        let node = at.node();
        self.hybrid(line_of(node));
        if else_present(at, self.source) {
            self.inside(|this| this.walk_brace_body(at, state));
        }
    }

    fn skip_fn_body(&mut self, at: TokenAt<'a, '_>, state: &mut TokenState<'_>) {
        let Some(body) = fn_body_after(at.siblings, at.index, self.source) else {
            return;
        };
        remember(state.skipped, body);
    }
    fn walk_macro_definition(&mut self, node: Node<'a>, nesting: u32) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() != "macro_rule" {
                continue;
            }
            if let Some(right) = child.child_by_field_name("right") {
                self.walk_tokens(right, nesting);
            }
        }
    }

    fn walk_brace_body(&mut self, at: TokenAt<'a, '_>, state: &mut TokenState<'_>) {
        let Some(body) = brace_after(at.siblings, at.index) else {
            return;
        };
        remember(state.skipped, body);
        self.walk_tokens(body, state.nesting);
    }

    fn consume_chain(&mut self, at: TokenAt<'a, '_>, state: &mut TokenState<'_>) -> bool {
        let node = at.node();
        if dot_token(node, self.source) {
            mark_dot(state);
            return true;
        }
        let Some((name, rooted)) = chain_name(node, self.source) else {
            return false;
        };
        if chain_after_dot(state.chain) {
            self.extend_chain(at, name, state);
            return true;
        }
        self.start_chain(at, rooted, state)
    }

    fn extend_chain(&mut self, at: TokenAt<'a, '_>, name: String, state: &mut TokenState<'_>) {
        let call = next_call(at);
        let line = line_of(at.node());
        let built = live_chain(state);
        push_member(built, Member { name, call }, line);
    }

    fn start_chain(
        &mut self,
        at: TokenAt<'a, '_>,
        rooted: bool,
        state: &mut TokenState<'_>,
    ) -> bool {
        if !followed_by_dot(at, self.source) {
            return false;
        }
        self.finish_chain(state);
        let line = line_of(at.node());
        *state.chain = Some(TokenChain {
            rooted,
            members: Vec::new(),
            line,
            after_dot: false,
        });
        true
    }

    fn finish_chain(&mut self, state: &mut TokenState<'_>) {
        let Some(built) = take_built(state) else {
            return;
        };
        self.store_token_chain(built);
    }

    fn store_token_chain(&mut self, built: TokenChain) {
        if empty_members(&built) {
            return;
        }
        self.chains.push(Chain {
            line: built.line,
            rooted: built.rooted,
            members: built.members,
        });
    }

    fn bool_token(&mut self, node: Node<'a>, operator: BoolOp, state: &mut TokenState<'_>) {
        let line = line_of(node);
        self.decision(line, state.nesting);
        if *state.last_bool != Some(operator) {
            self.fundamental(line);
            *state.last_bool = Some(operator);
        }
    }

    fn record_chain(&mut self, node: Node<'a>) {
        if chain_continues(node) {
            return;
        }
        let Some(peeled) = peel(node, self.source) else {
            return;
        };
        if hops_empty(&peeled) {
            return;
        }
        if forward_root(&self.skip, &peeled, node) {
            return;
        }
        self.keep_peeled(peeled);
    }

    fn keep_peeled(&mut self, peeled: Peeled) {
        let line = hop_line(&peeled);
        let rooted = same_receiver(self.receiver, &peeled);
        let members = hop_members(peeled);
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

fn has_label(node: Node) -> bool {
    let label = node.child_by_field_name("label");
    if label.is_some() {
        return true;
    }
    let children = named_nodes(node);
    children.iter().any(|child| child.kind() == "label")
}

fn arm_is_block(value: Node) -> bool {
    value.kind() == "block" || is_transparent(value.kind())
}

fn first_named(node: Node<'_>) -> Option<Node<'_>> {
    named_nodes(node).into_iter().next()
}

fn operator_line(node: Node) -> u32 {
    let operator = node.child_by_field_name("operator");
    if let Some(operator) = operator {
        return line_of(operator);
    }
    line_of(node)
}

fn skip_token(child: Node, skipped: &BTreeSet<(usize, usize)>) -> bool {
    if child.kind() != "token_tree" {
        return false;
    }
    skipped.contains(&range_key(child))
}

fn token_group(node: Node) -> bool {
    node.kind() == "token_tree" || node.kind() == "token_repetition"
}

fn grouped_tokens(node: Node) -> bool {
    opens_with(node, "(") || opens_with(node, "{")
}

fn else_present(at: TokenAt<'_, '_>, source: &[u8]) -> bool {
    let body = else_body(at.siblings, at.index, source);
    body.is_some()
}

fn remember(skipped: &mut BTreeSet<(usize, usize)>, body: Node) {
    skipped.insert(range_key(body));
}

fn dot_token(node: Node, source: &[u8]) -> bool {
    anonymous_token(node, source).as_deref() == Some(".")
}

fn mark_dot(state: &mut TokenState<'_>) {
    let chain = &mut state.chain;
    let Some(built) = chain.as_mut() else {
        return;
    };
    built.after_dot = true;
}

fn chain_after_dot(chain: &Option<TokenChain>) -> bool {
    chain.as_ref().is_some_and(|built| built.after_dot)
}

fn next_call(at: TokenAt<'_, '_>) -> bool {
    let next = at.siblings.get(at.index + 1);
    next.is_some_and(|next| opens_with(*next, "("))
}

fn followed_by_dot(at: TokenAt<'_, '_>, source: &[u8]) -> bool {
    let next = at.siblings.get(at.index + 1);
    next.is_some_and(|next| dot_token(*next, source))
}

fn live_chain<'a, 'b>(state: &'b mut TokenState<'a>) -> &'b mut TokenChain {
    let chain = &mut state.chain;
    chain.as_mut().expect("dot started a chain")
}

fn take_built(state: &mut TokenState<'_>) -> Option<TokenChain> {
    let chain = &mut state.chain;
    chain.take()
}

fn empty_members(built: &TokenChain) -> bool {
    let members = &built.members;
    members.is_empty()
}

fn push_member(built: &mut TokenChain, member: Member, line: u32) {
    if empty_members(built) {
        built.line = line;
    }
    add_member(built, member);
    built.after_dot = false;
}

fn add_member(built: &mut TokenChain, member: Member) {
    let members = &mut built.members;
    members.push(member);
}

fn hops_empty(peeled: &Peeled) -> bool {
    let hops = &peeled.hops;
    hops.is_empty()
}

fn hop_line(peeled: &Peeled) -> u32 {
    let hops = &peeled.hops;
    let first = &hops[0];
    first.line
}

fn one_hop(peeled: &Peeled) -> bool {
    let hops = &peeled.hops;
    hops.len() == 1
}

fn missing_root(peeled: &Peeled) -> bool {
    let root = &peeled.root;
    root.is_none()
}

fn peeled_root(peeled: &Peeled) -> Option<&str> {
    let root = &peeled.root;
    root.as_deref()
}

fn same_receiver(receiver: Option<&str>, peeled: &Peeled) -> bool {
    let root = peeled_root(peeled);
    receiver.is_some_and(|name| root == Some(name))
}

fn hop_members(peeled: Peeled) -> Vec<Member> {
    let hops = peeled.hops;
    hops.into_iter()
        .map(|hop| Member {
            name: hop.name,
            call: hop.call,
        })
        .collect()
}

fn forward_root(skip: &Option<std::ops::Range<usize>>, peeled: &Peeled, node: Node) -> bool {
    same_range(skip, node) && missing_root(peeled) && one_hop(peeled)
}

fn same_range(skip: &Option<std::ops::Range<usize>>, node: Node) -> bool {
    let range = node.byte_range();
    skip.as_ref() == Some(&range)
}

fn forwarding_call<'a>(node: Node<'a>, source: &[u8]) -> Option<(u32, Node<'a>)> {
    match node.kind() {
        "function_item" => forward_function(node, source),
        "closure_expression" => forward_closure(node, source),
        _ => None,
    }
}

fn forward_function<'a>(node: Node<'a>, source: &[u8]) -> Option<(u32, Node<'a>)> {
    let body = node.child_by_field_name("body")?;
    let statement = sole_forward(&code_nodes(body), source)?;
    let call = unwrap_call(statement, source)?;
    Some((line_of(statement), call))
}

fn forward_closure<'a>(node: Node<'a>, source: &[u8]) -> Option<(u32, Node<'a>)> {
    let body = node.child_by_field_name("body")?;
    if body.kind() == "block" {
        return forward_block(body, source);
    }
    let call = unwrap_call(body, source)?;
    Some((line_of(body), call))
}

fn forward_block<'a>(body: Node<'a>, source: &[u8]) -> Option<(u32, Node<'a>)> {
    let statement = sole_forward(&code_nodes(body), source)?;
    let call = unwrap_call(statement, source)?;
    Some((line_of(statement), call))
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
        | "parenthesized_expression"
        | "await_expression"
        | "try_expression"
        | "return_expression" => {
            let children = code_nodes(node);
            let [child] = children.as_slice() else {
                return None;
            };
            logging_leaf(*child, source)
        }
        "macro_invocation" => Some(macro_name(node, source)),
        "call_expression" => Some(callee_leaf(node.child_by_field_name("function")?, source)),
        _ => None,
    }
}

fn callee_leaf(node: Node, source: &[u8]) -> String {
    match node.kind() {
        "identifier" | "field_identifier" => node_text(node, source),
        "field_expression" => node
            .child_by_field_name("field")
            .map(|field| callee_leaf(field, source))
            .unwrap_or_default(),
        "scoped_identifier" | "generic_function" => node
            .child_by_field_name("name")
            .map(|name| callee_leaf(name, source))
            .unwrap_or_default(),
        "call_expression" => node
            .child_by_field_name("function")
            .map(|function| callee_leaf(function, source))
            .unwrap_or_default(),
        "macro_invocation" => macro_name(node, source),
        _ => String::new(),
    }
}

fn is_dead_assignment(node: Node, source: &[u8], later: &[Node]) -> bool {
    let names = match node.kind() {
        "let_declaration" => node
            .child_by_field_name("pattern")
            .map(|pattern| bound_names(pattern, source))
            .unwrap_or_default(),
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
                .map(|left| bound_names(left, source))
                .unwrap_or_default()
        }
        _ => return false,
    };
    !names.is_empty() && !names.iter().any(|name| mentioned(later, source, name))
}

fn bound_names(node: Node, source: &[u8]) -> Vec<String> {
    match node.kind() {
        "identifier" => vec![node_text(node, source)],
        "tuple_pattern" | "slice_pattern" | "tuple_struct_pattern" | "or_pattern" => {
            code_nodes(node)
                .into_iter()
                .flat_map(|child| bound_names(child, source))
                .collect()
        }
        "ref_pattern" => node
            .child_by_field_name("pattern")
            .map(|pattern| bound_names(pattern, source))
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn mentioned(nodes: &[Node], source: &[u8], name: &str) -> bool {
    nodes.iter().any(|node| contains_name(*node, source, name))
}

fn contains_name(node: Node, source: &[u8], name: &str) -> bool {
    name_in(node, (source, name))
}

fn name_in(node: Node, sought: (&[u8], &str)) -> bool {
    if same_identifier(node, sought) {
        return true;
    }
    child_contains(node, sought)
}

fn same_identifier(node: Node, sought: (&[u8], &str)) -> bool {
    let (source, name) = sought;
    node.kind() == "identifier" && node_text(node, source) == name
}

fn child_contains(node: Node, sought: (&[u8], &str)) -> bool {
    for index in 0..node.child_count() {
        if child_has_name(node, index, sought) {
            return true;
        }
    }
    false
}

fn child_has_name(node: Node, index: u32, sought: (&[u8], &str)) -> bool {
    let Some(child) = node.child(index) else {
        return false;
    };
    name_in(child, sought)
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
        | "return_expression"
        | "parenthesized_expression"
        | "await_expression"
        | "try_expression" => {
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
    match node.kind() {
        "integer_literal" | "float_literal" | "string_literal" | "char_literal"
        | "boolean_literal" | "raw_string_literal" | "negative_literal" => true,
        "parenthesized_expression" => {
            let children = code_nodes(node);
            let [child] = children.as_slice() else {
                return false;
            };
            is_literal(*child)
        }
        _ => false,
    }
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
        return identity_wraps_call(node);
    }
    if node.kind() == "call_expression" {
        return true;
    }
    child_is_call(node, source)
}

fn identity_wraps_call(node: Node) -> bool {
    let receiver = wrapper_receiver(node);
    receiver.is_some_and(contains_call)
}

fn child_is_call(node: Node, source: &[u8]) -> bool {
    for index in 0..node.child_count() {
        if one_child_calls(node, index, source) {
            return true;
        }
    }
    false
}

fn one_child_calls(node: Node, index: u32, source: &[u8]) -> bool {
    let Some(child) = node.child(index) else {
        return false;
    };
    nested_call(child, source)
}
fn contains_call(node: Node) -> bool {
    if node.kind() == "call_expression" {
        return true;
    }
    child_contains_call(node)
}

fn child_contains_call(node: Node) -> bool {
    for index in 0..node.child_count() {
        if one_contains_call(node, index) {
            return true;
        }
    }
    false
}

fn one_contains_call(node: Node, index: u32) -> bool {
    let Some(child) = node.child(index) else {
        return false;
    };
    contains_call(child)
}
fn wrapper_receiver(node: Node) -> Option<Node> {
    let function = node.child_by_field_name("function")?;
    function.child_by_field_name("value")
}
fn forwarded_names(call: Node, source: &[u8], ordinary: &[String]) -> Vec<String> {
    let mut found = BTreeSet::new();
    collect_receiver(call, source, &mut found);
    collect_arguments(call, source, &mut found);
    ordinary
        .iter()
        .filter(|name| found.contains(*name))
        .cloned()
        .collect()
}

fn collect_receiver(call: Node, source: &[u8], found: &mut BTreeSet<String>) {
    let Some(function) = call.child_by_field_name("function") else {
        return;
    };
    if function.kind() != "field_expression" {
        return;
    }
    insert_bare(function, source, found);
}

fn insert_bare(function: Node, source: &[u8], found: &mut BTreeSet<String>) {
    let Some(object) = function.child_by_field_name("value") else {
        return;
    };
    let Some(name) = bare_name(object, source) else {
        return;
    };
    found.insert(name);
}

fn collect_arguments(call: Node, source: &[u8], found: &mut BTreeSet<String>) {
    let Some(arguments) = call.child_by_field_name("arguments") else {
        return;
    };
    for argument in code_nodes(arguments) {
        insert_argument(argument, source, found);
    }
}

fn insert_argument(argument: Node, source: &[u8], found: &mut BTreeSet<String>) {
    let Some(name) = bare_name(argument, source) else {
        return;
    };
    found.insert(name);
}
fn bare_name(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => Some(node_text(node, source)),
        "parenthesized_expression" => bare_parenthesized(node, source),
        "reference_expression" => bare_reference(node, source),
        "unary_expression" => bare_unary(node, source),
        "call_expression" => bare_identity(node, source),
        _ => None,
    }
}

fn bare_parenthesized(node: Node, source: &[u8]) -> Option<String> {
    let children = code_nodes(node);
    let [child] = children.as_slice() else {
        return None;
    };
    bare_name(*child, source)
}

fn bare_reference(node: Node, source: &[u8]) -> Option<String> {
    let value = node.child_by_field_name("value")?;
    bare_name(value, source)
}

fn bare_unary(node: Node, source: &[u8]) -> Option<String> {
    if operator_text(node, source) != "*" {
        return None;
    }
    let children = code_nodes(node);
    let child = children.first()?;
    bare_name(*child, source)
}

fn bare_identity(node: Node, source: &[u8]) -> Option<String> {
    if !identity_named(node, source) {
        return None;
    }
    let receiver = wrapper_receiver(node)?;
    bare_name(receiver, source)
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
    if !identity_field(field, source) {
        return false;
    }
    arguments_absent(node)
}

fn identity_field(field: Node, source: &[u8]) -> bool {
    let name = node_text(field, source);
    IDENTITY.contains(&name.as_str())
}

fn arguments_absent(node: Node) -> bool {
    let arguments = node.child_by_field_name("arguments");
    arguments.is_none_or(argument_list_empty)
}

fn argument_list_empty(arguments: Node) -> bool {
    let nodes = code_nodes(arguments);
    nodes.is_empty()
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

fn all_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).collect()
}

fn range_key(node: Node<'_>) -> (usize, usize) {
    let range = node.byte_range();
    (range.start, range.end)
}

fn anonymous_token(node: Node<'_>, source: &[u8]) -> Option<String> {
    if node.is_named() {
        None
    } else {
        Some(node_text(node, source))
    }
}

fn chain_name(node: Node<'_>, source: &[u8]) -> Option<(String, bool)> {
    match node.kind() {
        "self" => Some((node_text(node, source), true)),
        "identifier" if node_text(node, source) != "else" => Some((node_text(node, source), false)),
        _ => None,
    }
}

fn control_token(node: Node<'_>, source: &[u8]) -> Option<String> {
    if let Some(token) = anonymous_token(node, source) {
        return Some(token);
    }
    if node.kind() == "identifier" && node_text(node, source) == "else" {
        return Some("else".to_string());
    }
    None
}

fn opens_with(node: Node<'_>, token: &str) -> bool {
    if node.kind() != "token_tree" {
        return false;
    }
    let child = node.child(0);
    child.is_some_and(|child| child.kind() == token)
}
fn brace_after<'a>(siblings: &[Node<'a>], index: usize) -> Option<Node<'a>> {
    siblings[index + 1..]
        .iter()
        .copied()
        .find(|child| opens_with(*child, "{"))
}

fn previous_is_else(siblings: &[Node<'_>], index: usize, source: &[u8]) -> bool {
    index > 0 && control_token(siblings[index - 1], source).as_deref() == Some("else")
}

fn is_guard(siblings: &[Node<'_>], index: usize, source: &[u8]) -> bool {
    for child in &siblings[index + 1..] {
        if anonymous_token(*child, source).as_deref() == Some("=>") {
            return true;
        }
        if opens_with(*child, "{") {
            return false;
        }
    }
    false
}

fn else_body<'a>(siblings: &[Node<'a>], index: usize, source: &[u8]) -> Option<Node<'a>> {
    for child in &siblings[index + 1..] {
        if anonymous_token(*child, source).as_deref() == Some("if") {
            return None;
        }
        if opens_with(*child, "{") {
            return Some(*child);
        }
    }
    None
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
    let line = row_of(comment);
    let mut cursor = comment.prev_sibling();
    while let Some(sibling) = cursor {
        if !is_line_comment(sibling) {
            return shares_row(sibling, line);
        }
        if occupied_end(sibling) < line {
            return false;
        }
        cursor = sibling.prev_sibling();
    }
    false
}

fn is_line_comment(node: Node) -> bool {
    node.kind() == "line_comment"
}

fn shares_row(sibling: Node, line: usize) -> bool {
    let start = row_of(sibling);
    start <= line && occupied_end(sibling) >= line
}
fn occupied_end(node: Node) -> usize {
    let end = node.end_position();
    if end.column == 0 && end.row > row_of(node) {
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
        match chain_parent(parent, current) {
            ParentStep::Stop(found) => return found,
            ParentStep::Rise(next) => current = next,
        }
    }
}

enum ParentStep<'a> {
    Stop(bool),
    Rise(Node<'a>),
}

fn chain_parent<'a>(parent: Node<'a>, current: Node<'a>) -> ParentStep<'a> {
    match parent.kind() {
        "field_expression" => ParentStep::Stop(field_value_is(parent, current)),
        "index_expression" => index_step(parent, current),
        "parenthesized_expression" | "await_expression" | "try_expression" => {
            wrapped_step(parent, current)
        }
        "call_expression" | "generic_function" => call_step(parent, current),
        _ => ParentStep::Stop(false),
    }
}

fn field_value_is(parent: Node, current: Node) -> bool {
    parent.child_by_field_name("value") == Some(current)
}

fn index_step<'a>(parent: Node<'a>, current: Node<'a>) -> ParentStep<'a> {
    let children = code_nodes(parent);
    if children.first() != Some(&current) {
        return ParentStep::Stop(false);
    }
    ParentStep::Rise(parent)
}

fn wrapped_step<'a>(parent: Node<'a>, current: Node<'a>) -> ParentStep<'a> {
    let children = code_nodes(parent);
    if children.len() != 1 || children.first() != Some(&current) {
        return ParentStep::Stop(false);
    }
    ParentStep::Rise(parent)
}

fn call_step<'a>(parent: Node<'a>, current: Node<'a>) -> ParentStep<'a> {
    if parent.child_by_field_name("function") != Some(current) {
        return ParentStep::Stop(false);
    }
    ParentStep::Rise(parent)
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

fn discarded_call(node: Node<'_>) -> bool {
    wildcard_let(node) && call_value(node)
}

fn wildcard_let(node: Node<'_>) -> bool {
    let pattern = node.child_by_field_name("pattern");
    pattern.is_some_and(is_wildcard)
}

fn is_wildcard(node: Node<'_>) -> bool {
    node.kind() == "_" || node.kind() == "wildcard_pattern"
}

fn call_value(node: Node<'_>) -> bool {
    let value = node.child_by_field_name("value");
    value.is_some_and(|value| value.kind() == "call_expression")
}

fn ends_ok(node: Node<'_>, source: &[u8]) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    ok_field(function, source) && empty_args(node)
}

fn ok_field(function: Node<'_>, source: &[u8]) -> bool {
    if function.kind() != "field_expression" {
        return false;
    }
    let field = function.child_by_field_name("field");
    field.is_some_and(|field| node_text(field, source) == "ok")
}

fn empty_args(node: Node<'_>) -> bool {
    let args = node.child_by_field_name("arguments");
    args.is_some_and(|args| args.named_child_count() == 0)
}

fn swallows_err(node: Node<'_>, source: &[u8]) -> bool {
    match node.kind() {
        "let_condition" => let_swallows(node, source),
        "let_chain" => chain_swallows(node, source),
        _ => false,
    }
}

fn let_swallows(node: Node<'_>, source: &[u8]) -> bool {
    let pattern = node.child_by_field_name("pattern");
    pattern.is_some_and(|pattern| err_pattern(pattern, source))
}

fn chain_swallows(node: Node<'_>, source: &[u8]) -> bool {
    let children = named_nodes(node);
    let mut iter = children.iter();
    iter.any(|child| swallows_err(*child, source))
}

fn err_pattern(node: Node<'_>, source: &[u8]) -> bool {
    let Some(pattern) = bare_pattern(node) else {
        return false;
    };
    err_tuple(pattern, source)
}

fn bare_pattern(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() != "match_pattern" {
        return Some(node);
    }
    named_nodes(node)
        .into_iter()
        .find(|child| node.child_by_field_name("condition") != Some(*child))
}

fn err_tuple(pattern: Node<'_>, source: &[u8]) -> bool {
    if pattern.kind() != "tuple_struct_pattern" {
        return false;
    }
    err_type(pattern, source) && one_plain_payload(pattern)
}

fn err_type(pattern: Node<'_>, source: &[u8]) -> bool {
    let ty = pattern.child_by_field_name("type");
    ty.is_some_and(|ty| node_text(ty, source) == "Err")
}

fn one_plain_payload(pattern: Node<'_>) -> bool {
    let payloads = payloads_of(pattern);
    match payloads.as_slice() {
        [only] => plain_name(*only),
        _ => false,
    }
}

fn payloads_of(pattern: Node<'_>) -> Vec<Node<'_>> {
    all_children(pattern)
        .into_iter()
        .filter(|child| pattern.child_by_field_name("type") != Some(*child))
        .filter(|child| child.is_named() || is_wildcard(*child))
        .collect()
}

fn plain_name(node: Node<'_>) -> bool {
    is_wildcard(node) || node.kind() == "identifier"
}

fn quiet_body(node: Node<'_>) -> bool {
    match node.kind() {
        "unit_expression" => true,
        "block" => quiet_block(node),
        _ => false,
    }
}

fn quiet_block(node: Node<'_>) -> bool {
    let children = code_nodes(node);
    match children.as_slice() {
        [] => true,
        [only] => unit_value(*only),
        _ => false,
    }
}

fn unit_value(node: Node<'_>) -> bool {
    if node.kind() == "unit_expression" {
        return true;
    }
    if node.kind() != "expression_statement" {
        return false;
    }
    let children = code_nodes(node);
    matches!(children.as_slice(), [only] if only.kind() == "unit_expression")
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
    let start_row = row_of(node);
    let start = start_row as u32 + 1;
    let end_point = node.end_position();
    let end = if end_point.column == 0 && end_point.row > start_row {
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
    let text = node_text(node, source);
    let parts: Vec<_> = text.split_whitespace().collect();
    parts.join(" ")
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

fn operator_text(node: Node, source: &[u8]) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !child.is_named() {
            return node_text(child, source);
        }
    }
    String::new()
}

const IO_NAMES: &[&str] = &[
    "File::open",
    "File::create",
    "TcpStream::connect",
    "TcpListener::bind",
    "Command::new",
    "reqwest::Client::new",
    "reqwest::blocking::Client::new",
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
    !root && node.kind() == "function_item"
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
    if node.kind() != "call_expression" {
        return None;
    }
    let function = node.child_by_field_name("function")?;
    written_name(function, source)
}

fn written_name(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => Some(node_text(node, source)),
        "scoped_identifier" => scoped_name(node, source),
        "generic_function" => generic_name(node, source),
        _ => None,
    }
}

fn generic_name(node: Node, source: &[u8]) -> Option<String> {
    let function = node.child_by_field_name("function")?;
    written_name(function, source)
}

fn scoped_name(node: Node, source: &[u8]) -> Option<String> {
    let path = node.child_by_field_name("path")?;
    let left = written_name(path, source)?;
    let name = node.child_by_field_name("name")?;
    let right = node_text(name, source);
    Some(format!("{left}::{right}"))
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
    fn a_pass_through_ignores_logging_and_a_dead_assignment() {
        let source = "\
fn logged(a: i32, b: i32, c: i32) -> i32 {
    log(a);
    run(a, b, c)
}

fn printed(a: i32, b: i32) -> i32 {
    println!(\"{a}\");
    run(a, b)
}

fn dead(a: i32, b: i32) -> i32 {
    let unused = 1;
    run(a, b)
}

fn alias(a: i32) -> i32 {
    let tmp = a;
    f(tmp)
}

fn wrapped(a: i32, b: i32) -> i32 {
    Ok(run(a, b))
}

fn layered(a: i32, b: i32) -> i32 {
    Ok(Some(run(a, b)))
}

fn boxed(a: i32, b: i32) -> i32 {
    Box::new(run(a, b))
}

fn keyed(xs: i32) -> i32 {
    sorted(xs, key)
}

fn literal(xs: i32) -> i32 {
    sorted(xs, true)
}

fn method(repo: Repo, user: User) -> i32 {
    repo.save(user)
}
";
        let file = parse("a.rs", source.as_bytes());
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
            function(&file, "printed")
                .body
                .forward
                .as_ref()
                .unwrap()
                .names,
            vec!["a".to_string(), "b".to_string()]
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

    #[test]
    fn nesting_is_the_deepest_structural_level() {
        let cases = [
            (
                "\
fn boolean(a: bool, b: bool) -> bool {
    a && b
}
",
                "boolean",
                0,
                None,
            ),
            (
                "\
fn conditional(a: bool, b: bool) -> i32 {
    if a && b {
        1
    } else {
        0
    }
}
",
                "conditional",
                1,
                Some(2),
            ),
            (
                "\
fn ternary_bool(a: bool, b: bool) -> i32 {
    if a && b { 1 } else { 0 }
}
",
                "ternary_bool",
                1,
                Some(2),
            ),
            (
                "\
fn in_lambda(a: bool) -> i32 {
    let pick = |y: i32| if a { y } else { 0 };
    pick(1)
}
",
                "in_lambda",
                1,
                Some(2),
            ),
            (
                "\
fn nested_keeps_its_own(a: bool) -> bool {
    fn inner(x: bool) -> bool {
        if x {
            return x;
        }
        false
    }
    inner(a)
}
",
                "nested_keeps_its_own",
                0,
                None,
            ),
            (
                "\
fn nested_keeps_its_own(a: bool) -> bool {
    fn inner(x: bool) -> bool {
        if x {
            return x;
        }
        false
    }
    inner(a)
}
",
                "nested_keeps_its_own::inner",
                1,
                Some(3),
            ),
            (
                "\
fn chained(a: bool, b: bool) -> i32 {
    if a {
        1
    } else {
        if b { 2 } else { 3 }
    }
}
",
                "chained",
                2,
                Some(5),
            ),
            (
                "\
fn else_if_chain(a: bool, b: bool) -> i32 {
    if a {
        1
    } else if b {
        2
    } else {
        3
    }
}
",
                "else_if_chain",
                1,
                Some(2),
            ),
            (
                "\
fn in_condition(a: bool) -> i32 {
    if if a { true } else { false } {
        1
    } else {
        0
    }
}
",
                "in_condition",
                1,
                Some(2),
            ),
            (
                "\
fn operand(a: bool) -> i32 {
    if if a { 2 } else { 0 } == 1 { 1 } else { 3 }
}
",
                "operand",
                1,
                Some(2),
            ),
            (
                "\
fn in_body(a: bool, b: bool) -> i32 {
    if a {
        if b { 1 } else { 0 }
    } else {
        0
    }
}
",
                "in_body",
                2,
                Some(3),
            ),
            (
                "\
fn if_else(a: bool) -> i32 {
    if a {
        1
    } else {
        0
    }
}
",
                "if_else",
                1,
                Some(2),
            ),
            (
                "\
fn else_holds_if(a: bool, b: bool) -> i32 {
    if a {
        1
    } else {
        if b {
            2
        } else {
            0
        }
    }
}
",
                "else_holds_if",
                2,
                Some(5),
            ),
            (
                "\
fn guard(x: i32) -> i32 {
    match x {
        y if y > 0 => 1,
        _ => 0,
    }
}
",
                "guard",
                1,
                Some(2),
            ),
            (
                "\
fn arm_if(x: i32) -> i32 {
    match x {
        y => {
            if y > 0 {
                1
            } else {
                0
            }
        }
    }
}
",
                "arm_if",
                2,
                Some(4),
            ),
            (
                "\
fn tied(a: bool, b: bool) -> i32 {
    if a {
        if b { 1 } else { 0 }
    } else {
        if b { 2 } else { 0 }
    }
}
",
                "tied",
                2,
                Some(3),
            ),
            (
                "\
fn loops(xs: &[i32]) -> i32 {
    for x in xs {
        while *x > 0 {
            return *x;
        }
    }
    0
}
",
                "loops",
                2,
                Some(3),
            ),
            (
                "\
fn let_else(value: Option<i32>) -> i32 {
    let Some(found) = value else {
        if value.is_none() {
            return 0;
        }
        return 1;
    };
    found
}
",
                "let_else",
                1,
                Some(3),
            ),
        ];
        for (source, name, depth, line) in cases {
            assert_eq!(structural_nesting(source, name), (depth, line), "{name}");
        }
    }

    fn structural_nesting(source: &str, name: &str) -> (u32, Option<u32>) {
        let file = parse("nesting.rs", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0, "{name}\n{source}");
        let measured = crate::analyzers::measure(&file, &[]);
        let found = measured
            .iter()
            .find(|item| item.name == name)
            .unwrap_or_else(|| panic!("missing {name}"));
        (found.nesting, found.nesting_line)
    }

    fn function<'a>(file: &'a File, name: &str) -> &'a Function {
        file.functions
            .iter()
            .find(|function| function.name == name)
            .unwrap_or_else(|| panic!("missing {name}"))
    }

    #[test]
    fn a_macro_invocation_scores_on_the_enclosing_function() {
        let source = "\
fn check(a: i32) -> bool {
    dbg!(if a > 0 { true } else { false })
}

fn bits(a: i32) -> i32 {
    dbg!(a | 1)
}

fn asserted(a: bool, b: bool) {
    assert!(a && b);
}

lazy_static! {
    static ref FLAG: bool = if ready() { true } else { false };
}
";
        let file = parse("a.rs", source.as_bytes());
        let check = function(&file, "check");
        assert_eq!(check.body.decisions.len(), 1);
        assert_eq!(cognitive_total(&check.body), 2);
        assert_eq!(function(&file, "bits").body.decisions.len(), 0);
        assert_eq!(function(&file, "asserted").body.decisions.len(), 2);
        assert!(file
            .functions
            .iter()
            .all(|function| function.name != "lazy_static"));
    }

    #[test]
    fn else_if_match_and_parentheses_follow_the_walker() {
        let source = "\
fn branched(a: bool, b: bool) -> i32 {
    dbg!(if a { 1 } else if b { 2 } else { 3 })
}

fn guarded(a: i32) -> i32 {
    dbg!(match a { 1 if a > 0 => 1, _ => 0 })
}

fn grouped(a: bool, b: bool, c: bool) -> bool {
    dbg!((a && b) && c)
}
";
        let file = parse("a.rs", source.as_bytes());
        let branched = function(&file, "branched");
        assert_eq!(branched.body.decisions.len(), 2);
        assert_eq!(cognitive_total(&branched.body), 3);
        let guarded = function(&file, "guarded");
        assert_eq!(guarded.body.decisions.len(), 1);
        assert_eq!(cognitive_total(&guarded.body), 1);
        let grouped = function(&file, "grouped");
        assert_eq!(grouped.body.decisions.len(), 2);
        assert_eq!(cognitive_total(&grouped.body), 2);
    }

    #[test]
    fn a_dot_run_inside_a_macro_is_a_member_chain() {
        let source = "\
impl Repo {
    fn check(&self) {
        dbg!(self.a.b.c());
        let _ = vec![self.a.b, other.c.d];
    }
}
";
        let file = parse("a.rs", source.as_bytes());
        let check = function(&file, "Repo::check");
        assert_eq!(check.body.chains.len(), 3);
        assert!(check.body.chains[0].rooted);
        assert_eq!(member_names(&check.body.chains[0]), vec!["a", "b", "c"]);
        assert!(check.body.chains[0].members[2].call);
        assert_eq!(member_names(&check.body.chains[1]), vec!["a", "b"]);
        assert_eq!(member_names(&check.body.chains[2]), vec!["c", "d"]);
    }

    #[test]
    fn a_macro_rules_body_is_scored_where_it_is_written() {
        let source = "\
macro_rules! hide {
    ($e:expr) => { if $e > 0 { true } else { false } };
}

fn check(a: i32) -> bool {
    hide!(a)
}

fn outer() {
    macro_rules! local {
        () => { if ready() { 1 } else { 0 } };
    }
    wrap!(fn inner(a: i32) -> i32 {
        if a > 0 { 1 } else { 0 }
    });
}
";
        let file = parse("a.rs", source.as_bytes());
        let hide = function(&file, "hide");
        assert_eq!(hide.body.decisions.len(), 1);
        assert_eq!(cognitive_total(&hide.body), 2);
        assert_eq!(function(&file, "check").body.decisions.len(), 0);
        let outer = function(&file, "outer");
        assert_eq!(outer.body.decisions.len(), 1);
        assert_eq!(cognitive_total(&outer.body), 2);
        let inner = function(&file, "outer::inner");
        assert_eq!(inner.body.decisions.len(), 1);
        assert_eq!(cognitive_total(&inner.body), 2);
        assert!(!outer
            .body
            .decisions
            .iter()
            .any(|decision| decision.line == inner.body.decisions[0].line));
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

    #[test]
    fn a_swallowed_result_records_its_line() {
        let cases = [
            (
                "\
fn load() {
    let _ = read();
}
",
                "load",
                vec![2],
            ),
            (
                "\
fn load() {
    let _ = path.read();
}
",
                "load",
                vec![2],
            ),
            (
                "\
fn load() {
    read().ok();
}
",
                "load",
                vec![2],
            ),
            (
                "\
fn load() {
    path.read().ok();
}
",
                "load",
                vec![2],
            ),
            (
                "\
fn load(value: Result<i32, ()>) -> i32 {
    match value {
        Ok(n) => n,
        Err(_) => {},
    }
}
",
                "load",
                vec![4],
            ),
            (
                "\
fn load(value: Result<i32, i32>) -> i32 {
    match value {
        Ok(n) => n,
        Err(err) => (),
    }
}
",
                "load",
                vec![4],
            ),
            (
                "\
fn load(value: Result<i32, ()>) -> i32 {
    match value {
        Ok(n) => n,
        Err(_) => {
            ()
        },
    }
}
",
                "load",
                vec![4],
            ),
            (
                "\
fn load(value: Result<i32, ()>) {
    if let Err(_) = value {}
}
",
                "load",
                vec![2],
            ),
            (
                "\
fn load(value: Result<i32, ()>) {
    if let Err(err) = value {
        ()
    }
}
",
                "load",
                vec![2],
            ),
            (
                "\
fn load() {
    let _ = read();
    read().ok();
}
",
                "load",
                vec![2, 3],
            ),
            (
                "\
fn load() {
    let inner = || {
        let _ = read();
    };
}
",
                "load",
                vec![3],
            ),
        ];
        for (source, name, lines) in cases {
            let file = parse("a.rs", source.as_bytes());
            assert_eq!(file.unparsed_ratio, 0.0, "{source}");
            assert_eq!(function(&file, name).body.swallowing, lines, "{source}");
        }
    }

    #[test]
    fn a_handled_result_records_nothing() {
        let cases = [
            "\
fn load() {
    let _ = 1;
}
",
            "\
fn load() {
    let _ = read()?;
}
",
            "\
fn load() {
    let value = read();
}
",
            "\
fn load() {
    read().ok()?;
}
",
            "\
fn load() {
    let x = read().ok();
}
",
            "\
fn load() {
    read().ok().unwrap();
}
",
            "\
fn load() {
    read().ok(1);
}
",
            "\
fn load(value: Result<i32, i32>) -> i32 {
    match value {
        Ok(n) => n,
        Err(err) => err,
    }
}
",
            "\
fn load(value: Result<i32, ()>) -> i32 {
    match value {
        Ok(n) => n,
        Err(_) => {
            return 0;
        },
    }
}
",
            "\
fn load(value: Result<i32, ()>) {
    if let Err(err) = value {
        return;
    }
}
",
            "\
fn load(value: Result<i32, ()>) -> i32 {
    if let Ok(n) = value {
    }
    0
}
",
        ];
        for source in cases {
            let file = parse("a.rs", source.as_bytes());
            assert_eq!(file.unparsed_ratio, 0.0, "{source}");
            assert!(
                function(&file, "load").body.swallowing.is_empty(),
                "{source}"
            );
        }
    }

    #[test]
    fn a_swallowed_result_stays_on_the_function_that_writes_it() {
        let source = "\
fn outer() {
    fn inner() {
        let _ = read();
    }
}
";
        let file = parse("a.rs", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0);
        assert!(function(&file, "outer").body.swallowing.is_empty());
        assert_eq!(function(&file, "outer::inner").body.swallowing, vec![3]);
    }

    #[test]
    fn an_inline_open_stays_on_the_function_that_writes_it() {
        let source = "\
fn load(path: &str) {
    let _file = File::open(path);
    let _full = std::fs::File::open(path);
    fn inner() {
        TcpStream::connect(path);
    }
}
";
        let file = parse("a.rs", source.as_bytes());
        assert_eq!(file.unparsed_ratio, 0.0);
        let outer = &function(&file, "load").body.io;
        assert_eq!(outer.len(), 1, "{outer:?}");
        assert_eq!(outer[0].name, "File::open");
        assert_eq!(outer[0].line, 2);
        let inner = &function(&file, "load::inner").body.io;
        assert_eq!(inner.len(), 1, "{inner:?}");
        assert_eq!(inner[0].name, "TcpStream::connect");
        assert_eq!(inner[0].line, 5);
    }

    fn member_names(chain: &Chain) -> Vec<&str> {
        chain
            .members
            .iter()
            .map(|member| member.name.as_str())
            .collect()
    }
}
