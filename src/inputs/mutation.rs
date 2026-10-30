use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

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
    clear_scores(measurements, rust);
    let Some(report) = report else {
        return missing_report(measurements, rust);
    };
    if !report.is_file() {
        return absent_report(measurements, rust, report);
    }
    let records = read_outcomes(report)?;
    Ok(apply_if_any(measurements, rust, &records))
}

pub fn score_python(
    measurements: &mut [Measurement],
    python: &[(usize, usize)],
    source: (Option<&Path>, &Path),
) -> Result<Vec<Note>, MutationError> {
    let (report, source_root) = source;
    clear_scores(measurements, python);
    let Some(report) = report else {
        return missing_report(measurements, python);
    };
    if !report.exists() {
        return absent_report(measurements, python, report);
    }
    let paths = PythonPaths {
        report,
        source_root,
    };
    finish_python(measurements, python, &paths)
}

pub fn score_cpp(
    measurements: &mut [Measurement],
    cpp: &[(usize, usize)],
    report: Option<&Path>,
) -> Result<Vec<Note>, MutationError> {
    clear_scores(measurements, cpp);
    let Some(report) = report else {
        return missing_report(measurements, cpp);
    };
    if !report.is_file() {
        return absent_report(measurements, cpp, report);
    }
    let records = read_elements(report)?;
    Ok(apply_if_any(measurements, cpp, &records))
}

fn clear_scores(measurements: &mut [Measurement], ranges: &[(usize, usize)]) {
    for (start, end) in ranges {
        for measurement in &mut measurements[*start..*end] {
            measurement.mutation = None;
        }
    }
}

fn missing_report(
    measurements: &mut [Measurement],
    ranges: &[(usize, usize)],
) -> Result<Vec<Note>, MutationError> {
    if ranges.is_empty() {
        return Ok(Vec::new());
    }
    mark_unshown(measurements, ranges);
    Ok(lone_note(NO_REPORT.to_string()))
}

fn absent_report(
    measurements: &mut [Measurement],
    ranges: &[(usize, usize)],
    report: &Path,
) -> Result<Vec<Note>, MutationError> {
    if !ranges.is_empty() {
        mark_unshown(measurements, ranges);
    }
    let shown = report.display();
    Ok(lone_note(format!("No mutation report at {shown}.")))
}

fn lone_note(message: String) -> Vec<Note> {
    vec![Note {
        path: None,
        message,
    }]
}

fn apply_if_any(
    measurements: &mut [Measurement],
    ranges: &[(usize, usize)],
    records: &[Record],
) -> Vec<Note> {
    if ranges.is_empty() {
        return Vec::new();
    }
    apply(measurements, ranges, records)
}

struct PythonPaths<'a> {
    report: &'a Path,
    source_root: &'a Path,
}

fn finish_python(
    measurements: &mut [Measurement],
    python: &[(usize, usize)],
    paths: &PythonPaths<'_>,
) -> Result<Vec<Note>, MutationError> {
    let report = paths.report;
    let mut records = read_python(report)?;
    if python.is_empty() {
        return Ok(Vec::new());
    }
    let source_root = paths.source_root;
    Resolver {
        records: &mut records,
        measurements,
        ranges: python,
        mutants_dir: report,
        source_root,
    }
    .resolve()?;
    Ok(apply(measurements, python, &records))
}

fn read_python(report: &Path) -> Result<Vec<Record>, MutationError> {
    if !report.is_dir() {
        return Err(bad_report(report));
    }
    read_store(report)
}

fn bad_report(path: &Path) -> MutationError {
    MutationError::BadReport(path.to_path_buf())
}

fn read_json(path: &Path) -> Result<Value, MutationError> {
    let text = fs::read_to_string(path).map_err(|err| read_error(path, err))?;
    serde_json::from_str(&text).map_err(|_| bad_report(path))
}

fn read_error(path: &Path, err: std::io::Error) -> MutationError {
    MutationError::Read(path.to_path_buf(), err)
}

fn array_field<'a>(value: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
    let field = value.get(key);
    field.and_then(Value::as_array)
}

fn object_field<'a>(value: &'a Value, key: &str) -> Option<&'a Map<String, Value>> {
    let field = value.get(key);
    field.and_then(Value::as_object)
}

fn str_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    let field = value.get(key);
    field.and_then(Value::as_str)
}

