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

pub fn score_python(
    measurements: &mut [Measurement],
    python: &[(usize, usize)],
    report: Option<&Path>,
    source_root: &Path,
) -> Result<Vec<Note>, MutationError> {
    for (start, end) in python {
        for measurement in &mut measurements[*start..*end] {
            measurement.mutation = None;
        }
    }
    let Some(report) = report else {
        if python.is_empty() {
            return Ok(Vec::new());
        }
        mark_unshown(measurements, python);
        return Ok(vec![Note {
            path: None,
            message: NO_REPORT.to_string(),
        }]);
    };
    if !report.exists() {
        if !python.is_empty() {
            mark_unshown(measurements, python);
        }
        return Ok(vec![Note {
            path: None,
            message: format!("No mutation report at {}.", report.display()),
        }]);
    }
    if !report.is_dir() {
        return Err(MutationError::BadReport(report.to_path_buf()));
    }
    let mut records = read_store(report)?;
    if python.is_empty() {
        return Ok(Vec::new());
    }
    resolve_lines(&mut records, measurements, python, report, source_root)?;
    Ok(apply(measurements, python, &records))
}

pub fn score_cpp(
    measurements: &mut [Measurement],
    cpp: &[(usize, usize)],
    report: Option<&Path>,
) -> Result<Vec<Note>, MutationError> {
    for (start, end) in cpp {
        for measurement in &mut measurements[*start..*end] {
            measurement.mutation = None;
        }
    }
    let Some(report) = report else {
        if cpp.is_empty() {
            return Ok(Vec::new());
        }
        mark_unshown(measurements, cpp);
        return Ok(vec![Note {
            path: None,
            message: NO_REPORT.to_string(),
        }]);
    };
    if !report.is_file() {
        if !cpp.is_empty() {
            mark_unshown(measurements, cpp);
        }
        return Ok(vec![Note {
            path: None,
            message: format!("No mutation report at {}.", report.display()),
        }]);
    }
    let records = read_elements(report)?;
    if cpp.is_empty() {
        return Ok(Vec::new());
    }
    Ok(apply(measurements, cpp, &records))
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
        locator: None,
    }))
}

fn start_line(mutant: &Value) -> Option<u32> {
    let line = mutant.pointer("/span/start/line")?.as_u64()?;
    u32::try_from(line).ok().filter(|line| *line >= 1)
}

fn read_elements(path: &Path) -> Result<Vec<Record>, MutationError> {
    let text =
        fs::read_to_string(path).map_err(|err| MutationError::Read(path.to_path_buf(), err))?;
    let value: Value =
        serde_json::from_str(&text).map_err(|_| MutationError::BadReport(path.to_path_buf()))?;
    let files = value
        .get("files")
        .and_then(Value::as_object)
        .ok_or_else(|| MutationError::BadReport(path.to_path_buf()))?;
    let mut records = Vec::new();
    for (file, body) in files {
        let mutants = body
            .get("mutants")
            .and_then(Value::as_array)
            .ok_or_else(|| MutationError::BadReport(path.to_path_buf()))?;
        for mutant in mutants {
            records.push(element_record(path, file, mutant)?);
        }
    }
    Ok(records)
}

fn element_record(path: &Path, file: &str, mutant: &Value) -> Result<Record, MutationError> {
    let status = mutant
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| MutationError::BadReport(path.to_path_buf()))?;
    let file = normalize(file);
    if file.is_empty() {
        return Err(MutationError::BadReport(path.to_path_buf()));
    }
    let line = element_line(mutant);
    Ok(Record {
        file,
        line,
        kind: element_kind(status, line),
        locator: None,
    })
}

fn element_line(mutant: &Value) -> Option<u32> {
    let line = mutant.pointer("/location/start/line")?.as_u64()?;
    u32::try_from(line).ok().filter(|line| *line >= 1)
}

fn element_kind(status: &str, line: Option<u32>) -> Kind {
    if line.is_none() {
        return Kind::Unfinished;
    }
    match status {
        "Killed" | "Timeout" => Kind::Detected,
        "Survived" | "NoCoverage" => Kind::Undetected,
        "CompileError" | "RuntimeError" => Kind::LeftOut,
        _ => Kind::Unfinished,
    }
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
    locator: Option<Locator>,
}

