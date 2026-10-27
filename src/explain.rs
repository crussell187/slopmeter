pub fn page(rule: &str) -> Option<&'static str> {
    Some(match rule {
        "F01" => include_str!("../docs/explain/F01.txt"),
        "F02" => include_str!("../docs/explain/F02.txt"),
        "F03" => include_str!("../docs/explain/F03.txt"),
        "F04" => include_str!("../docs/explain/F04.txt"),
        "F05" => include_str!("../docs/explain/F05.txt"),
        "F06" => include_str!("../docs/explain/F06.txt"),
        "F07" => include_str!("../docs/explain/F07.txt"),
        "F08" => include_str!("../docs/explain/F08.txt"),
        "F11" => include_str!("../docs/explain/F11.txt"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::page;

    const RULES: [&str; 9] = [
        "F01", "F02", "F03", "F04", "F05", "F06", "F07", "F08", "F11",
    ];
    const GATES: [&str; 8] = ["F01", "F02", "F03", "F04", "F05", "F06", "F08", "F11"];
    const WAIVER: &str = "A contiguous `# slopmeter: waive <RULE> <NUMBER> <REASON>` run directly above the function excuses this gate in Python, and the same tokens after `//` do so in Rust and C++, with a whole number except on F02, which may be fractional.";

    #[test]
    fn a_known_rule_has_a_page_and_an_unknown_rule_does_not() {
        for rule in RULES {
            assert!(page(rule).is_some(), "{rule}");
        }
        assert!(page("F09").is_none());
        assert!(page("f01").is_none());
    }

    #[test]
    fn each_page_has_the_frozen_sections_in_order() {
        for rule in RULES {
            let text = page(rule).unwrap();
            let counting = text.find("\nCounting\n").unwrap();
            let threshold = text.find("\nThreshold\n").unwrap();
            let before = text.find("\nBefore\n").unwrap();
            let after = text.find("\nAfter\n").unwrap();
            let anti = text.find("\nAnti-fixes\n").unwrap();
            assert!(counting < threshold && threshold < before && before < after && after < anti);
            assert!(!text[anti + "\nAnti-fixes\n".len()..].trim().is_empty());
        }
    }

    #[test]
    fn every_gate_page_carries_the_same_waiver_sentence() {
        for rule in GATES {
            let text = page(rule)
                .unwrap()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            assert!(text.contains(WAIVER), "{rule}");
        }
        let f07 = page("F07").unwrap();
        assert!(f07.contains("A waiver line for F07 fails the run, because F07 is not a gate."));
        assert!(!f07.contains("excuses this gate"));
    }

    #[test]
    fn each_threshold_is_marked_as_the_shipped_stand_in() {
        for rule in RULES {
            let text = page(rule).unwrap();
            let threshold = text.find("\nThreshold\n").unwrap();
            let before = text.find("\nBefore\n").unwrap();
            assert!(text[threshold..before].contains("Shipped stand-in:"));
        }
    }

    #[test]
    fn each_page_stays_within_a_page() {
        for rule in RULES {
            let text = page(rule).unwrap();
            let lines: Vec<&str> = text.lines().collect();
            assert!(lines.len() <= 60, "{rule} has {} lines", lines.len());
            for line in lines {
                assert!(line.chars().count() <= 72, "{rule}: {line}");
                assert!(!line.ends_with(' '), "{rule}");
            }
        }
    }
}
