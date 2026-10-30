use serde_json::{Map, Value};

use crate::rules::{Finding, Level, Waiver};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Changed,
    Strict,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub path: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub scope: Scope,
    pub tool_version: String,
    pub kloc: Option<f64>,
    pub findings: Vec<Finding>,
    pub inherited: Vec<Finding>,
    pub notes: Vec<Note>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rendered {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

struct Widths {
    rule: usize,
    level: usize,
    name: usize,
    location: usize,
}

#[derive(Clone, Copy)]
struct Summary {
    score: Option<f64>,
    fail: bool,
}

struct Listed<'a> {
    findings: &'a [Finding],
    inherited: &'a [Finding],
}

pub fn render(report: &Report, json: bool) -> Rendered {
    let mut findings = report.findings.clone();
    let mut inherited = report.inherited.clone();
    sort_findings(&mut findings);
    sort_findings(&mut inherited);
    let fail = verdict_fails(&findings);
    let score = slop_score(&findings, &inherited, report.kloc);
    let summary = Summary { score, fail };
    let rows = Listed {
        findings: &findings,
        inherited: &inherited,
    };
    let stdout = if json {
        pretty(document(report, &rows, summary))
    } else {
        table(&rows, &report.notes, summary)
    };
    Rendered {
        stdout,
        stderr: String::new(),
        code: if fail { 1 } else { 0 },
    }
}

pub fn no_check(reason: &str) -> Rendered {
    let mut stderr = reason.to_string();
    if !stderr.ends_with('\n') {
        stderr.push('\n');
    }
    Rendered {
        stdout: String::new(),
        stderr,
        code: 2,
    }
}

fn sort_findings(findings: &mut [Finding]) {
    findings.sort_by(compare_findings);
}

fn compare_findings(left: &Finding, right: &Finding) -> std::cmp::Ordering {
    let by_level = compare_rank(left, right);
    let by_weight = by_level.then(weight_order(left, right));
    let by_fraction = by_weight.then_with(|| fraction_order(left, right));
    let by_path = by_fraction.then(path_order(left, right));
    by_path.then(line_order(left, right))
}

fn compare_rank(left: &Finding, right: &Finding) -> std::cmp::Ordering {
    let left_rank = level_rank(left.level);
    let right_rank = level_rank(right.level);
    left_rank.cmp(&right_rank)
}

fn weight_order(left: &Finding, right: &Finding) -> std::cmp::Ordering {
    let right_weight = right.weight;
    let left_weight = left.weight;
    right_weight.cmp(&left_weight)
}

fn fraction_order(left: &Finding, right: &Finding) -> std::cmp::Ordering {
    let right_fraction = fraction(right);
    let left_fraction = fraction(left);
    right_fraction.total_cmp(&left_fraction)
}

fn path_order(left: &Finding, right: &Finding) -> std::cmp::Ordering {
    let left_path = &left.path;
    let right_path = &right.path;
    left_path.cmp(right_path)
}

fn line_order(left: &Finding, right: &Finding) -> std::cmp::Ordering {
    let left_line = left.line;
    let right_line = right.line;
    left_line.cmp(&right_line)
}

fn level_rank(level: Level) -> u8 {
    match level {
        Level::Gate => 0,
        Level::Warn => 1,
    }
}

fn fraction(finding: &Finding) -> f64 {
    if matches!(finding.rule.as_str(), "S06" | "S07") {
        return match finding.measured_value {
            Some(measured) => -measured,
            None => f64::INFINITY,
        };
    }
    match (finding.measured_value, finding.threshold) {
        (Some(measured), Some(threshold)) if threshold != 0.0 => (measured - threshold) / threshold,
        _ => f64::NEG_INFINITY,
    }
}

fn verdict_fails(findings: &[Finding]) -> bool {
    findings.iter().any(open_gate)
}