struct Locator {
    mutant: String,
    orig: String,
    plain: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Detected,
    Undetected,
    LeftOut,
    Unfinished,
    Present,
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
                Kind::LeftOut | Kind::Unfinished | Kind::Present => {}
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

fn read_store(dir: &Path) -> Result<Vec<Record>, MutationError> {
    let mut metas = Vec::new();
    collect_meta(dir, &mut metas)?;
    let mut records = Vec::new();
    for meta in metas {
        records.extend(read_meta(dir, &meta)?);
    }
    Ok(records)
}

fn collect_meta(dir: &Path, metas: &mut Vec<PathBuf>) -> Result<(), MutationError> {
    let entries = fs::read_dir(dir).map_err(|err| MutationError::Read(dir.to_path_buf(), err))?;
    for entry in entries {
        let entry = entry.map_err(|err| MutationError::Read(dir.to_path_buf(), err))?;
        let path = entry.path();
        if ignored_store_path(&path) {
            continue;
        }
        let kind = entry
            .file_type()
            .map_err(|err| MutationError::Read(path.clone(), err))?;
        if kind.is_dir() {
            collect_meta(&path, metas)?;
        } else if path.extension().is_some_and(|ext| ext == "meta") {
            metas.push(path);
        }
    }
    Ok(())
}

fn ignored_store_path(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == "export-cicd-stats")
        || path
            .file_name()
            .is_some_and(|name| name == "mutmut-cicd-stats.json")
}

fn read_meta(root: &Path, meta: &Path) -> Result<Vec<Record>, MutationError> {
    let text =
        fs::read_to_string(meta).map_err(|err| MutationError::Read(meta.to_path_buf(), err))?;
    let value: Value =
        serde_json::from_str(&text).map_err(|_| MutationError::BadReport(meta.to_path_buf()))?;
    let codes = value
        .get("exit_code_by_key")
        .and_then(Value::as_object)
        .ok_or_else(|| MutationError::BadReport(meta.to_path_buf()))?;
    let file = meta
        .strip_prefix(root)
        .ok()
        .and_then(|path| path.to_str())
        .and_then(|path| path.strip_suffix(".meta"))
        .map(normalize)
        .filter(|file| !file.is_empty())
        .ok_or_else(|| MutationError::BadReport(meta.to_path_buf()))?;
    if codes.is_empty() {
        return Ok(vec![Record {
            file,
            line: None,
            kind: Kind::Present,
            locator: None,
        }]);
    }
    let mut records = Vec::new();
    for (key, code) in codes {
        let exit = exit_code(code).map_err(|_| MutationError::BadReport(meta.to_path_buf()))?;
        let kind = python_kind(exit);
        let locator = mutant_locator(key);
        records.push(Record {
            file: file.clone(),
            line: None,
            kind: if locator.is_none() {
                Kind::Unfinished
            } else {
                kind
            },
            locator,
        });
    }
    Ok(records)
}

fn exit_code(value: &Value) -> Result<Option<i64>, ()> {
    if value.is_null() {
        return Ok(None);
    }
    value.as_i64().map(Some).ok_or(())
}

fn python_kind(code: Option<i64>) -> Kind {
    match code {
        Some(1 | 3) => Kind::Detected,
        Some(0 | 5 | 33) => Kind::Undetected,
        Some(36 | 24 | 152 | 255 | -24) => Kind::Detected,
        Some(37) => Kind::LeftOut,
        _ => Kind::Unfinished,
    }
}

fn mutant_locator(key: &str) -> Option<Locator> {
    let mutant = key.split('.').find(|part| {
        (part.starts_with("x_") || part.starts_with("x\u{01c1}")) && part.contains("__mutmut_")
    })?;
    let (prefix, _) = mutant.split_once("__mutmut_")?;
    let plain = if prefix.contains('\u{01c1}') {
        prefix.rsplit('\u{01c1}').next()?
    } else {
        prefix.strip_prefix("x_")?
    };
    if plain.is_empty() {
        return None;
    }
    Some(Locator {
        mutant: mutant.to_string(),
        orig: format!("{prefix}__mutmut_orig"),
        plain: plain.to_string(),
    })
}

