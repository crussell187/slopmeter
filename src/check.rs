use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::adapters::{self, ParseInput};
use crate::analyzers::{measure, Coverage, Measurement, Mutation};
use crate::fresh::{newest_stale, older_than, Stamped};
use crate::inputs::{
    cover, coverage_path, load_config, mutation_path, score_cpp, score_python, score_rust, select,
    Config, ConfigError, LcovError, MutationError, SelectError, SelectOptions, Selected,
};
use crate::ir::{File, Language};
use crate::reporters::{Note, Report};
use crate::rules::{evaluate, Finding};
use crate::scope::{partition, ScopeError, ScopeOptions, Scoped};

pub struct CheckOptions {
    pub strict: bool,
    pub tests: bool,
    pub changed: Option<String>,
    pub coverage: Option<String>,
    pub mutation_rust: Option<String>,
    pub mutation_python: Option<String>,
    pub mutation_cpp: Option<String>,
}

#[derive(Debug)]
pub enum CheckError {
    Config(ConfigError),
    Select(SelectError),
    Read(PathBuf, std::io::Error),
    Adapter(String),
    Coverage(LcovError),
    Mutation(MutationError),
    Scope(ScopeError),
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckError::Config(err) => write!(f, "{err}"),
            CheckError::Select(err) => write!(f, "{err}"),
            CheckError::Read(path, err) => write!(f, "reading {}: {err}", path.display()),
            CheckError::Adapter(message) => write!(f, "{message}"),
            CheckError::Coverage(err) => write!(f, "{err}"),
            CheckError::Mutation(err) => write!(f, "{err}"),
            CheckError::Scope(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for CheckError {}

struct Prepared {
    repo: PathBuf,
    in_repo: bool,
    config: Config,
    python_report: Option<PathBuf>,
    store_prefix: Option<PathBuf>,
    prefix: String,
}

struct Scan<'a> {
    root: &'a Path,
    options: &'a CheckOptions,
    prepared: &'a Prepared,
}

struct Loaded {
    files: Vec<File>,
    measurements: Vec<Measurement>,
    spans: Vec<(Language, usize, usize)>,
    lines: usize,
}

pub fn score(root: &Path, options: &CheckOptions) -> Result<Report, CheckError> {
    let prepared = prepare(root, options)?;
    let scan = Scan {
        root,
        options,
        prepared: &prepared,
    };
    let loaded = load_files(&scan)?;
    finish(&scan, loaded)
}

fn prepare(root: &Path, options: &CheckOptions) -> Result<Prepared, CheckError> {
    let found = crate::scope::repo_root(root).map_err(CheckError::Scope)?;
    let in_repo = found.is_some();
    let repo = found.unwrap_or_else(|| root.to_path_buf());
    let config = configured(&repo, options)?;
    let python_report = python_report(options, &config, (root, repo.as_path()));
    let store_prefix = store_prefix(root, python_report.as_deref());
    let prefix = subtree_prefix(&repo, root);
    Ok(Prepared {
        repo,
        in_repo,
        config,
        python_report,
        store_prefix,
        prefix,
    })
}

fn configured(repo: &Path, options: &CheckOptions) -> Result<Config, CheckError> {
    let mut config = load_config(repo).map_err(CheckError::Config)?;
    apply_overrides(&mut config, options);
    Ok(config)
}

fn apply_overrides(config: &mut Config, options: &CheckOptions) {
    if let Some(coverage) = coverage_override(options) {
        config.set_coverage_override(coverage);
    }
    apply_mutation_overrides(config, options);
}

fn apply_mutation_overrides(config: &mut Config, options: &CheckOptions) {
    if let Some(mutation) = rust_override(options) {
        config.set_mutation_rust_override(mutation);
    }
    if let Some(mutation) = python_override(options) {
        config.set_mutation_python_override(mutation);
    }
    if let Some(mutation) = cpp_override(options) {
        config.set_mutation_cpp_override(mutation);
    }
}

fn coverage_override(options: &CheckOptions) -> Option<String> {
    let coverage = &options.coverage;
    coverage.clone()
}

fn rust_override(options: &CheckOptions) -> Option<String> {
    let mutation = &options.mutation_rust;
    mutation.clone()
}

fn python_override(options: &CheckOptions) -> Option<String> {
    let mutation = &options.mutation_python;
    mutation.clone()
}

fn cpp_override(options: &CheckOptions) -> Option<String> {
    let mutation = &options.mutation_cpp;
    mutation.clone()
}

fn python_report(
    options: &CheckOptions,
    config: &Config,
    roots: (&Path, &Path),
) -> Option<PathBuf> {
    let mutation = &options.mutation_python;
    let base = mutation_root(roots.0, roots.1, mutation.is_some());
    let configured = &config.mutation_python;
    mutation_path(base, configured.as_deref())
}

fn mutation_root<'a>(scan: &'a Path, repo: &'a Path, overridden: bool) -> &'a Path {
    if overridden {
        scan
    } else {
        repo
    }
}

fn store_prefix(scan: &Path, store: Option<&Path>) -> Option<PathBuf> {
    let Some(store) = store else {
        return None;
    };
    relative_store(scan, store)
}

fn load_files(scan: &Scan<'_>) -> Result<Loaded, CheckError> {
    let prepared = scan.prepared;
    let options = scan.options;
    let selected =
        select(scan.root, select_options(options, prepared.in_repo)).map_err(CheckError::Select)?;
    let mut loaded = Loaded {
        files: Vec::new(),
        measurements: Vec::new(),
        spans: Vec::new(),
        lines: 0,
    };
    for item in selected {
        if kept(prepared, &item.path) {
            read_item(scan, &mut loaded, &item)?;
        }
    }
    Ok(loaded)
}

fn select_options(options: &CheckOptions, in_repo: bool) -> SelectOptions {
    let tests = options.tests;
    SelectOptions {
        tests,
        parent_gitignore: in_repo,
    }
}

fn kept(prepared: &Prepared, path: &str) -> bool {
    let config = &prepared.config;
    let prefix = &prepared.prefix;
    if !config.retains(&repo_relative(prefix, path)) {
        return false;
    }
    let store = &prepared.store_prefix;
    outside_store(store.as_deref(), path)
}

fn outside_store(store: Option<&Path>, path: &str) -> bool {
    match store {
        Some(store) => !starts_with_store(path, store),
        None => true,
    }
}

fn starts_with_store(path: &str, store: &Path) -> bool {
    let file = Path::new(path);
    file.starts_with(store)
}

fn read_item(scan: &Scan<'_>, loaded: &mut Loaded, item: &Selected) -> Result<(), CheckError> {
    let root = scan.root;
    let full = root.join(&item.path);
    let source = fs::read(&full).map_err(|err| CheckError::Read(full, err))?;
    let count = newline_count(&source);
    loaded.lines += count;
    let options = scan.options;
    let prepared = scan.prepared;
    let config = &prepared.config;
    let file = adapters::parse(
        item.language,
        ParseInput {
            path: &item.path,
            source: &source,
            include_tests: options.tests,
        },
    )
    .map_err(CheckError::Adapter)?;
    push_file(loaded, file, config);
    Ok(())
}

fn newline_count(source: &[u8]) -> usize {
    source.iter().filter(|byte| **byte == b'\n').count()
}

fn push_file(loaded: &mut Loaded, file: File, config: &Config) {
    let measurements = &mut loaded.measurements;
    let start = measurements.len();
    measurements.extend(measure_file(&file, config));
    let end = measurements.len();
    let language = file.language;
    let spans = &mut loaded.spans;
    spans.push((language, start, end));
    let files = &mut loaded.files;
    files.push(file);
}

fn finish(scan: &Scan<'_>, mut loaded: Loaded) -> Result<Report, CheckError> {
    let prepared = scan.prepared;
    let config = &prepared.config;
    let mut notes = file_notes(&loaded.files, config);
    let changed = diff_paths(scan)?;
    let coverage = coverage_notes(scan, &mut loaded, changed.as_ref())?;
    notes.extend(coverage);
    let mutation = mutation_notes(&mut loaded, scan, changed.as_ref())?;
    notes.extend(mutation);
    let findings = evaluate_spans(&loaded.measurements, &loaded.spans, config);
    let scoped = scope_findings(scan, &loaded.files, &findings)?;
    publish(loaded.lines, notes, scoped)
}

fn publish(lines: usize, mut notes: Vec<Note>, scoped: Scoped) -> Result<Report, CheckError> {
    let Scoped {
        scope,
        findings,
        inherited,
        notes: extra,
    } = scoped;
    notes.extend(extra);
    Ok(Report {
        scope,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        kloc: kloc(lines),
        findings,
        inherited,
        notes,
    })
}

fn kloc(lines: usize) -> Option<f64> {
    if lines == 0 {
        return None;
    }
    Some(lines as f64 / 1000.0)
}

fn scope_findings(
    scan: &Scan<'_>,
    files: &[File],
    findings: &[Finding],
) -> Result<Scoped, CheckError> {
    let options = scan.options;
    let changed = &options.changed;
    partition(
        scan.root,
        files,
        findings,
        ScopeOptions {
            changed: changed.clone(),
            strict: options.strict,
        },
    )
    .map_err(CheckError::Scope)
}

fn file_notes(files: &[File], config: &Config) -> Vec<Note> {
    let mut notes = departure_notes(config);
    notes.extend(unparsed_notes(files));
    notes.extend(macro_notes(files));
    notes
}

fn coverage_notes(
    scan: &Scan<'_>,
    loaded: &mut Loaded,
    changed: Option<&BTreeSet<String>>,
) -> Result<Vec<Note>, CheckError> {
    let report = coverage_report(scan);
    let root = scan.root;
    if let Some(note) = language_stale(StaleQuery {
        root,
        report: report.as_deref(),
        changed,
        meta: false,
    })? {
        hide_coverage(&mut loaded.measurements);
        return Ok(vec![note]);
    }
    let measurements = &mut loaded.measurements;
    let scored = scored_paths(&loaded.files);
    let mut notes =
        cover(measurements, &scored, report.as_deref()).map_err(CheckError::Coverage)?;
    if per_file(scan) {
        notes = drop_stale_coverage(CoverageDrop {
            root,
            measurements,
            report: report.as_deref(),
            notes,
        })?;
    }
    Ok(notes)
}

fn coverage_report(scan: &Scan<'_>) -> Option<PathBuf> {
    let prepared = scan.prepared;
    let options = scan.options;
    let coverage = &options.coverage;
    let root = mutation_root(scan.root, &prepared.repo, coverage.is_some());
    let config = &prepared.config;
    let configured = &config.coverage;
    coverage_path(root, configured.as_deref())
}

fn scored_paths(files: &[File]) -> Vec<&str> {
    let mut paths = Vec::new();
    for file in files {
        let path = &file.path;
        paths.push(path.as_str());
    }
    paths
}

fn mutation_notes(
    loaded: &mut Loaded,
    scan: &Scan<'_>,
    changed: Option<&BTreeSet<String>>,
) -> Result<Vec<Note>, CheckError> {
    let mut notes = Vec::new();
    notes.extend(rust_mutation(loaded, scan, changed)?);
    notes.extend(python_mutation(loaded, scan, changed)?);
    notes.extend(cpp_mutation(loaded, scan, changed)?);
    Ok(notes)
}

fn rust_mutation(
    loaded: &mut Loaded,
    scan: &Scan<'_>,
    changed: Option<&BTreeSet<String>>,
) -> Result<Vec<Note>, CheckError> {
    let report = rust_report(scan);
    let ranges = language_ranges(&loaded.spans, Language::Rust);
    if let Some(note) = language_stale(StaleQuery {
        root: scan.root,
        report: report.as_deref(),
        changed,
        meta: false,
    })? {
        hide_mutation(&mut loaded.measurements, &ranges);
        return Ok(vec![note]);
    }
    let measurements = &mut loaded.measurements;
    let notes =
        score_rust(measurements, &ranges, report.as_deref()).map_err(CheckError::Mutation)?;
    finish_mutation(MutationFinish {
        scan,
        measurements,
        ranges: &ranges,
        report: report.as_deref(),
        notes,
        meta: false,
    })
}

fn rust_report(scan: &Scan<'_>) -> Option<PathBuf> {
    let options = scan.options;
    let flag = &options.mutation_rust;
    let prepared = scan.prepared;
    let config = &prepared.config;
    let configured = &config.mutation_rust;
    let repo = &prepared.repo;
    let root = mutation_root(scan.root, repo, flag.is_some());
    mutation_path(root, configured.as_deref())
}

fn language_ranges(spans: &[(Language, usize, usize)], wanted: Language) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    for (language, start, end) in spans {
        if *language == wanted {
            ranges.push((*start, *end));
        }
    }
    ranges
}

