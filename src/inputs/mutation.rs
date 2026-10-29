use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::analyzers::{Measurement, Mutation};
use crate::ir::Span;
use crate::reporters::Note;

const NO_REPORT: &str = "No mutation report was given.";
const MISSING_FILE: &str = "No mutation record matches this file.";
const TIE: &str = "Mutation records tie for this file.";
const UNFINISHED: &str = "A mutant in this file has no finished outcome.";

#[derive(Debug)]
pub enum MutationError {
    Read(PathBuf, std::io::Error),
    BadReport(PathBuf),
}

impl std::fmt::Display for MutationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MutationError::Read(path, err) => write!(f, "reading {}: {err}", path.display()),
            MutationError::BadReport(path) => {
                write!(f, "mutation report {} cannot be parsed", path.display())
            }
        }
    }
}

impl std::error::Error for MutationError {}

pub fn mutation_path(root: &Path, mutation: Option<&str>) -> Option<PathBuf> {
    let mutation = mutation?;
    let path = Path::new(mutation);
    Some(if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    })
}

pub fn score_rust(
    measurements: &mut [Measurement],
    rust: &[(usize, usize)],
    report: Option<&Path>,
) -> Result<Vec<Note>, MutationError> {
    for (start, end) in rust {
        for measurement in &mut measurements[*start..*end] {
            measurement.mutation = None;
        }
    }
    let Some(report) = report else {
        if rust.is_empty() {
            return Ok(Vec::new());
        }
        mark_unshown(measurements, rust);
        return Ok(vec![Note {
            path: None,
            message: NO_REPORT.to_string(),
        }]);
    };
    if !report.is_file() {
        if !rust.is_empty() {
            mark_unshown(measurements, rust);
        }
        return Ok(vec![Note {
            path: None,
            message: format!("No mutation report at {}.", report.display()),
        }]);
    }
    let records = read_outcomes(report)?;
    if rust.is_empty() {
        return Ok(Vec::new());
    }
    Ok(apply(measurements, rust, &records))
}

fn read_outcomes(path: &Path) -> Result<Vec<Record>, MutationError> {
    let text =
        fs::read_to_string(path).map_err(|err| MutationError::Read(path.to_path_buf(), err))?;
    let value: Value =
        serde_json::from_str(&text).map_err(|_| MutationError::BadReport(path.to_path_buf()))?;
    let outcomes = value
        .get("outcomes")
        .and_then(Value::as_array)
        .ok_or_else(|| MutationError::BadReport(path.to_path_buf()))?;
    let mut records = Vec::new();
    for outcome in outcomes {
        match record(outcome) {
            Ok(Some(record)) => records.push(record),
            Ok(None) => {}
            Err(()) => return Err(MutationError::BadReport(path.to_path_buf())),
        }
    }
    Ok(records)
}

fn record(value: &Value) -> Result<Option<Record>, ()> {
    let scenario = value.get("scenario").ok_or(())?;
    if scenario.as_str() == Some("Baseline") {
        value.get("summary").and_then(Value::as_str).ok_or(())?;
        return Ok(None);
    }
    let mutant = scenario.get("Mutant").ok_or(())?;
    let summary = value.get("summary").and_then(Value::as_str).ok_or(())?;
    let file = mutant
        .get("file")
        .and_then(Value::as_str)
        .map(normalize)
        .filter(|file| !file.is_empty())
        .ok_or(())?;
    let line = start_line(mutant);
    Ok(Some(Record {
        file,
        line,
        kind: kind(summary, line),
    }))
}

fn start_line(mutant: &Value) -> Option<u32> {
    let line = mutant.pointer("/span/start/line")?.as_u64()?;
    u32::try_from(line).ok().filter(|line| *line >= 1)
}

fn kind(summary: &str, line: Option<u32>) -> Kind {
    if line.is_none() {
        return Kind::Unfinished;
    }
    match summary {
        "CaughtMutant" | "Timeout" => Kind::Detected,
        "MissedMutant" => Kind::Undetected,
        "Unviable" => Kind::LeftOut,
        _ => Kind::Unfinished,
    }
}

struct Record {
    file: String,
    line: Option<u32>,
    kind: Kind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Detected,
    Undetected,
    LeftOut,
    Unfinished,
}

struct ScoredFile {
    key: String,
    path: String,
    indexes: Vec<usize>,
}

