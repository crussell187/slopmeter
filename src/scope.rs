use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
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
    ChangedOutsideGit,
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
            ScopeError::ChangedOutsideGit => write!(f, "--changed needs a git repository"),
            ScopeError::Git(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for ScopeError {}

const NOT_A_REPOSITORY: &str = "not a git repository; scoring the whole tree";

macro_rules! define_partition {
    () => {
        pub fn partition(
            root: &Path,
            files: &[File],
            findings: &[Finding],
            options: ScopeOptions,
        ) -> Result<Scoped, ScopeError> {
            apply_scope(
                root,
                ScopeJob {
                    files,
                    findings,
                    options,
                },
            )
        }
    };
}

define_partition!();

struct ScopeJob<'a> {
    files: &'a [File],
    findings: &'a [Finding],
    options: ScopeOptions,
}

fn apply_scope(root: &Path, job: ScopeJob<'_>) -> Result<Scoped, ScopeError> {
    if strict_job(&job) {
        let findings = job.findings;
        return Ok(whole(findings, Vec::new()));
    }
    if missing_repo(root, &job)? {
        return Ok(outside_note(&job));
    }
    score_diff(root, &job)
}

fn strict_job(job: &ScopeJob<'_>) -> bool {
    let options = &job.options;
    options.strict
}

fn missing_repo(root: &Path, job: &ScopeJob<'_>) -> Result<bool, ScopeError> {
    if work_tree(root)? {
        return Ok(false);
    }
    require_repo(job)?;
    Ok(true)
}

fn require_repo(job: &ScopeJob<'_>) -> Result<(), ScopeError> {
    let options = &job.options;
    let changed = &options.changed;
    if changed.is_some() {
        return Err(ScopeError::ChangedOutsideGit);
    }
    Ok(())
}

fn outside_note(job: &ScopeJob<'_>) -> Scoped {
    let findings = job.findings;
    let message = NOT_A_REPOSITORY.to_string();
    whole(
        findings,
        vec![Note {
            path: None,
            message,
        }],
    )
}

fn score_diff(root: &Path, job: &ScopeJob<'_>) -> Result<Scoped, ScopeError> {
    let changed = changed_ref(job);
    let change = load_change(root, changed)?;
    let files = job.files;
    let touched = touched_symbols(root, files, &change);
    let findings = job.findings;
    Ok(bucket_findings(findings, &touched))
}

fn changed_ref<'a, 'b>(job: &'a ScopeJob<'b>) -> Option<&'a str> {
    let options = &job.options;
    let changed = &options.changed;
    changed.as_deref()
}

struct Buckets {
    kept: Vec<Finding>,
    inherited: Vec<Finding>,
}

fn bucket_findings(findings: &[Finding], touched: &BTreeSet<Symbol>) -> Scoped {
    let mut buckets = Buckets {
        kept: Vec::new(),
        inherited: Vec::new(),
    };
    for finding in findings {
        place_changed(finding, touched, &mut buckets);
    }
    Scoped {
        scope: Scope::Changed,
        findings: buckets.kept,
        inherited: buckets.inherited,
        notes: Vec::new(),
    }
}

fn place_changed(finding: &Finding, touched: &BTreeSet<Symbol>, buckets: &mut Buckets) {
    if diff_only(finding) && unmeasured(finding) {
        keep_touched(finding, touched, buckets);
        return;
    }
    if symbol_touched(touched, finding) {
        push_kept(buckets, finding);
        return;
    }
    push_inherited(buckets, finding);
}

fn unmeasured(finding: &Finding) -> bool {
    let measured = finding.measured_value;
    measured.is_none()
}

fn symbol_touched(touched: &BTreeSet<Symbol>, finding: &Finding) -> bool {
    let symbol = symbol(finding);
    touched.contains(&symbol)
}

fn keep_touched(finding: &Finding, touched: &BTreeSet<Symbol>, buckets: &mut Buckets) {
    if symbol_touched(touched, finding) {
        push_kept(buckets, finding);
    }
}

