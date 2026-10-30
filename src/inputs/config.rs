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
    pub f09_allow: Vec<String>,
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
            other => other.fmt_rest(f),
        }
    }
}

impl ConfigError {
    fn fmt_rest(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
            _ => Ok(()),
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

struct BuiltinGates {
    f01: u32,
    f03_warn: u32,
    f03: u32,
    f04: u32,
    f05: u32,
    f06: u32,
    f08_receiver: u32,
    f08_other: u32,
    f11: u32,
}

impl LanguageProfile {
    fn builtin(language: Language) -> Self {
        let gates = match language {
            Language::Python => python_gates(),
            Language::Rust => rust_gates(),
            Language::Cpp => cpp_gates(),
        };
        profile_from(gates)
    }
}

fn python_gates() -> BuiltinGates {
    BuiltinGates {
        f01: PYTHON_F01_GATE,
        f03_warn: PYTHON_F03_WARN,
        f03: PYTHON_F03_GATE,
        f04: PYTHON_F04_GATE,
        f05: PYTHON_F05_GATE,
        f06: PYTHON_F06_GATE,
        f08_receiver: PYTHON_F08_RECEIVER_HOPS,
        f08_other: PYTHON_F08_OTHER_HOPS,
        f11: PYTHON_F11_GATE,
    }
}

fn rust_gates() -> BuiltinGates {
    BuiltinGates {
        f01: RUST_F01_GATE,
        f03_warn: RUST_F03_WARN,
        f03: RUST_F03_GATE,
        f04: RUST_F04_GATE,
        f05: RUST_F05_GATE,
        f06: RUST_F06_GATE,
        f08_receiver: RUST_F08_RECEIVER_HOPS,
        f08_other: RUST_F08_OTHER_HOPS,
        f11: RUST_F11_GATE,
    }
}

fn cpp_gates() -> BuiltinGates {
    BuiltinGates {
        f01: CPP_F01_GATE,
        f03_warn: CPP_F03_WARN,
        f03: CPP_F03_GATE,
        f04: CPP_F04_GATE,
        f05: CPP_F05_GATE,
        f06: CPP_F06_GATE,
        f08_receiver: CPP_F08_RECEIVER_HOPS,
        f08_other: CPP_F08_OTHER_HOPS,
        f11: CPP_F11_GATE,
    }
}

fn profile_from(gates: BuiltinGates) -> LanguageProfile {
    LanguageProfile {
        f01_gate: gates.f01,
        f02_gate: F02_GATE,
        f03_warn: gates.f03_warn,
        f03_gate: gates.f03,
        f04_gate: gates.f04,
        f05_gate: gates.f05,
        f06_gate: gates.f06,
        f07_percent: F07_PASS_THROUGH_PERCENT,
        f08_receiver_hops: gates.f08_receiver,
        f08_other_hops: gates.f08_other,
        f08_allow: Vec::new(),
        f09_allow: Vec::new(),
        f11_gate: gates.f11,
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
    let parsed = text.parse::<toml::Table>();
    let table = parsed.map_err(invalid_toml)?;
    let mut config = Config::builtin();
    for (key, value) in &table {
        apply_root(&mut config, key.as_str(), value)?;
    }
    Ok(config)
}

fn invalid_toml(err: toml::de::Error) -> ConfigError {
    ConfigError::InvalidToml(err.to_string())
}

fn apply_root(config: &mut Config, key: &str, value: &toml::Value) -> Result<(), ConfigError> {
    if key == "coverage" {
        let path = report_path(value, "coverage")?;
        config.coverage = Some(path);
        return Ok(());
    }
    if key == "profile" {
        return apply_profile(config, value);
    }
    apply_listed(config, key, value)
}

fn apply_listed(config: &mut Config, key: &str, value: &toml::Value) -> Result<(), ConfigError> {
    match key {
        "mutation" => apply_mutation(config, value)?,
        "include" => {
            let items = pattern_list(value, "include")?;
            config.include = Some(items);
        }
        "exclude" => {
            let items = pattern_list(value, "exclude")?;
            config.exclude = Some(items);
        }
        other => return Err(ConfigError::UnknownKey(other.to_string())),
    }
    Ok(())
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
        apply_mutation_key(config, key.as_str(), item)?;
    }
    Ok(())
}

fn apply_mutation_key(
    config: &mut Config,
    key: &str,
    item: &toml::Value,
) -> Result<(), ConfigError> {
    match key {
        "rust" => {
            let path = report_path(item, "mutation.rust")?;
            config.mutation_rust = Some(path);
        }
        "python" => {
            let path = report_path(item, "mutation.python")?;
            config.mutation_python = Some(path);
        }
        "cpp" => {
            let path = report_path(item, "mutation.cpp")?;
            config.mutation_cpp = Some(path);
        }
        other => return Err(ConfigError::UnknownKey(format!("mutation.{other}"))),
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

struct ProfileRule<'a> {
    label: &'a str,
    name: &'a str,
    body: &'a toml::Value,
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
        let entry = ProfileRule {
            label,
            name: rule.as_str(),
            body,
        };
        apply_profile_rule(config, language, entry)?;
    }
    Ok(())
}

fn apply_profile_rule(
    config: &mut Config,
    language: Language,
    rule: ProfileRule<'_>,
) -> Result<(), ConfigError> {
    if matches!(rule.name, "F01" | "F02" | "F04" | "F05" | "F06" | "F11") {
        return apply_count_gate(
            config,
            CountGate {
                language,
                rule: rule.name,
                value: rule.body,
            },
        );
    }
    match rule.name {
        "F03" => apply_f03(config, language, rule.body)?,
        "F08" => apply_f08(config, language, rule.body)?,
        "F09" => apply_f09(config, language, rule.body)?,
        other => {
            return Err(ConfigError::UnknownKey(format!(
                "profile.{}.{other}",
                rule.label
            )))
        }
    }
    Ok(())
}

struct CountGate<'a> {
    language: Language,
    rule: &'a str,
    value: &'a toml::Value,
}

fn apply_count_gate(config: &mut Config, gate: CountGate<'_>) -> Result<(), ConfigError> {
    let key = count_key(gate.language, gate.rule);
    let number = count_gate_number(gate.value, &key)?;
    let ceiling = builtin_gate(gate.language, gate.rule).saturating_mul(2);
    if number > ceiling {
        return Err(ConfigError::OutOfRange(format!("{key}.gate")));
    }
    let profile = config.profile_mut(gate.language);
    assign_count_gate(
        profile,
        AssignedGate {
            rule: gate.rule,
            gate: number,
            key: &key,
        },
    )
}

fn count_key(language: Language, rule: &str) -> String {
    let label = language_name(language);
    format!("profile.{label}.{rule}")
}

fn count_gate_number(value: &toml::Value, key: &str) -> Result<u32, ConfigError> {
    let Some(table) = value.as_table() else {
        return Err(ConfigError::WrongType(key.to_string()));
    };
    if table.is_empty() {
        return Err(ConfigError::EmptyOverride(key.to_string()));
    }
    let mut gate = None;
    for (name, item) in table {
        gate = one_count_gate(name.as_str(), item, key)?;
    }
    gate.ok_or_else(|| ConfigError::EmptyOverride(key.to_string()))
}

fn one_count_gate(name: &str, item: &toml::Value, key: &str) -> Result<Option<u32>, ConfigError> {
    match name {
        "gate" => {
            let number = nonneg(item, &format!("{key}.gate"))?;
            Ok(Some(number))
        }
        other => Err(ConfigError::UnknownKey(format!("{key}.{other}"))),
    }
}

struct AssignedGate<'a> {
    rule: &'a str,
    gate: u32,
    key: &'a str,
}

