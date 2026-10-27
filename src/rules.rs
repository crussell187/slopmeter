use std::collections::{BTreeMap, BTreeSet};

use crate::analyzers::{ChainDepth, Measurement};
use crate::inputs::LanguageProfile;
use crate::ir::{Span, WaiverLine};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Gate,
    Warn,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Waiver {
    pub excused: Option<f64>,
    pub reason: Option<String>,
    pub holds: bool,
    pub rejection: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub rule: String,
    pub level: Level,
    pub path: String,
    pub name: String,
    pub param_types: Vec<String>,
    pub qualifier: String,
    pub line: u32,
    pub measured_value: Option<f64>,
    pub threshold: Option<f64>,
    pub weight: u32,
    pub principle: String,
    pub fix_shape: String,
    pub anti_fix: String,
    pub waiver: Option<Waiver>,
}

pub fn evaluate(measurements: &[Measurement], profile: &LanguageProfile) -> Vec<Finding> {
    measurements
        .iter()
        .flat_map(|measurement| evaluate_one(measurement, profile))
        .collect()
}

struct Breach {
    rule: &'static str,
    level: Level,
    line: u32,
    measured: f64,
    threshold: f64,
}

struct Hit {
    line: u32,
    depth: u32,
    limit: u32,
}

enum Outcome {
    Accepted(f64),
    Rejected(String),
}

struct Classified {
    line: u32,
    rule: Option<String>,
    reason: Option<String>,
    outcome: Outcome,
}

impl Classified {
    fn names(&self, rule: &str) -> bool {
        self.rule.as_deref() == Some(rule)
    }
}

fn evaluate_one(measurement: &Measurement, profile: &LanguageProfile) -> Vec<Finding> {
    let classes = classify(&measurement.waivers);
    let mut findings = Vec::new();
    let mut gated = BTreeSet::new();
    for breach in breaches(measurement, profile) {
        let rejected = classes
            .iter()
            .any(|class| class.names(breach.rule) && matches!(class.outcome, Outcome::Rejected(_)));
        if rejected && breach.level == Level::Warn {
            continue;
        }
        if breach.level == Level::Gate {
            gated.insert(breach.rule);
        }
        findings.push(metric_finding(
            measurement,
            &breach,
            attach(&classes, breach.rule, breach.measured, breach.level),
        ));
    }
    for class in &classes {
        let Outcome::Rejected(message) = &class.outcome else {
            continue;
        };
        if class
            .rule
            .as_deref()
            .is_some_and(|rule| gated.contains(rule))
        {
            continue;
        }
        findings.push(rejection_finding(measurement, profile, class, message));
    }
    findings
}

fn breaches(measurement: &Measurement, profile: &LanguageProfile) -> Vec<Breach> {
    let mut breaches = Vec::new();
    let site = measurement.span.start;
    if measurement.cyclomatic > profile.f01_gate {
        breaches.push(breach(
            "F01",
            Level::Gate,
            site,
            measurement.cyclomatic,
            profile.f01_gate,
        ));
    }
    if let Some(crap) = measurement.crap {
        if crap > f64::from(profile.f02_gate) {
            breaches.push(Breach {
                rule: "F02",
                level: Level::Gate,
                line: site,
                measured: crap,
                threshold: f64::from(profile.f02_gate),
            });
        }
    }
    if measurement.cognitive > profile.f03_gate {
        breaches.push(breach(
            "F03",
            Level::Gate,
            site,
            measurement.cognitive,
            profile.f03_gate,
        ));
    } else if measurement.cognitive > profile.f03_warn {
        breaches.push(breach(
            "F03",
            Level::Warn,
            site,
            measurement.cognitive,
            profile.f03_warn,
        ));
    }
    if measurement.params > profile.f04_gate {
        breaches.push(breach(
            "F04",
            Level::Gate,
            site,
            measurement.params,
            profile.f04_gate,
        ));
    }
    if measurement.nesting > profile.f05_gate {
        breaches.push(breach(
            "F05",
            Level::Gate,
            measurement.nesting_line.unwrap_or(site),
            measurement.nesting,
            profile.f05_gate,
        ));
    }
    let lines = body_lines(measurement.span);
    if lines > profile.f06_gate {
        breaches.push(breach("F06", Level::Gate, site, lines, profile.f06_gate));
    }
    if let Some(ratio) = percent(measurement) {
        if ratio >= profile.f07_percent {
            let line = measurement
                .forward
                .as_ref()
                .map(|forward| forward.line)
                .unwrap_or(site);
            breaches.push(breach("F07", Level::Warn, line, ratio, profile.f07_percent));
        }
    }
    if let Some(hit) = chain_hit(measurement, profile, true) {
        breaches.push(Breach {
            rule: "F08",
            level: Level::Gate,
            line: hit.line,
            measured: f64::from(hit.depth),
            threshold: f64::from(hit.limit),
        });
    }
    if measurement.lloc > profile.f11_gate {
        breaches.push(breach(
            "F11",
            Level::Gate,
            site,
            measurement.lloc,
            profile.f11_gate,
        ));
    }
    breaches
}