fn push_kept(buckets: &mut Buckets, finding: &Finding) {
    let kept = &mut buckets.kept;
    kept.push(finding.clone());
}

fn push_inherited(buckets: &mut Buckets, finding: &Finding) {
    let inherited = &mut buckets.inherited;
    inherited.push(finding.clone());
}

fn whole(findings: &[Finding], notes: Vec<Note>) -> Scoped {
    let mut kept = Vec::new();
    let mut inherited = Vec::new();
    for finding in findings {
        place_strict(finding, &mut kept, &mut inherited);
    }
    Scoped {
        scope: Scope::Strict,
        findings: kept,
        inherited,
        notes,
    }
}

fn place_strict(finding: &Finding, kept: &mut Vec<Finding>, inherited: &mut Vec<Finding>) {
    if !diff_only(finding) {
        kept.push(finding.clone());
        return;
    }
    keep_measured(finding, inherited);
}

fn keep_measured(finding: &Finding, inherited: &mut Vec<Finding>) {
    let measured = finding.measured_value;
    if measured.is_some() {
        inherited.push(finding.clone());
    }
}

fn diff_only(finding: &Finding) -> bool {
    matches!(finding.rule.as_str(), "S06" | "S07") && finding.weight == 10
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

struct Touch<'a> {
    root: &'a Path,
    change: &'a Change,
    touched: &'a mut BTreeSet<Symbol>,
}

fn touched_symbols(root: &Path, files: &[File], change: &Change) -> BTreeSet<Symbol> {
    let mut touched = BTreeSet::new();
    {
        let mut state = Touch {
            root,
            change,
            touched: &mut touched,
        };
        for file in files {
            touch_file(file, &mut state);
        }
    }
    touched
}

fn touch_file(file: &File, state: &mut Touch<'_>) {
    let change = state.change;
    let listed = &change.files;
    let path = file_path(file);
    let delta = listed.get(path);
    let old = delta.map(|delta| old_spans(state, file, delta));
    let query = HitQuery {
        change,
        file,
        delta,
        old: &old,
    };
    for function in file_functions(file) {
        if function_hit(&query, function) {
            remember(state, file, function);
        }
    }
}

fn file_path(file: &File) -> &str {
    let path = &file.path;
    path.as_str()
}

fn file_functions(file: &File) -> &[Function] {
    &file.functions
}

struct HitQuery<'a> {
    change: &'a Change,
    file: &'a File,
    delta: Option<&'a Delta>,
    old: &'a Option<OldSpans>,
}

fn function_hit(query: &HitQuery<'_>, function: &Function) -> bool {
    if tracked_new(query) {
        return true;
    }
    let Some(delta) = query.delta else {
        return false;
    };
    added_touches(delta, function) || removed_touches(query, delta, function)
}

fn tracked_new(query: &HitQuery<'_>) -> bool {
    let change = query.change;
    let untracked = &change.untracked;
    let file = query.file;
    untracked.contains(file_path(file))
}

fn added_touches(delta: &Delta, function: &Function) -> bool {
    let added = &delta.added;
    let touch = function.touch;
    added.iter().any(|line| contains(*line, touch))
}

fn removed_touches(query: &HitQuery<'_>, delta: &Delta, function: &Function) -> bool {
    let old = query.old;
    let spans = old.as_ref();
    spans.is_some_and(|spans| deleted_hit(spans, delta, function))
}

fn remember(state: &mut Touch<'_>, file: &File, function: &Function) {
    let symbol = symbol_of(file_path(file), function);
    let touched = &mut state.touched;
    touched.insert(symbol);
}

fn deleted_hit(old: &OldSpans, delta: &Delta, function: &Function) -> bool {
    match old {
        OldSpans::None => false,
        OldSpans::Whole => deletions_present(delta),
        OldSpans::Precise(functions) => precise_deleted(functions, delta, function),
    }
}

fn deletions_present(delta: &Delta) -> bool {
    let deleted = &delta.deleted;
    !deleted.is_empty()
}