fn python_mutation(
    loaded: &mut Loaded,
    scan: &Scan<'_>,
    changed: Option<&BTreeSet<String>>,
) -> Result<Vec<Note>, CheckError> {
    let prepared = scan.prepared;
    let stored = &prepared.python_report;
    let report = stored.as_deref();
    let ranges = language_ranges(&loaded.spans, Language::Python);
    if let Some(note) = language_stale(StaleQuery {
        root: scan.root,
        report,
        changed,
        meta: true,
    })? {
        hide_mutation(&mut loaded.measurements, &ranges);
        return Ok(vec![note]);
    }
    let measurements = &mut loaded.measurements;
    let notes =
        score_python(measurements, &ranges, (report, scan.root)).map_err(CheckError::Mutation)?;
    finish_mutation(MutationFinish {
        scan,
        measurements,
        ranges: &ranges,
        report,
        notes,
        meta: true,
    })
}

fn cpp_mutation(
    loaded: &mut Loaded,
    scan: &Scan<'_>,
    changed: Option<&BTreeSet<String>>,
) -> Result<Vec<Note>, CheckError> {
    let report = cpp_report(scan);
    let ranges = language_ranges(&loaded.spans, Language::Cpp);
    if let Some(note) = language_stale(StaleQuery {
        root: scan.root,
        report: report.as_deref(),
        changed,
        meta: false,
    })? {
        hide_mutation(&mut loaded.measurements, &ranges);
        return Ok(vec![note]);
    }
    let measurements = &mut loaded.measurements;
    let notes =
        score_cpp(measurements, &ranges, report.as_deref()).map_err(CheckError::Mutation)?;
    finish_mutation(MutationFinish {
        scan,
        measurements,
        ranges: &ranges,
        report: report.as_deref(),
        notes,
        meta: false,
    })
}

