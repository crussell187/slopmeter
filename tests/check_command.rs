use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use slopmeter::inputs::Config;

fn fixture(name: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "slopmeter-check-{name}-{}-{}",
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
    invoke_from(&std::env::temp_dir(), args)
}

fn invoke_from(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_slopmeter"))
        .args(args)
        .current_dir(cwd)
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

    let mutation = invoke(&["--mutation-rust"]);
    assert_eq!(mutation.status.code(), Some(2));
    assert!(mutation.stdout.is_empty());
    assert_eq!(mutation.stderr, b"mutation-rust needs a path\n");

    let python = invoke(&["--mutation-python"]);
    assert_eq!(python.status.code(), Some(2));
    assert!(python.stdout.is_empty());
    assert_eq!(python.stderr, b"mutation-python needs a path\n");

    let cpp = invoke(&["--mutation-cpp"]);
    assert_eq!(cpp.status.code(), Some(2));
    assert!(cpp.stdout.is_empty());
    assert_eq!(cpp.stderr, b"mutation-cpp needs a path\n");

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
fn a_rust_file_and_a_cpp_file_are_scored() {
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
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(0), "{stdout}");
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
fn an_inline_rust_test_is_absent_until_tests() {
    let root = fixture("inline-test");
    let params = Config::builtin().rust.f04_gate + 1;
    write(&root, "src/lib.rs", &eleven_asserts(params));

    let strict = run(&root, &["--strict"]);
    let stdout = String::from_utf8(strict.stdout).unwrap();
    let stderr = String::from_utf8(strict.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(strict.status.code(), Some(0), "{stdout}");
    assert!(!stdout.contains("eleven"), "{stdout}");

    let tests = run(&root, &["--strict", "--tests"]);
    let stdout = String::from_utf8(tests.stdout).unwrap();
    let stderr = String::from_utf8(tests.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(tests.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("eleven"), "{stdout}");
    assert!(stdout.contains("F04"), "{stdout}");
}

fn changed_subdirectory() -> PathBuf {
    let root = fixture("subdir-scope");
    git(&root, &["init", "-b", "main"]);
    let over = Config::builtin().python.f04_gate + 1;
    write(&root, "nested/wide.py", &function("wide", over));
    write(&root, "nested/stale.py", &function("stale", over));
    write(&root, "outside.py", &function("outside", over));
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-m", "base"]);
    write(
        &root,
        "nested/wide.py",
        &function("wide", over).replace("return p0", "return p0 + 1"),
    );
    write(
        &root,
        "outside.py",
        &function("outside", over).replace("return p0", "return p0 + 1"),
    );
    root
}

fn assert_subdirectory_scope(output: std::process::Output) {
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("\"scope\": \"changed\""), "{stdout}");
    assert!(stdout.contains("\"verdict\": \"fail\""), "{stdout}");
    assert!(!stdout.contains("not a git repository"), "{stdout}");
    assert!(!stdout.contains("outside"), "{stdout}");
    let inherited_at = stdout.find("\"inherited\"").unwrap_or_else(|| {
        panic!("{stdout}");
    });
    let findings = &stdout[..inherited_at];
    let inherited = &stdout[inherited_at..];
    assert!(findings.contains("\"name\": \"wide\""), "{stdout}");
    assert!(findings.contains("\"path\": \"wide.py\""), "{stdout}");
    assert!(!findings.contains("\"name\": \"stale\""), "{stdout}");
    assert!(inherited.contains("\"name\": \"stale\""), "{stdout}");
}

#[test]
fn a_run_from_a_subdirectory_scopes_the_diff() {
    let root = changed_subdirectory();
    assert_subdirectory_scope(invoke_from(&root.join("nested"), &["--json"]));
}

#[test]
fn a_subdirectory_path_scopes_the_diff() {
    let root = changed_subdirectory();
    assert_subdirectory_scope(invoke_from(&root, &["--json", "nested"]));
}

#[test]
fn a_subdirectory_run_reads_the_root_config_and_ignores_a_nested_one() {
    let root = fixture("subdir-config");
    git(&root, &["init", "-b", "main"]);
    let over = Config::builtin().python.f04_gate + 1;
    write(&root, "nested/wide.py", &function("wide", over));
    write(&root, "nested/ok.py", &function("ok", over));
    write(&root, "slopmeter.toml", "exclude = [\"nested/wide.py\"]\n");
    write(
        &root,
        "nested/slopmeter.toml",
        "exclude = [\"nested/ok.py\"]\n",
    );

    let output = invoke_from(&root.join("nested"), &["--strict"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("ok"), "{stdout}");
    assert!(!stdout.contains("wide"), "{stdout}");
    assert!(!stdout.contains("not a git repository"), "{stdout}");
}

#[test]
fn a_subdirectory_run_honors_the_root_gitignore() {
    let root = fixture("subdir-ignore");
    git(&root, &["init", "-b", "main"]);
    let over = Config::builtin().python.f04_gate + 1;
    let at = Config::builtin().python.f04_gate;
    write(&root, ".gitignore", "secret.py\n");
    write(&root, "nested/secret.py", &function("secret", over));
    write(&root, "nested/ok.py", &function("ok", at));
    write(&root, "outside.py", &function("outside", over));

    let output = invoke_from(&root.join("nested"), &["--strict"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    assert!(!stdout.contains("secret"), "{stdout}");
    assert!(!stdout.contains("outside"), "{stdout}");
    assert!(!stdout.contains("not a git repository"), "{stdout}");
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

#[test]
fn an_unparsed_file_is_noted_and_a_clean_file_is_not() {
    let root = fixture("unparsed");
    write(
        &root,
        "pkg/wrap.hpp",
        "\
int ok() { return 1; }
int bad( { return 2; }
int also() { return 3; }
int more() { return 4; }
",
    );
    write(&root, "pkg/ok.py", "def ok():\n    return 1\n");
    let output = run(&root, &["--strict", "--json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(doc["verdict"], "pass");
    let notes = doc["notes"].as_array().unwrap();
    let unparsed: Vec<_> = notes
        .iter()
        .filter(|note| note.get("path").is_some())
        .collect();
    assert_eq!(unparsed.len(), 1, "{notes:?}");
    assert_eq!(unparsed[0]["path"], "pkg/wrap.hpp");
    assert_eq!(
        unparsed[0]["message"],
        "The parser left 0.250 of the lines unscored."
    );
}

#[test]
fn a_function_like_macro_is_noted_and_an_object_like_macro_is_not() {
    let root = fixture("macro-note");
    write(
        &root,
        "pkg/check.hpp",
        "\
#define CHECK(x) if (!(x)) return false;
bool run(int x) { CHECK(x); return true; }
",
    );
    write(
        &root,
        "pkg/two.hpp",
        "\
#define CHECK(x) if (!(x)) return false;
#define ALSO(x) if (x) return true;
int quiet() { return 1; }
",
    );
    write(
        &root,
        "pkg/const.hpp",
        "\
#define MAX 3
int ok() { return MAX; }
",
    );
    let output = run(&root, &["--strict", "--json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(doc["verdict"], "pass");
    let notes = doc["notes"].as_array().unwrap();
    let filed: Vec<_> = notes
        .iter()
        .filter(|note| note.get("path").is_some())
        .collect();
    assert_eq!(filed.len(), 2, "{notes:?}");
    assert_eq!(filed[0]["path"], "pkg/check.hpp");
    assert_eq!(
        filed[0]["message"],
        "The parser left 1 function-like macro body unscored."
    );
    assert_eq!(filed[1]["path"], "pkg/two.hpp");
    assert_eq!(
        filed[1]["message"],
        "The parser left 2 function-like macro bodies unscored."
    );
}

fn eleven_asserts(params: u32) -> String {
    let names = (0..params)
        .map(|index| format!("a{index}: i32"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut source = format!("fn kept() {{}}\n\n#[test]\nfn eleven({names}) {{\n");
    for _ in 0..11 {
        source.push_str("    assert!(true);\n");
    }
    source.push_str("}\n");
    source
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

const CLASSIFY: &str = "fn classify(n: i32) -> i32 {\n    n\n}\n";

fn outcomes(file: &str, summaries: &[(&str, u32)]) -> String {
    let mut rows = vec![r#"{"scenario":"Baseline","summary":"Success"}"#.to_string()];
    for (summary, line) in summaries {
        rows.push(format!(
            r#"{{"scenario":{{"Mutant":{{"file":"{file}","function":{{"function_name":"ignored"}},"span":{{"start":{{"line":{line},"column":1}},"end":{{"line":{line},"column":2}}}}}}}},"summary":"{summary}"}}"#
        ));
    }
    format!(r#"{{"outcomes":[{}]}}"#, rows.join(","))
}

fn touched_rust(name: &str) -> PathBuf {
    let root = fixture(name);
    git(&root, &["init", "-b", "main"]);
    write(&root, "src/lib.rs", CLASSIFY);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-m", "base"]);
    write(
        &root,
        "src/lib.rs",
        &CLASSIFY.replace("    n\n", "    n + 0\n"),
    );
    root
}

fn s07_rows<'a>(doc: &'a serde_json::Value, key: &str) -> Vec<&'a serde_json::Value> {
    doc[key]
        .as_array()
        .unwrap()
        .iter()
        .filter(|finding| finding["rule"] == "S07")
        .collect()
}

#[test]
fn a_cargo_mutants_report_gates_a_touched_rust_function_at_the_missed_mutant() {
    let root = touched_rust("mutants");
    write(
        &root,
        "outcomes.json",
        &outcomes(
            "src/lib.rs",
            &[("Timeout", 1), ("MissedMutant", 2), ("Unviable", 1)],
        ),
    );

    let output = run(&root, &["--json", "--mutation-rust=outcomes.json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let rows = s07_rows(&doc, "findings");
    assert_eq!(rows.len(), 1, "{stdout}");
    assert_eq!(rows[0]["symbol"]["name"], "classify");
    assert_eq!(rows[0]["line"], 2);
    assert_eq!(rows[0]["measured_value"], 0.5);
    assert_eq!(rows[0]["threshold"], 1.0);
    assert_eq!(rows[0]["weight"], 10);
    assert!(s07_rows(&doc, "inherited").is_empty(), "{stdout}");

    let strict = run(
        &root,
        &["--strict", "--json", "--mutation-rust=outcomes.json"],
    );
    let stdout = String::from_utf8(strict.stdout).unwrap();
    assert_eq!(strict.status.code(), Some(0), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(s07_rows(&doc, "findings").is_empty(), "{stdout}");
    let inherited = s07_rows(&doc, "inherited");
    assert_eq!(inherited.len(), 1, "{stdout}");
    assert_eq!(inherited[0]["measured_value"], 0.5);
}

#[test]
fn a_missing_rust_mutation_report_fails_the_touch_and_strict_adds_no_row() {
    let root = touched_rust("missing-mutants");
    let missing = run(&root, &["--json", "--mutation-rust=outcomes.json"]);
    let stdout = String::from_utf8(missing.stdout).unwrap();
    assert_eq!(missing.status.code(), Some(1), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let rows = s07_rows(&doc, "findings");
    assert_eq!(rows.len(), 1, "{stdout}");
    assert!(rows[0]["measured_value"].is_null(), "{stdout}");
    assert_eq!(rows[0]["line"], 1);
    let notes = doc["notes"].as_array().unwrap();
    assert!(
        notes
            .iter()
            .any(|note| note["message"].as_str().unwrap().contains("outcomes.json")),
        "{notes:?}"
    );

    let strict = run(
        &root,
        &["--strict", "--json", "--mutation-rust=outcomes.json"],
    );
    let stdout = String::from_utf8(strict.stdout).unwrap();
    assert_eq!(strict.status.code(), Some(0), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(s07_rows(&doc, "findings").is_empty(), "{stdout}");
    assert!(s07_rows(&doc, "inherited").is_empty(), "{stdout}");

    let unset = run(&root, &["--json"]);
    let stdout = String::from_utf8(unset.stdout).unwrap();
    assert_eq!(unset.status.code(), Some(1), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(s07_rows(&doc, "findings").len(), 1, "{stdout}");
    assert!(
        doc["notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note["message"] == "No mutation report was given."),
        "{stdout}"
    );
}

#[test]
fn an_unreadable_mutation_report_exits_2_and_a_waiver_for_s07_fails() {
    let root = touched_rust("bad-mutants");
    write(&root, "outcomes.json", "{");
    let output = run(&root, &["--strict", "--mutation-rust=outcomes.json"]);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(output.stdout.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.contains("cannot be parsed"), "{stderr}");

    write(
        &root,
        "src/lib.rs",
        &format!("// slopmeter: waive S07 0 later\n{CLASSIFY}"),
    );
    write(
        &root,
        "outcomes.json",
        &outcomes("src/lib.rs", &[("CaughtMutant", 1), ("Timeout", 2)]),
    );
    let output = run(&root, &["--json", "--mutation-rust=outcomes.json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let rows = s07_rows(&doc, "findings");
    assert!(
        rows.iter()
            .any(|row| row["waiver"]["rejection"] == "S07 cannot be waived."),
        "{stdout}"
    );
    assert!(
        rows.iter().all(|row| row["measured_value"].is_null()),
        "{stdout}"
    );
}

#[test]
fn outside_git_a_shown_mutation_row_is_inherited_and_the_config_names_the_report() {
    let root = fixture("outside-mutants");
    write(&root, "src/lib.rs", CLASSIFY);
    write(
        &root,
        "outcomes.json",
        &outcomes("src/lib.rs", &[("MissedMutant", 2)]),
    );
    let output = run(&root, &["--json", "--mutation-rust=outcomes.json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(s07_rows(&doc, "findings").is_empty(), "{stdout}");
    assert_eq!(s07_rows(&doc, "inherited").len(), 1, "{stdout}");
    assert_eq!(s07_rows(&doc, "inherited")[0]["measured_value"], 0.0);
    assert!(stdout.contains("not a git repository"), "{stdout}");

    let root = touched_rust("config-mutants");
    write(
        &root,
        "slopmeter.toml",
        "[mutation]\nrust = \"outcomes.json\"\n",
    );
    write(
        &root,
        "outcomes.json",
        &outcomes("src/lib.rs", &[("CaughtMutant", 1), ("MissedMutant", 2)]),
    );
    let output = run(&root, &["--json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(s07_rows(&doc, "findings").len(), 1, "{stdout}");
    assert_eq!(s07_rows(&doc, "findings")[0]["measured_value"], 0.5);
}

const PY_CLASSIFY: &str = "def classify(n):\n    return n > 1\n";

fn touched_python(name: &str) -> PathBuf {
    let root = fixture(name);
    git(&root, &["init", "-b", "main"]);
    write(&root, "pkg/a.py", "def classify(n):\n    return n > 0\n");
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-m", "base"]);
    write(&root, "pkg/a.py", PY_CLASSIFY);
    root
}

fn write_mutmut(root: &Path, meta: &str) {
    let mutated = "\
def x_classify__mutmut_orig(n):
    return n > 1

def x_classify__mutmut_1(n):
    return n > 0

def x_classify__mutmut_2(n):
    return n < 1
";
    write(root, "mutants/pkg/a.py", mutated);
    write(
        root,
        "mutants/pkg/a.py.meta",
        &format!("{{\"exit_code_by_key\": {meta}, \"hash_by_function_name\": {{}}}}"),
    );
    write(root, "mutants/mutmut-cicd-stats.json", "{");
}

#[test]
fn a_mutmut_store_gates_a_touched_python_function_at_the_survived_mutant() {
    let root = touched_python("mutmut");
    write_mutmut(
        &root,
        r#"{"pkg.a.x_classify__mutmut_1": 1, "pkg.a.x_classify__mutmut_2": 0}"#,
    );

    let output = run(&root, &["--json", "--mutation-python=mutants"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let rows = s07_rows(&doc, "findings");
    assert_eq!(rows.len(), 1, "{stdout}");
    assert_eq!(rows[0]["symbol"]["name"], "classify");
    assert_eq!(rows[0]["line"], 2);
    assert_eq!(rows[0]["measured_value"], 0.5);
    assert_eq!(rows[0]["weight"], 10);
    assert!(s07_rows(&doc, "inherited").is_empty(), "{stdout}");

    let strict = run(&root, &["--strict", "--json", "--mutation-python=mutants"]);
    let stdout = String::from_utf8(strict.stdout).unwrap();
    assert_eq!(strict.status.code(), Some(0), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(s07_rows(&doc, "findings").is_empty(), "{stdout}");
    assert_eq!(s07_rows(&doc, "inherited").len(), 1, "{stdout}");
    assert_eq!(s07_rows(&doc, "inherited")[0]["measured_value"], 0.5);
}

#[test]
fn a_missing_python_mutation_store_fails_the_touch_and_a_bad_store_exits_2() {
    let root = touched_python("missing-mutmut");
    let missing = run(&root, &["--json", "--mutation-python=mutants"]);
    let stdout = String::from_utf8(missing.stdout).unwrap();
    assert_eq!(missing.status.code(), Some(1), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let rows = s07_rows(&doc, "findings");
    assert_eq!(rows.len(), 1, "{stdout}");
    assert!(rows[0]["measured_value"].is_null(), "{stdout}");
    assert!(
        doc["notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note["message"].as_str().unwrap().contains("mutants")),
        "{stdout}"
    );

    write(&root, "mutants/pkg/a.py.meta", "{");
    let output = run(&root, &["--strict", "--mutation-python=mutants"]);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("cannot be parsed"), "{stderr}");

    write(
        &root,
        "pkg/a.py",
        &format!("# slopmeter: waive S07 0 later\n{PY_CLASSIFY}"),
    );
    write_mutmut(&root, r#"{"pkg.a.x_classify__mutmut_1": 1}"#);
    let output = run(&root, &["--json", "--mutation-python=mutants"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(
        s07_rows(&doc, "findings")
            .iter()
            .any(|row| row["waiver"]["rejection"] == "S07 cannot be waived."),
        "{stdout}"
    );

    let root = touched_python("config-mutmut");
    write(
        &root,
        "slopmeter.toml",
        "[mutation]\npython = \"mutants\"\n",
    );
    write_mutmut(
        &root,
        r#"{"pkg.a.x_classify__mutmut_1": 36, "pkg.a.x_classify__mutmut_2": 5}"#,
    );
    let output = run(&root, &["--json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(s07_rows(&doc, "findings").len(), 1, "{stdout}");
    assert_eq!(s07_rows(&doc, "findings")[0]["measured_value"], 0.5);
}

const CPP_CLASSIFY: &str = "int classify(int n) {\n    return n > 1;\n}\n";

fn elements(file: &str, statuses: &[(&str, u32)]) -> String {
    let rows: Vec<String> = statuses
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

fn touched_cpp(name: &str) -> PathBuf {
    let root = fixture(name);
    git(&root, &["init", "-b", "main"]);
    write(
        &root,
        "src/classify.cpp",
        "int classify(int n) {\n    return n > 0;\n}\n",
    );
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-m", "base"]);
    write(&root, "src/classify.cpp", CPP_CLASSIFY);
    root
}

#[test]
fn a_mull_report_gates_a_touched_cpp_function_at_the_survived_mutant() {
    let root = touched_cpp("mull");
    write(
        &root,
        "mull.json",
        &elements(
            "src/classify.cpp",
            &[
                ("Timeout", 1),
                ("Survived", 2),
                ("CompileError", 2),
                ("RuntimeError", 2),
            ],
        ),
    );

    let output = run(&root, &["--json", "--mutation-cpp=mull.json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let rows = s07_rows(&doc, "findings");
    assert_eq!(rows.len(), 1, "{stdout}");
    assert_eq!(rows[0]["symbol"]["name"], "classify");
    assert_eq!(rows[0]["line"], 2);
    assert_eq!(rows[0]["measured_value"], 0.5);
    assert_eq!(rows[0]["threshold"], 1.0);
    assert_eq!(rows[0]["weight"], 10);
    assert!(s07_rows(&doc, "inherited").is_empty(), "{stdout}");

    let strict = run(&root, &["--strict", "--json", "--mutation-cpp=mull.json"]);
    let stdout = String::from_utf8(strict.stdout).unwrap();
    assert_eq!(strict.status.code(), Some(0), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(s07_rows(&doc, "findings").is_empty(), "{stdout}");
    let inherited = s07_rows(&doc, "inherited");
    assert_eq!(inherited.len(), 1, "{stdout}");
    assert_eq!(inherited[0]["measured_value"], 0.5);
}

#[test]
fn a_missing_cpp_mutation_report_fails_the_touch_and_a_bad_report_exits_2() {
    let root = touched_cpp("missing-mull");
    let missing = run(&root, &["--json", "--mutation-cpp=mull.json"]);
    let stdout = String::from_utf8(missing.stdout).unwrap();
    assert_eq!(missing.status.code(), Some(1), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let rows = s07_rows(&doc, "findings");
    assert_eq!(rows.len(), 1, "{stdout}");
    assert!(rows[0]["measured_value"].is_null(), "{stdout}");
    assert_eq!(rows[0]["line"], 1);
    assert!(
        doc["notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note["message"].as_str().unwrap().contains("mull.json")),
        "{stdout}"
    );

    let strict = run(&root, &["--strict", "--json", "--mutation-cpp=mull.json"]);
    let stdout = String::from_utf8(strict.stdout).unwrap();
    assert_eq!(strict.status.code(), Some(0), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(s07_rows(&doc, "findings").is_empty(), "{stdout}");
    assert!(s07_rows(&doc, "inherited").is_empty(), "{stdout}");

    write(&root, "mull.json", "{");
    let output = run(&root, &["--strict", "--mutation-cpp=mull.json"]);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("cannot be parsed"), "{stderr}");

    write(
        &root,
        "src/classify.cpp",
        &format!("// slopmeter: waive S07 0 later\n{CPP_CLASSIFY}"),
    );
    write(
        &root,
        "mull.json",
        &elements("src/classify.cpp", &[("Killed", 2), ("Pending", 2)]),
    );
    let output = run(&root, &["--json", "--mutation-cpp=mull.json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(
        s07_rows(&doc, "findings")
            .iter()
            .any(|row| row["waiver"]["rejection"] == "S07 cannot be waived."),
        "{stdout}"
    );
    assert!(
        s07_rows(&doc, "findings")
            .iter()
            .all(|row| row["measured_value"].is_null()),
        "{stdout}"
    );

    let root = fixture("outside-mull");
    write(&root, "src/classify.cpp", CPP_CLASSIFY);
    write(
        &root,
        "mull.json",
        &elements("src/classify.cpp", &[("NoCoverage", 2)]),
    );
    let output = run(&root, &["--json", "--mutation-cpp=mull.json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(s07_rows(&doc, "findings").is_empty(), "{stdout}");
    assert_eq!(s07_rows(&doc, "inherited").len(), 1, "{stdout}");
    assert_eq!(s07_rows(&doc, "inherited")[0]["measured_value"], 0.0);

    let root = touched_cpp("config-mull");
    write(&root, "slopmeter.toml", "[mutation]\ncpp = \"mull.json\"\n");
    write(
        &root,
        "mull.json",
        &elements("src/classify.cpp", &[("Killed", 1), ("Survived", 2)]),
    );
    let output = run(&root, &["--json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(s07_rows(&doc, "findings").len(), 1, "{stdout}");
    assert_eq!(s07_rows(&doc, "findings")[0]["measured_value"], 0.5);
}

#[test]
fn a_departure_is_noted_before_the_run_notes_and_the_finding_uses_it() {
    let root = fixture("departure");
    let builtin = Config::builtin();
    let python = &builtin.python;
    let gate = python.f04_gate + 1;
    let rust_gate = builtin.rust.f01_gate + 1;
    write(
        &root,
        "slopmeter.toml",
        &format!(
            r#"
[profile.python.F01]
gate = {f01}

[profile.python.F03]
warn = 7

[profile.python.F04]
gate = {gate}

[profile.python.F08]
allow = ["builder", "with_timeout"]

[profile.rust.F01]
gate = {rust_gate}

[profile.cpp.F11]
gate = 0
"#,
            f01 = python.f01_gate,
            gate = gate,
            rust_gate = rust_gate,
        ),
    );
    write(&root, "pkg/wide.py", &function("wide", gate));

    let output = run(&root, &["--strict", "--json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(doc["findings"].as_array().unwrap().is_empty(), "{stdout}");
    let notes = doc["notes"].as_array().unwrap();
    let messages: Vec<_> = notes
        .iter()
        .map(|note| note["message"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        messages,
        vec![
            "Python F03 warn is 7, built-in 8".to_string(),
            format!("Python F04 gate is {gate}, built-in {}", python.f04_gate),
            "Python F08 allow adds builder, with_timeout".to_string(),
            format!(
                "Rust F01 gate is {rust_gate}, built-in {}",
                builtin.rust.f01_gate
            ),
            format!("C++ F11 gate is 0, built-in {}", builtin.cpp.f11_gate),
            "No coverage report was given. CRAP was not computed.".to_string(),
            "No mutation report was given.".to_string(),
        ],
        "{stdout}"
    );
    assert!(notes.iter().take(5).all(|note| note.get("path").is_none()));

    write(&root, "pkg/wide.py", &function("wide", gate + 1));
    let output = run(&root, &["--strict", "--json"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let findings = doc["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{stdout}");
    assert_eq!(findings[0]["rule"], "F04");
    assert_eq!(findings[0]["threshold"], gate);
    assert_eq!(
        doc["notes"][1]["message"],
        format!("Python F04 gate is {gate}, built-in {}", python.f04_gate)
    );
}

#[test]
fn a_gate_outside_its_range_exits_2_and_names_the_key() {
    let root = fixture("range");
    write(&root, "pkg/ok.py", "def ok():\n    return 1\n");
    write(&root, "slopmeter.toml", "[profile.python.F01]\ngate = 97\n");
    let output = run(&root, &["--strict"]);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(output.stdout.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stderr, "profile.python.F01.gate is out of range\n");

    write(&root, "slopmeter.toml", "[profile.python.F03]\ngate = 0\n");
    let output = run(&root, &["--strict"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "profile.python.F03.gate is out of range\n"
    );

    write(&root, "slopmeter.toml", "[profile.python.F03]\nwarn = 21\n");
    let output = run(&root, &["--strict"]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "profile.python.F03.warn is out of range\n"
    );
}
