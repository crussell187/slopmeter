use std::path::{Path, PathBuf};

use crate::check::{score, CheckOptions};
use crate::explain::page;
use crate::reporters::{no_check, render, Rendered};

pub fn run(cwd: &Path, args: &[String]) -> Rendered {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return help();
    }
    if args.iter().any(|arg| arg == "--version") {
        return version();
    }
    match args.first().map(String::as_str) {
        Some("explain") => explain(&args[1..]),
        _ => check(cwd, args),
    }
}

fn help() -> Rendered {
    Rendered {
        stdout: format!(
            "\
slopmeter {version}

Score Python, Rust, and C++ against the function-scope rules.

Usage: slopmeter [options] [path]
       slopmeter explain <rule>

Options:
  --strict                 score the whole tree
  --changed <ref>          diff against this base
  --json                   print the JSON report
  --tests                  score tests too
  --coverage <path>        read this LCOV report
  --mutation-rust <path>   read cargo-mutants outcomes.json
  --mutation-python <path> read a mutmut store
  --mutation-cpp <path>    read a mull report
  -h, --help               print this help
  --version                print the version
",
            version = env!("CARGO_PKG_VERSION"),
        ),
        stderr: String::new(),
        code: 0,
    }
}

fn version() -> Rendered {
    Rendered {
        stdout: format!("slopmeter {}\n", env!("CARGO_PKG_VERSION")),
        stderr: String::new(),
        code: 0,
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

struct CheckParse<'a> {
    options: &'a mut CheckOptions,
    json: &'a mut bool,
    path: &'a mut Option<PathBuf>,
}

fn parse_check(cwd: &Path, args: &[String]) -> Result<Parsed, String> {
    let mut options = default_options();
    let mut json = false;
    let mut path = None;
    walk_args(
        &mut CheckParse {
            options: &mut options,
            json: &mut json,
            path: &mut path,
        },
        args,
    )?;
    reject_combined(&options)?;
    Ok(Parsed {
        root: resolve_root(cwd, path),
        options,
        json,
    })
}

fn walk_args(parsed: &mut CheckParse<'_>, args: &[String]) -> Result<(), String> {
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        take_arg(parsed, arg, &mut rest)?;
    }
    Ok(())
}

fn default_options() -> CheckOptions {
    CheckOptions {
        strict: false,
        tests: false,
        changed: None,
        coverage: None,
        mutation_rust: None,
        mutation_python: None,
        mutation_cpp: None,
    }
}

fn take_arg(
    parsed: &mut CheckParse<'_>,
    arg: &str,
    rest: &mut std::slice::Iter<'_, String>,
) -> Result<(), String> {
    if take_mutation_equals(parsed.options, arg)? {
        return Ok(());
    }
    if take_path_equals(parsed.options, arg)? {
        return Ok(());
    }
    if take_switch(parsed, arg) {
        return Ok(());
    }
    if take_report_value(parsed.options, arg, rest)? {
        return Ok(());
    }
    if take_changed_value(parsed.options, arg, rest)? {
        return Ok(());
    }
    take_positional(parsed.path, arg)
}

fn take_mutation_equals(options: &mut CheckOptions, arg: &str) -> Result<bool, String> {
    if let Some(value) = arg.strip_prefix("--mutation-rust=") {
        options.mutation_rust = Some(required(value, "mutation-rust needs a path")?);
        return Ok(true);
    }
    if let Some(value) = arg.strip_prefix("--mutation-python=") {
        options.mutation_python = Some(required(value, "mutation-python needs a path")?);
        return Ok(true);
    }
    if let Some(value) = arg.strip_prefix("--mutation-cpp=") {
        options.mutation_cpp = Some(required(value, "mutation-cpp needs a path")?);
        return Ok(true);
    }
    Ok(false)
}

fn take_path_equals(options: &mut CheckOptions, arg: &str) -> Result<bool, String> {
    if let Some(value) = equals_value(arg, "--coverage=") {
        options.coverage = Some(required(value, "coverage needs a path")?);
        return Ok(true);
    }
    if let Some(value) = equals_value(arg, "--changed=") {
        options.changed = Some(required(value, "changed needs a ref")?);
        return Ok(true);
    }
    Ok(false)
}

fn equals_value<'a>(arg: &'a str, prefix: &str) -> Option<&'a str> {
    arg.strip_prefix(prefix)
}

fn take_switch(parsed: &mut CheckParse<'_>, arg: &str) -> bool {
    match arg {
        "--strict" => set_strict(parsed.options),
        "--json" => set_json(parsed.json),
        "--tests" => set_tests(parsed.options),
        _ => return false,
    }
    true
}

fn set_strict(options: &mut CheckOptions) {
    options.strict = true;
}

fn set_json(json: &mut bool) {
    *json = true;
}

fn set_tests(options: &mut CheckOptions) {
    options.tests = true;
}

fn take_report_value(
    options: &mut CheckOptions,
    arg: &str,
    rest: &mut std::slice::Iter<'_, String>,
) -> Result<bool, String> {
    match arg {
        "--coverage" => options.coverage = Some(next_value(rest, "coverage needs a path")?),
        "--mutation-rust" => {
            options.mutation_rust = Some(next_value(rest, "mutation-rust needs a path")?)
        }
        "--mutation-python" => {
            options.mutation_python = Some(next_value(rest, "mutation-python needs a path")?)
        }
        _ => return Ok(false),
    }
    Ok(true)
}

fn take_changed_value(
    options: &mut CheckOptions,
    arg: &str,
    rest: &mut std::slice::Iter<'_, String>,
) -> Result<bool, String> {
    match arg {
        "--mutation-cpp" => {
            options.mutation_cpp = Some(next_value(rest, "mutation-cpp needs a path")?)
        }
        "--changed" => options.changed = Some(next_value(rest, "changed needs a ref")?),
        _ => return Ok(false),
    }
    Ok(true)
}

fn take_positional(path: &mut Option<PathBuf>, arg: &str) -> Result<(), String> {
    if arg.starts_with('-') {
        return Err(format!("unknown argument {arg}"));
    }
    if path.is_some() {
        return Err("the check takes one path".to_string());
    }
    *path = Some(PathBuf::from(arg));
    Ok(())
}

fn reject_combined(options: &CheckOptions) -> Result<(), String> {
    let changed = &options.changed;
    if options.strict && changed.is_some() {
        return Err("--changed and --strict cannot be combined".to_string());
    }
    Ok(())
}

fn resolve_root(cwd: &Path, path: Option<PathBuf>) -> PathBuf {
    match path {
        Some(path) if path.is_absolute() => path,
        Some(path) => cwd.join(path),
        None => cwd.to_path_buf(),
    }
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