fn breach(rule: &'static str, level: Level, line: u32, measured: u32, threshold: u32) -> Breach {
    Breach {
        rule,
        level,
        line,
        measured: f64::from(measured),
        threshold: f64::from(threshold),
    }
}

fn body_lines(span: Span) -> u32 {
    span.end.saturating_sub(span.start).saturating_add(1)
}

fn percent(measurement: &Measurement) -> Option<u32> {
    let forward = measurement.forward.as_ref()?;
    if measurement.params == 0 {
        return None;
    }
    Some(forward.forwarded.saturating_mul(100) / measurement.params)
}

fn chain_hit(measurement: &Measurement, profile: &LanguageProfile, over_only: bool) -> Option<Hit> {
    let mut hits = Vec::new();
    push_hit(
        &mut hits,
        measurement.receiver_chain.as_ref(),
        profile.f08_receiver_hops,
        over_only,
    );
    push_hit(
        &mut hits,
        measurement.other_chain.as_ref(),
        profile.f08_other_hops,
        over_only,
    );
    hits.into_iter().reduce(worse_chain)
}

fn push_hit(hits: &mut Vec<Hit>, chain: Option<&ChainDepth>, limit: u32, over_only: bool) {
    let Some(chain) = chain else {
        return;
    };
    if over_only && chain.depth <= limit {
        return;
    }
    hits.push(Hit {
        line: chain.line,
        depth: chain.depth,
        limit,
    });
}

fn worse_chain(left: Hit, right: Hit) -> Hit {
    let left_score = i64::from(left.depth) - i64::from(left.limit);
    let right_score = i64::from(right.depth) - i64::from(right.limit);
    let left_cross = left_score * i64::from(right.limit);
    let right_cross = right_score * i64::from(left.limit);
    if left_cross > right_cross || (left_cross == right_cross && left.line <= right.line) {
        left
    } else {
        right
    }
}

fn metric_finding(measurement: &Measurement, breach: &Breach, waiver: Option<Waiver>) -> Finding {
    Finding {
        rule: breach.rule.to_string(),
        level: breach.level,
        path: measurement.path.clone(),
        name: measurement.name.clone(),
        param_types: measurement.param_types.clone(),
        qualifier: measurement.qualifier.clone(),
        line: breach.line,
        measured_value: Some(breach.measured),
        threshold: Some(breach.threshold),
        weight: 0,
        principle: principle(breach.rule),
        fix_shape: fix_shape(breach, &measurement.name),
        anti_fix: anti_fix(breach.rule),
        waiver,
    }
}

fn rejection_finding(
    measurement: &Measurement,
    profile: &LanguageProfile,
    class: &Classified,
    message: &str,
) -> Finding {
    let rule = class.rule.clone().unwrap_or_default();
    let (measured_value, threshold) = rejection_numbers(measurement, profile, &rule);
    Finding {
        rule,
        level: Level::Gate,
        path: measurement.path.clone(),
        name: measurement.name.clone(),
        param_types: measurement.param_types.clone(),
        qualifier: measurement.qualifier.clone(),
        line: class.line,
        measured_value,
        threshold,
        weight: 0,
        principle: principle(class.rule.as_deref().unwrap_or("")),
        fix_shape: format!(
            "Correct the waiver at line {} of {}. {message}",
            class.line, measurement.name
        ),
        anti_fix: anti_fix(class.rule.as_deref().unwrap_or("")),
        waiver: Some(Waiver {
            excused: None,
            reason: class.reason.clone(),
            holds: false,
            rejection: Some(message.to_string()),
        }),
    }
}