fn open_gate(finding: &Finding) -> bool {
    if finding.level != Level::Gate {
        return false;
    }
    let waiver = finding.waiver.as_ref();
    !waiver.is_some_and(waiver_holds)
}

fn waiver_holds(waiver: &Waiver) -> bool {
    waiver.holds
}

fn slop_score(findings: &[Finding], inherited: &[Finding], kloc: Option<f64>) -> Option<f64> {
    let kloc = kloc.filter(|kloc| *kloc > 0.0)?;
    let weight: u32 = findings
        .iter()
        .chain(inherited)
        .map(|finding| finding.weight)
        .sum();
    Some(f64::from(weight) / kloc)
}

fn table(rows: &Listed<'_>, notes: &[Note], summary: Summary) -> String {
    let widths = widths(rows.findings, rows.inherited);
    let mut lines = Vec::new();
    push_listed(&mut lines, rows.findings, &widths);
    push_inherited(&mut lines, rows, &widths);
    if listed_any(rows) {
        lines.push(String::new());
    }
    push_notes(&mut lines, notes);
    lines.push(score_line(summary.score, summary.fail));
    join_lines(lines)
}

fn push_listed(lines: &mut Vec<String>, findings: &[Finding], widths: &Widths) {
    for finding in findings {
        lines.push(write_row(finding, widths));
    }
}

fn push_inherited(lines: &mut Vec<String>, rows: &Listed<'_>, widths: &Widths) {
    let inherited = rows.inherited;
    if inherited.is_empty() {
        return;
    }
    let findings = rows.findings;
    if !findings.is_empty() {
        lines.push(String::new());
    }
    lines.push("inherited".to_string());
    push_listed(lines, inherited, widths);
}

fn listed_any(rows: &Listed<'_>) -> bool {
    let findings = rows.findings;
    if !findings.is_empty() {
        return true;
    }
    let inherited = rows.inherited;
    !inherited.is_empty()
}

fn push_notes(lines: &mut Vec<String>, notes: &[Note]) {
    for note in notes {
        lines.push(note_line(note));
    }
}