fn assign_count_gate(
    profile: &mut LanguageProfile,
    assigned: AssignedGate<'_>,
) -> Result<(), ConfigError> {
    match assigned.rule {
        "F01" => profile.f01_gate = assigned.gate,
        "F02" => profile.f02_gate = assigned.gate,
        "F04" => profile.f04_gate = assigned.gate,
        "F05" => profile.f05_gate = assigned.gate,
        "F06" => profile.f06_gate = assigned.gate,
        "F11" => profile.f11_gate = assigned.gate,
        _ => return Err(ConfigError::UnknownKey(assigned.key.to_string())),
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

struct F03Levels {
    warn: Option<u32>,
    gate: Option<u32>,
}

struct ResolvedF03 {
    warn: u32,
    gate: u32,
}

struct F03Key<'a> {
    label: &'a str,
    key: &'a str,
    item: &'a toml::Value,
}

fn apply_f03(
    config: &mut Config,
    language: Language,
    value: &toml::Value,
) -> Result<(), ConfigError> {
    let label = language_name(language);
    let parsed = f03_levels(value, label)?;
    let levels = checked_f03(language, label, parsed)?;
    let profile = config.profile_mut(language);
    profile.f03_warn = levels.warn;
    profile.f03_gate = levels.gate;
    Ok(())
}

fn f03_levels(value: &toml::Value, label: &str) -> Result<F03Levels, ConfigError> {
    let Some(table) = value.as_table() else {
        return Err(ConfigError::WrongType(format!("profile.{label}.F03")));
    };
    if table.is_empty() {
        return Err(ConfigError::EmptyOverride(format!("profile.{label}.F03")));
    }
    let mut levels = F03Levels {
        warn: None,
        gate: None,
    };
    for (key, item) in table {
        let field = F03Key {
            label,
            key: key.as_str(),
            item,
        };
        read_f03_key(&mut levels, field)?;
    }
    if levels_unset(&levels) {
        return Err(ConfigError::EmptyOverride(format!("profile.{label}.F03")));
    }
    Ok(levels)
}

fn levels_unset(levels: &F03Levels) -> bool {
    let warn = levels.warn;
    let gate = levels.gate;
    warn.is_none() && gate.is_none()
}

fn read_f03_key(levels: &mut F03Levels, field: F03Key<'_>) -> Result<(), ConfigError> {
    match field.key {
        "warn" => {
            let key = format!("profile.{}.F03.warn", field.label);
            let number = nonneg(field.item, &key)?;
            levels.warn = Some(number);
        }
        "gate" => {
            let key = format!("profile.{}.F03.gate", field.label);
            let number = nonneg(field.item, &key)?;
            levels.gate = Some(number);
        }
        other => {
            return Err(ConfigError::UnknownKey(format!(
                "profile.{}.F03.{other}",
                field.label
            )));
        }
    }
    Ok(())
}

fn checked_f03(
    language: Language,
    label: &str,
    levels: F03Levels,
) -> Result<ResolvedF03, ConfigError> {
    let builtin_profile = LanguageProfile::builtin(language);
    let builtin_warn = builtin_profile.f03_warn;
    let builtin_gate = builtin_profile.f03_gate;
    let chosen_warn = levels.warn.unwrap_or(builtin_warn);
    let chosen_gate = levels.gate.unwrap_or(builtin_gate);
    if chosen_warn > F03_WARN_MAX {
        return Err(ConfigError::OutOfRange(format!("profile.{label}.F03.warn")));
    }
    let ceiling = builtin_gate.saturating_mul(2);
    if chosen_gate < 1 || chosen_gate > ceiling {
        return Err(ConfigError::OutOfRange(format!("profile.{label}.F03.gate")));
    }
    if chosen_warn >= chosen_gate {
        return Err(ConfigError::WarnNotBelowGate {
            language: label.to_string(),
            warn: chosen_warn,
            gate: chosen_gate,
        });
    }
    Ok(ResolvedF03 {
        warn: chosen_warn,
        gate: chosen_gate,
    })
}

fn apply_f08(
    config: &mut Config,
    language: Language,
    value: &toml::Value,
) -> Result<(), ConfigError> {
    let label = language_name(language);
    let names = f08_names(value, label)?;
    reject_allow_names(&names)?;
    let profile = config.profile_mut(language);
    profile.f08_allow = names;
    Ok(())
}

fn apply_f09(
    config: &mut Config,
    language: Language,
    value: &toml::Value,
) -> Result<(), ConfigError> {
    let label = language_name(language);
    let names = f09_names(value, label)?;
    reject_io_names(&names)?;
    let profile = config.profile_mut(language);
    profile.f09_allow = names;
    Ok(())
}

fn f09_names(value: &toml::Value, label: &str) -> Result<Vec<String>, ConfigError> {
    let Some(table) = value.as_table() else {
        return Err(ConfigError::WrongType(format!("profile.{label}.F09")));
    };
    if table.is_empty() {
        return Err(ConfigError::EmptyOverride(format!("profile.{label}.F09")));
    }
    let mut allow = None;
    for (key, item) in table {
        allow = one_f09_allow(key.as_str(), item, label)?;
    }
    allow.ok_or_else(|| ConfigError::EmptyOverride(format!("profile.{label}.F09")))
}

fn one_f09_allow(
    key: &str,
    item: &toml::Value,
    label: &str,
) -> Result<Option<Vec<String>>, ConfigError> {
    match key {
        "allow" => {
            let names = string_list(item, &format!("profile.{label}.F09.allow"))?;
            Ok(Some(names))
        }
        other => Err(ConfigError::UnknownKey(format!(
            "profile.{label}.F09.{other}"
        ))),
    }
}

fn reject_io_names(names: &[String]) -> Result<(), ConfigError> {
    for name in names {
        if bad_io_name(name) {
            return Err(ConfigError::InvalidName(name.clone()));
        }
    }
    Ok(())
}

fn bad_io_name(name: &str) -> bool {
    if name.is_empty() {
        return true;
    }
    let mut chars = name.chars();
    chars.any(char::is_whitespace)
}

fn f08_names(value: &toml::Value, label: &str) -> Result<Vec<String>, ConfigError> {
    let Some(table) = value.as_table() else {
        return Err(ConfigError::WrongType(format!("profile.{label}.F08")));
    };
    if table.is_empty() {
        return Err(ConfigError::EmptyOverride(format!("profile.{label}.F08")));
    }
    let mut allow = None;
    for (key, item) in table {
        allow = one_f08_allow(key.as_str(), item, label)?;
    }
    allow.ok_or_else(|| ConfigError::EmptyOverride(format!("profile.{label}.F08")))
}

fn one_f08_allow(
    key: &str,
    item: &toml::Value,
    label: &str,
) -> Result<Option<Vec<String>>, ConfigError> {
    match key {
        "allow" => {
            let names = string_list(item, &format!("profile.{label}.F08.allow"))?;
            Ok(Some(names))
        }
        other => Err(ConfigError::UnknownKey(format!(
            "profile.{label}.F08.{other}"
        ))),
    }
}

fn reject_allow_names(names: &[String]) -> Result<(), ConfigError> {
    for name in names {
        if bad_allow_name(name) {
            return Err(ConfigError::InvalidName(name.clone()));
        }
    }
    Ok(())
}

fn bad_allow_name(name: &str) -> bool {
    name.is_empty() || name.contains(['.', ':', '/'])
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
    let mut segments = path.split(['/', '\\']);
    if segments.any(|segment| segment == "..") {
        return Err(ConfigError::InvalidPath(path.to_string()));
    }
    Ok(())
}

fn validate_pattern(pattern: &str) -> Result<(), ConfigError> {
    if pattern.is_empty() || pattern.contains('!') {
        return Err(ConfigError::InvalidPath(pattern.to_string()));
    }
    let trimmed = pattern.trim_end_matches('/');
    if bad_pattern_root(trimmed) {
        return Err(ConfigError::InvalidPath(pattern.to_string()));
    }
    for segment in trimmed.split(['/', '\\']) {
        if bad_pattern_segment(segment) {
            return Err(ConfigError::InvalidPath(pattern.to_string()));
        }
    }
    Ok(())
}

fn bad_pattern_root(trimmed: &str) -> bool {
    trimmed.is_empty()
        || trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || has_drive_prefix(trimmed)
}

fn bad_pattern_segment(segment: &str) -> bool {
    if segment.is_empty() || segment == "." || segment == ".." {
        return true;
    }
    segment.contains("**") && segment != "**"
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
    let prefix = parts[0];
    if !segment.starts_with(prefix) {
        return false;
    }
    let rest = &segment[prefix.len()..];
    suffix_matches(&parts, rest)
}

fn suffix_matches(parts: &[&str], mut rest: &str) -> bool {
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
        let length = part.len();
        rest = &rest[found + length..];
    }
    true
}

