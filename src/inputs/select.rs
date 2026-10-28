use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::ir::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectOptions {
    pub tests: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selected {
    pub path: String,
    pub language: Language,
}

#[derive(Debug)]
pub enum SelectError {
    NotADirectory(PathBuf),
    Walk(ignore::Error),
}

impl std::fmt::Display for SelectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SelectError::NotADirectory(path) => {
                write!(f, "{} is not a directory", path.display())
            }
            SelectError::Walk(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for SelectError {}

pub fn select(root: &Path, options: SelectOptions) -> Result<Vec<Selected>, SelectError> {
    let meta = fs::metadata(root).map_err(|_| SelectError::NotADirectory(root.to_path_buf()))?;
    if !meta.is_dir() {
        return Err(SelectError::NotADirectory(root.to_path_buf()));
    }

    let in_git = root.join(".git").exists();
    let walker = WalkBuilder::new(root)
        .standard_filters(false)
        .hidden(false)
        .parents(false)
        .follow_links(false)
        .git_ignore(in_git)
        .git_global(in_git)
        .git_exclude(in_git)
        .filter_entry(|entry| !is_denied_directory(entry))
        .build();

    let mut selected = Vec::new();
    for entry in walker {
        let entry = entry.map_err(SelectError::Walk)?;
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let Some(rel) = relative_path(root, entry.path()) else {
            continue;
        };
        let Some(language) = language_of(&rel) else {
            continue;
        };
        if is_protobuf(&rel) || (!options.tests && is_test_file(&rel)) {
            continue;
        }
        selected.push(Selected {
            path: rel,
            language,
        });
    }
    selected.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(selected)
}

fn is_denied_directory(entry: &ignore::DirEntry) -> bool {
    if entry.depth() == 0 {
        return false;
    }
    entry.file_type().is_some_and(|kind| kind.is_dir()) && is_denied_name(entry.file_name())
}

fn is_denied_name(name: &std::ffi::OsStr) -> bool {
    matches!(
        name.to_str(),
        Some(
            "target"
                | ".venv"
                | "venv"
                | "__pycache__"
                | "build"
                | "dist"
                | "vendor"
                | "third_party"
                | "third-party"
                | "node_modules"
                | ".git"
        )
    )
}

fn relative_path(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for component in rel.components() {
        match component {
            std::path::Component::Normal(part) => parts.push(part.to_str()?),
            _ => return None,
        }
    }
    Some(parts.join("/"))
}

fn file_name(rel: &str) -> &str {
    rel.rsplit('/').next().unwrap_or(rel)
}

fn language_of(rel: &str) -> Option<Language> {
    let ext = file_name(rel).rsplit_once('.')?.1;
    match ext {
        "py" => Some(Language::Python),
        "rs" => Some(Language::Rust),
        "cpp" | "cc" | "cxx" | "c++" | "h" | "hh" | "hpp" | "hxx" | "inl" | "ipp" | "cppm"
        | "ixx" => Some(Language::Cpp),
        _ => None,
    }
}

fn is_protobuf(rel: &str) -> bool {
    let name = file_name(rel);
    name.ends_with("_pb2.py")
        || name.ends_with("_pb2_grpc.py")
        || name.ends_with(".pb.cc")
        || name.ends_with(".pb.cpp")
        || name.ends_with(".pb.h")
        || name.ends_with(".pb.hpp")
}

fn is_test_file(rel: &str) -> bool {
    let mut segments = rel.split('/');
    let name = segments.next_back().unwrap_or(rel);
    if segments.any(|segment| matches!(segment, "test" | "tests" | "fuzz" | "testing")) {
        return true;
    }
    if name == "conftest.py"
        || (name.starts_with("test_") && name.ends_with(".py"))
        || name.ends_with("_test.py")
    {
        return true;
    }
    let Some((stem, ext)) = name.rsplit_once('.') else {
        return false;
    };
    matches!(
        ext,
        "cpp" | "cc" | "cxx" | "c++" | "h" | "hh" | "hpp" | "hxx" | "inl" | "ipp" | "cppm" | "ixx"
    ) && (stem.ends_with("_test") || stem.ends_with("_tests") || stem.ends_with("_unittest"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Language;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    fn fixture(name: &str) -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("select-fixtures")
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

    fn paths(root: &Path, tests: bool) -> Vec<String> {
        select(root, SelectOptions { tests })
            .unwrap()
            .into_iter()
            .map(|s| s.path)
            .collect()
    }

    #[test]
    fn the_default_set_keeps_source_and_build_rs() {
        let root = fixture("default");
        write(&root, "src/a.py", "x = 1\n");
        write(&root, "tests/test_a.py", "x = 1\n");
        write(&root, "target/b.rs", "fn b() {}\n");
        write(&root, "foo.pb.cc", "void foo() {}\n");
        write(&root, "build.rs", "fn main() {}\n");

        let selected = select(&root, SelectOptions { tests: false }).unwrap();
        assert_eq!(
            selected.iter().map(|s| s.path.as_str()).collect::<Vec<_>>(),
            vec!["build.rs", "src/a.py"]
        );
        assert_eq!(selected[0].language, Language::Rust);
        assert_eq!(selected[1].language, Language::Python);
    }

    #[test]
    fn the_tests_switch_brings_test_files_back() {
        let root = fixture("tests-switch");
        write(&root, "src/a.py", "x = 1\n");
        write(&root, "tests/test_a.py", "x = 1\n");
        write(&root, "target/b.rs", "fn b() {}\n");
        write(&root, "foo.pb.cc", "void foo() {}\n");
        write(&root, "build.rs", "fn main() {}\n");

        assert_eq!(
            paths(&root, true),
            vec![
                "build.rs".to_string(),
                "src/a.py".to_string(),
                "tests/test_a.py".to_string(),
            ]
        );
    }

    #[test]
    fn a_git_repo_drops_gitignored_files() {
        let root = fixture("git-ignore");
        write(&root, "src/a.py", "x = 1\n");
        write(&root, "secret.py", "x = 1\n");
        fs::write(root.join(".gitignore"), "secret.py\n").unwrap();
        assert!(std::process::Command::new("git")
            .arg("-C")
            .arg(&root)
            .arg("init")
            .status()
            .unwrap()
            .success());

        assert_eq!(paths(&root, false), vec!["src/a.py".to_string()]);
    }

    #[test]
    fn outside_git_a_gitignore_file_does_not_apply() {
        let root = fixture("no-git");
        write(&root, "src/a.py", "x = 1\n");
        write(&root, "secret.py", "x = 1\n");
        fs::write(root.join(".gitignore"), "secret.py\n").unwrap();

        assert_eq!(
            paths(&root, false),
            vec!["secret.py".to_string(), "src/a.py".to_string()]
        );
    }

    #[test]
    fn denylist_dirs_protobuf_and_unclaimed_extensions_drop() {
        let root = fixture("names");
        write(&root, "build.rs", "fn main() {}\n");
        write(&root, "build/lib.rs", "fn lib() {}\n");
        write(&root, "vendor/a.py", "x = 1\n");
        write(&root, "node_modules/a.py", "x = 1\n");
        write(&root, "examples/demo.py", "x = 1\n");
        write(&root, "benches/bench.rs", "fn bench() {}\n");
        write(&root, "src/testing/a.py", "x = 1\n");
        write(&root, "src/test/nested/a.py", "x = 1\n");
        write(&root, "src/WidgetTest.cpp", "void w() {}\n");
        write(&root, "tests/WidgetTest.cpp", "void w() {}\n");
        write(&root, "src/foo_test.cpp", "void f() {}\n");
        write(&root, "src/foo_tests.hpp", "void f() {}\n");
        write(&root, "src/foo_unittest.cc", "void f() {}\n");
        write(&root, "src/foo_test.rs", "fn f() {}\n");
        write(&root, "conftest.py", "x = 1\n");
        write(&root, "pkg/test_util.py", "x = 1\n");
        write(&root, "pkg/util_test.py", "x = 1\n");
        write(&root, "api_pb2.py", "x = 1\n");
        write(&root, "api_pb2_grpc.py", "x = 1\n");
        write(&root, "api.grpc.pb.h", "void f() {}\n");
        write(&root, "readme.md", "x\n");
        write(&root, "plain.c", "void f() {}\n");
        write(&root, "stub.pyi", "x = 1\n");

        assert_eq!(
            paths(&root, false),
            vec![
                "benches/bench.rs".to_string(),
                "build.rs".to_string(),
                "examples/demo.py".to_string(),
                "src/WidgetTest.cpp".to_string(),
                "src/foo_test.rs".to_string(),
            ]
        );
    }

    #[test]
    fn fuzz_and_testing_directories_stay_out_until_tests() {
        let root = fixture("fuzz-testing");
        write(&root, "src/lib.rs", "fn kept() {}\n");
        write(&root, "src/testing.rs", "fn tool() {}\n");
        write(&root, "fuzz/harness.rs", "fn fuzz() {}\n");
        write(&root, "testing/wide.py", "x = 1\n");
        write(&root, "src/testing/a.py", "x = 1\n");
        write(&root, "pkg/fuzz/a.py", "x = 1\n");

        assert_eq!(
            paths(&root, false),
            vec!["src/lib.rs".to_string(), "src/testing.rs".to_string()]
        );
        assert_eq!(
            paths(&root, true),
            vec![
                "fuzz/harness.rs".to_string(),
                "pkg/fuzz/a.py".to_string(),
                "src/lib.rs".to_string(),
                "src/testing.rs".to_string(),
                "src/testing/a.py".to_string(),
                "testing/wide.py".to_string(),
            ]
        );
    }

    #[test]
    fn the_tests_switch_keeps_test_names_and_still_drops_protobuf() {
        let root = fixture("tests-names");
        write(&root, "src/foo_test.cpp", "void f() {}\n");
        write(&root, "api_pb2.py", "x = 1\n");
        write(&root, "src/a.py", "x = 1\n");

        assert_eq!(
            paths(&root, true),
            vec!["src/a.py".to_string(), "src/foo_test.cpp".to_string()]
        );
    }

    #[test]
    fn a_missing_root_is_an_error() {
        let missing = fixture("missing-parent").join("nope");
        let err = select(&missing, SelectOptions { tests: false }).unwrap_err();
        assert!(matches!(err, SelectError::NotADirectory(_)));
    }
}