fn resolve_lines(
    records: &mut [Record],
    measurements: &[Measurement],
    ranges: &[(usize, usize)],
    mutants_dir: &Path,
    source_root: &Path,
) -> Result<(), MutationError> {
    let files = scored_files(measurements, ranges);
    let mut jobs = Vec::new();
    for file in &files {
        let pointers: Vec<*const Record> = match choose(records, &file.key) {
            Choice::Rows(rows) => rows
                .into_iter()
                .filter(|row| row.locator.is_some())
                .map(std::ptr::from_ref)
                .collect(),
            _ => continue,
        };
        for (index, record) in records.iter().enumerate() {
            if pointers.contains(&std::ptr::from_ref(record)) {
                jobs.push((index, file.path.clone()));
            }
        }
    }
    let mut sources = BTreeMap::<String, Option<String>>::new();
    let mut copies = BTreeMap::<String, StoreCopy>::new();
    for (index, scored_path) in &jobs {
        if !sources.contains_key(scored_path) {
            sources.insert(
                scored_path.clone(),
                read_optional(&source_root.join(scored_path))?,
            );
        }
        let store_file = records[*index].file.clone();
        if !copies.contains_key(&store_file) {
            copies.insert(store_file.clone(), load_copy(mutants_dir, &store_file)?);
        }
    }
    for (index, scored_path) in jobs {
        let Some(locator) = records[index].locator.take() else {
            continue;
        };
        let source = sources.get(&scored_path).and_then(Option::as_deref);
        let copy = copies.get(&records[index].file);
        let line = source
            .zip(copy)
            .and_then(|(source, copy)| recover_line(source, copy, &locator));
        match line {
            Some(line) => records[index].line = Some(line),
            None => {
                records[index].kind = Kind::Unfinished;
                records[index].line = None;
            }
        }
    }
    Ok(())
}

fn read_optional(path: &Path) -> Result<Option<String>, MutationError> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(MutationError::Read(path.to_path_buf(), err)),
    }
}

struct StoreCopy {
    mutated: Option<String>,
    spans: Option<BTreeMap<String, (u32, u32)>>,
}

fn load_copy(dir: &Path, file: &str) -> Result<StoreCopy, MutationError> {
    let mutated_path = dir.join(file);
    let mutated = read_optional(&mutated_path)?;
    let spans_path = dir.join(format!("{file}.spans"));
    let spans = match read_optional(&spans_path)? {
        None => None,
        Some(text) => Some(read_spans(&spans_path, &text)?),
    };
    Ok(StoreCopy { mutated, spans })
}

fn read_spans(path: &Path, text: &str) -> Result<BTreeMap<String, (u32, u32)>, MutationError> {
    let value: Value =
        serde_json::from_str(text).map_err(|_| MutationError::BadReport(path.to_path_buf()))?;
    if value.get("version").and_then(Value::as_u64) != Some(1) {
        return Ok(BTreeMap::new());
    }
    let spans = value
        .get("spans")
        .and_then(Value::as_object)
        .ok_or_else(|| MutationError::BadReport(path.to_path_buf()))?;
    let mut parsed = BTreeMap::new();
    for (name, span) in spans {
        let Some(pair) = span.as_array() else {
            return Err(MutationError::BadReport(path.to_path_buf()));
        };
        let Some([start, end]) = pair.get(0..2) else {
            return Err(MutationError::BadReport(path.to_path_buf()));
        };
        let (Some(start), Some(end)) = (start.as_u64(), end.as_u64()) else {
            return Err(MutationError::BadReport(path.to_path_buf()));
        };
        let (Ok(start), Ok(end)) = (u32::try_from(start), u32::try_from(end)) else {
            return Err(MutationError::BadReport(path.to_path_buf()));
        };
        parsed.insert(name.clone(), (start, end));
    }
    Ok(parsed)
}

fn recover_line(source: &str, copy: &StoreCopy, locator: &Locator) -> Option<u32> {
    let mutated = copy.mutated.as_deref()?;
    let orig = function_lines(mutated, copy, &locator.orig)?;
    let mutant = function_lines(mutated, copy, &locator.mutant)?;
    let offset = first_diff(
        &orig,
        &mutant,
        &locator.orig,
        &locator.mutant,
        &locator.plain,
    )?;
    let mut restored = orig;
    trim_trailing_blank(&mut restored);
    let def = restored
        .iter()
        .position(|line| line.contains(&locator.orig))?;
    restored[def] = restored[def].replacen(&locator.orig, &locator.plain, 1);
    let start = find_block(source, &restored)?;
    start.checked_add(offset)
}