fn precise_deleted(functions: &[Function], delta: &Delta, function: &Function) -> bool {
    functions
        .iter()
        .any(|before| same_function(before, function) && deleted_line(delta, before.touch))
}

fn same_function(before: &Function, function: &Function) -> bool {
    before.name == function.name
        && before.param_types == function.param_types
        && before.qualifier == function.qualifier
}

fn deleted_line(delta: &Delta, touch: Span) -> bool {
    let deleted = &delta.deleted;
    deleted.iter().any(|line| contains(*line, touch))
}

struct OldQuery<'a> {
    root: &'a Path,
    base: &'a str,
    language: Language,
}

fn old_spans(state: &Touch<'_>, file: &File, delta: &Delta) -> OldSpans {
    if no_deletions(delta) {
        return OldSpans::None;
    }
    let Some(old_path) = previous_path(delta) else {
        return OldSpans::Whole;
    };
    loaded_spans(state, file, old_path)
}

fn no_deletions(delta: &Delta) -> bool {
    let deleted = &delta.deleted;
    deleted.is_empty()
}

fn previous_path(delta: &Delta) -> Option<&str> {
    let old_path = &delta.old_path;
    old_path.as_deref()
}

fn loaded_spans(state: &Touch<'_>, file: &File, old_path: &str) -> OldSpans {
    let query = OldQuery {
        root: state.root,
        base: change_base(state),
        language: file_language(file),
    };
    match load_old(&query, old_path) {
        Some(functions) => OldSpans::Precise(functions),
        None => OldSpans::Whole,
    }
}

fn change_base<'a>(state: &'a Touch<'_>) -> &'a str {
    let change = state.change;
    let base = &change.base;
    base.as_str()
}

fn file_language(file: &File) -> Language {
    file.language
}

fn load_old(query: &OldQuery<'_>, old_path: &str) -> Option<Vec<Function>> {
    let root = query.root;
    let prefix = work_tree_prefix(root).ok()?;
    let base = query.base;
    let spec = format!("{base}:{prefix}{old_path}");
    let out = git(root, &["--no-pager", "show", &spec]).ok()?;
    if !succeeded(&out) {
        return None;
    }
    parse_old(query, old_path, &out)
}

fn parse_old(query: &OldQuery<'_>, old_path: &str, out: &Output) -> Option<Vec<Function>> {
    let source = &out.stdout;
    let language = query.language;
    let parsed = crate::adapters::parse(
        language,
        crate::adapters::ParseInput {
            path: old_path,
            source,
            include_tests: false,
        },
    );
    parsed.ok().map(owned_functions)
}

fn owned_functions(file: File) -> Vec<Function> {
    file.functions
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
    for name in ["main", "master", "origin/main", "origin/master"] {
        if !rev_exists(root, name)? {
            continue;
        }
        return merge_base(root, name);
    }
    Err(ScopeError::NoBase)
}

fn merge_base(root: &Path, name: &str) -> Result<String, ScopeError> {
    let out = git(root, &["merge-base", "HEAD", name])?;
    if !succeeded(&out) {
        return Err(ScopeError::NoBase);
    }
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if sha.is_empty() {
        Err(ScopeError::NoBase)
    } else {
        Ok(sha)
    }
}

pub fn repo_root(start: &Path) -> Result<Option<PathBuf>, ScopeError> {
    if !start.is_dir() {
        return Ok(None);
    }
    let out = git(start, &["rev-parse", "--show-toplevel"])?;
    if !succeeded(&out) {
        return Ok(None);
    }
    let printed = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if printed.is_empty() {
        return Ok(None);
    }
    let path = PathBuf::from(&printed);
    Ok(Some(path.canonicalize().unwrap_or(path)))
}

fn work_tree(root: &Path) -> Result<bool, ScopeError> {
    Ok(repo_root(root)?.is_some())
}

