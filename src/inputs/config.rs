use std::fs;
use std::path::{Path, PathBuf};

use crate::ir::Language;

pub const F01_GATE: u32 = 8;
pub const F02_GATE: u32 = 8;
pub const F03_WARN: u32 = 10;
pub const F03_WARN_MAX: u32 = 20;
pub const F03_GATE: u32 = 15;
pub const F04_GATE: u32 = 3;
pub const F05_GATE: u32 = 2;
pub const F06_GATE: u32 = 19;
pub const F07_PASS_THROUGH_PERCENT: u32 = 80;
pub const F08_RECEIVER_HOPS: u32 = 2;
pub const F08_OTHER_HOPS: u32 = 1;
pub const F11_GATE: u32 = 19;
pub const PYTHON_F01_GATE: u32 = 10;
pub const PYTHON_F03_WARN: u32 = 8;
pub const PYTHON_F03_GATE: u32 = 12;
pub const PYTHON_F04_GATE: u32 = 10;
pub const PYTHON_F05_GATE: u32 = 3;
pub const PYTHON_F06_GATE: u32 = 51;
pub const PYTHON_F08_RECEIVER_HOPS: u32 = F08_RECEIVER_HOPS;
pub const PYTHON_F08_OTHER_HOPS: u32 = F08_OTHER_HOPS;
pub const PYTHON_F11_GATE: u32 = 19;
pub const RUST_F01_GATE: u32 = 10;
pub const RUST_F03_WARN: u32 = 3;
pub const RUST_F03_GATE: u32 = 6;
pub const RUST_F04_GATE: u32 = 3;
pub const RUST_F05_GATE: u32 = 3;
pub const RUST_F06_GATE: u32 = 45;
pub const RUST_F08_RECEIVER_HOPS: u32 = F08_RECEIVER_HOPS;
pub const RUST_F08_OTHER_HOPS: u32 = F08_OTHER_HOPS;
pub const RUST_F11_GATE: u32 = 17;
pub const CPP_F01_GATE: u32 = 12;
pub const CPP_F03_WARN: u32 = 7;
pub const CPP_F03_GATE: u32 = 15;
pub const CPP_F04_GATE: u32 = 4;
pub const CPP_F05_GATE: u32 = 3;
pub const CPP_F06_GATE: u32 = 41;
pub const CPP_F08_RECEIVER_HOPS: u32 = F08_RECEIVER_HOPS;
pub const CPP_F08_OTHER_HOPS: u32 = F08_OTHER_HOPS;
pub const CPP_F11_GATE: u32 = 25;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LanguageProfile {
    pub f01_gate: u32,
    pub f02_gate: u32,
    pub f03_warn: u32,
    pub f03_gate: u32,
    pub f04_gate: u32,
    pub f05_gate: u32,
    pub f06_gate: u32,
    pub f07_percent: u32,
    pub f08_receiver_hops: u32,
    pub f08_other_hops: u32,
    pub f08_allow: Vec<String>,
    pub f11_gate: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub coverage: Option<String>,
    pub mutation_rust: Option<String>,
    pub mutation_python: Option<String>,
    pub mutation_cpp: Option<String>,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub python: LanguageProfile,
    pub rust: LanguageProfile,
    pub cpp: LanguageProfile,
}

