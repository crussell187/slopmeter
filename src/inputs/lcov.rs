use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::analyzers::{Coverage as Shown, Measurement};
use crate::ir::Span;
use crate::reporters::Note;

const NO_COVERAGE: &str = "No coverage report was given. CRAP was not computed.";
const MISSING: &str = "No coverage record matches this file.";
const TIE: &str = "Coverage records tie for this file.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub path: String,
    pub lines: BTreeMap<u32, bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Coverage {
    pub sections: Vec<Section>,
}

#[derive(Debug)]
pub enum LcovError {
    Read(PathBuf, std::io::Error),
    BadRecord { path: PathBuf, line: usize },
}

impl std::fmt::Display for LcovError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LcovError::Read(path, err) => write!(f, "reading {}: {err}", path.display()),
            LcovError::BadRecord { path, line } => {
                write!(
                    f,
                    "coverage report {}:{} has a bad record",
                    path.display(),
                    line
                )
            }
        }
    }
}

impl std::error::Error for LcovError {}

pub fn read_lcov(path: &Path) -> Result<Coverage, LcovError> {
    let text = fs::read_to_string(path).map_err(|err| LcovError::Read(path.to_path_buf(), err))?;
    parse_lcov(path, &text)
}

pub fn coverage_path(root: &Path, coverage: Option<&str>) -> Option<PathBuf> {
    let coverage = coverage?;
    let path = Path::new(coverage);
    Some(if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    })
}

pub fn cover(
    measurements: &mut [Measurement],
    scored: &[&str],
    report: Option<&Path>,
) -> Result<Vec<Note>, LcovError> {
    for measurement in measurements.iter_mut() {
        measurement.crap = None;
        measurement.coverage = None;
    }
    let Some(report) = report else {
        mark_unshown(measurements);
        return Ok(vec![Note {
            path: None,
            message: NO_COVERAGE.to_string(),
        }]);
    };
    if !report.is_file() {
        mark_unshown(measurements);
        return Ok(vec![Note {
            path: None,
            message: format!(
                "No coverage report at {}. CRAP was not computed.",
                report.display()
            ),
        }]);
    }
    let coverage = read_lcov(report)?;
    Ok(apply(measurements, scored, &coverage))
}

struct Records<'a> {
    path: &'a Path,
    sections: Vec<Section>,
    current: Option<Section>,
}

fn parse_lcov(path: &Path, text: &str) -> Result<Coverage, LcovError> {
    let mut records = Records {
        path,
        sections: Vec::new(),
        current: None,
    };
    for (index, raw) in text.lines().enumerate() {
        read_line(&mut records, index, raw)?;
    }
    close_record(&mut records);
    Ok(Coverage {
        sections: records.sections,
    })
}

fn read_line(records: &mut Records<'_>, index: usize, raw: &str) -> Result<(), LcovError> {
    let line_no = index + 1;
    let line = raw.trim();
    if line.is_empty() {
        return Ok(());
    }
    if line == "end_of_record" {
        close_record(records);
        return Ok(());
    }
    if let Some(rest) = line.strip_prefix("SF:") {
        return read_source(records, line_no, rest);
    }
    if let Some(rest) = line.strip_prefix("DA:") {
        return read_hits(records, line_no, rest);
    }
    Ok(())
}

fn close_record(records: &mut Records<'_>) {
    let current = &mut records.current;
    let Some(section) = current.take() else {
        return;
    };
    push_section(&mut records.sections, section);
}

fn read_source(records: &mut Records<'_>, line_no: usize, rest: &str) -> Result<(), LcovError> {
    close_record(records);
    let sf = normalize(rest.trim());
    if sf.is_empty() {
        return Err(bad_record(records.path, line_no));
    }
    records.current = Some(Section {
        path: sf,
        lines: BTreeMap::new(),
    });
    Ok(())
}

fn read_hits(records: &mut Records<'_>, line_no: usize, rest: &str) -> Result<(), LcovError> {
    let current = &records.current;
    if current.is_none() {
        return Err(bad_record(records.path, line_no));
    }
    let Some((listed, hit)) = parse_da(rest) else {
        return Err(bad_record(records.path, line_no));
    };
    note_hit(records, listed, hit);
    Ok(())
}