fn apply(
    measurements: &mut [Measurement],
    rust: &[(usize, usize)],
    records: &[Record],
) -> Vec<Note> {
    let files = scored_files(measurements, rust);
    let mut notes = Vec::new();
    for file in files {
        match choose(records, &file.key) {
            Choice::Absent => {
                mark_indexes(measurements, &file.indexes, Mutation::Unshown);
                notes.push(Note {
                    path: Some(file.path),
                    message: MISSING_FILE.to_string(),
                });
            }
            Choice::Tie => {
                mark_indexes(measurements, &file.indexes, Mutation::Unshown);
                notes.push(Note {
                    path: Some(file.path),
                    message: TIE.to_string(),
                });
            }
            Choice::Rows(rows) => {
                if rows.iter().any(|row| row.kind == Kind::Unfinished) {
                    mark_indexes(measurements, &file.indexes, Mutation::Unshown);
                    notes.push(Note {
                        path: Some(file.path),
                        message: UNFINISHED.to_string(),
                    });
                    continue;
                }
                score_file(measurements, &file, &rows);
            }
        }
    }
    notes
}

fn score_file(measurements: &mut [Measurement], file: &ScoredFile, rows: &[&Record]) {
    let spans: Vec<Span> = file
        .indexes
        .iter()
        .map(|index| measurements[*index].span)
        .collect();
    for (slot, index) in file.indexes.iter().enumerate() {
        let span = spans[slot];
        let mut detected = 0u32;
        let mut undetected = 0u32;
        let mut first_miss = None;
        for row in rows {
            let Some(line) = row.line else {
                continue;
            };
            if !owns(&spans, slot, line) {
                continue;
            }
            match row.kind {
                Kind::Detected => detected += 1,
                Kind::Undetected => {
                    undetected += 1;
                    first_miss = Some(first_miss.map_or(line, |have: u32| have.min(line)));
                }
                Kind::LeftOut | Kind::Unfinished => {}
            }
        }
        let total = detected + undetected;
        if total == 0 || undetected == 0 {
            continue;
        }
        let score = f64::from(detected) / f64::from(total);
        measurements[*index].mutation = Some(Mutation::Score {
            score,
            line: first_miss.unwrap_or(span.start),
        });
    }
}

fn owns(spans: &[Span], index: usize, line: u32) -> bool {
    let span = spans[index];
    if line < span.start || line > span.end {
        return false;
    }
    !spans.iter().enumerate().any(|(other, inner)| {
        other != index && line >= inner.start && line <= inner.end && strictly_inside(*inner, span)
    })
}

fn strictly_inside(inner: Span, outer: Span) -> bool {
    inner.start >= outer.start
        && inner.end <= outer.end
        && (inner.start > outer.start || inner.end < outer.end)
}

enum Choice<'a> {
    Absent,
    Tie,
    Rows(Vec<&'a Record>),
}

fn choose<'a>(records: &'a [Record], key: &str) -> Choice<'a> {
    let exact: Vec<&Record> = records.iter().filter(|row| row.file == key).collect();
    if !exact.is_empty() {
        return Choice::Rows(exact);
    }
    let suffix: Vec<&Record> = records
        .iter()
        .filter(|row| boundary_suffix(&row.file, key))
        .collect();
    if suffix.is_empty() {
        return Choice::Absent;
    }
    let paths: BTreeSet<&str> = suffix.iter().map(|row| row.file.as_str()).collect();
    if paths.len() > 1 {
        return Choice::Tie;
    }
    Choice::Rows(suffix)
}

fn scored_files(measurements: &[Measurement], rust: &[(usize, usize)]) -> Vec<ScoredFile> {
    let mut order = Vec::new();
    let mut groups: BTreeMap<String, ScoredFile> = BTreeMap::new();
    for (start, end) in rust {
        for (index, measurement) in measurements.iter().enumerate().take(*end).skip(*start) {
            let path = &measurement.path;
            let key = normalize(path);
            if key.is_empty() {
                continue;
            }
            if let Some(file) = groups.get_mut(&key) {
                file.indexes.push(index);
                continue;
            }
            order.push(key.clone());
            groups.insert(
                key.clone(),
                ScoredFile {
                    key,
                    path: path.clone(),
                    indexes: vec![index],
                },
            );
        }
    }
    order
        .into_iter()
        .filter_map(|key| groups.remove(&key))
        .collect()
}

fn mark_unshown(measurements: &mut [Measurement], rust: &[(usize, usize)]) {
    for (start, end) in rust {
        for measurement in &mut measurements[*start..*end] {
            measurement.mutation = Some(Mutation::Unshown);
        }
    }
}

fn mark_indexes(measurements: &mut [Measurement], indexes: &[usize], mutation: Mutation) {
    for index in indexes {
        measurements[*index].mutation = Some(mutation.clone());
    }
}

fn boundary_suffix(report: &str, scored: &str) -> bool {
    if scored.is_empty() || report.len() <= scored.len() || !report.ends_with(scored) {
        return false;
    }
    report.as_bytes()[report.len() - scored.len() - 1] == b'/'
}