fn rejection_numbers(
    measurement: &Measurement,
    profile: &LanguageProfile,
    rule: &str,
) -> (Option<f64>, Option<f64>) {
    match rule {
        "F01" => (
            Some(f64::from(measurement.cyclomatic)),
            Some(f64::from(profile.f01_gate)),
        ),
        "F02" => (measurement.crap, Some(f64::from(profile.f02_gate))),
        "F03" => (
            Some(f64::from(measurement.cognitive)),
            Some(f64::from(profile.f03_gate)),
        ),
        "F04" => (
            Some(f64::from(measurement.params)),
            Some(f64::from(profile.f04_gate)),
        ),
        "F05" => (
            Some(f64::from(measurement.nesting)),
            Some(f64::from(profile.f05_gate)),
        ),
        "F06" => (
            Some(f64::from(body_lines(measurement.span))),
            Some(f64::from(profile.f06_gate)),
        ),
        "F07" => (percent(measurement).map(f64::from), None),
        "F08" => chain_hit(measurement, profile, false).map_or((None, None), |hit| {
            (Some(f64::from(hit.depth)), Some(f64::from(hit.limit)))
        }),
        "F11" => (
            Some(f64::from(measurement.lloc)),
            Some(f64::from(profile.f11_gate)),
        ),
        _ => (None, None),
    }
}

fn attach(classes: &[Classified], rule: &str, measured: f64, level: Level) -> Option<Waiver> {
    let class = classes.iter().find(|class| class.names(rule))?;
    match &class.outcome {
        Outcome::Rejected(message) => Some(Waiver {
            excused: None,
            reason: class.reason.clone(),
            holds: false,
            rejection: Some(message.clone()),
        }),
        Outcome::Accepted(number) => Some(Waiver {
            excused: Some(*number),
            reason: class.reason.clone(),
            holds: level == Level::Gate && measured <= *number,
            rejection: None,
        }),
    }
}

fn classify(lines: &[WaiverLine]) -> Vec<Classified> {
    let mut counts = BTreeMap::<String, usize>::new();
    for line in lines {
        if let Some(rule) = &line.rule {
            *counts.entry(rule.clone()).or_insert(0) += 1;
        }
    }
    lines
        .iter()
        .map(|line| classify_line(line, &counts))
        .collect()
}

fn classify_line(line: &WaiverLine, counts: &BTreeMap<String, usize>) -> Classified {
    let rule = line.rule.clone();
    let reason = line.reason.clone();
    let outcome = match rule.as_deref() {
        None => Outcome::Rejected("The waiver line has no rule.".to_string()),
        Some(rule) if counts.get(rule).copied().unwrap_or(0) > 1 && gate_id(rule) => {
            Outcome::Rejected(format!("{rule} is waived twice."))
        }
        Some(rule) if !gate_id(rule) => Outcome::Rejected(format!("{rule} is not a gate.")),
        Some(rule) => number_outcome(rule, line.number.as_deref(), reason.is_some()),
    };
    Classified {
        line: line.line,
        rule,
        reason,
        outcome,
    }
}

fn number_outcome(rule: &str, number: Option<&str>, has_reason: bool) -> Outcome {
    let Some(token) = number else {
        return Outcome::Rejected("The waiver line has no number.".to_string());
    };
    if rule != "F02" && token.contains('.') {
        return Outcome::Rejected(format!("{rule} takes a whole number."));
    }
    if !has_reason {
        return Outcome::Rejected("The waiver line has no reason.".to_string());
    }
    match token.parse::<f64>() {
        Ok(number) => Outcome::Accepted(number),
        Err(_) => Outcome::Rejected("The waiver line has no number.".to_string()),
    }
}

fn gate_id(rule: &str) -> bool {
    matches!(
        rule,
        "F01" | "F02" | "F03" | "F04" | "F05" | "F06" | "F08" | "F11"
    )
}

fn principle(rule: &str) -> String {
    match rule {
        "F01" => "A function should have few independent paths. McCabe.",
        "F02" => "Untested complexity is where defects hide. Change Risk Anti-Patterns.",
        "F03" => "Nested structure costs more to understand than a path count. SonarSource Cognitive Complexity.",
        "F04" => "A function with many ordinary parameters is doing more than one job.",
        "F05" => "A reader should not hold three decisions at once.",
        "F06" => "A function's body should stay short enough to read at once.",
        "F07" => "A function that only hands its parameters to one call adds a hop and no behavior.",
        "F08" => "A member chain that does not start at the receiver may be one deep, and a chain that starts at the receiver may be two deep. Law of Demeter.",
        "F11" => "A function should say little. The count is statements, not wrapped lines.",
        _ => "A waiver excuses one gate, and this line does not name one.",
    }
    .to_string()
}

fn anti_fix(rule: &str) -> String {
    match rule {
        "F01" => "Splitting one condition onto two lines leaves the path count in place.",
        "F02" => "Extracting an uncovered branch leaves that branch's score in place.",
        "F03" => "Adding a comment inside the nested block leaves the score in place.",
        "F04" => "Giving a parameter a default leaves the count in place.",
        "F05" => "Indenting the inner condition less leaves the depth in place.",
        "F06" => "Breaking one statement across more lines makes the count larger.",
        "F07" => "Renaming the forwarded parameters leaves the ratio in place.",
        "F08" => "Saving the chain in locals leaves the finding in place.",
        "F11" => "Breaking one statement across more lines leaves the count in place.",
        _ => "Deleting the waiver line leaves the run with no such line.",
    }
    .to_string()
}

