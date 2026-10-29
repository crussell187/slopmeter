use std::path::{Path, PathBuf};

use crate::check::{score, CheckOptions};
use crate::explain::page;
use crate::reporters::{no_check, render, Rendered};

pub fn run(cwd: &Path, args: &[String]) -> Rendered {
    match args.first().map(String::as_str) {
        Some("explain") => explain(&args[1..]),
        _ => check(cwd, args),
    }
}

fn explain(args: &[String]) -> Rendered {
    let [rule] = args else {
        return no_check(if args.is_empty() {
            "explain needs a rule"
        } else {
            "explain takes one rule"
        });
    };
    match page(rule) {
        Some(text) => Rendered {
            stdout: text.to_string(),
            stderr: String::new(),
            code: 0,
        },
        None => no_check(&format!("unknown rule {rule}")),
    }
}

fn check(cwd: &Path, args: &[String]) -> Rendered {
    let parsed = match parse_check(cwd, args) {
        Ok(parsed) => parsed,
        Err(reason) => return no_check(&reason),
    };
    match score(&parsed.root, &parsed.options) {
        Ok(report) => render(&report, parsed.json),
        Err(err) => no_check(&err.to_string()),
    }
}

struct Parsed {
    root: PathBuf,
    options: CheckOptions,
    json: bool,
}

fn parse_check(cwd: &Path, args: &[String]) -> Result<Parsed, String> {
    let mut options = CheckOptions {
        strict: false,
        tests: false,
        changed: None,
        coverage: None,
        mutation_rust: None,
    };
    let mut json = false;
    let mut path = None;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if let Some(value) = arg.strip_prefix("--mutation-rust=") {
            options.mutation_rust = Some(required(value, "mutation-rust needs a path")?);
            continue;
        }
        if let Some(value) = arg.strip_prefix("--coverage=") {
            options.coverage = Some(required(value, "coverage needs a path")?);
            continue;
        }
        if let Some(value) = arg.strip_prefix("--changed=") {
            options.changed = Some(required(value, "changed needs a ref")?);
            continue;
        }
        match arg.as_str() {
            "--strict" => options.strict = true,
            "--json" => json = true,
            "--tests" => options.tests = true,
            "--coverage" => {
                options.coverage = Some(next_value(&mut rest, "coverage needs a path")?)
            }
            "--mutation-rust" => {
                options.mutation_rust = Some(next_value(&mut rest, "mutation-rust needs a path")?)
            }
            "--changed" => options.changed = Some(next_value(&mut rest, "changed needs a ref")?),
            other if other.starts_with('-') => return Err(format!("unknown argument {other}")),
            other => {
                if path.is_some() {
                    return Err("the check takes one path".to_string());
                }
                path = Some(PathBuf::from(other));
            }
        }
    }
    let root = match path {
        Some(path) if path.is_absolute() => path,
        Some(path) => cwd.join(path),
        None => cwd.to_path_buf(),
    };
    Ok(Parsed {
        root,
        options,
        json,
    })
}

fn required(value: &str, reason: &str) -> Result<String, String> {
    if value.is_empty() {
        Err(reason.to_string())
    } else {
        Ok(value.to_string())
    }
}

fn next_value(args: &mut std::slice::Iter<'_, String>, reason: &str) -> Result<String, String> {
    match args.next() {
        Some(value) if !value.is_empty() && !value.starts_with('-') => Ok(value.clone()),
        _ => Err(reason.to_string()),
    }
}