fn normalize(path: &str) -> String {
    let path = path.replace('\\', "/");
    let absolute = path.starts_with('/');
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    let joined = parts.join("/");
    if absolute {
        format!("/{joined}")
    } else {
        joined
    }
}

#[cfg(test)]
mod tests {
    use super::{mutation_path, score_rust, MutationError};
    use crate::analyzers::{Measurement, Mutation};
    use crate::ir::Span;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    fn fixture(name: &str) -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("mutation-fixtures")
            .join(format!(
                "{name}-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn measured(path: &str, name: &str, start: u32, end: u32) -> Measurement {
        Measurement {
            path: path.to_string(),
            name: name.to_string(),
            param_types: vec![],
            qualifier: String::new(),
            span: Span { start, end },
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
            coverage: None,
            mutation: None,
        }
    }

    fn mutant(file: &str, line: u32, summary: &str) -> String {
        format!(
            r#"{{"scenario":{{"Mutant":{{"file":"{file}","function":{{"function_name":"ignored"}},"span":{{"start":{{"line":{line},"column":1}},"end":{{"line":{line},"column":2}}}}}}}},"summary":"{summary}"}}"#
        )
    }

    fn report(parts: &[&str]) -> String {
        format!(
            r#"{{"cargo_mutants_version":"27.1.0","outcomes":[{{"scenario":"Baseline","summary":"Success"}},{}]}}"#,
            parts.join(",")
        )
    }

    fn write_report(root: &Path, body: &str) -> PathBuf {
        let path = root.join("outcomes.json");
        fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn a_caught_mutant_and_a_missed_mutant_score_one_half_at_the_missed_line() {
        let root = fixture("half");
        let path = write_report(
            &root,
            &report(&[
                &mutant("src/lib.rs", 4, "CaughtMutant"),
                &mutant("src/lib.rs", 8, "MissedMutant"),
                &mutant("src/lib.rs", 6, "MissedMutant"),
            ]),
        );
        let mut measurements = vec![measured("src/lib.rs", "classify", 1, 12)];

        let notes = score_rust(&mut measurements, &[(0, 1)], Some(&path)).unwrap();

        assert!(notes.is_empty());
        assert_eq!(
            measurements[0].mutation,
            Some(Mutation::Score {
                score: 1.0 / 3.0,
                line: 6
            })
        );
    }

    #[test]
    fn timeout_is_detected_and_unviable_is_left_out() {
        let root = fixture("timeout");
        let path = write_report(
            &root,
            &report(&[
                &mutant("src/lib.rs", 2, "Timeout"),
                &mutant("src/lib.rs", 3, "Unviable"),
                &mutant("src/lib.rs", 4, "CaughtMutant"),
            ]),
        );
        let mut measurements = vec![measured("src/lib.rs", "classify", 1, 5)];

        let notes = score_rust(&mut measurements, &[(0, 1)], Some(&path)).unwrap();

        assert!(notes.is_empty());
        assert_eq!(measurements[0].mutation, None);
    }

    #[test]
    fn a_file_absent_from_the_report_is_unshown_and_a_span_with_no_valid_mutant_stays_off() {
        let root = fixture("absent");
        let path = write_report(
            &root,
            &report(&[
                &mutant("src/lib.rs", 2, "Unviable"),
                &mutant("src/other.rs", 2, "MissedMutant"),
                &mutant("src/lib.rs", 20, "MissedMutant"),
            ]),
        );
        let mut measurements = vec![
            measured("src/lib.rs", "classify", 1, 5),
            measured("src/missing.rs", "gone", 1, 4),
        ];

        let notes = score_rust(&mut measurements, &[(0, 2)], Some(&path)).unwrap();

        assert_eq!(measurements[0].mutation, None);
        assert_eq!(measurements[1].mutation, Some(Mutation::Unshown));
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].path.as_deref(), Some("src/missing.rs"));
        assert_eq!(notes[0].message, "No mutation record matches this file.");
    }

    #[test]
    fn the_report_function_name_is_ignored_and_a_nested_span_owns_its_line() {
        let root = fixture("nested");
        let path = write_report(
            &root,
            &report(&[
                &mutant("src/lib.rs", 3, "MissedMutant"),
                &mutant("src/lib.rs", 8, "CaughtMutant"),
            ]),
        );
        let mut measurements = vec![
            measured("src/lib.rs", "outer", 1, 10),
            measured("src/lib.rs", "inner", 6, 9),
        ];

        score_rust(&mut measurements, &[(0, 2)], Some(&path)).unwrap();

        assert_eq!(
            measurements[0].mutation,
            Some(Mutation::Score {
                score: 0.0,
                line: 3
            })
        );
        assert_eq!(measurements[1].mutation, None);
    }

