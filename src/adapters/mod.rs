pub mod cpp;
pub mod python;
pub mod rust;

use crate::ir::{File, Language};

pub fn parse(language: Language, path: &str, source: &[u8]) -> Result<File, String> {
    match language {
        Language::Python => Ok(python::parse(path, source)),
        Language::Rust => Ok(rust::parse(path, source)),
        Language::Cpp => Ok(cpp::parse(path, source)),
    }
}

pub fn allow(language: Language) -> &'static [&'static str] {
    match language {
        Language::Python => python::ALLOW,
        Language::Rust => rust::ALLOW,
        Language::Cpp => cpp::ALLOW,
    }
}