fn work_tree_prefix(root: &Path) -> Result<String, ScopeError> {
    let out = git(root, &["rev-parse", "--show-prefix"])?;
    if !succeeded(&out) {
        return Err(ScopeError::Git(command_error(&out)));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn relative_to_prefix(prefix: &str, path: &str) -> Option<String> {
    if prefix.is_empty() {
        return Some(path.to_string());
    }
    path.strip_prefix(prefix).map(str::to_string)
}

fn rev_exists(root: &Path, rev: &str) -> Result<bool, ScopeError> {
    let spec = format!("{rev}^{{commit}}");
    let out = git(root, &["rev-parse", "--verify", "--quiet", &spec])?;
    Ok(succeeded(&out))
}

fn diff_files(root: &Path, base: &str) -> Result<BTreeMap<String, Delta>, ScopeError> {
    let out = git(
        root,
        &[
            "--no-pager",
            "-c",
            "core.quotePath=false",
            "-c",
            "diff.noprefix=false",
            "-c",
            "diff.mnemonicPrefix=false",
            "diff",
            "--src-prefix=a/",
            "--dst-prefix=b/",
            "--no-color",
            "--no-ext-diff",
            "--unified=0",
            "--find-renames",
            "--relative",
            base,
        ],
    )?;
    if !succeeded(&out) {
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
    if !succeeded(&out) {
        return Err(ScopeError::Git(command_error(&out)));
    }
    let prefix = work_tree_prefix(root)?;
    Ok(parse_untracked(&out.stdout)
        .into_iter()
        .filter_map(|path| relative_to_prefix(&prefix, &path))
        .collect())
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
        read_diff_line(line, &mut open, &mut files);
    }
    flush(&mut open, &mut files);
    files
}

fn read_diff_line(line: &str, open: &mut Open, files: &mut BTreeMap<String, Delta>) {
    if diff_header(line) {
        flush(open, files);
        return;
    }
    if apply_side(line, open) {
        return;
    }
    apply_hunk(line, open);
}

fn diff_header(line: &str) -> bool {
    line.starts_with("diff --git ")
}

fn apply_side(line: &str, open: &mut Open) -> bool {
    if let Some(side) = parse_side(line, "--- ") {
        open.old_path = path_of(side);
        return true;
    }
    if let Some(side) = parse_side(line, "+++ ") {
        open.new_path = path_of(side);
        return true;
    }
    false
}

fn apply_hunk(line: &str, open: &mut Open) {
    let Some((old_start, old_count, new_start, new_count)) = parse_hunk(line) else {
        return;
    };
    let deleted = &mut open.deleted;
    insert_lines(deleted, old_start, old_count);
    let added = &mut open.added;
    insert_lines(added, new_start, new_count);
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
    let mut finished = std::mem::take(open);
    let Some(path) = take_path(&mut finished) else {
        return;
    };
    if empty_change(&finished) {
        return;
    }
    store_delta(files, path, finished);
}

fn take_path(finished: &mut Open) -> Option<String> {
    let path = &mut finished.new_path;
    path.take()
}

fn empty_change(finished: &Open) -> bool {
    let added = &finished.added;
    let deleted = &finished.deleted;
    added.is_empty() && deleted.is_empty()
}

fn store_delta(files: &mut BTreeMap<String, Delta>, path: String, finished: Open) {
    let slot = files.entry(path);
    let updated = slot.and_modify(|delta| merge_delta(delta, &finished));
    updated.or_insert(new_delta(finished));
}

fn merge_delta(delta: &mut Delta, finished: &Open) {
    extend_lines(&mut delta.added, &finished.added);
    extend_lines(&mut delta.deleted, &finished.deleted);
    if missing_old(delta) {
        copy_old(delta, finished);
    }
}

fn extend_lines(lines: &mut BTreeSet<u32>, extra: &BTreeSet<u32>) {
    lines.extend(extra);
}

fn missing_old(delta: &Delta) -> bool {
    let old_path = &delta.old_path;
    old_path.is_none()
}

fn copy_old(delta: &mut Delta, finished: &Open) {
    let old_path = &mut delta.old_path;
    let source = &finished.old_path;
    old_path.clone_from(source);
}

fn new_delta(finished: Open) -> Delta {
    Delta {
        old_path: finished.old_path,
        added: finished.added,
        deleted: finished.deleted,
    }
}

fn parse_untracked(bytes: &[u8]) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    let mut rest = bytes;
    while !rest.is_empty() {
        let Some(next) = read_untracked_entry(rest, &mut paths) else {
            break;
        };
        rest = next;
    }
    paths
}

fn read_untracked_entry<'a>(rest: &'a [u8], paths: &mut BTreeSet<String>) -> Option<&'a [u8]> {
    let Some(end) = null_at(rest) else {
        return None;
    };
    let entry = &rest[..end];
    let mut next = &rest[end + 1..];
    next = skip_rename(entry, next);
    keep_untracked(entry, paths);
    Some(next)
}