struct GateNote<'a> {
    language: &'a str,
    rule: &'a str,
    value: u32,
    builtin: u32,
}

struct Departure<'a> {
    name: &'a str,
    current: &'a LanguageProfile,
    builtin: &'a LanguageProfile,
}

pub fn departures(config: &Config) -> Vec<String> {
    let mut notes = Vec::new();
    for language in [Language::Python, Language::Rust, Language::Cpp] {
        let current = config.profile(language);
        let builtin = LanguageProfile::builtin(language);
        let seen = Departure {
            name: language_label(language),
            current,
            builtin: &builtin,
        };
        note_language(&mut notes, &seen);
    }
    notes
}

fn note_language(notes: &mut Vec<String>, seen: &Departure<'_>) {
    note_rule(notes, seen, "F01");
    note_rule(notes, seen, "F02");
    note_f03_warn(notes, seen);
    note_rule(notes, seen, "F03");
    note_rule(notes, seen, "F04");
    note_rule(notes, seen, "F05");
    note_rule(notes, seen, "F06");
    note_f08_allow(notes, seen);
    note_f09_allow(notes, seen);
    note_rule(notes, seen, "F11");
}

fn note_rule(notes: &mut Vec<String>, seen: &Departure<'_>, rule: &str) {
    let current = seen.current;
    let builtin = seen.builtin;
    let (value, baseline) = gate_pair(current, builtin, rule);
    let name = seen.name;
    note_gate(
        notes,
        GateNote {
            language: name,
            rule,
            value,
            builtin: baseline,
        },
    );
}