fn note_hit(records: &mut Records<'_>, listed: u32, hit: bool) {
    let current = &mut records.current;
    let Some(section) = current.as_mut() else {
        return;
    };
    let lines = &mut section.lines;
    let slot = lines.entry(listed).or_insert(false);
    if hit {
        *slot = true;
    }
}

fn bad_record(path: &Path, line: usize) -> LcovError {
    LcovError::BadRecord {
        path: path.to_path_buf(),
        line,
    }
}

fn parse_da(rest: &str) -> Option<(u32, bool)> {
    let mut parts = rest.split(',');
    let line = parsed_u32(parts.next()?)?;
    let count = parsed_u64(parts.next()?)?;
    if !checksum_ok(&mut parts) {
        return None;
    }
    Some((line, count > 0))
}

fn parsed_u32(field: &str) -> Option<u32> {
    let trimmed = field.trim();
    trimmed.parse().ok()
}

fn parsed_u64(field: &str) -> Option<u64> {
    let trimmed = field.trim();
    trimmed.parse().ok()
}

fn checksum_ok<'a>(parts: &mut impl Iterator<Item = &'a str>) -> bool {
    let third = parts.next();
    let Some(_) = third else {
        return true;
    };
    let fourth = parts.next();
    fourth.is_none()
}

fn push_section(sections: &mut Vec<Section>, section: Section) {
    sections.push(section);
}

fn merge_lines(into: &mut BTreeMap<u32, bool>, from: &BTreeMap<u32, bool>) {
    for (line, hit) in from {
        let slot = into.entry(*line).or_insert(false);
        if *hit {
            *slot = true;
        }
    }
}

struct Owned {
    original: String,
    lines: BTreeMap<u32, bool>,
    tied: bool,
    matched: bool,
}

struct Score<'a> {
    measurements: &'a mut [Measurement],
    described: &'a [(String, String, Span)],
    notes: &'a mut Vec<Note>,
}

#[derive(Clone, Copy)]
struct Hit<'a> {
    file: &'a Owned,
    key: &'a str,
    index: usize,
}

struct Prepared {
    cyclomatic: u32,
    name: String,
    found: Option<(f64, Option<u32>)>,
}

struct Saved {
    index: usize,
    cyclomatic: u32,
    hit_fraction: f64,
    first_miss: Option<u32>,
}

fn apply(measurements: &mut [Measurement], scored: &[&str], coverage: &Coverage) -> Vec<Note> {
    let mut owned = own_files(measurements, scored);
    assign_all(&mut owned, coverage);
    let described = describe(measurements);
    score_owned(measurements, &described, &owned)
}

fn own_files(measurements: &[Measurement], scored: &[&str]) -> Vec<Owned> {
    file_list(measurements, scored)
        .into_iter()
        .map(owned_file)
        .collect()
}

fn owned_file(original: String) -> Owned {
    Owned {
        original,
        lines: BTreeMap::new(),
        tied: false,
        matched: false,
    }
}

fn assign_all(owned: &mut [Owned], coverage: &Coverage) {
    for section in &coverage.sections {
        assign_section(owned, section);
    }
}

fn describe(measurements: &[Measurement]) -> Vec<(String, String, Span)> {
    measurements.iter().map(describe_one).collect()
}

fn describe_one(measurement: &Measurement) -> (String, String, Span) {
    let path = normalize(&measurement.path);
    let name = measurement.name.clone();
    (path, name, measurement.span)
}

fn score_owned(
    measurements: &mut [Measurement],
    described: &[(String, String, Span)],
    owned: &[Owned],
) -> Vec<Note> {
    let mut notes = Vec::new();
    {
        let mut score = Score {
            measurements,
            described,
            notes: &mut notes,
        };
        for file in owned {
            score_file(file, &mut score);
        }
    }
    notes
}

