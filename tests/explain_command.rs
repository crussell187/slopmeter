use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    let dir = std::env::temp_dir().join(format!("slopmeter-explain-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    Command::new(env!("CARGO_BIN_EXE_slopmeter"))
        .args(args)
        .current_dir(&dir)
        .output()
        .expect("run slopmeter")
}

#[test]
fn explain_prints_the_shipped_threshold_when_the_config_raises_a_gate() {
    let dir = std::env::temp_dir().join(format!("slopmeter-explain-config-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("slopmeter.toml"),
        "[profile.python.F04]\ngate = 20\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_slopmeter"))
        .args(["explain", "F04"])
        .current_dir(&dir)
        .output()
        .expect("run slopmeter");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    assert!(output.stderr.is_empty());
    assert!(stdout.contains("Python    10"));
    assert!(!stdout.contains("gate = 20"));
}

#[test]
fn explain_prints_the_shipped_page_and_exits_0() {
    for rule in [
        "F01", "F02", "F03", "F04", "F05", "F06", "F07", "F08", "F11", "S06", "S07",
    ] {
        let output = run(&["explain", rule]);
        assert_eq!(output.status.code(), Some(0), "{rule}");
        assert!(output.stderr.is_empty(), "{rule}");
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(stdout, slopmeter::explain::page(rule).unwrap());
        assert!(!stdout.contains("Shipped stand-in:"), "{rule}");
    }
}

#[test]
fn an_unknown_rule_exits_2() {
    for rule in ["F09", "f08"] {
        let output = run(&["explain", rule]);
        assert_eq!(output.status.code(), Some(2), "{rule}");
        assert!(output.stdout.is_empty(), "{rule}");
        assert_eq!(output.stderr, format!("unknown rule {rule}\n").into_bytes());
    }
}

#[test]
fn explain_with_the_wrong_number_of_arguments_exits_2() {
    let missing = run(&["explain"]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(missing.stdout.is_empty());
    assert_eq!(missing.stderr, b"explain needs a rule\n");

    let extra = run(&["explain", "F08", "F09"]);
    assert_eq!(extra.status.code(), Some(2));
    assert!(extra.stdout.is_empty());
    assert_eq!(extra.stderr, b"explain takes one rule\n");
}