fn null_at(rest: &[u8]) -> Option<usize> {
    rest.iter().position(|byte| *byte == 0)
}

fn skip_rename<'a>(entry: &[u8], rest: &'a [u8]) -> &'a [u8] {
    if !renamed(entry) {
        return rest;
    }
    let Some(extra) = null_at(rest) else {
        return rest;
    };
    &rest[extra + 1..]
}

fn renamed(entry: &[u8]) -> bool {
    let first = entry.first();
    first.is_some_and(is_rename)
}

fn is_rename(byte: &u8) -> bool {
    *byte == b'R' || *byte == b'C'
}

fn keep_untracked(entry: &[u8], paths: &mut BTreeSet<String>) {
    let Some(path) = entry.strip_prefix(b"?? ") else {
        return;
    };
    let Ok(path) = std::str::from_utf8(path) else {
        return;
    };
    paths.insert(path.to_string());
}

enum Side {
    Null,
    Path(String),
}

fn parse_side(line: &str, prefix: &str) -> Option<Side> {
    let Some(stripped) = line.strip_prefix(prefix) else {
        return None;
    };
    let rest = stripped.trim_end();
    let raw = unquote(rest)?;
    if raw == "/dev/null" {
        return Some(Side::Null);
    }
    let marker = side_marker(prefix);
    let Some(path) = raw.strip_prefix(marker) else {
        return None;
    };
    Some(Side::Path(path.to_string()))
}

fn side_marker(prefix: &str) -> &'static str {
    if prefix.starts_with("+++") {
        "b/"
    } else {
        "a/"
    }
}

fn unquote(raw: &str) -> Option<String> {
    if !raw.starts_with('"') {
        return Some(raw.to_string());
    }
    if short_quote(raw) {
        return None;
    }
    let mut out = String::new();
    let mut chars = quote_chars(raw);
    while let Some(ch) = chars.next() {
        push_unquoted(&mut out, ch, &mut chars)?;
    }
    Some(out)
}

fn short_quote(raw: &str) -> bool {
    let len = raw.len();
    len < 2 || !raw.ends_with('"')
}

fn quote_chars(raw: &str) -> std::str::Chars<'_> {
    let len = raw.len();
    let inner = &raw[1..len - 1];
    inner.chars()
}

fn push_unquoted(out: &mut String, ch: char, chars: &mut std::str::Chars<'_>) -> Option<()> {
    if ch != '\\' {
        out.push(ch);
        return Some(());
    }
    let next = chars.next()?;
    push_escape(out, next)
}

fn push_escape(out: &mut String, ch: char) -> Option<()> {
    let decoded = match ch {
        'n' => '\n',
        't' => '\t',
        '"' => '"',
        '\\' => '\\',
        other => other,
    };
    out.push(decoded);
    Some(())
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
    let mut command = Command::new("git");
    command.args(args);
    command.current_dir(root);
    command.env("GIT_CONFIG_NOSYSTEM", "1");
    command.env("GIT_CONFIG_GLOBAL", "/dev/null");
    command.env("GIT_CONFIG_SYSTEM", "/dev/null");
    command.env_remove("GIT_DIR");
    command.env_remove("GIT_WORK_TREE");
    command.env_remove("GIT_INDEX_FILE");
    command.env_remove("GIT_CEILING_DIRECTORIES");
    let result = command.output();
    result.map_err(|err| ScopeError::Git(err.to_string()))
}