fn u64_field(value: &Value, key: &str) -> Option<u64> {
    let field = value.get(key);
    field.and_then(Value::as_u64)
}

fn read_outcomes(path: &Path) -> Result<Vec<Record>, MutationError> {
    let value = read_json(path)?;
    let outcomes = array_field(&value, "outcomes").ok_or_else(|| bad_report(path))?;
    let mut records = Vec::new();
    for outcome in outcomes {
        push_outcome(&mut records, path, outcome)?;
    }
    Ok(records)
}

fn push_outcome(
    records: &mut Vec<Record>,
    path: &Path,
    outcome: &Value,
) -> Result<(), MutationError> {
    match record(outcome) {
        Ok(Some(found)) => records.push(found),
        Ok(None) => {}
        Err(()) => return Err(bad_report(path)),
    }
    Ok(())
}

fn record(value: &Value) -> Result<Option<Record>, ()> {
    let scenario = value.get("scenario").ok_or(())?;
    if scenario.as_str() == Some("Baseline") {
        return baseline(value);
    }
    mutant_record(value, scenario)
}

fn baseline(value: &Value) -> Result<Option<Record>, ()> {
    let summary = str_field(value, "summary");
    summary.ok_or(())?;
    Ok(None)
}

fn mutant_record(value: &Value, scenario: &Value) -> Result<Option<Record>, ()> {
    let mutant = scenario.get("Mutant").ok_or(())?;
    let summary = str_field(value, "summary").ok_or(())?;
    let file = mutant_path(mutant)?;
    let line = json_line(mutant, "/span/start/line");
    Ok(Some(Record {
        file,
        line,
        kind: kind(summary, line),
        locator: None,
    }))
}

fn mutant_path(mutant: &Value) -> Result<String, ()> {
    let file = str_field(mutant, "file");
    let file = file.map(normalize);
    file.filter(nonempty).ok_or(())
}

fn nonempty(file: &String) -> bool {
    let empty = file.is_empty();
    !empty
}

fn json_line(value: &Value, pointer: &str) -> Option<u32> {
    let found = value.pointer(pointer)?;
    let line = found.as_u64()?;
    u32::try_from(line).ok().filter(|line| *line >= 1)
}

struct ElementOut<'a> {
    records: &'a mut Vec<Record>,
    path: &'a Path,
}

fn read_elements(path: &Path) -> Result<Vec<Record>, MutationError> {
    let value = read_json(path)?;
    let files = object_field(&value, "files").ok_or_else(|| bad_report(path))?;
    let mut records = Vec::new();
    let mut out = ElementOut {
        records: &mut records,
        path,
    };
    for (file, body) in files {
        append_file(&mut out, file, body)?;
    }
    Ok(records)
}

fn append_file(out: &mut ElementOut<'_>, file: &str, body: &Value) -> Result<(), MutationError> {
    let mutants = file_mutants(out.path, body)?;
    for mutant in mutants {
        let record = element_record(out.path, file, mutant)?;
        push_record(out, record);
    }
    Ok(())
}

fn file_mutants<'a>(path: &Path, body: &'a Value) -> Result<&'a Vec<Value>, MutationError> {
    array_field(body, "mutants").ok_or_else(|| bad_report(path))
}

fn push_record(out: &mut ElementOut<'_>, record: Record) {
    let records = &mut out.records;
    records.push(record);
}

fn element_record(path: &Path, file: &str, mutant: &Value) -> Result<Record, MutationError> {
    let status = str_field(mutant, "status").ok_or_else(|| bad_report(path))?;
    let file = normalize(file);
    if file.is_empty() {
        return Err(bad_report(path));
    }
    let line = json_line(mutant, "/location/start/line");
    Ok(Record {
        file,
        line,
        kind: element_kind(status, line),
        locator: None,
    })
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

impl ScoredFile {
    fn push_index(&mut self, index: usize) {
        let indexes = &mut self.indexes;
        indexes.push(index);
    }
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
    let spans = spans_of(measurements, file);
    let indexes = &file.indexes;
    for (slot, index) in indexes.iter().enumerate() {
        let tally = count_owned(rows, &spans, slot);
        let scored = Slot {
            index: *index,
            span: spans[slot],
            tally: &tally,
        };
        write_score(measurements, &scored);
    }
}

fn spans_of(measurements: &[Measurement], file: &ScoredFile) -> Vec<Span> {
    let indexes = &file.indexes;
    indexes
        .iter()
        .map(|index| measurements[*index].span)
        .collect()
}

struct Slot<'a> {
    index: usize,
    span: Span,
    tally: &'a Tally,
}