fn cpp_report(scan: &Scan<'_>) -> Option<PathBuf> {
    let options = scan.options;
    let flag = &options.mutation_cpp;
    let prepared = scan.prepared;
    let config = &prepared.config;
    let configured = &config.mutation_cpp;
    let repo = &prepared.repo;
    let root = mutation_root(scan.root, repo, flag.is_some());
    mutation_path(root, configured.as_deref())
}

fn departure_notes(config: &Config) -> Vec<Note> {
    crate::inputs::departures(config)
        .into_iter()
        .map(|message| Note {
            path: None,
            message,
        })
        .collect()
}

fn macro_notes(files: &[File]) -> Vec<Note> {
    files
        .iter()
        .filter(|file| file.unscored_macros > 0)
        .map(|file| Note {
            path: Some(file.path.clone()),
            message: macro_note(file.unscored_macros),
        })
        .collect()
}

fn relative_store(scan: &Path, store: &Path) -> Option<PathBuf> {
    let scan = scan.canonicalize().ok()?;
    let store = store.canonicalize().ok()?;
    let relative = store.strip_prefix(&scan).ok()?;
    if empty_path(&relative) {
        return None;
    }
    Some(relative.to_path_buf())
}

fn macro_note(count: u32) -> String {
    if count == 1 {
        "The parser left 1 function-like macro body unscored.".to_string()
    } else {
        format!("The parser left {count} function-like macro bodies unscored.")
    }
}