fn score_file(file: &Owned, score: &mut Score<'_>) {
    if file.tied {
        note_gap(score, file, TIE);
        return;
    }
    if !file.matched {
        note_gap(score, file, MISSING);
        return;
    }
    score_matched(file, score);
}

fn note_gap(score: &mut Score<'_>, file: &Owned, message: &str) {
    let key = normalize(&file.original);
    mark_file_unshown(score.measurements, score.described, &key);
    push_note(score, file, message);
}

fn push_note(score: &mut Score<'_>, file: &Owned, message: &str) {
    let path = file.original.clone();
    let notes = &mut score.notes;
    notes.push(Note {
        path: Some(path),
        message: message.to_string(),
    });
}

fn score_matched(file: &Owned, score: &mut Score<'_>) {
    let key = normalize(&file.original);
    let count = measurement_count(score);
    for index in 0..count {
        let hit = Hit {
            file,
            key: &key,
            index,
        };
        cover_at(score, hit);
    }
}

fn measurement_count(score: &Score<'_>) -> usize {
    let measurements = &score.measurements;
    measurements.len()
}

fn cover_at(score: &mut Score<'_>, hit: Hit<'_>) {
    if !path_matches(score.described, hit.index, hit.key) {
        return;
    }
    write_coverage(score, hit);
}

fn path_matches(described: &[(String, String, Span)], index: usize, key: &str) -> bool {
    let path = &described[index].0;
    path == key
}

fn write_coverage(score: &mut Score<'_>, hit: Hit<'_>) {
    let prepared = prepare_coverage(score, hit);
    store_coverage(score, hit, prepared);
}

fn prepare_coverage(score: &Score<'_>, hit: Hit<'_>) -> Prepared {
    let measurement = &score.measurements[hit.index];
    let span = measurement.span;
    let cyclomatic = measurement.cyclomatic;
    let name = name_at(score.described, hit.index);
    let gaps = holes(score.described, hit.index, span);
    let file = hit.file;
    let lines = &file.lines;
    Prepared {
        cyclomatic,
        name,
        found: fraction(lines, span, &gaps),
    }
}

fn name_at(described: &[(String, String, Span)], index: usize) -> String {
    let name = &described[index].1;
    name.clone()
}

fn store_coverage(score: &mut Score<'_>, hit: Hit<'_>, prepared: Prepared) {
    let Some((hit_fraction, first_miss)) = prepared.found else {
        let message = format!("{} has no DA lines.", prepared.name);
        push_note(score, hit.file, &message);
        return;
    };
    save_measurement(
        score,
        Saved {
            index: hit.index,
            cyclomatic: prepared.cyclomatic,
            hit_fraction,
            first_miss,
        },
    );
}

fn save_measurement(score: &mut Score<'_>, saved: Saved) {
    let measurement = &mut score.measurements[saved.index];
    measurement.crap = Some(crap(saved.cyclomatic, saved.hit_fraction));
    let Some(line) = saved.first_miss else {
        return;
    };
    measurement.coverage = Some(Shown::Fraction {
        fraction: saved.hit_fraction,
        line,
    });
}

fn assign_section(owned: &mut [Owned], section: &Section) {
    let matches = matching_indexes(owned, section);
    let Some(best) = longest_match(owned, &matches) else {
        return;
    };
    let winners = winners_of(owned, matches, best);
    if winners.len() > 1 {
        mark_tied(owned, &winners);
        return;
    }
    keep_section(owned, winners[0], section);
}

fn matching_indexes(owned: &[Owned], section: &Section) -> Vec<usize> {
    let mut seen = BTreeSet::new();
    owned
        .iter()
        .enumerate()
        .filter(|(_, file)| newly_matched(section, file, &mut seen))
        .map(|(index, _)| index)
        .collect()
}

fn newly_matched(section: &Section, file: &Owned, seen: &mut BTreeSet<String>) -> bool {
    let key = normalize(&file.original);
    let matched = boundary_suffix(&section.path, &key);
    matched && seen.insert(key)
}

fn longest_match(owned: &[Owned], matches: &[usize]) -> Option<usize> {
    matches.iter().map(|index| path_len(owned, *index)).max()
}