fn gate_pair(current: &LanguageProfile, builtin: &LanguageProfile, rule: &str) -> (u32, u32) {
    match rule {
        "F01" => (current.f01_gate, builtin.f01_gate),
        "F02" => (current.f02_gate, builtin.f02_gate),
        "F03" => (current.f03_gate, builtin.f03_gate),
        "F04" => (current.f04_gate, builtin.f04_gate),
        "F05" => (current.f05_gate, builtin.f05_gate),
        "F06" => (current.f06_gate, builtin.f06_gate),
        "F11" => (current.f11_gate, builtin.f11_gate),
        _ => (0, 0),
    }
}

fn note_f03_warn(notes: &mut Vec<String>, seen: &Departure<'_>) {
    let current = seen.current;
    let builtin = seen.builtin;
    let warn = current.f03_warn;
    let baseline = builtin.f03_warn;
    if warn != baseline {
        let name = seen.name;
        notes.push(format!("{name} F03 warn is {warn}, built-in {baseline}"));
    }
}

fn note_f08_allow(notes: &mut Vec<String>, seen: &Departure<'_>) {
    let current = seen.current;
    let allow = &current.f08_allow;
    if allow.is_empty() {
        return;
    }
    let listed = allow.join(", ");
    let name = seen.name;
    notes.push(format!("{name} F08 allow adds {listed}"));
}