fn write_score(measurements: &mut [Measurement], slot: &Slot<'_>) {
    let tally = slot.tally;
    let span = slot.span;
    let Some(score) = tally.score(span) else {
        return;
    };
    let index = slot.index;
    let measurement = &mut measurements[index];
    measurement.mutation = Some(score);
}

struct Tally {
    detected: u32,
    undetected: u32,
    first_miss: Option<u32>,
}

impl Tally {
    fn new() -> Self {
        Self {
            detected: 0,
            undetected: 0,
            first_miss: None,
        }
    }

    fn add(&mut self, kind: Kind, line: u32) {
        match kind {
            Kind::Detected => self.detected += 1,
            Kind::Undetected => self.note_miss(line),
            Kind::LeftOut | Kind::Unfinished | Kind::Present => {}
        }
    }

    fn note_miss(&mut self, line: u32) {
        self.undetected += 1;
        let have = self.first_miss;
        self.first_miss = Some(earlier(have, line));
    }

    fn score(&self, span: Span) -> Option<Mutation> {
        let detected = self.detected;
        let undetected = self.undetected;
        let total = detected + undetected;
        if total == 0 || undetected == 0 {
            return None;
        }
        let ratio = f64::from(detected) / f64::from(total);
        Some(Mutation::Score {
            score: ratio,
            line: self.miss_line(span),
        })
    }

    fn miss_line(&self, span: Span) -> u32 {
        let miss = self.first_miss;
        miss.unwrap_or(span.start)
    }
}

fn earlier(have: Option<u32>, line: u32) -> u32 {
    match have {
        Some(have) => have.min(line),
        None => line,
    }
}

fn count_owned(rows: &[&Record], spans: &[Span], slot: usize) -> Tally {
    let mut tally = Tally::new();
    for row in rows {
        let Some(line) = owned_line(row, spans, slot) else {
            continue;
        };
        tally.add(row.kind, line);
    }
    tally
}