fn join_lines(lines: Vec<String>) -> String {
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

fn widths(findings: &[Finding], inherited: &[Finding]) -> Widths {
    let rows = finding_rows(findings, inherited);
    Widths {
        rule: column_width(&rows, rule_width, 3),
        level: 4,
        name: column_width(&rows, name_width, 0),
        location: column_width(&rows, location_width, 0),
    }
}

fn finding_rows<'a>(findings: &'a [Finding], inherited: &'a [Finding]) -> Vec<&'a Finding> {
    findings.iter().chain(inherited).collect()
}

fn column_width(rows: &[&Finding], measure: fn(&Finding) -> usize, floor: usize) -> usize {
    let mut widest = floor;
    for finding in rows {
        let width = measure(*finding);
        if width > widest {
            widest = width;
        }
    }
    widest
}

fn rule_width(finding: &Finding) -> usize {
    let rule = &finding.rule;
    rule.len()
}

fn name_width(finding: &Finding) -> usize {
    let name = &finding.name;
    name.len()
}

fn location_width(finding: &Finding) -> usize {
    location(finding).len()
}

fn write_row(finding: &Finding, widths: &Widths) -> String {
    format!(
        "{:<rule$}  {:<level$}  {:<name$}  {:<location$}  {}",
        finding.rule,
        level_name(finding.level),
        finding.name,
        location(finding),
        detail(finding),
        rule = widths.rule,
        level = widths.level,
        name = widths.name,
        location = widths.location,
    )
    .trim_end()
    .to_string()
}

fn location(finding: &Finding) -> String {
    format!("{}:{}", finding.path, finding.line)
}

fn detail(finding: &Finding) -> String {
    let mut text = metric_text(finding);
    append_waiver(&mut text, finding);
    text
}

fn metric_text(finding: &Finding) -> String {
    let rule = finding.rule.as_str();
    if coverage_rule(rule) {
        return below_text(finding);
    }
    above_text(finding)
}

fn coverage_rule(rule: &str) -> bool {
    matches!(rule, "S06" | "S07")
}

fn below_text(finding: &Finding) -> String {
    let measured = finding.measured_value;
    let threshold = finding.threshold;
    match (measured, threshold) {
        (Some(measured), Some(threshold)) => format!("{} < {}", show(measured), show(threshold)),
        (None, Some(threshold)) => format!("absent < {}", show(threshold)),
        _ => String::new(),
    }
}

fn above_text(finding: &Finding) -> String {
    let measured = finding.measured_value;
    let threshold = finding.threshold;
    match (measured, threshold) {
        (Some(measured), Some(threshold)) => format!("{} > {}", show(measured), show(threshold)),
        _ => String::new(),
    }
}

fn append_waiver(text: &mut String, finding: &Finding) {
    let waiver = finding.waiver.as_ref();
    let Some(extra) = waiver_text(waiver) else {
        return;
    };
    if text.is_empty() {
        *text = extra;
        return;
    }
    text.push_str("  ");
    text.push_str(&extra);
}

fn waiver_text(waiver: Option<&Waiver>) -> Option<String> {
    let waiver = waiver?;
    if let Some(rejection) = &waiver.rejection {
        return Some(rejection.clone());
    }
    let excused = waiver.excused?;
    Some(match &waiver.reason {
        Some(reason) => format!("waived {} {reason}", show(excused)),
        None => format!("waived {}", show(excused)),
    })
}

fn note_line(note: &Note) -> String {
    match &note.path {
        Some(path) => format!("note  {path}  {}", note.message),
        None => format!("note  {}", note.message),
    }
}

fn score_line(score: Option<f64>, fail: bool) -> String {
    let verdict = if fail { "fail" } else { "pass" };
    match score {
        Some(score) => format!("slop_score {}  {verdict}", show_score(score)),
        None => verdict.to_string(),
    }
}

fn show(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}

fn show_score(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.1}")
    } else {
        value.to_string()
    }
}

fn level_name(level: Level) -> &'static str {
    match level {
        Level::Gate => "gate",
        Level::Warn => "warn",
    }
}

fn document(report: &Report, rows: &Listed<'_>, summary: Summary) -> Value {
    let mut doc = Map::new();
    doc.insert("verdict".to_string(), verdict_value(summary.fail));
    doc.insert(
        "scope".to_string(),
        Value::String(scope_name(report.scope).to_string()),
    );
    doc.insert("slop_score".to_string(), optional_float(summary.score));
    doc.insert("kloc".to_string(), optional_float(report.kloc));
    doc.insert(
        "tool_version".to_string(),
        Value::String(report.tool_version.clone()),
    );
    doc.insert("findings".to_string(), finding_array(rows.findings));
    doc.insert("inherited".to_string(), finding_array(rows.inherited));
    doc.insert("notes".to_string(), note_array(&report.notes));
    Value::Object(doc)
}

fn verdict_value(fail: bool) -> Value {
    let verdict = if fail { "fail" } else { "pass" };
    Value::String(verdict.to_string())
}

fn optional_float(value: Option<f64>) -> Value {
    value.map(json_float).unwrap_or(Value::Null)
}

fn finding_array(findings: &[Finding]) -> Value {
    let values = findings.iter().map(finding_json).collect();
    Value::Array(values)
}

fn note_array(notes: &[Note]) -> Value {
    let values = notes.iter().map(note_json).collect();
    Value::Array(values)
}

fn finding_json(finding: &Finding) -> Value {
    let mut map = Map::new();
    map.insert("rule".to_string(), Value::String(finding.rule.clone()));
    map.insert("level".to_string(), level_json(finding.level));
    map.insert("symbol".to_string(), symbol_json(finding));
    map.insert("line".to_string(), Value::from(finding.line));
    insert_numbers(&mut map, finding);
    map.insert("weight".to_string(), Value::from(finding.weight));
    insert_text(&mut map, finding);
    insert_waiver_field(&mut map, finding);
    Value::Object(map)
}

