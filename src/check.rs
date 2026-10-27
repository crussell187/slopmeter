use std::fs;
use std::path::{Path, PathBuf};

use crate::adapters;
use crate::analyzers::{measure, Measurement};
use crate::inputs::{
    cover, coverage_path, load_config, select, Config, ConfigError, LcovError, SelectError,
    SelectOptions,
};
use crate::ir::{File, Language};
use crate::reporters::Report;
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
    let mut config = load_config(root).map_err(CheckError::Config)?;
    if let Some(coverage) = &options.coverage {
        config.set_coverage_override(coverage.clone());
    }
    let selected = select(
        root,
        SelectOptions {
            tests: options.tests,
        },
    )
    .map_err(CheckError::Select)?;

    let mut files = Vec::new();
    let mut measurements = Vec::new();
    let mut spans = Vec::new();
    let mut lines = 0usize;
    for item in selected
        .into_iter()
        .filter(|item| config.retains(&item.path))
    {
        let full = root.join(&item.path);
        let source = fs::read(&full).map_err(|err| CheckError::Read(full, err))?;
        lines += source.iter().filter(|byte| **byte == b'\n').count();
        let file =
            adapters::parse(item.language, &item.path, &source).map_err(CheckError::Adapter)?;
        let start = measurements.len();
        measurements.extend(measure_file(&file, &config));
        spans.push((file.language, start, measurements.len()));
        files.push(file);
    }

    let scored: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
    let report = coverage_path(root, config.coverage.as_deref());
    let mut notes =
        cover(&mut measurements, &scored, report.as_deref()).map_err(CheckError::Coverage)?;
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
