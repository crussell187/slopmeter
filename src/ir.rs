#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Python,
    Rust,
    Cpp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub receiver: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    pub line: u32,
    pub depth: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CognitiveKind {
    Structural,
    Hybrid,
    Fundamental,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cognitive {
    pub line: u32,
    pub kind: CognitiveKind,
    pub depth: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    pub name: String,
    pub call: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chain {
    pub line: u32,
    pub rooted: bool,
    pub members: Vec<Member>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Forward {
    pub line: u32,
    pub names: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Body {
    pub lloc: u32,
    pub decisions: Vec<Decision>,
    pub cognitive: Vec<Cognitive>,
    pub chains: Vec<Chain>,
    pub forward: Option<Forward>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub param_types: Vec<String>,
    pub qualifier: String,
    pub span: Span,
    pub touch: Span,
    pub params: Vec<Param>,
    pub body: Body,
}

#[derive(Clone, Debug, PartialEq)]
pub struct File {
    pub path: String,
    pub language: Language,
    pub unparsed_ratio: f64,
    pub functions: Vec<Function>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hand_built_function_keeps_its_classification() {
        let function = Function {
            name: "get".to_string(),
            param_types: vec![],
            qualifier: String::new(),
            span: Span { start: 74, end: 87 },
            touch: Span { start: 74, end: 87 },
            params: vec![
                Param {
                    name: "url".to_string(),
                    receiver: false,
                },
                Param {
                    name: "params".to_string(),
                    receiver: false,
                },
                Param {
                    name: "kwargs".to_string(),
                    receiver: false,
                },
            ],
            body: Body {
                lloc: 1,
                decisions: vec![],
                cognitive: vec![],
                chains: vec![],
                forward: Some(Forward {
                    line: 87,
                    names: vec![
                        "url".to_string(),
                        "params".to_string(),
                        "kwargs".to_string(),
                    ],
                }),
            },
        };
        let file = File {
            path: "src/requests/api.py".to_string(),
            language: Language::Python,
            unparsed_ratio: 0.0,
            functions: vec![function],
        };

        assert_eq!(file.functions[0].name, "get");
        assert_eq!(file.functions[0].span, Span { start: 74, end: 87 });
        assert_eq!(file.functions[0].body.lloc, 1);
        assert_eq!(
            file.functions[0]
                .body
                .forward
                .as_ref()
                .map(|f| f.names.len()),
            Some(3)
        );
        assert_eq!(file.language, Language::Python);
    }
}