fn unparsed_notes(files: &[File]) -> Vec<Note> {
    files
        .iter()
        .filter(|file| file.unparsed_ratio > 0.0)
        .map(|file| Note {
            path: Some(file.path.clone()),
            message: format!(
                "The parser left {:.3} of the lines unscored.",
                file.unparsed_ratio
            ),
        })
        .collect()
}

fn repo_relative(prefix: &str, path: &str) -> String {
    if prefix.is_empty() {
        path.to_string()
    } else {
        format!("{prefix}/{path}")
    }
}

fn empty_path(path: &Path) -> bool {
    let text = path.as_os_str();
    text.is_empty()
}

fn subtree_prefix(repo: &Path, start: &Path) -> String {
    let repo = canonical(repo);
    let start = canonical(start);
    match start.strip_prefix(&repo) {
        Ok(rel) => join_components(rel),
        Err(_) => String::new(),
    }
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn join_components(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        if let Some(part) = normal_name(component) {
            parts.push(part);
        }
    }
    parts.join("/")
}

fn normal_name(component: std::path::Component<'_>) -> Option<&str> {
    match component {
        std::path::Component::Normal(part) => part.to_str(),
        _ => None,
    }
}

fn measure_file(file: &File, config: &Config) -> Vec<Measurement> {
    let names = allow_names(file, config);
    measure(file, &names)
}

