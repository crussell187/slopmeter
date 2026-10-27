use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::{Command, Output};

use crate::ir::{File, Function, Language, Span};
use crate::reporters::{Note, Scope};
use crate::rules::Finding;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeOptions {
    pub changed: Option<String>,
    pub strict: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scoped {
    pub scope: Scope,
    pub findings: Vec<Finding>,
    pub inherited: Vec<Finding>,
    pub notes: Vec<Note>,
}

#[derive(Debug)]
pub enum ScopeError {
    NoBase,
    BadRef(String),
    Git(String),
}

impl std::fmt::Display for ScopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScopeError::NoBase => write!(f, "no base ref; pass --changed=<ref> or --strict"),
            ScopeError::BadRef(rev) => {
                write!(
                    f,
                    "changed ref {rev} is not a revision; pass --changed=<ref> or --strict"
                )
            }
            ScopeError::Git(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for ScopeError {}

const NOT_A_REPOSITORY: &str = "not a git repository; scoring the whole tree";

pub fn partition(
    root: &Path,
    files: &[File],
    findings: &[Finding],
    options: ScopeOptions,
) -> Result<Scoped, ScopeError> {
    if options.strict {
        return Ok(whole(findings, Vec::new()));
    }
    if !work_tree(root)? {
        return Ok(whole(
            findings,
            vec![Note {
                path: None,
                message: NOT_A_REPOSITORY.to_string(),
            }],
        ));
    }
    let change = load_change(root, options.changed.as_deref())?;
    let touched = touched_symbols(root, files, &change);
    let mut kept = Vec::new();
    let mut inherited = Vec::new();
    for finding in findings {
        if touched.contains(&symbol(finding)) {
            kept.push(finding.clone());
        } else {
            inherited.push(finding.clone());
        }
    }
    Ok(Scoped {
        scope: Scope::Changed,
        findings: kept,
        inherited,
        notes: Vec::new(),
    })
}

fn whole(findings: &[Finding], notes: Vec<Note>) -> Scoped {
    Scoped {
        scope: Scope::Strict,
        findings: findings.to_vec(),
        inherited: Vec::new(),
        notes,
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Symbol {
    path: String,
    name: String,
    param_types: Vec<String>,
    qualifier: String,
}

struct Change {
    base: String,
    files: BTreeMap<String, Delta>,
    untracked: BTreeSet<String>,
}

struct Delta {
    old_path: Option<String>,
    added: BTreeSet<u32>,
    deleted: BTreeSet<u32>,
}

enum OldSpans {
    None,
    Precise(Vec<Function>),
    Whole,
}

fn symbol(finding: &Finding) -> Symbol {
    Symbol {
        path: finding.path.clone(),
        name: finding.name.clone(),
        param_types: finding.param_types.clone(),
        qualifier: finding.qualifier.clone(),
    }
}

fn symbol_of(path: &str, function: &Function) -> Symbol {
    Symbol {
        path: path.to_string(),
        name: function.name.clone(),
        param_types: function.param_types.clone(),
        qualifier: function.qualifier.clone(),
    }
}

fn touched_symbols(root: &Path, files: &[File], change: &Change) -> BTreeSet<Symbol> {
    let mut touched = BTreeSet::new();
    for file in files {
        let delta = change.files.get(&file.path);
        let old = delta.map(|delta| old_spans(root, &change.base, file, delta));
        for function in &file.functions {
            let hit = change.untracked.contains(&file.path)
                || delta.is_some_and(|delta| {
                    delta
                        .added
                        .iter()
                        .any(|line| contains(*line, function.touch))
                        || old
                            .as_ref()
                            .is_some_and(|old| deleted_hit(old, delta, function))
                });
            if hit {
                touched.insert(symbol_of(&file.path, function));
            }
        }
    }
    touched
}

fn deleted_hit(old: &OldSpans, delta: &Delta, function: &Function) -> bool {
    match old {
        OldSpans::None => false,
        OldSpans::Whole => !delta.deleted.is_empty(),
        OldSpans::Precise(functions) => functions.iter().any(|before| {
            before.name == function.name
                && before.param_types == function.param_types
                && before.qualifier == function.qualifier
                && delta
                    .deleted
                    .iter()
                    .any(|line| contains(*line, before.touch))
        }),
    }
}

fn old_spans(root: &Path, base: &str, file: &File, delta: &Delta) -> OldSpans {
    if delta.deleted.is_empty() {
        return OldSpans::None;
    }
    let Some(old_path) = delta.old_path.as_deref() else {
        return OldSpans::Whole;
    };
    match load_old(root, base, file.language, old_path) {
        Some(functions) => OldSpans::Precise(functions),
        None => OldSpans::Whole,
    }
}

fn load_old(root: &Path, base: &str, language: Language, old_path: &str) -> Option<Vec<Function>> {
    let spec = format!("{base}:{old_path}");
    let out = git(root, &["--no-pager", "show", &spec]).ok()?;
    if !out.status.success() {
        return None;
    }
    match language {
        Language::Python => Some(crate::adapters::python::parse(old_path, &out.stdout).functions),
        // Those adapters are later sessions. Until they exist, a deletion touches the whole file.
        Language::Rust | Language::Cpp => None,
    }
}

fn contains(line: u32, span: Span) -> bool {
    line >= span.start && line <= span.end
}

fn load_change(root: &Path, changed: Option<&str>) -> Result<Change, ScopeError> {
    let base = resolve_base(root, changed)?;
    Ok(Change {
        files: diff_files(root, &base)?,
        untracked: untracked_paths(root)?,
        base,
    })
}

fn resolve_base(root: &Path, changed: Option<&str>) -> Result<String, ScopeError> {
    if let Some(rev) = changed {
        if !rev_exists(root, rev)? {
            return Err(ScopeError::BadRef(rev.to_string()));
        }
        return Ok(rev.to_string());
    }
    let name = if rev_exists(root, "main")? {
        "main"
    } else if rev_exists(root, "master")? {
        "master"
    } else {
        return Err(ScopeError::NoBase);
    };
    let out = git(root, &["merge-base", "HEAD", name])?;
    if !out.status.success() {
        return Err(ScopeError::NoBase);
    }
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if sha.is_empty() {
        Err(ScopeError::NoBase)
    } else {
        Ok(sha)
    }
}

fn work_tree(root: &Path) -> Result<bool, ScopeError> {
    let out = git(root, &["rev-parse", "--show-toplevel"])?;
    if !out.status.success() {
        return Ok(false);
    }
    let printed = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let Ok(top) = Path::new(&printed).canonicalize() else {
        return Ok(false);
    };
    let Ok(here) = root.canonicalize() else {
        return Ok(false);
    };
    Ok(top == here)
}

fn rev_exists(root: &Path, rev: &str) -> Result<bool, ScopeError> {
    let spec = format!("{rev}^{{commit}}");
    let out = git(root, &["rev-parse", "--verify", "--quiet", &spec])?;
    Ok(out.status.success())
}

fn diff_files(root: &Path, base: &str) -> Result<BTreeMap<String, Delta>, ScopeError> {
    let out = git(
        root,
        &[
            "--no-pager",
            "-c",
            "core.quotePath=false",
            "diff",
            "--no-color",
            "--no-ext-diff",
            "--unified=0",
            "--find-renames",
            "--relative",
            base,
        ],
    )?;
    if !out.status.success() {
        return Err(ScopeError::Git(command_error(&out)));
    }
    Ok(parse_diff(&String::from_utf8_lossy(&out.stdout)))
}

fn untracked_paths(root: &Path) -> Result<BTreeSet<String>, ScopeError> {
    let out = git(
        root,
        &[
            "--no-pager",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
        ],
    )?;
    if !out.status.success() {
        return Err(ScopeError::Git(command_error(&out)));
    }
    Ok(parse_untracked(&out.stdout))
}

fn command_error(out: &Output) -> String {
    let message = String::from_utf8_lossy(&out.stderr).trim().to_string();
    if message.is_empty() {
        format!("git exited {}", out.status)
    } else {
        message
    }
}

#[derive(Default)]
struct Open {
    old_path: Option<String>,
    new_path: Option<String>,
    added: BTreeSet<u32>,
    deleted: BTreeSet<u32>,
}

fn parse_diff(diff: &str) -> BTreeMap<String, Delta> {
    let mut files = BTreeMap::new();
    let mut open = Open::default();
    for line in diff.lines() {
        if line.starts_with("diff --git ") {
            flush(&mut open, &mut files);
            continue;
        }
        if let Some(side) = parse_side(line, "--- ") {
            open.old_path = path_of(side);
            continue;
        }
        if let Some(side) = parse_side(line, "+++ ") {
            open.new_path = path_of(side);
            continue;
        }
        let Some((old_start, old_count, new_start, new_count)) = parse_hunk(line) else {
            continue;
        };
        insert_lines(&mut open.deleted, old_start, old_count);
        insert_lines(&mut open.added, new_start, new_count);
    }
    flush(&mut open, &mut files);
    files
}

fn path_of(side: Side) -> Option<String> {
    match side {
        Side::Null => None,
        Side::Path(path) => Some(path),
    }
}

fn insert_lines(lines: &mut BTreeSet<u32>, start: u32, count: u32) {
    if count == 0 {
        return;
    }
    for line_no in start..start.saturating_add(count) {
        if line_no > 0 {
            lines.insert(line_no);
        }
    }
}

fn flush(open: &mut Open, files: &mut BTreeMap<String, Delta>) {
    let finished = std::mem::take(open);
    let Some(path) = finished.new_path else {
        return;
    };
    if finished.added.is_empty() && finished.deleted.is_empty() {
        return;
    }
    files
        .entry(path)
        .and_modify(|delta| {
            delta.added.extend(&finished.added);
            delta.deleted.extend(&finished.deleted);
            if delta.old_path.is_none() {
                delta.old_path.clone_from(&finished.old_path);
            }
        })
        .or_insert(Delta {
            old_path: finished.old_path,
            added: finished.added,
            deleted: finished.deleted,
        });
}

fn parse_untracked(bytes: &[u8]) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    let mut rest = bytes;
    while !rest.is_empty() {
        let Some(end) = rest.iter().position(|byte| *byte == 0) else {
            break;
        };
        let entry = &rest[..end];
        rest = &rest[end + 1..];
        if entry
            .first()
            .is_some_and(|byte| *byte == b'R' || *byte == b'C')
        {
            if let Some(extra) = rest.iter().position(|byte| *byte == 0) {
                rest = &rest[extra + 1..];
            }
        }
        if let Some(path) = entry.strip_prefix(b"?? ") {
            if let Ok(path) = std::str::from_utf8(path) {
                paths.insert(path.to_string());
            }
        }
    }
    paths
}

enum Side {
    Null,
    Path(String),
}

fn parse_side(line: &str, prefix: &str) -> Option<Side> {
    let rest = line.strip_prefix(prefix)?.trim_end();
    let raw = unquote(rest)?;
    if raw == "/dev/null" {
        return Some(Side::Null);
    }
    let marker = if prefix.starts_with("+++") {
        "b/"
    } else {
        "a/"
    };
    Some(Side::Path(raw.strip_prefix(marker)?.to_string()))
}

fn unquote(raw: &str) -> Option<String> {
    if !raw.starts_with('"') {
        return Some(raw.to_string());
    }
    if raw.len() < 2 || !raw.ends_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut chars = raw[1..raw.len() - 1].chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next()? {
            'n' => out.push('\n'),
            't' => out.push('\t'),
            '"' => out.push('"'),
            '\\' => out.push('\\'),
            other => out.push(other),
        }
    }
    Some(out)
}

fn parse_hunk(line: &str) -> Option<(u32, u32, u32, u32)> {
    let rest = line.strip_prefix("@@ -")?;
    let (old_start, old_count, rest) = parse_range(rest)?;
    let rest = rest.strip_prefix(" +")?;
    let (new_start, new_count, _) = parse_range(rest)?;
    Some((old_start, old_count, new_start, new_count))
}

fn parse_range(input: &str) -> Option<(u32, u32, &str)> {
    let (start, rest) = split_number(input)?;
    if let Some(rest) = rest.strip_prefix(',') {
        let (count, rest) = split_number(rest)?;
        Some((start, count, rest))
    } else {
        Some((start, 1, rest))
    }
}

fn split_number(input: &str) -> Option<(u32, &str)> {
    let end = input
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(input.len());
    if end == 0 {
        return None;
    }
    let number = input[..end].parse().ok()?;
    Some((number, &input[end..]))
}

fn git(root: &Path, args: &[&str]) -> Result<Output, ScopeError> {
    let parent = root.parent().unwrap_or(root);
    let ceiling = parent
        .canonicalize()
        .unwrap_or_else(|_| parent.to_path_buf());
    Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_CEILING_DIRECTORIES", ceiling)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .map_err(|err| ScopeError::Git(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{partition, ScopeOptions};
    use crate::adapters::python::parse;
    use crate::analyzers::measure;
    use crate::inputs::Config;
    use crate::ir::{Body, File, Function, Language, Span};
    use crate::reporters::Scope;
    use crate::rules::{evaluate, Finding, Level};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    const PAIR: &str = "\
def touched():
    if True:
        if True:
            if True:
                if True:
                    if True:
                        if True:
                            if True:
                                return 1
    return 0

def other():
    if True:
        if True:
            if True:
                if True:
                    if True:
                        if True:
                            if True:
                                return 1
    return 0
";

    fn fixture(name: &str) -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("scope-fixtures")
            .join(format!(
                "{name}-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
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

    fn repo(name: &str, branch: &str) -> PathBuf {
        let root = fixture(name);
        git(&root, &["init", "-b", branch]);
        root
    }

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    fn commit_all(root: &Path, message: &str) {
        git(root, &["add", "-A"]);
        git(root, &["commit", "-m", message]);
    }

    fn gate(path: &str, name: &str) -> Finding {
        finding(path, name, Level::Gate)
    }

    fn finding(path: &str, name: &str, level: Level) -> Finding {
        Finding {
            rule: "F05".to_string(),
            level,
            path: path.to_string(),
            name: name.to_string(),
            param_types: vec![],
            qualifier: String::new(),
            line: 1,
            measured_value: Some(3.0),
            threshold: Some(2.0),
            weight: 0,
            principle: String::new(),
            fix_shape: String::new(),
            anti_fix: String::new(),
            waiver: None,
        }
    }

    fn function(name: &str, start: u32, end: u32) -> Function {
        Function {
            name: name.to_string(),
            param_types: vec![],
            qualifier: String::new(),
            span: Span { start, end },
            touch: Span { start, end },
            params: vec![],
            body: Body {
                lloc: 1,
                decisions: vec![],
                cognitive: vec![],
                chains: vec![],
                forward: None,
            },
            waivers: vec![],
        }
    }

    fn source_file(path: &str, functions: Vec<Function>) -> File {
        File {
            path: path.to_string(),
            language: Language::Python,
            unparsed_ratio: 0.0,
            functions,
        }
    }

    fn names(findings: &[Finding]) -> Vec<&str> {
        findings
            .iter()
            .map(|finding| finding.name.as_str())
            .collect()
    }

    #[test]
    fn a_changed_line_gates_that_function_and_strict_gates_both() {
        let root = repo("changed-line", "main");
        write(&root, "src/a.py", PAIR);
        commit_all(&root, "base");
        let edited = PAIR.replacen("return 1", "return 2", 1);
        write(&root, "src/a.py", &edited);

        let bytes = fs::read(root.join("src/a.py")).unwrap();
        let file = parse("src/a.py", &bytes);
        let profile = Config::builtin().python;
        let findings = evaluate(&measure(&file, &[]), &profile);
        let files = [file];
        let open = ScopeOptions {
            changed: None,
            strict: false,
        };
        let scoped = partition(&root, &files, &findings, open).unwrap();

        assert_eq!(scoped.scope, Scope::Changed);
        assert!(scoped.notes.is_empty());
        assert!(scoped
            .findings
            .iter()
            .any(|finding| finding.name == "touched" && finding.level == Level::Gate));
        assert!(scoped
            .findings
            .iter()
            .all(|finding| finding.name == "touched"));
        assert!(scoped
            .inherited
            .iter()
            .any(|finding| finding.name == "other" && finding.level == Level::Gate));
        assert!(scoped
            .inherited
            .iter()
            .all(|finding| finding.name == "other"));

        let strict = partition(
            &root,
            &files,
            &findings,
            ScopeOptions {
                changed: None,
                strict: true,
            },
        )
        .unwrap();
        assert_eq!(strict.scope, Scope::Strict);
        assert!(strict.inherited.is_empty());
        assert!(strict
            .findings
            .iter()
            .any(|finding| finding.name == "touched"));
        assert!(strict
            .findings
            .iter()
            .any(|finding| finding.name == "other"));
    }

    #[test]
    fn every_finding_in_the_touched_function_moves_together() {
        let root = repo("together", "main");
        let body = "\
def touched():
    return 1

def other():
    return 0
";
        write(&root, "src/a.py", body);
        commit_all(&root, "base");
        write(&root, "src/a.py", &body.replacen("return 1", "return 2", 1));
        let file = source_file(
            "src/a.py",
            vec![function("touched", 1, 2), function("other", 4, 5)],
        );
        let findings = vec![
            gate("src/a.py", "touched"),
            finding("src/a.py", "touched", Level::Warn),
            gate("src/a.py", "other"),
        ];

        let scoped = partition(
            &root,
            &[file],
            &findings,
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();

        assert_eq!(names(&scoped.findings), vec!["touched", "touched"]);
        assert_eq!(scoped.findings[1].level, Level::Warn);
        assert_eq!(names(&scoped.inherited), vec!["other"]);
    }

    #[test]
    fn a_line_outside_every_touch_span_gates_nothing() {
        let root = repo("outside", "main");
        let body = "\
def touched():
    return 1

def other():
    return 0
";
        write(&root, "src/a.py", body);
        commit_all(&root, "base");
        write(
            &root,
            "src/a.py",
            "\
def touched():
    return 1
# moved
def other():
    return 0
",
        );
        let file = source_file(
            "src/a.py",
            vec![function("touched", 1, 2), function("other", 4, 5)],
        );
        let findings = vec![gate("src/a.py", "touched"), gate("src/a.py", "other")];

        let scoped = partition(
            &root,
            &[file],
            &findings,
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();

        assert!(scoped.findings.is_empty());
        assert_eq!(names(&scoped.inherited), vec!["touched", "other"]);
        assert_eq!(scoped.scope, Scope::Changed);
    }

    #[test]
    fn an_empty_diff_gates_nothing() {
        let root = repo("empty", "main");
        write(&root, "src/a.py", "def touched():\n    return 1\n");
        commit_all(&root, "base");
        let file = source_file("src/a.py", vec![function("touched", 1, 2)]);
        let findings = vec![gate("src/a.py", "touched")];

        let scoped = partition(
            &root,
            &[file],
            &findings,
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();

        assert!(scoped.findings.is_empty());
        assert_eq!(names(&scoped.inherited), vec!["touched"]);
    }

    #[test]
    fn a_staged_change_counts() {
        let root = repo("staged", "main");
        let body = "def touched():\n    return 1\n";
        write(&root, "src/a.py", body);
        commit_all(&root, "base");
        write(&root, "src/a.py", &body.replacen("return 1", "return 2", 1));
        git(&root, &["add", "src/a.py"]);
        let file = source_file("src/a.py", vec![function("touched", 1, 2)]);

        let scoped = partition(
            &root,
            &[file],
            &[gate("src/a.py", "touched")],
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();

        assert_eq!(names(&scoped.findings), vec!["touched"]);
    }

    #[test]
    fn a_pure_rename_gates_nothing() {
        let root = repo("rename", "main");
        write(&root, "src/a.py", "def touched():\n    return 1\n");
        commit_all(&root, "base");
        git(&root, &["mv", "src/a.py", "src/b.py"]);
        let file = source_file("src/b.py", vec![function("touched", 1, 2)]);

        let scoped = partition(
            &root,
            &[file],
            &[gate("src/b.py", "touched")],
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();

        assert!(scoped.findings.is_empty());
        assert_eq!(names(&scoped.inherited), vec!["touched"]);
    }

    #[test]
    fn a_rename_with_a_changed_line_gates_that_function() {
        let root = repo("rename-edit", "main");
        let body = "\
def touched():
    return 1

def other():
    return 0
";
        write(&root, "src/a.py", body);
        commit_all(&root, "base");
        git(&root, &["mv", "src/a.py", "src/b.py"]);
        write(&root, "src/b.py", &body.replacen("return 1", "return 2", 1));
        let file = source_file(
            "src/b.py",
            vec![function("touched", 1, 2), function("other", 4, 5)],
        );
        let findings = vec![gate("src/b.py", "touched"), gate("src/b.py", "other")];

        let scoped = partition(
            &root,
            &[file],
            &findings,
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();

        assert_eq!(names(&scoped.findings), vec!["touched"]);
        assert_eq!(names(&scoped.inherited), vec!["other"]);
    }

    #[test]
    fn a_commit_on_main_gates_nothing() {
        let root = repo("on-main", "main");
        let body = "def touched():\n    return 1\n";
        write(&root, "src/a.py", body);
        commit_all(&root, "base");
        write(&root, "src/a.py", &body.replacen("return 1", "return 2", 1));
        commit_all(&root, "edit");
        let file = source_file("src/a.py", vec![function("touched", 1, 2)]);

        let scoped = partition(
            &root,
            &[file],
            &[gate("src/a.py", "touched")],
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();

        assert!(scoped.findings.is_empty());
        assert_eq!(names(&scoped.inherited), vec!["touched"]);
    }

    #[test]
    fn a_change_in_another_file_does_not_touch_this_one() {
        let root = repo("other-file", "main");
        write(&root, "src/a.py", "def alpha():\n    return 1\n");
        write(&root, "src/b.py", "def beta():\n    return 1\n");
        commit_all(&root, "base");
        write(&root, "src/a.py", "def alpha():\n    return 2\n");
        let files = vec![
            source_file("src/a.py", vec![function("alpha", 1, 2)]),
            source_file("src/b.py", vec![function("beta", 1, 2)]),
        ];
        let findings = vec![gate("src/a.py", "alpha"), gate("src/b.py", "beta")];

        let scoped = partition(
            &root,
            &files,
            &findings,
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();

        assert_eq!(names(&scoped.findings), vec!["alpha"]);
        assert_eq!(names(&scoped.inherited), vec!["beta"]);
    }

    #[test]
    fn an_untracked_file_is_touched() {
        let root = repo("untracked", "main");
        write(&root, "src/old.py", "def other():\n    return 0\n");
        commit_all(&root, "base");
        write(&root, "src/new.py", "def touched():\n    return 1\n");
        let files = vec![
            source_file("src/old.py", vec![function("other", 1, 2)]),
            source_file("src/new.py", vec![function("touched", 1, 2)]),
        ];
        let findings = vec![gate("src/old.py", "other"), gate("src/new.py", "touched")];

        let scoped = partition(
            &root,
            &files,
            &findings,
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();

        assert_eq!(names(&scoped.findings), vec!["touched"]);
        assert_eq!(names(&scoped.inherited), vec!["other"]);
    }

    #[test]
    fn outside_git_the_whole_set_is_scored_and_the_run_says_so() {
        let root = fixture("no-git");
        let file = source_file(
            "src/a.py",
            vec![function("touched", 1, 2), function("other", 4, 5)],
        );
        let findings = vec![gate("src/a.py", "touched"), gate("src/a.py", "other")];

        let scoped = partition(
            &root,
            &[file],
            &findings,
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();

        assert_eq!(scoped.scope, Scope::Strict);
        assert!(scoped.inherited.is_empty());
        assert_eq!(names(&scoped.findings), vec!["touched", "other"]);
        assert_eq!(scoped.notes.len(), 1);
        assert_eq!(scoped.notes[0].path, None);
        assert_eq!(
            scoped.notes[0].message,
            "not a git repository; scoring the whole tree"
        );
    }

    #[test]
    fn no_main_or_master_fails_and_asks_for_a_ref_or_strict() {
        let root = repo("no-base", "other");
        write(&root, "src/a.py", "def touched():\n    return 1\n");
        commit_all(&root, "base");

        let err = partition(
            &root,
            &[source_file("src/a.py", vec![function("touched", 1, 2)])],
            &[gate("src/a.py", "touched")],
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "no base ref; pass --changed=<ref> or --strict"
        );
    }

    #[test]
    fn master_is_the_base_when_main_is_missing() {
        let root = repo("master-base", "master");
        let body = "def touched():\n    return 1\n";
        write(&root, "src/a.py", body);
        commit_all(&root, "base");
        write(&root, "src/a.py", &body.replacen("return 1", "return 2", 1));

        let scoped = partition(
            &root,
            &[source_file("src/a.py", vec![function("touched", 1, 2)])],
            &[gate("src/a.py", "touched")],
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();

        assert_eq!(names(&scoped.findings), vec!["touched"]);
        assert_eq!(scoped.scope, Scope::Changed);
    }

    #[test]
    fn a_branch_commit_counts_and_changed_head_is_only_the_work_tree() {
        let root = repo("branch", "main");
        let body = "\
def touched():
    return 1

def other():
    return 0
";
        write(&root, "src/a.py", body);
        commit_all(&root, "base");
        git(&root, &["checkout", "-b", "feature"]);
        write(&root, "src/a.py", &body.replacen("return 1", "return 2", 1));
        commit_all(&root, "edit");
        let file = source_file(
            "src/a.py",
            vec![function("touched", 1, 2), function("other", 4, 5)],
        );
        let files = [file];
        let findings = vec![gate("src/a.py", "touched"), gate("src/a.py", "other")];

        let scoped = partition(
            &root,
            &files,
            &findings,
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();
        assert_eq!(names(&scoped.findings), vec!["touched"]);
        assert_eq!(names(&scoped.inherited), vec!["other"]);

        let head = partition(
            &root,
            &files,
            &findings,
            ScopeOptions {
                changed: Some("HEAD".to_string()),
                strict: false,
            },
        )
        .unwrap();
        assert!(head.findings.is_empty());
        assert_eq!(names(&head.inherited), vec!["touched", "other"]);
    }

    #[test]
    fn a_bad_changed_ref_fails() {
        let root = repo("bad-ref", "main");
        write(&root, "src/a.py", "def touched():\n    return 1\n");
        commit_all(&root, "base");

        let err = partition(
            &root,
            &[source_file("src/a.py", vec![function("touched", 1, 2)])],
            &[gate("src/a.py", "touched")],
            ScopeOptions {
                changed: Some("nope".to_string()),
                strict: false,
            },
        )
        .unwrap_err();

        assert_eq!(
            err.to_string(),
            "changed ref nope is not a revision; pass --changed=<ref> or --strict"
        );
    }

    #[test]
    fn strict_does_not_need_a_base() {
        let root = repo("strict-no-base", "other");
        write(&root, "src/a.py", "def touched():\n    return 1\n");
        commit_all(&root, "base");
        let findings = vec![gate("src/a.py", "touched"), gate("src/a.py", "other")];

        let scoped = partition(
            &root,
            &[source_file(
                "src/a.py",
                vec![function("touched", 1, 2), function("other", 4, 5)],
            )],
            &findings,
            ScopeOptions {
                changed: Some("nope".to_string()),
                strict: true,
            },
        )
        .unwrap();

        assert_eq!(scoped.scope, Scope::Strict);
        assert!(scoped.notes.is_empty());
        assert!(scoped.inherited.is_empty());
        assert_eq!(names(&scoped.findings), vec!["touched", "other"]);
    }

    #[test]
    fn a_changed_line_in_a_path_with_a_space_still_counts() {
        let root = repo("space", "main");
        let body = "def touched():\n    return 1\n";
        write(&root, "src/odd name.py", body);
        commit_all(&root, "base");
        write(
            &root,
            "src/odd name.py",
            &body.replacen("return 1", "return 2", 1),
        );

        let scoped = partition(
            &root,
            &[source_file(
                "src/odd name.py",
                vec![function("touched", 1, 2)],
            )],
            &[gate("src/odd name.py", "touched")],
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();

        assert_eq!(names(&scoped.findings), vec!["touched"]);
    }

    fn parsed(root: &Path, rel: &str) -> File {
        parse(rel, &fs::read(root.join(rel)).unwrap())
    }

    fn delete_case(before: &str, after: &str) -> (PathBuf, File) {
        let root = repo("delete", "main");
        write(&root, "src/a.py", before);
        commit_all(&root, "base");
        write(&root, "src/a.py", after);
        let file = parsed(&root, "src/a.py");
        (root, file)
    }

    #[test]
    fn deleting_a_line_inside_the_function_touches_it() {
        let (root, file) = delete_case(
            "\
def touched():
    value = 1
    return value

def other():
    return 0
",
            "\
def touched():
    return value

def other():
    return 0
",
        );
        let findings = vec![gate("src/a.py", "touched"), gate("src/a.py", "other")];
        let scoped = partition(
            &root,
            &[file],
            &findings,
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();
        assert_eq!(names(&scoped.findings), vec!["touched"]);
        assert_eq!(names(&scoped.inherited), vec!["other"]);
    }

    #[test]
    fn deleting_the_last_line_of_a_function_touches_it() {
        let (root, file) = delete_case(
            "\
def touched():
    value = 1
    return value

def other():
    return 0
",
            "\
def touched():
    value = 1

def other():
    return 0
",
        );
        let findings = vec![gate("src/a.py", "touched"), gate("src/a.py", "other")];
        let scoped = partition(
            &root,
            &[file],
            &findings,
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();
        assert_eq!(names(&scoped.findings), vec!["touched"]);
        assert_eq!(names(&scoped.inherited), vec!["other"]);
    }

    #[test]
    fn deleting_the_blank_line_between_functions_touches_neither() {
        let (root, file) = delete_case(
            "\
def touched():
    return 1

def other():
    return 0
",
            "\
def touched():
    return 1
def other():
    return 0
",
        );
        let findings = vec![gate("src/a.py", "touched"), gate("src/a.py", "other")];
        let scoped = partition(
            &root,
            &[file],
            &findings,
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();
        assert!(scoped.findings.is_empty());
        assert_eq!(names(&scoped.inherited), vec!["touched", "other"]);
    }

    #[test]
    fn deleting_a_comment_above_a_function_does_not_touch_it() {
        let (root, file) = delete_case(
            "\
def touched():
    return 1

# note
def other():
    return 0
",
            "\
def touched():
    return 1

def other():
    return 0
",
        );
        let findings = vec![gate("src/a.py", "touched"), gate("src/a.py", "other")];
        let scoped = partition(
            &root,
            &[file],
            &findings,
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();
        assert!(scoped.findings.is_empty());
        assert_eq!(names(&scoped.inherited), vec!["touched", "other"]);
    }

    #[test]
    fn a_deletion_in_an_unparsed_language_touches_the_file() {
        let root = repo("rust-delete", "main");
        write(&root, "src/a.rs", "fn touched() {\n    let n = 1;\n}\n");
        commit_all(&root, "base");
        write(&root, "src/a.rs", "fn touched() {\n}\n");
        let mut file = source_file("src/a.rs", vec![function("touched", 1, 2)]);
        file.language = Language::Rust;

        let scoped = partition(
            &root,
            &[file],
            &[gate("src/a.rs", "touched")],
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();

        assert_eq!(names(&scoped.findings), vec!["touched"]);
    }
}