    #[test]
    fn an_unfinished_mutant_fails_the_file_and_the_finished_subset_is_not_the_score() {
        let root = fixture("unfinished");
        let broken = r#"{"scenario":{"Mutant":{"file":"src/lib.rs","span":{"end":{"line":1,"column":1}}}},"summary":"CaughtMutant"}"#.to_string();
        let path = write_report(
            &root,
            &report(&[
                &mutant("src/lib.rs", 2, "CaughtMutant"),
                &broken,
                &mutant("src/lib.rs", 4, "Failure"),
            ]),
        );
        let mut measurements = vec![
            measured("src/lib.rs", "one", 1, 3),
            measured("src/lib.rs", "two", 4, 6),
        ];

        let notes = score_rust(&mut measurements, &[(0, 2)], Some(&path)).unwrap();

        assert_eq!(measurements[0].mutation, Some(Mutation::Unshown));
        assert_eq!(measurements[1].mutation, Some(Mutation::Unshown));
        assert_eq!(
            notes[0].message,
            "A mutant in this file has no finished outcome."
        );
    }

    #[test]
    fn a_longer_report_path_matches_on_a_boundary_and_two_suffixes_tie() {
        let root = fixture("suffix");
        let path = write_report(
            &root,
            &report(&[&mutant("/repo/src/lib.rs", 2, "MissedMutant")]),
        );
        let mut measurements = vec![measured("src/lib.rs", "classify", 1, 4)];
        score_rust(&mut measurements, &[(0, 1)], Some(&path)).unwrap();
        assert_eq!(
            measurements[0].mutation,
            Some(Mutation::Score {
                score: 0.0,
                line: 2
            })
        );

        let path = write_report(
            &root,
            &report(&[
                &mutant("src/lib.rs", 2, "CaughtMutant"),
                &mutant("other/lib.rs", 2, "MissedMutant"),
            ]),
        );
        let mut measurements = vec![measured("lib.rs", "classify", 1, 4)];
        let notes = score_rust(&mut measurements, &[(0, 1)], Some(&path)).unwrap();
        assert_eq!(measurements[0].mutation, Some(Mutation::Unshown));
        assert_eq!(notes[0].message, "Mutation records tie for this file.");
    }

    #[test]
    fn a_missing_report_names_the_path_and_an_unset_path_names_none() {
        let root = fixture("missing");
        let mut measurements = vec![measured("src/lib.rs", "classify", 1, 3)];
        let notes = score_rust(&mut measurements, &[(0, 1)], None).unwrap();
        assert_eq!(measurements[0].mutation, Some(Mutation::Unshown));
        assert_eq!(notes[0].path, None);
        assert_eq!(notes[0].message, "No mutation report was given.");

        let missing = root.join("outcomes.json");
        let mut measurements = vec![measured("src/lib.rs", "classify", 1, 3)];
        let notes = score_rust(&mut measurements, &[(0, 1)], Some(&missing)).unwrap();
        assert_eq!(measurements[0].mutation, Some(Mutation::Unshown));
        assert!(notes[0].message.contains(&missing.display().to_string()));
        assert!(notes[0].message.starts_with("No mutation report at "));
    }

    #[test]
    fn a_report_that_cannot_be_parsed_is_an_error_and_a_python_range_is_left_alone() {
        let root = fixture("bad");
        let path = write_report(&root, "{");
        let mut measurements = vec![measured("src/lib.rs", "classify", 1, 3)];
        let err = score_rust(&mut measurements, &[(0, 1)], Some(&path)).unwrap_err();
        assert!(matches!(err, MutationError::BadReport(_)));
        assert!(err.to_string().contains("cannot be parsed"));

        let path = write_report(&root, &report(&[&mutant("src/a.py", 2, "MissedMutant")]));
        let mut measurements = vec![measured("src/a.py", "classify", 1, 3)];
        let notes = score_rust(&mut measurements, &[], Some(&path)).unwrap();
        assert!(notes.is_empty());
        assert_eq!(measurements[0].mutation, None);
    }

    #[test]
    fn the_flag_replaces_a_relative_config_path() {
        let root = fixture("path");
        let joined = mutation_path(&root, Some("mutants.out/outcomes.json")).unwrap();
        assert_eq!(joined, root.join("mutants.out/outcomes.json"));
        let absolute = mutation_path(Path::new("/tmp"), Some("/tmp/outcomes.json")).unwrap();
        assert_eq!(absolute, PathBuf::from("/tmp/outcomes.json"));
    }
}
