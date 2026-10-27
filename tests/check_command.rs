use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use slopmeter::inputs::Config;

fn fixture(name: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("check-fixtures")
        .join(format!(
            "{name}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

fn run(root: &Path, args: &[&str]) -> std::process::Output {
    let mut all = Vec::from(args);
    all.push(root.to_str().unwrap());
    invoke(&all)
}

fn invoke(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_slopmeter"))
        .args(args)
        .output()
        .expect("run slopmeter")
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_AUTHOR_NAME", "slopmeter")
        .env("GIT_AUTHOR_EMAIL", "slopmeter@example.com")
        .env("GIT_COMMITTER_NAME", "slopmeter")
        .env("GIT_COMMITTER_EMAIL", "slopmeter@example.com")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn params(count: u32) -> String {
    (0..count)
        .map(|index| format!("p{index}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn function(name: &str, count: u32) -> String {
    if count == 0 {
        format!("def {name}():\n    return 1\n")
    } else {
        format!("def {name}({}):\n    return p0\n", params(count))
    }
}

#[test]
fn strict_fails_when_a_function_is_over_its_gate() {
    let root = fixture("wide");
    let count = Config::builtin().python.f04_gate + 1;
    write(&root, "pkg/wide.py", &function("wide", count));
    let output = run(&root, &["--strict"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("F04"), "{stdout}");
    assert!(stdout.contains("wide"), "{stdout}");
    assert!(stdout.contains("fail"), "{stdout}");
    assert!(!stdout.contains("not a git repository"), "{stdout}");
}

#[test]
fn strict_passes_when_every_function_is_at_or_under_its_gate() {
    let root = fixture("ok");
    let count = Config::builtin().python.f04_gate;
    write(&root, "pkg/ok.py", &function("ok", count));
    let output = run(&root, &["--strict"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    assert!(!stdout.contains("F04"), "{stdout}");
    assert!(stdout.contains("pass"), "{stdout}");
}

#[test]
fn json_reports_the_same_verdict_as_the_table() {
    let root = fixture("json");
    let count = Config::builtin().python.f04_gate + 1;
    write(&root, "pkg/wide.py", &function("wide", count));
    let output = run(&root, &["--strict", "--json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("\"verdict\": \"fail\""), "{stdout}");
    assert!(stdout.contains("\"rule\": \"F04\""), "{stdout}");
    assert!(stdout.contains("\"scope\": \"strict\""), "{stdout}");
}

#[test]
fn a_wrong_argument_exits_2() {
    let unknown = invoke(&["--nope"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(unknown.stdout.is_empty());
    assert_eq!(unknown.stderr, b"unknown argument --nope\n");

    let coverage = invoke(&["--coverage"]);
    assert_eq!(coverage.status.code(), Some(2));
    assert!(coverage.stdout.is_empty());
    assert_eq!(coverage.stderr, b"coverage needs a path\n");

    let changed = invoke(&["--changed="]);
    assert_eq!(changed.status.code(), Some(2));
    assert!(changed.stdout.is_empty());
    assert_eq!(changed.stderr, b"changed needs a ref\n");

    let root = fixture("two");
    let extra = root.join("other");
    let both = invoke(&["--strict", root.to_str().unwrap(), extra.to_str().unwrap()]);
    assert_eq!(both.status.code(), Some(2));
    assert!(both.stdout.is_empty());
    assert_eq!(both.stderr, b"the check takes one path\n");
}

#[test]
fn a_directory_outside_git_scores_the_whole_tree() {
    let root = fixture("outside");
    let count = Config::builtin().python.f04_gate + 1;
    write(&root, "pkg/wide.py", &function("wide", count));
    let output = run(&root, &[]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert!(
        stdout.contains("not a git repository; scoring the whole tree"),
        "{stdout}"
    );
    assert!(stdout.contains("F04"), "{stdout}");
}

#[test]
fn a_clean_checkout_gates_nothing_until_strict() {
    let root = fixture("clean");
    git(&root, &["init", "-b", "main"]);
    let count = Config::builtin().python.f04_gate + 1;
    write(&root, "pkg/wide.py", &function("wide", count));
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-m", "add wide"]);

    let inherited = run(&root, &[]);
    let stdout = String::from_utf8(inherited.stdout).unwrap();
    let stderr = String::from_utf8(inherited.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(inherited.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("inherited"), "{stdout}");
    assert!(stdout.contains("pass"), "{stdout}");

    let strict = run(&root, &["--strict"]);
    let stdout = String::from_utf8(strict.stdout).unwrap();
    assert!(strict.stderr.is_empty());
    assert_eq!(strict.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("fail"), "{stdout}");
    assert!(!stdout.contains("inherited"), "{stdout}");
}

#[test]
fn a_rust_file_is_scored_and_cpp_still_waits() {
    let root = fixture("src/lib.rs");
    write(&root, "src/lib.rs", "fn ready() {}\n");
    let output = run(&root, &["--strict"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(0), "{stdout}");

    let header = "include/widget.hpp";
    let root = fixture(header);
    write(&root, header, "int ready() { return 1; }\n");
    let output = run(&root, &["--strict"]);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(output.stdout.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert_eq!(stderr, "no C++ adapter for include/widget.hpp\n");
}

#[test]
fn an_exclude_drops_the_file_and_tests_stay_out_until_asked() {
    let root = fixture("exclude");
    let over = Config::builtin().python.f04_gate + 1;
    let at = Config::builtin().python.f04_gate;
    write(&root, "pkg/wide.py", &function("wide", over));
    write(&root, "pkg/ok.py", &function("ok", at));
    write(&root, "slopmeter.toml", "exclude = [\"pkg/wide.py\"]\n");
    let hidden = run(&root, &["--strict"]);
    let stdout = String::from_utf8(hidden.stdout).unwrap();
    assert!(hidden.stderr.is_empty(), "{stdout}");
    assert_eq!(hidden.status.code(), Some(0), "{stdout}");
    assert!(!stdout.contains("wide"), "{stdout}");

    let tests = fixture("tests");
    write(&tests, "tests/test_wide.py", &function("wide", over));
    let skipped = run(&tests, &["--strict"]);
    let stdout = String::from_utf8(skipped.stdout).unwrap();
    assert_eq!(skipped.status.code(), Some(0), "{stdout}");
    assert!(!stdout.contains("F04"), "{stdout}");
    let scored = run(&tests, &["--strict", "--tests"]);
    let stdout = String::from_utf8(scored.stdout).unwrap();
    assert_eq!(scored.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("F04"), "{stdout}");
}

#[test]
fn coverage_of_zero_gates_crap() {
    let root = fixture("crap");
    let decisions = decisions_for_uncovered_crap(Config::builtin().python.f02_gate);
    let source = cold_source(decisions);
    write(&root, "pkg/cold.py", &source);
    write(&root, "coverage.lcov", &lcov("pkg/cold.py", &source));
    let output = run(&root, &["--strict", "--coverage=coverage.lcov"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("F02"), "{stdout}");
    assert!(stdout.contains("cold"), "{stdout}");
}

#[test]
fn a_bad_config_and_a_missing_directory_exit_2() {
    let root = fixture("config");
    write(&root, "slopmeter.toml", "nope = 1\n");
    let output = run(&root, &["--strict"]);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(output.stdout.is_empty());
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stderr, "unknown key nope\n");

    let missing = root.join("missing");
    let output = invoke(&["--strict", missing.to_str().unwrap()]);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(output.stdout.is_empty());
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.contains("is not a directory"), "{stderr}");
}

fn decisions_for_uncovered_crap(gate: u32) -> u32 {
    let mut cyclomatic = 1u32;
    loop {
        let score = f64::from(cyclomatic) * f64::from(cyclomatic) + f64::from(cyclomatic);
        if score > f64::from(gate) {
            return cyclomatic - 1;
        }
        cyclomatic += 1;
    }
}

fn cold_source(decisions: u32) -> String {
    let mut source = String::from("def cold():\n");
    for index in 0..decisions {
        source.push_str(&format!(
            "    if {index} == {index}:\n        return {index}\n"
        ));
    }
    source.push_str("    return 0\n");
    source
}

fn lcov(path: &str, source: &str) -> String {
    let mut text = format!("SF:{path}\n");
    for (index, _) in source.lines().enumerate() {
        text.push_str(&format!("DA:{},0\n", index + 1));
    }
    text.push_str("end_of_record\n");
    text
}