fn fix_shape(breach: &Breach, name: &str) -> String {
    let measured = breach.measured.to_string();
    let threshold = breach.threshold.to_string();
    match breach.rule {
        "F01" => format!(
            "Split {name}. Its cyclomatic complexity is {measured}, and the gate is {threshold}."
        ),
        "F02" => {
            format!("Cover or split {name}. Its CRAP is {measured}, and the gate is {threshold}.")
        }
        "F03" => {
            let band = match breach.level {
                Level::Gate => "gate",
                Level::Warn => "warn",
            };
            format!(
                "Simplify {name}. Its cognitive complexity is {measured}, and the {band} is {threshold}."
            )
        }
        "F04" => format!(
            "Group the parameters of {name}. It has {measured} ordinary parameters, and the gate is {threshold}."
        ),
        "F05" => format!(
            "Flatten the decision at line {} in {name}. Nesting is {measured}, and the gate is {threshold}.",
            breach.line
        ),
        "F06" => {
            format!("Shorten {name}. Its body is {measured} lines, and the gate is {threshold}.")
        }
        "F07" => format!(
            "Inline {name}. It forwards {measured} percent of its ordinary parameters, and the limit is {threshold}."
        ),
        "F08" => format!(
            "Break the chain at line {} in {name}. It is {measured} deep, and the limit is {threshold}.",
            breach.line
        ),
        "F11" => {
            format!("Split {name}. It has {measured} logical lines, and the gate is {threshold}.")
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::{ChainDepth, Forwarding, Measurement};
    use crate::inputs::{Config, LanguageProfile};
    use crate::ir::{Span, WaiverLine};

    fn profile() -> LanguageProfile {
        Config::builtin().python
    }

    fn base() -> Measurement {
        Measurement {
            path: "src/paint.py".to_string(),
            name: "paint".to_string(),
            param_types: vec![],
            qualifier: String::new(),
            span: Span { start: 10, end: 12 },
            cyclomatic: 1,
            cognitive: 0,
            nesting: 0,
            nesting_line: None,
            params: 0,
            lloc: 1,
            receiver_chain: None,
            other_chain: None,
            forward: None,
            waivers: vec![],
            crap: None,
        }
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

    fn by_rule<'a>(findings: &'a [Finding], rule: &str) -> &'a Finding {
        findings
            .iter()
            .find(|finding| finding.rule == rule)
            .unwrap()
    }

    #[test]
    fn a_value_at_the_gate_emits_nothing() {
        let mut quiet = base();
        quiet.cyclomatic = 8;
        quiet.cognitive = 10;
        quiet.params = 3;
        quiet.nesting = 2;
        quiet.nesting_line = Some(11);
        quiet.span = Span { start: 1, end: 19 };
        quiet.lloc = 19;
        quiet.receiver_chain = Some(ChainDepth { line: 4, depth: 2 });
        quiet.other_chain = Some(ChainDepth { line: 5, depth: 1 });
        quiet.forward = Some(Forwarding {
            line: 6,
            forwarded: 2,
        });
        quiet.crap = Some(8.0);

        assert!(evaluate(&[quiet], &profile()).is_empty());
    }

    #[test]
    fn a_value_over_the_gate_emits_one_finding_at_the_worst_site() {
        let mut hot = base();
        hot.name = "classify".to_string();
        hot.param_types = vec!["int".to_string()];
        hot.qualifier = "const".to_string();
        hot.span = Span { start: 30, end: 50 };
        hot.cyclomatic = 9;
        hot.cognitive = 16;
        hot.nesting = 3;
        hot.nesting_line = Some(44);
        hot.params = 4;
        hot.lloc = 20;
        hot.receiver_chain = Some(ChainDepth { line: 40, depth: 3 });
        hot.other_chain = Some(ChainDepth { line: 20, depth: 2 });
        hot.forward = Some(Forwarding {
            line: 47,
            forwarded: 4,
        });
        hot.crap = Some(8.5);

        let findings = evaluate(&[hot], &profile());
        let rules: Vec<_> = findings
            .iter()
            .map(|finding| finding.rule.as_str())
            .collect();
        assert_eq!(
            rules,
            vec!["F01", "F02", "F03", "F04", "F05", "F06", "F07", "F08", "F11"]
        );

        let f01 = &findings[0];
        assert_eq!(f01.level, Level::Gate);
        assert_eq!(f01.path, "src/paint.py");
        assert_eq!(f01.name, "classify");
        assert_eq!(f01.param_types, vec!["int".to_string()]);
        assert_eq!(f01.qualifier, "const");
        assert_eq!(f01.line, 30);
        assert_eq!(f01.measured_value, Some(9.0));
        assert_eq!(f01.threshold, Some(8.0));
        assert_eq!(f01.weight, 0);
        assert_eq!(
            f01.principle,
            "A function should have few independent paths. McCabe."
        );
        assert_eq!(
            f01.fix_shape,
            "Split classify. Its cyclomatic complexity is 9, and the gate is 8."
        );
        assert_eq!(
            f01.anti_fix,
            "Splitting one condition onto two lines leaves the path count in place."
        );
        assert!(f01.waiver.is_none());

        let f02 = &findings[1];
        assert_eq!(f02.line, 30);
        assert_eq!(f02.measured_value, Some(8.5));
        assert_eq!(f02.threshold, Some(8.0));
        assert_eq!(
            f02.principle,
            "Untested complexity is where defects hide. Change Risk Anti-Patterns."
        );
        assert_eq!(
            f02.fix_shape,
            "Cover or split classify. Its CRAP is 8.5, and the gate is 8."
        );
        assert_eq!(
            f02.anti_fix,
            "Extracting an uncovered branch leaves that branch's score in place."
        );

        let f03 = &findings[2];
        assert_eq!(f03.level, Level::Gate);
        assert_eq!(f03.line, 30);
        assert_eq!(f03.measured_value, Some(16.0));
        assert_eq!(f03.threshold, Some(15.0));
        assert_eq!(
            f03.principle,
            "Nested structure costs more to understand than a path count. SonarSource Cognitive Complexity."
        );
        assert_eq!(
            f03.fix_shape,
            "Simplify classify. Its cognitive complexity is 16, and the gate is 15."
        );
        assert_eq!(
            f03.anti_fix,
            "Adding a comment inside the nested block leaves the score in place."
        );

        let f04 = &findings[3];
        assert_eq!(f04.line, 30);
        assert_eq!(f04.measured_value, Some(4.0));
        assert_eq!(f04.threshold, Some(3.0));
        assert_eq!(
            f04.principle,
            "A function with many ordinary parameters is doing more than one job."
        );
        assert_eq!(
            f04.fix_shape,
            "Group the parameters of classify. It has 4 ordinary parameters, and the gate is 3."
        );
        assert_eq!(
            f04.anti_fix,
            "Giving a parameter a default leaves the count in place."
        );

        let f05 = &findings[4];
        assert_eq!(f05.line, 44);
        assert_eq!(f05.measured_value, Some(3.0));
        assert_eq!(f05.threshold, Some(2.0));
        assert_eq!(
            f05.principle,
            "A reader should not hold three decisions at once."
        );
        assert_eq!(
            f05.fix_shape,
            "Flatten the decision at line 44 in classify. Nesting is 3, and the gate is 2."
        );
        assert_eq!(
            f05.anti_fix,
            "Indenting the inner condition less leaves the depth in place."
        );

        let f06 = &findings[5];
        assert_eq!(f06.line, 30);
        assert_eq!(f06.measured_value, Some(21.0));
        assert_eq!(f06.threshold, Some(19.0));
        assert_eq!(
            f06.principle,
            "A function's body should stay short enough to read at once."
        );
        assert_eq!(
            f06.fix_shape,
            "Shorten classify. Its body is 21 lines, and the gate is 19."
        );
        assert_eq!(
            f06.anti_fix,
            "Breaking one statement across more lines makes the count larger."
        );

        let f07 = &findings[6];
        assert_eq!(f07.level, Level::Warn);
        assert_eq!(f07.line, 47);
        assert_eq!(f07.measured_value, Some(100.0));
        assert_eq!(f07.threshold, Some(80.0));
        assert_eq!(
            f07.principle,
            "A function that only hands its parameters to one call adds a hop and no behavior."
        );
        assert_eq!(
            f07.fix_shape,
            "Inline classify. It forwards 100 percent of its ordinary parameters, and the limit is 80."
        );
        assert_eq!(
            f07.anti_fix,
            "Renaming the forwarded parameters leaves the ratio in place."
        );

        let f08 = &findings[7];
        assert_eq!(f08.level, Level::Gate);
        assert_eq!(f08.line, 20);
        assert_eq!(f08.measured_value, Some(2.0));
        assert_eq!(f08.threshold, Some(1.0));
        assert_eq!(
            f08.principle,
            "A member chain that does not start at the receiver may be one deep, and a chain that starts at the receiver may be two deep. Law of Demeter."
        );
        assert_eq!(
            f08.fix_shape,
            "Break the chain at line 20 in classify. It is 2 deep, and the limit is 1."
        );
        assert_eq!(
            f08.anti_fix,
            "Saving the chain in locals leaves the finding in place."
        );

        let f11 = &findings[8];
        assert_eq!(f11.line, 30);
        assert_eq!(f11.measured_value, Some(20.0));
        assert_eq!(f11.threshold, Some(19.0));
        assert_eq!(
            f11.principle,
            "A function should say little. The count is statements, not wrapped lines."
        );
        assert_eq!(
            f11.fix_shape,
            "Split classify. It has 20 logical lines, and the gate is 19."
        );
        assert_eq!(
            f11.anti_fix,
            "Breaking one statement across more lines leaves the count in place."
        );
    }

    #[test]
    fn cognitive_complexity_warns_between_the_levels() {
        let mut warn = base();
        warn.cognitive = 11;
        let mut still_warn = base();
        still_warn.name = "edge".to_string();
        still_warn.cognitive = 15;
        let mut gate = base();
        gate.name = "deep".to_string();
        gate.cognitive = 16;

        let findings = evaluate(&[warn, still_warn, gate], &profile());

        assert_eq!(findings.len(), 3);
        assert_eq!(findings[0].level, Level::Warn);
        assert_eq!(findings[0].threshold, Some(10.0));
        assert_eq!(findings[0].measured_value, Some(11.0));
        assert_eq!(
            findings[0].fix_shape,
            "Simplify paint. Its cognitive complexity is 11, and the warn is 10."
        );
        assert_eq!(findings[1].name, "edge");
        assert_eq!(findings[1].level, Level::Warn);
        assert_eq!(findings[1].threshold, Some(10.0));
        assert_eq!(findings[2].name, "deep");
        assert_eq!(findings[2].level, Level::Gate);
        assert_eq!(findings[2].threshold, Some(15.0));
    }

    #[test]
    fn an_equal_chain_fraction_keeps_the_earlier_line() {
        let mut tied = base();
        tied.receiver_chain = Some(ChainDepth { line: 30, depth: 4 });
        tied.other_chain = Some(ChainDepth { line: 12, depth: 2 });

        let findings = evaluate(&[tied], &profile());

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, "F08");
        assert_eq!(findings[0].line, 12);
        assert_eq!(findings[0].measured_value, Some(2.0));
        assert_eq!(findings[0].threshold, Some(1.0));
    }

    #[test]
    fn a_forward_with_no_ordinary_parameters_emits_nothing() {
        let mut forwarded = base();
        forwarded.forward = Some(Forwarding {
            line: 4,
            forwarded: 1,
        });

        assert!(evaluate(&[forwarded], &profile()).is_empty());
    }

    #[test]
    fn eighty_percent_warns_and_seventy_nine_does_not() {
        let mut at = base();
        at.params = 5;
        at.forward = Some(Forwarding {
            line: 8,
            forwarded: 4,
        });
        let mut under = base();
        under.name = "part".to_string();
        under.params = 100;
        under.forward = Some(Forwarding {
            line: 9,
            forwarded: 79,
        });
        let mut raised = profile();
        raised.f04_gate = 100;

        let findings = evaluate(&[at, under], &raised);
        let rules: Vec<_> = findings
            .iter()
            .map(|finding| finding.rule.as_str())
            .collect();
        assert_eq!(rules, vec!["F07"]);
        assert_eq!(findings[0].name, "paint");
        assert_eq!(findings[0].measured_value, Some(80.0));
        assert_eq!(findings[0].level, Level::Warn);
    }

    #[test]
    fn absent_crap_emits_nothing() {
        let mut measured = base();
        measured.cyclomatic = 40;
        measured.crap = None;
        let mut raised = profile();
        raised.f01_gate = 100;

        assert!(evaluate(&[measured], &raised).is_empty());
    }

    #[test]
    fn thresholds_come_from_the_profile() {
        let mut measured = base();
        measured.cyclomatic = 9;
        let mut raised = profile();
        raised.f01_gate = 9;

        assert!(evaluate(&[measured], &raised).is_empty());
    }

    #[test]
    fn waiver_a_number_at_the_measurement_holds_the_gate() {
        let mut measured = base();
        measured.cyclomatic = 9;
        measured.cognitive = 16;
        measured.waivers = vec![
            waiver(
                4,
                Some("F01"),
                Some("9"),
                Some("the branch table is the point"),
            ),
            waiver(5, Some("F03"), Some("20"), Some("nested on purpose")),
        ];

        let findings = evaluate(&[measured], &profile());
        let rules: Vec<_> = findings
            .iter()
            .map(|finding| finding.rule.as_str())
            .collect();
        assert_eq!(rules, vec!["F01", "F03"]);

        let f01 = by_rule(&findings, "F01");
        let held = f01.waiver.as_ref().unwrap();
        assert!(held.holds);
        assert_eq!(held.excused, Some(9.0));
        assert_eq!(
            held.reason.as_deref(),
            Some("the branch table is the point")
        );
        assert!(held.rejection.is_none());
        assert_eq!(f01.level, Level::Gate);
        assert_eq!(f01.threshold, Some(8.0));
        assert_eq!(f01.line, 10);

        let f03 = by_rule(&findings, "F03");
        assert_eq!(f03.level, Level::Gate);
        assert!(f03.waiver.as_ref().unwrap().holds);
    }

    #[test]
    fn waiver_a_number_below_the_measurement_does_not_hold() {
        let mut measured = base();
        measured.cyclomatic = 9;
        measured.waivers = vec![waiver(4, Some("F01"), Some("8"), Some("still over"))];

        let findings = evaluate(&[measured], &profile());
        assert_eq!(findings.len(), 1);
        let attached = findings[0].waiver.as_ref().unwrap();
        assert!(!attached.holds);
        assert_eq!(attached.excused, Some(8.0));
        assert_eq!(attached.reason.as_deref(), Some("still over"));
        assert!(attached.rejection.is_none());
        assert_eq!(findings[0].line, 10);
        assert_eq!(
            findings[0].fix_shape,
            "Split paint. Its cyclomatic complexity is 9, and the gate is 8."
        );
    }

    #[test]
    fn waiver_a_fraction_holds_crap_and_not_a_whole_number_gate() {
        let mut crap = base();
        crap.crap = Some(9.5);
        crap.waivers = vec![waiver(2, Some("F02"), Some("9.5"), Some("covered later"))];
        let mut whole = base();
        whole.name = "paths".to_string();
        whole.cyclomatic = 9;
        whole.waivers = vec![waiver(3, Some("F01"), Some("9.5"), Some("split"))];

        let findings = evaluate(&[crap, whole], &profile());

        assert!(findings[0].waiver.as_ref().unwrap().holds);
        assert_eq!(findings[0].waiver.as_ref().unwrap().excused, Some(9.5));
        let rejected = findings[1].waiver.as_ref().unwrap();
        assert!(!rejected.holds);
        assert_eq!(
            rejected.rejection.as_deref(),
            Some("F01 takes a whole number.")
        );
        assert!(rejected.excused.is_none());
        assert_eq!(findings[1].line, 10);
    }

    #[test]
    fn waiver_a_rejected_line_on_a_metric_gate_stays_on_that_finding() {
        let mut measured = base();
        measured.cyclomatic = 9;
        measured.waivers = vec![waiver(4, Some("F01"), Some("12"), None)];

        let findings = evaluate(&[measured], &profile());

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, 10);
        assert_eq!(findings[0].weight, 0);
        let attached = findings[0].waiver.as_ref().unwrap();
        assert!(!attached.holds);
        assert_eq!(
            attached.rejection.as_deref(),
            Some("The waiver line has no reason.")
        );
        assert_eq!(
            findings[0].fix_shape,
            "Split paint. Its cyclomatic complexity is 9, and the gate is 8."
        );
    }

    #[test]
    fn waiver_a_rejected_line_with_no_metric_gate_is_its_own_finding() {
        let mut measured = base();
        measured.cyclomatic = 1;
        measured.waivers = vec![
            waiver(4, Some("F01"), None, None),
            waiver(6, Some("F07"), Some("100"), Some("not a gate")),
            waiver(7, None, None, None),
        ];

        let findings = evaluate(&[measured], &profile());

        assert_eq!(findings.len(), 3);
        assert_eq!(findings[0].rule, "F01");
        assert_eq!(findings[0].level, Level::Gate);
        assert_eq!(findings[0].line, 4);
        assert_eq!(findings[0].weight, 0);
        assert_eq!(findings[0].measured_value, Some(1.0));
        assert_eq!(findings[0].threshold, Some(8.0));
        assert_eq!(
            findings[0].waiver.as_ref().unwrap().rejection.as_deref(),
            Some("The waiver line has no number.")
        );
        assert_eq!(
            findings[0].fix_shape,
            "Correct the waiver at line 4 of paint. The waiver line has no number."
        );
        assert_eq!(
            findings[0].principle,
            "A function should have few independent paths. McCabe."
        );

        assert_eq!(findings[1].rule, "F07");
        assert_eq!(findings[1].line, 6);
        assert_eq!(findings[1].measured_value, None);
        assert_eq!(findings[1].threshold, None);
        assert_eq!(
            findings[1].waiver.as_ref().unwrap().rejection.as_deref(),
            Some("F07 is not a gate.")
        );
        assert_eq!(
            findings[1].principle,
            "A function that only hands its parameters to one call adds a hop and no behavior."
        );

        assert_eq!(findings[2].rule, "");
        assert_eq!(findings[2].line, 7);
        assert_eq!(findings[2].measured_value, None);
        assert_eq!(findings[2].threshold, None);
        assert_eq!(
            findings[2].waiver.as_ref().unwrap().rejection.as_deref(),
            Some("The waiver line has no rule.")
        );
        assert_eq!(
            findings[2].principle,
            "A waiver excuses one gate, and this line does not name one."
        );
        assert_eq!(
            findings[2].anti_fix,
            "Deleting the waiver line leaves the run with no such line."
        );
    }

    #[test]
    fn waiver_the_same_rule_twice_rejects_both_lines() {
        let mut under = base();
        under.waivers = vec![
            waiver(2, Some("F06"), Some("40"), Some("first")),
            waiver(5, Some("F06"), Some("40"), Some("second")),
        ];
        let mut over = base();
        over.name = "long".to_string();
        over.cyclomatic = 9;
        over.waivers = vec![
            waiver(1, Some("F01"), Some("20"), Some("one")),
            waiver(2, Some("F01"), Some("20"), Some("two")),
        ];

        let findings = evaluate(&[under, over], &profile());

        assert_eq!(findings.len(), 3);
        assert_eq!(findings[0].name, "paint");
        assert_eq!(findings[0].line, 2);
        assert_eq!(findings[1].line, 5);
        assert!(findings.iter().take(2).all(|finding| {
            finding.waiver.as_ref().unwrap().rejection.as_deref() == Some("F06 is waived twice.")
        }));
        assert_eq!(findings[2].name, "long");
        assert_eq!(findings[2].rule, "F01");
        assert_eq!(findings[2].line, 10);
        assert_eq!(
            findings[2].waiver.as_ref().unwrap().rejection.as_deref(),
            Some("F01 is waived twice.")
        );
    }

    #[test]
    fn waiver_a_valid_line_under_the_gate_emits_nothing() {
        let mut measured = base();
        measured.cyclomatic = 8;
        measured.waivers = vec![waiver(2, Some("F01"), Some("20"), Some("room to grow"))];

        assert!(evaluate(&[measured], &profile()).is_empty());
    }

    #[test]
    fn waiver_a_bad_line_in_the_warn_band_replaces_the_warn() {
        let mut bad = base();
        bad.cognitive = 12;
        bad.waivers = vec![waiver(3, Some("F03"), Some("30"), None)];
        let mut kept = base();
        kept.name = "noted".to_string();
        kept.cognitive = 12;
        kept.waivers = vec![waiver(3, Some("F03"), Some("30"), Some("still a warn"))];

        let findings = evaluate(&[bad, kept], &profile());

        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].level, Level::Gate);
        assert_eq!(findings[0].line, 3);
        assert_eq!(findings[0].measured_value, Some(12.0));
        assert_eq!(findings[0].threshold, Some(15.0));
        assert_eq!(
            findings[0].waiver.as_ref().unwrap().rejection.as_deref(),
            Some("The waiver line has no reason.")
        );
        assert_eq!(findings[1].name, "noted");
        assert_eq!(findings[1].level, Level::Warn);
        assert_eq!(findings[1].line, 10);
        assert!(!findings[1].waiver.as_ref().unwrap().holds);
        assert_eq!(findings[1].waiver.as_ref().unwrap().excused, Some(30.0));
        assert_eq!(
            findings[1].waiver.as_ref().unwrap().reason.as_deref(),
            Some("still a warn")
        );
    }

    #[test]
    fn waiver_a_missing_coverage_score_shows_no_measurement() {
        let mut measured = base();
        measured.waivers = vec![waiver(2, Some("F02"), None, Some("no report"))];

        let findings = evaluate(&[measured], &profile());

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, "F02");
        assert_eq!(findings[0].measured_value, None);
        assert_eq!(findings[0].threshold, Some(8.0));
        assert_eq!(
            findings[0].waiver.as_ref().unwrap().rejection.as_deref(),
            Some("The waiver line has no number.")
        );
    }
}