fn allow_names<'a>(file: &File, config: &'a Config) -> Vec<&'a str> {
    let profile = config.profile(file.language);
    let extra = &profile.f08_allow;
    let mut names = Vec::new();
    names.extend(adapters::allow(file.language));
    names.extend(extra.iter().map(String::as_str));
    names
}

fn evaluate_spans(
    measurements: &[Measurement],
    spans: &[(Language, usize, usize)],
    config: &Config,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (language, start, end) in spans {
        findings.extend(evaluate(
            &measurements[*start..*end],
            config.profile(*language),
        ));
    }
    findings
}

const COVERAGE_MISSING: &str = "No coverage record matches this file.";
const MUTATION_MISSING: &str = "No mutation record matches this file.";

fn diff_paths(scan: &Scan<'_>) -> Result<Option<BTreeSet<String>>, CheckError> {
    if !diff_mode(scan) {
        return Ok(None);
    }
    let options = scan.options;
    let changed = options.changed.as_deref();
    crate::scope::changed_paths(scan.root, changed)
        .map(Some)
        .map_err(CheckError::Scope)
}

fn diff_mode(scan: &Scan<'_>) -> bool {
    let options = scan.options;
    let prepared = scan.prepared;
    !options.strict && prepared.in_repo
}

fn per_file(scan: &Scan<'_>) -> bool {
    let options = scan.options;
    let prepared = scan.prepared;
    options.strict || !prepared.in_repo
}

struct StaleQuery<'a> {
    root: &'a Path,
    report: Option<&'a Path>,
    changed: Option<&'a BTreeSet<String>>,
    meta: bool,
}

fn language_stale(query: StaleQuery<'_>) -> Result<Option<Note>, CheckError> {
    let Some(changed) = query.changed else {
        return Ok(None);
    };
    let Some(report) = query.report else {
        return Ok(None);
    };
    diff_stale(DiffStale {
        root: query.root,
        report,
        changed,
        meta: query.meta,
    })
}

struct DiffStale<'a> {
    root: &'a Path,
    report: &'a Path,
    changed: &'a BTreeSet<String>,
    meta: bool,
}

fn diff_stale(query: DiffStale<'_>) -> Result<Option<Note>, CheckError> {
    let Some(time) = report_time(query.report, query.meta)? else {
        return Ok(None);
    };
    let stamped = stamp_changed(query.root, query.report, query.changed)?;
    let Some(path) = newest_stale(time, &stamped) else {
        return Ok(None);
    };
    let label = report_label(query.root, query.report);
    Ok(Some(run_note(&older_than(&label, &path))))
}

fn report_time(report: &Path, meta: bool) -> Result<Option<SystemTime>, CheckError> {
    if meta {
        return newest_meta(report);
    }
    if !report.is_file() {
        return Ok(None);
    }
    Ok(Some(required_mtime(report)?))
}

fn stamp_changed(
    root: &Path,
    report: &Path,
    changed: &BTreeSet<String>,
) -> Result<Vec<Stamped>, CheckError> {
    let mut stamped = Vec::new();
    for path in changed {
        if report_owns(root, report, path) {
            continue;
        }
        let full = root.join(path);
        stamped.push(Stamped {
            path: path.clone(),
            time: metadata_time(&full)?,
        });
    }
    Ok(stamped)
}

fn report_owns(root: &Path, report: &Path, path: &str) -> bool {
    let full = root.join(path);
    if nested(&full, report) {
        return true;
    }
    let Ok(full) = full.canonicalize() else {
        return false;
    };
    let Ok(report) = report.canonicalize() else {
        return false;
    };
    nested(&full, &report)
}

