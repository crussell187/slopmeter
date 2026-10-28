use std::fs;
use std::path::{Path, PathBuf};

use crate::adapters;
use crate::analyzers::{measure, Measurement};
use crate::inputs::{
    cover, coverage_path, load_config, select, Config, ConfigError, LcovError, SelectError,
    SelectOptions,
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
}

#[derive(Debug)]
pub enum CheckError {
    Config(ConfigError),
    Select(SelectError),
    Read(PathBuf, std::io::Error),
    Adapter(String),
    Coverage(LcovError),
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
    let mut notes = unparsed_notes(&files);
    notes.extend(macro_notes(&files));
    notes.extend(
        cover(&mut measurements, &scored, report.as_deref()).map_err(CheckError::Coverage)?,
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