fn owned_line(row: &Record, spans: &[Span], slot: usize) -> Option<u32> {
    let line = row.line?;
    if owns(spans, slot, line) {
        return Some(line);
    }
    None
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

fn scored_files(measurements: &[Measurement], ranges: &[(usize, usize)]) -> Vec<ScoredFile> {
    let mut files = FileGroups::new();
    for (start, end) in ranges {
        let range = (*start, *end);
        add_range(measurements, range, &mut files);
    }
    files.finish()
}

fn add_range(measurements: &[Measurement], range: (usize, usize), files: &mut FileGroups) {
    let (start, end) = range;
    for (index, measurement) in measurements.iter().enumerate().take(end).skip(start) {
        files.add(index, measurement);
    }
}

struct FileGroups {
    order: Vec<String>,
    groups: BTreeMap<String, ScoredFile>,
}

impl FileGroups {
    fn new() -> Self {
        Self {
            order: Vec::new(),
            groups: BTreeMap::new(),
        }
    }

    fn add(&mut self, index: usize, measurement: &Measurement) {
        let path = &measurement.path;
        let key = normalize(path);
        if key.is_empty() {
            return;
        }
        if self.existing(&key, index) {
            return;
        }
        self.insert(key, path, index);
    }

    fn existing(&mut self, key: &str, index: usize) -> bool {
        let groups = &mut self.groups;
        let Some(file) = groups.get_mut(key) else {
            return false;
        };
        file.push_index(index);
        true
    }

    fn insert(&mut self, key: String, path: &str, index: usize) {
        let order_key = key.clone();
        let order = &mut self.order;
        order.push(order_key);
        let map_key = key.clone();
        let groups = &mut self.groups;
        groups.insert(map_key, new_scored(key, path, index));
    }

    fn finish(self) -> Vec<ScoredFile> {
        let order = self.order;
        let mut groups = self.groups;
        order
            .into_iter()
            .filter_map(|key| groups.remove(&key))
            .collect()
    }
}

fn new_scored(key: String, path: &str, index: usize) -> ScoredFile {
    ScoredFile {
        key,
        path: path.to_owned(),
        indexes: vec![index],
    }
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
    let entries = fs::read_dir(dir).map_err(|err| read_error(dir, err))?;
    for entry in entries {
        visit_entry(dir, metas, entry)?;
    }
    Ok(())
}

fn visit_entry(
    dir: &Path,
    metas: &mut Vec<PathBuf>,
    entry: Result<fs::DirEntry, std::io::Error>,
) -> Result<(), MutationError> {
    let entry = entry.map_err(|err| read_error(dir, err))?;
    let path = entry.path();
    if ignored_store_path(&path) {
        return Ok(());
    }
    if entry_is_dir(&entry, &path)? {
        return collect_meta(&path, metas);
    }
    if is_meta(&path) {
        metas.push(path);
    }
    Ok(())
}

fn entry_is_dir(entry: &fs::DirEntry, path: &Path) -> Result<bool, MutationError> {
    let kind = entry.file_type();
    let kind = kind.map_err(|err| read_error(path, err))?;
    Ok(kind.is_dir())
}

fn is_meta(path: &Path) -> bool {
    let extension = path.extension();
    extension.is_some_and(|ext| ext == "meta")
}

fn ignored_store_path(path: &Path) -> bool {
    export_stats(path) || cicd_json(path)
}

fn export_stats(path: &Path) -> bool {
    let mut components = path.components();
    components.any(|component| component.as_os_str() == "export-cicd-stats")
}

fn cicd_json(path: &Path) -> bool {
    let name = path.file_name();
    name.is_some_and(|name| name == "mutmut-cicd-stats.json")
}

fn read_meta(root: &Path, meta: &Path) -> Result<Vec<Record>, MutationError> {
    let value = read_json(meta)?;
    let codes = object_field(&value, "exit_code_by_key").ok_or_else(|| bad_report(meta))?;
    let file = stored_file(root, meta)?;
    if codes.is_empty() {
        return Ok(vec![present_record(file)]);
    }
    code_records(meta, &file, codes)
}

fn present_record(file: String) -> Record {
    Record {
        file,
        line: None,
        kind: Kind::Present,
        locator: None,
    }
}

fn stored_file(root: &Path, meta: &Path) -> Result<String, MutationError> {
    let relative = meta.strip_prefix(root);
    let relative = relative.ok();
    let text = relative.and_then(Path::to_str);
    let stripped = text.and_then(|path| path.strip_suffix(".meta"));
    let file = stripped.map(normalize);
    file.filter(nonempty).ok_or_else(|| bad_report(meta))
}

fn code_records(
    meta: &Path,
    file: &str,
    codes: &Map<String, Value>,
) -> Result<Vec<Record>, MutationError> {
    let mut records = Vec::new();
    for (key, code) in codes {
        let item = MetaCode {
            meta,
            file,
            key,
            code,
        };
        records.push(code_record(&item)?);
    }
    Ok(records)
}

struct MetaCode<'a> {
    meta: &'a Path,
    file: &'a str,
    key: &'a str,
    code: &'a Value,
}

fn code_record(item: &MetaCode<'_>) -> Result<Record, MutationError> {
    let exit = exit_code(item.code).map_err(|_| bad_report(item.meta))?;
    let kind = python_kind(exit);
    let locator = mutant_locator(item.key);
    let file = item.file;
    Ok(Record {
        file: file.to_owned(),
        line: None,
        kind: finished_kind(kind, &locator),
        locator,
    })
}

fn finished_kind(kind: Kind, locator: &Option<Locator>) -> Kind {
    if locator.is_none() {
        return Kind::Unfinished;
    }
    kind
}

fn exit_code(value: &Value) -> Result<Option<i64>, ()> {
    if value.is_null() {
        return Ok(None);
    }
    let code = value.as_i64();
    code.map(Some).ok_or(())
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
    let mut parts = key.split('.');
    let mutant = parts.find(mutmut_part)?;
    let (prefix, _) = mutant.split_once("__mutmut_")?;
    let plain = plain_name(prefix)?;
    if plain.is_empty() {
        return None;
    }
    Some(Locator {
        mutant: mutant.to_string(),
        orig: format!("{prefix}__mutmut_orig"),
        plain: plain.to_string(),
    })
}

fn mutmut_part(part: &&str) -> bool {
    let part = *part;
    if !part.contains("__mutmut_") {
        return false;
    }
    mutant_prefix(part)
}

fn mutant_prefix(part: &str) -> bool {
    part.starts_with("x_") || part.starts_with("x\u{01c1}")
}