fn nested(path: &Path, report: &Path) -> bool {
    path.starts_with(report)
}

fn hide_coverage(measurements: &mut [Measurement]) {
    for measurement in measurements.iter_mut() {
        measurement.crap = None;
        measurement.coverage = Some(Coverage::Unshown);
    }
}

fn hide_mutation(measurements: &mut [Measurement], ranges: &[(usize, usize)]) {
    for (start, end) in ranges {
        hide_range(measurements, *start, *end);
    }
}

fn hide_range(measurements: &mut [Measurement], start: usize, end: usize) {
    let mut index = start;
    while index < end {
        let measurement = &mut measurements[index];
        measurement.mutation = Some(Mutation::Unshown);
        index += 1;
    }
}

struct CoverageDrop<'a> {
    root: &'a Path,
    measurements: &'a mut [Measurement],
    report: Option<&'a Path>,
    notes: Vec<Note>,
}

fn drop_stale_coverage(job: CoverageDrop<'_>) -> Result<Vec<Note>, CheckError> {
    let Some(report) = job.report else {
        return Ok(job.notes);
    };
    if !report.is_file() {
        return Ok(job.notes);
    }
    let paths = measurement_paths(job.measurements);
    withhold(Withhold {
        root: job.root,
        measurements: job.measurements,
        paths: &paths,
        notes: job.notes,
        clock: FileClock::Shared(required_mtime(report)?),
        report,
        missing: COVERAGE_MISSING,
        kind: RowKind::Coverage,
    })
}

struct MutationFinish<'a> {
    scan: &'a Scan<'a>,
    measurements: &'a mut [Measurement],
    ranges: &'a [(usize, usize)],
    report: Option<&'a Path>,
    notes: Vec<Note>,
    meta: bool,
}

fn finish_mutation(job: MutationFinish<'_>) -> Result<Vec<Note>, CheckError> {
    let scan = job.scan;
    if !per_file(scan) {
        return Ok(job.notes);
    }
    let Some(report) = job.report else {
        return Ok(job.notes);
    };
    let Some(clock) = mutation_clock(report, job.meta)? else {
        return Ok(job.notes);
    };
    let paths = range_paths(job.measurements, job.ranges);
    withhold(Withhold {
        root: scan.root,
        measurements: job.measurements,
        paths: &paths,
        notes: job.notes,
        clock,
        report,
        missing: MUTATION_MISSING,
        kind: RowKind::Mutation,
    })
}

enum FileClock {
    Shared(SystemTime),
    Store(PathBuf),
}

#[derive(Clone, Copy)]
enum RowKind {
    Coverage,
    Mutation,
}

fn mutation_clock(report: &Path, meta: bool) -> Result<Option<FileClock>, CheckError> {
    if meta {
        return store_clock(report);
    }
    if !report.is_file() {
        return Ok(None);
    }
    Ok(Some(FileClock::Shared(required_mtime(report)?)))
}

fn store_clock(report: &Path) -> Result<Option<FileClock>, CheckError> {
    if !report.is_dir() {
        return Ok(None);
    }
    Ok(Some(FileClock::Store(report.to_path_buf())))
}

struct Withhold<'a> {
    root: &'a Path,
    measurements: &'a mut [Measurement],
    paths: &'a [String],
    notes: Vec<Note>,
    clock: FileClock,
    report: &'a Path,
    missing: &'a str,
    kind: RowKind,
}

fn withhold(mut job: Withhold<'_>) -> Result<Vec<Note>, CheckError> {
    let listed = job.paths;
    let paths = listed.to_vec();
    let mut stale = Vec::new();
    let mut newest = None;
    for path in &paths {
        newest = track_file(Track {
            root: job.root,
            notes: &job.notes,
            clock: &job.clock,
            missing: job.missing,
            path,
            newest,
            stale: &mut stale,
        })?;
    }
    let kind = job.kind;
    clear_rows(job.measurements, &stale, kind);
    drop_notes(&mut job.notes, &stale);
    let Some((_, path)) = newest else {
        return Ok(job.notes);
    };
    let label = report_label(job.root, job.report);
    let message = older_than(&label, &path);
    let notes = &mut job.notes;
    notes.push(run_note(&message));
    Ok(job.notes)
}

