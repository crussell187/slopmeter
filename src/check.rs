use std::fs;
use std::path::{Path, PathBuf};

use crate::adapters;
use crate::analyzers::{measure, Measurement};
use crate::inputs::{
    cover, coverage_path, load_config, mutation_path, score_cpp, score_python, score_rust, select,
    Config, ConfigError, LcovError, MutationError, SelectError, SelectOptions,
};
use crate::ir::{File, Language};
use crate::reporters::{Note, Report};
use crate::rules::{evaluate, Finding};
use crate::scope::{partition, ScopeError, ScopeOptions};

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

pub fn score(root: &Path, options: &CheckOptions) -> Result<Report, CheckError> {
    let found = crate::scope::repo_root(root).map_err(CheckError::Scope)?;
    let in_repo = found.is_some();
    let repo = found.unwrap_or_else(|| root.to_path_buf());
    let mut config = load_config(&repo).map_err(CheckError::Config)?;
    if let Some(coverage) = &options.coverage {
        config.set_coverage_override(coverage.clone());
    }
    if let Some(mutation) = &options.mutation_rust {
        config.set_mutation_rust_override(mutation.clone());
    }
    if let Some(mutation) = &options.mutation_python {
        config.set_mutation_python_override(mutation.clone());
    }
    if let Some(mutation) = &options.mutation_cpp {
        config.set_mutation_cpp_override(mutation.clone());
    }
    let python_root = if options.mutation_python.is_some() {
        root
    } else {
        repo.as_path()
    };
    let python_report = mutation_path(python_root, config.mutation_python.as_deref());
    let store_prefix = python_report
        .as_ref()
        .and_then(|store| relative_store(root, store));
    let prefix = subtree_prefix(&repo, root);
    let selected = select(
        root,
        SelectOptions {
            tests: options.tests,
            parent_gitignore: in_repo,
        },
    )
    .map_err(CheckError::Select)?;

    let mut files = Vec::new();
    let mut measurements = Vec::new();
    let mut spans = Vec::new();
    let mut lines = 0usize;
    for item in selected
        .into_iter()
        .filter(|item| config.retains(&repo_relative(&prefix, &item.path)))
    {
        if store_prefix
            .as_ref()
            .is_some_and(|store| Path::new(&item.path).starts_with(store))
        {
            continue;
        }
        let full = root.join(&item.path);
        let source = fs::read(&full).map_err(|err| CheckError::Read(full, err))?;
        lines += source.iter().filter(|byte| **byte == b'\n').count();
        let file = adapters::parse(item.language, &item.path, &source, options.tests)
            .map_err(CheckError::Adapter)?;
        let start = measurements.len();
        measurements.extend(measure_file(&file, &config));
        spans.push((file.language, start, measurements.len()));
        files.push(file);
    }

    let scored: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
    let coverage_root = if options.coverage.is_some() {
        root
    } else {
        repo.as_path()
    };
    let report = coverage_path(coverage_root, config.coverage.as_deref());
    let mut notes = departure_notes(&config);
    notes.extend(unparsed_notes(&files));
    notes.extend(macro_notes(&files));
    notes.extend(
        cover(&mut measurements, &scored, report.as_deref()).map_err(CheckError::Coverage)?,
    );
    let rust_root = if options.mutation_rust.is_some() {
        root
    } else {
        repo.as_path()
    };
    let rust_report = mutation_path(rust_root, config.mutation_rust.as_deref());
    let rust_ranges: Vec<(usize, usize)> = spans
        .iter()
        .filter(|(language, _, _)| *language == Language::Rust)
        .map(|(_, start, end)| (*start, *end))
        .collect();
    let python_ranges: Vec<(usize, usize)> = spans
        .iter()
        .filter(|(language, _, _)| *language == Language::Python)
        .map(|(_, start, end)| (*start, *end))
        .collect();
    notes.extend(
        score_rust(&mut measurements, &rust_ranges, rust_report.as_deref())
            .map_err(CheckError::Mutation)?,
    );
    notes.extend(
        score_python(
            &mut measurements,
            &python_ranges,
            python_report.as_deref(),
            root,
        )
        .map_err(CheckError::Mutation)?,
    );
    let cpp_root = if options.mutation_cpp.is_some() {
        root
    } else {
        repo.as_path()
    };
    let cpp_report = mutation_path(cpp_root, config.mutation_cpp.as_deref());
    let cpp_ranges: Vec<(usize, usize)> = spans
        .iter()
        .filter(|(language, _, _)| *language == Language::Cpp)
        .map(|(_, start, end)| (*start, *end))
        .collect();
    notes.extend(
        score_cpp(&mut measurements, &cpp_ranges, cpp_report.as_deref())
            .map_err(CheckError::Mutation)?,
    );
    let findings = evaluate_spans(&measurements, &spans, &config);
    let scoped = partition(
        root,
        &files,
        &findings,
        ScopeOptions {
            changed: options.changed.clone(),
            strict: options.strict,
        },
    )
    .map_err(CheckError::Scope)?;
    notes.extend(scoped.notes);
    Ok(Report {
        scope: scoped.scope,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        kloc: (lines > 0).then_some(lines as f64 / 1000.0),
        findings: scoped.findings,
        inherited: scoped.inherited,
        notes,
    })
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
    if relative.as_os_str().is_empty() {
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

fn subtree_prefix(repo: &Path, start: &Path) -> String {
    let repo = repo.canonicalize().unwrap_or_else(|_| repo.to_path_buf());
    let start = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
    start
        .strip_prefix(&repo)
        .map(|rel| {
            rel.components()
                .filter_map(|component| match component {
                    std::path::Component::Normal(part) => part.to_str(),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("/")
        })
        .unwrap_or_default()
}

fn measure_file(file: &File, config: &Config) -> Vec<Measurement> {
    let extra = &config.profile(file.language).f08_allow;
    let mut names = adapters::allow(file.language).to_vec();
    names.extend(extra.iter().map(String::as_str));
    measure(file, &names)
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