fn plain_name(prefix: &str) -> Option<&str> {
    if prefix.contains('\u{01c1}') {
        let mut parts = prefix.rsplit('\u{01c1}');
        return parts.next();
    }
    prefix.strip_prefix("x_")
}

struct Resolver<'a> {
    records: &'a mut [Record],
    measurements: &'a [Measurement],
    ranges: &'a [(usize, usize)],
    mutants_dir: &'a Path,
    source_root: &'a Path,
}

impl<'a> Resolver<'a> {
    fn resolve(&mut self) -> Result<(), MutationError> {
        let jobs = self.jobs();
        let texts = self.load_texts(&jobs)?;
        self.store_lines(&jobs, &texts);
        Ok(())
    }

    fn jobs(&self) -> Vec<LineJob> {
        let measurements = self.measurements;
        let ranges = self.ranges;
        let files = scored_files(measurements, ranges);
        let mut jobs = Vec::new();
        for file in &files {
            self.collect_file(file, &mut jobs);
        }
        jobs
    }

    fn collect_file(&self, file: &ScoredFile, jobs: &mut Vec<LineJob>) {
        let key = &file.key;
        let pointers = self.pointers(key);
        for (index, record) in self.records.iter().enumerate() {
            if listed(&pointers, record) {
                jobs.push(job_for(index, file));
            }
        }
    }

    fn pointers(&self, key: &str) -> Vec<*const Record> {
        let records = &*self.records;
        match choose(records, key) {
            Choice::Rows(rows) => rows
                .into_iter()
                .filter(has_locator)
                .map(std::ptr::from_ref)
                .collect(),
            _ => Vec::new(),
        }
    }

    fn load_texts(&self, jobs: &[LineJob]) -> Result<Texts, MutationError> {
        let mut texts = Texts::new();
        for job in jobs {
            self.load_job(&mut texts, job)?;
        }
        Ok(texts)
    }

    fn load_job(&self, texts: &mut Texts, job: &LineJob) -> Result<(), MutationError> {
        let root = self.source_root;
        let path = &job.path;
        texts.remember_source(root, path)?;
        let dir = self.mutants_dir;
        let file = self.file_name(job.index);
        texts.remember_copy(dir, file)
    }

    fn file_name(&self, index: usize) -> &str {
        let record = &self.records[index];
        record.file.as_str()
    }

    fn store_lines(&mut self, jobs: &[LineJob], texts: &Texts) {
        for job in jobs {
            self.store_job(job, texts);
        }
    }

    fn store_job(&mut self, job: &LineJob, texts: &Texts) {
        let Some(locator) = self.take_locator(job.index) else {
            return;
        };
        let line = self.recovered(job, texts, &locator);
        self.write_line(job.index, line);
    }

    fn take_locator(&mut self, index: usize) -> Option<Locator> {
        let record = &mut self.records[index];
        let locator = &mut record.locator;
        locator.take()
    }

    fn recovered(&self, job: &LineJob, texts: &Texts, locator: &Locator) -> Option<u32> {
        let path = &job.path;
        let source = texts.source(path);
        let file = self.file_name(job.index);
        let copy = texts.copy(file);
        join_line(source, copy, locator)
    }

    fn write_line(&mut self, index: usize, line: Option<u32>) {
        let record = &mut self.records[index];
        match line {
            Some(line) => record.line = Some(line),
            None => mark_unfinished(record),
        }
    }
}

fn has_locator(row: &&Record) -> bool {
    let record = *row;
    let locator = &record.locator;
    locator.is_some()
}

fn listed(pointers: &[*const Record], record: &Record) -> bool {
    pointers.contains(&std::ptr::from_ref(record))
}

fn job_for(index: usize, file: &ScoredFile) -> LineJob {
    let path = &file.path;
    LineJob {
        index,
        path: path.clone(),
    }
}

struct LineJob {
    index: usize,
    path: String,
}

fn join_line(source: Option<&str>, copy: Option<&StoreCopy>, locator: &Locator) -> Option<u32> {
    let paired = source.zip(copy);
    paired.and_then(|(source, copy)| recover_line(source, copy, locator))
}

fn mark_unfinished(record: &mut Record) {
    record.kind = Kind::Unfinished;
    record.line = None;
}

struct Texts {
    sources: BTreeMap<String, Option<String>>,
    copies: BTreeMap<String, StoreCopy>,
}

impl Texts {
    fn new() -> Self {
        Self {
            sources: BTreeMap::new(),
            copies: BTreeMap::new(),
        }
    }