struct Track<'a> {
    root: &'a Path,
    notes: &'a [Note],
    clock: &'a FileClock,
    missing: &'a str,
    path: &'a str,
    newest: Option<(SystemTime, String)>,
    stale: &'a mut Vec<String>,
}

fn track_file(mut track: Track<'_>) -> Result<Option<(SystemTime, String)>, CheckError> {
    let Some(time) = source_newer(&track)? else {
        return Ok(track.newest);
    };
    if !report_matched(track.notes, track.path, track.missing) {
        return Ok(track.newest);
    }
    let path = track.path;
    let stale = &mut track.stale;
    stale.push(path.to_string());
    Ok(Some(prefer_pair(track.newest, time, path)))
}

fn source_newer(track: &Track<'_>) -> Result<Option<SystemTime>, CheckError> {
    let Some(report_time) = clock_time(track.clock, track.path)? else {
        return Ok(None);
    };
    let root = track.root;
    let full = root.join(track.path);
    let Some(time) = metadata_time(&full)? else {
        return Ok(None);
    };
    if time > report_time {
        return Ok(Some(time));
    }
    Ok(None)
}

fn clock_time(clock: &FileClock, source: &str) -> Result<Option<SystemTime>, CheckError> {
    match clock {
        FileClock::Shared(time) => Ok(Some(*time)),
        FileClock::Store(store) => meta_time(store, source),
    }
}

fn meta_time(store: &Path, source: &str) -> Result<Option<SystemTime>, CheckError> {
    let name = format!("{source}.meta");
    let path = store.join(name);
    metadata_time(&path)
}

fn prefer_pair(
    best: Option<(SystemTime, String)>,
    time: SystemTime,
    path: &str,
) -> (SystemTime, String) {
    let Some((have, name)) = best else {
        return (time, path.to_string());
    };
    if later_rank(
        &Rank { time, path },
        &Rank {
            time: have,
            path: &name,
        },
    ) {
        return (time, path.to_string());
    }
    (have, name)
}

fn later_rank(next: &Rank<'_>, have: &Rank<'_>) -> bool {
    let next_time = next.time;
    let have_time = have.time;
    if next_time > have_time {
        return true;
    }
    if next_time != have_time {
        return false;
    }
    let next_path = next.path;
    let have_path = have.path;
    next_path > have_path
}

struct Rank<'a> {
    time: SystemTime,
    path: &'a str,
}

fn report_matched(notes: &[Note], path: &str, missing: &str) -> bool {
    !file_note(notes, path, missing)
}

fn file_note(notes: &[Note], path: &str, message: &str) -> bool {
    notes.iter().any(|note| note_is(note, path, message))
}

fn note_is(note: &Note, path: &str, message: &str) -> bool {
    let noted = &note.path;
    let Some(noted) = noted.as_deref() else {
        return false;
    };
    noted == path && note.message == message
}

fn clear_rows(measurements: &mut [Measurement], stale: &[String], kind: RowKind) {
    for measurement in measurements.iter_mut() {
        clear_one(measurement, stale, kind);
    }
}

fn clear_one(measurement: &mut Measurement, stale: &[String], kind: RowKind) {
    let path = &measurement.path;
    if !listed(stale, path) {
        return;
    }
    match kind {
        RowKind::Coverage => clear_coverage(measurement),
        RowKind::Mutation => {
            measurement.mutation = None;
        }
    }
}

fn clear_coverage(measurement: &mut Measurement) {
    measurement.crap = None;
    measurement.coverage = None;
}

fn drop_notes(notes: &mut Vec<Note>, stale: &[String]) {
    notes.retain(|note| keep_note(note, stale));
}

fn keep_note(note: &Note, stale: &[String]) -> bool {
    let path = &note.path;
    let Some(path) = path.as_deref() else {
        return true;
    };
    !listed(stale, path)
}

fn listed(stale: &[String], path: &str) -> bool {
    stale.iter().any(|item| item == path)
}

fn measurement_paths(measurements: &[Measurement]) -> Vec<String> {
    let mut paths = Vec::new();
    for measurement in measurements {
        let path = &measurement.path;
        push_unique(&mut paths, path);
    }
    paths
}