fn path_len(owned: &[Owned], index: usize) -> usize {
    let path = &owned[index].original;
    normalize(path).len()
}

fn winners_of(owned: &[Owned], matches: Vec<usize>, best: usize) -> Vec<usize> {
    matches
        .into_iter()
        .filter(|index| path_len(owned, *index) == best)
        .collect()
}

fn mark_tied(owned: &mut [Owned], winners: &[usize]) {
    for index in winners {
        clear_file(&mut owned[*index]);
    }
}

fn clear_file(file: &mut Owned) {
    file.tied = true;
    file.matched = false;
    let lines = &mut file.lines;
    lines.clear();
}

fn keep_section(owned: &mut [Owned], index: usize, section: &Section) {
    if owned[index].tied {
        return;
    }
    if owned[index].matched {
        clear_file(&mut owned[index]);
        return;
    }
    let lines = &mut owned[index].lines;
    merge_lines(lines, &section.lines);
    owned[index].matched = true;
}

fn file_list(measurements: &[Measurement], scored: &[&str]) -> Vec<String> {
    let mut files = Vec::new();
    for path in scored {
        push_file(&mut files, path);
    }
    for measurement in measurements {
        push_file(&mut files, &measurement.path);
    }
    files
}

fn push_file(files: &mut Vec<String>, path: &str) {
    let key = normalize(path);
    if key.is_empty() || files.iter().any(|have| normalize(have) == key) {
        return;
    }
    files.push(path.to_string());
}

fn mark_unshown(measurements: &mut [Measurement]) {
    for measurement in measurements {
        measurement.coverage = Some(Shown::Unshown);
    }
}

fn mark_file_unshown(
    measurements: &mut [Measurement],
    described: &[(String, String, Span)],
    key: &str,
) {
    for (index, measurement) in measurements.iter_mut().enumerate() {
        if described[index].0 == key {
            measurement.coverage = Some(Shown::Unshown);
        }
    }
}

fn holes(described: &[(String, String, Span)], index: usize, span: Span) -> Vec<Span> {
    let key = &described[index].0;
    described
        .iter()
        .enumerate()
        .filter(|(other, (path, _, other_span))| {
            *other != index && path == key && strictly_inside(*other_span, span)
        })
        .map(|(_, (_, _, other_span))| *other_span)
        .collect()
}

fn strictly_inside(inner: Span, outer: Span) -> bool {
    inner.start >= outer.start
        && inner.end <= outer.end
        && (inner.start > outer.start || inner.end < outer.end)
}

struct Tally {
    hit: u32,
    first_miss: Option<u32>,
}

fn fraction(lines: &BTreeMap<u32, bool>, span: Span, holes: &[Span]) -> Option<(f64, Option<u32>)> {
    let mut total = 0u32;
    let mut tally = Tally {
        hit: 0,
        first_miss: None,
    };
    for (line, was_hit) in lines {
        if !line_counts(*line, span, holes) {
            continue;
        }
        total += 1;
        note_line(&mut tally, *was_hit, *line);
    }
    finish_fraction(tally.hit, total, tally.first_miss)
}

fn line_counts(line: u32, span: Span, holes: &[Span]) -> bool {
    if line < span.start || line > span.end {
        return false;
    }
    !hole_contains(line, holes)
}

fn hole_contains(line: u32, holes: &[Span]) -> bool {
    holes.iter().any(|hole| span_contains(*hole, line))
}

fn span_contains(hole: Span, line: u32) -> bool {
    line >= hole.start && line <= hole.end
}

fn note_line(tally: &mut Tally, was_hit: bool, line: u32) {
    if was_hit {
        tally.hit += 1;
        return;
    }
    let miss = &tally.first_miss;
    if miss.is_some() {
        return;
    }
    tally.first_miss = Some(line);
}

fn finish_fraction(hit: u32, total: u32, first_miss: Option<u32>) -> Option<(f64, Option<u32>)> {
    if total == 0 {
        return None;
    }
    Some((f64::from(hit) / f64::from(total), first_miss))
}

