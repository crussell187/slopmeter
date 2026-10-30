use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use slopmeter::inputs::Config;

fn corpus() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus/rust")
}

#[test]
fn pins_record_the_clean_repos_and_the_rust_gates() {
    let text = std::fs::read_to_string(corpus().join("pins.txt")).unwrap();
    let mut lines = text.lines().filter(|line| !line.is_empty());
    for repo in ["serde-rs/serde", "rust-lang/regex", "tokio-rs/bytes"] {
        let line = lines.next().unwrap_or_else(|| panic!("missing {repo}"));
        let (name, sha) = line.split_once(' ').unwrap_or_else(|| panic!("{line}"));
        assert_eq!(name, repo);
        assert_eq!(sha.len(), 40, "{sha}");
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()), "{sha}");
    }
    let profile = Config::builtin().rust;
    let gates = [
        ("F01", profile.f01_gate.to_string()),
        ("F02", profile.f02_gate.to_string()),
        ("F03", format!("{} {}", profile.f03_warn, profile.f03_gate)),
        ("F04", profile.f04_gate.to_string()),
        ("F05", profile.f05_gate.to_string()),
        ("F06", profile.f06_gate.to_string()),
        (
            "F08",
            format!("{} {}", profile.f08_receiver_hops, profile.f08_other_hops),
        ),
        ("F11", profile.f11_gate.to_string()),
    ];
    for (rule, value) in gates {
        let line = lines.next().unwrap_or_else(|| panic!("missing {rule}"));
        assert_eq!(line, format!("{rule} {value}"));
    }
    assert!(lines.next().is_none());
}

#[test]
fn the_seed_names_no_rule_and_no_threshold() {
    let seed = std::fs::read_to_string(corpus().join("seed.txt")).unwrap();
    let folded = seed.to_ascii_lowercase();
    assert!(!folded.contains("f0"), "{seed}");
    assert!(!folded.contains("threshold"), "{seed}");
    assert!(!folded.contains("gate"), "{seed}");
}

#[test]
fn the_sloppy_tree_gates_every_gating_rule_and_warns_on_f07() {
    let root = sloppy_outside_this_repo("rust");
    let output = Command::new(env!("CARGO_BIN_EXE_slopmeter"))
        .args(["--strict", "--json", "--coverage=coverage.lcov"])
        .arg(&root)
        .output()
        .expect("run slopmeter");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.is_empty(), "{stderr}");
    assert_eq!(output.status.code(), Some(1), "{stdout}");
    let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let inherited = doc["inherited"].as_array().unwrap();
    assert!(
        !inherited.is_empty()
            && inherited.iter().all(|finding| {
                finding["rule"] == "S06" && finding["measured_value"].as_f64() == Some(0.0)
            }),
        "{stdout}"
    );
    let found: BTreeSet<_> = doc["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| {
            (
                finding["rule"].as_str().unwrap(),
                finding["level"].as_str().unwrap(),
            )
        })
        .collect();
    let expected = BTreeSet::from([
        ("F01", "gate"),
        ("F02", "gate"),
        ("F03", "gate"),
        ("F04", "gate"),
        ("F05", "gate"),
        ("F06", "gate"),
        ("F07", "warn"),
        ("F08", "gate"),
        ("F11", "gate"),
        ("F15", "gate"),
    ]);
    assert_eq!(found, expected, "{stdout}");
    assert_wide_margin(&doc, &stdout);
}

fn sloppy_outside_this_repo(language: &str) -> PathBuf {
    let dest = std::env::temp_dir().join(format!(
        "slopmeter-{language}-sloppy-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dest);
    copy_tree(&corpus().join("sloppy"), &dest);
    freshen(&dest.join("coverage.lcov"));
    dest
}

fn freshen(path: &Path) {
    let file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    file.set_modified(std::time::SystemTime::now()).unwrap();
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), &dest).unwrap();
        }
    }
}

fn assert_wide_margin(doc: &serde_json::Value, stdout: &str) {
    let mut worst: BTreeMap<&str, (f64, f64)> = BTreeMap::new();
    for finding in doc["findings"].as_array().unwrap() {
        let rule = finding["rule"].as_str().unwrap();
        let measured = finding["measured_value"].as_f64().unwrap();
        let threshold = finding["threshold"].as_f64().unwrap();
        worst
            .entry(rule)
            .and_modify(|(best, gate)| {
                if measured > *best {
                    *best = measured;
                    *gate = threshold;
                }
            })
            .or_insert((measured, threshold));
    }
    for (rule, (measured, threshold)) in worst {
        if rule == "F07" {
            assert_eq!(measured, 100.0, "{stdout}");
            continue;
        }
        if rule == "F15" {
            assert_eq!(measured, 1.0, "{stdout}");
            assert_eq!(threshold, 0.0, "{stdout}");
            continue;
        }
        assert!(
            measured >= threshold * 1.5 && measured > threshold + 1.0,
            "{rule} measured {measured} against {threshold}\n{stdout}"
        );
    }
}