fn range_paths(measurements: &[Measurement], ranges: &[(usize, usize)]) -> Vec<String> {
    let mut paths = Vec::new();
    for (start, end) in ranges {
        collect_paths(SpanPaths {
            measurements,
            start: *start,
            end: *end,
            paths: &mut paths,
        });
    }
    paths
}

struct SpanPaths<'a> {
    measurements: &'a [Measurement],
    start: usize,
    end: usize,
    paths: &'a mut Vec<String>,
}

fn collect_paths(span: SpanPaths<'_>) {
    let mut index = span.start;
    while index < span.end {
        let measurement = &span.measurements[index];
        let path = &measurement.path;
        push_unique(span.paths, path);
        index += 1;
    }
}

fn push_unique(paths: &mut Vec<String>, path: &str) {
    if paths.iter().any(|have| have == path) {
        return;
    }
    paths.push(path.to_string());
}

fn newest_meta(dir: &Path) -> Result<Option<SystemTime>, CheckError> {
    if !dir.is_dir() {
        return Ok(None);
    }
    let mut newest = None;
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = next_dir(&mut pending) {
        let mut walk = MetaWalk {
            newest: &mut newest,
            pending: &mut pending,
        };
        read_meta_dir(&dir, &mut walk)?;
    }
    Ok(newest)
}

fn next_dir(pending: &mut Vec<PathBuf>) -> Option<PathBuf> {
    pending.pop()
}

struct MetaWalk<'a> {
    newest: &'a mut Option<SystemTime>,
    pending: &'a mut Vec<PathBuf>,
}

fn read_meta_dir(dir: &Path, walk: &mut MetaWalk<'_>) -> Result<(), CheckError> {
    let entries = fs::read_dir(dir).map_err(|err| CheckError::Read(dir.to_path_buf(), err))?;
    for entry in entries {
        visit_meta(dir, walk, entry)?;
    }
    Ok(())
}

fn visit_meta(
    dir: &Path,
    walk: &mut MetaWalk<'_>,
    entry: Result<fs::DirEntry, std::io::Error>,
) -> Result<(), CheckError> {
    let entry = entry.map_err(|err| CheckError::Read(dir.to_path_buf(), err))?;
    let path = entry.path();
    if path.is_dir() {
        let pending = &mut walk.pending;
        pending.push(path);
        return Ok(());
    }
    if !meta_name(&path) {
        return Ok(());
    }
    let time = required_mtime(&path)?;
    let current = *walk.newest;
    *walk.newest = later_time(current, time);
    Ok(())
}

fn meta_name(path: &Path) -> bool {
    let extension = path.extension();
    extension.is_some_and(|ext| ext == "meta")
}

fn later_time(have: Option<SystemTime>, time: SystemTime) -> Option<SystemTime> {
    match have {
        Some(have) if have >= time => Some(have),
        _ => Some(time),
    }
}

fn required_mtime(path: &Path) -> Result<SystemTime, CheckError> {
    let Some(time) = metadata_time(path)? else {
        return Err(missing_file(path));
    };
    Ok(time)
}

fn missing_file(path: &Path) -> CheckError {
    CheckError::Read(
        path.to_path_buf(),
        std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
    )
}

fn metadata_time(path: &Path) -> Result<Option<SystemTime>, CheckError> {
    match fs::metadata(path) {
        Ok(meta) => Ok(Some(modified_at(&meta, path)?)),
        Err(err) if not_found(&err) => Ok(None),
        Err(err) => Err(CheckError::Read(path.to_path_buf(), err)),
    }
}

fn not_found(err: &std::io::Error) -> bool {
    let kind = err.kind();
    kind == std::io::ErrorKind::NotFound
}

fn modified_at(meta: &fs::Metadata, path: &Path) -> Result<SystemTime, CheckError> {
    let time = meta.modified();
    time.map_err(|err| CheckError::Read(path.to_path_buf(), err))
}

fn report_label(root: &Path, report: &Path) -> String {
    match report.strip_prefix(root) {
        Ok(path) => join_components(path),
        Err(_) => join_components(report),
    }
}

fn run_note(message: &str) -> Note {
    Note {
        path: None,
        message: message.to_string(),
    }
}