fn succeeded(out: &Output) -> bool {
    let status = out.status;
    status.success()
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

    fn outside_repo(name: &str) -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "slopmeter-scope-{name}-{}-{}",
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
                levels: vec![],
                cognitive: vec![],
                chains: vec![],
                swallowing: vec![],
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
            unscored_macros: 0,
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
    fn coverage_rows_follow_the_change_and_strict_keeps_shown_rows_inherited() {
        let root = repo("coverage-scope", "main");
        write(&root, "src/a.py", PAIR);
        commit_all(&root, "base");
        let edited = PAIR.replacen("return 1", "return 2", 1);
        write(&root, "src/a.py", &edited);
        let bytes = fs::read(root.join("src/a.py")).unwrap();
        let file = parse("src/a.py", &bytes);
        let files = [file];
        let mut rejection = finding("src/a.py", "other", Level::Gate);
        rejection.rule = "S06".to_string();
        rejection.measured_value = None;
        rejection.threshold = None;
        let findings = [
            s06("src/a.py", "touched", Some(0.0)),
            s06("src/a.py", "other", Some(0.5)),
            s06("src/a.py", "touched", None),
            s06("src/a.py", "other", None),
            rejection,
        ];
        let open = ScopeOptions {
            changed: None,
            strict: false,
        };

        let scoped = partition(&root, &files, &findings, open).unwrap();

        assert_eq!(
            placed(&scoped.findings),
            vec![("touched", Some(0.0), 10), ("touched", None, 10)]
        );
        assert_eq!(
            placed(&scoped.inherited),
            vec![("other", Some(0.5), 10), ("other", None, 0)]
        );

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
        assert_eq!(placed(&strict.findings), vec![("other", None, 0)]);
        assert_eq!(
            placed(&strict.inherited),
            vec![("touched", Some(0.0), 10), ("other", Some(0.5), 10)]
        );
    }

    #[test]
    fn mutation_rows_follow_the_change_and_strict_keeps_shown_rows_inherited() {
        let root = repo("mutation-scope", "main");
        write(&root, "src/a.py", PAIR);
        commit_all(&root, "base");
        write(&root, "src/a.py", &PAIR.replacen("return 1", "return 2", 1));
        let bytes = fs::read(root.join("src/a.py")).unwrap();
        let file = parse("src/a.py", &bytes);
        let files = [file];
        let mut rejection = finding("src/a.py", "other", Level::Gate);
        rejection.rule = "S07".to_string();
        rejection.measured_value = None;
        rejection.threshold = None;
        let findings = [
            s07("src/a.py", "touched", Some(0.0)),
            s07("src/a.py", "other", Some(0.5)),
            s07("src/a.py", "touched", None),
            s07("src/a.py", "other", None),
            rejection,
        ];

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

        assert_eq!(
            placed(&scoped.findings),
            vec![("touched", Some(0.0), 10), ("touched", None, 10)]
        );
        assert_eq!(
            placed(&scoped.inherited),
            vec![("other", Some(0.5), 10), ("other", None, 0)]
        );

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
        assert_eq!(placed(&strict.findings), vec![("other", None, 0)]);
        assert_eq!(
            placed(&strict.inherited),
            vec![("touched", Some(0.0), 10), ("other", Some(0.5), 10)]
        );
    }

    fn s07(path: &str, name: &str, measured: Option<f64>) -> Finding {
        let mut finding = s06(path, name, measured);
        finding.rule = "S07".to_string();
        finding
    }

    fn s06(path: &str, name: &str, measured: Option<f64>) -> Finding {
        let mut finding = finding(path, name, Level::Gate);
        finding.rule = "S06".to_string();
        finding.weight = 10;
        finding.measured_value = measured;
        finding.threshold = Some(1.0);
        finding
    }

    fn placed(findings: &[Finding]) -> Vec<(&str, Option<f64>, u32)> {
        findings
            .iter()
            .map(|finding| {
                (
                    finding.name.as_str(),
                    finding.measured_value,
                    finding.weight,
                )
            })
            .collect()
    }

    #[test]
    fn noprefix_still_touches_the_edited_function() {
        let root = repo("noprefix", "main");
        write(&root, "src/a.py", PAIR);
        commit_all(&root, "base");
        git(&root, &["config", "diff.noprefix", "true"]);
        write(&root, "src/a.py", &PAIR.replacen("return 1", "return 2", 1));
        let bytes = fs::read(root.join("src/a.py")).unwrap();
        let file = parse("src/a.py", &bytes);
        let findings = evaluate(&measure(&file, &[]), &Config::builtin().python);

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

        assert_eq!(scoped.scope, Scope::Changed);
        assert!(scoped
            .findings
            .iter()
            .any(|finding| finding.name == "touched"));
        assert!(scoped
            .findings
            .iter()
            .all(|finding| finding.name == "touched"));
        assert!(scoped
            .inherited
            .iter()
            .all(|finding| finding.name == "other"));
    }

    #[test]
    fn mnemonic_prefix_still_touches_the_edited_function() {
        let root = repo("mnemonic", "main");
        write(&root, "src/a.py", PAIR);
        commit_all(&root, "base");
        git(&root, &["config", "diff.mnemonicPrefix", "true"]);
        write(&root, "src/a.py", &PAIR.replacen("return 1", "return 2", 1));
        let bytes = fs::read(root.join("src/a.py")).unwrap();
        let file = parse("src/a.py", &bytes);
        let findings = evaluate(&measure(&file, &[]), &Config::builtin().python);

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

        assert!(scoped
            .findings
            .iter()
            .any(|finding| finding.name == "touched"));
        assert!(scoped
            .inherited
            .iter()
            .all(|finding| finding.name == "other"));
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
    fn a_subdirectory_diff_touches_only_the_edited_function() {
        let root = repo("subdir-diff", "main");
        write(&root, "nested/src/a.py", PAIR);
        write(&root, "src/b.py", "def outside():\n    return 1\n");
        commit_all(&root, "base");
        write(
            &root,
            "nested/src/a.py",
            &PAIR.replacen("return 1", "return 2", 1),
        );
        write(&root, "src/b.py", "def outside():\n    return 2\n");
        let nested = root.join("nested");
        let bytes = fs::read(nested.join("src/a.py")).unwrap();
        let file = parse("src/a.py", &bytes);
        let findings = evaluate(&measure(&file, &[]), &Config::builtin().python);

        let scoped = partition(
            &nested,
            &[file],
            &findings,
            ScopeOptions {
                changed: None,
                strict: false,
            },
        )
        .unwrap();

        assert_eq!(scoped.scope, Scope::Changed);
        assert!(scoped.notes.is_empty());
        assert!(scoped
            .findings
            .iter()
            .any(|finding| finding.name == "touched"));
        assert!(scoped
            .findings
            .iter()
            .all(|finding| finding.name == "touched"));
        assert!(scoped
            .inherited
            .iter()
            .all(|finding| finding.name == "other"));
    }

    #[test]
    fn a_subdirectory_deletion_touches_only_that_function() {
        let root = repo("subdir-delete", "main");
        let before = "\
def touched():
    value = 1
    return value

def other():
    return 0
";
        let after = "\
def touched():
    return value

def other():
    return 0
";
        write(&root, "nested/src/a.py", before);
        commit_all(&root, "base");
        write(&root, "nested/src/a.py", after);
        let nested = root.join("nested");
        let file = parse("src/a.py", &fs::read(nested.join("src/a.py")).unwrap());
        let findings = vec![gate("src/a.py", "touched"), gate("src/a.py", "other")];

        let scoped = partition(
            &nested,
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
    fn an_untracked_file_in_a_subdirectory_is_touched() {
        let root = repo("subdir-untracked", "main");
        write(&root, "nested/src/old.py", "def other():\n    return 0\n");
        commit_all(&root, "base");
        write(&root, "nested/src/new.py", "def touched():\n    return 1\n");
        let nested = root.join("nested");
        let files = vec![
            source_file("src/old.py", vec![function("other", 1, 2)]),
            source_file("src/new.py", vec![function("touched", 1, 2)]),
        ];
        let findings = vec![gate("src/old.py", "other"), gate("src/new.py", "touched")];

        let scoped = partition(
            &nested,
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
        let root = outside_repo("no-git");
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
    fn changed_outside_git_is_an_error() {
        let root = outside_repo("changed-outside");
        let err = partition(
            &root,
            &[source_file("src/a.py", vec![function("touched", 1, 2)])],
            &[gate("src/a.py", "touched")],
            ScopeOptions {
                changed: Some("HEAD".to_string()),
                strict: false,
            },
        )
        .unwrap_err();

        assert_eq!(err.to_string(), "--changed needs a git repository");
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
    fn origin_main_is_the_base_when_no_local_branch_exists() {
        let root = repo("origin-main", "feature");
        let body = "def touched():\n    return 1\n";
        write(&root, "src/a.py", body);
        commit_all(&root, "base");
        git(&root, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
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
    }

    #[test]
    fn origin_master_is_the_base_when_origin_main_is_missing() {
        let root = repo("origin-master", "feature");
        let body = "def touched():\n    return 1\n";
        write(&root, "src/a.py", body);
        commit_all(&root, "base");
        git(&root, &["update-ref", "refs/remotes/origin/master", "HEAD"]);
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
    }

    #[test]
    fn local_main_wins_over_origin_main() {
        let root = repo("local-wins", "main");
        let body = "def touched():\n    return 1\n";
        write(&root, "src/a.py", body);
        commit_all(&root, "base");
        git(&root, &["checkout", "-b", "feature"]);
        write(&root, "src/a.py", &body.replacen("return 1", "return 2", 1));
        commit_all(&root, "edit");
        git(&root, &["update-ref", "refs/remotes/origin/main", "HEAD"]);

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

    fn delete_lang(
        name: &str,
        rel: &str,
        before: &str,
        after: &str,
        language: Language,
    ) -> (PathBuf, File) {
        let root = repo(name, "main");
        write(&root, rel, before);
        commit_all(&root, "base");
        write(&root, rel, after);
        let source = fs::read(root.join(rel)).unwrap();
        let file = crate::adapters::parse(
            language,
            crate::adapters::ParseInput {
                path: rel,
                source: &source,
                include_tests: false,
            },
        )
        .unwrap();
        (root, file)
    }

    fn assert_only_touched(root: &Path, file: File) {
        let findings: Vec<Finding> = file
            .functions
            .iter()
            .map(|function| {
                let mut finding = gate(&file.path, &function.name);
                finding.param_types.clone_from(&function.param_types);
                finding.qualifier.clone_from(&function.qualifier);
                finding
            })
            .collect();
        let scoped = partition(
            root,
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
    fn deleting_a_line_in_one_rust_function_touches_only_that_function() {
        let (root, file) = delete_lang(
            "rust-delete",
            "src/a.rs",
            "\
fn touched() {
    let n = 1;
    let _ = n;
}

fn other() {
    let _ = 0;
}
",
            "\
fn touched() {
    let _ = n;
}

fn other() {
    let _ = 0;
}
",
            Language::Rust,
        );
        assert_only_touched(&root, file);
    }

    #[test]
    fn deleting_a_line_in_one_cpp_function_touches_only_that_function() {
        let (root, file) = delete_lang(
            "cpp-delete",
            "src/a.cpp",
            "\
int touched() {
    int n = 1;
    return n;
}

int other() {
    return 0;
}
",
            "\
int touched() {
    return n;
}

int other() {
    return 0;
}
",
            Language::Cpp,
        );
        assert_only_touched(&root, file);
    }
}
