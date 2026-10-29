use crate::ir::{Body, Chain, CognitiveKind, File, Function, Member, Span, WaiverLine};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainDepth {
    pub line: u32,
    pub depth: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Forwarding {
    pub line: u32,
    pub forwarded: u32,
}

/// A coverage fraction the report produced, or a report that never showed one.
#[derive(Clone, Debug, PartialEq)]
pub enum Coverage {
    Fraction { fraction: f64, line: u32 },
    Unshown,
}

/// A mutation score the report produced, or a report that never showed one.
#[derive(Clone, Debug, PartialEq)]
pub enum Mutation {
    Score { score: f64, line: u32 },
    Unshown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Measurement {
    pub path: String,
    pub name: String,
    pub param_types: Vec<String>,
    pub qualifier: String,
    pub span: Span,
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
    pub nesting_line: Option<u32>,
    pub params: u32,
    pub lloc: u32,
    pub receiver_chain: Option<ChainDepth>,
    pub other_chain: Option<ChainDepth>,
    pub forward: Option<Forwarding>,
    pub waivers: Vec<WaiverLine>,
    pub crap: Option<f64>,
    pub coverage: Option<Coverage>,
    pub mutation: Option<Mutation>,
}

pub fn measure(file: &File, allow: &[&str]) -> Vec<Measurement> {
    file.functions
        .iter()
        .map(|function| measure_function(&file.path, function, allow))
        .collect()
}

fn measure_function(path: &str, function: &Function, allow: &[&str]) -> Measurement {
    let (receiver_chain, other_chain) = chain_depths(&function.body, allow);
    Measurement {
        path: path.to_string(),
        name: function.name.clone(),
        param_types: function.param_types.clone(),
        qualifier: function.qualifier.clone(),
        span: function.span,
        cyclomatic: 1 + function.body.decisions.len() as u32,
        cognitive: cognitive(&function.body),
        nesting: nesting(&function.body),
        nesting_line: nesting_line(&function.body),
        params: parameter_count(function),
        lloc: function.body.lloc,
        receiver_chain,
        other_chain,
        forward: function.body.forward.as_ref().map(|forward| Forwarding {
            line: forward.line,
            forwarded: forward.names.len() as u32,
        }),
        waivers: function.waivers.clone(),
        crap: None,
        coverage: None,
        mutation: None,
    }
}

fn cognitive(body: &Body) -> u32 {
    body.cognitive
        .iter()
        .map(|increment| match increment.kind {
            CognitiveKind::Structural => 1 + increment.depth,
            CognitiveKind::Hybrid | CognitiveKind::Fundamental => 1,
        })
        .sum()
}

fn nesting(body: &Body) -> u32 {
    body.levels
        .iter()
        .map(|level| level.depth)
        .max()
        .unwrap_or(0)
}

fn nesting_line(body: &Body) -> Option<u32> {
    let depth = nesting(body);
    if depth == 0 {
        return None;
    }
    body.levels
        .iter()
        .filter(|level| level.depth == depth)
        .map(|level| level.line)
        .min()
}

fn parameter_count(function: &Function) -> u32 {
    function
        .params
        .iter()
        .filter(|param| !param.receiver)
        .count() as u32
}

fn chain_depths(body: &Body, allow: &[&str]) -> (Option<ChainDepth>, Option<ChainDepth>) {
    let mut receiver = None;
    let mut other = None;
    for chain in &body.chains {
        let depth = chain_depth(chain, allow);
        let slot = if chain.rooted {
            &mut receiver
        } else {
            &mut other
        };
        keep_deeper(slot, chain.line, depth);
    }
    (receiver, other)
}

fn chain_depth(chain: &Chain, allow: &[&str]) -> u32 {
    hops_before_build_run(&chain.members)
        .iter()
        .filter(|member| !allow.contains(&member.name.as_str()))
        .count() as u32
}

fn hops_before_build_run(members: &[Member]) -> &[Member] {
    let Some(last) = members.last() else {
        return members;
    };
    if !(last.call && last.name == "build") {
        return members;
    }
    let mut end = members.len();
    while end > 0 && members[end - 1].call {
        end -= 1;
    }
    &members[..end]
}

fn keep_deeper(slot: &mut Option<ChainDepth>, line: u32, depth: u32) {
    if depth == 0 {
        return;
    }
    let replace = match slot {
        None => true,
        Some(current) => depth > current.depth || (depth == current.depth && line < current.line),
    };
    if replace {
        *slot = Some(ChainDepth { line, depth });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{
        Body, Chain, Cognitive, CognitiveKind, Decision, File, Forward, Function, Language, Member,
        Param, Span,
    };

    fn param(name: &str, receiver: bool) -> Param {
        Param {
            name: name.to_string(),
            receiver,
        }
    }

    fn hop(name: &str, call: bool) -> Member {
        Member {
            name: name.to_string(),
            call,
        }
    }

    fn chain(line: u32, rooted: bool, hops: &[(&str, bool)]) -> Chain {
        Chain {
            line,
            rooted,
            members: hops.iter().map(|(name, call)| hop(name, *call)).collect(),
        }
    }

    fn function(name: &str, params: Vec<Param>, body: Body) -> Function {
        Function {
            name: name.to_string(),
            param_types: vec![],
            qualifier: String::new(),
            span: Span { start: 10, end: 20 },
            touch: Span { start: 8, end: 20 },
            params,
            body,
            waivers: vec![],
        }
    }

    fn file(functions: Vec<Function>) -> File {
        File {
            path: "src/paint.py".to_string(),
            language: Language::Python,
            unparsed_ratio: 0.0,
            unscored_macros: 0,
            functions,
        }
    }

    fn empty() -> Body {
        Body {
            lloc: 0,
            decisions: vec![],
            levels: vec![],
            cognitive: vec![],
            chains: vec![],
            forward: None,
        }
    }

    #[test]
    fn an_empty_body_is_one_path() {
        let measured = measure(&file(vec![function("paint", vec![], empty())]), &[]);
        let measurement = &measured[0];

        assert_eq!(measurement.cyclomatic, 1);
        assert_eq!(measurement.cognitive, 0);
        assert_eq!(measurement.nesting, 0);
        assert_eq!(measurement.params, 0);
        assert_eq!(measurement.receiver_chain, None);
        assert_eq!(measurement.other_chain, None);
        assert_eq!(measurement.forward, None);
    }

    #[test]
    fn decisions_add_a_path_and_nesting_is_the_deepest_level() {
        let mut body = empty();
        body.decisions = vec![Decision { line: 12, depth: 9 }];
        body.levels = vec![
            Decision { line: 14, depth: 3 },
            Decision { line: 15, depth: 2 },
            Decision { line: 11, depth: 3 },
        ];
        let measured = measure(&file(vec![function("choose", vec![], body)]), &[]);

        assert_eq!(measured[0].cyclomatic, 2);
        assert_eq!(measured[0].nesting, 3);
        assert_eq!(measured[0].nesting_line, Some(11));
    }

    #[test]
    fn cognitive_complexity_sums_each_kind() {
        let mut body = empty();
        body.cognitive = vec![
            Cognitive {
                line: 12,
                kind: CognitiveKind::Structural,
                depth: 0,
            },
            Cognitive {
                line: 14,
                kind: CognitiveKind::Structural,
                depth: 2,
            },
            Cognitive {
                line: 13,
                kind: CognitiveKind::Hybrid,
                depth: 9,
            },
            Cognitive {
                line: 15,
                kind: CognitiveKind::Fundamental,
                depth: 9,
            },
        ];
        let measured = measure(&file(vec![function("choose", vec![], body)]), &[]);

        assert_eq!(measured[0].cognitive, 6);
    }

    #[test]
    fn parameter_count_leaves_out_the_receiver() {
        let measured = measure(
            &file(vec![function(
                "save",
                vec![
                    param("self", true),
                    param("user", false),
                    param("flags", false),
                ],
                empty(),
            )]),
            &[],
        );

        assert_eq!(measured[0].params, 2);
    }

    fn with_chain(name: &str, rooted: bool, hops: &[(&str, bool)]) -> Function {
        let mut body = empty();
        body.chains = vec![chain(4, rooted, hops)];
        function(name, vec![], body)
    }

    #[test]
    fn chain_depth_drops_listed_names_and_a_trailing_build() {
        let measured = measure(
            &file(vec![
                with_chain(
                    "counted",
                    true,
                    &[("repo", false), ("save", true), ("user", false)],
                ),
                with_chain("listed", false, &[("len", true), ("min", true)]),
                with_chain("gone", false, &[("iter", true), ("zip", true)]),
                with_chain(
                    "fluent",
                    true,
                    &[("with_name", true), ("with_value", true), ("build", true)],
                ),
                with_chain("suffix", false, &[("field", false), ("build", true)]),
                with_chain("middle", false, &[("build", true), ("extra", true)]),
                with_chain("capital", false, &[("field", false), ("Build", true)]),
            ]),
            &["min", "iter", "zip", "Min"],
        );

        assert_eq!(
            measured[0].receiver_chain,
            Some(ChainDepth { line: 4, depth: 3 })
        );
        assert_eq!(measured[0].other_chain, None);
        assert_eq!(
            measured[1].other_chain,
            Some(ChainDepth { line: 4, depth: 1 })
        );
        assert_eq!(measured[2].receiver_chain, None);
        assert_eq!(measured[2].other_chain, None);
        assert_eq!(measured[3].receiver_chain, None);
        assert_eq!(
            measured[4].other_chain,
            Some(ChainDepth { line: 4, depth: 1 })
        );
        assert_eq!(
            measured[5].other_chain,
            Some(ChainDepth { line: 4, depth: 2 })
        );
        assert_eq!(
            measured[6].other_chain,
            Some(ChainDepth { line: 4, depth: 2 })
        );
    }

    #[test]
    fn the_deeper_chain_wins_and_a_tie_keeps_the_earlier_line() {
        let mut body = empty();
        body.chains = vec![
            chain(20, false, &[("a", false), ("b", false)]),
            chain(15, false, &[("c", false), ("d", false)]),
            chain(30, false, &[("e", false)]),
            chain(10, true, &[("a", false)]),
            chain(40, true, &[("b", false), ("c", false), ("d", false)]),
        ];
        let measured = measure(&file(vec![function("paint", vec![], body)]), &[]);

        assert_eq!(
            measured[0].other_chain,
            Some(ChainDepth { line: 15, depth: 2 })
        );
        assert_eq!(
            measured[0].receiver_chain,
            Some(ChainDepth { line: 40, depth: 3 })
        );
    }

    #[test]
    fn the_forward_ratio_counts_handed_off_names() {
        let mut forwarded = empty();
        forwarded.forward = Some(Forward {
            line: 87,
            names: vec!["url".to_string(), "kwargs".to_string()],
        });
        let mut missed = empty();
        missed.forward = Some(Forward {
            line: 3,
            names: vec![],
        });
        let measured = measure(
            &file(vec![
                function(
                    "get",
                    vec![
                        param("self", true),
                        param("url", false),
                        param("params", false),
                        param("kwargs", false),
                    ],
                    forwarded,
                ),
                function("head", vec![param("url", false)], empty()),
                function("miss", vec![param("url", false)], missed),
            ]),
            &[],
        );

        assert_eq!(measured[0].params, 3);
        assert_eq!(
            measured[0].forward,
            Some(Forwarding {
                line: 87,
                forwarded: 2,
            })
        );
        assert_eq!(measured[1].params, 1);
        assert_eq!(measured[1].forward, None);
        assert_eq!(measured[2].params, 1);
        assert_eq!(
            measured[2].forward,
            Some(Forwarding {
                line: 3,
                forwarded: 0,
            })
        );
    }

    #[test]
    fn measurements_follow_the_functions_and_keep_the_symbol() {
        let mut join = function("CLI::detail::join", vec![param("parts", false)], empty());
        join.param_types = vec!["std::string &".to_string()];
        join.qualifier = "const".to_string();
        join.span = Span { start: 40, end: 55 };
        let mut other = function("trim", vec![], empty());
        other.span = Span { start: 60, end: 70 };
        let mut source = file(vec![join, other]);
        source.path = "include/CLI/StringTools.hpp".to_string();
        source.language = Language::Cpp;

        let measured = measure(&source, &[]);

        assert_eq!(measured.len(), 2);
        assert_eq!(measured[0].path, "include/CLI/StringTools.hpp");
        assert_eq!(measured[0].name, "CLI::detail::join");
        assert_eq!(measured[0].param_types, vec!["std::string &".to_string()]);
        assert_eq!(measured[0].qualifier, "const");
        assert_eq!(measured[0].span, Span { start: 40, end: 55 });
        assert_eq!(measured[1].name, "trim");
        assert_eq!(measured[1].span, Span { start: 60, end: 70 });
    }

    #[test]
    fn the_measurement_keeps_lines_nesting_and_waivers() {
        let mut body = empty();
        body.lloc = 4;
        body.levels = vec![
            Decision { line: 14, depth: 3 },
            Decision { line: 11, depth: 3 },
            Decision { line: 12, depth: 1 },
        ];
        let mut function = function("choose", vec![], body);
        function.waivers = vec![crate::ir::WaiverLine {
            line: 4,
            rule: Some("F01".to_string()),
            number: Some("9".to_string()),
            reason: Some("the branch table is the point".to_string()),
        }];

        let measured = measure(&file(vec![function]), &[]);

        assert_eq!(measured[0].lloc, 4);
        assert_eq!(measured[0].nesting_line, Some(11));
        assert_eq!(measured[0].waivers.len(), 1);
        assert_eq!(measured[0].waivers[0].line, 4);
        assert_eq!(measured[0].crap, None);
    }
}
