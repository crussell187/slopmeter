pub mod python;

use crate::ir::{File, Language};

pub fn parse(language: Language, path: &str, source: &[u8]) -> Result<File, String> {
    match language {
        Language::Python => Ok(python::parse(path, source)),
        Language::Rust => Err(format!("no Rust adapter for {path}")),
        Language::Cpp => Err(format!("no C++ adapter for {path}")),
    }
}

pub fn allow(language: Language) -> &'static [&'static str] {
    match language {
        Language::Python => python::ALLOW,
        Language::Rust | Language::Cpp => &[],
    }
}