fn note_f09_allow(notes: &mut Vec<String>, seen: &Departure<'_>) {
    let current = seen.current;
    let allow = &current.f09_allow;
    let name = seen.name;
    for listed in allow {
        notes.push(format!("{name} F09 allow adds {listed}"));
    }
}

fn note_gate(notes: &mut Vec<String>, note: GateNote<'_>) {
    if note.value != note.builtin {
        notes.push(format!(
            "{} {} gate is {}, built-in {}",
            note.language, note.rule, note.value, note.builtin
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
    fn an_f09_allow_is_a_departure_per_name_and_a_gate_key_fails() {
        let root = fixture("f09");
        write_config(
            &root,
            "[profile.python.F09]\nallow = [\"open\", \"sqlite3.connect\"]\n",
        );
        let config = load_config(&root).unwrap();
        assert_eq!(
            config.profile(Language::Python).f09_allow,
            vec!["open".to_string(), "sqlite3.connect".to_string()]
        );
        assert_eq!(
            departures(&config),
            vec![
                "Python F09 allow adds open".to_string(),
                "Python F09 allow adds sqlite3.connect".to_string(),
            ]
        );

        write_config(&root, "[profile.python.F09]\ngate = 1\n");
        let err = load_config(&root).unwrap_err().to_string();
        assert!(err.contains("profile.python.F09.gate"), "{err}");

        write_config(&root, "[profile.rust.F09]\nallow = [\"\"]\n");
        assert!(matches!(
            load_config(&root).unwrap_err(),
            ConfigError::InvalidName(_)
        ));
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
            "[profile.python.F15]\ngate = 0\n",
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