#[derive(Debug)]
pub enum ConfigError {
    NotADirectory(PathBuf),
    Read(PathBuf, std::io::Error),
    InvalidToml(String),
    UnknownKey(String),
    WrongType(String),
    EmptyOverride(String),
    InvalidPath(String),
    InvalidName(String),
    Negative(String),
    OutOfRange(String),
    WarnNotBelowGate {
        language: String,
        warn: u32,
        gate: u32,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::NotADirectory(path) => write!(f, "{} is not a directory", path.display()),
            ConfigError::Read(path, err) => write!(f, "reading {}: {err}", path.display()),
            ConfigError::InvalidToml(err) => write!(f, "invalid TOML: {err}"),
            ConfigError::UnknownKey(key) => write!(f, "unknown key {key}"),
            ConfigError::WrongType(key) => write!(f, "{key} has the wrong type"),
            ConfigError::EmptyOverride(key) => write!(f, "{key} is empty"),
            ConfigError::InvalidPath(path) => write!(f, "invalid path {path}"),
            ConfigError::InvalidName(name) => write!(f, "invalid allow name {name}"),
            ConfigError::Negative(key) => write!(f, "{key} is negative"),
            ConfigError::OutOfRange(key) => write!(f, "{key} is out of range"),
            ConfigError::WarnNotBelowGate {
                language,
                warn,
                gate,
            } => {
                write!(f, "{language} F03 warn {warn} is not below gate {gate}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn builtin() -> Self {
        Self {
            coverage: None,
            mutation_rust: None,
            mutation_python: None,
            mutation_cpp: None,
            include: None,
            exclude: None,
            python: LanguageProfile::builtin(Language::Python),
            rust: LanguageProfile::builtin(Language::Rust),
            cpp: LanguageProfile::builtin(Language::Cpp),
        }
    }

    pub fn profile(&self, language: Language) -> &LanguageProfile {
        match language {
            Language::Python => &self.python,
            Language::Rust => &self.rust,
            Language::Cpp => &self.cpp,
        }
    }

    pub fn set_coverage_override(&mut self, path: impl Into<String>) {
        self.coverage = Some(path.into());
    }

    pub fn set_mutation_rust_override(&mut self, path: impl Into<String>) {
        self.mutation_rust = Some(path.into());
    }

    pub fn set_mutation_python_override(&mut self, path: impl Into<String>) {
        self.mutation_python = Some(path.into());
    }

    pub fn set_mutation_cpp_override(&mut self, path: impl Into<String>) {
        self.mutation_cpp = Some(path.into());
    }

    pub fn retains(&self, path: &str) -> bool {
        if let Some(include) = &self.include {
            if !include.iter().any(|pattern| pattern_matches(pattern, path)) {
                return false;
            }
        }
        if let Some(exclude) = &self.exclude {
            if exclude.iter().any(|pattern| pattern_matches(pattern, path)) {
                return false;
            }
        }
        true
    }
}

impl LanguageProfile {
    fn builtin(language: Language) -> Self {
        let (f01, f03_warn, f03, f04, f05, f06, f08_receiver, f08_other, f11) = match language {
            Language::Python => (
                PYTHON_F01_GATE,
                PYTHON_F03_WARN,
                PYTHON_F03_GATE,
                PYTHON_F04_GATE,
                PYTHON_F05_GATE,
                PYTHON_F06_GATE,
                PYTHON_F08_RECEIVER_HOPS,
                PYTHON_F08_OTHER_HOPS,
                PYTHON_F11_GATE,
            ),
            Language::Rust => (
                RUST_F01_GATE,
                RUST_F03_WARN,
                RUST_F03_GATE,
                RUST_F04_GATE,
                RUST_F05_GATE,
                RUST_F06_GATE,
                RUST_F08_RECEIVER_HOPS,
                RUST_F08_OTHER_HOPS,
                RUST_F11_GATE,
            ),
            Language::Cpp => (
                CPP_F01_GATE,
                CPP_F03_WARN,
                CPP_F03_GATE,
                CPP_F04_GATE,
                CPP_F05_GATE,
                CPP_F06_GATE,
                CPP_F08_RECEIVER_HOPS,
                CPP_F08_OTHER_HOPS,
                CPP_F11_GATE,
            ),
        };
        Self {
            f01_gate: f01,
            f02_gate: F02_GATE,
            f03_warn,
            f03_gate: f03,
            f04_gate: f04,
            f05_gate: f05,
            f06_gate: f06,
            f07_percent: F07_PASS_THROUGH_PERCENT,
            f08_receiver_hops: f08_receiver,
            f08_other_hops: f08_other,
            f08_allow: Vec::new(),
            f11_gate: f11,
        }
    }
}

pub fn load_config(root: &Path) -> Result<Config, ConfigError> {
    if !root.is_dir() {
        return Err(ConfigError::NotADirectory(root.to_path_buf()));
    }
    let path = root.join("slopmeter.toml");
    if !path.exists() {
        return Ok(Config::builtin());
    }
    let text = fs::read_to_string(&path).map_err(|err| ConfigError::Read(path, err))?;
    parse_config(&text)
}

fn parse_config(text: &str) -> Result<Config, ConfigError> {
    let table: toml::Table = text
        .parse()
        .map_err(|err: toml::de::Error| ConfigError::InvalidToml(err.to_string()))?;
    let mut config = Config::builtin();
    for (key, value) in &table {
        match key.as_str() {
            "coverage" => config.coverage = Some(report_path(value, "coverage")?),
            "mutation" => apply_mutation(&mut config, value)?,
            "include" => config.include = Some(pattern_list(value, "include")?),
            "exclude" => config.exclude = Some(pattern_list(value, "exclude")?),
            "profile" => apply_profile(&mut config, value)?,
            other => return Err(ConfigError::UnknownKey(other.to_string())),
        }
    }
    Ok(config)
}

fn report_path(value: &toml::Value, key: &str) -> Result<String, ConfigError> {
    let Some(path) = value.as_str() else {
        return Err(ConfigError::WrongType(key.to_string()));
    };
    if path.is_empty() {
        return Err(ConfigError::EmptyOverride(key.to_string()));
    }
    validate_repo_path(path)?;
    Ok(path.to_string())
}

fn apply_mutation(config: &mut Config, value: &toml::Value) -> Result<(), ConfigError> {
    let Some(table) = value.as_table() else {
        return Err(ConfigError::WrongType("mutation".to_string()));
    };
    if table.is_empty() {
        return Err(ConfigError::EmptyOverride("mutation".to_string()));
    }
    for (key, item) in table {
        match key.as_str() {
            "rust" => config.mutation_rust = Some(report_path(item, "mutation.rust")?),
            "python" => config.mutation_python = Some(report_path(item, "mutation.python")?),
            "cpp" => config.mutation_cpp = Some(report_path(item, "mutation.cpp")?),
            other => return Err(ConfigError::UnknownKey(format!("mutation.{other}"))),
        }
    }
    Ok(())
}

fn pattern_list(value: &toml::Value, key: &str) -> Result<Vec<String>, ConfigError> {
    let items = string_list(value, key)?;
    for pattern in &items {
        validate_pattern(pattern)?;
    }
    Ok(items)
}

fn string_list(value: &toml::Value, key: &str) -> Result<Vec<String>, ConfigError> {
    let Some(array) = value.as_array() else {
        return Err(ConfigError::WrongType(key.to_string()));
    };
    if array.is_empty() {
        return Err(ConfigError::EmptyOverride(key.to_string()));
    }
    let mut items = Vec::with_capacity(array.len());
    for item in array {
        let Some(text) = item.as_str() else {
            return Err(ConfigError::WrongType(key.to_string()));
        };
        items.push(text.to_string());
    }
    Ok(items)
}

fn apply_profile(config: &mut Config, value: &toml::Value) -> Result<(), ConfigError> {
    let Some(table) = value.as_table() else {
        return Err(ConfigError::WrongType("profile".to_string()));
    };
    if table.is_empty() {
        return Err(ConfigError::EmptyOverride("profile".to_string()));
    }
    for (name, rules) in table {
        let language = match name.as_str() {
            "python" => Language::Python,
            "rust" => Language::Rust,
            "cpp" => Language::Cpp,
            other => return Err(ConfigError::UnknownKey(format!("profile.{other}"))),
        };
        apply_language(config, language, rules)?;
    }
    Ok(())
}

fn apply_language(
    config: &mut Config,
    language: Language,
    value: &toml::Value,
) -> Result<(), ConfigError> {
    let label = language_name(language);
    let Some(table) = value.as_table() else {
        return Err(ConfigError::WrongType(format!("profile.{label}")));
    };
    if table.is_empty() {
        return Err(ConfigError::EmptyOverride(format!("profile.{label}")));
    }
    for (rule, body) in table {
        match rule.as_str() {
            "F01" | "F02" | "F04" | "F05" | "F06" | "F11" => {
                apply_count_gate(config, language, rule, body)?;
            }
            "F03" => apply_f03(config, language, body)?,
            "F08" => apply_f08(config, language, body)?,
            other => return Err(ConfigError::UnknownKey(format!("profile.{label}.{other}"))),
        }
    }
    Ok(())
}

fn apply_count_gate(
    config: &mut Config,
    language: Language,
    rule: &str,
    value: &toml::Value,
) -> Result<(), ConfigError> {
    let label = language_name(language);
    let key = format!("profile.{label}.{rule}");
    let Some(table) = value.as_table() else {
        return Err(ConfigError::WrongType(key));
    };
    if table.is_empty() {
        return Err(ConfigError::EmptyOverride(key));
    }
    let mut gate = None;
    for (name, item) in table {
        match name.as_str() {
            "gate" => gate = Some(nonneg(item, &format!("{key}.gate"))?),
            other => return Err(ConfigError::UnknownKey(format!("{key}.{other}"))),
        }
    }
    let Some(gate) = gate else {
        return Err(ConfigError::EmptyOverride(key));
    };
    let ceiling = builtin_gate(language, rule).saturating_mul(2);
    if gate > ceiling {
        return Err(ConfigError::OutOfRange(format!("{key}.gate")));
    }
    let profile = config.profile_mut(language);
    match rule {
        "F01" => profile.f01_gate = gate,
        "F02" => profile.f02_gate = gate,
        "F04" => profile.f04_gate = gate,
        "F05" => profile.f05_gate = gate,
        "F06" => profile.f06_gate = gate,
        "F11" => profile.f11_gate = gate,
        _ => return Err(ConfigError::UnknownKey(key)),
    }
    Ok(())
}

fn builtin_gate(language: Language, rule: &str) -> u32 {
    let profile = LanguageProfile::builtin(language);
    match rule {
        "F01" => profile.f01_gate,
        "F02" => profile.f02_gate,
        "F03" => profile.f03_gate,
        "F04" => profile.f04_gate,
        "F05" => profile.f05_gate,
        "F06" => profile.f06_gate,
        "F11" => profile.f11_gate,
        _ => 0,
    }
}

fn apply_f03(
    config: &mut Config,
    language: Language,
    value: &toml::Value,
) -> Result<(), ConfigError> {
    let label = language_name(language);
    let Some(table) = value.as_table() else {
        return Err(ConfigError::WrongType(format!("profile.{label}.F03")));
    };
    if table.is_empty() {
        return Err(ConfigError::EmptyOverride(format!("profile.{label}.F03")));
    }
    let mut warn = None;
    let mut gate = None;
    for (key, item) in table {
        match key.as_str() {
            "warn" => warn = Some(nonneg(item, &format!("profile.{label}.F03.warn"))?),
            "gate" => gate = Some(nonneg(item, &format!("profile.{label}.F03.gate"))?),
            other => {
                return Err(ConfigError::UnknownKey(format!(
                    "profile.{label}.F03.{other}"
                )))
            }
        }
    }
    if warn.is_none() && gate.is_none() {
        return Err(ConfigError::EmptyOverride(format!("profile.{label}.F03")));
    }
    let builtin_profile = LanguageProfile::builtin(language);
    let warn = warn.unwrap_or(builtin_profile.f03_warn);
    let builtin = builtin_profile.f03_gate;
    let gate = gate.unwrap_or(builtin);
    if warn > F03_WARN_MAX {
        return Err(ConfigError::OutOfRange(format!("profile.{label}.F03.warn")));
    }
    if gate < 1 || gate > builtin.saturating_mul(2) {
        return Err(ConfigError::OutOfRange(format!("profile.{label}.F03.gate")));
    }
    if warn >= gate {
        return Err(ConfigError::WarnNotBelowGate {
            language: label.to_string(),
            warn,
            gate,
        });
    }
    let profile = config.profile_mut(language);
    profile.f03_warn = warn;
    profile.f03_gate = gate;
    Ok(())
}

fn apply_f08(
    config: &mut Config,
    language: Language,
    value: &toml::Value,
) -> Result<(), ConfigError> {
    let label = language_name(language);
    let Some(table) = value.as_table() else {
        return Err(ConfigError::WrongType(format!("profile.{label}.F08")));
    };
    if table.is_empty() {
        return Err(ConfigError::EmptyOverride(format!("profile.{label}.F08")));
    }
    let mut allow = None;
    for (key, item) in table {
        match key.as_str() {
            "allow" => allow = Some(string_list(item, &format!("profile.{label}.F08.allow"))?),
            other => {
                return Err(ConfigError::UnknownKey(format!(
                    "profile.{label}.F08.{other}"
                )))
            }
        }
    }
    let Some(names) = allow else {
        return Err(ConfigError::EmptyOverride(format!("profile.{label}.F08")));
    };
    for name in &names {
        if name.is_empty() || name.contains(['.', ':', '/']) {
            return Err(ConfigError::InvalidName(name.clone()));
        }
    }
    config.profile_mut(language).f08_allow = names;
    Ok(())
}

fn nonneg(value: &toml::Value, key: &str) -> Result<u32, ConfigError> {
    let Some(number) = value.as_integer() else {
        return Err(ConfigError::WrongType(key.to_string()));
    };
    if number < 0 {
        return Err(ConfigError::Negative(key.to_string()));
    }
    u32::try_from(number).map_err(|_| ConfigError::WrongType(key.to_string()))
}

fn validate_repo_path(path: &str) -> Result<(), ConfigError> {
    if path.starts_with('/') || path.starts_with('\\') || has_drive_prefix(path) {
        return Err(ConfigError::InvalidPath(path.to_string()));
    }
    if path.split(['/', '\\']).any(|segment| segment == "..") {
        return Err(ConfigError::InvalidPath(path.to_string()));
    }
    Ok(())
}

fn validate_pattern(pattern: &str) -> Result<(), ConfigError> {
    if pattern.is_empty() || pattern.contains('!') {
        return Err(ConfigError::InvalidPath(pattern.to_string()));
    }
    let trimmed = pattern.trim_end_matches('/');
    if trimmed.is_empty()
        || trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || has_drive_prefix(trimmed)
    {
        return Err(ConfigError::InvalidPath(pattern.to_string()));
    }
    for segment in trimmed.split(['/', '\\']) {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(ConfigError::InvalidPath(pattern.to_string()));
        }
        if segment.contains("**") && segment != "**" {
            return Err(ConfigError::InvalidPath(pattern.to_string()));
        }
    }
    Ok(())
}

fn has_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn pattern_matches(pattern: &str, path: &str) -> bool {
    let pattern = pattern.trim_end_matches('/');
    if !pattern.contains('*') {
        return path == pattern || path.starts_with(&format!("{pattern}/"));
    }
    let pattern_segments: Vec<&str> = pattern
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let path_segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    segments_match(&pattern_segments, &path_segments)
}

fn segments_match(pattern: &[&str], path: &[&str]) -> bool {
    let Some(head) = pattern.first() else {
        return path.is_empty();
    };
    if *head == "**" {
        if segments_match(&pattern[1..], path) {
            return true;
        }
        return !path.is_empty() && segments_match(pattern, &path[1..]);
    }
    !path.is_empty() && segment_matches(head, path[0]) && segments_match(&pattern[1..], &path[1..])
}

fn segment_matches(pattern: &str, segment: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == segment;
    }
    if !segment.starts_with(parts[0]) {
        return false;
    }
    let mut rest = &segment[parts[0].len()..];
    for (index, part) in parts.iter().enumerate().skip(1) {
        if index + 1 == parts.len() {
            return rest.ends_with(part);
        }
        if part.is_empty() {
            continue;
        }
        let Some(found) = rest.find(part) else {
            return false;
        };
        rest = &rest[found + part.len()..];
    }
    true
}

