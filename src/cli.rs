use crate::explain::page;
use crate::reporters::{no_check, Rendered};

pub fn run(args: &[String]) -> Option<Rendered> {
    match args.first().map(String::as_str) {
        Some("explain") => Some(explain(&args[1..])),
        _ => None,
    }
}

fn explain(args: &[String]) -> Rendered {
    let [rule] = args else {
        return no_check(if args.is_empty() {
            "explain needs a rule"
        } else {
            "explain takes one rule"
        });
    };
    match page(rule) {
        Some(text) => Rendered {
            stdout: text.to_string(),
            stderr: String::new(),
            code: 0,
        },
        None => no_check(&format!("unknown rule {rule}")),
    }
}