    fn remember_source(&mut self, root: &Path, path: &str) -> Result<(), MutationError> {
        if self.has_source(path) {
            return Ok(());
        }
        let full = root.join(path);
        let text = read_optional(&full)?;
        self.insert_source(path, text);
        Ok(())
    }

    fn has_source(&self, path: &str) -> bool {
        let sources = &self.sources;
        sources.contains_key(path)
    }

    fn insert_source(&mut self, path: &str, text: Option<String>) {
        let sources = &mut self.sources;
        sources.insert(path.to_owned(), text);
    }

    fn remember_copy(&mut self, dir: &Path, file: &str) -> Result<(), MutationError> {
        if self.has_copy(file) {
            return Ok(());
        }
        let copy = load_copy(dir, file)?;
        self.insert_copy(file, copy);
        Ok(())
    }

    fn has_copy(&self, file: &str) -> bool {
        let copies = &self.copies;
        copies.contains_key(file)
    }

    fn insert_copy(&mut self, file: &str, copy: StoreCopy) {
        let copies = &mut self.copies;
        copies.insert(file.to_owned(), copy);
    }

    fn source(&self, path: &str) -> Option<&str> {
        let sources = &self.sources;
        let text = sources.get(path);
        text.and_then(Option::as_deref)
    }

    fn copy(&self, file: &str) -> Option<&StoreCopy> {
        let copies = &self.copies;
        copies.get(file)
    }
}

fn read_optional(path: &Path) -> Result<Option<String>, MutationError> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(read_error(path, err)),
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
    let value: Value = serde_json::from_str(text).map_err(|_| bad_report(path))?;
    if u64_field(&value, "version") != Some(1) {
        return Ok(BTreeMap::new());
    }
    let spans = object_field(&value, "spans").ok_or_else(|| bad_report(path))?;
    parse_spans(path, spans)
}

fn parse_spans(
    path: &Path,
    spans: &Map<String, Value>,
) -> Result<BTreeMap<String, (u32, u32)>, MutationError> {
    let mut parsed = BTreeMap::new();
    for (name, span) in spans {
        parsed.insert(name.clone(), span_pair(path, span)?);
    }
    Ok(parsed)
}

fn span_pair(path: &Path, span: &Value) -> Result<(u32, u32), MutationError> {
    let Some(pair) = span.as_array() else {
        return Err(bad_report(path));
    };
    let Some(ends) = pair.get(0..2) else {
        return Err(bad_report(path));
    };
    line_pair(path, ends)
}

fn line_pair(path: &Path, ends: &[Value]) -> Result<(u32, u32), MutationError> {
    let start = ends.first();
    let start = start.and_then(Value::as_u64);
    let end = ends.get(1);
    let end = end.and_then(Value::as_u64);
    let (Some(start), Some(end)) = (start, end) else {
        return Err(bad_report(path));
    };
    checked_span(path, start, end)
}

fn checked_span(path: &Path, start: u64, end: u64) -> Result<(u32, u32), MutationError> {
    let (Ok(start), Ok(end)) = (u32::try_from(start), u32::try_from(end)) else {
        return Err(bad_report(path));
    };
    Ok((start, end))
}

fn recover_line(source: &str, copy: &StoreCopy, locator: &Locator) -> Option<u32> {
    let mutated = copy.mutated.as_deref()?;
    let orig = function_lines(mutated, copy, &locator.orig)?;
    let mutant = function_lines(mutated, copy, &locator.mutant)?;
    let offset = first_diff(&orig, &mutant, locator)?;
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
    let from = decorator_from(&lines, start);
    let end = body_end(&lines, start);
    Some(copy_lines(&lines[from..end]))
}

fn copy_lines(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| (*line).to_string()).collect()
}

fn decorator_from(lines: &[&str], start: usize) -> usize {
    let mut from = start;
    while from > 0 && is_decorator(lines[from - 1]) {
        from -= 1;
    }
    from
}

fn is_decorator(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with('@')
}

fn body_end(lines: &[&str], start: usize) -> usize {
    let indent = indent_of(lines[start]);
    let mut end = start + 1;
    while end < lines.len() && inside_body(lines[end], indent) {
        end += 1;
    }
    drop_blank_tail(lines, start, end)
}

fn inside_body(line: &str, indent: usize) -> bool {
    if is_blank(line) {
        return true;
    }
    indent_of(line) > indent
}