pub fn departures(config: &Config) -> Vec<String> {
    let mut notes = Vec::new();
    for language in [Language::Python, Language::Rust, Language::Cpp] {
        let current = config.profile(language);
        let builtin = LanguageProfile::builtin(language);
        let name = language_label(language);
        note_gate(&mut notes, name, "F01", current.f01_gate, builtin.f01_gate);
        note_gate(&mut notes, name, "F02", current.f02_gate, builtin.f02_gate);
        if current.f03_warn != builtin.f03_warn {
            notes.push(format!(
                "{name} F03 warn is {}, built-in {}",
                current.f03_warn, builtin.f03_warn
            ));
        }
        note_gate(&mut notes, name, "F03", current.f03_gate, builtin.f03_gate);
        note_gate(&mut notes, name, "F04", current.f04_gate, builtin.f04_gate);
        note_gate(&mut notes, name, "F05", current.f05_gate, builtin.f05_gate);
        note_gate(&mut notes, name, "F06", current.f06_gate, builtin.f06_gate);
        if !current.f08_allow.is_empty() {
            notes.push(format!(
                "{name} F08 allow adds {}",
                current.f08_allow.join(", ")
            ));
        }
        note_gate(&mut notes, name, "F11", current.f11_gate, builtin.f11_gate);
    }
    notes
}