fn level_json(level: Level) -> Value {
    let name = level_name(level);
    Value::String(name.to_string())
}

fn symbol_json(finding: &Finding) -> Value {
    let mut symbol = Map::new();
    symbol.insert("path".to_string(), Value::String(finding.path.clone()));
    symbol.insert("name".to_string(), Value::String(finding.name.clone()));
    symbol.insert("param_types".to_string(), param_list(finding));
    symbol.insert(
        "qualifier".to_string(),
        Value::String(finding.qualifier.clone()),
    );
    Value::Object(symbol)
}

fn param_list(finding: &Finding) -> Value {
    let values = finding
        .param_types
        .iter()
        .cloned()
        .map(Value::String)
        .collect();
    Value::Array(values)
}

fn insert_numbers(map: &mut Map<String, Value>, finding: &Finding) {
    map.insert(
        "measured_value".to_string(),
        number_field(finding.measured_value),
    );
    map.insert("threshold".to_string(), number_field(finding.threshold));
}

fn number_field(value: Option<f64>) -> Value {
    value.map(json_number).unwrap_or(Value::Null)
}

fn insert_text(map: &mut Map<String, Value>, finding: &Finding) {
    map.insert(
        "principle".to_string(),
        Value::String(finding.principle.clone()),
    );
    map.insert(
        "fix_shape".to_string(),
        Value::String(finding.fix_shape.clone()),
    );
    map.insert(
        "anti_fix".to_string(),
        Value::String(finding.anti_fix.clone()),
    );
}

fn insert_waiver_field(map: &mut Map<String, Value>, finding: &Finding) {
    let waiver = finding.waiver.as_ref();
    let Some(waiver) = waiver else {
        return;
    };
    map.insert("waiver".to_string(), waiver_json(waiver));
}

fn waiver_json(waiver: &Waiver) -> Value {
    let mut map = Map::new();
    map.insert(
        "excused".to_string(),
        waiver.excused.map(json_number).unwrap_or(Value::Null),
    );
    map.insert(
        "reason".to_string(),
        match &waiver.reason {
            Some(reason) => Value::String(reason.clone()),
            None => Value::Null,
        },
    );
    map.insert("holds".to_string(), Value::Bool(waiver.holds));
    map.insert(
        "rejection".to_string(),
        match &waiver.rejection {
            Some(rejection) => Value::String(rejection.clone()),
            None => Value::Null,
        },
    );
    Value::Object(map)
}

fn note_json(note: &Note) -> Value {
    let mut map = Map::new();
    if let Some(path) = &note.path {
        map.insert("path".to_string(), Value::String(path.clone()));
    }
    map.insert("message".to_string(), Value::String(note.message.clone()));
    Value::Object(map)
}

fn json_number(value: f64) -> Value {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        Value::from(value as i64)
    } else {
        json_float(value)
    }
}

fn json_float(value: f64) -> Value {
    serde_json::Number::from_f64(value)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

fn pretty(value: Value) -> String {
    let mut text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string());
    text.push('\n');
    text
}