fn function_lines(mutated: &str, copy: &StoreCopy, name: &str) -> Option<Vec<String>> {
    if let Some(spans) = &copy.spans {
        if let Some((start, end)) = spans.get(name) {
            return slice_lines(mutated, *start, *end);
        }
    }
    slice_def(mutated, name)
}

fn slice_lines(source: &str, start: u32, end: u32) -> Option<Vec<String>> {
    if start < 1 || end < start {
        return None;
    }
    let lines: Vec<&str> = source.lines().collect();
    let start = usize::try_from(start).ok()?;
    let end = usize::try_from(end).ok()?;
    if end > lines.len() {
        return None;
    }
    let mut lines: Vec<String> = lines[start - 1..end]
        .iter()
        .map(|line| (*line).to_string())
        .collect();
    trim_trailing_blank(&mut lines);
    Some(lines)
}

fn slice_def(source: &str, name: &str) -> Option<Vec<String>> {
    let lines: Vec<&str> = source.lines().collect();
    let start = lines.iter().position(|line| is_def_line(line, name))?;
    let mut from = start;
    while from > 0 && lines[from - 1].trim_start().starts_with('@') {
        from -= 1;
    }
    let indent = indent_of(lines[start]);
    let mut end = start + 1;
    while end < lines.len() {
        let line = lines[end];
        if line.trim().is_empty() {
            end += 1;
            continue;
        }
        if indent_of(line) <= indent {
            break;
        }
        end += 1;
    }
    while end > start + 1 && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    Some(
        lines[from..end]
            .iter()
            .map(|line| (*line).to_string())
            .collect(),
    )
}

