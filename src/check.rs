use std::fs;
use std::path::{Path, PathBuf};

use crate::adapters::{self, ParseInput};
use crate::analyzers::{measure, Measurement};
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
    let coverage = coverage_notes(scan, &mut loaded)?;
    notes.extend(coverage);
    let mutation = mutation_notes(&mut loaded, scan)?;
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

fn coverage_notes(scan: &Scan<'_>, loaded: &mut Loaded) -> Result<Vec<Note>, CheckError> {
    let prepared = scan.prepared;
    let options = scan.options;
    let coverage = &options.coverage;
    let root = mutation_root(scan.root, &prepared.repo, coverage.is_some());
    let config = &prepared.config;
    let configured = &config.coverage;
    let report = coverage_path(root, configured.as_deref());
    let scored = scored_paths(&loaded.files);
    let measurements = &mut loaded.measurements;
    cover(measurements, &scored, report.as_deref()).map_err(CheckError::Coverage)
}

fn scored_paths(files: &[File]) -> Vec<&str> {
    let mut paths = Vec::new();
    for file in files {
        let path = &file.path;
        paths.push(path.as_str());
    }
    paths
}

fn mutation_notes(loaded: &mut Loaded, scan: &Scan<'_>) -> Result<Vec<Note>, CheckError> {
    let mut notes = Vec::new();
    notes.extend(rust_mutation(loaded, scan)?);
    notes.extend(python_mutation(loaded, scan)?);
    notes.extend(cpp_mutation(loaded, scan)?);
    Ok(notes)
}

fn rust_mutation(loaded: &mut Loaded, scan: &Scan<'_>) -> Result<Vec<Note>, CheckError> {
    let options = scan.options;
    let flag = &options.mutation_rust;
    let prepared = scan.prepared;
    let config = &prepared.config;
    let configured = &config.mutation_rust;
    let repo = &prepared.repo;
    let root = mutation_root(scan.root, repo, flag.is_some());
    let report = mutation_path(root, configured.as_deref());
    let ranges = language_ranges(&loaded.spans, Language::Rust);
    let measurements = &mut loaded.measurements;
    score_rust(measurements, &ranges, report.as_deref()).map_err(CheckError::Mutation)
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

fn python_mutation(loaded: &mut Loaded, scan: &Scan<'_>) -> Result<Vec<Note>, CheckError> {
    let prepared = scan.prepared;
    let report = &prepared.python_report;
    let ranges = language_ranges(&loaded.spans, Language::Python);
    let measurements = &mut loaded.measurements;
    score_python(measurements, &ranges, (report.as_deref(), scan.root))
        .map_err(CheckError::Mutation)
}

fn cpp_mutation(loaded: &mut Loaded, scan: &Scan<'_>) -> Result<Vec<Note>, CheckError> {
    let options = scan.options;
    let flag = &options.mutation_cpp;
    let prepared = scan.prepared;
    let config = &prepared.config;
    let configured = &config.mutation_cpp;
    let repo = &prepared.repo;
    let root = mutation_root(scan.root, repo, flag.is_some());
    let report = mutation_path(root, configured.as_deref());
    let ranges = language_ranges(&loaded.spans, Language::Cpp);
    let measurements = &mut loaded.measurements;
    score_cpp(measurements, &ranges, report.as_deref()).map_err(CheckError::Mutation)
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