fn scope_name(scope: Scope) -> &'static str {
    match scope {
        Scope::Changed => "changed",
        Scope::Strict => "strict",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{Finding, Level, Waiver};
    use serde_json::Value;

    struct Metric {
        measured: f64,
        threshold: f64,
        weight: u32,
    }

    fn finding(
        rule: &str,
        level: Level,
        path: &str,
        name: &str,
        line: u32,
        metric: Metric,
    ) -> Finding {
        Finding {
            rule: rule.to_string(),
            level,
            path: path.to_string(),
            name: name.to_string(),
            param_types: vec![],
            qualifier: String::new(),
            line,
            measured_value: Some(metric.measured),
            threshold: Some(metric.threshold),
            weight: metric.weight,
            principle: String::new(),
            fix_shape: String::new(),
            anti_fix: String::new(),
            waiver: None,
        }
    }

    fn sample() -> Report {
        let mut chain = finding(
            "F08",
            Level::Gate,
            "src/buf/take.rs",
            "<Take<T> as Buf>::chunks_vectored",
            170,
            Metric {
                measured: 2.0,
                threshold: 1.0,
                weight: 10,
            },
        );
        chain.principle =
            "A member chain that does not start at the receiver may be one deep. Law of Demeter."
                .to_string();
        chain.fix_shape = "Break the chain len().min at line 170 in <Take<T> as Buf>::chunks_vectored. It is 2 deep, and the limit is 1.".to_string();
        chain.anti_fix = "Saving the chain in locals leaves the finding in place.".to_string();

        let mut forward = finding(
            "F07",
            Level::Warn,
            "src/requests/api.py",
            "get",
            87,
            Metric {
                measured: 100.0,
                threshold: 80.0,
                weight: 1,
            },
        );
        forward.principle =
            "A function that only forwards its parameters adds a hop without a decision. Martin's pass-through.".to_string();
        forward.fix_shape =
            "Inline get. It forwards 100 percent of its ordinary parameters, and the limit is 80."
                .to_string();
        forward.anti_fix =
            "Adding a log line beside the forward leaves the finding in place.".to_string();

        let mut inherited = finding(
            "F03",
            Level::Gate,
            "include/CLI/StringTools.hpp",
            "CLI::detail::join",
            54,
            Metric {
                measured: 18.0,
                threshold: 15.0,
                weight: 10,
            },
        );
        inherited.param_types = vec!["const T &".to_string(), "std::string".to_string()];
        inherited.principle =
            "Nested control flow costs more than a flat branch. SonarSource cognitive complexity."
                .to_string();
        inherited.fix_shape =
            "Simplify CLI::detail::join. Its cognitive complexity is 18, and the gate is 15."
                .to_string();
        inherited.anti_fix =
            "Extracting the body into a lambda leaves the finding in place.".to_string();

        Report {
            scope: Scope::Changed,
            tool_version: "0.0.0".to_string(),
            kloc: Some(0.04),
            findings: vec![forward, chain],
            inherited: vec![inherited],
            notes: vec![Note {
                path: None,
                message: "No coverage report was given. CRAP was not computed.".to_string(),
            }],
        }
    }

    #[test]
    fn the_sample_table_matches_the_prototype_and_fails() {
        let rendered = render(&sample(), false);

        assert_eq!(rendered.code, 1);
        assert!(rendered.stderr.is_empty());
        assert_eq!(
            rendered.stdout,
            include_str!("../prototype/text-output/table.txt")
        );
        let rows: Vec<_> = rendered
            .stdout
            .lines()
            .filter(|line| line.starts_with('F'))
            .collect();
        let locations: Vec<_> = ["src/buf/take.rs:", "src/requests/api.py:", "include/CLI/"]
            .iter()
            .map(|marker| {
                rows.iter()
                    .find(|line| line.contains(marker))
                    .unwrap()
                    .find(marker)
                    .unwrap()
            })
            .collect();
        assert!(locations.windows(2).all(|pair| pair[0] == pair[1]));
        assert!(!rendered.stdout.contains("const T"));
    }

    #[test]
    fn the_sample_json_matches_the_prototype_and_keeps_the_exit_code() {
        let rendered = render(&sample(), true);

        assert_eq!(rendered.code, 1);
        assert!(rendered.stderr.is_empty());
        assert!(rendered.stdout.starts_with("{\n  \"verdict\": \"fail\""));
        assert!(!rendered.stdout.contains("\ninherited\n"));
        let actual: Value = serde_json::from_str(&rendered.stdout).unwrap();
        let expected: Value =
            serde_json::from_str(include_str!("../prototype/report-schema/report.json")).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn an_inherited_gate_passes_and_an_empty_list_prints_nothing() {
        let mut report = sample();
        report.findings.clear();

        let rendered = render(&report, false);

        assert_eq!(rendered.code, 0);
        assert!(rendered.stdout.starts_with("inherited\n"));
        assert!(!rendered.stdout.starts_with('\n'));
    }

    #[test]
    fn a_warn_passes_and_gates_sort_ahead_by_weight_then_fraction_then_path() {
        let heavy = finding(
            "F03",
            Level::Gate,
            "a.rs",
            "heavy",
            1,
            Metric {
                measured: 20.0,
                threshold: 10.0,
                weight: 5,
            },
        );
        let early = finding(
            "F01",
            Level::Gate,
            "a.rs",
            "early",
            2,
            Metric {
                measured: 12.0,
                threshold: 8.0,
                weight: 1,
            },
        );
        let later = finding(
            "F01",
            Level::Gate,
            "b.rs",
            "later",
            1,
            Metric {
                measured: 12.0,
                threshold: 8.0,
                weight: 1,
            },
        );
        let warn = finding(
            "F07",
            Level::Warn,
            "a.rs",
            "soft",
            3,
            Metric {
                measured: 100.0,
                threshold: 80.0,
                weight: 9,
            },
        );
        let report = Report {
            scope: Scope::Strict,
            tool_version: "0.0.0".to_string(),
            kloc: Some(1.0),
            findings: vec![warn, later, early, heavy],
            inherited: vec![],
            notes: vec![],
        };

        let rendered = render(&report, false);
        let rules: Vec<_> = rendered
            .stdout
            .lines()
            .filter(|line| line.starts_with('F'))
            .map(|line| line.split_whitespace().next().unwrap())
            .collect();

        assert_eq!(rendered.code, 1);
        assert_eq!(rules, vec!["F03", "F01", "F01", "F07"]);
        assert!(rendered.stdout.contains("a.rs:2"));
        assert!(rendered.stdout.find("a.rs:2").unwrap() < rendered.stdout.find("b.rs:1").unwrap());
        assert!(!rendered.stdout.contains("inherited"));
        assert!(rendered.stdout.ends_with("slop_score 16.0  fail\n"));
    }

    #[test]
    fn a_held_waiver_passes_and_the_row_shows_the_number_and_the_reason() {
        let mut held = finding(
            "F01",
            Level::Gate,
            "src/paint.py",
            "paint",
            10,
            Metric {
                measured: 9.0,
                threshold: 8.0,
                weight: 0,
            },
        );
        held.waiver = Some(Waiver {
            excused: Some(9.0),
            reason: Some("the branch table is the point".to_string()),
            holds: true,
            rejection: None,
        });
        let report = Report {
            scope: Scope::Changed,
            tool_version: "0.0.0".to_string(),
            kloc: Some(1.0),
            findings: vec![held],
            inherited: vec![],
            notes: vec![],
        };

        let text = render(&report, false);
        let json = render(&report, true);

        assert_eq!(text.code, 0);
        assert_eq!(json.code, 0);
        assert!(text
            .stdout
            .contains("9 > 8  waived 9 the branch table is the point"));
        let document: Value = serde_json::from_str(&json.stdout).unwrap();
        assert_eq!(document["verdict"], "pass");
        assert_eq!(document["findings"][0]["waiver"]["excused"], 9);
        assert_eq!(
            document["findings"][0]["waiver"]["reason"],
            "the branch table is the point"
        );
        assert_eq!(document["findings"][0]["waiver"]["holds"], true);
    }

    #[test]
    fn a_rejected_waiver_fails_and_names_what_the_line_lacks() {
        let mut rejected = finding(
            "F01",
            Level::Gate,
            "src/paint.py",
            "paint",
            10,
            Metric {
                measured: 9.0,
                threshold: 8.0,
                weight: 0,
            },
        );
        rejected.waiver = Some(Waiver {
            excused: None,
            reason: None,
            holds: false,
            rejection: Some("The waiver line has no reason.".to_string()),
        });
        let mut bare = finding(
            "F02",
            Level::Gate,
            "src/paint.py",
            "paint",
            2,
            Metric {
                measured: 0.0,
                threshold: 0.0,
                weight: 0,
            },
        );
        bare.measured_value = None;
        bare.threshold = None;
        bare.waiver = Some(Waiver {
            excused: None,
            reason: None,
            holds: false,
            rejection: Some("The waiver line has no number.".to_string()),
        });
        let report = Report {
            scope: Scope::Changed,
            tool_version: "0.0.0".to_string(),
            kloc: None,
            findings: vec![rejected, bare],
            inherited: vec![],
            notes: vec![Note {
                path: Some("src/paint.py".to_string()),
                message: "the parser left two lines unscored".to_string(),
            }],
        };

        let rendered = render(&report, false);

        assert_eq!(rendered.code, 1);
        assert!(rendered
            .stdout
            .contains("9 > 8  The waiver line has no reason."));
        let bare = rendered
            .stdout
            .lines()
            .find(|line| line.contains("src/paint.py:2"))
            .unwrap();
        assert!(bare.contains("The waiver line has no number."));
        assert!(!bare.contains('>'));
        assert!(rendered
            .stdout
            .contains("note  src/paint.py  the parser left two lines unscored"));
        assert!(rendered.stdout.ends_with("fail\n"));
        assert!(!rendered.stdout.contains("slop_score"));
    }

    #[test]
    fn no_check_writes_the_reason_and_exits_2() {
        let rendered = no_check("unknown rule F09");

        assert_eq!(rendered.code, 2);
        assert!(rendered.stdout.is_empty());
        assert_eq!(rendered.stderr, "unknown rule F09\n");
    }

    #[test]
    fn a_coverage_row_reads_below_the_gate_and_an_absent_fraction_sorts_first() {
        let partial = finding(
            "S06",
            Level::Gate,
            "src/a.py",
            "classify",
            6,
            Metric {
                measured: 0.5,
                threshold: 1.0,
                weight: 10,
            },
        );
        let mut absent = finding(
            "S06",
            Level::Gate,
            "src/a.py",
            "missing",
            1,
            Metric {
                measured: 0.0,
                threshold: 1.0,
                weight: 10,
            },
        );
        absent.measured_value = None;
        let report = Report {
            scope: Scope::Changed,
            tool_version: "0.0.0".to_string(),
            kloc: Some(1.0),
            findings: vec![partial, absent],
            inherited: vec![],
            notes: vec![],
        };

        let rendered = render(&report, false);

        let partial_at = rendered.stdout.find("0.5 < 1").unwrap();
        let absent_at = rendered.stdout.find("absent < 1").unwrap();
        assert!(absent_at < partial_at);
        assert_eq!(rendered.code, 1);

        let mutation = finding(
            "S07",
            Level::Gate,
            "src/a.py",
            "classify",
            6,
            Metric {
                measured: 0.5,
                threshold: 1.0,
                weight: 10,
            },
        );
        let mut absent_score = finding(
            "S07",
            Level::Gate,
            "src/a.py",
            "missing",
            1,
            Metric {
                measured: 0.0,
                threshold: 1.0,
                weight: 10,
            },
        );
        absent_score.measured_value = None;
        let rendered = render(
            &Report {
                scope: Scope::Changed,
                tool_version: "0.0.0".to_string(),
                kloc: Some(1.0),
                findings: vec![mutation, absent_score],
                inherited: vec![],
                notes: vec![],
            },
            false,
        );
        assert!(rendered.stdout.contains("0.5 < 1"));
        assert!(rendered.stdout.contains("absent < 1"));
    }
}