fn is_def_line(line: &str, name: &str) -> bool {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix("async ").unwrap_or(trimmed);
    let Some(rest) = rest.strip_prefix("def ") else {
        return false;
    };
    let Some(rest) = rest.strip_prefix(name) else {
        return false;
    };
    rest.starts_with(['(', ':', ' ', '\t'])
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn first_diff(
    orig: &[String],
    mutant: &[String],
    orig_name: &str,
    mutant_name: &str,
    plain: &str,
) -> Option<u32> {
    let mut orig = orig.to_vec();
    let mut mutant = mutant.to_vec();
    trim_trailing_blank(&mut orig);
    trim_trailing_blank(&mut mutant);
    let width = orig.len().max(mutant.len());
    for index in 0..width {
        let left = orig
            .get(index)
            .map(|line| line.replacen(orig_name, plain, 1));
        let right = mutant
            .get(index)
            .map(|line| line.replacen(mutant_name, plain, 1));
        if left != right {
            return u32::try_from(index).ok();
        }
    }
    None
}

fn trim_trailing_blank(lines: &mut Vec<String>) {
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
}

fn find_block(source: &str, block: &[String]) -> Option<u32> {
    if block.is_empty() {
        return None;
    }
    let lines: Vec<&str> = source.lines().collect();
    if block.len() > lines.len() {
        return None;
    }
    let mut found = None;
    for start in 0..=lines.len() - block.len() {
        let same = lines[start..start + block.len()]
            .iter()
            .zip(block)
            .all(|(have, want)| *have == want);
        if !same {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = u32::try_from(start).ok().map(|start| start + 1);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::{mutation_path, score_cpp, score_python, score_rust, MutationError};
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

    fn write_python(root: &Path, file: &str, source: &str, mutated: &str, meta: &str) -> PathBuf {
        fs::create_dir_all(root.join(file).parent().unwrap()).unwrap();
        fs::write(root.join(file), source).unwrap();
        let mutants = root.join("mutants");
        fs::create_dir_all(mutants.join(file).parent().unwrap()).unwrap();
        fs::write(mutants.join(file), mutated).unwrap();
        fs::write(mutants.join(format!("{file}.meta")), meta).unwrap();
        mutants
    }

    fn spans_for(root: &Path, file: &str, body: &str) {
        fs::write(root.join("mutants").join(format!("{file}.spans")), body).unwrap();
    }

    fn meta(entries: &[(&str, &str)]) -> String {
        let body = entries
            .iter()
            .map(|(key, code)| format!("\"{key}\": {code}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("{{\"exit_code_by_key\": {{{body}}}}}")
    }

    const CLASSIFY: &str = "def classify(n):\n    return n > 0\n";

    fn mutated_classify() -> String {
        "\
def x_classify__mutmut_orig(n):
    return n > 0

def x_classify__mutmut_1(n):
    return n < 0

def x_classify__mutmut_2(n):
    return n >= 0

def x_classify__mutmut_3(n):
    return n > 1

def x_classify__mutmut_4(n):
    return n == 0
"
        .to_string()
    }

    fn classify_spans() -> &'static str {
        r#"{"version":1,"spans":{"x_classify__mutmut_orig":[1,2],"x_classify__mutmut_1":[4,5],"x_classify__mutmut_2":[7,8],"x_classify__mutmut_3":[10,11],"x_classify__mutmut_4":[13,14]}}"#
    }

    #[test]
    fn killed_and_survived_score_one_half_and_timeout_and_type_check_stay_in_their_buckets() {
        let root = fixture("py-half");
        let mutants = write_python(
            &root,
            "pkg/a.py",
            CLASSIFY,
            &mutated_classify(),
            &meta(&[
                ("pkg.a.x_classify__mutmut_1", "1"),
                ("pkg.a.x_classify__mutmut_2", "0"),
                ("pkg.a.x_classify__mutmut_3", "255"),
                ("pkg.a.x_classify__mutmut_4", "37"),
            ]),
        );
        spans_for(&root, "pkg/a.py", classify_spans());
        let mut measurements = vec![measured("pkg/a.py", "classify", 1, 2)];

        let notes = score_python(&mut measurements, &[(0, 1)], Some(&mutants), &root).unwrap();

        assert!(notes.is_empty());
        assert_eq!(
            measurements[0].mutation,
            Some(Mutation::Score {
                score: 2.0 / 3.0,
                line: 2
            })
        );
    }

    #[test]
    fn no_tests_is_undetected_and_a_nested_span_owns_its_line() {
        let root = fixture("py-nested");
        let source = "\
def outer(n):
    if n > 0:
        return 1
    def inner(n):
        return n
    return inner(n)
";
        let mutated = "\
def x_outer__mutmut_orig(n):
    if n > 0:
        return 1
    def inner(n):
        return n
    return inner(n)

def x_outer__mutmut_1(n):
    if n < 0:
        return 1
    def inner(n):
        return n
    return inner(n)

def x_outer__mutmut_2(n):
    if n > 0:
        return 1
    def inner(n):
        return n + 0
    return inner(n)
";
        let mutants = write_python(
            &root,
            "pkg/a.py",
            source,
            mutated,
            &meta(&[
                ("pkg.a.x_outer__mutmut_1", "33"),
                ("pkg.a.x_outer__mutmut_2", "5"),
            ]),
        );
        let mut measurements = vec![
            measured("pkg/a.py", "outer", 1, 6),
            measured("pkg/a.py", "inner", 4, 5),
        ];

        score_python(&mut measurements, &[(0, 2)], Some(&mutants), &root).unwrap();

        assert_eq!(
            measurements[0].mutation,
            Some(Mutation::Score {
                score: 0.0,
                line: 2
            })
        );
        assert_eq!(
            measurements[1].mutation,
            Some(Mutation::Score {
                score: 0.0,
                line: 5
            })
        );
    }

    #[test]
    fn an_unfinished_mutant_or_a_missing_line_fails_the_file() {
        let root = fixture("py-unfinished");
        let mutants = write_python(
            &root,
            "pkg/a.py",
            CLASSIFY,
            &mutated_classify(),
            &meta(&[
                ("pkg.a.x_classify__mutmut_1", "1"),
                ("pkg.a.x_classify__mutmut_2", "null"),
                ("pkg.a.x_classify__mutmut_3", "2"),
                ("pkg.a.x_classify__mutmut_4", "34"),
            ]),
        );
        spans_for(&root, "pkg/a.py", classify_spans());
        let mut measurements = vec![
            measured("pkg/a.py", "one", 1, 1),
            measured("pkg/a.py", "two", 2, 2),
        ];

        let notes = score_python(&mut measurements, &[(0, 2)], Some(&mutants), &root).unwrap();

        assert_eq!(measurements[0].mutation, Some(Mutation::Unshown));
        assert_eq!(measurements[1].mutation, Some(Mutation::Unshown));
        assert_eq!(
            notes[0].message,
            "A mutant in this file has no finished outcome."
        );

        let mutants = write_python(
            &root,
            "pkg/a.py",
            CLASSIFY,
            &mutated_classify(),
            &meta(&[
                ("pkg.a.x_classify__mutmut_1", "1"),
                ("pkg.a.x_missing__mutmut_1", "0"),
            ]),
        );
        let mut measurements = vec![measured("pkg/a.py", "classify", 1, 2)];
        let notes = score_python(&mut measurements, &[(0, 1)], Some(&mutants), &root).unwrap();
        assert_eq!(measurements[0].mutation, Some(Mutation::Unshown));
        assert_eq!(
            notes[0].message,
            "A mutant in this file has no finished outcome."
        );
    }

    #[test]
    fn a_method_line_is_recovered_without_spans_and_an_empty_meta_stays_off_the_rule() {
        let root = fixture("py-method");
        let source = "\
class Box:
    def classify(self, n):
        return n > 0
";
        let mutated = "\
class Box:
    def x\u{01c1}Box\u{01c1}classify__mutmut_orig(self, n):
        return n > 0

    def x\u{01c1}Box\u{01c1}classify__mutmut_1(self, n):
        return n < 0
";
        let mutants = write_python(
            &root,
            "pkg/a.py",
            source,
            mutated,
            &meta(&[("pkg.a.x\u{01c1}Box\u{01c1}classify__mutmut_1", "0")]),
        );
        fs::write(mutants.join("mutmut-cicd-stats.json"), "{").unwrap();
        let mut measurements = vec![measured("pkg/a.py", "Box.classify", 2, 3)];

        let notes = score_python(&mut measurements, &[(0, 1)], Some(&mutants), &root).unwrap();

        assert!(notes.is_empty());
        assert_eq!(
            measurements[0].mutation,
            Some(Mutation::Score {
                score: 0.0,
                line: 3
            })
        );

        let mutants = write_python(
            &root,
            "pkg/a.py",
            CLASSIFY,
            &mutated_classify(),
            "{\"exit_code_by_key\": {}}",
        );
        let mut measurements = vec![measured("pkg/a.py", "classify", 1, 2)];
        let notes = score_python(&mut measurements, &[(0, 1)], Some(&mutants), &root).unwrap();
        assert!(notes.is_empty());
        assert_eq!(measurements[0].mutation, None);
    }

    #[test]
    fn a_bad_meta_is_an_error_and_a_missing_store_names_the_path() {
        let root = fixture("py-bad");
        let mutants = write_python(&root, "pkg/a.py", CLASSIFY, CLASSIFY, "{");
        let mut measurements = vec![measured("pkg/a.py", "classify", 1, 2)];
        let err = score_python(&mut measurements, &[(0, 1)], Some(&mutants), &root).unwrap_err();
        assert!(matches!(err, MutationError::BadReport(_)));

        let file = root.join("outcomes.json");
        fs::write(&file, "{}").unwrap();
        let err = score_python(&mut measurements, &[(0, 1)], Some(&file), &root).unwrap_err();
        assert!(err.to_string().contains("cannot be parsed"));

        let missing = root.join("absent");
        let notes = score_python(&mut measurements, &[(0, 1)], Some(&missing), &root).unwrap();
        assert_eq!(measurements[0].mutation, Some(Mutation::Unshown));
        assert!(notes[0].message.contains("absent"));

        let mutants = write_python(
            &root,
            "pkg/a.py",
            CLASSIFY,
            &mutated_classify(),
            &meta(&[("pkg.a.x_classify__mutmut_1", "1")]),
        );
        let mut measurements = vec![measured("pkg/a.py", "classify", 1, 2)];
        let notes = score_python(&mut measurements, &[], Some(&mutants), &root).unwrap();
        assert!(notes.is_empty());
        assert_eq!(measurements[0].mutation, None);
    }

    #[test]
    fn a_longer_store_path_matches_on_a_boundary_and_two_suffixes_tie() {
        let root = fixture("py-suffix");
        let mutants = write_python(
            &root,
            "repo/pkg/a.py",
            CLASSIFY,
            &mutated_classify(),
            &meta(&[("repo.pkg.a.x_classify__mutmut_2", "0")]),
        );
        fs::create_dir_all(root.join("pkg")).unwrap();
        fs::write(root.join("pkg/a.py"), CLASSIFY).unwrap();
        let mut measurements = vec![measured("pkg/a.py", "classify", 1, 2)];
        score_python(&mut measurements, &[(0, 1)], Some(&mutants), &root).unwrap();
        assert_eq!(
            measurements[0].mutation,
            Some(Mutation::Score {
                score: 0.0,
                line: 2
            })
        );

        write_python(
            &root,
            "other/a.py",
            CLASSIFY,
            &mutated_classify(),
            &meta(&[("other.a.x_classify__mutmut_2", "1")]),
        );
        let mut measurements = vec![measured("a.py", "classify", 1, 2)];
        let notes = score_python(&mut measurements, &[(0, 1)], Some(&mutants), &root).unwrap();
        assert_eq!(measurements[0].mutation, Some(Mutation::Unshown));
        assert_eq!(notes[0].message, "Mutation records tie for this file.");
    }

    fn element(file: &str, mutants: &[(&str, u32)]) -> String {
        let rows: Vec<String> = mutants
            .iter()
            .enumerate()
            .map(|(index, (status, line))| {
                format!(
                    r#"{{"id":"{index}","mutatorName":"cxx_eq_to_ne","replacement":"!=","location":{{"start":{{"line":{line},"column":1}},"end":{{"line":{line},"column":3}}}},"status":"{status}"}}"#
                )
            })
            .collect();
        format!(
            r#"{{"schemaVersion":"1.7","thresholds":{{"high":80,"low":60}},"files":{{"{file}":{{"language":"cpp","source":"","mutants":[{}]}}}}}}"#,
            rows.join(",")
        )
    }

    fn write_elements(root: &Path, body: &str) -> PathBuf {
        let path = root.join("mull.json");
        fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn a_killed_mutant_and_a_survived_mutant_score_one_half_at_the_survived_line() {
        let root = fixture("mull-half");
        let path = write_elements(
            &root,
            &element(
                "src/classify.cpp",
                &[("Killed", 4), ("Survived", 8), ("Survived", 6)],
            ),
        );
        let mut measurements = vec![measured("src/classify.cpp", "classify", 1, 12)];

        let notes = score_cpp(&mut measurements, &[(0, 1)], Some(&path)).unwrap();

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
    fn timeout_is_detected_no_coverage_is_undetected_and_errors_are_left_out() {
        let root = fixture("mull-status");
        let path = write_elements(
            &root,
            &element(
                "src/classify.cpp",
                &[
                    ("Timeout", 2),
                    ("NoCoverage", 3),
                    ("CompileError", 4),
                    ("RuntimeError", 5),
                    ("Killed", 6),
                ],
            ),
        );
        let mut measurements = vec![measured("src/classify.cpp", "classify", 1, 8)];

        let notes = score_cpp(&mut measurements, &[(0, 1)], Some(&path)).unwrap();

        assert!(notes.is_empty());
        assert_eq!(
            measurements[0].mutation,
            Some(Mutation::Score {
                score: 2.0 / 3.0,
                line: 3
            })
        );
    }

    #[test]
    fn a_pending_or_ignored_mutant_fails_every_function_in_the_file() {
        let root = fixture("mull-unfinished");
        let path = write_elements(
            &root,
            &element(
                "src/classify.cpp",
                &[
                    ("Killed", 2),
                    ("Pending", 3),
                    ("Ignored", 4),
                    ("Survived", 5),
                ],
            ),
        );
        let mut measurements = vec![
            measured("src/classify.cpp", "one", 1, 3),
            measured("src/classify.cpp", "two", 4, 6),
        ];

        let notes = score_cpp(&mut measurements, &[(0, 2)], Some(&path)).unwrap();

        assert_eq!(measurements[0].mutation, Some(Mutation::Unshown));
        assert_eq!(measurements[1].mutation, Some(Mutation::Unshown));
        assert_eq!(
            notes[0].message,
            "A mutant in this file has no finished outcome."
        );
    }

    #[test]
    fn a_mutant_with_no_start_line_fails_the_file() {
        let root = fixture("mull-noline");
        let body = r#"{"files":{"src/classify.cpp":{"language":"cpp","source":"","mutants":[{"id":"1","mutatorName":"cxx_eq_to_ne","location":{"end":{"line":2,"column":1}},"status":"Killed"},{"id":"2","mutatorName":"cxx_eq_to_ne","location":{"start":{"line":0,"column":1},"end":{"line":2,"column":1}},"status":"Survived"}]}}}"#;
        let path = write_elements(&root, body);
        let mut measurements = vec![measured("src/classify.cpp", "classify", 1, 4)];

        let notes = score_cpp(&mut measurements, &[(0, 1)], Some(&path)).unwrap();

        assert_eq!(measurements[0].mutation, Some(Mutation::Unshown));
        assert_eq!(
            notes[0].message,
            "A mutant in this file has no finished outcome."
        );
    }

    #[test]
    fn the_report_joins_on_the_file_and_the_start_line() {
        let root = fixture("mull-join");
        let path = write_elements(
            &root,
            &element("/repo/src/classify.cpp", &[("Survived", 3), ("Killed", 8)]),
        );
        let mut measurements = vec![
            measured("src/classify.cpp", "outer", 1, 10),
            measured("src/classify.cpp", "inner", 6, 9),
        ];
        score_cpp(&mut measurements, &[(0, 2)], Some(&path)).unwrap();
        assert_eq!(
            measurements[0].mutation,
            Some(Mutation::Score {
                score: 0.0,
                line: 3
            })
        );
        assert_eq!(measurements[1].mutation, None);

        let path = write_elements(
            &root,
            &format!(
                r#"{{"files":{{"src/classify.cpp":{{"mutants":[{}]}},"other/classify.cpp":{{"mutants":[{}]}}}}}}"#,
                element_row("Killed", 2, 0),
                element_row("Survived", 2, 1),
            ),
        );
        let mut measurements = vec![measured("classify.cpp", "classify", 1, 4)];
        let notes = score_cpp(&mut measurements, &[(0, 1)], Some(&path)).unwrap();
        assert_eq!(measurements[0].mutation, Some(Mutation::Unshown));
        assert_eq!(notes[0].message, "Mutation records tie for this file.");
    }

    fn element_row(status: &str, line: u32, id: u32) -> String {
        format!(
            r#"{{"id":"{id}","mutatorName":"cxx_eq_to_ne","location":{{"start":{{"line":{line},"column":1}},"end":{{"line":{line},"column":2}}}},"status":"{status}"}}"#
        )
    }

    #[test]
    fn a_missing_elements_report_names_the_path_and_a_bad_report_is_an_error() {
        let root = fixture("mull-missing");
        let mut measurements = vec![measured("src/classify.cpp", "classify", 1, 3)];
        let notes = score_cpp(&mut measurements, &[(0, 1)], None).unwrap();
        assert_eq!(measurements[0].mutation, Some(Mutation::Unshown));
        assert_eq!(notes[0].message, "No mutation report was given.");

        let missing = root.join("mull.json");
        let mut measurements = vec![measured("src/classify.cpp", "classify", 1, 3)];
        let notes = score_cpp(&mut measurements, &[(0, 1)], Some(&missing)).unwrap();
        assert_eq!(measurements[0].mutation, Some(Mutation::Unshown));
        assert!(notes[0].message.contains(&missing.display().to_string()));

        let path = write_elements(&root, "{");
        let mut measurements = vec![measured("src/classify.cpp", "classify", 1, 3)];
        let err = score_cpp(&mut measurements, &[(0, 1)], Some(&path)).unwrap_err();
        assert!(matches!(err, MutationError::BadReport(_)));

        let database = root.join("compile_commands.json");
        fs::write(
            &database,
            r#"[{"directory":"/tmp","command":"c++","file":"src/classify.cpp"}]"#,
        )
        .unwrap();
        let err = score_cpp(&mut measurements, &[(0, 1)], Some(&database)).unwrap_err();
        assert!(matches!(err, MutationError::BadReport(_)));

        let path = write_elements(&root, &element("src/a.rs", &[("Survived", 2)]));
        let mut measurements = vec![measured("src/a.rs", "classify", 1, 3)];
        let notes = score_cpp(&mut measurements, &[], Some(&path)).unwrap();
        assert!(notes.is_empty());
        assert_eq!(measurements[0].mutation, None);
    }
}