fn crap(cyclomatic: u32, coverage: f64) -> f64 {
    let cc = f64::from(cyclomatic);
    let uncovered = 1.0 - coverage;
    cc * cc * uncovered * uncovered * uncovered + cc
}

fn boundary_suffix(sf: &str, scored: &str) -> bool {
    if scored.is_empty() || sf.len() < scored.len() || !sf.ends_with(scored) {
        return false;
    }
    sf.len() == scored.len() || sf.as_bytes()[sf.len() - scored.len() - 1] == b'/'
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
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use crate::analyzers::{Coverage, Measurement};
    use crate::inputs::config::{load_config, Config};
    use crate::inputs::lcov::{cover, coverage_path, read_lcov};
    use crate::ir::Span;
    use crate::reporters::Note;
    use crate::rules::evaluate;

    fn fixture(name: &str) -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("lcov-fixtures")
            .join(format!(
                "{name}-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_report(root: &Path, name: &str, body: &str) -> PathBuf {
        let path = root.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, body).unwrap();
        path
    }

    fn measured(path: &str, name: &str, start: u32, end: u32, cyclomatic: u32) -> Measurement {
        Measurement {
            path: path.to_string(),
            name: name.to_string(),
            param_types: vec![],
            qualifier: String::new(),
            span: Span { start, end },
            cyclomatic,
            cognitive: 0,
            nesting: 0,
            nesting_line: None,
            params: 0,
            lloc: 0,
            receiver_chain: None,
            other_chain: None,
            forward: None,
            waivers: vec![],
            crap: None,
            coverage: None,
            mutation: None,
        }
    }

    #[test]
    fn half_coverage_of_a_complex_function_is_the_published_crap() {
        let root = fixture("half");
        let report = write_report(
            &root,
            "coverage.lcov",
            "SF:/proj/src/a.py\nDA:1,1,deadbeef\nDA:2,0\nFN:1,other\nend_of_record\n",
        );
        let mut measurements = vec![measured("src/a.py", "classify", 1, 4, 10)];

        let notes = cover(&mut measurements, &["src/a.py"], Some(&report)).unwrap();

        assert!(notes.is_empty());
        assert_eq!(measurements[0].crap, Some(22.5));
    }

    #[test]
    fn lines_outside_the_body_span_do_not_count() {
        let root = fixture("outside");
        let report = write_report(
            &root,
            "coverage.lcov",
            "SF:src/a.py\nDA:1,0\nDA:9,1\nend_of_record\n",
        );
        let mut measurements = vec![measured("src/a.py", "classify", 1, 2, 2)];

        cover(&mut measurements, &["src/a.py"], Some(&report)).unwrap();

        assert_eq!(measurements[0].crap, Some(6.0));
    }

    #[test]
    fn all_zero_counts_run_crap_and_can_fail_f02() {
        let root = fixture("zero");
        let report = write_report(
            &root,
            "coverage.lcov",
            "SF:src/a.py\nDA:1,0\nDA:2,0\nend_of_record\n",
        );
        let mut measurements = vec![measured("src/a.py", "classify", 1, 2, 3)];

        cover(&mut measurements, &["src/a.py"], Some(&report)).unwrap();
        let findings = evaluate(&measurements, &Config::builtin().python);

        assert_eq!(measurements[0].crap, Some(12.0));
        assert!(findings
            .iter()
            .any(|finding| { finding.rule == "F02" && finding.measured_value == Some(12.0) }));
        assert!(findings.iter().any(|finding| {
            finding.rule == "S06" && finding.line == 1 && finding.measured_value == Some(0.0)
        }));
    }

    #[test]
    fn full_coverage_leaves_crap_equal_to_cyclomatic_complexity() {
        let root = fixture("full");
        let report = write_report(
            &root,
            "coverage.lcov",
            "SF:src/a.py\nDA:1,4\nDA:2,1\nend_of_record\n",
        );
        let mut measurements = vec![measured("src/a.py", "classify", 1, 2, 10)];

        cover(&mut measurements, &["src/a.py"], Some(&report)).unwrap();

        assert_eq!(measurements[0].crap, Some(10.0));
        assert_eq!(measurements[0].coverage, None);
    }

    #[test]
    fn a_nested_function_keeps_its_lines() {
        let root = fixture("nested");
        let report = write_report(
            &root,
            "coverage.lcov",
            "SF:src/a.py\nDA:1,1\nDA:4,0\nDA:5,1\nDA:8,1\nend_of_record\n",
        );
        let mut measurements = vec![
            measured("src/a.py", "outer", 1, 10, 4),
            measured("src/a.py", "inner", 4, 6, 2),
        ];

        let notes = cover(&mut measurements, &["src/a.py"], Some(&report)).unwrap();

        assert!(notes.is_empty());
        assert_eq!(measurements[0].crap, Some(4.0));
        assert_eq!(measurements[0].coverage, None);
        assert_eq!(measurements[1].crap, Some(2.5));
        assert_eq!(
            measurements[1].coverage,
            Some(Coverage::Fraction {
                fraction: 0.5,
                line: 4,
            })
        );
    }

    #[test]
    fn a_line_inside_the_parent_span_stays_with_that_function() {
        let root = fixture("lambda");
        let report = write_report(
            &root,
            "coverage.lcov",
            "SF:src/a.py\nDA:2,1\nDA:4,1\nend_of_record\n",
        );
        let mut measurements = vec![measured("src/a.py", "outer", 1, 5, 1)];

        cover(&mut measurements, &["src/a.py"], Some(&report)).unwrap();

        assert_eq!(measurements[0].crap, Some(1.0));
    }

    #[test]
    fn function_records_do_not_change_the_fraction() {
        let root = fixture("names");
        let report = write_report(
            &root,
            "coverage.lcov",
            "\
SF:src/a.py
FN:1,other
FNDA:0,other
FNL:1,2,other
FNA:0,other
BRDA:1,0,0,1
DA:1,1
end_of_record
",
        );
        let mut measurements = vec![measured("src/a.py", "classify", 1, 1, 5)];

        cover(&mut measurements, &["src/a.py"], Some(&report)).unwrap();

        assert_eq!(measurements[0].crap, Some(5.0));
    }

    #[test]
    fn no_listed_lines_in_the_span_leave_crap_unset() {
        let root = fixture("empty-span");
        let report = write_report(
            &root,
            "coverage.lcov",
            "SF:src/a.py\nDA:9,1\nend_of_record\n",
        );
        let mut measurements = vec![
            measured("src/a.py", "classify", 1, 2, 4),
            measured("src/a.py", "other", 9, 9, 1),
        ];

        let notes = cover(&mut measurements, &["src/a.py"], Some(&report)).unwrap();

        assert_eq!(measurements[0].crap, None);
        assert_eq!(measurements[0].coverage, None);
        assert_eq!(measurements[1].crap, Some(1.0));
        assert_eq!(
            notes,
            vec![Note {
                path: Some("src/a.py".to_string()),
                message: "classify has no DA lines.".to_string(),
            }]
        );
    }

    #[test]
    fn a_file_missing_from_the_report_is_noted_and_another_file_still_scores() {
        let root = fixture("missing-file");
        let report = write_report(
            &root,
            "coverage.lcov",
            "SF:/build/src/b.py\nDA:1,1\nend_of_record\n",
        );
        let mut measurements = vec![
            measured("src/a.py", "absent", 1, 1, 3),
            measured("src/b.py", "present", 1, 1, 1),
        ];

        let notes = cover(&mut measurements, &["src/a.py", "src/b.py"], Some(&report)).unwrap();

        assert_eq!(measurements[0].crap, None);
        assert_eq!(measurements[0].coverage, Some(Coverage::Unshown));
        assert_eq!(measurements[1].crap, Some(1.0));
        assert_eq!(
            notes,
            vec![Note {
                path: Some("src/a.py".to_string()),
                message: "No coverage record matches this file.".to_string(),
            }]
        );
    }

    #[test]
    fn the_longer_scored_path_takes_the_record() {
        let root = fixture("longer");
        let report = write_report(
            &root,
            "coverage.lcov",
            "\
SF:/one/src/a.py
DA:1,0
DA:2,1
end_of_record
",
        );
        let mut measurements = vec![
            measured("a.py", "short", 1, 2, 1),
            measured("src/a.py", "long", 1, 2, 1),
        ];

        let notes = cover(&mut measurements, &["a.py", "src/a.py"], Some(&report)).unwrap();

        assert_eq!(measurements[0].crap, None);
        assert_eq!(measurements[1].crap, Some(1.125));
        assert_eq!(
            notes,
            vec![Note {
                path: Some("a.py".to_string()),
                message: "No coverage record matches this file.".to_string(),
            }]
        );
    }

    #[test]
    fn two_records_that_suffix_match_one_file_leave_coverage_absent() {
        let root = fixture("tie");
        let report = write_report(
            &root,
            "coverage.lcov",
            "\
SF:/one/src/a.py
DA:1,0
end_of_record
SF:/two/src/a.py
DA:1,1
end_of_record
",
        );
        let mut measurements = vec![measured("src/a.py", "run", 1, 1, 1)];

        let notes = cover(&mut measurements, &["src/a.py"], Some(&report)).unwrap();

        assert_eq!(measurements[0].crap, None);
        assert_eq!(measurements[0].coverage, Some(Coverage::Unshown));
        assert_eq!(
            notes,
            vec![Note {
                path: Some("src/a.py".to_string()),
                message: "Coverage records tie for this file.".to_string(),
            }]
        );
    }

    #[test]
    fn a_header_is_covered_only_by_its_own_record() {
        let root = fixture("header");
        let report = write_report(
            &root,
            "coverage.lcov",
            "SF:src/foo.cpp\nDA:1,1\nend_of_record\n",
        );
        let mut measurements = vec![
            measured("include/foo.hpp", "widget", 1, 1, 2),
            measured("src/foo.cpp", "widget", 1, 1, 2),
        ];

        let notes = cover(
            &mut measurements,
            &["include/foo.hpp", "src/foo.cpp"],
            Some(&report),
        )
        .unwrap();

        assert_eq!(measurements[0].crap, None);
        assert_eq!(measurements[1].crap, Some(2.0));
        assert_eq!(
            notes,
            vec![Note {
                path: Some("include/foo.hpp".to_string()),
                message: "No coverage record matches this file.".to_string(),
            }]
        );
    }

    #[test]
    fn no_report_leaves_crap_unset_and_adds_the_run_note() {
        let mut measurements = vec![measured("src/a.py", "classify", 1, 2, 4)];
        measurements[0].crap = Some(1.0);

        let notes = cover(&mut measurements, &["src/a.py"], None).unwrap();

        assert_eq!(measurements[0].crap, None);
        assert_eq!(measurements[0].coverage, Some(Coverage::Unshown));
        assert_eq!(
            notes,
            vec![Note {
                path: None,
                message: "No coverage report was given. CRAP was not computed.".to_string(),
            }]
        );
    }

    #[test]
    fn a_missing_report_path_names_that_path() {
        let root = fixture("absent-report");
        let mut measurements = vec![measured("src/a.py", "classify", 1, 1, 1)];

        let notes = cover(
            &mut measurements,
            &["src/a.py"],
            Some(&root.join("missing.lcov")),
        )
        .unwrap();

        assert_eq!(measurements[0].crap, None);
        assert_eq!(
            notes,
            vec![Note {
                path: None,
                message: format!(
                    "No coverage report at {}. CRAP was not computed.",
                    root.join("missing.lcov").display()
                ),
            }]
        );
    }

    #[test]
    fn the_coverage_flag_replaces_the_file_path() {
        let root = fixture("flag");
        fs::write(root.join("slopmeter.toml"), "coverage = \"missing.lcov\"\n").unwrap();
        let report = write_report(
            &root,
            "target/real.lcov",
            "SF:src/a.py\nDA:1,1\nend_of_record\n",
        );
        let mut config = load_config(&root).unwrap();
        config.set_coverage_override(report.to_string_lossy());
        let path = coverage_path(&root, config.coverage.as_deref());
        let mut measurements = vec![measured("src/a.py", "run", 1, 1, 1)];

        let notes = cover(&mut measurements, &["src/a.py"], path.as_deref()).unwrap();

        assert!(notes.is_empty());
        assert_eq!(measurements[0].crap, Some(1.0));
    }

    #[test]
    fn a_repo_relative_report_is_read_from_the_root() {
        let root = fixture("relative");
        fs::write(
            root.join("slopmeter.toml"),
            "coverage = \"coverage.lcov\"\n",
        )
        .unwrap();
        write_report(
            &root,
            "coverage.lcov",
            "SF:src\\a.py\nDA:1,1\nend_of_record\n",
        );
        let config = load_config(&root).unwrap();
        let path = coverage_path(&root, config.coverage.as_deref());
        let mut measurements = vec![measured("src/a.py", "run", 1, 1, 2)];

        cover(&mut measurements, &["src/a.py"], path.as_deref()).unwrap();

        assert_eq!(measurements[0].crap, Some(2.0));
    }

    #[test]
    fn a_bad_da_record_fails_the_read() {
        let root = fixture("bad");
        let report = write_report(&root, "coverage.lcov", "SF:src/a.py\nDA:nope\n");

        let error = read_lcov(&report).unwrap_err();

        assert_eq!(
            error.to_string(),
            format!("coverage report {}:2 has a bad record", report.display())
        );
    }

    #[test]
    fn dot_segments_in_the_source_path_resolve_before_the_join() {
        let root = fixture("dots");
        let report = write_report(
            &root,
            "coverage.lcov",
            "SF:/proj/src/../src/./a.py\nDA:1,1\nend_of_record\n",
        );
        let mut measurements = vec![measured("src/a.py", "run", 1, 1, 1)];

        let notes = cover(&mut measurements, &["src/a.py"], Some(&report)).unwrap();

        assert!(notes.is_empty());
        assert_eq!(measurements[0].crap, Some(1.0));
    }

    #[test]
    fn an_empty_checksum_is_a_listed_line() {
        let root = fixture("checksum");
        let report = write_report(
            &root,
            "coverage.lcov",
            "SF:src/a.py\nDA:1,1,\nend_of_record\n",
        );
        let mut measurements = vec![measured("src/a.py", "run", 1, 1, 1)];

        let notes = cover(&mut measurements, &["src/a.py"], Some(&report)).unwrap();

        assert!(notes.is_empty());
        assert_eq!(measurements[0].crap, Some(1.0));
    }

    #[test]
    fn duplicate_sections_for_one_file_are_a_tie() {
        let root = fixture("dup");
        let report = write_report(
            &root,
            "coverage.lcov",
            "\
SF:/proj/src/a.py
DA:1,0
end_of_record
SF:/proj/src/a.py
DA:1,3
end_of_record
",
        );
        let coverage = read_lcov(&report).unwrap();
        let mut measurements = vec![measured("src/a.py", "run", 1, 1, 1)];

        let notes = cover(&mut measurements, &["src/a.py"], Some(&report)).unwrap();

        assert_eq!(coverage.sections.len(), 2);
        assert_eq!(measurements[0].crap, None);
        assert_eq!(measurements[0].coverage, Some(Coverage::Unshown));
        assert_eq!(
            notes,
            vec![Note {
                path: Some("src/a.py".to_string()),
                message: "Coverage records tie for this file.".to_string(),
            }]
        );
    }

    #[test]
    fn a_da_record_before_any_source_file_fails() {
        let root = fixture("early");
        let report = write_report(&root, "coverage.lcov", "DA:1,1\n");

        let error = read_lcov(&report).unwrap_err();

        assert!(error.to_string().contains("1 has a bad record"));
    }

    #[test]
    fn an_empty_source_file_record_fails() {
        let root = fixture("empty-sf");
        let report = write_report(&root, "coverage.lcov", "SF:\nDA:1,1\n");

        let error = read_lcov(&report).unwrap_err();

        assert!(error.to_string().contains("1 has a bad record"));
    }
}