fn drop_blank_tail(lines: &[&str], start: usize, mut end: usize) -> usize {
    while end > start + 1 && is_blank(lines[end - 1]) {
        end -= 1;
    }
    end
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
    let trimmed = line.trim_start();
    line.len() - trimmed.len()
}

struct DiffText<'a> {
    orig: &'a [String],
    mutant: &'a [String],
    locator: &'a Locator,
}

fn first_diff(orig: &[String], mutant: &[String], locator: &Locator) -> Option<u32> {
    let mut orig = orig.to_vec();
    let mut mutant = mutant.to_vec();
    trim_trailing_blank(&mut orig);
    trim_trailing_blank(&mut mutant);
    let text = DiffText {
        orig: &orig,
        mutant: &mutant,
        locator,
    };
    diff_at(&text)
}

fn diff_at(text: &DiffText<'_>) -> Option<u32> {
    let width = wider(text.orig, text.mutant);
    for index in 0..width {
        if differs(text, index) {
            return u32::try_from(index).ok();
        }
    }
    None
}

fn wider(orig: &[String], mutant: &[String]) -> usize {
    let orig_len = orig.len();
    let mutant_len = mutant.len();
    orig_len.max(mutant_len)
}

struct Swap<'a> {
    name: &'a str,
    plain: &'a str,
}

fn differs(text: &DiffText<'_>, index: usize) -> bool {
    let locator = text.locator;
    let plain = &locator.plain;
    let orig = text.orig;
    let mutant = text.mutant;
    let left = replaced(
        orig,
        index,
        Swap {
            name: &locator.orig,
            plain,
        },
    );
    let right = replaced(
        mutant,
        index,
        Swap {
            name: &locator.mutant,
            plain,
        },
    );
    left != right
}

fn replaced(lines: &[String], index: usize, swap: Swap<'_>) -> Option<String> {
    let name = swap.name;
    let plain = swap.plain;
    let line = lines.get(index);
    line.map(|line| line.replacen(name, plain, 1))
}

fn trim_trailing_blank(lines: &mut Vec<String>) {
    while lines.last().is_some_and(|line| is_blank(line)) {
        lines.pop();
    }
}

fn is_blank(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty()
}

fn find_block(source: &str, block: &[String]) -> Option<u32> {
    if block.is_empty() {
        return None;
    }
    let lines: Vec<&str> = source.lines().collect();
    if longer_than(block, &lines) {
        return None;
    }
    let window = Window {
        lines: &lines,
        block,
    };
    sole_match(&window)
}

fn longer_than(block: &[String], lines: &[&str]) -> bool {
    let block_len = block.len();
    let line_len = lines.len();
    block_len > line_len
}

struct Window<'a> {
    lines: &'a [&'a str],
    block: &'a [String],
}

fn sole_match(window: &Window<'_>) -> Option<u32> {
    let mut found = None;
    let last = window_last(window);
    for start in 0..=last {
        found = next_match(found, window, start)?;
    }
    found
}

fn window_last(window: &Window<'_>) -> usize {
    let lines = window.lines;
    let block = window.block;
    lines.len() - block.len()
}

fn next_match(found: Option<u32>, window: &Window<'_>, start: usize) -> Option<Option<u32>> {
    if !same_window(window, start) {
        return Some(found);
    }
    if found.is_some() {
        return None;
    }
    Some(line_no(start))
}

fn same_window(window: &Window<'_>, start: usize) -> bool {
    let block = window.block;
    let end = start + block.len();
    let lines = window.lines;
    let have = &lines[start..end];
    have.iter().zip(block).all(|(left, right)| *left == right)
}

fn line_no(start: usize) -> Option<u32> {
    let number = u32::try_from(start).ok();
    number.map(|start| start + 1)
}

#[cfg(test)]
mod tests {
    use super::{mutation_path, score_cpp, score_rust, MutationError};
    use crate::analyzers::{Measurement, Mutation};
    use crate::ir::Span;
    use crate::reporters::Note;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    fn score_python(
        measurements: &mut [Measurement],
        python: &[(usize, usize)],
        report: Option<&Path>,
        source_root: &Path,
    ) -> Result<Vec<Note>, MutationError> {
        super::score_python(measurements, python, (report, source_root))
    }

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
            swallowing: vec![],
            io: vec![],
            mutations: vec![],
            nulls: vec![],
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
