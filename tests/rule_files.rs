use std::path::PathBuf;

use slopmeter::adapters::python::{parse, ALLOW};
use slopmeter::analyzers::measure;
use slopmeter::inputs::{cover, Config};
use slopmeter::ir::Language;
use slopmeter::reporters::{render, Report};
use slopmeter::rules::evaluate;
use slopmeter::scope::{partition, ScopeOptions};

const RULES: [(&str, &str); 9] = [
    ("F01.py", "F01"),
    ("F02.py", "F02"),
    ("F03.py", "F03"),
    ("F04.py", "F04"),
    ("F05.py", "F05"),
    ("F06.py", "F06"),
    ("F07.py", "F07"),
    ("F08.py", "F08"),
    ("F11.py", "F11"),
];

fn rules_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/rules")
}

fn report() -> Report {
    let dir = rules_dir();
    let mut files = Vec::new();
    let mut measurements = Vec::new();
    let mut lines = 0usize;
    for (name, _) in RULES {
        let path = format!("tests/rules/{name}");
        let source = std::fs::read(dir.join(name)).unwrap();
        lines += source.iter().filter(|byte| **byte == b'\n').count();
        let file = parse(&path, &source);
        measurements.extend(measure(&file, ALLOW));
        files.push(file);
    }
    let scored: Vec<String> = RULES
        .iter()
        .map(|(name, _)| format!("tests/rules/{name}"))
        .collect();
    let scored_refs: Vec<&str> = scored.iter().map(String::as_str).collect();
    let notes = cover(
        &mut measurements,
        &scored_refs,
        Some(&dir.join("coverage.lcov")),
    )
    .unwrap();
    let config = Config::builtin();
    let findings = evaluate(&measurements, config.profile(Language::Python));
    let scoped = partition(
        &dir,
        &files,
        &findings,
        ScopeOptions {
            changed: None,
            strict: true,
        },
    )
    .unwrap();
    Report {
        scope: scoped.scope,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        kloc: Some(lines as f64 / 1000.0),
        findings: scoped.findings,
        inherited: scoped.inherited,
        notes,
    }
}

#[test]
fn each_file_trips_only_its_rule_and_the_goldens_match() {
    let report = report();
    assert_eq!(
        report
            .inherited
            .iter()
            .map(|finding| finding.rule.as_str())
            .collect::<Vec<_>>(),
        vec!["S06"]
    );
    for (name, rule) in RULES {
        let path = format!("tests/rules/{name}");
        let found: Vec<_> = report
            .findings
            .iter()
            .filter(|finding| finding.path == path)
            .map(|finding| finding.rule.as_str())
            .collect();
        // Seven structural levels are also an F03 warn. The same forms make both counts.
        let expected = if rule == "F05" {
            vec!["F03", "F05"]
        } else {
            vec![rule]
        };
        assert_eq!(found, expected, "{name}");
    }
    let table = render(&report, false);
    let json = render(&report, true);
    assert_eq!(table.code, 1);
    assert_eq!(json.code, 1);
    assert_eq!(table.stdout, include_str!("rules/table.txt"));
    assert_eq!(json.stdout, include_str!("rules/report.json"));
}