fn note_gate(notes: &mut Vec<String>, language: &str, rule: &str, value: u32, builtin: u32) {
    if value != builtin {
        notes.push(format!(
            "{language} {rule} gate is {value}, built-in {builtin}"
        ));
    }
}

fn language_name(language: Language) -> &'static str {
    match language {
        Language::Python => "python",
        Language::Rust => "rust",
        Language::Cpp => "cpp",
    }
}

fn language_label(language: Language) -> &'static str {
    match language {
        Language::Python => "Python",
        Language::Rust => "Rust",
        Language::Cpp => "C++",
    }
}

impl Config {
    fn profile_mut(&mut self, language: Language) -> &mut LanguageProfile {
        match language {
            Language::Python => &mut self.python,
            Language::Rust => &mut self.rust,
            Language::Cpp => &mut self.cpp,
        }
    }
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
            .join("config-fixtures")
            .join(format!(
                "{name}-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_config(root: &Path, body: &str) {
        fs::write(root.join("slopmeter.toml"), body).unwrap();
    }

    #[test]
    fn a_missing_file_is_the_built_in_profile() {
        let root = fixture("missing");
        fs::write(
            root.join("nested").join("slopmeter.toml"),
            "coverage = \"nope.lcov\"\n",
        )
        .unwrap_or_else(|_| {
            fs::create_dir_all(root.join("nested")).unwrap();
            fs::write(
                root.join("nested").join("slopmeter.toml"),
                "coverage = \"nope.lcov\"\n",
            )
            .unwrap();
        });

        let config = load_config(&root).unwrap();
        assert_eq!(config.coverage, None);
        assert_eq!(config.include, None);
        assert_eq!(config.exclude, None);
        let python = config.profile(Language::Python);
        assert_eq!(python.f01_gate, PYTHON_F01_GATE);
        assert_eq!(python.f02_gate, F02_GATE);
        assert_eq!(python.f03_warn, PYTHON_F03_WARN);
        assert_eq!(python.f03_gate, PYTHON_F03_GATE);
        assert_eq!(python.f04_gate, PYTHON_F04_GATE);
        assert_eq!(python.f05_gate, PYTHON_F05_GATE);
        assert_eq!(python.f06_gate, PYTHON_F06_GATE);
        assert_eq!(python.f07_percent, F07_PASS_THROUGH_PERCENT);
        assert_eq!(python.f08_receiver_hops, PYTHON_F08_RECEIVER_HOPS);
        assert_eq!(python.f08_other_hops, PYTHON_F08_OTHER_HOPS);
        assert!(python.f08_allow.is_empty());
        assert_eq!(python.f11_gate, PYTHON_F11_GATE);
        let rust = config.profile(Language::Rust);
        assert_eq!(rust.f01_gate, RUST_F01_GATE);
        assert_eq!(rust.f03_warn, RUST_F03_WARN);
        assert_eq!(rust.f03_gate, RUST_F03_GATE);
        assert_eq!(rust.f04_gate, RUST_F04_GATE);
        assert_eq!(rust.f05_gate, RUST_F05_GATE);
        assert_eq!(rust.f06_gate, RUST_F06_GATE);
        assert_eq!(rust.f08_receiver_hops, RUST_F08_RECEIVER_HOPS);
        assert_eq!(rust.f08_other_hops, RUST_F08_OTHER_HOPS);
        assert_eq!(rust.f11_gate, RUST_F11_GATE);
        let cpp = config.profile(Language::Cpp);
        assert_eq!(cpp.f01_gate, CPP_F01_GATE);
        assert_eq!(cpp.f02_gate, F02_GATE);
        assert_eq!(cpp.f03_warn, CPP_F03_WARN);
        assert_eq!(cpp.f03_gate, CPP_F03_GATE);
        assert_eq!(cpp.f04_gate, CPP_F04_GATE);
        assert_eq!(cpp.f05_gate, CPP_F05_GATE);
        assert_eq!(cpp.f06_gate, CPP_F06_GATE);
        assert_eq!(cpp.f07_percent, F07_PASS_THROUGH_PERCENT);
        assert_eq!(cpp.f08_receiver_hops, CPP_F08_RECEIVER_HOPS);
        assert_eq!(cpp.f08_other_hops, CPP_F08_OTHER_HOPS);
        assert!(cpp.f08_allow.is_empty());
        assert_eq!(cpp.f11_gate, CPP_F11_GATE);
        assert!(config.retains("src/a.py"));
        assert!(config.retains("build.rs"));
    }

    #[test]
    fn coverage_include_exclude_f03_gate_and_f08_allow_parse() {
        let root = fixture("departures");
        write_config(
            &root,
            r#"
coverage = "coverage.lcov"
include = ["crates/**", "python/**"]
exclude = ["crates/xtask/**"]

[profile.python.F03]
gate = 12

[profile.rust.F08]
allow = ["with_timeout"]
"#,
        );

        let config = load_config(&root).unwrap();
        assert_eq!(config.coverage.as_deref(), Some("coverage.lcov"));
        assert!(config.retains("crates/slopmeter/src/lib.rs"));
        assert!(config.retains("python/app.py"));
        assert!(!config.retains("src/a.py"));
        assert!(!config.retains("crates/xtask/src/main.rs"));
        assert!(!config.retains("Crates/slopmeter/src/lib.rs"));
        assert_eq!(config.profile(Language::Python).f03_warn, PYTHON_F03_WARN);
        assert_eq!(config.profile(Language::Python).f03_gate, 12);
        assert_eq!(config.profile(Language::Rust).f03_gate, RUST_F03_GATE);
        assert_eq!(config.profile(Language::Cpp).f03_gate, CPP_F03_GATE);
        assert_eq!(
            config.profile(Language::Rust).f08_allow,
            vec!["with_timeout".to_string()]
        );
        assert!(config.profile(Language::Python).f08_allow.is_empty());
        assert_eq!(
            config.profile(Language::Rust).f07_percent,
            F07_PASS_THROUGH_PERCENT
        );
    }

    #[test]
    fn a_directory_pattern_covers_its_contents_and_leaves_the_sibling_file() {
        let root = fixture("directory");
        write_config(&root, "exclude = [\"build\"]\n");
        let config = load_config(&root).unwrap();
        assert!(!config.retains("build/lib.rs"));
        assert!(!config.retains("build/nested/a.rs"));
        assert!(config.retains("build.rs"));
        assert!(config.retains("src/build/lib.rs"));
    }

    #[test]
    fn a_star_stays_inside_one_segment() {
        let root = fixture("star");
        write_config(&root, "include = [\"src/*.py\"]\n");
        let config = load_config(&root).unwrap();
        assert!(config.retains("src/a.py"));
        assert!(!config.retains("src/sub/a.py"));
        assert!(!config.retains("src/a.rs"));
    }

    #[test]
    fn an_unknown_key_an_empty_include_and_a_gate_below_warn_fail() {
        let root = fixture("errors");
        write_config(&root, "weight = 1\n");
        assert!(load_config(&root).is_err());

        write_config(&root, "include = []\n");
        assert!(matches!(
            load_config(&root).unwrap_err(),
            ConfigError::EmptyOverride(_)
        ));

        write_config(&root, "[profile.python.F03]\ngate = 8\n");
        assert!(matches!(
            load_config(&root).unwrap_err(),
            ConfigError::WarnNotBelowGate {
                warn: 8,
                gate: 8,
                ..
            }
        ));

        write_config(&root, "[profile.python.F03]\ngate = 7\n");
        assert!(load_config(&root).is_err());
    }

    #[test]
    fn a_warn_only_table_keeps_the_language_gate() {
        let root = fixture("warn-only");
        write_config(&root, "[profile.python.F03]\nwarn = 11\n");
        let python = load_config(&root)
            .unwrap()
            .profile(Language::Python)
            .clone();
        assert_eq!(python.f03_warn, 11);
        assert_eq!(python.f03_gate, PYTHON_F03_GATE);
        assert_eq!(python.f01_gate, PYTHON_F01_GATE);
    }

    #[test]
    fn a_count_gate_runs_from_zero_to_twice_the_built_in() {
        let root = fixture("bounds");
        write_config(&root, "[profile.python.F01]\ngate = 0\n");
        assert_eq!(
            load_config(&root)
                .unwrap()
                .profile(Language::Python)
                .f01_gate,
            0
        );

        write_config(&root, "[profile.python.F01]\ngate = 20\n");
        assert_eq!(
            load_config(&root)
                .unwrap()
                .profile(Language::Python)
                .f01_gate,
            20
        );

        write_config(
            &root,
            "[profile.rust.F04]\ngate = 0\n\n[profile.cpp.F06]\ngate = 82\n\n[profile.python.F02]\ngate = 16\n",
        );
        let config = load_config(&root).unwrap();
        assert_eq!(config.profile(Language::Rust).f04_gate, 0);
        assert_eq!(config.profile(Language::Cpp).f06_gate, 82);
        assert_eq!(config.profile(Language::Python).f02_gate, 16);
        assert_eq!(config.profile(Language::Python).f04_gate, PYTHON_F04_GATE);

        write_config(&root, "[profile.python.F01]\ngate = 21\n");
        let err = load_config(&root).unwrap_err().to_string();
        assert!(err.contains("profile.python.F01.gate"), "{err}");

        write_config(&root, "[profile.python.F03]\nwarn = 0\ngate = 1\n");
        let python = load_config(&root)
            .unwrap()
            .profile(Language::Python)
            .clone();
        assert_eq!(python.f03_warn, 0);
        assert_eq!(python.f03_gate, 1);

        write_config(&root, "[profile.python.F03]\nwarn = 20\ngate = 24\n");
        let python = load_config(&root)
            .unwrap()
            .profile(Language::Python)
            .clone();
        assert_eq!(python.f03_warn, 20);
        assert_eq!(python.f03_gate, 24);

        write_config(&root, "[profile.python.F03]\ngate = 0\nwarn = 0\n");
        let err = load_config(&root).unwrap_err().to_string();
        assert!(err.contains("profile.python.F03.gate"), "{err}");

        write_config(&root, "[profile.python.F03]\nwarn = 21\n");
        let err = load_config(&root).unwrap_err().to_string();
        assert!(err.contains("profile.python.F03.warn"), "{err}");

        write_config(&root, "[profile.rust.F03]\ngate = 12\nwarn = 5\n");
        let rust_profile = load_config(&root).unwrap().profile(Language::Rust).clone();
        assert_eq!(rust_profile.f03_gate, 12);
        assert_eq!(rust_profile.f03_warn, 5);

        write_config(&root, "[profile.rust.F03]\ngate = 13\nwarn = 5\n");
        let err = load_config(&root).unwrap_err().to_string();
        assert!(err.contains("profile.rust.F03.gate"), "{err}");
    }

    #[test]
    fn other_rules_wrong_types_and_bad_paths_fail() {
        let root = fixture("rejected");
        let rejected = [
            "[profile.python.F07]\nwarn = 80\n",
            "[profile.python.S06]\ngate = 1\n",
            "[profile.python.S07]\ngate = 1\n",
            "[profile.python.F01]\nwarn = 10\n",
            "[profile.python.F03]\nallow = [\"x\"]\n",
            "[profile.rust.F08]\ngate = 2\n",
            "coverage = \"\"\n",
            "coverage = \"/tmp/coverage.lcov\"\n",
            "coverage = \"../coverage.lcov\"\n",
            "coverage = 1\n",
            "include = \"src/**\"\n",
            "[profile.python.F03]\ngate = -1\n",
            "[profile.python.F03]\ngate = 12.5\n",
            "include = [\"../secret\"]\n",
            "include = [\"!src\"]\n",
            "exclude = []\n",
            "[profile.rust.F08]\nallow = []\n",
            "[profile.rust.F08]\nallow = [\"\"]\n",
            "[profile.rust.F08]\nallow = [\"a.b\"]\n",
            "[profile.rust.F08]\nallow = [\"a:b\"]\n",
            "[profile.rust.F08]\nallow = [\"a/b\"]\n",
            "[profile.python.F03]\n",
            "coverage =\n",
            "[profile.python]\n",
            "[mutation]\n",
            "[mutation]\nrust = \"\"\n",
            "[mutation]\nrust = \"/tmp/outcomes.json\"\n",
            "[mutation]\nrust = 1\n",
        ];
        for body in rejected {
            write_config(&root, body);
            assert!(load_config(&root).is_err(), "{body}");
        }
    }

    #[test]
    fn a_coverage_flag_may_be_absolute() {
        let root = fixture("flag");
        write_config(&root, "coverage = \"coverage.lcov\"\n");
        let mut config = load_config(&root).unwrap();
        config.set_coverage_override("/tmp/target/coverage.lcov");
        assert_eq!(
            config.coverage.as_deref(),
            Some("/tmp/target/coverage.lcov")
        );
    }

    #[test]
    fn a_missing_coverage_file_still_loads() {
        let root = fixture("absent-lcov");
        write_config(&root, "coverage = \"target/coverage.lcov\"\n");
        let config = load_config(&root).unwrap();
        assert_eq!(config.coverage.as_deref(), Some("target/coverage.lcov"));
    }

    #[test]
    fn a_mutation_rust_path_loads_and_the_flag_replaces_it() {
        let root = fixture("mutation");
        write_config(&root, "[mutation]\nrust = \"mutants.out/outcomes.json\"\n");
        let mut config = load_config(&root).unwrap();
        assert_eq!(
            config.mutation_rust.as_deref(),
            Some("mutants.out/outcomes.json")
        );
        config.set_mutation_rust_override("/tmp/outcomes.json");
        assert_eq!(config.mutation_rust.as_deref(), Some("/tmp/outcomes.json"));
    }

    #[test]
    fn a_mutation_python_path_loads_and_the_flag_replaces_it() {
        let root = fixture("mutation-python");
        write_config(&root, "[mutation]\npython = \"mutants\"\n");
        let mut config = load_config(&root).unwrap();
        assert_eq!(config.mutation_python.as_deref(), Some("mutants"));
        config.set_mutation_python_override("/tmp/mutants");
        assert_eq!(config.mutation_python.as_deref(), Some("/tmp/mutants"));
    }

    #[test]
    fn a_mutation_cpp_path_loads_and_the_flag_replaces_it() {
        let root = fixture("mutation-cpp");
        write_config(&root, "[mutation]\ncpp = \"mull.json\"\n");
        let mut config = load_config(&root).unwrap();
        assert_eq!(config.mutation_cpp.as_deref(), Some("mull.json"));
        config.set_mutation_cpp_override("/tmp/mull.json");
        assert_eq!(config.mutation_cpp.as_deref(), Some("/tmp/mull.json"));
    }

    #[test]
    fn departures_name_each_changed_level_and_skip_a_written_built_in() {
        let root = fixture("departure-notes");
        write_config(
            &root,
            r#"
coverage = "coverage.lcov"
include = ["pkg/**"]

[mutation]
rust = "outcomes.json"

[profile.python.F01]
gate = 10

[profile.python.F03]
warn = 7

[profile.python.F04]
gate = 20

[profile.python.F08]
allow = ["builder", "with_timeout"]

[profile.rust.F01]
gate = 11

[profile.cpp.F02]
gate = 8

[profile.cpp.F11]
gate = 0
"#,
        );

        let notes = departures(&load_config(&root).unwrap());
        assert_eq!(
            notes,
            vec![
                "Python F03 warn is 7, built-in 8".to_string(),
                "Python F04 gate is 20, built-in 10".to_string(),
                "Python F08 allow adds builder, with_timeout".to_string(),
                "Rust F01 gate is 11, built-in 10".to_string(),
                "C++ F11 gate is 0, built-in 25".to_string(),
            ]
        );
    }
}
